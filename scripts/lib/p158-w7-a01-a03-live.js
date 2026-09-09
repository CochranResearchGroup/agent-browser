import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { Agent, request as httpRequest } from 'node:http';
import { fileURLToPath } from 'node:url';

import {
  getServiceTabHandle,
  postServiceRequest,
  releaseServiceTabHandle,
  requestServiceTab,
} from '../../packages/client/src/service-request.js';
import {
  getServiceEvents,
  getServiceStatus,
  getServiceTabs,
  getServiceTrace,
} from '../../packages/client/src/service-observability.js';
import { sha256 } from './p158-campaign-controller.js';
import { createP158CaseAdapter } from './p158-execution-schedule.js';
import { enumerateP158W7ActionPlans } from './p158-w7-development-adapters.js';

export const P158_W7_A01_A03_CASE_IDS = Object.freeze(['A01', 'A02', 'A03']);
export const P158_W7_A01_A03_HOOK_ID = 'w7.a01_a03.service_concurrency';
export const P158_W7_A01_A03_SOURCE_PATH = 'scripts/lib/p158-w7-a01-a03-live.js';

const EXPECTED_CARDINALITIES = Object.freeze({
  A01: Object.freeze({ sequential_clients: 100, concurrent_clients: 25 }),
  A02: Object.freeze({ clients_per_repetition: 10 }),
  A03: Object.freeze({ live_clients: 10 }),
});
const LIVE_TRANSPORT_FACTORY = Symbol('p158-w7-a01-a03-live-transport');
const HISTORICAL_PRODUCT_FAILURES = new Set([
  'open_state_oracle_failed', 'trace_provenance_oracle_failed', 'event_causality_oracle_failed',
  'release_state_oracle_failed', 'shared_browser_barrier_oracle_failed',
  'same_label_connection_oracle_failed', 'cross_client_theft_oracle_failed',
  'retained_browser_postcondition_failed',
]);
const INCONCLUSIVE_FAILURES = new Set([
  'service_transport_failed', 'service_request_failed', 'ownership_status_mismatch',
]);

export class P158W7A01A03Error extends Error {
  constructor(code, message, details) {
    super(message);
    this.name = 'P158W7A01A03Error';
    this.code = code;
    this.details = details;
  }
}

function fail(code, message, details) {
  throw new P158W7A01A03Error(code, message, details);
}

function classifiedError(code, message, classification, effectState, details) {
  const error = new P158W7A01A03Error(code, message, details);
  error.classification = classification;
  error.effectState = effectState;
  return error;
}

function normalizeFailure(error, defaultEffectState = 'no_effect') {
  const code = error?.code ?? 'service_transport_failed';
  return {
    code,
    message: error?.message ?? String(error),
    classification: error?.classification ?? (HISTORICAL_PRODUCT_FAILURES.has(code)
      ? 'reproduced_historical_failure'
      : (INCONCLUSIVE_FAILURES.has(code) ? 'inconclusive' : 'harness_failure')),
    effectState: error?.effectState ?? defaultEffectState,
  };
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

function exactSubset(actual, expected, path = 'status') {
  if (!expected || typeof expected !== 'object' || Array.isArray(expected)) {
    if (!Object.is(actual, expected)) fail('ownership_status_mismatch', `${path} did not match the frozen value`);
    return;
  }
  if (!actual || typeof actual !== 'object' || Array.isArray(actual)) {
    fail('ownership_status_mismatch', `${path} was absent from Service Status`);
  }
  for (const [key, value] of Object.entries(expected)) exactSubset(actual[key], value, `${path}.${key}`);
}

function validateOwnershipManifest(manifest, schedule) {
  const body = manifest && typeof manifest === 'object'
    ? Object.fromEntries(Object.entries(manifest).filter(([key]) => key !== 'manifestSha256'))
    : null;
  if (manifest?.schemaVersion !== 'agent-browser.p158-w7-a01-a03-ownership.v1' ||
      !/^[a-f0-9]{64}$/u.test(manifest?.candidateSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(manifest?.liveHookManifestSha256 ?? '') ||
      manifest?.manifestSha256 !== sha256(body) ||
      !manifest?.environmentSealSha256s ||
      ['E0', 'E1'].some((environmentId) =>
        !/^[a-f0-9]{64}$/u.test(manifest.environmentSealSha256s[environmentId] ?? '')) ||
      typeof manifest?.campaignRunId !== 'string' || manifest.campaignRunId.length === 0) {
    fail('frozen_ownership_manifest_invalid', 'A01-A03 require an exact self-hashed ownership manifest');
  }
  const environments = new Set(schedule.attempts
    .filter((attempt) => P158_W7_A01_A03_CASE_IDS.includes(attempt.caseId))
    .flatMap((attempt) => attempt.environmentIds));
  for (const environmentId of environments) {
    const environment = manifest.environments?.[environmentId];
    let url;
    try { url = new URL(environment?.serviceOrigin); } catch {
      fail('development_service_origin_invalid', `Missing Service origin for ${environmentId}`);
    }
    if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname) ||
        !url.port || environment.runtimeLane !== 'development' || environment.production !== false ||
        environment.runtimeEnvironmentId !== environmentId ||
        typeof environment.targetId !== 'string' || typeof environment.ownershipStatus !== 'object') {
      fail('development_service_origin_invalid', `Environment ${environmentId} is not isolated development`);
    }
  }
  return freeze(structuredClone(manifest));
}

function clientActions(schedule, caseId, attemptId) {
  const actions = enumerateP158W7ActionPlans({ schedule })
    .filter((action) => action.caseId === caseId && action.attemptId === attemptId)
    .map((action) => ({
      ...action,
      cardinalityId: Object.keys(EXPECTED_CARDINALITIES[caseId])
        .find((id) => action.actionId.startsWith(`${attemptId}:${id}:`)) ?? null,
    }))
    .filter((action) => action.cardinalityId !== null);
  const grouped = Object.groupBy(actions, (action) => action.cardinalityId);
  for (const [id, count] of Object.entries(EXPECTED_CARDINALITIES[caseId])) {
    if ((grouped[id]?.length ?? 0) !== count) {
      fail('scheduled_cardinality_mismatch', `${attemptId} expected ${count} ${id}`);
    }
  }
  return actions;
}

function recordArray(value, key) {
  if (Array.isArray(value)) return value;
  if (Array.isArray(value?.[key])) return value[key];
  return [];
}

function findTab(tabs, handle) {
  return recordArray(tabs, 'tabs').find((tab) =>
    tab.id === handle.tabId || tab.serviceTabHandle?.tabId === handle.tabId);
}

function findJob(trace, requestId, predicate) {
  const jobs = recordArray(trace, 'jobs').filter((job) =>
    (requestId ? job.provenance?.requestId === requestId : predicate(job)));
  return jobs.length === 1 ? jobs[0] : null;
}

function assertOpenOracle({ tabs, trace, events, handle, requestId, subjectId, runtimeEnvironment }) {
  const tab = findTab(tabs, handle);
  const job = findJob(trace, requestId, (candidate) =>
    candidate.provenance?.clientSubjectId === subjectId &&
    candidate.provenance?.runtimeEnvironmentId === runtimeEnvironment);
  if (!tab || !['opening', 'loading', 'ready'].includes(tab.lifecycle) ||
      tab.browserId !== handle.browserId || tab.sessionId !== (handle.sessionName ?? handle.ownerSessionId) ||
      tab.serviceTabHandle?.profileAccess?.subjectId !== subjectId) {
    fail('open_state_oracle_failed', `${requestId} returned HTTP success without the owned tab state`);
  }
  if (!job || job.provenance?.clientSubjectId !== subjectId ||
      job.provenance?.runtimeEnvironmentId !== runtimeEnvironment ||
      typeof job.provenance?.connectionInstanceId !== 'string' ||
      job.provenance.connectionInstanceId.length === 0) {
    fail('trace_provenance_oracle_failed', `${requestId} lacks exact client and connection provenance`);
  }
  if (!recordArray(events, 'events').some((event) =>
    event.provenance?.jobId === job.id || event.provenance?.requestId === requestId ||
    event.terminalOutcome?.provenance?.requestId === requestId)) {
    fail('event_causality_oracle_failed', `${requestId} has no causal Service Event`);
  }
  return job.provenance.connectionInstanceId;
}

function assertReleasedOracle({ tabs, trace, requestId, handle, subjectId }) {
  const tab = findTab(tabs, handle);
  const job = findJob(trace, requestId, (candidate) =>
    candidate.provenance?.clientSubjectId === subjectId &&
    candidate.provenance?.action === 'tab_handle_release');
  if (!tab || tab.lifecycle !== 'closed' || !job || job.provenance?.clientSubjectId !== subjectId ||
      job.provenance?.action !== 'tab_handle_release') {
    fail('release_state_oracle_failed', `${requestId} did not close exactly the owned tab`);
  }
}

async function observe(base, taskName, fetch) {
  const options = { baseUrl: base, fetch, query: { taskName, limit: 100 } };
  const [tabs, events, trace] = await Promise.all([
    getServiceTabs({ baseUrl: base, fetch }), getServiceEvents(options), getServiceTrace(options),
  ]);
  return { tabs, events, trace };
}

function commonRequest(context, action, subjectId, requestId) {
  return {
    serviceName: `p158-${context.caseId.toLowerCase()}`,
    agentName: 'p158-w7-live-runner',
    taskName: context.caseId === 'A03' ? context.fixture.sharedLabel : action.actionId,
    clientSubjectId: subjectId,
    identityAssurance: 'self-declared',
    runtimeProfile: context.fixture.profileId,
    profileId: context.fixture.profileId,
    sessionName: context.fixture.sessionName,
    ...(context.fixture.browserId ? { browserId: context.fixture.browserId } : {}),
  };
}

async function revalidateOwnership(context, fetch) {
  const status = await getServiceStatus({ baseUrl: context.environment.serviceOrigin, fetch });
  exactSubset(status, context.environment.ownershipStatus);
  return status;
}

async function openClient(context, action, subjectId, fetch) {
  await revalidateOwnership(context, fetch);
  const plannedRequestId = `${context.manifest.campaignRunId}:${action.actionId}:open`;
  const request = commonRequest(context, action, subjectId, plannedRequestId);
  let response;
  try {
    response = await requestServiceTab({
      baseUrl: context.environment.serviceOrigin,
      fetch,
      ...request,
      url: context.fixture.url,
    });
  } catch (error) {
    throw classifiedError('service_transport_failed', error.message, 'inconclusive', 'effect_uncertain');
  }
  if (typeof response?.success !== 'boolean') {
    throw classifiedError('service_response_malformed', `${plannedRequestId} returned malformed JSON`,
      'harness_failure', 'effect_uncertain', response);
  }
  if (response.success !== true) {
    throw classifiedError('service_request_failed', `${plannedRequestId} was not successful`,
      'inconclusive', 'no_effect', response);
  }
  const requestId = typeof response.id === 'string' && response.id.length > 0 ? response.id : null;
  const handle = getServiceTabHandle(response);
  if (!handle?.valid || !handle.tabId || !handle.browserId) {
    throw classifiedError('service_tab_handle_invalid', `${requestId ?? plannedRequestId} returned no valid handle`,
      'harness_failure', 'effect_uncertain');
  }
  let connectionInstanceId;
  try {
    const evidence = await observe(context.environment.serviceOrigin,
      context.caseId === 'A03' ? context.fixture.sharedLabel : action.actionId, fetch);
    connectionInstanceId = assertOpenOracle({
      ...evidence, handle, requestId, subjectId, runtimeEnvironment: context.environment.runtimeLane,
    });
  } catch (error) {
    error.effectState ??= 'effect_uncertain';
    throw error;
  }
  return { handle, connectionInstanceId, openRequestId: requestId,
    openOperationCorrelationId: plannedRequestId };
}

async function releaseClient(context, action, subjectId, fetch, opened) {
  await revalidateOwnership(context, fetch);
  const plannedRequestId = `${context.manifest.campaignRunId}:${action.actionId}:release`;
  let response;
  try {
    response = await releaseServiceTabHandle({
      baseUrl: context.environment.serviceOrigin,
      fetch,
      ...commonRequest(context, action, subjectId, plannedRequestId),
      serviceTabHandle: opened.handle,
    });
  } catch (error) {
    throw classifiedError('service_transport_failed', error.message, 'inconclusive', 'effect_uncertain');
  }
  if (typeof response?.success !== 'boolean') {
    throw classifiedError('service_response_malformed', `${plannedRequestId} returned malformed JSON`,
      'harness_failure', 'effect_uncertain', response);
  }
  if (response.success !== true) {
    throw classifiedError('service_request_failed', `${plannedRequestId} was not successful`,
      'inconclusive', 'no_effect', response);
  }
  const requestId = typeof response.id === 'string' && response.id.length > 0 ? response.id : null;
  try {
    const evidence = await observe(context.environment.serviceOrigin,
      context.caseId === 'A03' ? context.fixture.sharedLabel : action.actionId, fetch);
    assertReleasedOracle({ ...evidence, requestId, handle: opened.handle, subjectId });
  } catch (error) {
    error.effectState ??= 'effect_uncertain';
    throw error;
  }
  return { requestId, operationCorrelationId: plannedRequestId };
}

async function runClient(context, action, index) {
  const subjectId = `${context.manifest.campaignRunId}:${context.caseId}:${context.attempt.attemptId}:client-${String(index + 1).padStart(3, '0')}`;
  const fetch = context.transportFor({ action, subjectId, index });
  if (typeof fetch !== 'function') fail('client_transport_missing', action.actionId);
  try {
    const opened = await openClient(context, action, subjectId, fetch);
    return { action, subjectId, fetch, opened, error: null };
  } catch (error) {
    return { action, subjectId, fetch, opened: null, error };
  }
}

async function appendReceipt(store, receipt) {
  if (typeof store?.append !== 'function') fail('append_only_receipt_store_missing', receipt.actionId);
  await store.append(freeze(structuredClone(receipt)));
}

async function runAttempt({ schedule, manifest, attempt, receiptStore, transportFor, clock }) {
  const caseId = attempt.caseId;
  const environmentId = attempt.environmentIds?.[0] ?? attempt.environmentId;
  const environment = manifest.environments[environmentId];
  const fixture = manifest.fixtures?.[caseId]?.[environmentId];
  if (!fixture || typeof fixture.url !== 'string' || typeof fixture.profileId !== 'string' ||
      typeof fixture.sessionName !== 'string' ||
      (caseId !== 'A01' && typeof fixture.browserId !== 'string') ||
      (caseId === 'A03' && typeof fixture.sharedLabel !== 'string')) {
    fail('frozen_fixture_missing', `${caseId}/${environmentId} fixture is incomplete`);
  }
  const actions = clientActions(schedule, caseId, attempt.attemptId);
  const context = { schedule, manifest, attempt, caseId, environmentId, environment, fixture, transportFor };
  const rows = [];
  const attemptFailures = [];
  const record = async (client) => {
    let terminal;
    try {
      if (client.error) throw client.error;
      const released = await releaseClient(context, client.action, client.subjectId, client.fetch, client.opened);
      terminal = {
        schemaVersion: 'agent-browser.p158-w7-action-receipt.v1',
        campaignRunId: manifest.campaignRunId,
        caseId,
        attemptId: attempt.attemptId,
        actionId: client.action.actionId,
        environmentId,
        clientSubjectId: client.subjectId,
        connectionInstanceId: client.opened.connectionInstanceId,
        browserId: client.opened.handle.browserId,
        sessionId: client.opened.handle.sessionName ?? client.opened.handle.ownerSessionId,
        tabId: client.opened.handle.tabId,
        openRequestId: client.opened.openRequestId,
        releaseRequestId: released.requestId,
        openOperationCorrelationId: client.opened.openOperationCorrelationId,
        releaseOperationCorrelationId: released.operationCorrelationId,
        requestCorrelations: [
          { operationCorrelationId: client.opened.openOperationCorrelationId,
            productRequestId: client.opened.openRequestId },
          ...(client.foreignProbeOperationCorrelationId ? [{
            operationCorrelationId: client.foreignProbeOperationCorrelationId,
            productRequestId: client.foreignProbeRequestId ?? null,
          }] : []),
          { operationCorrelationId: released.operationCorrelationId,
            productRequestId: released.requestId },
        ],
        state: 'passed',
        attempt: 1,
        observedAt: clock(),
        repairAttempted: false,
        retryAttempted: false,
      };
    } catch (error) {
      const failure = normalizeFailure(error, client.opened ? 'effect_uncertain' : 'no_effect');
      terminal = {
        schemaVersion: 'agent-browser.p158-w7-action-receipt.v1',
        campaignRunId: manifest.campaignRunId, caseId, attemptId: attempt.attemptId,
        actionId: client.action.actionId, environmentId, clientSubjectId: client.subjectId,
        connectionInstanceId: client.opened?.connectionInstanceId ?? null,
        browserId: client.opened?.handle.browserId ?? null,
        sessionId: client.opened ? (client.opened.handle.sessionName ?? client.opened.handle.ownerSessionId) : null,
        tabId: client.opened?.handle.tabId ?? null, openRequestId: client.opened?.openRequestId ?? null,
        releaseRequestId: null,
        openOperationCorrelationId: client.opened?.openOperationCorrelationId ?? null,
        releaseOperationCorrelationId: null, state: 'failed', attempt: 1, observedAt: clock(),
        requestCorrelations: [
          ...(client.opened?.openOperationCorrelationId ? [{
            operationCorrelationId: client.opened.openOperationCorrelationId,
            productRequestId: client.opened.openRequestId ?? null,
          }] : []),
          ...(client.foreignProbeOperationCorrelationId ? [{
            operationCorrelationId: client.foreignProbeOperationCorrelationId,
            productRequestId: client.foreignProbeRequestId ?? null,
          }] : []),
        ],
        effectState: failure.effectState,
        failure,
        repairAttempted: false, retryAttempted: false,
      };
    }
    terminal.receiptSha256 = sha256(terminal);
    await appendReceipt(receiptStore, terminal);
    rows.push(terminal);
  };

  const concurrent = caseId !== 'A01'
    ? actions
    : actions.filter((action) => action.cardinalityId === 'concurrent_clients');
  const sequential = caseId === 'A01'
    ? actions.filter((action) => action.cardinalityId === 'sequential_clients')
    : [];
  for (const [index, action] of sequential.entries()) {
    const client = await runClient(context, action, index);
    await record(client);
  }
  const concurrentClients = await Promise.all(concurrent.map((action, index) =>
    runClient(context, action, sequential.length + index)));

  if (caseId === 'A02') {
    const opened = concurrentClients.filter((row) => row.opened);
    const browserIds = new Set(opened.map((row) => row.opened.handle.browserId));
    const tabIds = new Set(opened.map((row) => row.opened.handle.tabId));
    if (opened.length === 10 &&
        (browserIds.size !== 1 || !browserIds.has(fixture.browserId) || tabIds.size !== 10)) {
      const error = new P158W7A01A03Error(
        'shared_browser_barrier_oracle_failed',
        `${attempt.attemptId} did not reach the ten-client barrier`,
      );
      for (const client of concurrentClients) client.error ??= error;
    }
  }
  if (caseId === 'A03') {
    const opened = concurrentClients.filter((row) => row.opened);
    if (opened.length === 10) {
      const connectionIds = new Set(opened.map((row) => row.opened.connectionInstanceId));
      if (connectionIds.size !== 10 || new Set(opened.map((row) => row.subjectId)).size !== 10) {
        const error = new P158W7A01A03Error(
          'same_label_connection_oracle_failed',
          `${attempt.attemptId} did not preserve ten connections`,
        );
        for (const client of opened) client.error ??= error;
      } else {
        for (let index = 0; index < opened.length; index += 1) {
          const own = opened[index];
          const foreign = opened[(index + 1) % opened.length];
          try {
            await revalidateOwnership(context, own.fetch);
            const requestId = `${manifest.campaignRunId}:${own.action.actionId}:foreign-probe`;
            own.foreignProbeOperationCorrelationId = requestId;
            const response = await postServiceRequest({
              baseUrl: environment.serviceOrigin,
              fetch: own.fetch,
              request: {
                ...commonRequest(context, own.action, own.subjectId, requestId),
                action: 'diagnostics',
                serviceTabHandle: foreign.opened.handle,
              },
            });
            own.foreignProbeRequestId = typeof response?.id === 'string' && response.id.length > 0
              ? response.id : null;
            if (response?.success !== false || response?.error?.code !== 'profile_access_denied') {
              fail('cross_client_theft_oracle_failed', `${requestId} was not denied`);
            }
          } catch (error) {
            own.error ??= error;
          }
        }
      }
    }
  }
  await Promise.all(concurrentClients.map(record));
  if (caseId === 'A02' && concurrentClients.some((row) => row.opened)) {
    try {
      const status = await getServiceStatus({
        baseUrl: environment.serviceOrigin,
        fetch: concurrentClients.find((row) => row.opened).fetch,
      });
      const expectedOwnership = structuredClone(environment.ownershipStatus);
      if (expectedOwnership.service_state) delete expectedOwnership.service_state.browsers;
      exactSubset(status, expectedOwnership);
      const browser = status.service_state?.browsers?.[fixture.browserId];
      if (!browser || !['ready', 'retained'].includes(browser.lifecycle)) {
        throw classifiedError('retained_browser_postcondition_failed',
          `${fixture.browserId} was not retained after the barrier`,
          'reproduced_historical_failure', 'verified_effect');
      }
    } catch (error) {
      attemptFailures.push(normalizeFailure(error, 'effect_uncertain'));
    }
  }
  const passed = rows.every((row) => row.state === 'passed') && attemptFailures.length === 0;
  const failureClasses = new Set([
    ...rows.filter((row) => row.failure).map((row) => row.failure.classification),
    ...attemptFailures.map((failure) => failure.classification),
  ]);
  const resultState = passed
    ? 'passed'
    : failureClasses.has('harness_failure')
      ? 'harness_failure'
      : failureClasses.has('inconclusive')
        ? 'inconclusive'
        : failureClasses.has('new_product_failure')
          ? 'new_product_failure'
          : 'reproduced_historical_failure';
  const effectState = rows.some((row) => row.effectState === 'effect_uncertain') ||
      attemptFailures.some((failure) => failure.effectState === 'effect_uncertain')
    ? 'effect_uncertain'
    : (rows.some((row) => row.state === 'passed') ? 'verified_effect' : 'no_effect');
  return freeze({
    resultState,
    actionCount: rows.length,
    actionIds: rows.map((row) => row.actionId).sort(),
    artifactIds: rows.map((row) => `p158-w7-action:${row.receiptSha256}`),
    receipts: rows.sort((left, right) => left.actionId.localeCompare(right.actionId)),
    attemptFailures,
    effectState,
    retryDisposition: 'prohibited_opportunistic_retry',
    repairAttempted: false,
    retryAttempted: false,
    garbageCollectionAttempted: false,
  });
}

function fetchResponse(status, headers, bytes) {
  return {
    ok: status >= 200 && status < 300,
    status,
    headers,
    async json() { return JSON.parse(bytes.toString('utf8')); },
    async text() { return bytes.toString('utf8'); },
    async arrayBuffer() { return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength); },
  };
}

/**
 * Create one keep-alive HTTP transport per frozen client action. Durable
 * ownership remains bound to clientSubjectId; connectionInstanceId is retained
 * as per-request transport provenance and is never promoted to durable identity.
 * Frame JSON by byte length because native Service ingress requires Content-Length.
 */
export function createP158W7PinnedDevelopmentTransports() {
  const transports = new Map();
  const agents = new Map();
  const factory = ({ action }) => {
    if (transports.has(action.actionId)) return transports.get(action.actionId);
    const agent = new Agent({ keepAlive: true, maxSockets: 1, maxFreeSockets: 1 });
    const fetch = async (input, init = {}) => {
      const url = new URL(input);
      if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)) {
        fail('development_service_origin_invalid', `Pinned transport refused ${url.origin}`);
      }
      const body = init.body === undefined ? null : Buffer.from(String(init.body));
      return new Promise((resolve, reject) => {
        const request = httpRequest(url, {
          agent,
          method: init.method ?? 'GET',
          headers: {
            ...init.headers,
            ...(body !== null ? { 'content-length': String(body.length) } : {}),
          },
          signal: init.signal,
        }, (response) => {
          const chunks = [];
          response.on('data', (chunk) => chunks.push(chunk));
          response.on('end', () => resolve(fetchResponse(
            response.statusCode ?? 0,
            response.headers,
            Buffer.concat(chunks),
          )));
        });
        request.on('error', reject);
        if (body) request.write(body);
        request.end();
      });
    };
    transports.set(action.actionId, fetch);
    agents.set(action.actionId, agent);
    return fetch;
  };
  Object.defineProperty(factory, LIVE_TRANSPORT_FACTORY, { value: true });
  Object.defineProperty(factory, 'close', {
    value: () => {
      for (const agent of agents.values()) agent.destroy();
      transports.clear();
      agents.clear();
    },
  });
  return factory;
}

export function createP158W7A01A03LiveBundle({
  schedule,
  ownershipManifest,
  receiptStore,
  transportFor = createP158W7PinnedDevelopmentTransports(),
  clock = () => new Date().toISOString(),
}) {
  const manifest = validateOwnershipManifest(ownershipManifest, schedule);
  if (typeof transportFor !== 'function' || typeof clock !== 'function') {
    fail('live_dependency_missing', 'A01-A03 require transportFor and clock functions');
  }
  const contracts = new Map(schedule.caseContracts.map((contract) => [contract.caseId, contract]));
  const adapters = P158_W7_A01_A03_CASE_IDS.map((caseId) => {
    const contract = contracts.get(caseId);
    if (!contract) fail('case_contract_missing', caseId);
    return createP158CaseAdapter({
      caseId,
      evidenceProfile: contract.evidenceProfile,
      executionContract: contract.executionContract,
      execute: async ({ attempt }) => runAttempt({
        schedule, manifest, attempt, receiptStore, transportFor, clock,
      }),
    });
  });
  const source = freeze({ sourcePath: P158_W7_A01_A03_SOURCE_PATH, sourceSha256: sourceSha256() });
  const loggingRequestExpectations = enumerateP158W7A01A03LoggingRequests({
    schedule, campaignRunId: manifest.campaignRunId,
  });
  return freeze({
    schemaVersion: 'agent-browser.p158-w7-a01-a03-live-bundle.v1',
    freezeEligible: transportFor[LIVE_TRANSPORT_FACTORY] === true,
    providerFree: false,
    concreteCaseIds: [...P158_W7_A01_A03_CASE_IDS],
    adapters,
    ownershipManifestSha256: manifest.manifestSha256,
    campaignRunId: manifest.campaignRunId,
    candidateSha256: manifest.candidateSha256,
    liveHookManifestSha256: manifest.liveHookManifestSha256,
    environmentSealSha256s: structuredClone(manifest.environmentSealSha256s),
    liveHookIds: [P158_W7_A01_A03_HOOK_ID],
    loggingRequestExpectations,
    driverSource: source,
    adapterBindingSha256: sha256({
      caseIds: P158_W7_A01_A03_CASE_IDS,
      ownershipManifestSha256: manifest.manifestSha256,
      campaignRunId: manifest.campaignRunId,
      candidateSha256: manifest.candidateSha256,
      liveHookManifestSha256: manifest.liveHookManifestSha256,
      environmentSealSha256s: manifest.environmentSealSha256s,
      source,
      liveHookIds: [P158_W7_A01_A03_HOOK_ID],
    }),
  });
}

export function enumerateP158W7A01A03LoggingRequests({ schedule, campaignRunId }) {
  return freeze(schedule.attempts.filter((attempt) => P158_W7_A01_A03_CASE_IDS.includes(attempt.caseId))
    .flatMap((attempt) => clientActions(schedule, attempt.caseId, attempt.attemptId).flatMap((action) => {
      const suffixes = ['open', ...(attempt.caseId === 'A03' ? ['foreign-probe'] : []), 'release'];
      const environmentId = attempt.environmentIds[0];
      return suffixes.map((suffix) => {
        const operationCorrelationId = `${campaignRunId}:${action.actionId}:${suffix}`;
        return {
          expectationId: operationCorrelationId, operationCorrelationId,
          productRequestId: null, productRequestIdState: 'assigned_at_runtime',
          requestKind: suffix === 'foreign-probe'
            ? 'rejected_request' : 'accepted_request',
          actionId: action.actionId, attemptId: attempt.attemptId, caseId: attempt.caseId,
          phaseId: 'W7', environmentId,
        };
      });
    })));
}

export function p158W7A01A03SourceBinding() {
  return freeze({
    hookId: P158_W7_A01_A03_HOOK_ID,
    sourcePath: P158_W7_A01_A03_SOURCE_PATH,
    sourceSha256: sourceSha256(),
  });
}

export function createP158W7A01A03OwnershipManifest(input) {
  const body = structuredClone(input);
  return freeze({ ...body, manifestSha256: sha256(body) });
}
