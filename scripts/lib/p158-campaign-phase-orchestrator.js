import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { canonicalJson, createFileArtifactStore, sha256 } from './p158-campaign-controller.js';
import { runP158W9Phase } from './p158-w9-campaign-orchestrator.js';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const PRE_PHASES = Object.freeze(['W7', 'W8']);

export class P158CampaignPhaseError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'P158CampaignPhaseError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158CampaignPhaseError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

async function readOptional(store, path) {
  try {
    const value = await store.read(path);
    return value === undefined ? null : JSON.parse(value.toString('utf8'));
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function writeCheckpoint(store, path, value) {
  const body = { ...clone(value), checkpointSha256: sha256(value) };
  await store.writeOnce(path, canonicalJson(body));
  return body;
}

function verifyCheckpoint(value) {
  const body = Object.fromEntries(Object.entries(value ?? {}).filter(([key]) => key !== 'checkpointSha256'));
  if (!value || value.checkpointSha256 !== sha256(body)) {
    fail('phase_checkpoint_integrity_mismatch', 'A phase checkpoint is missing or changed');
  }
  return value;
}

function validateBundle({ phaseId, bundle, schedule, liveHookManifestSha256 }) {
  const adapters = phaseId === 'W7' ? bundle?.w7Adapters : bundle?.w8Adapters;
  const expectedCases = schedule.caseContracts.filter((entry) => entry.phaseId === phaseId).map((entry) => entry.caseId);
  if (!Array.isArray(adapters) || !Array.isArray(bundle?.adapterBindings) ||
      adapters.length !== expectedCases.length || bundle.adapterBindings.length !== expectedCases.length) {
    fail('phase_adapter_matrix_incomplete', `${phaseId} adapter matrix is incomplete`);
  }
  const byCase = new Map(adapters.map((adapter) => [adapter.caseId, adapter]));
  const bindingByCase = new Map(bundle.adapterBindings.map((binding) => [binding.caseId, binding]));
  for (const caseId of expectedCases) {
    const adapter = byCase.get(caseId);
    const binding = bindingByCase.get(caseId);
    if (!adapter || !binding || adapter.executionMode !== binding.mode || adapter.providerFree !== false ||
        adapter.effectsAllowed !== binding.effectsAllowed || adapter.sourcePath !== binding.sourcePath ||
        adapter.sourceSha256 !== binding.sourceSha256 || adapter.liveHookManifestSha256 !== liveHookManifestSha256 ||
        adapter.liveBindingSha256 !== sha256(binding) ||
        JSON.stringify(adapter.liveHookIds) !== JSON.stringify(binding.hookIds) ||
        (binding.mode === 'concrete_live' && (binding.effectsAllowed !== true || adapter.blocker !== null)) ||
        (binding.mode === 'explicit_blocked' && (binding.effectsAllowed !== false || !adapter.blocker))) {
      fail('phase_adapter_binding_unproven', `${phaseId}/${caseId} is not bound to its exact executable adapter`);
    }
  }
  return { adapters: byCase, bindings: bindingByCase, expectedCases };
}

function correlationIds(runId, phaseId, attemptId, effectId = null, ordinal = null) {
  return {
    requestId: `p158:${runId}:${attemptId}:request`,
    eventId: `p158:${runId}:${attemptId}:${effectId ?? 'terminal'}:${ordinal ?? 0}`,
    traceId: `p158:${runId}:${phaseId}`,
  };
}

function terminalResult({ attempt, result, phaseId, runId, checkpointPath }) {
  const body = {
    attemptId: attempt.attemptId,
    caseId: attempt.caseId,
    phaseId,
    resultState: result.resultState,
    effectState: result.effectState ?? (result.resultState === 'skipped_blocked' ? 'not_started' : 'verified_effect'),
    retryDisposition: ['not_applicable', 'prohibited_opportunistic_retry', 'predetermined_distinct_attempt']
      .includes(result.retryDisposition) ? result.retryDisposition : 'prohibited_opportunistic_retry',
    completedAt: result.completedAt,
    causalIds: correlationIds(runId, phaseId, attempt.attemptId),
    evidence: {
      phaseCheckpointPath: checkpointPath,
      artifactIds: [`p158:${runId}:${phaseId}:${attempt.attemptId}:terminal`],
      resultSha256: sha256(result),
    },
    ...(result.blocker ? { blocker: clone(result.blocker) } : {}),
    ...(result.resultState === 'skipped_blocked' ? { requestedEffects: [] } : {}),
  };
  return body;
}

export function buildP158CampaignPhasePreparation({ schedule, w7Bundle, w8Bundle, liveHookManifestSha256, runId }) {
  if (typeof runId !== 'string' || runId.length === 0) {
    fail('phase_run_id_missing', 'Preparation requires the exact frozen campaign run ID');
  }
  const bundles = {
    W7: validateBundle({ phaseId: 'W7', bundle: w7Bundle, schedule, liveHookManifestSha256 }),
    W8: validateBundle({ phaseId: 'W8', bundle: w8Bundle, schedule, liveHookManifestSha256 }),
  };
  const preExecutionBlockers = PRE_PHASES.flatMap((phaseId) => schedule.attempts
    .filter((attempt) => attempt.phaseId === phaseId)
    .flatMap((attempt) => {
      const binding = bundles[phaseId].bindings.get(attempt.caseId);
      const adapter = bundles[phaseId].adapters.get(attempt.caseId);
      return binding.mode === 'explicit_blocked' ? [{
        phaseId,
        caseId: attempt.caseId,
        attemptId: attempt.attemptId,
        blocker: clone(adapter.blocker),
        bindingSha256: sha256(binding),
      }] : [];
    }));
  const loggingExpectations = schedule.attempts.map((attempt) => ({
    attemptId: attempt.attemptId,
    requestId: `p158:${runId}:${attempt.attemptId}:request`,
    incidentExpected: false,
    operatorVisible: attempt.externalIngressRequired === true,
    expectedSurfaceRoles: [
      'ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome',
      ...(attempt.externalIngressRequired ? ['dashboard_projection'] : []),
    ],
  }));
  const body = {
    schemaVersion: 'agent-browser.p158-campaign-phase-preparation.v1',
    scheduleSha256: schedule.scheduleSha256,
    runId,
    liveHookManifestSha256,
    preExecutionBlockers,
    loggingExpectations,
  };
  return Object.freeze({ ...body, preparationSha256: sha256(body) });
}

export function applyP158PhasePreparationToControllerSchedule({ controllerSchedule, phasePreparation }) {
  const { preparationSha256, ...body } = phasePreparation ?? {};
  if (preparationSha256 !== sha256(body) || !Array.isArray(controllerSchedule)) {
    fail('phase_preparation_unproven', 'Controller schedule requires an intact phase preparation seal');
  }
  const blockers = new Map(phasePreparation.preExecutionBlockers.map((entry) => [entry.attemptId, entry.blocker]));
  const known = new Set(controllerSchedule.map((attempt) => attempt.attemptId));
  if ([...blockers.keys()].some((attemptId) => !known.has(attemptId))) {
    fail('phase_preparation_attempt_unknown', 'A pre-execution blocker names an unknown controller attempt');
  }
  return controllerSchedule.map((attempt) => ({
    ...clone(attempt),
    preExecutionBlocker: clone(blockers.get(attempt.attemptId) ?? attempt.preExecutionBlocker ?? null),
  }));
}

export async function runP158CampaignPhases({
  schedule, controller, w7Bundle, w8Bundle, w9, runRoot, artifactStore,
  liveHookManifestSha256, clock = { wallNow: () => new Date().toISOString() },
  phasePreparation,
  runW9 = runP158W9Phase,
}) {
  if (!isAbsolute(runRoot ?? '')) fail('phase_run_root_invalid', 'Phase runtime root must be absolute');
  const fromRepo = relative(REPO_ROOT, resolve(runRoot));
  if (fromRepo === '' || (!fromRepo.startsWith('..') && !isAbsolute(fromRepo))) {
    fail('phase_run_root_inside_repository', 'Phase evidence must remain outside the repository');
  }
  if (!/^[a-f0-9]{64}$/u.test(liveHookManifestSha256 ?? '')) {
    fail('live_hook_manifest_unproven', 'The frozen live-hook manifest digest is required');
  }
  const bundles = {
    W7: validateBundle({ phaseId: 'W7', bundle: w7Bundle, schedule, liveHookManifestSha256 }),
    W8: validateBundle({ phaseId: 'W8', bundle: w8Bundle, schedule, liveHookManifestSha256 }),
  };
  const expectedPreparation = buildP158CampaignPhasePreparation({
    schedule, w7Bundle, w8Bundle, liveHookManifestSha256, runId: w9.target.runId,
  });
  if (sha256(phasePreparation) !== sha256(expectedPreparation)) {
    fail('phase_preparation_unproven', 'Execution requires the exact preparation-time blocker and logging declaration');
  }
  const blockers = phasePreparation.preExecutionBlockers.map((entry) => clone(entry));
  const store = artifactStore ?? createFileArtifactStore(runRoot);
  const sourceDigest = sha256({ scheduleSha256: schedule.scheduleSha256, liveHookManifestSha256,
    bindings: PRE_PHASES.flatMap((phaseId) => [...bundles[phaseId].bindings.values()]) });
  const blockersPath = 'campaign-phases/pre-execution-blockers.json';
  let blockerCheckpoint = await readOptional(store, blockersPath);
  if (!blockerCheckpoint) {
    blockerCheckpoint = await writeCheckpoint(store, blockersPath, {
      schemaVersion: 'agent-browser.p158-pre-execution-blockers.v1', sourceDigest,
      blockerCount: blockers.length, blockers, loggingExpectationsSha256: sha256(phasePreparation.loggingExpectations),
      phasePreparationSha256: phasePreparation.preparationSha256, recordedAt: clock.wallNow(),
    });
  } else {
    verifyCheckpoint(blockerCheckpoint);
    if (blockerCheckpoint.sourceDigest !== sourceDigest || sha256(blockerCheckpoint.blockers) !== sha256(blockers)) {
      fail('pre_execution_blockers_changed', 'Pre-execution blockers changed across resume');
    }
  }
  if (controller.snapshot().state === 'frozen') await controller.startExecution();
  if (controller.snapshot().state !== 'executing') fail('controller_state_invalid', 'Campaign must be executing');

  for (const phaseId of PRE_PHASES) {
    const bundle = phaseId === 'W7' ? w7Bundle : w8Bundle;
    for (const attempt of schedule.attempts.filter((entry) => entry.phaseId === phaseId)) {
      const terminalPath = `campaign-phases/${phaseId}/attempts-terminal/${attempt.attemptId}.json`;
      let terminal = await readOptional(store, terminalPath);
      if (!terminal) {
        const startedPath = `campaign-phases/${phaseId}/attempts-started/${attempt.attemptId}.json`;
        const started = await readOptional(store, startedPath);
        if (started) {
          verifyCheckpoint(started);
          terminal = await writeCheckpoint(store, terminalPath, terminalResult({
            attempt, phaseId, runId: controller.snapshot().runId ?? w9.target.runId, checkpointPath: terminalPath,
            result: { resultState: 'harness_failure', effectState: 'effect_uncertain',
              retryDisposition: 'prohibited_opportunistic_retry', completedAt: clock.wallNow() },
          }));
        } else {
          await writeCheckpoint(store, startedPath, {
            state: 'started', sourceDigest, correlationIds: correlationIds(
              controller.snapshot().runId ?? w9.target.runId, phaseId, attempt.attemptId,
            ), observedAt: clock.wallNow(),
          });
          const adapter = bundles[phaseId].adapters.get(attempt.caseId);
          const binding = bundles[phaseId].bindings.get(attempt.caseId);
          let effectOrdinal = 0;
          const preparedBlocker = phasePreparation.preExecutionBlockers
            .find((entry) => entry.attemptId === attempt.attemptId);
          const result = binding.mode === 'explicit_blocked' ? {
            resultState: 'skipped_blocked',
            effectState: 'not_started',
            retryDisposition: 'prohibited_opportunistic_retry',
            requestedEffects: [],
            blocker: clone(preparedBlocker?.blocker),
            repairAttempted: false,
            retryAttempted: false,
            garbageCollectionAttempted: false,
          } : await adapter.execute({
            attempt: clone(attempt),
            requestEffect: async (effectId, payload) => {
              if (binding.effectsAllowed !== true || !attempt.declaredEffectIds.includes(effectId) ||
                  typeof bundle.effects?.[effectId] !== 'function') {
                fail('undeclared_phase_effect', `${attempt.attemptId} requested ${effectId}`);
              }
              effectOrdinal += 1;
              const effectPath = `campaign-phases/${phaseId}/effects/${attempt.attemptId}/${String(effectOrdinal).padStart(6, '0')}.json`;
              await writeCheckpoint(store, `${effectPath}.started`, {
                state: 'started', sourceDigest, correlationIds: correlationIds(
                  controller.snapshot().runId ?? w9.target.runId, phaseId, attempt.attemptId, effectId, effectOrdinal,
                ), payloadSha256: sha256(payload), observedAt: clock.wallNow(),
              });
              const value = await bundle.effects[effectId](clone(payload));
              await writeCheckpoint(store, `${effectPath}.terminal`, {
                state: 'terminal', resultSha256: sha256(value), artifactIds: value?.artifactIds ?? [],
                observedAt: clock.wallNow(),
              });
              return value;
            },
          });
          const normalizedResult = {
            ...clone(result),
            effectState: result.effectState === 'completed' ? 'verified_effect' : result.effectState,
            completedAt: clock.wallNow(),
          };
          terminal = await writeCheckpoint(store, terminalPath, terminalResult({
            attempt, result: normalizedResult, phaseId,
            runId: controller.snapshot().runId ?? w9.target.runId, checkpointPath: terminalPath,
          }));
        }
      } else verifyCheckpoint(terminal);
      if (!controller.snapshot().results.some((entry) => entry.attemptId === attempt.attemptId)) {
        const attemptResult = Object.fromEntries(Object.entries(terminal)
          .filter(([key]) => key !== 'checkpointSha256'));
        const artifactId = attemptResult.evidence.artifactIds[0];
        const knownArtifacts = new Set((controller.snapshot().evidence?.artifacts ?? [])
          .map((entry) => entry.artifactId));
        if (!knownArtifacts.has(artifactId)) {
          if (typeof controller.writeArtifact !== 'function') {
            fail('controller_artifact_writer_missing', 'Campaign-bound terminal evidence must use the controller writer');
          }
          await controller.writeArtifact({
            artifactId,
            relativePath: `campaign-phases/${phaseId}/controller-evidence/${attempt.attemptId}.json`,
            content: canonicalJson(attemptResult),
            metadata: { capturePurpose: 'phase_attempt_terminal' },
          });
        }
        await controller.recordAttempt(attemptResult);
      }
    }
  }
  const expectedPreAttempts = schedule.attempts.filter((entry) => PRE_PHASES.includes(entry.phaseId));
  const recordedIds = new Set(controller.snapshot().results.map((entry) => entry.attemptId));
  const missing = expectedPreAttempts.filter((entry) => !recordedIds.has(entry.attemptId)).map((entry) => entry.attemptId);
  if (missing.length > 0) fail('pre_execution_not_terminal', 'W9 cannot start before W7/W8 terminal closure', { missing });
  const w9Result = await runW9({ ...w9, schedule, controller });
  return {
    state: controller.snapshot().state,
    sourceDigest,
    preExecutionBlockers: blockers,
    loggingExpectations: clone(phasePreparation.loggingExpectations),
    preExecutionBlockersSha256: blockerCheckpoint.checkpointSha256,
    terminalPreAttemptCount: expectedPreAttempts.length,
    w9: clone(w9Result),
  };
}
