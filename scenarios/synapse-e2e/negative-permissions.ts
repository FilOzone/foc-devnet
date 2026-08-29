import assert from 'node:assert/strict'
import * as SP from '@filoz/synapse-core/sp'
import { getRail } from '@filoz/synapse-core/pay'
import { getActivePieceCount } from '@filoz/synapse-core/pdp-verifier'
import { getPdpDataSet } from '@filoz/synapse-core/warm-storage'
import { createSynapse, prepareAccount, readAccountState } from './account.ts'
import { freshMetadata, resolveEnvironment } from './environment.ts'
import { fileSize, uploadFile } from './storage.ts'

type DataSetSnapshot = {
  live: boolean
  activePieceCount: bigint
  rail: { endEpoch: bigint; paymentRate: bigint; lockupPeriod: bigint }
  payment: { funds: bigint; availableFunds: bigint; lockupCurrent: bigint; lockupRate: bigint }
}

async function snapshot(synapse: ReturnType<typeof createSynapse>, dataSetId: bigint): Promise<DataSetSnapshot> {
  const dataSet = await getPdpDataSet(synapse.client, { dataSetId })
  assert(dataSet != null, `Data set ${dataSetId} is not readable`)
  const [activePieceCount, rail, payment] = await Promise.all([
    getActivePieceCount(synapse.client, { dataSetId }),
    getRail(synapse.client, { railId: dataSet.pdpRailId }),
    readAccountState(synapse),
  ])
  return {
    live: dataSet.live,
    activePieceCount,
    rail: { endEpoch: rail.endEpoch, paymentRate: rail.paymentRate, lockupPeriod: rail.lockupPeriod },
    payment: {
      funds: payment.funds,
      availableFunds: payment.availableFunds,
      lockupCurrent: payment.lockupCurrent,
      lockupRate: payment.lockupRate,
    },
  }
}

async function main(): Promise<void> {
  const ownerEnvironment = resolveEnvironment({ defaultUserIndex: 0, requireFiles: true })
  if (ownerEnvironment.filePaths.length !== 1) throw new Error('negative-permissions.ts accepts exactly one file path')
  const owner = createSynapse(ownerEnvironment)
  const intruder = createSynapse(resolveEnvironment({ defaultUserIndex: 1 }))
  const [filePath] = ownerEnvironment.filePaths

  await prepareAccount(owner, await fileSize(filePath))
  const { result } = await uploadFile(owner, filePath, freshMetadata('negative-permissions'), 1)
  const copy = result.copies[0]
  assert(copy != null, 'Expected one live data set for permission checks')
  const dataSet = await getPdpDataSet(owner.client, { dataSetId: copy.dataSetId })
  assert(dataSet != null && dataSet.live, `Data set ${copy.dataSetId} was not created live`)
  const serviceURL = dataSet.provider.pdp.serviceURL
  const before = await snapshot(owner, copy.dataSetId)

  await assert.rejects(
    SP.terminateService(intruder.client, { serviceURL, dataSetId: copy.dataSetId }),
    'a non-owner must not be able to relay termination'
  )
  await assert.rejects(
    SP.terminateServiceApiRequest({ serviceURL, dataSetId: copy.dataSetId, extraData: '0x' }),
    'malformed relayed termination data must be rejected'
  )
  await assert.rejects(
    SP.schedulePieceDeletion(intruder.client, {
      serviceURL,
      dataSetId: copy.dataSetId,
      clientDataSetId: dataSet.clientDataSetId,
      pieceId: copy.pieceId,
    }),
    'a non-owner must not be able to schedule removal'
  )
  await assert.rejects(
    SP.deletePiece({ serviceURL, dataSetId: copy.dataSetId, pieceId: copy.pieceId, extraData: '0x' }),
    'malformed removal data must be rejected'
  )

  const after = await snapshot(owner, copy.dataSetId)
  assert.deepEqual(after, before, 'rejected requests must not mutate the data set, pieces, rail, or payment account')
  console.log(`=== SUCCESS: rejected requests left data set ${copy.dataSetId} unchanged ===`)
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
