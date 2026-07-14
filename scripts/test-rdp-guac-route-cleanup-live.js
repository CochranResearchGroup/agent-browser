#!/usr/bin/env node

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

const serviceName = 'RdpGuacRouteCleanupSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacRouteCleanupLaunch';
const routeCheckoutTaskName = 'rdpGuacRouteCleanupCheckout';
const repairTaskName = 'rdpGuacRouteCleanupRepair';
const closeTaskName = 'rdpGuacRouteCleanupClose';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-route-cleanup-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
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

async function serviceStatus(streamPort, label) {
  const status = await httpJson(streamPort, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

function browserRecord(status, browserId) {
  const browser = status.data?.service_state?.browsers?.[browserId];
  assert(browser, `Missing browser ${browserId}: ${JSON.stringify(status.data)}`);
  return browser;
}

function routePoolEntry(status, entryId) {
  const entry = status.data?.service_state?.routePool?.[entryId];
  assert(entry, `Missing route-pool entry ${entryId}: ${JSON.stringify(status.data)}`);
  return entry;
}

function remoteViewRoute(status, routeId) {
  const route = status.data?.service_state?.remoteViewRoutes?.[routeId];
  assert(route, `Missing remote-view route ${routeId}: ${JSON.stringify(status.data)}`);
  return route;
}

async function serviceRequest(streamPort, body, label) {
  const response = await httpJson(streamPort, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
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
        url: smokeDataUrl('RDP Guac Route Cleanup', 'RDP Guac Route Cleanup'),
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
  connectionId,
  connectionName,
  displayAllocationId,
  externalUrl,
  frameUrl,
  poolEntryId,
  routeId,
}) {
  const statePath = join(context.agentHome, 'service', 'state.json');
  assert(existsSync(statePath), `service state file does not exist: ${statePath}`);
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.routePool = state.routePool || {};
  state.routePool[poolEntryId] = {
    id: poolEntryId,
    provider: 'rdp_gateway',
    routeId,
    connectionId: connectionId || null,
    connectionName: connectionName || null,
    frameUrl: frameUrl || null,
    externalUrl: externalUrl || frameUrl || null,
    target: {
      displayAllocationId,
      browserId: browser.id,
      sessionId: browser.sessionId || browser.activeSessionIds?.[0] || null,
    },
    providerMode: 'single_controller',
    state: 'available',
    currentRouteAllocationId: null,
    readiness: {
      state: 'ready',
      source: 'rdp_guac_route_cleanup_live_smoke',
      updatedAt: new Date().toISOString(),
    },
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
  writeArtifact('seeded-route-pool-entry.json', state.routePool[poolEntryId]);
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

async function browserPid(streamPort) {
  const response = await httpJson(streamPort, 'POST', '/api/command', {
    action: 'browser_pid',
    serviceName,
    agentName,
    taskName: 'rdpGuacRouteCleanupBrowserPid',
  });
  writeArtifact('browser-pid-response.json', response);
  assert(response.success === true, `browser_pid failed: ${JSON.stringify(response)}`);
  const pid = response.data?.pid;
  assert(Number.isInteger(pid) && pid > 0, `browser_pid did not return a pid: ${JSON.stringify(response)}`);
  return pid;
}

async function reconcile(streamPort, label) {
  const response = await httpJson(streamPort, 'POST', '/api/service/reconcile');
  writeArtifact(`${label}-reconcile-response.json`, response);
  assert(response.success === true, `${label} reconcile failed: ${JSON.stringify(response)}`);
  assert(response.data?.reconciled === true, `${label} reconcile did not report reconciled=true`);
  return response;
}

const timeout = setTimeout(() => {
  console.error('Timed out waiting for route cleanup live smoke to complete');
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, 420000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-route-cleanup-',
  sessionPrefix: 'rdp-guac-route-cleanup',
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
  const browser = browserRecord(afterLaunch, workspace.browserId);
  assert(browser.health === 'ready', `browser not ready after launch: ${JSON.stringify(browser)}`);
  assert(
    browser.displayAllocationId,
    `browser did not record display allocation id: ${JSON.stringify(browser)}`,
  );

  const poolEntryId = `pool-route-cleanup-${process.pid}`;
  const routeId = `route-cleanup-${process.pid}`;
  seedRoutePoolEntry(context, {
    browser,
    connectionId: remoteConfig.connectionId,
    connectionName: remoteConfig.connectionName,
    displayAllocationId: browser.displayAllocationId,
    externalUrl: remoteConfig.externalUrl,
    frameUrl: remoteConfig.frameUrl,
    poolEntryId,
    routeId,
  });

  const checkout = await serviceRequest(
    streamPort,
    {
      action: 'service_remote_view_route_checkout',
      serviceName,
      agentName,
      taskName: routeCheckoutTaskName,
      params: {
        displayAllocationId: browser.displayAllocationId,
        routePoolEntryId: poolEntryId,
        routeId,
        browserId: workspace.browserId,
        sessionName: workspace.sessionName,
        provider: 'rdp_gateway',
      },
      jobTimeoutMs: 30000,
    },
    'route-checkout',
  );
  assert(checkout.data?.routePoolEntryId === poolEntryId, `checkout used wrong pool entry: ${JSON.stringify(checkout)}`);
  const afterCheckout = await serviceStatus(streamPort, 'after-route-checkout');
  assert(routePoolEntry(afterCheckout, poolEntryId).state === 'checked_out', 'route pool did not check out');
  assert(remoteViewRoute(afterCheckout, routeId).state === 'ready', 'route did not become ready');

  streamPort = await restartStream(context, context.session);
  const afterRestart = await serviceStatus(streamPort, 'after-stream-restart');
  assert(
    browserRecord(afterRestart, workspace.browserId).health === 'ready',
    `browser did not survive stream restart: ${JSON.stringify(afterRestart.data?.service_state?.browsers?.[workspace.browserId])}`,
  );
  assert(
    routePoolEntry(afterRestart, poolEntryId).state === 'checked_out',
    `route pool checkout did not persist after restart: ${JSON.stringify(routePoolEntry(afterRestart, poolEntryId))}`,
  );
  await reconcile(streamPort, 'after-restart');
  const healthyRepair = await serviceRequest(
    streamPort,
    {
      action: 'service_route_pool_repair',
      serviceName,
      agentName,
      taskName: repairTaskName,
      params: {
        apply: false,
        staleCheckouts: true,
      },
      jobTimeoutMs: 30000,
    },
    'healthy-route-pool-repair-dry-run',
  );
  assert(
    healthyRepair.data?.candidateCounts?.staleCheckouts === 0,
    `healthy route was incorrectly repairable: ${JSON.stringify(healthyRepair)}`,
  );

  const pid = await browserPid(streamPort);
  process.kill(pid, 'SIGKILL');
  await new Promise((resolve) => setTimeout(resolve, 1000));

  const crashReconcile = await reconcile(streamPort, 'after-browser-crash');
  const crashedState = crashReconcile.data?.service_state || {};
  const crashedBrowser = crashedState.browsers?.[workspace.browserId];
  const orphanedRoute = crashedState.remoteViewRoutes?.[routeId];
  const checkedOutPool = crashedState.routePool?.[poolEntryId];
  writeArtifact('crash-cleanup-proof-before-repair.json', {
    browser: crashedBrowser,
    route: orphanedRoute,
    routePoolEntry: checkedOutPool,
    remoteViewSummary: crashReconcile.data?.summary?.remoteView || crashReconcile.data?.remoteView,
  });
  assert(
    crashedBrowser?.health !== 'ready',
    `crashed browser still reported ready: ${JSON.stringify(crashedBrowser)}`,
  );
  assert(orphanedRoute?.state === 'orphaned', `route was not orphaned: ${JSON.stringify(orphanedRoute)}`);
  assert(checkedOutPool?.state === 'checked_out', `pool entry was not still checked out before repair: ${JSON.stringify(checkedOutPool)}`);

  const staleRepair = await serviceRequest(
    streamPort,
    {
      action: 'service_route_pool_repair',
      serviceName,
      agentName,
      taskName: repairTaskName,
      params: {
        apply: false,
        staleCheckouts: true,
      },
      jobTimeoutMs: 30000,
    },
    'stale-route-pool-repair-dry-run',
  );
  assert(
    staleRepair.data?.candidateCounts?.staleCheckouts === 1,
    `stale route-pool checkout was not reported: ${JSON.stringify(staleRepair)}`,
  );
  assert(
    staleRepair.data?.candidateCounts?.staleRoutes === 1,
    `stale route record was not reported: ${JSON.stringify(staleRepair)}`,
  );
  assert(
    staleRepair.data?.candidateCounts?.staleDisplayAllocations === 1,
    `stale display allocation was not reported: ${JSON.stringify(staleRepair)}`,
  );
  assert(
    staleRepair.data?.candidates?.staleCheckouts?.includes(poolEntryId),
    `stale repair candidates did not include ${poolEntryId}: ${JSON.stringify(staleRepair)}`,
  );
  assert(
    staleRepair.data?.candidates?.staleRoutes?.includes(routeId),
    `stale route candidates did not include ${routeId}: ${JSON.stringify(staleRepair)}`,
  );
  assert(
    staleRepair.data?.candidates?.staleDisplayAllocations?.includes(browser.displayAllocationId),
    `stale display candidates did not include ${browser.displayAllocationId}: ${JSON.stringify(staleRepair)}`,
  );

  const appliedRepair = await serviceRequest(
    streamPort,
    {
      action: 'service_route_pool_repair',
      serviceName,
      agentName,
      taskName: repairTaskName,
      params: {
        apply: true,
        staleCheckouts: true,
      },
      jobTimeoutMs: 30000,
    },
    'stale-route-pool-repair-apply',
  );
  assert(
    appliedRepair.data?.repairedCounts?.staleCheckouts === 1,
    `stale route-pool checkout was not repaired: ${JSON.stringify(appliedRepair)}`,
  );
  assert(
    appliedRepair.data?.repairedCounts?.staleRoutes === 1,
    `stale route record was not released: ${JSON.stringify(appliedRepair)}`,
  );
  assert(
    appliedRepair.data?.repairedCounts?.staleDisplayAllocations === 1,
    `stale display allocation was not released: ${JSON.stringify(appliedRepair)}`,
  );

  const afterRepair = await serviceStatus(streamPort, 'after-route-pool-repair');
  const repairedEntry = routePoolEntry(afterRepair, poolEntryId);
  const repairedRoute = afterRepair.data?.service_state?.remoteViewRoutes?.[routeId];
  const repairedDisplay = afterRepair.data?.service_state?.displayAllocations?.[browser.displayAllocationId];
  assert(repairedEntry.state === 'available', `repaired pool entry is not available: ${JSON.stringify(repairedEntry)}`);
  assert(
    repairedEntry.currentRouteAllocationId === null || repairedEntry.currentRouteAllocationId === undefined,
    `repaired pool entry still points at a route: ${JSON.stringify(repairedEntry)}`,
  );
  assert(repairedRoute?.state === 'released', `repaired route is not released: ${JSON.stringify(repairedRoute)}`);
  assert(
    repairedDisplay?.state === 'released',
    `repaired display allocation is not released: ${JSON.stringify(repairedDisplay)}`,
  );
  writeArtifact('route-cleanup-summary.json', {
    artifactDir,
    browserId: workspace.browserId,
    displayAllocationId: browser.displayAllocationId,
    poolEntryId,
    routeId,
    killedPid: pid,
    repairedEntry,
    repairedRoute,
    repairedDisplay,
  });

  await cleanup();
  console.log(`RDP Guacamole route-cleanup live smoke passed; artifacts: ${artifactDir}`);
} catch (err) {
  console.error(err.stack || err.message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}
