#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import {
  buildManagedProfilePlan,
  runManagedProfileWorkflow,
} from '../examples/service-client/managed-profile-flow.mjs';

const profile = {
  id: 'canva-default',
  name: 'CanvaCLI canva managed profile',
  authenticatedServiceIds: ['canva'],
  targetServiceIds: ['canva'],
  sharedServiceIds: ['CanvaCLI'],
};

await testExistingProfileSelection();
await testExistingProfileBrowserCapabilityPreflight();
await testExistingProfileCdpFreeLaunch();
await testExistingProfileFreshnessUpdate();
await testExistingProfileDueMonitorRun();
await testMissingProfileRegistration();
await testMissingProfileRegistrationWithReadinessMonitor();
await testMissingProfileRegistrationThenDueMonitorRun();
await testManualSeedingReadinessSummary();
await testIdentityFirstGuidanceDrift();

console.log('Managed profile flow no-launch smoke passed');

async function testExistingProfileSelection() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    url: 'https://www.canva.com/',
  });

  assert.equal(result.dryRun, false);
  assert.equal(result.selectedProfile?.id, 'canva-default');
  assert.equal(result.selectedProfileMatch?.reason, 'authenticated_target');
  assert.equal(result.selectedProfileMatch?.profileId, 'canva-default');
  assert.equal(result.profileRegistration, null);
  assert.equal(result.readinessSummary?.needsManualSeeding, false);
  assert.equal(result.readinessSummary?.manualSeedingRequired, false);
  assert.equal(result.accessDecision?.recommendedAction, 'use_selected_profile');
  assert.deepEqual(result.accessAttention, {
    required: false,
    owner: 'none',
    severity: 'info',
    reason: 'use_selected_profile',
    message: 'No intervention required.',
    suggestedActions: ['request_service_tab'],
  });
  assert.equal(result.accessPlan?.decision?.recommendedAction, 'use_selected_profile');
  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: 'canva-default',
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: false,
    browserCapabilityPreflightRan: false,
    initialRecommendedAction: 'use_selected_profile',
    refreshedRecommendedAction: 'use_selected_profile',
    browserCapabilityPreflightApplied: null,
    browserCapabilityPreflightReason: null,
    monitorRunDueChecked: null,
    monitorRunDueFailed: null,
    monitorRunDueRecommendedAction: null,
    monitorRunDueFreshTargetServiceIds: [],
    monitorRunDueStaleProfileIds: [],
    initialAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
    refreshedAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
  });
  assert.equal(result.tab?.success, true);
  assert.equal(result.tab?.data?.url, 'https://www.canva.com/');

  const registrationCalls = calls.filter(
    (call) => call.method === 'POST' && call.path.startsWith('/api/service/profiles/'),
  );
  assert.deepEqual(registrationCalls, []);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    ['GET /api/service/access-plan', 'POST /api/service/request'],
  );
}

async function testExistingProfileBrowserCapabilityPreflight() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    runBrowserCapabilityPreflight: true,
    url: 'https://www.canva.com/',
  });

  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: 'canva-default',
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: false,
    browserCapabilityPreflightRan: true,
    initialRecommendedAction: 'use_selected_profile',
    refreshedRecommendedAction: 'use_selected_profile',
    browserCapabilityPreflightApplied: true,
    browserCapabilityPreflightReason: 'validated_preference_binding',
    monitorRunDueChecked: null,
    monitorRunDueFailed: null,
    monitorRunDueRecommendedAction: null,
    monitorRunDueFreshTargetServiceIds: [],
    monitorRunDueStaleProfileIds: [],
    initialAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
    refreshedAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
  });
  assert.equal(result.browserCapabilityPreflight?.wouldLaunch, false);
  assert.equal(result.tab?.success, true);
  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'GET /api/service/browser-capability/preflight',
      'POST /api/service/request',
    ],
  );
}

async function testExistingProfileCdpFreeLaunch() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    cdpFreeRequired: true,
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    url: 'https://www.canva.com/',
  });

  assert.equal(result.dryRun, false);
  assert.equal(result.selectedProfile?.id, 'canva-default');
  assert.equal(result.accessPlan?.decision?.launchPosture?.requiresCdpFree, true);
  assert.equal(result.cdpFreeAvailability?.applies, true);
  assert.deepEqual(result.cdpFreeAvailability?.availableCommands, ['cdp_free_launch']);
  assert.equal(result.cdpFreeAvailability?.unsupportedCommands.includes('snapshot'), true);
  assert.equal(result.cdpFreeAvailability?.unsupportedCommands.includes('click'), true);
  assert.equal(result.cdpFreeAvailability?.summaryHelper, 'summarizeServiceCdpFreeLaunchAvailability');
  assert.equal(result.tab?.success, true);
  assert.equal(result.tab?.mode, 'cdp_free_launch');
  assert.equal(result.tab?.data?.cdpFree, true);
  assert.deepEqual(result.tab?.data?.unsupportedOperations, [
    'cdp_commands',
    'snapshot',
    'screenshot',
    'dom_interaction',
  ]);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    ['GET /api/service/access-plan', 'POST /api/service/request'],
  );
  const serviceRequest = JSON.parse(String(calls[1].body));
  assert.equal(serviceRequest.action, 'cdp_free_launch');
  assert.equal(serviceRequest.requiresCdpFree, true);
  assert.equal(serviceRequest.cdpAttachmentAllowed, false);
  assert.equal(serviceRequest.url, 'https://www.canva.com/');
  assert.equal(serviceRequest.params.url, 'https://www.canva.com/');
}

async function testExistingProfileFreshnessUpdate() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    freshnessProfileId: 'canva-default',
    freshnessReadinessState: 'fresh',
    freshnessEvidence: 'auth_probe_cookie_present',
    freshnessLastVerifiedAt: '2026-05-06T15:00:00Z',
    freshnessExpiresAt: '2026-05-06T16:00:00Z',
    url: 'https://www.canva.com/',
  });

  assert.equal(result.selectedProfile?.id, 'canva-default');
  assert.equal(result.profileRegistration, null);
  assert.equal(result.profileFreshnessUpdate?.upserted, true);
  assert.equal(result.profileFreshnessUpdate?.profile?.id, 'canva-default');
  assert.deepEqual(result.profileFreshnessUpdate?.profile?.authenticatedServiceIds, ['canva']);
  assert.equal(result.profileFreshnessUpdate?.profile?.targetReadiness?.[0]?.state, 'fresh');
  assert.equal(result.profileFreshnessUpdate?.profile?.targetReadiness?.[0]?.evidence, 'auth_probe_cookie_present');
  assert.equal(result.tab?.success, true);

  const freshnessCall = calls.find(
    (call) => call.method === 'POST' && call.path === '/api/service/profiles/canva-default/freshness',
  );
  assert(freshnessCall, 'existing-profile path did not post freshness evidence');
  const freshnessBody = JSON.parse(String(freshnessCall.body));
  assert.equal(freshnessBody.loginId, 'canva');
  assert.equal(freshnessBody.targetServiceId, 'canva');
  assert.deepEqual(freshnessBody.targetServiceIds, ['canva']);
  assert.equal(freshnessBody.readinessState, 'fresh');
  assert.equal(freshnessBody.readinessEvidence, 'auth_probe_cookie_present');
  assert.equal(freshnessBody.lastVerifiedAt, '2026-05-06T15:00:00Z');
  assert.equal(freshnessBody.freshnessExpiresAt, '2026-05-06T16:00:00Z');
  assert.equal(freshnessBody.updateAuthenticatedServiceIds, true);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/profiles/canva-default/freshness',
      'GET /api/service/access-plan',
      'POST /api/service/request',
    ],
  );
}

async function testExistingProfileDueMonitorRun() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    dueMonitorRecommended: true,
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    runDueReadinessMonitor: true,
    url: 'https://www.canva.com/',
  });

  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: 'canva-default',
    registered: false,
    monitorRegistered: false,
    monitorRunDueRan: true,
    browserCapabilityPreflightRan: false,
    initialRecommendedAction: 'run_due_profile_readiness_monitor',
    refreshedRecommendedAction: 'use_selected_profile',
    browserCapabilityPreflightApplied: null,
    browserCapabilityPreflightReason: null,
    monitorRunDueChecked: 1,
    monitorRunDueFailed: 0,
    monitorRunDueRecommendedAction: 'use_selected_profile',
    monitorRunDueFreshTargetServiceIds: ['canva'],
    monitorRunDueStaleProfileIds: [],
    initialAttention: {
      required: true,
      owner: 'service',
      severity: 'warning',
      reason: 'run_due_profile_readiness_monitor',
      message: 'Run the due profile-readiness monitor before trusting retained profile freshness.',
      suggestedActions: ['run_due_profile_readiness_monitor', 'inspect_monitor_result'],
    },
    refreshedAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
  });
  assert.equal(result.monitorRunDue?.checked, 1);
  assert.equal(result.tab?.success, true);
  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/monitors/run-due',
      'GET /api/service/access-plan',
      'POST /api/service/request',
    ],
  );
}

async function testMissingProfileRegistration() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [],
    calls,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    registerProfileId: 'canva-default',
    url: 'https://www.canva.com/',
  });

  assert.equal(result.dryRun, false);
  assert.equal(result.selectedProfile, null);
  assert.equal(result.selectedProfileMatch, null);
  assert.equal(
    result.accessDecision?.recommendedAction,
    'register_managed_profile_or_request_throwaway_browser',
  );
  assert.equal(result.profileRegistration?.upserted, true);
  assert.equal(result.profileRegistration?.profile?.id, 'canva-default');
  assert.deepEqual(result.profileRegistration?.profile?.targetServiceIds, ['canva']);
  assert.deepEqual(result.profileRegistration?.profile?.authenticatedServiceIds, []);
  assert.deepEqual(result.profileRegistration?.profile?.sharedServiceIds, ['CanvaCLI']);
  assert.equal(result.tab?.success, true);
  assert.equal(result.tab?.data?.url, 'https://www.canva.com/');

  const registrationCall = calls.find(
    (call) => call.method === 'POST' && call.path === '/api/service/profiles/canva-default',
  );
  assert(registrationCall, 'missing-profile path did not register a managed profile');
  const registeredProfile = JSON.parse(String(registrationCall.body));
  assert.equal(registeredProfile.name, 'CanvaCLI canva managed profile');
  assert.equal(registeredProfile.persistent, true);
  assert.equal(registeredProfile.keyring, 'basic_password_store');
  assert.deepEqual(registeredProfile.targetServiceIds, ['canva']);
  assert.deepEqual(registeredProfile.authenticatedServiceIds, []);
  assert.deepEqual(registeredProfile.sharedServiceIds, ['CanvaCLI']);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/profiles/canva-default',
      'GET /api/service/access-plan',
      'POST /api/service/request',
    ],
  );
}

async function testMissingProfileRegistrationWithReadinessMonitor() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [],
    calls,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    registerProfileId: 'canva-default',
    registerReadinessMonitor: true,
    readinessMonitorIntervalMs: 900000,
    url: 'https://www.canva.com/',
  });

  assert.equal(result.profileRegistration?.upserted, true);
  assert.equal(result.profileReadinessMonitor?.upserted, true);
  assert.equal(result.profileReadinessMonitor?.monitor?.id, 'canvacli-canva-profile-readiness');
  assert.deepEqual(result.profileReadinessMonitor?.monitor?.target, { profile_readiness: 'canva' });
  assert.equal(result.profileReadinessMonitor?.monitor?.intervalMs, 900000);
  assert.equal(result.profileReadinessMonitor?.monitor?.state, 'active');
  assert.equal(result.tab?.success, true);

  const monitorCall = calls.find(
    (call) => call.method === 'POST' && call.path === '/api/service/monitors/canvacli-canva-profile-readiness',
  );
  assert(monitorCall, 'missing-profile path did not register a profile-readiness monitor');
  const monitor = JSON.parse(String(monitorCall.body));
  assert.equal(monitor.name, 'CanvaCLI canva profile readiness');
  assert.deepEqual(monitor.target, { profile_readiness: 'canva' });
  assert.equal(monitor.intervalMs, 900000);
  assert.equal(monitor.state, 'active');

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/profiles/canva-default',
      'POST /api/service/monitors/canvacli-canva-profile-readiness',
      'GET /api/service/access-plan',
      'POST /api/service/request',
    ],
  );
}

async function testMissingProfileRegistrationThenDueMonitorRun() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [],
    calls,
    selectRegisteredProfile: true,
    dueMonitorRecommended: true,
    dueMonitorRecommendedOnAccessPlan: 2,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    registerProfileId: 'canva-default',
    registerReadinessMonitor: true,
    readinessMonitorIntervalMs: 900000,
    runDueReadinessMonitor: true,
    url: 'https://www.canva.com/',
  });

  assert.deepEqual(result.profileAcquisitionSummary, {
    selectedProfileId: 'canva-default',
    registered: true,
    monitorRegistered: true,
    monitorRunDueRan: true,
    browserCapabilityPreflightRan: false,
    initialRecommendedAction: 'register_managed_profile_or_request_throwaway_browser',
    refreshedRecommendedAction: 'use_selected_profile',
    browserCapabilityPreflightApplied: null,
    browserCapabilityPreflightReason: null,
    monitorRunDueChecked: 1,
    monitorRunDueFailed: 0,
    monitorRunDueRecommendedAction: 'use_selected_profile',
    monitorRunDueFreshTargetServiceIds: ['canva'],
    monitorRunDueStaleProfileIds: [],
    initialAttention: {
      required: true,
      owner: 'client',
      severity: 'warning',
      reason: 'register_managed_profile_or_request_throwaway_browser',
      message: 'No matching managed profile was found.',
      suggestedActions: ['register_managed_profile', 'request_throwaway_browser'],
    },
    refreshedAttention: {
      required: false,
      owner: 'none',
      severity: 'info',
      reason: 'use_selected_profile',
      message: 'No intervention required.',
      suggestedActions: ['request_service_tab'],
    },
  });
  assert.equal(result.profileRegistration?.upserted, true);
  assert.equal(result.profileReadinessMonitor?.upserted, true);
  assert.equal(result.monitorRunDue?.checked, 1);
  assert.equal(result.accessPlan?.decision?.recommendedAction, 'use_selected_profile');
  assert.equal(result.selectedProfile?.id, 'canva-default');
  assert.equal(result.tab?.success, true);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/profiles/canva-default',
      'POST /api/service/monitors/canvacli-canva-profile-readiness',
      'GET /api/service/access-plan',
      'POST /api/service/monitors/run-due',
      'GET /api/service/access-plan',
      'POST /api/service/request',
    ],
  );
}

async function testManualSeedingReadinessSummary() {
  const calls = [];
  const fetch = createMockFetch({
    profiles: [profile],
    calls,
    readinessState: 'needs_manual_seeding',
    readinessRecommendedAction: 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
    rejectRegistration: true,
  });

  const result = await runManagedProfileWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch,
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    readinessProfileId: 'canva-default',
    registerProfileId: 'canva-default',
    url: 'https://www.canva.com/',
  });

  assert.equal(result.selectedProfile?.id, 'canva-default');
  assert.equal(result.profileRegistration, null);
  assert.equal(result.readinessSummary?.needsManualSeeding, true);
  assert.equal(result.readinessSummary?.manualSeedingRequired, true);
  assert.deepEqual(result.readinessSummary?.targetServiceIds, ['canva']);
  assert.deepEqual(result.readinessSummary?.recommendedActions, [
    'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
  ]);
  assert.equal(
    result.accessDecision?.recommendedAction,
    'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
  );
  assert.deepEqual(result.accessAttention, {
    required: true,
    owner: 'operator',
    severity: 'blocking',
    reason: 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
    message: 'Launch the profile without CDP, complete sign-in or setup, close the browser, then run the post-seeding probe.',
    suggestedActions: ['launch_detached_seeding', 'close_seeded_browser', 'run_post_seeding_probe'],
  });
  assert.equal(result.tab?.success, false);
  assert.equal(result.tab?.skipped, true);
  assert.equal(result.tab?.reason, 'manual_seeding_required');
  assert.equal(
    result.tab?.seedingHandoff?.command,
    'agent-browser --runtime-profile canva-default runtime login about:blank',
  );
  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    ['GET /api/service/access-plan'],
  );
}

async function testIdentityFirstGuidanceDrift() {
  const plan = buildManagedProfilePlan({
    serviceName: 'CanvaCLI',
    agentName: 'canva-cli-agent',
    taskName: 'openCanvaWorkspace',
    loginId: 'canva',
    targetServiceId: 'canva',
    registerProfileId: 'canva-default',
  });

  assert.equal(plan.profileInspection.helper, 'getServiceAccessPlan');
  assert.equal(plan.tabRequest.helper, 'requestServiceTab or requestServiceCdpFreeLaunch');
  assert.equal(plan.tabRequest.accessPlan, 'getServiceAccessPlan response');
  assert.deepEqual(plan.tabRequest.overrides, ['url', 'jobTimeoutMs']);
  assert.deepEqual(plan.decisionOrder.slice(0, 4), [
    'ask agent-browser for the no-launch access plan',
    'inspect the service-owned profile, readiness, policy, provider, challenge, and decision fields',
    'inspect decision.attention before choosing a client prompt, log, or popup',
    'register a managed profile only when agent-browser has no suitable one',
  ]);
  assert(plan.profileInspection.includes.includes('decision.attention'));

  const [readme, serviceModeDocs, commandsDocs, skill] = await Promise.all([
    readFile(new URL('../README.md', import.meta.url), 'utf8'),
    readFile(new URL('../docs/src/app/service-mode/page.mdx', import.meta.url), 'utf8'),
    readFile(new URL('../docs/src/app/commands/page.mdx', import.meta.url), 'utf8'),
    readFile(new URL('../skills/agent-browser/SKILL.md', import.meta.url), 'utf8'),
  ]);

  assertContainsAll(readme, [
    'The normal service request is identity-first',
    'should call `getServiceAccessPlan()`',
    'request the tab by the same identity through `requestServiceTab()`',
    'Direct profile selection is an override',
    'updateServiceProfileFreshness()',
    'upsertServiceProfileReadinessMonitor()',
    '`decision.attention`',
  ]);
  assertContainsAll(serviceModeDocs, [
    'The normal request model is identity-first',
    '<code>getServiceAccessPlan()</code> for the target identity',
    '<code>requestServiceTab</code> or <code>POST /api/service/request</code>',
    'bring-your-own-profile workflows',
    '<code>updateServiceProfileFreshness()</code>',
    '<code>upsertServiceProfileReadinessMonitor()</code>',
    '<code>attention</code>',
  ]);
  assertContainsAll(commandsDocs, [
    'Service requests should be identity-first',
    '`siteId`, `loginId`, or `targetServiceId`',
    'known-login overrides or bring-your-own-profile workflows',
    'POST /api/service/profiles/<id>/freshness',
    '`profile_readiness` monitor',
    'decision.attention',
  ]);
  assertContainsAll(skill, [
    '`getServiceAccessPlan()` with `serviceName`, `agentName`, `taskName`',
    'request the tab by the same identity through `requestServiceTab()`',
    'Register a new managed login profile only when agent-browser',
    'MCP `service_profile_freshness_update`',
    '`upsertServiceProfileReadinessMonitor()`',
    '`attention`',
  ]);
}

function assertContainsAll(text, needles) {
  const normalizedText = normalizeWhitespace(text);
  for (const needle of needles) {
    assert(
      normalizedText.includes(normalizeWhitespace(needle)),
      `expected documentation guidance to include: ${needle}`,
    );
  }
}

function normalizeWhitespace(value) {
  return value.replace(/\s+/g, ' ').trim();
}

function createMockFetch({
  profiles,
  calls,
  readinessState = 'ready',
  readinessRecommendedAction = 'request_tab_by_login_identity',
  cdpFreeRequired = false,
  dueMonitorRecommended = false,
  dueMonitorRecommendedOnAccessPlan = 1,
  selectRegisteredProfile = false,
  rejectRegistration = false,
}) {
  let accessPlanCount = 0;
  let registeredProfile = null;
  return async (url, init = {}) => {
    const parsed = new URL(String(url));
    const method = init.method || 'GET';
    calls.push({ method, path: parsed.pathname, body: init.body });

    if (method === 'GET' && parsed.pathname === '/api/service/access-plan') {
      accessPlanCount += 1;
      const selectedProfile = profiles[0] ?? (selectRegisteredProfile ? registeredProfile : null);
      const shouldRecommendDueMonitor =
        dueMonitorRecommended &&
        accessPlanCount === dueMonitorRecommendedOnAccessPlan &&
        Boolean(selectedProfile);
      const targetReadiness = selectedProfile
        ? [
            {
              targetServiceId: 'canva',
              loginId: 'canva',
              state: readinessState,
              manualSeedingRequired: readinessState === 'needs_manual_seeding',
              evidence: 'mocked no-launch readiness',
              recommendedAction: readinessRecommendedAction,
              lastVerifiedAt: null,
              freshnessExpiresAt: null,
            },
          ]
        : [];
      const readiness = selectedProfile
        ? {
            profileId: 'canva-default',
            targetReadiness,
            count: targetReadiness.length,
          }
        : null;
      const recommendedAction = shouldRecommendDueMonitor
        ? 'run_due_profile_readiness_monitor'
        : readinessState === 'needs_manual_seeding' && selectedProfile
          ? readinessRecommendedAction
          : selectedProfile
            ? 'use_selected_profile'
            : 'register_managed_profile_or_request_throwaway_browser';
      return serviceResponse({
        query: {
          serviceName: 'CanvaCLI',
          targetServiceIds: ['canva'],
          sitePolicyId: null,
          challengeId: null,
          readinessProfileId: parsed.searchParams.get('readinessProfileId'),
        },
        selectedProfile,
        selectedProfileMatch: selectedProfile
          ? {
              profileId: selectedProfile.id,
              profile: selectedProfile,
              reason: 'authenticated_target',
              matchedField: 'authenticatedServiceIds',
              matchedIdentity: 'canva',
            }
          : null,
        readiness,
        readinessSummary: {
          needsManualSeeding: readinessState === 'needs_manual_seeding' && Boolean(selectedProfile),
          manualSeedingRequired: readinessState === 'needs_manual_seeding' && Boolean(selectedProfile),
          targetServiceIds: readinessState === 'needs_manual_seeding' && selectedProfile ? ['canva'] : [],
          recommendedActions:
            readinessState === 'needs_manual_seeding' && selectedProfile ? [readinessRecommendedAction] : [],
        },
        seedingHandoff:
          readinessState === 'needs_manual_seeding' && selectedProfile
            ? {
                profileId: selectedProfile.id,
                targetServiceId: 'canva',
                command: 'agent-browser --runtime-profile canva-default runtime login about:blank',
              }
            : null,
        sitePolicy: null,
        providers: [],
        challenges: [],
        decision: {
          recommendedAction,
          attention: mockAccessPlanAttention(recommendedAction),
          monitorRunDue: {
            available: shouldRecommendDueMonitor,
            recommendedBeforeUse: shouldRecommendDueMonitor,
            monitorIds: shouldRecommendDueMonitor ? ['canvacli-canva-profile-readiness'] : [],
            neverCheckedMonitorIds: [],
            targetServiceIds: shouldRecommendDueMonitor ? ['canva'] : [],
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
            available: !(readinessState === 'needs_manual_seeding' && Boolean(selectedProfile)) && !cdpFreeRequired,
            blockedByManualAction: readinessState === 'needs_manual_seeding' && Boolean(selectedProfile),
            blockedByCdpFree: cdpFreeRequired,
            requiresCdpFree: cdpFreeRequired,
            cdpAttachmentAllowed: cdpFreeRequired ? false : true,
            cdpFreeAvailability: mockCdpFreeAvailability(cdpFreeRequired),
            request: {
              serviceName: 'CanvaCLI',
              agentName: 'canva-cli-agent',
              taskName: 'openCanvaWorkspace',
              loginId: 'canva',
              targetServiceId: 'canva',
              profileLeasePolicy: 'wait',
              action: 'tab_new',
              ...(cdpFreeRequired
                ? {
                    requiresCdpFree: true,
                    cdpAttachmentAllowed: false,
                  }
                : {}),
              ...(readinessState === 'needs_manual_seeding' && Boolean(selectedProfile)
                ? {
                    blockedByManualAction: true,
                    manualSeedingRequired: true,
                  }
                : {}),
            },
          },
          launchPosture: {
            browserHost: 'local_headed',
            requiresCdpFree: cdpFreeRequired,
            cdpAttachmentAllowed: !cdpFreeRequired,
          },
          browserCapabilityPreflight: {
            available: Boolean(selectedProfile),
            recommendedBeforeUse: Boolean(selectedProfile),
            reason: selectedProfile ? 'browser_capability_evidence_available' : 'browser_build_unavailable',
            selectedProfileId: selectedProfile?.id ?? null,
            browserBuild: selectedProfile ? 'stealthcdp_chromium' : null,
            request: selectedProfile
              ? {
                  browserBuild: 'stealthcdp_chromium',
                  serviceName: 'CanvaCLI',
                  agentName: 'canva-cli-agent',
                  taskName: 'openCanvaWorkspace',
                  targetServiceIds: ['canva'],
                  runtimeProfile: selectedProfile.id,
                  headed: true,
                }
              : {},
          },
          browserHost: null,
          interactionMode: null,
          challengePolicy: null,
          profileId: selectedProfile?.id ?? null,
          manualActionRequired: readinessState === 'needs_manual_seeding' && Boolean(selectedProfile),
          manualSeedingRequired: readinessState === 'needs_manual_seeding' && Boolean(selectedProfile),
          providerIds: [],
          challengeIds: [],
          reasons: selectedProfile ? ['selected_profile_has_readiness_evidence'] : ['no_matching_profile'],
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/monitors/run-due') {
      return serviceResponse({
        checked: 1,
        succeeded: 1,
        failed: 0,
        monitorIds: ['canvacli-canva-profile-readiness'],
        results: [
          {
            monitorId: 'canvacli-canva-profile-readiness',
            checkedAt: '2026-05-10T00:00:00Z',
            success: true,
            result: 'profile_readiness_fresh',
            target: { profile_readiness: 'canva' },
            staleProfileIds: [],
          },
        ],
      });
    }

    if (method === 'GET' && parsed.pathname === '/api/service/browser-capability/preflight') {
      assert.equal(parsed.searchParams.get('browserBuild'), 'stealthcdp_chromium');
      assert.equal(parsed.searchParams.get('serviceName'), 'CanvaCLI');
      assert.equal(parsed.searchParams.get('agentName'), 'canva-cli-agent');
      assert.equal(parsed.searchParams.get('taskName'), 'openCanvaWorkspace');
      assert.equal(parsed.searchParams.get('targetServiceIds'), 'canva');
      assert.equal(parsed.searchParams.get('runtimeProfile'), 'canva-default');
      return serviceResponse({
        preflight: true,
        wouldLaunch: false,
        wouldApplyExecutable: true,
        browserCapabilityLaunch: {
          applied: true,
          reason: 'validated_preference_binding',
          browserBuild: 'stealthcdp_chromium',
          profileId: 'canva-default',
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/profiles/canva-default') {
      if (rejectRegistration) {
        return jsonResponse({ error: 'profile registration should not be called' }, { status: 500 });
      }
      const requestedProfile = JSON.parse(String(init.body));
      registeredProfile = {
        id: 'canva-default',
        ...requestedProfile,
      };
      return serviceResponse({
        upserted: true,
        profile: registeredProfile,
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/profiles/canva-default/freshness') {
      const requestedFreshness = JSON.parse(String(init.body));
      return serviceResponse({
        upserted: true,
        profile: {
          ...profile,
          targetReadiness: [
            {
              targetServiceId: requestedFreshness.targetServiceId,
              loginId: requestedFreshness.loginId,
              state: requestedFreshness.readinessState,
              manualSeedingRequired: requestedFreshness.readinessState === 'needs_manual_seeding',
              evidence: requestedFreshness.readinessEvidence,
              recommendedAction: 'use_profile',
              lastVerifiedAt: requestedFreshness.lastVerifiedAt,
              freshnessExpiresAt: requestedFreshness.freshnessExpiresAt,
            },
          ],
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/monitors/canvacli-canva-profile-readiness') {
      const requestedMonitor = JSON.parse(String(init.body));
      return serviceResponse({
        upserted: true,
        monitor: {
          id: 'canvacli-canva-profile-readiness',
          lastCheckedAt: null,
          lastSucceededAt: null,
          lastFailedAt: null,
          lastResult: null,
          consecutiveFailures: 0,
          ...requestedMonitor,
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/request') {
      const request = JSON.parse(String(init.body));
      assert.equal(request.serviceName, 'CanvaCLI');
      assert.equal(request.agentName, 'canva-cli-agent');
      assert.equal(request.taskName, 'openCanvaWorkspace');
      assert.equal(request.loginId, 'canva');
      assert.equal(request.targetServiceId, 'canva');
      assert.equal(request.profileLeasePolicy, 'wait');
      if (request.action === 'cdp_free_launch') {
        assert.equal(request.requiresCdpFree, true);
        assert.equal(request.cdpAttachmentAllowed, false);
        return jsonResponse({
          success: true,
          data: {
            launched: true,
            cdpFree: true,
            cdpAttachmentAllowed: false,
            browserId: 'session:canva-cli-agent',
            browserPid: 4242,
            userDataDir: '/tmp/canva-default',
            supportedOperations: ['process_lifecycle', 'profile_lease', 'service_state'],
            unsupportedOperations: ['cdp_commands', 'snapshot', 'screenshot', 'dom_interaction'],
            unsupportedCommands: ['snapshot', 'screenshot', 'click', 'fill'],
          },
        });
      }
      return jsonResponse({
        success: true,
        data: {
          index: 0,
          url: request.params?.url,
        },
      });
    }

    return jsonResponse({ error: `unexpected route: ${method} ${parsed.pathname}` }, { status: 404 });
  };
}

function mockCdpFreeAvailability(applies) {
  return {
    applies,
    controlPlaneMode: 'cdp_free',
    lifecycleOnly: applies,
    cdpAttachmentAllowed: !applies,
    supportedOperations: applies ? ['process_lifecycle', 'profile_lease', 'service_state'] : [],
    unsupportedOperations: applies
      ? ['cdp_commands', 'snapshot', 'screenshot', 'dom_interaction']
      : [],
    unsupportedCommands: applies ? ['snapshot', 'screenshot', 'click', 'fill'] : [],
    availableCommands: applies ? ['cdp_free_launch'] : [],
    hasUnsupportedCommandList: applies,
    client: {
      package: '@agent-browser/client/service-request',
      summaryHelper: 'summarizeServiceCdpFreeLaunchAvailability',
      predicateHelper: 'isServiceCdpFreeActionAvailable',
    },
  };
}

function mockAccessPlanAttention(recommendedAction) {
  switch (recommendedAction) {
    case 'run_due_profile_readiness_monitor':
      return {
        required: true,
        owner: 'service',
        severity: 'warning',
        reason: recommendedAction,
        message: 'Run the due profile-readiness monitor before trusting retained profile freshness.',
        suggestedActions: ['run_due_profile_readiness_monitor', 'inspect_monitor_result'],
        presentation: 'client_decides',
      };
    case 'register_managed_profile_or_request_throwaway_browser':
      return {
        required: true,
        owner: 'client',
        severity: 'warning',
        reason: recommendedAction,
        message: 'No matching managed profile was found.',
        suggestedActions: ['register_managed_profile', 'request_throwaway_browser'],
        presentation: 'client_decides',
      };
    case 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable':
      return {
        required: true,
        owner: 'operator',
        severity: 'blocking',
        reason: recommendedAction,
        message:
          'Launch the profile without CDP, complete sign-in or setup, close the browser, then run the post-seeding probe.',
        suggestedActions: ['launch_detached_seeding', 'close_seeded_browser', 'run_post_seeding_probe'],
        presentation: 'client_decides',
      };
    default:
      return {
        required: false,
        owner: 'none',
        severity: 'info',
        reason: recommendedAction,
        message: 'No intervention required.',
        suggestedActions: ['request_service_tab'],
        presentation: 'client_decides',
      };
  }
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
