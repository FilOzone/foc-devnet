import assert from 'node:assert/strict'
import * as ERC20 from '@filoz/synapse-core/erc20'
import { accounts, deposit, setOperatorApproval } from '@filoz/synapse-core/pay'
import { Synapse } from '@filoz/synapse-sdk'
import type { StorageContext } from '@filoz/synapse-sdk/storage'
import { type Hash, http } from 'viem'
import { privateKeyToAccount } from 'viem/accounts'
import type { ScenarioEnvironment } from './environment.ts'

export type ScenarioSynapse = Synapse
export type AccountState = accounts.OutputType

export function createSynapse(environment: ScenarioEnvironment): ScenarioSynapse {
  const account = privateKeyToAccount(environment.privateKey)
  if (environment.user != null) {
    assert.equal(
      account.address.toLowerCase(),
      environment.user.evm_addr.toLowerCase(),
      'private key does not match devnet user'
    )
  }
  return Synapse.create({
    chain: environment.chain,
    transport: http(),
    account,
    source: 'foc-devnet-synapse-e2e',
  })
}

export async function readAccountState(synapse: ScenarioSynapse): Promise<AccountState> {
  return accounts(synapse.client, { address: synapse.client.account.address })
}

function delay(milliseconds: number): Promise<void> {
  const { promise, resolve } = Promise.withResolvers<void>()
  setTimeout(resolve, milliseconds)
  return promise
}

async function waitForTransaction(synapse: ScenarioSynapse, hash: Hash, label: string): Promise<void> {
  let lastError: unknown
  for (let attempt = 0; attempt < 90; attempt++) {
    try {
      const receipt = await synapse.client.request({
        method: 'eth_getTransactionReceipt',
        params: [hash],
      })
      if (receipt != null) return
    } catch (error) {
      lastError = error
    }
    await delay(1000)
  }
  const details = lastError instanceof Error ? `: ${lastError.message}` : ''
  throw new Error(`${label} transaction ${hash} was not confirmed${details}`)
}

export async function prepareAccount(synapse: ScenarioSynapse, dataSize: bigint): Promise<AccountState> {
  const before = await readAccountState(synapse)
  const { costs, transaction } = await synapse.storage.prepare({ dataSize })
  assert.equal(transaction == null, costs.ready, 'Account readiness and preparation transaction disagree')
  console.log(
    `Account preparation: ready=${costs.ready} deposit=${costs.depositNeeded} approval=${costs.needsFwssMaxApproval}`
  )
  if (transaction != null) {
    assert.equal(
      transaction.depositAmount,
      costs.depositNeeded,
      'Preparation transaction deposit differs from calculated cost'
    )
    assert.equal(
      transaction.includesApproval,
      costs.needsFwssMaxApproval,
      'Preparation transaction approval differs from calculated requirement'
    )
    const result = await transaction.execute({
      onHash: (hash) => console.log(`Account funding submitted: ${hash}`),
    })
    console.log(`Account funding confirmed: ${result.hash}`)
  }

  const ready = await synapse.storage.prepare({ dataSize })
  assert.equal(ready.costs.ready, true, 'Payment account remains unprepared')
  assert.equal(ready.transaction, null, 'Prepared payment account still requires a transaction')
  const after = await readAccountState(synapse)
  assert(after.funds >= before.funds, `Payment funds decreased during preparation: ${before.funds} -> ${after.funds}`)
  assert(after.availableFunds > 0n, 'Prepared payment account has no available funds')
  console.log(
    `Prepared payment state: funds=${after.funds} available=${after.availableFunds} lockup=${after.lockupCurrent}`
  )
  return after
}

export async function prepareAccountWithPlainErc20(
  synapse: ScenarioSynapse,
  dataSize: bigint,
  context: StorageContext
): Promise<AccountState> {
  const { costs, transaction } = await synapse.storage.prepare({ context, dataSize })
  assert.equal(transaction == null, costs.ready, 'Account readiness and preparation transaction disagree')
  console.log(
    `Plain ERC20 preparation: ready=${costs.ready} deposit=${costs.depositNeeded} approval=${costs.needsFwssMaxApproval}`
  )
  if (costs.depositNeeded > 0n) {
    const approvalHash = await ERC20.approve(synapse.client, {
      amount: costs.depositNeeded,
    })
    console.log(`USDFC approval submitted: ${approvalHash}`)
    await waitForTransaction(synapse, approvalHash, 'USDFC approval')

    const depositHash = await deposit(synapse.client, {
      amount: costs.depositNeeded,
    })
    console.log(`FilecoinPay deposit submitted: ${depositHash}`)
    await waitForTransaction(synapse, depositHash, 'FilecoinPay deposit')
  }
  if (costs.needsFwssMaxApproval) {
    const approvalHash = await setOperatorApproval(synapse.client, {
      approve: true,
    })
    console.log(`FWSS approval submitted: ${approvalHash}`)
    await waitForTransaction(synapse, approvalHash, 'FWSS approval')
  }

  const ready = await synapse.storage.prepare({ context, dataSize })
  assert.equal(ready.costs.ready, true, 'Plain ERC20 payment account remains unprepared')
  assert.equal(ready.transaction, null, 'Prepared plain ERC20 account still requires a transaction')
  const account = await readAccountState(synapse)
  assert(account.availableFunds > 0n, 'Prepared plain ERC20 account has no available funds')
  return account
}
