import { createHash } from 'node:crypto';
import { isAbsolute, join, normalize } from 'node:path';

import { developmentExternalIngressBinding } from './development-presentation-provider.js';
import { sha256 } from './p158-campaign-controller.js';
import {
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
      basePort + (preseedPlan.actionCount * 3) > 65535) {
    fail('campaign_plan_invalid', 'W8 dashboard campaign inputs are missing, changed, or outside the port range');
  }
  const roots = preseedPlan.roots.map((root, index) => {
    const runtimeRoot = normalize(root.target.disposableRoot);
    const xdgRoot = join(runtimeRoot, 'xdg');
    const runtimePort = basePort + (index * 3);
    const dashboardPort = runtimePort + 1;
    const streamPort = runtimePort + 2;
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
        AGENT_BROWSER_STREAM_PORT: String(streamPort),
        AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
      },
      ports: { runtime: runtimePort, dashboard: dashboardPort, stream: streamPort },
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
  for (const name of ['startExact', 'selectExternalIngress', 'openExternalPage', 'stopExact']) {
    if (typeof effects?.[name] !== 'function') fail('lifecycle_effect_missing', `W8 dashboard effect ${name} is required`);
  }
  let started = null;
  let selected = null;
  let pageHandle = null;
  let churnReceipt = null;
  let projection = null;
  let firstFailure = null;
  let teardown = { attempted: false, state: 'not_started', pid: null, failure: null };
  try {
    started = await effects.startExact(structuredClone(root));
    if (started?.state !== 'ready' || started?.candidateSha256 !== root.candidate.executableSha256 ||
        !Number.isInteger(started?.pid) || started.pid < 1 || started?.statePath !== root.target.statePath) {
      fail('wrong_runtime_state', 'Started dashboard instance is not the exact frozen root and candidate');
    }
    selected = await effects.selectExternalIngress({
      actionId: root.actionId,
      reviewedRevision: root.externalIngress.reviewedRevision,
      bindingSha256: root.externalIngress.bindingSha256,
      dashboardPort: root.ports.dashboard,
      runtimeRootSha256: sha256(root.target.disposableRoot),
      expectedPid: started.pid,
    });
    const publicUrl = new URL(selected?.publicUrl ?? 'invalid:');
    if (publicUrl.protocol !== 'https:' || publicUrl.origin !== root.externalIngress.publicOperatorUrl ||
        publicUrl.search || publicUrl.hash || selected?.publicPath !== publicUrl.pathname ||
        !selected.publicPath.startsWith('/p158/') || selected?.bindingSha256 !== root.externalIngress.bindingSha256 ||
        selected?.selected !== true || selected?.actionId !== root.actionId ||
        selected?.runtimeRootSha256 !== sha256(root.target.disposableRoot) ||
        selected?.dashboardPort !== root.ports.dashboard || selected?.expectedPid !== started.pid ||
        selected?.reviewedRevision !== root.externalIngress.reviewedRevision ||
        selected?.selectionReceiptSha256 !== receiptDigest(selected, 'selectionReceiptSha256')) {
      fail('external_ingress_selection_invalid', 'Reviewed ingress did not select the exact dashboard root');
    }
    pageHandle = await effects.openExternalPage({ publicUrl: publicUrl.href, root: structuredClone(root) });
    if (root.caseId === 'D09') {
      if (typeof effects.produceChurn !== 'function') fail('lifecycle_effect_missing', 'D09 requires active churn');
      const churnPlan = buildP158D09ChurnPlan({ root });
      churnReceipt = await effects.produceChurn({ page: pageHandle.page, root: structuredClone(root), churnPlan });
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
          dashboardPort: root.ports.dashboard,
          statePath: root.target.statePath,
        });
        if (stopped?.state !== 'stopped' || stopped?.pid !== started.pid) {
          fail('wrong_runtime_state', 'Exact W8 dashboard instance did not stop cleanly');
        }
        teardown = { attempted: true, state: 'stopped', pid: stopped.pid, failure: null };
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
    churnReceipt,
    firstFailure,
    teardown,
    terminalState: 'completed',
    resultState: firstFailure === null && teardown.state === 'stopped' ? 'passed' : 'harness_failure',
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

export function sha256Bytes(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}
