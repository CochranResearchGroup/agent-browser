#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  findServiceProfileForIdentity,
  deleteServiceMonitor,
  getServiceAccessPlan,
  getServiceContracts,
  getServiceMonitors,
  getServiceProfileAllocation,
  getServiceProfileForIdentity,
  getServiceProfileReadiness,
  getServiceStatus,
  getServiceTrace,
  lookupServiceProfile,
  pauseServiceMonitor,
  registerServiceLoginProfile,
  resumeServiceMonitor,
  resetServiceMonitorFailures,
  runDueServiceMonitors,
  summarizeServiceProfileReadiness,
  updateServiceProfileFreshness,
  upsertServiceMonitor,
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
        service_monitor_interval_ms: 60000,
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
  assert.equal(statusResult.control_plane.service_monitor_interval_ms, 60000);
  assert.deepEqual(statusResult.profileAllocations, []);

  const monitors = createFetchRecorder({
    success: true,
    data: {
      monitors: [
        {
          id: 'google-login-freshness',
          name: 'Google login freshness',
          target: {
            site_policy: 'google',
          },
          intervalMs: 60000,
          state: 'faulted',
          lastCheckedAt: null,
          lastSucceededAt: null,
          lastFailedAt: null,
          lastResult: null,
          consecutiveFailures: 1,
        },
      ],
      count: 1,
      matched: 1,
      total: 2,
      filters: {
        state: 'faulted',
        failedOnly: true,
        summary: true,
      },
      summary: {
        total: 1,
        active: 0,
        paused: 0,
        faulted: 1,
        failing: 1,
        repeatedFailures: 0,
        neverChecked: 0,
        failingMonitorIds: ['google-login-freshness'],
        repeatedFailureMonitorIds: [],
        neverCheckedMonitorIds: [],
        lastFailedAt: null,
      },
    },
  });
  const monitorsResult = await getServiceMonitors({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitors.fetch,
    state: 'faulted',
    failedOnly: true,
    summary: true,
  });
  assert.equal(
    monitors.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors?state=faulted&failed=true&summary=true',
  );
  assert.equal(monitors.calls[0].init.method, 'GET');
  assert.equal(monitorsResult.monitors[0].target.site_policy, 'google');
  assert.equal(monitorsResult.summary.failing, 1);

  const monitorUpsert = createFetchRecorder({
    success: true,
    data: {
      id: 'google-login-freshness',
      upserted: true,
      monitor: monitorsResult.monitors[0],
    },
  });
  const monitorUpsertResult = await upsertServiceMonitor({
    id: 'google-login-freshness',
    monitor: monitorsResult.monitors[0],
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorUpsert.fetch,
  });
  assert.equal(
    monitorUpsert.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness',
  );
  assert.equal(monitorUpsert.calls[0].init.method, 'POST');
  assert.equal(monitorUpsertResult.monitor.id, 'google-login-freshness');

  const monitorDelete = createFetchRecorder({
    success: true,
    data: {
      id: 'google-login-freshness',
      deleted: true,
      monitor: monitorsResult.monitors[0],
    },
  });
  const monitorDeleteResult = await deleteServiceMonitor({
    id: 'google-login-freshness',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorDelete.fetch,
  });
  assert.equal(
    monitorDelete.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness',
  );
  assert.equal(monitorDelete.calls[0].init.method, 'DELETE');
  assert.equal(monitorDeleteResult.deleted, true);

  const monitorPause = createFetchRecorder({
    success: true,
    data: {
      id: 'google-login-freshness',
      monitor: {
        ...monitorsResult.monitors[0],
        state: 'paused',
      },
      state: 'paused',
      updated: true,
    },
  });
  const monitorPauseResult = await pauseServiceMonitor({
    id: 'google-login-freshness',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorPause.fetch,
  });
  assert.equal(
    monitorPause.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness/pause',
  );
  assert.equal(monitorPause.calls[0].init.method, 'POST');
  assert.equal(monitorPauseResult.state, 'paused');

  const monitorResume = createFetchRecorder({
    success: true,
    data: {
      id: 'google-login-freshness',
      monitor: monitorsResult.monitors[0],
      state: 'active',
      updated: true,
    },
  });
  const monitorResumeResult = await resumeServiceMonitor({
    id: 'google-login-freshness',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorResume.fetch,
  });
  assert.equal(
    monitorResume.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness/resume',
  );
  assert.equal(monitorResume.calls[0].init.method, 'POST');
  assert.equal(monitorResumeResult.state, 'active');

  const monitorReset = createFetchRecorder({
    success: true,
    data: {
      id: 'google-login-freshness',
      monitor: {
        ...monitorsResult.monitors[0],
        consecutiveFailures: 0,
        state: 'active',
      },
      state: 'active',
      updated: true,
      resetFailures: true,
    },
  });
  const monitorResetResult = await resetServiceMonitorFailures({
    id: 'google-login-freshness',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorReset.fetch,
  });
  assert.equal(
    monitorReset.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness/reset-failures',
  );
  assert.equal(monitorReset.calls[0].init.method, 'POST');
  assert.equal(monitorResetResult.resetFailures, true);

  const monitorRunDue = createFetchRecorder({
    success: true,
    data: {
      checked: 1,
      succeeded: 0,
      failed: 1,
      monitorIds: ['google-login-freshness'],
    },
  });
  const monitorRunDueResult = await runDueServiceMonitors({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorRunDue.fetch,
  });
  assert.equal(
    monitorRunDue.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/run-due',
  );
  assert.equal(monitorRunDue.calls[0].init.method, 'POST');
  assert.equal(monitorRunDueResult.failed, 1);

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
    assert.equal(
      url,
      'http://127.0.0.1:4849/api/service/profiles/lookup?serviceName=CanvaCLI&loginId=canva',
    );
    return {
      success: true,
      data: {
        query: {
          serviceName: 'CanvaCLI',
          targetServiceIds: ['canva'],
          readinessProfileId: null,
        },
        selectedProfile: profileMatches[2],
        selectedProfileMatch: {
          profileId: 'authenticated',
          profile: profileMatches[2],
          reason: 'authenticated_target',
          matchedField: 'authenticatedServiceIds',
          matchedIdentity: 'canva',
        },
        readiness: {
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
        readinessSummary: {
          needsManualSeeding: false,
          manualSeedingRequired: false,
          targetServiceIds: [],
          recommendedActions: [],
        },
      },
    };
  });
  const lookupResult = await getServiceProfileForIdentity({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: profileLookup.fetch,
    serviceName: 'CanvaCLI',
    loginId: 'canva',
  });
  assert.equal(profileLookup.calls.length, 1);
  assert.equal(lookupResult.selectedProfile?.id, 'authenticated');
  assert.equal(lookupResult.selectedProfileMatch.reason, 'authenticated_target');
  assert.equal(lookupResult.selectedProfileMatch.matchedField, 'authenticatedServiceIds');
  assert.equal(lookupResult.selectedProfileMatch.matchedIdentity, 'canva');
  assert.equal(lookupResult.readiness?.profileId, 'authenticated');
  assert.equal(lookupResult.readinessSummary.needsManualSeeding, false);

  const lookupAlias = createFetchRecorder((url) => {
    assert.equal(
      url,
      'http://127.0.0.1:4849/api/service/profiles/lookup?serviceName=CanvaCLI&loginId=canva',
    );
    return {
      success: true,
      data: {
        query: {
          serviceName: 'CanvaCLI',
          targetServiceIds: ['canva'],
          readinessProfileId: null,
        },
        selectedProfile: profileMatches[2],
        selectedProfileMatch: {
          profileId: 'authenticated',
          profile: profileMatches[2],
          reason: 'authenticated_target',
          matchedField: 'authenticatedServiceIds',
          matchedIdentity: 'canva',
        },
        readiness: null,
        readinessSummary: {
          needsManualSeeding: false,
          manualSeedingRequired: false,
          targetServiceIds: [],
          recommendedActions: [],
        },
      },
    };
  });
  const lookupAliasResult = await lookupServiceProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: lookupAlias.fetch,
    serviceName: 'CanvaCLI',
    loginId: 'canva',
  });
  assert.equal(lookupAlias.calls.length, 1);
  assert.equal(lookupAliasResult.selectedProfile?.id, 'authenticated');

  const accessPlan = createFetchRecorder((url) => {
    assert.equal(
      url,
      'http://127.0.0.1:4849/api/service/access-plan?serviceName=CanvaCLI&loginId=canva&sitePolicyId=canva&challengeId=challenge-1',
    );
    return {
      success: true,
      data: {
        query: {
          serviceName: 'CanvaCLI',
          targetServiceIds: ['canva'],
          sitePolicyId: 'canva',
          challengeId: 'challenge-1',
          readinessProfileId: null,
        },
        selectedProfile: profileMatches[2],
        selectedProfileMatch: {
          profileId: 'authenticated',
          profile: profileMatches[2],
          reason: 'authenticated_target',
          matchedField: 'authenticatedServiceIds',
          matchedIdentity: 'canva',
        },
        readiness: null,
        readinessSummary: {
          needsManualSeeding: false,
          manualSeedingRequired: false,
          targetServiceIds: [],
          recommendedActions: [],
        },
        sitePolicy: {
          id: 'canva',
          originPattern: 'https://www.canva.com',
          browserHost: 'local_headed',
          viewStream: null,
          controlInput: null,
          interactionMode: 'human_like_input',
          rateLimit: {},
          manualLoginPreferred: true,
          profileRequired: true,
          authProviders: ['browser'],
          challengePolicy: 'avoid_first',
          allowedChallengeProviders: [],
          notes: null,
        },
        providers: [],
        challenges: [],
        decision: {
          recommendedAction: 'use_selected_profile',
          browserHost: 'local_headed',
          interactionMode: 'human_like_input',
          challengePolicy: 'avoid_first',
          profileId: 'authenticated',
          manualActionRequired: false,
          manualSeedingRequired: false,
          providerIds: [],
          challengeIds: [],
          reasons: ['site_policy_selected'],
        },
      },
    };
  });
  const accessPlanResult = await getServiceAccessPlan({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: accessPlan.fetch,
    serviceName: 'CanvaCLI',
    loginId: 'canva',
    sitePolicyId: 'canva',
    challengeId: 'challenge-1',
  });
  assert.equal(accessPlan.calls.length, 1);
  assert.equal(accessPlanResult.selectedProfile?.id, 'authenticated');
  assert.equal(accessPlanResult.sitePolicy?.id, 'canva');
  assert.equal(accessPlanResult.decision.recommendedAction, 'use_selected_profile');

  const emptyLookup = createFetchRecorder({
    success: true,
    data: {
      query: {
        serviceName: 'CanvaCLI',
        targetServiceIds: ['missing'],
        readinessProfileId: null,
      },
      selectedProfile: null,
      selectedProfileMatch: null,
      readiness: null,
      readinessSummary: {
        needsManualSeeding: false,
        manualSeedingRequired: false,
        targetServiceIds: [],
        recommendedActions: [],
      },
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

  const freshness = createFetchRecorder();
  await registerServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: freshness.fetch,
    id: 'journal-google',
    serviceName: 'JournalDownloader',
    loginId: 'google',
    readinessState: 'fresh',
    readinessEvidence: 'auth_probe_cookie_present',
    lastVerifiedAt: '2026-05-06T12:00:00Z',
    freshnessExpiresAt: '2026-05-06T13:00:00Z',
  });
  assert.deepEqual(freshness.calls[0].body.targetReadiness, [
    {
      targetServiceId: 'google',
      loginId: 'google',
      state: 'fresh',
      manualSeedingRequired: false,
      evidence: 'auth_probe_cookie_present',
      recommendedAction: 'use_profile',
      lastVerifiedAt: '2026-05-06T12:00:00Z',
      freshnessExpiresAt: '2026-05-06T13:00:00Z',
    },
  ]);

  const explicitReadiness = createFetchRecorder();
  await registerServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: explicitReadiness.fetch,
    id: 'journal-google',
    serviceName: 'JournalDownloader',
    loginId: 'google',
    readinessState: 'fresh',
    targetReadiness: [
      {
        targetServiceId: 'google',
        loginId: 'google',
        state: 'stale',
        manualSeedingRequired: false,
        evidence: 'operator_reported_timeout',
        recommendedAction: 'probe_target_auth_or_reseed_if_needed',
        lastVerifiedAt: '2026-05-06T12:30:00Z',
        freshnessExpiresAt: null,
      },
    ],
  });
  assert.deepEqual(explicitReadiness.calls[0].body.targetReadiness, [
    {
      targetServiceId: 'google',
      loginId: 'google',
      state: 'stale',
      manualSeedingRequired: false,
      evidence: 'operator_reported_timeout',
      recommendedAction: 'probe_target_auth_or_reseed_if_needed',
      lastVerifiedAt: '2026-05-06T12:30:00Z',
      freshnessExpiresAt: null,
    },
  ]);

  const freshnessUpdate = createFetchRecorder((_url, _init, calls) => {
    return {
      success: true,
      data: {
        id: 'journal-google',
        upserted: true,
        profile: calls.at(-1)?.body,
      },
    };
  });
  await updateServiceProfileFreshness({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: freshnessUpdate.fetch,
    id: 'journal-google',
    loginId: 'google',
    readinessState: 'stale',
    readinessEvidence: 'auth_probe_cookie_missing',
    lastVerifiedAt: '2026-05-06T14:00:00Z',
  });
  assert.equal(
    freshnessUpdate.calls[0].url,
    'http://127.0.0.1:4849/api/service/profiles/journal-google/freshness',
  );
  assert.equal(freshnessUpdate.calls[0].init.method, 'POST');
  assert.deepEqual(freshnessUpdate.calls[0].body, {
    loginId: 'google',
    targetServiceIds: ['google'],
    readinessState: 'stale',
    readinessEvidence: 'auth_probe_cookie_missing',
    lastVerifiedAt: '2026-05-06T14:00:00Z',
    updateAuthenticatedServiceIds: true,
  });

  console.log('Service observability client helper tests passed');
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
