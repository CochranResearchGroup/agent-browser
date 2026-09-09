#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  canonicalP158LoggingExpectationSetDigest,
  createP158LoggingHarvestHook,
  createP158ServiceLoggingObserver,
  harvestP158LoggingEvidence,
  p158LoggingEvidenceSourceBinding,
  P158LoggingEvidenceError,
} from './lib/p158-logging-evidence-harvester.js';
import { auditCausalEnvelopes } from './lib/p158-logging-auditor.js';

const runId = 'p158-harvester-run';
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const fixtureSchema = JSON.parse(await readFile(
  'docs/dev/contracts/p158-logging-causal-fixtures.v1.schema.json', 'utf8'));
ajv.addSchema(fixtureSchema);
ajv.addSchema(JSON.parse(await readFile(
  'docs/dev/contracts/service-failure-record.v1.schema.json', 'utf8')));
const validateCorpus = ajv.compile(JSON.parse(await readFile(
  'docs/dev/contracts/p158-logging-evidence-corpus.v1.schema.json', 'utf8')));
const requestId = `p158:${runId}:A01-E0-r001:request`;
const expectation = {
  campaignRunId: runId, expectationId: 'A01-E0-r001:open:001', requestKind: 'accepted_request',
  phaseId: 'W7', environmentId: 'E0', caseId: 'A01',
  attemptId: 'A01-E0-r001', operationCorrelationId: `${runId}:A01-E0-r001:open`,
  productRequestId: null, productRequestIdState: 'assigned_at_runtime',
  jobId: 'job-1', eventId: 'event-1', traceId: 'trace-1',
  incidentExpected: false, operatorVisible: false, executionMode: 'concrete_live',
  expectedSurfaceRoles: ['ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome'],
};
function causalEnvelope(entry = expectation, overrides = {}) {
  return {
    expectationId: entry.expectationId, actionId: entry.actionId ?? null,
    environmentId: entry.environmentId, operationCorrelationId: entry.operationCorrelationId,
    observedCausalIds: {
      requestId: entry.observedProductRequestId === undefined ? requestId : entry.observedProductRequestId,
      jobId: entry.jobId ?? null, eventId: entry.eventId ?? null,
      traceId: entry.traceId ?? null, incidentId: entry.incidentId ?? null,
    },
    ...overrides,
  };
}
const provenance = {
  schemaVersion: 'agent-browser.service-request-provenance.v1', requestId, jobId: 'job-1', traceId: 'trace-1',
  clientSubjectId: 'client-1', identityAssurance: 'self-declared', connectionInstanceId: 'connection-1',
  runtimeEnvironmentId: 'E0', runtimeLaneId: 'development', profileId: 'profile-1', browserId: 'browser-1',
  sessionId: 'session-1', tabId: 'tab-1', action: 'launch', policyRevision: 1,
};
const times = {
  ingress_request: '2026-09-03T10:00:01.000Z', immediate_response: '2026-09-03T10:00:02.000Z',
  durable_job: '2026-09-03T10:00:03.000Z', terminal_event: '2026-09-03T10:00:04.000Z',
  trace_outcome: '2026-09-03T10:00:05.000Z', controller_transition: '2026-09-03T10:00:01.000Z',
  pre_execution_blocker: '2026-09-03T10:00:02.000Z',
};
const parents = {
  ingress_request: null,
  immediate_response: 'record-ingress_request',
  durable_job: 'record-immediate_response',
  terminal_event: 'record-durable_job',
  trace_outcome: 'record-terminal_event',
};

function record(surfaceRole, overrides = {}) {
  return {
    campaignRunId: runId, attemptId: expectation.attemptId, surfaceRole,
    transport: ['ingress_request', 'immediate_response'].includes(surfaceRole) ? 'http' : 'service',
    recordId: `record-${surfaceRole}`, requestId, jobId: 'job-1',
    eventId: surfaceRole === 'terminal_event' || surfaceRole === 'trace_outcome' ? 'event-1' : undefined,
    traceId: 'trace-1', timestamp: times[surfaceRole], parentId: parents[surfaceRole] ?? null,
    terminal: surfaceRole === 'terminal_event',
    state: surfaceRole === 'terminal_event' || surfaceRole === 'trace_outcome' ? 'succeeded' : 'accepted',
    phase: surfaceRole === 'ingress_request' ? 'ingress'
      : surfaceRole === 'immediate_response' ? 'queue_admission'
        : surfaceRole === 'durable_job' ? 'dispatch' : 'finalize',
    effectState: surfaceRole === 'terminal_event' || surfaceRole === 'trace_outcome' ? 'verified_effect' : 'no_effect',
    retryDisposition: 'do_not_retry', failure: null, provenance,
    captureState: 'complete', captureGap: null,
    ...overrides,
  };
}

function memoryStore() {
  const entries = new Map();
  return {
    entries, writes: 0,
    async read(path) {
      if (!entries.has(path)) throw Object.assign(new Error('missing'), { code: 'ENOENT' });
      return entries.get(path);
    },
    async writeOnce(path, value) {
      assert.equal(entries.has(path), false, `replayed write ${path}`);
      entries.set(path, Buffer.from(value));
      this.writes += 1;
    },
  };
}

function observerResult(records, target = expectation) {
  return {
    records: structuredClone(records),
    observerReceipts: [{
      receiptId: `${target.expectationId}:observer:01`, expectationId: target.expectationId,
      environmentId: target.environmentId, capturePlane: true, operation: 'injected_observer',
      requestSha256: sha256({ expectationId: target.expectationId, operation: 'injected_observer' }),
    }],
  };
}

function baseInput({ observer, artifactStore = memoryStore(), expectations = [expectation],
  causalEnvelopes = expectations.map((entry) => causalEnvelope(entry)),
  checkpointRecords = [],
  dashboardProjections = [] } = {}) {
  return {
    runId, candidateSha256: '11'.repeat(32), phaseId: 'W7',
    environment: { environmentId: 'E0', serviceOrigin: 'http://127.0.0.1:48158',
      runtimeLane: 'development', production: false },
    environmentSealSha256: '22'.repeat(32),
    window: { startedAt: '2026-09-03T10:00:00.000Z', completedAt: '2026-09-03T10:01:00.000Z' },
    expectations, expectationSetSha256: canonicalP158LoggingExpectationSetDigest(expectations),
    causalEnvelopes, checkpointRecords, dashboardProjections, observer,
    artifactStore, runRoot: '/tmp/p158-harvester-provider-free', capturedAt: '2026-09-03T10:00:59.000Z',
  };
}

async function harvestWith(observerRecords, overrides = {}) {
  let calls = 0;
  const input = baseInput({
    observer: async ({ expectation: target }) => { calls += 1; return observerResult(observerRecords, target); },
    ...overrides,
  });
  const original = structuredClone({
    runId: input.runId, candidateSha256: input.candidateSha256, phaseId: input.phaseId,
    environment: input.environment, window: input.window, expectations: input.expectations,
    causalEnvelopes: input.causalEnvelopes, checkpointRecords: input.checkpointRecords,
  });
  const result = await harvestP158LoggingEvidence(input);
  assert.deepEqual({
    runId: input.runId, candidateSha256: input.candidateSha256, phaseId: input.phaseId,
    environment: input.environment, window: input.window, expectations: input.expectations,
    causalEnvelopes: input.causalEnvelopes, checkpointRecords: input.checkpointRecords,
  }, original, 'harvester mutated inputs');
  return { ...result, calls, store: input.artifactStore, input };
}

function findingCodes(corpus) {
  return [...new Set(auditCausalEnvelopes({ fixtureSet: corpus }).findings.map((entry) => entry.code))];
}

const serviceRecords = ['ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome']
  .map((surfaceRole) => record(surfaceRole));
const liveObserverCalls = [];
const liveFetch = async (url) => {
    liveObserverCalls.push(String(url));
    const pathname = new URL(url).pathname;
    const terminalOutcome = { effectState: 'verified_effect', retryDisposition: 'do_not_retry', failure: null };
    const job = { id: 'job-1', state: 'succeeded', submittedAt: times.durable_job,
      completedAt: times.terminal_event, provenance, terminalOutcome };
    const event = { id: 'event-1', state: 'succeeded', timestamp: times.terminal_event,
      provenance, terminalOutcome };
    const outcome = { id: 'trace-1', state: 'succeeded', timestamp: times.trace_outcome,
      provenance, terminalOutcome };
    const failureRecord = {
      schemaVersion: 'agent-browser.service-failure-record.v1', occurrenceId: 'failure-1',
      occurredAt: times.terminal_event, bootEpoch: 'boot-1', runtimeEnvironment: 'development',
      category: 'browser_launch', source: 'control_plane', stage: 'terminal_job', code: 'launch_failed',
      summary: 'Browser launch failed', action: 'launch', references: { requestId, jobId: 'job-1' },
    };
    const data = pathname.endsWith('/failures')
      ? { schemaVersion: 'agent-browser.service-failure-journal-readback.v1', records: [failureRecord],
        malformedLineCount: 0, writeFailureCount: 0 }
      : pathname.endsWith('/trace') ? { jobs: [job], events: [event], incidents: [], outcomes: [outcome] }
      : pathname.endsWith('/events') ? { events: [event] }
        : { job };
    return { ok: true, async json() { return { success: true, data }; } };
  };
const liveObserver = createP158ServiceLoggingObserver({ fetch: liveFetch });
const observedFromService = await liveObserver({
  environment: baseInput({ observer: async () => observerResult([]) }).environment,
  expectation: { ...expectation, productRequestId: requestId, productRequestIdState: 'observed' },
  window: baseInput({ observer: async () => observerResult([]) }).window,
});
assert.deepEqual(observedFromService.records.map((entry) => entry.surfaceRole),
  ['durable_job', 'terminal_event', 'trace_outcome']);
assert.equal(observedFromService.observerReceipts.length, 3);
const observedFailureJournal = await liveObserver.readFailureJournal({
  environment: baseInput({ observer: async () => observerResult([]) }).environment,
  window: baseInput({ observer: async () => observerResult([]) }).window,
});
assert.equal(observedFailureJournal.records.length, 1);
assert.equal(observedFailureJournal.observerReceipt.operation, 'service_failure_journal');
assert.equal(liveObserverCalls.length, 4);
assert(liveObserverCalls.filter((url) => !url.endsWith('/api/service/jobs/job-1') &&
  !new URL(url).pathname.endsWith('/failures'))
  .every((url) => new URL(url).searchParams.get('requestId') === requestId));
assert(liveObserverCalls.some((url) => url.endsWith('/api/service/jobs/job-1')));

const clean = await harvestWith(serviceRecords);
assert.equal(clean.calls, 1);
assert.equal(clean.corpus.fixtureCount, 1);
assert.equal(clean.corpus.observerRequestCount, 1);
assert.equal(clean.corpus.observerReceipts[0].capturePlane, true);
assert.equal(clean.corpus.effectsAttempted, false);
assert.equal(clean.corpus.redactionPolicy.mode, 'allowlist_projection');
assert.equal(clean.corpus.redactionPolicy.rawSensitiveMaterialDisposition, 'reject');
assert.equal(clean.corpus.failureJournal.captureState, 'unavailable');
const { corpusSha256, ...corpusBody } = clean.corpus;
assert.equal(corpusSha256, sha256(corpusBody));
assert.equal(clean.store.writes, 1);
assert.equal(validateCorpus(clean.corpus), true, JSON.stringify(validateCorpus.errors));
assert.deepEqual(auditCausalEnvelopes({ fixtureSet: clean.corpus }).findings, []);
const journalAwareObserver = async ({ expectation: target }) => observerResult(serviceRecords, target);
journalAwareObserver.readFailureJournal = liveObserver.readFailureJournal;
const journalAware = await harvestP158LoggingEvidence(baseInput({
  observer: journalAwareObserver,
  artifactStore: memoryStore(),
}));
assert.equal(journalAware.corpus.failureJournal.captureState, 'complete');
assert.equal(journalAware.corpus.failureJournal.records[0].category, 'browser_launch');
assert(journalAware.corpus.observerReceipts.some((entry) => entry.operation === 'service_failure_journal'));
const source = p158LoggingEvidenceSourceBinding();
assert.equal(source.sourceSha256, sha256(await readFile(source.sourcePath)));

let replayCalls = 0;
const replay = await harvestP158LoggingEvidence({
  ...clean.input,
  observer: async () => { replayCalls += 1; throw new Error('must not replay observation'); },
});
assert.equal(replay.resumed, true);
assert.equal(replayCalls, 0);
assert.equal(clean.store.writes, 1);
assert.deepEqual(replay.corpus, clean.corpus);
const tamperedStore = memoryStore();
const tamperedInput = baseInput({ observer: async ({ expectation: target }) =>
  observerResult(serviceRecords, target), artifactStore: tamperedStore });
await harvestP158LoggingEvidence(tamperedInput);
const tamperedPath = [...tamperedStore.entries.keys()][0];
const tamperedCorpus = JSON.parse(tamperedStore.entries.get(tamperedPath).toString('utf8'));
tamperedCorpus.fixtureCount = 2;
tamperedStore.entries.set(tamperedPath, Buffer.from(JSON.stringify(tamperedCorpus)));
await assert.rejects(() => harvestP158LoggingEvidence(tamperedInput),
  (error) => error.code === 'logging_evidence_integrity_invalid');

const missing = await harvestWith(serviceRecords.filter((entry) => entry.surfaceRole !== 'trace_outcome'));
assert.equal(validateCorpus(missing.corpus), true, JSON.stringify(validateCorpus.errors));
assert.equal(missing.corpus.fixtures[0].records.find((entry) => entry.surfaceRole === 'trace_outcome').captureState, 'missing');
assert.deepEqual(findingCodes(missing.corpus), ['capture_gap']);

const uncorrelatableExpectation = {
  ...expectation,
  expectationId: `${expectation.expectationId}:uncorrelatable`,
  operationCorrelationId: `${runId}:A01-E0-r001:uncorrelatable`,
  observedProductRequestId: null,
  jobId: null, eventId: null, traceId: null,
};
const uncorrelatable = await harvestWith([], {
  expectations: [uncorrelatableExpectation],
  causalEnvelopes: [causalEnvelope(uncorrelatableExpectation, { observedCausalIds: {
    requestId: null, jobId: null, eventId: null, traceId: null, incidentId: null,
  } })],
});
assert.ok(uncorrelatable.corpus.fixtures[0].records.every((entry) =>
  entry.requestId === null && entry.captureGap.includes('request_id_correlation_unavailable')),
'an unavailable product request ID must remain an explicit correlation gap');

const duplicate = await harvestWith([...serviceRecords, record('terminal_event', {
  recordId: 'record-terminal-event-duplicate', eventId: 'event-duplicate',
  timestamp: '2026-09-03T10:00:04.001Z',
})]);
assert.deepEqual(findingCodes(duplicate.corpus), ['duplicate_terminal']);

const conflicting = await harvestWith(serviceRecords.map((entry) => entry.surfaceRole === 'trace_outcome'
  ? { ...entry, requestId: 'request-conflicting', provenance: { ...provenance, requestId: 'request-conflicting' } }
  : entry));
assert.deepEqual(findingCodes(conflicting.corpus), ['conflicting_projection']);

const reordered = await harvestWith([...serviceRecords].reverse());
assert.deepEqual(reordered.corpus.fixtures[0].records.map((entry) => entry.recordId),
  clean.corpus.fixtures[0].records.map((entry) => entry.recordId));

const nullFailure = await harvestWith(serviceRecords.map((entry) => entry.surfaceRole === 'terminal_event'
  ? { ...entry, state: 'failed', failure: null } : entry));
assert.deepEqual(findingCodes(nullFailure.corpus), ['null_failure']);
const nullProvenance = await harvestWith(serviceRecords.map((entry) => entry.surfaceRole === 'terminal_event'
  ? { ...entry, provenance: null } : entry));
assert.deepEqual(findingCodes(nullProvenance.corpus), ['null_provenance']);

await assert.rejects(() => harvestWith([record('durable_job', { campaignRunId: 'another-run' })]),
  (error) => error instanceof P158LoggingEvidenceError && error.code === 'cross_run_record_rejected');
await assert.rejects(() => harvestWith([record('durable_job', { authorization: 'Bearer secret-secret-secret' })]),
  (error) => error instanceof P158LoggingEvidenceError && error.code === 'sensitive_evidence_rejected');

function combinedEnvelopeInput(environmentId, complete, artifactStore) {
  const combinedExpectation = {
    ...expectation, attemptId: 'X10-E1_E2-r001', caseId: 'X10', environmentId,
    expectationId: `X10-E1_E2-r001:${environmentId}:epoch:001`,
    operationCorrelationId: `${runId}:X10-E1_E2-r001:${environmentId}:request`,
    observedProductRequestId: `service-request-${environmentId}`,
    jobId: `job-${environmentId}`, eventId: `event-${environmentId}`, traceId: `trace-${environmentId}`,
  };
  const roles = ['ingress_request', 'immediate_response', 'durable_job', 'terminal_event', 'trace_outcome'];
  const rows = roles.map((surfaceRole, index) => {
    const recordId = `${environmentId}-${surfaceRole}`;
    return {
      ...record(surfaceRole), attemptId: combinedExpectation.attemptId,
      expectationId: combinedExpectation.expectationId,
      requestId: combinedExpectation.observedProductRequestId, jobId: combinedExpectation.jobId,
      eventId: ['terminal_event', 'trace_outcome'].includes(surfaceRole) ? combinedExpectation.eventId : undefined,
      traceId: combinedExpectation.traceId, recordId,
      parentId: index === 0 ? null : `${environmentId}-${roles[index - 1]}`,
      provenance: { ...provenance, requestId: combinedExpectation.observedProductRequestId,
        jobId: combinedExpectation.jobId, traceId: combinedExpectation.traceId,
        runtimeEnvironmentId: environmentId },
    };
  });
  const serviceRows = rows.filter((entry) => complete || entry.surfaceRole !== 'trace_outcome');
  return baseInput({
    artifactStore, expectations: [combinedExpectation], causalEnvelopes: [causalEnvelope(combinedExpectation)],
    checkpointRecords: [],
    observer: async ({ expectation: target }) => observerResult(serviceRows, target),
  });
}

const combinedStore = memoryStore();
const e1Input = combinedEnvelopeInput('E1', true, combinedStore);
e1Input.environment = { ...e1Input.environment, environmentId: 'E1' };
const e2Input = combinedEnvelopeInput('E2', false, combinedStore);
e2Input.environment = { ...e2Input.environment, environmentId: 'E2',
  serviceOrigin: 'https://synthetic-development.example.test' };
const e1 = await harvestP158LoggingEvidence(e1Input);
const e2 = await harvestP158LoggingEvidence(e2Input);
assert.deepEqual(auditCausalEnvelopes({ fixtureSet: e1.corpus }).findings, []);
assert.deepEqual(findingCodes(e2.corpus), ['capture_gap']);
assert.equal(e2.corpus.fixtures[0].records.some((entry) => entry.traceId === 'trace-E1'), false,
  'E1 evidence incorrectly satisfied E2');
assert.notEqual(e1.relativePath, e2.relativePath);

const secondExpectation = {
  ...expectation, expectationId: 'A01-E0-r001:release:001', requestKind: 'accepted_request',
  operationCorrelationId: `${runId}:A01-E0-r001:release`, observedProductRequestId: 'service-request-2',
  jobId: 'job-2', eventId: 'event-2', traceId: 'trace-2',
};
function rewriteRequest(entry, target) {
  const suffix = target.expectationId.includes('release') ? '-2' : '-1';
  const targetRequestId = target.observedProductRequestId ?? requestId;
  return {
    ...entry, expectationId: target.expectationId, requestId: targetRequestId,
    jobId: target.jobId, eventId: entry.eventId ? target.eventId : undefined,
    traceId: target.traceId, recordId: `${entry.recordId}${suffix}`,
    parentId: entry.parentId ? `${entry.parentId}${suffix}` : null,
    provenance: { ...entry.provenance, requestId: targetRequestId, jobId: target.jobId, traceId: target.traceId },
  };
}
const multiRequestRows = [expectation, secondExpectation].flatMap((target) =>
  [record('ingress_request'), record('immediate_response')].map((entry) => rewriteRequest(entry, target)));
const multiRequestServiceRows = [expectation, secondExpectation].flatMap((target) =>
  serviceRecords.map((entry) => rewriteRequest(entry, target)));
const multiRequest = await harvestP158LoggingEvidence(baseInput({
  expectations: [expectation, secondExpectation],
  causalEnvelopes: [causalEnvelope(expectation), causalEnvelope(secondExpectation)],
  checkpointRecords: multiRequestRows,
  observer: async ({ expectation: target }) => observerResult(multiRequestServiceRows.filter((entry) =>
    entry.expectationId === target.expectationId), target),
}));
assert.equal(multiRequest.corpus.fixtureCount, 2);
assert.deepEqual(multiRequest.corpus.fixtures.map((fixture) => fixture.fixtureId),
  [expectation.expectationId, secondExpectation.expectationId]);

const a05Expectation = {
  ...expectation,
  expectationId: `${runId}:A05-E0-r001:policy-mutate`,
  operationCorrelationId: `${runId}:A05-E0-r001:policy-mutate`,
  attemptId: 'A05-E0-r001',
  caseId: 'A05',
};
const a05IngressRows = [record('ingress_request'), record('immediate_response')]
  .map((entry) => rewriteRequest(entry, a05Expectation));
const a05ServiceRows = serviceRecords.map((entry) => rewriteRequest(entry, a05Expectation));
const a05PrefixBound = await harvestP158LoggingEvidence(baseInput({
  expectations: [a05Expectation],
  causalEnvelopes: [causalEnvelope(a05Expectation)],
  checkpointRecords: a05IngressRows,
  observer: async ({ expectation: target }) => observerResult(a05ServiceRows, target),
}));
assert.deepEqual(auditCausalEnvelopes({ fixtureSet: a05PrefixBound.corpus }).findings, [],
  'an exact live driver request ID beginning with the campaign run ID was rejected');
assert.deepEqual(auditCausalEnvelopes({ fixtureSet: multiRequest.corpus }).findings, []);

const blockedExpectation = {
  campaignRunId: runId, expectationId: 'A11-E0-r001:blocker', requestKind: 'rejected_request',
  phaseId: 'W7', environmentId: 'E0', caseId: 'A11', attemptId: 'A11-E0-r001',
  operationCorrelationId: `${runId}:A11-E0-r001:blocked`,
  productRequestId: null, productRequestIdState: 'not_applicable', executionMode: 'explicit_blocked',
  incidentExpected: false, operatorVisible: false,
  expectedSurfaceRoles: ['controller_transition', 'pre_execution_blocker', 'terminal_event'],
};
const blockedProvenance = { ...provenance, requestId: 'controller-blocked-record',
  jobId: 'blocked-controller-job', traceId: null, action: 'explicit_blocked' };
const blockedRows = ['controller_transition', 'pre_execution_blocker', 'terminal_event'].map((surfaceRole, index) => ({
  campaignRunId: runId, attemptId: blockedExpectation.attemptId, surfaceRole, transport: 'controller',
  expectationId: blockedExpectation.expectationId,
  recordId: `blocked-${surfaceRole}`, requestId: null,
  jobId: 'blocked-controller-job', timestamp: `2026-09-03T10:00:0${index + 1}.000Z`,
  parentId: index === 0 ? null : `blocked-${blockedExpectation.expectedSurfaceRoles[index - 1]}`,
  terminal: surfaceRole === 'terminal_event', state: surfaceRole === 'terminal_event' ? 'rejected' : 'accepted',
  phase: surfaceRole === 'terminal_event' ? 'finalize' : 'scheduler_admission', effectState: 'no_effect',
  retryDisposition: 'do_not_retry', failure: null, provenance: blockedProvenance,
  captureState: 'complete', captureGap: null,
}));
let blockedObserverCalls = 0;
const blocked = await harvestWith([], {
  expectations: [blockedExpectation], checkpointRecords: blockedRows,
  observer: async () => { blockedObserverCalls += 1; throw new Error('blocked attempt observed Service'); },
});
assert.equal(blockedObserverCalls, 0);
assert.deepEqual(auditCausalEnvelopes({ fixtureSet: blocked.corpus }).findings, []);

const hookStore = memoryStore();
const loggingExpectations = [expectation];
const loggingOperationGaps = [{
  descriptorId: `${runId}:A13:operation-1`, operationCorrelationId: `${runId}:A13:operation-1`,
  productRequestId: null, correlationState: 'product_request_id_unavailable',
  operationKind: 'handoff-prepare', actionId: 'A13:action-1', attemptId: 'A13-E1-r001',
  caseId: 'A13', phaseId: 'W7', environmentId: 'E1',
  loggingGap: { code: 'product_request_id_not_preserved', detail: 'Synthetic test gap.' },
}];
const hook = createP158LoggingHarvestHook({
  configuration: {
    runId, scheduleSha256: '51'.repeat(32), candidateSha256: '11'.repeat(32),
    environments: { E0: baseInput({ observer: async () => observerResult([]) }).environment },
    environmentSealSha256s: { E0: '22'.repeat(32) },
    loggingExpectations,
    loggingExpectationsSha256: sha256(loggingExpectations),
    loggingOperationGaps, loggingOperationGapsSha256: sha256(loggingOperationGaps),
    windowsByEnvironmentPhase: { 'W7:E0': baseInput({ observer: async () => observerResult([]) }).window },
  },
  artifactStore: hookStore, runRoot: '/tmp/p158-harvester-hook-provider-free',
  clock: { wallNow: () => '2026-09-03T10:01:00.000Z' },
  fetchByEnvironment: { E0: liveFetch },
});
const registeredArtifacts = [];
const hookReceipt = await hook.execute({
  schedule: { scheduleSha256: '51'.repeat(32) }, target: { runId },
  sourceDigest: '52'.repeat(32), causalEnvelopes: [causalEnvelope(expectation)],
  checkpointRecords: [record('ingress_request'), record('immediate_response')],
  registerArtifact: async (artifact) => {
    registeredArtifacts.push(structuredClone(artifact));
    await hookStore.writeOnce(artifact.relativePath, artifact.content);
  },
});
const { receiptSha256, ...hookReceiptBody } = hookReceipt;
assert.equal(receiptSha256, sha256(hookReceiptBody));
assert.equal(hookReceipt.state, 'capture_gap');
assert.equal(hookReceipt.artifactIds.length, 2);
assert.deepEqual(registeredArtifacts.map((artifact) => artifact.artifactId).sort(), hookReceipt.artifactIds);
assert.equal(registeredArtifacts.find((artifact) => artifact.relativePath.endsWith('operation-gaps.json'))
  .metadata.analysisRole, 'logging_operation_gaps');
assert(registeredArtifacts.filter((artifact) => !artifact.relativePath.endsWith('operation-gaps.json'))
  .every((artifact) => artifact.metadata.analysisRole === 'logging_evidence'));
assert.equal(hookReceipt.operationGapCount, 1);
assert.ok(hookReceipt.findingCodes.includes('request_id_correlation_unavailable'));
assert.ok(hookReceipt.findingCodes.includes('unobserved_due_to_uncorrelatable_id'));
assert.equal(hookReceipt.repairAttempted, false);
assert.equal(hookReceipt.retryAttempted, false);
await assert.rejects(() => hook.execute({
  schedule: { scheduleSha256: 'bad' }, target: { runId }, sourceDigest: '52'.repeat(32),
  causalEnvelopes: [causalEnvelope(expectation)], checkpointRecords: [],
}), (error) => error.code === 'logging_harvest_execution_binding_invalid');

const gapFetch = async (url) => {
  const response = await liveFetch(url);
  const payload = await response.json();
  if (new URL(url).pathname.endsWith('/trace')) payload.data.outcomes = [];
  return { ok: true, async json() { return payload; } };
};
const gapHook = createP158LoggingHarvestHook({
  configuration: {
    runId, scheduleSha256: '51'.repeat(32), candidateSha256: '11'.repeat(32),
    environments: { E0: baseInput({ observer: async () => observerResult([]) }).environment },
    environmentSealSha256s: { E0: '22'.repeat(32) }, loggingExpectations,
    loggingExpectationsSha256: sha256(loggingExpectations),
    windowsByEnvironmentPhase: { 'W7:E0': baseInput({ observer: async () => observerResult([]) }).window },
  },
  artifactStore: memoryStore(), runRoot: '/tmp/p158-harvester-gap-hook-provider-free',
  clock: { wallNow: () => '2026-09-03T10:01:00.000Z' }, fetchByEnvironment: { E0: gapFetch },
});
const gapReceipt = await gapHook.execute({
  schedule: { scheduleSha256: '51'.repeat(32) }, target: { runId }, sourceDigest: '52'.repeat(32),
  causalEnvelopes: [causalEnvelope(expectation)],
  checkpointRecords: [record('ingress_request'), record('immediate_response')],
});
assert.equal(gapReceipt.state, 'capture_gap');
assert.deepEqual(gapReceipt.findingCodes, ['capture_gap', 'one_transport_only', 'timestamp_inversion']);

process.stdout.write('P158 logging evidence harvester passed exact live, adversarial, blocked, and resume contracts\n');
