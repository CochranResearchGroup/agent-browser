#!/usr/bin/env node

import assert from 'node:assert/strict';

import { sha256 } from './lib/p158-campaign-controller.js';
import {
  executeP158W7A11PredispatchProbe,
  p158W7A11PredispatchSourceBinding,
  P158W7A11PredispatchError,
} from './lib/p158-w7-a11-predispatch-live.js';

const requestId = 'http-service-request-tab_new-a11-fixture';
const occurrenceId = 'a11-occurrence-1';
const failure = {
  schemaVersion: 'agent-browser.service-failure-record.v1', occurrenceId,
  occurredAt: '2026-09-03T12:00:01.000Z', bootEpoch: 'boot-a11',
  runtimeEnvironment: 'development', category: 'service_action', source: 'http_service_request',
  stage: 'ingress_validation', code: 'invalid_bounded_recipe',
  summary: 'tab_new cannot execute remote-view route intent; use authenticated remote_view_open to acquire the route and serviceTabHandle',
  action: 'tab_new', references: { requestId, sessionId: 'p158-a11' },
};
const input = {
  runId: 'p158-a11-run', candidateSha256: '11'.repeat(32),
  environment: { environmentId: 'E1', runtimeLane: 'development', production: false,
    serviceOrigin: 'http://127.0.0.1:48158' },
  environmentSealSha256: '22'.repeat(32),
  clock: () => '2026-09-03T12:00:02.000Z',
};

function response(status, payload) {
  return { status, ok: status >= 200 && status < 300, async json() { return structuredClone(payload); } };
}

function fixtureFetch({ records = [failure], jobs = [] } = {}) {
  const calls = [];
  let journalReads = 0;
  const fetch = async (url, options = {}) => {
    const parsed = new URL(url);
    calls.push({ url: parsed.href, method: options.method ?? 'GET', body: options.body ?? null });
    if (parsed.pathname === '/api/service/failures') {
      journalReads += 1;
      return response(200, { success: true, data: {
        schemaVersion: 'agent-browser.service-failure-journal-readback.v1',
        records: journalReads === 1 ? [] : records,
        malformedLineCount: 0, writeFailureCount: 0,
      } });
    }
    if (parsed.pathname === '/api/service/request') {
      return response(400, { success: false,
        error: 'tab_new cannot execute remote-view route intent; use authenticated remote_view_open to acquire the route and serviceTabHandle' });
    }
    if (parsed.pathname === '/api/service/trace') {
      assert.equal(parsed.searchParams.get('requestId'), requestId);
      return response(200, { success: true, data: { jobs, events: [], incidents: [], outcomes: [] } });
    }
    throw new Error(`unexpected URL ${parsed.href}`);
  };
  return { fetch, calls };
}

const successful = fixtureFetch();
const receipt = await executeP158W7A11PredispatchProbe({ ...input, fetch: successful.fetch });
assert.equal(receipt.caseId, 'A11');
assert.equal(receipt.terminalBoundary, 'pre_dispatch_denial');
assert.equal(receipt.requestId, requestId);
assert.equal(receipt.occurrenceId, occurrenceId);
assert.equal(receipt.failureCode, 'invalid_bounded_recipe');
assert.equal(receipt.noJobCreated, true);
assert.equal(receipt.retryAttempted, false);
assert.equal(receipt.repairAttempted, false);
assert.equal(receipt.remainingA11TerminalBoundaries.length, 5);
const { receiptSha256, ...receiptBody } = receipt;
assert.equal(receiptSha256, sha256(receiptBody));
assert.equal(successful.calls.length, 4, 'the A11 probe must make one attempt and three observations');
assert.equal(successful.calls.filter((entry) => entry.method === 'POST').length, 1,
  'the rejected request must never be retried');
assert.equal(p158W7A11PredispatchSourceBinding().sourceSha256.length, 64);

const unrelated = fixtureFetch({ jobs: [{ id: 'unrelated-job',
  provenance: { requestId: 'another-request' } }] });
const unrelatedReceipt = await executeP158W7A11PredispatchProbe({ ...input, fetch: unrelated.fetch });
assert.equal(unrelatedReceipt.noJobCreated, true,
  'an unrelated job in a broad trace response must not be attributed to A11');

for (const [code, overrides] of [
  ['a11_failure_journal_correlation_invalid', { records: [] }],
  ['a11_predispatch_job_created', { jobs: [{ id: 'forbidden-job', provenance: { requestId } }] }],
]) {
  const fixture = fixtureFetch(overrides);
  await assert.rejects(executeP158W7A11PredispatchProbe({ ...input, fetch: fixture.fetch }),
    (error) => error instanceof P158W7A11PredispatchError && error.code === code);
  assert.equal(fixture.calls.filter((entry) => entry.method === 'POST').length, 1);
}

await assert.rejects(executeP158W7A11PredispatchProbe({
  ...input, environment: { ...input.environment, production: true }, fetch: fixtureFetch().fetch,
}), (error) => error instanceof P158W7A11PredispatchError &&
  error.code === 'a11_predispatch_identity_unproven');

process.stdout.write('P158 W7 A11 pre-dispatch journal correlation tests passed\n');
