#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  acquireServiceLoginProfile,
  applyServiceRemedies,
  findServiceProfileForIdentity,
  deleteServiceMonitor,
  getServiceAccessPlan,
  getServiceContracts,
  getServiceMonitors,
  getServiceProfileAllocation,
  getServiceProfileForIdentity,
  getServiceProfileReadiness,
  getServiceProfileSeedingHandoff,
  getServiceStatus,
  getServiceTrace,
  lookupServiceProfile,
  pauseServiceMonitor,
  registerServiceLoginProfile,
  resumeServiceMonitor,
  resetServiceMonitorFailures,
  runDueServiceMonitors,
  runServiceAccessPlanMonitorRunDue,
  runServiceAccessPlanPostSeedingProbe,
  summarizeServiceProfileReadiness,
  triageServiceMonitor,
  updateServiceProfileFreshness,
  updateServiceProfileSeedingHandoff,
  upsertServiceMonitor,
  verifyServiceProfileSeeding,
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

  const monitorTriage = createFetchRecorder({
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
      acknowledged: true,
      incident: {
        id: 'monitor:google-login-freshness',
        monitorId: 'google-login-freshness',
      },
    },
  });
  const monitorTriageResult = await triageServiceMonitor({
    id: 'google-login-freshness',
    by: 'operator',
    note: 'reviewed',
    serviceName: 'JournalDownloader',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: monitorTriage.fetch,
  });
  assert.equal(
    monitorTriage.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/google-login-freshness/triage?by=operator&note=reviewed&service-name=JournalDownloader',
  );
  assert.equal(monitorTriage.calls[0].init.method, 'POST');
  assert.equal(monitorTriageResult.acknowledged, true);

  const monitorRunDue = createFetchRecorder({
    success: true,
    data: {
      checked: 1,
      succeeded: 0,
      failed: 1,
      monitorIds: ['google-login-freshness'],
      results: [
        {
          monitorId: 'google-login-freshness',
          checkedAt: '2026-05-07T00:00:00Z',
          success: false,
          result: 'profile_readiness_expired',
          target: { profile_readiness: 'google' },
          staleProfileIds: ['journal-google'],
        },
      ],
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
  assert.equal(monitorRunDueResult.results[0].monitorId, 'google-login-freshness');
  assert.deepEqual(monitorRunDueResult.results[0].staleProfileIds, ['journal-google']);

  const accessPlanMonitorRunDue = createFetchRecorder({
    success: true,
    data: {
      checked: 1,
      failed: 0,
      monitors: [{ id: 'google-login-freshness' }],
    },
  });
  const accessPlanMonitorRunDueResult = await runServiceAccessPlanMonitorRunDue({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: accessPlanMonitorRunDue.fetch,
    accessPlan: serviceAccessPlanWithDueMonitor(),
  });
  assert.equal(
    accessPlanMonitorRunDue.calls[0].url,
    'http://127.0.0.1:4849/api/service/monitors/run-due',
  );
  assert.equal(accessPlanMonitorRunDue.calls[0].init.method, 'POST');
  assert.equal(accessPlanMonitorRunDueResult.checked, 1);

  const unavailableAccessPlanMonitorRunDue = createFetchRecorder({
    success: true,
    data: {
      checked: 1,
      failed: 0,
      monitors: [{ id: 'google-login-freshness' }],
    },
  });
  assert.throws(
    () =>
      runServiceAccessPlanMonitorRunDue({
        baseUrl: 'http://127.0.0.1:4849',
        fetch: unavailableAccessPlanMonitorRunDue.fetch,
        accessPlan: serviceAccessPlanWithoutDueMonitor(),
      }),
    /access plan monitorRunDue is not available/,
  );
  assert.equal(unavailableAccessPlanMonitorRunDue.calls.length, 0);

  const remediesApply = createFetchRecorder({
    success: true,
    data: {
      applied: true,
      escalation: 'monitor_attention',
      count: 1,
      monitorIds: ['google-login-freshness'],
      monitorResults: [monitorTriageResult],
      browserIds: [],
      browserResults: [],
    },
  });
  const remediesApplyResult = await applyServiceRemedies({
    escalation: 'monitor_attention',
    by: 'operator',
    note: 'reviewed',
    serviceName: 'JournalDownloader',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: remediesApply.fetch,
  });
  assert.equal(
    remediesApply.calls[0].url,
    'http://127.0.0.1:4849/api/service/remedies/apply?escalation=monitor_attention&by=operator&note=reviewed&service-name=JournalDownloader',
  );
  assert.equal(remediesApply.calls[0].init.method, 'POST');
  assert.equal(remediesApplyResult.count, 1);

  const degradedRemediesApply = createFetchRecorder({
    success: true,
    data: {
      applied: true,
      escalation: 'browser_degraded',
      count: 1,
      monitorIds: [],
      monitorResults: [],
      browserIds: ['browser-degraded'],
      browserResults: [
        {
          id: 'browser-degraded',
          retryEnabled: true,
          browser: { id: 'browser-degraded', health: 'process_exited' },
          incident: null,
        },
      ],
    },
  });
  const degradedRemediesApplyResult = await applyServiceRemedies({
    escalation: 'browser_degraded',
    by: 'operator',
    note: 'reviewed',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: degradedRemediesApply.fetch,
  });
  assert.equal(
    degradedRemediesApply.calls[0].url,
    'http://127.0.0.1:4849/api/service/remedies/apply?escalation=browser_degraded&by=operator&note=reviewed',
  );
  assert.equal(degradedRemediesApply.calls[0].init.method, 'POST');
  assert.equal(degradedRemediesApplyResult.browserIds[0], 'browser-degraded');

  const osRemediesApply = createFetchRecorder({
    success: true,
    data: {
      applied: true,
      escalation: 'os_degraded_possible',
      count: 1,
      monitorIds: [],
      monitorResults: [],
      browserIds: ['browser-1'],
      browserResults: [
        {
          id: 'browser-1',
          retryEnabled: true,
          browser: { id: 'browser-1', health: 'process_exited' },
          incident: null,
        },
      ],
    },
  });
  const osRemediesApplyResult = await applyServiceRemedies({
    escalation: 'os_degraded_possible',
    by: 'operator',
    note: 'host inspected',
    baseUrl: 'http://127.0.0.1:4849',
    fetch: osRemediesApply.fetch,
  });
  assert.equal(
    osRemediesApply.calls[0].url,
    'http://127.0.0.1:4849/api/service/remedies/apply?escalation=os_degraded_possible&by=operator&note=host+inspected',
  );
  assert.equal(osRemediesApply.calls[0].init.method, 'POST');
  assert.equal(osRemediesApplyResult.browserIds[0], 'browser-1');

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
            seedingMode: 'detached_headed_no_cdp',
            cdpAttachmentAllowedDuringSeeding: false,
            preferredKeyring: 'basic_password_store',
            setupScopes: ['signin', 'chrome_sync', 'passkeys', 'browser_plugins'],
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

  const handoff = createFetchRecorder({
    success: true,
    data: {
      profileId: 'work',
      profileName: 'Work',
      targetServiceId: 'google',
      loginId: null,
      manualSeedingRequired: true,
      seedingMode: 'detached_headed_no_cdp',
      cdpAttachmentAllowedDuringSeeding: false,
      preferredKeyring: 'basic_password_store',
      setupScopes: ['signin', 'chrome_sync', 'passkeys', 'browser_plugins'],
      recommendedAction: 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
      url: 'https://accounts.google.com',
      command: 'agent-browser --runtime-profile work runtime login https://accounts.google.com',
      lifecycle: {
        id: 'work:google',
        profileId: 'work',
        targetServiceId: 'google',
        state: 'needs_manual_seeding',
        pid: null,
        startedAt: null,
        expiresAt: null,
        lastPromptedAt: null,
        declaredCompleteAt: null,
        closedAt: null,
        updatedAt: null,
        actor: null,
        note: null,
      },
      operatorSteps: ['Run the command exactly as shown.'],
      operatorIntervention: {
        state: 'needs_manual_seeding',
        severity: 'action_required',
        title: 'Seed profile work for google',
        message:
          'Launch the detached headed browser, complete setup, close Chrome, then let agent-browser verify freshness after CDP is allowed again.',
        ownedBy: 'agent-browser',
        defaultChannels: ['api', 'mcp', 'dashboard'],
        optionalChannels: ['desktop', 'webhook', 'agent'],
        desktopPopupPolicy: 'optional_policy_controlled',
        blocksProfileLease: true,
        completionSignals: [
          'seeding_browser_closed',
          'operator_or_agent_declared_complete',
          'post_seeding_probe_records_freshness',
        ],
        actions: [
          {
            id: 'run_detached_seeding_command',
            label: 'Run detached seeding command',
            kind: 'operator_command',
            safety: 'safe',
            command: 'agent-browser --runtime-profile work runtime login https://accounts.google.com',
            description: 'Launch headed Chrome without CDP or DevTools so first sign-in and setup can complete.',
          },
        ],
      },
      warnings: ['Do not add --attachable or any remote debugging/CDP flag during first seeding.'],
    },
  });
  const handoffResult = await getServiceProfileSeedingHandoff({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: handoff.fetch,
    id: 'work',
    targetServiceId: 'google',
  });
  assert.equal(
    handoff.calls[0].url,
    'http://127.0.0.1:4849/api/service/profiles/work/seeding-handoff?targetServiceId=google',
  );
  assert.equal(handoff.calls[0].init.method, 'GET');
  assert.equal(handoffResult.command, 'agent-browser --runtime-profile work runtime login https://accounts.google.com');
  assert.equal(handoffResult.operatorIntervention.severity, 'action_required');
  assert.equal(handoffResult.operatorIntervention.desktopPopupPolicy, 'optional_policy_controlled');
  assert.equal(handoffResult.operatorIntervention.blocksProfileLease, true);
  assert.equal(handoffResult.operatorIntervention.actions[0].id, 'run_detached_seeding_command');
  assert.equal(handoffResult.lifecycle.state, 'needs_manual_seeding');

  const handoffUpdate = createFetchRecorder({
    success: true,
    data: {
      id: 'work:google',
      profileId: 'work',
      targetServiceId: 'google',
      handoff: {
        id: 'work:google',
        profileId: 'work',
        targetServiceId: 'google',
        state: 'seeding_launched_detached',
        pid: 1234,
        startedAt: '2026-05-10T12:00:00Z',
        expiresAt: '2026-05-10T12:30:00Z',
        lastPromptedAt: null,
        declaredCompleteAt: null,
        closedAt: null,
        updatedAt: '2026-05-10T12:01:00Z',
        actor: 'operator',
        note: 'manual seeding started',
      },
      seedingHandoff: {
        ...handoffResult,
        lifecycle: {
          ...handoffResult.lifecycle,
          state: 'seeding_launched_detached',
          pid: 1234,
          startedAt: '2026-05-10T12:00:00Z',
          expiresAt: '2026-05-10T12:30:00Z',
          updatedAt: '2026-05-10T12:01:00Z',
          actor: 'operator',
          note: 'manual seeding started',
        },
        operatorIntervention: {
          ...handoffResult.operatorIntervention,
          state: 'seeding_launched_detached',
          severity: 'attention',
        },
      },
      updated: true,
    },
  });
  const handoffUpdateResult = await updateServiceProfileSeedingHandoff({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: handoffUpdate.fetch,
    id: 'work',
    targetServiceId: 'google',
    state: 'seeding_launched_detached',
    pid: 1234,
    startedAt: '2026-05-10T12:00:00Z',
    expiresAt: '2026-05-10T12:30:00Z',
    actor: 'operator',
    note: 'manual seeding started',
  });
  assert.equal(
    handoffUpdate.calls[0].url,
    'http://127.0.0.1:4849/api/service/profiles/work/seeding-handoff',
  );
  assert.equal(handoffUpdate.calls[0].init.method, 'POST');
  assert.deepEqual(handoffUpdate.calls[0].body, {
    targetServiceId: 'google',
    state: 'seeding_launched_detached',
    pid: 1234,
    startedAt: '2026-05-10T12:00:00Z',
    expiresAt: '2026-05-10T12:30:00Z',
    actor: 'operator',
    note: 'manual seeding started',
  });
  assert.equal(handoffUpdateResult.handoff.state, 'seeding_launched_detached');
  assert.equal(handoffUpdateResult.seedingHandoff.operatorIntervention.severity, 'attention');

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
        monitorFindings: {
          profileReadinessAttentionRequired: true,
          profileReadinessIncidentIds: ['monitor:canva-freshness'],
          profileReadinessMonitorIds: ['canva-freshness'],
          profileReadinessResults: ['profile_readiness_expired'],
          targetServiceIds: ['canva'],
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
          monitorAttentionRequired: true,
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
  assert.equal(accessPlanResult.monitorFindings.profileReadinessAttentionRequired, true);
  assert.deepEqual(accessPlanResult.monitorFindings.profileReadinessMonitorIds, [
    'canva-freshness',
  ]);
  assert.equal(accessPlanResult.decision.monitorAttentionRequired, true);
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

  const existingAcquisition = createFetchRecorder({
    success: true,
    data: {
      query: {
        serviceName: 'JournalDownloader',
        targetServiceIds: ['acs'],
        readinessProfileId: null,
      },
      selectedProfile: {
        id: 'journal-existing',
        name: 'JournalDownloader ACS',
        targetServiceIds: ['acs'],
        authenticatedServiceIds: ['acs'],
        sharedServiceIds: ['JournalDownloader'],
      },
      selectedProfileMatch: {
        profileId: 'journal-existing',
        reason: 'authenticated_target',
        matchedField: 'authenticatedServiceIds',
        matchedIdentity: 'acs',
      },
      selectedProfileSource: 'config',
      readiness: null,
      readinessSummary: {
        needsManualSeeding: false,
        manualSeedingRequired: false,
        targetServiceIds: [],
        recommendedActions: [],
      },
      monitorFindings: {
        profileReadinessAttentionRequired: false,
        profileReadinessIncidentIds: [],
        profileReadinessMonitorIds: [],
        profileReadinessResults: [],
        targetServiceIds: [],
      },
      sitePolicy: null,
      sitePolicySource: null,
      providers: [],
      challenges: [],
      decision: {
        recommendedAction: 'use_selected_profile',
        browserHost: 'local_headed',
        interactionMode: 'human_like_input',
        challengePolicy: 'avoid_first',
        profileId: 'journal-existing',
        manualActionRequired: false,
        manualSeedingRequired: false,
        monitorAttentionRequired: false,
        providerIds: [],
        challengeIds: [],
        reasons: ['profile_selected'],
      },
    },
  });
  const existingAcquisitionResult = await acquireServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: existingAcquisition.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'agent-a',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    registerProfileId: 'journal-fallback',
    registerReadinessMonitor: true,
  });
  assert.equal(existingAcquisition.calls.length, 1);
  assert.equal(existingAcquisition.calls[0].url, 'http://127.0.0.1:4849/api/service/access-plan?serviceName=JournalDownloader&agentName=agent-a&taskName=probeACSwebsite&loginId=acs');
  assert.equal(existingAcquisitionResult.selectedProfile?.id, 'journal-existing');
  assert.equal(existingAcquisitionResult.profileRegistration, null);
  assert.equal(existingAcquisitionResult.profileReadinessMonitor, null);
  assert.equal(existingAcquisitionResult.monitorRunDue, null);
  assert.equal(existingAcquisitionResult.registered, false);
  assert.equal(existingAcquisitionResult.monitorRegistered, false);
  assert.equal(existingAcquisitionResult.monitorRunDueRan, false);
  assert.equal(existingAcquisitionResult.accessPlan, existingAcquisitionResult.initialAccessPlan);

  const dueMonitorAcquisition = createFetchRecorder((_url, _init, calls) => {
    if (calls.length === 1) {
      return {
        success: true,
        data: {
          query: {
            serviceName: 'JournalDownloader',
            targetServiceIds: ['acs'],
            readinessProfileId: null,
          },
          selectedProfile: {
            id: 'journal-existing',
            name: 'JournalDownloader ACS',
            targetServiceIds: ['acs'],
            authenticatedServiceIds: ['acs'],
            sharedServiceIds: ['JournalDownloader'],
          },
          selectedProfileMatch: {
            profileId: 'journal-existing',
            reason: 'authenticated_target',
            matchedField: 'authenticatedServiceIds',
            matchedIdentity: 'acs',
          },
          selectedProfileSource: 'config',
          readiness: null,
          readinessSummary: {
            needsManualSeeding: false,
            manualSeedingRequired: false,
            targetServiceIds: [],
            recommendedActions: [],
          },
          monitorFindings: {
            profileReadinessAttentionRequired: false,
            profileReadinessProbeDue: true,
            profileReadinessIncidentIds: [],
            profileReadinessMonitorIds: [],
            profileReadinessDueMonitorIds: ['acs-freshness'],
            profileReadinessNeverCheckedMonitorIds: ['acs-freshness'],
            profileReadinessResults: [],
            targetServiceIds: [],
            dueTargetServiceIds: ['acs'],
          },
          sitePolicy: null,
          sitePolicySource: null,
          providers: [],
          challenges: [],
          decision: {
            recommendedAction: 'run_due_profile_readiness_monitor',
            browserHost: 'local_headed',
            interactionMode: 'human_like_input',
            challengePolicy: 'avoid_first',
            profileId: 'journal-existing',
            manualActionRequired: false,
            manualSeedingRequired: false,
            monitorAttentionRequired: false,
            monitorProbeDue: true,
            monitorRunDue: {
              available: true,
              recommendedBeforeUse: true,
              monitorIds: ['acs-freshness'],
              neverCheckedMonitorIds: ['acs-freshness'],
              targetServiceIds: ['acs'],
              http: { method: 'POST', route: '/api/service/monitors/run-due' },
              mcp: { tool: 'service_monitors_run_due' },
              client: {
                package: '@agent-browser/client/service-observability',
                helper: 'runServiceAccessPlanMonitorRunDue',
              },
              fallbackClient: {
                package: '@agent-browser/client/service-observability',
                helper: 'runDueServiceMonitors',
              },
              cli: { command: 'agent-browser service monitors run-due' },
              requestFields: [],
              notes: [],
            },
            providerIds: [],
            challengeIds: [],
            reasons: ['profile_readiness_probe_due'],
          },
        },
      };
    }
    if (calls.length === 2) {
      return {
        success: true,
        data: {
          checked: 1,
          failed: 0,
          monitors: [{ id: 'acs-freshness' }],
        },
      };
    }
    return {
      success: true,
      data: {
        query: {
          serviceName: 'JournalDownloader',
          targetServiceIds: ['acs'],
          readinessProfileId: null,
        },
        selectedProfile: {
          id: 'journal-existing',
          name: 'JournalDownloader ACS',
          targetServiceIds: ['acs'],
          authenticatedServiceIds: ['acs'],
          sharedServiceIds: ['JournalDownloader'],
        },
        selectedProfileMatch: {
          profileId: 'journal-existing',
          reason: 'authenticated_target',
          matchedField: 'authenticatedServiceIds',
          matchedIdentity: 'acs',
        },
        selectedProfileSource: 'config',
        readiness: null,
        readinessSummary: {
          needsManualSeeding: false,
          manualSeedingRequired: false,
          targetServiceIds: [],
          recommendedActions: [],
        },
        monitorFindings: {
          profileReadinessAttentionRequired: false,
          profileReadinessProbeDue: false,
          profileReadinessIncidentIds: [],
          profileReadinessMonitorIds: [],
          profileReadinessDueMonitorIds: [],
          profileReadinessNeverCheckedMonitorIds: [],
          profileReadinessResults: [],
          targetServiceIds: [],
          dueTargetServiceIds: [],
        },
        sitePolicy: null,
        sitePolicySource: null,
        providers: [],
        challenges: [],
        decision: {
          recommendedAction: 'use_selected_profile',
          browserHost: 'local_headed',
          interactionMode: 'human_like_input',
          challengePolicy: 'avoid_first',
          profileId: 'journal-existing',
          manualActionRequired: false,
          manualSeedingRequired: false,
          monitorAttentionRequired: false,
          monitorProbeDue: false,
          monitorRunDue: {
            available: false,
            recommendedBeforeUse: false,
            monitorIds: [],
            neverCheckedMonitorIds: [],
            targetServiceIds: [],
            http: { method: 'POST', route: '/api/service/monitors/run-due' },
            mcp: { tool: 'service_monitors_run_due' },
            client: {
              package: '@agent-browser/client/service-observability',
              helper: 'runServiceAccessPlanMonitorRunDue',
            },
            fallbackClient: {
              package: '@agent-browser/client/service-observability',
              helper: 'runDueServiceMonitors',
            },
            cli: { command: 'agent-browser service monitors run-due' },
            requestFields: [],
            notes: [],
          },
          providerIds: [],
          challengeIds: [],
          reasons: ['profile_selected'],
        },
      },
    };
  });
  const dueMonitorAcquisitionResult = await acquireServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: dueMonitorAcquisition.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'agent-a',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    runDueReadinessMonitor: true,
  });
  assert.equal(dueMonitorAcquisition.calls.length, 3);
  assert.equal(dueMonitorAcquisition.calls[0].init.method, 'GET');
  assert.equal(
    dueMonitorAcquisition.calls[1].url,
    'http://127.0.0.1:4849/api/service/monitors/run-due',
  );
  assert.equal(dueMonitorAcquisition.calls[1].init.method, 'POST');
  assert.equal(dueMonitorAcquisition.calls[2].init.method, 'GET');
  assert.equal(dueMonitorAcquisitionResult.monitorRunDue?.checked, 1);
  assert.equal(dueMonitorAcquisitionResult.monitorRunDueRan, true);
  assert.equal(dueMonitorAcquisitionResult.accessPlan.decision?.monitorProbeDue, false);
  assert.equal(dueMonitorAcquisitionResult.selectedProfile?.id, 'journal-existing');

  const fallbackAcquisition = createFetchRecorder((_url, _init, calls) => {
    if (calls.length === 1) {
      return {
        success: true,
        data: {
          query: {
            serviceName: 'JournalDownloader',
            targetServiceIds: ['acs'],
            readinessProfileId: null,
          },
          selectedProfile: null,
          selectedProfileMatch: null,
          selectedProfileSource: null,
          readiness: null,
          readinessSummary: {
            needsManualSeeding: false,
            manualSeedingRequired: false,
            targetServiceIds: [],
            recommendedActions: [],
          },
          monitorFindings: {
            profileReadinessAttentionRequired: false,
            profileReadinessIncidentIds: [],
            profileReadinessMonitorIds: [],
            profileReadinessResults: [],
            targetServiceIds: [],
          },
          sitePolicy: null,
          sitePolicySource: null,
          providers: [],
          challenges: [],
          decision: {
            recommendedAction: 'register_profile',
            browserHost: null,
            interactionMode: null,
            challengePolicy: null,
            profileId: null,
            manualActionRequired: false,
            manualSeedingRequired: false,
            monitorAttentionRequired: false,
            providerIds: [],
            challengeIds: [],
            reasons: ['no_matching_profile'],
          },
        },
      };
    }
    if (calls.length === 2) {
      return {
        success: true,
        data: {
          id: 'journal-fallback',
          upserted: true,
          profile: calls.at(-1)?.body,
        },
      };
    }
    if (calls.length === 3) {
      return {
        success: true,
        data: {
          id: 'journal-acs-readiness',
          upserted: true,
          monitor: calls.at(-1)?.body,
        },
      };
    }
    return {
      success: true,
      data: {
        query: {
          serviceName: 'JournalDownloader',
          targetServiceIds: ['acs'],
          readinessProfileId: null,
        },
        selectedProfile: {
          id: 'journal-fallback',
          name: 'Journal ACS fallback',
          targetServiceIds: ['acs'],
          authenticatedServiceIds: [],
          sharedServiceIds: ['JournalDownloader'],
        },
        selectedProfileMatch: {
          profileId: 'journal-fallback',
          reason: 'target_match',
          matchedField: 'targetServiceIds',
          matchedIdentity: 'acs',
        },
        selectedProfileSource: 'persisted_state',
        readiness: null,
        readinessSummary: {
          needsManualSeeding: false,
          manualSeedingRequired: false,
          targetServiceIds: [],
          recommendedActions: [],
        },
        monitorFindings: {
          profileReadinessAttentionRequired: false,
          profileReadinessIncidentIds: [],
          profileReadinessMonitorIds: [],
          profileReadinessResults: [],
          targetServiceIds: [],
        },
        sitePolicy: null,
        sitePolicySource: null,
        providers: [],
        challenges: [],
        decision: {
          recommendedAction: 'use_selected_profile',
          browserHost: 'local_headed',
          interactionMode: 'human_like_input',
          challengePolicy: 'avoid_first',
          profileId: 'journal-fallback',
          manualActionRequired: false,
          manualSeedingRequired: false,
          monitorAttentionRequired: false,
          providerIds: [],
          challengeIds: [],
          reasons: ['profile_selected'],
        },
      },
    };
  });
  const fallbackAcquisitionResult = await acquireServiceLoginProfile({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: fallbackAcquisition.fetch,
    serviceName: 'JournalDownloader',
    agentName: 'agent-a',
    taskName: 'probeACSwebsite',
    loginId: 'acs',
    registerProfileId: 'journal-fallback',
    profileName: 'Journal ACS fallback',
    profileUserDataDir: '/tmp/journal-fallback',
    registerAuthenticated: false,
    registerReadinessMonitor: true,
    readinessMonitorId: 'journal-acs-readiness',
    readinessMonitorIntervalMs: 60000,
  });
  assert.equal(fallbackAcquisition.calls.length, 4);
  assert.equal(fallbackAcquisition.calls[0].init.method, 'GET');
  assert.equal(fallbackAcquisition.calls[1].url, 'http://127.0.0.1:4849/api/service/profiles/journal-fallback');
  assert.equal(fallbackAcquisition.calls[1].init.method, 'POST');
  assert.deepEqual(fallbackAcquisition.calls[1].body, {
    name: 'Journal ACS fallback',
    allocation: 'per_service',
    keyring: 'basic_password_store',
    persistent: true,
    targetServiceIds: ['acs'],
    authenticatedServiceIds: [],
    sharedServiceIds: ['JournalDownloader'],
    userDataDir: '/tmp/journal-fallback',
  });
  assert.equal(fallbackAcquisition.calls[2].url, 'http://127.0.0.1:4849/api/service/monitors/journal-acs-readiness');
  assert.equal(fallbackAcquisition.calls[2].init.method, 'POST');
  assert.deepEqual(fallbackAcquisition.calls[2].body, {
    name: 'JournalDownloader acs profile readiness',
    target: { profile_readiness: 'acs' },
    intervalMs: 60000,
    state: 'active',
  });
  assert.equal(fallbackAcquisition.calls[3].init.method, 'GET');
  assert.equal(fallbackAcquisitionResult.initialAccessPlan.selectedProfile, null);
  assert.equal(fallbackAcquisitionResult.accessPlan.selectedProfile?.id, 'journal-fallback');
  assert.equal(fallbackAcquisitionResult.selectedProfile?.id, 'journal-fallback');
  assert.equal(fallbackAcquisitionResult.profileRegistration?.id, 'journal-fallback');
  assert.equal(fallbackAcquisitionResult.profileReadinessMonitor?.id, 'journal-acs-readiness');
  assert.equal(fallbackAcquisitionResult.monitorRunDue, null);
  assert.equal(fallbackAcquisitionResult.registered, true);
  assert.equal(fallbackAcquisitionResult.monitorRegistered, true);
  assert.equal(fallbackAcquisitionResult.monitorRunDueRan, false);

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
      seedingMode: 'not_required',
      cdpAttachmentAllowedDuringSeeding: false,
      preferredKeyring: null,
      setupScopes: [],
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

  const seedingVerification = createFetchRecorder();
  await verifyServiceProfileSeeding({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: seedingVerification.fetch,
    id: 'journal-google',
    targetServiceId: 'google',
  });
  assert.equal(
    seedingVerification.calls[0].url,
    'http://127.0.0.1:4849/api/service/profiles/journal-google/freshness',
  );
  assert.deepEqual(seedingVerification.calls[0].body, {
    targetServiceId: 'google',
    targetServiceIds: ['google'],
    readinessState: 'fresh',
    readinessEvidence: 'post_seeding_auth_probe_fresh',
    updateAuthenticatedServiceIds: true,
  });

  const accessPlanProbe = createFetchRecorder((url, init, calls) => {
    const parsed = new URL(url);
    const body = init.body ? JSON.parse(String(init.body)) : null;
    if (parsed.pathname === '/api/service/profiles/lookup') {
      return {
        success: true,
        data: {
          selectedProfile: {
            id: 'journal-google',
            targetServiceIds: ['google'],
            authenticatedServiceIds: ['google'],
          },
          selectedProfileMatch: {
            reason: 'authenticated_target',
            matchedField: 'authenticatedServiceIds',
            matchedIdentity: 'google',
          },
        },
      };
    }
    if (parsed.pathname === '/api/service/request') {
      if (body.action === 'tab_new') {
        return { success: true, data: { index: 0, url: 'https://myaccount.google.com/' } };
      }
      if (body.action === 'url') {
        return { success: true, data: { url: 'https://myaccount.google.com/' } };
      }
      if (body.action === 'title') {
        return { success: true, data: { title: 'Google Account' } };
      }
    }
    if (parsed.pathname === '/api/service/profiles/journal-google/freshness') {
      return {
        success: true,
        data: {
          id: 'journal-google',
          upserted: true,
          profile: calls.at(-1)?.body,
        },
      };
    }
    throw new Error(`Unexpected post-seeding probe request: ${init.method || 'GET'} ${parsed.pathname}`);
  });
  const accessPlanProbeResult = await runServiceAccessPlanPostSeedingProbe({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: accessPlanProbe.fetch,
    accessPlan: serviceAccessPlan(),
    expectedUrlIncludes: 'myaccount.google.com',
    expectedTitleIncludes: 'Google Account',
  });
  assert.equal(accessPlanProbeResult.checks.fresh, true);
  assert.deepEqual(
    accessPlanProbe.calls.map((call) => `${call.init.method || 'GET'} ${new URL(call.url).pathname}`),
    [
      'GET /api/service/profiles/lookup',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/profiles/journal-google/freshness',
    ],
  );
  assert.equal(accessPlanProbe.calls[1].body.action, 'tab_new');
  assert.equal(accessPlanProbe.calls[1].body.params.url, 'https://myaccount.google.com/');
  assert.equal(accessPlanProbe.calls[4].body.readinessState, 'fresh');
  assert.match(accessPlanProbe.calls[4].body.readinessEvidence, /^post_seeding_auth_probe_passed:/);

  const accessPlanProbeMismatch = createFetchRecorder((url) => {
    const parsed = new URL(url);
    if (parsed.pathname === '/api/service/profiles/lookup') {
      return {
        success: true,
        data: {
          selectedProfile: {
            id: 'personal-google',
          },
        },
      };
    }
    throw new Error(`Unexpected mismatch request: ${parsed.pathname}`);
  });
  await assert.rejects(
    () =>
      runServiceAccessPlanPostSeedingProbe({
        baseUrl: 'http://127.0.0.1:4849',
        fetch: accessPlanProbeMismatch.fetch,
        accessPlan: serviceAccessPlan(),
      }),
    /broker selected personal-google/,
  );
  assert.equal(accessPlanProbeMismatch.calls.length, 1);

  console.log('Service observability client helper tests passed');
}

function serviceAccessPlan() {
  return {
    query: {
      serviceName: 'JournalDownloader',
      agentName: 'codex',
      taskName: 'verifyGoogle',
      targetServiceIds: ['google'],
    },
    sitePolicy: {
      originPattern: 'https://myaccount.google.com/',
    },
    decision: {
      postSeedingProbe: {
        available: true,
        recommendedAfterClose: true,
        profileId: 'journal-google',
        targetServiceId: 'google',
        targetServiceIds: ['google'],
        boundedChecks: ['broker_selected_profile_matches_profile_id', 'url_read', 'title_read'],
        http: {
          method: 'POST',
          route: '/api/service/profiles/journal-google/freshness',
          routeTemplate: '/api/service/profiles/<id>/freshness',
        },
        mcp: {
          tool: 'service_profile_freshness_update',
        },
        client: {
          package: '@agent-browser/client/service-observability',
          helper: 'verifyServiceProfileSeeding',
        },
        serviceClientExample: {
          package: 'agent-browser-service-client-example',
          script: 'examples/service-client/post-seeding-probe.mjs',
          command:
            'pnpm --filter agent-browser-service-client-example exec node examples/service-client/post-seeding-probe.mjs --base-url http://127.0.0.1:<stream-port> --profile-id journal-google --target-service-id google',
        },
        cli: {
          command:
            'agent-browser service profiles journal-google verify-seeding google --state fresh --evidence <probe-evidence>',
        },
        requestFields: ['profileId', 'targetServiceId', 'readinessState'],
        notes: ['Run only after detached CDP-free seeding has closed.'],
      },
    },
  };
}

function serviceAccessPlanWithDueMonitor() {
  return {
    query: {
      serviceName: 'JournalDownloader',
      agentName: 'codex',
      taskName: 'verifyGoogle',
      targetServiceIds: ['google'],
    },
    decision: {
      monitorRunDue: {
        available: true,
        recommendedBeforeUse: true,
        monitorIds: ['google-login-freshness'],
        neverCheckedMonitorIds: ['google-login-freshness'],
        targetServiceIds: ['google'],
        http: {
          method: 'POST',
          route: '/api/service/monitors/run-due',
        },
        mcp: {
          tool: 'service_monitors_run_due',
        },
        client: {
          package: '@agent-browser/client/service-observability',
          helper: 'runServiceAccessPlanMonitorRunDue',
        },
        fallbackClient: {
          package: '@agent-browser/client/service-observability',
          helper: 'runDueServiceMonitors',
        },
        cli: {
          command: 'agent-browser service monitors run-due',
        },
        requestFields: [],
        notes: ['Runs all due active monitors through the service worker queue.'],
      },
    },
  };
}

function serviceAccessPlanWithoutDueMonitor() {
  return {
    query: {
      serviceName: 'JournalDownloader',
      agentName: 'codex',
      taskName: 'verifyGoogle',
      targetServiceIds: ['google'],
    },
    decision: {
      monitorRunDue: {
        available: false,
        recommendedBeforeUse: false,
        monitorIds: [],
        neverCheckedMonitorIds: [],
        targetServiceIds: [],
      },
    },
  };
}

main().catch((err) => {
  console.error(err?.stack || err?.message || String(err));
  process.exit(1);
});
