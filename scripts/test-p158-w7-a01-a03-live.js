#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import fs from 'node:fs';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  createP158W7LiveDevelopmentAdapterBundle,
  enumerateP158W7ActionPlans,
} from './lib/p158-w7-development-adapters.js';
import {
  createP158W7A01A03LiveBundle,
  createP158W7A01A03OwnershipManifest,
  createP158W7PinnedDevelopmentTransports,
  P158_W7_A01_A03_CASE_IDS,
  P158_W7_A01_A03_HOOK_ID,
  P158W7A01A03Error,
} from './lib/p158-w7-a01-a03-live.js';

const registry = JSON.parse(fs.readFileSync(new URL(
  '../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url,
), 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-a01-a03-live' });

function json(response, status, value) {
  const body = JSON.stringify(value);
  response.writeHead(status, {
    'content-type': 'application/json',
    'content-length': Buffer.byteLength(body),
  });
  response.end(body);
}

async function requestBody(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  const bytes = Buffer.concat(chunks);
  // Native Service ingress frames request bodies by Content-Length.
  assert.equal(Number(request.headers['content-length']), bytes.length,
    'live transport must frame its JSON body for native Service ingress');
  return JSON.parse(bytes.toString('utf8'));
}

async function startFakeService({
  wrongStateRequestId = null,
  resetRequestId = null,
  malformedRequestId = null,
  wrongRetainedBrowserId = null,
} = {}) {
  const tabs = new Map();
  const browsers = new Map();
  const openedBrowsers = new Set();
  for (const caseId of ['a02', 'a03']) {
    for (const environmentId of ['e0', 'e1']) {
      const id = `p158-${caseId}-${environmentId}-browser`;
      browsers.set(id, { id, lifecycle: 'ready', retained: true });
    }
  }
  const jobs = [];
  const events = [];
  const sockets = new WeakMap();
  let socketOrdinal = 0;
  let tabOrdinal = 0;
  let requestCount = 0;
  const seenByAction = new Map();
  const server = createServer(async (request, response) => {
    requestCount += 1;
    if (!sockets.has(request.socket)) sockets.set(request.socket, `connection-${++socketOrdinal}`);
    const connectionInstanceId = sockets.get(request.socket);
    const url = new URL(request.url, 'http://127.0.0.1');
    if (request.method === 'GET' && url.pathname === '/api/service/status') {
      const projectedBrowsers = Object.fromEntries([...browsers].map(([id, browser]) => {
        const hasOpenTab = [...tabs.values()].some((tab) => tab.browserId === id && tab.lifecycle !== 'closed');
        const lostAfterRelease = id === wrongRetainedBrowserId && openedBrowsers.has(id) && !hasOpenTab;
        return [id, lostAfterRelease ? { ...browser, lifecycle: 'closed', retained: false } : browser];
      }));
      return json(response, 200, { success: true, data: {
        runtimeLifecycle: { environment: 'development', runId: 'p158-a01-a03-run' },
        service_state: {
          candidateSha256: 'a'.repeat(64),
          browsers: projectedBrowsers,
        },
      } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/tabs') {
      return json(response, 200, { success: true, data: { tabs: [...tabs.values()] } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/events') {
      const taskName = url.searchParams.get('taskName');
      return json(response, 200, { success: true, data: {
        events: events.filter((event) => !taskName || event.taskName === taskName),
      } });
    }
    if (request.method === 'GET' && url.pathname === '/api/service/trace') {
      const taskName = url.searchParams.get('taskName');
      return json(response, 200, { success: true, data: {
        jobs: jobs.filter((job) => !taskName || job.provenance.taskName === taskName),
      } });
    }
    if (request.method !== 'POST' || url.pathname !== '/api/service/request') {
      return json(response, 404, { success: false, error: { code: 'not_found' } });
    }
    const command = await requestBody(request);
    const operationId = `p158-a01-a03-run:${command.taskName}:${command.action === 'tab_new' ? 'open' :
      command.action === 'tab_handle_release' ? 'release' : 'foreign-probe'}`;
    const requestId = `service-request-${jobs.length + 1}`;
    if (operationId === resetRequestId) {
      request.socket.destroy();
      return;
    }
    if (operationId === malformedRequestId) {
      return json(response, 200, { malformed: true });
    }
    const jobId = `job-${jobs.length + 1}`;
    const provenance = {
      requestId,
      jobId,
      traceId: `trace-${requestId}`,
      clientSubjectId: command.clientSubjectId,
      identityAssurance: command.identityAssurance,
      connectionInstanceId,
      runtimeEnvironmentId: command.runtimeEnvironmentId,
      runtimeLaneId: 'development',
      profileId: command.profileId,
      browserId: command.browserId ?? null,
      sessionId: command.sessionName,
      tabId: command.serviceTabHandle?.tabId ?? null,
      serviceName: command.serviceName,
      agentName: command.agentName,
      taskName: command.taskName,
      action: command.action,
    };
    jobs.push({ id: jobId, state: 'completed', provenance });
    events.push({ id: `event-${events.length + 1}`, jobId, requestId, traceId: `trace-${requestId}`,
      taskName: command.taskName, kind: 'service_request_completed', provenance });
    seenByAction.set(command.action, (seenByAction.get(command.action) ?? 0) + 1);

    if (command.action === 'tab_new') {
      const tabId = `tab-${++tabOrdinal}`;
      const browserId = command.browserId ?? `browser-${tabOrdinal}`;
      openedBrowsers.add(browserId);
      browsers.set(browserId, { id: browserId, lifecycle: 'ready', retained: true });
      const handle = {
        browserId,
        sessionName: command.sessionName,
        tabId,
        targetId: `target-${tabOrdinal}`,
        url: command.params?.url ?? null,
        title: command.taskName,
        profileId: command.profileId,
        profileOrigin: 'agent_browser_owned',
        leaseId: `lease-${tabOrdinal}`,
        leaseState: 'shared',
        cleanupPolicy: 'close_tabs',
        leaseHeartbeatExpected: false,
        ownerSessionId: command.sessionName,
        profileAccess: {
          schemaVersion: 'agent-browser.profile-child-access.v1',
          parentPolicyRevision: 1,
          accessDecisionId: `decision-${tabOrdinal}`,
          subjectId: command.clientSubjectId,
          identityAssurance: command.identityAssurance,
          connectionInstanceId,
          connectionState: 'active',
          permissions: ['profile_use', 'tab_create', 'tab_control_own', 'tab_close_own'],
        },
        jobId,
        traceFilter: { browserId, profileId: command.profileId, sessionId: command.sessionName,
          serviceName: command.serviceName, agentName: command.agentName, taskName: command.taskName },
        valid: true,
        staleReason: null,
      };
      tabs.set(tabId, {
        id: tabId,
        browserId,
        targetId: handle.targetId,
        sessionId: command.sessionName,
        lifecycle: operationId === wrongStateRequestId ? 'closed' : 'ready',
        serviceTabHandle: handle,
      });
      return json(response, 200, { id: requestId, success: true, data: { serviceTabHandle: handle } });
    }
    if (command.action === 'diagnostics') {
      const ownerSubject = command.serviceTabHandle?.profileAccess?.subjectId;
      if (ownerSubject !== command.clientSubjectId) {
        return json(response, 200, { id: requestId, success: false, error: { code: 'profile_access_denied' } });
      }
      return json(response, 200, { id: requestId, success: true, data: {} });
    }
    if (command.action === 'tab_handle_release') {
      const handle = command.serviceTabHandle;
      const tab = tabs.get(handle?.tabId);
      if (!tab || handle.profileAccess?.subjectId !== command.clientSubjectId) {
        return json(response, 200, { id: requestId, success: false, error: { code: 'profile_access_denied' } });
      }
      tab.lifecycle = 'closed';
      tab.serviceTabHandle = { ...tab.serviceTabHandle, valid: false, leaseState: 'released' };
      return json(response, 200, { id: requestId, success: true, data: { serviceTabHandle: tab.serviceTabHandle } });
    }
    return json(response, 400, { success: false, error: { code: 'unexpected_action' } });
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return {
    origin: `http://127.0.0.1:${server.address().port}`,
    requestCount: () => requestCount,
    seenByAction,
    jobs: () => structuredClone(jobs),
    close: () => new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve())),
  };
}

function manifest(origin) {
  const fixture = (caseId, id) => ({
    url: `http://127.0.0.1:43158/${caseId.toLowerCase()}/${id.toLowerCase()}`,
    profileId: `p158-${caseId.toLowerCase()}-${id.toLowerCase()}-profile`,
    sessionName: `p158-${caseId.toLowerCase()}-${id.toLowerCase()}-session`,
    ...(caseId === 'A01' ? {} : { browserId: `p158-${caseId.toLowerCase()}-${id.toLowerCase()}-browser` }),
    ...(caseId === 'A03' ? { sharedLabel: 'same-label' } : {}),
  });
  const fixtures = Object.fromEntries(P158_W7_A01_A03_CASE_IDS.map((caseId) => [caseId, {
    E0: fixture(caseId, 'E0'), E1: fixture(caseId, 'E1'),
  }]));
  const environment = (id) => ({
    serviceOrigin: origin,
    runtimeLane: 'development',
    production: false,
    runtimeEnvironmentId: id,
    targetId: `p158-${id.toLowerCase()}-target`,
    ownershipStatus: {
      runtimeLifecycle: { environment: 'development', runId: 'p158-a01-a03-run' },
      service_state: {
        candidateSha256: 'a'.repeat(64),
        browsers: Object.fromEntries(['A02', 'A03'].map((caseId) => {
          const browserId = fixtures[caseId][id].browserId;
          return [browserId, { id: browserId, lifecycle: 'ready', retained: true }];
        })),
      },
    },
  });
  return createP158W7A01A03OwnershipManifest({
    schemaVersion: 'agent-browser.p158-w7-a01-a03-ownership.v1',
    campaignRunId: 'p158-a01-a03-run',
    candidateSha256: 'a'.repeat(64),
    liveHookManifestSha256: 'b'.repeat(64),
    environmentSealSha256s: { E0: 'c'.repeat(64), E1: 'd'.repeat(64) },
    environments: { E0: environment('E0'), E1: environment('E1') },
    fixtures,
  });
}

function memoryReceiptStore() {
  const receipts = [];
  const ids = new Set();
  return {
    receipts,
    async append(receipt) {
      assert(!ids.has(receipt.actionId), `duplicate action receipt ${receipt.actionId}`);
      ids.add(receipt.actionId);
      receipts.push(structuredClone(receipt));
    },
  };
}

async function runAll(bundle) {
  const results = [];
  for (const attempt of schedule.attempts.filter((entry) =>
    P158_W7_A01_A03_CASE_IDS.includes(entry.caseId))) {
    const adapter = bundle.adapters.find((entry) => entry.caseId === attempt.caseId);
    results.push({ attempt, result: await adapter.execute({ attempt }) });
  }
  return results;
}

const service = await startFakeService();
const store = memoryReceiptStore();
const transports = createP158W7PinnedDevelopmentTransports();
try {
  const frozenManifest = manifest(service.origin);
  const inputBefore = JSON.stringify(frozenManifest);
  const bundle = createP158W7A01A03LiveBundle({
    schedule,
    ownershipManifest: frozenManifest,
    receiptStore: store,
    transportFor: transports,
    clock: () => '2026-09-03T12:00:00.000Z',
  });
  assert.equal(bundle.freezeEligible, true);
  assert.deepEqual(bundle.concreteCaseIds, ['A01', 'A02', 'A03']);
  assert.deepEqual(bundle.liveHookIds, [P158_W7_A01_A03_HOOK_ID]);
  const actionPlans = enumerateP158W7ActionPlans({ schedule });
  const browserBindingsByActionId = Object.fromEntries(actionPlans
    .filter((action) => action.caseId === 'A07')
    .map((action, index) => [action.actionId, {
      pid: 6000 + index,
      browserId: `a07-browser-${index}`,
      profilePath: `/tmp/p158-a01-a03-run/a07-${index}`,
    }]));
  const desktopFixtureBindingsByActionId = Object.fromEntries(actionPlans
    .filter((action) => action.caseId === 'X06')
    .map((action, index) => [action.actionId, {
      browserId: `x06-browser-${index}`,
      profilePath: `/tmp/p158-a01-a03-run/x06-${index}`,
      displayName: `:${200 + index}`,
      locatorId: 'p110-control-v1',
      windowState: action.dimensionAssignments.find((entry) => entry.dimensionId === 'window_state').value,
    }]));
  const target = {
    targetId: 'p158-a01-a03-target', campaignRunId: 'p158-a01-a03-run',
    candidateSha256: 'a'.repeat(64),
    runtimeLane: 'development', isolationState: 'isolated', ownership: 'p158_campaign',
    production: false, foreign: false, tenantDataPresent: false, redactedComparisonOnly: true,
    disposableRoot: '/tmp/p158-a01-a03-run', browserBindingsByActionId,
    desktopFixtureBindingsByActionId,
    daemonUnit: 'agent-browser-development.service',
    supervisorUnit: 'agent-browser-development-supervisor.service',
    developmentBinary: '/opt/agent-browser-dev', agentBrowserId: 'agent-browser-main',
    agentProfilePath: '/tmp/p158-a01-a03-run/main', runtimeProfile: 'p158-a01-a03',
    sessionName: 'p158-a01-a03', localFixtureOrigin: 'http://127.0.0.1:43158',
    evidenceSince: '2026-09-03T00:00:00Z',
    allowedExecutables: ['/opt/agent-browser-dev', '/usr/bin/journalctl', '/usr/bin/systemctl', '/usr/bin/ps'],
    allowedSystemdUnits: ['agent-browser-development.service', 'agent-browser-development-supervisor.service'],
    allowedProcessIds: Object.values(browserBindingsByActionId).map((entry) => entry.pid),
    allowedBrowserIds: ['agent-browser-main',
      ...Object.values(browserBindingsByActionId).map((entry) => entry.browserId),
      ...Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.browserId)],
    allowedProfilePaths: ['/tmp/p158-a01-a03-run/main',
      ...Object.values(browserBindingsByActionId).map((entry) => entry.profilePath),
      ...Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.profilePath)],
    allowedDisplayNames: Object.values(desktopFixtureBindingsByActionId).map((entry) => entry.displayName),
  };
  const noop = async () => ({ resultState: 'passed', artifactIds: [] });
  const integrated = createP158W7LiveDevelopmentAdapterBundle({
    schedule,
    target,
    primitives: {
      captureEvidence: noop, captureLogs: noop, executeCli: noop, executeBrowser: noop,
      executeDisplay: noop, executeShutdown: noop, executeSystemd: noop, executeProcess: noop,
    },
    a01A03LiveBundle: bundle,
    liveHookManifestSha256: 'b'.repeat(64),
  });
  assert.deepEqual(integrated.adapterBindings.filter((entry) => ['A01', 'A02', 'A03'].includes(entry.caseId))
    .map((entry) => [entry.caseId, entry.mode, entry.hookIds]), [
      ['A01', 'concrete_live', [P158_W7_A01_A03_HOOK_ID]],
      ['A02', 'concrete_live', [P158_W7_A01_A03_HOOK_ID]],
      ['A03', 'concrete_live', [P158_W7_A01_A03_HOOK_ID]],
    ]);
  const results = await runAll(bundle);
  assert(results.every(({ result }) => result.resultState === 'passed'));
  assert(results.every(({ result }) => result.effectState === 'verified_effect'));
  assert.equal(JSON.stringify(frozenManifest), inputBefore, 'runner mutated its frozen ownership input');

  assert.equal(store.receipts.filter((row) => row.caseId === 'A01').length, 250);
  assert.equal(store.receipts.filter((row) => row.caseId === 'A02').length, 400);
  assert.equal(store.receipts.filter((row) => row.caseId === 'A03').length, 20);
  assert.equal(new Set(store.receipts.map((row) => row.actionId)).size, 670);
  assert(store.receipts.every((row) => row.attempt === 1 && row.repairAttempted === false &&
    row.retryAttempted === false && /^[a-f0-9]{64}$/u.test(row.receiptSha256)));
  const frozenRequestIds = new Set(bundle.loggingRequestExpectations.map((entry) => entry.operationCorrelationId));
  assert(store.receipts.every((row) => frozenRequestIds.has(row.openOperationCorrelationId) &&
    frozenRequestIds.has(row.releaseOperationCorrelationId)),
  'live operation correlation IDs differ from pre-freeze enumeration');
  assert(store.receipts.every((row) => row.openRequestId.startsWith('service-request-') &&
    row.releaseRequestId.startsWith('service-request-')),
  'live product request IDs must come from Service responses');
  for (const environmentId of ['E0', 'E1']) {
    const a01 = store.receipts.filter((row) => row.caseId === 'A01' && row.environmentId === environmentId);
    assert.equal(a01.length, 125);
    assert.equal(new Set(a01.map((row) => row.clientSubjectId)).size, 125);
    const a02 = store.receipts.filter((row) => row.caseId === 'A02' && row.environmentId === environmentId);
    assert.equal(a02.length, 200);
    assert.equal(new Set(a02.map((row) => row.attemptId)).size, 20);
    for (const rows of Object.values(Object.groupBy(a02, (row) => row.attemptId))) {
      assert.equal(rows.length, 10);
      assert.equal(new Set(rows.map((row) => row.browserId)).size, 1);
      assert.equal(new Set(rows.map((row) => row.tabId)).size, 10);
    }
    const a03 = store.receipts.filter((row) => row.caseId === 'A03' && row.environmentId === environmentId);
    assert.equal(a03.length, 10);
    assert.equal(new Set(a03.map((row) => row.clientSubjectId)).size, 10);
    assert.equal(new Set(a03.map((row) => row.connectionInstanceId)).size, 10);
    const a03Jobs = service.jobs().filter((job) =>
      job.provenance.runtimeEnvironmentId === environmentId && job.provenance.serviceName === 'p158-a03');
    assert.equal(new Set(a03Jobs.map((job) => job.provenance.taskName)).size, 1,
      'same-label clients did not retain one shared task label');
  }
  assert.equal(service.seenByAction.get('tab_new'), 670);
  assert.equal(service.seenByAction.get('tab_handle_release'), 670);
  assert.equal(service.seenByAction.get('diagnostics'), 20);
} finally {
  transports.close();
  await service.close();
}

const wrongActionId = schedule.attempts.find((entry) => entry.caseId === 'A01').cardinalityAllocations
  .find((entry) => entry.id === 'sequential_clients').actionIds[0];
const wrongRequestId = `p158-a01-a03-run:${wrongActionId}:open`;
const wrongService = await startFakeService({ wrongStateRequestId: wrongRequestId });
const wrongStore = memoryReceiptStore();
const wrongTransports = createP158W7PinnedDevelopmentTransports();
try {
  const bundle = createP158W7A01A03LiveBundle({
    schedule,
    ownershipManifest: manifest(wrongService.origin),
    receiptStore: wrongStore,
    transportFor: wrongTransports,
    clock: () => '2026-09-03T12:00:00.000Z',
  });
  const attempt = schedule.attempts.find((entry) => entry.caseId === 'A01');
  const result = await bundle.adapters.find((entry) => entry.caseId === 'A01').execute({ attempt });
  assert.equal(result.resultState, 'reproduced_historical_failure');
  assert.equal(result.effectState, 'effect_uncertain');
  assert.equal(result.receipts.length, 125, 'wrong-state response did not terminalize every scheduled client');
  const failed = result.receipts.find((row) => row.actionId === wrongActionId);
  assert.equal(failed.state, 'failed');
  assert.equal(failed.failure.code, 'open_state_oracle_failed');
  assert.equal(failed.attempt, 1);
  assert.equal(wrongService.seenByAction.get('tab_new'), 125, 'wrong state triggered a retry or stopped independent actions');
} finally {
  wrongTransports.close();
  await wrongService.close();
}

assert.throws(() => createP158W7A01A03LiveBundle({
  schedule,
  ownershipManifest: { ...manifest('http://127.0.0.1:43158'), manifestSha256: '0'.repeat(64) },
  receiptStore: memoryReceiptStore(),
}), (error) => error instanceof P158W7A01A03Error && error.code === 'frozen_ownership_manifest_invalid');

const injected = createP158W7A01A03LiveBundle({
  schedule,
  ownershipManifest: manifest('http://127.0.0.1:43158'),
  receiptStore: memoryReceiptStore(),
  transportFor: () => async () => { throw new Error('not called'); },
});
assert.equal(injected.freezeEligible, false, 'injected fake transport was promoted as concrete live');

for (const [fault, expectedState, expectedCode] of [
  ['reset', 'inconclusive', 'service_transport_failed'],
  ['malformed', 'harness_failure', 'service_response_malformed'],
]) {
  const requestId = `p158-a01-a03-run:${wrongActionId}:open`;
  const faultService = await startFakeService({
    ...(fault === 'reset' ? { resetRequestId: requestId } : { malformedRequestId: requestId }),
  });
  const faultStore = memoryReceiptStore();
  const faultTransports = createP158W7PinnedDevelopmentTransports();
  try {
    const bundle = createP158W7A01A03LiveBundle({
      schedule,
      ownershipManifest: manifest(faultService.origin),
      receiptStore: faultStore,
      transportFor: faultTransports,
      clock: () => '2026-09-03T12:00:00.000Z',
    });
    const attempt = schedule.attempts.find((entry) => entry.caseId === 'A01');
    const result = await bundle.adapters.find((entry) => entry.caseId === 'A01').execute({ attempt });
    assert.equal(result.resultState, expectedState);
    assert.notEqual(result.resultState, 'reproduced_historical_failure');
    assert.equal(result.effectState, 'effect_uncertain');
    assert.equal(result.receipts.length, 125);
    assert.equal(result.receipts.find((row) => row.actionId === wrongActionId).failure.code, expectedCode);
  } finally {
    faultTransports.close();
    await faultService.close();
  }
}

const a02Attempt = schedule.attempts.find((entry) => entry.caseId === 'A02');
const a02Environment = a02Attempt.environmentIds[0];
const a02BrowserId = manifest('http://127.0.0.1:43158').fixtures.A02[a02Environment].browserId;
const retainedService = await startFakeService({ wrongRetainedBrowserId: a02BrowserId });
const retainedStore = memoryReceiptStore();
const retainedTransports = createP158W7PinnedDevelopmentTransports();
try {
  const bundle = createP158W7A01A03LiveBundle({
    schedule,
    ownershipManifest: manifest(retainedService.origin),
    receiptStore: retainedStore,
    transportFor: retainedTransports,
    clock: () => '2026-09-03T12:00:00.000Z',
  });
  const result = await bundle.adapters.find((entry) => entry.caseId === 'A02').execute({ attempt: a02Attempt });
  assert.equal(result.resultState, 'reproduced_historical_failure');
  assert.equal(result.effectState, 'verified_effect');
  assert.equal(result.receipts.length, 10, 'retained-browser loss discarded action terminals');
  assert(result.receipts.every((receipt) => receipt.state === 'passed'));
  assert.equal(result.attemptFailures[0].code, 'retained_browser_postcondition_failed');
} finally {
  retainedTransports.close();
  await retainedService.close();
}

console.log('P158 W7 A01-A03 live runner tests passed: 670 exact client terminals plus classified fault rejection');
