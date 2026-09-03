#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { basename, join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { buildP158DashboardServiceState } from './lib/p158-w8-dashboard-live.js';
import { buildP158DashboardPreseedPlan } from './lib/p158-w8-dashboard-live.js';
import {
  aggregateP158DashboardCampaignReceipts,
  buildP158DashboardCampaignPlan,
  executeP158DashboardCampaignAction,
  prepareP158DashboardCampaign,
  p158DashboardIngressSelectorIdentity,
} from './lib/p158-w8-dashboard-campaign.js';
import {
  P158_D05_BLOCKED_TARGETS,
  P158_D05_SUPPORTED_TARGETS,
  P158W8DashboardScenarioError,
  applyP158DashboardScenarioToFixture,
  auditP158DashboardScenarioReceipt,
  buildP158DashboardScenarioPlan,
  sealP158DashboardScenarioReceipt,
} from './lib/p158-w8-dashboard-scenarios.js';

const externalRunnerEndpoint = 'wss://playwright-runner.example.test/connect';
const externalRunnerBody = {
  schemaVersion: 'agent-browser.p158-external-playwright-runner-attestation.v1',
  endpointSha256: sha256(externalRunnerEndpoint),
  offHost: true,
  outsideServiceHost: true,
  outsideServiceNetworkNamespace: true,
  reviewedRevision: 'external-runner-scenario-test-001',
};
const externalRunner = {
  endpoint: externalRunnerEndpoint,
  attestation: { ...externalRunnerBody, attestationSha256: sha256(externalRunnerBody) },
};

function root(caseId, value) {
  const disposableRoot = `/tmp/p158-dashboard-scenario-${caseId.toLowerCase()}-${value.replaceAll('_', '-')}`;
  return {
    actionId: `${caseId}-${value}:action:001`, attemptId: `${caseId}-${value}`, caseId,
    environmentId: caseId === 'D04' ? 'E2' : 'E0', density: caseId === 'D04' ? 'normal' : 'sparse',
    scenario: { caseId, value },
    target: {
      runtimeLane: 'development', production: false, foreign: false, tenantDataPresent: false,
      ownership: 'p158_campaign', providerFree: false, serviceStopped: true,
      runId: basename(disposableRoot), disposableRoot, pseudoHome: join(disposableRoot, 'home'),
      statePath: join(disposableRoot, 'home', '.agent-browser', 'service', 'state.json'),
    },
  };
}

function planned(caseId, value) {
  const campaignRoot = root(caseId, value);
  const built = buildP158DashboardServiceState({
    target: campaignRoot.target, density: campaignRoot.density, scenario: campaignRoot.scenario,
  });
  return {
    root: campaignRoot,
    built,
    plan: buildP158DashboardScenarioPlan({
      root: campaignRoot, expectedState: built.state, materializationReceipt: built.receipt,
    }),
  };
}

function receipt(plan, additions) {
  return sealP158DashboardScenarioReceipt({
    schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
    actionId: plan.actionId, caseId: plan.caseId, scenarioPlanSha256: plan.scenarioPlanSha256,
    ...additions,
    repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
  });
}

function expectCode(code, action) {
  assert.throws(action, (error) => {
    assert(error instanceof P158W8DashboardScenarioError);
    assert.equal(error.code, code);
    return true;
  });
}

const d03 = planned('D03', 'duplicate_labels');
const d03Truth = d03.plan.scenarioTruth;
assert.equal(d03Truth.duplicateResourceIds.length, 2);
assert.notEqual(d03Truth.duplicateResourceIds[0], d03Truth.duplicateResourceIds[1]);
assert.equal(new Set(d03Truth.crossProfileBindings.map((entry) => entry.profileId)).size, 2);
const d03Receipt = receipt(d03.plan, {
  duplicateRows: d03Truth.duplicateResourceIds.map((resourceId) => ({ resourceId, label: d03Truth.duplicateLabel })),
  crossProfileBindings: d03Truth.crossProfileBindings,
  selectedResourceId: d03Truth.expectedSelectedResourceId,
  inspectorResourceId: d03Truth.expectedSelectedResourceId,
  actionTargetResourceId: d03Truth.expectedActionTargetResourceId,
  wrongResourceSelected: false,
  wrongResourceActioned: false,
});
assert.equal(auditP158DashboardScenarioReceipt({ plan: d03.plan, receipt: d03Receipt }).passed, true);
expectCode('d03_selection_oracle_failed', () => auditP158DashboardScenarioReceipt({
  plan: d03.plan,
  receipt: receipt(d03.plan, { ...d03Receipt, receiptSha256: undefined,
    selectedResourceId: d03Truth.duplicateResourceIds[0] }),
}));
expectCode('d03_selection_oracle_failed', () => auditP158DashboardScenarioReceipt({
  plan: d03.plan,
  receipt: receipt(d03.plan, { ...d03Receipt, receiptSha256: undefined,
    inspectorResourceId: d03Truth.duplicateResourceIds[0] }),
}));
expectCode('d03_selection_oracle_failed', () => auditP158DashboardScenarioReceipt({
  plan: d03.plan,
  receipt: receipt(d03.plan, { ...d03Receipt, receiptSha256: undefined,
    actionTargetResourceId: d03Truth.crossProfileBindings[0].browserId }),
}));

const d04 = planned('D04', 'navigate');
const publicPath = `/p158/${sha256(d04.root.actionId).slice(0, 16)}`;
const selectionReceiptSha256 = sha256('exact-action-route-selection');
const d04Receipt = receipt(d04.plan, {
  publicPath,
  selectionReceiptSha256,
  clients: d04.plan.scenarioTruth.clients.map((client) => ({
    clientId: client.clientId,
    offHost: true,
    outsideServiceNetworkNamespace: true,
    clientIngressReceiptSha256: sha256({
      actionId: d04.plan.actionId, clientId: client.clientId, publicPath, selectionReceiptSha256,
      offHost: true, outsideServiceNetworkNamespace: true,
    }),
    completedOperations: client.operations,
    expectedSelectedResourceId: client.expectedSelectedResourceId,
    observedSelectedResourceId: client.expectedSelectedResourceId,
    observedInspectorResourceId: client.expectedSelectedResourceId,
    selectionAfterRefresh: client.expectedSelectedResourceId,
    selectionAfterBackForward: client.expectedSelectedResourceId,
    selectionAfterDeepLink: client.expectedSelectedResourceId,
    finalBarrierSelectedResourceId: client.expectedSelectedResourceId,
    finalBarrierInspectorResourceId: client.expectedSelectedResourceId,
  })),
});
assert.equal(auditP158DashboardScenarioReceipt({ plan: d04.plan, receipt: d04Receipt }).passed, true);
const nineClients = receipt(d04.plan, { ...d04Receipt, receiptSha256: undefined, clients: d04Receipt.clients.slice(1) });
expectCode('d04_client_set_invalid', () => auditP158DashboardScenarioReceipt({ plan: d04.plan, receipt: nineClients }));
const leaked = structuredClone(d04Receipt);
leaked.clients[1].observedSelectedResourceId = leaked.clients[0].expectedSelectedResourceId;
expectCode('d04_client_isolation_failed', () => auditP158DashboardScenarioReceipt({
  plan: d04.plan, receipt: sealP158DashboardScenarioReceipt(leaked),
}));
const localClient = structuredClone(d04Receipt);
localClient.clients[0].offHost = false;
localClient.clients[0].clientIngressReceiptSha256 = sha256({
  actionId: d04.plan.actionId, clientId: localClient.clients[0].clientId, publicPath,
  selectionReceiptSha256, offHost: false, outsideServiceNetworkNamespace: true,
});
expectCode('d04_client_isolation_failed', () => auditP158DashboardScenarioReceipt({
  plan: d04.plan, receipt: sealP158DashboardScenarioReceipt(localClient),
}));
const wrongInspector = structuredClone(d04Receipt);
wrongInspector.clients[2].observedInspectorResourceId = wrongInspector.clients[1].expectedSelectedResourceId;
expectCode('d04_client_isolation_failed', () => auditP158DashboardScenarioReceipt({
  plan: d04.plan, receipt: sealP158DashboardScenarioReceipt(wrongInspector),
}));
const finalLeak = structuredClone(d04Receipt);
finalLeak.clients[0].finalBarrierSelectedResourceId = finalLeak.clients[1].expectedSelectedResourceId;
expectCode('d04_client_isolation_failed', () => auditP158DashboardScenarioReceipt({
  plan: d04.plan, receipt: sealP158DashboardScenarioReceipt(finalLeak),
}));
expectCode('scenario_receipt_unsafe_url', () => auditP158DashboardScenarioReceipt({
  plan: d04.plan,
  receipt: receipt(d04.plan, { ...d04Receipt, receiptSha256: undefined, rawHandoffUrl: 'https://internal.example.test/raw' }),
}));

for (const targetType of P158_D05_SUPPORTED_TARGETS) {
  const d05 = planned('D05', targetType);
  assert.equal(d05.plan.scenarioTruth.executable, true);
  const truth = d05.plan.scenarioTruth;
  const d05Receipt = receipt(d05.plan, {
    queryKey: truth.queryKey,
    staleRequestedId: truth.staleRequestedId,
    initialStaleSelectionObserved: true,
    recoveryEventCount: 1,
    recoveryMethod: 'dashboard_history_replace',
    recoveryExplanation: `${truth.expectedExplanationFragment} ${truth.staleRequestedId}; using current live target ${truth.expectedResolvedSelectionId}.`,
    resolvedSelectionId: truth.expectedResolvedSelectionId,
    resolvedResourceId: truth.expectedResolvedResourceId,
    resolvedWorkspaceId: truth.expectedWorkspaceId,
  });
  assert.equal(auditP158DashboardScenarioReceipt({ plan: d05.plan, receipt: d05Receipt }).passed, true);
  const fixture = applyP158DashboardScenarioToFixture({
    fixture: { selection: {}, clientSelections: [] }, plan: d05.plan, receipt: d05Receipt,
  });
  assert.equal(fixture.selection.deepLinkRequestedId, truth.staleRequestedId);
  expectCode('d05_deep_link_recovery_failed', () => auditP158DashboardScenarioReceipt({
    plan: d05.plan,
    receipt: receipt(d05.plan, { ...d05Receipt, receiptSha256: undefined, recoveryEventCount: 2 }),
  }));
}

for (const targetType of P158_D05_BLOCKED_TARGETS) {
  const blocked = planned('D05', targetType);
  assert.equal(blocked.plan.scenarioTruth.executable, false);
  assert.equal(blocked.plan.scenarioTruth.blocker.code, 'dashboard_deep_link_target_unsupported');
}

const changedState = structuredClone(d03.built.state);
Object.values(changedState.profiles)[0].name = 'not duplicated';
expectCode('scenario_state_binding_invalid', () => buildP158DashboardScenarioPlan({
  root: d03.root, expectedState: changedState, materializationReceipt: d03.built.receipt,
}));

const campaignRoot = await mkdtemp('/tmp/p158-dashboard-scenario-campaign-');
try {
  const actions = [
    { caseId: 'D03', value: 'duplicate_labels' },
    { caseId: 'D04', value: 'navigate' },
    { caseId: 'D05', value: 'tab' },
    { caseId: 'D05', value: 'handoff' },
  ].map(({ caseId, value }, index) => ({
    actionId: `${caseId}-${value}:action:${String(index + 1).padStart(3, '0')}`,
    attemptId: `${caseId}-${value}`,
    caseId,
    environmentId: caseId === 'D04' ? 'E2' : 'E0',
    externalIngressRequired: true,
    assignment: caseId === 'D03' ? { row_ambiguity: value }
      : caseId === 'D04' ? { navigation_action: value }
        : { missing_resource: value },
    cardinalities: {},
  }));
  const candidate = {
    executablePath: '/tmp/p158-dashboard-scenario-candidate/agent-browser',
    executableSha256: sha256('scenario-candidate'),
  };
  const preseedPlan = buildP158DashboardPreseedPlan({ actions, campaignRoot });
  const campaignPlan = buildP158DashboardCampaignPlan({
    preseedPlan,
    candidate,
    externalIngress: {
      publicOperatorUrl: 'https://p158-dashboard-scenarios.example.test',
      reviewedRevision: 'p158-scenario-test-001',
    },
    ingressSelector: {
      executablePath: '/tmp/p158-ingress-selector',
      executableSha256: sha256('p158-ingress-selector'),
      sourcePath: '/tmp/p158-ingress-selector.py',
      sourceSha256: sha256('p158-ingress-selector-source'),
      sourceCommit: 'e70368d',
    },
    basePort: 53800,
  });
  const preparation = await prepareP158DashboardCampaign({
    campaignPlan,
    preseedPlan,
    freezeState: 'pre_freeze',
    apply: true,
    validateState: async ({ stateSha256 }) => ({
      accepted: true,
      classification: 'accepted',
      stateSha256,
      parserIdentitySha256: candidate.executableSha256,
    }),
  });
  const lifecycle = [];
  const startedIdentities = new Map();
  const effects = {
    startExact: async (campaignActionRoot) => {
      lifecycle.push(`start:${campaignActionRoot.actionId}`);
      const ordinal = lifecycle.length;
      const started = {
        state: 'ready', pid: 6000 + ordinal, backendPid: 6100 + ordinal,
        runtimeHostPid: 6200 + ordinal,
        processIdentities: {
          ingress: { pid: 6000 + ordinal, startToken: `i${ordinal}`, executableSha256: candidate.executableSha256 },
          backend: { pid: 6100 + ordinal, startToken: `b${ordinal}`, executableSha256: candidate.executableSha256 },
          runtimeHost: { pid: 6200 + ordinal, startToken: `r${ordinal}`, executableSha256: candidate.executableSha256 },
        },
        candidateSha256: candidate.executableSha256,
        statePath: campaignActionRoot.target.statePath,
      };
      startedIdentities.set(campaignActionRoot.actionId, started.processIdentities);
      return started;
    },
    selectExternalIngress: async (request) => {
      const root = campaignPlan.roots.find((entry) => entry.actionId === request.actionId);
      const { identity, selectionReceiptSha256 } = p158DashboardIngressSelectorIdentity(
        root, startedIdentities.get(request.actionId),
      );
      const body = {
        schemaVersion: identity.schemaVersion,
        operation: 'select',
        selected: true,
        unchanged: true,
        ...identity,
        selectionReceiptSha256,
        inventorySha256: sha256('inventory'), localDeployedConfigSha256: sha256('local'),
        bastionDeployedConfigSha256: sha256('bastion'), deployedConfigSha256: sha256('deployed'),
        deployedRevisionSha256: sha256('revision'), productionRouteTouched: false, retryAttempted: false,
      };
      return { ...body, observationReceiptSha256: sha256(body) };
    },
    openExternalPage: async ({ root: campaignActionRoot }) => {
      const state = JSON.parse(await readFile(campaignActionRoot.target.statePath, 'utf8'));
      const profiles = Object.values(state.profiles);
      const browsers = Object.values(state.browsers);
      const capture = {
        collections: {
          profiles: { profiles }, browsers: { browsers }, tabs: { tabs: Object.values(state.tabs) },
          jobs: { jobs: Object.values(state.jobs) }, events: { events: state.events },
        },
        railRows: [
          ...profiles.map((entry, index) => ({
            rowId: `row-${entry.id}`, resourceId: entry.id, resourceType: 'profile',
            label: entry.name, state: 'ready', orderKey: index,
          })),
          ...browsers.map((entry, index) => ({
            rowId: `row-${entry.id}`, resourceId: entry.id, resourceType: 'browser',
            label: entry.id, state: entry.health, orderKey: profiles.length + index,
          })),
        ],
        actionButtons: [], warnings: [], locationPath: '/p158/test',
        domNodeCount: profiles.length + browsers.length + 20,
        performance: [{ durationMs: 50, domInteractiveMs: 30, loadEventEndMs: 45 }],
      };
      return {
        externalRunner,
        page: {
          evaluate: async () => structuredClone(capture),
          screenshot: async ({ path }) => {
            await mkdir(join(campaignActionRoot.target.disposableRoot, 'artifacts'), { recursive: true });
            await writeFile(path, `scenario:${campaignActionRoot.actionId}`);
          },
        },
        close: async () => lifecycle.push(`close:${campaignActionRoot.actionId}`),
      };
    },
    exerciseScenario: async ({ scenarioPlan, publicPath: selectedPublicPath, selectionReceiptSha256: selectedSha }) => {
      const truth = scenarioPlan.scenarioTruth;
      if (scenarioPlan.caseId === 'D03') return receipt(scenarioPlan, {
        duplicateRows: truth.duplicateResourceIds.map((resourceId) => ({ resourceId, label: truth.duplicateLabel })),
        crossProfileBindings: truth.crossProfileBindings,
        selectedResourceId: truth.expectedSelectedResourceId,
        inspectorResourceId: truth.expectedSelectedResourceId,
        actionTargetResourceId: truth.expectedActionTargetResourceId,
        wrongResourceSelected: false,
        wrongResourceActioned: false,
      });
      if (scenarioPlan.caseId === 'D04') return receipt(scenarioPlan, {
        publicPath: selectedPublicPath,
        selectionReceiptSha256: selectedSha,
        clients: truth.clients.map((client) => ({
          clientId: client.clientId,
          offHost: true,
          outsideServiceNetworkNamespace: true,
          clientIngressReceiptSha256: sha256({
            actionId: scenarioPlan.actionId, clientId: client.clientId,
            publicPath: selectedPublicPath, selectionReceiptSha256: selectedSha,
            offHost: true, outsideServiceNetworkNamespace: true,
          }),
          completedOperations: client.operations,
          expectedSelectedResourceId: client.expectedSelectedResourceId,
          observedSelectedResourceId: client.expectedSelectedResourceId,
          observedInspectorResourceId: client.expectedSelectedResourceId,
          selectionAfterRefresh: client.expectedSelectedResourceId,
          selectionAfterBackForward: client.expectedSelectedResourceId,
          selectionAfterDeepLink: client.expectedSelectedResourceId,
          finalBarrierSelectedResourceId: client.expectedSelectedResourceId,
          finalBarrierInspectorResourceId: client.expectedSelectedResourceId,
        })),
      });
      return receipt(scenarioPlan, {
        queryKey: truth.queryKey,
        staleRequestedId: truth.staleRequestedId,
        initialStaleSelectionObserved: true,
        recoveryEventCount: 1,
        recoveryMethod: 'dashboard_history_replace',
        recoveryExplanation: `${truth.expectedExplanationFragment} ${truth.staleRequestedId}; using current live target ${truth.expectedResolvedSelectionId}.`,
        resolvedSelectionId: truth.expectedResolvedSelectionId,
        resolvedResourceId: truth.expectedResolvedResourceId,
        resolvedWorkspaceId: truth.expectedWorkspaceId,
      });
    },
    stopExact: async ({ actionId, expectedPid, processIdentities }) => {
      lifecycle.push(`stop:${actionId}`);
      return {
        state: 'stopped', pid: expectedPid, backendPid: processIdentities.backend.pid,
        runtimeHostPid: processIdentities.runtimeHost.pid,
      };
    },
  };
  const receipts = [];
  for (const campaignAction of actions) {
    receipts.push(await executeP158DashboardCampaignAction({
      campaignPlan, preparation, freezeState: 'frozen', actionId: campaignAction.actionId, effects,
    }));
  }
  assert.equal(receipts.filter((entry) => entry.resultState === 'passed').length, 3,
    JSON.stringify(receipts.map((entry) => ({ actionId: entry.actionId, failure: entry.firstFailure }))));
  const blockedReceipt = receipts.find((entry) => entry.actionId === actions[3].actionId);
  assert.equal(blockedReceipt.resultState, 'skipped_blocked');
  assert.equal(blockedReceipt.blocker.code, 'dashboard_deep_link_target_unsupported');
  assert.equal(lifecycle.some((entry) => entry === `start:${actions[3].actionId}`), false);
  const aggregate = aggregateP158DashboardCampaignReceipts({ campaignPlan, receipts });
  assert.deepEqual(aggregate.resultCounts, { passed: 3, skippedBlocked: 1, failed: 0 });
  assert.equal(aggregate.caseResults.find((entry) => entry.caseId === 'D03').success, true);
  assert.equal(aggregate.caseResults.find((entry) => entry.caseId === 'D04').success, true);
  assert.equal(aggregate.caseResults.find((entry) => entry.caseId === 'D05').success, false);
  assert.deepEqual(aggregate.blockedActionIds, [actions[3].actionId]);
  assert.equal(aggregate.success, false);
} finally {
  await rm(campaignRoot, { recursive: true, force: true });
}

process.stdout.write('Plan 0158 W8 D03/D04/D05 dashboard scenario tests passed\n');
