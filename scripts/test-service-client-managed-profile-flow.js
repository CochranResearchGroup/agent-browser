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
await testMissingProfileRegistration();
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
  assert.equal(result.accessPlan?.decision?.recommendedAction, 'use_selected_profile');
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
    ['GET /api/service/access-plan', 'POST /api/service/profiles/canva-default', 'POST /api/service/request'],
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
  assert.equal(result.tab?.success, true);
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
  assert.equal(plan.tabRequest.helper, 'requestServiceTab');
  assert.deepEqual(plan.decisionOrder.slice(0, 4), [
    'ask agent-browser for the no-launch access plan',
    'inspect the service-owned profile, readiness, policy, provider, challenge, and decision fields',
    'request a tab by login or target identity',
    'register a managed profile only when agent-browser has no suitable one',
  ]);

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
  ]);
  assertContainsAll(serviceModeDocs, [
    'The normal request model is identity-first',
    '<code>getServiceAccessPlan()</code> for the target identity',
    '<code>requestServiceTab</code> or <code>POST /api/service/request</code>',
    'bring-your-own-profile workflows',
  ]);
  assertContainsAll(commandsDocs, [
    'Service requests should be identity-first',
    '`siteId`, `loginId`, or `targetServiceId`',
    'known-login overrides or bring-your-own-profile workflows',
  ]);
  assertContainsAll(skill, [
    '`getServiceAccessPlan()` with `serviceName`, `agentName`, `taskName`',
    'request the tab by the same identity through `requestServiceTab()`',
    'Register a new managed login profile only when agent-browser',
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
  rejectRegistration = false,
}) {
  return async (url, init = {}) => {
    const parsed = new URL(String(url));
    const method = init.method || 'GET';
    calls.push({ method, path: parsed.pathname, body: init.body });

    if (method === 'GET' && parsed.pathname === '/api/service/access-plan') {
      const selectedProfile = profiles[0] ?? null;
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
        sitePolicy: null,
        providers: [],
        challenges: [],
        decision: {
          recommendedAction:
            readinessState === 'needs_manual_seeding' && selectedProfile
              ? readinessRecommendedAction
              : selectedProfile
                ? 'use_selected_profile'
                : 'register_managed_profile_or_request_throwaway_browser',
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

    if (method === 'POST' && parsed.pathname === '/api/service/profiles/canva-default') {
      if (rejectRegistration) {
        return jsonResponse({ error: 'profile registration should not be called' }, { status: 500 });
      }
      const requestedProfile = JSON.parse(String(init.body));
      return serviceResponse({
        upserted: true,
        profile: {
          id: 'canva-default',
          ...requestedProfile,
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

function jsonResponse(body, { status = 200 } = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function serviceResponse(data, options) {
  return jsonResponse({ success: true, data }, options);
}
