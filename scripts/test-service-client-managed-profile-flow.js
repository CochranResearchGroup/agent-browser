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

const calls = [];

const fetch = async (url, init = {}) => {
  const parsed = new URL(String(url));
  const method = init.method || 'GET';
  calls.push({ method, path: parsed.pathname, body: init.body });

  if (method === 'GET' && parsed.pathname === '/api/service/profiles') {
    return serviceResponse({ profiles: [profile], profileAllocations: [], count: 1 });
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

  if (method === 'POST' && parsed.pathname.startsWith('/api/service/profiles/')) {
    return jsonResponse({ error: 'profile registration should not be called' }, { status: 500 });
  }

  return jsonResponse({ error: `unexpected route: ${method} ${parsed.pathname}` }, { status: 404 });
};

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

console.log('Managed profile flow no-launch smoke passed');

function jsonResponse(body, { status = 200 } = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}

function serviceResponse(data, options) {
  return jsonResponse({ success: true, data }, options);
}
