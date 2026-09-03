import { sha256 } from './p158-campaign-controller.js';
import { aggregateP158DashboardCampaignReceipts } from './p158-w8-dashboard-campaign.js';
import {
  buildP158DashboardExternalManifest,
  validateP158DashboardExternalManifest,
  validateP158DashboardExternalResult,
} from './p158-w8-dashboard-external.js';
import { buildP158DashboardServiceState } from './p158-w8-dashboard-live.js';
import { buildP158DashboardScenarioPlan } from './p158-w8-dashboard-scenarios.js';

const COMMIT = /^[a-f0-9]{40}$/u;

export class P158W8DashboardHostHandshakeError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'P158W8DashboardHostHandshakeError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new P158W8DashboardHostHandshakeError(code, message);
}

function without(value, field) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== field));
}

function errorRecord(error, fallback = 'dashboard_host_handshake_failed') {
  return {
    code: error?.code ?? fallback,
    message: error instanceof Error ? error.message : String(error),
  };
}

function validateInputs({ campaignPlan, preparation, freezeState, actionId, expectedCommit }) {
  const { campaignPlanSha256, ...campaignBody } = campaignPlan ?? {};
  if (campaignPlan?.schemaVersion !== 'agent-browser.p158-dashboard-campaign-plan.v1' ||
      campaignPlanSha256 !== sha256(campaignBody) || freezeState !== 'frozen' ||
      !COMMIT.test(expectedCommit ?? '')) {
    fail('host_handshake_input_invalid', 'Host handshake requires the exact frozen campaign and reviewed commit');
  }
  const root = campaignPlan.roots.find((entry) => entry.actionId === actionId);
  const preseed = preparation?.preseedReceipt?.receipts?.find((entry) => entry.actionId === actionId);
  if (!root || !['D03', 'D04', 'D05'].includes(root.caseId) ||
      preparation?.campaignPlanSha256 !== campaignPlanSha256 ||
      preparation?.receiptSha256 !== sha256(without(preparation, 'receiptSha256')) ||
      preparation.materializedBeforeFreeze !== true || !preseed?.written ||
      preseed.parserReceipt?.accepted !== true ||
      preseed.parserReceipt?.parserIdentitySha256 !== root.candidate.executableSha256) {
    fail('host_handshake_input_invalid', 'Host handshake action lacks exact parser-bound pre-freeze materialization');
  }
  const sealed = buildP158DashboardServiceState({
    target: root.target, density: root.density, scenario: root.scenario,
  });
  if (sealed.receipt.receiptSha256 !== preseed.materializationReceipt?.receiptSha256) {
    fail('host_handshake_input_invalid', 'Host handshake preseed changed after freeze');
  }
  const scenarioPlan = buildP158DashboardScenarioPlan({
    root, expectedState: sealed.state, materializationReceipt: sealed.receipt,
  });
  if (scenarioPlan.scenarioTruth.executable === false) {
    fail('host_handshake_action_blocked', 'Unsupported dashboard actions cannot enter the external handshake');
  }
  return { root, preseed, sealed, scenarioPlan };
}

function validateStarted(root, started) {
  if (started?.state !== 'ready' || started.candidateSha256 !== root.candidate.executableSha256 ||
      started.statePath !== root.target.statePath ||
      !Number.isInteger(started.pid) || !Number.isInteger(started.backendPid) ||
      !Number.isInteger(started.runtimeHostPid) || ['ingress', 'backend', 'runtimeHost'].some((role) => {
        const identity = started.processIdentities?.[role];
        const expectedPid = role === 'ingress' ? started.pid
          : role === 'backend' ? started.backendPid : started.runtimeHostPid;
        return identity?.pid !== expectedPid || typeof identity.startToken !== 'string' ||
          identity.executableSha256 !== root.candidate.executableSha256 ||
          typeof identity.executablePath !== 'string';
      })) {
    fail('host_runtime_identity_invalid', 'Started host runtime is incomplete, foreign, or not bound to the frozen candidate');
  }
}

function validateSelection(root, started, selected) {
  let publicUrl;
  try { publicUrl = new URL(selected?.publicUrl); } catch {
    fail('host_ingress_identity_invalid', 'Selected host ingress URL is invalid');
  }
  if (publicUrl.protocol !== 'https:' || selected.selected !== true || selected.actionId !== root.actionId ||
      selected.publicPath !== publicUrl.pathname || !selected.publicPath.startsWith('/p158/') ||
      selected.bindingSha256 !== root.externalIngress.bindingSha256 ||
      selected.reviewedRevision !== root.externalIngress.reviewedRevision ||
      selected.runtimeRootSha256 !== sha256(root.target.disposableRoot) ||
      selected.dashboardPort !== root.ports.dashboardIngress ||
      selected.dashboardBackendPort !== root.ports.dashboardBackend ||
      selected.runtimeStreamPort !== root.ports.runtimeStream ||
      selected.expectedPid !== started.pid || selected.expectedBackendPid !== started.backendPid ||
      selected.expectedRuntimeHostPid !== started.runtimeHostPid ||
      selected.processIdentitySha256 !== sha256(started.processIdentities) ||
      selected.selectionReceiptSha256 !== sha256(without(selected, 'selectionReceiptSha256'))) {
    fail('host_ingress_identity_invalid', 'Selected ingress does not bind the exact action root, ports, and processes');
  }
  return publicUrl.href;
}

function validateStopped(stopped, processIdentities) {
  if (stopped?.state !== 'stopped' || stopped.pid !== processIdentities.ingress.pid ||
      stopped.backendPid !== processIdentities.backend.pid ||
      stopped.runtimeHostPid !== processIdentities.runtimeHost.pid) {
    fail('host_teardown_failed', 'Exact paused host runtime did not stop cleanly');
  }
}

function checkpointBody({ campaignPlan, root, preseed, expectedCommit, started, selected, externalManifest }) {
  return {
    schemaVersion: 'agent-browser.p158-dashboard-host-dispatch-ready.v1',
    planId: 'P158',
    state: 'dispatch_ready',
    actionId: root.actionId,
    attemptId: root.attemptId,
    caseId: root.caseId,
    expectedCommit,
    campaignPlanSha256: campaignPlan.campaignPlanSha256,
    candidateSha256: root.candidate.executableSha256,
    parserReceiptSha256: sha256(preseed.parserReceipt),
    materializationReceiptSha256: preseed.materializationReceipt.receiptSha256,
    runtimeRootSha256: sha256(root.target.disposableRoot),
    statePathSha256: sha256(root.target.statePath),
    environmentSha256: sha256(root.environment),
    ports: structuredClone(root.ports),
    processIdentities: structuredClone(started.processIdentities),
    ingress: {
      publicUrlSha256: sha256(selected.publicUrl),
      publicPath: selected.publicPath,
      reviewedRevision: selected.reviewedRevision,
      bindingSha256: selected.bindingSha256,
      selectionReceiptSha256: selected.selectionReceiptSha256,
      processIdentitySha256: selected.processIdentitySha256,
    },
    externalManifestSha256: externalManifest.manifestSha256,
    automaticDispatchAllowed: false,
    retryAllowed: false,
    repairAllowed: false,
    garbageCollectionAllowed: false,
  };
}

function validateCheckpoint(checkpoint, inputs) {
  const { checkpointSha256, ...body } = checkpoint ?? {};
  const resolved = validateInputs(inputs);
  const { root, preseed } = resolved;
  validateP158DashboardExternalManifest(inputs.externalManifest);
  if (checkpoint?.schemaVersion !== 'agent-browser.p158-dashboard-host-dispatch-ready.v1' ||
      checkpoint.state !== 'dispatch_ready' || checkpointSha256 !== sha256(body) ||
      checkpoint.actionId !== root.actionId || checkpoint.expectedCommit !== inputs.expectedCommit ||
      checkpoint.campaignPlanSha256 !== inputs.campaignPlan.campaignPlanSha256 ||
      checkpoint.candidateSha256 !== root.candidate.executableSha256 ||
      checkpoint.runtimeRootSha256 !== sha256(root.target.disposableRoot) ||
      checkpoint.statePathSha256 !== sha256(root.target.statePath) ||
      checkpoint.environmentSha256 !== sha256(root.environment) ||
      sha256(checkpoint.ports) !== sha256(root.ports) ||
      checkpoint.parserReceiptSha256 !== sha256(preseed.parserReceipt) ||
      checkpoint.materializationReceiptSha256 !== preseed.materializationReceipt.receiptSha256 ||
      checkpoint.externalManifestSha256 !== inputs.externalManifest?.manifestSha256 ||
      inputs.externalManifest?.actionId !== root.actionId ||
      inputs.externalManifest?.expectedCommit !== inputs.expectedCommit ||
      inputs.externalManifest?.campaignPlanSha256 !== inputs.campaignPlan.campaignPlanSha256 ||
      inputs.externalManifest?.candidateSha256 !== root.candidate.executableSha256 ||
      inputs.externalManifest?.publicUrlSha256 !== checkpoint.ingress.publicUrlSha256 ||
      inputs.externalManifest?.publicPath !== checkpoint.ingress.publicPath ||
      inputs.externalManifest?.selectionReceiptSha256 !== checkpoint.ingress.selectionReceiptSha256 ||
      inputs.externalManifest?.materializationReceipt?.receiptSha256 !== checkpoint.materializationReceiptSha256) {
    fail('host_checkpoint_invalid', 'Dispatch-ready checkpoint is missing, changed, or bound to another root');
  }
  return resolved;
}

function runtimeObservationMatches(checkpoint, observation) {
  return observation?.unchanged === true &&
    sha256(observation.processIdentities) === sha256(checkpoint.processIdentities) &&
    observation.runtimeRootSha256 === checkpoint.runtimeRootSha256 &&
    observation.statePathSha256 === checkpoint.statePathSha256 &&
    sha256(observation.ports) === sha256(checkpoint.ports) &&
    observation.candidateSha256 === checkpoint.candidateSha256;
}

function ingressObservationMatches(checkpoint, observation) {
  return observation?.unchanged === true && observation.publicUrlSha256 === checkpoint.ingress.publicUrlSha256 &&
    observation.publicPath === checkpoint.ingress.publicPath &&
    observation.reviewedRevision === checkpoint.ingress.reviewedRevision &&
    observation.bindingSha256 === checkpoint.ingress.bindingSha256 &&
    observation.selectionReceiptSha256 === checkpoint.ingress.selectionReceiptSha256 &&
    observation.processIdentitySha256 === checkpoint.ingress.processIdentitySha256;
}

function uncertainTerminal(checkpoint, detail) {
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-host-terminal.v1',
    planId: 'P158', actionId: checkpoint.actionId, attemptId: checkpoint.attemptId,
    caseId: checkpoint.caseId, checkpointSha256: checkpoint.checkpointSha256,
    terminalState: 'completed', hostState: 'effect_uncertain', resultState: 'harness_failure',
    firstFailure: { code: 'action_effect_uncertain', message: detail },
    teardown: { attempted: false, state: 'identity_lost', failure: null },
    automaticDispatchAttempted: false, retryAttempted: false, repairAttempted: false,
    garbageCollectionAttempted: false,
  };
  return { ...body, receiptSha256: sha256(body) };
}

/** Start and expose one exact host runtime, returning a safe append-only dispatch checkpoint. */
export async function pauseP158DashboardHostAction(inputs) {
  const { campaignPlan, expectedCommit, effects } = inputs;
  const { root, preseed, sealed, scenarioPlan } = validateInputs(inputs);
  for (const name of ['startExact', 'selectExternalIngress', 'persistDispatchReady', 'stopExact']) {
    if (typeof effects?.[name] !== 'function') fail('host_effect_missing', `Host handshake requires ${name}`);
  }
  let started = null;
  let firstFailure = null;
  try {
    started = await effects.startExact(structuredClone(root));
    validateStarted(root, started);
    const selected = await effects.selectExternalIngress({
      operation: 'select', actionId: root.actionId,
      reviewedRevision: root.externalIngress.reviewedRevision,
      bindingSha256: root.externalIngress.bindingSha256,
      dashboardPort: root.ports.dashboardIngress,
      dashboardBackendPort: root.ports.dashboardBackend,
      runtimeStreamPort: root.ports.runtimeStream,
      runtimeRootSha256: sha256(root.target.disposableRoot),
      expectedPid: started.pid, expectedBackendPid: started.backendPid,
      expectedRuntimeHostPid: started.runtimeHostPid,
      processIdentitySha256: sha256(started.processIdentities),
    });
    const publicUrl = validateSelection(root, started, selected);
    const externalManifest = buildP158DashboardExternalManifest({
      expectedCommit,
      campaignPlanSha256: campaignPlan.campaignPlanSha256,
      candidateSha256: root.candidate.executableSha256,
      scenarioPlan,
      expectedState: sealed.state,
      materializationReceipt: sealed.receipt,
      publicUrlSha256: sha256(publicUrl),
      publicPath: selected.publicPath,
      selectionReceiptSha256: selected.selectionReceiptSha256,
    });
    const body = checkpointBody({
      campaignPlan, root, preseed, expectedCommit, started, selected, externalManifest,
    });
    const checkpoint = { ...body, checkpointSha256: sha256(body) };
    await effects.persistDispatchReady({
      checkpoint: structuredClone(checkpoint),
      externalManifest: structuredClone(externalManifest),
    });
    return {
      schemaVersion: 'agent-browser.p158-dashboard-host-pause.v1',
      checkpoint,
      externalManifest,
      state: 'dispatch_ready',
      automaticDispatchAttempted: false,
      retryAttempted: false,
    };
  } catch (error) {
    firstFailure = errorRecord(error);
    let teardown = { attempted: false, state: 'not_started', failure: null };
    if (started) {
      teardown = { attempted: true, state: 'failed', failure: null };
      try {
        const stopped = await effects.stopExact({
          actionId: root.actionId, expectedPid: started.pid, environment: structuredClone(root.environment),
          dashboardPort: root.ports.dashboardIngress, statePath: root.target.statePath,
          processIdentities: structuredClone(started.processIdentities),
        });
        validateStopped(stopped, started.processIdentities);
        teardown.state = 'stopped';
      } catch (stopError) {
        teardown.failure = errorRecord(stopError, 'host_teardown_failed');
      }
    }
    const body = {
      schemaVersion: 'agent-browser.p158-dashboard-host-terminal.v1', planId: 'P158',
      actionId: root.actionId, attemptId: root.attemptId, caseId: root.caseId,
      terminalState: 'completed', hostState: 'pause_failed', resultState: 'harness_failure',
      firstFailure, teardown, automaticDispatchAttempted: false, retryAttempted: false,
      repairAttempted: false, garbageCollectionAttempted: false,
    };
    return { ...body, receiptSha256: sha256(body) };
  }
}

/** Resume one paused action without replay, then validate one external receipt and tear down exactly. */
export async function resumeP158DashboardHostAction({
  ...inputs
}) {
  const { checkpoint, externalManifest, externalResult = null, expectedWorkflowRunId = null,
    expectedWorkflowRunAttempt = null, effects } = inputs;
  const { root, preseed } = validateCheckpoint(checkpoint, inputs);
  for (const name of ['observeExactRuntime', 'observeExactIngress', 'stopExact']) {
    if (typeof effects?.[name] !== 'function') fail('host_effect_missing', `Host resume requires ${name}`);
  }
  let runtimeObservation;
  try {
    runtimeObservation = await effects.observeExactRuntime({
      checkpoint: structuredClone(checkpoint), root: structuredClone(root),
    });
  } catch {
    return uncertainTerminal(checkpoint, 'Claimed action runtime identity could not be observed');
  }
  if (!runtimeObservationMatches(checkpoint, runtimeObservation)) {
    return uncertainTerminal(checkpoint, 'Claimed action lost exact PID, start token, executable, root, state, or port identity');
  }
  let ingressObservation;
  try {
    ingressObservation = await effects.observeExactIngress({
      operation: 'observe', checkpoint: structuredClone(checkpoint), root: structuredClone(root),
    });
  } catch {
    return uncertainTerminal(checkpoint, 'Claimed action ingress identity could not be observed');
  }
  if (!ingressObservationMatches(checkpoint, ingressObservation)) {
    return uncertainTerminal(checkpoint, 'Claimed action lost its exact reviewed public ingress identity');
  }
  if (externalResult === null) {
    return {
      schemaVersion: 'agent-browser.p158-dashboard-host-awaiting-external.v1',
      actionId: root.actionId,
      checkpointSha256: checkpoint.checkpointSha256,
      state: 'awaiting_external_receipt',
      automaticDispatchAttempted: false,
      retryAttempted: false,
      repairAttempted: false,
    };
  }
  let firstFailure = null;
  try {
    validateP158DashboardExternalResult({ result: externalResult, manifest: externalManifest });
    if (!/^\d+$/u.test(expectedWorkflowRunId ?? '') ||
        !Number.isInteger(expectedWorkflowRunAttempt) || expectedWorkflowRunAttempt < 1 ||
        externalResult.runnerAttestation?.runIdSha256 !== sha256(expectedWorkflowRunId) ||
        externalResult.runnerAttestation?.runAttempt !== expectedWorkflowRunAttempt ||
        externalResult.projection?.stateSha256 !== preseed.materializationReceipt.stateSha256 ||
        externalResult.oracleBinding?.passed !== true) {
      fail('external_receipt_binding_invalid', 'External receipt is not bound to the action, root, commit, run, attempt, or oracle');
    }
  } catch (error) {
    firstFailure = errorRecord(error, 'external_receipt_binding_invalid');
  }
  let teardown = { attempted: true, state: 'failed', failure: null };
  try {
    const stopped = await effects.stopExact({
      actionId: root.actionId,
      expectedPid: checkpoint.processIdentities.ingress.pid,
      environment: structuredClone(root.environment),
      dashboardPort: root.ports.dashboardIngress,
      statePath: root.target.statePath,
      processIdentities: structuredClone(checkpoint.processIdentities),
    });
    validateStopped(stopped, checkpoint.processIdentities);
    teardown = { attempted: true, state: 'stopped', pid: stopped.pid,
      backendPid: stopped.backendPid, runtimeHostPid: stopped.runtimeHostPid, failure: null };
  } catch (error) {
    teardown.failure = errorRecord(error, 'host_teardown_failed');
    firstFailure ??= teardown.failure;
  }
  if (firstFailure) {
    const body = {
      schemaVersion: 'agent-browser.p158-dashboard-host-terminal.v1', planId: 'P158',
      actionId: root.actionId, attemptId: root.attemptId, caseId: root.caseId,
      checkpointSha256: checkpoint.checkpointSha256, terminalState: 'completed',
      hostState: 'failed', resultState: 'harness_failure', firstFailure, teardown,
      automaticDispatchAttempted: false, retryAttempted: false, repairAttempted: false,
      garbageCollectionAttempted: false,
    };
    return { ...body, receiptSha256: sha256(body) };
  }
  const body = {
    schemaVersion: 'agent-browser.p158-dashboard-campaign-action-receipt.v1',
    planId: 'P158', actionId: root.actionId, attemptId: root.attemptId, caseId: root.caseId,
    environmentId: root.environmentId, candidateSha256: root.candidate.executableSha256,
    parserReceiptSha256: checkpoint.parserReceiptSha256,
    materializationReceiptSha256: checkpoint.materializationReceiptSha256,
    externalIngressBindingSha256: checkpoint.ingress.bindingSha256,
    projection: externalResult.projection,
    dashboardFixture: externalResult.dashboardFixture,
    oracleBinding: externalResult.oracleBinding,
    scenarioPlanSha256: externalManifest.scenarioPlan.scenarioPlanSha256,
    scenarioReceipt: externalResult.scenarioReceipt,
    scenarioOracle: externalResult.scenarioOracle,
    externalResultSha256: externalResult.resultSha256,
    externalWorkflowRunIdSha256: sha256(expectedWorkflowRunId),
    externalWorkflowRunAttempt: expectedWorkflowRunAttempt,
    churnReceipt: null, firstFailure: null, teardown, terminalState: 'completed', resultState: 'passed',
    productionStateTouched: false, automaticDispatchAttempted: false, repairAttempted: false,
    retryAttempted: false, garbageCollectionAttempted: false,
  };
  return { ...body, receiptSha256: sha256(body) };
}

/** Build the existing W8 aggregate shape from exact successful handshake terminals. */
export function aggregateP158DashboardHostReceipts({ campaignPlan, receipts }) {
  return aggregateP158DashboardCampaignReceipts({ campaignPlan, receipts });
}

/** Produce the result-file shape consumed by the reviewed W8 adapter and campaign assembly. */
export function buildP158DashboardHostCampaignExecution({ campaignPlan, receipts }) {
  return {
    receipts: structuredClone(receipts),
    aggregate: aggregateP158DashboardHostReceipts({ campaignPlan, receipts }),
  };
}
