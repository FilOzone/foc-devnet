import assert from 'node:assert/strict'
import * as SP from '@filoz/synapse-core/sp'
import { getRail } from '@filoz/synapse-core/pay'
import { getActivePieceCount } from '@filoz/synapse-core/pdp-verifier'
import { getPdpDataSet } from '@filoz/synapse-core/warm-storage'
import { createSynapse, prepareAccount, readAccountState } from './account.ts'
import { freshMetadata, resolveDistinctDevnetEnvironment, resolveEnvironment } from './environment.ts'
import { assertDeletionRejected, assertTerminationRejected } from './permission-checks.ts'
import { fileSize, uploadFile } from './storage.ts'

type DataSetSnapshot = {
  dataSetId: bigint
  pieceId: bigint
  clientDataSetId: bigint
  serviceURL: string
  live: boolean
  activePieceCount: bigint
  rail: { railId: bigint; endEpoch: bigint; paymentRate: bigint; lockupPeriod: bigint }
  payment: { funds: bigint; lockupRate: bigint }
}

async function snapshot(
  synapse: ReturnType<typeof createSynapse>,
  dataSetId: bigint,
  pieceId: bigint
): Promise<DataSetSnapshot> {
  const dataSet = await getPdpDataSet(synapse.client, { dataSetId })
  if (dataSet == null) throw new Error(`Data set ${dataSetId} is not readable`)
  const [activePieceCount, rail, payment] = await Promise.all([
    getActivePieceCount(synapse.client, { dataSetId }),
    getRail(synapse.client, { railId: dataSet.pdpRailId }),
    readAccountState(synapse),
  ])
  return {
    dataSetId,
    pieceId,
    clientDataSetId: dataSet.clientDataSetId,
    serviceURL: dataSet.provider.pdp.serviceURL,
    live: dataSet.live,
    activePieceCount,
    rail: {
      railId: dataSet.pdpRailId,
      endEpoch: rail.endEpoch,
      paymentRate: rail.paymentRate,
      lockupPeriod: rail.lockupPeriod,
    },
    payment: {
      funds: payment.funds,
      lockupRate: payment.lockupRate,
    },
  }
}

async function createPermissionTarget(
  owner: ReturnType<typeof createSynapse>,
  filePath: string,
  label: string
): Promise<DataSetSnapshot> {
  const { result } = await uploadFile(owner, filePath, freshMetadata(`negative-permissions-${label}`), 1)
  const copy = result.copies[0]
  assert(copy != null, `Expected one live data set for ${label}`)
  return snapshot(owner, copy.dataSetId, copy.pieceId)
}

function assertStableAfterRejectedRequest(before: DataSetSnapshot, after: DataSetSnapshot, label: string): void {
  assert.equal(after.live, true, `${label} mutated data set liveness`)
  assert.equal(after.activePieceCount, before.activePieceCount, `${label} mutated active piece count`)
  assert.deepEqual(after.rail, before.rail, `${label} mutated rail identity, rate, lockup period, or end epoch`)
  assert.deepEqual(after.payment, before.payment, `${label} mutated payment funds or lockup rate`)
}

async function main(): Promise<void> {
  const ownerEnvironment = resolveEnvironment({ defaultUserIndex: 0, requireFiles: true })
  if (ownerEnvironment.filePaths.length !== 1) throw new Error('negative-permissions.ts accepts exactly one file path')
  const owner = createSynapse(ownerEnvironment)
  const intruder = createSynapse(resolveDistinctDevnetEnvironment(ownerEnvironment))
  assert.notEqual(
    intruder.client.account.address.toLowerCase(),
    owner.client.account.address.toLowerCase(),
    'Permission checks require distinct owner and intruder signing addresses'
  )
  const [filePath] = ownerEnvironment.filePaths

  await prepareAccount(owner, (await fileSize(filePath)) * 4n)

  const nonOwnerTerminate = await createPermissionTarget(owner, filePath, 'non-owner-terminate')
  await assertTerminationRejected(
    owner,
    () =>
      SP.terminateService(intruder.client, {
        serviceURL: nonOwnerTerminate.serviceURL,
        dataSetId: nonOwnerTerminate.dataSetId,
      }),
    'non-owner',
    'non-owner relayed termination'
  )
  assertStableAfterRejectedRequest(
    nonOwnerTerminate,
    await snapshot(owner, nonOwnerTerminate.dataSetId, nonOwnerTerminate.pieceId),
    'non-owner relayed termination'
  )

  const malformedTerminate = await createPermissionTarget(owner, filePath, 'malformed-terminate')
  await assertTerminationRejected(
    owner,
    () =>
      SP.terminateServiceApiRequest({
        serviceURL: malformedTerminate.serviceURL,
        dataSetId: malformedTerminate.dataSetId,
        extraData: '0x',
      }),
    'malformed',
    'malformed relayed termination data'
  )
  assertStableAfterRejectedRequest(
    malformedTerminate,
    await snapshot(owner, malformedTerminate.dataSetId, malformedTerminate.pieceId),
    'malformed relayed termination data'
  )

  const nonOwnerDeletion = await createPermissionTarget(owner, filePath, 'non-owner-deletion')
  await assertDeletionRejected(
    owner,
    () =>
      SP.schedulePieceDeletion(intruder.client, {
        serviceURL: nonOwnerDeletion.serviceURL,
        dataSetId: nonOwnerDeletion.dataSetId,
        clientDataSetId: nonOwnerDeletion.clientDataSetId,
        pieceId: nonOwnerDeletion.pieceId,
      }),
    'non-owner',
    'non-owner piece deletion'
  )
  assertStableAfterRejectedRequest(
    nonOwnerDeletion,
    await snapshot(owner, nonOwnerDeletion.dataSetId, nonOwnerDeletion.pieceId),
    'non-owner piece deletion'
  )

  const malformedDeletion = await createPermissionTarget(owner, filePath, 'malformed-deletion')
  await assertDeletionRejected(
    owner,
    () =>
      SP.deletePiece({
        serviceURL: malformedDeletion.serviceURL,
        dataSetId: malformedDeletion.dataSetId,
        pieceId: malformedDeletion.pieceId,
        extraData: '0x',
      }),
    'malformed',
    'malformed piece deletion data'
  )
  assertStableAfterRejectedRequest(
    malformedDeletion,
    await snapshot(owner, malformedDeletion.dataSetId, malformedDeletion.pieceId),
    'malformed piece deletion data'
  )

  console.log('=== SUCCESS: rejected permission requests left independent data sets unchanged ===')
}

main().catch((error: unknown) => {
  console.error(error)
  process.exitCode = 1
})
