#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceTabRequest,
  postServiceRequest,
  requestServiceTab,
} from '../packages/client/src/service-request.js';

function assertServiceRequestActionDataCoverage() {
  const schema = JSON.parse(
    readFileSync(
      new URL('../docs/dev/contracts/service-request.v1.schema.json', import.meta.url),
      'utf8',
    ),
  );
  const generatorSource = readFileSync(
    new URL('./generate-service-request-client.js', import.meta.url),
    'utf8',
  );
  const mapBody = generatorSource.match(/export interface ServiceRequestActionDataMap \{([\s\S]*?)\n\}/)?.[1];
  assert.ok(mapBody, 'ServiceRequestActionDataMap must exist in the service request generator');

  const actions = schema.properties.action.enum;
  const mappedActions = [...mapBody.matchAll(/^\s*([a-z0-9_]+):/gm)].map((match) => match[1]);
  assert.deepEqual(actions.filter((action) => !mappedActions.includes(action)), []);
  assert.deepEqual(mappedActions.filter((action) => !actions.includes(action)), []);
}

function createFetchRecorder(payload = { success: true, data: { jobId: 'job-1' } }) {
  const calls = [];
  const fetch = async (url, init = {}) => {
    calls.push({
      url: String(url),
      init,
      body: init.body ? JSON.parse(String(init.body)) : null,
    });
    return {
      ok: true,
      status: 200,
      json: async () => payload,
    };
  };
  return { calls, fetch };
}

async function main() {
  assertServiceRequestActionDataCoverage();

  assert.throws(
    () =>
      createServiceRequest({
        action: 'unsupported',
      }),
    /Unsupported service request action/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        params: 'not-an-object',
      }),
    /service request params must be an object/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        jobTimeoutMs: 0,
      }),
    /jobTimeoutMs must be a positive integer/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        profileLeaseWaitTimeoutMs: 0,
      }),
    /profileLeaseWaitTimeoutMs must be a positive integer/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        serviceName: 42,
      }),
    /service request serviceName must be a string/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        profileLeasePolicy: 'maybe',
      }),
    /profileLeasePolicy must be reject or wait/,
  );
  assert.throws(
    () =>
      createServiceRequest({
        action: 'navigate',
        loginIds: ['acs', 42],
      }),
    /service request loginIds must be an array of strings/,
  );

  const request = createServiceRequest({
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
  });
  assert.deepEqual(request, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    loginIds: ['acs', 'google'],
    profile: 'journal-acs',
    action: 'navigate',
    params: {
      url: 'https://example.com',
      waitUntil: 'load',
    },
    jobTimeoutMs: 30_000,
    profileLeasePolicy: 'wait',
    profileLeaseWaitTimeoutMs: 5_000,
  });

  const mcpToolCall = createServiceRequestMcpToolCall(request);
  assert.deepEqual(mcpToolCall, {
    name: 'service_request',
    arguments: request,
  });

  const tabRequest = createServiceTabRequest({
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    url: 'https://example.com/articles',
    params: {
      waitUntil: 'domcontentloaded',
    },
    jobTimeoutMs: 45_000,
  });
  assert.deepEqual(tabRequest, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    action: 'tab_new',
    params: {
      waitUntil: 'domcontentloaded',
      url: 'https://example.com/articles',
    },
    jobTimeoutMs: 45_000,
  });

  const postRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-post' } });
  const postResponse = await postServiceRequest({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: postRecorder.fetch,
    request,
  });
  assert.deepEqual(postResponse, { success: true, data: { jobId: 'job-post' } });
  assert.equal(postRecorder.calls.length, 1);
  assert.equal(postRecorder.calls[0].url, 'http://127.0.0.1:4849/api/service/request');
  assert.equal(postRecorder.calls[0].init.method, 'POST');
  assert.deepEqual(postRecorder.calls[0].init.headers, { 'content-type': 'application/json' });
  assert.deepEqual(postRecorder.calls[0].body, request);

  const tabRecorder = createFetchRecorder({ success: true, data: { jobId: 'job-tab' } });
  const tabResponse = await requestServiceTab({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: tabRecorder.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    url: 'https://example.com/new',
  });
  assert.deepEqual(tabResponse, { success: true, data: { jobId: 'job-tab' } });
  assert.deepEqual(tabRecorder.calls[0].body, {
    serviceName: 'JournalDownloader',
    agentName: 'article-probe-agent',
    taskName: 'probeACSwebsite',
    siteId: 'acs',
    action: 'tab_new',
    params: {
      url: 'https://example.com/new',
    },
  });

  console.log('Service request client helper tests passed');
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
