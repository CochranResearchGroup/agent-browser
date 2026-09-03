import { execFile as execFileCallback } from 'node:child_process';
import { readFile, readdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

import {
  canonicalJson,
  createFileArtifactStore,
  sha256,
} from './p158-campaign-controller.js';
import { runP158CampaignPhases } from './p158-campaign-phase-orchestrator.js';

const execFile = promisify(execFileCallback);
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const SHA256 = /^[a-f0-9]{64}$/u;
const STATE_ROOT = 'live-campaign-entrypoint';

export class P158LiveCampaignEntrypointError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158LiveCampaignEntrypointError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158LiveCampaignEntrypointError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function without(value, fields) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([field]) => !fields.includes(field)));
}

function checkpoint(value) {
  const body = clone(value);
  return { ...body, checkpointSha256: sha256(body) };
}

function verifyCheckpoint(value, code = 'entrypoint_checkpoint_changed') {
  if (!value || value.checkpointSha256 !== sha256(without(value, ['checkpointSha256']))) {
    fail(code, 'An append-only live campaign checkpoint is missing or changed');
  }
  return value;
}

async function readOptionalJson(store, path) {
  try {
    const bytes = await store.read(path);
    return JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function writeCheckpoint(store, path, value) {
  const sealed = checkpoint(value);
  await store.writeOnce(path, canonicalJson(sealed));
  return sealed;
}

function pathInside(parent, child) {
  const path = relative(resolve(parent), resolve(child));
  return path === '' || (!path.startsWith(`..${sep}`) && path !== '..' && !isAbsolute(path));
}

function assertRunRoot(runRoot) {
  if (!isAbsolute(runRoot ?? '')) fail('run_root_invalid', 'The frozen campaign run root must be absolute');
  if (pathInside(REPO_ROOT, runRoot)) fail('run_root_inside_repository', 'Campaign runtime state cannot be stored in the repository');
  const defaultRoot = join(homedir(), '.agent-browser');
  if (pathInside(defaultRoot, runRoot) || pathInside(runRoot, defaultRoot) || resolve(runRoot) === resolve(homedir())) {
    fail('production_root_prohibited', 'The live campaign entrypoint cannot access the default or production runtime root');
  }
}

function assertIsolatedPath(runRoot, path, field) {
  if (!isAbsolute(path ?? '') || !pathInside(runRoot, path)) {
    fail('isolated_runtime_path_invalid', `${field} must be an absolute child of the frozen run root`);
  }
}

function assertRelativeSourcePath(path, field) {
  if (typeof path !== 'string' || path.length === 0 || isAbsolute(path) || path.split('/').includes('..')) {
    fail('source_path_invalid', `${field} must be a repository-relative reviewed source path`);
  }
  return path;
}

async function loadBoundJson({ runRoot, descriptor, field }) {
  assertIsolatedPath(runRoot, descriptor?.path, `${field}.path`);
  if (!SHA256.test(descriptor?.sha256 ?? '')) fail('authority_digest_invalid', `${field}.sha256 is required`);
  const bytes = await readFile(descriptor.path);
  const actual = sha256(bytes);
  if (actual !== descriptor.sha256) {
    fail('authority_digest_mismatch', `${field} changed after its entrypoint descriptor was sealed`, {
      field, expected: descriptor.sha256, actual,
    });
  }
  try {
    return { value: JSON.parse(bytes.toString('utf8')), bytes };
  } catch {
    fail('authority_json_invalid', `${field} is not valid JSON`);
  }
}

function assertSelfDigest(value, field, digestField) {
  if (!SHA256.test(value?.[digestField] ?? '') || value[digestField] !== sha256(without(value, [digestField]))) {
    fail('authority_self_digest_mismatch', `${field} does not preserve its canonical self-digest`);
  }
}

function manifestScheduleProjection(attempt) {
  return {
    scheduleSequence: attempt.scheduleSequence,
    scheduleId: attempt.scheduleId,
    caseId: attempt.caseId,
    attemptId: attempt.attemptId,
    repetition: attempt.repetition,
    seed: attempt.seed,
    environmentIds: attempt.environmentIds,
    dependsOnAttemptIds: attempt.dependsOnAttemptIds,
    preconditionIds: attempt.preconditionIds,
    stimuli: attempt.stimuli,
    evidenceProfile: attempt.evidenceProfile,
    externalIngressRequired: attempt.externalIngressRequired,
    preExecutionBlocker: attempt.preExecutionBlocker,
  };
}

function verifyRuntimeIdentity({ descriptor, manifest, runtimeIdentity, expectedRuntimeIdentity = null }) {
  const environmentById = new Map(runtimeIdentity?.environments?.map((entry) => [entry.environmentId, entry]));
  if (runtimeIdentity?.runtimeLane !== 'development' || runtimeIdentity.production !== false ||
      runtimeIdentity.runId !== descriptor.runId || runtimeIdentity.candidateSha256 !== manifest.candidate.candidateSha256 ||
      runtimeIdentity.repairAllowed !== false || runtimeIdentity.retryAllowed !== false ||
      runtimeIdentity.garbageCollectionAllowed !== false) {
    fail('runtime_identity_drift', 'Current runtime identity is not the frozen development campaign identity');
  }
  for (const seal of manifest.environmentSeals) {
    const observed = environmentById.get(seal.environmentId);
    if (!observed || observed.identitySha256 !== seal.identitySha256 || sha256(observed.identity) !== seal.identitySha256) {
      fail('runtime_identity_drift', `${seal.environmentId} current runtime identity differs from its frozen seal`);
    }
  }
  if (expectedRuntimeIdentity && sha256(runtimeIdentity) !== sha256(expectedRuntimeIdentity)) {
    fail('runtime_identity_drift', 'The fresh runtime identity readback differs from the sealed runtime identity artifact');
  }
}

function verifyFrozenAuthorities({ descriptor, manifest, manifestBytes, freeze, schedule, phasePreparation, liveHooks, runtimeIdentity }) {
  if (manifest?.schemaVersion !== 'agent-browser.p158-campaign-manifest.v1' || manifest.planId !== 'P158' ||
      freeze?.schemaVersion !== 'agent-browser.p158-campaign-freeze.v1' || freeze.controllerState !== 'frozen' ||
      manifest.runId !== descriptor.runId || freeze.runId !== descriptor.runId ||
      freeze.manifestSha256 !== sha256(manifestBytes) || freeze.candidateSha256 !== manifest.candidate?.candidateSha256 ||
      freeze.startedCaseCount !== 0 || freeze.startedAttemptCount !== 0) {
    fail('frozen_campaign_identity_mismatch', 'Manifest and freeze receipt do not describe the exact zero-start frozen campaign');
  }
  if (manifest.candidate.candidateSha256 !== sha256(without(manifest.candidate, ['candidateSha256']))) {
    fail('candidate_identity_drift', 'The frozen candidate identity is not self-consistent');
  }
  if (freeze.artifactBindingsSha256 !== sha256(manifest.artifactBindings) ||
      freeze.environmentSealsSha256 !== sha256(manifest.environmentSeals) ||
      freeze.calibrationSha256 !== sha256(manifest.calibration) ||
      freeze.fixtureSealSha256 !== sha256(manifest.fixtureSeal)) {
    fail('frozen_campaign_identity_mismatch', 'A manifest prerequisite differs from the freeze receipt');
  }
  if (!SHA256.test(schedule?.scheduleSha256 ?? '') ||
      schedule.scheduleSha256 !== sha256(without(schedule, ['scheduleSha256', 'adapterReadiness']))) {
    fail('authority_self_digest_mismatch', 'schedule does not preserve its canonical self-digest');
  }
  if (schedule.schemaVersion !== 'agent-browser.p158-execution-schedule.v1' || schedule.planId !== 'P158' ||
      schedule.registrySha256 !== manifest.registrySha256 ||
      sha256(schedule.attempts.map(manifestScheduleProjection)) !== sha256(manifest.schedule)) {
    fail('schedule_identity_drift', 'The loaded schedule is not the schedule frozen into the campaign manifest');
  }
  assertSelfDigest(phasePreparation, 'phasePreparation', 'preparationSha256');
  assertSelfDigest(liveHooks, 'liveHookManifest', 'manifestSha256');
  if (phasePreparation.schemaVersion !== 'agent-browser.p158-campaign-phase-preparation.v1' ||
      phasePreparation.runId !== descriptor.runId || phasePreparation.scheduleSha256 !== schedule.scheduleSha256 ||
      phasePreparation.liveHookManifestSha256 !== liveHooks.manifestSha256 ||
      liveHooks.schemaVersion !== 'agent-browser.p158-live-hook-manifest.v1' ||
      liveHooks.scheduleSha256 !== schedule.scheduleSha256 ||
      liveHooks.candidateSha256 !== manifest.candidate.candidateSha256 ||
      liveHooks.repairAllowed !== false || liveHooks.retryAllowed !== false || liveHooks.garbageCollectionAllowed !== false) {
    fail('phase_preparation_identity_drift', 'The schedule, phase preparation, and live-hook manifest are not one frozen identity');
  }
  verifyRuntimeIdentity({ descriptor, manifest, runtimeIdentity });
}

async function verifyCandidateAndSources({ descriptor, manifest, liveHooks, sourceCommitReadback }) {
  const candidateBytes = await readFile(descriptor.candidateExecutablePath);
  if (sha256(candidateBytes) !== manifest.candidate.binarySha256) {
    fail('candidate_binary_drift', 'The installed candidate executable differs from the frozen candidate digest');
  }
  const currentCommit = sourceCommitReadback
    ? await sourceCommitReadback()
    : (await execFile('git', ['rev-parse', 'HEAD'], { cwd: REPO_ROOT })).stdout.trim();
  if (currentCommit !== manifest.candidate.sourceCommit) {
    fail('source_commit_drift', 'The reviewed source checkout differs from the frozen candidate source commit');
  }
  const sources = new Map();
  for (const binding of [...liveHooks.hookBindings, ...liveHooks.adapterBindings]) {
    const path = assertRelativeSourcePath(binding.sourcePath, 'liveHookManifest.sourcePath');
    const prior = sources.get(path);
    if (prior && prior !== binding.sourceSha256) fail('source_identity_drift', `${path} has conflicting frozen digests`);
    sources.set(path, binding.sourceSha256);
  }
  const assemblyPath = assertRelativeSourcePath(descriptor.bundleAssembly.sourcePath, 'bundleAssembly.sourcePath');
  if (sources.get(assemblyPath) !== descriptor.bundleAssembly.sourceSha256) {
    fail('bundle_assembly_unsealed', 'The bundle assembly module is not an exact live-hook source binding');
  }
  for (const [path, expected] of sources) {
    const actual = sha256(await readFile(resolve(REPO_ROOT, path)));
    if (actual !== expected) fail('source_identity_drift', `${path} changed after live-hook freeze`, { expected, actual });
  }
}

async function loadTerminalFiles(store, schedule) {
  const results = [];
  for (const attempt of schedule.attempts) {
    const path = `${STATE_ROOT}/attempts/${attempt.attemptId}.json`;
    const value = await readOptionalJson(store, path);
    if (value) results.push(without(verifyCheckpoint(value), ['checkpointSha256']));
  }
  return results;
}

function ledgerArtifact(receipt) {
  return {
    artifactId: receipt.artifactId,
    relativePath: receipt.relativePath,
    mediaType: receipt.mediaType ?? receipt.metadata?.mediaType ?? 'application/octet-stream',
    sha256: receipt.sha256,
    byteCount: receipt.byteCount,
    captureState: receipt.captureState ?? receipt.metadata?.captureState ?? 'complete',
    captureGap: receipt.captureGap ?? receipt.metadata?.captureGap ?? null,
    redactions: clone(receipt.redactions ?? receipt.metadata?.redactions ?? []),
    parentArtifactSha256s: clone(receipt.parentArtifactSha256s ?? receipt.metadata?.parentArtifactSha256s ?? []),
  };
}

async function loadCanonicalLedger({ store, runRoot, manifestSha256, freeze }) {
  let names;
  try { names = (await readdir(join(runRoot, 'ledger'))).filter((name) => /^\d{8}-.+\.json$/u.test(name)).sort(); }
  catch (error) { if (error?.code === 'ENOENT') names = []; else throw error; }
  if (names.length === 0) fail('canonical_ledger_missing', 'The frozen campaign canonical ledger is missing');
  const records = [];
  let previous = null;
  for (const [sequence, name] of names.entries()) {
    const bytes = await store.read(`ledger/${name}`);
    const record = JSON.parse(bytes.toString('utf8'));
    if (record.schemaVersion !== 'agent-browser.p158-campaign-result.v1' || record.planId !== 'P158' ||
        record.runId !== freeze.runId || record.manifestSha256 !== manifestSha256 || record.sequence !== sequence ||
        record.recordId !== `${freeze.runId}:record:${String(sequence).padStart(8, '0')}` ||
        record.previousRecordSha256 !== previous ||
        name !== `${String(sequence).padStart(8, '0')}-${record.recordType}.json`) {
      fail('canonical_ledger_invalid', `Canonical ledger record ${name} is not in the frozen chain`);
    }
    previous = sha256(bytes);
    records.push({ ...record, sha256: previous, byteCount: bytes.byteLength });
  }
  const sealIndex = records.findIndex((record) => record.recordType === 'evidence_seal');
  if (records.some((record) => record.recordType === 'analysis_terminal') ||
      (sealIndex !== -1 && sealIndex !== records.length - 1) ||
      records.filter((record) => record.recordType === 'evidence_seal').length > 1) {
    fail('canonical_ledger_post_seal_invalid', 'Live execution cannot accept analysis records or records after evidence seal');
  }
  if (records[0]?.recordType !== 'controller_transition' || records[0]?.payload?.to !== 'prepared' ||
      records[0].sha256 !== freeze.preparedLedgerHeadSha256 ||
      records[1]?.recordType !== 'controller_transition' || records[1]?.payload?.to !== 'frozen') {
    fail('canonical_ledger_freeze_boundary_invalid', 'The ledger does not preserve the exact prepared and frozen boundary');
  }
  return records;
}

async function createResumableController({ store, runRoot, manifest, manifestSha256, freeze, schedule, scheduledTeardown, clock }) {
  const startedPath = `${STATE_ROOT}/controller-started.json`;
  const executionTerminalPath = `${STATE_ROOT}/controller-execution-terminal.json`;
  const evidenceSealPath = `${STATE_ROOT}/controller-evidence-sealed.json`;
  const existingStart = await readOptionalJson(store, startedPath);
  if (existingStart) verifyCheckpoint(existingStart);
  const existingExecutionTerminal = await readOptionalJson(store, executionTerminalPath);
  if (existingExecutionTerminal) verifyCheckpoint(existingExecutionTerminal);
  const existingSeal = await readOptionalJson(store, evidenceSealPath);
  if (existingSeal) verifyCheckpoint(existingSeal);
  const ledger = await loadCanonicalLedger({ store, runRoot, manifestSha256, freeze });
  const sealedRecord = ledger.find((record) => record.recordType === 'evidence_seal');
  const executionTerminalRecord = [...ledger].reverse().find((record) =>
    record.recordType === 'controller_transition' && record.payload?.to === 'execution_terminal');
  let state = sealedRecord ? 'evidence_sealed' : executionTerminalRecord ? 'execution_terminal' :
    ledger.some((record) => record.recordType === 'controller_transition' && record.payload?.to === 'executing')
      ? 'executing' : 'frozen';
  if ((existingSeal && !sealedRecord) || (existingExecutionTerminal && !executionTerminalRecord) ||
      (existingStart && state === 'frozen')) {
    fail('checkpoint_without_canonical_ledger', 'A live checkpoint exists without its contemporaneous canonical ledger record');
  }
  const terminalByAttempt = new Map(ledger.filter((record) => record.recordType === 'attempt_terminal')
    .map((record) => [record.payload.attempt.attemptId, record]));
  const checkpointResults = await loadTerminalFiles(store, schedule);
  const results = checkpointResults.filter((result) => terminalByAttempt.has(result.attemptId));
  for (const record of terminalByAttempt.values()) {
    if (!results.some((result) => result.attemptId === record.payload.attempt.attemptId)) {
      results.push({ ...clone(record.payload), ...clone(record.payload.attempt), recordId: record.recordId });
    }
  }
  const artifacts = [];
  for (const attempt of schedule.attempts) {
    const metadata = await readOptionalJson(store, `${STATE_ROOT}/artifacts/${attempt.attemptId}.json`);
    if (metadata) artifacts.push(without(verifyCheckpoint(metadata), ['checkpointSha256']));
  }
  let teardown = await readOptionalJson(store, `${STATE_ROOT}/scheduled-teardown.json`);
  if (teardown) teardown = without(verifyCheckpoint(teardown), ['checkpointSha256']);
  else {
    const teardownRecord = [...ledger].reverse().find((record) => record.recordType === 'scheduled_teardown_terminal');
    teardown = teardownRecord ? { ...clone(scheduledTeardown), ...clone(teardownRecord.payload) } : clone(scheduledTeardown);
  }
  let previousLedgerSha256 = ledger.at(-1).sha256;
  const monotonic = () => {
    const candidate = typeof clock.monotonicNow === 'function' ? Number(clock.monotonicNow()) : NaN;
    return Number.isInteger(candidate) && candidate > ledger.at(-1).monotonicTimeNanoseconds
      ? candidate : ledger.at(-1).monotonicTimeNanoseconds + 1;
  };
  const appendLedger = async (recordType, controllerState, payload, recordArtifacts = []) => {
    const sequence = ledger.length;
    const record = {
      schemaVersion: 'agent-browser.p158-campaign-result.v1', planId: 'P158', runId: manifest.runId,
      manifestSha256, recordId: `${manifest.runId}:record:${String(sequence).padStart(8, '0')}`,
      sequence, previousRecordSha256: previousLedgerSha256, recordType, controllerState,
      wallTime: clock.wallNow(), monotonicTimeNanoseconds: monotonic(), clockOffsetMilliseconds: 0,
      payload: clone(payload), artifacts: recordArtifacts.map(ledgerArtifact),
    };
    const bytes = canonicalJson(record);
    await store.writeOnce(`ledger/${String(sequence).padStart(8, '0')}-${recordType}.json`, bytes);
    previousLedgerSha256 = sha256(bytes);
    ledger.push({ ...record, sha256: previousLedgerSha256, byteCount: Buffer.byteLength(bytes) });
    return ledger.at(-1);
  };
  const snapshot = () => ({
    schemaVersion: 'agent-browser.p158-live-resumed-controller.v1',
    state,
    runId: manifest.runId,
    candidate: clone(manifest.candidate),
    results: clone(results),
    evidence: { artifacts: clone(artifacts), events: clone(ledger), eventHeadSha256: previousLedgerSha256 },
    scheduledTeardown: clone(teardown),
  });
  return {
    snapshot,
    async startExecution() {
      if (state !== 'frozen') fail('controller_state_invalid', `Cannot start from ${state}`);
      await appendLedger('controller_transition', 'executing', {
        kind: 'controller_transition', from: 'frozen', to: 'executing',
        reason: 'frozen live campaign execution started', terminal: false,
      });
      await writeCheckpoint(store, startedPath, {
        state: 'executing', runId: manifest.runId, manifestSha256: sha256(canonicalJson(manifest)), observedAt: clock.wallNow(),
      });
      state = 'executing';
    },
    async writeArtifact({ artifactId, relativePath, content, metadata = {} }) {
      if (state === 'evidence_sealed') fail('post_seal_refused', 'No evidence may be written after sealing');
      const bytes = typeof content === 'string' || content instanceof Uint8Array ? content : canonicalJson(content);
      let existing;
      try { existing = await store.read(relativePath); } catch (error) { if (error?.code !== 'ENOENT') throw error; }
      if (existing && sha256(existing) !== sha256(bytes)) fail('artifact_changed_across_resume', `${relativePath} changed across resume`);
      if (!existing) await store.writeOnce(relativePath, bytes);
      const previousArtifact = artifacts.at(-1);
      const receipt = { artifactId, relativePath, sha256: sha256(bytes), byteCount: Buffer.byteLength(bytes),
        mediaType: metadata.mediaType ?? 'application/octet-stream', captureState: metadata.captureState ?? 'complete',
        captureGap: metadata.captureGap ?? null, redactions: clone(metadata.redactions ?? []),
        parentArtifactSha256s: clone(metadata.parentArtifactSha256s ?? (previousArtifact ? [previousArtifact.sha256] : [])),
        metadata: clone(metadata) };
      const attemptId = artifactId.split(':').at(-2) ?? sha256(artifactId);
      const receiptPath = `${STATE_ROOT}/artifacts/${attemptId}.json`;
      const prior = await readOptionalJson(store, receiptPath);
      if (prior) {
        verifyCheckpoint(prior);
        if (sha256(without(prior, ['checkpointSha256'])) !== sha256(receipt)) fail('artifact_changed_across_resume', `${artifactId} changed across resume`);
      } else await writeCheckpoint(store, receiptPath, receipt);
      if (!artifacts.some((entry) => entry.artifactId === artifactId)) artifacts.push(receipt);
      if (!ledger.some((entry) => entry.recordType === 'artifact_recorded' && entry.payload?.artifactId === artifactId)) {
        await appendLedger('artifact_recorded', state, { kind: 'artifact_recorded', artifactId,
          capturePurpose: metadata.capturePurpose ?? 'campaign_evidence', terminal: false }, [receipt]);
      }
      return clone(receipt);
    },
    async recordAttempt(result) {
      if (state !== 'executing') fail('controller_state_invalid', `Cannot record an attempt from ${state}`);
      if (!schedule.attempts.some((entry) => entry.attemptId === result.attemptId)) fail('attempt_outside_schedule', result.attemptId);
      const path = `${STATE_ROOT}/attempts/${result.attemptId}.json`;
      const prior = await readOptionalJson(store, path);
      if (prior) {
        verifyCheckpoint(prior);
        if (sha256(without(prior, ['checkpointSha256'])) !== sha256(result)) fail('terminal_result_changed', `${result.attemptId} changed across resume`);
      } else await writeCheckpoint(store, path, result);
      if (!terminalByAttempt.has(result.attemptId)) {
        const attempt = schedule.attempts.find((entry) => entry.attemptId === result.attemptId);
        const payload = { kind: 'attempt_terminal', attempt: {
          scheduleId: attempt.scheduleId, caseId: attempt.caseId, attemptId: attempt.attemptId,
          repetition: attempt.repetition, seed: attempt.seed, environmentIds: clone(attempt.environmentIds),
        }, resultState: result.resultState,
        effectState: result.effectState ?? (result.resultState === 'passed' ? 'verified_effect' : 'no_effect'),
        retryDisposition: result.retryDisposition ?? 'prohibited_opportunistic_retry',
        completedAt: result.completedAt ?? clock.wallNow(), terminal: true,
        firstFailureSignature: result.firstFailureSignature ?? result.evidence?.signature ?? null,
        blocker: result.resultState === 'skipped_blocked' ? clone(result.blocker ?? attempt.preExecutionBlocker) : null,
        safetyStop: result.resultState === 'safety_stopped' ? clone(result.safetyStop) : null,
        causalIds: clone(result.causalIds ?? result.evidence?.causalIds ?? {}),
        };
        const evidenceIds = new Set(result.evidence?.artifactIds ?? []);
        const record = await appendLedger('attempt_terminal', 'executing', payload,
          artifacts.filter((artifact) => evidenceIds.has(artifact.artifactId)));
        terminalByAttempt.set(result.attemptId, record);
        result = { ...clone(result), recordId: record.recordId };
      }
      if (!results.some((entry) => entry.attemptId === result.attemptId)) results.push(clone(result));
    },
    async recordScheduledTeardown(result) {
      if (state !== 'executing') fail('controller_state_invalid', `Cannot record teardown from ${state}`);
      const path = `${STATE_ROOT}/scheduled-teardown.json`;
      const prior = await readOptionalJson(store, path);
      if (prior) fail('terminal_result_changed', 'Scheduled teardown is already terminal');
      teardown = { ...clone(scheduledTeardown), ...clone(result) };
      await appendLedger('scheduled_teardown_terminal', 'executing', {
        kind: 'scheduled_teardown_terminal', scheduleId: scheduledTeardown.attemptId,
        resultState: result.resultState,
        effectState: result.effectState ?? (result.resultState === 'passed' ? 'verified_effect' : 'no_effect'),
        retryDisposition: result.retryDisposition ?? 'prohibited_opportunistic_retry',
        completedAt: result.completedAt ?? clock.wallNow(), terminal: true,
      });
      await writeCheckpoint(store, path, teardown);
    },
    async finishExecution() {
      if (state !== 'executing') fail('controller_state_invalid', `Cannot finish from ${state}`);
      const missing = schedule.attempts.filter((attempt) => !results.some((result) => result.attemptId === attempt.attemptId));
      if (missing.length > 0 || !teardown?.resultState) fail('execution_not_terminal', 'All attempts and scheduled teardown must be terminal');
      if (!executionTerminalRecord) {
        const states = ['passed', 'reproduced_historical_failure', 'new_product_failure', 'harness_failure',
          'inconclusive', 'skipped_blocked', 'safety_stopped'];
        const resultCounts = Object.fromEntries(states.map((value) => [value,
          [...results, teardown].filter((entry) => entry.resultState === value).length]));
        await appendLedger('controller_transition', 'execution_terminal', {
          kind: 'controller_transition', from: 'executing', to: 'execution_terminal',
          reason: 'all live attempts and scheduled teardown are terminal', terminal: false, resultCounts,
        });
      }
      if (!existingExecutionTerminal) await writeCheckpoint(store, executionTerminalPath, {
        state: 'execution_terminal', resultSetSha256: sha256(results), teardownSha256: sha256(teardown), observedAt: clock.wallNow(),
      });
      state = 'execution_terminal';
    },
    async sealEvidence() {
      if (state !== 'execution_terminal') fail('controller_state_invalid', `Cannot seal from ${state}`);
      const body = {
        schemaVersion: 'agent-browser.p158-evidence-manifest.v1', runId: manifest.runId,
        candidateSha256: sha256(manifest.candidate), registrySha256: manifest.registrySha256,
        scheduleSha256: sha256(schedule), resultsSha256: sha256(results),
        events: clone(ledger), artifacts: clone(artifacts), eventHeadSha256: previousLedgerSha256,
      };
      const evidenceBytes = canonicalJson(body);
      await store.writeOnce('artifacts/manifest/sealed-evidence-manifest.json', evidenceBytes);
      const evidenceReceipt = { artifactId: `${manifest.runId}:sealed-manifest`,
        relativePath: 'artifacts/manifest/sealed-evidence-manifest.json', mediaType: 'application/json',
        sha256: sha256(evidenceBytes), byteCount: Buffer.byteLength(evidenceBytes), captureState: 'complete',
        captureGap: null, redactions: [], parentArtifactSha256s: artifacts.at(-1) ? [artifacts.at(-1).sha256] : [] };
      const sealedAt = clock.wallNow();
      await appendLedger('evidence_seal', 'evidence_sealed', { kind: 'evidence_seal',
        manifestSha256: evidenceReceipt.sha256, ledgerHeadSha256: previousLedgerSha256,
        artifactCount: artifacts.length + 1,
        artifactBytes: artifacts.reduce((sum, artifact) => sum + artifact.byteCount, 0) + evidenceReceipt.byteCount,
        allScheduledAttemptsTerminal: true, teardownTerminal: true, sealedAt, terminal: true,
      }, [evidenceReceipt]);
      await writeCheckpoint(store, evidenceSealPath, { state: 'evidence_sealed', manifestSha256: evidenceReceipt.sha256, sealedAt });
      state = 'evidence_sealed';
    },
  };
}

async function defaultBundleAssemblyLoader(sourcePath) {
  return import(`${pathToFileURL(resolve(REPO_ROOT, sourcePath)).href}?p158=${Date.now()}`);
}

async function defaultSourceCommitReadback() {
  return (await execFile('git', ['rev-parse', 'HEAD'], { cwd: REPO_ROOT })).stdout.trim();
}

/**
 * Executes one immutable Plan 0158 live campaign from its exact frozen files.
 * Preflight is read-only. It refuses default runtime roots, identity drift,
 * retries, repairs, garbage collection, and any new work after evidence seal.
 */
export async function runP158LiveCampaignEntrypoint({
  descriptorPath,
  descriptorSha256,
  clock = { wallNow: () => new Date().toISOString() },
  bundleAssemblyLoader = defaultBundleAssemblyLoader,
  sourceCommitReadback = defaultSourceCommitReadback,
  runCampaignPhases = runP158CampaignPhases,
  testing = false,
}) {
  if (!isAbsolute(descriptorPath ?? '') || !SHA256.test(descriptorSha256 ?? '')) {
    fail('entrypoint_descriptor_invalid', 'An absolute descriptor path and its frozen SHA-256 are required');
  }
  const descriptorBytes = await readFile(descriptorPath);
  if (sha256(descriptorBytes) !== descriptorSha256) fail('entrypoint_descriptor_changed', 'The live campaign descriptor changed after dispatch');
  const descriptor = JSON.parse(descriptorBytes.toString('utf8'));
  if (descriptor.schemaVersion !== 'agent-browser.p158-live-campaign-entrypoint.v1' || descriptor.planId !== 'P158' ||
      descriptor.runtimeLane !== 'development' || descriptor.production !== false || descriptor.repairAllowed !== false ||
      descriptor.retryAllowed !== false || descriptor.garbageCollectionAllowed !== false) {
    fail('entrypoint_descriptor_invalid', 'The descriptor must be development-only with repair, retry, and GC disabled');
  }
  assertRunRoot(descriptor.runRoot);
  if (!pathInside(descriptor.runRoot, descriptorPath)) fail('entrypoint_descriptor_invalid', 'The descriptor must be inside its frozen run root');
  for (const [field, path] of Object.entries(descriptor.isolation ?? {})) assertIsolatedPath(descriptor.runRoot, path, `isolation.${field}`);
  for (const field of ['home', 'xdgConfigHome', 'xdgRuntimeDir', 'xdgStateHome']) {
    if (!descriptor.isolation?.[field]) fail('isolated_runtime_path_invalid', `isolation.${field} is required`);
  }
  if (!isAbsolute(descriptor.candidateExecutablePath ?? '')) fail('candidate_path_invalid', 'Candidate executable path must be absolute');
  if (!descriptor.scheduledTeardown?.attemptId || !descriptor.scheduledTeardown?.environmentId) {
    fail('entrypoint_descriptor_invalid', 'The exact frozen scheduled teardown identity is required');
  }
  const store = createFileArtifactStore(descriptor.runRoot);
  const terminalPath = `${STATE_ROOT}/entrypoint-terminal.json`;
  const priorTerminal = await readOptionalJson(store, terminalPath);
  if (priorTerminal) verifyCheckpoint(priorTerminal);
  const postSeal = await readOptionalJson(store, `${STATE_ROOT}/controller-evidence-sealed.json`);
  let sealedManifest = null;
  for (const path of ['artifacts/manifest/sealed-evidence-manifest.json', 'manifest/sealed-evidence-manifest.json']) {
    try { sealedManifest = await store.read(path); } catch (error) { if (error?.code !== 'ENOENT') throw error; }
    if (sealedManifest) break;
  }
  if ((postSeal || sealedManifest) && !priorTerminal) fail('post_seal_refused', 'A sealed campaign cannot be executed or resumed');

  const [manifestLoaded, freezeLoaded, scheduleLoaded, preparationLoaded, hooksLoaded, runtimeLoaded, assemblyConfigLoaded] =
    await Promise.all([
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.manifest, field: 'manifest' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.freeze, field: 'freeze' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.schedule, field: 'schedule' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.phasePreparation, field: 'phasePreparation' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.liveHookManifest, field: 'liveHookManifest' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.runtimeIdentity, field: 'runtimeIdentity' }),
      loadBoundJson({ runRoot: descriptor.runRoot, descriptor: descriptor.bundleAssembly.configuration, field: 'bundleAssembly.configuration' }),
    ]);
  const manifest = manifestLoaded.value;
  const freeze = freezeLoaded.value;
  const schedule = scheduleLoaded.value;
  const phasePreparation = preparationLoaded.value;
  const liveHooks = hooksLoaded.value;
  const runtimeIdentity = runtimeLoaded.value;
  verifyFrozenAuthorities({ descriptor, manifest, manifestBytes: manifestLoaded.bytes, freeze, schedule, phasePreparation, liveHooks, runtimeIdentity });
  if (!testing && (schedule.caseCount !== 54 || schedule.attemptCount !== 1592 || schedule.attempts.length !== 1592)) {
    fail('schedule_identity_drift', 'A live P158 campaign requires the exact 54-case, 1,592-attempt schedule');
  }
  await verifyCandidateAndSources({ descriptor, manifest, liveHooks, sourceCommitReadback });

  if (priorTerminal) {
    if (priorTerminal.descriptorSha256 !== descriptorSha256) fail('entrypoint_terminal_drift', 'Existing terminal result belongs to another descriptor');
    if (priorTerminal.outcome !== 'completed') {
      const error = new P158LiveCampaignEntrypointError(
        'prior_terminal_failure',
        'The campaign already has an append-only failed terminal result and cannot be replayed',
      );
      error.terminalReceipt = clone(priorTerminal);
      throw error;
    }
    return clone(priorTerminal);
  }

  const sourceDigest = sha256({ descriptorSha256, manifestSha256: descriptor.manifest.sha256,
    scheduleSha256: schedule.scheduleSha256, phasePreparationSha256: phasePreparation.preparationSha256,
    liveHookManifestSha256: liveHooks.manifestSha256, runtimeIdentitySha256: descriptor.runtimeIdentity.sha256,
    bundleAssemblySourceSha256: descriptor.bundleAssembly.sourceSha256,
    bundleAssemblyConfigurationSha256: descriptor.bundleAssembly.configuration.sha256 });
  const startedPath = `${STATE_ROOT}/entrypoint-started.json`;
  const priorStart = await readOptionalJson(store, startedPath);
  if (priorStart) {
    verifyCheckpoint(priorStart);
    if (priorStart.sourceDigest !== sourceDigest) fail('entrypoint_source_drift', 'Entrypoint source or configuration changed across resume');
  } else await writeCheckpoint(store, startedPath, {
    state: 'started', descriptorSha256, sourceDigest, observedAt: clock.wallNow(),
    repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
  });

  let terminal;
  try {
    const assemblyModule = await bundleAssemblyLoader(descriptor.bundleAssembly.sourcePath);
    const readRuntimeIdentity = assemblyModule?.[descriptor.bundleAssembly.runtimeIdentityExport];
    if (typeof readRuntimeIdentity !== 'function') {
      fail('runtime_identity_probe_missing', 'The exact sealed assembly module omits its read-only current runtime identity probe');
    }
    const currentRuntimeIdentity = await readRuntimeIdentity(Object.freeze({
      descriptor: clone(descriptor), manifest: clone(manifest), expectedRuntimeIdentity: clone(runtimeIdentity),
      isolation: clone(descriptor.isolation),
    }));
    verifyRuntimeIdentity({ descriptor, manifest, runtimeIdentity: currentRuntimeIdentity, expectedRuntimeIdentity: runtimeIdentity });
    const construct = assemblyModule?.[descriptor.bundleAssembly.exportName];
    if (typeof construct !== 'function') fail('bundle_assembly_invalid', 'The exact sealed bundle assembly export is missing');
    const bundles = await construct(Object.freeze({
      descriptor: clone(descriptor), manifest: clone(manifest), freeze: clone(freeze), schedule: clone(schedule),
      phasePreparation: clone(phasePreparation), liveHookManifest: clone(liveHooks),
      runtimeIdentity: clone(currentRuntimeIdentity), configuration: clone(assemblyConfigLoaded.value),
      artifactStore: store, clock,
    }));
    if (bundles?.repairAttempted === true || bundles?.retryAttempted === true || bundles?.garbageCollectionAttempted === true) {
      fail('prohibited_lifecycle_effect', 'Bundle construction reported a repair, retry, or garbage-collection effect');
    }
    const controller = await createResumableController({
      store, runRoot: descriptor.runRoot, manifest, manifestSha256: descriptor.manifest.sha256, freeze, schedule,
      scheduledTeardown: descriptor.scheduledTeardown, clock,
    });
    const result = await runCampaignPhases({
      schedule, controller, w7Bundle: bundles.w7Bundle, w8Bundle: bundles.w8Bundle, w9: bundles.w9,
      runRoot: descriptor.runRoot, artifactStore: store, liveHookManifestSha256: liveHooks.manifestSha256,
      clock, phasePreparation,
    });
    terminal = await writeCheckpoint(store, terminalPath, {
      schemaVersion: 'agent-browser.p158-live-campaign-entrypoint-result.v1', state: 'terminal', outcome: 'completed',
      runId: descriptor.runId, descriptorSha256, sourceDigest, result: clone(result), completedAt: clock.wallNow(),
      repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
    });
  } catch (error) {
    terminal = await writeCheckpoint(store, terminalPath, {
      schemaVersion: 'agent-browser.p158-live-campaign-entrypoint-result.v1', state: 'terminal', outcome: 'failed',
      runId: descriptor.runId, descriptorSha256, sourceDigest, completedAt: clock.wallNow(),
      failure: { code: error?.code ?? 'campaign_entrypoint_failed', message: error?.message ?? String(error) },
      repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
    });
    throw Object.assign(error, { terminalReceipt: clone(terminal) });
  }
  return terminal;
}
