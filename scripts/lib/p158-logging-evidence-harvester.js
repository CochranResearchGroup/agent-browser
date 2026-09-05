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
import { auditCausalEnvelopes } from './p158-logging-auditor.js';

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
  const expectationIds = new Set();
  const operationCorrelationIds = new Set();
  return expectations.map((entry) => {
    if (typeof entry?.expectationId !== 'string' || expectationIds.has(entry.expectationId) ||
        typeof entry?.attemptId !== 'string' || operationCorrelationIds.has(entry.operationCorrelationId) ||
        entry.phaseId !== phaseId || entry.environmentId !== environmentId ||
        typeof entry.operationCorrelationId !== 'string' ||
        !(entry.operationCorrelationId.startsWith(`${runId}:`) ||
          entry.operationCorrelationId.includes(`:${runId}:`)) ||
        entry.productRequestId !== null ||
        !['accepted_request', 'rejected_request', 'transition', 'dashboard_action']
          .includes(entry.requestKind) ||
        !Array.isArray(entry.expectedSurfaceRoles) || entry.expectedSurfaceRoles.length === 0 ||
        new Set(entry.expectedSurfaceRoles).size !== entry.expectedSurfaceRoles.length ||
        entry.expectedSurfaceRoles.some((role) => !P158_LOGGING_SURFACE_ROLES.includes(role))) {
      fail('logging_expectation_invalid', `Invalid expectation for ${entry?.attemptId ?? 'unknown'}`);
    }
    const blocked = entry.executionMode === 'explicit_blocked';
    if (entry.productRequestIdState !== (blocked ? 'not_applicable' : 'assigned_at_runtime')) {
      fail('logging_expectation_invalid', `${entry.attemptId} has invalid product request identity state`);
    }
    if (blocked && JSON.stringify([...entry.expectedSurfaceRoles].sort()) !==
        JSON.stringify([...BLOCKED_ROLES].sort())) {
      fail('logging_expectation_invalid', `${entry.attemptId} has invalid blocked surfaces`);
    }
    expectationIds.add(entry.expectationId);
    operationCorrelationIds.add(entry.operationCorrelationId);
    return freeze(clone(entry));
  }).sort((left, right) => left.expectationId.localeCompare(right.expectationId));
}

export function canonicalP158LoggingExpectationSetDigest(expectations) {
  return sha256([...clone(expectations ?? [])]
    .sort((left, right) => String(left.expectationId).localeCompare(String(right.expectationId))));
}

function validateCausalEnvelopes(causalEnvelopes, expectations, runId, phaseId, environmentId) {
  if (!Array.isArray(causalEnvelopes) || causalEnvelopes.length !== expectations.length) {
    fail('causal_envelope_cardinality_invalid', 'Every environment-specific expectation needs one terminal causal envelope');
  }
  const byExpectation = new Map();
  for (const envelope of causalEnvelopes) {
    if (envelope?.environmentId !== environmentId ||
        typeof envelope.expectationId !== 'string' ||
        byExpectation.has(envelope.expectationId) ||
        typeof envelope.operationCorrelationId !== 'string' || !envelope.observedCausalIds ||
        (envelope.observedCausalIds.requestId !== null &&
          typeof envelope.observedCausalIds.requestId !== 'string')) {
      fail('causal_envelope_invalid', `Invalid causal envelope for ${envelope?.attemptId ?? 'unknown'}`);
    }
    byExpectation.set(envelope.expectationId, freeze(clone(envelope)));
  }
  return expectations.map((expectation) => {
    const envelope = byExpectation.get(expectation.expectationId);
    if (!envelope || envelope.actionId !== (expectation.actionId ?? null) ||
        envelope.operationCorrelationId !== expectation.operationCorrelationId) {
      fail('causal_envelope_invalid', `${expectation.attemptId} causal identity does not match preparation`);
    }
    const ids = envelope.observedCausalIds;
    return freeze({
      ...clone(expectation), productRequestId: ids.requestId ?? null,
      productRequestIdState: ids.requestId ? 'observed' :
        (expectation.executionMode === 'explicit_blocked' ? 'not_applicable' : 'not_returned'),
      jobId: ids.jobId ?? null, eventId: ids.eventId ?? null,
      traceId: ids.traceId ?? null, incidentId: ids.incidentId ?? null,
      causalEnvelopeSha256: sha256(envelope),
    });
  });
}

function causalIds(record, expectation) {
  const provenance = record.provenance ?? {};
  return {
    // An expectation ID is a harness correlation key, not an observed product
    // request ID. Only independently captured product evidence may populate it.
    requestId: record.requestId ?? provenance.requestId ?? null,
    jobId: record.jobId ?? provenance.jobId ?? expectation.jobId ?? null,
    eventId: record.eventId ?? expectation.eventId ?? null,
    traceId: record.traceId ?? provenance.traceId ?? expectation.traceId ?? null,
    incidentId: record.incidentId ?? expectation.incidentId ?? null,
  };
}

function normalizeRecord(record, expectation, index, window, runId) {
  if (!P158_LOGGING_SURFACE_ROLES.includes(record?.surfaceRole)) {
    fail('logging_record_invalid', `${expectation.attemptId} record ${index} has an unknown surface`);
  }
  if (record.campaignRunId !== undefined && record.campaignRunId !== runId) {
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
  const correlationUnavailable = expectation.productRequestIdState === 'not_returned';
  return {
    surfaceRole: role,
    transport: role === 'dashboard_projection' ? 'dashboard' : 'service',
    recordId: `${expectation.expectationId}:${role}:capture-gap:${ordinal}`,
    requestId: null,
    ...(expectation.jobId ? { jobId: expectation.jobId } : {}),
    ...(expectation.eventId ? { eventId: expectation.eventId } : {}),
    ...(expectation.traceId ? { traceId: expectation.traceId } : {}),
    ...(expectation.incidentId ? { incidentId: expectation.incidentId } : {}),
    causalIds: {
      requestId: null, jobId: expectation.jobId ?? null,
      eventId: expectation.eventId ?? null, traceId: expectation.traceId ?? null,
      incidentId: expectation.incidentId ?? null,
    },
    timestamp: capturedAt, parentId: null, terminal: false, state: 'rejected', phase: 'finalize',
    effectState: 'no_effect', retryDisposition: 'inspect_before_retry', failure: null, provenance: null,
    captureState: 'missing', captureGap: correlationUnavailable
      ? `request_id_correlation_unavailable; unobserved_due_to_uncorrelatable_id:${role}`
      : `expected ${role} was absent from exact bound evidence`,
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
    requestId: provenance?.requestId ?? null,
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
  const observer = async ({ environment, expectation, window }) => {
    if (!expectation.productRequestId) {
      const operation = { operation: 'product_request_id_correlation_unavailable',
        expectationId: expectation.expectationId, window };
      return { records: [], observerReceipts: [{
        receiptId: `${expectation.expectationId}:observer:01`, expectationId: expectation.expectationId,
        environmentId: environment.environmentId, capturePlane: true, operation: operation.operation,
        requestSha256: sha256(operation),
      }] };
    }
    const query = { requestId: expectation.productRequestId, since: window.startedAt, limit: 500 };
    const [trace, events, job, incident] = await Promise.all([
      getServiceTrace({ baseUrl: environment.serviceOrigin, fetch, query }),
      getServiceEvents({ baseUrl: environment.serviceOrigin, fetch, query }),
      expectation.jobId ? getServiceJob({ baseUrl: environment.serviceOrigin, fetch, id: expectation.jobId }) : null,
      expectation.incidentId
        ? getServiceIncident({ baseUrl: environment.serviceOrigin, fetch, id: expectation.incidentId }) : null,
    ]);
    assertNoSensitiveMaterial({ trace, events, job, incident });
    const matching = (records) => (records ?? []).filter((record) =>
      (record.provenance?.requestId ?? record.requestId) === expectation.productRequestId);
    const records = [
      ...matching(job ? [job.job ?? job] : trace.jobs).map((record) => serviceRecord('durable_job', record, expectation)),
      ...matching(events.events ?? trace.events).map((record) => serviceRecord('terminal_event', record, expectation)),
      ...matching(incident ? [incident.incident ?? incident] : trace.incidents)
        .map((record) => serviceRecord('incident', record, expectation)),
      ...matching(trace.outcomes).map((record) => serviceRecord('trace_outcome', record, expectation)),
    ];
    const operations = [
      { operation: 'service_trace', query },
      { operation: 'service_events', query },
      ...(expectation.jobId ? [{ operation: 'service_job', id: expectation.jobId }] : []),
      ...(expectation.incidentId ? [{ operation: 'service_incident', id: expectation.incidentId }] : []),
    ];
    return {
      records,
      observerReceipts: operations.map((operation, index) => ({
        receiptId: `${expectation.expectationId}:observer:${String(index + 1).padStart(2, '0')}`,
        expectationId: expectation.expectationId, environmentId: environment.environmentId,
        capturePlane: true, operation: operation.operation,
        requestSha256: sha256(operation),
      })),
    };
  };
  observer.readFailureJournal = async ({ environment, window }) => {
    const url = new URL('/api/service/failures', environment.serviceOrigin);
    url.searchParams.set('limit', '1000');
    const response = await fetch(url, { method: 'GET' });
    if (!response.ok) fail('logging_failure_journal_unavailable', `Failure journal read failed: ${response.status}`);
    const payload = await response.json();
    if (!payload?.success || !Array.isArray(payload.data?.records)) {
      fail('logging_failure_journal_invalid', 'Failure journal readback is not a valid Service response');
    }
    assertNoSensitiveMaterial(payload.data);
    const started = parseTime(window.startedAt, 'window.startedAt');
    const completed = parseTime(window.completedAt, 'window.completedAt');
    return {
      captureState: 'complete',
      records: payload.data.records.filter((record) => {
        const occurred = Date.parse(record.occurredAt);
        return Number.isFinite(occurred) && occurred >= started && occurred <= completed;
      }),
      malformedLineCount: payload.data.malformedLineCount ?? 0,
      writeFailureCount: payload.data.writeFailureCount ?? 0,
      captureGap: null,
      observerReceipt: {
        receiptId: `${environment.environmentId}:failure-journal:observer:01`,
        expectationId: normalizedFailureJournalReceiptExpectationId(environment.environmentId),
        environmentId: environment.environmentId,
        capturePlane: true,
        operation: 'service_failure_journal',
        requestSha256: sha256({ operation: 'service_failure_journal', window, limit: 1000 }),
      },
    };
  };
  return observer;
}

function normalizedFailureJournalReceiptExpectationId(environmentId) {
  return `failure-journal:${environmentId}`;
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
  expectations, expectationSetSha256, causalEnvelopes,
  checkpointRecords = [], dashboardProjections = [], observer,
  artifactStore, artifactWriter = null, runRoot, capturedAt,
}) {
  const times = validateIdentity({ runId, candidateSha256, phaseId, environment, environmentSealSha256, window });
  if (!isAbsolute(runRoot ?? '')) fail('logging_run_root_invalid', 'Logging run root must be absolute');
  const repoRoot = resolve(fileURLToPath(new URL('../..', import.meta.url)));
  const relativeToRepo = relative(repoRoot, resolve(runRoot));
  if (relativeToRepo === '' || (!relativeToRepo.startsWith('..') && !isAbsolute(relativeToRepo))) {
    fail('logging_run_root_inside_repository', 'Logging evidence belongs outside the product repository');
  }
  if (typeof artifactStore?.read !== 'function' || typeof artifactStore?.writeOnce !== 'function' ||
      typeof observer !== 'function' || (artifactWriter !== null && typeof artifactWriter !== 'function')) {
    fail('logging_dependency_missing', 'Logging harvest requires observer and append-only artifact store');
  }
  parseTime(capturedAt, 'capturedAt');
  const preparedExpectations = validateExpectations(expectations, runId, phaseId, environment.environmentId);
  if (expectationSetSha256 !== canonicalP158LoggingExpectationSetDigest(preparedExpectations)) {
    fail('logging_expectation_set_unfrozen', 'Logging request expectations do not match their pre-freeze digest');
  }
  const normalizedExpectations = validateCausalEnvelopes(
    causalEnvelopes, preparedExpectations, runId, phaseId, environment.environmentId,
  );
  const sourceBinding = {
    sourcePath: P158_LOGGING_EVIDENCE_SOURCE_PATH, sourceSha256: sourceSha256(),
  };
  const inputIdentity = {
    runId, candidateSha256, phaseId, environmentId: environment.environmentId,
    environmentSealSha256, serviceOrigin: new URL(environment.serviceOrigin).origin,
    window: clone(window), expectationSetSha256, expectations: normalizedExpectations, sourceBinding,
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

  assertNoSensitiveMaterial(checkpointRecords);
  assertNoSensitiveMaterial(dashboardProjections);
  const fixtures = [];
  const observerReceipts = [];
  let failureJournal = {
    captureState: 'unavailable', records: [], malformedLineCount: 0, writeFailureCount: 0,
    captureGap: 'failure_journal_observer_not_configured',
  };
  if (typeof observer.readFailureJournal === 'function') {
    const observedJournal = await observer.readFailureJournal({
      environment: clone(environment), window: clone(window),
    });
    const { observerReceipt, ...journal } = observedJournal ?? {};
    if (!observerReceipt || observerReceipt.capturePlane !== true ||
        observerReceipt.operation !== 'service_failure_journal' ||
        !/^[a-f0-9]{64}$/u.test(observerReceipt.requestSha256 ?? '')) {
      fail('logging_observer_invalid', 'Failure journal observer must expose a capture-plane receipt');
    }
    assertNoSensitiveMaterial(journal);
    failureJournal = clone(journal);
    observerReceipts.push(clone(observerReceipt));
  }
  for (const expectation of normalizedExpectations) {
    // Controller checkpoints are authoritative only for requests deliberately
    // rejected before dispatch. Concrete product surfaces must be returned by
    // the independent observer and cannot be self-attested by the harness.
    const records = expectation.executionMode === 'explicit_blocked'
      ? checkpointRecords.filter((record) => record.expectationId === expectation.expectationId)
      : [];
    if (expectation.executionMode !== 'explicit_blocked') {
      records.push(...dashboardProjections.filter((record) =>
        record.expectationId === expectation.expectationId && record.actionId === expectation.actionId &&
        record.surfaceRole === 'dashboard_projection' && record.capturePlane === true &&
        record.sourceBinding?.implementationKind === 'concrete_live' &&
        typeof record.sourceBinding.sourcePath === 'string' &&
        /^[a-f0-9]{64}$/u.test(record.sourceBinding.sourceSha256 ?? '') &&
        record.provenance?.source !== 'p158_controller' && record.provenance?.source !== 'p158_harness'));
    }
    if (expectation.executionMode !== 'explicit_blocked' &&
        expectation.expectedSurfaceRoles.some((role) => SERVICE_ROLES.has(role))) {
      const observed = await observer({ environment: clone(environment), expectation: clone(expectation), window: clone(window) });
      if (!Array.isArray(observed?.records) || !Array.isArray(observed?.observerReceipts) ||
          observed.observerReceipts.length === 0 || observed.observerReceipts.some((receipt) =>
            receipt.capturePlane !== true || receipt.expectationId !== expectation.expectationId ||
            !/^[a-f0-9]{64}$/u.test(receipt.requestSha256 ?? ''))) {
        fail('logging_observer_invalid', 'Logging observer must expose capture-plane request receipts');
      }
      assertNoSensitiveMaterial(observed);
      records.push(...observed.records);
      observerReceipts.push(...observed.observerReceipts.map(clone));
    }
    const normalized = records.map((record, index) => normalizeRecord(record, expectation, index, times, runId));
    for (const [ordinal, role] of expectation.expectedSurfaceRoles.entries()) {
      if (!normalized.some((record) => record.surfaceRole === role)) {
        normalized.push(missingRecord(expectation, role, ordinal, capturedAt));
      }
    }
    normalized.sort((left, right) => left.timestamp.localeCompare(right.timestamp) ||
      left.recordId.localeCompare(right.recordId));
    fixtures.push({
      fixtureId: expectation.expectationId,
      description: `Live logging evidence for ${expectation.expectationId}`,
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
    expectationSetSha256, fixtureCount: fixtures.length, fixtures,
    observerRequestCount: observerReceipts.length,
    observerReceipts: observerReceipts.sort((left, right) => left.receiptId.localeCompare(right.receiptId)),
    failureJournal,
    redactionPolicy: {
      mode: 'allowlist_projection', excludedFieldNames: [...REDACTED_SOURCE_FIELDS],
      rawSensitiveMaterialDisposition: 'reject',
    },
    effectsAttempted: false, repairAttempted: false, retryAttempted: false,
  };
  const corpus = freeze({ ...body, corpusSha256: sha256(body) });
  if (artifactWriter) {
    await artifactWriter({ artifactId: `p158-logging-evidence:${corpus.corpusSha256}`, relativePath,
      content: canonicalJson(corpus), metadata: { mediaType: 'application/json', analysisRole: 'logging_evidence',
        capturePurpose: 'logging_evidence', captureState: 'complete', redactions: [], parentArtifactSha256s: [] } });
  } else await artifactStore.writeOnce(relativePath, canonicalJson(corpus));
  return { corpus, resumed: false, relativePath };
}

export function createP158LoggingHarvestHook({
  configuration = null, artifactStore, runRoot, clock = { wallNow: () => new Date().toISOString() },
  fetchByEnvironment = {}, ...direct
}) {
  const config = configuration ?? direct;
  const {
    runId, scheduleSha256, candidateSha256, environments, environmentSealSha256s,
    loggingExpectations, loggingExpectationsSha256, windowsByEnvironmentPhase,
    dashboardProjections = [], loggingOperationGaps = [], loggingOperationGapsSha256 = sha256([]),
  } = config;
  if (loggingExpectationsSha256 !== sha256(loggingExpectations) ||
      loggingOperationGapsSha256 !== sha256(loggingOperationGaps) ||
      loggingOperationGaps.some((entry) => entry?.productRequestId !== null ||
        entry.correlationState !== 'product_request_id_unavailable' ||
        entry.loggingGap?.code !== 'product_request_id_not_preserved') ||
      !/^[a-f0-9]{64}$/u.test(scheduleSha256 ?? '') || typeof clock?.wallNow !== 'function') {
    fail('logging_expectation_set_unfrozen', 'Pre-seal harvest requires the exact preparation expectation seal');
  }
  const sourcePath = P158_LOGGING_EVIDENCE_SOURCE_PATH;
  const sourceDigest = sourceSha256();
  const frozenExpectations = freeze(clone(loggingExpectations));
  return freeze({
    sourcePath,
    sourceSha256: sourceDigest,
    async execute({ schedule, target, sourceDigest: executionSourceDigest,
      causalEnvelopes, checkpointRecords, registerArtifact = null }) {
      if (schedule?.scheduleSha256 !== scheduleSha256 || target?.runId !== runId ||
          !/^[a-f0-9]{64}$/u.test(executionSourceDigest ?? '') ||
          !Array.isArray(causalEnvelopes) || !Array.isArray(checkpointRecords) ||
          (registerArtifact !== null && typeof registerArtifact !== 'function')) {
        fail('logging_harvest_execution_binding_invalid', 'Pre-seal harvest invocation is not campaign-bound');
      }
      const groups = new Map();
      for (const expectation of frozenExpectations) {
        const key = `${expectation.phaseId}:${expectation.environmentId}`;
        if (!groups.has(key)) groups.set(key, []);
        groups.get(key).push(expectation);
      }
      const artifactIds = [];
      const corpusSha256s = [];
      const findingCodes = new Set();
      {
        const operationGapBody = {
          schemaVersion: 'agent-browser.p158-logging-operation-gaps.v1', planId: 'P158', runId, scheduleSha256,
          sourcePath, sourceSha256: sourceDigest, loggingOperationGapsSha256,
          operationGapCount: loggingOperationGaps.length,
          operations: clone(loggingOperationGaps), capturedAt: clock.wallNow(),
          repairAttempted: false, retryAttempted: false,
        };
        const operationGapArtifact = { ...operationGapBody, artifactSha256: sha256(operationGapBody) };
        const operationGapArtifactId = `p158-logging-operation-gaps:${operationGapArtifact.artifactSha256}`;
        const operationGapWrite = { artifactId: operationGapArtifactId,
          relativePath: 'artifacts/logging/operation-gaps.json', content: canonicalJson(operationGapArtifact),
          metadata: { mediaType: 'application/json', analysisRole: 'logging_operation_gaps',
            capturePurpose: 'logging_operation_gaps', captureState: 'complete', redactions: [],
            parentArtifactSha256s: [] } };
        if (registerArtifact) await registerArtifact(operationGapWrite);
        else await artifactStore.writeOnce(operationGapWrite.relativePath, operationGapWrite.content);
        artifactIds.push(operationGapArtifactId);
        if (loggingOperationGaps.length > 0) {
          findingCodes.add('request_id_correlation_unavailable');
          findingCodes.add('unobserved_due_to_uncorrelatable_id');
        }
      }
      for (const key of [...groups.keys()].sort()) {
        const [phaseId, environmentId] = key.split(':');
        const expectations = groups.get(key);
        const environment = environments?.[environmentId];
        const window = windowsByEnvironmentPhase?.[key];
        if (!environment || !window) {
          fail('logging_harvest_group_unconfigured', `Missing logging harvest group ${key}`);
        }
        const matchingEnvelopeIds = new Set(expectations.map((entry) => entry.expectationId));
        const groupEnvelopes = causalEnvelopes.filter((entry) =>
          matchingEnvelopeIds.has(entry.expectationId) && entry.environmentId === environmentId);
        const observer = createP158ServiceLoggingObserver({
          fetch: fetchByEnvironment[environmentId] ?? globalThis.fetch,
        });
        const result = await harvestP158LoggingEvidence({
          runId, candidateSha256, phaseId, environment,
          environmentSealSha256: environmentSealSha256s?.[environmentId],
          window, expectations,
          expectationSetSha256: canonicalP158LoggingExpectationSetDigest(expectations),
          causalEnvelopes: groupEnvelopes, checkpointRecords,
          dashboardProjections, observer, artifactStore,
          artifactWriter: registerArtifact, runRoot,
          capturedAt: window.capturedAt ?? clock.wallNow(),
        });
        const audit = auditCausalEnvelopes({ fixtureSet: result.corpus });
        for (const finding of audit.findings) findingCodes.add(finding.code);
        artifactIds.push(`p158-logging-evidence:${result.corpus.corpusSha256}`);
        corpusSha256s.push(result.corpus.corpusSha256);
      }
      const body = {
        schemaVersion: 'agent-browser.p158-logging-harvest-receipt.v1', runId, scheduleSha256,
        sourcePath, sourceSha256: sourceDigest, executionSourceDigest,
        state: findingCodes.size === 0 ? 'complete' : 'capture_gap',
        artifactIds: artifactIds.sort(), corpusSha256s: corpusSha256s.sort(),
        findingCodes: [...findingCodes].sort(),
        operationGapCount: loggingOperationGaps.length,
        completedAt: clock.wallNow(),
        repairAttempted: false, retryAttempted: false,
      };
      return freeze({ ...body, receiptSha256: sha256(body) });
    },
  });
}

export function p158LoggingEvidenceSourceBinding() {
  return freeze({ sourcePath: P158_LOGGING_EVIDENCE_SOURCE_PATH, sourceSha256: sourceSha256() });
}
