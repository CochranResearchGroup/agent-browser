import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { isAbsolute } from 'node:path';
import { fileURLToPath } from 'node:url';

import { auditDashboardFixture } from './p158-dashboard-oracle.js';
import { RETAINED_IDENTITY_FIELDS, classifyOperatorUrl } from './p158-external-handoff-oracle.js';
import { sha256 } from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';
import { aggregateExternalVantageReceipts } from '../run-p158-external-vantage.js';

export const P158_W8_CASE_IDS = Object.freeze([
  ...Array.from({ length: 12 }, (_, index) => `H${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 12 }, (_, index) => `D${String(index + 1).padStart(2, '0')}`),
]);

const W8_SOURCE_PATH = 'scripts/lib/p158-w8-hd-adapters.js';
const REVIEWED_SOURCE_URLS = Object.freeze({
  externalVantageRunner: new URL('../run-p158-external-vantage.js', import.meta.url),
  externalVantageWorkflow: new URL('../../.github/workflows/p158-external-vantage.yml', import.meta.url),
  syntheticVisualFixture: new URL('../p158-synthetic-visual-fixture.js', import.meta.url),
  dashboardOracle: new URL('./p158-dashboard-oracle.js', import.meta.url),
  dashboardLiveSmoke: new URL('../smoke-dashboard-operator-plan0022-live.js', import.meta.url),
  dashboardLiveFoundation: new URL('./p158-w8-dashboard-live.js', import.meta.url),
  dashboardCampaignRunner: new URL('../run-p158-w8-dashboard-campaign.js', import.meta.url),
});

export const P158_W8_LIVE_HOOK_GAPS = Object.freeze({
  H01: 'external_workflow_action_manifest_missing',
  H02: 'url_role_host_scheme_injection_driver_missing',
  H03: 'presentation_rebind_transition_driver_missing',
  H04: 'multi_viewer_controller_contention_driver_missing',
  H05: 'agent_human_controller_barrier_driver_missing',
  H06: 'remote_desktop_visible_state_driver_missing',
  H07: 'route_capacity_saturation_driver_missing',
  H08: 'presentation_failure_injection_driver_missing',
  H09: 'external_network_profile_driver_missing',
  H10: 'durable_handoff_disruption_driver_missing',
  H11: 'secure_surface_action_driver_and_operator_gate_missing',
  H12: 'scheduled_24_hour_reconnect_driver_missing',
  D01: 'reviewed_dashboard_campaign_execution_artifact_missing',
  D02: 'live_resource_transition_barrier_driver_missing',
  D03: 'live_ambiguous_rail_fixture_driver_missing',
  D04: 'external_multi_client_dashboard_driver_missing',
  D05: 'live_missing_resource_deep_link_driver_missing',
  D06: 'live_health_axis_matrix_driver_missing',
  D07: 'live_snapshot_stream_fault_driver_missing',
  D08: 'external_dashboard_handoff_action_scan_missing',
  D09: 'declared_lock_respecting_service_state_churn_api_missing',
  D10: 'live_interaction_timing_capture_driver_missing',
  D11: 'scheduled_8_hour_resource_capture_driver_missing',
  D12: 'live_responsive_accessibility_matrix_driver_missing',
});

export const P158_W8_REVIEWED_SOURCE_COVERAGE = Object.freeze({
  externalVantageRunner: Object.freeze({
    path: 'scripts/run-p158-external-vantage.js',
    cases: Object.freeze(['H01', 'H02', 'H12']),
    coverage: 'Public HTTPS DNS, TLS, cookie, redirect, WebSocket, iframe, form-action, reconnect, pixels, and retained-identity evidence for one readiness visit or the fixed C01 calibration workload.',
    missing: 'It does not consume the W8 frozen action IDs or implement the H02 injected host matrix and H12 500-action 24-hour schedule.',
  }),
  externalVantageWorkflow: Object.freeze({
    path: '.github/workflows/p158-external-vantage.yml',
    cases: Object.freeze(['H01']),
    coverage: 'Two distinct off-host runners and one aggregate receipt over the same durable handoff.',
    missing: 'It has no W8 case action manifest or exact per-action terminal receipts.',
  }),
  syntheticVisualFixture: Object.freeze({
    path: 'scripts/p158-synthetic-visual-fixture.js',
    cases: Object.freeze(['H02', 'H06', 'H11', 'D12']),
    coverage: 'Synthetic popup, prompt-like dialog, iframe, form, WebSocket, redirect, focus, overflow, reduced-motion, and pixel-marker surfaces.',
    missing: 'It does not emulate native chooser, LastPass, passkey, desktop minimize or obscuration, nor execute frozen W8 actions.',
  }),
  dashboardOracle: Object.freeze({
    path: 'scripts/lib/p158-dashboard-oracle.js',
    cases: Object.freeze(Array.from({ length: 12 }, (_, index) => `D${String(index + 1).padStart(2, '0')}`)),
    coverage: 'Deterministic audit of materialized dashboard truth, rail, action, warning, URL, UI, timing, and resource evidence.',
    missing: 'It audits supplied fixtures; it does not capture a live dashboard or apply declared stimuli.',
  }),
  dashboardLiveSmoke: Object.freeze({
    path: 'scripts/smoke-dashboard-operator-plan0022-live.js',
    cases: Object.freeze(['D01', 'D03', 'D04', 'D05', 'D08', 'D12']),
    coverage: 'Development dashboard navigation, selection, action, and UI observation primitives.',
    missing: 'It is a Plan 0022 smoke and does not bind the P158 schedule, matrices, external-client set, or oracle receipt contract.',
  }),
  dashboardLiveFoundation: Object.freeze({
    path: 'scripts/lib/p158-w8-dashboard-live.js',
    cases: Object.freeze(['D01', 'D09']),
    coverage: 'Exact twelve-action pre-freeze root plan, disposable Service State density materialization, and authoritative API, full rail, action, warning, screenshot, and navigation-performance capture through public external ingress.',
    missing: 'It requires installed-candidate parser receipts and an external runtime selector; D09 also lacks a reviewed declared active-churn stream driver.',
  }),
  dashboardCampaignRunner: Object.freeze({
    path: 'scripts/run-p158-w8-dashboard-campaign.js',
    cases: Object.freeze(['D01', 'D09']),
    coverage: 'Installed-parser-bound immutable preseeds, isolated per-action Service and dashboard roots, exact reviewed HTTPS route selection, off-host Playwright capture, D09 churn planning, exact teardown, and append-only terminal receipts.',
    missing: 'D01 remains blocked until one frozen reviewed campaign aggregate is supplied. D09 remains blocked because Service has no declared lock-respecting development state-churn API.',
  }),
});

export const P158_W8_ERROR_CODES = Object.freeze([
  'action_set_mismatch',
  'dashboard_barrier_missing',
  'dashboard_oracle_failed',
  'external_ingress_missing',
  'external_action_manifest_invalid',
  'external_action_result_invalid',
  'frozen_seal_invalid',
  'handoff_digest_mismatch',
  'hook_missing',
  'identity_mismatch',
  'operator_gate_invalid',
  'operator_gate_missing',
  'private_content_prohibited',
  'ready_before_pixels_unproven',
  'receipt_binding_mismatch',
  'receipt_invalid',
  'registry_invalid',
  'schedule_invalid',
  'stimulus_receipt_invalid',
  'unsafe_url_prohibited',
]);

const SHA256 = /^[a-f0-9]{64}$/;
const COMMON_RECEIPT_FIELDS = Object.freeze([
  'actionId', 'attemptId', 'caseId', 'candidateSha256', 'workflowSha256',
]);
const STIMULUS_CASES = Object.freeze({
  H03: 'rebind_transition',
  H05: 'controller_transfer',
  H07: 'route_request',
  H08: 'failure_injection',
  H09: 'network_profile',
  H10: 'disruption_transition',
  H12: 'lease_or_client_state',
  D02: 'resource_transition',
  D07: 'response_fault',
  D09: 'stream_state',
});
const FORBIDDEN_URL_ROLES = new Set([
  'providerExternalUrl', 'routeBinding', 'localEmbedUrl', 'dashboardEmbedUrl', 'healthUrl',
  'rawGuacamoleUrl',
]);
const FORBIDDEN_CAPTURE_FIELDS = new Set([
  'authorization', 'cookie', 'credentialInput', 'password', 'privateContent',
  'rawBody', 'secret', 'token', 'vaultContent',
]);

export class P158W8AdapterError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W8AdapterError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W8AdapterError(code, message, details);
}

function clone(value) {
  return structuredClone(value);
}

function withoutReceiptHash(value) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => field !== 'receiptSha256'));
}

function requireDigest(value, field) {
  if (!SHA256.test(value ?? '')) fail('frozen_seal_invalid', `${field} must be a frozen SHA-256 digest`);
  return value;
}

function isUrl(value) {
  return typeof value === 'string' && /^(?:https?|wss?):\/\//i.test(value);
}

function inspectUrls(value, path = 'receipt', seen = new Set()) {
  if (!value || typeof value !== 'object' || seen.has(value)) return;
  seen.add(value);
  for (const [key, entry] of Object.entries(value)) {
    const nextPath = `${path}.${key}`;
    if (FORBIDDEN_CAPTURE_FIELDS.has(key) && entry !== undefined && entry !== null) {
      fail('private_content_prohibited', `${nextPath} is forbidden at capture`);
    }
    if (FORBIDDEN_URL_ROLES.has(key) && entry !== undefined && entry !== null) {
      fail('unsafe_url_prohibited', `${nextPath} is an internal URL role and cannot enter W8 evidence`);
    }
    if (typeof entry === 'string' &&
        (/^[a-z][a-z0-9+.-]*:\/\//i.test(entry) || /^(?:data|javascript|file|blob):/i.test(entry)) &&
        !isUrl(entry)) {
      fail('unsafe_url_prohibited', `${nextPath} uses a prohibited URL scheme`);
    }
    if (typeof entry === 'string' && /^\/(?:guacamole|internal|local-embed|health)(?:\/|$)/i.test(entry)) {
      fail('unsafe_url_prohibited', `${nextPath} exposes an internal relative URL`);
    }
    if (isUrl(entry)) {
      const role = entry.startsWith('wss:') ? 'websocket_endpoint' : 'location_header';
      const classification = classifyOperatorUrl(entry, { role });
      if (classification.findingCodes.length > 0) {
        fail('unsafe_url_prohibited', `${nextPath} is not a public secure URL`, {
          findingCodes: classification.findingCodes,
        });
      }
    } else if (entry && typeof entry === 'object') inspectUrls(entry, nextPath, seen);
  }
  seen.delete(value);
}

function validateSeals({ registry, schedule, seals }) {
  if (registry?.registryState !== 'frozen') fail('registry_invalid', 'W8 requires the frozen P158 registry');
  if (schedule?.planId !== 'P158' || !Array.isArray(schedule.caseContracts) || !Array.isArray(schedule.attempts)) {
    fail('schedule_invalid', 'W8 requires the compiled P158 execution schedule');
  }
  if (seals?.freezeState !== 'frozen') fail('frozen_seal_invalid', 'W8 seals must be frozen before adapters are constructed');
  for (const field of [
    'scheduleSha256', 'registrySha256', 'candidateSha256', 'workflowSha256',
    'handoffUrlSha256', 'externalVantageReceiptSha256',
    'externalHandoffOracleReportSha256', 'fixtureRedactionReceiptSha256',
  ]) requireDigest(seals?.[field], field);
  if (seals.scheduleSha256 !== schedule.scheduleSha256 || seals.registrySha256 !== schedule.registrySha256) {
    fail('frozen_seal_invalid', 'W8 seal does not bind the compiled schedule and registry');
  }
  if (typeof seals.fixtureId !== 'string' || !seals.fixtureId || !seals.expectedIdentity) {
    fail('frozen_seal_invalid', 'W8 seal omits synthetic fixture or retained identity');
  }
  for (const field of RETAINED_IDENTITY_FIELDS) {
    if (typeof seals.expectedIdentity[field] !== 'string' || !seals.expectedIdentity[field]) {
      fail('frozen_seal_invalid', `expectedIdentity.${field} is required`);
    }
  }
  inspectUrls(seals, 'seals');
}

function validateOperatorGate(operatorAssisted, seals) {
  if (!operatorAssisted?.enabled) return { enabled: false, gateArtifactSha256: null };
  const gate = operatorAssisted.gateArtifact;
  if (!gate) fail('operator_gate_missing', 'operator-assisted H11 is not freeze-ready without a gate artifact');
  if (gate.mode !== 'nonproduction_operator_assisted' || gate.approved !== true ||
      typeof gate.artifactId !== 'string' || !gate.artifactId || gate.freezeEligible !== true ||
      gate.candidateSha256 !== seals.candidateSha256 || gate.workflowSha256 !== seals.workflowSha256 ||
      gate.handoffUrlSha256 !== seals.handoffUrlSha256 ||
      gate.secretsCaptured !== false || gate.vaultContentCaptured !== false ||
      gate.credentialInputCaptured !== false || !SHA256.test(gate.artifactSha256 ?? '')) {
    fail('operator_gate_invalid', 'operator-assisted H11 gate does not prove nonproduction exclusion-at-capture');
  }
  inspectUrls(gate, 'operatorAssisted.gateArtifact');
  return { enabled: true, gateArtifactSha256: gate.artifactSha256 };
}

function cartesian(dimensions, index = 0, prefix = {}) {
  if (index === dimensions.length) return [prefix];
  const dimension = dimensions[index];
  return dimension.values.flatMap((value) => cartesian(dimensions, index + 1, {
    ...prefix,
    [dimension.id]: value,
  }));
}

function assignmentsFor(testCase, attempt) {
  const { executionContract: contract } = testCase;
  if (contract.expansion.strategy === 'dimension') {
    const assignment = attempt.executionUnit?.dimensionAssignment;
    if (!assignment) fail('schedule_invalid', `${attempt.attemptId} omits its dimension assignment`);
    return [{ [assignment.dimensionId]: assignment.value }];
  }
  if (contract.expansion.strategy === 'repeat') {
    return [Object.fromEntries(contract.dimensions.map((dimension) => [
      dimension.id,
      dimension.values[(attempt.repetition - 1) % dimension.values.length],
    ]))];
  }
  return cartesian(contract.dimensions);
}

function cardinalitiesFor(attempt) {
  return Object.fromEntries((attempt.cardinalityAllocations ?? []).map((entry) => [
    entry.id,
    { mode: entry.mode, scope: entry.scope, value: entry.assignedValue, actionIds: [...entry.actionIds] },
  ]));
}

function stimulusFor(caseId, assignment) {
  const dimensionId = STIMULUS_CASES[caseId];
  if (!dimensionId || assignment[dimensionId] === undefined) return null;
  return { dimensionId, kind: assignment[dimensionId] };
}

export function buildP158W8ActionPlan({ testCase, attempt, operatorAssisted = { enabled: false } }) {
  if (!P158_W8_CASE_IDS.includes(testCase?.id) || attempt?.caseId !== testCase.id) {
    fail('schedule_invalid', 'W8 action plan case and attempt do not match');
  }
  const assignments = assignmentsFor(testCase, attempt);
  const actions = assignments.map((assignment, index) => {
    const ordinal = index + 1;
    const secureFixture = assignment.secure_fixture;
    const operatorAction = testCase.id === 'H11' && operatorAssisted.enabled &&
      ['nonproduction_lastpass_vault', 'test_passkey_relying_party'].includes(secureFixture);
    if (operatorAction && !SHA256.test(operatorAssisted.gateArtifactSha256 ?? '')) {
      fail('operator_gate_missing', 'operator-assisted H11 action plan lacks its frozen gate artifact digest');
    }
    return {
      actionId: `${attempt.attemptId}:action:${String(ordinal).padStart(3, '0')}`,
      ordinal,
      attemptId: attempt.attemptId,
      caseId: testCase.id,
      environmentId: attempt.environmentId,
      environmentIds: [...attempt.environmentIds],
      surface: testCase.id.startsWith('D') ? 'dashboard' : 'human_remote_view',
      assignment,
      cardinalities: cardinalitiesFor(attempt),
      stimulus: stimulusFor(testCase.id, assignment),
      externalIngressRequired: attempt.externalIngressRequired === true,
      plannedOffsetSeconds: attempt.executionUnit?.plannedOffsetSeconds ?? null,
      contentClass: operatorAction ? 'nonproduction_operator_assisted' : 'synthetic',
      secureSurfaceMode: testCase.id === 'H11'
        ? operatorAction ? 'operator_assisted' : 'synthetic_fixture'
        : null,
      secureFixtureVariant: testCase.id === 'H11' && !operatorAction
        ? {
            synthetic_chooser: 'synthetic_chooser',
            synthetic_prompt: 'synthetic_prompt',
            nonproduction_lastpass_vault: 'synthetic_vault_chooser',
            test_passkey_relying_party: 'synthetic_passkey_prompt',
          }[secureFixture]
        : secureFixture ?? null,
      operatorGateArtifactSha256: operatorAction ? operatorAssisted.gateArtifactSha256 : null,
      duration: testCase.executionContract.duration ?? null,
    };
  });
  return {
    schemaVersion: 'agent-browser.p158-w8-action-plan.v1',
    planId: 'P158',
    caseId: testCase.id,
    attemptId: attempt.attemptId,
    executionContractSha256: attempt.executionContractSha256,
    actionCount: actions.length,
    actions,
    repairAllowed: false,
    retryAllowed: false,
    gcAllowed: false,
  };
}

export function sealP158W8Receipt(receipt) {
  const { receiptSha256: _ignored, ...body } = clone(receipt);
  return { ...body, receiptSha256: sha256(body) };
}

export function buildP158W8ExternalActionManifest({
  registry,
  schedule,
  seals,
  caseIds = ['H01'],
}) {
  validateSeals({ registry, schedule, seals });
  const selected = [...new Set(caseIds)].sort();
  if (selected.some((caseId) => !['H01', 'H02'].includes(caseId))) {
    fail('schedule_invalid', 'The reviewed external action runner currently supports only H01 and H02');
  }
  const cases = new Map(registry.cases.map((entry) => [entry.id, entry]));
  const attempts = schedule.attempts.filter((attempt) => selected.includes(attempt.caseId));
  const actions = attempts.flatMap((attempt) => buildP158W8ActionPlan({
    testCase: cases.get(attempt.caseId),
    attempt,
  }).actions.map((action) => ({
    ...action,
    executorKind: action.caseId === 'H01'
      ? 'external_vantage_aggregate_projection'
      : 'external_url_policy_scan',
  })));
  const body = {
    schemaVersion: 'agent-browser.p158-w8-external-action-manifest.v1',
    planId: 'P158',
    scheduleSha256: schedule.scheduleSha256,
    registrySha256: schedule.registrySha256,
    candidateSha256: seals.candidateSha256,
    workflowSha256: seals.workflowSha256,
    handoffUrlSha256: seals.handoffUrlSha256,
    caseIds: selected,
    actionCount: actions.length,
    actions,
    repairAllowed: false,
    retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  return Object.freeze({ ...body, manifestSha256: sha256(body) });
}

function validateCommonReceipt({ receipt, action, seals }) {
  if (!receipt || typeof receipt !== 'object') fail('receipt_invalid', `${action.actionId} returned no receipt`);
  for (const field of COMMON_RECEIPT_FIELDS) {
    const expected = field === 'actionId' ? action.actionId
      : field === 'attemptId' ? action.attemptId
        : field === 'caseId' ? action.caseId
          : seals[field];
    if (receipt[field] !== expected) {
      fail('receipt_binding_mismatch', `${action.actionId} receipt ${field} mismatch`, { expected, observed: receipt[field] ?? null });
    }
  }
  const { receiptSha256, ...body } = receipt;
  if (receiptSha256 !== sha256(body)) fail('receipt_binding_mismatch', `${action.actionId} receipt self-hash mismatch`);
  if (receipt.terminalState !== 'completed' || receipt.scenarioOraclePassed !== true || receipt.attemptNumber !== 1) {
    fail('receipt_invalid', `${action.actionId} did not complete its declared scenario oracle exactly once`);
  }
  if (receipt.repairAttempted !== false) fail('receipt_invalid', `${action.actionId} attempted reactionary repair`);
  if (receipt.retryAttempted !== false) fail('receipt_invalid', `${action.actionId} attempted an opportunistic retry`);
  if (receipt.gcAttempted !== false) fail('receipt_invalid', `${action.actionId} attempted undeclared garbage collection`);
  if (receipt.contentClass !== action.contentClass) {
    fail('private_content_prohibited', `${action.actionId} content class is outside its frozen plan`);
  }
  if (receipt.operatorGateArtifactSha256 !== action.operatorGateArtifactSha256) {
    fail('receipt_binding_mismatch', `${action.actionId} did not bind its operator gate`);
  }
  if (receipt.scheduledOffsetSeconds !== action.plannedOffsetSeconds) {
    fail('receipt_binding_mismatch', `${action.actionId} did not bind its declared schedule offset`);
  }
  if (action.duration?.mode === 'minimum' && action.ordinal === 1 && action.caseId === 'D11' &&
      (!Number.isFinite(receipt.observedDurationSeconds) ||
       receipt.observedDurationSeconds < action.duration.seconds)) {
    fail('receipt_binding_mismatch', `${action.actionId} did not satisfy its minimum duration`);
  }
  if (action.surface === 'human_remote_view' && action.contentClass === 'synthetic' &&
      (receipt.fixtureId !== seals.fixtureId ||
       receipt.fixtureRedactionReceiptSha256 !== seals.fixtureRedactionReceiptSha256)) {
    fail('receipt_binding_mismatch', `${action.actionId} is not bound to the frozen synthetic fixture`);
  }
  if (receipt.capture?.credentialsCaptured !== false || receipt.capture?.secretInputCaptured !== false ||
      receipt.capture?.privateContentCaptured !== false) {
    fail('private_content_prohibited', `${action.actionId} lacks exclusion-at-capture proof`);
  }
  const expectedCardinalities = Object.fromEntries(Object.entries(action.cardinalities).map(([id, value]) => [id, value.value]));
  if (sha256(receipt.observedCardinalities ?? {}) !== sha256(expectedCardinalities)) {
    fail('receipt_binding_mismatch', `${action.actionId} cardinality evidence differs from the sealed allocation`);
  }
  inspectUrls(receipt, `receipt.${action.actionId}`);
}

function parsedTime(value) {
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function validateExternalReceipt({ receipt, action, seals }) {
  validateCommonReceipt({ receipt, action, seals });
  const ingress = receipt.externalIngress;
  if (!ingress || ingress.vantage !== 'off_host' || ingress.passed !== true ||
      ingress.externalVantageReceiptSha256 !== seals.externalVantageReceiptSha256 ||
      ingress.externalHandoffOracleReportSha256 !== seals.externalHandoffOracleReportSha256) {
    fail('external_ingress_missing', `${action.actionId} lacks frozen external-ingress proof`);
  }
  if (ingress.handoffUrlSha256 !== seals.handoffUrlSha256) {
    fail('handoff_digest_mismatch', `${action.actionId} used a different durable handoff`);
  }
  const ready = parsedTime(ingress.readyObservedAt);
  const pixels = parsedTime(ingress.firstUsablePixelsAt);
  if (ingress.operatorVisibleState !== 'ready' || ready === null || pixels === null || ready > pixels) {
    fail('ready_before_pixels_unproven', `${action.actionId} did not prove ready before usable pixels`);
  }
  for (const field of RETAINED_IDENTITY_FIELDS) {
    if (ingress.identity?.[field] !== seals.expectedIdentity[field]) {
      fail('identity_mismatch', `${action.actionId} changed retained identity ${field}`);
    }
  }
}

function validateStimulusReceipt({ receipt, action, seals }) {
  try {
    validateCommonReceipt({ receipt, action, seals });
  } catch (error) {
    fail('stimulus_receipt_invalid', `${action.actionId} stimulus receipt is invalid`, { cause: error.code ?? error.message });
  }
  if (receipt.stimulus?.dimensionId !== action.stimulus.dimensionId || receipt.stimulus?.kind !== action.stimulus.kind ||
      receipt.stimulus?.scheduled !== true) {
    fail('stimulus_receipt_invalid', `${action.actionId} stimulus differs from its declared dimension`);
  }
}

function validateDashboardReceipt({ receipt, action, seals }) {
  validateCommonReceipt({ receipt, action, seals });
  if (!receipt.snapshotBarrierId || receipt.snapshotBarrierId !== receipt.renderedBarrierId ||
      !SHA256.test(receipt.authoritativeSnapshotSha256 ?? '')) {
    fail('dashboard_barrier_missing', `${action.actionId} lacks one authoritative rendered snapshot barrier`);
  }
  if (!receipt.dashboardFixture || receipt.dashboardFixture.truth?.snapshotRevision === undefined) {
    fail('dashboard_barrier_missing', `${action.actionId} lacks immutable dashboard evidence`);
  }
  if (action.caseId === 'D09') {
    const expected = {
      profiles: action.cardinalities.profiles?.value,
      browsers: action.cardinalities.browsers_or_historical_rows?.value,
      tabs: action.cardinalities.tabs?.value,
      jobs: action.cardinalities.jobs?.value,
      events: action.cardinalities.events?.value,
    };
    if (sha256(receipt.dashboardFixture.truth.counts ?? {}) !== sha256(expected)) {
      fail('dashboard_oracle_failed', `${action.actionId} dense inventory differs from its aggregate allocation`);
    }
  }
  const report = auditDashboardFixture({ fixture: receipt.dashboardFixture });
  const railClean = report.summary.missingRailRowCount === 0 && report.summary.duplicateRailRowCount === 0 &&
    report.summary.staleRailRowCount === 0 && report.summary.wrongRailRowCount === 0;
  if (!report.passed || !railClean || report.timingDistributions.length === 0 ||
      Object.keys(report.resourceSlopes).length === 0) {
    fail('dashboard_oracle_failed', `${action.actionId} dashboard truth or performance oracle failed`, {
      findingCounts: report.summary.findingCounts,
    });
  }
  return report;
}

export function validateP158W8Receipt({ receipt, action, seals, kind }) {
  if (kind === 'external') return validateExternalReceipt({ receipt, action, seals });
  if (kind === 'dashboard') return validateDashboardReceipt({ receipt, action, seals });
  if (kind === 'stimulus') return validateStimulusReceipt({ receipt, action, seals });
  return validateCommonReceipt({ receipt, action, seals });
}

function requireHook(hooks, path) {
  const hook = path.split('.').reduce((value, key) => value?.[key], hooks);
  if (typeof hook !== 'function') fail('hook_missing', `W8 requires injected hook ${path}`);
  return hook;
}

function hookRequest(action, seals, extra = {}) {
  return clone({
    ...action,
    candidateSha256: seals.candidateSha256,
    workflowSha256: seals.workflowSha256,
    ...extra,
  });
}

function effectDriver({ testCase, seals, hooks, operatorGate }) {
  return async (payload, attempt) => {
    if (payload.planSha256 !== sha256(payload.plan) || payload.plan.attemptId !== attempt.attemptId) {
      fail('action_set_mismatch', `${attempt.attemptId} effect payload is not its sealed action plan`);
    }
    const actionReceipts = [];
    for (const action of payload.plan.actions) {
      if (action.contentClass === 'nonproduction_operator_assisted' && !operatorGate.enabled) {
        fail('operator_gate_missing', `${action.actionId} requires the frozen operator-assisted gate`);
      }
      if (action.stimulus) {
        const stimulusReceipt = await requireHook(hooks, 'stimulus.schedule')(
          hookRequest(action, seals),
        );
        validateStimulusReceipt({ receipt: stimulusReceipt, action, seals });
      }

      let primaryReceipt;
      if (action.externalIngressRequired) {
        primaryReceipt = await requireHook(hooks, 'externalWorkflow.execute')(hookRequest(action, seals, {
          handoffUrlSha256: seals.handoffUrlSha256,
          gateArtifactSha256: action.secureSurfaceMode === 'operator_assisted'
            ? operatorGate.gateArtifactSha256 : null,
        }));
        validateExternalReceipt({ receipt: primaryReceipt, action, seals });
      } else if (action.surface === 'dashboard') {
        primaryReceipt = await requireHook(hooks, 'dashboard.execute')(hookRequest(action, seals));
        validateCommonReceipt({ receipt: primaryReceipt, action, seals });
      } else {
        primaryReceipt = await requireHook(hooks, 'playwright.execute')(hookRequest(action, seals));
        validateCommonReceipt({ receipt: primaryReceipt, action, seals });
      }

      let dashboardReportSha256 = null;
      if (action.surface === 'dashboard') {
        const dashboardReceipt = action.externalIngressRequired
          ? await requireHook(hooks, 'dashboard.capture')(
            hookRequest(action, seals, { externalReceiptSha256: primaryReceipt.receiptSha256 }),
          )
          : primaryReceipt;
        const dashboardReport = validateDashboardReceipt({ receipt: dashboardReceipt, action, seals });
        dashboardReportSha256 = sha256(dashboardReport);
      }
      actionReceipts.push({
        actionId: action.actionId,
        primaryReceiptSha256: primaryReceipt.receiptSha256,
        dashboardReportSha256,
      });
    }
    if (actionReceipts.length !== payload.plan.actionCount) {
      fail('action_set_mismatch', `${attempt.attemptId} did not execute every declared action exactly once`);
    }
    return {
      schemaVersion: 'agent-browser.p158-w8-effect-result.v1',
      caseId: testCase.id,
      attemptId: attempt.attemptId,
      planSha256: payload.planSha256,
      actionReceipts,
      repairAttempted: false,
      retryAttempted: false,
      gcAttempted: false,
    };
  };
}

export function createP158W8AdapterBundle({
  registry,
  schedule,
  seals,
  hooks,
  operatorAssisted = { enabled: false },
}) {
  validateSeals({ registry, schedule, seals });
  const operatorGate = validateOperatorGate(operatorAssisted, seals);
  const frozenHooks = Object.freeze({
    externalWorkflow: Object.freeze({ execute: requireHook(hooks, 'externalWorkflow.execute') }),
    playwright: Object.freeze({ execute: requireHook(hooks, 'playwright.execute') }),
    dashboard: Object.freeze({
      execute: requireHook(hooks, 'dashboard.execute'),
      capture: requireHook(hooks, 'dashboard.capture'),
    }),
    stimulus: Object.freeze({ schedule: requireHook(hooks, 'stimulus.schedule') }),
  });
  const registryCases = new Map(registry.cases.map((entry) => [entry.id, clone(entry)]));
  const scheduleContracts = new Map(schedule.caseContracts.map((entry) => [entry.caseId, clone(entry)]));
  const adapters = [];
  const effects = {};
  for (const caseId of P158_W8_CASE_IDS) {
    const testCase = registryCases.get(caseId);
    const scheduleContract = scheduleContracts.get(caseId);
    if (!testCase || !scheduleContract || scheduleContract.phaseId !== 'W8' ||
        scheduleContract.executionContractSha256 !== sha256(testCase.executionContract)) {
      fail('schedule_invalid', `${caseId} is not bound to its frozen execution contract`);
    }
    const effectId = `p158.effect.${caseId}.declared`;
    adapters.push(createP158CaseAdapter({
      caseId,
      evidenceProfile: testCase.evidenceProfile,
      executionContract: testCase.executionContract,
      execute: async ({ attempt, requestEffect }) => {
        const plan = buildP158W8ActionPlan({ testCase, attempt, operatorAssisted: operatorGate });
        const planSha256 = sha256(plan);
        const effect = await requestEffect(effectId, { plan, planSha256 });
        return {
          resultState: 'passed',
          evidence: {
            schemaVersion: 'agent-browser.p158-w8-attempt-evidence.v1',
            planSha256,
            effectResultSha256: sha256(effect),
            actionCount: plan.actionCount,
            repairAttempted: false,
            retryAttempted: false,
            gcAttempted: false,
          },
        };
      },
    }));
    effects[effectId] = effectDriver({ testCase, seals: clone(seals), hooks: frozenHooks, operatorGate });
  }
  return {
    schemaVersion: 'agent-browser.p158-w8-adapter-bundle.v1',
    planId: 'P158',
    scheduleSha256: schedule.scheduleSha256,
    registrySha256: schedule.registrySha256,
    adapterCount: adapters.length,
    adapters,
    effects,
    operatorAssistedReady: !operatorAssisted.enabled || operatorGate.enabled,
    reactionaryRepairAllowed: false,
    opportunisticRetryAllowed: false,
    undeclaredGcAllowed: false,
  };
}

function hashSource(url) {
  return createHash('sha256').update(readFileSync(fileURLToPath(url))).digest('hex');
}

function reviewedSourceInventory() {
  return Object.entries(P158_W8_REVIEWED_SOURCE_COVERAGE).map(([sourceId, description]) => ({
    sourceId,
    sourcePath: description.path,
    sourceSha256: hashSource(REVIEWED_SOURCE_URLS[sourceId]),
    cases: Object.freeze([...description.cases]),
    coverage: description.coverage,
    missing: description.missing,
  }));
}

function w8SourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function w8ActionCounts({ registry, schedule, operatorGate }) {
  const registryCases = new Map(registry.cases.map((entry) => [entry.id, entry]));
  return new Map(P158_W8_CASE_IDS.map((caseId) => {
    const testCase = registryCases.get(caseId);
    const attempts = schedule.attempts.filter((attempt) => attempt.caseId === caseId);
    if (!testCase || attempts.length === 0) fail('schedule_invalid', `${caseId} has no frozen W8 schedule`);
    const count = attempts.reduce((sum, attempt) => sum + buildP158W8ActionPlan({
      testCase,
      attempt,
      operatorAssisted: operatorGate,
    }).actionCount, 0);
    return [caseId, count];
  }));
}

function reviewedHookIds(caseId) {
  const hookIds = [];
  if (caseId.startsWith('H')) hookIds.push('w8.external_workflow', 'w8.playwright');
  if (caseId.startsWith('D')) hookIds.push('w8.dashboard_capture', 'w8.dashboard_execute');
  if (STIMULUS_CASES[caseId]) hookIds.push('w8.stimulus');
  return hookIds.sort();
}

export function assessP158W8ReviewedLiveSources({
  registry,
  schedule,
  seals,
  operatorAssisted = { enabled: false },
}) {
  validateSeals({ registry, schedule, seals });
  const operatorGate = validateOperatorGate(operatorAssisted, seals);
  const actionCounts = w8ActionCounts({ registry, schedule, operatorGate });
  const sources = reviewedSourceInventory();
  const blockers = P158_W8_CASE_IDS.map((caseId) => ({
    caseId,
    code: 'live_case_hook_missing',
    detail: P158_W8_LIVE_HOOK_GAPS[caseId],
    affectedActionCount: actionCounts.get(caseId),
    reviewedHookIds: Object.freeze(reviewedHookIds(caseId)),
  }));
  return Object.freeze({
    schemaVersion: 'agent-browser.p158-w8-reviewed-live-source-readiness.v1',
    planId: 'P158',
    scheduleSha256: schedule.scheduleSha256,
    registrySha256: schedule.registrySha256,
    ready: false,
    concreteCaseIds: Object.freeze([]),
    explicitlyBlockedCaseIds: Object.freeze([...P158_W8_CASE_IDS]),
    reviewedSourceCount: sources.length,
    reviewedSources: Object.freeze(sources.map((entry) => Object.freeze(entry))),
    blockerCount: blockers.length,
    blockers: Object.freeze(blockers.map((entry) => Object.freeze(entry))),
    scheduledActionCount: [...actionCounts.values()].reduce((sum, count) => sum + count, 0),
    effectsExecuted: false,
  });
}

export function createP158W8ReviewedLiveAdapterBundle({
  registry,
  schedule,
  seals,
  operatorAssisted = { enabled: false },
  externalActionExecution = null,
  dashboardCampaignExecution = null,
  liveHookManifestSha256 = null,
  additionalAdapters = [],
}) {
  const readiness = assessP158W8ReviewedLiveSources({
    registry,
    schedule,
    seals,
    operatorAssisted,
  });
  const sourceSha256 = w8SourceSha256();
  const actionCounts = new Map(readiness.blockers.map((entry) => [entry.caseId, entry.affectedActionCount]));
  const contracts = new Map(schedule.caseContracts.map((entry) => [entry.caseId, entry]));
  const concreteCaseIds = new Set();
  const effects = {};
  const consumedActionIds = new Set();
  let loadExternalReceipts = null;
  let loadDashboardReceipts = null;
  if (externalActionExecution) {
    const expectedManifest = buildP158W8ExternalActionManifest({
      registry,
      schedule,
      seals,
      caseIds: ['H01'],
    });
    if (sha256(externalActionExecution.manifest) !== sha256(expectedManifest) ||
        !isAbsolute(externalActionExecution.resultPath ?? '') ||
        !isAbsolute(externalActionExecution.aggregatePath ?? '') ||
        !Array.isArray(externalActionExecution.receiptPaths) ||
        externalActionExecution.receiptPaths.length !== 2 ||
        externalActionExecution.receiptPaths.some((entry) => !isAbsolute(entry))) {
      fail('external_action_manifest_invalid',
        'Reviewed W8 external execution requires the exact manifest, aggregate, result, and two receipt paths');
    }
    concreteCaseIds.add('H01');
    let cached = null;
    loadExternalReceipts = async () => {
      if (cached) return cached;
      const [resultRaw, aggregateRaw, ...receiptRaw] = await Promise.all([
        readFile(externalActionExecution.resultPath, 'utf8'),
        readFile(externalActionExecution.aggregatePath, 'utf8'),
        ...externalActionExecution.receiptPaths.map((entry) => readFile(entry, 'utf8')),
      ]);
      const result = JSON.parse(resultRaw);
      const aggregate = JSON.parse(aggregateRaw);
      const sourceReceipts = receiptRaw.map((entry) => JSON.parse(entry));
      let recomputedAggregate;
      try {
        recomputedAggregate = aggregateExternalVantageReceipts(sourceReceipts, { runId: aggregate.runId });
      } catch (error) {
        fail('external_action_result_invalid', `W8 external provenance is invalid: ${error.message}`);
      }
      if (sha256(recomputedAggregate) !== sha256(aggregate) ||
          result.externalVantageAggregateSha256 !== aggregate.aggregateSha256) {
        fail('external_action_result_invalid',
          'W8 external aggregate does not match its two independently verified runner receipts');
      }
      const { resultSha256, ...body } = result;
      if (result?.schemaVersion !== 'agent-browser.p158-w8-external-action-result.v1' ||
          result.manifestSha256 !== expectedManifest.manifestSha256 ||
          resultSha256 !== sha256(body) || result.actionCount !== expectedManifest.actionCount ||
          !Array.isArray(result.actionReceipts) || result.actionReceipts.length !== result.actionCount) {
        fail('external_action_result_invalid', 'W8 external workflow result is missing or changed');
      }
      cached = new Map(result.actionReceipts.map((receipt) => [receipt.actionId, receipt]));
      if (cached.size !== expectedManifest.actionCount ||
          expectedManifest.actions.some((action) => !cached.has(action.actionId))) {
        fail('external_action_result_invalid', 'W8 external workflow result does not cover the exact action set');
      }
      return cached;
    };
  }
  if (dashboardCampaignExecution) {
    if (!isAbsolute(dashboardCampaignExecution.resultPath ?? '') ||
        !SHA256.test(dashboardCampaignExecution.campaignPlanSha256 ?? '')) {
      fail('external_action_manifest_invalid',
        'Reviewed W8 dashboard execution requires an absolute result and frozen campaign plan digest');
    }
    concreteCaseIds.add('D01');
    let cached = null;
    loadDashboardReceipts = async () => {
      if (cached) return cached;
      const result = JSON.parse(await readFile(dashboardCampaignExecution.resultPath, 'utf8'));
      const aggregate = result?.aggregate;
      const { aggregateSha256, ...aggregateBody } = aggregate ?? {};
      const expectedActions = schedule.attempts.filter((attempt) => attempt.caseId === 'D01')
        .flatMap((attempt) => buildP158W8ActionPlan({
          testCase: registry.cases.find((entry) => entry.id === attempt.caseId),
          attempt,
        }).actions.map((action) => action.actionId)).sort();
      const receipts = result?.receipts ?? [];
      cached = new Map(receipts.map((receipt) => [receipt.actionId, receipt]));
      if (aggregate?.schemaVersion !== 'agent-browser.p158-dashboard-campaign-aggregate.v1' ||
          aggregate.campaignPlanSha256 !== dashboardCampaignExecution.campaignPlanSha256 ||
          aggregate.candidateSha256 !== seals.candidateSha256 || aggregate.success !== true ||
          aggregate.retryCount !== 0 || aggregate.repairAttempted !== false ||
          aggregateSha256 !== sha256(aggregateBody) || cached.size !== expectedActions.length ||
          sha256([...cached.keys()].sort()) !== sha256(expectedActions) ||
          receipts.some((receipt) => receipt.resultState !== 'passed' ||
            receipt.terminalState !== 'completed' || receipt.candidateSha256 !== seals.candidateSha256 ||
            receipt.receiptSha256 !== sha256(withoutReceiptHash(receipt)) ||
            receipt.oracleBinding?.passed !== true || receipt.teardown?.state !== 'stopped')) {
        fail('external_action_result_invalid',
          'W8 dashboard campaign aggregate or exact action receipts are missing, failed, or changed');
      }
      return cached;
    };
  }
  const baseAdapters = P158_W8_CASE_IDS.map((caseId) => {
    const contract = contracts.get(caseId);
    if (!contract || contract.phaseId !== 'W8') fail('schedule_invalid', `${caseId} lacks a W8 contract`);
    if (concreteCaseIds.has(caseId)) {
      const testCase = registry.cases.find((entry) => entry.id === caseId);
      const effectId = `p158.effect.${caseId}.reviewed_external`;
      effects[effectId] = async ({ action }) => {
        if (consumedActionIds.has(action.actionId)) {
          fail('external_action_result_invalid', `${action.actionId} was already consumed`);
        }
        const dashboardCampaignReceipt = ['D01', 'D09'].includes(caseId)
          ? (await loadDashboardReceipts()).get(action.actionId)
          : null;
        if (dashboardCampaignReceipt &&
            (dashboardCampaignReceipt.caseId !== action.caseId ||
             dashboardCampaignReceipt.attemptId !== action.attemptId ||
             (action.caseId === 'D01' &&
              (dashboardCampaignReceipt.projection?.density !== action.assignment.inventory_density ||
               dashboardCampaignReceipt.dashboardFixture?.density !== action.assignment.inventory_density)))) {
          fail('external_action_result_invalid', `${action.actionId} dashboard campaign identity is invalid`);
        }
        const receipt = dashboardCampaignReceipt
          ? sealP158W8Receipt({
            actionId: action.actionId,
            attemptId: action.attemptId,
            caseId: action.caseId,
            candidateSha256: seals.candidateSha256,
            workflowSha256: seals.workflowSha256,
            terminalState: 'completed',
            scenarioOraclePassed: true,
            attemptNumber: 1,
            repairAttempted: false,
            retryAttempted: false,
            gcAttempted: false,
            contentClass: action.contentClass,
            operatorGateArtifactSha256: action.operatorGateArtifactSha256,
            scheduledOffsetSeconds: action.plannedOffsetSeconds,
            capture: {
              credentialsCaptured: false,
              secretInputCaptured: false,
              privateContentCaptured: false,
            },
            observedCardinalities: Object.fromEntries(
              Object.entries(action.cardinalities).map(([id, value]) => [id, value.value]),
            ),
            snapshotBarrierId: dashboardCampaignReceipt.projection.authoritativeSnapshotSha256,
            renderedBarrierId: dashboardCampaignReceipt.projection.authoritativeSnapshotSha256,
            authoritativeSnapshotSha256: dashboardCampaignReceipt.projection.authoritativeSnapshotSha256,
            dashboardFixture: clone(dashboardCampaignReceipt.dashboardFixture),
            dashboardCampaignReceiptSha256: dashboardCampaignReceipt.receiptSha256,
            dashboardOracleBindingSha256: sha256(dashboardCampaignReceipt.oracleBinding),
          })
          : (await loadExternalReceipts()).get(action.actionId);
        const { receiptSha256, ...body } = receipt ?? {};
        if (dashboardCampaignReceipt) {
          validateDashboardReceipt({ receipt, action, seals });
        } else if (!receipt || receipt.caseId !== action.caseId || receipt.attemptId !== action.attemptId ||
          receipt.candidateSha256 !== seals.candidateSha256 ||
          receipt.workflowSha256 !== seals.workflowSha256 || receipt.resultState !== 'passed' ||
          receipt.terminalState !== 'completed' || receipt.attemptNumber !== 1 ||
          receipt.repairAttempted !== false || receipt.retryAttempted !== false ||
          receipt.garbageCollectionAttempted !== false || receiptSha256 !== sha256(body)) {
          fail('external_action_result_invalid', `${action.actionId} external receipt binding is invalid`);
        }
        consumedActionIds.add(action.actionId);
        return clone(receipt);
      };
      return createP158CaseAdapter({
        caseId,
        evidenceProfile: contract.evidenceProfile,
        executionContract: contract.executionContract,
        execute: async ({ attempt, requestEffect }) => {
          const plan = buildP158W8ActionPlan({ testCase, attempt });
          const receipts = [];
          for (const action of plan.actions) receipts.push(await requestEffect(effectId, { action }));
          return {
            resultState: 'passed',
            actionCount: plan.actionCount,
            actionIds: plan.actions.map((action) => action.actionId),
            receiptSha256s: receipts.map((receipt) => receipt.receiptSha256),
            effectState: 'verified_effect',
            retryDisposition: 'prohibited_opportunistic_retry',
            repairAttempted: false,
            retryAttempted: false,
            garbageCollectionAttempted: false,
          };
        },
      });
    }
    const blocker = Object.freeze({
      code: 'live_case_hook_missing',
      detail: P158_W8_LIVE_HOOK_GAPS[caseId],
      sourcePath: W8_SOURCE_PATH,
      sourceSha256,
    });
    return createP158CaseAdapter({
      caseId,
      evidenceProfile: contract.evidenceProfile,
      executionContract: contract.executionContract,
      execute: async () => ({
        resultState: 'skipped_blocked',
        blocker,
        effectState: 'not_started',
        requestedEffects: [],
        retryDisposition: 'prohibited_opportunistic_retry',
        repairAttempted: false,
        retryAttempted: false,
        garbageCollectionAttempted: false,
      }),
    });
  });
  const adapterBindings = P158_W8_CASE_IDS.map((caseId) => Object.freeze({
    caseId,
    adapterId: contracts.get(caseId).adapterId,
    executionContractSha256: contracts.get(caseId).executionContractSha256,
    mode: concreteCaseIds.has(caseId) ? 'concrete_live' : 'explicit_blocked',
    providerFree: false,
    sourcePath: W8_SOURCE_PATH,
    sourceSha256,
    hookIds: Object.freeze(reviewedHookIds(caseId)),
    implementedActionCount: concreteCaseIds.has(caseId) ? actionCounts.get(caseId) : 0,
    blockedActionCount: concreteCaseIds.has(caseId) ? 0 : actionCounts.get(caseId),
    effectsAllowed: concreteCaseIds.has(caseId),
    blocker: concreteCaseIds.has(caseId) ? null : Object.freeze({
      code: 'live_case_hook_missing',
      detail: P158_W8_LIVE_HOOK_GAPS[caseId],
    }),
  }));
  const adapters = baseAdapters.map((adapter, index) => {
    const binding = adapterBindings[index];
    return Object.freeze({
      ...adapter,
      executionMode: binding.mode,
      providerFree: false,
      effectsAllowed: binding.effectsAllowed,
      sourcePath: binding.sourcePath,
      sourceSha256: binding.sourceSha256,
      liveHookManifestSha256,
      liveBindingSha256: sha256(binding),
      liveHookIds: Object.freeze([...binding.hookIds]),
      blocker: binding.blocker === null ? null : Object.freeze({
        ...binding.blocker,
        sourcePath: binding.sourcePath,
        sourceSha256: binding.sourceSha256,
      }),
    });
  });
  const activeBlockers = readiness.blockers.filter((entry) => !concreteCaseIds.has(entry.caseId));
  const classifiedReadiness = Object.freeze({
    ...readiness,
    concreteCaseIds: Object.freeze([...concreteCaseIds]),
    explicitlyBlockedCaseIds: Object.freeze(P158_W8_CASE_IDS.filter((caseId) => !concreteCaseIds.has(caseId))),
    blockerCount: activeBlockers.length,
    blockers: Object.freeze(activeBlockers),
  });
  return {
    schemaVersion: 'agent-browser.p158-w8-reviewed-live-adapter-bundle.v1',
    planId: 'P158',
    scheduleSha256: schedule.scheduleSha256,
    registrySha256: schedule.registrySha256,
    ready: true,
    executionReady: concreteCaseIds.size > 0,
    adapters: [...adapters, ...additionalAdapters],
    w8Adapters: adapters,
    effects,
    adapterBindings: Object.freeze(adapterBindings),
    reviewedLiveSources: classifiedReadiness,
    reactionaryRepairAllowed: false,
    opportunisticRetryAllowed: false,
    undeclaredGcAllowed: false,
  };
}
