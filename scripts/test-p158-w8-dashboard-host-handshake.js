#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtemp, rm } from 'node:fs/promises';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  buildP158DashboardCampaignPlan,
  prepareP158DashboardCampaign,
} from './lib/p158-w8-dashboard-campaign.js';
import {
  aggregateP158DashboardHostReceipts,
  buildP158DashboardHostCampaignExecution,
  pauseP158DashboardHostAction,
  resumeP158DashboardHostAction,
} from './lib/p158-w8-dashboard-host-handshake.js';
import { buildP158DashboardIngressSelectorRequest } from './lib/p158-w8-dashboard-campaign.js';
import { buildP158DashboardGithubRunnerAttestation, sealP158DashboardExternalResult } from './lib/p158-w8-dashboard-external.js';
import { buildP158DashboardPreseedPlan } from './lib/p158-w8-dashboard-live.js';
import { sealP158DashboardScenarioReceipt } from './lib/p158-w8-dashboard-scenarios.js';

const candidate = {
  executablePath: '/tmp/p158-frozen-candidate/agent-browser',
  executableSha256: sha256('p158-frozen-candidate'),
};
const externalIngress = {
  publicOperatorUrl: 'https://p158-dashboard.example.test',
  reviewedRevision: 'p158-w8-reviewed-001',
};
const ingressSelector = {
  executablePath: '/tmp/p158-ingress-selector',
  executableSha256: sha256('p158-ingress-selector'),
  sourcePath: '/tmp/p158-ingress-selector.py',
  sourceSha256: sha256('p158-ingress-selector-source'),
  sourceCommit: 'e70368d',
};
const expectedCommit = 'a'.repeat(40);
const workflowRunId = '1588003';
const workflowRunAttempt = 1;

function processIdentities(root) {
  return {
    ingress: { pid: 7101, startToken: '101', executablePath: candidate.executablePath, executableSha256: root.candidate.executableSha256 },
    backend: { pid: 7102, startToken: '102', executablePath: candidate.executablePath, executableSha256: root.candidate.executableSha256 },
    runtimeHost: { pid: 7103, startToken: '103', executablePath: candidate.executablePath, executableSha256: root.candidate.executableSha256 },
  };
}

function selectorResponse(operation, request, deployed = null) {
  const identity = {
    schemaVersion: 'cooper.p158-action-route-selector.v1',
    runIdSha256: sha256(request.runId),
    actionIdSha256: sha256(request.actionId),
    publicPath: `/p158/${encodeURIComponent(request.runId)}/${encodeURIComponent(request.actionId)}`,
    reviewedRevision: request.reviewedRevision,
    bindingSha256: request.bindingSha256,
    runtimeRootSha256: request.runtimeRootSha256,
    candidateSha256: request.candidateSha256,
    processIdentitySha256: request.processIdentitySha256,
    rootPortsSha256: request.rootPortsSha256,
    ingressPidSha256: request.ingressPidSha256,
    backendPidSha256: request.backendPidSha256,
    runtimeHostPidSha256: request.runtimeHostPidSha256,
    ingressStartTokenSha256: request.ingressStartTokenSha256,
    backendStartTokenSha256: request.backendStartTokenSha256,
    runtimeHostStartTokenSha256: request.runtimeHostStartTokenSha256,
    sourceHostUpstreamSha256: sha256(request.sourceHostUpstream),
    sourceHostPort: request.sourceHostPort,
    sourceHostRouteIdentitySha256: sha256({
      upstreamSha256: sha256(request.sourceHostUpstream), port: request.sourceHostPort,
      runtimeRootSha256: request.runtimeRootSha256, candidateSha256: request.candidateSha256,
      processIdentitySha256: request.processIdentitySha256, rootPortsSha256: request.rootPortsSha256,
    }),
  };
  const deployment = deployed ?? {
    inventorySha256: sha256('inventory'),
    localDeployedConfigSha256: sha256('local-config'),
    bastionDeployedConfigSha256: sha256('bastion-config'),
    deployedConfigSha256: sha256('deployed-config'),
    deployedRevisionSha256: sha256('deployed-revision'),
  };
  const body = {
    schemaVersion: identity.schemaVersion,
    operation,
    selected: operation === 'select',
    unchanged: true,
    ...identity,
    selectionReceiptSha256: sha256(identity),
    ...deployment,
    productionRouteTouched: false,
    retryAttempted: false,
  };
  return { ...body, observationReceiptSha256: sha256(body) };
}

function successExternalResult(manifest) {
  const truth = manifest.scenarioPlan.scenarioTruth;
  const scenarioReceipt = sealP158DashboardScenarioReceipt({
    schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
    actionId: manifest.actionId,
    caseId: manifest.caseId,
    scenarioPlanSha256: manifest.scenarioPlan.scenarioPlanSha256,
    duplicateRows: truth.duplicateResourceIds.map((resourceId) => ({ resourceId, label: truth.duplicateLabel })),
    crossProfileBindings: truth.crossProfileBindings,
    selectedResourceId: truth.expectedSelectedResourceId,
    inspectorResourceId: truth.expectedSelectedResourceId,
    actionTargetResourceId: truth.expectedActionTargetResourceId,
    wrongResourceSelected: false,
    wrongResourceActioned: false,
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  });
  return sealP158DashboardExternalResult({
    manifest,
    scenarioReceipt,
    runnerAttestation: buildP158DashboardGithubRunnerAttestation({
      GITHUB_ACTIONS: 'true', RUNNER_ENVIRONMENT: 'github-hosted', RUNNER_OS: 'Linux',
      RUNNER_ARCH: 'X64', GITHUB_RUN_ID: workflowRunId, GITHUB_RUN_ATTEMPT: String(workflowRunAttempt),
    }),
    projection: { stateSha256: manifest.materializationReceipt.stateSha256 },
    dashboardFixture: { independentlyDerived: true },
    oracleBinding: { passed: true },
  });
}

const campaignRoot = await mkdtemp('/tmp/p158-dashboard-host-handshake-');
try {
  const actions = [{
    actionId: 'p158-d03-attempt:action:001', attemptId: 'p158-d03-attempt', caseId: 'D03',
    environmentId: 'E0', externalIngressRequired: true,
    assignment: { row_ambiguity: 'duplicate_labels' }, cardinalities: {},
  }];
  const preseedPlan = buildP158DashboardPreseedPlan({ actions, campaignRoot });
  const campaignPlan = buildP158DashboardCampaignPlan({
    preseedPlan, candidate, externalIngress, basePort: 54100,
    ingressSelector,
  });
  const preparation = await prepareP158DashboardCampaign({
    campaignPlan, preseedPlan, freezeState: 'pre_freeze', apply: true,
    validateState: async ({ stateSha256 }) => ({
      accepted: true, classification: 'accepted', stateSha256,
      parserIdentitySha256: candidate.executableSha256,
    }),
  });
  const root = campaignPlan.roots[0];
  const lifecycle = [];
  const started = {
    state: 'ready', pid: 7101, backendPid: 7102, runtimeHostPid: 7103,
    processIdentities: processIdentities(root), candidateSha256: candidate.executableSha256,
    statePath: root.target.statePath,
  };
  let selected;
  let persisted = null;
  const pauseEffects = {
    startExact: async () => { lifecycle.push('start'); return started; },
    selectExternalIngress: async (request) => {
      lifecycle.push('select');
      selected = selectorResponse('select', request);
      return selected;
    },
    persistDispatchReady: async (artifacts) => { lifecycle.push('persist'); persisted = artifacts; },
    stopExact: async () => { lifecycle.push('stop'); throw new Error('unexpected stop'); },
  };
  const shared = {
    campaignPlan, preparation, freezeState: 'frozen', actionId: root.actionId, expectedCommit,
  };
  const paused = await pauseP158DashboardHostAction({ ...shared, effects: pauseEffects });
  assert.equal(paused.state, 'dispatch_ready');
  assert.deepEqual(lifecycle, ['start', 'select', 'persist']);
  assert.equal(persisted.checkpoint.checkpointSha256, paused.checkpoint.checkpointSha256);
  assert.equal(paused.automaticDispatchAttempted, false);
  assert.equal(paused.retryAttempted, false);
  const checkpointText = JSON.stringify(paused.checkpoint);
  assert(!checkpointText.includes(root.target.disposableRoot));
  assert(!checkpointText.includes(externalIngress.publicOperatorUrl));
  assert(!checkpointText.includes('expectedState'));

  const persistFailure = await pauseP158DashboardHostAction({
    ...shared,
    effects: {
      ...pauseEffects,
      persistDispatchReady: async () => { throw new Error('append-only checkpoint refused'); },
      stopExact: async () => ({ state: 'stopped', pid: 7101, backendPid: 7102, runtimeHostPid: 7103 }),
    },
  });
  assert.equal(persistFailure.hostState, 'pause_failed');
  assert.equal(persistFailure.firstFailure.code, 'dashboard_host_handshake_failed');
  assert.equal(persistFailure.teardown.state, 'stopped');

  let stops = 0;
  const unchangedEffects = {
    observeExactRuntime: async () => ({
      unchanged: true, processIdentities: started.processIdentities,
      runtimeRootSha256: paused.checkpoint.runtimeRootSha256,
      statePathSha256: paused.checkpoint.statePathSha256,
      ports: root.ports, candidateSha256: candidate.executableSha256,
    }),
    observeExactIngress: async () => selectorResponse('observe',
      buildP158DashboardIngressSelectorRequest({
        root, processIdentities: started.processIdentities, operation: 'observe',
      }), {
        inventorySha256: paused.checkpoint.ingress.inventorySha256,
        localDeployedConfigSha256: selected.localDeployedConfigSha256,
        bastionDeployedConfigSha256: selected.bastionDeployedConfigSha256,
        deployedConfigSha256: paused.checkpoint.ingress.deployedConfigSha256,
        deployedRevisionSha256: paused.checkpoint.ingress.deployedRevisionSha256,
      }),
    stopExact: async () => {
      stops += 1;
      return { state: 'stopped', pid: 7101, backendPid: 7102, runtimeHostPid: 7103 };
    },
  };
  const resumeBase = {
    ...shared, checkpoint: paused.checkpoint, externalManifest: paused.externalManifest,
  };
  const awaiting = await resumeP158DashboardHostAction({ ...resumeBase, effects: unchangedEffects });
  assert.equal(awaiting.state, 'awaiting_external_receipt');
  assert.equal(stops, 0);
  assert.equal(awaiting.automaticDispatchAttempted, false);

  const externalResult = successExternalResult(paused.externalManifest);
  const terminal = await resumeP158DashboardHostAction({
    ...resumeBase, externalResult, expectedWorkflowRunId: workflowRunId,
    expectedWorkflowRunAttempt: workflowRunAttempt, effects: unchangedEffects,
  });
  assert.equal(terminal.resultState, 'passed');
  assert.equal(stops, 1);
  assert.equal(terminal.externalWorkflowRunIdSha256, sha256(workflowRunId));
  assert.equal(aggregateP158DashboardHostReceipts({ campaignPlan, receipts: [terminal] }).success, true);
  const adapterExecution = buildP158DashboardHostCampaignExecution({ campaignPlan, receipts: [terminal] });
  assert.equal(adapterExecution.aggregate.success, true);
  assert.equal(adapterExecution.receipts[0].scenarioOracle.passed, true);

  let uncertainStop = 0;
  const uncertain = await resumeP158DashboardHostAction({
    ...resumeBase,
    effects: {
      ...unchangedEffects,
      observeExactRuntime: async () => ({ unchanged: false }),
      stopExact: async () => { uncertainStop += 1; },
    },
  });
  assert.equal(uncertain.hostState, 'effect_uncertain');
  assert.equal(uncertainStop, 0);
  assert.equal(uncertain.retryAttempted, false);

  const wrongReceipt = await resumeP158DashboardHostAction({
    ...resumeBase, externalResult, expectedWorkflowRunId: '999999',
    expectedWorkflowRunAttempt: workflowRunAttempt, effects: unchangedEffects,
  });
  assert.equal(wrongReceipt.resultState, 'harness_failure');
  assert.equal(wrongReceipt.firstFailure.code, 'external_receipt_binding_invalid');
  assert.equal(wrongReceipt.teardown.state, 'stopped');
  assert.equal(stops, 2);

  await assert.rejects(() => resumeP158DashboardHostAction({
    ...resumeBase,
    campaignPlan: { ...campaignPlan, campaignPlanSha256: sha256('changed') },
    effects: unchangedEffects,
  }), (error) => error.code === 'host_handshake_input_invalid');
} finally {
  await rm(campaignRoot, { recursive: true, force: true });
}

process.stdout.write('Plan 0158 W8 dashboard host handshake provider-free checks passed\n');
