#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { canonicalJson, createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
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
    C04: { environmentId: 'E3', observationMode: 'passive_segmented',
      completionMode: 'asynchronous_nonblocking', minimumDurationSeconds: 28_800,
      productionActionsGenerated: false, blocksInstallationOrRepair: false },
    C05: { environmentId: 'E3', observationMode: 'passive_segmented',
      completionMode: 'asynchronous_nonblocking', minimumDurationSeconds: 86_400,
      productionActionsGenerated: false, blocksInstallationOrRepair: false },
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
  let snapshotCount = 0;
  const artifacts = [];
  return {
    get startCount() { return startCount; },
    get sealCount() { return sealCount; },
    get snapshotCount() { return snapshotCount; },
    snapshot: () => {
      snapshotCount += 1;
      return { state, results: structuredClone(results), scheduledTeardown: structuredClone(scheduledTeardown),
        evidence: { artifacts: structuredClone(artifacts) } };
    },
    startExecution: async () => { assert.equal(state, 'frozen'); state = 'executing'; startCount += 1; },
    recordAttempt: async (result) => {
      assert.equal(results.some((entry) => entry.attemptId === result.attemptId), false);
      results.push(structuredClone(result));
    },
    recordScheduledTeardown: async (result) => { scheduledTeardown = structuredClone(result); },
    writeArtifact: async ({ artifactId, relativePath, content, metadata = {} }) => {
      const bytes = Buffer.from(content);
      const receipt = { artifactId, relativePath, sha256: sha256(bytes), byteCount: bytes.byteLength,
        ...structuredClone(metadata) };
      artifacts.push(receipt);
      return receipt;
    },
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
    operationCorrelationId: action.requestId ?? `p158:${binding.runId}:${action.actionId}:request`,
    productRequestId: null, correlationState: 'product_request_id_unavailable',
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
  const passiveCalls = [];
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
    'executeDeclaredSupervisorTransition', 'schedulePassiveObservation',
    'executeScheduledTeardown', 'verifyEvidenceArtifact',
  ];
  return {
    enduranceCaseWindowsSha256: sha256({ C04: windows().C04, C05: windows().C05 }),
    hookBindings: Object.fromEntries(hookNames.map((method, index) => [method, {
      implementationKind: 'concrete_live', sourcePath: `scripts/live-hooks/${method}.js`,
      sourceSha256: String(index + 1).padStart(64, '0'),
    }])),
    calls, passiveCalls,
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
    schedulePassiveObservation: async ({ caseId, attempt }) => {
      passiveCalls.push({ caseId, attemptId: attempt.attemptId });
      const body = {
        schemaVersion: 'agent-browser.p158-production-observation-descriptor.v1',
        caseId, environmentId: 'E3', observationMode: 'passive_segmented',
        completionMode: 'asynchronous_nonblocking', productionActionsGenerated: false,
        blocksInstallationOrRepair: false, waitPerformed: false,
      };
      return { ...body, descriptorSha256: sha256(body),
        evidenceArtifactIds: [`artifact:${caseId}:passive-observation`] };
    },
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

function adapterBindings(blockedCaseIds = []) {
  const blocked = new Set(blockedCaseIds);
  const actionCounts = new Map(buildP158W9ActionPlan(schedule).map((entry) => [entry.attempt.caseId, 0]));
  for (const entry of buildP158W9ActionPlan(schedule)) {
    actionCounts.set(entry.attempt.caseId, actionCounts.get(entry.attempt.caseId) + entry.actions.length);
  }
  return ['C01', 'C02', 'C03', 'C04', 'C05'].map((caseId) => {
    const sourcePath = 'scripts/lib/p158-w9-concrete-drivers.js';
    const sourceSha256 = 'ab'.repeat(32);
    const isBlocked = blocked.has(caseId);
    const isPassive = !isBlocked && ['C04', 'C05'].includes(caseId);
    return {
      caseId, mode: isBlocked ? 'explicit_blocked' : isPassive ? 'passive_observer' : 'concrete_live', providerFree: false,
      sourcePath, sourceSha256, hookIds: [],
      implementedActionCount: isBlocked || isPassive ? 0 : actionCounts.get(caseId),
      blockedActionCount: isBlocked ? actionCounts.get(caseId) : 0,
      effectsAllowed: !isBlocked && !isPassive,
      observationScheduled: isPassive,
      blocker: isBlocked ? { code: 'live_case_hook_missing', detail: `missing ${caseId}` } : null,
    };
  });
}

function loggingHarvest(state = 'complete') {
  const sourcePath = 'scripts/lib/p158-logging-evidence-harvester.js';
  const sourceSha256 = 'cd'.repeat(32);
  return {
    sourcePath, sourceSha256,
    execute: async ({ target: binding, schedule: frozenSchedule, registerArtifact }) => {
      assert.equal(typeof registerArtifact, 'function');
      await registerArtifact({ artifactId: 'logging-corpus', relativePath: 'logging/w3-corpus.json',
        content: canonicalJson({ schemaVersion: 'p158-test-logging-corpus.v1', state }),
        metadata: { mediaType: 'application/json', analysisRole: 'logging_evidence',
          capturePurpose: 'logging_evidence', captureState: 'complete' } });
      const body = {
        schemaVersion: 'agent-browser.p158-logging-harvest-receipt.v1', runId: binding.runId,
        scheduleSha256: frozenSchedule.scheduleSha256, sourcePath, sourceSha256,
        state, artifactIds: ['logging-corpus'], repairAttempted: false, retryAttempted: false,
      };
      return { ...body, receiptSha256: sha256(body) };
    },
  };
}

function loggingExpectations(blockedCaseIds = []) {
  const blocked = new Set(blockedCaseIds);
  return buildP158W9ActionPlan(schedule).flatMap((entry) => {
    const descriptors = blocked.has(entry.attempt.caseId)
      ? entry.attempt.environmentIds.map((environmentId) => ({ environmentId, actionId: null,
          expectationId: `${entry.attempt.attemptId}:${environmentId}:blocked` }))
      : entry.actions.map((action) => ({ environmentId: action.environmentId, actionId: action.actionId,
          expectationId: `${entry.attempt.attemptId}:${action.actionId}` }));
    return descriptors.map((descriptor) => {
      const action = descriptor.actionId === null ? null : entry.actions.find((item) => item.actionId === descriptor.actionId);
      const operationCorrelationId = descriptor.actionId === null
        ? `p158:p158-w9-live:${entry.attempt.attemptId}:${descriptor.environmentId}:blocked`
        : `p158:p158-w9-live:${descriptor.actionId}:request`;
      return { ...descriptor, attemptId: entry.attempt.attemptId, caseId: entry.attempt.caseId,
        phaseId: 'W9', operationCorrelationId, productRequestId: null,
        productRequestIdState: descriptor.actionId === null ? 'not_applicable' : 'assigned_at_runtime',
        requestKind: descriptor.actionId === null ? 'rejected_request'
          : action.declaredFault ? 'transition'
            : action.transport === 'external_ingress' ? 'dashboard_action' : 'accepted_request',
        executionMode: descriptor.actionId === null ? 'explicit_blocked' : 'concrete_live',
        expectedSurfaceRoles: descriptor.actionId === null
          ? ['controller_transition', 'pre_execution_blocker', 'terminal_event']
          : action.kind === 'service_command'
            ? ['ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome']
            : action.declaredFault
              ? ['controller_transition', 'terminal_event', 'trace_outcome']
              : ['ingress_request', 'immediate_response', 'terminal_event', 'dashboard_projection'],
        causalIds: { requestId: null, jobId: null, eventId: null, traceId: null, incidentId: null } };
    });
  });
}

async function withRoot(body) {
  const root = await mkdtemp(join(tmpdir(), 'p158-w9-'));
  try { return await body(root); } finally { await rm(root, { recursive: true, force: true }); }
}

async function runTest(name, body) {
  try { await body(); process.stdout.write(`PASS ${name}\n`); }
  catch (error) { error.message = `${name}: ${error.message}`; throw error; }
}

await runTest('materializes active C01 through C03 actions and passive C04/C05 descriptors', async () => {
  const plan = buildP158W9ActionPlan(schedule);
  assert.deepEqual(Object.fromEntries(['C01', 'C02', 'C03', 'C04', 'C05'].map((caseId) => [
    caseId, plan.filter((entry) => entry.attempt.caseId === caseId).length,
  ])), { C01: 10, C02: 100, C03: 25, C04: 1, C05: 1 });
  const counts = {};
  for (const action of plan.flatMap((entry) => entry.actions)) {
    counts[`${action.caseId}:${action.kind}`] = (counts[`${action.caseId}:${action.kind}`] ?? 0) + 1;
  }
  assert.deepEqual(counts, {
    'C01:service_command': 500, 'C01:dashboard_action': 50, 'C01:handoff_reconnect': 10,
    'C02:service_command': 2000, 'C02:dashboard_action': 500, 'C02:handoff_reconnect': 100,
    'C02:declared_browser_crash': 20,
    'C03:declared_supervisor_transition': 25,
  });
  assert.ok(plan.flatMap((entry) => entry.actions)
    .filter((action) => ['dashboard_action', 'handoff_reconnect'].includes(action.kind))
    .every((action) => action.environmentId === 'E2' && action.transport === 'external_ingress'));
  assert.deepEqual([...new Set(plan.filter((entry) => entry.attempt.caseId === 'C03')
    .flatMap((entry) => entry.actions.map((action) => action.mixedLoad)))].sort(),
  ['dashboard_use', 'durable_handoff_reopen', 'retained_browser_commands']);
  assert.ok(plan.filter((entry) => ['C04', 'C05'].includes(entry.attempt.caseId))
    .every((entry) => entry.attempt.executionMode === 'passive_observer' && entry.actions.length === 0));
});

await runTest('executes active work once, schedules passive epochs without waiting, tears down, and seals', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  const analysisOrder = [];
  const finalAnalysis = { rawArtifactInventory: [], loggingOperationGaps: [], hook: {
    prepareBeforeSeal: async () => {
      assert.equal(controller.snapshot().state, 'execution_terminal');
      analysisOrder.push('prepare');
      return { preparationSha256: 'aa'.repeat(32) };
    },
    finalizeAfterSeal: async () => {
      assert.equal(controller.snapshot().state, 'evidence_sealed');
      analysisOrder.push('finalize');
      return { descriptorSha256: 'bb'.repeat(32) };
    },
  } };
  const frozenInputs = { schedule: structuredClone(schedule), target: target(), caseWindows: windows(),
    adapterBindings: adapterBindings() };
  const result = await runP158W9Phase({
    ...frozenInputs, drivers, controller, runRoot, artifactStore, clock: time.clock, scheduler: time.scheduler,
    loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations(), finalAnalysis,
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.deepEqual(analysisOrder, ['prepare', 'finalize']);
  assert.equal(result.finalAnalysisDescriptorSha256, 'bb'.repeat(32));
  assert.equal(controller.startCount, 1);
  assert.equal(controller.sealCount, 1);
  assert.equal(controller.snapshotCount, 7,
    'W9 plus two-stage analysis must take a bounded controller snapshot set on the clean path');
  assert.equal(controller.snapshot().results.length, 137);
  assert.equal(drivers.calls.length, 2645);
  assert.deepEqual(drivers.passiveCalls.map((entry) => entry.caseId), ['C04', 'C05']);
  assert.equal(new Set(drivers.calls.map((call) => call.actionId)).size, drivers.calls.length);
  assert.equal(drivers.calls.filter((call) => call.kind === 'declared_browser_crash').length, 20);
  assert.equal(drivers.calls.filter((call) => call.kind === 'declared_supervisor_transition').length, 25);
  assert.ok(drivers.calls.filter((call) => ['dashboard_action', 'handoff_reconnect'].includes(call.kind))
    .every((call) => call.environmentId === 'E2' && call.transport === 'external_ingress'));
  assert.equal(artifactStore.paths().filter((path) => path.includes('/actions-started/')).length, 3205);
  assert.equal(artifactStore.paths().filter((path) => path.includes('/actions-terminal/')).length, 3205);
  assert.equal(artifactStore.paths().filter((path) => path.includes('/attempts/')).length, 137);
  const passiveResults = controller.snapshot().results.filter((entry) => ['C04', 'C05'].includes(entry.caseId));
  assert.ok(passiveResults.every((entry) => entry.resultState === 'inconclusive' &&
    entry.effectState === 'no_effect' && entry.blocksDependents === false));
  const externalResult = controller.snapshot().results.find((entry) =>
    entry.causalEnvelopes?.some((envelope) => envelope.environmentId === 'E2' && envelope.actionId));
  const externalExpectation = loggingExpectations().find((entry) =>
    externalResult.causalEnvelopes.some((envelope) => envelope.expectationId === entry.expectationId) &&
    entry.requestKind === 'dashboard_action');
  assert.ok(externalExpectation.expectedSurfaceRoles.includes('ingress_request'));
  assert.equal(externalResult.causalRecords.some((record) =>
    record.expectationId === externalExpectation.expectationId && record.surfaceRole === 'ingress_request'), false,
  'external ingress must remain an explicit harvest gap unless the workflow receipt supplies it');
  assert.deepEqual(frozenInputs, { schedule, target: target(), caseWindows: windows(), adapterBindings: adapterBindings() });
}));

await runTest('resumes append-only after interruption without replaying the uncertain action', () => withRoot(async (runRoot) => {
  const plan = buildP158W9ActionPlan(schedule);
  const interrupted = plan.find((entry) => entry.attempt.caseId === 'C02').actions[3].actionId;
  const controller = controllerHarness();
  const drivers = driverHarness({ failOnceAt: interrupted });
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  await assert.rejects(() => runP158W9Phase({
    schedule, adapterBindings: adapterBindings(), target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations(),
  }), /injected process interruption/u);
  const callsAtCrash = drivers.calls.length;
  const result = await runP158W9Phase({
    schedule, adapterBindings: adapterBindings(), target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations(),
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
    schedule, adapterBindings: adapterBindings(), target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations(),
    safetyStop: async () => (++observations === 1 ? { code: 'injected_safety_stop' } : null),
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(result.safetyStop.code, 'injected_safety_stop');
  assert.equal(drivers.calls.length, 0);
  assert.ok(controller.snapshot().results.every((entry) =>
    entry.resultState === 'safety_stopped' ||
    (entry.resultState === 'skipped_blocked' && entry.effectState === 'not_started')));
}));

await runTest('refuses production and source mutation before undeclared effects', () => withRoot(async (runRoot) => {
  const bad = target(); bad.production = true;
  const drivers = driverHarness();
  const controller = controllerHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  await assert.rejects(
    () => runP158W9Phase({ schedule, adapterBindings: adapterBindings(), target: bad, caseWindows: windows(), drivers, controller, runRoot, artifactStore,
      clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations() }),
    (error) => error instanceof P158W9OrchestrationError && error.code === 'development_target_unproven',
  );
  assert.equal(drivers.calls.length, 0);
  const blocked = driverHarness();
  blocked.hookBindings.executeServiceCommand = {
    implementationKind: 'explicit_blocked', sourcePath: 'scripts/live-hooks/service.js',
    sourceSha256: '88'.repeat(32), reason: 'operator authority withheld',
  };
  await assert.rejects(
    () => runP158W9Phase({ schedule, adapterBindings: adapterBindings(), target: target(), caseWindows: windows(), drivers: blocked,
      controller: controllerHarness(), runRoot, artifactStore: createMemoryArtifactStore(),
      clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations() }),
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
          schedule, adapterBindings: adapterBindings(), target: target(), caseWindows: windows(), drivers, controller, runRoot,
          artifactStore: createMemoryArtifactStore(), clock: time.clock, scheduler: time.scheduler,
          loggingHarvest: loggingHarvest(), loggingExpectations: loggingExpectations(),
        }),
        (error) => error.code === expectedCode,
      );
    });
  }
});

await runTest('terminalizes a mixed explicit blocker without invoking its action drivers', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const result = await runP158W9Phase({
    schedule, adapterBindings: adapterBindings(['C02']), target: target(), caseWindows: windows(),
    drivers, controller, runRoot, artifactStore: createMemoryArtifactStore(),
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(),
    loggingExpectations: loggingExpectations(['C02']),
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(drivers.calls.some((call) => call.caseId === 'C02'), false);
  assert.equal(drivers.calls.some((call) => call.caseId === 'C03'), false,
    'dependent C03 must be blocked before any concrete action');
  const blocked = controller.snapshot().results.filter((entry) => entry.caseId === 'C02');
  assert.equal(blocked.length, 100);
  assert.ok(blocked.every((entry) => entry.resultState === 'skipped_blocked' &&
    entry.effectState === 'not_started' && entry.requestedEffects.length === 0));
  const dependent = controller.snapshot().results.filter((entry) => entry.caseId === 'C03');
  assert.ok(dependent.every((entry) => entry.resultState === 'skipped_blocked' &&
    entry.blockerCode === 'dependency_terminal_unusable' && entry.actionCount === 0));
  assert.deepEqual(blocked[0].causalRecords.map((entry) => entry.surfaceRole),
    ['controller_transition', 'pre_execution_blocker', 'terminal_event',
      'controller_transition', 'pre_execution_blocker', 'terminal_event']);
  assert.ok(blocked[0].causalRecords.every((entry) => entry.transport === 'controller'));
}));

await runTest('resumes an all-blocked W9 phase without effects or duplicate terminals', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  const input = {
    schedule, adapterBindings: adapterBindings(['C01', 'C02', 'C03', 'C04', 'C05']),
    target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: loggingHarvest(),
    loggingExpectations: loggingExpectations(['C01', 'C02', 'C03', 'C04', 'C05']),
  };
  const first = await runP158W9Phase(input);
  const second = await runP158W9Phase(input);
  assert.equal(first.state, 'evidence_sealed');
  assert.equal(second.state, 'evidence_sealed');
  assert.equal(drivers.calls.length, 0);
  assert.equal(controller.snapshot().results.length, 137);
  assert.equal(controller.startCount, 1);
  assert.equal(controller.sealCount, 1);
}));

await runTest('retains a failed pre-seal harvest without replaying campaign work on resume', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const artifactStore = createMemoryArtifactStore();
  let harvestCalls = 0;
  const failedHarvest = loggingHarvest();
  failedHarvest.execute = async () => {
    harvestCalls += 1;
    throw Object.assign(new Error('synthetic logging capture failed'), { code: 'logging_capture_failed' });
  };
  const input = {
    schedule, adapterBindings: adapterBindings(['C01', 'C02', 'C03', 'C04', 'C05']),
    target: target(), caseWindows: windows(), drivers, controller, runRoot, artifactStore,
    clock: time.clock, scheduler: time.scheduler, loggingHarvest: failedHarvest,
    loggingExpectations: loggingExpectations(['C01', 'C02', 'C03', 'C04', 'C05']),
  };
  await assert.rejects(() => runP158W9Phase(input), (error) => error.code === 'logging_capture_failed');
  assert.equal(controller.snapshot().state, 'executing');
  assert.equal(controller.sealCount, 0);
  assert.equal(drivers.calls.length, 0);
  await assert.rejects(() => runP158W9Phase({ ...input, loggingHarvest: loggingHarvest() }),
    (error) => error.code === 'logging_harvest_failed');
  assert.equal(harvestCalls, 1);
  assert.equal(drivers.calls.length, 0);
  assert.equal(controller.snapshot().results.length, 137);
}));

await runTest('seals an explicit logging capture gap for independent W10 analysis', () => withRoot(async (runRoot) => {
  const controller = controllerHarness();
  const drivers = driverHarness();
  const time = clockHarness();
  const result = await runP158W9Phase({
    schedule, adapterBindings: adapterBindings(['C01', 'C02', 'C03', 'C04', 'C05']),
    target: target(), caseWindows: windows(), drivers, controller, runRoot,
    artifactStore: createMemoryArtifactStore(), clock: time.clock, scheduler: time.scheduler,
    loggingHarvest: loggingHarvest('capture_gap'),
    loggingExpectations: loggingExpectations(['C01', 'C02', 'C03', 'C04', 'C05']),
  });
  assert.equal(result.state, 'evidence_sealed');
  assert.equal(controller.sealCount, 1);
  assert.equal(drivers.calls.length, 0);
}));

process.stdout.write('P158 W9 campaign orchestration test passed\n');
