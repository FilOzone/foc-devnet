import { createSynapse, prepareAccount } from './account.ts'
import { freshMetadata, resolveEnvironment } from './environment.ts'
import { assertOnchainState } from './onchain.ts'
import {
  assertCompleteReplication,
  assertDirectRetrievals,
  assertDownloadedBytes,
  fileSize,
  uploadFile,
} from './storage.ts'

async function main(): Promise<void> {
  const environment = resolveEnvironment({ defaultUserIndex: 0, requireFiles: true })
  if (environment.filePaths.length !== 1) throw new Error('system-e2e.ts accepts exactly one file path')

  const [filePath] = environment.filePaths
  const metadata = freshMetadata('system-e2e')
  console.log('=== Synapse system E2E ===')
  console.log(`Network: ${environment.network}${environment.runId == null ? '' : ` run=${environment.runId}`}`)
  console.log(`Fresh metadata: ${JSON.stringify(metadata)}`)

  console.log('\nPhase 1: initialize account')
  const synapse = createSynapse(environment)
  console.log(`Wallet: ${synapse.client.account.address}`)

  console.log('\nPhase 2: prepare payment account')
  const preparedAccount = await prepareAccount(synapse, await fileSize(filePath))

  console.log('\nPhase 3: upload and replicate')
  const { result, milestones } = await uploadFile(synapse, filePath, metadata, 2)
  assertCompleteReplication(result, milestones)
  console.log(`Replication complete: piece=${result.pieceCid}`)

  console.log('\nPhase 4: verify provider-agnostic retrieval')
  await assertDownloadedBytes(synapse, result.pieceCid, filePath)
  console.log('Provider-agnostic retrieval verified')

  console.log('\nPhase 5: verify every provider directly')
  await assertDirectRetrievals(result, filePath)

  console.log('\nPhase 6: verify observable on-chain state')
  await assertOnchainState(synapse, result, metadata, preparedAccount)
  console.log('\n=== SUCCESS: complete two-provider Synapse journey verified ===')
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
