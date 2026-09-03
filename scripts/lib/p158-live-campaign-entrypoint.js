import { execFile as execFileCallback } from 'node:child_process';
import { readFile } from 'node:fs/promises';
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

async function createResumableController({ store, runRoot, manifest, schedule, scheduledTeardown, clock }) {
  const startedPath = `${STATE_ROOT}/controller-started.json`;
  const executionTerminalPath = `${STATE_ROOT}/controller-execution-terminal.json`;
  const evidenceSealPath = `${STATE_ROOT}/controller-evidence-sealed.json`;
  const existingStart = await readOptionalJson(store, startedPath);
  if (existingStart) verifyCheckpoint(existingStart);
  const existingExecutionTerminal = await readOptionalJson(store, executionTerminalPath);
  if (existingExecutionTerminal) verifyCheckpoint(existingExecutionTerminal);
  const existingSeal = await readOptionalJson(store, evidenceSealPath);
  if (existingSeal) verifyCheckpoint(existingSeal);
  let state = existingSeal ? 'evidence_sealed' : existingStart ? 'executing' : 'frozen';
  const results = await loadTerminalFiles(store, schedule);
  const artifacts = [];
  for (const attempt of schedule.attempts) {
    const metadata = await readOptionalJson(store, `${STATE_ROOT}/artifacts/${attempt.attemptId}.json`);
    if (metadata) artifacts.push(without(verifyCheckpoint(metadata), ['checkpointSha256']));
  }
  let teardown = await readOptionalJson(store, `${STATE_ROOT}/scheduled-teardown.json`);
  if (teardown) teardown = without(verifyCheckpoint(teardown), ['checkpointSha256']);
  else teardown = clone(scheduledTeardown);
  const snapshot = () => ({
    schemaVersion: 'agent-browser.p158-live-resumed-controller.v1',
    state,
    runId: manifest.runId,
    candidate: clone(manifest.candidate),
    results: clone(results),
    evidence: { artifacts: clone(artifacts) },
    scheduledTeardown: clone(teardown),
  });
  return {
    snapshot,
    async startExecution() {
      if (state !== 'frozen') fail('controller_state_invalid', `Cannot start from ${state}`);
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
      const receipt = { artifactId, relativePath, sha256: sha256(bytes), byteCount: Buffer.byteLength(bytes), metadata: clone(metadata) };
      const attemptId = artifactId.split(':').at(-2) ?? sha256(artifactId);
      const receiptPath = `${STATE_ROOT}/artifacts/${attemptId}.json`;
      const prior = await readOptionalJson(store, receiptPath);
      if (prior) {
        verifyCheckpoint(prior);
        if (sha256(without(prior, ['checkpointSha256'])) !== sha256(receipt)) fail('artifact_changed_across_resume', `${artifactId} changed across resume`);
      } else await writeCheckpoint(store, receiptPath, receipt);
      if (!artifacts.some((entry) => entry.artifactId === artifactId)) artifacts.push(receipt);
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
      if (!results.some((entry) => entry.attemptId === result.attemptId)) results.push(clone(result));
    },
    async recordScheduledTeardown(result) {
      if (state !== 'executing') fail('controller_state_invalid', `Cannot record teardown from ${state}`);
      const path = `${STATE_ROOT}/scheduled-teardown.json`;
      const prior = await readOptionalJson(store, path);
      if (prior) fail('terminal_result_changed', 'Scheduled teardown is already terminal');
      teardown = { ...clone(scheduledTeardown), ...clone(result) };
      await writeCheckpoint(store, path, teardown);
    },
    async finishExecution() {
      if (state !== 'executing') fail('controller_state_invalid', `Cannot finish from ${state}`);
      const missing = schedule.attempts.filter((attempt) => !results.some((result) => result.attemptId === attempt.attemptId));
      if (missing.length > 0 || !teardown?.resultState) fail('execution_not_terminal', 'All attempts and scheduled teardown must be terminal');
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
        events: [], artifacts: clone(artifacts), eventHeadSha256: null,
      };
      await store.writeOnce('artifacts/manifest/sealed-evidence-manifest.json', canonicalJson(body));
      await writeCheckpoint(store, evidenceSealPath, { state: 'evidence_sealed', manifestSha256: sha256(body), sealedAt: clock.wallNow() });
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
      store, runRoot: descriptor.runRoot, manifest, schedule,
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
