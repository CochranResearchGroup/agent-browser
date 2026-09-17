#!/usr/bin/env node

import { request } from 'node:http';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { ensureStreamPort } from './smoke-remote-headed-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-tab-handle-refresh-',
  sessionPrefix: 'service-tab-handle-refresh',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'ServiceTabHandleRefreshSmoke';
const agentName = 'smoke-agent';
const taskName = 'plan0034TabHandleRefresh';
const profileId = 'tab-handle-refresh-profile';
const targetServiceId = 'generic-tab-handle-site';
const primaryHtml = '<!doctype html><title>Plan 0034 Refresh Primary</title><main>primary</main>';
const fallbackHtml = '<!doctype html><title>Plan 0034 Refresh Fallback</title><main>fallback</main>';
const primaryUrl = `data:text/html;charset=utf-8,${encodeURIComponent(primaryHtml)}`;
const fallbackUrl = `data:text/html;charset=utf-8,${encodeURIComponent(fallbackHtml)}`;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service tab handle refresh live smoke to complete');
}, 180000);

let streamPort;

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

function httpJsonWithTimeout(port, method, path, body, timeoutMs) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          try {
            const parsed = JSON.parse(text);
            if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
              reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
              return;
            }
            resolve(parsed);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
          }
        });
      },
    );
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out after ${timeoutMs}ms`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

async function serviceRequest(body, label) {
  let response;
  try {
    response = await httpJsonWithTimeout(streamPort, 'POST', '/api/service/request', {
      serviceName,
      agentName,
      taskName,
      targetServiceId,
      runtimeProfile: profileId,
      jobTimeoutMs: 60000,
      ...body,
    }, 90000);
  } catch (err) {
    throw new Error(`${label} failed: ${err.message}`);
  }
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

async function serviceTrace() {
  const result = await runCli(
    context,
    [
      '--json',
      '--session',
      session,
      'service',
      'trace',
      '--service-name',
      serviceName,
      '--agent-name',
      agentName,
      '--task-name',
      taskName,
      '--limit',
      '100',
    ],
    60000,
  );
  const trace = parseJsonOutput(result.stdout, 'service trace');
  assert(trace.success === true, `service trace failed: ${result.stdout}${result.stderr}`);
  return trace.data;
}

async function registerStockChromeCapability() {
  const checkedAt = new Date().toISOString();
  const records = [
    ['browserHosts', 'local-linux', {
      id: 'local-linux',
      name: 'Disposable local Linux host',
      hostKind: 'local',
      operatingSystem: 'linux',
      displaySupport: 'x11',
      remoteViewSupport: false,
      reachable: true,
      lifecycleOwner: 'agent_browser',
      health: 'ready',
      lastCheckedAt: checkedAt,
      tags: ['disposable-smoke'],
    }],
    ['browserExecutables', 'local-google-chrome', {
      id: 'local-google-chrome',
      hostId: 'local-linux',
      browserFamily: 'chrome',
      vendor: 'google',
      channel: 'stable',
      buildLabel: 'stock_chrome',
      executablePath: '/usr/bin/google-chrome',
      source: 'system',
      manifestPath: null,
      version: null,
      patchsetId: null,
      fresh: true,
      lastCheckedAt: checkedAt,
      tags: ['disposable-smoke'],
    }],
    ['browserCapabilities', 'local-google-chrome-capability', {
      id: 'local-google-chrome-capability',
      hostId: 'local-linux',
      executableId: 'local-google-chrome',
      cdpSupported: true,
      cdpFreeLaunchSupported: true,
      extensionsSupported: true,
      passkeysSupported: false,
      headedSupported: true,
      headlessSupported: true,
      streamingSupported: true,
      profileLockBehavior: 'exclusive_user_data_dir',
      keyringBehavior: 'basic_password_store',
      knownLimits: ['Disposable unauthenticated smoke only'],
    }],
    ['profileCompatibility', 'refresh-profile-local-google-chrome', {
      id: 'refresh-profile-local-google-chrome',
      profileId,
      hostId: 'local-linux',
      executableId: 'local-google-chrome',
      compatible: true,
      requiresOperatorOverride: false,
      reason: 'same_browser_family',
      notes: 'Disposable profile created for the tab-handle refresh smoke.',
    }],
    ['browserPreferenceBindings', 'refresh-smoke-stock-chrome', {
      id: 'refresh-smoke-stock-chrome',
      scope: 'service',
      targetServiceIds: [targetServiceId],
      accountIds: [],
      serviceNames: [serviceName],
      taskNames: [taskName],
      preferredHostId: 'local-linux',
      preferredExecutableId: 'local-google-chrome',
      preferredCapabilityId: 'local-google-chrome-capability',
      browserBuild: 'stock_chrome',
      priority: 100,
      reason: 'Disposable real-browser acceptance binding.',
    }],
    ['validationEvidence', 'refresh-smoke-stock-chrome-launch', {
      id: 'refresh-smoke-stock-chrome-launch',
      hostId: 'local-linux',
      executableId: 'local-google-chrome',
      capabilityId: 'local-google-chrome-capability',
      kind: 'launch',
      state: 'passed',
      checkedAt,
      evidence: 'Local executable presence checked before disposable smoke launch.',
      artifactPath: null,
    }],
  ];
  for (const [collection, id, record] of records) {
    const result = await httpJsonWithTimeout(
      streamPort,
      'POST',
      `/api/service/browser-capability-registry/${collection}/${id}`,
      record,
      60000,
    );
    assert(result.success === true, `${collection}/${id} upsert failed: ${JSON.stringify(result)}`);
  }
}

try {
  streamPort = await ensureStreamPort(context, 120000);

  const profileUpsert = await httpJsonWithTimeout(
    streamPort,
    'POST',
    `/api/service/profiles/${encodeURIComponent(profileId)}`,
    {
      name: 'Tab Handle Refresh Profile',
      allocation: 'per_service',
      keyring: 'basic_password_store',
      persistent: true,
      targetServiceIds: [targetServiceId],
      sharedServiceIds: [serviceName],
    },
    60000,
  );
  assert(profileUpsert.success === true, `profile upsert failed: ${JSON.stringify(profileUpsert)}`);
  assert(profileUpsert.data?.profile?.id === profileId, `profile id mismatch: ${JSON.stringify(profileUpsert)}`);
  await registerStockChromeCapability();

  const primaryTab = await serviceRequest(
    {
      action: 'tab_new',
      params: {
        headless: true,
        url: primaryUrl,
        waitUntil: 'load',
      },
    },
    'primary tab_new',
  );
  const handle = primaryTab.data?.serviceTabHandle;
  assert(handle?.valid === true, `primary tab_new did not return a valid handle: ${JSON.stringify(primaryTab)}`);
  const browserId = handle?.browserId;
  assert(typeof browserId === 'string' && browserId, `primary handle missing browserId: ${JSON.stringify(handle)}`);
  assert(typeof handle?.targetId === 'string' && handle.targetId, `primary handle missing targetId: ${JSON.stringify(handle)}`);

  const validRefresh = await serviceRequest(
    {
      action: 'tab_handle_refresh',
      serviceTabHandle: handle,
      repairPolicy: 'reject_only',
      desiredUrl: primaryUrl,
    },
    'valid tab_handle_refresh',
  );
  assert(validRefresh.data?.ok === true, `valid refresh was not ok: ${JSON.stringify(validRefresh)}`);
  assert(validRefresh.data?.decision === 'exact_handle_still_valid', `valid refresh decision mismatch: ${JSON.stringify(validRefresh.data)}`);
  assert(validRefresh.data?.serviceTabHandle?.targetId === handle.targetId, `valid refresh changed target: ${JSON.stringify(validRefresh.data)}`);
  assert(validRefresh.data?.duplicateCleanupAttempted === false, `valid refresh attempted duplicate cleanup: ${JSON.stringify(validRefresh.data)}`);
  assert(validRefresh.data?.peerCleanupAttempted === false, `valid refresh attempted peer cleanup: ${JSON.stringify(validRefresh.data)}`);

  const fallbackTab = await serviceRequest(
    {
      action: 'tab_new',
      params: {
        url: fallbackUrl,
        waitUntil: 'load',
      },
    },
    'fallback tab_new',
  );
  const fallbackHandle = fallbackTab.data?.serviceTabHandle;
  assert(fallbackHandle?.valid === true, `fallback tab_new did not return a valid handle: ${JSON.stringify(fallbackTab)}`);
  assert(fallbackHandle?.targetId !== handle.targetId, `fallback reused primary target unexpectedly: ${JSON.stringify(fallbackHandle)}`);

  const switchToPrimary = await serviceRequest(
    {
      action: 'tab_handle_refresh',
      serviceTabHandle: handle,
      repairPolicy: 'reuse_compatible',
      desiredUrl: primaryUrl,
    },
    'switch primary tab_handle_refresh',
  );
  assert(switchToPrimary.data?.ok === true, `switch refresh was not ok: ${JSON.stringify(switchToPrimary)}`);
  assert(switchToPrimary.data?.serviceTabHandle?.targetId === handle.targetId, `switch refresh did not select original target: ${JSON.stringify(switchToPrimary.data)}`);

  const closePrimary = await serviceRequest(
    {
      action: 'tab_close',
      serviceTabHandle: handle,
    },
    'primary tab_close',
  );
  assert(closePrimary.data, `tab_close returned no data: ${JSON.stringify(closePrimary)}`);

  const staleHandle = { ...handle, valid: false, staleReason: 'tab_closed' };
  const rejectRefresh = await serviceRequest(
    {
      action: 'tab_handle_refresh',
      serviceTabHandle: staleHandle,
      repairPolicy: 'reject_only',
      desiredUrl: primaryUrl,
    },
    'reject stale tab_handle_refresh',
  );
  assert(rejectRefresh.data?.ok === false, `reject refresh unexpectedly succeeded: ${JSON.stringify(rejectRefresh)}`);
  assert(rejectRefresh.data?.decision === 'rejected_stale_or_missing_target', `reject refresh decision mismatch: ${JSON.stringify(rejectRefresh.data)}`);
  assert(Array.isArray(rejectRefresh.data?.candidates), `reject refresh missing candidates: ${JSON.stringify(rejectRefresh.data)}`);
  assert(rejectRefresh.data?.duplicateCleanupAttempted === false, `reject refresh attempted duplicate cleanup: ${JSON.stringify(rejectRefresh.data)}`);
  assert(rejectRefresh.data?.peerCleanupAttempted === false, `reject refresh attempted peer cleanup: ${JSON.stringify(rejectRefresh.data)}`);

  const openRefresh = await serviceRequest(
    {
      action: 'tab_handle_refresh',
      serviceTabHandle: staleHandle,
      repairPolicy: 'open_if_missing',
      desiredUrl: primaryUrl,
    },
    'open stale tab_handle_refresh',
  );
  assert(openRefresh.data?.ok === true, `open refresh was not ok: ${JSON.stringify(openRefresh)}`);
  assert(openRefresh.data?.decision === 'opened_replacement_target', `open refresh decision mismatch: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.valid === true, `open refresh did not return valid handle: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.targetId !== handle.targetId, `open refresh reused stale target: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.browserId === browserId, `open refresh browser mismatch: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.traceFilter?.serviceName === serviceName, `open refresh lost service attribution: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.traceFilter?.agentName === agentName, `open refresh lost agent attribution: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.traceFilter?.taskName === taskName, `open refresh lost task attribution: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.duplicateCleanupAttempted === false, `open refresh attempted duplicate cleanup: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.peerCleanupAttempted === false, `open refresh attempted peer cleanup: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.duplicateTargetCleanup?.attempted === false, `open refresh nested cleanup evidence mismatch: ${JSON.stringify(openRefresh.data)}`);

  const preservedFallback = await serviceRequest(
    {
      action: 'tab_handle_refresh',
      serviceTabHandle: fallbackHandle,
      repairPolicy: 'reject_only',
      desiredUrl: fallbackUrl,
    },
    'preserved fallback tab_handle_refresh',
  );
  assert(preservedFallback.data?.ok === true, `fallback peer was not preserved: ${JSON.stringify(preservedFallback)}`);
  assert(preservedFallback.data?.serviceTabHandle?.targetId === fallbackHandle.targetId, `fallback peer target changed: ${JSON.stringify(preservedFallback.data)}`);

  const trace = await serviceTrace();
  const refreshJobs = (trace?.jobs ?? []).filter((job) => job.action === 'tab_handle_refresh');
  assert(refreshJobs.length >= 4, `service trace missing tab_handle_refresh jobs: ${JSON.stringify(trace?.jobs ?? [])}`);
  const refreshEvents = (trace?.events ?? []).filter((event) => event.details?.action === 'tab_handle_refresh');
  const decisions = new Set(refreshEvents.map((event) => event.details?.decision));
  assert(decisions.has('exact_handle_still_valid'), `trace missing exact refresh event: ${JSON.stringify(refreshEvents)}`);
  assert(decisions.has('rejected_stale_or_missing_target'), `trace missing stale reject event: ${JSON.stringify(refreshEvents)}`);
  assert(
    decisions.has('opened_replacement_target') || decisions.has('reused_compatible_target'),
    `trace missing repair event: ${JSON.stringify(refreshEvents)}`,
  );
  assert(
    refreshEvents.some((event) => Number(event.details?.candidateCount ?? 0) >= 1),
    `trace refresh events did not expose candidate counts: ${JSON.stringify(refreshEvents)}`,
  );

  await cleanup();
  console.log(`Service tab handle refresh live smoke passed (${browserId}, stream ${streamPort})`);
} catch (err) {
  await fail(err.stack || err.message);
}
