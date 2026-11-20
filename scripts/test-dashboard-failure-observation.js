#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  DashboardFailureDeliveryQueue,
  installDashboardFetchFailureInstrumentation,
  MAX_PENDING_DASHBOARD_READ_FAILURES,
} from '../packages/dashboard/src/lib/failure-observation.ts';

const originalWindow = globalThis.window;
const originalLocation = globalThis.location;
const originalDocument = globalThis.document;
const originalFetch = globalThis.fetch;

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));
const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
assert.match(
  dashboardPage,
  /useEffect\(\(\) => installDashboardFetchFailureInstrumentation\(\), \[\]\)/,
  'DashboardExperience must install the tested fetch observer for the document lifetime',
);

async function withInstrumentedFetch(fetcher, run, visibility = 'visible') {
  const documentState = { visibilityState: visibility };
  globalThis.window = globalThis;
  globalThis.location = {
    href: 'https://dashboard.example.test/service',
    origin: 'https://dashboard.example.test',
  };
  globalThis.document = documentState;
  globalThis.fetch = fetcher;
  let restore = installDashboardFetchFailureInstrumentation();
  try {
    await run({
      documentState,
      reinstall() {
        restore();
        restore = installDashboardFetchFailureInstrumentation();
      },
    });
  } finally {
    restore();
    globalThis.fetch = originalFetch;
    globalThis.location = originalLocation;
    globalThis.document = originalDocument;
    globalThis.window = originalWindow;
  }
}

function recordedObservation(init={}) {
 const observation=JSON.parse(init.body??'{}');
 return jsonResponse({success:true,data:{schemaVersion:'agent-browser.service-failure-record.v1',recorded:true,occurrenceId:'11111111-1111-4111-8111-111111111111',originalClientObservationId:observation.observationId}},202);
}
function jsonResponse(payload = { success: true }, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

async function testExistingActionObservation() {
  const calls = [];
  await withInstrumentedFetch(async (input, init = {}) => {
    const path = new URL(String(input), globalThis.location.href).pathname;
    calls.push({ path, init: structuredClone(init) });
    if (path === '/api/service/failure-observation') return recordedObservation(init);
    if (path === '/api/service/request' && init.method !== 'DELETE') {
      return jsonResponse({ success: false, error: 'synthetic rejection' }, 409);
    }
    throw new Error('synthetic network failure');
  }, async () => {
    const response = await globalThis.fetch('/api/service/request', {
      method: 'POST',
      headers: { Authorization: 'Bearer must-not-be-copied' },
      body: JSON.stringify({ action: 'remote_view_open', target: 'must-not-be-copied' }),
    });
    assert.equal(response.status, 409);
    await tick();

    assert.equal(calls.length, 2);
    const report = JSON.parse(calls[1].init.body);
    assert.equal(report.category, 'dashboard_action');
    assert.equal(report.stage, 'http_action');
    assert.equal(report.code, 'http_409');
    assert.equal(report.action, 'remote_view_open');
    assert(!calls[1].init.body.includes('must-not-be-copied'));
    assert(!calls[1].init.body.includes('Bearer'));

    calls.length = 0;
    await assert.rejects(() => globalThis.fetch('/api/service/request', {
      method: 'DELETE',
      body: JSON.stringify({ action: 'service_session_delete' }),
    }), /synthetic network failure/);
    await tick();
    assert.equal(calls.length, 2);
    const rejectedReport = JSON.parse(calls[1].init.body);
    assert.equal(rejectedReport.code, 'fetch_rejected');
    assert.equal(rejectedReport.action, 'service_session_delete');
  });
}

async function testReadFailuresDeliverOnlyAfterRecovery() {
  const calls = [];
  const reports = [];
  let recovered = false;
  let clock = 0;
  const previousPerformance = globalThis.performance;
  globalThis.performance = { now: () => clock };
  try {
  await withInstrumentedFetch(async (input, init = {}) => {
    clock += 17.25;
    const parsed = new URL(String(input), globalThis.location.href);
    calls.push({ path: parsed.pathname, search: parsed.search, init: structuredClone(init) });
    if (parsed.pathname === '/api/service/failure-observation') {
      reports.push(JSON.parse(init.body));
      return recordedObservation(init);
    }
    if (recovered) return jsonResponse({ success: true, data: {} });
    if (parsed.pathname === '/api/service/status') {
      return new Response('<html>gateway timeout secret-body</html>', {
        status: 504,
        headers: { 'content-type': 'text/html' },
      });
    }
    if (parsed.pathname === '/api/service/resources') {
      return new Response('bad gateway secret-body', {
        status: 502,
        headers: { 'content-type': 'text/plain' },
      });
    }
    if (parsed.pathname === '/api/runtime/health') {
      return new Response('<html>unexpected secret-body</html>', {
        status: 200,
        headers: { 'content-type': 'text/html' },
      });
    }
    if (parsed.pathname === '/api/session-tabs') throw new Error('offline secret-message');
    throw new Error(`unexpected ${parsed.pathname}`);
  }, async ({ reinstall }) => {
    const status = await globalThis.fetch('/api/service/status?projection=dashboard-summary&secret=query');
    const resources = await globalThis.fetch('/api/service/resources');
    const nonJson = await globalThis.fetch('/api/runtime/health');
    await assert.rejects(() => globalThis.fetch('/api/session-tabs?port=9222&token=secret'), /offline/);
    assert.equal(status.status, 504);
    assert.equal(resources.status, 502);
    assert.equal(await nonJson.text(), '<html>unexpected secret-body</html>');
    await tick();
    assert.equal(reports.length, 0, 'failed reads must not cause an eager reporting storm');
    assert.equal(calls.length, 4, 'the observer must not retry failed reads');

    reinstall();
    recovered = true;
    clock += 10000;
    await globalThis.fetch('/api/service/status?projection=dashboard-summary');
    await tick();
    await tick();
    assert.deepEqual(reports.map((report) => [report.action, report.code]), [
      ['service_status_read', 'http_504'],
      ['service_resources_read', 'http_502'],
      ['runtime_health_read', 'response_non_json'],
      ['session_tabs_read', 'fetch_rejected'],
    ]);
    assert.deepEqual(reports.map(report => report.elapsedMs), [17, 17, 17, 17],
      'failure duration must use integer wire milliseconds and exclude deferred delivery');
    const serializedReports = JSON.stringify(reports);
    for (const forbidden of ['secret-body', 'secret-message', 'token=secret', 'secret=query', '9222']) {
      assert(!serializedReports.includes(forbidden), `failure report leaked ${forbidden}`);
    }
    assert(reports.every((report) => typeof report.observationId === 'string'));
  });
  } finally {
    globalThis.performance = previousPerformance;
  }
}

async function testQueueDedupeAndGapAccounting() {
  const reports = [];
  const queue = new DashboardFailureDeliveryQueue({
    maxPending: 2,
    isVisible: () => true,
    fetcher: async (_input, init = {}) => {
      reports.push(JSON.parse(init.body));
      return recordedObservation(init);
    },
  });
  const base = {
    category: 'dashboard_action',
    stage: 'http_read',
    code: 'http_504',
    summary: 'bounded',
    action: 'service_status_read',
  };
  queue.enqueue({ ...base, observationId: 'occurrence-1' });
  queue.enqueue({ ...base, observationId: 'occurrence-1' });
  queue.enqueue({ ...base, observationId: 'occurrence-2' });
  queue.enqueue({ ...base, observationId: 'occurrence-3' });
  assert.deepEqual(queue.counts(), { dropped: 1, pending: 2 });
  await Promise.all([queue.flushAfterRecovery(), queue.flushAfterRecovery()]);
  assert.equal(reports.length, 3, 'concurrent recovery must use one delivery flight');
  assert.equal(reports[0].code, 'dashboard_read_failure_delivery_gap');
  assert.equal(reports[0].summary, 'Dashboard read failure delivery queue dropped 1 occurrence.');
  assert.deepEqual(reports.slice(1).map((report) => report.observationId), [
    'occurrence-2',
    'occurrence-3',
  ]);
  assert.deepEqual(queue.counts(), { dropped: 0, pending: 0 });
}

async function testFrozenCampaignCeilingHasNoGap() {
  const reports = [];
  const queue = new DashboardFailureDeliveryQueue({
    isVisible: () => true,
    fetcher: async (_input, init = {}) => {
      reports.push(JSON.parse(init.body));
      return recordedObservation(init);
    },
  });
  for (let index = 0; index < MAX_PENDING_DASHBOARD_READ_FAILURES; index += 1) {
    queue.enqueue({
      category: 'dashboard_action',
      stage: 'http_read',
      code: 'http_504',
      summary: 'bounded campaign occurrence',
      action: 'service_status_read',
      observationId: `campaign-occurrence-${index}`,
    });
  }
  assert.deepEqual(queue.counts(), {
    dropped: 0,
    pending: MAX_PENDING_DASHBOARD_READ_FAILURES,
  });
  await queue.flushAfterRecovery();
  assert.equal(reports.length, MAX_PENDING_DASHBOARD_READ_FAILURES);
  assert(!reports.some((report) => report.code === 'dashboard_read_failure_delivery_gap'));
}

async function testFailedDeliveryRetainedUntilNextRecovery() {
  const attempts = [];
  let accept = false;
  const queue = new DashboardFailureDeliveryQueue({
    maxPending: 4,
    isVisible: () => true,
    fetcher: async (_input, init = {}) => {
      attempts.push(JSON.parse(init.body));
      return accept?recordedObservation(init):jsonResponse({},503);
    },
  });
  queue.enqueue({
    category: 'dashboard_action',
    stage: 'http_read',
    code: 'fetch_rejected',
    summary: 'offline',
    action: 'runtime_health_read',
    observationId: 'retained-occurrence',
  });
  await queue.flushAfterRecovery();
  assert.deepEqual(queue.counts(), { dropped: 0, pending: 1 });
  accept = true;
  await queue.flushAfterRecovery();
  assert.deepEqual(queue.counts(), { dropped: 0, pending: 0 });
  assert.deepEqual(attempts.map((attempt) => attempt.observationId), [
    'retained-occurrence',
    'retained-occurrence',
  ]);
}

async function testHiddenPageDoesNotObserveOrFlushReads() {
  const reports = [];
  let recovered = false;
  await withInstrumentedFetch(async (input, init = {}) => {
    const path = new URL(String(input), globalThis.location.href).pathname;
    if (path === '/api/service/failure-observation') {
      reports.push(JSON.parse(init.body));
      return recordedObservation(init);
    }
    if (!recovered) return new Response('unavailable', { status: 504 });
    return jsonResponse({ success: true });
  }, async ({ documentState }) => {
    await globalThis.fetch('/api/service/status');
    documentState.visibilityState = 'visible';
    recovered = true;
    await globalThis.fetch('/api/service/status');
    await tick();
    await tick();
    assert.equal(reports.length, 0, 'hidden-page reads must not create deferred failures');
  }, 'hidden');
}

try {
  await testExistingActionObservation();
  await testReadFailuresDeliverOnlyAfterRecovery();
  await testQueueDedupeAndGapAccounting();
  await testFrozenCampaignCeilingHasNoGap();
  await testFailedDeliveryRetainedUntilNextRecovery();
  await testHiddenPageDoesNotObserveOrFlushReads();
} finally {
  globalThis.fetch = originalFetch;
  globalThis.location = originalLocation;
  globalThis.document = originalDocument;
  globalThis.window = originalWindow;
}

process.stdout.write('Dashboard failure observation behavior passed\n');

const projected = [];
await withInstrumentedFetch(async (input, init) => {
  if (String(input).includes('failure-observation')) {projected.push(JSON.parse(init.body));return recordedObservation(init);}
  return new Response(JSON.stringify({success:true,data:{ok:false,failure:{code:'browser_task_operation_outcome_uncertain',occurrenceId:'failure-original-760',privateValue:'never-log-this'}}}), {headers:{'content-type':'application/json'}});
}, async()=> {
  const response=await fetch('/api/command',{method:'POST'});
  assert.equal((await response.json()).data.ok,false,'observer preserves caller body');
  await tick();await tick();
});
assert.equal(projected[0]?.code,'browser_task_operation_outcome_uncertain');
assert.equal(projected[0]?.observationId,'failure-original-760');
assert.ok(!JSON.stringify(projected).includes('never-log-this'));

const {dashboardConfirmationFailure,dashboardLocalFailure}=await import('../packages/dashboard/src/lib/response-failure.ts');
const {reportDashboardComponentFailure}=await import('../packages/dashboard/src/lib/failure-observation.ts');
const {inspectDashboardBrowserOperation}=await import('../packages/dashboard/src/lib/browser-operation-readback.ts');
let ownerReads=0;
await inspectDashboardBrowserOperation({action:'browser_session_mode_convert',sessionId:'task-original',operationId:'operation-original'},async(_url,init)=>{ownerReads++;assert.equal(JSON.parse(init.body).choice,'inspect');return new Response('{}');});
assert.throws(()=>inspectDashboardBrowserOperation({action:'browser_session_mode_convert',choice:'apply'},async()=>{ownerReads++;return new Response('{}');}),error=>error instanceof TypeError&&error.code==='dashboard_browser_inspection_scope_invalid');
assert.equal(ownerReads,1,'inspection cannot accept or dispatch a mutation choice');
const ownerFailure={code:'browser_task_operation_outcome_uncertain',occurrenceId:'owner-publication-760',recourse:{recoveryRegistry:{recipe:'original'}}};
const realOwnerLogging={state:'owner_recorded',receipt:{id:'original-owner-log'}};
const publication=dashboardConfirmationFailure({success:true,logging:realOwnerLogging,data:{ok:false,failure:ownerFailure}},'unconfirmed_exact_close_receipt',{taskId:'task-original',operationId:'operation-original'});
assert.equal(publication.failure,ownerFailure);assert.equal(publication.logging,realOwnerLogging);assert.equal(publication.effectState,'unknown');assert.equal(publication.originalOperationId,'operation-original');assert.equal(publication.clientRecovery.coverage,'registered');
const guard=dashboardLocalFailure(new TypeError('profileId is required'),'dashboard_argument_invalid','fixture-source','fixture-function');
assert.equal(guard.clientLogging.state,'not_attempted');assert.equal(guard.clientLogging.ownerOccurrenceId,null);
const fetchBeforePublication=globalThis.fetch;const publicationEvents=[];
try {
 globalThis.fetch=async(_url,init)=>{publicationEvents.push(JSON.parse(init.body));return new Response('{}',{status:503});};
 await reportDashboardComponentFailure(guard,'local_guard');assert.equal(publicationEvents.length,0);
 await reportDashboardComponentFailure(publication,'exact_close');assert.equal(publication.clientLogging.state,'observer_receipt_unverified');assert.equal(publication.clientLogging.ownerOccurrenceId,ownerFailure.occurrenceId);assert.equal(publication.occurrenceId,ownerFailure.occurrenceId);assert.ok(!JSON.stringify(publicationEvents).includes('operation-original'));
 globalThis.fetch=async(_url,init)=>recordedObservation(init);await reportDashboardComponentFailure(publication,'exact_close');assert.equal(publication.clientLogging.state,'observer_receipt_confirmed');assert.equal(publication.clientLogging.observerReceipt.data.recorded,true);assert.equal(publication.clientLogging.observerReceipt.data.occurrenceId,'11111111-1111-4111-8111-111111111111');assert.equal(publication.clientLogging.durableOwnerJournal,'not_asserted');assert.equal(publication.logging,realOwnerLogging);
 for(const payload of [{success:false},{success:true,data:{recorded:false}},{success:true,data:{ok:false}},{success:true,data:{recorded:true}}]){globalThis.fetch=async()=>jsonResponse(payload);await reportDashboardComponentFailure(publication,'malformed_receipt');assert.equal(publication.clientLogging.state,'observer_receipt_unverified');assert.equal(publication.clientLogging.observerReceipt,null);assert.equal(publication.failure,ownerFailure);}
 for(const extra of [{failure:{code:'journal_write_failed'}},{logging:{state:'failed'}},{logging:{success:false}}]){globalThis.fetch=async()=>jsonResponse({success:true,...extra,data:{schemaVersion:'agent-browser.service-failure-record.v1',recorded:true,occurrenceId:'11111111-1111-4111-8111-111111111111'}});await reportDashboardComponentFailure(publication,'explicit_logging_failure');assert.equal(publication.clientLogging.state,'observer_receipt_unverified');assert.equal(publication.clientLogging.observerReceipt,null);}
 globalThis.fetch=async()=>new Response('not-json');await reportDashboardComponentFailure(publication,'missing_json');assert.equal(publication.clientLogging.state,'observer_receipt_unverified');
 await reportDashboardComponentFailure(Object.freeze(publication),'frozen_original');
} finally {globalThis.fetch=fetchBeforePublication;}
