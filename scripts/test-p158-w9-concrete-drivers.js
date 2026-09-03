#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { createMemoryArtifactStore, sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  buildP158W9ActionPlan,
  canonicalP158W9TargetBindingDigest,
} from './lib/p158-w9-campaign-orchestrator.js';
import {
  canonicalW9PlanDigest,
  createP158W9ConcreteDriverBundle,
  createP158W9FreezeAdapterEntries,
  p158W9HookManifestEntries,
  P158_W9_MANIFEST_HOOK_IDS,
} from './lib/p158-w9-concrete-drivers.js';

const registry = JSON.parse(await readFile(
  new URL('../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url), 'utf8',
));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w9-concrete-drivers' });
const actions = buildP158W9ActionPlan(schedule).flatMap((entry) => entry.actions);

function target() {
  const value = {
    runId: 'p158-w9-concrete', candidateSha256: '11'.repeat(32),
    runtimeLane: 'development', production: false, repairAllowed: false,
    retryAllowed: false, garbageCollectionAllowed: false,
    environmentIds: ['E1', 'E2'],
    workflowRunId: '123456789', workflowRunAttempt: 1,
    handoffUrlSha256: '22'.repeat(32), retainedIdentitySha256: '33'.repeat(32),
    externalVantageAggregateSha256: '44'.repeat(32), externalHandoffOracleSha256: '55'.repeat(32),
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

function externalReceipt(action, workflowPlanSha256) {
  const body = {
    actionId: action.actionId, caseId: action.caseId, attemptId: action.attemptId,
    kind: action.kind, environmentId: 'E2', state: 'passed', attempt: 1,
    observedAt: '2026-09-03T12:00:00.000Z', retryAttempted: false, repairAttempted: false,
    offHost: true, outsideServiceNetworkNamespace: true, operatorVisibleState: 'ready',
    readyBeforePixels: true, pixelsObserved: true,
    handoffUrlSha256: target().handoffUrlSha256,
    retainedIdentitySha256: target().retainedIdentitySha256,
    externalVantageAggregateSha256: target().externalVantageAggregateSha256,
    externalHandoffOracleSha256: target().externalHandoffOracleSha256,
    workflowPlanSha256,
  };
  return { ...body, receiptSha256: sha256(body) };
}

function plans({ omitActionId = null } = {}) {
  const externalActions = actions.filter((action) =>
    action.caseId !== 'C01' && ['dashboard_action', 'handoff_reconnect'].includes(action.kind) &&
    action.actionId !== omitActionId);
  const external = {
    schemaVersion: 'agent-browser.p158-w9-external-workflow-plan.v1',
    runId: target().runId, candidateSha256: target().candidateSha256,
    scheduleSha256: schedule.scheduleSha256, workflowRunId: target().workflowRunId,
    workflowRunAttempt: target().workflowRunAttempt,
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
    actions: externalActions.map((action) => ({ actionId: action.actionId, receiptPath: `/evidence/${action.actionId}.json` })),
  };
  external.planSha256 = canonicalW9PlanDigest(external);
  for (let index = 0; index < external.actions.length; index += 1) {
    external.actions[index].receipt = externalReceipt(externalActions[index], external.planSha256);
  }
  const transition = {
    schemaVersion: 'agent-browser.p158-w9-declared-transition-plan.v1',
    runId: target().runId, candidateSha256: target().candidateSha256,
    scheduleSha256: schedule.scheduleSha256, workflowRunId: target().workflowRunId,
    workflowRunAttempt: target().workflowRunAttempt,
    repairAllowed: false, retryAllowed: false, garbageCollectionAllowed: false,
    actions: actions.filter((action) => action.declaredFault).map((action) => ({
      actionId: action.actionId, transitionKind: action.kind,
      beforeState: 'ready', afterState: 'declared_transition_complete',
      ...(action.kind === 'declared_browser_crash'
        ? { process: { pid: 4242, signal: 'SIGTERM' } }
        : { systemd: { unit: 'agent-browser-development.service', verb: 'restart' } }),
    })),
    teardown: { systemd: { unit: 'agent-browser-development.service', verb: 'stop' } },
  };
  transition.planSha256 = canonicalW9PlanDigest(transition);
  return { external, transition };
}

function c01() {
  return {
    driverId: 'p158.distributed-c01-live.v1',
    config: { runId: target().runId, candidate: { candidateSha256: target().candidateSha256 } },
    runRoot: '/tmp/p158-c01',
    externalAggregatePath: '/tmp/p158-c01-downloads/aggregate.json',
    externalReceiptPaths: ['/tmp/p158-c01-downloads/receipt-1.json', '/tmp/p158-c01-downloads/receipt-2.json'],
    clock: { wallNow: () => '2026-09-03T00:00:00.000Z' },
    scheduler: { waitUntil: async () => {} },
  };
}

function transitionPrimitives() {
  return {
    executeProcess: async () => ({ resultState: 'passed', observedAt: '2026-09-03T12:00:00.000Z' }),
    executeSystemd: async () => ({ resultState: 'passed', observedAt: '2026-09-03T12:00:00.000Z' }),
  };
}

function makeBundle({ testing, omitActionId = null, targetOverride = null, fetchOverride = null } = {}) {
  const prepared = plans({ omitActionId });
  const options = {
    schedule, target: targetOverride ?? target(), artifactStore: createMemoryArtifactStore(),
    externalWorkflowPlan: prepared.external, declaredTransitionPlan: prepared.transition,
    c01: c01(), testing,
    clock: { wallNow: () => '2026-09-03T12:00:00.000Z', monotonicNow: () => 1_000_000 },
  };
  if (testing) {
    options.fetch = fetchOverride ?? (async (url) => ({ ok: true, status: 200, url, json: async () => ({ success: true, data: {} }) }));
    options.transitionPrimitives = transitionPrimitives();
  }
  return createP158W9ConcreteDriverBundle(options);
}

async function runTest(name, body) {
  try { await body(); process.stdout.write(`PASS ${name}\n`); }
  catch (error) { error.message = `${name}: ${error.message}`; throw error; }
}

await runTest('classifies complete reviewed C01 through C05 plans as concrete live', async () => {
  const bundle = makeBundle({ testing: false });
  assert.equal(bundle.freezeEligible, true);
  const c01Binding = bundle.drivers.hookBindings.executeDistributedC01;
  assert.equal(c01Binding.sourcePath, 'scripts/run-p158-distributed-calibration-live.js');
  assert.equal(c01Binding.sourceSha256, sha256(await readFile(c01Binding.sourcePath)));
  assert.deepEqual(Object.fromEntries(bundle.classification), Object.fromEntries(
    ['C01', 'C02', 'C03', 'C04', 'C05'].map((caseId) => [caseId, { mode: 'concrete_live', blocker: null }])),
  );
  const entries = createP158W9FreezeAdapterEntries({
    schedule, bundle, liveHookManifestSha256: '66'.repeat(32),
  });
  assert.equal(entries.adapters.length, 5);
  assert.equal(entries.adapterBindings.length, 5);
  assert.ok(entries.adapterBindings.every((entry) => entry.mode === 'concrete_live' &&
    entry.effectsAllowed === true && entry.blockedActionCount === 0));
  assert.deepEqual(entries.adapterBindings.map((entry) => entry.implementedActionCount),
    [560, 2620, 25, 12250, 500]);
  assert.deepEqual(p158W9HookManifestEntries().map((entry) => entry.hookId), P158_W9_MANIFEST_HOOK_IDS);
  assert.ok(p158W9HookManifestEntries().every((entry) =>
    entry.implementationKind === 'concrete_live' && /^[a-f0-9]{64}$/u.test(entry.sourceSha256)));
});

await runTest('never promotes injected provider-free drivers into live adapter readiness', async () => {
  const selectedFetch = async (url) => ({ ok: true, status: 200, url, json: async () => ({ success: true, data: {} }) });
  const bundle = makeBundle({ testing: true, fetchOverride: selectedFetch });
  assert.equal(bundle.freezeEligible, false);
  assert.equal(bundle.c01FetchSource, 'supplied');
  const entries = createP158W9FreezeAdapterEntries({ schedule, bundle, liveHookManifestSha256: '66'.repeat(32) });
  assert.ok(entries.adapterBindings.every((entry) => entry.mode === 'explicit_blocked' &&
    entry.effectsAllowed === false && entry.implementedActionCount === 0 &&
    entry.blocker.code === 'provider_free_test_driver'));
  let effects = 0;
  const outcome = await entries.adapters[0].execute({
    attempt: schedule.attempts.find((attempt) => attempt.caseId === 'C01'),
    requestEffect: async () => { effects += 1; },
  });
  assert.equal(outcome.resultState, 'skipped_blocked');
  assert.equal(effects, 0);
});

await runTest('rejects every unreviewed or non-development E1/E2 target variant', async () => {
  const variants = [
    (value) => { value.serviceOrigins.E1 = 'https://service.p158.example'; },
    (value) => { value.reviewedLocalDevelopmentOrigin = 'http://127.0.0.1:19102'; },
    (value) => { value.serviceOrigins.E2 = 'http://service.p158.example'; },
    (value) => { value.serviceOrigins.E2 = 'https://127.0.0.1:19101'; },
    (value) => { value.serviceResolvedAddresses.E2 = ['10.0.0.7']; },
    (value) => { value.allowedExternalServiceOrigins = ['https://other.p158.example']; },
    (value) => { value.productionHostnames = ['service.p158.example']; },
    (value) => { value.syntheticTarget = false; },
    (value) => { value.reviewedOriginBindingSha256 = 'ff'.repeat(32); },
  ];
  for (const mutate of variants) {
    const invalid = target();
    mutate(invalid);
    // Rebind all but the explicit stale-digest case. Structural checks must
    // still reject a self-consistent caller assertion.
    if (invalid.reviewedOriginBindingSha256 !== 'ff'.repeat(32)) {
      invalid.reviewedOriginBindingSha256 = canonicalP158W9TargetBindingDigest(invalid);
    }
    assert.throws(
      () => makeBundle({ testing: true, targetOverride: invalid }),
      (error) => error.code === 'development_target_unproven',
    );
  }
});

await runTest('classifies a missing external action as exact zero-effect blocked case', async () => {
  const missing = actions.find((action) => action.caseId === 'C05' && action.kind === 'handoff_reconnect');
  const bundle = makeBundle({ testing: false, omitActionId: missing.actionId });
  assert.equal(bundle.classification.get('C05').mode, 'explicit_blocked');
  assert.match(bundle.classification.get('C05').blocker.detail, new RegExp(missing.actionId));
  const entries = createP158W9FreezeAdapterEntries({ schedule, bundle, liveHookManifestSha256: '66'.repeat(32) });
  const binding = entries.adapterBindings.find((entry) => entry.caseId === 'C05');
  assert.equal(binding.implementedActionCount, 0);
  assert.equal(binding.blockedActionCount, 500);
  assert.equal(binding.effectsAllowed, false);
});

await runTest('classifies an unbound C01 live driver as explicit blocked', async () => {
  const prepared = plans();
  const invalidC01 = c01();
  delete invalidC01.externalAggregatePath;
  const bundle = createP158W9ConcreteDriverBundle({
    schedule, target: target(), artifactStore: createMemoryArtifactStore(),
    externalWorkflowPlan: prepared.external, declaredTransitionPlan: prepared.transition,
    c01: invalidC01,
  });
  assert.equal(bundle.classification.get('C01').mode, 'explicit_blocked');
  assert.equal(bundle.classification.get('C01').blocker.detail, 'distributed_c01_live_driver');
});

await runTest('executes concrete service, external receipt, and declared transition seams once', async () => {
  const bundle = makeBundle({ testing: true });
  const service = bundle.actions.find((action) => action.caseId === 'C02' && action.kind === 'service_command');
  const dashboard = bundle.actions.find((action) => action.caseId === 'C02' && action.kind === 'dashboard_action');
  const crash = bundle.actions.find((action) => action.caseId === 'C02' && action.kind === 'declared_browser_crash');
  const receipts = [
    await bundle.drivers.executeServiceCommand(service),
    await bundle.drivers.executeExternalDashboardAction(dashboard),
    await bundle.drivers.executeDeclaredBrowserCrash(crash),
  ];
  assert.deepEqual(receipts.map((receipt) => receipt.effectClass),
    ['read_only', 'external_ingress', 'declared_fault']);
  assert.ok(receipts.every((receipt) => receipt.receiptSha256.length === 64 &&
    receipt.retryAttempted === false && receipt.repairAttempted === false &&
    receipt.garbageCollectionAttempted === false));
  assert.equal(await bundle.drivers.verifyEvidenceArtifact(receipts[0].evidenceArtifactIds[0]), true);
  const teardown = await bundle.drivers.executeScheduledTeardown();
  assert.equal(teardown.effectClass, 'scheduled_teardown');
  assert.equal(await bundle.drivers.verifyEvidenceArtifact(teardown.evidenceArtifactIds[0]), true);
});

process.stdout.write('P158 W9 concrete driver bundle test passed\n');
