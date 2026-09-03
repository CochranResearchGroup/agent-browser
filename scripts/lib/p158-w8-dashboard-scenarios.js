import { sha256 } from './p158-campaign-controller.js';

const SHA256 = /^[a-f0-9]{64}$/u;
export const P158_D05_SUPPORTED_TARGETS = Object.freeze(['tab', 'target']);
export const P158_D05_BLOCKED_TARGETS = Object.freeze(['browser', 'session', 'view', 'handoff']);

export class P158W8DashboardScenarioError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'P158W8DashboardScenarioError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new P158W8DashboardScenarioError(code, message);
}

function clone(value) {
  return structuredClone(value);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function exactState(root, state, materializationReceipt) {
  if (!root?.actionId || !['D03', 'D04', 'D05'].includes(root.caseId) ||
      materializationReceipt?.stateSha256 !== sha256(`${JSON.stringify(canonical(state))}\n`) ||
      sha256(materializationReceipt?.scenario) !== sha256(root.scenario)) {
    fail('scenario_state_binding_invalid', 'Dashboard scenario truth is not bound to the exact sealed preseed state');
  }
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
  }
  return value;
}

function receiptBody(receipt) {
  return without(receipt, 'receiptSha256');
}

function assertSafeReceipt(value, path = 'receipt', seen = new Set()) {
  if (!value || typeof value !== 'object' || seen.has(value)) return;
  seen.add(value);
  for (const [key, entry] of Object.entries(value)) {
    const nextPath = `${path}.${key}`;
    if (/url|handoff/iu.test(key) && entry !== null && entry !== undefined) {
      fail('scenario_receipt_unsafe_url', `${nextPath} may not expose URL or handoff material`);
    }
    if (typeof entry === 'string' && /^(?:https?|wss?|file|data|javascript):/iu.test(entry)) {
      fail('scenario_receipt_unsafe_url', `${nextPath} may not expose a raw URL`);
    }
    if (entry && typeof entry === 'object') assertSafeReceipt(entry, nextPath, seen);
  }
  seen.delete(value);
}

export function sealP158DashboardScenarioReceipt(receipt) {
  const body = receiptBody(receipt);
  return { ...body, receiptSha256: sha256(body) };
}

function ordered(record) {
  return Object.values(record ?? {});
}

function scenarioValue(root) {
  if (root?.scenario?.caseId !== root?.caseId || typeof root.scenario.value !== 'string') {
    fail('scenario_state_binding_invalid', 'Dashboard root omits its exact frozen scenario value');
  }
  return root.scenario.value;
}

function d03Plan(root, state) {
  const profiles = ordered(state.profiles);
  const browsers = ordered(state.browsers);
  const duplicate = profiles.slice(0, 2);
  const crossProfile = browsers.slice(0, 2);
  if (duplicate.length !== 2 || duplicate[0].name !== duplicate[1].name || duplicate[0].id === duplicate[1].id ||
      crossProfile.length !== 2 || crossProfile[0].profileId === crossProfile[1].profileId) {
    fail('d03_preseed_truth_invalid', 'D03 requires two distinct duplicate-label Profiles and cross-Profile browsers');
  }
  return {
    scenario: scenarioValue(root),
    duplicateLabel: duplicate[0].name,
    duplicateResourceIds: duplicate.map((entry) => entry.id),
    expectedSelectedResourceId: duplicate[1].id,
    expectedActionTargetResourceId: crossProfile[1].id,
    crossProfileBindings: crossProfile.map((entry) => ({ browserId: entry.id, profileId: entry.profileId })),
  };
}

function d04Plan(root, state) {
  const browsers = ordered(state.browsers);
  if (browsers.length < 10) fail('d04_preseed_truth_invalid', 'D04 requires at least ten independently selectable browsers');
  return {
    scenario: scenarioValue(root),
    clients: browsers.slice(0, 10).map((browser, index) => ({
      clientId: `${root.actionId}:client:${String(index + 1).padStart(2, '0')}`,
      expectedSelectedResourceId: browser.id,
      alternateResourceId: browsers[(index + 1) % 10].id,
      operations: ['selection', 'refresh', 'back_forward', 'deep_link'],
    })),
  };
}

function d05Plan(root, state) {
  const targetType = scenarioValue(root);
  if (P158_D05_BLOCKED_TARGETS.includes(targetType)) {
    return {
      scenario: targetType,
      executable: false,
      blocker: {
        code: 'dashboard_deep_link_target_unsupported',
        detail: `${targetType} has no authoritative selector and sealed preseed projection in the current dashboard route`,
      },
    };
  }
  if (!P158_D05_SUPPORTED_TARGETS.includes(targetType)) fail('d05_target_invalid', 'D05 target class is outside the frozen contract');
  const browser = ordered(state.browsers)[0];
  const tab = ordered(state.tabs).find((entry) => entry.browserId === browser?.id);
  if (!browser || (targetType !== 'browser' && !tab)) {
    fail('d05_preseed_truth_invalid', 'D05 supported targets require one exact current fallback resource');
  }
  return {
    scenario: targetType,
    executable: true,
    queryKey: 'tab',
    selectedBrowserId: browser.id,
    staleRequestedId: `p158-missing-${targetType}-${sha256(root.actionId).slice(0, 12)}`,
    expectedResolvedSelectionId: tab.id,
    expectedResolvedResourceId: browser.id,
    expectedWorkspaceId: `browser:${browser.id}`,
    expectedExplanationFragment: `Recovered stale selected tab identity`,
  };
}

/** Build scenario truth only from the immutable root and sealed preseed state. */
export function buildP158DashboardScenarioPlan({ root, expectedState, materializationReceipt }) {
  exactState(root, expectedState, materializationReceipt);
  const scenarioTruth = root.caseId === 'D03'
    ? d03Plan(root, expectedState)
    : root.caseId === 'D04'
      ? d04Plan(root, expectedState)
      : d05Plan(root, expectedState);
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-scenario-plan.v1',
    planId: 'P158',
    actionId: root.actionId,
    attemptId: root.attemptId,
    caseId: root.caseId,
    stateSha256: materializationReceipt.stateSha256,
    scenarioTruth,
    repairAllowed: false,
    retryAllowed: false,
    garbageCollectionAllowed: false,
  };
  return { ...body, scenarioPlanSha256: sha256(body) };
}

function validateReceipt(plan, receipt) {
  assertSafeReceipt(receipt);
  if (!receipt || receipt.receiptSha256 !== sha256(receiptBody(receipt)) ||
      receipt.schemaVersion !== 'agent-browser.p158-dashboard-scenario-receipt.v1' ||
      receipt.actionId !== plan.actionId || receipt.caseId !== plan.caseId ||
      receipt.scenarioPlanSha256 !== plan.scenarioPlanSha256 ||
      receipt.repairAttempted !== false || receipt.retryAttempted !== false ||
      receipt.garbageCollectionAttempted !== false) {
    fail('scenario_receipt_invalid', 'Dashboard scenario receipt is missing, changed, foreign, or unsafe');
  }
  return receipt;
}

function auditD03(plan, receipt) {
  const truth = plan.scenarioTruth;
  const observed = receipt.duplicateRows ?? [];
  if (sha256(observed.map((entry) => entry.resourceId).sort()) !== sha256([...truth.duplicateResourceIds].sort()) ||
      observed.some((entry) => entry.label !== truth.duplicateLabel) ||
      sha256(receipt.crossProfileBindings) !== sha256(truth.crossProfileBindings) ||
      receipt.selectedResourceId !== truth.expectedSelectedResourceId ||
      receipt.inspectorResourceId !== truth.expectedSelectedResourceId ||
      receipt.actionTargetResourceId !== truth.expectedActionTargetResourceId ||
      receipt.wrongResourceSelected !== false || receipt.wrongResourceActioned !== false) {
    fail('d03_selection_oracle_failed', 'D03 selected or actioned the wrong resource among duplicate labels or Profiles');
  }
}

function auditD04(plan, receipt) {
  const expected = plan.scenarioTruth.clients;
  if (!Array.isArray(receipt.clients) || receipt.clients.length !== 10 ||
      typeof receipt.publicPath !== 'string' || !receipt.publicPath.startsWith('/p158/') ||
      !SHA256.test(receipt.selectionReceiptSha256 ?? '') ||
      new Set(receipt.clients.map((entry) => entry.clientId)).size !== 10) {
    fail('d04_client_set_invalid', 'D04 requires exactly ten distinct off-host dashboard clients');
  }
  for (const expectedClient of expected) {
    const observed = receipt.clients.find((entry) => entry.clientId === expectedClient.clientId);
    const ingressBody = {
      actionId: plan.actionId,
      clientId: expectedClient.clientId,
      publicPath: receipt.publicPath,
      selectionReceiptSha256: receipt.selectionReceiptSha256,
      offHost: observed?.offHost,
      outsideServiceNetworkNamespace: observed?.outsideServiceNetworkNamespace,
    };
    if (!observed || observed.offHost !== true || observed.outsideServiceNetworkNamespace !== true ||
        observed.clientIngressReceiptSha256 !== sha256(ingressBody) ||
        sha256(observed.completedOperations) !== sha256(expectedClient.operations) ||
        observed.expectedSelectedResourceId !== expectedClient.expectedSelectedResourceId ||
        observed.observedSelectedResourceId !== expectedClient.expectedSelectedResourceId ||
        observed.observedInspectorResourceId !== expectedClient.expectedSelectedResourceId ||
        observed.selectionAfterRefresh !== expectedClient.expectedSelectedResourceId ||
        observed.selectionAfterBackForward !== expectedClient.expectedSelectedResourceId ||
        observed.selectionAfterDeepLink !== expectedClient.expectedSelectedResourceId ||
        observed.finalBarrierSelectedResourceId !== expectedClient.expectedSelectedResourceId ||
        observed.finalBarrierInspectorResourceId !== expectedClient.expectedSelectedResourceId) {
      fail('d04_client_isolation_failed', `${expectedClient.clientId} leaked or lost its action-specific selection`);
    }
  }
}

function auditD05(plan, receipt) {
  const truth = plan.scenarioTruth;
  if (truth.executable !== true) fail('d05_target_blocked', 'Unsupported D05 targets cannot produce executable receipts');
  if (receipt.queryKey !== truth.queryKey || receipt.staleRequestedId !== truth.staleRequestedId ||
      receipt.initialStaleSelectionObserved !== true || receipt.recoveryEventCount !== 1 ||
      receipt.recoveryMethod !== 'dashboard_history_replace' ||
      receipt.recoveryExplanation?.includes(truth.expectedExplanationFragment) !== true ||
      receipt.recoveryExplanation?.includes(truth.staleRequestedId) !== true ||
      receipt.recoveryExplanation?.includes(truth.expectedResolvedSelectionId) !== true ||
      receipt.resolvedSelectionId !== truth.expectedResolvedSelectionId ||
      receipt.resolvedResourceId !== truth.expectedResolvedResourceId ||
      receipt.resolvedWorkspaceId !== truth.expectedWorkspaceId) {
    fail('d05_deep_link_recovery_failed', 'D05 stale deep-link recovery did not resolve once to the exact current resource');
  }
}

/** Audit externally observed behavior against independently derived preseed truth. */
export function auditP158DashboardScenarioReceipt({ plan, receipt }) {
  validateReceipt(plan, receipt);
  if (plan.caseId === 'D03') auditD03(plan, receipt);
  else if (plan.caseId === 'D04') auditD04(plan, receipt);
  else auditD05(plan, receipt);
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-scenario-oracle.v1',
    actionId: plan.actionId,
    caseId: plan.caseId,
    scenarioPlanSha256: plan.scenarioPlanSha256,
    receiptSha256: receipt.receiptSha256,
    passed: true,
    repairAttempted: false,
  };
  return { ...body, oracleSha256: sha256(body) };
}

export function applyP158DashboardScenarioToFixture({ fixture, plan, receipt }) {
  const value = clone(fixture);
  if (plan.caseId === 'D03') {
    value.selection = {
      ...value.selection,
      selectedResourceId: receipt.selectedResourceId,
      inspectorResourceId: receipt.inspectorResourceId,
      selectedExists: true,
      deepLinkRequestedId: receipt.selectedResourceId,
      deepLinkResolvedId: receipt.selectedResourceId,
    };
  } else if (plan.caseId === 'D04') {
    value.clientSelections = receipt.clients.map((entry) => ({
      clientId: entry.clientId,
      expectedSelectedResourceId: entry.expectedSelectedResourceId,
      expectedInspectorResourceId: entry.expectedSelectedResourceId,
      observedSelectedResourceId: entry.observedSelectedResourceId,
      observedInspectorResourceId: entry.observedInspectorResourceId,
    }));
  } else if (plan.scenarioTruth.executable) {
    value.selection = {
      ...value.selection,
      selectedResourceId: receipt.resolvedResourceId,
      inspectorResourceId: receipt.resolvedResourceId,
      selectedExists: true,
      recoveryActionCount: receipt.recoveryEventCount,
      deepLinkRequestedId: receipt.staleRequestedId,
      deepLinkResolvedId: receipt.resolvedResourceId,
    };
  }
  return value;
}

export function assertP158DashboardScenarioPlan(plan) {
  const { scenarioPlanSha256, ...body } = plan ?? {};
  if (!SHA256.test(scenarioPlanSha256 ?? '') || scenarioPlanSha256 !== sha256(body)) {
    fail('scenario_plan_invalid', 'Dashboard scenario plan is missing or changed');
  }
  return plan;
}
