import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import * as Abis from '@filoz/synapse-core/abis'
import {
  DeletePieceError,
  TerminateServiceError,
  WaitForTerminateServiceRejectedError,
} from '@filoz/synapse-core/errors'
import * as SP from '@filoz/synapse-core/sp'
import { createPublicClient, custom, encodeErrorResult, encodeFunctionData } from 'viem'
import { privateKeyToAccount } from 'viem/accounts'
import { createSynapse, type ScenarioSynapse } from './account.ts'
import { resolveDistinctDevnetEnvironment, resolveEnvironment } from './environment.ts'
import { assertDeletionRejected, assertTerminationRejected } from './permission-checks.ts'

const address = '0x1111111111111111111111111111111111111111'
const otherAddress = '0x2222222222222222222222222222222222222222'
const hash = `0x${'ab'.repeat(32)}` as const
const invalidSignature = encodeErrorResult({
  abi: Abis.fwss,
  errorName: 'InvalidSignature',
  args: [address, otherAddress],
})
const extraDataRequired = encodeErrorResult({ abi: Abis.fwss, errorName: 'ExtraDataRequired' })
const unusedSynapse = { client: {} } as ScenarioSynapse

test('intruder remains distinct with owner index and private-key overrides', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'permission-users-'))
  const previousEnv = { ...process.env }
  t.after(() => {
    for (const key of ['NETWORK', 'DEVNET_INFO_PATH', 'DEVNET_USER_INDEX', 'PRIVATE_KEY']) {
      if (previousEnv[key] == null) delete process.env[key]
      else process.env[key] = previousEnv[key]
    }
    rmSync(directory, { recursive: true, force: true })
  })
  const users = [1, 2].map((index) => {
    const privateKey = `0x${index.toString(16).padStart(64, '0')}` as const
    return {
      name: `USER_${index}`,
      evm_addr: privateKeyToAccount(privateKey).address,
      native_addr: 't0100',
      private_key_hex: privateKey,
    }
  })
  const info = {
    version: 2,
    info: {
      run_id: 'permission-regression',
      start_time: '',
      startup_duration: '1s',
      users,
      contracts: Object.fromEntries(
        [
          'multicall3',
          'mockusdfc',
          'fwss_service_proxy',
          'fwss_state_view',
          'fwss_impl',
          'pdp_verifier_proxy',
          'pdp_verifier_impl',
          'service_provider_registry_proxy',
          'service_provider_registry_impl',
          'filecoin_pay_v1',
          'endorsements',
          'session_key_registry',
        ].map((name) => [`${name}_addr`, address])
      ),
      lotus: { host_rpc_url: 'http://localhost:1234', container_id: 'lotus', container_name: 'lotus' },
      lotus_miner: { container_id: 'miner', container_name: 'miner', api_port: 1235 },
      pdp_sps: [
        {
          provider_id: 1,
          eth_addr: address,
          native_addr: 't0101',
          pdp_service_url: 'http://localhost:1236',
          container_id: 'sp',
          container_name: 'sp',
          is_approved: true,
          is_endorsed: true,
        },
      ],
    },
  }
  process.env.NETWORK = 'devnet'
  process.env.DEVNET_INFO_PATH = join(directory, 'devnet-info.json')
  writeFileSync(process.env.DEVNET_INFO_PATH, JSON.stringify(info))
  for (const index of [0, 1]) {
    process.env.DEVNET_USER_INDEX = String(index)
    process.env.PRIVATE_KEY = users[index].private_key_hex
    const owner = resolveEnvironment({ defaultUserIndex: 0 })
    const peer = resolveDistinctDevnetEnvironment(owner)
    assert.notEqual(createSynapse(owner).client.account.address, createSynapse(peer).client.account.address)
    assert.equal(peer.chain, owner.chain)
  }
  info.info.users = [users[0]]
  writeFileSync(process.env.DEVNET_INFO_PATH, JSON.stringify(info))
  process.env.DEVNET_USER_INDEX = '0'
  process.env.PRIVATE_KEY = users[0].private_key_hex
  assert.throws(
    () => resolveDistinctDevnetEnvironment(resolveEnvironment({ defaultUserIndex: 0 })),
    /at least two distinct/
  )
})

test('HTTP permission checks accept only the expected rejection reason', async (t) => {
  let status = 500
  let body = 'database unavailable'
  const server = createServer((request, response) => {
    request.resume()
    response.writeHead(status, { 'content-type': 'text/plain' })
    response.end(body)
  })
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve))
  t.after(async () => {
    server.closeAllConnections()
    await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())))
  })
  const listener = server.address()
  assert(listener != null && typeof listener !== 'string')
  const serviceURL = `http://127.0.0.1:${listener.port}/`
  const terminate = () => SP.terminateServiceApiRequest({ serviceURL, dataSetId: 1n, extraData: '0x' })
  const remove = () => SP.deletePiece({ serviceURL, dataSetId: 1n, pieceId: 1n, extraData: '0x' })
  for (status of [500, 403, 404]) {
    await assert.rejects(
      assertTerminationRejected(unusedSynapse, terminate, 'non-owner', 'termination'),
      TerminateServiceError
    )
    await assert.rejects(assertDeletionRejected(unusedSynapse, remove, 'non-owner', 'deletion'), DeletePieceError)
  }
  status = 500 // Curio may wrap contract simulation failures in HTTP 500.
  body = `revert reason=[${invalidSignature}]`
  await assertTerminationRejected(unusedSynapse, terminate, 'non-owner', 'termination')
  await assertDeletionRejected(unusedSynapse, remove, 'non-owner', 'deletion')
  await assert.rejects(assertDeletionRejected(unusedSynapse, remove, 'malformed', 'deletion'), DeletePieceError)
  body = `revert reason=[${extraDataRequired}]`
  await assertDeletionRejected(unusedSynapse, remove, 'malformed', 'deletion')
  await assert.rejects(assertDeletionRejected(unusedSynapse, remove, 'non-owner', 'deletion'), DeletePieceError)
  status = 400
  body = 'extraData is required for client-requested termination\n'
  await assertTerminationRejected(unusedSynapse, terminate, 'malformed', 'termination')
  await assert.rejects(
    assertTerminationRejected(unusedSynapse, terminate, 'non-owner', 'termination'),
    TerminateServiceError
  )
  const timeout = Object.assign(new Error('Still queued'), { name: 'TimeoutError' })
  await assert.rejects(
    assertTerminationRejected(
      unusedSynapse,
      async () => {
        throw timeout
      },
      'non-owner',
      'termination'
    ),
    (error) => error === timeout
  )
  status = 404
  body = 'not found'
  await assert.rejects(
    assertTerminationRejected(unusedSynapse, async () => ({ statusUrl: serviceURL }), 'non-owner', 'termination')
  )
})

test('reverted transactions must have the expected contract rejection', async () => {
  let revertData = invalidSignature
  let receiptStatus = '0x0'
  let callSucceeds = false
  const client = createPublicClient({
    transport: custom(
      {
        async request({ method, params }) {
          if (method === 'eth_getTransactionReceipt')
            return {
              transactionHash: hash,
              blockHash: hash,
              blockNumber: '0x1',
              transactionIndex: '0x0',
              from: address,
              to: otherAddress,
              cumulativeGasUsed: '0x1',
              gasUsed: '0x1',
              logs: [],
              logsBloom: `0x${'00'.repeat(256)}`,
              status: receiptStatus,
              effectiveGasPrice: '0x1',
              type: '0x0',
              contractAddress: null,
            }
          if (method === 'eth_getTransactionByHash')
            return {
              hash,
              blockHash: hash,
              blockNumber: '0x1',
              transactionIndex: '0x0',
              nonce: '0x0',
              from: address,
              to: otherAddress,
              gas: '0x100000',
              gasPrice: '0x1',
              value: '0x0',
              type: '0x0',
              input: encodeFunctionData({
                abi: Abis.pdp,
                functionName: 'schedulePieceDeletions',
                args: [1n, [1n], '0x'],
              }),
              v: '0x1b',
              r: hash,
              s: hash,
            }
          if (method === 'eth_call') {
            assert.equal(params?.[1], '0x1', 'Replay must use the receipt block')
            if (callSucceeds) return '0x'
            throw { code: 3, message: 'execution reverted', data: revertData }
          }
          throw new Error(`Unexpected RPC method: ${method}`)
        },
      },
      { retryCount: 0 }
    ),
  })
  const synapse = { client } as unknown as ScenarioSynapse
  const remove = () => Promise.resolve({ hash })
  const terminate = async () => {
    throw new WaitForTerminateServiceRejectedError(hash)
  }
  await assertDeletionRejected(synapse, remove, 'non-owner', 'deletion')
  await assertTerminationRejected(synapse, terminate, 'non-owner', 'termination')
  revertData = extraDataRequired
  await assert.rejects(assertDeletionRejected(synapse, remove, 'non-owner', 'deletion'))
  await assert.rejects(assertTerminationRejected(synapse, terminate, 'non-owner', 'termination'))
  await assertDeletionRejected(synapse, remove, 'malformed', 'deletion')
  callSucceeds = true
  await assert.rejects(assertDeletionRejected(synapse, remove, 'malformed', 'deletion'), /could not establish/)
  receiptStatus = '0x1'
  await assert.rejects(assertDeletionRejected(synapse, remove, 'malformed', 'deletion'), /unexpectedly succeeded/)
})
