#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  assertServiceStatusDidNotLaunch,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  parseMcpToolPayload,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-browser-row-remedies-',
  sessionPrefix: 'browser-row-remedies',
});

const { agentHome, session } = context;
const serviceName = 'BrowserRowRemedySmoke';
const agentName = 'smoke-agent';
const httpRepairBrowserId = 'browser-row-remedy-http-degraded';
const mcpRepairBrowserId = 'browser-row-remedy-mcp-faulted';
const httpRejectedCloseBrowserId = 'session:not-active-http-remedy-smoke';
const mcpRejectedCloseBrowserId = 'session:not-active-mcp-remedy-smoke';
const statePath = join(agentHome, 'service', 'state.json');

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service browser row remedies no-launch smoke to complete');
}, 120000);

function readState() {
  return JSON.parse(readFileSync(statePath, 'utf8'));
}

function writeState(state) {
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
}

function seedBrowser(browserId, health, lastError) {
  const state = readState();
  state.browsers = state.browsers ?? {};
  state.events = state.events ?? [];
  state.jobs = state.jobs ?? {};
  state.browsers[browserId] = {
    id: browserId,
    pid: null,
    cdpPort: null,
    cdpUrl: null,
    profileId: 'browser-row-remedy-profile',
    runtimeProfile: null,
    sessionId: session,
    activeSessionIds: [session],
    tabIds: [],
    health,
    lastError,
    lastHealthObservation: {
      observedAt: '2026-05-20T10:00:00Z',
      health,
      reason: lastError,
      failureClass: health === 'faulted' ? 'force_kill_failed' : 'polite_close_failed',
    },
    launchedAt: null,
    updatedAt: '2026-05-20T10:00:00Z',
  };
  writeState(state);
}

async function ensureStreamPort() {
  const streamStatusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function assertContractsExposeRowRemedies(contracts, label) {
  const actions = contracts.data?.contracts?.serviceRequest?.actions ?? [];
  assert(actions.includes('service_browser_close'), `${label} missing service_browser_close: ${JSON.stringify(actions)}`);
  assert(actions.includes('service_browser_repair'), `${label} missing service_browser_repair: ${JSON.stringify(actions)}`);
}

async function assertNoBrowserLaunch(label) {
  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, `${label} service status`);
  assert(status.success === true, `${label} service status failed: ${statusResult.stdout}${statusResult.stderr}`);
  assertServiceStatusDidNotLaunch(status, label);

  const state = readState();
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => !['launch', 'navigate', 'tab_new', 'cdp_free_launch'].includes(job.action)),
    `${label} recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  for (const browser of Object.values(state.browsers ?? {})) {
    assert(browser.pid === null || browser.pid === undefined, `${label} browser has pid: ${JSON.stringify(browser)}`);
    assert(
      browser.cdpPort === null || browser.cdpPort === undefined,
      `${label} browser has cdpPort: ${JSON.stringify(browser)}`,
    );
    assert(
      browser.cdpUrl === null || browser.cdpUrl === undefined,
      `${label} browser has cdpUrl: ${JSON.stringify(browser)}`,
    );
  }
}

function assertRepairEvent(browserId, actor, label) {
  const state = readState();
  const event = (state.events ?? []).find(
    (item) =>
      item.kind === 'browser_recovery_override' &&
      item.browserId === browserId &&
      item.details?.actor === actor,
  );
  assert(event, `${label} missing browser_recovery_override event: ${JSON.stringify(state.events)}`);
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
}

function send(method, params) {
  return mcp.send(method, params);
}

function notify(method, params) {
  mcp.notify(method, params);
}

try {
  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'initial service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);
  assertServiceStatusDidNotLaunch(status, 'initial browser row remedy smoke');

  const port = await ensureStreamPort();
  const contracts = await httpJson(port, 'GET', '/api/service/contracts');
  assert(contracts.success === true, `HTTP service contracts failed: ${JSON.stringify(contracts)}`);
  assertContractsExposeRowRemedies(contracts, 'HTTP contracts');

  seedBrowser(
    httpRepairBrowserId,
    'degraded',
    'Polite browser close failed; force kill was required',
  );
  seedBrowser(
    mcpRepairBrowserId,
    'faulted',
    'Force kill failed; host OS may be degraded',
  );

  const httpRepair = await httpJson(port, 'POST', '/api/service/request', {
    action: 'service_browser_repair',
    serviceName,
    agentName,
    taskName: 'httpBrowserRepairNoLaunch',
    params: {
      browserId: httpRepairBrowserId,
      by: 'http-row-remedy-smoke',
      note: 'HTTP no-launch row repair smoke',
    },
    jobTimeoutMs: 10000,
  });
  assert(httpRepair.success === true, `HTTP service_browser_repair failed: ${JSON.stringify(httpRepair)}`);
  assert(httpRepair.data?.repaired === true, `HTTP repair did not report repaired: ${JSON.stringify(httpRepair)}`);
  assert(
    httpRepair.data?.browser?.health === 'process_exited',
    `HTTP repair did not reset browser health: ${JSON.stringify(httpRepair)}`,
  );
  assertRepairEvent(httpRepairBrowserId, 'http-row-remedy-smoke', 'HTTP repair');

  const httpClose = await httpJson(port, 'POST', '/api/service/request', {
    action: 'service_browser_close',
    serviceName,
    agentName,
    taskName: 'httpBrowserCloseRejectNoLaunch',
    params: {
      browserId: httpRejectedCloseBrowserId,
    },
    jobTimeoutMs: 10000,
  });
  assert(httpClose.success === false, `HTTP service_browser_close unexpectedly succeeded: ${JSON.stringify(httpClose)}`);
  assert(
    typeof httpClose.error === 'string' && httpClose.error.includes('can only close the active service browser'),
    `HTTP service_browser_close returned wrong error: ${JSON.stringify(httpClose)}`,
  );

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-browser-row-remedies-no-launch', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const mcpRepairResult = await send('tools/call', {
    name: 'service_request',
    arguments: {
      action: 'service_browser_repair',
      serviceName,
      agentName,
      taskName: 'mcpBrowserRepairNoLaunch',
      params: {
        browserId: mcpRepairBrowserId,
        by: 'mcp-row-remedy-smoke',
        note: 'MCP no-launch row repair smoke',
      },
      jobTimeoutMs: 10000,
    },
  });
  const mcpRepair = parseMcpToolPayload(mcpRepairResult, 'MCP service_browser_repair');
  assert(mcpRepair.success === true, `MCP service_browser_repair failed: ${JSON.stringify(mcpRepair)}`);
  assert(mcpRepair.data?.repaired === true, `MCP repair did not report repaired: ${JSON.stringify(mcpRepair)}`);
  assert(
    mcpRepair.data?.browser?.health === 'process_exited',
    `MCP repair did not reset browser health: ${JSON.stringify(mcpRepair)}`,
  );
  assertRepairEvent(mcpRepairBrowserId, 'mcp-row-remedy-smoke', 'MCP repair');

  const mcpCloseResult = await send('tools/call', {
    name: 'service_request',
    arguments: {
      action: 'service_browser_close',
      serviceName,
      agentName,
      taskName: 'mcpBrowserCloseRejectNoLaunch',
      params: {
        browserId: mcpRejectedCloseBrowserId,
      },
      jobTimeoutMs: 10000,
    },
  });
  const mcpClose = parseMcpToolPayload(mcpCloseResult, 'MCP service_browser_close');
  assert(mcpClose.success === false, `MCP service_browser_close unexpectedly succeeded: ${JSON.stringify(mcpClose)}`);
  assert(
    typeof mcpClose.error === 'string' && mcpClose.error.includes('can only close the active service browser'),
    `MCP service_browser_close returned wrong error: ${JSON.stringify(mcpClose)}`,
  );

  await assertNoBrowserLaunch('browser row remedy no-launch smoke');
  await cleanup();
  console.log('Service browser row remedies no-launch smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
