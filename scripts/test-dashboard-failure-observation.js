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
    if (path === '/api/service/failure-observation') return jsonResponse({}, 202);
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
  await withInstrumentedFetch(async (input, init = {}) => {
    const parsed = new URL(String(input), globalThis.location.href);
    calls.push({ path: parsed.pathname, search: parsed.search, init: structuredClone(init) });
    if (parsed.pathname === '/api/service/failure-observation') {
      reports.push(JSON.parse(init.body));
      return jsonResponse({}, 202);
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
    await globalThis.fetch('/api/service/status?projection=dashboard-summary');
    await tick();
    await tick();
    assert.deepEqual(reports.map((report) => [report.action, report.code]), [
      ['service_status_read', 'http_504'],
      ['service_resources_read', 'http_502'],
      ['runtime_health_read', 'response_non_json'],
      ['session_tabs_read', 'fetch_rejected'],
    ]);
    const serializedReports = JSON.stringify(reports);
    for (const forbidden of ['secret-body', 'secret-message', 'token=secret', 'secret=query', '9222']) {
      assert(!serializedReports.includes(forbidden), `failure report leaked ${forbidden}`);
    }
    assert(reports.every((report) => typeof report.observationId === 'string'));
  });
}

async function testQueueDedupeAndGapAccounting() {
  const reports = [];
  const queue = new DashboardFailureDeliveryQueue({
    maxPending: 2,
    isVisible: () => true,
    fetcher: async (_input, init = {}) => {
      reports.push(JSON.parse(init.body));
      return jsonResponse({}, 202);
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
      return jsonResponse({}, 202);
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
      return jsonResponse({}, accept ? 202 : 503);
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
      return jsonResponse({}, 202);
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
