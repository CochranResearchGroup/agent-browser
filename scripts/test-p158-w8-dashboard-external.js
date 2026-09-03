#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { sha256 } from './lib/p158-campaign-controller.js';
import { buildP158DashboardServiceState } from './lib/p158-w8-dashboard-live.js';
import {
  buildP158DashboardScenarioPlan,
  sealP158DashboardScenarioReceipt,
} from './lib/p158-w8-dashboard-scenarios.js';
import {
  P158W8DashboardExternalError,
  buildP158DashboardGithubRunnerAttestation,
  buildP158DashboardExternalManifest,
  sealP158DashboardExternalResult,
  validateP158DashboardExternalActionUrl,
  validateP158DashboardExternalManifest,
  validateP158DashboardExternalResult,
} from './lib/p158-w8-dashboard-external.js';

function scenario(caseId, value) {
  const disposableRoot = `/tmp/p158-external-${caseId.toLowerCase()}-${value}`;
  const root = {
    actionId: `${caseId}-${value}:action:001`, attemptId: `${caseId}-${value}`, caseId,
    environmentId: 'E2', density: caseId === 'D04' ? 'normal' : 'sparse',
    scenario: { caseId, value },
    target: {
      runtimeLane: 'development', production: false, foreign: false, tenantDataPresent: false,
      ownership: 'p158_campaign', providerFree: false, serviceStopped: true,
      runId: `p158-external-${caseId.toLowerCase()}-${value}`,
      disposableRoot,
      pseudoHome: `${disposableRoot}/home`,
      statePath: `${disposableRoot}/home/.agent-browser/service/state.json`,
    },
  };
  const built = buildP158DashboardServiceState({
    target: root.target, density: root.density, scenario: root.scenario,
  });
  return buildP158DashboardScenarioPlan({
    root, expectedState: built.state, materializationReceipt: built.receipt,
  });
}

function expectCode(code, action) {
  assert.throws(action, (error) => error instanceof P158W8DashboardExternalError && error.code === code);
}

const scenarioPlan = scenario('D03', 'duplicate_labels');
const publicUrl = 'https://dashboard-actions.example.test/p158/action-001/service';
const manifest = buildP158DashboardExternalManifest({
  expectedCommit: 'a'.repeat(40),
  campaignPlanSha256: sha256('campaign-plan'),
  candidateSha256: sha256('candidate'),
  scenarioPlan,
  publicUrlSha256: sha256(publicUrl),
  publicPath: '/p158/action-001',
  selectionReceiptSha256: sha256('selection'),
});
assert.equal(validateP158DashboardExternalManifest(manifest), manifest);
assert.equal(validateP158DashboardExternalActionUrl({ manifest, publicUrl }), publicUrl);
expectCode('external_url_invalid', () => validateP158DashboardExternalActionUrl({
  manifest, publicUrl: 'https://dashboard-actions.example.test/p158/foreign/service',
}));
const localUrl = 'https://127.0.0.1/p158/action-001';
const localManifest = buildP158DashboardExternalManifest({
  ...manifest,
  publicUrlSha256: sha256(localUrl),
});
expectCode('external_url_invalid', () => validateP158DashboardExternalActionUrl({
  manifest: localManifest, publicUrl: localUrl,
}));
expectCode('external_manifest_invalid', () => validateP158DashboardExternalManifest({
  ...manifest, candidateSha256: sha256('changed-candidate'),
}));

const truth = scenarioPlan.scenarioTruth;
const scenarioReceipt = sealP158DashboardScenarioReceipt({
  schemaVersion: 'agent-browser.p158-dashboard-scenario-receipt.v1',
  actionId: scenarioPlan.actionId,
  caseId: scenarioPlan.caseId,
  scenarioPlanSha256: scenarioPlan.scenarioPlanSha256,
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
const runnerAttestation = buildP158DashboardGithubRunnerAttestation({
  GITHUB_ACTIONS: 'true', RUNNER_ENVIRONMENT: 'github-hosted', RUNNER_OS: 'Linux',
  RUNNER_ARCH: 'X64', GITHUB_RUN_ID: '15800304', GITHUB_RUN_ATTEMPT: '1',
});
expectCode('external_runner_invalid', () => buildP158DashboardGithubRunnerAttestation({
  GITHUB_ACTIONS: 'true', RUNNER_ENVIRONMENT: 'self-hosted', RUNNER_OS: 'Linux',
  RUNNER_ARCH: 'X64', GITHUB_RUN_ID: '15800304', GITHUB_RUN_ATTEMPT: '1',
}));
const result = sealP158DashboardExternalResult({ manifest, scenarioReceipt, runnerAttestation });
assert.equal(validateP158DashboardExternalResult({ result, manifest }), result);
expectCode('external_result_invalid', () => validateP158DashboardExternalResult({
  result: { ...result, actionId: 'foreign-action' }, manifest,
}));
const failure = sealP158DashboardExternalResult({
  manifest,
  failure: { code: 'external_capture_failed', message: 'synthetic failure' },
});
assert.equal(validateP158DashboardExternalResult({ result: failure, manifest }), failure);
expectCode('external_result_invalid', () => sealP158DashboardExternalResult({
  manifest,
  failure: { code: 'external_capture_failed', message: 'failed at http://127.0.0.1/internal' },
}));

const blockedD05 = scenario('D05', 'handoff');
expectCode('external_manifest_invalid', () => buildP158DashboardExternalManifest({
  expectedCommit: 'a'.repeat(40),
  campaignPlanSha256: sha256('campaign-plan'),
  candidateSha256: sha256('candidate'),
  scenarioPlan: blockedD05,
  publicUrlSha256: sha256(publicUrl),
  publicPath: '/p158/action-001',
  selectionReceiptSha256: sha256('selection'),
}));

const workflow = await readFile('.github/workflows/p158-w8-dashboard-external.yml', 'utf8');
assert(workflow.includes('workflow_dispatch:'));
assert(!workflow.match(/\b(?:push|pull_request|schedule):/u));
assert(workflow.includes('runs-on: ubuntu-latest'));
assert(workflow.includes('pnpm exec playwright install --with-deps chromium'));
assert(workflow.includes('external-capture'));
assert(workflow.includes('if: always()'));
assert(workflow.includes('cancel-in-progress: false'));

process.stdout.write('Plan 0158 W8 external dashboard producer provider-free checks passed\n');
