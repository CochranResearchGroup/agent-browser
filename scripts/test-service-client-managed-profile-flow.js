#!/usr/bin/env node

import assert from 'node:assert/strict';

import { runManagedProfileWorkflow } from '../examples/service-client/managed-profile-flow.mjs';

const profile = {
  id: 'canva-default',
  name: 'CanvaCLI canva managed profile',
  authenticatedServiceIds: ['canva'],
  targetServiceIds: ['canva'],
  sharedServiceIds: ['CanvaCLI'],
};

await testExistingProfileSelection();
await testMissingProfileRegistration();

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
  assert.equal(result.profileRegistration, null);
  assert.equal(result.tab?.success, true);
  assert.equal(result.tab?.data?.url, 'https://www.canva.com/');

  const registrationCalls = calls.filter(
    (call) => call.method === 'POST' && call.path.startsWith('/api/service/profiles/'),
  );
  assert.deepEqual(registrationCalls, []);

  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    ['GET /api/service/profiles', 'GET /api/service/profiles/canva-default/readiness', 'POST /api/service/request'],
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
    ['GET /api/service/profiles', 'POST /api/service/profiles/canva-default', 'POST /api/service/request'],
  );
}

function createMockFetch({ profiles, calls, rejectRegistration = false }) {
  return async (url, init = {}) => {
    const parsed = new URL(String(url));
    const method = init.method || 'GET';
    calls.push({ method, path: parsed.pathname, body: init.body });

    if (method === 'GET' && parsed.pathname === '/api/service/profiles') {
      return serviceResponse({ profiles, profileAllocations: [], count: profiles.length });
    }

    if (method === 'GET' && parsed.pathname === '/api/service/profiles/canva-default/readiness') {
      return serviceResponse({
        profileId: 'canva-default',
        targetReadiness: [
          {
            targetServiceId: 'canva',
            readiness: 'ready',
            recommendedAction: 'request_tab_by_login_identity',
          },
        ],
        count: 1,
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
