// Copied into synapse-sdk/utils before execution; imports are relative to that destination.
import { readFileSync } from 'fs'
import { homedir } from 'os'
import { join } from 'path'
import { http as viemHttp, maxUint256 } from 'viem'
import { privateKeyToAccount } from 'viem/accounts'
import { Synapse } from '../packages/synapse-sdk/src/index.ts'
import * as ERC20 from '../packages/synapse-core/src/erc20/index.ts'
import * as Pay from '../packages/synapse-core/src/pay/index.ts'
import * as SP from '../packages/synapse-core/src/sp/index.ts'
import { getPdpDataSet } from '@filoz/synapse-core/warm-storage'
import { toChain, validateDevnetInfo } from '../packages/synapse-core/src/devnet/index.ts'

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

function assert(condition, message) {
  if (!condition) {
    throw new Error(message)
  }
}

async function waitForDataSet(client, dataSetId) {
  for (let attempt = 1; attempt <= 15; attempt++) {
    const dataSet = await getPdpDataSet(client, { dataSetId })
    if (dataSet != null) {
      return dataSet
    }
    await sleep(2000)
  }
  throw new Error(`Created data set ${dataSetId} was not returned by getPdpDataSet`)
}

async function waitForTransactionReceipt(client, hash, label) {
  let lastError = null
  for (let attempt = 1; attempt <= 90; attempt++) {
    try {
      const receipt = await client.request({
        method: 'eth_getTransactionReceipt',
        params: [hash],
      })
      if (receipt != null) {
        return receipt
      }
    } catch (error) {
      lastError = error
    }

    await sleep(1000)
  }

  const suffix = lastError == null ? '' : `; last error: ${lastError.message}`
  throw new Error(`${label} transaction ${hash} was not confirmed${suffix}`)
}

async function prepareWithPlainErc20(synapse, context) {
  const { costs, transaction } = await synapse.storage.prepare({
    context,
    dataSize: 1n,
  })
  console.log(`Prepared account: ready=${costs.ready}, depositNeeded=${costs.depositNeeded}`)
  if (transaction == null) {
    return
  }

  if (costs.depositNeeded > 0n) {
    const approveHash = await ERC20.approve(synapse.client, {
      amount: costs.depositNeeded,
    })
    console.log(`ERC20 approve tx submitted: ${approveHash}`)
    const approved = await waitForTransactionReceipt(synapse.client, approveHash, 'ERC20 approve')
    console.log(`ERC20 approve confirmed in block ${approved.blockNumber}`)

    const depositHash = await Pay.deposit(synapse.client, {
      amount: costs.depositNeeded,
    })
    console.log(`FilecoinPay deposit tx submitted: ${depositHash}`)
    const deposited = await waitForTransactionReceipt(synapse.client, depositHash, 'FilecoinPay deposit')
    console.log(`FilecoinPay deposit confirmed in block ${deposited.blockNumber}`)
  }

  if (costs.needsFwssMaxApproval) {
    const operatorApprovalHash = await Pay.setOperatorApproval(synapse.client, {
      approve: true,
      rateAllowance: maxUint256,
      lockupAllowance: maxUint256,
    })
    console.log(`FWSS operator approval tx submitted: ${operatorApprovalHash}`)
    const approvedOperator = await waitForTransactionReceipt(
      synapse.client,
      operatorApprovalHash,
      'FWSS operator approval'
    )
    console.log(`FWSS operator approval confirmed in block ${approvedOperator.blockNumber}`)
  }
}

async function main() {
  const devnetInfoPath =
    process.env.DEVNET_INFO_PATH || join(homedir(), '.foc-devnet', 'state', 'latest', 'devnet-info.json')
  const userIndex = Number(process.env.DEVNET_USER_INDEX || '1')
  const raw = JSON.parse(readFileSync(devnetInfoPath, 'utf8'))
  const devnetInfo = validateDevnetInfo(raw)
  const { info } = devnetInfo

  assert(Number.isInteger(userIndex), `DEVNET_USER_INDEX must be an integer; got ${process.env.DEVNET_USER_INDEX}`)
  assert(userIndex >= 0 && userIndex < info.users.length, `DEVNET_USER_INDEX=${userIndex} out of range`)

  const devnetProvider = info.pdp_sps.find((provider) => provider.is_approved)
  assert(devnetProvider != null, 'No approved PDP service provider found in devnet-info.json')

  const user = info.users[userIndex]
  const chain = toChain(devnetInfo)
  if (process.env.RPC_URL) {
    chain.rpcUrls = {
      ...chain.rpcUrls,
      default: { http: [process.env.RPC_URL] },
      public: { http: [process.env.RPC_URL] },
    }
  }

  const account = privateKeyToAccount(user.private_key_hex)
  assert(
    account.address.toLowerCase() === user.evm_addr.toLowerCase(),
    `Derived address ${account.address} does not match ${user.name} address ${user.evm_addr}`
  )

  const source = 'foc-devnet-smoke'
  const smokeId = process.env.CREATE_DATASET_SMOKE_ID || `cds-${Date.now().toString(36)}`
  const metadata = { smoke: smokeId, source }

  console.log(`Devnet run: ${info.run_id}`)
  console.log(`User: ${user.name} (${account.address})`)
  console.log(`Provider: ${devnetProvider.provider_id} (${devnetProvider.pdp_service_url})`)
  console.log(`Smoke metadata: smoke=${smokeId}, source=${source}`)

  const synapse = Synapse.create({
    chain,
    transport: viemHttp(),
    account,
    source,
  })

  const provider = await synapse.providers.getProvider({ providerId: BigInt(devnetProvider.provider_id) })
  assert(provider != null, `Provider ${devnetProvider.provider_id} not found in registry`)

  const preExisting = await synapse.storage.findDataSets()
  assert(
    !preExisting.some((dataSet) => dataSet.metadata?.smoke === smokeId),
    `Smoke metadata ${smokeId} already exists before createDataSet`
  )

  const context = await synapse.storage.createContext({
    providerId: provider.id,
    metadata,
    withCDN: false,
  })
  assert(context.dataSetId == null, `Expected unique metadata to create a new data set, got ${context.dataSetId}`)

  await prepareWithPlainErc20(synapse, context)

  const create = await SP.createDataSet(synapse.client, {
    cdn: false,
    payee: provider.serviceProvider,
    payer: account.address,
    serviceURL: devnetProvider.pdp_service_url,
    recordKeeper: chain.contracts.fwss.address,
    metadata,
  })
  console.log(`createDataSet tx submitted: ${create.txHash}`)

  const confirmed = await SP.waitForCreateDataSet({
    statusUrl: create.statusUrl,
    timeout: 180000,
  })
  assert(confirmed.dataSetId > 0n, `Expected positive dataSetId, got ${confirmed.dataSetId}`)
  console.log(`createDataSet confirmed: dataSetId=${confirmed.dataSetId}`)

  const dataSet = await waitForDataSet(synapse.client, confirmed.dataSetId)
  assert(dataSet.live === true, `Data set ${confirmed.dataSetId} is not live`)
  assert(dataSet.managed === true, `Data set ${confirmed.dataSetId} is not managed by FWSS`)
  assert(dataSet.providerId === provider.id, `Data set providerId ${dataSet.providerId} != ${provider.id}`)
  assert(
    dataSet.payer.toLowerCase() === account.address.toLowerCase(),
    `Data set payer ${dataSet.payer} != ${account.address}`
  )
  assert(
    dataSet.payee.toLowerCase() === provider.serviceProvider.toLowerCase(),
    `Data set payee ${dataSet.payee} != ${provider.serviceProvider}`
  )
  assert(dataSet.metadata?.smoke === smokeId, `Data set smoke metadata missing or wrong: ${dataSet.metadata?.smoke}`)
  assert(dataSet.metadata?.source === source, `Data set source metadata missing or wrong: ${dataSet.metadata?.source}`)

  console.log(`Verified createDataSet smoke dataSetId=${confirmed.dataSetId}`)
}

main().catch((error) => {
  console.error(error)
  process.exit(1)
})
