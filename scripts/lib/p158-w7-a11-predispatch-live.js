import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { sha256 } from './p158-campaign-controller.js';

export const P158_W7_A11_PREDISPATCH_SOURCE_PATH =
  'scripts/lib/p158-w7-a11-predispatch-live.js';

const SHA256 = /^[a-f0-9]{64}$/u;
const EXPECTED_MESSAGE =
  'tab_new cannot execute remote-view route intent; use authenticated remote_view_open to acquire the route and serviceTabHandle';

export class P158W7A11PredispatchError extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158W7A11PredispatchError';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7A11PredispatchError(code, message, details);
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

function validateInput({ runId, candidateSha256, environment, environmentSealSha256, fetch, clock }) {
  if (typeof runId !== 'string' || runId.length === 0 || !SHA256.test(candidateSha256 ?? '') ||
      !SHA256.test(environmentSealSha256 ?? '') || typeof fetch !== 'function' ||
      typeof clock !== 'function' || !['E0', 'E1'].includes(environment?.environmentId) ||
      environment.runtimeLane !== 'development' || environment.production !== false) {
    fail('a11_predispatch_identity_unproven',
      'A11 pre-dispatch probing requires a sealed isolated development environment');
  }
  let origin;
  try { origin = new URL(environment.serviceOrigin); } catch {
    fail('a11_predispatch_identity_unproven', 'A11 requires an exact Service origin');
  }
  if (!['http:', 'https:'].includes(origin.protocol)) {
    fail('a11_predispatch_identity_unproven', 'A11 Service origin must use HTTP or HTTPS');
  }
  return origin.origin;
}

async function jsonResponse(response, label) {
  if (!response || typeof response.json !== 'function' || !Number.isInteger(response.status)) {
    fail('a11_predispatch_transport_invalid', `${label} did not return an HTTP response`);
  }
  try { return await response.json(); } catch {
    fail('a11_predispatch_response_invalid', `${label} did not return JSON`);
  }
}

async function readJournal(fetch, origin) {
  const response = await fetch(new URL('/api/service/failures?limit=1000', origin), { method: 'GET' });
  const payload = await jsonResponse(response, 'Failure journal read');
  if (!response.ok || payload?.success !== true || !Array.isArray(payload.data?.records)) {
    fail('a11_failure_journal_unavailable', 'A11 could not read the append-only failure journal');
  }
  return payload.data;
}

function occurrenceIds(readback) {
  return new Set(readback.records.map((record) => record.occurrenceId).filter(Boolean));
}

function matchingPredispatchRecords(readback, baselineIds) {
  return readback.records.filter((record) => !baselineIds.has(record.occurrenceId) &&
    record.schemaVersion === 'agent-browser.service-failure-record.v1' &&
    record.runtimeEnvironment === 'development' && record.category === 'service_action' &&
    record.source === 'http_service_request' && record.stage === 'ingress_validation' &&
    record.code === 'invalid_bounded_recipe' && record.action === 'tab_new' &&
    record.summary === EXPECTED_MESSAGE &&
    typeof record.references?.requestId === 'string' && record.references.requestId.length > 0);
}

/**
 * Exercise only A11's pre_dispatch_denial dimension against a frozen
 * development candidate. The other A11 terminal boundaries remain blocked.
 */
export async function executeP158W7A11PredispatchProbe({
  runId, candidateSha256, environment, environmentSealSha256,
  fetch = globalThis.fetch, clock = () => new Date().toISOString(),
} = {}) {
  const origin = validateInput({
    runId, candidateSha256, environment, environmentSealSha256, fetch, clock,
  });
  const baseline = await readJournal(fetch, origin);
  const baselineIds = occurrenceIds(baseline);
  const request = {
    action: 'tab_new',
    serviceName: 'P158SyntheticService',
    agentName: 'p158-a11',
    taskName: 'predispatchDenial',
    params: {
      url: 'data:text/html,p158-a11-synthetic',
      routePoolEntryId: 'p158-a11-forbidden-route-intent',
    },
  };
  const response = await fetch(new URL('/api/service/request', origin), {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(request),
  });
  const responsePayload = await jsonResponse(response, 'Rejected Service request');
  if (response.ok || response.status !== 400 || responsePayload?.success !== false ||
      !JSON.stringify(responsePayload).includes(EXPECTED_MESSAGE)) {
    fail('a11_predispatch_rejection_missing',
      'Route-bearing tab_new did not fail at the ingress boundary', {
        responseStatus: response.status,
        responseSuccess: responsePayload?.success ?? null,
        responseCode: responsePayload?.code ?? responsePayload?.error?.code ?? null,
        responseSha256: sha256(responsePayload),
      });
  }
  const after = await readJournal(fetch, origin);
  const matches = matchingPredispatchRecords(after, baselineIds);
  if (matches.length !== 1) {
    fail('a11_failure_journal_correlation_invalid',
      'A11 requires exactly one new correlated pre-dispatch failure record', {
        observedMatchingRecordCount: matches.length,
      });
  }
  const failureRecord = matches[0];
  const requestId = failureRecord.references.requestId;
  const traceUrl = new URL('/api/service/trace', origin);
  traceUrl.searchParams.set('requestId', requestId);
  const traceResponse = await fetch(traceUrl, { method: 'GET' });
  const tracePayload = await jsonResponse(traceResponse, 'Service trace read');
  const trace = tracePayload?.data ?? tracePayload;
  const correlatedJobs = Array.isArray(trace?.jobs) ? trace.jobs.filter((job) =>
    (job?.provenance?.requestId ?? job?.requestId) === requestId) : null;
  if (!traceResponse.ok || tracePayload?.success !== true ||
      correlatedJobs === null || correlatedJobs.length !== 0) {
    fail('a11_predispatch_job_created',
      'A request rejected before dispatch must not create a Service job');
  }
  const body = {
    schemaVersion: 'agent-browser.p158-w7-a11-predispatch-receipt.v1',
    planId: 'P158', caseId: 'A11', terminalBoundary: 'pre_dispatch_denial',
    runId, candidateSha256, environmentId: environment.environmentId,
    environmentSealSha256, serviceOriginSha256: sha256(origin),
    requestId, requestIdSha256: sha256(requestId),
    occurrenceId: failureRecord.occurrenceId,
    occurrenceIdSha256: sha256(failureRecord.occurrenceId),
    failureCode: failureRecord.code, responseStatus: response.status,
    journalRecordCountDelta: after.records.length - baseline.records.length,
    matchingJournalRecordCount: 1, noJobCreated: true,
    resultState: 'passed', effectState: 'verified_no_effect',
    observedAt: clock(), retryDisposition: 'prohibited_opportunistic_retry',
    retryAttempted: false, repairAttempted: false, garbageCollectionAttempted: false,
    remainingA11TerminalBoundaries: [
      'queue_full', 'wait_reschedule', 'cancellation', 'worker_stop',
      'terminal_persistence_failure',
    ],
    sourceBinding: {
      sourcePath: P158_W7_A11_PREDISPATCH_SOURCE_PATH,
      sourceSha256: sourceSha256(),
    },
  };
  return freeze({ ...body, receiptSha256: sha256(body) });
}

export function p158W7A11PredispatchSourceBinding() {
  return freeze({
    sourcePath: P158_W7_A11_PREDISPATCH_SOURCE_PATH,
    sourceSha256: sourceSha256(),
  });
}
