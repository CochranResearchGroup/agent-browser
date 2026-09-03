#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  installDashboardFetchFailureInstrumentation,
} from '../packages/dashboard/src/lib/failure-observation.ts';

const originalWindow = globalThis.window;
const originalLocation = globalThis.location;
const originalFetch = globalThis.fetch;
const calls = [];

globalThis.window = globalThis;
globalThis.location = { href: 'https://dashboard.example.test/service' };
globalThis.fetch = async (input, init = {}) => {
  const path = new URL(String(input), globalThis.location.href).pathname;
  calls.push({ path, init: structuredClone(init) });
  if (path === '/api/service/failure-observation') {
    return new Response(JSON.stringify({ success: true }), {
      status: 202,
      headers: { 'content-type': 'application/json' },
    });
  }
  if (path === '/api/service/request' && init.method !== 'DELETE') {
    return new Response(JSON.stringify({ success: false, error: 'synthetic rejection' }), {
      status: 409,
      headers: { 'content-type': 'application/json' },
    });
  }
  throw new Error('synthetic network failure');
};

const restore = installDashboardFetchFailureInstrumentation();
try {
  const response = await globalThis.fetch('/api/service/request', {
    method: 'POST',
    headers: { Authorization: 'Bearer must-not-be-copied' },
    body: JSON.stringify({ action: 'remote_view_open', target: 'must-not-be-copied' }),
  });
  assert.equal(response.status, 409);
  await new Promise((resolve) => setTimeout(resolve, 0));

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
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(calls.length, 2);
  const rejectedReport = JSON.parse(calls[1].init.body);
  assert.equal(rejectedReport.code, 'fetch_rejected');
  assert.equal(rejectedReport.action, 'service_session_delete');
} finally {
  restore();
  globalThis.fetch = originalFetch;
  globalThis.location = originalLocation;
  globalThis.window = originalWindow;
}

process.stdout.write('Dashboard failure observation behavior passed\n');
