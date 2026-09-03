#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import {
  C01_WORKLOAD,
  CalibrationError,
  runC01Calibration,
} from './lib/p158-calibration-runner.js';
import { canonicalJson, sha256 } from './lib/p158-campaign-controller.js';
import { canonicalCalibrationDigest } from './lib/p158-campaign-preparation.js';

const START_MS = Date.parse('2026-09-02T19:40:00.000Z');
const root = new URL('..', import.meta.url).pathname;
const preparationFixtureSchema = JSON.parse(readFileSync(
  `${root}/docs/dev/contracts/p158-campaign-preparation-fixtures.v1.schema.json`,
  'utf8',
));
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validateCalibration = ajv.compile({
  $ref: `${preparationFixtureSchema.$id}#/$defs/calibration`,
  $defs: preparationFixtureSchema.$defs,
  $id: preparationFixtureSchema.$id,
});
const validateArtifact = ajv.compile({
  $ref: `${preparationFixtureSchema.$id}-artifact#/$defs/artifact`,
  $defs: preparationFixtureSchema.$defs,
  $id: `${preparationFixtureSchema.$id}-artifact`,
});

function targets() {
  return [
    {
      environmentId: 'E1',
      scope: 'development',
      serviceUrl: 'http://127.0.0.1:19001',
      dashboardUrl: 'http://dashboard.dev.localhost',
      handoffUrl: 'https://handoff.dev.example.test/remote-view/e1',
      profileRoot: '/tmp/agent-browser-development/e1/profile',
    },
    {
      environmentId: 'E2',
      scope: 'development',
      serviceUrl: 'https://service.dev.example.test',
      dashboardUrl: 'https://dashboard.dev.example.test',
      handoffUrl: 'https://handoff.dev.example.test/remote-view/e2',
      profileRoot: '/tmp/agent-browser-development/e2/profile',
    },
  ];
}

function viewers() {
  return [1, 2].map((ordinal) => ({
    viewerId: `external-viewer-${ordinal}`,
    receiptId: `external-receipt-${ordinal}`,
    capturedAt: `2026-09-02T19:39:0${ordinal}.000Z`,
    handoffUrl: 'https://handoff.dev.example.test/remote-view/e2',
    external: true,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    publicEgressObserved: true,
  }));
}

function agentIds() {
  return Array.from({ length: 25 }, (_, index) => `calibration-agent-${String(index + 1).padStart(2, '0')}`);
}

function harness({ failures = new Map(), stopAt = null } = {}) {
  let now = START_MS;
  const calls = [];
  let startExecutionCalls = 0;
  let retryCalls = 0;
  let repairCalls = 0;
  const execute = async (kind, request) => {
    calls.push(structuredClone({ kind, request }));
    now += 2_000;
    const error = failures.get(`${kind}:${request.ordinal}`);
    if (error) throw error;
    return { outcome: 'passed', latencyMs: request.ordinal % 17 + 1 };
  };
  return {
    calls,
    get startExecutionCalls() { return startExecutionCalls; },
    get retryCalls() { return retryCalls; },
    get repairCalls() { return repairCalls; },
    clock: { wallNow: () => new Date(now).toISOString(), monotonicNow: () => now * 1_000_000 },
    scheduler: {
      waitUntil: async ({ wallTime }) => { now = Math.max(now, Date.parse(wallTime)); },
    },
    safetyStop: ({ completedActionCount }) => stopAt === completedActionCount
      ? { code: 'host_pressure', message: 'Synthetic safety boundary' }
      : null,
    effects: {
      executeServiceCommand: (request) => execute('service_command', request),
      executeDashboardAction: (request) => execute('dashboard_action', request),
      executeHandoffReconnect: (request) => execute('handoff_reconnect', request),
      startExecution: async () => { startExecutionCalls += 1; },
      retry: async () => { retryCalls += 1; },
      repair: async () => { repairCalls += 1; },
    },
  };
}

function input(overrides = {}) {
  const injected = harness(overrides.harness);
  return {
    injected,
    value: {
      calibrationId: 'p158-calibration-c01',
      developmentTargets: targets(),
      agentClientIds: agentIds(),
      externalViewerReceipts: viewers(),
      effects: injected.effects,
      scheduler: injected.scheduler,
      safetyStop: injected.safetyStop,
      clock: injected.clock,
      ...overrides.value,
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

await runTest('runs the exact C01 workload once and emits hash-bound artifacts', async () => {
  const { value, injected } = input();
  const result = await runC01Calibration(value);
  assert.deepEqual(C01_WORKLOAD, {
    durationMinutes: 20,
    agentClients: 25,
    externalViewers: 2,
    controllers: 1,
    serviceCommands: 500,
    dashboardActions: 50,
    handoffReconnects: 10,
  });
  assert.equal(injected.calls.length, 560);
  assert.deepEqual(
    result.observations.map((entry) => entry.ordinal),
    Array.from({ length: 560 }, (_, index) => index + 1),
  );
  assert.equal(new Set(injected.calls.slice(0, 500).map((entry) => entry.request.clientId)).size, 25);
  assert.deepEqual(injected.calls.map((entry) => entry.kind).reduce((counts, kind) => {
    counts[kind] = (counts[kind] ?? 0) + 1;
    return counts;
  }, {}), { service_command: 500, dashboard_action: 50, handoff_reconnect: 10 });
  assert.ok(injected.calls.every((entry) => entry.request.target.scope === 'development'));
  assert.ok(injected.calls.slice(500).every((entry) => entry.request.target.environmentId === 'E2'));
  assert.ok(injected.calls.slice(500).every((entry) => entry.request.externalViewerReceiptId));
  assert.equal(injected.startExecutionCalls, 0);
  assert.equal(injected.retryCalls, 0);
  assert.equal(injected.repairCalls, 0);
  assert.equal(result.calibration.clean, true);
  assert.deepEqual(result.calibration.workload, C01_WORKLOAD);
  assert.deepEqual(result.calibration.environmentIds, ['E1', 'E2']);
  assert.equal(Date.parse(result.calibration.completedAt) - Date.parse(result.calibration.startedAt), 20 * 60_000);
  assert.equal(result.calibration.declaredSha256, canonicalCalibrationDigest(result.calibration));
  assert.equal(validateCalibration(result.calibration), true, ajv.errorsText(validateCalibration.errors));
  assert.deepEqual(result.artifacts.map((artifact) => artifact.kind), [
    'calibration_raw', 'calibration_summary', 'calibration_budget',
  ]);
  for (const artifact of result.artifacts) {
    assert.equal(validateArtifact(artifact), true, ajv.errorsText(validateArtifact.errors));
    assert.equal(artifact.contentEncoding, 'utf8');
    assert.equal(artifact.declaredSha256, sha256(artifact.content));
    assert.equal(artifact.declaredByteCount, Buffer.byteLength(artifact.content));
    assert.equal(artifact.content, canonicalJson(JSON.parse(artifact.content)));
  }
  assert.equal(result.calibration.rawArtifactSha256, result.artifacts[0].declaredSha256);
  assert.equal(result.calibration.summaryArtifactSha256, result.artifacts[1].declaredSha256);
  assert.equal(result.calibration.budgetSha256, result.artifacts[2].declaredSha256);
  const raw = JSON.parse(result.artifacts[0].content);
  assert.deepEqual(raw.externalViewerReceipts, viewers());
  const frozenBudget = JSON.parse(result.artifacts[2].content);
  assert.equal(frozenBudget.frozen, true);
  assert.ok(frozenBudget.environmentRelativeBudgets.agentCommandP95Ms > 0);
});

await runTest('preserves every first failure and never retries or repairs', async () => {
  const failures = new Map([
    ['service_command:7', Object.assign(new Error('first command failure'), { code: 'command_failed' })],
    ['dashboard_action:503', Object.assign(new Error('first dashboard failure'), { code: 'dashboard_failed' })],
    ['handoff_reconnect:552', Object.assign(new Error('first reconnect failure'), { code: 'reconnect_failed' })],
  ]);
  const { value, injected } = input({ harness: { failures } });
  const result = await runC01Calibration(value);
  assert.equal(injected.calls.length, 560);
  assert.equal(result.calibration.clean, false);
  assert.deepEqual(
    result.observations.filter((entry) => entry.state === 'failed').map((entry) => [entry.ordinal, entry.failure.code, entry.failure.message]),
    [
      [7, 'command_failed', 'first command failure'],
      [503, 'dashboard_failed', 'first dashboard failure'],
      [552, 'reconnect_failed', 'first reconnect failure'],
    ],
  );
  assert.ok(result.observations.every((entry) => entry.attempt === 1));
  assert.equal(injected.calls.filter((entry) => entry.request.ordinal === 7).length, 1);
  assert.equal(injected.startExecutionCalls, 0);
  assert.equal(injected.retryCalls, 0);
  assert.equal(injected.repairCalls, 0);
});

await runTest('safety stop terminalizes every remaining action without effects', async () => {
  const { value, injected } = input({ harness: { stopAt: 12 } });
  const result = await runC01Calibration(value);
  assert.equal(injected.calls.length, 12);
  assert.equal(result.observations.length, 560);
  assert.deepEqual(result.observations.slice(12).map((entry) => entry.state), Array(548).fill('safety_stopped'));
  assert.ok(result.observations.slice(12).every((entry) => entry.safetyStop.code === 'host_pressure'));
  assert.equal(result.calibration.clean, false);
  assert.equal(injected.startExecutionCalls, 0);
});

await runTest('fails closed before effects for invalid custody or development targets', async () => {
  const invalidInputs = [
    { agentClientIds: agentIds().slice(1) },
    { agentClientIds: [...agentIds().slice(0, 24), agentIds()[0]] },
    { externalViewerReceipts: viewers().slice(1) },
    { externalViewerReceipts: [viewers()[0], { ...viewers()[1], outsideServiceHost: false }] },
    { externalViewerReceipts: [viewers()[0], { ...viewers()[1], capturedAt: '2026-09-02T19:41:00.000Z' }] },
    { externalViewerReceipts: [{ ...viewers()[0], viewerId: agentIds()[0] }, viewers()[1]] },
    { developmentTargets: [{ ...targets()[0], scope: 'production' }, targets()[1]] },
    { developmentTargets: [{ ...targets()[0], serviceUrl: 'not-a-url' }, targets()[1]] },
    { developmentTargets: [{ ...targets()[0], profileRoot: 'relative/profile' }, targets()[1]] },
  ];
  for (const override of invalidInputs) {
    const { value, injected } = input({ value: override });
    await assert.rejects(() => runC01Calibration(value), CalibrationError);
    assert.deepEqual(injected.calls, []);
    assert.equal(injected.startExecutionCalls, 0);
  }
});

await runTest('rejects a short calibration after recording results without mislabeling success', async () => {
  const { value, injected } = input();
  delete value.scheduler;
  await assert.rejects(
    () => runC01Calibration(value),
    (error) => error instanceof CalibrationError && error.code === 'calibration_duration_short',
  );
  assert.equal(injected.calls.length, 560);
  assert.equal(injected.startExecutionCalls, 0);
});

await runTest('is deterministic and does not mutate caller inputs', async () => {
  const first = input();
  const firstSnapshot = structuredClone({
    calibrationId: first.value.calibrationId,
    developmentTargets: first.value.developmentTargets,
    agentClientIds: first.value.agentClientIds,
    externalViewerReceipts: first.value.externalViewerReceipts,
  });
  const firstResult = await runC01Calibration(first.value);
  assert.deepEqual({
    calibrationId: first.value.calibrationId,
    developmentTargets: first.value.developmentTargets,
    agentClientIds: first.value.agentClientIds,
    externalViewerReceipts: first.value.externalViewerReceipts,
  }, firstSnapshot);
  const second = input();
  assert.deepEqual(await runC01Calibration(second.value), firstResult);
});

process.stdout.write('P158 C01 calibration runner self-test passed\n');
