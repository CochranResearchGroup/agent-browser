#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { EventEmitter } from 'node:events';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  retainedAnchorReceiptSha256,
} from './lib/p158-retained-authenticated-anchor.js';
import {
  p158ExternalRunName,
  readExactDownloadedReceipt,
  runP158RetainedAnchorLiveAdapter,
  selectExactDispatchedWorkflowRun,
} from './lib/p158-retained-anchor-live-adapter.js';
import { createP158GitHubLiveProvider } from './lib/p158-retained-anchor-github-provider.js';

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
}

function digest(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

const runId = 'p158-live-adapter-test';
const anchorId = 'retained-anchor-1';
const expectedCommit = 'a'.repeat(40);
const handoffUrlSha256 = 'b'.repeat(64);
const branch = 'test/p158-live-adapter';
const runName = p158ExternalRunName(runId, expectedCommit);
const privateValues = [
  'https://public.example.test/remote-view/private-handoff',
  'private-operator',
  'private-password',
];
const config = {
  runId, anchorId, expectedCommit, handoffUrlSha256, branch,
  calibrationStartAt: '2026-09-05T15:00:00.000Z',
  probeMode: 'readiness', artifactRetentionDays: 2,
  anchorTimeoutMs: 1000, runIdentityTimeoutMs: 1000,
  workflowTimeoutMs: 1000, anchorExitTimeoutMs: 1000,
  sensitiveValues: privateValues,
};

function anchorReceipt(phase) {
  const body = {
    schemaVersion: 'agent-browser.p158-retained-authenticated-anchor-receipt.v1',
    planId: 'P158', runId, anchorId, phase, sequence: phase === 'ready' ? 1 : 2,
    result: 'passed', observedAt: '2026-09-05T14:00:00.000Z', handoffUrlSha256,
    expectedMarkerSha256: 'c'.repeat(64),
    evidence: { authenticatedSession: true, markerMatched: true, iframeReady: true,
      oraclePassed: true, oracleFindingCodes: [] },
    stopReason: phase === 'final' ? 'sigterm' : null, failureCode: null,
    maximumNavigationAttempts: 1, retryAttempted: false, repairAttempted: false,
    reconnectAttempted: false, productActionAttempted: false,
    privatePixelsRetained: false, rawUrlRetained: false, secretInputRetained: false,
  };
  return { ...body, receiptSha256: retainedAnchorReceiptSha256(body) };
}

function externalEvidence() {
  const client = (clientId, paceProfile) => ({
    schemaVersion: 'agent-browser.p158-external-vantage-receipt.v1', planId: 'P158',
    runId, clientId, paceProfile, success: true, handoff: { urlSha256: handoffUrlSha256 },
    repairAttempted: false, retryCount: 0, runnerRetryCount: 0,
    oracle: { passed: true }, serverPhysicalBrowserLaunchDelta: 0, internalUrlLeakCount: 0,
  });
  const human = client('external-runner-human', 'human_controller');
  const slow = client('external-runner-slow', 'slow_concurrency');
  const body = {
    schemaVersion: 'agent-browser.p158-external-vantage-aggregate.v1', planId: 'P158',
    runId, success: true, handoffUrlSha256, repairAttempted: false, retryCount: 0,
    runnerRetryCount: 0, clientIds: [human.clientId, slow.clientId],
    receiptSha256s: [digest(human), digest(slow)],
    checks: { distinctOffHostClients: true, sameDurableHandoff: true,
      exactRetainedIdentity: true, noDuplicateServerBrowserLaunch: true,
      noInternalUrlLeak: true, allIngressChecks: true },
  };
  return { human, slow, aggregate: { ...body, aggregateSha256: digest(body) } };
}

function artifactDirs(root, evidence, { missing = null, duplicate = null } = {}) {
  const result = {};
  for (const role of ['human', 'slow', 'aggregate']) {
    const dir = join(root, role);
    mkdirSync(dir, { recursive: true });
    result[role] = dir;
    if (role === missing) continue;
    const name = role === 'aggregate' ? 'p158-external-vantage-receipt.json' : 'receipt.json';
    const value = role === 'aggregate' ? evidence.aggregate : evidence[role];
    writeFileSync(join(dir, name), JSON.stringify(value));
    if (role === duplicate) writeFileSync(join(dir, 'failure-receipt.json'), JSON.stringify(value));
  }
  return result;
}

function fakeProvider({ root, workflowConclusion = 'success', runSelector = null,
  terminalError = null, artifactOptions = {} } = {}) {
  const evidence = externalEvidence();
  const dirs = artifactDirs(root, evidence, artifactOptions);
  const events = [];
  const aggregates = [];
  const child = { terminate: async (signal) => events.push(`terminate:${signal}`) };
  return {
    events, aggregates,
    provider: {
      startAnchor: async () => { events.push('start'); return child; },
      waitForAnchorReceipts: async ({ phase, signal }) => {
        events.push(`anchor:${phase}:${signal ? 'signal' : 'cleanup'}`);
        return phase === 'ready' ? [anchorReceipt('ready')] : [anchorReceipt('ready'), anchorReceipt('final')];
      },
      dispatchWorkflow: async ({ inputs }) => {
        events.push('dispatch');
        assert.equal(inputs.expected_commit, expectedCommit);
        assert.equal(inputs.campaign_run_id, runId);
      },
      waitForDispatchedWorkflowRun: async () => {
        events.push('identify');
        if (runSelector) return runSelector();
        return { workflowRunId: '33950000001' };
      },
      waitForWorkflowTerminal: async () => {
        events.push('terminal');
        if (terminalError) throw terminalError;
        return { conclusion: workflowConclusion };
      },
      downloadArtifact: async ({ role }) => { events.push(`download:${role}`); return dirs[role]; },
      waitForAnchorExit: async () => events.push('exit'),
      emitAggregate: async (aggregate) => { events.push('aggregate'); aggregates.push(aggregate); },
    },
  };
}

const root = mkdtempSync(join(tmpdir(), 'p158-live-adapter-'));
try {
  const success = fakeProvider({ root: join(root, 'success') });
  const aggregate = await runP158RetainedAnchorLiveAdapter({ config, provider: success.provider });
  assert.equal(aggregate.success, true);
  assert.equal(aggregate.bindings.workflowRunId, '33950000001');
  const { aggregateSha256, ...aggregateBody } = aggregate;
  assert.equal(aggregateSha256, digest(aggregateBody));
  assert.deepEqual(success.events, [
    'start', 'anchor:ready:cleanup', 'dispatch', 'identify', 'terminal',
    'download:human', 'download:slow', 'download:aggregate',
    'terminate:SIGTERM', 'anchor:final:cleanup', 'exit', 'aggregate',
  ]);
  for (const value of privateValues) assert.doesNotMatch(JSON.stringify(aggregate), new RegExp(value));

  const failedWorkflow = fakeProvider({ root: join(root, 'failed-workflow'), workflowConclusion: 'failure' });
  const failedAggregate = await runP158RetainedAnchorLiveAdapter({ config, provider: failedWorkflow.provider });
  assert.equal(failedAggregate.success, false);
  assert.equal(failedAggregate.checks.externalWorkflowPassed, false);
  assert(failedAggregate.failureCodes.includes('external_workflow_not_successful'));
  assert.deepEqual(failedWorkflow.events.filter((event) => event.startsWith('download:')), [
    'download:human', 'download:slow', 'download:aggregate',
  ]);
  assert(failedWorkflow.events.includes('terminate:SIGTERM'));

  const ambiguousRuns = [1, 2].map((databaseId) => ({
    databaseId, displayTitle: runName, headSha: expectedCommit, headBranch: branch,
    event: 'workflow_dispatch', createdAt: '2026-09-05T14:00:01.000Z',
  }));
  assert.throws(() => selectExactDispatchedWorkflowRun(ambiguousRuns, {
    runName, expectedCommit, branch, dispatchedAfter: '2026-09-05T14:00:00.000Z',
  }), (error) => error.code === 'workflow_run_ambiguous');
  assert.deepEqual(selectExactDispatchedWorkflowRun([{
    databaseId: 3, displayTitle: runName, headSha: expectedCommit, headBranch: branch,
    event: 'workflow_dispatch', createdAt: '2026-09-05T14:00:00.000Z',
  }], {
    runName, expectedCommit, branch, dispatchedAfter: '2026-09-05T14:00:00.900Z',
  }), { workflowRunId: '3' }, 'same-second GitHub timestamps remain eligible');
  const ambiguous = fakeProvider({
    root: join(root, 'ambiguous'),
    runSelector: () => selectExactDispatchedWorkflowRun(ambiguousRuns, {
      runName, expectedCommit, branch, dispatchedAfter: '2026-09-05T14:00:00.000Z',
    }),
  });
  const ambiguousAggregate = await runP158RetainedAnchorLiveAdapter({ config, provider: ambiguous.provider });
  assert.equal(ambiguousAggregate.success, false);
  assert.equal(ambiguous.events.some((event) => event.startsWith('download:')), false);
  assert(ambiguous.events.includes('terminate:SIGTERM'));

  for (const [label, terminalError] of [
    ['timeout', Object.assign(new Error(privateValues[0]), { code: 'observation_timeout' })],
    ['interruption', Object.assign(new Error(privateValues[1]), { code: 'observation_aborted' })],
  ]) {
    const failed = fakeProvider({ root: join(root, label), terminalError });
    const result = await runP158RetainedAnchorLiveAdapter({ config, provider: failed.provider });
    assert.equal(result.success, false);
    assert.equal(result.bindings.workflowRunId, '33950000001');
    assert.equal(result.workflowObservation.failureCode, terminalError.code);
    assert.deepEqual(failed.events.filter((event) => event.startsWith('download:')), [
      'download:human', 'download:slow', 'download:aggregate',
    ]);
    assert(failed.events.includes('terminate:SIGTERM'));
    assert(failed.events.includes('anchor:final:cleanup'));
    for (const value of privateValues) assert.doesNotMatch(JSON.stringify(result), new RegExp(value));
  }

  const missingRoot = join(root, 'missing-unit');
  mkdirSync(missingRoot, { recursive: true });
  assert.throws(() => readExactDownloadedReceipt(missingRoot, 'human'),
    (error) => error.code === 'artifact_receipt_missing');
  const duplicateRoot = join(root, 'duplicate-unit');
  mkdirSync(duplicateRoot, { recursive: true });
  writeFileSync(join(duplicateRoot, 'receipt.json'), '{}');
  writeFileSync(join(duplicateRoot, 'failure-receipt.json'), '{}');
  assert.throws(() => readExactDownloadedReceipt(duplicateRoot, 'human'),
    (error) => error.code === 'artifact_receipt_duplicate');

  const missing = fakeProvider({ root: join(root, 'missing-integration'), artifactOptions: { missing: 'slow' } });
  const missingAggregate = await runP158RetainedAnchorLiveAdapter({ config, provider: missing.provider });
  assert.equal(missingAggregate.success, false);
  assert(missing.events.includes('terminate:SIGTERM'));

  const partial = fakeProvider({ root: join(root, 'partial-download') });
  const originalDownload = partial.provider.downloadArtifact;
  partial.provider.downloadArtifact = async (args) => {
    if (args.role === 'human') throw new Error(privateValues[0]);
    return originalDownload(args);
  };
  const partialAggregate = await runP158RetainedAnchorLiveAdapter({ config, provider: partial.provider });
  assert.equal(partialAggregate.success, false);
  assert.equal(partialAggregate.bindings.workflowRunId, '33950000001');
  assert(partial.events.includes('download:slow'));
  assert(partial.events.includes('download:aggregate'));
  assert.equal(partialAggregate.workflowObservation.artifacts.human.failureCode, 'artifact_download_failed');

  const staleRoot = join(root, 'stale');
  mkdirSync(join(staleRoot, 'anchor'), { recursive: true });
  writeFileSync(join(staleRoot, 'anchor', '1-ready-receipt.json'), JSON.stringify(anchorReceipt('ready')));
  assert.throws(() => createP158GitHubLiveProvider({
    repoRoot: process.cwd(), outputRoot: staleRoot, anchorEnv: {},
  }), /empty/);

  for (const invalid of [
    { probeMode: 'other' },
    { artifactRetentionDays: 4 },
    { calibrationStartAt: 'not-a-time' },
  ]) {
    await assert.rejects(() => runP158RetainedAnchorLiveAdapter({
      config: { ...config, ...invalid }, provider: success.provider,
    }));
  }

  const forcedRoot = join(root, 'forced-exit');
  const forcedProvider = createP158GitHubLiveProvider({
    repoRoot: process.cwd(), outputRoot: forcedRoot, anchorEnv: {},
  });
  const stuckChild = new EventEmitter();
  stuckChild.exitCode = null;
  const forcedSignals = [];
  stuckChild.kill = (signal) => { forcedSignals.push(signal); return true; };
  await assert.rejects(
    () => forcedProvider.waitForAnchorExit({ child: stuckChild, timeoutMs: 5 }),
    (error) => error.code === 'anchor_exit_timeout',
  );
  assert.deepEqual(forcedSignals, ['SIGKILL']);

  const fakeBin = join(root, 'fake-gh-bin');
  mkdirSync(fakeBin);
  writeFileSync(join(fakeBin, 'gh'), '#!/bin/sh\nexit 0\n');
  chmodSync(join(fakeBin, 'gh'), 0o755);
  const priorPath = process.env.PATH;
  try {
    process.env.PATH = `${fakeBin}:${priorPath}`;
    assert.equal(await forcedProvider.downloadArtifact({
      workflowRunId: '123', artifactName: 'fixture', role: 'null-signal', signal: null,
    }), join(forcedRoot, 'downloads', 'null-signal'));
    const aborted = new AbortController();
    aborted.abort();
    await assert.rejects(() => forcedProvider.downloadArtifact({
      workflowRunId: '123', artifactName: 'fixture', role: 'aborted-signal', signal: aborted.signal,
    }), (error) => error.name === 'AbortError');
  } finally {
    process.env.PATH = priorPath;
  }

  const signaledChild = new EventEmitter();
  signaledChild.exitCode = null;
  signaledChild.signalCode = 'SIGKILL';
  await assert.rejects(() => forcedProvider.waitForAnchorExit({ child: signaledChild, timeoutMs: 1000 }),
    /abnormally/);
  await assert.rejects(() => forcedProvider.waitForAnchorReceipts({
    child: signaledChild, phase: 'ready', timeoutMs: 1000,
  }), /stopped/);

  const spawnFailureProvider = createP158GitHubLiveProvider({
    repoRoot: join(root, 'absent-repository'), outputRoot: join(root, 'spawn-failure'), anchorEnv: {},
  });
  await assert.rejects(() => spawnFailureProvider.startAnchor({ runId, anchorId }), /startup failed/);

  const earlyExitProvider = createP158GitHubLiveProvider({
    repoRoot: root, outputRoot: join(root, 'early-exit'), anchorEnv: {},
  });
  const earlyChild = await earlyExitProvider.startAnchor({ runId, anchorId });
  await new Promise((resolveExit) => earlyChild.once('exit', resolveExit));
  await assert.rejects(() => earlyExitProvider.waitForAnchorReceipts({
    child: earlyChild, phase: 'ready', timeoutMs: 1000,
  }), /stopped/);
  await assert.rejects(() => earlyExitProvider.waitForAnchorExit({ child: earlyChild, timeoutMs: 1000 }),
    /abnormally/);

  const workflowSource = readFileSync('.github/workflows/p158-external-vantage.yml', 'utf8');
  const providerSource = readFileSync('scripts/lib/p158-retained-anchor-github-provider.js', 'utf8');
  const anchorRunnerSource = readFileSync('scripts/run-p158-retained-authenticated-anchor.js', 'utf8');
  assert.match(workflowSource, /run-name: p158-\$\{\{ inputs\.campaign_run_id \}\}-\$\{\{ inputs\.expected_commit \}\}/);
  assert.match(providerSource, /cwd: repoRoot/);
  assert(anchorRunnerSource.indexOf("process.once('SIGTERM'") <
    anchorRunnerSource.indexOf("import('playwright')"),
  'anchor signal capture must be installed before Playwright is dynamically loaded');
} finally {
  rmSync(root, { recursive: true, force: true });
}

process.stdout.write('P158 retained anchor live adapter provider-free tests passed.\n');
