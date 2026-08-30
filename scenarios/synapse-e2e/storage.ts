import assert from 'node:assert/strict'
import { createReadStream } from 'node:fs'
import { readFile, stat } from 'node:fs/promises'
import { Readable } from 'node:stream'
import type { UploadResult } from '@filoz/synapse-sdk'
import type { ScenarioSynapse } from './account.ts'

export type UploadMilestones = {
  selected: Set<bigint>
  stored: Set<bigint>
  copied: Set<bigint>
  failed: Set<bigint>
  submitted: Set<bigint>
  confirmed: Set<bigint>
  uploadedBytes: number
}

export async function fileSize(filePath: string): Promise<bigint> {
  const info = await stat(filePath)
  assert(info.isFile(), `Path is not a file: ${filePath}`)
  return BigInt(info.size)
}

export async function createFreshContext(
  synapse: ScenarioSynapse,
  providerId: bigint,
  metadata: Record<string, string>
) {
  const context = await synapse.storage.createContext({ providerId, metadata, withCDN: false })
  assert(
    context.dataSetId === null || context.dataSetId === undefined,
    'Fresh metadata unexpectedly resolved an existing data set'
  )
  return context
}

export async function uploadFile(
  synapse: ScenarioSynapse,
  filePath: string,
  metadata: Record<string, string>,
  copies = 2
): Promise<{ result: UploadResult; milestones: UploadMilestones }> {
  const size = await fileSize(filePath)
  const milestones: UploadMilestones = {
    selected: new Set(),
    stored: new Set(),
    copied: new Set(),
    failed: new Set(),
    submitted: new Set(),
    confirmed: new Set(),
    uploadedBytes: 0,
  }
  let lastProgress = 0

  console.log(`Uploading ${filePath} (${size} bytes) with ${copies} copy target`)
  const result = await synapse.storage.upload(Readable.toWeb(createReadStream(filePath)), {
    copies,
    metadata,
    callbacks: {
      onProviderSelected: (provider) => {
        milestones.selected.add(provider.id)
        console.log(`Provider selected: ${provider.id} (${provider.serviceProvider})`)
      },
      onStored: (providerId, pieceCid) => {
        milestones.stored.add(providerId)
        console.log(`Primary stored: provider=${providerId} piece=${pieceCid}`)
      },
      onProgress: (bytesUploaded) => {
        milestones.uploadedBytes = bytesUploaded
        if (bytesUploaded === Number(size) || bytesUploaded - lastProgress >= 10 * 1024 * 1024) {
          lastProgress = bytesUploaded
          console.log(`Upload progress: ${bytesUploaded}/${size}`)
        }
      },
      onPullProgress: (providerId, pieceCid, status) =>
        console.log(`Replication: provider=${providerId} piece=${pieceCid} status=${status}`),
      onCopyComplete: (providerId, pieceCid) => {
        milestones.copied.add(providerId)
        console.log(`Replication complete: provider=${providerId} piece=${pieceCid}`)
      },
      onCopyFailed: (providerId, pieceCid, error) => {
        milestones.failed.add(providerId)
        console.error(`Replication failed: provider=${providerId} piece=${pieceCid}: ${error.message}`)
      },
      onPiecesAdded: (hash, providerId) => {
        milestones.submitted.add(providerId)
        console.log(`Commit submitted: provider=${providerId} tx=${hash}`)
      },
      onPiecesConfirmed: (dataSetId, providerId) => {
        milestones.confirmed.add(providerId)
        console.log(`Commit confirmed: provider=${providerId} dataSet=${dataSetId}`)
      },
    },
  })
  return { result, milestones }
}

export async function assertDownloadedBytes(
  synapse: ScenarioSynapse,
  pieceCid: UploadResult['pieceCid'],
  filePath: string
): Promise<void> {
  const [expected, downloaded] = await Promise.all([readFile(filePath), synapse.storage.download({ pieceCid })])
  assert(Buffer.from(downloaded).equals(expected), `Provider-agnostic download differs for piece ${pieceCid}`)
}

export async function assertDirectRetrievals(result: UploadResult, filePath: string): Promise<void> {
  const expected = await readFile(filePath)
  for (const copy of result.copies) {
    const response = await fetch(copy.retrievalUrl)
    assert(response.ok, `Direct retrieval from provider ${copy.providerId} failed with HTTP ${response.status}`)
    const body = new Uint8Array(await response.arrayBuffer())
    assert(Buffer.from(body).equals(expected), `Direct retrieval differs for provider ${copy.providerId}`)
    console.log(`Direct retrieval verified: provider=${copy.providerId} (${body.byteLength} bytes)`)
  }
}

export function assertCompleteReplication(result: UploadResult, milestones: UploadMilestones): void {
  assert.equal(result.requestedCopies, 2, `Expected exactly two requested copies, got ${result.requestedCopies}`)
  assert.equal(result.complete, true, `Upload incomplete: ${result.copies.length}/${result.requestedCopies} copies`)
  assert.equal(result.copies.length, 2, `Expected exactly two complete copies, got ${result.copies.length}`)
  assert.equal(result.failedAttempts.length, 0, 'Default replication produced failed provider attempts')

  const providerIds = new Set(result.copies.map((copy) => copy.providerId))
  const primaryIds = new Set(result.copies.filter((copy) => copy.role === 'primary').map((copy) => copy.providerId))
  const secondaryIds = new Set(result.copies.filter((copy) => copy.role === 'secondary').map((copy) => copy.providerId))
  assert.equal(providerIds.size, 2, 'Copies must be on distinct providers')
  assert.equal(primaryIds.size, 1, 'Expected one primary copy')
  assert.equal(secondaryIds.size, 1, 'Expected one secondary copy')
  assert.deepEqual(milestones.selected, providerIds, 'Provider selection callbacks differ from committed copies')
  assert.deepEqual(milestones.stored, primaryIds, 'Primary store callback differs from committed primary')
  assert.deepEqual(milestones.copied, secondaryIds, 'Copy callback differs from committed secondary')
  assert.deepEqual(milestones.failed, new Set(), 'Replication emitted a failure callback')
  assert.deepEqual(milestones.submitted, providerIds, 'Commit submissions differ from committed copies')
  assert.deepEqual(milestones.confirmed, providerIds, 'Commit confirmations differ from committed copies')
  assert.equal(milestones.uploadedBytes, result.size, 'Upload progress did not reach the complete input size')
}
