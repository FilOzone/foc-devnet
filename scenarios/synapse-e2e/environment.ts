import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { calibration, mainnet } from '@filoz/synapse-core/chains'
import { toChain, validateDevnetInfo } from '@filoz/synapse-core/devnet'
import type { SynapseOptions } from '@filoz/synapse-sdk'

export type ScenarioEnvironment = {
  chain: NonNullable<SynapseOptions['chain']>
  filePaths: string[]
  network: string
  privateKey: `0x${string}`
  runId?: string
  user?: { name: string; evm_addr: string }
}

export function freshMetadata(kind: string): Record<string, string> {
  return {
    scenario: kind,
    run: `${Date.now().toString(36)}-${process.pid.toString(36)}`,
  }
}

export function resolveEnvironment(options: { defaultUserIndex: number; requireFiles?: boolean }): ScenarioEnvironment {
  const filePaths = process.argv.slice(2)
  assert(!options.requireFiles || filePaths.length > 0, 'Usage: node <entrypoint> <file-path>')

  const network = process.env.NETWORK ?? 'devnet'
  const rpcUrl = process.env.RPC_URL
  if (network !== 'devnet') {
    const chain = network === 'mainnet' ? mainnet : calibration
    const privateKey = process.env.PRIVATE_KEY
    assert(privateKey?.startsWith('0x'), 'PRIVATE_KEY must be a 0x-prefixed private key outside devnet')
    return {
      chain: rpcUrl ? { ...chain, rpcUrls: { ...chain.rpcUrls, default: { http: [rpcUrl] } } } : chain,
      filePaths,
      network,
      privateKey: privateKey as `0x${string}`,
    }
  }

  const baseDir = process.env.FOC_DEVNET_BASEDIR ?? join(homedir(), '.foc-devnet')
  const infoPath = process.env.DEVNET_INFO_PATH ?? join(baseDir, 'state', 'latest', 'devnet-info.json')
  const devnet = validateDevnetInfo(JSON.parse(readFileSync(infoPath, 'utf8')))
  const userIndex = Number(process.env.DEVNET_USER_INDEX ?? options.defaultUserIndex)
  assert(Number.isInteger(userIndex), `DEVNET_USER_INDEX must be an integer, got ${process.env.DEVNET_USER_INDEX}`)
  assert(userIndex >= 0 && userIndex < devnet.info.users.length, `DEVNET_USER_INDEX ${userIndex} is out of range`)

  const user = devnet.info.users[userIndex]
  const chain = toChain(devnet)
  return {
    chain: rpcUrl
      ? { ...chain, rpcUrls: { ...chain.rpcUrls, default: { http: [rpcUrl] }, public: { http: [rpcUrl] } } }
      : chain,
    filePaths,
    network,
    privateKey: (process.env.PRIVATE_KEY ?? user.private_key_hex) as `0x${string}`,
    runId: devnet.info.run_id,
    user,
  }
}
