import assert from 'node:assert/strict'
import { getRail } from '@filoz/synapse-core/pay'
import { findPieceIdsByCidCall, getActivePieceCount } from '@filoz/synapse-core/pdp-verifier'
import { getPdpDataSet } from '@filoz/synapse-core/warm-storage'
import type { UploadResult } from '@filoz/synapse-sdk'
import { readContract } from 'viem/actions'
import type { AccountState, ScenarioSynapse } from './account.ts'

const delay = (milliseconds: number) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds))

async function waitForDataSet(synapse: ScenarioSynapse, dataSetId: bigint) {
  for (let attempt = 1; attempt <= 15; attempt++) {
    const dataSet = await getPdpDataSet(synapse.client, { dataSetId })
    if (dataSet != null) return dataSet
    await delay(2000)
  }
  throw new Error(`Data set ${dataSetId} was not observable through the public PDP API`)
}

export async function assertOnchainState(
  synapse: ScenarioSynapse,
  result: UploadResult,
  metadata: Record<string, string>,
  beforeUpload: AccountState
): Promise<void> {
  const payment = await synapse.payments.accountInfo()
  assert(
    payment.funds <= beforeUpload.funds,
    `Payment funds increased unexpectedly during upload: ${beforeUpload.funds} -> ${payment.funds}`
  )
  assert(
    payment.lockupRate > beforeUpload.lockupRate,
    `Payment lockup rate did not increase: ${beforeUpload.lockupRate} -> ${payment.lockupRate}`
  )
  assert(
    payment.availableFunds < beforeUpload.availableFunds,
    `Available funds did not decrease: ${beforeUpload.availableFunds} -> ${payment.availableFunds}`
  )
  assert(payment.availableFunds > 0n, 'Payment account has no available funds after upload')
  console.log(
    `Payment state: funds=${payment.funds} available=${payment.availableFunds} lockup=${payment.lockupCurrent} rate=${payment.lockupRate}`
  )

  const dataSetIds = new Set(result.copies.map((copy) => copy.dataSetId))
  assert.equal(dataSetIds.size, result.copies.length, 'Copies must use distinct data sets')
  for (const copy of result.copies) {
    const dataSet = await waitForDataSet(synapse, copy.dataSetId)
    assert.equal(dataSet.live, true, `Data set ${copy.dataSetId} is not live`)
    assert.equal(dataSet.managed, true, `Data set ${copy.dataSetId} is not FWSS-managed`)
    const activePieceCount = await getActivePieceCount(synapse.client, {
      dataSetId: copy.dataSetId,
    })
    assert.equal(activePieceCount, 1n, `Data set ${copy.dataSetId} does not contain exactly one active piece`)
    const matchingPieceIds = await readContract(
      synapse.client,
      findPieceIdsByCidCall({
        chain: synapse.client.chain,
        dataSetId: copy.dataSetId,
        pieceCid: result.pieceCid,
        startPieceId: 0n,
        limit: 2n,
      })
    )
    assert.deepEqual(matchingPieceIds, [copy.pieceId], `Data set ${copy.dataSetId} has the wrong piece`)
    assert.equal(dataSet.providerId, copy.providerId, `Data set ${copy.dataSetId} has the wrong provider`)
    assert.equal(
      dataSet.payer.toLowerCase(),
      synapse.client.account.address.toLowerCase(),
      `Data set ${copy.dataSetId} has the wrong payer`
    )
    assert(dataSet.pdpRailId > 0n, `Data set ${copy.dataSetId} has no PDP payment rail`)
    const rail = await getRail(synapse.client, { railId: dataSet.pdpRailId })
    assert.equal(
      rail.from.toLowerCase(),
      synapse.client.account.address.toLowerCase(),
      `Rail ${dataSet.pdpRailId} has the wrong payer`
    )
    assert.equal(rail.to.toLowerCase(), dataSet.payee.toLowerCase(), `Rail ${dataSet.pdpRailId} has the wrong payee`)
    assert.equal(
      rail.operator.toLowerCase(),
      synapse.chain.contracts.fwss.address.toLowerCase(),
      `Rail ${dataSet.pdpRailId} has the wrong operator`
    )
    assert.equal(
      rail.validator.toLowerCase(),
      synapse.chain.contracts.fwss.address.toLowerCase(),
      `Rail ${dataSet.pdpRailId} has the wrong validator`
    )
    assert(rail.paymentRate > 0n, `Rail ${dataSet.pdpRailId} has no payment rate`)
    assert(rail.lockupPeriod > 0n, `Rail ${dataSet.pdpRailId} has no lockup period`)
    assert.equal(rail.endEpoch, 0n, `Rail ${dataSet.pdpRailId} is terminated`)
    for (const [key, value] of Object.entries(metadata)) {
      assert.equal(dataSet.metadata?.[key], value, `Data set ${copy.dataSetId} metadata ${key} differs`)
    }
    console.log(
      `On-chain data set verified: id=${copy.dataSetId} provider=${copy.providerId} piece=${copy.pieceId} rail=${dataSet.pdpRailId}`
    )
  }
}

export async function assertCreatedDataSet(
  synapse: ScenarioSynapse,
  dataSetId: bigint,
  providerId: bigint,
  expectedPayee: string,
  metadata: Record<string, string>
): Promise<void> {
  const dataSet = await waitForDataSet(synapse, dataSetId)
  assert.equal(dataSet.live, true, `Data set ${dataSetId} is not live`)
  assert.equal(dataSet.managed, true, `Data set ${dataSetId} is not FWSS-managed`)
  assert.equal(dataSet.providerId, providerId, `Data set ${dataSetId} has the wrong provider`)
  assert.equal(
    dataSet.payer.toLowerCase(),
    synapse.client.account.address.toLowerCase(),
    `Data set ${dataSetId} has the wrong payer`
  )
  assert.equal(dataSet.payee.toLowerCase(), expectedPayee.toLowerCase(), `Data set ${dataSetId} has the wrong payee`)
  for (const [key, value] of Object.entries(metadata)) {
    assert.equal(dataSet.metadata?.[key], value, `Data set ${dataSetId} metadata ${key} differs`)
  }
}
