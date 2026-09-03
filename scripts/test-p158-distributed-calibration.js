#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  canonicalExternalRunnerReceiptDigest,
  DistributedCalibrationError,
  runDistributedC01Calibration,
} from './lib/p158-distributed-calibration.js';
import { sha256 } from './lib/p158-campaign-controller.js';

const START_MS = Date.parse('2026-09-02T19:40:00.000Z');
const END_MS = START_MS + 20 * 60_000;

function developmentTargets() {
  return [
    {
      environmentId: 'E1', scope: 'development',
      serviceUrl: 'http://127.0.0.1:19001', dashboardUrl: 'http://dashboard.dev.localhost',
      handoffUrl: 'https://handoff.dev.example.test/remote-view/e1',
      profileRoot: '/tmp/agent-browser-development/e1/profile',
    },
    {
      environmentId: 'E2', scope: 'development',
      serviceUrl: 'https://service.dev.example.test', dashboardUrl: 'https://dashboard.dev.example.test',
      handoffUrl: 'https://handoff.dev.example.test/remote-view/e2',
      profileRoot: '/tmp/agent-browser-development/e2/profile',
    },
  ];
}

function agentClientIds() {
  return Array.from({ length: 25 }, (_, index) => `distributed-agent-${String(index + 1).padStart(2, '0')}`);
}

function makeReceipts() {
  const receipts = [1, 2].map((number) => ({
    receiptId: `github-external-receipt-${number}`,
    schemaVersion: 'agent-browser.p158-external-calibration-receipt.v1',
    runId: 'p158-live-c01',
    sourceCommit: 'ab'.repeat(20),
    workflowRunId: '987654321',
    workflowRunAttempt: 1,
    runnerIdentity: {
      provider: 'github_actions',
      runnerId: `github-runner-${number}`,
      runnerName: `external-linux-${number}`,
      runnerOs: 'Linux',
      runnerArch: 'X64',
    },
    viewerId: `external-viewer-${number}`,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    publicEgressObserved: true,
    handoffUrl: 'https://handoff.dev.example.test/remote-view/e2',
    startedAt: new Date(START_MS).toISOString(),
    completedAt: new Date(END_MS).toISOString(),
    actions: [],
  }));
  for (const [kind, count] of [['dashboard_action', 50], ['handoff_reconnect', 10]]) {
    for (let ordinal = 1; ordinal <= count; ordinal += 1) {
      const receipt = receipts[(ordinal - 1) % receipts.length];
      receipt.actions.push({
        kind,
        ordinal,
        viewerId: receipt.viewerId,
        attempt: 1,
        state: 'passed',
        observedAt: new Date(START_MS + Math.floor((ordinal * (END_MS - START_MS)) / (count + 1))).toISOString(),
        latencyMs: ordinal + (kind === 'handoff_reconnect' ? 100 : 0),
        retryAttempted: false,
        repairAttempted: false,
      });
    }
  }
  for (const receipt of receipts) receipt.receiptSha256 = canonicalExternalRunnerReceiptDigest(receipt);
  return receipts;
}

function harness() {
  let now = START_MS;
  const calls = [];
  let startExecutionCalls = 0;
  let mutationCalls = 0;
  return {
    calls,
    get startExecutionCalls() { return startExecutionCalls; },
    get mutationCalls() { return mutationCalls; },
    clock: {
      wallNow: () => new Date(now).toISOString(),
      monotonicNow: () => now * 1_000_000,
    },
    scheduler: {
      waitUntil: async ({ wallTime }) => { now = Math.max(now, Date.parse(wallTime)); },
    },
    serviceTransport: {
      executeReadOnlyCommand: async (request) => {
        calls.push(structuredClone(request));
        now += 1_000;
        return {
          state: 'passed',
          effectClass: request.effectClass,
          latencyMs: request.actionOrdinal % 13 + 1,
          observedAt: new Date(now).toISOString(),
          attempt: 1,
          retryAttempted: false,
          repairAttempted: false,
        };
      },
      startExecution: async () => { startExecutionCalls += 1; },
      mutateProduction: async () => { mutationCalls += 1; },
    },
  };
}

function input(overrides = {}) {
  const injected = harness();
  return {
    injected,
    value: {
      calibrationId: 'p158-live-c01-calibration',
      runId: 'p158-live-c01',
      sourceCommit: 'ab'.repeat(20),
      workflowRunId: '987654321',
      workflowRunAttempt: 1,
      developmentTargets: developmentTargets(),
      agentClientIds: agentClientIds(),
      externalRunnerReceipts: makeReceipts(),
      serviceTransport: injected.serviceTransport,
      scheduler: injected.scheduler,
      clock: injected.clock,
      ...overrides,
    },
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

await runTest('merges 500 harmless Service commands with exact external receipt evidence', async () => {
  const { value, injected } = input();
  const before = structuredClone({
    developmentTargets: value.developmentTargets,
    agentClientIds: value.agentClientIds,
    externalRunnerReceipts: value.externalRunnerReceipts,
  });
  const result = await runDistributedC01Calibration(value);
  assert.equal(injected.calls.length, 500);
  assert.deepEqual(injected.calls.map((call) => call.ordinal), Array.from({ length: 500 }, (_, index) => index + 1));
  assert.equal(new Set(injected.calls.map((call) => call.clientId)).size, 25);
  assert.equal(injected.calls[0].plannedAt, new Date(START_MS).toISOString());
  assert.ok(Date.parse(injected.calls.at(-1).plannedAt) > END_MS - 3_000);
  assert.ok(injected.calls.every((call) => call.effectClass === 'read_only' || call.effectClass === 'harmless'));
  assert.ok(injected.calls.every((call) => call.target.scope === 'development'));
  assert.equal(injected.startExecutionCalls, 0);
  assert.equal(injected.mutationCalls, 0);
  assert.equal(result.calibration.clean, true);
  assert.equal(result.observations.length, 560);
  assert.equal(result.distributedEvidence.serviceCommandCount, 500);
  assert.equal(result.distributedEvidence.dashboardActionCount, 50);
  assert.equal(result.distributedEvidence.handoffReconnectCount, 10);
  assert.equal(result.distributedEvidence.externalReplayEffectCount, 0);
  assert.equal(result.distributedEvidence.sharedWindowDurationMs, 20 * 60_000);
  assert.equal(result.artifacts.length, 3);
  assert.ok(result.artifacts.every((artifact) => artifact.declaredSha256 === sha256(artifact.content)));
  const external = result.observations.slice(500);
  assert.ok(external.every((entry) => entry.result.source === 'external_runner_receipt'));
  assert.ok(external.every((entry) => entry.result.performedLocally === false));
  const receiptActions = new Map(value.externalRunnerReceipts.flatMap((receipt) =>
    receipt.actions.map((action) => [`${action.kind}:${action.ordinal}`, action])));
  assert.ok(external.every((entry) =>
    entry.result.observedAt === receiptActions.get(`${entry.kind}:${entry.actionOrdinal}`).observedAt));
  const raw = JSON.parse(result.artifacts[0].content);
  assert.deepEqual(
    raw.externalViewerReceipts.map((receipt) => receipt.receiptSha256),
    value.externalRunnerReceipts.map((receipt) => receipt.receiptSha256),
  );
  assert.deepEqual({
    developmentTargets: value.developmentTargets,
    agentClientIds: value.agentClientIds,
    externalRunnerReceipts: value.externalRunnerReceipts,
  }, before);
});

await runTest('fails closed before Service effects for receipt integrity and binding defects', async () => {
  const mutations = [
    ['external_receipt_hash_mismatch', false, (receipts) => { receipts[0].receiptSha256 = '00'.repeat(32); }],
    ['external_receipt_binding_mismatch', true, (receipts) => { receipts[1].workflowRunId = 'different-run'; }],
    ['external_receipt_binding_mismatch', true, (receipts) => { receipts[1].sourceCommit = 'cd'.repeat(20); }],
    ['external_receipt_identity_mismatch', true, (receipts) => {
      receipts[1].runnerIdentity.runnerId = receipts[0].runnerIdentity.runnerId;
    }],
    ['external_receipt_identity_mismatch', true, (receipts) => { receipts[1].viewerId = receipts[0].viewerId; }],
    ['external_receipt_window_mismatch', true, (receipts) => {
      receipts[0].completedAt = new Date(END_MS - 1).toISOString();
    }],
    ['external_action_count_mismatch', true, (receipts) => { receipts[0].actions.pop(); }],
    ['external_action_count_mismatch', true, (receipts) => {
      receipts[0].actions[0].ordinal = receipts[1].actions[0].ordinal;
    }],
    ['external_action_contract_mismatch', true, (receipts) => { receipts[0].actions[0].attempt = 2; }],
    ['external_action_contract_mismatch', true, (receipts) => { receipts[0].actions[0].retryAttempted = true; }],
    ['external_action_contract_mismatch', true, (receipts) => { receipts[0].actions[0].repairAttempted = true; }],
    ['external_action_contract_mismatch', true, (receipts) => {
      receipts[0].actions[0].observedAt = new Date(END_MS + 1).toISOString();
    }],
    ['external_receipt_identity_mismatch', true, (receipts) => {
      receipts[0].handoffUrl = 'http://127.0.0.1:9999/raw-guacamole';
    }],
  ];
  for (const [expectedCode, rehash, mutate] of mutations) {
    const receipts = makeReceipts();
    mutate(receipts);
    if (rehash) {
      for (const receipt of receipts) receipt.receiptSha256 = canonicalExternalRunnerReceiptDigest(receipt);
    }
    const { value, injected } = input({ externalRunnerReceipts: receipts });
    await assert.rejects(
      () => runDistributedC01Calibration(value),
      (error) => error instanceof DistributedCalibrationError && error.code === expectedCode,
    );
    assert.deepEqual(injected.calls, []);
    assert.equal(injected.startExecutionCalls, 0);
    assert.equal(injected.mutationCalls, 0);
  }
});

await runTest('preserves external terminal failure without replaying it or retrying Service work', async () => {
  const receipts = makeReceipts();
  const failed = receipts[0].actions.find((action) => action.kind === 'dashboard_action');
  failed.state = 'failed';
  failed.failure = { code: 'external_click_failed', message: 'external first failure' };
  receipts[0].receiptSha256 = canonicalExternalRunnerReceiptDigest(receipts[0]);
  const { value, injected } = input({ externalRunnerReceipts: receipts });
  const result = await runDistributedC01Calibration(value);
  assert.equal(injected.calls.length, 500);
  assert.equal(result.calibration.clean, false);
  const observation = result.observations.find(
    (entry) => entry.kind === 'dashboard_action' && entry.actionOrdinal === failed.ordinal,
  );
  assert.equal(observation.state, 'failed');
  assert.equal(observation.failure.code, 'external_click_failed');
  assert.equal(injected.startExecutionCalls, 0);
});

await runTest('records a non-read-only Service response as a first failure and continues once', async () => {
  const { value, injected } = input();
  const original = value.serviceTransport.executeReadOnlyCommand;
  value.serviceTransport.executeReadOnlyCommand = async (request) => {
    const response = await original(request);
    return request.ordinal === 9 ? { ...response, effectClass: 'mutation' } : response;
  };
  const result = await runDistributedC01Calibration(value);
  assert.equal(injected.calls.length, 500);
  assert.equal(result.calibration.clean, false);
  assert.equal(result.observations[8].state, 'failed');
  assert.equal(result.observations[8].failure.code, 'service_effect_contract_mismatch');
  assert.equal(injected.calls.filter((call) => call.ordinal === 9).length, 1);
  assert.equal(injected.startExecutionCalls, 0);
});

await runTest('records Service retry and repair claims as contract failures without another attempt', async () => {
  for (const defect of [{ retryAttempted: true }, { repairAttempted: true }]) {
    const { value, injected } = input();
    const original = value.serviceTransport.executeReadOnlyCommand;
    value.serviceTransport.executeReadOnlyCommand = async (request) => {
      const response = await original(request);
      return request.ordinal === 11 ? { ...response, ...defect } : response;
    };
    const result = await runDistributedC01Calibration(value);
    assert.equal(injected.calls.length, 500);
    assert.equal(result.calibration.clean, false);
    assert.equal(result.observations[10].failure.code, 'service_effect_contract_mismatch');
    assert.equal(injected.calls.filter((call) => call.ordinal === 11).length, 1);
    assert.equal(injected.startExecutionCalls, 0);
  }
});

await runTest('rejects a late Service start before issuing any command', async () => {
  const { value, injected } = input();
  value.clock = {
    wallNow: () => new Date(START_MS + 1).toISOString(),
    monotonicNow: () => (START_MS + 1) * 1_000_000,
  };
  await assert.rejects(
    () => runDistributedC01Calibration(value),
    (error) => error instanceof DistributedCalibrationError && error.code === 'shared_window_timing_mismatch',
  );
  assert.deepEqual(injected.calls, []);
});

await runTest('is deterministic under identical injected receipts, clocks, and transports', async () => {
  const first = input();
  const second = input();
  assert.deepEqual(
    await runDistributedC01Calibration(first.value),
    await runDistributedC01Calibration(second.value),
  );
});

process.stdout.write('P158 distributed C01 calibration self-test passed\n');
