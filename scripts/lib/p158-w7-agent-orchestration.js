import { execFile as nodeExecFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { sha256 } from './p158-campaign-controller.js';

const CASES = Object.freeze(['A01', 'A02', 'A03', 'A04', 'A05', 'A06', 'A08', 'A09', 'A10', 'A15']);
const PRODUCT_BLOCKED = new Set(['A11', 'A12', 'A14']);
const CASE_READINESS_BLOCKERS = Object.freeze({
  A01: 'client_resource_identity_oracle_missing',
  A02: 'shared_browser_ownership_oracle_missing',
  A03: 'connection_tab_identity_oracle_missing',
  A04: 'acl_decision_oracle_missing',
  A05: 'barrier_release_seam_missing',
  A06: 'revocation_barrier_release_seam_missing',
  A08: 'profile_identity_oracle_missing',
  A09: 'target_pathology_oracle_missing',
  A10: 'inventory_ownership_oracle_missing',
  A15: 'cross_transport_history_oracle_missing',
});
const SEAMS = new Set([
  'service_http',
  'browser_cli',
  'profile_service',
  'history_cli',
  'history_http',
  'history_mcp',
  'history_dashboard',
  'history_external_remote_control',
  'process_inventory',
  'barrier',
]);
const HISTORY_SEAM = Object.freeze({
  cli: 'history_cli',
  http: 'history_http',
  mcp: 'history_mcp',
  dashboard: 'history_dashboard',
  remote_control: 'history_external_remote_control',
});
const EXISTING_SERVICE_ACTIONS = new Set([
  'navigate', 'tab_new', 'tab_switch', 'tab_close', 'tab_handle_release',
  'cdp_free_launch', 'remote_view_open', 'view_focus', 'diagnostics',
  'service_profile_upsert', 'service_session_upsert', 'service_profile_policy_mutate',
  'service_profile_tab_evict', 'service_browsers', 'service_tabs',
]);
const DRIVER_BRAND = Symbol('p158-w7-existing-seam-drivers');
const SOURCE_PATH = 'scripts/lib/p158-w7-agent-orchestration.js';

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

export class P158W7OrchestrationError extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158W7OrchestrationError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7OrchestrationError(code, message, details);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function assertTarget(target) {
  if (target?.runtimeLane !== 'development' || target?.isolationState !== 'isolated' ||
      target?.ownership !== 'p158_campaign' || target?.production !== false ||
      target?.foreign !== false || target?.tenantDataPresent !== false ||
      typeof target?.targetId !== 'string' || typeof target?.campaignRunId !== 'string') {
    fail('development_target_unproven', 'Agent-case workflows require an isolated development target');
  }
}

function dimension(action, id) {
  return action.dimensionAssignments?.find((entry) => entry.dimensionId === id)?.value ?? null;
}

function expectedWorkflowKind(caseId) {
  return ({
    A01: 'client_lifecycle', A02: 'shared_browser_lifecycle',
    A03: 'same_label_isolation', A04: 'acl_matrix', A05: 'acl_transition_barrier',
    A06: 'revocation_eviction_barrier', A08: 'identity_fixture_replay',
    A09: 'target_pathology', A10: 'inventory_adjacency', A15: 'history_marker',
  })[caseId];
}

function requireOperations(binding, operations) {
  const actual = new Set(binding.steps.map((step) => step.operation));
  for (const operation of operations) {
    if (!actual.has(operation)) fail('workflow_operation_missing', `${binding.actionId} omitted ${operation}`);
  }
}

function validateBinding(binding, expected, target) {
  if (binding?.actionId !== expected.actionId || binding.caseId !== expected.caseId ||
      binding.attemptId !== expected.attemptId || binding.targetId !== target.targetId ||
      binding.campaignRunId !== target.campaignRunId ||
      binding.workflowKind !== expectedWorkflowKind(expected.caseId) ||
      !Array.isArray(binding.steps) || binding.steps.length === 0 ||
      binding.repair === true || binding.retry === true || binding.garbageCollect === true) {
    fail('workflow_binding_invalid', expected.actionId);
  }
  const stepIds = new Set();
  for (const step of binding.steps) {
    if (typeof step?.stepId !== 'string' || step.stepId.length === 0 || stepIds.has(step.stepId) ||
        !SEAMS.has(step.seam) || typeof step.operation !== 'string' || step.operation.length === 0) {
      fail('workflow_step_invalid', expected.actionId);
    }
    if (step.correlationIds?.campaignRunId !== target.campaignRunId ||
        step.correlationIds?.attemptId !== expected.attemptId ||
        step.correlationIds?.actionId !== expected.actionId ||
        step.correlationIds?.stepId !== step.stepId) {
      fail('workflow_logging_correlation_invalid', step.stepId);
    }
    if (['service_http', 'profile_service', 'history_http', 'history_dashboard',
      'history_external_remote_control', 'barrier'].includes(step.seam)) {
      let origin;
      try { origin = new URL(step.url).origin; } catch { fail('workflow_http_target_invalid', step.stepId); }
      if (!Array.isArray(target.allowedHttpOrigins) || !target.allowedHttpOrigins.includes(origin)) {
        fail('workflow_http_target_not_owned', step.stepId);
      }
      if (['service_http', 'profile_service', 'history_http'].includes(step.seam)) {
        const parsed = new URL(step.url);
        if (parsed.pathname !== '/api/service/request' ||
            !EXISTING_SERVICE_ACTIONS.has(step.body?.action)) {
          fail('workflow_existing_service_action_invalid', step.stepId);
        }
      }
    } else if (step.seam !== 'barrier') {
      if (!Array.isArray(target.allowedExecutables) || !target.allowedExecutables.includes(step.executable)) {
        fail('workflow_executable_not_owned', step.stepId);
      }
    }
    stepIds.add(step.stepId);
  }
  const value = (id) => dimension(expected, id);
  switch (expected.caseId) {
    case 'A01':
      if (typeof binding.clientId !== 'string' ||
          !['serial', 'concurrent'].includes(binding.dispatchMode)) fail('client_binding_invalid', expected.actionId);
      requireOperations(binding, ['acquire_session', 'acquire_tab', 'release_own_resources']);
      break;
    case 'A02':
      if (typeof binding.clientId !== 'string' || typeof binding.sharedBrowserId !== 'string' ||
          binding.dispatchMode !== 'concurrent') fail('shared_browser_binding_invalid', expected.actionId);
      requireOperations(binding, ['join_retained_browser', 'create_attributable_tab', 'release_own_tab']);
      break;
    case 'A03':
      if (typeof binding.clientId !== 'string' || typeof binding.connectionInstanceId !== 'string' ||
          typeof binding.sharedLabel !== 'string' || binding.dispatchMode !== 'concurrent') {
        fail('same_label_binding_invalid', expected.actionId);
      }
      requireOperations(binding, ['command_targeting', 'tab_targeting', 'release_targeting']);
      break;
    case 'A04':
      if (![value('role'), value('profile_mode'), value('operation'), value('decision')]
        .every((entry) => typeof entry === 'string')) fail('acl_matrix_binding_invalid', expected.actionId);
      requireOperations(binding, ['materialize_policy', value('operation'), 'assert_decision']);
      break;
    case 'A05':
      requireOperations(binding, ['barrier_arrive', value('transition'), 'barrier_release']);
      break;
    case 'A06':
      requireOperations(binding, ['barrier_arrive', 'revoke', value('eviction_mode'), 'barrier_release']);
      break;
    case 'A08':
      requireOperations(binding, ['materialize_identity_fixture', value('action'), 'assert_identity_result']);
      break;
    case 'A09':
      requireOperations(binding, ['create_target_pathology', value('target_pathology'), 'observe_target_result']);
      break;
    case 'A10':
      requireOperations(binding, ['stage_inventory', value('ownership'), value('inventory_state'), 'observe_inventory']);
      break;
    case 'A15': {
      const transport = value('control_transport');
      const allowed = transport ? [HISTORY_SEAM[transport]] : Object.values(HISTORY_SEAM);
      if (!binding.steps.some((step) => allowed.includes(step.seam))) {
        fail('history_transport_binding_invalid', expected.actionId);
      }
      requireOperations(binding, ['navigate_marker', 'reconcile_history']);
      break;
    }
  }
  return freeze(structuredClone(binding));
}

function validateCrossActionInvariants(caseId, bindings) {
  if (caseId === 'A02') {
    const byAttempt = Map.groupBy(bindings, (binding) => binding.attemptId);
    for (const rows of byAttempt.values()) {
      if (new Set(rows.map((row) => row.sharedBrowserId)).size !== 1) {
        fail('shared_browser_cardinality_invalid', rows[0].attemptId);
      }
    }
  }
  if (caseId === 'A03') {
    const byAttempt = Map.groupBy(bindings, (binding) => binding.attemptId);
    for (const rows of byAttempt.values()) {
      if (new Set(rows.map((row) => row.sharedLabel)).size !== 1 ||
          new Set(rows.map((row) => row.connectionInstanceId)).size !== rows.length) {
        fail('same_label_connection_cardinality_invalid', rows[0].attemptId);
      }
    }
  }
}

export function createP158W7ExistingSeamDrivers({
  fetchImpl = globalThis.fetch,
  execFile = promisify(nodeExecFile),
} = {}) {
  if (typeof fetchImpl !== 'function' || typeof execFile !== 'function') {
    fail('existing_seam_dependency_missing', 'Existing-seam drivers require fetch and execFile');
  }
  const http = async (step) => {
    if (typeof step.url !== 'string' || !['http:', 'https:'].includes(new URL(step.url).protocol)) {
      fail('existing_http_binding_invalid', step.stepId);
    }
    const response = await fetchImpl(step.url, {
      method: step.method ?? 'POST',
      redirect: 'error',
      cache: 'no-store',
      headers: { 'content-type': 'application/json', ...(step.headers ?? {}) },
      body: step.body === undefined ? undefined : JSON.stringify(step.body),
    });
    if (response.redirected || !response.ok) {
      const error = new Error(`Existing Service seam returned HTTP ${response.status}`);
      error.code = 'existing_service_request_failed';
      throw error;
    }
    return response.json();
  };
  const cli = async (step) => {
    if (typeof step.executable !== 'string' || !step.executable.startsWith('/') ||
        !Array.isArray(step.args) || step.args.some((value) => typeof value !== 'string')) {
      fail('existing_cli_binding_invalid', step.stepId);
    }
    const result = await execFile(step.executable, step.args, {
      cwd: step.cwd,
      env: step.env,
      timeout: step.timeoutMilliseconds,
      maxBuffer: step.maxBufferBytes ?? 4 * 1024 * 1024,
    });
    return {
      stdoutSha256: createHash('sha256').update(result.stdout ?? '').digest('hex'),
      stderrSha256: createHash('sha256').update(result.stderr ?? '').digest('hex'),
    };
  };
  const drivers = {
    service_http: http,
    profile_service: http,
    history_http: http,
    history_dashboard: http,
    history_external_remote_control: http,
    barrier: http,
    browser_cli: cli,
    history_cli: cli,
    history_mcp: cli,
    process_inventory: cli,
  };
  Object.defineProperty(drivers, DRIVER_BRAND, { value: true });
  Object.defineProperty(drivers, 'metadata', { value: freeze({
    mode: 'transport_only',
    freezeEligible: false,
    sourcePath: SOURCE_PATH,
    sourceSha256: sourceSha256(),
    effectsExecuted: false,
  }), enumerable: true });
  return Object.freeze(drivers);
}

export function compileP158W7AgentOrchestration({ schedule, target, actionPlans, manifest, drivers }) {
  assertTarget(target);
  if (manifest?.schemaVersion !== 'agent-browser.p158-w7-agent-workflows.v1' ||
      manifest.scheduleSha256 !== schedule.scheduleSha256 || manifest.targetSha256 !== sha256(target) ||
      !Array.isArray(manifest.actions)) fail('workflow_manifest_invalid', 'Workflow manifest is not seal-bound');
  const expected = actionPlans.filter((action) => CASES.includes(action.caseId));
  const expectedById = new Map(expected.map((action) => [action.actionId, action]));
  const supplied = new Map();
  for (const binding of manifest.actions) {
    if (supplied.has(binding?.actionId)) fail('workflow_action_duplicate', binding?.actionId);
    const action = expectedById.get(binding?.actionId);
    if (!action) fail('workflow_action_unexpected', binding?.actionId);
    supplied.set(binding.actionId, validateBinding(binding, action, target));
  }
  const sourceBound = drivers?.[DRIVER_BRAND] === true &&
    drivers.metadata?.mode === 'transport_only' && drivers.metadata?.freezeEligible === false &&
    drivers.metadata?.sourcePath === SOURCE_PATH && /^[a-f0-9]{64}$/.test(drivers.metadata?.sourceSha256);
  const concreteCaseIds = [];
  const blocked = [];
  const adapters = [];
  const effects = {};
  for (const caseId of CASES) {
    const caseActions = expected.filter((action) => action.caseId === caseId);
    const bindings = caseActions.map((action) => supplied.get(action.actionId)).filter(Boolean);
    if (!sourceBound) {
      blocked.push(freeze({ caseId, code: 'driver_bundle_not_freeze_eligible', expectedActionCount: caseActions.length, boundActionCount: bindings.length }));
      continue;
    }
    if (bindings.length !== caseActions.length) {
      blocked.push(freeze({ caseId, code: 'workflow_matrix_incomplete', expectedActionCount: caseActions.length, boundActionCount: bindings.length }));
      continue;
    }
    validateCrossActionInvariants(caseId, bindings);
    // These bindings prove only that an owned development transport can be
    // called. They do not yet prove the case-specific postcondition. A 2xx
    // response or zero CLI exit is never promoted to campaign evidence.
    blocked.push(freeze({
      caseId,
      code: CASE_READINESS_BLOCKERS[caseId],
      expectedActionCount: caseActions.length,
      boundActionCount: bindings.length,
    }));
  }
  return freeze({
    schemaVersion: 'agent-browser.p158-w7-agent-orchestration.v1',
    concreteCaseIds,
    blockedCaseIds: blocked.map((entry) => entry.caseId),
    blockers: blocked,
    adapters,
    effects,
    productBlockedCaseIds: [...PRODUCT_BLOCKED],
    effectsExecuted: false,
    freezeEligible: concreteCaseIds.length > 0,
    sourceBound,
    driverSource: sourceBound ? drivers.metadata : null,
  });
}

export const P158_W7_AGENT_ORCHESTRATION_CASE_IDS = CASES;
