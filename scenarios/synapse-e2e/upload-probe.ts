/**
 * Uploads one file and verifies its Synapse and on-chain result.
 *
 * The cache scenario runs this probe before inspecting Curio's Scylla rows.
 * `UPLOAD_COPIES` selects the copy count and defaults to one.
 */
import assert from 'node:assert/strict'
import { createSynapse, prepareAccount } from './account.ts'
import { freshMetadata, resolveEnvironment } from './environment.ts'
import { assertOnchainState } from './onchain.ts'
import { assertDirectRetrievals, assertDownloadedBytes, fileSize, uploadFile } from './storage.ts'

async function main(): Promise<void> {
  const environment = resolveEnvironment({ defaultUserIndex: 0, requireFiles: true })
  if (environment.filePaths.length !== 1) throw new Error('upload-probe.ts accepts exactly one file path')
  const copies = Number(process.env.UPLOAD_COPIES ?? '1')
  assert(
    Number.isInteger(copies) && copies > 0,
    `UPLOAD_COPIES must be a positive integer, got ${process.env.UPLOAD_COPIES}`
  )

  const [filePath] = environment.filePaths
  const metadata = freshMetadata('upload-probe')
  const synapse = createSynapse(environment)
  console.log(`=== Synapse upload probe: ${copies} copy target ===`)
  const preparedAccount = await prepareAccount(synapse, await fileSize(filePath))

  const { result } = await uploadFile(synapse, filePath, metadata, copies)
  assert.equal(result.complete, true, `Upload incomplete: ${result.copies.length}/${result.requestedCopies} copies`)
  assert.equal(result.copies.length, copies, `Expected ${copies} committed copies, got ${result.copies.length}`)
  await assertDownloadedBytes(synapse, result.pieceCid, filePath)
  await assertDirectRetrievals(result, filePath)
  await assertOnchainState(synapse, result, metadata, preparedAccount)
  console.log(`Upload probe verified: ${result.pieceCid}`)
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
