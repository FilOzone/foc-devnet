import assert from 'node:assert/strict'
import * as SP from '@filoz/synapse-core/sp'
import { createSynapse, prepareAccountWithPlainErc20 } from './account.ts'
import { freshMetadata, resolveEnvironment } from './environment.ts'
import { assertCreatedDataSet } from './onchain.ts'
import { createFreshContext } from './storage.ts'

async function main(): Promise<void> {
  const environment = resolveEnvironment({ defaultUserIndex: 1 })
  const synapse = createSynapse(environment)
  const metadata = freshMetadata('create-data-set')

  console.log('=== Synapse data set creation probe ===')
  const providers = await synapse.providers.getAllActiveProviders()
  const provider = providers[0]
  assert(provider != null, 'No active PDP provider is available')
  console.log(`Provider: ${provider.id} (${provider.pdp.serviceURL})`)

  const context = await createFreshContext(synapse, provider.id, metadata)
  await prepareAccountWithPlainErc20(synapse, 1n, context)

  const created = await SP.createDataSet(synapse.client, {
    cdn: false,
    payee: provider.serviceProvider,
    payer: synapse.client.account.address,
    serviceURL: provider.pdp.serviceURL,
    recordKeeper: environment.chain.contracts.fwss.address,
    metadata,
  })
  console.log(`Data set creation submitted: ${created.txHash}`)
  const confirmed = await SP.waitForCreateDataSet({ statusUrl: created.statusUrl, timeout: 180_000 })
  assert(confirmed.dataSetId > 0n, `Expected a positive data set ID, got ${confirmed.dataSetId}`)

  await assertCreatedDataSet(synapse, confirmed.dataSetId, provider.id, provider.serviceProvider, metadata)
  console.log(`Data set created and verified: ${confirmed.dataSetId}`)
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
