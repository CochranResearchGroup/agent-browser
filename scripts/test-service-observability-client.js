#!/usr/bin/env node

import assert from 'node:assert/strict';

import { registerServiceLoginProfile } from '../packages/client/src/service-observability.js';

function createFetchRecorder() {
  const calls = [];
  const fetch = async (url, init = {}) => {
    calls.push({
      url: String(url),
      init,
      body: init.body ? JSON.parse(String(init.body)) : null,
    });
    return {
      ok: true,
      json: async () => ({
        success: true,
        data: {
          id: 'profile-id',
          upserted: true,
          profile: calls.at(-1)?.body,
        },
      }),
    };
  };
  return { calls, fetch };
}

async function main() {
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
