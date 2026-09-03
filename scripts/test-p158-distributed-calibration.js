#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  canonicalExternalDispatchDigest,
  canonicalExternalRunnerReceiptDigest,
  DistributedCalibrationError,
  finalizeDistributedC01Calibration,
  prepareDistributedC01Calibration,
  startDistributedC01Calibration,
} from './lib/p158-distributed-calibration.js';
import { createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';

const START_MS = Date.parse('2026-09-02T19:40:00.000Z');
const END_MS = START_MS + 20 * 60_000;
const HANDOFF_SHA256 = '9a'.repeat(32);

function targets() {
  return ['E1', 'E2'].map((environmentId) => ({
    environmentId,
    scope: 'development',
    serviceUrl: environmentId === 'E1' ? 'http://127.0.0.1:19001' : 'https://service.dev.example.test',
    dashboardUrl: environmentId === 'E1' ? 'http://dashboard.dev.localhost' : 'https://dashboard.dev.example.test',
    handoffUrlSha256: HANDOFF_SHA256,
    profileRoot: `/tmp/agent-browser-development/${environmentId.toLowerCase()}/profile`,
  }));
}

function agents() {
  return Array.from({ length: 25 }, (_, index) => `distributed-agent-${String(index + 1).padStart(2, '0')}`);
}

function clients() {
  return [
    { clientId: 'external-runner-human', viewerId: 'external-viewer-human', paceProfile: 'human_controller' },
    { clientId: 'external-runner-slow', viewerId: 'external-viewer-slow', paceProfile: 'slow_concurrency' },
  ];
}

function dispatch() {
  const value = {
    schemaVersion: 'agent-browser.p158-external-calibration-dispatch.v1',
    planId: 'P158',
    runId: 'p158-live-c01',
    candidateCommit: 'ab'.repeat(20),
    calibrationStartAt: new Date(START_MS).toISOString(),
    calibrationEndAt: new Date(END_MS).toISOString(),
    durationMs: 20 * 60_000,
    lateToleranceMs: 30_000,
    actionCountPerClient: 25,
    reconnectCountPerClient: 5,
    scheduleSha256: 'pending',
  };
  const schedule = [];
  for (let ordinal = 1; ordinal <= 25; ordinal += 1) {
    const offsetMs = Math.floor((ordinal * value.durationMs) / 25);
    schedule.push({ kind: 'dashboard_action', ordinal, offsetMs });
    if (ordinal % 5 === 0) schedule.push({ kind: 'handoff_reconnect', ordinal: ordinal / 5, offsetMs });
  }
  const canonical = (entry) => Array.isArray(entry)
    ? entry.map(canonical)
    : entry && typeof entry === 'object'
      ? Object.fromEntries(Object.keys(entry).sort().map((key) => [key, canonical(entry[key])]))
      : entry;
  value.scheduleSha256 = sha256(Buffer.from(JSON.stringify(canonical(schedule))));
  value.descriptorSha256 = canonicalExternalDispatchDigest(value);
  return value;
}

function clockHarness(initial = START_MS - 60_000) {
  let now = initial;
  return {
    clock: {
      wallNow: () => new Date(now).toISOString(),
      monotonicNow: () => now * 1_000_000,
    },
    scheduler: { waitUntil: async ({ wallTime }) => { now = Math.max(now, Date.parse(wallTime)); } },
    advanceTo: (value) => { now = value; },
  };
}

function prepareInput(clock) {
  return {
    calibrationId: 'p158-live-c01-calibration',
    runId: 'p158-live-c01',
    sourceCommit: 'ab'.repeat(20),
    workflowRunId: '987654321',
    workflowRunAttempt: 1,
    developmentTargets: targets(),
    agentClientIds: agents(),
    externalClients: clients(),
    externalDispatchDescriptor: dispatch(),
    clock,
  };
}

function makeReceipts(prepared) {
  const receipts = clients().map((client, clientIndex) => ({
    schemaVersion: 'agent-browser.p158-external-calibration-receipt.v1',
    planId: 'P158',
    runId: prepared.runId,
    receiptId: `receipt-${client.clientId}`,
    clientId: client.clientId,
    viewerId: client.viewerId,
    paceProfile: client.paceProfile,
    mode: 'calibration',
    success: true,
    repairAttempted: false,
    retryCount: 0,
    startedAt: prepared.externalDispatchDescriptor.calibrationStartAt,
    completedAt: prepared.externalDispatchDescriptor.calibrationEndAt,
    sourceCommit: prepared.sourceCommit,
    workflowRunId: prepared.workflowRunId,
    workflowRunAttempt: prepared.workflowRunAttempt,
    runner: { runId: prepared.workflowRunId, runAttempt: String(prepared.workflowRunAttempt) },
    runnerIdentity: {
      provider: 'github_actions', runnerId: `github-runner-${clientIndex + 1}`,
      runnerName: `GitHub Actions ${clientIndex + 1}`, runnerOs: 'Linux', runnerArch: 'X64',
    },
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    publicEgressObserved: true,
    handoff: { urlSha256: HANDOFF_SHA256 },
    urlEvidence: [{ role: 'copied_action', url: 'https://example.test/remote-view/<redacted>' }],
    sensitiveCanary: 'must-not-enter-final-artifacts',
    calibration: { dispatchDescriptor: prepared.externalDispatchDescriptor },
    actions: [],
  }));
  for (const [kind, count] of [['dashboard_action', 50], ['handoff_reconnect', 10]]) {
    for (let ordinal = 1; ordinal <= count; ordinal += 1) {
      const receipt = receipts[(ordinal - 1) % 2];
      receipt.actions.push({
        kind, ordinal, viewerId: receipt.viewerId, attempt: 1, state: 'passed',
        observedAt: new Date(START_MS + Math.floor((ordinal * 20 * 60_000) / (count + 1))).toISOString(),
        latencyMs: ordinal, retryAttempted: false, repairAttempted: false,
      });
    }
  }
  for (const receipt of receipts) receipt.receiptSha256 = canonicalExternalRunnerReceiptDigest(receipt);
  return receipts;
}

function serviceHarness(time) {
  const calls = [];
  let forbiddenCalls = 0;
  return {
    calls,
    get forbiddenCalls() { return forbiddenCalls; },
    transport: {
      executeReadOnlyCommand: async (request) => {
        calls.push(structuredClone(request));
        return {
          state: 'passed', effectClass: 'read_only', latencyMs: request.ordinal % 13 + 1,
          observedAt: request.plannedAt, attempt: 1, retryAttempted: false, repairAttempted: false,
        };
      },
      startExecution: async () => { forbiddenCalls += 1; },
      mutateProduction: async () => { forbiddenCalls += 1; },
      retry: async () => { forbiddenCalls += 1; },
      repair: async () => { forbiddenCalls += 1; },
    },
    scheduler: time.scheduler,
  };
}

async function runTest(name, body) {
  try {
    await body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

async function preparedLocalRun() {
  const time = clockHarness();
  const prepared = prepareDistributedC01Calibration(prepareInput(time.clock));
  const service = serviceHarness(time);
  const store = createMemoryArtifactStore();
  const localRun = await startDistributedC01Calibration({
    prepared, serviceTransport: service.transport, scheduler: service.scheduler,
    artifactStore: store, clock: time.clock,
  });
  return { time, prepared, service, store, localRun };
}

await runTest('prepares before the window and persists exactly 500 local observations', async () => {
  const { prepared, service, store, localRun } = await preparedLocalRun();
  assert.equal(prepared.state, 'prepared');
  assert.equal(service.calls.length, 500);
  assert.equal(new Set(service.calls.map((call) => call.clientId)).size, 25);
  assert.deepEqual(service.calls.map((call) => call.ordinal), Array.from({ length: 500 }, (_, index) => index + 1));
  assert.equal(service.calls[0].plannedAt, new Date(START_MS).toISOString());
  assert.ok(Date.parse(service.calls.at(-1).plannedAt) > END_MS - 3_000);
  assert.ok(service.calls.every((call) => call.target.scope === 'development' && call.effectClass === 'read_only'));
  assert.equal(service.forbiddenCalls, 0);
  assert.deepEqual(store.paths(), [localRun.localObservationArtifact.relativePath]);
  const persisted = await store.read(localRun.localObservationArtifact.relativePath);
  assert.equal(sha256(persisted), localRun.localObservationArtifact.declaredSha256);
  assert.equal(JSON.parse(persisted).observations.length, 500);
});

await runTest('finalizes after late receipt availability without replay or time travel', async () => {
  const context = await preparedLocalRun();
  const callsBeforeFinalize = context.service.calls.length;
  context.time.advanceTo(END_MS + 6 * 60 * 60_000);
  const receipts = makeReceipts(context.prepared);
  const result = finalizeDistributedC01Calibration({
    prepared: context.prepared, localRun: context.localRun,
    externalRunnerReceipts: receipts, clock: context.time.clock,
  });
  assert.equal(context.service.calls.length, callsBeforeFinalize);
  assert.equal(context.service.forbiddenCalls, 0);
  assert.equal(result.calibration.clean, true);
  assert.equal(result.observations.length, 560);
  assert.equal(result.distributedEvidence.serviceCommandCount, 500);
  assert.equal(result.distributedEvidence.dashboardActionCount, 50);
  assert.equal(result.distributedEvidence.handoffReconnectCount, 10);
  assert.equal(result.distributedEvidence.externalReplayEffectCount, 0);
  assert.equal(result.distributedEvidence.finalizedAt, new Date(END_MS + 6 * 60 * 60_000).toISOString());
  assert.ok(result.artifacts.every((artifact) => artifact.declaredSha256 === sha256(artifact.content)));
  assert.doesNotMatch(JSON.stringify(result), /\/remote-view\//u);
  assert.doesNotMatch(JSON.stringify(result), /must-not-enter-final-artifacts/u);
  assert.ok(result.observations.slice(500).every((entry) =>
    entry.result.source === 'external_runner_receipt' && entry.result.performedLocally === false));
});

await runTest('rejects a late local start before any Service command', async () => {
  const time = clockHarness();
  const prepared = prepareDistributedC01Calibration(prepareInput(time.clock));
  time.advanceTo(START_MS + 1);
  const service = serviceHarness(time);
  await assert.rejects(
    () => startDistributedC01Calibration({
      prepared, serviceTransport: service.transport, scheduler: service.scheduler,
      artifactStore: createMemoryArtifactStore(), clock: time.clock,
    }),
    (error) => error instanceof DistributedCalibrationError && error.code === 'late_local_start',
  );
  assert.deepEqual(service.calls, []);
});

await runTest('does not finalize before external receipts can exist', async () => {
  const context = await preparedLocalRun();
  context.time.advanceTo(END_MS - 1);
  assert.throws(
    () => finalizeDistributedC01Calibration({
      prepared: context.prepared, localRun: context.localRun,
      externalRunnerReceipts: makeReceipts(context.prepared), clock: context.time.clock,
    }),
    (error) => error instanceof DistributedCalibrationError && error.code === 'early_finalization',
  );
  assert.equal(context.service.calls.length, 500);
  assert.equal(context.service.forbiddenCalls, 0);
});

await runTest('preserves a Service retry claim as one failure and continues without retrying', async () => {
  const time = clockHarness();
  const prepared = prepareDistributedC01Calibration(prepareInput(time.clock));
  const service = serviceHarness(time);
  const original = service.transport.executeReadOnlyCommand;
  service.transport.executeReadOnlyCommand = async (request) => {
    const response = await original(request);
    return request.ordinal === 9 ? { ...response, retryAttempted: true } : response;
  };
  const localRun = await startDistributedC01Calibration({
    prepared, serviceTransport: service.transport, scheduler: service.scheduler,
    artifactStore: createMemoryArtifactStore(), clock: time.clock,
  });
  assert.equal(service.calls.length, 500);
  assert.equal(service.calls.filter((call) => call.ordinal === 9).length, 1);
  const observations = JSON.parse(localRun.localObservationArtifact.content).observations;
  assert.equal(observations[8].state, 'failed');
  assert.equal(observations[8].failure.code, 'service_effect_contract_mismatch');
  assert.equal(service.forbiddenCalls, 0);
});

await runTest('rejects external evidence defects only during effect-free finalization', async () => {
  const mutations = [
    ['external_receipt_hash_mismatch', false, (receipts) => { receipts[0].receiptSha256 = '00'.repeat(32); }],
    ['external_receipt_binding_mismatch', true, (receipts) => { receipts[0].handoff.urlSha256 = '11'.repeat(32); }],
    ['external_receipt_identity_mismatch', true, (receipts) => {
      receipts[1].runnerIdentity.runnerId = receipts[0].runnerIdentity.runnerId;
    }],
    ['external_action_count_mismatch', true, (receipts) => { receipts[0].actions.pop(); }],
    ['external_action_contract_mismatch', true, (receipts) => { receipts[0].actions[0].retryAttempted = true; }],
    ['external_action_contract_mismatch', true, (receipts) => { receipts[0].actions[0].repairAttempted = true; }],
  ];
  for (const [code, rehash, mutate] of mutations) {
    const context = await preparedLocalRun();
    context.time.advanceTo(END_MS + 1);
    const receipts = makeReceipts(context.prepared);
    mutate(receipts);
    if (rehash) {
      for (const receipt of receipts) receipt.receiptSha256 = canonicalExternalRunnerReceiptDigest(receipt);
    }
    const calls = context.service.calls.length;
    assert.throws(
      () => finalizeDistributedC01Calibration({
        prepared: context.prepared, localRun: context.localRun,
        externalRunnerReceipts: receipts, clock: context.time.clock,
      }),
      (error) => error instanceof DistributedCalibrationError && error.code === code,
    );
    assert.equal(context.service.calls.length, calls);
    assert.equal(context.service.forbiddenCalls, 0);
  }
});

await runTest('rejects changed preparation or persisted local observations', async () => {
  const context = await preparedLocalRun();
  const changedPrepared = structuredClone(context.prepared);
  changedPrepared.agentClientIds[0] = 'changed-client';
  await assert.rejects(
    () => startDistributedC01Calibration({
      prepared: changedPrepared, serviceTransport: context.service.transport,
      scheduler: context.service.scheduler, artifactStore: createMemoryArtifactStore(), clock: context.time.clock,
    }),
    (error) => error.code === 'prepared_descriptor_mismatch',
  );
  context.time.advanceTo(END_MS + 1);
  const changedLocal = structuredClone(context.localRun);
  changedLocal.localObservationArtifact.content += ' ';
  assert.throws(
    () => finalizeDistributedC01Calibration({
      prepared: context.prepared, localRun: changedLocal,
      externalRunnerReceipts: makeReceipts(context.prepared), clock: context.time.clock,
    }),
    (error) => error.code === 'local_observation_integrity_mismatch',
  );
});

await runTest('rejects raw handoff custody and is deterministic with identical evidence', async () => {
  const time = clockHarness();
  const invalid = prepareInput(time.clock);
  invalid.developmentTargets[1].handoffUrl = 'https://example.test/remote-view/secret';
  assert.throws(() => prepareDistributedC01Calibration(invalid), /only handoffUrlSha256/u);
  const first = await preparedLocalRun();
  const second = await preparedLocalRun();
  first.time.advanceTo(END_MS + 1);
  second.time.advanceTo(END_MS + 1);
  assert.deepEqual(
    finalizeDistributedC01Calibration({
      prepared: first.prepared, localRun: first.localRun,
      externalRunnerReceipts: makeReceipts(first.prepared), clock: first.time.clock,
    }),
    finalizeDistributedC01Calibration({
      prepared: second.prepared, localRun: second.localRun,
      externalRunnerReceipts: makeReceipts(second.prepared), clock: second.time.clock,
    }),
  );
});

process.stdout.write('P158 distributed C01 calibration phase test passed\n');
