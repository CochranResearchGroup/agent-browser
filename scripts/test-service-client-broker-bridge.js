#!/usr/bin/env node

import assert from 'node:assert/strict';

import { runBrokerBridgeWorkflow } from '../examples/service-client/broker-bridge.mjs';

const serviceTabHandle = {
  browserId: 'session:example',
  sessionName: 'example',
  tabId: 'target:target-1',
  targetId: 'target-1',
  profileOrigin: 'agent_browser_owned',
  leaseHeartbeatExpected: true,
  traceFilter: {
    browserId: 'session:example',
    profileId: 'example-profile',
    sessionId: 'example',
  },
  valid: true,
};

await testDryRunPlan();
await testGenericBridgeSequence();

console.log('Service client broker bridge no-launch smoke passed');

async function testDryRunPlan() {
  const result = await runBrokerBridgeWorkflow({ dryRun: true });

  assert.equal(result.dryRun, true);
  assert.equal(result.plan.serviceName, 'ExampleBridge');
  assert.deepEqual(result.plan.bridgeSequence, [
    'read the no-launch access plan',
    'request a service-owned tab from the access plan',
    'extract the lease-backed service tab handle',
    'attach through the policy-gated CDP descriptor',
    'run bounded evaluate against the handle',
    'collect compact diagnostics for the handle',
    'detach without closing the browser process',
  ]);
}

async function testGenericBridgeSequence() {
  const calls = [];
  const result = await runBrokerBridgeWorkflow({
    baseUrl: 'http://127.0.0.1:4849',
    fetch: createBridgeFetch(calls),
    url: 'https://example.com/',
    includeDiagnosticsScreenshot: true,
  });

  assert.equal(result.dryRun, false);
  assert.deepEqual(
    calls.map((call) => `${call.method} ${call.path}`),
    [
      'GET /api/service/access-plan',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
    ],
  );
  assert.equal(calls[1].body.action, 'tab_new');
  assert.equal(calls[2].body.action, 'cdp_attach');
  assert.equal(calls[3].body.action, 'evaluate');
  assert.equal(calls[3].body.maxReturnBytes, 256);
  assert.equal(calls[4].body.action, 'diagnostics');
  assert.equal(calls[4].body.includeScreenshot, true);
  assert.equal(calls[5].body.action, 'cdp_detach');
  assert.deepEqual(result.serviceTabHandle, serviceTabHandle);
}

function createBridgeFetch(calls) {
  return async (url, init = {}) => {
    const parsed = new URL(String(url));
    const method = String(init.method || 'GET').toUpperCase();
    const body = init.body ? JSON.parse(String(init.body)) : null;
    calls.push({ method, path: parsed.pathname, search: parsed.search, body });

    if (method === 'GET' && parsed.pathname === '/api/service/access-plan') {
      return jsonResponse({
        success: true,
        data: {
          query: {
            serviceName: 'ExampleBridge',
            loginId: 'example',
            targetServiceId: 'example',
          },
          decision: {
            cdpAttachmentAllowed: true,
            serviceRequest: {
              request: {
                serviceName: 'ExampleBridge',
                agentName: 'example-bridge-agent',
                taskName: 'brokerFirstBridge',
                loginId: 'example',
                targetServiceId: 'example',
                action: 'tab_new',
                params: {
                  url: 'https://example.com/',
                },
              },
            },
          },
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/request') {
      return jsonResponse(responseForAction(body?.action));
    }

    return jsonResponse({ error: 'unexpected request' }, 404);
  };
}

function responseForAction(action) {
  if (action === 'tab_new') {
    return {
      success: true,
      data: {
        serviceTabHandle,
      },
    };
  }
  if (action === 'cdp_attach') {
    return {
      success: true,
      data: {
        attached: true,
        browserProcessPreserved: true,
      },
    };
  }
  if (action === 'evaluate') {
    return {
      success: true,
      data: {
        ok: true,
        result: 'Example Domain',
        resultTruncated: false,
        resultBytes: 16,
      },
    };
  }
  if (action === 'diagnostics') {
    return {
      success: true,
      data: {
        ok: true,
        compact: true,
        screenshot: {
          captured: false,
          reason: 'not_live',
        },
      },
    };
  }
  if (action === 'cdp_detach') {
    return {
      success: true,
      data: {
        detached: true,
        browserProcessPreserved: true,
      },
    };
  }
  return {
    success: false,
    error: `unexpected action ${action}`,
  };
}

function jsonResponse(payload, status = 200) {
  return {
    ok: status >= 200 && status < 300,
    status,
    async json() {
      return payload;
    },
  };
}
