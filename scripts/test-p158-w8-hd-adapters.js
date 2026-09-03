#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  P158_W8_CASE_IDS,
  P158_W8_ERROR_CODES,
  P158_W8_LIVE_HOOK_GAPS,
  P158_W8_REVIEWED_SOURCE_COVERAGE,
  P158W8AdapterError,
  assessP158W8ReviewedLiveSources,
  buildP158W8ExternalActionManifest,
  buildP158W8ActionPlan,
  createP158W8AdapterBundle,
  createP158W8ReviewedLiveAdapterBundle,
  sealP158W8Receipt,
} from './lib/p158-w8-hd-adapters.js';
import {
  EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
  aggregateExternalVantageReceipts,
  canonicalHash,
  executeP158W8ExternalActionManifest,
} from './run-p158-external-vantage.js';
import { RETAINED_IDENTITY_FIELDS } from './lib/p158-external-handoff-oracle.js';
import { generateDenseDashboardFixture } from './lib/p158-dashboard-oracle.js';
import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';

const readJson = (path) => JSON.parse(readFileSync(new URL(`../${path}`, import.meta.url), 'utf8'));
const registry = readJson('docs/dev/contracts/p158-historical-failure-registry.v1.json');
const dashboardCorpus = readJson('docs/dev/fixtures/p158/dashboard-oracle-fixtures.v1.json');
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w8-provider-free-seed' });
const digest = (label) => sha256(`p158-w8:${label}`);
const expectedIdentity = Object.fromEntries(RETAINED_IDENTITY_FIELDS.map((field) => [field, `${field}-synthetic-v1`]));
expectedIdentity.pixelHash = digest('pixel-marker');
const seals = {
  freezeState: 'frozen',
  scheduleSha256: schedule.scheduleSha256,
  registrySha256: schedule.registrySha256,
  candidateSha256: digest('candidate'),
  workflowSha256: digest('workflow'),
  handoffUrlSha256: digest('handoff-url'),
  externalVantageReceiptSha256: digest('external-vantage'),
  externalHandoffOracleReportSha256: digest('external-oracle'),
  fixtureRedactionReceiptSha256: digest('fixture-redaction'),
  fixtureId: 'p158-synthetic-visual-v1',
  expectedIdentity,
};
const liveHookManifestSha256 = digest('live-hook-manifest');
const denseFixture = generateDenseDashboardFixture({
  generatorVersion: 'p158-dashboard-dense.v1',
  seed: 158,
  profiles: 100,
  browsers: 500,
  tabs: 2000,
  jobs: 10000,
  events: 10000,
  idNamespace: 'p158-w8-dense',
  labelCardinality: 8,
});
denseFixture.timings = structuredClone(dashboardCorpus.baseline.timings);
denseFixture.resourceSamples = structuredClone(dashboardCorpus.baseline.resourceSamples);
denseFixture.resourceSlopeBudgets = structuredClone(dashboardCorpus.baseline.resourceSlopeBudgets);

function observedCardinalities(request) {
  return Object.fromEntries(Object.entries(request.cardinalities).map(([id, value]) => [id, value.value]));
}

function commonReceipt(request, extra = {}) {
  return sealP158W8Receipt({
    schemaVersion: 'agent-browser.p158-w8-hook-receipt.v1',
    actionId: request.actionId,
    attemptId: request.attemptId,
    caseId: request.caseId,
    candidateSha256: seals.candidateSha256,
    workflowSha256: seals.workflowSha256,
    terminalState: 'completed',
    scenarioOraclePassed: true,
    attemptNumber: 1,
    repairAttempted: false,
    retryAttempted: false,
    gcAttempted: false,
    scheduledOffsetSeconds: request.plannedOffsetSeconds,
    observedDurationSeconds: request.caseId === 'D11' ? 28800 : 0,
    contentClass: request.contentClass,
    operatorGateArtifactSha256: request.operatorGateArtifactSha256,
    fixtureId: seals.fixtureId,
    fixtureRedactionReceiptSha256: seals.fixtureRedactionReceiptSha256,
    capture: {
      credentialsCaptured: false,
      secretInputCaptured: false,
      privateContentCaptured: false,
    },
    observedCardinalities: observedCardinalities(request),
    ...extra,
  });
}

function externalReceipt(request, override = {}) {
  return commonReceipt(request, {
    fixtureId: seals.fixtureId,
    fixtureRedactionReceiptSha256: seals.fixtureRedactionReceiptSha256,
    externalIngress: {
      vantage: 'off_host',
      passed: true,
      externalVantageReceiptSha256: seals.externalVantageReceiptSha256,
      externalHandoffOracleReportSha256: seals.externalHandoffOracleReportSha256,
      handoffUrlSha256: seals.handoffUrlSha256,
      operatorVisibleState: 'ready',
      readyObservedAt: '2026-09-03T00:00:00.000Z',
      firstUsablePixelsAt: '2026-09-03T00:00:00.100Z',
      identity: expectedIdentity,
      ...override,
    },
  });
}

function dashboardReceipt(
  request,
  fixture = request.caseId === 'D09' ? denseFixture : dashboardCorpus.baseline,
) {
  return commonReceipt(request, {
    snapshotBarrierId: `${request.actionId}:barrier`,
    renderedBarrierId: `${request.actionId}:barrier`,
    authoritativeSnapshotSha256: digest(`${request.actionId}:snapshot`),
    dashboardFixture: structuredClone(fixture),
  });
}

const calls = { external: [], playwright: [], dashboard: [], capture: [], stimulus: [] };
const hooks = {
  externalWorkflow: {
    execute: async (request) => {
      calls.external.push(structuredClone(request));
      return externalReceipt(request);
    },
  },
  playwright: {
    execute: async (request) => {
      calls.playwright.push(structuredClone(request));
      return commonReceipt(request, {
        fixtureId: seals.fixtureId,
        fixtureRedactionReceiptSha256: seals.fixtureRedactionReceiptSha256,
      });
    },
  },
  dashboard: {
    execute: async (request) => {
      calls.dashboard.push(structuredClone(request));
      return dashboardReceipt(request);
    },
    capture: async (request) => {
      calls.capture.push(structuredClone(request));
      return dashboardReceipt(request);
    },
  },
  stimulus: {
    schedule: async (request) => {
      calls.stimulus.push(structuredClone(request));
      return commonReceipt(request, { stimulus: { ...request.stimulus, scheduled: true } });
    },
  },
};

async function expectCode(code, operation) {
  await assert.rejects(operation, (error) => {
    assert(error instanceof P158W8AdapterError);
    assert.equal(error.code, code);
    return true;
  });
}

assert.deepEqual(P158_W8_CASE_IDS, [
  'H01', 'H02', 'H03', 'H04', 'H05', 'H06', 'H07', 'H08', 'H09', 'H10', 'H11', 'H12',
  'D01', 'D02', 'D03', 'D04', 'D05', 'D06', 'D07', 'D08', 'D09', 'D10', 'D11', 'D12',
]);
assert.equal(new Set(P158_W8_ERROR_CODES).size, P158_W8_ERROR_CODES.length);
const originalRegistry = structuredClone(registry);
const originalSchedule = structuredClone(schedule);
const originalSeals = structuredClone(seals);
const bundle = createP158W8AdapterBundle({ registry, schedule, seals, hooks });
assert.equal(bundle.adapterCount, 24);
assert.equal(Object.keys(bundle.effects).length, 24);
assert.equal(bundle.reactionaryRepairAllowed, false);
assert.equal(bundle.opportunisticRetryAllowed, false);
assert.equal(bundle.undeclaredGcAllowed, false);
assert.deepEqual(registry, originalRegistry);
assert.deepEqual(schedule, originalSchedule);
assert.deepEqual(seals, originalSeals);

const cases = new Map(registry.cases.map((entry) => [entry.id, entry]));
const adapters = new Map(bundle.adapters.map((entry) => [entry.caseId, entry]));
let expectedExternalActions = 0;
let expectedPlaywrightActions = 0;
let expectedDashboardActions = 0;
let expectedDashboardCaptures = 0;
let expectedStimuli = 0;
for (const attempt of schedule.attempts.filter((entry) => P158_W8_CASE_IDS.includes(entry.caseId))) {
  const testCase = cases.get(attempt.caseId);
  const plan = buildP158W8ActionPlan({ testCase, attempt });
  assert.equal(new Set(plan.actions.map((action) => action.actionId)).size, plan.actionCount);
  assert(plan.actions.every((action) => action.attemptId === attempt.attemptId));
  assert(plan.actions.every((action) => action.externalIngressRequired === attempt.externalIngressRequired));
  expectedExternalActions += plan.actions.filter((action) => action.externalIngressRequired).length;
  expectedPlaywrightActions += plan.actions.filter((action) => !action.externalIngressRequired && action.surface === 'human_remote_view').length;
  expectedDashboardActions += plan.actions.filter((action) => !action.externalIngressRequired && action.surface === 'dashboard').length;
  expectedDashboardCaptures += plan.actions.filter((action) => action.externalIngressRequired && action.surface === 'dashboard').length;
  expectedStimuli += plan.actions.filter((action) => action.stimulus).length;
  const adapter = adapters.get(attempt.caseId);
  const result = await adapter.execute({
    attempt: structuredClone(attempt),
    requestEffect: async (effectId, payload) => bundle.effects[effectId](payload, structuredClone(attempt)),
  });
  assert.equal(result.resultState, 'passed');
  assert.equal(result.evidence.actionCount, plan.actionCount);
  assert.equal(result.evidence.repairAttempted, false);
  assert.equal(result.evidence.retryAttempted, false);
  assert.equal(result.evidence.gcAttempted, false);
}
assert.equal(calls.external.length, expectedExternalActions);
assert.equal(calls.playwright.length, expectedPlaywrightActions);
assert.equal(calls.dashboard.length, expectedDashboardActions);
assert.equal(calls.capture.length, expectedDashboardCaptures);
assert.equal(calls.stimulus.length, expectedStimuli);
assert(calls.external.every((request) => request.environmentIds.includes('E2')));
assert(calls.external.every((request) => request.handoffUrlSha256 === seals.handoffUrlSha256));
assert(calls.external.every((request) => request.candidateSha256 === seals.candidateSha256));
assert(calls.external.every((request) => request.workflowSha256 === seals.workflowSha256));
assert(calls.dashboard.every((request) => !request.environmentIds.includes('E2')));
assert(calls.capture.every((request) => request.environmentIds.includes('E2')));
assert(calls.stimulus.every((request) => request.stimulus?.dimensionId && request.stimulus?.kind));
assert.deepEqual(
  [...new Set(calls.stimulus.map((request) => `${request.caseId}:${request.stimulus.dimensionId}`))].sort(),
  [
    'D02:resource_transition', 'D07:response_fault', 'D09:stream_state',
    'H03:rebind_transition', 'H05:controller_transfer', 'H07:route_request',
    'H08:failure_injection', 'H09:network_profile', 'H10:disruption_transition',
    'H12:lease_or_client_state',
  ],
);

const h02E2 = schedule.attempts.find((entry) => entry.attemptId === 'H02-E2-r001');
const h02Plan = buildP158W8ActionPlan({ testCase: cases.get('H02'), attempt: h02E2 });
assert.equal(h02Plan.actionCount, 130);
assert.equal(new Set(h02Plan.actions.map((action) => `${action.assignment.url_role}:${action.assignment.host_or_scheme}`)).size, 130);
const d06E2 = schedule.attempts.find((entry) => entry.attemptId === 'D06-E2-r001');
assert.equal(buildP158W8ActionPlan({ testCase: cases.get('D06'), attempt: d06E2 }).actionCount, 8);
const d10E2 = schedule.attempts.find((entry) => entry.attemptId === 'D10-E2-r001');
assert.equal(buildP158W8ActionPlan({ testCase: cases.get('D10'), attempt: d10E2 }).actionCount, 28);
const d12E2 = schedule.attempts.find((entry) => entry.attemptId === 'D12-E2-r001');
assert.equal(buildP158W8ActionPlan({ testCase: cases.get('D12'), attempt: d12E2 }).actionCount, 21);
const h12 = schedule.attempts.filter((entry) => entry.caseId === 'H12');
assert.equal(h12.length, 500);
assert.deepEqual(
  h12.slice(0, 4).map((attempt) => buildP158W8ActionPlan({ testCase: cases.get('H12'), attempt }).actions[0].assignment.lease_or_client_state),
  ['client_restart', 'viewer_lease_expired', 'controller_lease_expired', 'client_restart'],
);
assert.equal(h12.at(-1).executionUnit.plannedOffsetSeconds, 86400);

const h11Attempt = schedule.attempts.find((entry) => entry.caseId === 'H11');
const defaultH11 = buildP158W8ActionPlan({ testCase: cases.get('H11'), attempt: h11Attempt });
assert(defaultH11.actions.every((action) => action.contentClass === 'synthetic'));
assert(defaultH11.actions.every((action) => action.secureSurfaceMode === 'synthetic_fixture'));
assert.throws(
  () => createP158W8AdapterBundle({ registry, schedule, seals, hooks, operatorAssisted: { enabled: true } }),
  (error) => error.code === 'operator_gate_missing',
);
const gateArtifact = {
  artifactId: 'p158-h11-nonproduction-operator-gate',
  mode: 'nonproduction_operator_assisted',
  approved: true,
  freezeEligible: true,
  candidateSha256: seals.candidateSha256,
  workflowSha256: seals.workflowSha256,
  handoffUrlSha256: seals.handoffUrlSha256,
  secretsCaptured: false,
  vaultContentCaptured: false,
  credentialInputCaptured: false,
  artifactSha256: digest('operator-gate'),
};
const assisted = createP158W8AdapterBundle({
  registry, schedule, seals, hooks,
  operatorAssisted: { enabled: true, gateArtifact },
});
assert.equal(assisted.operatorAssistedReady, true);
const assistedH11 = buildP158W8ActionPlan({
  testCase: cases.get('H11'), attempt: h11Attempt,
  operatorAssisted: { enabled: true, gateArtifactSha256: gateArtifact.artifactSha256 },
});
assert.deepEqual(
  assistedH11.actions.filter((action) => action.secureSurfaceMode === 'operator_assisted').map((action) => action.assignment.secure_fixture),
  ['nonproduction_lastpass_vault', 'test_passkey_relying_party'],
);

assert.throws(
  () => createP158W8AdapterBundle({ registry, schedule, seals: { ...seals, freezeState: 'prepared' }, hooks }),
  (error) => error.code === 'frozen_seal_invalid',
);
assert.throws(
  () => createP158W8AdapterBundle({ registry, schedule, seals, hooks: { ...hooks, externalWorkflow: {} } }),
  (error) => error.code === 'hook_missing',
);

const probeAction = h02Plan.actions[0];
async function executeProbeWith(receiptFactory, kind = 'external') {
  const localHooks = structuredClone({});
  Object.assign(localHooks, hooks, { externalWorkflow: { execute: async (request) => receiptFactory(request) } });
  const localBundle = createP158W8AdapterBundle({ registry, schedule, seals, hooks: localHooks });
  const plan = { ...h02Plan, actionCount: 1, actions: [probeAction] };
  return localBundle.effects['p158.effect.H02.declared'](
    { plan, planSha256: sha256(plan) },
    h02E2,
  );
}

await expectCode('receipt_binding_mismatch', () => executeProbeWith((request) => externalReceipt({ ...request, actionId: 'wrong-action' })));
await expectCode('receipt_invalid', () => executeProbeWith((request) => commonReceipt(request, {
  retryAttempted: true,
  externalIngress: externalReceipt(request).externalIngress,
})));
await expectCode('private_content_prohibited', () => executeProbeWith((request) => commonReceipt(request, {
  contentClass: 'production_private',
  externalIngress: externalReceipt(request).externalIngress,
})));
await expectCode('unsafe_url_prohibited', () => executeProbeWith((request) => commonReceipt(request, {
  evidenceUrl: 'http://127.0.0.1:9222/internal',
  externalIngress: externalReceipt(request).externalIngress,
})));
await expectCode('handoff_digest_mismatch', () => executeProbeWith((request) => externalReceipt(request, {
  handoffUrlSha256: digest('wrong-handoff'),
})));
await expectCode('ready_before_pixels_unproven', () => executeProbeWith((request) => externalReceipt(request, {
  readyObservedAt: '2026-09-03T00:00:01.000Z',
  firstUsablePixelsAt: '2026-09-03T00:00:00.000Z',
})));
await expectCode('identity_mismatch', () => executeProbeWith((request) => externalReceipt(request, {
  identity: { ...expectedIdentity, browserId: 'wrong-browser' },
})));

const d01E0 = schedule.attempts.find((entry) => entry.attemptId === 'D01-E0-r001');
const d01Plan = buildP158W8ActionPlan({ testCase: cases.get('D01'), attempt: d01E0 });
const brokenFixture = structuredClone(dashboardCorpus.baseline);
brokenFixture.railRows = [];
const badDashboardHooks = {
  ...hooks,
  dashboard: {
    ...hooks.dashboard,
    execute: async (request) => dashboardReceipt(request, brokenFixture),
  },
};
const badDashboardBundle = createP158W8AdapterBundle({ registry, schedule, seals, hooks: badDashboardHooks });
await expectCode('dashboard_oracle_failed', () => badDashboardBundle.effects['p158.effect.D01.declared'](
  { plan: d01Plan, planSha256: sha256(d01Plan) },
  d01E0,
));

const readinessInputs = { registry, schedule, seals };
const originalReadinessInputs = structuredClone(readinessInputs);
const reviewed = assessP158W8ReviewedLiveSources(readinessInputs);
assert.equal(reviewed.ready, false);
assert.deepEqual(reviewed.concreteCaseIds, []);
assert.deepEqual(reviewed.explicitlyBlockedCaseIds, P158_W8_CASE_IDS);
assert.equal(reviewed.blockerCount, 24);
assert.equal(reviewed.reviewedSourceCount, 10);
assert.equal(reviewed.effectsExecuted, false);
assert.equal(reviewed.scheduledActionCount, 1017);
assert.deepEqual(readinessInputs, originalReadinessInputs);
assert.deepEqual(Object.keys(P158_W8_LIVE_HOOK_GAPS).sort(), [...P158_W8_CASE_IDS].sort());
assert.deepEqual(
  reviewed.reviewedSources.map((entry) => entry.sourcePath).sort(),
  Object.values(P158_W8_REVIEWED_SOURCE_COVERAGE).map((entry) => entry.path).sort(),
);
for (const source of reviewed.reviewedSources) {
  assert.equal(source.sourceSha256, sha256(readFileSync(new URL(`../${source.sourcePath}`, import.meta.url))));
  assert(source.cases.length > 0);
  assert(source.coverage.length > 20);
  assert(source.missing.length > 20);
}
assert.match(
  reviewed.blockers.find((entry) => entry.caseId === 'H01').detail,
  /action_manifest_missing/,
);
assert.match(
  reviewed.blockers.find((entry) => entry.caseId === 'H11').detail,
  /operator_gate_missing/,
);
assert.match(
  reviewed.blockers.find((entry) => entry.caseId === 'D11').detail,
  /8_hour/,
);

const reviewedBundle = createP158W8ReviewedLiveAdapterBundle({
  ...readinessInputs,
  liveHookManifestSha256,
});
assert.equal(reviewedBundle.ready, true, 'explicitly blocked adapters are freeze-ready');
assert.equal(reviewedBundle.executionReady, false);
assert.equal(reviewedBundle.w8Adapters.length, 24);
assert.equal(Object.keys(reviewedBundle.effects).length, 0);
assert.equal(reviewedBundle.adapterBindings.length, 24);
assert(reviewedBundle.adapterBindings.every((binding) => binding.mode === 'explicit_blocked'));
assert(reviewedBundle.adapterBindings.every((binding) => binding.providerFree === false));
assert(reviewedBundle.adapterBindings.every((binding) => binding.effectsAllowed === false));
assert(reviewedBundle.adapterBindings.every((binding) => binding.implementedActionCount === 0));
assert(reviewedBundle.adapterBindings.every((binding) => binding.blockedActionCount > 0));
assert(reviewedBundle.adapterBindings.every((binding) => binding.blocker.code === 'live_case_hook_missing'));
for (const [index, adapter] of reviewedBundle.w8Adapters.entries()) {
  const binding = reviewedBundle.adapterBindings[index];
  assert.equal(adapter.executionMode, binding.mode);
  assert.equal(adapter.providerFree, false);
  assert.equal(adapter.effectsAllowed, binding.effectsAllowed);
  assert.equal(adapter.sourcePath, binding.sourcePath);
  assert.equal(adapter.sourceSha256, binding.sourceSha256);
  assert.equal(adapter.liveHookManifestSha256, liveHookManifestSha256);
  assert.equal(adapter.liveBindingSha256, sha256(binding));
  assert.deepEqual(adapter.liveHookIds, binding.hookIds);
}
assert.equal(
  reviewedBundle.adapterBindings.reduce((sum, binding) => sum + binding.blockedActionCount, 0),
  reviewed.scheduledActionCount,
);
assert.deepEqual(
  reviewedBundle.adapterBindings.find((binding) => binding.caseId === 'H01').hookIds,
  ['w8.external_workflow', 'w8.playwright'],
);
assert.deepEqual(
  reviewedBundle.adapterBindings.find((binding) => binding.caseId === 'D07').hookIds,
  ['w8.dashboard_capture', 'w8.dashboard_execute', 'w8.stimulus'],
);
let blockedEffectRequests = 0;
for (const adapter of reviewedBundle.w8Adapters) {
  const attempt = schedule.attempts.find((entry) => entry.caseId === adapter.caseId);
  const result = await adapter.execute({
    attempt,
    requestEffect: async () => {
      blockedEffectRequests += 1;
      throw new Error('blocked W8 adapter attempted an effect');
    },
  });
  assert.equal(result.resultState, 'skipped_blocked');
  assert.equal(result.effectState, 'not_started');
  assert.equal(result.retryDisposition, 'prohibited_opportunistic_retry');
  assert.equal(result.repairAttempted, false);
  assert.equal(result.retryAttempted, false);
  assert.equal(result.garbageCollectionAttempted, false);
  assert.equal(result.blocker.detail, P158_W8_LIVE_HOOK_GAPS[adapter.caseId]);
  assert.match(result.blocker.sourceSha256, /^[a-f0-9]{64}$/);
}
assert.equal(blockedEffectRequests, 0);
assert.throws(
  () => assessP158W8ReviewedLiveSources({
    registry, schedule, seals, operatorAssisted: { enabled: true },
  }),
  (error) => error instanceof P158W8AdapterError && error.code === 'operator_gate_missing',
);

const publicHandoffUrl = 'https://handoff.public.example/remote-view/p158-test';
const externalSeals = { ...seals, handoffUrlSha256: sha256(publicHandoffUrl) };
const externalManifest = buildP158W8ExternalActionManifest({
  registry,
  schedule,
  seals: externalSeals,
});
assert.deepEqual(externalManifest.caseIds, ['H01']);
assert.equal(externalManifest.actionCount, 4);
assert.equal(new Set(externalManifest.actions.map((action) => action.actionId)).size, 4);
assert.deepEqual(
  [...new Set(externalManifest.actions.map((action) => action.executorKind))].sort(),
  ['external_vantage_aggregate_projection'],
);
const sourceReceipts = ['external-runner-human', 'external-runner-slow'].map((clientId, clientIndex) => ({
  schemaVersion: EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
  planId: 'P158',
  runId: 'p158-w8-live-test',
  mode: 'readiness',
  clientId,
  paceProfile: clientIndex === 0 ? 'human_controller' : 'slow_concurrency',
  success: true,
  repairAttempted: false,
  retryCount: 0,
  runner: { runnerIdentitySha256: digest(`runner-${clientIndex}`) },
  handoff: { urlSha256: externalSeals.handoffUrlSha256 },
  expectedIdentity: externalSeals.expectedIdentity,
  initialIdentity: externalSeals.expectedIdentity,
  reconnectIdentity: externalSeals.expectedIdentity,
  serverPhysicalBrowserLaunchDelta: 0,
  internalUrlLeakCount: 0,
  ingressChecks: ['dns', 'tls', 'redirect', 'cookie', 'websocket', 'iframe', 'form_action', 'reconnect']
    .map((kind) => ({ kind, state: 'passed' })),
  artifacts: [
    { relativePath: 'network.redacted.har', sha256: digest(`${clientId}-har`), bytes: 1 },
    { relativePath: 'initial.png', sha256: digest(`${clientId}-png`), bytes: 1 },
    { relativePath: 'video/initial.webm', sha256: digest(`${clientId}-video`), bytes: 1 },
  ],
  visualFixtureAttestation: {
    fixtureId: 'p158-synthetic-visual-v1',
    syntheticOnly: true,
    forbiddenPrivateFieldsExcluded: true,
    redactionReceiptSha256: digest(`${clientId}-redaction`),
  },
  oracle: { passed: true },
  w8ActionManifestSha256: externalManifest.manifestSha256,
  w8ActionObservations: externalManifest.actions.map((action, actionIndex) => ({
      actionId: action.actionId,
      attemptId: action.attemptId,
      caseId: action.caseId,
      runnerAction: action.assignment.runner_action,
      clientId,
      viewerId: `viewer-${clientIndex + 1}`,
      observedAt: `2026-09-03T01:00:0${actionIndex}.000Z`,
      eventKind: ['page_open_ready', 'human_paced_interaction_completed', 'playwright_page_closed', 'same_handoff_reopened_ready'][actionIndex],
      evidenceArtifactId: `artifact-${clientIndex + 1}-${actionIndex + 1}`,
      handoffContinuityObserved: true,
      retainedIdentityObserved: true,
      retryAttempted: false,
      repairAttempted: false,
  })),
}));
const aggregate = aggregateExternalVantageReceipts(sourceReceipts, { runId: 'p158-w8-live-test' });
const externalResult = executeP158W8ExternalActionManifest({
  manifest: externalManifest,
  externalVantageAggregate: aggregate,
  publicHandoffUrl,
  observedAt: '2026-09-03T01:00:00.000Z',
});
assert.equal(externalResult.actionCount, 4);
assert.equal(new Set(externalResult.actionReceipts.map((receipt) => receipt.actionId)).size, 4);
assert(externalResult.actionReceipts.every((receipt) => receipt.resultState === 'passed'));
assert(externalResult.actionReceipts.every((receipt) => receipt.attemptNumber === 1));
assert(externalResult.actionReceipts.every((receipt) => receipt.repairAttempted === false));
assert(externalResult.actionReceipts.every((receipt) => receipt.retryAttempted === false));
assert(externalResult.actionReceipts.every((receipt) => receipt.garbageCollectionAttempted === false));
const h01Receipts = externalResult.actionReceipts.filter((receipt) => receipt.caseId === 'H01');
assert.deepEqual(h01Receipts.map((receipt) => receipt.evidence.runnerAction), [
  'open', 'interact', 'disconnect', 'reopen',
]);
assert(h01Receipts.every((receipt) => receipt.evidence.clientIds.length === 2));
assert(h01Receipts.every((receipt) => receipt.evidence.observations.length === 2));
assert.throws(
  () => executeP158W8ExternalActionManifest({
    manifest: { ...externalManifest, actionCount: 1 },
    externalVantageAggregate: aggregate,
    publicHandoffUrl,
  }),
  /manifest is missing, changed/,
);
const resultRoot = mkdtempSync(join(tmpdir(), 'p158-w8-external-result-'));
const resultPath = join(resultRoot, 'result.json');
const aggregatePath = join(resultRoot, 'aggregate.json');
const receiptPaths = sourceReceipts.map((_, index) => join(resultRoot, `receipt-${index}.json`));
writeFileSync(resultPath, `${JSON.stringify(externalResult)}\n`);
writeFileSync(aggregatePath, `${JSON.stringify(aggregate)}\n`);
sourceReceipts.forEach((receipt, index) => writeFileSync(receiptPaths[index], `${JSON.stringify(receipt)}\n`));
try {
  const concreteBundle = createP158W8ReviewedLiveAdapterBundle({
    registry,
    schedule,
    seals: externalSeals,
    liveHookManifestSha256,
    externalActionExecution: { manifest: externalManifest, resultPath, aggregatePath, receiptPaths },
  });
  assert.equal(concreteBundle.executionReady, true);
  assert.deepEqual(
    concreteBundle.adapterBindings.filter((binding) => binding.mode === 'concrete_live')
      .map((binding) => binding.caseId),
    ['H01'],
  );
  assert.equal(concreteBundle.adapterBindings.find((binding) => binding.caseId === 'H01').implementedActionCount, 4);
  assert.equal(concreteBundle.adapterBindings.find((binding) => binding.caseId === 'H02').mode, 'explicit_blocked');
  assert.equal(concreteBundle.adapterBindings.filter((binding) => binding.mode === 'explicit_blocked').length, 23);
  const concreteAdapters = new Map(concreteBundle.w8Adapters.map((adapter) => [adapter.caseId, adapter]));
  for (const attempt of schedule.attempts.filter((entry) => entry.caseId === 'H01')) {
    const adapter = concreteAdapters.get(attempt.caseId);
    const outcome = await adapter.execute({
      attempt,
      requestEffect: (effectId, payload) => concreteBundle.effects[effectId](payload),
    });
    assert.equal(outcome.resultState, 'passed');
    assert.equal(outcome.actionCount, outcome.actionIds.length);
    assert.equal(outcome.effectState, 'verified_effect');
  }
  const blockedD01 = await concreteAdapters.get('D01').execute({
    attempt: schedule.attempts.find((entry) => entry.caseId === 'D01'),
    requestEffect: async () => assert.fail('blocked D01 requested an effect'),
  });
  assert.equal(blockedD01.resultState, 'skipped_blocked');
  const repeatedH01 = schedule.attempts.find((entry) => entry.caseId === 'H01');
  await assert.rejects(
    () => concreteAdapters.get('H01').execute({
      attempt: repeatedH01,
      requestEffect: (effectId, payload) => concreteBundle.effects[effectId](payload),
    }),
    (error) => error instanceof P158W8AdapterError && error.code === 'external_action_result_invalid',
  );
  writeFileSync(receiptPaths[0], `${JSON.stringify({
    ...sourceReceipts[0],
    initialIdentity: { ...sourceReceipts[0].initialIdentity, browserId: 'forged-browser' },
  })}\n`);
  const forgedBundle = createP158W8ReviewedLiveAdapterBundle({
    registry,
    schedule,
    seals: externalSeals,
    liveHookManifestSha256,
    externalActionExecution: { manifest: externalManifest, resultPath, aggregatePath, receiptPaths },
  });
  const forgedAttempt = schedule.attempts.find((entry) => entry.caseId === 'H01');
  await assert.rejects(
    () => forgedBundle.w8Adapters.find((entry) => entry.caseId === 'H01').execute({
      attempt: forgedAttempt,
      requestEffect: (effectId, payload) => forgedBundle.effects[effectId](payload),
    }),
    (error) => error instanceof P158W8AdapterError && error.code === 'external_action_result_invalid',
  );
} finally {
  rmSync(resultRoot, { recursive: true, force: true });
}

const dashboardResultRoot = mkdtempSync(join(tmpdir(), 'p158-w8-dashboard-result-'));
const dashboardResultPath = join(dashboardResultRoot, 'result.json');
const dashboardCampaignPlanSha256 = digest('dashboard-campaign-plan');
const dashboardActions = schedule.attempts.filter((attempt) => attempt.caseId === 'D01')
  .flatMap((attempt) => buildP158W8ActionPlan({ testCase: cases.get(attempt.caseId), attempt }).actions);
const dashboardCampaignReceipts = dashboardActions.map((action) => {
  const counts = {
    empty: { profiles: 0, browsers: 0, tabs: 0, jobs: 0, events: 0 },
    sparse: { profiles: 2, browsers: 5, tabs: 20, jobs: 100, events: 100 },
    normal: { profiles: 10, browsers: 50, tabs: 200, jobs: 1000, events: 1000 },
    dense: { profiles: 100, browsers: 500, tabs: 2000, jobs: 10000, events: 10000 },
  }[action.assignment.inventory_density];
  const actionFixture = generateDenseDashboardFixture({
    ...counts,
    idNamespace: `p158-w8-${action.assignment.inventory_density}`,
  });
  actionFixture.density = action.assignment.inventory_density;
  actionFixture.timings = structuredClone(dashboardCorpus.baseline.timings);
  actionFixture.resourceSamples = structuredClone(dashboardCorpus.baseline.resourceSamples);
  actionFixture.resourceSlopeBudgets = structuredClone(dashboardCorpus.baseline.resourceSlopeBudgets);
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-action-receipt.v1',
    planId: 'P158',
    actionId: action.actionId,
    attemptId: action.attemptId,
    caseId: action.caseId,
    candidateSha256: seals.candidateSha256,
    projection: {
      authoritativeSnapshotSha256: digest(`snapshot:${action.actionId}`),
      density: action.assignment.inventory_density,
    },
    dashboardFixture: actionFixture,
    oracleBinding: { passed: true, reportSha256: digest(`oracle:${action.actionId}`) },
    teardown: { state: 'stopped', pid: 1000 + action.ordinal },
    terminalState: 'completed',
    resultState: 'passed',
    productionStateTouched: false,
    repairAttempted: false,
    retryAttempted: false,
  };
  return { ...body, receiptSha256: sha256(body) };
});
const dashboardAggregateBody = {
  schemaVersion: 'agent-browser.p158-dashboard-campaign-aggregate.v1',
  planId: 'P158',
  campaignPlanSha256: dashboardCampaignPlanSha256,
  candidateSha256: seals.candidateSha256,
  actionCount: dashboardCampaignReceipts.length,
  actionIds: dashboardCampaignReceipts.map((receipt) => receipt.actionId).sort(),
  receiptSha256s: dashboardCampaignReceipts.map((receipt) => receipt.receiptSha256).sort(),
  resultCounts: { passed: dashboardCampaignReceipts.length, failed: 0 },
  success: true,
  repairAttempted: false,
  retryCount: 0,
};
const dashboardResult = {
  receipts: dashboardCampaignReceipts,
  aggregate: { ...dashboardAggregateBody, aggregateSha256: sha256(dashboardAggregateBody) },
};
writeFileSync(dashboardResultPath, `${JSON.stringify(dashboardResult)}\n`);
try {
  const dashboardBundle = createP158W8ReviewedLiveAdapterBundle({
    registry,
    schedule,
    seals,
    liveHookManifestSha256,
    dashboardCampaignExecution: {
      resultPath: dashboardResultPath,
      campaignPlanSha256: dashboardCampaignPlanSha256,
    },
  });
  assert.deepEqual(
    dashboardBundle.adapterBindings.filter((binding) => binding.mode === 'concrete_live')
      .map((binding) => binding.caseId),
    ['D01'],
  );
  assert.equal(dashboardBundle.reviewedLiveSources.blockerCount, 23);
  const adapters = new Map(dashboardBundle.w8Adapters.map((adapter) => [adapter.caseId, adapter]));
  for (const caseId of ['D01']) {
    const attempt = schedule.attempts.find((entry) => entry.caseId === caseId);
    const outcome = await adapters.get(caseId).execute({
      attempt,
      requestEffect: (effectId, payload) => dashboardBundle.effects[effectId](payload),
    });
    assert.equal(outcome.resultState, 'passed');
    assert.equal(outcome.effectState, 'verified_effect');
  }

  const failedResultPath = join(dashboardResultRoot, 'failed-result.json');
  const failedReceipts = structuredClone(dashboardCampaignReceipts);
  failedReceipts[0].resultState = 'harness_failure';
  writeFileSync(failedResultPath, `${JSON.stringify({ ...dashboardResult, receipts: failedReceipts })}\n`);
  const failedBundle = createP158W8ReviewedLiveAdapterBundle({
    registry,
    schedule,
    seals,
    dashboardCampaignExecution: {
      resultPath: failedResultPath,
      campaignPlanSha256: dashboardCampaignPlanSha256,
    },
  });
  const failedAttempt = schedule.attempts.find((entry) => entry.caseId === 'D01');
  await assert.rejects(
    () => failedBundle.w8Adapters.find((entry) => entry.caseId === 'D01').execute({
      attempt: failedAttempt,
      requestEffect: (effectId, payload) => failedBundle.effects[effectId](payload),
    }),
    (error) => error instanceof P158W8AdapterError && error.code === 'external_action_result_invalid',
  );
} finally {
  rmSync(dashboardResultRoot, { recursive: true, force: true });
}

process.stdout.write(`Plan 0158 W8 H/D adapters provider-free checks passed (${calls.external.length} external actions, ${reviewed.scheduledActionCount} reviewed live actions blocked)\n`);
