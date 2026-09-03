#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  P158W8DashboardCampaignError,
  aggregateP158DashboardCampaignReceipts,
  buildP158D09ChurnPlan,
  buildP158DashboardCampaignPlan,
  executeP158DashboardCampaignAction,
  prepareP158DashboardCampaign,
  p158DashboardIngressSelectorIdentity,
  resolveP158DashboardActionResume,
  validateP158ExternalPlaywrightRunner,
} from './lib/p158-w8-dashboard-campaign.js';
import { buildP158DashboardPreseedPlan } from './lib/p158-w8-dashboard-live.js';

const candidate = {
  executablePath: '/tmp/p158-installed-candidate/agent-browser',
  executableSha256: sha256('p158-installed-candidate'),
};
const externalIngress = {
  publicOperatorUrl: 'https://p158-dashboard.example.test',
  reviewedRevision: 'cooper-p158-w8-test-001',
};
const ingressSelector = {
  executablePath: '/tmp/p158-ingress-selector',
  executableSha256: sha256('p158-ingress-selector'),
  sourcePath: '/tmp/p158-ingress-selector.py',
  sourceSha256: sha256('p158-ingress-selector-source'),
  sourceCommit: 'e70368d',
};

function action(caseId, ordinal, assignment) {
  return {
    actionId: `p158-${caseId.toLowerCase()}-attempt:action:${String(ordinal).padStart(3, '0')}`,
    attemptId: `p158-${caseId.toLowerCase()}-attempt`,
    caseId,
    environmentId: caseId === 'D09' ? 'E2' : 'E0',
    externalIngressRequired: true,
    assignment,
    cardinalities: {},
  };
}

const campaignRoot = await mkdtemp('/tmp/p158-dashboard-campaign-test-');
try {
  const actions = [
    action('D01', 1, { inventory_density: 'sparse' }),
    action('D09', 1, { stream_state: 'connected' }),
  ];
  const preseedPlan = buildP158DashboardPreseedPlan({ actions, campaignRoot });
  const campaignPlan = buildP158DashboardCampaignPlan({
    preseedPlan,
    candidate,
    externalIngress,
    ingressSelector,
    basePort: 53100,
  });

  assert.equal(campaignPlan.actionCount, 2);
  assert.deepEqual(campaignPlan.ingressSelector, ingressSelector);
  assert.throws(() => buildP158DashboardCampaignPlan({
    preseedPlan, candidate, externalIngress, basePort: 53000,
  }), (error) => error instanceof P158W8DashboardCampaignError && error.code === 'ingress_selector_invalid');
  assert.equal(new Set(campaignPlan.roots.flatMap((root) => Object.values(root.ports))).size, 8);
  assert.equal(new Set(campaignPlan.roots.map((root) => root.environment.HOME)).size, 2);
  for (const root of campaignPlan.roots) {
    assert(root.environment.HOME.startsWith(`${root.target.disposableRoot}/`));
    assert(root.environment.XDG_RUNTIME_DIR.startsWith(`${root.target.disposableRoot}/`));
    assert(root.environment.AGENT_BROWSER_SOCKET_DIR.startsWith(`${root.target.disposableRoot}/`));
    assert.equal(root.environment.AGENT_BROWSER_RUNTIME_ENVIRONMENT, 'development');
    assert.notEqual(root.environment.HOME, process.env.HOME);
  }

  assert.throws(
    () => buildP158DashboardCampaignPlan({
      preseedPlan,
      candidate,
      externalIngress: { ...externalIngress, publicOperatorUrl: 'http://127.0.0.1:53101' },
      ingressSelector,
      basePort: 53200,
    }),
    (error) => error instanceof P158W8DashboardCampaignError && error.code === 'external_ingress_invalid',
  );
  const runnerBody = {
    schemaVersion: 'agent-browser.p158-external-playwright-runner-attestation.v1',
    endpointSha256: sha256('wss://playwright-runner.example.test/connect'),
    offHost: true,
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    reviewedRevision: 'external-runner-test-001',
  };
  assert.equal(validateP158ExternalPlaywrightRunner({
    endpoint: 'wss://playwright-runner.example.test/connect',
    attestation: { ...runnerBody, attestationSha256: sha256(runnerBody) },
  }).endpoint, 'wss://playwright-runner.example.test/connect');
  assert.throws(
    () => validateP158ExternalPlaywrightRunner({
      endpoint: 'ws://127.0.0.1:9222/connect',
      attestation: { ...runnerBody, endpointSha256: sha256('ws://127.0.0.1:9222/connect') },
    }),
    (error) => error instanceof P158W8DashboardCampaignError && error.code === 'external_runner_invalid',
  );

  const validatorCalls = [];
  const preparation = await prepareP158DashboardCampaign({
    campaignPlan,
    preseedPlan,
    freezeState: 'pre_freeze',
    apply: true,
    validateState: async (request) => {
      validatorCalls.push(request);
      assert.equal(request.candidate.executablePath, candidate.executablePath);
      assert(request.validationInputPath.startsWith(`${request.root.target.disposableRoot}/`));
      return {
        schemaVersion: 'agent-browser.service-state-validation.v1',
        accepted: true,
        classification: 'accepted',
        stateSha256: request.stateSha256,
        parserIdentitySha256: candidate.executableSha256,
      };
    },
  });
  assert.equal(validatorCalls.length, 2);
  assert.equal(preparation.preseedReceipt.receipts.every((receipt) => receipt.written), true);
  assert.equal(preparation.ingressSelectorSha256, sha256(ingressSelector));
  assert.equal(preparation.preseedReceipt.receipts.every((receipt) =>
    receipt.parserReceipt.stateSha256 === receipt.materializationReceipt.stateSha256), true);

  await assert.rejects(
    () => prepareP158DashboardCampaign({
      campaignPlan,
      preseedPlan,
      freezeState: 'frozen',
      apply: false,
      validateState: async () => ({}),
    }),
    (error) => error instanceof P158W8DashboardCampaignError &&
      error.code === 'post_freeze_materialization_prohibited',
  );

  const alternateRoot = await mkdtemp('/tmp/p158-dashboard-parser-test-');
  try {
    const alternatePreseed = buildP158DashboardPreseedPlan({
      actions: [action('D01', 2, { inventory_density: 'empty' })],
      campaignRoot: alternateRoot,
    });
    const alternateCampaign = buildP158DashboardCampaignPlan({
      preseedPlan: alternatePreseed,
      candidate,
      externalIngress,
      ingressSelector,
      basePort: 53300,
    });
    await assert.rejects(
      () => prepareP158DashboardCampaign({
        campaignPlan: alternateCampaign,
        preseedPlan: alternatePreseed,
        freezeState: 'pre_freeze',
        apply: true,
        validateState: async ({ stateSha256 }) => ({
          accepted: true,
          classification: 'accepted',
          stateSha256,
          parserIdentitySha256: sha256('wrong-candidate'),
        }),
      }),
      (error) => error instanceof P158W8DashboardCampaignError &&
        error.code === 'parser_receipt_binding_invalid',
    );
  } finally {
    await rm(alternateRoot, { recursive: true, force: true });
  }

  const lifecycle = [];
  const effects = {
    startExact: async (root) => {
      lifecycle.push(`start:${root.actionId}`);
      return {
        state: 'ready',
        pid: root.caseId === 'D09' ? 9009 : 9001,
        backendPid: root.caseId === 'D09' ? 9109 : 9101,
        runtimeHostPid: root.caseId === 'D09' ? 9209 : 9201,
        processIdentities: {
          ingress: { pid: root.caseId === 'D09' ? 9009 : 9001, startToken: '101', executableSha256: root.candidate.executableSha256 },
          backend: { pid: root.caseId === 'D09' ? 9109 : 9101, startToken: '102', executableSha256: root.candidate.executableSha256 },
          runtimeHost: { pid: root.caseId === 'D09' ? 9209 : 9201, startToken: '103', executableSha256: root.candidate.executableSha256 },
        },
        candidateSha256: root.candidate.executableSha256,
        statePath: root.target.statePath,
      };
    },
    selectExternalIngress: async (request) => {
      lifecycle.push(`select:${request.actionId}`);
      const root = campaignPlan.roots.find((entry) => entry.actionId === request.actionId);
      const { identity, selectionReceiptSha256 } = p158DashboardIngressSelectorIdentity(root,
        (root.caseId === 'D09'
          ? { ingress: { pid: 9009, startToken: '101', executableSha256: candidate.executableSha256 }, backend: { pid: 9109, startToken: '102', executableSha256: candidate.executableSha256 }, runtimeHost: { pid: 9209, startToken: '103', executableSha256: candidate.executableSha256 } }
          : { ingress: { pid: 9001, startToken: '101', executableSha256: candidate.executableSha256 }, backend: { pid: 9101, startToken: '102', executableSha256: candidate.executableSha256 }, runtimeHost: { pid: 9201, startToken: '103', executableSha256: candidate.executableSha256 } }));
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
    openExternalPage: async ({ root }) => {
      lifecycle.push(`open:${root.actionId}`);
      const state = JSON.parse(await readFile(root.target.statePath, 'utf8'));
      const profiles = Object.values(state.profiles);
      const browsers = Object.values(state.browsers);
      const capture = {
        collections: {
          profiles: { profiles },
          browsers: { browsers },
          tabs: { tabs: Object.values(state.tabs) },
          jobs: { jobs: Object.values(state.jobs) },
          events: { events: state.events },
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
        actionButtons: [],
        warnings: [],
        locationPath: '/service',
        domNodeCount: profiles.length + browsers.length + 100,
        performance: [{ durationMs: 900, domInteractiveMs: 500, loadEventEndMs: 850 }],
      };
      return {
        page: {
          evaluate: async () => structuredClone(capture),
          screenshot: async ({ path }) => {
            await mkdir(join(root.target.disposableRoot, 'artifacts'), { recursive: true });
            await writeFile(path, `pixels:${root.actionId}`);
          },
        },
        close: async () => lifecycle.push(`close:${root.actionId}`),
      };
    },
    produceChurn: async ({ root, churnPlan }) => {
      lifecycle.push(`churn:${root.actionId}`);
      assert.equal(buildP158D09ChurnPlan({ root }).churnPlanSha256, churnPlan.churnPlanSha256);
      return {
        churnPlanSha256: churnPlan.churnPlanSha256,
        completedOperationCount: churnPlan.cycleCount,
        retryAttempted: false,
      };
    },
    stopExact: async ({ actionId, expectedPid }) => {
      lifecycle.push(`stop:${actionId}`);
      return { state: 'stopped', pid: expectedPid, backendPid: expectedPid + 100, runtimeHostPid: expectedPid + 200 };
    },
  };

  await assert.rejects(
    () => executeP158DashboardCampaignAction({
      campaignPlan,
      preparation,
      freezeState: 'prepared',
      actionId: actions[0].actionId,
      effects,
    }),
    (error) => error instanceof P158W8DashboardCampaignError && error.code === 'wrong_campaign_state',
  );

  const receipts = [];
  for (const planned of actions) {
    receipts.push(await executeP158DashboardCampaignAction({
      campaignPlan,
      preparation,
      freezeState: 'frozen',
      actionId: planned.actionId,
      effects,
    }));
  }
  const d01 = receipts.find((receipt) => receipt.caseId === 'D01');
  const d09 = receipts.find((receipt) => receipt.caseId === 'D09');
  assert.deepEqual(d01.projection.counts, { profiles: 2, browsers: 5, tabs: 20, jobs: 100, events: 100 });
  assert.equal(d01.projection.capture.railRows.length, 7);
  assert.deepEqual(d09.projection.counts, { profiles: 100, browsers: 500, tabs: 2000, jobs: 10000, events: 10000 });
  assert.equal(d09.projection.capture.railRows.length, 600);
  assert.equal(d09.churnReceipt.completedOperationCount, 32);
  assert.equal(d01.oracleBinding.passed, true);
  assert.equal(d09.oracleBinding.passed, true);
  assert(lifecycle.indexOf(`churn:${actions[1].actionId}`) < lifecycle.indexOf(`stop:${actions[1].actionId}`));

  const aggregate = aggregateP158DashboardCampaignReceipts({ campaignPlan, receipts });
  assert.equal(aggregate.success, true);
  assert.equal(aggregate.actionCount, 2);
  assert.equal(aggregate.retryCount, 0);
  assert.match(aggregate.aggregateSha256, /^[a-f0-9]{64}$/);
  assert.equal(resolveP158DashboardActionResume({
    campaignPlan,
    actionId: receipts[0].actionId,
    claim: { actionId: receipts[0].actionId, campaignPlanSha256: campaignPlan.campaignPlanSha256 },
    receipt: receipts[0],
  }).disposition, 'reuse_terminal');
  assert.throws(
    () => resolveP158DashboardActionResume({
      campaignPlan,
      actionId: receipts[0].actionId,
      claim: { actionId: receipts[0].actionId, campaignPlanSha256: campaignPlan.campaignPlanSha256 },
    }),
    (error) => error instanceof P158W8DashboardCampaignError && error.code === 'action_effect_uncertain',
  );

  const badRouteReceipt = await executeP158DashboardCampaignAction({
    campaignPlan,
    preparation,
    freezeState: 'frozen',
    actionId: actions[0].actionId,
    effects: {
      ...effects,
      selectExternalIngress: async (request) => {
        const body = {
          selected: true,
          publicUrl: `${externalIngress.publicOperatorUrl}/p158/foreign-root`,
          publicPath: '/p158/foreign-root',
          bindingSha256: campaignPlan.externalIngress.bindingSha256,
          actionId: request.actionId,
          runtimeRootSha256: sha256('foreign-root'),
          dashboardPort: request.dashboardPort,
          dashboardBackendPort: request.dashboardBackendPort,
          runtimeStreamPort: request.runtimeStreamPort,
          expectedPid: request.expectedPid,
          expectedBackendPid: request.expectedBackendPid,
          expectedRuntimeHostPid: request.expectedRuntimeHostPid,
          processIdentitySha256: request.processIdentitySha256,
          reviewedRevision: request.reviewedRevision,
          externalProof: {
            offHost: true, outsideServiceNetworkNamespace: true, publicHttps: true,
            operatorVisibleState: 'ready', handoffUrlSha256: sha256('foreign-root'),
          },
        };
        return { ...body, selectionReceiptSha256: sha256(body) };
      },
    },
  });
  assert.equal(badRouteReceipt.resultState, 'harness_failure');
  assert.equal(badRouteReceipt.firstFailure.code, 'external_ingress_selection_invalid');
  assert.equal(badRouteReceipt.teardown.state, 'stopped');

  const firstFailureReceipt = await executeP158DashboardCampaignAction({
    campaignPlan,
    preparation,
    freezeState: 'frozen',
    actionId: actions[0].actionId,
    effects: {
      ...effects,
      openExternalPage: async () => { throw Object.assign(new Error('first capture failure'), { code: 'capture_failed' }); },
      stopExact: async () => { throw Object.assign(new Error('teardown failure'), { code: 'teardown_failed' }); },
    },
  });
  assert.equal(firstFailureReceipt.resultState, 'harness_failure');
  assert.equal(firstFailureReceipt.firstFailure.code, 'capture_failed');
  assert.equal(firstFailureReceipt.teardown.failure.code, 'teardown_failed');
} finally {
  await rm(campaignRoot, { recursive: true, force: true });
}

process.stdout.write('Plan 0158 W8 dashboard campaign provider-free checks passed\n');
