import assert from 'node:assert/strict'
import * as Abis from '@filoz/synapse-core/abis'
import {
  DeletePieceError,
  TerminateServiceError,
  WaitForTerminateServiceRejectedError,
} from '@filoz/synapse-core/errors'
import * as SP from '@filoz/synapse-core/sp'
import { type Abi, BaseError, ContractFunctionRevertedError, decodeFunctionData, type Hash } from 'viem'
import { getTransaction, simulateContract, waitForTransactionReceipt } from 'viem/actions'
import type { ScenarioSynapse } from './account.ts'

type RejectionReason = 'non-owner' | 'malformed'

const EXPECTED_ERRORS = {
  'non-owner': ['InvalidSignature', 'CallerNotPayer'],
  malformed: ['ExtraDataRequired'],
} satisfies Record<RejectionReason, string[]>

function hasExpectedReason(error: DeletePieceError | TerminateServiceError, reason: RejectionReason): boolean {
  const details = error.details ?? ''
  // The SDK decodes contract errors in HTTP response bodies into ABI signatures.
  if (EXPECTED_ERRORS[reason].some((name) => details.startsWith(`Warm Storage\n${name}(`))) return true
  return (
    reason === 'malformed' &&
    details.trim() === 'Service Provider PDP\nextraData is required for client-requested termination'
  )
}

async function assertTransactionRejected(
  synapse: ScenarioSynapse,
  hash: Hash,
  reason: RejectionReason,
  label: string
): Promise<void> {
  const receipt = await waitForTransactionReceipt(synapse.client, { hash })
  assert.equal(receipt.status, 'reverted', `${label} transaction unexpectedly succeeded`)
  const transaction = await getTransaction(synapse.client, { hash })
  assert(transaction.to != null, `${label} transaction has no recipient`)
  const abi: Abi = [...Abis.pdp, ...Abis.fwss]
  const { functionName, args } = decodeFunctionData({ abi, data: transaction.input })
  try {
    // A failed receipt alone cannot distinguish an authorization rejection from
    // a provider/payment failure. Replay the call to decode the actual revert.
    await simulateContract(synapse.client, {
      abi,
      address: transaction.to,
      functionName,
      args,
      account: transaction.from,
      value: transaction.value,
      blockNumber: receipt.blockNumber,
    })
  } catch (error) {
    const revert =
      error instanceof BaseError ? error.walk((cause) => cause instanceof ContractFunctionRevertedError) : undefined
    if (
      revert instanceof ContractFunctionRevertedError &&
      revert.data != null &&
      EXPECTED_ERRORS[reason].includes(revert.data.errorName)
    ) {
      console.log(`Rejected as expected: ${label}: ${revert.data.errorName}, tx=${hash}`)
      return
    }
    throw error
  }
  throw new Error(`${label}: could not establish the expected ${reason} rejection for tx ${hash}`)
}

export async function assertTerminationRejected(
  synapse: ScenarioSynapse,
  operation: () => Promise<SP.terminateService.OutputType>,
  reason: RejectionReason,
  label: string
): Promise<void> {
  try {
    const { statusUrl } = await operation()
    await SP.waitForTerminateService({ statusUrl, timeout: 15_000, pollInterval: 1000 })
  } catch (error) {
    if (TerminateServiceError.is(error) && hasExpectedReason(error, reason)) {
      console.log(`Rejected as expected: ${label}: ${error.message}`)
      return
    }
    if (WaitForTerminateServiceRejectedError.is(error)) {
      await assertTransactionRejected(synapse, error.hash, reason, label)
      return
    }
    // Timeouts, missing statuses, and unrelated HTTP/RPC failures are inconclusive.
    throw error
  }
  throw new Error(`${label} unexpectedly terminated the data set`)
}

export async function assertDeletionRejected(
  synapse: ScenarioSynapse,
  operation: () => Promise<SP.deletePiece.OutputType>,
  reason: RejectionReason,
  label: string
): Promise<void> {
  let hash: Hash
  try {
    const result = await operation()
    hash = result.hash
  } catch (error) {
    if (DeletePieceError.is(error) && hasExpectedReason(error, reason)) {
      console.log(`Rejected as expected: ${label}: ${error.message}`)
      return
    }
    throw error
  }
  await assertTransactionRejected(synapse, hash, reason, label)
}
