#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import {
  buildP158W9ActionPlan,
  canonicalP158W9TargetBindingDigest,
  canonicalW9ReceiptDigest,
  P158W9OrchestrationError,
  runP158W9Phase,
} from './lib/p158-w9-campaign-orchestrator.js';

const registry = JSON.parse(await readFile(
  new URL('../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url),
  'utf8',
));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w9-provider-free' });
const BASE = Date.parse('2026-09-03T00:00:00.000Z');

function target() {
  const value = {
    targetId: 'p158-development-campaign', runtimeLane: 'development', production: false,
    environmentIds: ['E1', 'E2'], repairAllowed: false, retryAllowed: false,
    garbageCollectionAllowed: false,
    runId: 'p158-w9-live', candidateSha256: '11'.repeat(32), workflowRunId: '123456789',
    workflowRunAttempt: 1, handoffUrlSha256: '22'.repeat(32),
    retainedIdentitySha256: '33'.repeat(32), externalVantageAggregateSha256: '44'.repeat(32),
    externalHandoffOracleSha256: '55'.repeat(32),
    serviceOrigins: { E1: 'http://127.0.0.1:19101', E2: 'https://service.p158.example' },
    serviceResolvedAddresses: { E2: ['203.0.113.42'] },
    reviewedLocalDevelopmentOrigin: 'http://127.0.0.1:19101',
    allowedExternalServiceOrigins: ['https://service.p158.example'],
    syntheticTarget: true,
    productionHostnames: ['service.agent-browser.example'],
  };
  value.reviewedOriginBindingSha256 = canonicalP158W9TargetBindingDigest(value);
  return value;
}

function windows() {
  return {
    C01: { startAt: new Date(BASE + 60_000).toISOString(), endAt: new Date(BASE + 1_260_000).toISOString() },
    C02: { startAt: new Date(BASE + 1_270_000).toISOString(), endAt: new Date(BASE + 1_271_000).toISOString() },
    C03: { startAt: new Date(BASE + 1_280_000).toISOString(), endAt: new Date(BASE + 1_281_000).toISOString() },
    C04: { startAt: new Date(BASE + 1_300_000).toISOString(), endAt: new Date(BASE + 30_100_000).toISOString() },
    C05: { startAt: new Date(BASE + 30_200_000).toISOString(), endAt: new Date(BASE + 116_600_000).toISOString() },
  };
}

function clockHarness() {
  let now = BASE;
  return {
    clock: { wallNow: () => new Date(now).toISOString() },
    scheduler: { waitUntil: async ({ wallTime }) => { now = Math.max(now, Date.parse(wallTime)); } },
  };
}

function controllerHarness() {
  let state = 'frozen';
  const results = [];
  let scheduledTeardown = {};
  let startCount = 0;
  let sealCount = 0;
  return {
    get startCount() { return startCount; },
    get sealCount() { return sealCount; },
    snapshot: () => ({ state, results: structuredClone(results), scheduledTeardown: structuredClone(scheduledTeardown) }),
    startExecution: async () => { assert.equal(state, 'frozen'); state = 'executing'; startCount += 1; },
    recordAttempt: async (result) => {
      assert.equal(results.some((entry) => entry.attemptId === result.attemptId), false);
      results.push(structuredClone(result));
    },
    recordScheduledTeardown: async (result) => { scheduledTeardown = structuredClone(result); },
    finishExecution: async () => { state = 'execution_terminal'; },
    sealEvidence: async () => { state = 'evidence_sealed'; sealCount += 1; },
  };
}

function receipt(action) {
  const binding = action.target ?? target();
  const body = {
    schemaVersion: 'agent-browser.p158-w9-action-receipt.v1',
    runId: binding.runId, candidateSha256: binding.candidateSha256,
    scheduleSha256: schedule.scheduleSha256,
    workflowRunId: binding.workflowRunId, workflowRunAttempt: binding.workflowRunAttempt,
    caseId: action.caseId, attemptId: action.attemptId, actionId: action.actionId,
    environmentId: action.environmentId, kind: action.kind,
    attempt: 1, state: 'passed', observedAt: '2026-09-04T12:00:00.000Z',
    effectClass: action.transport === 'external_ingress'
      ? 'external_ingress' : action.declaredFault ? 'declared_fault' : 'read_only',
    evidenceArtifactIds: [`artifact:${action.actionId}`],
    retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    ...(action.declaredFault ? { declaredTransition: {
      declarationId: action.actionId, kind: action.kind, beforeState: 'ready', afterState: 'declared_transition_complete',
    } } : {}),
    ...(action.transport === 'external_ingress' ? { externalEvidence: {
      offHost: true, outsideServiceNetworkNamespace: true, operatorVisibleState: 'ready',
      readyBeforePixels: true, pixelsObserved: true,
      externalVantageAggregateSha256: binding.externalVantageAggregateSha256,
      externalHandoffOracleSha256: binding.externalHandoffOracleSha256,
      handoffUrlSha256: binding.handoffUrlSha256,
      retainedIdentitySha256: binding.retainedIdentitySha256,
    } } : {}),
  };
  return { ...body, receiptSha256: canonicalW9ReceiptDigest(body) };
}

function driverHarness({ failOnceAt = null } = {}) {
  const calls = [];
  let failed = false;
  const execute = async (action) => {
    calls.push(structuredClone(action));
    if (!failed && action.actionId === failOnceAt) {
      failed = true;
      throw Object.assign(new Error('declared injected process interruption'), { code: 'injected_interruption' });
    }
    return receipt(action);
  };
  const hookNames = [
    'executeDistributedC01', 'executeServiceCommand', 'executeExternalDashboardAction',
    'executeExternalHandoffReconnect', 'executeDeclaredBrowserCrash',
    'executeDeclaredSupervisorTransition', 'executeScheduledTeardown', 'verifyEvidenceArtifact',
  ];
  return {
    hookBindings: Object.fromEntries(hookNames.map((method, index) => [method, {
      implementationKind: 'concrete_live', sourcePath: `scripts/live-hooks/${method}.js`,
      sourceSha256: String(index + 1).padStart(64, '0'),
    }])),
    calls,
    executeDistributedC01: async ({ actions }) => {
      const artifact = { artifactId: 'c01-frozen-result', content: '{"clean":true}\n' };
      artifact.declaredSha256 = sha256(artifact.content);
      const result = {
        calibration: { clean: true },
        distributedEvidence: {
          serviceCommandCount: 500, dashboardActionCount: 50, handoffReconnectCount: 10,
          externalReplayEffectCount: 0,
        },
        observations: actions.map((action) => ({
          kind: action.kind, state: 'passed', observedAt: '2026-09-04T12:00:00.000Z',
        })),
        artifacts: [artifact],
      };
      return {
        preparationSha256: '66'.repeat(32), localSha256: '77'.repeat(32),
        resultSha256: sha256(result), result,
      };
    },
    executeServiceCommand: execute,
    executeExternalDashboardAction: execute,
    executeExternalHandoffReconnect: execute,
    executeDeclaredBrowserCrash: execute,
    executeDeclaredSupervisorTransition: execute,
    executeScheduledTeardown: async ({ target: binding }) => {
      const body = {
        schemaVersion: 'agent-browser.p158-w9-teardown-receipt.v1',
        runId: binding.runId, candidateSha256: binding.candidateSha256,
        scheduleSha256: schedule.scheduleSha256, attempt: 1, state: 'passed',
        effectClass: 'scheduled_teardown', declaredTeardownId: `${binding.runId}:scheduled-teardown`,
        evidenceArtifactIds: ['artifact:scheduled-teardown'],
        retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
      };
      return { ...body, receiptSha256: canonicalW9ReceiptDigest(body) };
    },
    verifyEvidenceArtifact: async () => true,
  };
}

async function withRoot(body) {
  const root = await mkdtemp(join(tmpdir(), 'p158-w9-'));
  try { return await body(root); } finally { await rm(root, { recursive: true, force: true }); }
}

async function runTest(name, body) {
  try { await body(); process.stdout.write(`PASS ${name}\n`); }
  catch (error) { error.message = `${name}: ${error.message}`; throw error; }
}

await runTest('materializes the exact corrected C01 through C05 action allocation', async () => {
  const plan = buildP158W9ActionPlan(schedule);
  assert.deepEqual(Object.fromEntries(['C01', 'C02', 'C03', 'C04', 'C05'].map((caseId) => [
    caseId, plan.filter((entry) => entry.attempt.caseId === caseId).length,
  ])), { C01: 10, C02: 100, C03: 25, C04: 200, C05: 500 });
  const counts = {};
  for (const action of plan.flatMap((entry) => entry.actions)) {
    counts[`${action.caseId}:${action.kind}`] = (counts[`${action.caseId}:${action.kind}`] ?? 0) + 1;
  }
  assert.deepEqual(counts, {
    'C01:service_command': 500, 'C01:dashboard_action': 50, 'C01:handoff_reconnect': 10,
    'C02:service_command': 2000, 'C02:dashboard_action': 500, 'C02:handoff_reconnect': 100,
    'C02:declared_browser_crash': 20,
    'C03:declared_supervisor_transition': 25,
    'C04:service_command': 10000, 'C04:dashboard_action': 2000, 'C04:handoff_reconnect': 200,
    'C04:declared_browser_crash': 50,
    'C05:handoff_reconnect': 500,
  });
  assert.ok(plan.flatMap((entry) => entry.actions)
    .filter((action) => ['dashboard_action', 'handoff_reconnect'].includes(action.kind))
    .every((action) => action.environmentId === 'E2' && action.transport === 'external_ingress'));
  assert.deepEqual([...new Set(plan.filter((entry) => entry.attempt.caseId === 'C03')
    .flatMap((entry) => entry.actions.map((action) => action.mixedLoad)))].sort(),
  ['dashboard_use', 'durable_handoff_reopen', 'retained_browser_commands']);
  assert.deepEqual([...new Set(plan.filter((entry) => entry.attempt.caseId === 'C05')
    .flatMap((entry) => entry.actions.map((action) => action.enduranceEvent)))].sort(),
  ['client_restart', 'controller_expiry', 'scheduled_network_profile', 'viewer_expiry']);
});

await runTest('executes once, crosses future 20m 8h and 24h barriers, tears down, and seals', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  const frozenInputs = { schedule: structuredClone(schedule), target: target(), caseWindows: windows() };
  const result = await runP158W9Phase({
    ...frozenInputs, drivers, controller, runRoot, artifactStore, clock: time.clock, scheduler: time.scheduler,
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(controller.startCount, 1);
  assert.equal(controller.sealCount, 1);
  assert.equal(controller.snapshot().results.length, 835);
  assert.equal(drivers.calls.length, 15395);
  assert.equal(new Set(drivers.calls.map((call) => call.actionId)).size, drivers.calls.length);
  assert.equal(drivers.calls.filter((call) => call.kind === 'declared_browser_crash').length, 70);
  assert.equal(drivers.calls.filter((call) => call.kind === 'declared_supervisor_transition').length, 25);
  assert.ok(drivers.calls.filter((call) => ['dashboard_action', 'handoff_reconnect'].includes(call.kind))
    .every((call) => call.environmentId === 'E2' && call.transport === 'external_ingress'));
  assert.equal(artifactStore.paths().filter((path) => path.includes('/actions-started/')).length, 15955);
  assert.equal(artifactStore.paths().filter((path) => path.includes('/actions-terminal/')).length, 15955);
  assert.equal(artifactStore.paths().filter((path) => path.includes('/attempts/')).length, 835);
  assert.deepEqual(frozenInputs, { schedule, target: target(), caseWindows: windows() });
}));

await runTest('resumes append-only after interruption without replaying the uncertain action', () => withRoot(async (runRoot) => {
  const plan = buildP158W9ActionPlan(schedule);
  const interrupted = plan.find((entry) => entry.attempt.caseId === 'C02').actions[3].actionId;
  const controller = controllerHarness();
  const drivers = driverHarness({ failOnceAt: interrupted });
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  await assert.rejects(() => runP158W9Phase({
    schedule, target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler,
  }), /injected process interruption/u);
  const callsAtCrash = drivers.calls.length;
  const result = await runP158W9Phase({
    schedule, target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler,
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(drivers.calls.filter((call) => call.actionId === interrupted).length, 1);
  assert.ok(drivers.calls.length > callsAtCrash);
  assert.equal(controller.startCount, 1);
  assert.equal(controller.snapshot().results.find((entry) => entry.caseId === 'C02').resultState,
    'harness_failure');
}));

await runTest('safety stop terminalizes remaining work without effects', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  let observations = 0;
  const result = await runP158W9Phase({
    schedule, target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler,
    safetyStop: async () => (++observations === 1 ? { code: 'injected_safety_stop' } : null),
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(result.safetyStop.code, 'injected_safety_stop');
  assert.equal(drivers.calls.length, 0);
  assert.ok(controller.snapshot().results.every((entry) => entry.resultState === 'safety_stopped'));
}));

await runTest('refuses production and source mutation before undeclared effects', () => withRoot(async (runRoot) => {
  const bad = target(); bad.production = true;
  const drivers = driverHarness();
  const controller = controllerHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  await assert.rejects(
    () => runP158W9Phase({ schedule, target: bad, caseWindows: windows(), drivers, controller, runRoot, artifactStore,
      clock: time.clock, scheduler: time.scheduler }),
    (error) => error instanceof P158W9OrchestrationError && error.code === 'development_target_unproven',
  );
  assert.equal(drivers.calls.length, 0);
  const blocked = driverHarness();
  blocked.hookBindings.executeServiceCommand = {
    implementationKind: 'explicit_blocked', sourcePath: 'scripts/live-hooks/service.js',
    sourceSha256: '88'.repeat(32), reason: 'operator authority withheld',
  };
  await assert.rejects(
    () => runP158W9Phase({ schedule, target: target(), caseWindows: windows(), drivers: blocked,
      controller: controllerHarness(), runRoot, artifactStore: createMemoryArtifactStore(),
      clock: time.clock, scheduler: time.scheduler }),
    (error) => error.code === 'live_hook_blocked',
  );
  assert.equal(blocked.calls.length, 0);
}));

await runTest('fails closed on unbound and externally unproven live receipts', async () => {
  for (const [expectedCode, corrupt] of [
    ['action_receipt_invalid', (value) => { value.candidateSha256 = 'ff'.repeat(32); }],
    ['external_ingress_receipt_unproven', (value) => { value.externalEvidence.readyBeforePixels = false; }],
  ]) {
    await withRoot(async (runRoot) => {
      const controller = controllerHarness();
      const drivers = driverHarness();
      const method = expectedCode === 'action_receipt_invalid'
        ? 'executeServiceCommand' : 'executeExternalDashboardAction';
      const original = drivers[method];
      drivers[method] = async (action) => {
        const value = await original(action);
        corrupt(value);
        value.receiptSha256 = canonicalW9ReceiptDigest(value);
        return value;
      };
      const time = clockHarness();
      await assert.rejects(
        () => runP158W9Phase({
          schedule, target: target(), caseWindows: windows(), drivers, controller, runRoot,
          artifactStore: createMemoryArtifactStore(), clock: time.clock, scheduler: time.scheduler,
        }),
        (error) => error.code === expectedCode,
      );
    });
  }
});

process.stdout.write('P158 W9 campaign orchestration test passed\n');
