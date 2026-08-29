import assert from 'node:assert/strict'
import * as SP from '@filoz/synapse-core/sp'
import { findPieceIdsByCidCall, getActivePieceCount } from '@filoz/synapse-core/pdp-verifier'
import { getPdpDataSet } from '@filoz/synapse-core/warm-storage'
import { readContract } from 'viem/actions'
import { createSynapse, prepareAccount, readAccountState } from './account.ts'
import { freshMetadata, resolveEnvironment } from './environment.ts'
import { fileSize, uploadFile } from './storage.ts'

const REQUIRED_PIECES = 40
const MAX_PIECES = 80
const SMALL_PIECE_BYTES = 64 * 1024

type PaymentSnapshot = {
  pieceNumber: number
  funds: bigint
  availableFunds: bigint
  lockupCurrent: bigint
  lockupRate: bigint
}

function fixtureFor(pieceNumber: number): File {
  const bytes = Buffer.alloc(SMALL_PIECE_BYTES, pieceNumber % 251)
  bytes.write(`foc-devnet-bulk-add-${pieceNumber}`, 0, 'utf8')
  return new File([bytes], `bulk-${pieceNumber.toString().padStart(3, '0')}.bin`, {
    type: 'application/octet-stream',
  })
}

async function main(): Promise<void> {
  const environment = resolveEnvironment({ defaultUserIndex: 0, requireFiles: true })
  if (environment.filePaths.length !== 1) throw new Error('bulk-add.ts accepts exactly one bootstrap file path')
  const [bootstrapPath] = environment.filePaths
  const synapse = createSynapse(environment)

  // Prepare enough headroom for creation plus the mandatory 40 small additions.
  await prepareAccount(synapse, (await fileSize(bootstrapPath)) + BigInt(SMALL_PIECE_BYTES * MAX_PIECES))
  const { result } = await uploadFile(synapse, bootstrapPath, freshMetadata('bulk-add'), 1)
  const copy = result.copies[0]
  assert(copy != null, 'Expected a bootstrap data set')
  const dataSet = await getPdpDataSet(synapse.client, { dataSetId: copy.dataSetId })
  assert(dataSet != null && dataSet.live, `Bootstrap data set ${copy.dataSetId} is not live`)
  const initialPieceCount = await getActivePieceCount(synapse.client, { dataSetId: copy.dataSetId })

  const snapshots: PaymentSnapshot[] = []
  const pieceCids = []
  let replenishedAt: number | undefined
  let previous = await readAccountState(synapse)

  for (let pieceNumber = 1; pieceNumber <= MAX_PIECES; pieceNumber++) {
    const added = await SP.upload(synapse.client, {
      dataSetId: copy.dataSetId,
      data: [fixtureFor(pieceNumber)],
    })
    assert.equal(added.pieces.length, 1, `Expected exactly one submitted piece at add ${pieceNumber}`)
    const confirmed = await SP.waitForAddPieces({ statusUrl: added.statusUrl, timeout: 180_000 })
    assert.equal(confirmed.piecesAdded, true, `Piece ${pieceNumber} was not added`)
    assert.equal(confirmed.confirmedPieceIds.length, 1, `Piece ${pieceNumber} confirmation was incomplete`)

    const current = await readAccountState(synapse)
    snapshots.push({
      pieceNumber,
      funds: current.funds,
      availableFunds: current.availableFunds,
      lockupCurrent: current.lockupCurrent,
      lockupRate: current.lockupRate,
    })
    pieceCids.push(added.pieces[0].pieceCid)
    console.log(
      `Added ${pieceNumber}: piece=${added.pieces[0].pieceCid} funds=${current.funds} ` +
        `available=${current.availableFunds} lockup=${current.lockupCurrent} rate=${current.lockupRate}`
    )

    if (pieceNumber >= REQUIRED_PIECES && current.lockupCurrent > previous.lockupCurrent) {
      replenishedAt = pieceNumber
      break
    }
    previous = current
  }

  assert.equal(snapshots.length >= REQUIRED_PIECES, true, `Expected at least ${REQUIRED_PIECES} additions`)
  assert(replenishedAt != null, `Lockup did not replenish within ${MAX_PIECES} additions`)
  console.log(`Lockup replenished after piece ${replenishedAt}`)

  const activePieceCount = await getActivePieceCount(synapse.client, { dataSetId: copy.dataSetId })
  assert.equal(
    activePieceCount,
    initialPieceCount + BigInt(snapshots.length),
    'On-chain active-piece count differs from successfully submitted pieces'
  )
  for (const pieceCid of pieceCids) {
    const ids = await readContract(
      synapse.client,
      findPieceIdsByCidCall({
        chain: synapse.client.chain,
        dataSetId: copy.dataSetId,
        pieceCid,
        startPieceId: 0n,
        limit: 2n,
      })
    )
    assert.equal(ids.length, 1, `Submitted piece ${pieceCid} is not discoverable on-chain`)
  }
  console.log(
    `=== SUCCESS: ${snapshots.length} distinct pieces are discoverable on data set ${copy.dataSetId}; ` +
      `lockup replenished at ${replenishedAt} ===`
  )
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
