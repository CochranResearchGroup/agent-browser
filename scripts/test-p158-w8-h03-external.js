#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { RETAINED_IDENTITY_FIELDS } from './lib/p158-external-handoff-oracle.js';
import { createP158W8ReviewedLiveAdapterBundle } from './lib/p158-w8-hd-adapters.js';
import {
  buildP158W8H03ExternalManifest,
  executeP158W8H03ExternalManifest,
  P158W8H03ExternalError,
  P158_W8_H03_H06_EXECUTION_CLASSIFICATION,
  validateP158W8H03ExternalManifest,
  validateP158W8H03ExternalResult,
} from './lib/p158-w8-h03-external.js';

const registry = JSON.parse(readFileSync('docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-h03-external-test' });
const expectedIdentity = Object.fromEntries(RETAINED_IDENTITY_FIELDS.map((field) => [field, `h03-${field}`]));
expectedIdentity.pixelHash = sha256(new Uint8Array([1, 2, 3]));
const seals = { freezeState: 'frozen', registrySha256: schedule.registrySha256, scheduleSha256: schedule.scheduleSha256,
  candidateSha256: '11'.repeat(32), workflowSha256: '22'.repeat(32),
  handoffUrlSha256: '33'.repeat(32), retainedIdentitySha256: sha256(expectedIdentity),
  externalVantageReceiptSha256: '45'.repeat(32), externalHandoffOracleReportSha256: '46'.repeat(32),
  fixtureRedactionReceiptSha256: '47'.repeat(32), fixtureId: 'p158-synthetic-visual-v1', expectedIdentity };
const producer = { workflowPath: '.github/workflows/p158-w8-h03-external.yml',
  workflowSha256: sha256(readFileSync(new URL('../.github/workflows/p158-w8-h03-external.yml', import.meta.url))),
  runnerPath: 'scripts/run-p158-w8-h03-external.js',
  runnerSha256: sha256(readFileSync(new URL('./run-p158-w8-h03-external.js', import.meta.url))),
  libraryPath: 'scripts/lib/p158-w8-h03-external.js',
  librarySha256: sha256(readFileSync(new URL('./lib/p158-w8-h03-external.js', import.meta.url))) };
const sourceCommit = 'a'.repeat(40);
const runnerAttestationSha256 = sha256({ provider: 'github_actions', runnerLabel: 'ubuntu-latest', sourceCommit,
  workflowPath: producer.workflowPath, workflowSha256: producer.workflowSha256, offHost: true,
  outsideServiceHost: true, outsideServiceNetworkNamespace: true });
const ids = ['browserIdSha256', 'profileIdSha256', 'sessionIdSha256', 'tabIdSha256', 'targetIdSha256'];
const stable = Object.fromEntries(ids.map((field) => [field, sha256(expectedIdentity[field.replace('Sha256', '')])]));
const transitions = ['viewer_expiry', 'route_switch', 'display_replacement', 'provider_session_replacement'];

function state(transition, phase, override = {}) {
  const after = phase === 'after';
  const changed = after ? 'bb'.repeat(32) : 'aa'.repeat(32);
  return {
    ...stable, routeIdSha256: transition === 'route_switch' ? changed : '81'.repeat(32),
    displayAllocationIdSha256: transition === 'display_replacement' ? changed : '82'.repeat(32),
    connectionIdSha256: transition === 'provider_session_replacement' ? changed : '83'.repeat(32),
    viewerLeaseIdSha256: transition === 'viewer_expiry' ? changed : '84'.repeat(32),
    presentationGeneration: after ? 2 : 1, pixelBytes: new Uint8Array([1, 2, 3]),
    operatorVisibleState: 'ready', readyBeforePixels: true,
    readyObservedAt: '2026-09-03T00:00:00.000Z', pixelsObservedAt: '2026-09-03T00:00:00.001Z',
    offHost: true, outsideServiceHost: true, outsideServiceNetworkNamespace: true,
    runnerAttestationSha256, runnerIdentitySha256: '8a'.repeat(32), handoffUrlSha256: seals.handoffUrlSha256,
    retainedIdentitySha256: seals.retainedIdentitySha256, browserLaunchCount: 1, ...override,
    websocketEndpointSha256: '89'.repeat(32),
  };
}

const h03Attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'H03');
const transitionBindings = h03Attempts.map((attempt, index) => {
  const transition = transitions[index];
  const actionId = `${attempt.attemptId}:action:001`;
  if (transition === 'viewer_expiry') return { actionId, transition, handoffUrlSha256: seals.handoffUrlSha256,
    viewerLeaseIdSha256: 'aa'.repeat(32), baselineGeneration: 1, timeoutMs: 60_000 };
  const after = state(transition, 'after');
  return { actionId, transition, handoffUrlSha256: seals.handoffUrlSha256,
    request: { action: 'service_remote_view_route_switch', params: { routeId: `synthetic-${transition}` } },
    expectedAfterProjectionSha256: sha256({ routeIdSha256: after.routeIdSha256,
      displayAllocationIdSha256: after.displayAllocationIdSha256,
      connectionIdSha256: after.connectionIdSha256, presentationGeneration: after.presentationGeneration }) };
});
const externalIngress = { publicOrigin: 'https://external.example.test', resolvedAddresses: ['203.0.113.10'],
  runnerAttestationSha256, offHost: true, outsideServiceHost: true,
  outsideServiceNetworkNamespace: true };
const manifest = buildP158W8H03ExternalManifest({ registry, schedule, seals,
  sourceCommit, externalIngress, transitionBindings, producer });
assert.equal(validateP158W8H03ExternalManifest({ manifest, registry, schedule, seals }), manifest);

function store() {
  const seen = new Set();
  return { writeArtifact: async ({ artifactId, relativePath, content }) => {
    assert(!seen.has(artifactId)); seen.add(artifactId);
    return { artifactId, relativePath, sha256: sha256(content), byteCount: content.byteLength };
  } };
}
const driver = {
  captureContinuity: async ({ action, phase }) => state(action.transition, phase),
  applyTransition: async ({ action }) => ({ actionId: action.actionId, observed: true,
    requestAttemptCount: 1, retryAttempted: false, repairAttempted: false }),
};
const result = await executeP158W8H03ExternalManifest({ manifest, driver, artifactStore: store(),
  clock: { wallNow: () => '2026-09-03T00:01:00.000Z' } });
assert.equal(validateP158W8H03ExternalResult({ result, manifest }), result);
assert.equal(result.actionCount, 4);
assert.deepEqual(P158_W8_H03_H06_EXECUTION_CLASSIFICATION.H03,
  { executableAttemptCount: 4, blockedAttemptCount: 0 });
assert.equal(Object.values(P158_W8_H03_H06_EXECUTION_CLASSIFICATION)
  .reduce((sum, entry) => sum + entry.blockedAttemptCount, 0), 5);
process.stdout.write('PASS executes four exact H03 durable-handoff transitions and blocks five unsupported H04-H06 attempts\n');

const workflowSource = readFileSync(new URL('../.github/workflows/p158-w8-h03-external.yml', import.meta.url), 'utf8');
const runnerSource = readFileSync(new URL('./run-p158-w8-h03-external.js', import.meta.url), 'utf8');
assert.match(workflowSource, /workflow_dispatch:/u);
assert.match(workflowSource, /runs-on: ubuntu-latest/u);
assert.match(workflowSource, /if: always\(\)/u);
assert.doesNotMatch(workflowSource, /continue-on-error/u);
assert.match(workflowSource, /actions\/checkout@[a-f0-9]{40}/u);
assert.match(workflowSource, /pnpm\/action-setup@fc06bc1257f339d1d5d8b3a19a8cae5388b55320/u);
assert.doesNotMatch(runnerSource, /^import .* from ['"]playwright['"]/mu);
assert.match(runnerSource, /page\.on\('websocket'/u);
assert.match(runnerSource, /await page\?\.close\(\);[\s\S]*lease\?\.state === 'expired'/u);
process.stdout.write('PASS manual workflow pins tooling and the runner proves WSS plus observed viewer expiry\n');

const resultRoot = mkdtempSync(join(tmpdir(), 'p158-w8-h03-result-'));
const resultPath = join(resultRoot, 'result.json');
writeFileSync(resultPath, `${JSON.stringify(result)}\n`);
try {
  const bundle = createP158W8ReviewedLiveAdapterBundle({ registry, schedule, seals,
    liveHookManifestSha256: '48'.repeat(32), h03ExternalExecution: { manifest, resultPath } });
  assert.equal(bundle.executionReady, true);
  assert.deepEqual(bundle.adapterBindings.filter((binding) => binding.mode === 'concrete_live')
    .map((binding) => binding.caseId), ['H03']);
  const h03Binding = bundle.adapterBindings.find((binding) => binding.caseId === 'H03');
  assert.equal(h03Binding.implementedActionCount, 4);
  assert.equal(h03Binding.blockedActionCount, 0);
  assert.equal(bundle.adapterBindings.find((binding) => binding.caseId === 'H04').mode, 'explicit_blocked');
  const adapter = bundle.w8Adapters.find((entry) => entry.caseId === 'H03');
  for (const attempt of h03Attempts) {
    const outcome = await adapter.execute({ attempt,
      requestEffect: (effectId, payload) => bundle.effects[effectId](payload) });
    assert.equal(outcome.resultState, 'passed');
    assert.equal(outcome.actionCount, 1);
  }
} finally {
  rmSync(resultRoot, { recursive: true, force: true });
}
process.stdout.write('PASS concrete-live composer requires and consumes the exact source-hashed H03 result once\n');

await assert.rejects(executeP158W8H03ExternalManifest({ manifest, driver: { ...driver,
  captureContinuity: async ({ action, phase }) => state(action.transition, phase, { handoffUrlSha256: 'ff'.repeat(32) }),
}, artifactStore: store(), clock: { wallNow: () => new Date().toISOString() } }),
(error) => error instanceof P158W8H03ExternalError && error.code === 'continuity_unproven');
process.stdout.write('PASS rejects a changed durable handoff digest\n');

assert.throws(() => buildP158W8H03ExternalManifest({ registry, schedule, seals,
  sourceCommit, externalIngress: { ...externalIngress,
    publicOrigin: 'https://127.0.0.1:9443', resolvedAddresses: ['127.0.0.1'] }, transitionBindings, producer }),
(error) => error instanceof P158W8H03ExternalError && error.code === 'external_ingress_unproven');
process.stdout.write('PASS rejects loopback ingress before effects\n');

await assert.rejects(executeP158W8H03ExternalManifest({ manifest, driver: { ...driver,
  captureContinuity: async ({ action, phase }) => state(action.transition, phase, {
    presentationGeneration: 1,
  }),
}, artifactStore: store(), clock: { wallNow: () => new Date().toISOString() } }),
(error) => error instanceof P158W8H03ExternalError && error.code === 'transition_unproven');
process.stdout.write('PASS rejects a transition without generation and declared-axis change\n');

const unsafe = structuredClone(transitionBindings);
unsafe[1].request.params.externalUrl = 'https://provider.example.test/raw';
assert.throws(() => buildP158W8H03ExternalManifest({ registry, schedule, seals,
  sourceCommit, externalIngress, transitionBindings: unsafe, producer }),
(error) => error instanceof P158W8H03ExternalError && error.code === 'unsafe_url_prohibited');
process.stdout.write('PASS rejects raw provider or internal URL material\n');

const forgedResult = structuredClone(result);
forgedResult.receipts[0].artifactReceipts = [];
forgedResult.receipts[0].receiptSha256 = sha256(Object.fromEntries(
  Object.entries(forgedResult.receipts[0]).filter(([field]) => field !== 'receiptSha256')));
forgedResult.resultSha256 = sha256(Object.fromEntries(
  Object.entries(forgedResult).filter(([field]) => field !== 'resultSha256')));
assert.throws(() => validateP158W8H03ExternalResult({ result: forgedResult, manifest }),
  (error) => error instanceof P158W8H03ExternalError && error.code === 'result_invalid');
process.stdout.write('PASS rejects a self-consistent result that omits action capture artifacts\n');
