#!/usr/bin/env node

import assert from 'node:assert/strict';

import { runServiceWorkflow } from '../examples/service-client/service-request-trace.mjs';

const serviceName = 'JournalDownloader';
const agentName = 'article-probe-agent';
const taskName = 'probeACSwebsite';
const loginId = 'example';
const profileId = 'journal-example';

await testSkipsRegistrationWhenBrokerSelectsProfile();
await testRegistersFallbackOnlyAfterAccessPlanMiss();
await testRunsDueMonitorAndShowsRefreshedRecommendation();
await testRegistersFallbackThenRunsDueMonitor();

console.log('Service client example no-launch broker tests passed');

async function testSkipsRegistrationWhenBrokerSelectsProfile() {
  const calls = [];
  const fetch = createBrokerFirstFetch({
    calls,
    initialSelectedProfile: brokerProfile(),
    failRegistration: true,
  });

  const result = await runServiceWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName,
    agentName,
    taskName,
    siteId: loginId,
    loginId,
    registerProfileId: profileId,
    registerReadinessMonitor: true,
    readinessMonitorIntervalMs: 900000,
    url: 'https://example.com',
  });

  assert.equal(result.initialAccessPlan?.selectedProfile?.id, profileId);
  assert.equal(result.accessPlan?.selectedProfile?.id, profileId);
  assert.equal(result.profileRegistration, null);
  assert.equal(result.profileReadinessMonitor, null);
  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: profileId,
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: false,
    initialRecommendedAction: 'use_selected_profile',
    refreshedRecommendedAction: 'use_selected_profile',
    monitorRunDueChecked: null,
    monitorRunDueFailed: null,
  });
  assert.equal(result.commandResult?.success, true);
  assert.deepEqual(callSequence(calls), [
    'GET /api/service/access-plan',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'GET /api/service/trace',
  ]);
}

async function testRegistersFallbackOnlyAfterAccessPlanMiss() {
  const calls = [];
  const fetch = createBrokerFirstFetch({
    calls,
    initialSelectedProfile: null,
    refreshedSelectedProfile: brokerProfile(),
  });

  const result = await runServiceWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName,
    agentName,
    taskName,
    siteId: loginId,
    loginId,
    registerProfileId: profileId,
    profileUserDataDir: '/tmp/journal-example-profile',
    registerReadinessMonitor: true,
    readinessMonitorIntervalMs: 900000,
    url: 'https://example.com',
  });

  assert.equal(result.initialAccessPlan?.selectedProfile, null);
  assert.equal(result.profileRegistration?.upserted, true);
  assert.equal(result.profileRegistration?.profile?.id, profileId);
  assert.deepEqual(result.profileRegistration?.profile?.targetServiceIds, [loginId]);
  assert.deepEqual(result.profileRegistration?.profile?.authenticatedServiceIds, [loginId]);
  assert.equal(result.profileReadinessMonitor?.upserted, true);
  assert.equal(result.profileReadinessMonitor?.monitor?.target?.profile_readiness, loginId);
  assert.equal(result.profileReadinessMonitor?.monitor?.intervalMs, 900000);
  assert.equal(result.accessPlan?.selectedProfile?.id, profileId);
  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: profileId,
    registered: true,
    monitorRegistered: true,
    monitorRunDueRan: false,
    initialRecommendedAction: 'register_managed_profile_or_request_throwaway_browser',
    refreshedRecommendedAction: 'use_selected_profile',
    monitorRunDueChecked: null,
    monitorRunDueFailed: null,
  });
  assert.equal(result.commandResult?.success, true);

  assert.deepEqual(callSequence(calls), [
    'GET /api/service/access-plan',
    `POST /api/service/profiles/${profileId}`,
    'POST /api/service/monitors/journaldownloader-example-profile-readiness',
    'GET /api/service/access-plan',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'GET /api/service/trace',
  ]);

  const registrationCall = calls.find((call) => call.path === `/api/service/profiles/${profileId}`);
  assert(registrationCall, 'missing-profile path did not register fallback profile');
  const registrationBody = JSON.parse(String(registrationCall.body));
  assert.equal(registrationBody.name, `${serviceName} ${loginId} profile`);
  assert.equal(registrationBody.userDataDir, '/tmp/journal-example-profile');
}

async function testRunsDueMonitorAndShowsRefreshedRecommendation() {
  const calls = [];
  const fetch = createBrokerFirstFetch({
    calls,
    initialSelectedProfile: brokerProfile(),
    dueMonitorRecommended: true,
  });

  const result = await runServiceWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName,
    agentName,
    taskName,
    siteId: loginId,
    loginId,
    runDueReadinessMonitor: true,
    url: 'https://example.com',
  });

  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: profileId,
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: true,
    initialRecommendedAction: 'run_due_profile_readiness_monitor',
    refreshedRecommendedAction: 'use_selected_profile',
    monitorRunDueChecked: 1,
    monitorRunDueFailed: 0,
  });
  assert.equal(result.monitorRunDue?.checked, 1);
  assert.deepEqual(callSequence(calls), [
    'GET /api/service/access-plan',
    'POST /api/service/monitors/run-due',
    'GET /api/service/access-plan',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'GET /api/service/trace',
  ]);
}

async function testRegistersFallbackThenRunsDueMonitor() {
  const calls = [];
  const fetch = createBrokerFirstFetch({
    calls,
    initialSelectedProfile: null,
    selectRegisteredProfile: true,
    dueMonitorRecommended: true,
    dueMonitorRecommendedOnAccessPlan: 2,
  });

  const result = await runServiceWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName,
    agentName,
    taskName,
    siteId: loginId,
    loginId,
    registerProfileId: profileId,
    profileUserDataDir: '/tmp/journal-example-profile',
    registerReadinessMonitor: true,
    readinessMonitorIntervalMs: 900000,
    runDueReadinessMonitor: true,
    url: 'https://example.com',
  });

  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: profileId,
    registered: true,
    monitorRegistered: true,
    monitorRunDueRan: true,
    initialRecommendedAction: 'register_managed_profile_or_request_throwaway_browser',
    refreshedRecommendedAction: 'use_selected_profile',
    monitorRunDueChecked: 1,
    monitorRunDueFailed: 0,
  });
  assert.equal(result.profileRegistration?.upserted, true);
  assert.equal(result.profileReadinessMonitor?.upserted, true);
  assert.equal(result.monitorRunDue?.checked, 1);
  assert.equal(result.accessPlan?.selectedProfile?.id, profileId);
  assert.equal(result.accessPlan?.decision?.recommendedAction, 'use_selected_profile');
  assert.equal(result.commandResult?.success, true);

  assert.deepEqual(callSequence(calls), [
    'GET /api/service/access-plan',
    `POST /api/service/profiles/${profileId}`,
    'POST /api/service/monitors/journaldownloader-example-profile-readiness',
    'GET /api/service/access-plan',
    'POST /api/service/monitors/run-due',
    'GET /api/service/access-plan',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'POST /api/service/request',
    'GET /api/service/trace',
  ]);
}

function createBrokerFirstFetch({
  calls,
  initialSelectedProfile,
  refreshedSelectedProfile = initialSelectedProfile,
  dueMonitorRecommended = false,
  dueMonitorRecommendedOnAccessPlan = 1,
  selectRegisteredProfile = false,
  failRegistration = false,
}) {
  let accessPlanCount = 0;
  let registeredProfile = null;

  return async (url, init = {}) => {
    const parsed = new URL(String(url));
    const method = init.method || 'GET';
    calls.push({ method, path: parsed.pathname, body: init.body });

    if (method === 'GET' && parsed.pathname === '/api/service/access-plan') {
      accessPlanCount += 1;
      const selectedProfile =
        accessPlanCount === 1
          ? initialSelectedProfile
          : selectRegisteredProfile
            ? registeredProfile
            : refreshedSelectedProfile;
      return serviceResponse(
        accessPlanResponse(selectedProfile, {
          dueMonitorRecommended: dueMonitorRecommended && accessPlanCount === dueMonitorRecommendedOnAccessPlan,
        }),
      );
    }

    if (method === 'POST' && parsed.pathname === '/api/service/monitors/run-due') {
      return serviceResponse({
        checked: 1,
        succeeded: 1,
        failed: 0,
        monitorIds: ['journaldownloader-example-profile-readiness'],
        results: [
          {
            monitorId: 'journaldownloader-example-profile-readiness',
            checkedAt: '2026-05-10T00:00:00Z',
            success: true,
            result: 'profile_readiness_fresh',
            target: { profile_readiness: 'acs' },
            staleProfileIds: [],
          },
        ],
      });
    }

    if (method === 'POST' && parsed.pathname === `/api/service/profiles/${profileId}`) {
      if (failRegistration) {
        return jsonResponse({ error: 'profile registration should not be called' }, { status: 500 });
      }
      const requestedProfile = JSON.parse(String(init.body));
      registeredProfile = {
        id: profileId,
        ...requestedProfile,
      };
      return serviceResponse({
        upserted: true,
        profile: registeredProfile,
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/monitors/journaldownloader-example-profile-readiness') {
      const requestedMonitor = JSON.parse(String(init.body));
      return serviceResponse({
        upserted: true,
        monitor: {
          id: 'journaldownloader-example-profile-readiness',
          ...requestedMonitor,
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/request') {
      const request = JSON.parse(String(init.body));
      assert.equal(request.serviceName, serviceName);
      assert.equal(request.agentName, agentName);
      assert.equal(request.taskName, taskName);
      assert.equal(request.loginId, loginId);
      if (request.action === 'tab_new') {
        assert.equal(request.profileLeasePolicy, 'wait');
        return jsonResponse({
          success: true,
          data: {
            index: 0,
            url: request.params?.url,
          },
        });
      }
      if (request.action === 'title') {
        return jsonResponse({ success: true, data: { title: 'Example title' } });
      }
      if (request.action === 'wait') {
        return jsonResponse({ success: true, data: { waited: 'timeout' } });
      }
      if (request.action === 'viewport') {
        return jsonResponse({ success: true, data: { width: request.params?.width, height: request.params?.height } });
      }
      if (request.action === 'console') {
        return jsonResponse({ success: true, data: { messages: [] } });
      }
    }

    if (method === 'GET' && parsed.pathname === '/api/service/trace') {
      return serviceResponse({
        counts: {
          events: 1,
          jobs: 1,
          incidents: 0,
          activity: 0,
        },
        jobs: [
          {
            id: 'job-tab-new',
            action: 'tab_new',
            state: 'succeeded',
            serviceName,
            agentName,
            taskName,
            controlPlaneMode: 'cdp',
            lifecycleOnly: false,
          },
        ],
      });
    }

    return jsonResponse({ error: `unexpected route: ${method} ${parsed.pathname}` }, { status: 404 });
  };
}

function accessPlanResponse(selectedProfile, { dueMonitorRecommended = false } = {}) {
  return {
    query: {
      serviceName,
      agentName,
      taskName,
      siteId: loginId,
      loginId,
      targetServiceIds: [loginId],
    },
    selectedProfile,
    selectedProfileMatch: selectedProfile
      ? {
          profileId: selectedProfile.id,
          profile: selectedProfile,
          reason: 'authenticated_target',
          matchedField: 'authenticatedServiceIds',
          matchedIdentity: loginId,
        }
      : null,
    readinessSummary: {
      needsManualSeeding: false,
      manualSeedingRequired: false,
      targetServiceIds: [],
      recommendedActions: [],
    },
    seedingHandoff: null,
    sitePolicy: null,
    providers: [],
    challenges: [],
    decision: {
      recommendedAction: dueMonitorRecommended
        ? 'run_due_profile_readiness_monitor'
        : selectedProfile
          ? 'use_selected_profile'
          : 'register_managed_profile_or_request_throwaway_browser',
      monitorRunDue: {
        available: dueMonitorRecommended,
        recommendedBeforeUse: dueMonitorRecommended,
        monitorIds: dueMonitorRecommended ? ['journaldownloader-example-profile-readiness'] : [],
        neverCheckedMonitorIds: [],
        targetServiceIds: dueMonitorRecommended ? [loginId] : [],
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
      serviceRequest: {
        available: true,
        recommendedAfterManualAction: false,
        blockedByManualAction: false,
        request: {
          serviceName,
          agentName,
          taskName,
          siteId: loginId,
          loginId,
          targetServiceIds: [loginId],
          profileLeasePolicy: 'wait',
          action: 'tab_new',
        },
      },
      manualActionRequired: false,
      manualSeedingRequired: false,
      reasons: selectedProfile ? ['selected_profile_has_readiness_evidence'] : ['no_matching_profile'],
    },
  };
}

function brokerProfile() {
  return {
    id: profileId,
    name: 'JournalDownloader example profile',
    targetServiceIds: [loginId],
    authenticatedServiceIds: [loginId],
    sharedServiceIds: [serviceName],
    persistent: true,
  };
}

function callSequence(calls) {
  return calls.map((call) => `${call.method} ${call.path}`);
}

function jsonResponse(body, { status = 200 } = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function serviceResponse(data, options) {
  return jsonResponse({ success: true, data }, options);
}
