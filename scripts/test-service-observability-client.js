#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  getServiceContracts,
  getServiceTrace,
  registerServiceLoginProfile,
} from '../packages/client/src/service-observability.js';

function createFetchRecorder(payload) {
  const calls = [];
  const fetch = async (url, init = {}) => {
    calls.push({
      url: String(url),
      init,
      body: init.body ? JSON.parse(String(init.body)) : null,
    });
    return {
      ok: true,
      json: async () =>
        payload ?? {
          success: true,
          data: {
            id: 'profile-id',
            upserted: true,
            profile: calls.at(-1)?.body,
          },
        },
    };
  };
  return { calls, fetch };
}

async function main() {
  const contracts = createFetchRecorder({
    success: true,
    data: {
      schemaVersion: 'v1',
      contracts: {
        serviceRequest: {
          version: 'v1',
          schemaId: 'https://agent-browser.local/contracts/service-request.v1.schema.json',
        },
      },
      http: {},
      mcp: {},
    },
  });
  const contractResult = await getServiceContracts({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: contracts.fetch,
  });
  assert.equal(contracts.calls[0].url, 'http://127.0.0.1:4849/api/service/contracts');
  assert.equal(contracts.calls[0].init.method, 'GET');
  assert.equal(contractResult.schemaVersion, 'v1');

  const trace = createFetchRecorder({
    success: true,
    data: {
      filters: {
        serviceName: 'JournalDownloader',
        profileId: 'profile-1',
        limit: 20,
      },
      events: [],
      jobs: [],
      incidents: [],
      activity: [],
      summary: {
        contextCount: 0,
        hasTraceContext: false,
        namingWarningCount: 0,
        profileLeaseWaits: {
          count: 1,
          activeCount: 0,
          completedCount: 1,
          waits: [
            {
              jobId: 'job-wait-complete',
              profileId: 'profile-1',
              outcome: 'ready',
              startedAt: '2026-04-25T12:00:10Z',
              endedAt: '2026-04-25T12:00:15Z',
              waitedMs: 5000,
              retryAfterMs: 250,
              conflictSessionIds: ['session-conflict'],
              serviceName: 'JournalDownloader',
              agentName: 'agent-a',
              taskName: 'probeACSwebsite',
            },
          ],
        },
        contexts: [],
      },
      counts: {
        events: 0,
        jobs: 0,
        incidents: 0,
        activity: 0,
      },
      matched: {
        events: 0,
        jobs: 0,
        incidents: 0,
        activity: 0,
      },
      total: {
        events: 0,
        jobs: 0,
        incidents: 0,
      },
    },
  });
  const traceResult = await getServiceTrace({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: trace.fetch,
    query: {
      'service-name': 'JournalDownloader',
      'profile-id': 'profile-1',
      limit: 20,
    },
  });
  assert.equal(
    trace.calls[0].url,
    'http://127.0.0.1:4849/api/service/trace?service-name=JournalDownloader&profile-id=profile-1&limit=20',
  );
  assert.equal(trace.calls[0].init.method, 'GET');
  assert.equal(traceResult.summary.profileLeaseWaits.count, 1);
  assert.equal(traceResult.summary.profileLeaseWaits.completedCount, 1);
  assert.equal(traceResult.summary.profileLeaseWaits.waits[0].jobId, 'job-wait-complete');
  assert.equal(traceResult.summary.profileLeaseWaits.waits[0].waitedMs, 5000);

  assert.throws(
    () =>
      registerServiceLoginProfile({
        baseUrl: 'http://127.0.0.1:4849',
        id: '',
        serviceName: 'JournalDownloader',
        loginId: 'acs',
      }),
    /requires an id string/,
  );
  assert.throws(
    () =>
      registerServiceLoginProfile({
        baseUrl: 'http://127.0.0.1:4849',
        id: 'journal-acs',
        serviceName: '',
        loginId: 'acs',
      }),
    /requires a serviceName string/,
  );
  assert.throws(
    () =>
      registerServiceLoginProfile({
        baseUrl: 'http://127.0.0.1:4849',
        id: 'journal-acs',
        serviceName: 'JournalDownloader',
      }),
    /requires loginId, siteId, targetServiceId, or targetServiceIds/,
  );

  const defaults = createFetchRecorder();
  const defaultResult = await registerServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: defaults.fetch,
    id: 'journal-acs',
    serviceName: 'JournalDownloader',
    loginId: 'acs',
  });
  assert.equal(defaults.calls.length, 1);
  assert.equal(defaults.calls[0].url, 'http://127.0.0.1:4849/api/service/profiles/journal-acs');
  assert.equal(defaults.calls[0].init.method, 'POST');
  assert.deepEqual(defaults.calls[0].body, {
    name: 'journal-acs',
    allocation: 'per_service',
    keyring: 'basic_password_store',
    persistent: true,
    targetServiceIds: ['acs'],
    authenticatedServiceIds: ['acs'],
    sharedServiceIds: ['JournalDownloader'],
  });
  assert.equal(defaultResult.upserted, true);

  const merged = createFetchRecorder();
  await registerServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: merged.fetch,
    id: 'journal-acs',
    serviceName: 'JournalDownloader',
    loginId: 'acs',
    targetServiceIds: ['acs', 'google'],
    authenticatedServiceIds: ['google'],
    sharedServiceIds: ['OtherService', 'JournalDownloader'],
    userDataDir: '/tmp/profile',
    profile: {
      name: 'ACS override',
      allocation: 'shared',
      customField: true,
    },
  });
  assert.deepEqual(merged.calls[0].body, {
    name: 'ACS override',
    allocation: 'shared',
    keyring: 'basic_password_store',
    persistent: true,
    targetServiceIds: ['acs', 'google'],
    authenticatedServiceIds: ['google', 'acs'],
    sharedServiceIds: ['OtherService', 'JournalDownloader'],
    userDataDir: '/tmp/profile',
    customField: true,
  });

  const unauthenticated = createFetchRecorder();
  await registerServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: unauthenticated.fetch,
    id: 'journal-login',
    serviceName: 'JournalDownloader',
    targetServiceIds: ['acs'],
    authenticatedServiceIds: ['google'],
    authenticated: false,
  });
  assert.deepEqual(unauthenticated.calls[0].body.authenticatedServiceIds, ['google']);

  console.log('Service observability client helper tests passed');
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
