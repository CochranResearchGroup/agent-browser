#!/usr/bin/env node

import assert from 'node:assert/strict';

import {
  buildComposedWorkflowPlan,
  runComposedWorkflow,
} from '../examples/service-client/composed-workflow.mjs';

const serviceTabHandle = {
  browserId: 'session:composed',
  sessionName: 'composed',
  tabId: 'target:composed-1',
  targetId: 'composed-1',
  profileOrigin: 'agent_browser_owned',
  leaseHeartbeatExpected: true,
  traceFilter: {
    browserId: 'session:composed',
    profileId: 'composed-profile',
    sessionId: 'composed',
  },
  valid: true,
};

await testDryRunPlan();
await testGenericComposedSequence();
await testFailureDetaches();

console.log('Service client composed workflow no-launch smoke passed');

async function testDryRunPlan() {
  const result = await runComposedWorkflow({ dryRun: true });
  const plan = buildComposedWorkflowPlan();

  assert.equal(result.dryRun, true);
  assert.equal(result.plan.serviceName, 'ExampleComposedWorkflow');
  assert.deepEqual(result.plan.sequence, plan.sequence);
  assert.deepEqual(result.plan.sequence, [
    'read no-launch access plan',
    'request service-owned tab from access plan',
    'attach through policy-gated descriptor',
    'run provider-neutral probe recipe',
    'run provider-neutral UI action recipe',
    'capture capped network evidence',
    'transfer service-owned upload and download files',
    'collect compact diagnostics',
    'detach without closing the browser process',
  ]);
}

async function testGenericComposedSequence() {
  const calls = [];
  const result = await runComposedWorkflow({
    baseUrl: 'http://127.0.0.1:4850',
    fetch: createComposedFetch(calls),
    url: 'https://example.test/workflow',
    accountId: 'composed-account@example.test',
    tabParams: {
      headless: true,
      waitUntil: 'load',
    },
    timeoutMs: 5000,
    maxReturnBytes: 384,
    maxTextBytes: 192,
    maxBodyBytes: 96,
    uploadFile: '/tmp/upload/report.txt',
    uploadAllowedPath: '/tmp/upload',
    downloadDir: '/tmp/downloads',
    downloadAllowedDir: '/tmp',
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
      'POST /api/service/request',
      'POST /api/service/request',
      'POST /api/service/request',
    ],
  );
  assert.deepEqual(
    calls.slice(1).map((call) => call.body.action),
    [
      'tab_new',
      'cdp_attach',
      'probe',
      'ui_action',
      'network_capture',
      'file_transfer',
      'diagnostics',
      'cdp_detach',
    ],
  );

  const probe = calls[3].body;
  assert.equal(calls[1].body.params.headless, true);
  assert.equal(calls[1].body.params.waitUntil, 'load');
  assert.equal(probe.timeoutMs, 5000);
  assert.equal(probe.maxReturnBytes, 384);
  assert.equal(probe.probe.recipeId, 'generic-composed-probe');
  assert.deepEqual(
    probe.probe.detectors.map((detector) => `${detector.id}:${detector.type}`),
    ['page:url_title', 'account-text:selector_text', 'identity-object:evaluate'],
  );

  const uiAction = calls[4].body;
  assert.equal(uiAction.maxTextBytes, 192);
  assert.equal(uiAction.uiAction.recipeId, 'generic-composed-ui');
  assert.deepEqual(
    uiAction.uiAction.steps.map((step) => step.type),
    ['find', 'fill', 'click', 'wait'],
  );

  const networkCapture = calls[5].body;
  assert.equal(networkCapture.maxBodyBytes, 96);
  assert.equal(networkCapture.networkCapture.recipeId, 'generic-composed-network');
  assert.equal(networkCapture.networkCapture.maxEvents, 1);
  assert.equal(networkCapture.networkCapture.captureBodies, true);
  assert.deepEqual(networkCapture.networkCapture.urlPatterns, ['/api/data']);

  const fileTransfer = calls[6].body;
  assert.equal(fileTransfer.fileTransfer.recipeId, 'generic-composed-files');
  assert.deepEqual(fileTransfer.fileTransfer.upload.files, ['/tmp/upload/report.txt']);
  assert.deepEqual(fileTransfer.fileTransfer.upload.allowedPaths, ['/tmp/upload']);
  assert.equal(fileTransfer.fileTransfer.upload.maxFiles, 1);
  assert.equal(fileTransfer.fileTransfer.download.maxBytes, 1024);
  assert.deepEqual(fileTransfer.fileTransfer.download.allowedDirectories, ['/tmp']);

  const diagnostics = calls[7].body;
  assert.equal(diagnostics.includeScreenshot, true);
  assert.equal(diagnostics.maxConsoleEntries, 10);

  assert.deepEqual(result.serviceTabHandle, serviceTabHandle);
  assert.equal(result.probe.ok, true);
  assert.equal(result.uiAction.ok, true);
  assert.equal(result.networkCapture.ok, true);
  assert.equal(result.fileTransfer.ok, true);
  assert.equal(result.diagnostics.ok, true);
  assert.equal(result.detach.detached, true);
}

async function testFailureDetaches() {
  const calls = [];
  await assert.rejects(
    runComposedWorkflow({
      baseUrl: 'http://127.0.0.1:4850',
      fetch: createComposedFetch(calls, { failAction: 'network_capture' }),
    }),
    /network capture failed: planned network failure/,
  );
  assert.deepEqual(
    calls
      .filter((call) => call.path === '/api/service/request')
      .map((call) => call.body.action),
    ['tab_new', 'cdp_attach', 'probe', 'ui_action', 'network_capture', 'cdp_detach'],
  );
}

function createComposedFetch(calls, { failAction } = {}) {
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
            serviceName: 'ExampleComposedWorkflow',
            loginId: 'example',
            targetServiceId: 'example',
          },
          decision: {
            launchPosture: {
              cdpAttachmentAllowed: true,
            },
            serviceRequest: {
              cdpAttachmentAllowed: true,
              request: {
                serviceName: 'ExampleComposedWorkflow',
                agentName: 'example-composed-agent',
                taskName: 'brokerFirstComposedWorkflow',
                loginId: 'example',
                targetServiceId: 'example',
                action: 'tab_new',
                params: {
                  url: 'https://example.test/workflow',
                },
              },
            },
          },
        },
      });
    }

    if (method === 'POST' && parsed.pathname === '/api/service/request') {
      if (body?.action === failAction) {
        return jsonResponse({
          success: false,
          error: 'planned network failure',
        });
      }
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
  if (action === 'probe') {
    return {
      success: true,
      data: {
        ok: true,
        identity: {
          matched: true,
          actual: 'composed-account@example.test',
        },
      },
    };
  }
  if (action === 'ui_action') {
    return {
      success: true,
      data: {
        ok: true,
        steps: [{ id: 'wait-applied', ok: true }],
      },
    };
  }
  if (action === 'network_capture') {
    return {
      success: true,
      data: {
        ok: true,
        events: [{ url: 'https://example.test/api/data', status: 200 }],
      },
    };
  }
  if (action === 'file_transfer') {
    return {
      success: true,
      data: {
        ok: true,
        upload: {
          uploaded: 1,
          selectedFileNames: ['report.txt'],
        },
        download: {
          fileName: 'composed-download.txt',
          size: 19,
        },
      },
    };
  }
  if (action === 'diagnostics') {
    return {
      success: true,
      data: {
        ok: true,
        compact: true,
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
