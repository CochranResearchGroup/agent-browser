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
const targetServiceId = 'generic-tab-handle-site';
const browserId = `session:${session}`;
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

try {
  streamPort = await ensureStreamPort(context, 120000);

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
  assert(handle?.browserId === browserId, `primary handle browser mismatch: ${JSON.stringify(handle)}`);
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
  assert(
    ['opened_replacement_target', 'reused_compatible_target'].includes(openRefresh.data?.decision),
    `open refresh decision mismatch: ${JSON.stringify(openRefresh.data)}`,
  );
  assert(openRefresh.data?.serviceTabHandle?.valid === true, `open refresh did not return valid handle: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.targetId !== handle.targetId, `open refresh reused stale target: ${JSON.stringify(openRefresh.data)}`);
  assert(openRefresh.data?.serviceTabHandle?.browserId === browserId, `open refresh browser mismatch: ${JSON.stringify(openRefresh.data)}`);

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
