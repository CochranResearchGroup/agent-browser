#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  P158_W8_CASE_IDS,
  P158_W8_ERROR_CODES,
  P158W8AdapterError,
  buildP158W8ActionPlan,
  createP158W8AdapterBundle,
  sealP158W8Receipt,
} from './lib/p158-w8-hd-adapters.js';
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

process.stdout.write(`Plan 0158 W8 H/D adapters provider-free checks passed (${calls.external.length} external actions)\n`);
