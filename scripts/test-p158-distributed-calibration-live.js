#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  canonicalExternalDispatchDigest,
  canonicalExternalRunnerReceiptDigest,
} from './lib/p158-distributed-calibration.js';
import {
  C01_READ_ONLY_ROTATION,
  canonicalRuntimeCandidateDigest,
  createDevelopmentC01ServiceTransport,
  finalizeLiveDistributedCalibration,
  LiveCalibrationError,
  prepareLiveDistributedCalibration,
  realScheduler,
  runCli,
  startLiveDistributedCalibration,
} from './run-p158-distributed-calibration-live.js';
import { canonicalHash } from './run-p158-external-vantage.js';

const START_MS = Date.parse('2026-09-02T22:00:00.000Z');
const END_MS = START_MS + 20 * 60_000;
const HANDOFF_SHA256 = '9a'.repeat(32);

function canonicalize(value) {
  if (Array.isArray(value)) return value.map(canonicalize);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
  }
  return value;
}

function runnerHash(value) {
  return sha256(Buffer.from(JSON.stringify(canonicalize(value))));
}

function makeDispatch() {
  const durationMs = END_MS - START_MS;
  const schedule = [];
  for (let ordinal = 1; ordinal <= 25; ordinal += 1) {
    const offsetMs = Math.floor((ordinal * durationMs) / 26);
    schedule.push({ kind: 'dashboard_action', ordinal, offsetMs });
    if (ordinal % 5 === 0) schedule.push({ kind: 'handoff_reconnect', ordinal: ordinal / 5, offsetMs });
  }
  const descriptor = {
    schemaVersion: 'agent-browser.p158-external-calibration-dispatch.v1',
    planId: 'P158',
    runId: 'p158-live-driver-c01',
    candidateCommit: 'ab'.repeat(20),
    workflowRunId: '123456789',
    workflowRunAttempt: 1,
    calibrationStartAt: new Date(START_MS).toISOString(),
    calibrationEndAt: new Date(END_MS).toISOString(),
    durationMs,
    lateToleranceMs: 30_000,
    actionCountPerClient: 25,
    reconnectCountPerClient: 5,
    handoffUrlSha256: HANDOFF_SHA256,
    scheduleSha256: runnerHash(schedule),
  };
  descriptor.descriptorSha256 = canonicalExternalDispatchDigest(descriptor);
  return descriptor;
}

function makeConfig() {
  const candidate = {
    runtimeEnvironment: 'development',
    executableSha256: '11'.repeat(32),
    dashboardSha256: '22'.repeat(32),
    packageVersion: '0.24.0-development.p158',
    serviceContractVersion: 'service-ui-runtime.v1',
    installedGenerationId: 'development-generation-p158',
    runtimeManifestRevision: 'p158-runtime-revision',
  };
  candidate.candidateSha256 = canonicalRuntimeCandidateDigest(candidate);
  return {
    calibrationId: 'p158-live-driver-c01-calibration',
    runId: 'p158-live-driver-c01',
    sourceCommit: 'ab'.repeat(20),
    workflowRunId: '123456789',
    workflowRunAttempt: 1,
    candidate,
    developmentTargets: [
      {
        environmentId: 'E1', scope: 'development',
        serviceUrl: 'http://127.0.0.1:19101', dashboardUrl: 'http://127.0.0.1:19102',
        profileRoot: '/tmp/agent-browser-p158/e1-profile', handoffUrlSha256: HANDOFF_SHA256,
      },
      {
        environmentId: 'E2', scope: 'development',
        serviceUrl: 'https://service.p158.test', dashboardUrl: 'https://dashboard.p158.test',
        profileRoot: '/tmp/agent-browser-p158/e2-profile', handoffUrlSha256: HANDOFF_SHA256,
      },
    ],
    agentClientIds: Array.from({ length: 25 }, (_, index) => `p158-agent-${String(index + 1).padStart(2, '0')}`),
    externalClients: [
      { clientId: 'github-human', viewerId: 'viewer-human', paceProfile: 'human_controller' },
      { clientId: 'github-slow', viewerId: 'viewer-slow', paceProfile: 'slow_concurrency' },
    ],
    externalDispatchDescriptor: makeDispatch(),
  };
}

function clockHarness(initial = START_MS - 60_000) {
  let now = initial;
  return {
    clock: {
      wallNow: () => new Date(now).toISOString(),
      monotonicNow: () => BigInt(now) * 1_000_000n,
    },
    scheduler: {
      waitUntil: async ({ wallTime }) => { now = Math.max(now, Date.parse(wallTime)); },
    },
    advanceTo: (value) => { now = value; },
  };
}

function manifest(candidate) {
  return {
    schemaVersion: 'agent-browser.runtime-manifest.v1',
    runtimeEnvironment: 'development',
    packageVersion: candidate.packageVersion,
    serviceContractVersion: candidate.serviceContractVersion,
    dashboard: { sha256: candidate.dashboardSha256 },
    executable: { path: '/opt/agent-browser-dev/bin/agent-browser', sha256: candidate.executableSha256 },
  };
}

function fetchHarness(candidate, { failedOrdinal = null } = {}) {
  const calls = [];
  return {
    calls,
    fetch: async (url, init) => {
      calls.push({ url, init: structuredClone(init) });
      const serviceCallIndex = calls.filter((call) => !call.url.endsWith('/api/runtime/manifest') &&
        !((call.url.endsWith('/api/service/status')) && calls.length <= 4)).length;
      const isFailure = failedOrdinal !== null && serviceCallIndex === failedOrdinal;
      const body = url.endsWith('/api/runtime/manifest')
        ? manifest(candidate)
        : isFailure
          ? { success: false, failure: { code: 'injected_read_failure', message: 'first failure retained' } }
          : { success: true, data: { observed: true } };
      return {
        ok: !isFailure,
        status: isFailure ? 503 : 200,
        redirected: false,
        url,
        json: async () => structuredClone(body),
      };
    },
  };
}

function makeReceipts(prepared) {
  const receipts = prepared.externalClients.map((client, index) => ({
    schemaVersion: 'agent-browser.p158-external-calibration-receipt.v1',
    planId: 'P158', runId: prepared.runId,
    receiptId: `external-receipt-${index + 1}`,
    clientId: client.clientId, viewerId: client.viewerId, paceProfile: client.paceProfile,
    mode: 'calibration', success: true, repairAttempted: false, retryCount: 0,
    startedAt: prepared.externalDispatchDescriptor.calibrationStartAt,
    completedAt: prepared.externalDispatchDescriptor.calibrationEndAt,
    sourceCommit: prepared.sourceCommit,
    workflowRunId: prepared.workflowRunId,
    workflowRunAttempt: prepared.workflowRunAttempt,
    runner: { runId: prepared.workflowRunId, runAttempt: String(prepared.workflowRunAttempt) },
    runnerIdentity: {
      provider: 'github_actions', runnerId: `github-runner-${index + 1}`,
      runnerName: `runner-${index + 1}`, runnerOs: 'Linux', runnerArch: 'X64',
    },
    outsideServiceHost: true, outsideServiceNetworkNamespace: true, publicEgressObserved: true,
    handoff: { urlSha256: HANDOFF_SHA256 },
    calibration: { dispatchDescriptor: prepared.externalDispatchDescriptor },
    actions: [],
  }));
  for (const [kind, count] of [['dashboard_action', 50], ['handoff_reconnect', 10]]) {
    for (let ordinal = 1; ordinal <= count; ordinal += 1) {
      const receipt = receipts[(ordinal - 1) % 2];
      receipt.actions.push({
        kind, ordinal, viewerId: receipt.viewerId, attempt: 1, state: 'passed',
        observedAt: new Date(START_MS + Math.floor((ordinal * (END_MS - START_MS)) / (count + 1))).toISOString(),
        latencyMs: ordinal, retryAttempted: false, repairAttempted: false,
      });
    }
  }
  for (const receipt of receipts) receipt.receiptSha256 = canonicalExternalRunnerReceiptDigest(receipt);
  return receipts;
}

function makeAggregate(receipts, prepared) {
  const body = {
    schemaVersion: 'agent-browser.p158-external-vantage-aggregate.v1',
    planId: 'P158', runId: prepared.runId, success: true,
    repairAttempted: false, retryCount: 0, mode: 'calibration',
    clientIds: receipts.map((receipt) => receipt.clientId).sort(),
    runnerIdentitySha256s: receipts.map((receipt) => canonicalHash(receipt.runnerIdentity)).sort(),
    handoffUrlSha256: HANDOFF_SHA256,
    retainedIdentitySha256: '33'.repeat(32),
    receiptSha256s: receipts.map((receipt) => canonicalHash(receipt)).sort(),
    checks: {
      distinctOffHostClients: true, sameDurableHandoff: true, exactRetainedIdentity: true,
      noDuplicateServerBrowserLaunch: true, noInternalUrlLeak: true, allIngressChecks: true,
      calibrationComplete: true,
      sharedCalibrationWindow: prepared.externalDispatchDescriptor.descriptorSha256,
    },
  };
  return { ...body, aggregateSha256: canonicalHash(body) };
}

async function runTest(name, body) {
  try {
    await body();
    process.stdout.write(`PASS ${name}\n`);
  } catch (error) {
    error.message = `${name}: ${error.message}`;
    throw error;
  }
}

async function withRunRoot(body) {
  const root = await mkdtemp(join(tmpdir(), 'p158-live-driver-'));
  try {
    return await body(root);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

await runTest('prepares exact E1 and E2 runtime bindings without mutating input', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const before = structuredClone(config);
  const time = clockHarness();
  const network = fetchHarness(config.candidate);
  const prepared = await prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock });
  assert.deepEqual(config, before);
  assert.equal(prepared.runtimeBindings.length, 2);
  assert.equal(prepared.effectsAttempted, false);
  assert.deepEqual(network.calls.map((call) => call.init.method), ['GET', 'GET', 'GET', 'GET']);
  assert.ok(network.calls.every((call) => call.init.redirect === 'error'));
  assert.deepEqual(new Set(network.calls.map((call) => new URL(call.url).origin)), new Set([
    'http://127.0.0.1:19101', 'http://127.0.0.1:19102',
    'https://service.p158.test', 'https://dashboard.p158.test',
  ]));
  const persisted = JSON.parse(await readFile(join(runRoot, 'distributed-c01/preparation.json'), 'utf8'));
  assert.deepEqual(persisted, prepared);
  assert.doesNotMatch(JSON.stringify(persisted), /remote-view/u);
}));

await runTest('runs the frozen one-shot GET rotation across 25 clients and both origins', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const time = clockHarness();
  const network = fetchHarness(config.candidate, { failedOrdinal: 17 });
  await prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock });
  const preparationCalls = network.calls.length;
  const localEnvelope = await startLiveDistributedCalibration({
    runRoot, fetch: network.fetch, clock: time.clock, scheduler: time.scheduler,
  });
  const calls = network.calls.slice(preparationCalls);
  assert.equal(calls.length, 500);
  assert.ok(calls.every((call) => call.init.method === 'GET' && call.init.redirect === 'error'));
  assert.ok(calls.every((call) => !call.url.includes('/api/service/request')));
  assert.equal(new Set(calls.map((call) => call.init.headers['x-agent-browser-client-id'])).size, 25);
  assert.deepEqual([...new Set(calls.map((call) => new URL(call.url).origin))].sort(), [
    'http://127.0.0.1:19101', 'https://service.p158.test',
  ]);
  assert.deepEqual(calls.slice(0, 5).map((call) => new URL(call.url).pathname),
    C01_READ_ONLY_ROTATION.map((entry) => new URL(entry.path, 'https://example.test').pathname));
  const observations = JSON.parse(localEnvelope.localRun.localObservationArtifact.content).observations;
  assert.equal(observations.length, 500);
  assert.equal(observations.filter((entry) => entry.state === 'failed').length, 1);
  assert.equal(observations.find((entry) => entry.state === 'failed').failure.code, 'service_http_status');
  assert.equal(localEnvelope.transportObservations.length, 500);
  const failedTransport = localEnvelope.transportObservations.find((entry) => entry.state === 'failed');
  assert.equal(failedTransport.httpStatus, 503);
  assert.equal(failedTransport.failure.code, 'service_http_status');
  assert.equal(failedTransport.attempt, 1);
  assert.equal(failedTransport.retryAttempted, false);
  assert.equal(failedTransport.repairAttempted, false);
  const failedObservation = observations.find((entry) => entry.state === 'failed');
  assert.equal(failedObservation.observedAt, failedObservation.timingEvidence.observedAt);
  assert.equal(
    failedObservation.timingEvidence.transportElapsedMs,
    failedTransport.latencyMs,
  );
  assert.deepEqual(JSON.parse(await readFile(join(runRoot, 'distributed-c01/local-run.json'), 'utf8')), localEnvelope);
}));

await runTest('finalizes late downloaded aggregate and receipts without replaying network effects', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const time = clockHarness();
  const network = fetchHarness(config.candidate);
  const envelope = await prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock });
  await startLiveDistributedCalibration({ runRoot, fetch: network.fetch, clock: time.clock, scheduler: time.scheduler });
  const beforeFinalize = network.calls.length;
  time.advanceTo(END_MS + 8 * 60 * 60_000);
  const receipts = makeReceipts(envelope.prepared);
  const aggregate = makeAggregate(receipts, envelope.prepared);
  const receiptsBefore = structuredClone(receipts);
  const aggregateBefore = structuredClone(aggregate);
  const result = await finalizeLiveDistributedCalibration({
    runRoot, externalAggregate: aggregate, externalReceipts: receipts, clock: time.clock,
  });
  assert.equal(network.calls.length, beforeFinalize);
  assert.deepEqual(receipts, receiptsBefore);
  assert.deepEqual(aggregate, aggregateBefore);
  assert.equal(result.calibration.clean, true);
  assert.equal(result.distributedEvidence.serviceCommandCount, 500);
  assert.equal(result.distributedEvidence.dashboardActionCount, 50);
  assert.equal(result.distributedEvidence.handoffReconnectCount, 10);
  assert.equal(result.distributedEvidence.externalReplayEffectCount, 0);
  assert.equal(result.distributedEvidence.finalizedAt, new Date(END_MS + 8 * 60 * 60_000).toISOString());
  assert.ok(result.artifacts.every((artifact) => artifact.declaredSha256 === sha256(artifact.content)));
  assert.doesNotMatch(JSON.stringify(result), /remote-view/u);
  assert.deepEqual(JSON.parse(await readFile(join(runRoot, 'distributed-c01/final-result.json'), 'utf8')), result);
}));

await runTest('fails closed on roots, runtime identity, overwrite, and aggregate disagreement', async () => {
  const config = makeConfig();
  const time = clockHarness();
  const network = fetchHarness(config.candidate);
  await assert.rejects(
    () => prepareLiveDistributedCalibration({ config, runRoot: 'relative/run', fetch: network.fetch, clock: time.clock }),
    (error) => error instanceof LiveCalibrationError && error.code === 'invalid_run_root',
  );
  await assert.rejects(
    () => prepareLiveDistributedCalibration({ config, runRoot: process.cwd(), fetch: network.fetch, clock: time.clock }),
    (error) => error instanceof LiveCalibrationError && error.code === 'run_root_inside_repository',
  );
  assert.equal(network.calls.length, 0);
  await withRunRoot(async (runRoot) => {
    const wrongNetwork = fetchHarness({ ...config.candidate, executableSha256: '44'.repeat(32) });
    await assert.rejects(
      () => prepareLiveDistributedCalibration({ config, runRoot, fetch: wrongNetwork.fetch, clock: time.clock }),
      (error) => error.code === 'candidate_runtime_identity_mismatch',
    );
  });
  await withRunRoot(async (runRoot) => {
    const envelope = await prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock });
    await assert.rejects(
      () => prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock }),
      (error) => error.code === 'artifact_already_exists',
    );
    await startLiveDistributedCalibration({ runRoot, fetch: network.fetch, clock: time.clock, scheduler: time.scheduler });
    time.advanceTo(END_MS + 1);
    const receipts = makeReceipts(envelope.prepared);
    const aggregate = makeAggregate(receipts, envelope.prepared);
    aggregate.receiptSha256s[0] = '00'.repeat(32);
    aggregate.aggregateSha256 = canonicalHash(Object.fromEntries(Object.entries(aggregate)
      .filter(([key]) => key !== 'aggregateSha256')));
    const beforeFinalize = network.calls.length;
    await assert.rejects(
      () => finalizeLiveDistributedCalibration({
        runRoot, externalAggregate: aggregate, externalReceipts: receipts, clock: time.clock,
      }),
      (error) => error.code === 'external_aggregate_receipt_mismatch',
    );
    assert.equal(network.calls.length, beforeFinalize);
  });
});

await runTest('rejects a reordered action, client, or environment before fetch', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const time = clockHarness();
  const network = fetchHarness(config.candidate);
  const envelope = await prepareLiveDistributedCalibration({ config, runRoot, fetch: network.fetch, clock: time.clock });
  const transport = createDevelopmentC01ServiceTransport({ preparation: envelope, fetch: network.fetch, clock: time.clock });
  const baseline = {
    ordinal: 1, action: 'service_status', clientId: config.agentClientIds[0],
    target: config.developmentTargets[0], attempt: 1, effectClass: 'read_only',
  };
  const before = network.calls.length;
  for (const changed of [
    { ...baseline, action: 'site_policy' },
    { ...baseline, clientId: config.agentClientIds[1] },
    { ...baseline, target: config.developmentTargets[1] },
  ]) {
    await assert.rejects(
      () => transport.executeReadOnlyCommand(changed),
      (error) => error.code === 'read_only_command_outside_freeze',
    );
  }
  assert.equal(network.calls.length, before);
}));

await runTest('supports provider-free prepare, start, and finalize CLI phases', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const time = clockHarness();
  const network = fetchHarness(config.candidate);
  const output = { lines: [], write(value) { this.lines.push(value); } };
  const configPath = join(runRoot, 'input-config.json');
  await writeFile(configPath, JSON.stringify(config));
  await runCli(['prepare', '--run-root', runRoot, '--config', configPath], {
    fetch: network.fetch, clock: time.clock, stdout: output,
  });
  await runCli(['start', '--run-root', runRoot], {
    fetch: network.fetch, clock: time.clock, scheduler: time.scheduler, stdout: output,
  });
  const preparation = JSON.parse(await readFile(join(runRoot, 'distributed-c01/preparation.json'), 'utf8'));
  const receipts = makeReceipts(preparation.prepared);
  const aggregate = makeAggregate(receipts, preparation.prepared);
  const aggregatePath = join(runRoot, 'downloaded-aggregate.json');
  const receiptPaths = [join(runRoot, 'downloaded-receipt-1.json'), join(runRoot, 'downloaded-receipt-2.json')];
  await writeFile(aggregatePath, JSON.stringify(aggregate));
  await Promise.all(receipts.map((receipt, index) => writeFile(receiptPaths[index], JSON.stringify(receipt))));
  time.advanceTo(END_MS + 1);
  const callsBeforeFinalize = network.calls.length;
  await runCli([
    'finalize', '--run-root', runRoot, '--external-aggregate', aggregatePath,
    '--external-receipt', receiptPaths[0], '--external-receipt', receiptPaths[1],
  ], { fetch: network.fetch, clock: time.clock, stdout: output });
  assert.equal(network.calls.length, callsBeforeFinalize);
  assert.equal(output.lines.length, 3);
  assert.deepEqual(output.lines.map((line) => JSON.parse(line).command), ['prepare', 'start', 'finalize']);
}));

await runTest('default scheduler measures its deadline against the injected wall clock', async () => {
  let wall = START_MS - 65_000;
  const waits = [];
  const scheduler = realScheduler(
    { wallNow: () => new Date(wall).toISOString() },
    async (milliseconds) => {
      waits.push(milliseconds);
      wall += milliseconds;
    },
  );
  await scheduler.waitUntil({ wallTime: new Date(START_MS).toISOString() });
  assert.deepEqual(waits, [30_000, 30_000, 5_000]);
  assert.equal(wall, START_MS);
});

await runTest('prepares E2 through an ephemeral auth file without serializing credentials', () => withRunRoot(async (runRoot) => {
  const config = makeConfig();
  const time = clockHarness();
  const output = { write() {} };
  const configPath = join(runRoot, 'authenticated-input.json');
  const authPath = join(runRoot, 'dashboard-auth.env');
  await writeFile(configPath, JSON.stringify(config));
  await writeFile(authPath,
    'P158_DEV_DASHBOARD_USERNAME=p158-user\nP158_DEV_DASHBOARD_PASSWORD=p158-secret\n',
    { mode: 0o600 });
  const calls = [];
  const fetch = async (url, init = {}) => {
    calls.push({ url, init: structuredClone(init) });
    const parsed = new URL(url);
    if (parsed.pathname === '/api/dashboard-auth/login') {
      return { ok: true, status: 200, redirected: false, url,
        headers: { get: (name) => name.toLowerCase() === 'set-cookie'
          ? 'p158_session=opaque; Path=/; HttpOnly; Secure' : null },
        json: async () => ({ authenticated: true }) };
    }
    return { ok: true, status: 200, redirected: false, url,
      headers: { get: () => null },
      json: async () => parsed.pathname === '/api/runtime/manifest'
        ? manifest(config.candidate) : { success: true } };
  };
  await runCli([
    'prepare', '--run-root', runRoot, '--config', configPath, '--e2-auth-env', authPath,
  ], { fetch, clock: time.clock, stdout: output });
  const loginCalls = calls.filter((call) => new URL(call.url).pathname === '/api/dashboard-auth/login');
  assert.equal(loginCalls.length, 1);
  const e2Calls = calls.filter((call) => ['https://service.p158.test', 'https://dashboard.p158.test']
    .includes(new URL(call.url).origin) && new URL(call.url).pathname !== '/api/dashboard-auth/login');
  assert(e2Calls.every((call) => call.init.headers.cookie === 'p158_session=opaque'));
  const persisted = await readFile(join(runRoot, 'distributed-c01/preparation.json'), 'utf8');
  assert.doesNotMatch(persisted, /p158-user|p158-secret|p158_session/u);
}));

process.stdout.write('P158 distributed live calibration driver test passed\n');
