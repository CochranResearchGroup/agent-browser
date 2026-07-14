#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  closeRemoteHeadedBrowser,
  configureRemoteHeadedContext,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacColdRestartReadbackSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacColdRestartLaunch';
const checkoutTaskName = 'rdpGuacColdRestartCheckout';
const closeTaskName = 'rdpGuacColdRestartClose';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-cold-restart-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function runReadinessReport() {
  const result = spawnSync(process.execPath, ['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], {
    cwd: new URL('..', import.meta.url).pathname,
    env: process.env,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  assert(
    result.status === 0,
    `route-pool readiness report failed: ${result.stdout}${result.stderr}`,
  );
  const report = parseJsonOutput(result.stdout, 'route-pool readiness report');
  writeArtifact('route-pool-readiness-report.json', report);
  assert(report.success === true, `route-pool readiness is not green: ${JSON.stringify(report)}`);
  assert(
    report.guacamole?.schema?.ok === true,
    `Guacamole schema is not ready: ${JSON.stringify(report.guacamole?.schema)}`,
  );
  assert(
    report.guacamole?.permissions?.ok === true,
    `Guacamole route permissions are not ready: ${JSON.stringify(report.guacamole?.permissions)}`,
  );
  assert(
    Array.isArray(report.routePoolJson) && report.routePoolJson.length > 0,
    `route-pool readiness did not return routePoolJson: ${JSON.stringify(report)}`,
  );
  return report;
}

async function ensureStreamPortForSession(context, sessionName, timeoutMs = 60000) {
  const statusResult = await runCli(
    context,
    ['--json', '--session', sessionName, 'stream', 'status'],
    timeoutMs,
  );
  let stream = parseJsonOutput(statusResult.stdout, `${sessionName} stream status`);
  assert(
    stream.success === true,
    `${sessionName} stream status failed: ${statusResult.stdout}${statusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const enableResult = await runCli(
      context,
      ['--json', '--session', sessionName, 'stream', 'enable'],
      timeoutMs,
    );
    stream = parseJsonOutput(enableResult.stdout, `${sessionName} stream enable`);
    assert(
      stream.success === true,
      `${sessionName} stream enable failed: ${enableResult.stdout}${enableResult.stderr}`,
    );
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `No stream port for ${sessionName}`);
  return port;
}

async function restartStream(context, sessionName) {
  const disabledResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'disable'], 60000);
  const disabled = parseJsonOutput(disabledResult.stdout, 'stream disable');
  writeArtifact('stream-disable-response.json', disabled);
  assert(disabled.success === true, `stream disable failed: ${disabledResult.stdout}${disabledResult.stderr}`);

  const enabledResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'enable'], 90000);
  const enabled = parseJsonOutput(enabledResult.stdout, 'stream enable after restart');
  writeArtifact('stream-enable-after-restart-response.json', enabled);
  assert(enabled.success === true, `stream enable after restart failed: ${enabledResult.stdout}${enabledResult.stderr}`);
  const port = enabled.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream restart did not return a port: ${JSON.stringify(enabled)}`);
  return port;
}

async function serviceStatus(streamPort, label) {
  const status = await httpJson(streamPort, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

async function serviceRequest(streamPort, body, label) {
  const response = await httpJson(streamPort, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

async function reconcile(streamPort, label) {
  const response = await httpJson(streamPort, 'POST', '/api/service/reconcile');
  writeArtifact(`${label}-reconcile-response.json`, response);
  assert(response.success === true, `${label} reconcile failed: ${JSON.stringify(response)}`);
  assert(response.data?.reconciled === true, `${label} reconcile did not report reconciled=true`);
  return response;
}

async function launchRemoteBrowser(context, remoteConfig, streamPort) {
  const response = await serviceRequest(
    streamPort,
    {
      action: 'navigate',
      serviceName,
      agentName,
      taskName: launchTaskName,
      params: {
        browserHost: 'remote_headed',
        displayIsolation: 'private_virtual_display',
        headless: false,
        runtimeProfile: `${context.session}-profile`,
        url: smokeDataUrl('RDP Guac Cold Restart', 'RDP Guac Cold Restart'),
        waitUntil: 'load',
        viewStreamProvider: remoteConfig.viewStreamProvider,
        controlInputProvider: remoteConfig.controlInputProvider,
        viewStreamUrl: remoteConfig.viewStreamUrl,
        frameUrl: remoteConfig.frameUrl,
        externalUrl: remoteConfig.externalUrl,
        routeId: remoteConfig.routeId || `route:${context.session}`,
        connectionId: remoteConfig.connectionId,
        connectionName: remoteConfig.connectionName,
      },
      jobTimeoutMs: 120000,
    },
    'launch',
  );
  return {
    browserId: `session:${context.session}`,
    launchResponse: response,
    sessionName: context.session,
  };
}

function seedRoutePoolEntry(context, {
  browser,
  displayAllocationId,
  routeEntry,
}) {
  const statePath = join(context.agentHome, 'service', 'state.json');
  assert(existsSync(statePath), `service state file does not exist: ${statePath}`);
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.routePool = state.routePool || {};
  state.routePool[routeEntry.id] = {
    ...routeEntry,
    target: {
      ...(routeEntry.target || {}),
      displayAllocationId,
      browserId: browser.id,
      sessionId: browser.sessionId || browser.activeSessionIds?.[0] || null,
    },
    state: 'available',
    currentRouteAllocationId: null,
    readiness: {
      ...(routeEntry.readiness || {}),
      state: 'ready',
      source: 'rdp_guac_cold_restart_readback_live_smoke',
      updatedAt: new Date().toISOString(),
    },
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
  writeArtifact('seeded-route-pool-entry.json', state.routePool[routeEntry.id]);
}

function serviceState(status) {
  return status.data?.service_state || status.data || {};
}

function assertReadbackAgreement(status, {
  browserId,
  displayAllocationId,
  readinessReport,
  routeEntry,
}) {
  const state = serviceState(status);
  const browser = state.browsers?.[browserId];
  const allocation = state.displayAllocations?.[displayAllocationId];
  const route = state.remoteViewRoutes?.[routeEntry.routeId];
  const poolEntry = state.routePool?.[routeEntry.id];
  const guacConnection = readinessReport.guacamole?.connections?.find(
    (connection) => String(connection.connectionId) === String(routeEntry.connectionId),
  );

  assert(browser, `missing browser ${browserId}: ${JSON.stringify(state.browsers)}`);
  assert(allocation, `missing display allocation ${displayAllocationId}: ${JSON.stringify(state.displayAllocations)}`);
  assert(route, `missing route ${routeEntry.routeId}: ${JSON.stringify(state.remoteViewRoutes)}`);
  assert(poolEntry, `missing route-pool entry ${routeEntry.id}: ${JSON.stringify(state.routePool)}`);
  assert(guacConnection, `missing Guacamole connection ${routeEntry.connectionId}: ${JSON.stringify(readinessReport.guacamole?.connections)}`);

  assert(browser.health === 'ready', `browser is not ready after restart: ${JSON.stringify(browser)}`);
  assert(browser.displayAllocationId === displayAllocationId, `browser/display allocation mismatch: ${JSON.stringify(browser)}`);
  assert(allocation.state === 'ready', `display allocation is not ready: ${JSON.stringify(allocation)}`);
  assert(allocation.ownerBrowserId === browserId, `display allocation owner mismatch: ${JSON.stringify(allocation)}`);
  assert(route.state === 'ready', `route is not ready: ${JSON.stringify(route)}`);
  assert(route.displayAllocationId === displayAllocationId, `route/display mismatch: ${JSON.stringify(route)}`);
  assert(route.browserId === browserId, `route/browser mismatch: ${JSON.stringify(route)}`);
  assert(route.connectionId === String(routeEntry.connectionId), `route/Guacamole connection mismatch: ${JSON.stringify(route)}`);
  assert(poolEntry.state === 'checked_out', `route-pool entry is not checked out: ${JSON.stringify(poolEntry)}`);
  assert(poolEntry.currentRouteAllocationId === routeEntry.routeId, `route-pool allocation mismatch: ${JSON.stringify(poolEntry)}`);
  assert(poolEntry.connectionId === String(routeEntry.connectionId), `route-pool/Guacamole connection mismatch: ${JSON.stringify(poolEntry)}`);
}

const timeout = setTimeout(() => {
  console.error('Timed out waiting for cold restart readback live smoke to complete');
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, 420000);

const readinessReport = runReadinessReport();
const routeEntry = readinessReport.routePoolJson[0];
assert(routeEntry.connectionId, `selected route entry has no connectionId: ${JSON.stringify(routeEntry)}`);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-cold-restart-',
  sessionPrefix: 'rdp-guac-cold-restart',
});
context.env.AGENT_BROWSER_ENGINE = 'chrome';
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
if (!process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE && existsSync('/usr/bin/google-chrome-stable')) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = '/usr/bin/google-chrome-stable';
} else if (process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE;
}
delete context.env.AGENT_BROWSER_CDP;
delete context.env.AGENT_BROWSER_AUTO_CONNECT;

const remoteConfig = configureRemoteHeadedContext(context);
let streamPort = null;
let workspace = null;

async function cleanup() {
  clearTimeout(timeout);
  if (streamPort && workspace?.browserId) {
    await closeRemoteHeadedBrowser({
      agentName,
      browserId: workspace.browserId,
      context,
      serviceName,
      streamPort,
      taskName: closeTaskName,
    }).catch(() => {});
  }
  cleanupSmokeHome(context);
}

try {
  streamPort = await ensureStreamPortForSession(context, context.session, 180000);
  await serviceStatus(streamPort, 'before-launch');

  workspace = await launchRemoteBrowser(context, remoteConfig, streamPort);
  const afterLaunch = await serviceStatus(streamPort, 'after-launch');
  const browser = serviceState(afterLaunch).browsers?.[workspace.browserId];
  assert(browser?.health === 'ready', `browser not ready after launch: ${JSON.stringify(browser)}`);
  assert(browser.displayAllocationId, `browser did not record display allocation id: ${JSON.stringify(browser)}`);

  seedRoutePoolEntry(context, {
    browser,
    displayAllocationId: browser.displayAllocationId,
    routeEntry,
  });

  const checkout = await serviceRequest(
    streamPort,
    {
      action: 'service_remote_view_route_checkout',
      serviceName,
      agentName,
      taskName: checkoutTaskName,
      params: {
        displayAllocationId: browser.displayAllocationId,
        routePoolEntryId: routeEntry.id,
        routeId: routeEntry.routeId,
        browserId: workspace.browserId,
        sessionName: workspace.sessionName,
        provider: 'rdp_gateway',
      },
      jobTimeoutMs: 30000,
    },
    'route-checkout',
  );
  assert(checkout.data?.routePoolEntryId === routeEntry.id, `checkout used wrong pool entry: ${JSON.stringify(checkout)}`);
  const afterCheckout = await serviceStatus(streamPort, 'after-route-checkout');
  assertReadbackAgreement(afterCheckout, {
    browserId: workspace.browserId,
    displayAllocationId: browser.displayAllocationId,
    readinessReport,
    routeEntry,
  });

  streamPort = await restartStream(context, context.session);
  const afterRestart = await serviceStatus(streamPort, 'after-stream-restart');
  assertReadbackAgreement(afterRestart, {
    browserId: workspace.browserId,
    displayAllocationId: browser.displayAllocationId,
    readinessReport,
    routeEntry,
  });

  const afterReconcile = await reconcile(streamPort, 'after-cold-restart');
  assertReadbackAgreement(afterReconcile, {
    browserId: workspace.browserId,
    displayAllocationId: browser.displayAllocationId,
    readinessReport,
    routeEntry,
  });

  writeArtifact('cold-restart-readback-summary.json', {
    artifactDir,
    browserId: workspace.browserId,
    displayAllocationId: browser.displayAllocationId,
    routePoolEntryId: routeEntry.id,
    routeId: routeEntry.routeId,
    guacamoleConnectionId: routeEntry.connectionId,
  });

  await cleanup();
  console.log(`RDP Guacamole cold-restart readback live smoke passed; artifacts: ${artifactDir}`);
} catch (err) {
  console.error(err.stack || err.message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}
