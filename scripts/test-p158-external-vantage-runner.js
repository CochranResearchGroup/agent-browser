#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  EXTERNAL_CALIBRATION_RECEIPT_SCHEMA,
  EXTERNAL_VANTAGE_AGGREGATE_SCHEMA,
  EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
  PINNED_PLAYWRIGHT_VERSION,
  aggregateExternalVantageReceipts,
  aggregateExternalVantageDirectory,
  buildExternalCalibrationDescriptor,
  buildExternalCalibrationSchedule,
  canonicalHash,
  classifyRenderedStreamFailure,
  classifyHandoffResolutionFailure,
  externalVantageFailureRecord,
  findInternalUrlLeaks,
  handoffResolutionReadinessGaps,
  projectHandoffResolution,
  pixelMarkerClipForIframe,
  remoteViewIframeClipObservation,
  syntheticRemoteInteractionPoint,
  redactOperatorUrl,
  runExternalVantageProbe,
  validateExternalCalibrationLeadTime,
  validateExternalVantageConfiguration,
} from './run-p158-external-vantage.js';
import { sha256 as campaignSha256 } from './lib/p158-campaign-controller.js';
import {
  canonicalExternalDispatchDigest,
  canonicalExternalRunnerReceiptDigest,
} from './lib/p158-distributed-calibration.js';

const workflow = readFileSync('.github/workflows/p158-external-vantage.yml', 'utf8');
const runnerSource = readFileSync('scripts/run-p158-external-vantage.js', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));

assert.match(workflow, /^\s*workflow_dispatch:/m, 'external vantage must be manual-only');
assert.doesNotMatch(workflow, /^\s*(push|pull_request|schedule):/m, 'external vantage must have no automatic trigger');
assert.match(workflow, /human-controller:[\s\S]*runs-on: ubuntu-latest/);
assert.match(workflow, /slow-concurrency-client:[\s\S]*runs-on: ubuntu-latest/);
assert.match(workflow, /environment: p158-external-vantage/g);
assert.match(workflow, /P158_DEV_HANDOFF_URL: \$\{\{ secrets\.P158_DEV_HANDOFF_URL \}\}/g);
assert.match(workflow, /P158_DEV_DASHBOARD_PASSWORD: \$\{\{ secrets\.P158_DEV_DASHBOARD_PASSWORD \}\}/g);
assert.equal((workflow.match(/P158_DEV_VISUAL_FIXTURE_ATTESTATION_JSON:/g) || []).length, 2);
assert.match(workflow, /--pace-profile human_controller/);
assert.match(workflow, /--pace-profile slow_concurrency/);
assert.match(workflow, /Delay single-viewer readiness client[\s\S]*sleep 45/);
assert.match(workflow, /probe_mode:[\s\S]*default: calibration/);
assert.match(workflow, /calibration_start_at:[\s\S]*RFC3339 UTC start/);
assert.equal((workflow.match(/P158_CALIBRATION_START_AT:/g) || []).length, 2);
assert.equal((workflow.match(/timeout-minutes: 75/g) || []).length, 2);
assert.match(workflow, /aggregate:[\s\S]*timeout-minutes: 10/);
assert.match(workflow, /retention-days: \$\{\{ inputs\.artifact_retention_days \}\}/g);
assert.match(workflow, /actions\/checkout@[a-f0-9]{40}/g);
assert.match(workflow, /actions\/setup-node@[a-f0-9]{40}/g);
assert.match(workflow, /pnpm\/action-setup@[a-f0-9]{40}/g);
assert.equal(
  [...workflow.matchAll(/pnpm\/action-setup@([a-f0-9]{40})/g)].map((match) => match[1]).join(','),
  Array(3).fill('fc06bc1257f339d1d5d8b3a19a8cae5388b55320').join(','),
  'all pnpm setup uses must pin the API-verified v5 commit',
);
assert.match(workflow, /actions\/upload-artifact@[a-f0-9]{40}/g);
assert.match(workflow, /actions\/download-artifact@[a-f0-9]{40}/g);
assert.doesNotMatch(workflow, /nick-fields\/retry|retry-count|max-attempts/);
assert.match(workflow, /aggregate:[\s\S]*needs: \[human-controller, slow-concurrency-client\][\s\S]*if: always\(\)/);
assert.match(workflow, /P158_EXTERNAL_JOB_RESULTS_JSON/);
assert.equal(packageJson.devDependencies.playwright, PINNED_PLAYWRIGHT_VERSION);
assert.match(runnerSource, /recordVideo:/);
assert.match(runnerSource, /failure-receipt\.json/);
assert.match(runnerSource, /artifactReceipts\(outputDir\)/);
assert.doesNotMatch(runnerSource, /data-operator-visible-state/);
assert.doesNotMatch(runnerSource, /frameLocator\([^)]*\)\.locator/);
assert.doesNotMatch(runnerSource, /recordHar/);
assert.doesNotMatch(runnerSource, /jsonBodies/);
assert.equal((runnerSource.match(/response\.json\(\)/g) || []).length, 1);
assert.match(runnerSource, /responsePath === '\/api\/service\/request'/);
assert.match(runnerSource, /requestPayload\?\.action !== 'service_remote_view_handoff_resolve'/);
assert.match(runnerSource, /application\/x-content-excluded-at-capture/);
const calibrationSchedule = buildExternalCalibrationSchedule();
assert.equal(calibrationSchedule.filter((event) => event.kind === 'dashboard_action').length, 25);
assert.equal(calibrationSchedule.filter((event) => event.kind === 'handoff_reconnect').length, 5);
assert.equal(calibrationSchedule.at(-1).offsetMs < 20 * 60 * 1000, true);
assert.equal(calibrationSchedule.at(-1).offsetMs > 19 * 60 * 1000, true);
assert.throws(() => buildExternalCalibrationSchedule({ durationMs: 19 * 60 * 1000 }));
const calibrationDescriptor = buildExternalCalibrationDescriptor({
  runId: 'p158-test-run',
  candidateCommit: '1'.repeat(40),
  workflowRunId: '1001',
  workflowRunAttempt: 1,
  handoffUrlSha256: canonicalHash('same-handoff'),
  calibrationStartAt: '2026-09-02T12:00:00Z',
});
assert.equal(calibrationDescriptor.calibrationStartAt, '2026-09-02T12:00:00.000Z');
assert.equal(calibrationDescriptor.calibrationEndAt, '2026-09-02T12:20:00.000Z');
assert.match(calibrationDescriptor.descriptorSha256, /^[a-f0-9]{64}$/);
assert.equal(calibrationDescriptor.descriptorSha256, canonicalExternalDispatchDigest(calibrationDescriptor));
assert.equal(
  validateExternalCalibrationLeadTime(calibrationDescriptor, Date.parse('2026-09-02T11:57:00Z')).leadTimeMs,
  3 * 60 * 1000,
);
assert.throws(
  () => validateExternalCalibrationLeadTime(calibrationDescriptor, Date.parse('2026-09-02T11:59:00Z')),
  /at least 120000ms/,
);

const env = {
  P158_DEV_HANDOFF_URL: 'https://external.example.test/remote-view/handoff-secret',
  P158_DEV_DASHBOARD_USERNAME: 'operator',
  P158_DEV_DASHBOARD_PASSWORD: 'never-print-me',
  P158_DEV_EXPECTED_IDENTITY_JSON: JSON.stringify(identity()),
  P158_DEV_PIXEL_MARKER_REGION_JSON: JSON.stringify({
    coordinateSpace: 'remote-view-iframe', x: 100, y: 200, width: 80, height: 40,
  }),
  P158_DEV_VISUAL_FIXTURE_ATTESTATION_JSON: JSON.stringify({
    fixtureId: 'synthetic-pixel-marker-v1',
    syntheticOnly: true,
    forbiddenPrivateFieldsExcluded: true,
    redactionReceiptSha256: 'b'.repeat(64),
  }),
  P158_RUN_ID: 'p158-test-run',
  GITHUB_ACTIONS: 'true',
  RUNNER_ENVIRONMENT: 'github-hosted',
  GITHUB_RUN_ID: '1001',
  GITHUB_RUN_ATTEMPT: '1',
  GITHUB_JOB: 'human-controller',
  RUNNER_NAME: 'GitHub Actions 1',
  RUNNER_OS: 'Linux',
  RUNNER_ARCH: 'X64',
};
const typedStreamFailure = new Error('The external dashboard did not render the prepared remote pixel marker');
typedStreamFailure.code = 'external_stream_auth_failed';
typedStreamFailure.details = {
  iframeCount: 1,
  iframePathClasses: ['unexpected'],
  streamSignInExpired: true,
};
assert.deepEqual(externalVantageFailureRecord(typedStreamFailure, env), {
  code: 'external_stream_auth_failed',
  message: 'The external dashboard did not render the prepared remote pixel marker',
  details: typedStreamFailure.details,
});
assert.equal(externalVantageFailureRecord(new Error('plain failure'), env).code, 'external_vantage_probe_failed');
const unsafeDetailsFailure = new Error('bounded');
unsafeDetailsFailure.code = 'external_stream_route_invalid';
unsafeDetailsFailure.details = { rawUrl: env.P158_DEV_HANDOFF_URL, pageText: 'private content' };
assert.equal('details' in externalVantageFailureRecord(unsafeDetailsFailure, env), false);
assert.equal(classifyRenderedStreamFailure({
  bodyText: 'Stream sign-in expired.',
  iframePaths: ['/'],
  observedPixelHash: 'a'.repeat(64),
}).code, 'external_stream_auth_failed');
assert.equal(classifyRenderedStreamFailure({
  bodyText: 'Connecting to CDP stream',
  iframePaths: [],
  observedPixelHash: 'a'.repeat(64),
}).code, 'external_stream_not_rendered');
assert.equal(classifyRenderedStreamFailure({
  bodyText: 'Workspace ready',
  iframePaths: ['/'],
  observedPixelHash: 'a'.repeat(64),
}).code, 'external_stream_route_invalid');
assert.equal(classifyRenderedStreamFailure({
  bodyText: 'Workspace ready',
  iframePaths: ['/guacamole/'],
  observedPixelHash: 'a'.repeat(64),
}).code, 'external_stream_identity_marker_missing');
assert.equal(classifyRenderedStreamFailure({
  bodyText: 'No embeddable stream',
  iframePaths: [],
  observedPixelHash: null,
}).code, 'external_stream_not_embeddable');
assert.deepEqual(
  pixelMarkerClipForIframe(
    { coordinateSpace: 'remote-view-iframe', x: 100, y: 200, width: 80, height: 40 },
    { x: 310, y: 337, width: 1110, height: 641 },
  ),
  { x: 410, y: 537, width: 80, height: 40 },
);
assert.throws(
  () => pixelMarkerClipForIframe(
    { coordinateSpace: 'remote-view-iframe', x: 1000, y: 600, width: 200, height: 100 },
    { x: 310, y: 337, width: 1110, height: 641 },
  ),
  /does not fit the rendered remote-view iframe/,
);
assert.equal(
  remoteViewIframeClipObservation(
    { coordinateSpace: 'remote-view-iframe', x: 100, y: 200, width: 80, height: 40 },
    0,
    null,
  ),
  null,
  'a transient zero-iframe dashboard render must remain eligible to converge',
);
assert.deepEqual(
  remoteViewIframeClipObservation(
    { coordinateSpace: 'remote-view-iframe', x: 100, y: 200, width: 80, height: 40 },
    1,
    { x: 10, y: 20, width: 1000, height: 700 },
  ),
  { x: 110, y: 220, width: 80, height: 40 },
);
assert.throws(
  () => remoteViewIframeClipObservation(
    { coordinateSpace: 'remote-view-iframe', x: 100, y: 200, width: 80, height: 40 },
    2,
    { x: 10, y: 20, width: 1000, height: 700 },
  ),
  /exactly one iframe, observed 2/,
);
assert.deepEqual(
  syntheticRemoteInteractionPoint(
    { coordinateSpace: 'remote-view-iframe', x: 300, y: 300, width: 400, height: 100 },
    { x: 310, y: 337, width: 1110, height: 641 },
  ),
  { x: 810, y: 687 },
  'synthetic input must land in the attested page marker, not the remote browser tab strip',
);
assert.equal(
  validateExternalVantageConfiguration({ env, clientId: 'external-runner-human', paceProfile: 'human_controller' }).handoff.origin,
  'https://external.example.test',
);
for (const invalidUrl of [
  'http://external.example.test/remote-view/handoff-secret',
  'https://127.0.0.1/remote-view/handoff-secret',
  'https://10.1.2.3/remote-view/handoff-secret',
  'https://internal.local/remote-view/handoff-secret',
  'https://external.example.test/guacamole/#/client/raw',
]) {
  assert.throws(() => validateExternalVantageConfiguration({
    env: { ...env, P158_DEV_HANDOFF_URL: invalidUrl },
    clientId: 'external-runner-human',
    paceProfile: 'human_controller',
  }), /public HTTPS durable remote-view URL/);
}
assert.throws(() => validateExternalVantageConfiguration({
  env: { ...env, RUNNER_ENVIRONMENT: 'self-hosted' },
  clientId: 'external-runner-human',
  paceProfile: 'human_controller',
}), /GitHub-hosted runner/);
assert.throws(() => validateExternalVantageConfiguration({
  env: {
    ...env,
    P158_DEV_VISUAL_FIXTURE_ATTESTATION_JSON: JSON.stringify({
      fixtureId: 'unsafe',
      syntheticOnly: false,
      forbiddenPrivateFieldsExcluded: true,
      redactionReceiptSha256: 'b'.repeat(64),
    }),
  },
  clientId: 'external-runner-human',
  paceProfile: 'human_controller',
}), /synthetic redaction boundary/);
assert.throws(() => validateExternalVantageConfiguration({
  env: { ...env, P158_DEV_PIXEL_MARKER_REGION_JSON: JSON.stringify({ x: 1400, y: 0, width: 80, height: 40 }) },
  clientId: 'external-runner-human',
  paceProfile: 'human_controller',
}), /frozen 1440 by 1000 viewport/);
assert.equal(
  redactOperatorUrl(env.P158_DEV_HANDOFF_URL),
  'https://external.example.test/remote-view/%3Credacted%3E',
);

const leaks = findInternalUrlLeaks([
  { evidenceId: 'public', role: 'iframe_src', url: 'https://external.example.test/frame' },
  { evidenceId: 'public-guac-frame', role: 'iframe_src', url: 'https://external.example.test/guacamole/#/client/frame' },
  { evidenceId: 'public-guac-socket', role: 'websocket_endpoint', url: 'wss://external.example.test/guacamole/websocket-tunnel' },
  { evidenceId: 'loopback', role: 'location_header', url: 'http://127.0.0.1:4948/' },
  { evidenceId: 'private', role: 'websocket_endpoint', url: 'wss://10.1.2.3/socket' },
  { evidenceId: 'raw-provider', role: 'reconnect_target', url: 'https://external.example.test/guacamole/#/client/raw' },
]);
assert.deepEqual(leaks.map((item) => item.evidenceId), ['loopback', 'private', 'raw-provider']);

const urlPolicyFailure = new Error('Internal URL evidence detected in 3 observations');
urlPolicyFailure.code = 'external_url_policy_violation';
urlPolicyFailure.details = {
  urlFindingCount: 3,
  urlFindingRoles: ['location_header', 'reconnect_target', 'websocket_endpoint'],
  urlFindingCodes: ['loopback_url_leak', 'private_network_url_leak', 'raw_guacamole_url_leak'],
};
assert.deepEqual(externalVantageFailureRecord(urlPolicyFailure, env), {
  code: 'external_url_policy_violation',
  message: 'Internal URL evidence detected in 3 observations',
  details: urlPolicyFailure.details,
});

const projectedResolution = projectHandoffResolution({
  status: 'ready',
  resolved: true,
  handoffUrl: 'https://external.example.test/remote-view/secret',
  providerExternalUrl: 'https://external.example.test/provider/opaque',
  forbiddenPrivateBody: { password: 'must-never-survive-projection' },
  tab: { browserId: 'browser-1', profileId: 'profile-1' },
});
assert.equal(projectedResolution.status, 'ready');
assert.equal(projectedResolution.urlObservations.length, 2);
assert.doesNotMatch(JSON.stringify(projectedResolution), /must-never-survive-projection|password/);
const closedResolution = projectHandoffResolution({
  status: 'closed',
  resolved: false,
  reopenRequired: true,
  handoffId: 'must-not-be-projected-into-failure-details',
});
assert.deepEqual(classifyHandoffResolutionFailure(closedResolution), {
  code: 'handoff_target_closed_operator_action_required',
  message: 'The durable handoff target is closed and requires explicit operator reopening',
  details: { resolutionStatus: 'closed', reopenRequired: true },
});
assert.equal(classifyHandoffResolutionFailure({ status: 'converging' }), null);
assert.deepEqual(handoffResolutionReadinessGaps({
  status: 'ready',
  resolved: false,
  browserId: 'browser-1',
  targetId: 'target-1',
  viewStreamProvider: 'rdp_gateway',
  presentationGeneration: 2,
  presentationReceipt: {
    generation: 1,
    dashboardDeploymentGeneration: '',
    logicalBrowserId: 'browser-2',
    daemonOwnerGeneration: 0,
    processInstanceDigest: '',
    targetId: 'target-2',
    requiredStreamProvider: 'cdp',
    observedStreamProvider: 'rdp_gateway',
    state: 'converging',
  },
}), [
  'resolved_false',
  'receipt_generation_mismatch',
  'dashboard_generation_missing',
  'logical_browser_mismatch',
  'daemon_generation_missing',
  'process_identity_missing',
  'target_mismatch',
  'required_provider_mismatch',
  'observed_provider_mismatch',
  'receipt_not_ready',
]);
const resolutionTimeout = new Error('Authoritative ready handoff resolution was not observed before timeout');
resolutionTimeout.code = 'handoff_resolution_observation_timeout';
resolutionTimeout.details = {
  resolutionObservationCount: 1,
  resolutionStatuses: ['ready'],
  resolutionReadinessGaps: ['resolved_false', 'receipt_generation_mismatch'],
};
assert.deepEqual(externalVantageFailureRecord(resolutionTimeout, env), {
  code: 'handoff_resolution_observation_timeout',
  message: resolutionTimeout.message,
  details: resolutionTimeout.details,
});
assert.deepEqual(classifyHandoffResolutionFailure({
  status: 'failed',
  failureCode: 'service_state_lock_timeout',
  effectState: 'effect_uncertain',
  retryDisposition: 'inspect_before_retry',
  waitMs: 1001,
}), {
  code: 'service_state_lock_timeout',
  message: 'Durable handoff resolution failed while waiting for Service State',
  details: {
    resolutionStatus: 'failed',
    failureCode: 'service_state_lock_timeout',
    effectState: 'effect_uncertain',
    retryDisposition: 'inspect_before_retry',
    waitMs: 1001,
  },
});
assert.match(
  runnerSource,
  /resolverEnvelope\?\.success !== true[\s\S]*failureCode[\s\S]*handoffResolutions\.push/,
  'unsuccessful resolver envelopes must remain available to the typed failure classifier',
);

const receipts = [
  receipt('external-runner-human', 'runner-human'),
  receipt('external-runner-slow', 'runner-slow'),
];
const before = JSON.stringify(receipts);
const aggregate = aggregateExternalVantageReceipts(receipts, { runId: 'p158-test-run' });
assert.equal(aggregate.schemaVersion, EXTERNAL_VANTAGE_AGGREGATE_SCHEMA);
assert.equal(aggregate.success, true);
assert.equal(aggregate.checks.distinctOffHostClients, true);
assert.equal(JSON.stringify(receipts), before, 'aggregation must not mutate evidence');
assert.deepEqual(
  aggregateExternalVantageReceipts([...receipts].reverse(), { runId: 'p158-test-run' }),
  aggregate,
  'aggregation must be input-order deterministic',
);
const w8ActionIds = ['open', 'interact', 'disconnect', 'reopen'].map((kind) => `H01-E2-r001:action:${kind}`);
const w8Receipts = receipts.map((item) => ({
  ...structuredClone(item),
  w8ActionManifestSha256: 'd'.repeat(64),
  w8ActionObservations: ['open', 'interact', 'disconnect', 'reopen'].map((runnerAction, index) => ({
    actionId: w8ActionIds[index],
    attemptId: 'H01-E2-r001',
    caseId: 'H01',
    runnerAction,
    clientId: item.clientId,
    viewerId: `viewer-${item.clientId}`,
    observedAt: `2026-09-03T01:00:0${index}.000Z`,
    eventKind: ['page_open_ready', 'human_paced_interaction_completed', 'playwright_page_closed', 'same_handoff_reopened_ready'][index],
    evidenceArtifactId: `${item.clientId}-${runnerAction}`,
    handoffContinuityObserved: true,
    retainedIdentityObserved: true,
    retryAttempted: false,
    repairAttempted: false,
  })),
}));
const w8Aggregate = aggregateExternalVantageReceipts(w8Receipts, { runId: 'p158-test-run' });
assert.equal(w8Aggregate.w8ActionManifestSha256, 'd'.repeat(64));
assert.equal(w8Aggregate.w8ActionObservations.length, 4);
assert(w8Aggregate.w8ActionObservations.every((entry) => entry.observations.length === 2));
assert.throws(
  () => aggregateExternalVantageReceipts([
    w8Receipts[0],
    { ...w8Receipts[1], w8ActionObservations: w8Receipts[1].w8ActionObservations.slice(1) },
  ], { runId: 'p158-test-run' }),
  /lacks exact H01 observations/,
);

const calibrationReceipts = receipts.map((item) => {
  const isHuman = item.paceProfile === 'human_controller';
  const viewerId = isHuman ? 'external-viewer-human' : 'external-viewer-slow';
  const events = buildExternalCalibrationSchedule().map((event) => ({
    ...event,
    observedAt: new Date(
      Date.parse(calibrationDescriptor.calibrationStartAt) + event.offsetMs,
    ).toISOString(),
    latencyMs: 10,
  }));
  const body = {
    ...structuredClone(item),
    schemaVersion: EXTERNAL_CALIBRATION_RECEIPT_SCHEMA,
    mode: 'calibration',
    receiptId: `receipt-${viewerId}`,
    viewerId,
    sourceCommit: '1'.repeat(40),
    workflowRunId: '1001',
    workflowRunAttempt: 1,
    startedAt: calibrationDescriptor.calibrationStartAt,
    completedAt: calibrationDescriptor.calibrationEndAt,
    runner: { ...item.runner, runId: '1001', runAttempt: '1' },
    runnerIdentity: {
      provider: 'github_actions',
      runnerId: item.runner.runnerIdentitySha256,
      runnerName: item.clientId,
      runnerOs: 'Linux',
      runnerArch: 'X64',
    },
    outsideServiceHost: true,
    outsideServiceNetworkNamespace: true,
    publicEgressObserved: true,
    calibration: {
    actualDurationMs: 20 * 60 * 1000,
    actionCount: 25,
    reconnectCount: 5,
    dispatchDescriptor: calibrationDescriptor,
    runnerReadyAt: '2026-09-02T11:59:59.000Z',
    runnerStartDelayMs: -1000,
    runnerQueueDelayMs: 0,
      events,
    },
    actions: events.map((event) => ({
      kind: event.kind,
      ordinal: event.ordinal * 2 - (isHuman ? 1 : 0),
      viewerId,
      attempt: 1,
      state: 'passed',
      observedAt: event.observedAt,
      latencyMs: event.latencyMs,
      retryAttempted: false,
      repairAttempted: false,
    })),
  };
  return { ...body, receiptSha256: campaignSha256(body) };
});
assert.ok(calibrationReceipts.every((item) =>
  item.receiptSha256 === canonicalExternalRunnerReceiptDigest(item)));
assert.equal(
  aggregateExternalVantageReceipts(calibrationReceipts, { runId: 'p158-test-run' }).checks.calibrationComplete,
  true,
);
const incompleteCalibration = structuredClone(calibrationReceipts);
incompleteCalibration[1].calibration.actionCount = 24;
incompleteCalibration[1].receiptSha256 = campaignSha256((({ receiptSha256, ...body }) => body)(incompleteCalibration[1]));
assert.throws(
  () => aggregateExternalVantageReceipts(incompleteCalibration, { runId: 'p158-test-run' }),
  /calibration counts or duration are incomplete/,
);

for (const mutate of [
  (values) => { values[1].clientId = values[0].clientId; },
  (values) => { values[1].runner.runnerIdentitySha256 = values[0].runner.runnerIdentitySha256; },
  (values) => { values[1].handoff.urlSha256 = 'f'.repeat(64); },
  (values) => { values[1].initialIdentity.browserId = 'wrong-browser'; },
  (values) => { values[1].serverPhysicalBrowserLaunchDelta = 1; },
  (values) => { values[1].internalUrlLeakCount = 1; },
  (values) => { values[1].retryCount = 1; },
]) {
  const defective = structuredClone(receipts);
  mutate(defective);
  assert.throws(() => aggregateExternalVantageReceipts(defective, { runId: 'p158-test-run' }));
}

const fixtureRoot = mkdtempSync(join(tmpdir(), 'p158-external-runner-'));
try {
  const failureRoot = join(fixtureRoot, 'failure');
  mkdirSync(failureRoot, { recursive: true });
  writeFileSync(join(failureRoot, 'partial.har'), 'synthetic partial evidence');
  await assert.rejects(() => runExternalVantageProbe({
    env: { ...env, P158_DEV_DASHBOARD_PASSWORD: '' },
    clientId: 'external-runner-human',
    paceProfile: 'human_controller',
    outputDir: failureRoot,
  }));
  const failureReceipt = JSON.parse(readFileSync(join(failureRoot, 'failure-receipt.json'), 'utf8'));
  assert.equal(failureReceipt.success, false);
  assert.equal(failureReceipt.retryCount, 0);
  assert.equal(failureReceipt.artifacts[0].relativePath, 'partial.har');
  assert.equal(failureReceipt.artifacts[0].bytes, 26);
  assert.match(failureReceipt.artifacts[0].sha256, /^[a-f0-9]{64}$/);
  assert.doesNotMatch(JSON.stringify(failureReceipt), /never-print-me|handoff-secret/);

  const failureDownloads = join(fixtureRoot, 'failure-downloads');
  mkdirSync(join(failureDownloads, 'human'), { recursive: true });
  mkdirSync(join(failureDownloads, 'slow'), { recursive: true });
  writeFileSync(
    join(failureDownloads, 'human', 'failure-receipt.json'),
    JSON.stringify({ ...failureReceipt, clientId: 'external-runner-human' }),
  );
  writeFileSync(
    join(failureDownloads, 'slow', 'failure-receipt.json'),
    JSON.stringify({ ...failureReceipt, clientId: 'external-runner-slow' }),
  );
  const failedReceiptAggregate = await aggregateExternalVantageDirectory(
    failureDownloads,
    join(fixtureRoot, 'failure-aggregate', 'receipt.json'),
    'p158-test-run',
    { 'human-controller': 'failure', 'slow-concurrency-client': 'failure' },
  );
  assert.equal(failedReceiptAggregate.observedReceiptCount, 2);
  assert.deepEqual(
    failedReceiptAggregate.observedClientIds,
    ['external-runner-human', 'external-runner-slow'],
  );

  const aggregatePath = join(fixtureRoot, 'aggregate', 'receipt.json');
  const failedAggregate = await aggregateExternalVantageDirectory(
    join(fixtureRoot, 'missing-downloads'),
    aggregatePath,
    'p158-test-run',
    { 'human-controller': 'failure', 'slow-concurrency-client': 'cancelled' },
  );
  assert.equal(failedAggregate.success, false);
  assert.equal(existsSync(aggregatePath), true);
  assert.equal(JSON.parse(readFileSync(aggregatePath, 'utf8')).success, false);
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

const captureVisitSource = runnerSource.slice(
  runnerSource.indexOf('async function captureVisit'),
  runnerSource.indexOf('async function waitForExpectedPixelMarker'),
);
assert.ok(
  captureVisitSource.indexOf('const pixelHash = await waitForExpectedPixelMarker')
    < captureVisitSource.indexOf("await page.screenshot({ path: screenshotPath"),
  'the full screenshot must preserve the marker-ready state, not the transient pre-convergence state',
);

console.log('Plan 0158 external vantage runner provider-free checks passed');

function identity() {
  return {
    browserId: 'browser-1',
    profileId: 'profile-1',
    sessionId: 'session-1',
    tabId: 'tab-1',
    targetId: 'target-1',
    visibleUrl: 'https://synthetic.example.test/page',
    pageMarker: 'marker-1',
    pixelHash: 'a'.repeat(64),
  };
}

function receipt(clientId, runnerHashSeed) {
  const retained = identity();
  return {
    schemaVersion: EXTERNAL_VANTAGE_RECEIPT_SCHEMA,
    planId: 'P158',
    runId: 'p158-test-run',
    mode: 'readiness',
    clientId,
    paceProfile: clientId.includes('human') ? 'human_controller' : 'slow_concurrency',
    success: true,
    repairAttempted: false,
    retryCount: 0,
    runner: { runnerIdentitySha256: canonicalHash(runnerHashSeed) },
    handoff: { urlSha256: canonicalHash('same-handoff') },
    expectedIdentity: retained,
    initialIdentity: retained,
    reconnectIdentity: retained,
    serverPhysicalBrowserLaunchDelta: 0,
    internalUrlLeakCount: 0,
    ingressChecks: ['dns', 'tls', 'redirect', 'cookie', 'websocket', 'iframe', 'form_action', 'reconnect']
      .map((kind) => ({ kind, state: 'passed' })),
    artifacts: [
      { relativePath: 'network.redacted.har', sha256: 'a'.repeat(64), bytes: 1 },
      { relativePath: 'initial.png', sha256: 'b'.repeat(64), bytes: 1 },
      { relativePath: 'video/initial.webm', sha256: 'c'.repeat(64), bytes: 1 },
    ],
    visualFixtureAttestation: {
      fixtureId: 'synthetic-pixel-marker-v1',
      syntheticOnly: true,
      forbiddenPrivateFieldsExcluded: true,
      redactionReceiptSha256: 'd'.repeat(64),
    },
    oracle: { passed: true },
  };
}
