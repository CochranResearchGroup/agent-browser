import { execFile as nodeExecFile, spawn as nodeSpawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { sha256 } from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';

export const P158_W7_A08_SOURCE_PATH = 'scripts/lib/p158-w7-a08-live.js';
export const P158_W7_A08_FIXTURE_PATH =
  'docs/dev/fixtures/p158-a08-profile-identity-replay.v1.json';
export const P158_W7_A08_HOOK_ID = 'w7.a08.profile_identity_fixture_replay';

const execFile = promisify(nodeExecFile);
const BUILTIN_DRIVER = Symbol('p158-w7-a08-builtin-driver');
const SHA256 = /^[a-f0-9]{64}$/u;
const ACTIONS = Object.freeze(['launch', 'remote_view_open', 'tab_switch', 'view_focus']);
const IDENTITY_STATES = Object.freeze(['unproven', 'inconsistent']);
const FAILURE_BY_STATE = Object.freeze({
  unproven: 'existing_session_profile_identity_unproven',
  inconsistent: 'existing_session_profile_identity_inconsistent',
});

export class P158W7A08Error extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158W7A08Error';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7A08Error(code, message, details);
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function sourceSha256() {
  return createHash('sha256').update(
    // This is a source identity, not a claim about the installed candidate.
    // eslint-disable-next-line no-sync
    requireSourceBytes(fileURLToPath(import.meta.url)),
  ).digest('hex');
}

function requireSourceBytes(path) {
  // Keep source binding synchronous so bundle construction cannot race a source edit.
  return readFileSync(path);
}

function fixtureSourcePath() {
  return resolve(fileURLToPath(new URL('../..', import.meta.url)), P158_W7_A08_FIXTURE_PATH);
}

function fixtureSha256() {
  return sha256(requireSourceBytes(fixtureSourcePath()));
}

function assertExecutable(candidate) {
  if (!isAbsolute(candidate?.binaryPath ?? '') || !SHA256.test(candidate?.binarySha256 ?? '')) {
    fail('a08_candidate_identity_invalid', 'A08 requires an absolute, SHA-bound candidate');
  }
  let observed;
  try { observed = sha256(requireSourceBytes(candidate.binaryPath)); } catch (error) {
    fail('a08_candidate_unreadable', 'Frozen candidate cannot be read', { cause: error.message });
  }
  if (observed !== candidate.binarySha256) {
    fail('a08_candidate_identity_drift', 'Frozen candidate executable digest changed');
  }
}

function assertIsolatedEnvironment(environment) {
  const childPaths = [environment?.home, environment?.agentHome,
    environment?.xdgRuntimeDir, environment?.socketDir];
  if (environment?.environmentId !== 'E1' || environment.runtimeLane !== 'development' ||
      environment.production !== false || environment.tenantDataPresent !== false ||
      !isAbsolute(environment?.root ?? '') || childPaths.some((path) => !isAbsolute(path ?? '')) ||
      childPaths.some((path) => !resolve(path).startsWith(`${resolve(environment.root)}/`)) ||
      resolve(environment.root) === resolve(process.env.HOME ?? '/') ||
      resolve(environment.agentHome) === resolve(join(process.env.HOME ?? '/', '.agent-browser'))) {
    fail('a08_development_isolation_unproven',
      'A08 requires a non-production E1 root with isolated HOME, agent home, runtime, and socket paths');
  }
}

function buildState({ identityState, profilePath }) {
  const profileId = 'p158-a08-synthetic-profile';
  const otherProfileId = 'p158-a08-synthetic-other-profile';
  const sessionId = 'p158-a08-retained-session';
  const browserId = `session:${sessionId}`;
  const profiles = {
    [profileId]: { id: profileId, name: 'Synthetic retained profile', userDataDir: profilePath,
      persistent: true },
    [otherProfileId]: { id: otherProfileId, name: 'Synthetic mismatched profile',
      userDataDir: `${profilePath}-other`, persistent: true },
  };
  const state = {
    profiles,
    sessions: { [sessionId]: { id: sessionId, profileId, browserIds: [browserId],
      serviceName: 'P158SyntheticService', agentName: 'p158-a08', taskName: 'identityReplay' } },
    browsers: { [browserId]: { id: browserId,
      profileId: identityState === 'inconsistent' ? otherProfileId : profileId,
      activeSessionIds: [sessionId] } },
  };
  if (identityState === 'inconsistent') {
    const digest = '1'.repeat(64);
    state.runtimeOwnerRegistry = {
      revision: 1,
      owners: { [digest]: { ownerId: 'p158-a08-synthetic-owner', profileIdentityDigest: digest,
        state: 'ready', ownerGeneration: 1, browserId, daemonSessionRoute: sessionId,
        processInstanceDigest: '2'.repeat(64), browserFamily: 'chrome',
        cdpEndpointIdentityDigest: '3'.repeat(64), targetSetDigest: '4'.repeat(64) } },
    };
  }
  return state;
}

function cellId(identityState, action) {
  return `A08-E1-${identityState}-${action}`;
}

export function enumerateP158W7A08LoggingOperations({ campaignRunId }) {
  if (typeof campaignRunId !== 'string' || campaignRunId.length === 0) {
    fail('campaign_run_id_missing', 'A08 logging enumeration requires a campaign run ID');
  }
  return freeze(IDENTITY_STATES.flatMap((identityState) => ACTIONS.flatMap((action) =>
    ['materialize_identity_fixture', action, 'assert_identity_result'].map((operationKind) => {
      const actionId = cellId(identityState, action);
      const operationCorrelationId = `${campaignRunId}:${actionId}:${operationKind}`;
      const productSurface = operationKind === action;
      return {
        descriptorId: operationCorrelationId, operationCorrelationId,
        productRequestId: null,
        correlationState: productSurface
          ? 'product_request_id_observed_only_if_product_returns_one'
          : 'product_request_id_not_applicable_to_harness_operation',
        operationKind, actionId, attemptId: 'A08-E1-r001', caseId: 'A08', phaseId: 'W7',
        environmentId: 'E1',
        loggingGap: productSurface ? {
          code: 'caller_product_request_id_not_supported_by_selected_command_surface',
          detail: 'The frozen CLI and MCP adapters generate any product request ID internally.',
        } : null,
      };
    }))));
}

export async function prepareP158W7A08ReplayManifest({ campaignRunId, candidate, environment,
  scheduleSha256, liveHookManifestSha256, environmentSealSha256s, run = execFile } = {}) {
  assertExecutable(candidate);
  assertIsolatedEnvironment(environment);
  if (!SHA256.test(scheduleSha256 ?? '') || !SHA256.test(liveHookManifestSha256 ?? '') ||
      !SHA256.test(environmentSealSha256s?.E1 ?? '') || typeof run !== 'function') {
    fail('a08_preparation_binding_invalid', 'A08 preparation is missing frozen source bindings');
  }
  const cells = [];
  for (const identityState of IDENTITY_STATES) {
    for (const action of ACTIONS) {
      const id = cellId(identityState, action);
      const root = join(environment.root, id);
      const home = join(root, 'home');
      const agentHome = join(root, 'agent-home');
      const socketDir = join(root, 'socket');
      const xdgRuntimeDir = join(root, 'xdg-runtime');
      const profilePath = join(root, 'profile', 'user-data');
      const statePath = join(agentHome, 'service', 'state.json');
      await Promise.all([mkdir(join(agentHome, 'service'), { recursive: true, mode: 0o700 }),
        mkdir(socketDir, { recursive: true, mode: 0o700 }),
        mkdir(xdgRuntimeDir, { recursive: true, mode: 0o700 }),
        mkdir(profilePath, { recursive: true, mode: 0o700 }),
        mkdir(`${profilePath}-other`, { recursive: true, mode: 0o700 })]);
      const stateBytes = `${JSON.stringify(buildState({ identityState, profilePath }), null, 2)}\n`;
      await writeFile(statePath, stateBytes, { flag: 'wx', mode: 0o600 });
      const validation = await run(candidate.binaryPath,
        ['--json', 'service', 'state', 'validate', '--path', statePath], {
          env: { ...process.env, HOME: home, AGENT_BROWSER_HOME: agentHome,
            XDG_RUNTIME_DIR: xdgRuntimeDir, AGENT_BROWSER_SOCKET_DIR: socketDir,
            AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development' }, maxBuffer: 4 * 1024 * 1024,
        });
      let envelope;
      try { envelope = JSON.parse(validation.stdout); } catch {
        fail('a08_parser_receipt_invalid', `${id} validator did not return JSON`);
      }
      const receipt = envelope.data ?? envelope;
      const stateSha256 = sha256(stateBytes);
      if (receipt.accepted !== true || receipt.classification !== 'accepted' ||
          receipt.stateSha256 !== stateSha256 ||
          receipt.parserIdentitySha256 !== candidate.binarySha256) {
        fail('a08_fixture_parser_rejected', `${id} was not accepted by the frozen candidate`, receipt);
      }
      cells.push({ cellId: id, identityState, action, expectedFailure: FAILURE_BY_STATE[identityState],
        rootSha256: sha256(root), statePathSha256: sha256(statePath), stateSha256,
        parserReceiptSha256: sha256(receipt), home, agentHome, socketDir, xdgRuntimeDir,
        sessionId: 'p158-a08-retained-session' });
    }
  }
  const body = { schemaVersion: 'agent-browser.p158-w7-a08-replay-manifest.v1', campaignRunId,
    scheduleSha256, liveHookManifestSha256, environmentSealSha256s: { E1: environmentSealSha256s.E1 },
    candidate: structuredClone(candidate), environment: { ...structuredClone(environment),
      rootSha256: sha256(environment.root) }, fixtureSourceSha256: fixtureSha256(), cells };
  return freeze({ ...body, manifestSha256: sha256(body) });
}

function validateManifest(manifest, schedule) {
  const { manifestSha256, ...body } = manifest ?? {};
  assertExecutable(manifest?.candidate);
  assertIsolatedEnvironment(manifest?.environment);
  const expectedCells = new Set(IDENTITY_STATES.flatMap((state) =>
    ACTIONS.map((action) => cellId(state, action))));
  if (manifest?.schemaVersion !== 'agent-browser.p158-w7-a08-replay-manifest.v1' ||
      manifestSha256 !== sha256(body) || manifest.scheduleSha256 !== schedule.scheduleSha256 ||
      manifest.fixtureSourceSha256 !== fixtureSha256() ||
      !SHA256.test(manifest.liveHookManifestSha256 ?? '') ||
      !SHA256.test(manifest.environmentSealSha256s?.E1 ?? '') ||
      !Array.isArray(manifest.cells) || manifest.cells.length !== 8 ||
      new Set(manifest.cells.map((cell) => cell.cellId)).size !== 8 ||
      manifest.cells.some((cell) => !expectedCells.delete(cell.cellId) ||
        cell.expectedFailure !== FAILURE_BY_STATE[cell.identityState] ||
        !SHA256.test(cell.stateSha256 ?? '') || !SHA256.test(cell.parserReceiptSha256 ?? '') ||
        ![cell.home, cell.agentHome, cell.socketDir, cell.xdgRuntimeDir]
          .every((path) => isAbsolute(path ?? '') && resolve(path).startsWith(`${resolve(cell.root ??
            join(manifest.environment.root, cell.cellId))}/`)))) {
    fail('a08_frozen_replay_manifest_invalid', 'A08 replay manifest is incomplete or drifted');
  }
  return freeze(structuredClone(manifest));
}

function redact(value) {
  if (Array.isArray(value)) return value.map(redact);
  if (typeof value === 'string') return `sha256:${sha256(value)}`;
  if (!value || typeof value !== 'object') return value;
  const result = {};
  for (const [key, child] of Object.entries(value)) {
    if (/url|path|endpoint|userdatadir/iu.test(key) || /id$/iu.test(key)) {
      result[`${key}Sha256`] = sha256(String(child));
    }
    else result[key] = redact(child);
  }
  return result;
}

function extractFailure(value) {
  const text = JSON.stringify(value);
  return Object.values(FAILURE_BY_STATE).find((code) => text.includes(code)) ?? null;
}

function productRequestId(value) {
  const cli = value?.response?.stdout;
  const direct = cli?.data?.id ?? cli?.id ?? cli?.data?.jobId ?? cli?.jobId ??
    value?.data?.id ?? value?.id ?? value?.data?.jobId ?? value?.jobId;
  if (typeof direct === 'string' && direct.length > 0) return direct;
  const content = value?.response?.jsonrpc?.result?.content;
  const text = Array.isArray(content) ? content.find((entry) => entry?.type === 'text')?.text : null;
  if (typeof text !== 'string') return null;
  try {
    const payload = JSON.parse(text);
    return payload?.data?.id ?? payload?.id ?? payload?.data?.jobId ?? payload?.jobId ?? null;
  } catch { return null; }
}

function safeFailureCode(error) {
  const code = error?.code;
  return typeof code === 'string' && /^[A-Za-z0-9_]+$/u.test(code)
    ? code : 'a08_harness_failure';
}

export function createP158W7A08DevelopmentDriver({ manifest, invoke } = {}) {
  const injected = typeof invoke === 'function';
  const selectedInvoke = invoke ?? invokeFrozenCandidate;
  const driver = {
    async execute(cell, operationCorrelationId) {
      assertExecutable(manifest.candidate);
      const environment = { ...process.env, HOME: cell.home, AGENT_BROWSER_HOME: cell.agentHome,
        XDG_RUNTIME_DIR: cell.xdgRuntimeDir, AGENT_BROWSER_SOCKET_DIR: cell.socketDir,
        AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
        AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0' };
      const statePath = join(cell.agentHome, 'service', 'state.json');
      const beforeBytes = await readFile(statePath);
      if (sha256(beforeBytes) !== cell.stateSha256) {
        fail('a08_effect_time_state_drift', `${cell.cellId} changed after fixture sealing`);
      }
      const beforeState = JSON.parse(beforeBytes);
      const response = await selectedInvoke({ binaryPath: manifest.candidate.binaryPath, binarySha256:
        manifest.candidate.binarySha256, cell: structuredClone(cell), environment,
        operationCorrelationId });
      const afterBytes = await readFile(statePath);
      const afterState = JSON.parse(afterBytes);
      const liveBrowserCount = (state) => Object.values(state.browsers ?? {}).filter((browser) =>
        Number.isInteger(browser.pid) && browser.pid > 1).length;
      return { response, provenance: { requestedAction: cell.action,
        commandSurface: cell.action === 'view_focus' ? 'mcp_service_request' : 'candidate_cli',
        candidateSha256: manifest.candidate.binarySha256,
        fixtureStateSha256: cell.stateSha256, parserReceiptSha256: cell.parserReceiptSha256 },
      effectEvidence: { beforeStateSha256: sha256(beforeBytes), afterStateSha256: sha256(afterBytes),
        liveBrowserCountBefore: liveBrowserCount(beforeState),
        liveBrowserCountAfter: liveBrowserCount(afterState),
        browserEffectObserved: liveBrowserCount(afterState) > liveBrowserCount(beforeState) } };
    },
  };
  if (!injected) Object.defineProperty(driver, BUILTIN_DRIVER, { value: true });
  return freeze(driver);
}

async function invokeCli({ binaryPath, cell, environment }) {
  const marker = 'data:text/html,p158-a08-synthetic';
  const commandArgs = {
    launch: ['open', marker],
    remote_view_open: ['remote-view', 'open', marker, '--dry-run'],
    tab_switch: ['tab', '0'],
  }[cell.action];
  if (!commandArgs) fail('a08_cli_action_invalid', cell.action);
  try {
    const output = await execFile(binaryPath,
      ['--json', '--session', cell.sessionId, ...commandArgs], {
        env: environment, maxBuffer: 4 * 1024 * 1024, timeout: 120_000,
      });
    return { exitCode: 0, stdout: parseCandidateJson(output.stdout),
      stderrSha256: sha256(output.stderr ?? '') };
  } catch (error) {
    const stdout = String(error.stdout ?? '');
    return { exitCode: Number.isInteger(error.code) ? error.code : null,
      stdout: parseCandidateJson(stdout), stderrSha256: sha256(error.stderr ?? ''),
      transportFailureCode: Number.isInteger(error.code) ? null : (error.code ?? null) };
  }
}

function parseCandidateJson(stdout) {
  const lines = String(stdout).trim().split(/\r?\n/u).filter(Boolean);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    try { return JSON.parse(lines[index]); } catch { /* keep searching */ }
  }
  fail('a08_candidate_response_invalid', 'Frozen candidate emitted no JSON response');
}

function invokeMcp({ binaryPath, cell, environment }) {
  return new Promise((resolvePromise, reject) => {
    const child = nodeSpawn(binaryPath, ['--session', cell.sessionId, 'mcp', 'serve'], {
      env: environment, stdio: ['pipe', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    let settled = false;
    const finish = (callback, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      child.stdin.end();
      child.kill('SIGTERM');
      callback(value);
    };
    const timer = setTimeout(() => finish(reject, Object.assign(
      new Error('A08 MCP request timed out'), { code: 'a08_mcp_timeout' })), 120_000);
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.stdout.setEncoding('utf8');
    child.stdout.on('data', (chunk) => {
      stdout += chunk;
      const newline = stdout.indexOf('\n');
      if (newline < 0) return;
      try {
        const jsonrpc = JSON.parse(stdout.slice(0, newline));
        finish(resolvePromise, { exitCode: null, jsonrpc,
          stderrSha256: sha256(stderr) });
      } catch (error) {
        finish(reject, Object.assign(error, { code: 'a08_mcp_response_invalid' }));
      }
    });
    child.on('error', (error) => finish(reject, Object.assign(error,
      { code: error.code ?? 'a08_mcp_spawn_failed' })));
    child.on('exit', (code, signal) => {
      if (!settled) finish(reject, Object.assign(new Error('A08 MCP server exited before response'),
        { code: 'a08_mcp_early_exit', details: { code, signal } }));
    });
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: {
      name: 'service_request', arguments: { action: 'view_focus',
        serviceName: 'P158SyntheticService', agentName: 'p158-a08', taskName: 'identityReplay',
        sessionName: cell.sessionId, browserId: `session:${cell.sessionId}`,
        params: { sessionName: cell.sessionId, browserId: `session:${cell.sessionId}`, index: 0,
          targetId: 'p158-a08-synthetic-target' } },
    } })}\n`);
  });
}

async function invokeFrozenCandidate(input) {
  return input.cell.action === 'view_focus' ? invokeMcp(input) : invokeCli(input);
}

function validateStored(record, manifest, kind) {
  const { receiptSha256, ...body } = record;
  if (receiptSha256 !== sha256(body) || record.campaignRunId !== manifest.campaignRunId ||
      record.kind !== kind) fail('a08_append_only_receipt_invalid', record.cellId ?? kind);
}

async function executeAttempt({ attempt, manifest, receiptStore, driver, clock }) {
  if (attempt.environmentId !== 'E1') {
    return freeze({ resultState: 'skipped_blocked', actionCount: 0, actionIds: [], receipts: [],
      artifactIds: [], effectState: 'no_effect', retryDisposition: 'prohibited_opportunistic_retry',
      retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
      blocker: 'a08_isolated_e1_only' });
  }
  const receipts = [];
  for (const cell of manifest.cells) {
    const existingTerminal = await receiptStore.readTerminal(cell.cellId);
    if (existingTerminal) {
      validateStored(existingTerminal, manifest, 'terminal');
      receipts.push(freeze(structuredClone(existingTerminal)));
      continue;
    }
    const claim = await receiptStore.readClaim(cell.cellId);
    if (claim) {
      validateStored(claim, manifest, 'claim');
      const uncertain = { schemaVersion: 'agent-browser.p158-w7-a08-cell-receipt.v1', kind: 'terminal',
        campaignRunId: manifest.campaignRunId, caseId: 'A08', attemptId: attempt.attemptId,
        actionId: cell.cellId, cellId: cell.cellId, environmentId: 'E1', identityState: cell.identityState,
        action: cell.action,
        operationCorrelationId: `${manifest.campaignRunId}:${cell.cellId}:${cell.action}`,
        state: 'failed', resultState: 'safety_stopped',
        effectState: 'effect_uncertain', failure: { code: 'a08_claimed_without_terminal' },
        observedAt: clock(), retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
        repairAttempted: false, garbageCollectionAttempted: false };
      uncertain.receiptSha256 = sha256(uncertain);
      await receiptStore.appendTerminal(freeze(structuredClone(uncertain)));
      receipts.push(freeze(uncertain));
      continue;
    }
    const claimBody = { schemaVersion: 'agent-browser.p158-w7-a08-cell-claim.v1', kind: 'claim',
      campaignRunId: manifest.campaignRunId, caseId: 'A08', attemptId: attempt.attemptId,
      actionId: cell.cellId, cellId: cell.cellId, environmentId: 'E1', identityState: cell.identityState,
      action: cell.action, stateSha256: cell.stateSha256, parserReceiptSha256: cell.parserReceiptSha256,
      operationCorrelationId: `${manifest.campaignRunId}:${cell.cellId}:${cell.action}`,
      claimedAt: clock() };
    const claimed = freeze({ ...claimBody, receiptSha256: sha256(claimBody) });
    await receiptStore.appendClaim(claimed);
    let terminalBody;
    try {
      const operationCorrelationId = `${manifest.campaignRunId}:${cell.cellId}:${cell.action}`;
      const response = await driver.execute(cell, operationCorrelationId);
      const observedFailure = extractFailure(response);
      const sanitized = redact(response);
      const effectEvidenceValid = response?.effectEvidence?.beforeStateSha256 === cell.stateSha256 &&
        SHA256.test(response?.effectEvidence?.afterStateSha256 ?? '') &&
        response.effectEvidence.browserEffectObserved === false &&
        response?.provenance?.requestedAction === cell.action;
      const reproduced = observedFailure === cell.expectedFailure && effectEvidenceValid;
      const resultState = reproduced ? 'reproduced_historical_failure' :
        (observedFailure === cell.expectedFailure ? 'harness_failure' : 'new_product_failure');
      terminalBody = { schemaVersion: 'agent-browser.p158-w7-a08-cell-receipt.v1', kind: 'terminal',
        campaignRunId: manifest.campaignRunId, caseId: 'A08', attemptId: attempt.attemptId,
        actionId: cell.cellId, cellId: cell.cellId, environmentId: 'E1', identityState: cell.identityState,
        action: cell.action, expectedFailure: cell.expectedFailure, observedFailure,
        operationCorrelationId,
        responseSha256: sha256(sanitized), responseEvidence: sanitized,
        productRequestIdSha256: productRequestId(response) ? sha256(productRequestId(response)) : null,
        productRequestIdState: productRequestId(response) ? 'observed_hashed' : 'not_returned',
        stateSha256: cell.stateSha256, parserReceiptSha256: cell.parserReceiptSha256,
        state: reproduced ? 'passed' : 'failed',
        resultState, effectState: reproduced ? 'verified_no_browser_effect' : 'effect_uncertain',
        observedAt: clock(),
        retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
        repairAttempted: false, garbageCollectionAttempted: false };
    } catch (error) {
      terminalBody = { schemaVersion: 'agent-browser.p158-w7-a08-cell-receipt.v1', kind: 'terminal',
        campaignRunId: manifest.campaignRunId, caseId: 'A08', attemptId: attempt.attemptId,
        actionId: cell.cellId, cellId: cell.cellId, environmentId: 'E1', identityState: cell.identityState,
        action: cell.action,
        operationCorrelationId: `${manifest.campaignRunId}:${cell.cellId}:${cell.action}`,
        state: 'failed', resultState: 'harness_failure', effectState: 'effect_uncertain',
        failure: { code: safeFailureCode(error), messageSha256: sha256(error?.message ?? String(error)) },
        observedAt: clock(), retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
        repairAttempted: false, garbageCollectionAttempted: false };
    }
    const finalReceipt = freeze({ ...terminalBody, receiptSha256: sha256(terminalBody) });
    await receiptStore.appendTerminal(finalReceipt);
    receipts.push(finalReceipt);
  }
  const allReproduced = receipts.length === 8 && receipts.every((r) =>
    r.resultState === 'reproduced_historical_failure');
  const resultState = allReproduced ? 'reproduced_historical_failure' :
    (receipts.some((r) => r.resultState === 'safety_stopped') ? 'safety_stopped' :
      (receipts.some((r) => r.resultState === 'harness_failure') ? 'harness_failure' :
        'new_product_failure'));
  return freeze({ resultState, actionCount: receipts.length,
    actionIds: receipts.map((r) => r.actionId), receipts,
    artifactIds: receipts.map((r) => `p158-w7-a08:${r.receiptSha256}`),
    effectState: allReproduced ? 'verified_no_browser_effect' : 'effect_uncertain',
    retryDisposition: 'prohibited_opportunistic_retry', retryAttempted: false,
    repairAttempted: false, garbageCollectionAttempted: false });
}

export function createP158W7A08LiveBundle({ schedule, replayManifest, receiptStore, driver = null,
  clock = () => new Date().toISOString() } = {}) {
  const manifest = validateManifest(replayManifest, schedule);
  if (!['readClaim', 'appendClaim', 'readTerminal', 'appendTerminal']
    .every((method) => typeof receiptStore?.[method] === 'function')) {
    fail('a08_receipt_store_invalid', 'A08 requires append-only claim and terminal receipt custody');
  }
  const selectedDriver = driver ?? createP158W7A08DevelopmentDriver({ manifest });
  const contract = schedule.caseContracts.find((entry) => entry.caseId === 'A08');
  const adapter = createP158CaseAdapter({ caseId: 'A08', evidenceProfile: contract.evidenceProfile,
    executionContract: contract.executionContract,
    execute: ({ attempt }) => executeAttempt({ attempt, manifest, receiptStore,
      driver: selectedDriver, clock }) });
  const source = freeze({ sourcePath: P158_W7_A08_SOURCE_PATH, sourceSha256: sourceSha256(),
    fixturePath: P158_W7_A08_FIXTURE_PATH, fixtureSha256: fixtureSha256() });
  return freeze({ schemaVersion: 'agent-browser.p158-w7-a08-live-bundle.v1',
    freezeEligible: selectedDriver[BUILTIN_DRIVER] === true, providerFree: false,
    concreteCaseIds: ['A08'], adapters: [adapter], campaignRunId: manifest.campaignRunId,
    replayManifestSha256: manifest.manifestSha256, candidateSha256: manifest.candidate.binarySha256,
    liveHookManifestSha256: manifest.liveHookManifestSha256,
    environmentSealSha256s: structuredClone(manifest.environmentSealSha256s),
    liveHookIds: [P158_W7_A08_HOOK_ID], driverSource: source,
    loggingRequestExpectations: [],
    loggingOperationDescriptors: enumerateP158W7A08LoggingOperations({
      campaignRunId: manifest.campaignRunId }),
    loggingReadiness: { complete: false,
      gapCode: 'caller_product_request_id_not_supported_by_selected_command_surface' },
    adapterBindingSha256: sha256({ caseIds: ['A08'], campaignRunId: manifest.campaignRunId,
      replayManifestSha256: manifest.manifestSha256, candidateSha256: manifest.candidate.binarySha256,
      liveHookManifestSha256: manifest.liveHookManifestSha256,
      environmentSealSha256s: manifest.environmentSealSha256s, source,
      liveHookIds: [P158_W7_A08_HOOK_ID] }) });
}

export function p158W7A08SourceBinding() {
  return freeze({ hookId: P158_W7_A08_HOOK_ID, sourcePath: P158_W7_A08_SOURCE_PATH,
    sourceSha256: sourceSha256(), fixturePath: P158_W7_A08_FIXTURE_PATH,
    fixtureSha256: fixtureSha256() });
}
