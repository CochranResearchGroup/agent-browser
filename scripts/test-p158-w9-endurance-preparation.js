#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  P158W9EndurancePreparationError,
  prepareP158W9EndurancePostconditions,
} from './lib/p158-w9-endurance-preparation.js';

const handoffUrl = 'https://external.example.test/remote-view/opaque';
const config = {
  runtimeLane: 'development', production: false, syntheticOnly: true, caseId: 'C05',
  runId: 'p158-preparation-test', sourceCommit: 'a'.repeat(40), candidateSha256: '11'.repeat(32),
  scheduleSha256: '22'.repeat(32), handoffUrl, handoffUrlSha256: sha256(handoffUrl),
  retainedIdentitySha256: '33'.repeat(32), syntheticFixtureAttestationSha256: '44'.repeat(32),
  externalRunnerIdentitySha256: '55'.repeat(32),
  workflowRunId: '123456', workflowRunAttempt: 1, workflowJob: 'prepare-postconditions',
  resolvedAddresses: ['203.0.113.10'], leaseExpiryTimeoutMs: 60_000,
};
const actions = [{ actionId: 'C05:dashboard:0001', attemptId: 'C05-E2-r001', caseId: 'C05',
  kind: 'dashboard_action', environmentId: 'E2', transport: 'external_ingress' }];
const dashboardProbes = [{ actionId: actions[0].actionId, region: { x: 10, y: 10, width: 20, height: 20 },
  interaction: { kind: 'remote_pixel_click', x: 30, y: 30 } }];

function ready(overrides = {}) {
  return {
    operatorVisibleState: 'ready', readyBeforePixels: true,
    readyObservedAt: '2026-09-03T00:00:00.000Z', pixelsObservedAt: '2026-09-03T00:00:00.001Z',
    handoffUrlSha256: config.handoffUrlSha256, retainedIdentitySha256: config.retainedIdentitySha256,
    candidateSha256: config.candidateSha256, scheduleSha256: config.scheduleSha256, runId: config.runId,
    offHost: true, outsideServiceHost: true, outsideServiceNetworkNamespace: true,
    externalRunnerIdentitySha256: config.externalRunnerIdentitySha256,
    pixelBytes: new Uint8Array([9, 9, 9]), ...overrides,
  };
}

function browser(overrides = {}) {
  let capture = 0;
  return {
    openHandoff: async () => ready(), resetSyntheticFixture: async () => ready(),
    captureRegion: async () => new Uint8Array([++capture, 2, 3]),
    performDashboardAction: async ({ action }) => ({ actionId: action.actionId, observed: true }),
    readViewerLeases: async () => [
      { id: 'viewer-lease', viewerRole: 'viewer', state: 'active', generation: 7 },
      { id: 'controller-lease', viewerRole: 'controller', state: 'active', generation: 8 },
    ],
    probeNetworkRecovery: async () => ready({ offlineFailureObserved: true, pixelBytes: new Uint8Array([4, 5]) }),
    probeClientRestart: async () => ready({ clientRestartObserved: true, pixelBytes: new Uint8Array([6, 7]) }),
    ...overrides,
  };
}

function artifactStore() {
  const ids = new Set();
  return { writeArtifact: async ({ artifactId, relativePath, content }) => {
    assert(!ids.has(artifactId), `duplicate artifact ${artifactId}`);
    ids.add(artifactId);
    return { artifactId, relativePath, sha256: sha256(content), byteCount: content.byteLength };
  } };
}

const input = { config, actions, dashboardProbes, browser: browser(), artifactStore: artifactStore(),
  clock: { wallNow: () => '2026-09-03T00:01:00.000Z' } };
const result = await prepareP158W9EndurancePostconditions(input);
assert.equal(result.receipt.postconditionPreparationSha256,
  sha256(Object.fromEntries(Object.entries(result.receipt).filter(([field]) => field !== 'postconditionPreparationSha256'))));
assert.equal(result.receipt.dashboardActionCount, 1);
assert.equal(result.receipt.artifactReceipts.length, 4);
assert.equal(result.preparedActions[0].postcondition.kind, 'pixel_region_transition');
assert.equal(result.eventPostconditions.viewer_expiry.baselineGeneration, 7);
assert(!JSON.stringify(result).includes(handoffUrl));
process.stdout.write('PASS captures exact synthetic visual lease network and restart preparation without raw URL custody\n');

await assert.rejects(
  prepareP158W9EndurancePostconditions({ ...input, browser: browser({
    captureRegion: async () => new Uint8Array([1, 1, 1]),
  }), artifactStore: artifactStore() }),
  (error) => error instanceof P158W9EndurancePreparationError && error.code === 'static_visual_hash_rejected',
);
process.stdout.write('PASS rejects static fabricated before and after hashes\n');

await assert.rejects(
  prepareP158W9EndurancePostconditions({ ...input, browser: browser({
    readViewerLeases: async () => [
      { id: 'viewer-lease', viewerRole: 'viewer', state: 'active', generation: 7 },
      { id: 'controller-lease', viewerRole: 'controller', state: 'active', generation: 0 },
    ],
  }), artifactStore: artifactStore() }),
  (error) => error instanceof P158W9EndurancePreparationError && error.code === 'lease_baseline_unproven',
);
process.stdout.write('PASS rejects non-positive lease generation\n');

await assert.rejects(
  prepareP158W9EndurancePostconditions({ ...input, config: { ...config,
    handoffUrl: 'https://127.0.0.1:9443/remote-view/opaque',
    handoffUrlSha256: sha256('https://127.0.0.1:9443/remote-view/opaque'),
  }, browser: browser(), artifactStore: artifactStore() }),
  (error) => error instanceof P158W9EndurancePreparationError && error.code === 'external_ingress_unproven',
);
process.stdout.write('PASS rejects loopback ingress before browser effects\n');

for (const field of ['candidateSha256', 'scheduleSha256']) {
  await assert.rejects(
    prepareP158W9EndurancePostconditions({ ...input, browser: browser({
      openHandoff: async () => ready({ [field]: 'ff'.repeat(32) }),
    }), artifactStore: artifactStore() }),
    (error) => error instanceof P158W9EndurancePreparationError && error.code === 'handoff_readiness_mismatch',
  );
}
process.stdout.write('PASS rejects mismatched candidate and schedule before postcondition capture\n');

await assert.rejects(
  prepareP158W9EndurancePostconditions({ ...input, browser: browser({
    openHandoff: async () => ready({ offHost: false }),
  }), artifactStore: artifactStore() }),
  (error) => error instanceof P158W9EndurancePreparationError && error.code === 'handoff_readiness_mismatch',
);
process.stdout.write('PASS rejects a non-external or wrong-runner readiness proof\n');

const workflow = await readFile('.github/workflows/p158-w9-endurance-preparation.yml', 'utf8');
const runner = await readFile('scripts/run-p158-w9-endurance-preparation.js', 'utf8');
assert.match(workflow, /runs-on: ubuntu-latest/);
assert.match(workflow, /validate-passive-contract:/);
assert.match(workflow, /Installation and repair remain available at segment boundaries/);
assert(!workflow.includes('continue-on-error'));
assert(!/^import .*playwright/m.test(runner));
assert.match(runner, /import\('playwright'\)/);
process.stdout.write('PASS provides passive asynchronous preparation without blocking installation or repair\n');
