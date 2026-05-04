#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  findServiceProfileForIdentity,
  getServiceContracts,
  getServiceProfileAllocation,
  getServiceProfileForIdentity,
  getServiceProfileReadiness,
  getServiceStatus,
  getServiceTrace,
  registerServiceLoginProfile,
  summarizeServiceProfileReadiness,
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
        typeof payload === 'function'
          ? payload(String(url), init, calls)
          : (payload ?? {
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

  const status = createFetchRecorder({
    success: true,
    data: {
      control_plane: {
        waiting_profile_lease_job_count: 0,
      },
      service_state: {},
      profileAllocations: [],
    },
  });
  const statusResult = await getServiceStatus({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: status.fetch,
  });
  assert.equal(status.calls[0].url, 'http://127.0.0.1:4849/api/service/status');
  assert.equal(status.calls[0].init.method, 'GET');
  assert.deepEqual(statusResult.profileAllocations, []);

  const allocation = createFetchRecorder({
    success: true,
    data: {
      profileAllocation: {
        profileId: 'work',
        profileName: 'Work',
        allocation: 'per_service',
        keyring: 'basic_password_store',
        targetServiceIds: ['google'],
        authenticatedServiceIds: [],
        targetReadiness: [
          {
            targetServiceId: 'google',
            loginId: null,
            state: 'needs_manual_seeding',
            manualSeedingRequired: true,
            evidence: 'manual_seed_required_without_authenticated_hint',
            recommendedAction: 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
            lastVerifiedAt: null,
            freshnessExpiresAt: null,
          },
        ],
        sharedServiceIds: ['JournalDownloader'],
        holderSessionIds: ['session-1'],
        holderCount: 1,
        exclusiveHolderSessionIds: [],
        waitingJobIds: [],
        waitingJobCount: 0,
        conflictSessionIds: [],
        leaseState: 'shared',
        recommendedAction: 'shared_profile_in_use',
        serviceNames: ['JournalDownloader'],
        agentNames: ['codex'],
        taskNames: ['probeACSwebsite'],
        browserIds: ['browser-1'],
        tabIds: ['tab-1'],
      },
    },
  });
  const allocationResult = await getServiceProfileAllocation({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: allocation.fetch,
    id: 'work',
  });
  assert.equal(allocation.calls[0].url, 'http://127.0.0.1:4849/api/service/profiles/work/allocation');
  assert.equal(allocation.calls[0].init.method, 'GET');
  assert.equal(allocationResult.profileAllocation.profileId, 'work');
  assert.equal(allocationResult.profileAllocation.targetReadiness[0].state, 'needs_manual_seeding');

  const readiness = createFetchRecorder({
    success: true,
    data: {
      profileId: 'work',
      targetReadiness: allocationResult.profileAllocation.targetReadiness,
      count: 1,
    },
  });
  const readinessResult = await getServiceProfileReadiness({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: readiness.fetch,
    id: 'work',
  });
  assert.equal(readiness.calls[0].url, 'http://127.0.0.1:4849/api/service/profiles/work/readiness');
  assert.equal(readiness.calls[0].init.method, 'GET');
  assert.equal(readinessResult.profileId, 'work');
  assert.equal(readinessResult.targetReadiness[0].state, 'needs_manual_seeding');
  const readinessSummary = summarizeServiceProfileReadiness(readinessResult);
  assert.equal(readinessSummary.needsManualSeeding, true);
  assert.equal(readinessSummary.manualSeedingRequired, true);
  assert.deepEqual(readinessSummary.targetServiceIds, ['google']);
  assert.deepEqual(readinessSummary.recommendedActions, [
    'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
  ]);
  assert.deepEqual(summarizeServiceProfileReadiness(null), {
    needsManualSeeding: false,
    manualSeedingRequired: false,
    targetServiceIds: [],
    recommendedActions: [],
  });

  const profileMatches = [
    {
      id: 'shared',
      name: 'Shared',
      userDataDir: null,
      authenticatedServiceIds: [],
      targetServiceIds: [],
      sharedServiceIds: ['CanvaCLI'],
      targetReadiness: [],
    },
    {
      id: 'target',
      name: 'Target',
      userDataDir: null,
      authenticatedServiceIds: [],
      targetServiceIds: ['canva'],
      sharedServiceIds: ['OtherService'],
      targetReadiness: [],
    },
    {
      id: 'authenticated',
      name: 'Authenticated',
      userDataDir: null,
      authenticatedServiceIds: ['canva'],
      targetServiceIds: ['canva'],
      sharedServiceIds: ['CanvaCLI'],
      targetReadiness: [],
    },
  ];
  const authenticatedMatch = findServiceProfileForIdentity(profileMatches, {
    serviceName: 'CanvaCLI',
    loginId: 'canva',
  });
  assert.equal(authenticatedMatch.profile?.id, 'authenticated');
  assert.equal(authenticatedMatch.reason, 'authenticated_target');
  assert.equal(authenticatedMatch.matchedField, 'authenticatedServiceIds');
  assert.equal(authenticatedMatch.matchedIdentity, 'canva');

  const targetMatch = findServiceProfileForIdentity(profileMatches.slice(0, 2), {
    serviceName: 'CanvaCLI',
    targetServiceIds: ['canva'],
  });
  assert.equal(targetMatch.profile?.id, 'target');
  assert.equal(targetMatch.reason, 'target_match');

  const serviceMatch = findServiceProfileForIdentity(profileMatches.slice(0, 1), {
    serviceName: 'CanvaCLI',
    loginId: 'missing',
  });
  assert.equal(serviceMatch.profile?.id, 'shared');
  assert.equal(serviceMatch.reason, 'service_allow_list');

  assert.deepEqual(findServiceProfileForIdentity(null, { loginId: 'missing' }), {
    profile: null,
    reason: null,
    matchedField: null,
    matchedIdentity: null,
  });

  const profileLookup = createFetchRecorder((url) => {
    if (url.endsWith('/api/service/profiles')) {
      return {
        success: true,
        data: {
          profiles: profileMatches,
          count: profileMatches.length,
          profileAllocations: [],
        },
      };
    }
    return {
      success: true,
      data: {
        profileId: 'authenticated',
        targetReadiness: [
          {
            targetServiceId: 'canva',
            loginId: null,
            state: 'ready',
            manualSeedingRequired: false,
            evidence: 'authenticated_hint_present',
            recommendedAction: null,
            lastVerifiedAt: null,
            freshnessExpiresAt: null,
          },
        ],
        count: 1,
      },
    };
  });
  const lookupResult = await getServiceProfileForIdentity({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: profileLookup.fetch,
    serviceName: 'CanvaCLI',
    loginId: 'canva',
  });
  assert.equal(profileLookup.calls.length, 2);
  assert.equal(profileLookup.calls[0].url, 'http://127.0.0.1:4849/api/service/profiles');
  assert.equal(profileLookup.calls[1].url, 'http://127.0.0.1:4849/api/service/profiles/authenticated/readiness');
  assert.equal(lookupResult.selectedProfile?.id, 'authenticated');
  assert.equal(lookupResult.selectedProfileMatch.reason, 'authenticated_target');
  assert.equal(lookupResult.readiness?.profileId, 'authenticated');
  assert.equal(lookupResult.readinessSummary.needsManualSeeding, false);

  const emptyLookup = createFetchRecorder({
    success: true,
    data: {
      profiles: [],
      count: 0,
      profileAllocations: [],
    },
  });
  const emptyLookupResult = await getServiceProfileForIdentity({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: emptyLookup.fetch,
    serviceName: 'CanvaCLI',
    loginId: 'missing',
  });
  assert.equal(emptyLookup.calls.length, 1);
  assert.equal(emptyLookupResult.selectedProfile, null);
  assert.equal(emptyLookupResult.readiness, null);
  assert.deepEqual(emptyLookupResult.readinessSummary, {
    needsManualSeeding: false,
    manualSeedingRequired: false,
    targetServiceIds: [],
    recommendedActions: [],
  });

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
