import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { isAbsolute, join } from 'node:path';
import { readFileSync, realpathSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { sha256 } from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';

export const P158_W7_A07_A13_SOURCE_PATH = 'scripts/lib/p158-w7-a07-a13-live.js';
export const P158_W7_A07_A13_HOOK_ID = 'w7.a07_a13.retained_generation';
export const P158_W7_A07_A13_CONCRETE_CASE_IDS = Object.freeze(['A13']);

const BUILTIN_DRIVER = Symbol('p158-w7-a13-builtin-driver');
const SHA256 = /^[a-f0-9]{64}$/u;
const SUPERVISOR_UNIT = 'agent-browser-development-supervisor.service';

export class P158W7A07A13Error extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W7A07A13Error';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7A07A13Error(code, message, details);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function transitionOwner(attempt) {
  if (attempt.executionUnit?.dimensionAssignment?.dimensionId === 'transition_owner') {
    return attempt.executionUnit.dimensionAssignment.value;
  }
  const ordinal = attempt.executionUnit?.ordinal;
  return Number.isInteger(ordinal) && ordinal > 0
    ? ['daemon', 'supervisor'][(ordinal - 1) % 2] : null;
}

function transitionAction(attempt) {
  const actions = attempt.cardinalityAllocations?.flatMap((entry) => entry.actionIds) ?? [];
  if (actions.length !== 1) {
    fail('a13_transition_cardinality_invalid', `${attempt.attemptId} must bind exactly one transition`);
  }
  return actions[0];
}

function operationSuffixes(owner) {
  return ['pre-daemon-identity', 'pre-tabs', 'handoff-prepare',
    ...(owner === 'supervisor'
      ? ['supervisor-pid-before', 'supervisor-restart', 'supervisor-pid-after'] : []),
    'handoff-resume', 'candidate-daemon-identity', 'handoff-finalize', 'post-tabs'];
}

export function enumerateP158W7A13LoggingOperations({ schedule, campaignRunId }) {
  if (typeof campaignRunId !== 'string' || campaignRunId.length === 0) {
    fail('campaign_run_id_missing', 'A13 request enumeration requires a campaign run ID');
  }
  return freeze(schedule.attempts.filter((attempt) => attempt.caseId === 'A13').flatMap((attempt) => {
    const owner = transitionOwner(attempt);
    if (!['daemon', 'supervisor'].includes(owner)) {
      fail('a13_transition_owner_invalid', attempt.attemptId);
    }
    const actionId = transitionAction(attempt);
    return operationSuffixes(owner).map((operationKind) => {
      const operationCorrelationId = `${campaignRunId}:${actionId}:${operationKind}`;
      return {
        descriptorId: operationCorrelationId, operationCorrelationId, productRequestId: null,
        correlationState: 'product_request_id_unavailable', operationKind,
        actionId, attemptId: attempt.attemptId, caseId: 'A13', phaseId: 'W7',
        environmentId: attempt.environmentIds[0],
        loggingGap: {
          code: 'product_request_id_not_preserved',
          detail: 'The handoff, tabs, and systemctl surfaces do not accept or return a caller request ID.',
        },
      };
    });
  }));
}

// Compatibility export. These are operation correlations, not product request IDs.
export const enumerateP158W7A13LoggingRequests = enumerateP158W7A13LoggingOperations;

export function assessP158W7A07A13Readiness({ schedule }) {
  const a07Attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'A07');
  const requestActionCount = a07Attempts.reduce((count, attempt) => count +
    (attempt.cardinalityAllocations?.find((entry) =>
      entry.id === 'supported_service_request_actions')?.actionIds.length ?? 0), 0);
  const boundaryCount = 4;
  const requiredCartesianCellCount = requestActionCount * boundaryCount;
  const frozenBoundaryMarkerCount = a07Attempts.length * boundaryCount;
  const a13Attempts = schedule.attempts.filter((attempt) => attempt.caseId === 'A13');
  const daemon = a13Attempts.filter((attempt) => transitionOwner(attempt) === 'daemon').length;
  const supervisor = a13Attempts.filter((attempt) => transitionOwner(attempt) === 'supervisor').length;
  return freeze({
    schemaVersion: 'agent-browser.p158-w7-a07-a13-readiness.v1',
    cases: {
      A07: {
        executable: false, scheduledAttemptCount: a07Attempts.length, requestActionCount,
        frozenBoundaryMarkerCount, requiredCartesianCellCount,
        blocker: {
          code: 'command_boundary_crash_matrix_unexecutable',
          sourceSymbols: [
            'scripts/lib/p158-w7-development-adapters.js::plannedActions',
            'cli/src/native/service_renderer_crash.rs::wait_for_renderer_crash',
            'cli/src/native/control_plane.rs::run_worker',
          ],
          detail: 'The frozen planner separates action ordinals from boundary markers, and the product can observe a renderer crash but cannot hold and crash an exact command at all four boundaries.',
        },
      },
      A13: {
        executable: a13Attempts.length === 25 && daemon === 13 && supervisor === 12,
        scheduledAttemptCount: a13Attempts.length, daemonTransitionCount: daemon,
        supervisorTransitionCount: supervisor,
        blocker: null,
      },
    },
    effectsAttempted: false,
  });
}

function validateManifest(input, schedule) {
  const body = input && typeof input === 'object'
    ? Object.fromEntries(Object.entries(input).filter(([key]) => key !== 'manifestSha256')) : null;
  const environment = input?.environment;
  const identity = input?.retainedIdentity;
  if (input?.schemaVersion !== 'agent-browser.p158-w7-a07-a13-ownership.v1' ||
      input.manifestSha256 !== sha256(body) || typeof input.campaignRunId !== 'string' ||
      !SHA256.test(input.candidateSha256 ?? '') || !SHA256.test(input.liveHookManifestSha256 ?? '') ||
      ['E0', 'E1'].some((id) => !SHA256.test(input.environmentSealSha256s?.[id] ?? '')) ||
      environment?.environmentId !== 'E1' || environment.runtimeLane !== 'development' ||
      environment.production !== false || !isAbsolute(environment.binaryPath ?? '') ||
      !environment.binaryPath.endsWith('/agent-browser-dev') ||
      !SHA256.test(environment.binarySha256 ?? '') ||
      environment.systemctlPath !== '/usr/bin/systemctl' ||
      !SHA256.test(environment.systemctlSha256 ?? '') || environment.supervisorUnit !== SUPERVISOR_UNIT ||
      environment.runtimeHostMode !== 'per_session_daemon' || !isAbsolute(environment.socketDir ?? '') ||
      typeof identity?.sourceSession !== 'string' || !identity.sourceSession.startsWith('p158-') ||
      typeof identity?.logicalBrowserId !== 'string' || !identity.logicalBrowserId.startsWith('session:') ||
      !Number.isInteger(identity?.browserPid) || identity.browserPid <= 1 ||
      !SHA256.test(identity?.cdpUrlSha256 ?? '') || typeof identity?.runtimeProfile !== 'string' ||
      typeof identity?.activeTargetId !== 'string' || typeof identity?.tabId !== 'string' ||
      !SHA256.test(identity?.tabIdentitySha256 ?? '') ||
      !SHA256.test(identity?.sourceDaemonIdentitySha256 ?? '') ||
      schedule.attempts.filter((attempt) => attempt.caseId === 'A13').length !== 25) {
    fail('a13_frozen_ownership_manifest_invalid', 'A13 requires exact development ownership and retained identity');
  }
  assertExecutable({ path: environment.binaryPath, sha256: environment.binarySha256 });
  assertExecutable({ path: environment.systemctlPath, sha256: environment.systemctlSha256 });
  return freeze(structuredClone(input));
}

function commandData(value) {
  return value?.data ?? value;
}

function execFilePromise(executable, args, options) {
  return new Promise((resolve, reject) => execFile(executable, args, options, (error, stdout, stderr) => {
    if (error) return reject(Object.assign(error, { stdout, stderr }));
    resolve({ stdout, stderr });
  }));
}

function assertExecutable(binding) {
  let digest;
  try { digest = sha256(readFileSync(binding.path)); } catch (error) {
    fail('a13_executable_unreadable', binding.path, { cause: error.message });
  }
  if (digest !== binding.sha256) fail('a13_executable_identity_mismatch', binding.path);
}

export function createP158W7A13DevelopmentDriver({ manifest, exec = execFilePromise } = {}) {
  const injected = exec !== execFilePromise;
  const environment = manifest.environment;
  async function run(executable, args, operationCorrelationId) {
    assertExecutable(executable);
    let output;
    try {
      output = await exec(executable.path, args, { env: { ...process.env },
        maxBuffer: 4 * 1024 * 1024, p158OperationCorrelationId: operationCorrelationId });
    } catch (error) {
      fail('a13_command_failed', `${executable.path} ${args.join(' ')}`, {
        cause: error.message, transportCode: error.code ?? null, operationCorrelationId,
      });
    }
    return output;
  }
  async function cli(session, args, requestId) {
    const output = await run({ path: environment.binaryPath, sha256: environment.binarySha256 },
      ['--json', '--session', session, ...args], requestId);
    try { return commandData(JSON.parse(output.stdout)); } catch {
      fail('a13_command_response_invalid', requestId);
    }
  }
  const driver = {
    async daemonIdentity(session, requestId) {
      if (!/^[a-zA-Z0-9_-]+$/u.test(session)) fail('a13_daemon_session_invalid', session);
      let identity;
      try {
        identity = JSON.parse(readFileSync(join(environment.socketDir, `${session}.identity.json`), 'utf8'));
      } catch (error) {
        fail('a13_daemon_identity_unavailable', requestId, { cause: error.message });
      }
      if (!Number.isInteger(identity?.pid) || identity.pid <= 1 ||
          typeof identity.startToken !== 'string' || identity.executablePath !== environment.binaryPath) {
        fail('a13_daemon_identity_invalid', requestId, identity);
      }
      let observedStartToken;
      let observedExecutable;
      try {
        const bootId = readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim();
        const stat = readFileSync(`/proc/${identity.pid}/stat`, 'utf8');
        const fields = stat.slice(stat.lastIndexOf(')') + 2).trim().split(/\s+/u);
        observedStartToken = `linux:${bootId}:${fields[19]}`;
        observedExecutable = realpathSync(`/proc/${identity.pid}/exe`);
      } catch (error) {
        fail('a13_daemon_process_unavailable', requestId, { cause: error.message });
      }
      if (observedStartToken !== identity.startToken ||
          observedExecutable !== realpathSync(environment.binaryPath)) {
        fail('a13_daemon_process_identity_mismatch', requestId, identity);
      }
      return freeze(structuredClone(identity));
    },
    tabs: (session, requestId) => cli(session, ['service', 'tabs'], requestId),
    prepare: (session, requestId) => cli(session, ['handoff', 'prepare'], requestId),
    resume: (session, sourceSession, logicalBrowserId, requestId) => cli(session,
      ['handoff', 'resume', '--source-session', sourceSession, '--logical-browser-id', logicalBrowserId], requestId),
    finalize: (session, requestId) => cli(session, ['handoff', 'finalize'], requestId),
    async supervisorPid(requestId) {
      const output = await run({ path: environment.systemctlPath, sha256: environment.systemctlSha256 },
        ['--user', 'show', environment.supervisorUnit, '--property', 'MainPID', '--value'], requestId);
      const pid = Number.parseInt(output.stdout.trim(), 10);
      if (!Number.isInteger(pid) || pid <= 1) fail('a13_supervisor_identity_invalid', requestId);
      return pid;
    },
    async restartSupervisor(requestId) {
      await run({ path: environment.systemctlPath, sha256: environment.systemctlSha256 },
        ['--user', 'restart', environment.supervisorUnit], requestId);
      return { restarted: true };
    },
  };
  if (!injected) Object.defineProperty(driver, BUILTIN_DRIVER, { value: true });
  return freeze(driver);
}

function selectTab(tabsResponse, expected) {
  const rows = tabsResponse?.tabs ?? tabsResponse?.data?.tabs ?? [];
  const matches = rows.filter((tab) => tab.id === expected.tabId &&
    tab.browserId === expected.logicalBrowserId && tab.targetId === expected.activeTargetId);
  if (matches.length !== 1 || sha256({ tabId: matches[0].id, browserId: matches[0].browserId,
    targetId: matches[0].targetId, profileId: matches[0].profileId,
    urlSha256: sha256(matches[0].url ?? '') }) !== expected.tabIdentitySha256) {
    fail('a13_tab_continuity_failed', expected.tabId, rows);
  }
  return matches[0];
}

function assertHandoffIdentity(response, expected, { replayed = null } = {}) {
  if (response?.resumed !== true || response.browserPid !== expected.browserPid ||
      sha256(response.cdpUrl ?? '') !== expected.cdpUrlSha256 ||
      response.runtimeProfile !== expected.runtimeProfile || response.activeTargetId !== expected.activeTargetId ||
      !Number.isInteger(response.targetsReattached) || response.targetsReattached < 1 ||
      (replayed !== null && response.replayed !== replayed)) {
    fail('a13_retained_identity_mismatch', 'Handoff response changed the retained browser identity', response);
  }
}

function validateStoredReceipt(receipt, manifest) {
  const { receiptSha256, ...body } = receipt;
  if (receiptSha256 !== sha256(body)) fail('a13_receipt_integrity_mismatch', receipt.actionId);
  if (receipt.campaignRunId !== manifest.campaignRunId) fail('a13_receipt_run_mismatch', receipt.actionId);
}

async function priorState(receiptStore, manifest, schedule, attemptId) {
  let state = { session: manifest.retainedIdentity.sourceSession,
    ownerGeneration: manifest.retainedIdentity.ownerGeneration ?? 1,
    daemonIdentitySha256: manifest.retainedIdentity.sourceDaemonIdentitySha256 };
  const earlierAttemptIds = schedule.attempts.filter((entry) => entry.caseId === 'A13' &&
    entry.attemptId.localeCompare(attemptId) < 0).map((entry) => entry.attemptId).sort();
  for (const earlierAttemptId of earlierAttemptIds) {
    const receipt = await receiptStore.read(earlierAttemptId);
    if (receipt === null || receipt === undefined) {
      fail('a13_prior_transition_receipt_missing', earlierAttemptId);
    }
    validateStoredReceipt(receipt, manifest);
    if (receipt.state !== 'passed') fail('a13_prior_transition_failed', receipt.actionId);
    state = { session: receipt.newSession, ownerGeneration: receipt.ownerGeneration,
      daemonIdentitySha256: receipt.newDaemonIdentitySha256 };
  }
  return state;
}

async function runTransition({ manifest, schedule, attempt, driver, receiptStore, clock }) {
  const owner = transitionOwner(attempt);
  const actionId = transitionAction(attempt);
  const expectations = enumerateP158W7A13LoggingOperations({
    schedule: { attempts: [attempt] }, campaignRunId: manifest.campaignRunId,
  });
  const operationCorrelationIds = expectations.map((entry) => entry.operationCorrelationId);
  const correlation = (suffix) => `${manifest.campaignRunId}:${actionId}:${suffix}`;
  const retained = manifest.retainedIdentity;
  const existing = await receiptStore.read(attempt.attemptId);
  if (existing !== null && existing !== undefined) {
    validateStoredReceipt(existing, manifest);
    const receipt = freeze(structuredClone(existing));
    return freeze({ resultState: receipt.resultState, actionCount: 1, actionIds: [actionId],
      receipts: [receipt], artifactIds: [`p158-w7-a13:${receipt.receiptSha256}`],
      effectState: receipt.effectState, retryDisposition: receipt.retryDisposition,
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
      replayedFromReceipt: true });
  }
  let previous = { session: retained.sourceSession,
    ownerGeneration: retained.ownerGeneration ?? 1,
    daemonIdentitySha256: retained.sourceDaemonIdentitySha256 };
  let nextSession = previous.session;
  let nextGeneration = previous.ownerGeneration;
  let effectObserved = false;
  try {
    previous = await priorState(receiptStore, manifest, schedule, attempt.attemptId);
    const oldDaemonIdentity = await driver.daemonIdentity(previous.session,
      correlation('pre-daemon-identity'));
    const oldDaemonIdentitySha256 = sha256(oldDaemonIdentity);
    if (oldDaemonIdentitySha256 !== previous.daemonIdentitySha256) {
      fail('a13_effect_time_ownership_mismatch', actionId);
    }
    selectTab(await driver.tabs(previous.session, correlation('pre-tabs')), retained);
    const prepared = await driver.prepare(previous.session, correlation('handoff-prepare'));
    if (prepared?.prepared !== true || prepared.browserPresent !== true ||
        prepared.sessionName !== previous.session || prepared.browserPid !== retained.browserPid ||
        sha256(prepared.cdpUrl ?? '') !== retained.cdpUrlSha256 ||
        prepared.runtimeProfile !== retained.runtimeProfile || prepared.transferState !== 'awaiting_candidate' ||
        prepared.oldOwnerEffectCapable !== true || typeof prepared.candidateSessionName !== 'string' ||
        prepared.candidateSessionName === previous.session) {
      fail('a13_prepare_oracle_failed', actionId, prepared);
    }
    effectObserved = true;
    nextSession = prepared.candidateSessionName;
    let oldSupervisorPid = null;
    let newSupervisorPid = null;
    if (owner === 'supervisor') {
      oldSupervisorPid = await driver.supervisorPid(correlation('supervisor-pid-before'));
      await driver.restartSupervisor(correlation('supervisor-restart'));
      newSupervisorPid = await driver.supervisorPid(correlation('supervisor-pid-after'));
      if (newSupervisorPid === oldSupervisorPid) fail('a13_supervisor_generation_not_advanced', actionId);
    }
    const resumed = await driver.resume(nextSession, previous.session, retained.logicalBrowserId,
      correlation('handoff-resume'));
    assertHandoffIdentity(resumed, retained, { replayed: false });
    nextGeneration = resumed.ownerTransferReceipt?.newOwnerGeneration;
    if (!Number.isInteger(nextGeneration) || nextGeneration <= previous.ownerGeneration) {
      fail('a13_owner_generation_not_advanced', actionId, resumed);
    }
    const newDaemonIdentity = await driver.daemonIdentity(nextSession,
      correlation('candidate-daemon-identity'));
    const newDaemonIdentitySha256 = sha256(newDaemonIdentity);
    if (newDaemonIdentity.pid === oldDaemonIdentity.pid ||
        newDaemonIdentity.startToken === oldDaemonIdentity.startToken) {
      fail('a13_daemon_generation_not_advanced', actionId);
    }
    const finalized = await driver.finalize(previous.session, correlation('handoff-finalize'));
    if (finalized?.finalized !== true || finalized.browserPreserved !== true ||
        finalized.sessionName !== previous.session) fail('a13_finalize_oracle_failed', actionId, finalized);
    selectTab(await driver.tabs(nextSession, correlation('post-tabs')), retained);
    const transitionEvidence = { oldSession: previous.session, newSession: nextSession,
      oldOwnerGeneration: previous.ownerGeneration, newOwnerGeneration: nextGeneration,
      oldDaemonIdentitySha256, newDaemonIdentitySha256, oldSupervisorPid, newSupervisorPid };
    const receipt = {
      schemaVersion: 'agent-browser.p158-w7-a13-transition-receipt.v1',
      campaignRunId: manifest.campaignRunId, caseId: 'A13', attemptId: attempt.attemptId,
      actionId, environmentId: 'E1', transitionOwner: owner, operationCorrelationIds,
      productRequestIds: [], loggingCaptureGap: 'product_request_id_not_preserved',
      scheduledStimulus: owner === 'supervisor' ? 'development_supervisor_restart' : 'daemon_generation_transfer',
      oldSession: previous.session, newSession: nextSession, ownerGeneration: nextGeneration,
      oldDaemonIdentitySha256, newDaemonIdentitySha256,
      browserPid: retained.browserPid, cdpUrlSha256: retained.cdpUrlSha256,
      logicalBrowserId: retained.logicalBrowserId, tabId: retained.tabId,
      transitionEvidence, state: 'passed', resultState: 'passed', effectState: 'verified_effect',
      observedAt: clock(), retryDisposition: 'prohibited_opportunistic_retry',
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    };
    receipt.receiptSha256 = sha256(receipt);
    await receiptStore.append(freeze(structuredClone(receipt)));
    return freeze({ resultState: 'passed', actionCount: 1, actionIds: [actionId], receipts: [receipt],
      artifactIds: [`p158-w7-a13:${receipt.receiptSha256}`], effectState: 'verified_effect',
      retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
      repairAttempted: false, garbageCollectionAttempted: false });
  } catch (error) {
    const productCodes = new Set(['a13_prepare_oracle_failed', 'a13_retained_identity_mismatch',
      'a13_owner_generation_not_advanced', 'a13_finalize_oracle_failed',
      'a13_supervisor_generation_not_advanced', 'a13_tab_continuity_failed',
      'a13_daemon_generation_not_advanced']);
    const directTransportFailure = ['ECONNRESET', 'ECONNREFUSED', 'EPIPE', 'ETIMEDOUT']
      .includes(error?.code);
    const commandFailure = error?.code === 'a13_command_failed' || directTransportFailure;
    const daemonProofFailure = ['a13_daemon_identity_unavailable', 'a13_daemon_process_unavailable',
      'a13_daemon_process_identity_mismatch', 'a13_daemon_identity_invalid'].includes(error?.code);
    const resultState = daemonProofFailure
      ? (effectObserved ? 'reproduced_historical_failure' :
        (error?.code === 'a13_daemon_identity_unavailable' ? 'inconclusive' : 'safety_stopped'))
      : (['a13_effect_time_ownership_mismatch', 'a13_prior_transition_receipt_missing',
      'a13_prior_transition_failed'].includes(error?.code) ? 'safety_stopped'
      : (commandFailure
        ? (effectObserved ? 'reproduced_historical_failure' : 'inconclusive')
        : (error?.code === 'a13_command_response_invalid' && effectObserved
          ? 'new_product_failure'
          : (productCodes.has(error?.code) ? 'reproduced_historical_failure' : 'harness_failure'))));
    const receipt = {
      schemaVersion: 'agent-browser.p158-w7-a13-transition-receipt.v1',
      campaignRunId: manifest.campaignRunId, caseId: 'A13', attemptId: attempt.attemptId,
      actionId, environmentId: 'E1', transitionOwner: owner, operationCorrelationIds,
      productRequestIds: [], loggingCaptureGap: 'product_request_id_not_preserved',
      scheduledStimulus: owner === 'supervisor' ? 'development_supervisor_restart' : 'daemon_generation_transfer',
      oldSession: previous.session, newSession: nextSession, ownerGeneration: nextGeneration,
      browserPid: retained.browserPid, cdpUrlSha256: retained.cdpUrlSha256,
      logicalBrowserId: retained.logicalBrowserId, tabId: retained.tabId,
      state: 'failed', resultState, effectState: effectObserved ? 'effect_uncertain' : 'no_effect',
      failure: { code: directTransportFailure ? 'a13_command_failed' :
        (error?.code ?? 'a13_unknown_failure'), message: error?.message ?? String(error),
        operationCorrelationId: error?.details?.operationCorrelationId ??
          error?.operationCorrelationId ?? null,
        transportCode: directTransportFailure ? error.code : (error?.details?.transportCode ?? null) },
      observedAt: clock(), retryDisposition: 'prohibited_opportunistic_retry',
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    };
    receipt.receiptSha256 = sha256(receipt);
    await receiptStore.append(freeze(structuredClone(receipt)));
    return freeze({ resultState, actionCount: 1, actionIds: [actionId], receipts: [receipt],
      artifactIds: [`p158-w7-a13:${receipt.receiptSha256}`], effectState: receipt.effectState,
      retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
      repairAttempted: false, garbageCollectionAttempted: false });
  }
}

export function createP158W7A07A13LiveBundle({ schedule, ownershipManifest, receiptStore,
  driver = null, clock = () => new Date().toISOString() }) {
  const manifest = validateManifest(ownershipManifest, schedule);
  if (typeof receiptStore?.append !== 'function' || typeof receiptStore?.read !== 'function') {
    fail('a13_receipt_store_invalid', 'A13 requires deterministic read and append for no-replay resumption');
  }
  const selectedDriver = driver ?? createP158W7A13DevelopmentDriver({ manifest });
  const readiness = assessP158W7A07A13Readiness({ schedule });
  if (!readiness.cases.A13.executable) fail('a13_schedule_invalid', 'A13 requires 25 exact transitions');
  const contract = schedule.caseContracts.find((entry) => entry.caseId === 'A13');
  const adapter = createP158CaseAdapter({
    caseId: 'A13', evidenceProfile: contract.evidenceProfile,
    executionContract: contract.executionContract,
    execute: ({ attempt }) => runTransition({ manifest, schedule, attempt, driver: selectedDriver,
      receiptStore, clock }),
  });
  const source = freeze({ sourcePath: P158_W7_A07_A13_SOURCE_PATH, sourceSha256: sourceSha256() });
  const loggingOperationDescriptors = enumerateP158W7A13LoggingOperations({
    schedule, campaignRunId: manifest.campaignRunId,
  });
  return freeze({
    schemaVersion: 'agent-browser.p158-w7-a07-a13-live-bundle.v1',
    freezeEligible: selectedDriver[BUILTIN_DRIVER] === true, providerFree: false,
    concreteCaseIds: ['A13'], adapters: [adapter], readiness,
    ownershipManifestSha256: manifest.manifestSha256, campaignRunId: manifest.campaignRunId,
    candidateSha256: manifest.candidateSha256,
    liveHookManifestSha256: manifest.liveHookManifestSha256,
    environmentSealSha256s: structuredClone(manifest.environmentSealSha256s),
    liveHookIds: [P158_W7_A07_A13_HOOK_ID], driverSource: source,
    loggingRequestExpectations: [], loggingOperationDescriptors,
    loggingReadiness: { complete: false, gapCode: 'product_request_id_not_preserved' },
    adapterBindingSha256: sha256({ caseIds: ['A13'], ownershipManifestSha256: manifest.manifestSha256,
      campaignRunId: manifest.campaignRunId, candidateSha256: manifest.candidateSha256,
      liveHookManifestSha256: manifest.liveHookManifestSha256,
      environmentSealSha256s: manifest.environmentSealSha256s, source,
      liveHookIds: [P158_W7_A07_A13_HOOK_ID] }),
  });
}

export function createP158W7A07A13OwnershipManifest(input) {
  const body = structuredClone(input);
  return freeze({ ...body, manifestSha256: sha256(body) });
}

export function p158W7A07A13SourceBinding() {
  return freeze({ hookId: P158_W7_A07_A13_HOOK_ID,
    sourcePath: P158_W7_A07_A13_SOURCE_PATH, sourceSha256: sourceSha256() });
}
