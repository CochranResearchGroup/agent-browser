import { createHash } from 'node:crypto';
import { isAbsolute, join, normalize } from 'node:path';

import { developmentExternalIngressBinding } from './development-presentation-provider.js';
import { sha256 } from './p158-campaign-controller.js';
import { generateDenseDashboardFixture } from './p158-dashboard-oracle.js';
import {
  auditP158DashboardLiveProjection,
  buildP158DashboardServiceState,
  captureP158DashboardLiveProjection,
  materializeP158DashboardPreseedPlan,
} from './p158-w8-dashboard-live.js';

const SHA256 = /^[a-f0-9]{64}$/;

export class P158W8DashboardCampaignError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'P158W8DashboardCampaignError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new P158W8DashboardCampaignError(code, message);
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
  }
  return value;
}

function receiptDigest(value, omitted = 'receiptSha256') {
  return sha256(Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== omitted)));
}

function errorRecord(error) {
  return {
    code: error?.code ?? 'dashboard_campaign_effect_failed',
    message: error instanceof Error ? error.message : String(error),
  };
}

function validateCandidate(candidate) {
  if (!isAbsolute(candidate?.executablePath ?? '') || !SHA256.test(candidate?.executableSha256 ?? '')) {
    fail('candidate_invalid', 'W8 dashboard execution requires an absolute frozen candidate and its SHA-256');
  }
  return structuredClone(candidate);
}

function validateExternalIngress(externalIngress) {
  let binding;
  try {
    binding = developmentExternalIngressBinding({
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: externalIngress?.publicOperatorUrl,
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: externalIngress?.reviewedRevision,
    });
  } catch (error) {
    fail('external_ingress_invalid', error.message);
  }
  if (!binding.configured || (externalIngress.bindingSha256 &&
      externalIngress.bindingSha256 !== binding.bindingSha256)) {
    fail('external_ingress_invalid', 'W8 dashboard execution requires the exact reviewed public HTTPS ingress');
  }
  return binding;
}

/** Bind capture to a frozen public WSS Playwright runner outside the Service host. */
export function validateP158ExternalPlaywrightRunner({ endpoint, attestation }) {
  let parsed;
  try {
    parsed = new URL(endpoint);
  } catch {
    fail('external_runner_invalid', 'External Playwright endpoint is invalid');
  }
  const hostname = parsed.hostname.toLowerCase();
  const { attestationSha256, ...body } = attestation ?? {};
  if (parsed.protocol !== 'wss:' || !hostname.includes('.') || hostname === 'localhost' ||
      hostname.endsWith('.localhost') || hostname.endsWith('.local') ||
      /^(?:127\.|10\.|192\.168\.|169\.254\.|0\.)/.test(hostname) ||
      /^172\.(?:1[6-9]|2\d|3[01])\./.test(hostname) || hostname === '::1' ||
      attestation?.schemaVersion !== 'agent-browser.p158-external-playwright-runner-attestation.v1' ||
      attestation.endpointSha256 !== sha256(endpoint) || attestation.offHost !== true ||
      attestation.outsideServiceHost !== true || attestation.outsideServiceNetworkNamespace !== true ||
      !/^[A-Za-z0-9][A-Za-z0-9._:-]{0,159}$/.test(attestation.reviewedRevision ?? '') ||
      attestationSha256 !== sha256(body)) {
    fail('external_runner_invalid', 'Frozen public off-host Playwright runner attestation is missing or changed');
  }
  return { endpoint: parsed.href, attestation: structuredClone(attestation) };
}

/**
 * Derive a closed set of per-action development runtime parameters. Every path
 * is rooted below that action's disposable root and every TCP port is unique.
 * No ambient HOME, XDG directory, socket directory, session, or default port
 * is inherited by the campaign.
 */
export function buildP158DashboardCampaignPlan({
  preseedPlan,
  candidate,
  externalIngress,
  basePort = 52000,
}) {
  const frozenCandidate = validateCandidate(candidate);
  const ingress = validateExternalIngress(externalIngress);
  if (preseedPlan?.schemaVersion !== 'agent-browser.p158-dashboard-preseed-plan.v1' ||
      preseedPlan.planSha256 !== sha256(Object.fromEntries(
        Object.entries(preseedPlan).filter(([key]) => key !== 'planSha256'),
      )) || !Number.isInteger(basePort) || basePort < 1024 ||
      basePort + (preseedPlan.actionCount * 4) > 65535) {
    fail('campaign_plan_invalid', 'W8 dashboard campaign inputs are missing, changed, or outside the port range');
  }
  const roots = preseedPlan.roots.map((root, index) => {
    const runtimeRoot = normalize(root.target.disposableRoot);
    const xdgRoot = join(runtimeRoot, 'xdg');
    const runtimeStreamPort = basePort + (index * 4);
    const dashboardIngressPort = runtimeStreamPort + 1;
    const dashboardBackendPort = runtimeStreamPort + 2;
    const presentationStreamPort = runtimeStreamPort + 3;
    return {
      actionId: root.actionId,
      attemptId: root.attemptId,
      caseId: root.caseId,
      environmentId: root.environmentId,
      density: root.density,
      streamState: root.streamState,
      target: structuredClone(root.target),
      environment: {
        HOME: root.target.pseudoHome,
        XDG_CONFIG_HOME: join(xdgRoot, 'config'),
        XDG_CACHE_HOME: join(xdgRoot, 'cache'),
        XDG_DATA_HOME: join(xdgRoot, 'data'),
        XDG_STATE_HOME: join(xdgRoot, 'state'),
        XDG_RUNTIME_DIR: join(xdgRoot, 'runtime'),
        AGENT_BROWSER_SOCKET_DIR: join(runtimeRoot, 'runtime', 'sockets'),
        AGENT_BROWSER_SESSION: root.target.runId,
        AGENT_BROWSER_STREAM_PORT: String(runtimeStreamPort),
        AGENT_BROWSER_STREAM_PORT_STRICT: '1',
        AGENT_BROWSER_RUNTIME_HOST: '1',
        AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
      },
      ports: {
        runtimeStream: runtimeStreamPort,
        dashboardIngress: dashboardIngressPort,
        dashboardBackend: dashboardBackendPort,
        presentationStream: presentationStreamPort,
      },
      candidate: frozenCandidate,
      externalIngress: ingress,
      validationInputPath: join(runtimeRoot, 'preseed', 'state.candidate.json'),
      screenshotPath: join(runtimeRoot, 'artifacts', 'dashboard.png'),
    };
  });
  const allPaths = roots.flatMap((root) => Object.entries(root.environment)
    .filter(([key]) => key === 'HOME' || key.startsWith('XDG_') || key === 'AGENT_BROWSER_SOCKET_DIR')
    .map(([, value]) => value));
  const allPorts = roots.flatMap((root) => Object.values(root.ports));
  if (new Set(allPaths).size !== allPaths.length || new Set(allPorts).size !== allPorts.length ||
      roots.some((root) => allPathsFor(root).some((path) => !normalize(path).startsWith(`${normalize(root.target.disposableRoot)}/`)))) {
    fail('campaign_isolation_invalid', 'W8 dashboard roots, runtime paths, and ports must be pairwise isolated');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-plan.v1',
    planId: 'P158',
    preseedPlanSha256: preseedPlan.planSha256,
    candidate: frozenCandidate,
    externalIngress: ingress,
    actionCount: roots.length,
    roots,
    preFreezeMaterializationOnly: true,
    productionHomeAccessAllowed: false,
    repairAllowed: false,
    retryAllowed: false,
  };
  return { ...body, campaignPlanSha256: sha256(canonical(body)) };
}

function allPathsFor(root) {
  return Object.entries(root.environment)
    .filter(([key]) => key === 'HOME' || key.startsWith('XDG_') || key === 'AGENT_BROWSER_SOCKET_DIR')
    .map(([, value]) => value);
}

function validateCampaignPlan(plan) {
  const { campaignPlanSha256, ...body } = plan ?? {};
  if (plan?.schemaVersion !== 'agent-browser.p158-dashboard-campaign-plan.v1' ||
      campaignPlanSha256 !== sha256(canonical(body)) || plan.productionHomeAccessAllowed !== false ||
      plan.repairAllowed !== false || plan.retryAllowed !== false ||
      plan.roots?.length !== plan.actionCount) {
    fail('campaign_plan_invalid', 'W8 dashboard campaign plan is missing, changed, or unsafe');
  }
}

/** Materialize every immutable root before the campaign freeze and never after it. */
export async function prepareP158DashboardCampaign({
  campaignPlan,
  preseedPlan,
  freezeState,
  apply = false,
  validateState,
}) {
  validateCampaignPlan(campaignPlan);
  if (freezeState !== 'pre_freeze') {
    fail('post_freeze_materialization_prohibited', 'Dashboard roots may only be materialized before freeze');
  }
  if (campaignPlan.preseedPlanSha256 !== preseedPlan?.planSha256 || typeof validateState !== 'function') {
    fail('preseed_binding_invalid', 'Campaign and preseed plans or installed parser adapter do not match');
  }
  const byAction = new Map(campaignPlan.roots.map((root) => [root.actionId, root]));
  const preseedReceipt = await materializeP158DashboardPreseedPlan({
    plan: preseedPlan,
    apply,
    validateState: async (request) => {
      const campaignRoot = byAction.get(request.root.actionId);
      const parserReceipt = await validateState({
        ...request,
        validationInputPath: campaignRoot.validationInputPath,
        candidate: structuredClone(campaignRoot.candidate),
        environment: structuredClone(campaignRoot.environment),
      });
      if (parserReceipt?.parserIdentitySha256 !== campaignRoot.candidate.executableSha256 ||
          parserReceipt?.classification !== 'accepted') {
        fail('parser_receipt_binding_invalid',
          'Installed parser receipt is not bound to the frozen candidate executable');
      }
      return parserReceipt;
    },
  });
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-preparation.v1',
    planId: 'P158',
    campaignPlanSha256: campaignPlan.campaignPlanSha256,
    preseedReceipt,
    candidateSha256: campaignPlan.candidate.executableSha256,
    materializedBeforeFreeze: apply,
    productionStateTouched: false,
    repairAttempted: false,
    retryAttempted: false,
  };
  return { ...body, receiptSha256: sha256(canonical(body)) };
}

/** Build the fixed D09 stream/API churn sequence without consulting wall time or random state. */
export function buildP158D09ChurnPlan({ root, cycleCount = 32 }) {
  if (root?.caseId !== 'D09' || !Number.isInteger(cycleCount) || cycleCount < 1 || cycleCount > 1000) {
    fail('churn_plan_invalid', 'D09 churn requires its own root and a bounded positive cycle count');
  }
  const operations = Array.from({ length: cycleCount }, (_, index) => ({
    ordinal: index + 1,
    kind: index % 2 === 0 ? 'authoritative_snapshot' : 'rendered_stream_refresh',
    expectedStreamState: root.streamState,
    correlationId: `${root.actionId}:churn:${String(index + 1).padStart(4, '0')}`,
  }));
  const body = {
    schemaVersion: 'agent-browser.p158-d09-active-churn-plan.v1',
    actionId: root.actionId,
    disposableRuntimeRootSha256: sha256(root.target.disposableRoot),
    cycleCount,
    operations,
    repairAllowed: false,
    retryAllowed: false,
  };
  return { ...body, churnPlanSha256: sha256(canonical(body)) };
}

/** Convert the single live correlation barrier into the immutable W5 oracle shape. */
export function buildP158DashboardFixtureFromProjection({
  projection,
  actionId,
  expectedState,
  materializationReceipt,
}) {
  if (projection?.schemaVersion !== 'agent-browser.p158-dashboard-live-projection.v1' ||
      typeof actionId !== 'string' || !actionId ||
      materializationReceipt?.stateSha256 !== sha256(canonical(expectedState)) ||
      sha256(materializationReceipt.counts) !== sha256(projection.counts)) {
    fail('dashboard_projection_invalid',
      'Dashboard fixture requires one live projection bound to independent sealed preseed truth');
  }
  const fixture = generateDenseDashboardFixture({
    ...projection.counts,
    idNamespace: `p158-live-${sha256(actionId).slice(0, 12)}`,
  });
  const snapshotRevision = projection.authoritativeSnapshotSha256;
  fixture.fixtureId = `p158-live-${sha256(actionId).slice(0, 20)}`;
  fixture.description = 'Plan 0158 externally captured dashboard correlation barrier.';
  fixture.density = projection.density;
  const profileTruth = Object.values(expectedState.profiles).map((profile, index) => ({
    resourceId: profile.id,
    resourceType: 'profile',
    label: profile.name,
    state: 'ready',
    rowExpected: true,
    orderKey: index,
    rowId: `row-${profile.id}`,
    badge: null,
    count: 0,
  }));
  const browserTruth = Object.values(expectedState.browsers).map((browser, index) => ({
    resourceId: browser.id,
    resourceType: 'browser',
    label: browser.id,
    state: browser.health,
    rowExpected: true,
    orderKey: profileTruth.length + index,
    rowId: `row-${browser.id}`,
    badge: null,
    count: 0,
  }));
  fixture.truth = {
    snapshotRevision,
    counts: structuredClone(materializationReceipt.counts),
    resources: [...profileTruth, ...browserTruth],
  };
  fixture.railRows = projection.capture.railRows.map((row) => ({
    ...structuredClone(row), snapshotRevision, badge: null, count: 0,
  }));
  const selectedResourceId = fixture.railRows[0]?.resourceId ?? null;
  fixture.selection = {
    selectedResourceId,
    inspectorResourceId: selectedResourceId,
    selectedExists: selectedResourceId !== null,
    recoveryActionCount: 1,
    deepLinkRequestedId: selectedResourceId,
    deepLinkResolvedId: selectedResourceId,
  };
  fixture.actions = [];
  fixture.warnings.displayedAxes = [];
  fixture.stream = {
    streamId: `${actionId}:stream`, snapshotRevision, streamRevision: snapshotRevision,
    displayedReady: true, authoritativeReady: true,
  };
  const durations = projection.capture.performance.map((entry) => entry.durationMs).filter(Number.isFinite);
  fixture.timings = [{ interaction: 'initial_load', samplesMs: durations, p95BudgetMs: 3000 }];
  const resourceSample = {
    heapBytes: 0,
    domNodeCount: projection.capture.domNodeCount,
    listenerCount: 0,
    cpuMilliseconds: 0,
    networkBytes: 0,
    longTaskCount: 0,
    browserProcessCount: projection.counts.browsers,
    xvfbProcessCount: 0,
    routeAllocationCount: 0,
    profileLeaseCount: projection.counts.profiles,
    retainedSessionCount: 0,
    unresolvedJobCount: 0,
  };
  fixture.resourceSamples = [
    { elapsedMs: 0, ...resourceSample },
    { elapsedMs: 60_000, ...resourceSample },
  ];
  return fixture;
}

function matchingPreseedReceipt(campaignPlan, preparation, root) {
  if (preparation?.schemaVersion !== 'agent-browser.p158-dashboard-campaign-preparation.v1' ||
      preparation.campaignPlanSha256 !== campaignPlan.campaignPlanSha256 ||
      preparation.materializedBeforeFreeze !== true || preparation.receiptSha256 !== receiptDigest(preparation)) {
    fail('preseed_receipt_invalid', 'Frozen W8 execution requires an intact pre-freeze preparation receipt');
  }
  const receipt = preparation.preseedReceipt?.receipts?.find((entry) => entry.actionId === root.actionId);
  if (!receipt?.written || receipt.parserReceipt?.accepted !== true ||
      receipt.parserReceipt.parserIdentitySha256 !== campaignPlan.candidate.executableSha256 ||
      receipt.parserReceipt.stateSha256 !== receipt.materializationReceipt?.stateSha256) {
    fail('preseed_receipt_invalid', `${root.actionId} lacks exact parser and materialization binding`);
  }
  return receipt;
}

/**
 * Run one exact frozen action. Lifecycle effects are injected so provider-free
 * tests can prove orchestration without starting a Service, browser, or route.
 */
export async function executeP158DashboardCampaignAction({
  campaignPlan,
  preparation,
  freezeState,
  actionId,
  effects,
}) {
  validateCampaignPlan(campaignPlan);
  if (freezeState !== 'frozen') fail('wrong_campaign_state', 'W8 execution requires the frozen campaign state');
  const root = campaignPlan.roots.find((entry) => entry.actionId === actionId);
  if (!root) fail('action_not_planned', 'W8 dashboard action is not present in the frozen campaign plan');
  const preseed = matchingPreseedReceipt(campaignPlan, preparation, root);
  const sealedPreseed = buildP158DashboardServiceState({ target: root.target, density: root.density });
  if (sealedPreseed.receipt.receiptSha256 !== preseed.materializationReceipt.receiptSha256) {
    fail('preseed_receipt_invalid', `${root.actionId} deterministic preseed truth changed after freeze`);
  }
  for (const name of ['startExact', 'selectExternalIngress', 'openExternalPage', 'stopExact']) {
    if (typeof effects?.[name] !== 'function') fail('lifecycle_effect_missing', `W8 dashboard effect ${name} is required`);
  }
  let started = null;
  let selected = null;
  let pageHandle = null;
  let churnReceipt = null;
  let projection = null;
  let dashboardFixture = null;
  let oracleBinding = null;
  let firstFailure = null;
  let teardown = { attempted: false, state: 'not_started', pid: null, failure: null };
  try {
    started = await effects.startExact(structuredClone(root));
    if (started?.state !== 'ready' || started?.candidateSha256 !== root.candidate.executableSha256 ||
        !Number.isInteger(started?.pid) || started.pid < 1 || started?.statePath !== root.target.statePath) {
      fail('wrong_runtime_state', 'Started dashboard instance is not the exact frozen root and candidate');
    }
    if (!Number.isInteger(started.backendPid) || started.backendPid < 1 ||
        !Number.isInteger(started.runtimeHostPid) || started.runtimeHostPid < 1 ||
        ['ingress', 'backend', 'runtimeHost'].some((role) =>
          started.processIdentities?.[role]?.executableSha256 !== root.candidate.executableSha256 ||
          typeof started.processIdentities?.[role]?.startToken !== 'string')) {
      fail('wrong_runtime_state', 'Service host and dashboard process identities are incomplete or foreign');
    }
    selected = await effects.selectExternalIngress({
      actionId: root.actionId,
      reviewedRevision: root.externalIngress.reviewedRevision,
      bindingSha256: root.externalIngress.bindingSha256,
      dashboardPort: root.ports.dashboardIngress,
      dashboardBackendPort: root.ports.dashboardBackend,
      runtimeStreamPort: root.ports.runtimeStream,
      runtimeRootSha256: sha256(root.target.disposableRoot),
      expectedPid: started.pid,
      expectedBackendPid: started.backendPid,
      expectedRuntimeHostPid: started.runtimeHostPid,
      processIdentitySha256: sha256(started.processIdentities),
    });
    const publicUrl = new URL(selected?.publicUrl ?? 'invalid:');
    if (publicUrl.protocol !== 'https:' || publicUrl.origin !== root.externalIngress.publicOperatorUrl ||
        publicUrl.search || publicUrl.hash || selected?.publicPath !== publicUrl.pathname ||
        !selected.publicPath.startsWith('/p158/') || selected?.bindingSha256 !== root.externalIngress.bindingSha256 ||
        selected?.selected !== true || selected?.actionId !== root.actionId ||
        selected?.runtimeRootSha256 !== sha256(root.target.disposableRoot) ||
        selected?.dashboardPort !== root.ports.dashboardIngress ||
        selected?.dashboardBackendPort !== root.ports.dashboardBackend ||
        selected?.runtimeStreamPort !== root.ports.runtimeStream ||
        selected?.expectedPid !== started.pid || selected?.expectedBackendPid !== started.backendPid ||
        selected?.expectedRuntimeHostPid !== started.runtimeHostPid ||
        selected?.processIdentitySha256 !== sha256(started.processIdentities) ||
        selected?.reviewedRevision !== root.externalIngress.reviewedRevision ||
        selected?.selectionReceiptSha256 !== receiptDigest(selected, 'selectionReceiptSha256')) {
      fail('external_ingress_selection_invalid', 'Reviewed ingress did not select the exact dashboard root');
    }
    pageHandle = await effects.openExternalPage({ publicUrl: publicUrl.href, root: structuredClone(root) });
    if (root.caseId === 'D09') {
      if (typeof effects.produceChurn !== 'function') fail('lifecycle_effect_missing', 'D09 requires active churn');
      const churnPlan = buildP158D09ChurnPlan({ root });
      churnReceipt = await effects.produceChurn({ page: pageHandle.page, root: structuredClone(root), churnPlan });
      if (churnReceipt?.blocked === true) {
        fail('d09_state_churn_seam_missing', churnReceipt.detail ??
          'D09 requires a declared lock-respecting Service state churn seam');
      }
      if (churnReceipt?.churnPlanSha256 !== churnPlan.churnPlanSha256 ||
          churnReceipt?.completedOperationCount !== churnPlan.cycleCount || churnReceipt?.retryAttempted !== false) {
        fail('wrong_runtime_state', 'D09 churn did not complete the exact declared sequence');
      }
    }
    projection = await captureP158DashboardLiveProjection({
      page: pageHandle.page,
      materializationReceipt: preseed.materializationReceipt,
      externalProof: selected.externalProof,
      screenshotPath: root.screenshotPath,
    });
    dashboardFixture = buildP158DashboardFixtureFromProjection({
      projection,
      actionId: root.actionId,
      expectedState: sealedPreseed.state,
      materializationReceipt: preseed.materializationReceipt,
    });
    oracleBinding = auditP158DashboardLiveProjection({ projection, dashboardFixture });
    if (!oracleBinding.passed) fail('dashboard_oracle_failed', 'Externally captured dashboard oracle did not pass');
  } catch (error) {
    firstFailure = errorRecord(error);
  } finally {
    if (pageHandle?.close) {
      try {
        await pageHandle.close();
      } catch (error) {
        firstFailure ??= errorRecord(error);
      }
    }
    if (started) {
      teardown = { attempted: true, state: 'started', pid: started.pid ?? null, failure: null };
      try {
        const stopped = await effects.stopExact({
          actionId: root.actionId,
          expectedPid: started.pid,
          environment: structuredClone(root.environment),
          dashboardPort: root.ports.dashboardIngress,
          statePath: root.target.statePath,
          processIdentities: structuredClone(started.processIdentities),
        });
        if (stopped?.state !== 'stopped' || stopped?.pid !== started.pid ||
            stopped?.backendPid !== started.backendPid || stopped?.runtimeHostPid !== started.runtimeHostPid) {
          fail('wrong_runtime_state', 'Exact W8 dashboard instance did not stop cleanly');
        }
        teardown = {
          attempted: true,
          state: 'stopped',
          pid: stopped.pid,
          backendPid: stopped.backendPid,
          runtimeHostPid: stopped.runtimeHostPid,
          processIdentitySha256: sha256(started.processIdentities),
          failure: null,
        };
      } catch (error) {
        teardown = { attempted: true, state: 'failed', pid: started.pid ?? null, failure: errorRecord(error) };
        firstFailure ??= errorRecord(error);
      }
    }
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-action-receipt.v1',
    planId: 'P158',
    actionId: root.actionId,
    attemptId: root.attemptId,
    caseId: root.caseId,
    environmentId: root.environmentId,
    candidateSha256: root.candidate.executableSha256,
    parserReceiptSha256: sha256(preseed.parserReceipt),
    materializationReceiptSha256: preseed.materializationReceipt.receiptSha256,
    externalIngressBindingSha256: root.externalIngress.bindingSha256,
    projection,
    dashboardFixture,
    oracleBinding,
    churnReceipt,
    firstFailure,
    teardown,
    terminalState: 'completed',
    resultState: firstFailure === null && teardown.state === 'stopped'
      ? 'passed'
      : firstFailure?.code === 'd09_state_churn_seam_missing' ? 'skipped_blocked' : 'harness_failure',
    productionStateTouched: false,
    repairAttempted: false,
    retryAttempted: false,
  };
  return { ...body, receiptSha256: sha256(canonical(body)) };
}

/** Aggregate exact D01/D09 action receipts without re-running failed actions. */
export function aggregateP158DashboardCampaignReceipts({ campaignPlan, receipts }) {
  validateCampaignPlan(campaignPlan);
  const expected = campaignPlan.roots.map((root) => root.actionId).sort();
  const observed = (receipts ?? []).map((receipt) => receipt.actionId).sort();
  if (sha256(expected) !== sha256(observed) || receipts.some((receipt) =>
    receipt.receiptSha256 !== receiptDigest(receipt) || receipt.terminalState !== 'completed' ||
    receipt.candidateSha256 !== campaignPlan.candidate.executableSha256)) {
    fail('action_receipt_set_invalid', 'Dashboard campaign receipts are missing, changed, nonterminal, or foreign');
  }
  const passedCount = receipts.filter((receipt) => receipt.resultState === 'passed').length;
  const failedCount = receipts.length - passedCount;
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-aggregate.v1',
    planId: 'P158',
    campaignPlanSha256: campaignPlan.campaignPlanSha256,
    candidateSha256: campaignPlan.candidate.executableSha256,
    actionCount: receipts.length,
    actionIds: expected,
    receiptSha256s: receipts.map((receipt) => receipt.receiptSha256).sort(),
    resultCounts: { passed: passedCount, failed: failedCount },
    success: failedCount === 0,
    repairAttempted: false,
    retryCount: 0,
  };
  return { ...body, aggregateSha256: sha256(canonical(body)) };
}

/** Decide whether an action is new, terminally reusable, or effect-uncertain after process loss. */
export function resolveP158DashboardActionResume({ campaignPlan, actionId, claim = null, receipt = null }) {
  validateCampaignPlan(campaignPlan);
  const root = campaignPlan.roots.find((entry) => entry.actionId === actionId);
  if (!root) fail('action_not_planned', 'Resume action is not present in the immutable campaign plan');
  if (receipt) {
    if (receipt.actionId !== actionId || receipt.candidateSha256 !== campaignPlan.candidate.executableSha256 ||
        receipt.terminalState !== 'completed' || receipt.receiptSha256 !== receiptDigest(receipt)) {
      fail('action_receipt_set_invalid', 'Existing action receipt is changed, foreign, or nonterminal');
    }
    return { disposition: 'reuse_terminal', receipt: structuredClone(receipt) };
  }
  if (claim) {
    if (claim.actionId !== actionId || claim.campaignPlanSha256 !== campaignPlan.campaignPlanSha256) {
      fail('action_claim_invalid', 'Existing action claim is foreign or changed');
    }
    fail('action_effect_uncertain', 'Action was claimed without a terminal receipt and must not be replayed');
  }
  return { disposition: 'execute_once', receipt: null };
}

export function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}
