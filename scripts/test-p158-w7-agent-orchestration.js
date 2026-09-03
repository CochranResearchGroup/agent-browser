#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { sha256 } from './lib/p158-campaign-controller.js';
import {
  createP158W7LiveDevelopmentAdapterBundle,
  enumerateP158W7ActionPlans,
} from './lib/p158-w7-development-adapters.js';
import {
  compileP158W7AgentOrchestration,
  createP158W7ExistingSeamDrivers,
  P158_W7_AGENT_ORCHESTRATION_CASE_IDS,
  P158W7OrchestrationError,
} from './lib/p158-w7-agent-orchestration.js';

const registry = JSON.parse(fs.readFileSync(new URL(
  '../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url,
), 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-agent-orchestration' });
const plans = enumerateP158W7ActionPlans({ schedule });
const target = Object.freeze({
  targetId: 'p158-agent-development-01',
  campaignRunId: 'p158-agent-run-01',
  runtimeLane: 'development',
  isolationState: 'isolated',
  ownership: 'p158_campaign',
  production: false,
  foreign: false,
  tenantDataPresent: false,
  disposableRoot: '/tmp/p158-agent-run-01',
  allowedHttpOrigins: ['https://service.dev.example', 'https://dashboard.dev.example', 'https://external.dev.example'],
  allowedExecutables: ['/opt/agent-browser-dev', '/usr/bin/ps'],
});

function value(action, id) {
  return action.dimensionAssignments.find((entry) => entry.dimensionId === id)?.value ?? null;
}

function steps(action) {
  const operations = {
    A01: ['acquire_session', 'acquire_tab', 'release_own_resources'],
    A02: ['join_retained_browser', 'create_attributable_tab', 'release_own_tab'],
    A03: ['command_targeting', 'tab_targeting', 'release_targeting'],
    A04: ['materialize_policy', value(action, 'operation'), 'assert_decision'],
    A05: ['barrier_arrive', value(action, 'transition'), 'barrier_release'],
    A06: ['barrier_arrive', 'revoke', value(action, 'eviction_mode'), 'barrier_release'],
    A08: ['materialize_identity_fixture', value(action, 'action'), 'assert_identity_result'],
    A09: ['create_target_pathology', value(action, 'target_pathology'), 'observe_target_result'],
    A10: ['stage_inventory', value(action, 'ownership'), value(action, 'inventory_state'), 'observe_inventory'],
    A15: ['navigate_marker', 'reconcile_history'],
  }[action.caseId];
  const transportSeam = {
    cli: 'history_cli', http: 'history_http', mcp: 'history_mcp',
    dashboard: 'history_dashboard', remote_control: 'history_external_remote_control',
  }[value(action, 'control_transport')] ?? 'history_cli';
  const serviceAction = (operation) => ({
    acquire_session: 'navigate', acquire_tab: 'tab_new', release_own_resources: 'tab_handle_release',
    join_retained_browser: 'tab_new', create_attributable_tab: 'tab_new', release_own_tab: 'tab_handle_release',
    command_targeting: 'navigate', tab_targeting: 'tab_switch', release_targeting: 'tab_close',
    materialize_policy: 'service_profile_policy_mutate', acquire: 'navigate', navigate: 'navigate',
    tab_create: 'tab_new', own_tab_release: 'tab_handle_release', policy_mutate: 'service_profile_policy_mutate',
    evict: 'service_profile_tab_evict', widen: 'service_profile_policy_mutate', narrow: 'service_profile_policy_mutate',
    admission: 'navigate', revision_conflict: 'service_profile_policy_mutate', drain_completion: 'service_profile_policy_mutate',
    revoke: 'service_profile_policy_mutate', graceful: 'service_profile_tab_evict', forced_exact: 'service_profile_tab_evict',
    materialize_identity_fixture: 'service_profile_upsert', launch: 'cdp_free_launch', remote_view_open: 'remote_view_open',
    tab_switch: 'tab_switch', view_focus: 'view_focus', assert_identity_result: 'diagnostics',
    navigate_marker: 'navigate', reconcile_history: 'diagnostics',
  })[operation] ?? 'service_browsers';
  return operations.map((operation, index) => {
    const seam = action.caseId === 'A15' && index === 0
      ? transportSeam
      : (operation.startsWith('barrier_') ? 'barrier'
          : (action.caseId === 'A09' ? 'browser_cli'
              : (action.caseId === 'A10' ? 'process_inventory'
                  : (['A04', 'A05', 'A06', 'A08'].includes(action.caseId)
                      ? 'profile_service' : 'service_http'))));
    const http = ['service_http', 'profile_service', 'history_http', 'history_dashboard',
      'history_external_remote_control', 'barrier'].includes(seam);
    return {
    stepId: `${action.actionId}:step:${index + 1}`,
    seam,
    operation,
    correlationIds: {
      campaignRunId: target.campaignRunId,
      attemptId: action.attemptId,
      actionId: action.actionId,
      stepId: `${action.actionId}:step:${index + 1}`,
    },
    request: { id: `${action.actionId}:${index + 1}`, action: 'frozen-existing-seam' },
    ...(http ? {
      url: seam === 'history_dashboard' ? 'https://dashboard.dev.example/api/service/request'
        : (seam === 'history_external_remote_control'
            ? 'https://external.dev.example/receipt' : 'https://service.dev.example/api/service/request'),
      body: { id: `${action.actionId}:${index + 1}`, action: serviceAction(operation) },
    } : { executable: '/opt/agent-browser-dev', args: ['--json', 'service', 'status'] }),
    };
  });
}

function binding(action, index) {
  const workflowKind = {
    A01: 'client_lifecycle', A02: 'shared_browser_lifecycle', A03: 'same_label_isolation',
    A04: 'acl_matrix', A05: 'acl_transition_barrier', A06: 'revocation_eviction_barrier',
    A08: 'identity_fixture_replay', A09: 'target_pathology', A10: 'inventory_adjacency',
    A15: 'history_marker',
  }[action.caseId];
  return {
    actionId: action.actionId,
    caseId: action.caseId,
    attemptId: action.attemptId,
    targetId: target.targetId,
    campaignRunId: target.campaignRunId,
    workflowKind,
    dispatchMode: ['A02', 'A03'].includes(action.caseId) ||
      (action.caseId === 'A01' && action.cardinalityId === 'concurrent_clients')
      ? 'concurrent' : 'serial',
    clientId: `client:${action.attemptId}:${index}`,
    connectionInstanceId: `connection:${action.attemptId}:${index}`,
    sharedLabel: 'same-label',
    sharedBrowserId: `browser:${action.attemptId}`,
    steps: steps(action),
    repair: false,
    retry: false,
    garbageCollect: false,
  };
}

const selectedPlans = plans.filter((action) => P158_W7_AGENT_ORCHESTRATION_CASE_IDS.includes(action.caseId));
const manifest = {
  schemaVersion: 'agent-browser.p158-w7-agent-workflows.v1',
  scheduleSha256: schedule.scheduleSha256,
  targetSha256: sha256(target),
  actions: selectedPlans.map(binding),
};
const originalManifest = structuredClone(manifest);
const driverCalls = [];
const drivers = createP158W7ExistingSeamDrivers({
  fetchImpl: async (url, init) => {
    driverCalls.push({ seam: 'http', url, body: init.body });
    await Promise.resolve();
    return { ok: true, redirected: false, status: 200, json: async () => ({ success: true }) };
  },
  execFile: async (executable, args) => {
    driverCalls.push({ seam: 'cli', executable, args });
    await Promise.resolve();
    return { stdout: '{"success":true}', stderr: '' };
  },
});

const compiled = compileP158W7AgentOrchestration({
  schedule, target, actionPlans: plans, manifest, drivers,
});
assert.deepEqual(manifest, originalManifest, 'compile must not mutate the frozen manifest input');
assert.deepEqual(compiled.concreteCaseIds, P158_W7_AGENT_ORCHESTRATION_CASE_IDS);
assert.deepEqual(compiled.blockedCaseIds, []);
assert.deepEqual(compiled.productBlockedCaseIds, ['A11', 'A12', 'A14']);
assert.equal(compiled.effectsExecuted, false);
assert.equal(compiled.freezeEligible, true);
assert.equal(compiled.driverSource.sourcePath, 'scripts/lib/p158-w7-agent-orchestration.js');
assert.match(compiled.driverSource.sourceSha256, /^[a-f0-9]{64}$/);
assert.equal(compiled.adapters.length, P158_W7_AGENT_ORCHESTRATION_CASE_IDS.length);

const injectedOnly = compileP158W7AgentOrchestration({
  schedule,
  target,
  actionPlans: plans,
  manifest,
  drivers: Object.fromEntries([...new Set(manifest.actions.flatMap((entry) =>
    entry.steps.map((step) => step.seam)))].map((seam) => [seam, async () => ({ success: true })])),
});
assert.equal(injectedOnly.freezeEligible, false);
assert.deepEqual(injectedOnly.concreteCaseIds, []);
assert(injectedOnly.blockers.every((entry) => entry.code === 'driver_bundle_not_freeze_eligible'));

const integratedTarget = {
    ...target,
    browserBindingsByActionId: Object.fromEntries(plans.filter((action) => action.caseId === 'A07')
      .map((action, index) => [action.actionId, {
        pid: 7000 + index,
        browserId: `a07-${index}`,
        profilePath: `${target.disposableRoot}/profiles/a07-${index}`,
      }])),
    daemonUnit: 'agent-browser-development.service',
    supervisorUnit: 'agent-browser-development-supervisor.service',
    evidenceSince: '2026-09-03T00:00:00Z',
    developmentBinary: '/opt/agent-browser-dev',
    agentBrowserId: 'agent-browser-development',
    agentProfilePath: `${target.disposableRoot}/profiles/agent`,
    runtimeProfile: 'p158-agent-run-01',
    sessionName: 'p158-agent-run-01',
    localFixtureOrigin: 'http://127.0.0.1:43158',
    desktopFixtureBindingsByActionId: Object.fromEntries(plans.filter((action) => action.caseId === 'X06')
      .map((action, index) => [action.actionId, {
        browserId: `x06-${index}`,
        profilePath: `${target.disposableRoot}/profiles/x06-${index}`,
        displayName: `:${200 + index}`,
        locatorId: 'p110-control-v1',
        windowState: value(action, 'window_state'),
      }])),
    allowedExecutables: ['/opt/agent-browser-dev', '/usr/bin/journalctl', '/usr/bin/ps', '/usr/bin/systemctl'],
    allowedSystemdUnits: ['agent-browser-development.service', 'agent-browser-development-supervisor.service'],
    allowedProcessIds: plans.filter((action) => action.caseId === 'A07').map((_, index) => 7000 + index),
    allowedBrowserIds: [
      'agent-browser-development',
      ...plans.filter((action) => action.caseId === 'A07').map((_, index) => `a07-${index}`),
      ...plans.filter((action) => action.caseId === 'X06').map((_, index) => `x06-${index}`),
    ],
    allowedProfilePaths: [
      `${target.disposableRoot}/profiles/agent`,
      ...plans.filter((action) => action.caseId === 'A07').map((_, index) => `${target.disposableRoot}/profiles/a07-${index}`),
      ...plans.filter((action) => action.caseId === 'X06').map((_, index) => `${target.disposableRoot}/profiles/x06-${index}`),
    ],
    allowedDisplayNames: plans.filter((action) => action.caseId === 'X06').map((_, index) => `:${200 + index}`),
};
const integrated = createP158W7LiveDevelopmentAdapterBundle({
  schedule,
  target: integratedTarget,
  primitives: {
    captureEvidence: async () => ({ artifactId: 'unused' }),
    captureLogs: async () => ({ artifactId: 'unused' }),
    executeProcess: async () => ({ resultState: 'passed' }),
    executeSystemd: async () => ({ resultState: 'passed' }),
    executeDisplay: async () => ({ resultState: 'passed' }),
  },
  agentWorkflowManifest: { ...manifest, targetSha256: sha256(integratedTarget) },
  agentWorkflowDrivers: drivers,
});
assert.equal(integrated.w7Adapters.length, 25);
assert.equal(integrated.adapterBindings.filter((entry) => entry.mode === 'concrete_live').length, 13);
assert.equal(integrated.adapterBindings.filter((entry) => entry.mode === 'explicit_blocked').length, 12);
assert.deepEqual(integrated.adapterBindings.filter((entry) => ['A11', 'A12', 'A14'].includes(entry.caseId))
  .map((entry) => [entry.caseId, entry.mode]), [
  ['A11', 'explicit_blocked'], ['A12', 'explicit_blocked'], ['A14', 'explicit_blocked'],
]);

const actionCounts = Object.fromEntries(P158_W7_AGENT_ORCHESTRATION_CASE_IDS.map((caseId) => [
  caseId, selectedPlans.filter((action) => action.caseId === caseId).length,
]));
assert.deepEqual(actionCounts, {
  A01: 260, A02: 440, A03: 50, A04: 216, A05: 12,
  A06: 8, A08: 24, A09: 16, A10: 8, A15: 6,
});

for (const adapter of compiled.adapters) {
  const attempt = schedule.attempts.find((entry) => entry.caseId === adapter.caseId);
  const effectId = schedule.caseContracts.find((entry) => entry.caseId === adapter.caseId).declaredEffectIds[0];
  const before = driverCalls.length;
  const outcome = await adapter.execute({
    attempt,
    requestEffect: (requestedEffectId, payload) => {
      assert.equal(requestedEffectId, effectId);
      return compiled.effects[effectId](payload);
    },
  });
  assert.equal(outcome.resultState, 'passed', adapter.caseId);
  assert.equal(outcome.actionCount, outcome.receipts.length);
  assert.equal(new Set(outcome.actionIds).size, outcome.actionIds.length);
  assert(outcome.receipts.every((receipt) => receipt.receiptSha256.length === 64));
  assert(outcome.receipts.every((receipt) => receipt.stepReceipts.every((step) =>
    step.correlationIds.campaignRunId === target.campaignRunId && step.artifactIds.length > 0)));
  assert(driverCalls.length > before);
}

const incomplete = structuredClone(manifest);
incomplete.actions = incomplete.actions.filter((entry) => entry.caseId !== 'A09').slice();
const partial = compileP158W7AgentOrchestration({
  schedule, target, actionPlans: plans, manifest: incomplete, drivers,
});
assert(!partial.concreteCaseIds.includes('A09'));
assert(partial.blockers.some((entry) => entry.caseId === 'A09' &&
  entry.expectedActionCount === 16 && entry.boundActionCount === 0));

for (const badTarget of [
  { ...target, runtimeLane: 'production', production: true },
  { ...target, foreign: true },
  { ...target, tenantDataPresent: true },
]) {
  assert.throws(
    () => compileP158W7AgentOrchestration({
      schedule, target: badTarget, actionPlans: plans,
      manifest: { ...manifest, targetSha256: sha256(badTarget) }, drivers,
    }),
    (error) => error instanceof P158W7OrchestrationError && error.code === 'development_target_unproven',
  );
}

const wrongHistory = structuredClone(manifest);
const dashboard = wrongHistory.actions.find((entry) => entry.caseId === 'A15' &&
  entry.steps[0].seam === 'history_dashboard');
dashboard.steps[0].seam = 'history_http';
assert.throws(
  () => compileP158W7AgentOrchestration({
    schedule, target, actionPlans: plans, manifest: wrongHistory, drivers,
  }),
  (error) => error instanceof P158W7OrchestrationError && error.code === 'history_transport_binding_invalid',
);

const failingCalls = [];
const failingDrivers = createP158W7ExistingSeamDrivers({
  fetchImpl: async () => ({ ok: true, redirected: false, status: 200, json: async () => ({ success: true }) }),
  execFile: async () => {
    failingCalls.push('browser_cli');
    const error = new Error('first observation failed');
    error.code = 'synthetic_first_failure';
    throw error;
  },
});
const failed = compileP158W7AgentOrchestration({
  schedule,
  target,
  actionPlans: plans,
  manifest,
  drivers: failingDrivers,
});
const a09Adapter = failed.adapters.find((adapter) => adapter.caseId === 'A09');
const a09Attempt = schedule.attempts.find((entry) => entry.caseId === 'A09');
const a09Effect = schedule.caseContracts.find((entry) => entry.caseId === 'A09').declaredEffectIds[0];
const failure = await a09Adapter.execute({
  attempt: a09Attempt,
  requestEffect: (_effectId, payload) => failed.effects[a09Effect](payload),
});
assert.equal(failure.resultState, 'new_product_failure');
assert.equal(failingCalls.length, failure.actionCount, 'independent actions continue once without retry');
assert(failure.receipts.every((receipt) => receipt.stepCount === 1 &&
  receipt.stepReceipts[0].errorCode === 'synthetic_first_failure'));
assert(failure.receipts.every((receipt) => receipt.retryAttempted === false &&
  receipt.repairAttempted === false && receipt.garbageCollectionAttempted === false));

console.log(`p158 W7 agent orchestration passed (${selectedPlans.length} exact action workflows)`);
