import { createHash } from 'node:crypto';
import { link, mkdir, open, unlink } from 'node:fs/promises';
import { dirname, isAbsolute, join, normalize, relative, sep } from 'node:path';

export const CONTROLLER_STATES = Object.freeze([
  'prepared',
  'frozen',
  'executing',
  'execution_terminal',
  'evidence_sealed',
  'analyzed',
]);

export const RESULT_STATES = Object.freeze([
  'passed',
  'reproduced_historical_failure',
  'new_product_failure',
  'harness_failure',
  'inconclusive',
  'skipped_blocked',
  'safety_stopped',
]);

const REQUIRED_CANDIDATE_FIELDS = Object.freeze([
  'runId',
  'sourceCommit',
  'binarySha256',
  'dashboardSha256',
  'installedGenerationId',
  'browserExecutableSha256',
  'runtimeManifestRevision',
  'providerConfigurationRevision',
  'externalIngressDeploymentRevision',
  'aggregateFixtureManifestSha256',
  'preparedAt',
]);

const RESOURCE_SAMPLE_FIELDS = Object.freeze([
  ['availableMemoryBytes', 'minimumAvailableMemoryBytes', 'minimum'],
  [
    'availableMemoryPlusFreeSwapBytes',
    'minimumAvailableMemoryPlusFreeSwapBytes',
    'minimum',
  ],
  ['artifactBytes', 'artifactQuotaBytes', 'maximum'],
  ['filesystemUsedPercent', 'filesystemMaximumUsedPercent', 'maximum'],
  ['campaignProcessCount', 'campaignProcessCount', 'maximum'],
  ['chromeProcessCount', 'chromeProcessCount', 'maximum'],
  ['xvfbProcessCount', 'xvfbProcessCount', 'maximum'],
  ['allocatedDisplayCount', 'allocatedDisplayCount', 'maximum'],
  ['allocatedRouteCount', 'allocatedRouteCount', 'maximum'],
  ['externalConnectionCount', 'externalConnectionCount', 'maximum'],
  ['unresolvedJobCount', 'unresolvedJobCount', 'maximum'],
]);

const IMMEDIATE_SAFETY_FIELDS = Object.freeze([
  'productionStateObserved',
  'syntheticCanaryLeaked',
  'externalTrafficEscapedAllowlist',
  'disposableTargetIdentityUncertain',
  'evidenceCorruptionRisk',
]);

export class CampaignControllerError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'CampaignControllerError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new CampaignControllerError(code.toLowerCase(), message, details);
}

function canonicalize(value, seen = new Set()) {
  if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) fail('INVALID_CANONICAL_VALUE', 'Non-finite numbers cannot be hashed');
    return value;
  }
  if (typeof value === 'bigint') return value.toString();
  if (value instanceof Uint8Array) return { $bytes: Buffer.from(value).toString('base64') };
  if (typeof value !== 'object') {
    fail('INVALID_CANONICAL_VALUE', `Cannot canonicalize a ${typeof value} value`);
  }
  if (seen.has(value)) fail('INVALID_CANONICAL_VALUE', 'Circular values cannot be hashed');
  seen.add(value);
  let result;
  if (Array.isArray(value)) {
    result = value.map((item) => canonicalize(item, seen));
  } else {
    result = {};
    for (const key of Object.keys(value).sort()) {
      if (value[key] !== undefined) result[key] = canonicalize(value[key], seen);
    }
  }
  seen.delete(value);
  return result;
}

export function canonicalJson(value) {
  return `${JSON.stringify(canonicalize(value))}\n`;
}

export function sha256(value) {
  const bytes =
    typeof value === 'string' || value instanceof Uint8Array ? value : canonicalJson(value);
  return createHash('sha256').update(bytes).digest('hex');
}

function clone(value) {
  return structuredClone(value);
}

function candidateIdentityDigest(candidate) {
  const { candidateSha256: _declaredDigest, ...identity } = candidate ?? {};
  return sha256(identity);
}

function assertRelativePath(path) {
  if (typeof path !== 'string' || path.length === 0 || path.includes('\0') || isAbsolute(path)) {
    fail('INVALID_ARTIFACT_PATH', 'Artifact paths must be non-empty relative paths', { path });
  }
  const normalized = normalize(path);
  if (normalized === '..' || normalized.startsWith(`..${sep}`)) {
    fail('INVALID_ARTIFACT_PATH', 'Artifact paths cannot escape the campaign root', { path });
  }
  return normalized;
}

export function createFileArtifactStore(runRoot) {
  if (typeof runRoot !== 'string' || runRoot.length === 0) {
    fail('INVALID_RUN_ROOT', 'runRoot is required for the filesystem artifact store');
  }
  let temporarySequence = 0;
  return {
    runRoot,
    async writeOnce(relativePath, content) {
      const safePath = assertRelativePath(relativePath);
      const target = join(runRoot, safePath);
      if (relative(runRoot, target).startsWith('..')) {
        fail('INVALID_ARTIFACT_PATH', 'Artifact path escaped the campaign root', { relativePath });
      }
      await mkdir(dirname(target), { recursive: true });
      const temporary = join(
        dirname(target),
        `.${target.slice(target.lastIndexOf(sep) + 1)}.${process.pid}.${temporarySequence++}.tmp`,
      );
      const handle = await open(temporary, 'wx', 0o600);
      try {
        await handle.writeFile(content);
        await handle.sync();
      } finally {
        await handle.close();
      }
      try {
        await link(temporary, target);
      } catch (error) {
        await unlink(temporary).catch(() => {});
        if (error?.code === 'EEXIST') {
          fail('ARTIFACT_ALREADY_EXISTS', 'Append-only artifact already exists', { relativePath });
        }
        throw error;
      }
      await unlink(temporary);
      return { path: target, relativePath: safePath, byteCount: Buffer.byteLength(content) };
    },
    async read(relativePath) {
      const { readFile } = await import('node:fs/promises');
      return readFile(join(runRoot, assertRelativePath(relativePath)));
    },
  };
}

export function createMemoryArtifactStore() {
  const files = new Map();
  return {
    async writeOnce(relativePath, content) {
      const safePath = assertRelativePath(relativePath);
      if (files.has(safePath)) {
        fail('ARTIFACT_ALREADY_EXISTS', 'Append-only artifact already exists', { relativePath });
      }
      const bytes = Buffer.from(content);
      files.set(safePath, bytes);
      return { relativePath: safePath, byteCount: bytes.length };
    },
    read(relativePath) {
      const value = files.get(assertRelativePath(relativePath));
      return value ? Buffer.from(value) : undefined;
    },
    paths() {
      return [...files.keys()].sort();
    },
  };
}

function normalizeClock(clock = {}) {
  return {
    wallNow: clock.wallNow ?? clock.now ?? (() => new Date().toISOString()),
    monotonicNow: clock.monotonicNow ?? (() => Number(process.hrtime.bigint())),
  };
}

export class AppendOnlyEvidenceWriter {
  constructor({ store, runId, clock }) {
    this.store = store;
    this.runId = runId;
    this.clock = normalizeClock(clock);
    this.events = [];
    this.artifacts = [];
    this.eventIds = new Set();
    this.artifactIds = new Set();
    this.parentEventSha256 = null;
    this.manifestSha256 = null;
    this.queue = Promise.resolve();
  }

  #serialized(operation) {
    const result = this.queue.then(operation, operation);
    this.queue = result.catch(() => {});
    return result;
  }

  appendEvent(
    recordType,
    payload = {},
    { controllerState, artifacts = [], wallTime, monotonicTimeNanoseconds } = {},
  ) {
    return this.#serialized(async () => {
      const sequence = this.events.length;
      const recordId = `${this.runId}:record:${String(sequence).padStart(8, '0')}`;
      if (this.eventIds.has(recordId)) fail('DUPLICATE_EVENT_ID', 'Record ID is not unique', { recordId });
      const record = {
        schemaVersion: 'agent-browser.p158-campaign-result.v1',
        planId: 'P158',
        runId: this.runId,
        recordId,
        sequence,
        previousRecordSha256: this.parentEventSha256,
        recordType,
        controllerState,
        wallTime: wallTime ?? this.clock.wallNow(),
        monotonicTimeNanoseconds: monotonicTimeNanoseconds ?? this.clock.monotonicNow(),
        clockOffsetMilliseconds: 0,
        payload: clone(payload),
        artifacts: clone(artifacts.map(ledgerArtifact)),
      };
      if (this.manifestSha256) record.manifestSha256 = this.manifestSha256;
      const content = canonicalJson(record);
      const digest = sha256(content);
      await this.store.writeOnce(
        `ledger/${String(sequence).padStart(8, '0')}-${recordType}.json`,
        content,
      );
      const compatibilityTypes = {
        controller_transition: 'controller-state-transition',
        attempt_terminal: 'attempt-terminal',
        safety_observation: 'safety-observation',
        scheduled_teardown_terminal: 'scheduled-teardown-terminal',
        evidence_seal: 'evidence-seal',
        analysis_terminal: 'analysis-terminal',
        artifact_recorded: 'artifact-recorded',
      };
      const receipt = {
        ...record,
        type: compatibilityTypes[recordType] ?? recordType,
        sha256: digest,
        byteCount: Buffer.byteLength(content),
      };
      this.events.push(receipt);
      this.eventIds.add(recordId);
      this.parentEventSha256 = digest;
      return clone(receipt);
    });
  }

  writeArtifact({ artifactId, relativePath, content, metadata = {} }) {
    return this.#serialized(async () => {
      if (typeof artifactId !== 'string' || artifactId.length === 0) {
        fail('INVALID_ARTIFACT_ID', 'artifactId is required');
      }
      if (this.artifactIds.has(artifactId)) {
        fail('DUPLICATE_ARTIFACT_ID', 'Artifact IDs are append-only and unique', { artifactId });
      }
      const bytes =
        typeof content === 'string' || content instanceof Uint8Array
          ? content
          : canonicalJson(content);
      const safePath = assertRelativePath(relativePath);
      const storagePath = safePath.startsWith(`artifacts${sep}`) ? safePath : `artifacts/${safePath}`;
      const digest = sha256(bytes);
      const stored = await this.store.writeOnce(storagePath, bytes);
      const previousArtifact = this.artifacts.at(-1);
      const receipt = {
        artifactId,
        relativePath: stored.relativePath,
        path: stored.path,
        mediaType: metadata.mediaType ?? 'application/octet-stream',
        sha256: digest,
        byteCount: Buffer.byteLength(bytes),
        captureState: metadata.captureState ?? 'complete',
        captureGap: metadata.captureGap ?? null,
        redactions: clone(metadata.redactions ?? []),
        parentArtifactSha256s: clone(
          metadata.parentArtifactSha256s ?? (previousArtifact ? [previousArtifact.sha256] : []),
        ),
      };
      this.artifacts.push(receipt);
      this.artifactIds.add(artifactId);
      return clone(receipt);
    });
  }

  snapshot() {
    return clone({
      events: this.events,
      artifacts: this.artifacts,
      eventHeadSha256: this.parentEventSha256,
    });
  }
}

function deriveAttemptSeed(runSeed, caseId, attemptId, environmentId) {
  return Number.parseInt(
    sha256(`${runSeed}\0${caseId}\0${attemptId}\0${environmentId}`).slice(0, 13),
    16,
  );
}

function topologicalCaseOrder(registry) {
  const cases = new Map((registry.cases ?? []).map((entry) => [entry.id, entry]));
  const pending = new Set(cases.keys());
  const ordered = [];
  while (pending.size > 0) {
    const ready = [...pending]
      .filter((id) => (cases.get(id).dependsOn ?? []).every((dependency) => !pending.has(dependency)))
      .sort();
    if (ready.length === 0) {
      fail('CYCLIC_CASE_DEPENDENCY', 'Registry cases contain a dependency cycle', {
        pending: [...pending].sort(),
      });
    }
    for (const id of ready) {
      pending.delete(id);
      ordered.push(id);
    }
  }
  return ordered;
}

export function buildDeterministicSchedule({ registry, schedule, seed }) {
  if (!Array.isArray(schedule)) {
    fail('INVALID_SCHEDULE', 'prepare requires a frozen attempt schedule');
  }
  const registryCases = new Map((registry.cases ?? []).map((entry) => [entry.id, entry]));
  const caseOrder = new Map(topologicalCaseOrder(registry).map((id, index) => [id, index]));
  const attemptIds = new Set();
  const normalized = schedule.map((attempt, suppliedIndex) => {
    if (!registryCases.has(attempt.caseId)) {
      fail('UNKNOWN_CASE', 'Scheduled case is absent from the frozen registry', {
        caseId: attempt.caseId,
      });
    }
    if (typeof attempt.attemptId !== 'string' || attempt.attemptId.length === 0) {
      fail('INVALID_ATTEMPT_ID', 'Every scheduled attempt requires an attemptId', { attempt });
    }
    if (attemptIds.has(attempt.attemptId)) {
      fail('DUPLICATE_ATTEMPT_ID', 'Attempt IDs must be globally unique', {
        attemptId: attempt.attemptId,
      });
    }
    attemptIds.add(attempt.attemptId);
    if (!(registryCases.get(attempt.caseId).environmentIds ?? []).includes(attempt.environmentId)) {
      fail('INVALID_ATTEMPT_ENVIRONMENT', 'Attempt environment is not declared by its case', {
        caseId: attempt.caseId,
        environmentId: attempt.environmentId,
      });
    }
    return {
      caseId: attempt.caseId,
      attemptId: attempt.attemptId,
      environmentId: attempt.environmentId,
      seed:
        attempt.seed ?? deriveAttemptSeed(seed, attempt.caseId, attempt.attemptId, attempt.environmentId),
      dependsOn: [...new Set(attempt.dependsOn ?? registryCases.get(attempt.caseId).dependsOn ?? [])].sort(),
      suppliedIndex,
    };
  });
  const caseIds = new Set(registryCases.keys());
  for (const attempt of normalized) {
    for (const dependency of attempt.dependsOn) {
      if (!caseIds.has(dependency) && !attemptIds.has(dependency)) {
        fail('UNKNOWN_ATTEMPT_DEPENDENCY', 'Schedule dependency is neither a case nor an attempt', {
          attemptId: attempt.attemptId,
          dependency,
        });
      }
    }
  }
  normalized.sort(
    (left, right) =>
      caseOrder.get(left.caseId) - caseOrder.get(right.caseId) ||
      left.caseId.localeCompare(right.caseId) ||
      left.environmentId.localeCompare(right.environmentId) ||
      left.attemptId.localeCompare(right.attemptId) ||
      left.suppliedIndex - right.suppliedIndex,
  );
  const repetitions = new Map();
  const ordered = normalized.map(({ suppliedIndex: _suppliedIndex, ...attempt }, scheduleIndex) => {
    const repetitionKey = `${attempt.caseId}\0${attempt.environmentId}`;
    const repetition = (repetitions.get(repetitionKey) ?? 0) + 1;
    repetitions.set(repetitionKey, repetition);
    return {
      ...attempt,
      scheduleIndex,
      scheduleSequence: scheduleIndex,
      scheduleId: `${attempt.caseId}:${attempt.attemptId}`,
      repetition,
      environmentIds: [attempt.environmentId],
      preconditionIds: [],
      stimuli: [],
      evidenceProfile: registryCases.get(attempt.caseId).evidenceProfile,
      externalIngressRequired:
        attempt.environmentId === 'E2' &&
        (/^[HDC]/.test(attempt.caseId) || ['X06', 'X10'].includes(attempt.caseId)),
    };
  });
  for (const attempt of ordered) {
    attempt.dependsOnAttemptIds = attempt.dependsOn.flatMap((dependency) => {
      const direct = ordered.find((candidate) => candidate.attemptId === dependency);
      if (direct) return [direct.attemptId];
      return ordered
        .filter((candidate) => candidate.caseId === dependency)
        .map((candidate) => candidate.attemptId);
    });
  }
  return ordered;
}

function manifestScheduleAttempt(attempt) {
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
  };
}

function buildSafetyPolicy(resourceCeilings) {
  return {
    sampleIntervalMilliseconds: resourceCeilings.sampleIntervalMilliseconds,
    consecutiveViolationsBeforeStop:
      resourceCeilings.consecutiveResourceViolationsBeforeStop,
    rules: RESOURCE_SAMPLE_FIELDS.map(([metric, limitField, direction]) => ({
      ruleId: `resource:${metric}`,
      metric,
      comparison: direction === 'minimum' ? 'less_than' : 'greater_than',
      threshold: resourceCeilings[limitField],
    })).filter((rule) => rule.threshold !== undefined),
    stopAction: 'stop_load_generator_only',
  };
}

function ledgerArtifact(artifact) {
  const {
    artifactId,
    relativePath,
    mediaType,
    sha256: digest,
    byteCount,
    captureState,
    captureGap,
    redactions,
    parentArtifactSha256s,
  } = artifact;
  const result = {
    artifactId,
    relativePath,
    mediaType,
    sha256: digest,
    byteCount,
    captureState,
    redactions,
    parentArtifactSha256s,
  };
  if (captureState === 'partial' || captureState === 'missing') result.captureGap = captureGap;
  return result;
}

function attemptIdentity(attempt) {
  return {
    scheduleId: attempt.scheduleId,
    caseId: attempt.caseId,
    attemptId: attempt.attemptId,
    repetition: attempt.repetition,
    seed: attempt.seed,
    environmentIds: attempt.environmentIds,
  };
}

function assertCandidate(candidate, registry, runId) {
  const required = registry.candidateManifestRequiredFields ?? REQUIRED_CANDIDATE_FIELDS;
  const missing = required.filter((field) => candidate?.[field] === undefined || candidate[field] === null);
  if (missing.length > 0) {
    fail('INVALID_CANDIDATE', 'Candidate manifest is missing required fields', { missing });
  }
  if (candidate.runId !== runId) {
    fail('CANDIDATE_RUN_MISMATCH', 'Candidate runId must match the controller runId', {
      expected: runId,
      actual: candidate.runId,
    });
  }
  const actualCandidateSha256 = candidateIdentityDigest(candidate);
  if (candidate.candidateSha256 !== actualCandidateSha256) {
    fail('CANDIDATE_DIGEST_MISMATCH', 'Candidate digest does not bind its canonical identity', {
      expected: actualCandidateSha256,
      actual: candidate.candidateSha256,
    });
  }
}

function normalizeTeardown(teardown, seed) {
  if (!teardown || typeof teardown.caseId !== 'string' || typeof teardown.environmentId !== 'string') {
    fail('INVALID_SCHEDULED_TEARDOWN', 'A scheduled teardown case and environment are required');
  }
  return {
    caseId: teardown.caseId,
    attemptId: teardown.attemptId ?? `${teardown.caseId}:scheduled`,
    environmentId: teardown.environmentId,
    seed:
      teardown.seed ??
      deriveAttemptSeed(seed, teardown.caseId, teardown.attemptId ?? `${teardown.caseId}:scheduled`, teardown.environmentId),
    dependsOn: [...new Set(teardown.dependsOn ?? [])].sort(),
  };
}

export class CampaignController {
  constructor({ registry, runId, seed, store, clock }) {
    if (registry?.registryState !== 'frozen') {
      fail('REGISTRY_NOT_FROZEN', 'The historical failure registry must be frozen');
    }
    if (typeof runId !== 'string' || runId.length === 0) fail('INVALID_RUN_ID', 'runId is required');
    if (seed === undefined || seed === null || seed === '') fail('INVALID_SEED', 'seed is required');
    this.registryReference = registry;
    this.registry = clone(registry);
    this.runId = runId;
    this.seed = String(seed);
    this.state = 'prepared';
    this.prepared = false;
    this.writer = new AppendOnlyEvidenceWriter({ store, runId, clock });
    this.clock = normalizeClock(clock);
    this.candidate = null;
    this.schedule = [];
    this.scheduledTeardown = null;
    this.results = new Map();
    this.safety = new Map();
    this.safetyStops = [];
    this.frozenInputReferences = null;
    this.frozenInputDigests = null;
    this.seal = null;
    this.manifest = null;
    this.artifactBindings = [];
    this.environmentSeals = [];
    this.calibration = null;
    this.fixtureSeal = null;
    this.freezeContract = null;
    this.freezeReceipt = null;
  }

  #assertState(expected) {
    if (this.state !== expected) {
      fail('INVALID_STATE_TRANSITION', `Operation requires ${expected}, current state is ${this.state}`, {
        expected,
        actual: this.state,
      });
    }
  }

  #assertFrozenInputsUnchanged() {
    if (!this.frozenInputReferences) return;
    const actual = {
      registrySha256: sha256(this.registryReference),
      candidateSha256: sha256(this.frozenInputReferences.candidate),
      scheduleSha256: sha256(this.frozenInputReferences.schedule),
      scheduledTeardownSha256: sha256(this.frozenInputReferences.scheduledTeardown),
      artifactBindingsSha256: sha256(this.frozenInputReferences.artifactBindings),
      environmentSealsSha256: sha256(this.frozenInputReferences.environmentSeals),
      calibrationSha256: sha256(this.frozenInputReferences.calibration),
      fixtureSealSha256: sha256(this.frozenInputReferences.fixtureSeal),
      freezeContractSha256: sha256(this.frozenInputReferences.freezeContract),
    };
    for (const [field, expected] of Object.entries(this.frozenInputDigests)) {
      if (actual[field] !== expected) {
        fail('FROZEN_INPUT_MUTATED', 'Source, configuration, candidate, or schedule changed after freeze', {
          field,
          expected,
          actual: actual[field],
        });
      }
    }
  }

  async #transition(expected, next, reason, details = {}) {
    this.#assertState(expected);
    this.#assertFrozenInputsUnchanged();
    await this.writer.appendEvent('controller_transition', {
      kind: 'controller_transition',
      from: expected,
      to: next,
      reason,
      terminal: next === 'analyzed',
      ...clone(details),
    }, { controllerState: next });
    this.state = next;
  }

  async prepare({
    candidate,
    schedule,
    scheduledTeardown,
    artifactBindings,
    environmentSeals,
    calibration,
    fixtureSeal,
    freezeContract,
  }) {
    this.#assertState('prepared');
    if (this.prepared) fail('ALREADY_PREPARED', 'Campaign preparation is append-only');
    assertCandidate(candidate, this.registry, this.runId);
    const normalizedSchedule = buildDeterministicSchedule({
      registry: this.registry,
      schedule,
      seed: this.seed,
    });
    const normalizedTeardown = normalizeTeardown(scheduledTeardown, this.seed);
    if (normalizedSchedule.some((attempt) => attempt.attemptId === normalizedTeardown.attemptId)) {
      fail('DUPLICATE_ATTEMPT_ID', 'Scheduled teardown attemptId collides with the case schedule', {
        attemptId: normalizedTeardown.attemptId,
      });
    }
    try {
      await this.writer.store.writeOnce(
        '.campaign-root.json',
        canonicalJson({ schemaVersion: 'agent-browser.p158-campaign-root.v1', runId: this.runId }),
      );
    } catch (error) {
      if (error instanceof CampaignControllerError && error.code === 'artifact_already_exists') {
        fail('CAMPAIGN_ROOT_EXISTS', 'Campaign root is already claimed by an immutable run', {
          runId: this.runId,
        });
      }
      throw error;
    }
    this.candidateReference = candidate;
    this.scheduleReference = schedule;
    this.scheduledTeardownReference = scheduledTeardown;
    this.artifactBindingsReference = artifactBindings;
    this.environmentSealsReference = environmentSeals;
    this.calibrationReference = calibration;
    this.fixtureSealReference = fixtureSeal;
    this.freezeContractReference = freezeContract;
    this.candidate = clone(candidate);
    this.artifactBindings = clone(artifactBindings);
    this.environmentSeals = clone(environmentSeals);
    this.calibration = clone(calibration);
    this.fixtureSeal = clone(fixtureSeal);
    this.freezeContract = clone(freezeContract);
    this.schedule = normalizedSchedule;
    this.scheduledTeardown = normalizedTeardown;
    this.manifest = {
      schemaVersion: 'agent-browser.p158-campaign-manifest.v1',
      planId: 'P158',
      runId: this.runId,
      registrySha256: sha256(this.registry),
      controllerState: 'prepared',
      candidate: clone(this.candidate),
      artifactBindings: clone(this.artifactBindings),
      environmentSeals: clone(this.environmentSeals),
      calibration: clone(this.calibration),
      fixtureSeal: clone(this.fixtureSeal),
      freezeContract: clone(this.freezeContract),
      schedule: this.schedule.map(manifestScheduleAttempt),
      freezePolicy: clone(this.registry.freezeRules),
      safetyPolicy: buildSafetyPolicy(this.registry.resourceCeilings),
      evidencePolicy: {
        stateRoot: this.writer.store.runRoot ?? 'in-memory',
        appendOnly: true,
        atomicWrites: true,
        digestAlgorithm: 'sha256',
        forbiddenCapturedFields: clone(this.registry.forbiddenCapturedFields),
      },
    };
    const manifestContent = canonicalJson(this.manifest);
    this.writer.manifestSha256 = sha256(manifestContent);
    await this.writer.store.writeOnce('campaign-manifest.json', manifestContent);
    await this.writer.appendEvent('controller_transition', {
      kind: 'controller_transition',
      from: null,
      to: 'prepared',
      reason: 'candidate, fixtures, schedule, and safety policy prepared',
      terminal: false,
    }, { controllerState: 'prepared' });
    this.prepared = true;
    return this.snapshot();
  }

  async freeze() {
    this.#assertState('prepared');
    if (!this.prepared) fail('NOT_PREPARED', 'Campaign must be prepared before freeze');
    this.frozenInputReferences = {
      candidate: this.candidateReference,
      schedule: this.scheduleReference,
      scheduledTeardown: this.scheduledTeardownReference,
      artifactBindings: this.artifactBindingsReference,
      environmentSeals: this.environmentSealsReference,
      calibration: this.calibrationReference,
      fixtureSeal: this.fixtureSealReference,
      freezeContract: this.freezeContractReference,
    };
    this.frozenInputDigests = {
      registrySha256: sha256(this.registryReference),
      candidateSha256: sha256(this.candidateReference),
      scheduleSha256: sha256(this.scheduleReference),
      scheduledTeardownSha256: sha256(this.scheduledTeardownReference),
      artifactBindingsSha256: sha256(this.artifactBindingsReference),
      environmentSealsSha256: sha256(this.environmentSealsReference),
      calibrationSha256: sha256(this.calibrationReference),
      fixtureSealSha256: sha256(this.fixtureSealReference),
      freezeContractSha256: sha256(this.freezeContractReference),
    };
    const frozenAt = this.clock.wallNow();
    const monotonicTimeNanoseconds = this.clock.monotonicNow();
    const freezeReceipt = {
      schemaVersion: 'agent-browser.p158-campaign-freeze.v1',
      planId: 'P158',
      runId: this.runId,
      freezeId: this.freezeContract.freezeId,
      controllerState: 'frozen',
      manifestSha256: this.writer.manifestSha256,
      candidateSha256: this.candidate.candidateSha256,
      artifactBindingsSha256: sha256(this.artifactBindings),
      environmentSealsSha256: sha256(this.environmentSeals),
      calibrationSha256: sha256(this.calibration),
      fixtureSealSha256: sha256(this.fixtureSeal),
      preparedLedgerHeadSha256: this.writer.parentEventSha256,
      frozenAt,
      monotonicTimeNanoseconds,
      startedCaseCount: 0,
      startedAttemptCount: 0,
    };
    const freezeContent = canonicalJson(freezeReceipt);
    await this.writer.store.writeOnce('campaign-freeze.json', freezeContent);
    this.freezeReceipt = {
      ...freezeReceipt,
      sha256: sha256(freezeContent),
      byteCount: Buffer.byteLength(freezeContent),
    };
    this.#assertState('prepared');
    this.#assertFrozenInputsUnchanged();
    await this.writer.appendEvent('controller_transition', {
      kind: 'controller_transition',
      from: 'prepared',
      to: 'frozen',
      reason: 'prepared inputs and manifest digests frozen',
      terminal: false,
    }, { controllerState: 'frozen', wallTime: frozenAt, monotonicTimeNanoseconds });
    this.state = 'frozen';
    return this.snapshot();
  }

  async startExecution() {
    await this.#transition('frozen', 'executing', 'frozen deterministic schedule started');
    return this.snapshot();
  }

  #dependencyStatus(attempt) {
    const blockedBy = [];
    const incomplete = [];
    for (const dependency of attempt.dependsOn) {
      const dependencyAttempts = this.schedule.filter(
        (candidate) => candidate.caseId === dependency || candidate.attemptId === dependency,
      );
      if (dependencyAttempts.some((candidate) => !this.results.has(candidate.attemptId))) {
        incomplete.push(dependency);
        continue;
      }
      if (
        dependencyAttempts.some((candidate) => {
          const result = this.results.get(candidate.attemptId);
          return (
            result.resultState === 'skipped_blocked' ||
            result.resultState === 'safety_stopped' ||
            result.blocksDependents === true
          );
        })
      ) {
        blockedBy.push(dependency);
      }
    }
    return { blockedBy: blockedBy.sort(), incomplete: incomplete.sort() };
  }

  async recordAttempt(attemptResult) {
    this.#assertState('executing');
    this.#assertFrozenInputsUnchanged();
    const scheduled = this.schedule.find(
      (attempt) =>
        attempt.attemptId === attemptResult.attemptId && attempt.caseId === attemptResult.caseId,
    );
    if (!scheduled) {
      fail('UNSCHEDULED_ATTEMPT', 'Opportunistic or unknown attempts are prohibited', {
        caseId: attemptResult.caseId,
        attemptId: attemptResult.attemptId,
      });
    }
    if (this.results.has(scheduled.attemptId)) {
      fail('ATTEMPT_ALREADY_TERMINAL', 'A terminal attempt cannot be retried or overwritten', {
        attemptId: scheduled.attemptId,
      });
    }
    if (!RESULT_STATES.includes(attemptResult.resultState)) {
      fail('INVALID_RESULT_STATE', 'Attempt result is not a declared terminal state', {
        resultState: attemptResult.resultState,
      });
    }
    const dependencies = this.#dependencyStatus(scheduled);
    if (dependencies.incomplete.length > 0) {
      fail('DEPENDENCY_NOT_TERMINAL', 'Attempt dependencies are not terminal', {
        attemptId: scheduled.attemptId,
        dependencies: dependencies.incomplete,
      });
    }
    if (dependencies.blockedBy.length > 0 && attemptResult.resultState !== 'skipped_blocked') {
      fail('BLOCKED_RESULT_REQUIRED', 'Lost prerequisites require an exact skipped_blocked result', {
        attemptId: scheduled.attemptId,
        blockedBy: dependencies.blockedBy,
      });
    }
    if (attemptResult.resultState === 'skipped_blocked') {
      if (dependencies.blockedBy.length === 0) {
        fail('BLOCKED_PROPAGATION_MISMATCH', 'skipped_blocked requires a lost declared prerequisite', {
          attemptId: scheduled.attemptId,
        });
      }
      const reported = [...new Set(attemptResult.blockedBy ?? dependencies.blockedBy)].sort();
      if (
        reported.length !== dependencies.blockedBy.length ||
        reported.some((dependency, index) => dependency !== dependencies.blockedBy[index])
      ) {
        fail('BLOCKED_PROPAGATION_MISMATCH', 'skipped_blocked must name the exact lost prerequisites', {
          expected: dependencies.blockedBy,
          actual: reported,
        });
      }
    }
    const blocksDependents = attemptResult.blocksDependents ?? false;
    const requestedArtifactIds = attemptResult.evidence?.artifactIds ?? [];
    const evidenceArtifacts = requestedArtifactIds
      .map((artifactId) => this.writer.artifacts.find((artifact) => artifact.artifactId === artifactId))
      .filter(Boolean);
    if (evidenceArtifacts.length !== requestedArtifactIds.length) {
      const knownArtifactIds = new Set(evidenceArtifacts.map((artifact) => artifact.artifactId));
      fail('UNKNOWN_EVIDENCE_ARTIFACT', 'Terminal evidence must reference existing append-only artifacts', {
        unknownArtifactIds: requestedArtifactIds.filter((artifactId) => !knownArtifactIds.has(artifactId)),
      });
    }
    const blockingResult = dependencies.blockedBy.length > 0
      ? this.schedule
        .flatMap((candidate) => {
          if (
            dependencies.blockedBy.includes(candidate.caseId) ||
            dependencies.blockedBy.includes(candidate.attemptId)
          ) return [this.results.get(candidate.attemptId)];
          return [];
        })
        .find((candidate) => candidate?.blocksDependents)
      : null;
    const blocker = blockingResult
      ? {
          blockedByCaseId: blockingResult.caseId,
          blockedByAttemptId: blockingResult.attemptId,
          lostPrerequisite: blockingResult.firstFailureSignature ?? blockingResult.resultState,
          observationRecordIds: [blockingResult.recordId],
          observationArtifactIds: blockingResult.evidence?.artifactIds ?? [],
        }
      : null;
    const terminalPayload = {
      kind: 'attempt_terminal',
      attempt: attemptIdentity(scheduled),
      resultState: attemptResult.resultState,
      effectState:
        attemptResult.effectState ?? (attemptResult.resultState === 'passed' ? 'verified_effect' : 'no_effect'),
      retryDisposition: attemptResult.retryDisposition ?? 'prohibited_opportunistic_retry',
      completedAt: this.clock.wallNow(),
      terminal: true,
      firstFailureSignature:
        attemptResult.firstFailureSignature ?? attemptResult.evidence?.signature ?? null,
      blocker,
      safetyStop: attemptResult.safetyStop ?? null,
      causalIds: clone(attemptResult.causalIds ?? attemptResult.evidence?.causalIds ?? {}),
    };
    const record = await this.writer.appendEvent('attempt_terminal', terminalPayload, {
      controllerState: 'executing',
      artifacts: evidenceArtifacts,
    });
    const result = {
      ...clone(attemptResult),
      caseId: scheduled.caseId,
      attemptId: scheduled.attemptId,
      environmentId: scheduled.environmentId,
      seed: scheduled.seed,
      scheduleIndex: scheduled.scheduleIndex,
      blockedBy: blocker
        ? { caseId: blocker.blockedByCaseId, attemptId: blocker.blockedByAttemptId }
        : null,
      blocksDependents,
      recordId: record.recordId,
      firstFailureSignature: terminalPayload.firstFailureSignature,
    };
    this.results.set(scheduled.attemptId, result);
    if (blocksDependents) await this.#propagateBlocked(result);
    return clone(result);
  }

  async #propagateBlocked(blockingResult) {
    for (const attempt of this.schedule) {
      if (this.results.has(attempt.attemptId)) continue;
      if (
        !attempt.dependsOn.includes(blockingResult.caseId) &&
        !attempt.dependsOn.includes(blockingResult.attemptId)
      ) continue;
      const blocker = {
        blockedByCaseId: blockingResult.caseId,
        blockedByAttemptId: blockingResult.attemptId,
        lostPrerequisite: blockingResult.firstFailureSignature ?? blockingResult.resultState,
        observationRecordIds: [blockingResult.recordId],
        observationArtifactIds: blockingResult.evidence?.artifactIds ?? [],
      };
      const payload = {
        kind: 'attempt_terminal',
        attempt: attemptIdentity(attempt),
        resultState: 'skipped_blocked',
        effectState: 'no_effect',
        retryDisposition: 'prohibited_opportunistic_retry',
        completedAt: this.clock.wallNow(),
        terminal: true,
        firstFailureSignature: null,
        blocker,
        safetyStop: null,
        causalIds: {},
      };
      const record = await this.writer.appendEvent('attempt_terminal', payload, {
        controllerState: 'executing',
      });
      const result = {
        caseId: attempt.caseId,
        attemptId: attempt.attemptId,
        environmentId: attempt.environmentId,
        seed: attempt.seed,
        scheduleIndex: attempt.scheduleIndex,
        resultState: 'skipped_blocked',
        blockedBy: { caseId: blockingResult.caseId, attemptId: blockingResult.attemptId },
        blocksDependents: true,
        recordId: record.recordId,
        firstFailureSignature: null,
      };
      this.results.set(attempt.attemptId, result);
      await this.#propagateBlocked(result);
    }
  }

  async writeArtifact({ artifactId, relativePath, content, metadata = {} }) {
    if (this.state === 'evidence_sealed' || this.state === 'analyzed') {
      fail('EVIDENCE_ALREADY_SEALED', 'No campaign evidence may be added after sealing');
    }
    this.#assertFrozenInputsUnchanged();
    const receipt = await this.writer.writeArtifact({
      artifactId,
      relativePath,
      content,
      metadata,
    });
    await this.writer.appendEvent('artifact_recorded', {
      kind: 'artifact_recorded',
      artifactId,
      capturePurpose: metadata.capturePurpose ?? 'campaign_evidence',
      terminal: false,
    }, { controllerState: this.state, artifacts: [receipt] });
    return receipt;
  }

  async observeSafety(sample) {
    this.#assertState('executing');
    this.#assertFrozenInputsUnchanged();
    if (typeof sample?.environmentId !== 'string') {
      fail('INVALID_SAFETY_SAMPLE', 'Safety samples require environmentId');
    }
    const limits = this.registry.resourceCeilings ?? {};
    const violations = [];
    for (const [sampleField, limitField, direction] of RESOURCE_SAMPLE_FIELDS) {
      if (sample[sampleField] === undefined || limits[limitField] === undefined) continue;
      const violated =
        direction === 'minimum'
          ? sample[sampleField] < limits[limitField]
          : sample[sampleField] > limits[limitField];
      if (violated) {
        violations.push({ sampleField, limitField, direction, value: sample[sampleField], limit: limits[limitField] });
      }
    }
    for (const field of IMMEDIATE_SAFETY_FIELDS) {
      if (sample[field] === true) violations.push({ sampleField: field, direction: 'must_be_false', value: true });
    }
    const previous = this.safety.get(sample.environmentId) ?? { consecutiveViolations: 0, stopped: false };
    const immediate = violations.some((violation) => violation.direction === 'must_be_false');
    const consecutiveViolations = violations.length === 0 ? 0 : previous.consecutiveViolations + 1;
    const threshold = limits.consecutiveResourceViolationsBeforeStop ?? 2;
    const triggered = previous.stopped || immediate || consecutiveViolations >= threshold;
    const observation = {
      environmentId: sample.environmentId,
      sample: clone(sample),
      violations,
      consecutiveViolations,
      triggered,
      alreadyStopped: previous.stopped,
    };
    const observedRules = [];
    for (const [sampleField, limitField, direction] of RESOURCE_SAMPLE_FIELDS) {
      if (sample[sampleField] === undefined || limits[limitField] === undefined) continue;
      observedRules.push({
        ruleId: `resource:${sampleField}`,
        metric: sampleField,
        threshold: limits[limitField],
        observedValue: sample[sampleField],
        violation:
          direction === 'minimum'
            ? sample[sampleField] < limits[limitField]
            : sample[sampleField] > limits[limitField],
      });
    }
    for (const field of IMMEDIATE_SAFETY_FIELDS) {
      if (sample[field] === undefined) continue;
      observedRules.push({
        ruleId: `isolation:${field}`,
        metric: field,
        threshold: 0,
        observedValue: sample[field] ? 1 : 0,
        violation: sample[field] === true,
      });
    }
    if (observedRules.length === 0) {
      fail('INVALID_SAFETY_SAMPLE', 'Safety sample contains no recognized metrics');
    }
    for (const rule of observedRules) {
      await this.writer.appendEvent('safety_observation', {
        kind: 'safety_observation',
        ...rule,
        consecutiveViolations: rule.violation ? consecutiveViolations : 0,
        terminal: triggered && rule.violation,
      }, { controllerState: 'executing' });
    }
    this.safety.set(sample.environmentId, { consecutiveViolations, stopped: triggered });
    if (triggered && !previous.stopped) {
      this.safetyStops.push(clone(observation));
      const primaryViolation = violations[0];
      for (const attempt of this.schedule.filter(
        (entry) => entry.environmentId === sample.environmentId && !this.results.has(entry.attemptId),
      )) {
        const safetyStop = {
          ruleId: `${primaryViolation.direction === 'must_be_false' ? 'isolation' : 'resource'}:${primaryViolation.sampleField}`,
          metric: primaryViolation.sampleField,
          threshold: primaryViolation.limit ?? 0,
          observedValue: Number(primaryViolation.value),
          consecutiveViolations,
          action: 'stop_load_generator_only',
          testedRuntimeRepaired: false,
        };
        const payload = {
          kind: 'attempt_terminal',
          attempt: attemptIdentity(attempt),
          resultState: 'safety_stopped',
          effectState: 'effect_uncertain',
          retryDisposition: 'prohibited_opportunistic_retry',
          completedAt: this.clock.wallNow(),
          terminal: true,
          firstFailureSignature: `safety_stop:${primaryViolation.sampleField}`,
          blocker: null,
          safetyStop,
          causalIds: {},
        };
        const record = await this.writer.appendEvent('attempt_terminal', payload, {
          controllerState: 'executing',
        });
        const result = {
          caseId: attempt.caseId,
          attemptId: attempt.attemptId,
          environmentId: attempt.environmentId,
          seed: attempt.seed,
          scheduleIndex: attempt.scheduleIndex,
          resultState: 'safety_stopped',
          blocksDependents: true,
          stateObservation: clone(observation),
          evidence: sample.evidence ?? null,
          safetyStop,
          recordId: record.recordId,
          firstFailureSignature: payload.firstFailureSignature,
        };
        this.results.set(attempt.attemptId, result);
      }
    }
    return clone(observation);
  }

  async recordScheduledTeardown({ resultState, evidence = null, ...details }) {
    this.#assertState('executing');
    this.#assertFrozenInputsUnchanged();
    if (this.results.has(this.scheduledTeardown.attemptId)) {
      fail('SCHEDULED_TEARDOWN_ALREADY_TERMINAL', 'Scheduled teardown cannot be retried or overwritten', {
        attemptId: this.scheduledTeardown.attemptId,
      });
    }
    const incomplete = this.schedule
      .filter((attempt) => !this.results.has(attempt.attemptId))
      .map((attempt) => attempt.attemptId);
    if (incomplete.length > 0) {
      fail('TEARDOWN_NOT_SCHEDULED_YET', 'Scheduled teardown runs only after all campaign attempts are terminal', {
        incomplete,
      });
    }
    if (!RESULT_STATES.includes(resultState)) {
      fail('INVALID_RESULT_STATE', 'Scheduled teardown result is not terminal', { resultState });
    }
    const result = {
      ...clone(details),
      ...this.scheduledTeardown,
      resultState,
      evidence: clone(evidence),
      scheduledTeardown: true,
    };
    await this.writer.appendEvent('scheduled_teardown_terminal', {
      kind: 'scheduled_teardown_terminal',
      scheduleId: this.scheduledTeardown.attemptId,
      resultState,
      effectState: details.effectState ?? (resultState === 'passed' ? 'verified_effect' : 'effect_uncertain'),
      retryDisposition: details.retryDisposition ?? 'prohibited_opportunistic_retry',
      completedAt: this.clock.wallNow(),
      terminal: true,
    }, { controllerState: 'executing' });
    this.results.set(this.scheduledTeardown.attemptId, result);
    this.scheduledTeardown = { ...this.scheduledTeardown, ...result };
    return clone(result);
  }

  async finishExecution() {
    this.#assertState('executing');
    this.#assertFrozenInputsUnchanged();
    const expected = [...this.schedule.map((attempt) => attempt.attemptId), this.scheduledTeardown.attemptId];
    const missing = expected.filter((attemptId) => !this.results.has(attemptId));
    if (missing.length > 0) {
      fail('EXECUTION_NOT_TERMINAL', 'Every scheduled attempt and teardown must be terminal', { missing });
    }
    const resultCounts = RESULT_STATES.reduce((counts, state) => {
      counts[state] = [...this.results.values()].filter((result) => result.resultState === state).length;
      return counts;
    }, {});
    await this.#transition(
      'executing',
      'execution_terminal',
      'all attempts and scheduled teardown are terminal',
      { resultCounts },
    );
    return this.snapshot();
  }

  async sealEvidence() {
    this.#assertState('execution_terminal');
    this.#assertFrozenInputsUnchanged();
    const manifest = {
      schemaVersion: 'agent-browser.p158-evidence-manifest.v1',
      runId: this.runId,
      candidateSha256: sha256(this.candidate),
      registrySha256: sha256(this.registry),
      scheduleSha256: sha256(this.schedule),
      resultsSha256: sha256([...this.results.values()]),
      ...this.writer.snapshot(),
    };
    const receipt = await this.writer.writeArtifact({
      artifactId: `${this.runId}:sealed-manifest`,
      relativePath: 'manifest/sealed-evidence-manifest.json',
      content: canonicalJson(manifest),
      metadata: {
        mediaType: 'application/json',
        capturePurpose: 'sealed_evidence_manifest',
        captureState: 'complete',
      },
    });
    const ledgerHeadSha256 = this.writer.parentEventSha256;
    await this.writer.appendEvent('evidence_seal', {
      kind: 'evidence_seal',
      manifestSha256: receipt.sha256,
      ledgerHeadSha256,
      artifactCount: this.writer.artifacts.length,
      artifactBytes: this.writer.artifacts.reduce((sum, artifact) => sum + artifact.byteCount, 0),
      allScheduledAttemptsTerminal: true,
      teardownTerminal: true,
      sealedAt: this.clock.wallNow(),
      terminal: true,
    }, { controllerState: 'evidence_sealed', artifacts: [receipt] });
    this.seal = receipt;
    this.state = 'evidence_sealed';
    return this.snapshot();
  }

  async analyze(analysis = {}) {
    this.#assertState('evidence_sealed');
    this.#assertFrozenInputsUnchanged();
    await this.writer.appendEvent('analysis_terminal', {
      kind: 'analysis_terminal',
      sealedLedgerHeadSha256: this.writer.parentEventSha256,
      resultSetSha256: analysis.resultSetSha256 ?? sha256([...this.results.values()]),
      remediationGraphSha256: analysis.remediationGraphSha256 ?? sha256(analysis),
      analyzedAt: this.clock.wallNow(),
      terminal: true,
    }, { controllerState: 'analyzed' });
    this.state = 'analyzed';
    return this.snapshot();
  }

  async verifyIntegrity() {
    const mismatches = [];
    for (const artifact of this.writer.artifacts) {
      try {
        const content = await this.writer.store.read(artifact.relativePath);
        const actualSha256 = sha256(content);
        if (actualSha256 !== artifact.sha256) {
          mismatches.push({
            artifactId: artifact.artifactId,
            expectedSha256: artifact.sha256,
            actualSha256,
          });
        }
      } catch (error) {
        mismatches.push({
          artifactId: artifact.artifactId,
          expectedSha256: artifact.sha256,
          actualSha256: null,
          errorCode: error?.code ?? 'read_failed',
        });
      }
    }
    return { valid: mismatches.length === 0, mismatches };
  }

  snapshot() {
    const schedule = this.schedule.map((attempt) => {
      const result = this.results.get(attempt.attemptId);
      return {
        ...attempt,
        resultState: result?.resultState ?? null,
        blockedBy: result?.blockedBy ?? null,
        terminalRecordId: result?.recordId ?? null,
      };
    });
    const byResultState = Object.fromEntries(
      RESULT_STATES.map((state) => [
        state,
        schedule.filter((attempt) => attempt.resultState === state).length,
      ]),
    );
    return clone({
      schemaVersion: 'agent-browser.p158-campaign-snapshot.v1',
      runId: this.runId,
      state: this.state,
      prepared: this.prepared,
      registrySha256: sha256(this.registry),
      candidate: this.candidate,
      candidateSha256: this.candidate?.candidateSha256 ?? null,
      manifest: this.manifest,
      manifestSha256: this.writer.manifestSha256,
      freezeReceipt: this.freezeReceipt,
      schedule,
      scheduledTeardown: this.scheduledTeardown,
      results: [...this.results.values()].sort((left, right) => {
        const leftIndex = left.scheduleIndex ?? Number.MAX_SAFE_INTEGER;
        const rightIndex = right.scheduleIndex ?? Number.MAX_SAFE_INTEGER;
        return leftIndex - rightIndex || left.attemptId.localeCompare(right.attemptId);
      }),
      safety: Object.fromEntries([...this.safety.entries()].sort(([left], [right]) => left.localeCompare(right))),
      safetyStops: this.safetyStops,
      counts: {
        total: schedule.length,
        terminal: schedule.filter((attempt) => attempt.resultState !== null).length,
        byResultState,
      },
      seal: this.seal,
      evidence: this.writer.snapshot(),
    });
  }
}

export function createCampaignController({ runRoot, registry, runId, seed, clock, store }) {
  return new CampaignController({
    registry,
    runId,
    seed,
    clock,
    store: store ?? createFileArtifactStore(runRoot),
  });
}
