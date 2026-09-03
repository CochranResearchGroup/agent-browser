#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  assessP158W7A07A13Readiness,
  createP158W7A13DevelopmentDriver,
  createP158W7A07A13LiveBundle,
  createP158W7A07A13OwnershipManifest,
  enumerateP158W7A13LoggingOperations,
  p158W7A07A13SourceBinding,
} from './lib/p158-w7-a07-a13-live.js';

const registry = JSON.parse(fs.readFileSync(
  'docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-a07-a13-live' });
const runId = 'p158-a07-a13-run';
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'p158-a07-a13-'));
const candidateBinaryPath = path.join(temporary, 'agent-browser-dev');
fs.writeFileSync(candidateBinaryPath, 'p158 provider-free candidate fixture\n', { mode: 0o700 });
const socketDir = path.join(temporary, 'sockets');
fs.mkdirSync(socketDir);
const sourceDaemonIdentity = {
  pid: 8101, startToken: 'linux:test:8101', executablePath: candidateBinaryPath,
};
const retainedIdentity = {
  sourceSession: 'p158-retained-source', logicalBrowserId: 'session:p158-retained-source',
  browserPid: 7313, cdpUrlSha256: sha256('http://127.0.0.1:47313'),
  runtimeProfile: 'p158-retained-profile', activeTargetId: 'target-retained',
  tabId: 'target:target-retained', ownerGeneration: 1,
  sourceDaemonIdentitySha256: sha256(sourceDaemonIdentity),
};
retainedIdentity.tabIdentitySha256 = sha256({
  tabId: retainedIdentity.tabId, browserId: retainedIdentity.logicalBrowserId,
  targetId: retainedIdentity.activeTargetId, profileId: retainedIdentity.runtimeProfile,
  urlSha256: sha256('http://127.0.0.1:43158/retained'),
});
const ownershipManifest = createP158W7A07A13OwnershipManifest({
  schemaVersion: 'agent-browser.p158-w7-a07-a13-ownership.v1',
  campaignRunId: runId, candidateSha256: 'a'.repeat(64),
  liveHookManifestSha256: 'b'.repeat(64),
  environmentSealSha256s: { E0: 'c'.repeat(64), E1: 'd'.repeat(64) },
  environment: {
    environmentId: 'E1', runtimeLane: 'development', production: false,
    binaryPath: candidateBinaryPath, binarySha256: sha256(fs.readFileSync(candidateBinaryPath)),
    systemctlPath: '/usr/bin/systemctl', systemctlSha256: sha256(fs.readFileSync('/usr/bin/systemctl')),
    supervisorUnit: 'agent-browser-development-supervisor.service',
    runtimeHostMode: 'per_session_daemon', socketDir,
  },
  retainedIdentity,
});

function tab() {
  return {
    id: retainedIdentity.tabId, browserId: retainedIdentity.logicalBrowserId,
    targetId: retainedIdentity.activeTargetId, profileId: retainedIdentity.runtimeProfile,
    url: 'http://127.0.0.1:43158/retained', lifecycle: 'ready',
  };
}

function fakeDriver({ wrongBrowserAt = null, sameSupervisorPidAt = null,
  sameDaemonAt = null, daemonIdentityFailureAt = null, missingTabAt = null, throwAt = null } = {}) {
  let session = retainedIdentity.sourceSession;
  let generation = retainedIdentity.ownerGeneration;
  let supervisorPid = 9000;
  let daemonIdentity = sourceDaemonIdentity;
  const calls = [];
  function observe(kind, requestId) {
    calls.push({ kind, requestId });
    if (throwAt && requestId.includes(throwAt)) throw Object.assign(new Error('connection reset'), {
      code: 'ECONNRESET', operationCorrelationId: requestId,
    });
  }
  function identity(requestId, replayed) {
    return {
      resumed: true, replayed, browserPid: wrongBrowserAt && requestId.includes(wrongBrowserAt)
        ? retainedIdentity.browserPid + 1 : retainedIdentity.browserPid,
      cdpUrl: 'http://127.0.0.1:47313', runtimeProfile: retainedIdentity.runtimeProfile,
      activeTargetId: retainedIdentity.activeTargetId, targetsReattached: 1,
      transferState: 'candidate_committed',
      ownerTransferReceipt: { newOwnerGeneration: generation },
    };
  }
  return {
    calls,
    async daemonIdentity(current, requestId) {
      observe('daemon-identity', requestId);
      if (daemonIdentityFailureAt && requestId.includes(daemonIdentityFailureAt)) {
        throw Object.assign(new Error('daemon identity unavailable'), {
          code: 'a13_daemon_identity_unavailable', operationCorrelationId: requestId,
        });
      }
      assert.equal(current, session);
      return structuredClone(daemonIdentity);
    },
    async tabs(current, requestId) {
      observe('tabs', requestId);
      assert.equal(current, session);
      return { tabs: missingTabAt && requestId.includes(missingTabAt) ? [] : [tab()] };
    },
    async prepare(current, requestId) {
      observe('prepare', requestId);
      assert.equal(current, session);
      return { prepared: true, replayed: false, browserPresent: true, sessionName: session,
        browserPid: retainedIdentity.browserPid, cdpUrl: 'http://127.0.0.1:47313',
        runtimeProfile: retainedIdentity.runtimeProfile, transferState: 'awaiting_candidate',
        oldOwnerEffectCapable: true, candidateSessionName: `p158-handoff-${calls.length}`,
        previousOwnerGeneration: generation, candidateOwnerGeneration: generation + 1 };
    },
    async resume(candidate, source, logicalBrowserId, requestId) {
      observe('resume', requestId);
      assert.equal(source, session);
      assert.equal(logicalBrowserId, retainedIdentity.logicalBrowserId);
      const replayed = candidate === session;
      if (!replayed) {
        session = candidate;
        generation += 1;
        if (!sameDaemonAt || !requestId.includes(sameDaemonAt)) {
          daemonIdentity = { pid: daemonIdentity.pid + 1,
            startToken: `linux:test:${daemonIdentity.pid + 1}`, executablePath: candidateBinaryPath };
        }
      }
      return identity(requestId, replayed);
    },
    async finalize(source, requestId) {
      observe('finalize', requestId);
      return { finalized: true, browserPreserved: true, sessionName: source };
    },
    async supervisorPid(requestId) {
      observe('supervisor-pid', requestId);
      return supervisorPid;
    },
    async restartSupervisor(requestId) {
      observe('supervisor-restart', requestId);
      if (!sameSupervisorPidAt || !requestId.includes(sameSupervisorPidAt)) supervisorPid += 1;
      return { restarted: true };
    },
  };
}

function receiptStore() {
  const receipts = [];
  return {
    receipts,
    async read(attemptId) {
      const receipt = receipts.find((entry) => entry.attemptId === attemptId);
      return receipt ? structuredClone(receipt) : null;
    },
    async append(receipt) { receipts.push(structuredClone(receipt)); },
  };
}

const readiness = assessP158W7A07A13Readiness({ schedule });
assert.deepEqual(readiness.cases.A07, {
  executable: false, scheduledAttemptCount: 2, requestActionCount: 216,
  frozenBoundaryMarkerCount: 8, requiredCartesianCellCount: 864,
  blocker: {
    code: 'command_boundary_crash_matrix_unexecutable',
    sourceSymbols: [
      'scripts/lib/p158-w7-development-adapters.js::plannedActions',
      'cli/src/native/service_renderer_crash.rs::wait_for_renderer_crash',
      'cli/src/native/control_plane.rs::run_worker',
    ],
    detail: 'The frozen planner separates action ordinals from boundary markers, and the product can observe a renderer crash but cannot hold and crash an exact command at all four boundaries.',
  },
});
assert.deepEqual(readiness.cases.A13, {
  executable: true, scheduledAttemptCount: 25, daemonTransitionCount: 13,
  supervisorTransitionCount: 12, blocker: null,
});

const loggingRequests = enumerateP158W7A13LoggingOperations({ schedule, campaignRunId: runId });
assert.equal(loggingRequests.length, 211);
assert.equal(new Set(loggingRequests.map((entry) => entry.operationCorrelationId)).size, 211);
assert.equal(loggingRequests.filter((entry) => entry.operationKind === 'handoff-prepare').length, 25);
assert.equal(loggingRequests.filter((entry) => entry.operationKind === 'supervisor-restart').length, 12);
assert.ok(loggingRequests.every((entry) => entry.descriptorId === entry.operationCorrelationId &&
  entry.caseId === 'A13' && entry.phaseId === 'W7' && entry.environmentId === 'E1' &&
  entry.productRequestId === null && entry.correlationState === 'product_request_id_unavailable' &&
  entry.loggingGap.code === 'product_request_id_not_preserved'));
assert.deepEqual(enumerateP158W7A13LoggingOperations({ schedule, campaignRunId: runId }), loggingRequests);

const selectedCommands = [];
const selectedDriver = createP158W7A13DevelopmentDriver({
  manifest: ownershipManifest,
  exec: async (executable, args, options) => {
    selectedCommands.push({ executable, args: [...args],
      operationCorrelationId: options.p158OperationCorrelationId,
      productRequestEnvironment: options.env.P158_CAMPAIGN_REQUEST_ID });
    if (args.includes('--property')) return { stdout: '9123\n', stderr: '' };
    if (args.includes('restart')) return { stdout: '', stderr: '' };
    return { stdout: JSON.stringify({ success: true, data: { tabs: [tab()] } }), stderr: '' };
  },
});
assert.equal(await selectedDriver.supervisorPid(`${runId}:probe:pid`), 9123);
await selectedDriver.restartSupervisor(`${runId}:probe:restart`);
await selectedDriver.tabs(retainedIdentity.sourceSession, `${runId}:probe:tabs`);
assert.deepEqual(selectedCommands[0].args,
  ['--user', 'show', 'agent-browser-development-supervisor.service', '--property', 'MainPID', '--value']);
assert.deepEqual(selectedCommands[1].args,
  ['--user', 'restart', 'agent-browser-development-supervisor.service']);
assert.deepEqual(selectedCommands[2].args,
  ['--json', '--session', retainedIdentity.sourceSession, 'service', 'tabs']);
assert.deepEqual(selectedCommands.map((entry) => entry.operationCorrelationId),
  [`${runId}:probe:pid`, `${runId}:probe:restart`, `${runId}:probe:tabs`]);
assert.ok(selectedCommands.every((entry) => entry.productRequestEnvironment === undefined));

async function runCampaign(options = {}, select = () => true) {
  const store = receiptStore();
  const driver = fakeDriver(options);
  const before = JSON.stringify(ownershipManifest);
  const bundle = createP158W7A07A13LiveBundle({
    schedule, ownershipManifest, receiptStore: store, driver,
    clock: () => '2026-09-03T12:00:00.000Z',
  });
  const results = [];
  const a13Attempts = schedule.attempts.filter((entry) => entry.caseId === 'A13');
  const selectedOrdinals = a13Attempts.filter(select).map((entry) => entry.executionUnit.ordinal);
  const finalOrdinal = selectedOrdinals.length > 0 ? Math.max(...selectedOrdinals) : 0;
  for (const attempt of a13Attempts.filter((entry) => entry.executionUnit.ordinal <= finalOrdinal)) {
    results.push(await bundle.adapters[0].execute({ attempt }));
  }
  assert.equal(JSON.stringify(ownershipManifest), before);
  for (const receipt of store.receipts) {
    assert.deepEqual(receipt.operationCorrelationIds, bundle.loggingOperationDescriptors
      .filter((entry) => entry.attemptId === receipt.attemptId)
      .map((entry) => entry.operationCorrelationId));
    assert.deepEqual(receipt.productRequestIds, []);
    assert.equal(receipt.loggingCaptureGap, 'product_request_id_not_preserved');
    assert.equal(receipt.receiptSha256, sha256(
      Object.fromEntries(Object.entries(receipt).filter(([key]) => key !== 'receiptSha256'))));
  }
  return { bundle, driver, store, results };
}

const clean = await runCampaign();
assert.equal(clean.bundle.freezeEligible, false, 'injected effects must never qualify for live freeze');
assert.equal(clean.results.length, 25);
assert.ok(clean.results.every((entry) => entry.resultState === 'passed' && entry.actionCount === 1));
assert.equal(clean.store.receipts.length, 25);
assert.equal(clean.store.receipts.at(-1).ownerGeneration, 26);
assert.equal(clean.driver.calls.length, 211);
assert.deepEqual(clean.driver.calls.map((entry) => entry.requestId).sort(),
  loggingRequests.map((entry) => entry.operationCorrelationId).sort());
assert.equal(clean.driver.calls.filter((entry) => entry.kind === 'prepare').length, 25);
assert.equal(clean.driver.calls.filter((entry) => entry.kind === 'resume').length, 25);
assert.equal(clean.driver.calls.filter((entry) => entry.kind === 'finalize').length, 25);
assert.equal(clean.driver.calls.filter((entry) => entry.kind === 'supervisor-restart').length, 12);
assert.equal(clean.store.receipts.filter((entry) =>
  entry.scheduledStimulus === 'development_supervisor_restart' && entry.repairAttempted === false).length, 12);
assert.ok(clean.store.receipts.every((entry) => entry.browserPid === retainedIdentity.browserPid &&
  entry.logicalBrowserId === retainedIdentity.logicalBrowserId && entry.tabId === retainedIdentity.tabId &&
  entry.retryAttempted === false && entry.repairAttempted === false &&
  entry.garbageCollectionAttempted === false));

const resumedWithoutReplay = await runCampaign({}, (attempt) => attempt.attemptId === 'A13-E1-r001');
const callsBeforeResume = resumedWithoutReplay.driver.calls.length;
const firstAttempt = schedule.attempts.find((entry) => entry.attemptId === 'A13-E1-r001');
const replayedResult = await resumedWithoutReplay.bundle.adapters[0].execute({ attempt: firstAttempt });
assert.equal(replayedResult.replayedFromReceipt, true);
assert.equal(resumedWithoutReplay.driver.calls.length, callsBeforeResume);
assert.equal(resumedWithoutReplay.store.receipts.length, 1);

const defaultBundle = createP158W7A07A13LiveBundle({
  schedule, ownershipManifest, receiptStore: receiptStore(),
});
assert.equal(defaultBundle.freezeEligible, true);
assert.deepEqual(defaultBundle.concreteCaseIds, ['A13']);
assert.deepEqual(defaultBundle.loggingRequestExpectations, []);
assert.deepEqual(defaultBundle.loggingOperationDescriptors, loggingRequests);
assert.deepEqual(defaultBundle.loggingReadiness,
  { complete: false, gapCode: 'product_request_id_not_preserved' });

for (const mutate of [
  (value) => { value.environment.production = true; },
  (value) => { value.environment.runtimeHostMode = 'shared_runtime_host'; },
]) {
  const body = structuredClone(ownershipManifest);
  delete body.manifestSha256;
  mutate(body);
  const invalid = createP158W7A07A13OwnershipManifest(body);
  assert.throws(() => createP158W7A07A13LiveBundle({ schedule, ownershipManifest: invalid,
    receiptStore: receiptStore() }), (error) => error.code === 'a13_frozen_ownership_manifest_invalid');
}

const staleOwnershipBody = structuredClone(ownershipManifest);
delete staleOwnershipBody.manifestSha256;
staleOwnershipBody.retainedIdentity.sourceDaemonIdentitySha256 = '00'.repeat(32);
const staleOwnershipStore = receiptStore();
const staleOwnershipBundle = createP158W7A07A13LiveBundle({ schedule,
  ownershipManifest: createP158W7A07A13OwnershipManifest(staleOwnershipBody),
  receiptStore: staleOwnershipStore, driver: fakeDriver(), clock: () => '2026-09-03T12:00:00.000Z' });
const staleOwnershipResult = await staleOwnershipBundle.adapters[0].execute({
  attempt: schedule.attempts.find((entry) => entry.attemptId === 'A13-E1-r001'),
});
assert.equal(staleOwnershipResult.resultState, 'safety_stopped');
assert.equal(staleOwnershipStore.receipts[0].failure.code, 'a13_effect_time_ownership_mismatch');

for (const [options, attemptId, expectedState, failureCode] of [
  [{ wrongBrowserAt: 'handoff-resume' }, 'A13-E1-r001', 'reproduced_historical_failure',
    'a13_retained_identity_mismatch'],
  [{ sameSupervisorPidAt: 'supervisor-restart' }, 'A13-E1-r002', 'reproduced_historical_failure',
    'a13_supervisor_generation_not_advanced'],
  [{ sameDaemonAt: 'handoff-resume' }, 'A13-E1-r001', 'reproduced_historical_failure',
    'a13_daemon_generation_not_advanced'],
  [{ missingTabAt: 'post-tabs' }, 'A13-E1-r001', 'reproduced_historical_failure',
    'a13_tab_continuity_failed'],
  [{ throwAt: 'pre-tabs' }, 'A13-E1-r001', 'inconclusive', 'a13_command_failed'],
  [{ throwAt: 'handoff-resume' }, 'A13-E1-r001', 'reproduced_historical_failure',
    'a13_command_failed'],
  [{ daemonIdentityFailureAt: 'pre-daemon-identity' }, 'A13-E1-r001', 'inconclusive',
    'a13_daemon_identity_unavailable'],
  [{ daemonIdentityFailureAt: 'candidate-daemon-identity' }, 'A13-E1-r001',
    'reproduced_historical_failure', 'a13_daemon_identity_unavailable'],
]) {
  const run = await runCampaign(options, (attempt) => attempt.attemptId === attemptId);
  assert.equal(run.results.at(-1).resultState, expectedState);
  assert.equal(run.store.receipts.at(-1).failure.code, failureCode);
  if (failureCode === 'a13_command_failed') {
    assert.equal(run.store.receipts.at(-1).failure.transportCode, 'ECONNRESET');
    assert.ok(run.store.receipts.at(-1).failure.operationCorrelationId.startsWith(`${runId}:`));
  }
}

const source = p158W7A07A13SourceBinding();
assert.equal(source.sourceSha256, sha256(fs.readFileSync(source.sourcePath)));

process.stdout.write('P158 W7 A07/A13 boundary passed: A13=25 concrete transitions, A07=864 required crash cells blocked\n');
