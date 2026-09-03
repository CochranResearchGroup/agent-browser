import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  getServiceEvents,
  getServiceIncident,
  getServiceJob,
  getServiceTrace,
} from '../../packages/client/src/service-observability.js';
import { canonicalJson, sha256 } from './p158-campaign-controller.js';

export const P158_LOGGING_EVIDENCE_SOURCE_PATH = 'scripts/lib/p158-logging-evidence-harvester.js';
export const P158_LOGGING_SURFACE_ROLES = Object.freeze([
  'controller_transition', 'pre_execution_blocker', 'ingress_request', 'immediate_response',
  'durable_job', 'terminal_event', 'trace_outcome', 'incident', 'dashboard_projection',
]);

const SERVICE_ROLES = new Set([
  'ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome', 'incident',
]);
const BLOCKED_ROLES = Object.freeze([
  'controller_transition', 'pre_execution_blocker', 'terminal_event',
]);
const HARD_SENSITIVE_FIELDS = new Set([
  'authorization', 'cookie', 'setcookie', 'password', 'accesstoken', 'refreshtoken', 'bearertoken',
  'handoffurl', 'providerexternalurl', 'privatepagecontent', 'pagehtml', 'html', 'screenshot',
]);
const SENSITIVE_VALUE_PATTERNS = [
  /\bBearer\s+[A-Za-z0-9._~+/=-]{8,}/iu,
  /-----BEGIN [A-Z ]+PRIVATE KEY-----/u,
  /(?:password|access[_-]?token|refresh[_-]?token|cookie)\s*[:=]\s*[^\s,;]{4,}/iu,
];
const REDACTED_SOURCE_FIELDS = Object.freeze([
  'capturedValues', 'details', 'headers', 'message', 'requestBody', 'responseBody', 'result', 'target',
]);
const STATE_MAP = Object.freeze({
  queued: 'accepted', waiting_profile_lease: 'accepted', running: 'accepted',
  succeeded: 'succeeded', failed: 'failed', cancelled: 'cancelled', timed_out: 'timed_out',
});

export class P158LoggingEvidenceError extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158LoggingEvidenceError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158LoggingEvidenceError(code, message, details);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function sourceSha256() {
  return createHash('sha256').update(readFileSync(fileURLToPath(import.meta.url))).digest('hex');
}

function freeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value)) freeze(child);
  }
  return value;
}

function normalizedField(field) {
  return field.replaceAll(/[^a-z0-9]/giu, '').toLowerCase();
}

function assertNoSensitiveMaterial(value, path = '$', seen = new Set()) {
  if (typeof value === 'string') {
    if (SENSITIVE_VALUE_PATTERNS.some((pattern) => pattern.test(value))) {
      fail('sensitive_evidence_rejected', `Raw sensitive evidence at ${path}`);
    }
    return;
  }
  if (!value || typeof value !== 'object' || seen.has(value)) return;
  seen.add(value);
  for (const [field, child] of Object.entries(value)) {
    if (HARD_SENSITIVE_FIELDS.has(normalizedField(field)) && child !== null && child !== undefined) {
      fail('sensitive_evidence_rejected', `Forbidden evidence field ${path}.${field}`);
    }
    assertNoSensitiveMaterial(child, `${path}.${field}`, seen);
  }
  seen.delete(value);
}

function parseTime(value, label) {
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) fail('logging_time_invalid', `${label} must be an RFC 3339 timestamp`);
  return parsed;
}

function validateIdentity({ runId, candidateSha256, phaseId, environment, environmentSealSha256, window }) {
  if (typeof runId !== 'string' || runId.length === 0 || !/^[a-f0-9]{64}$/u.test(candidateSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(environmentSealSha256 ?? '') ||
      !['W7', 'W8', 'W9'].includes(phaseId) || typeof environment?.environmentId !== 'string' ||
      environment.runtimeLane !== 'development' || environment.production !== false) {
    fail('logging_identity_unproven', 'Logging evidence requires frozen development identities');
  }
  let origin;
  try { origin = new URL(environment.serviceOrigin); } catch {
    fail('logging_identity_unproven', 'Logging evidence requires an exact Service origin');
  }
  if (!['http:', 'https:'].includes(origin.protocol)) {
    fail('logging_identity_unproven', 'Logging evidence Service origin must be HTTP or HTTPS');
  }
  const started = parseTime(window?.startedAt, 'window.startedAt');
  const completed = parseTime(window?.completedAt, 'window.completedAt');
  if (completed < started) fail('logging_time_invalid', 'Logging evidence window is inverted');
  return { started, completed };
}

function validateExpectations(expectations, runId, phaseId, environmentId) {
  if (!Array.isArray(expectations) || expectations.length === 0) {
    fail('logging_expectations_missing', 'At least one frozen logging expectation is required');
  }
  const attemptIds = new Set();
  return expectations.map((entry) => {
    if (typeof entry?.attemptId !== 'string' || attemptIds.has(entry.attemptId) ||
        entry.phaseId !== phaseId || entry.environmentId !== environmentId ||
        typeof entry.requestId !== 'string' || !entry.requestId.includes(`:${runId}:`) ||
        !Array.isArray(entry.expectedSurfaceRoles) || entry.expectedSurfaceRoles.length === 0 ||
        new Set(entry.expectedSurfaceRoles).size !== entry.expectedSurfaceRoles.length ||
        entry.expectedSurfaceRoles.some((role) => !P158_LOGGING_SURFACE_ROLES.includes(role))) {
      fail('logging_expectation_invalid', `Invalid expectation for ${entry?.attemptId ?? 'unknown'}`);
    }
    const blocked = entry.executionMode === 'explicit_blocked';
    if (blocked && JSON.stringify([...entry.expectedSurfaceRoles].sort()) !==
        JSON.stringify([...BLOCKED_ROLES].sort())) {
      fail('logging_expectation_invalid', `${entry.attemptId} has invalid blocked surfaces`);
    }
    attemptIds.add(entry.attemptId);
    return freeze(clone(entry));
  }).sort((left, right) => left.attemptId.localeCompare(right.attemptId));
}

function validateCausalEnvelopes(causalEnvelopes, expectations, runId, phaseId, environmentId) {
  if (!Array.isArray(causalEnvelopes) || causalEnvelopes.length !== expectations.length) {
    fail('causal_envelope_cardinality_invalid', 'Every environment-specific expectation needs one terminal causal envelope');
  }
  const byAttempt = new Map();
  for (const envelope of causalEnvelopes) {
    const { envelopeSha256, ...body } = envelope ?? {};
    if (envelopeSha256 !== sha256(body) || envelope.schemaVersion !== 'agent-browser.p158-causal-envelope.v1' ||
        envelope.runId !== runId || envelope.phaseId !== phaseId || envelope.environmentId !== environmentId ||
        typeof envelope.attemptId !== 'string' || byAttempt.has(envelope.attemptId) ||
        typeof envelope.requestId !== 'string') {
      fail('causal_envelope_invalid', `Invalid causal envelope for ${envelope?.attemptId ?? 'unknown'}`);
    }
    byAttempt.set(envelope.attemptId, freeze(clone(envelope)));
  }
  return expectations.map((expectation) => {
    const envelope = byAttempt.get(expectation.attemptId);
    if (!envelope || envelope.requestId !== expectation.requestId) {
      fail('causal_envelope_invalid', `${expectation.attemptId} causal identity does not match preparation`);
    }
    return freeze({
      ...clone(expectation), jobId: envelope.jobId ?? null, eventId: envelope.eventId ?? null,
      traceId: envelope.traceId ?? null, incidentId: envelope.incidentId ?? null,
      causalEnvelopeSha256: envelope.envelopeSha256,
    });
  });
}

function causalIds(record, expectation) {
  const provenance = record.provenance ?? {};
  return {
    requestId: record.requestId ?? provenance.requestId ?? expectation.requestId ?? null,
    jobId: record.jobId ?? provenance.jobId ?? expectation.jobId ?? null,
    eventId: record.eventId ?? expectation.eventId ?? null,
    traceId: record.traceId ?? provenance.traceId ?? expectation.traceId ?? null,
    incidentId: record.incidentId ?? expectation.incidentId ?? null,
  };
}

function normalizeRecord(record, expectation, index, window) {
  if (!P158_LOGGING_SURFACE_ROLES.includes(record?.surfaceRole)) {
    fail('logging_record_invalid', `${expectation.attemptId} record ${index} has an unknown surface`);
  }
  if (record.campaignRunId !== undefined && record.campaignRunId !== expectation.campaignRunId) {
    fail('cross_run_record_rejected', `${expectation.attemptId} record belongs to another run`);
  }
  assertNoSensitiveMaterial(record);
  const timestamp = record.timestamp ?? record.wallTime ?? record.completedAt;
  const observed = parseTime(timestamp, `${expectation.attemptId}.records[${index}].timestamp`);
  if (observed < window.started || observed > window.completed) {
    fail('logging_record_outside_window', `${expectation.attemptId} record is outside the frozen window`);
  }
  const ids = causalIds(record, expectation);
  return {
    surfaceRole: record.surfaceRole,
    transport: record.transport ?? (record.surfaceRole === 'dashboard_projection' ? 'dashboard' : 'service'),
    recordId: String(record.recordId ?? record.id ?? `${expectation.attemptId}:${record.surfaceRole}:${index}`),
    requestId: ids.requestId,
    ...(ids.jobId ? { jobId: ids.jobId } : {}),
    ...(ids.eventId ? { eventId: ids.eventId } : {}),
    ...(ids.traceId ? { traceId: ids.traceId } : {}),
    ...(ids.incidentId ? { incidentId: ids.incidentId } : {}),
    causalIds: ids,
    timestamp,
    parentId: record.parentId ?? null,
    ...(Array.isArray(record.parentIds) ? { parentIds: [...new Set(record.parentIds)].sort() } : {}),
    terminal: record.terminal === true,
    state: STATE_MAP[record.state] ?? record.state ?? 'accepted',
    phase: record.phase ?? 'finalize',
    effectState: record.effectState ?? record.terminalOutcome?.effectState ?? 'no_effect',
    retryDisposition: record.retryDisposition ?? record.terminalOutcome?.retryDisposition ?? 'do_not_retry',
    failure: clone(record.failure ?? record.structuredFailure ?? record.terminalOutcome?.failure ?? null),
    provenance: clone(record.provenance ?? null),
    captureState: record.captureState ?? 'complete',
    captureGap: record.captureGap ?? null,
    capturedValues: [],
  };
}

function missingRecord(expectation, role, ordinal, capturedAt) {
  return {
    surfaceRole: role,
    transport: role === 'dashboard_projection' ? 'dashboard' : 'service',
    recordId: `${expectation.attemptId}:${role}:capture-gap:${ordinal}`,
    requestId: expectation.requestId,
    ...(expectation.jobId ? { jobId: expectation.jobId } : {}),
    ...(expectation.eventId ? { eventId: expectation.eventId } : {}),
    ...(expectation.traceId ? { traceId: expectation.traceId } : {}),
    ...(expectation.incidentId ? { incidentId: expectation.incidentId } : {}),
    causalIds: {
      requestId: expectation.requestId, jobId: expectation.jobId ?? null,
      eventId: expectation.eventId ?? null, traceId: expectation.traceId ?? null,
      incidentId: expectation.incidentId ?? null,
    },
    timestamp: capturedAt, parentId: null, terminal: false, state: 'rejected', phase: 'finalize',
    effectState: 'no_effect', retryDisposition: 'inspect_before_retry', failure: null, provenance: null,
    captureState: 'missing', captureGap: `expected ${role} was absent from exact bound evidence`,
    capturedValues: [],
  };
}

function serviceRecord(role, record, expectation) {
  const provenance = record.provenance ?? null;
  const terminalOutcome = record.terminalOutcome ?? null;
  return {
    surfaceRole: role,
    transport: 'service',
    recordId: record.id,
    requestId: provenance?.requestId ?? expectation.requestId,
    jobId: record.jobId ?? provenance?.jobId ?? (role === 'durable_job' ? record.id : expectation.jobId),
    eventId: role === 'terminal_event' ? record.id : expectation.eventId,
    traceId: provenance?.traceId ?? expectation.traceId,
    incidentId: role === 'incident' ? record.id : expectation.incidentId,
    timestamp: record.timestamp ?? record.submittedAt ?? record.completedAt,
    parentId: record.parentId ?? null,
    terminal: role === 'terminal_event' || terminalOutcome !== null,
    state: record.state,
    phase: terminalOutcome ? 'finalize' : 'dispatch',
    effectState: terminalOutcome?.effectState ?? 'no_effect',
    retryDisposition: terminalOutcome?.retryDisposition ?? 'do_not_retry',
    failure: terminalOutcome?.failure ?? record.error ?? null,
    provenance,
    captureState: 'complete', captureGap: null,
  };
}

export function createP158ServiceLoggingObserver({ fetch = globalThis.fetch } = {}) {
  if (typeof fetch !== 'function') fail('logging_observer_invalid', 'A fetch implementation is required');
  return async ({ environment, expectation, window }) => {
    const query = { requestId: expectation.requestId, since: window.startedAt, limit: 500 };
    const [trace, events, job, incident] = await Promise.all([
      getServiceTrace({ baseUrl: environment.serviceOrigin, fetch, query }),
      getServiceEvents({ baseUrl: environment.serviceOrigin, fetch, query }),
      expectation.jobId ? getServiceJob({ baseUrl: environment.serviceOrigin, fetch, id: expectation.jobId }) : null,
      expectation.incidentId
        ? getServiceIncident({ baseUrl: environment.serviceOrigin, fetch, id: expectation.incidentId }) : null,
    ]);
    assertNoSensitiveMaterial({ trace, events, job, incident });
    const matching = (records) => (records ?? []).filter((record) =>
      (record.provenance?.requestId ?? record.requestId) === expectation.requestId);
    return [
      ...matching(job ? [job.job ?? job] : trace.jobs).map((record) => serviceRecord('durable_job', record, expectation)),
      ...matching(events.events ?? trace.events).map((record) => serviceRecord('terminal_event', record, expectation)),
      ...matching(incident ? [incident.incident ?? incident] : trace.incidents)
        .map((record) => serviceRecord('incident', record, expectation)),
      ...matching(trace.outcomes).map((record) => serviceRecord('trace_outcome', record, expectation)),
    ];
  };
}

function artifactPath(environmentId, phaseId) {
  return `artifacts/logging/${environmentId}/${phaseId}/logging-evidence.json`;
}

function verifyCorpus(corpus, identitySha256) {
  const { corpusSha256, ...body } = corpus ?? {};
  if (corpusSha256 !== sha256(body) || corpus.inputIdentitySha256 !== identitySha256) {
    fail('logging_evidence_integrity_invalid', 'Stored logging evidence does not match the frozen harvest');
  }
  return freeze(clone(corpus));
}

export async function harvestP158LoggingEvidence({
  runId, candidateSha256, phaseId, environment, environmentSealSha256, window,
  expectations, causalEnvelopes, checkpointRecords = [], dashboardProjections = [], observer,
  artifactStore, runRoot, capturedAt,
}) {
  const times = validateIdentity({ runId, candidateSha256, phaseId, environment, environmentSealSha256, window });
  if (!isAbsolute(runRoot ?? '')) fail('logging_run_root_invalid', 'Logging run root must be absolute');
  const repoRoot = resolve(fileURLToPath(new URL('../..', import.meta.url)));
  const relativeToRepo = relative(repoRoot, resolve(runRoot));
  if (relativeToRepo === '' || (!relativeToRepo.startsWith('..') && !isAbsolute(relativeToRepo))) {
    fail('logging_run_root_inside_repository', 'Logging evidence belongs outside the product repository');
  }
  if (typeof artifactStore?.read !== 'function' || typeof artifactStore?.writeOnce !== 'function' ||
      typeof observer !== 'function') {
    fail('logging_dependency_missing', 'Logging harvest requires observer and append-only artifact store');
  }
  parseTime(capturedAt, 'capturedAt');
  const preparedExpectations = validateExpectations(
    expectations.map((entry) => ({ ...entry, campaignRunId: runId })),
    runId, phaseId, environment.environmentId,
  );
  const normalizedExpectations = validateCausalEnvelopes(
    causalEnvelopes, preparedExpectations, runId, phaseId, environment.environmentId,
  );
  const sourceBinding = {
    sourcePath: P158_LOGGING_EVIDENCE_SOURCE_PATH, sourceSha256: sourceSha256(),
  };
  const inputIdentity = {
    runId, candidateSha256, phaseId, environmentId: environment.environmentId,
    environmentSealSha256, serviceOrigin: new URL(environment.serviceOrigin).origin,
    window: clone(window), expectations: normalizedExpectations, sourceBinding,
  };
  const inputIdentitySha256 = sha256(inputIdentity);
  const relativePath = artifactPath(environment.environmentId, phaseId);
  const stored = await artifactStore.read(relativePath).catch((error) => {
    if (error?.code === 'ENOENT') return null;
    throw error;
  });
  if (stored !== null && stored !== undefined) {
    const parsed = JSON.parse(Buffer.from(stored).toString('utf8'));
    return { corpus: verifyCorpus(parsed, inputIdentitySha256), resumed: true, relativePath };
  }

  const suppliedRecords = [...checkpointRecords, ...dashboardProjections];
  assertNoSensitiveMaterial(suppliedRecords);
  const fixtures = [];
  for (const expectation of normalizedExpectations) {
    const records = suppliedRecords.filter((record) => record.attemptId === expectation.attemptId);
    if (expectation.executionMode !== 'explicit_blocked' &&
        expectation.expectedSurfaceRoles.some((role) => SERVICE_ROLES.has(role))) {
      const observed = await observer({ environment: clone(environment), expectation: clone(expectation), window: clone(window) });
      if (!Array.isArray(observed)) fail('logging_observer_invalid', 'Logging observer must return records');
      records.push(...observed);
    }
    const normalized = records.map((record, index) => normalizeRecord(record, expectation, index, times));
    for (const [ordinal, role] of expectation.expectedSurfaceRoles.entries()) {
      if (!normalized.some((record) => record.surfaceRole === role)) {
        normalized.push(missingRecord(expectation, role, ordinal, capturedAt));
      }
    }
    normalized.sort((left, right) => left.timestamp.localeCompare(right.timestamp) ||
      left.recordId.localeCompare(right.recordId));
    fixtures.push({
      fixtureId: expectation.attemptId,
      description: `Live logging evidence for ${expectation.attemptId}`,
      operatorVisible: expectation.operatorVisible === true,
      incidentExpected: expectation.incidentExpected === true,
      expectedSurfaceRoles: [...expectation.expectedSurfaceRoles],
      records: normalized,
      expectedFindingCodes: [],
    });
  }
  const body = {
    schemaVersion: 'agent-browser.p158-logging-evidence-corpus.v1', planId: 'P158', syntheticOnly: false,
    runId, candidateSha256, phaseId, environmentId: environment.environmentId, environmentSealSha256,
    sourceBinding, window: clone(window), capturedAt, inputIdentitySha256,
    fixtureCount: fixtures.length, fixtures,
    redactionPolicy: {
      mode: 'allowlist_projection', excludedFieldNames: [...REDACTED_SOURCE_FIELDS],
      rawSensitiveMaterialDisposition: 'reject',
    },
    effectsAttempted: false, repairAttempted: false, retryAttempted: false,
  };
  const corpus = freeze({ ...body, corpusSha256: sha256(body) });
  await artifactStore.writeOnce(relativePath, canonicalJson(corpus));
  return { corpus, resumed: false, relativePath };
}

export function p158LoggingEvidenceSourceBinding() {
  return freeze({ sourcePath: P158_LOGGING_EVIDENCE_SOURCE_PATH, sourceSha256: sourceSha256() });
}
