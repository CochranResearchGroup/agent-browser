#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const modes = new Set([
  'route-churn-soak',
  'restart-reconcile',
  'profile-identity',
  'viewer-contention',
  'rollback-and-close',
  'dashboard-rail-persistence',
]);

const mode = process.argv.find((arg) => arg.startsWith('--mode='))?.slice('--mode='.length) ||
  process.env.AGENT_BROWSER_P67_LIVE_MODE ||
  'route-churn-soak';
const fixtureOnly = process.argv.includes('--fixture') || process.env.AGENT_BROWSER_P67_FIXTURE === '1';
const keepBrowsers = process.env.AGENT_BROWSER_P67_KEEP_BROWSERS === '1';
const serviceName = 'P67RdpStressHardening';
const agentName = 'p67-live-smoke';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-p67-rdp-stress-${mode}-${timestamp}`);

assert(modes.has(mode), `Unknown P67 mode ${mode}. Expected one of: ${Array.from(modes).join(', ')}`);
mkdirSync(artifactDir, { recursive: true });

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function envValue(name) {
  const value = process.env[name]?.trim();
  return value || null;
}

function smokeDataUrl(title, heading = title) {
  return `data:text/html,${encodeURIComponent([
    '<!doctype html>',
    '<meta charset="utf-8">',
    `<title>${title}</title>`,
    '<body style="margin:0;font-family:Arial,sans-serif;background:white;color:#111;">',
    '<main style="min-height:100vh;display:grid;place-items:center;text-align:center;">',
    `<h1 style="font-size:42px;">${heading}</h1>`,
    '</main>',
    '</body>',
  ].join(''))}`;
}

function runNode(args, label, env) {
  const result = spawnSync(process.execPath, args, {
    cwd: new URL('..', import.meta.url).pathname,
    env,
    encoding: 'utf8',
    stdio: 'pipe',
    timeout: 120000,
  });
  assert(result.status === 0, `${label} failed: ${result.stdout}${result.stderr}`);
  return parseJsonOutput(result.stdout, label);
}

function parseEnvText(text) {
  const values = {};
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index <= 0) continue;
    const key = trimmed.slice(0, index).trim();
    let value = trimmed.slice(index + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    values[key] = value.replace(/\\"/g, '"');
  }
  return values;
}

function routePoolReadiness(env) {
  if (envValue('AGENT_BROWSER_RDP_ROUTE_POOL_JSON')) {
    return {
      success: true,
      routePoolJson: JSON.parse(process.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON),
      source: 'AGENT_BROWSER_RDP_ROUTE_POOL_JSON',
    };
  }
  return runNode(['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], 'route-pool readiness', env);
}

function selectTwoRouteEntries(report) {
  assert(report.success === true, `route-pool readiness is not green: ${JSON.stringify(report)}`);
  const entries = (report.routePoolJson || []).filter((entry) =>
    entry?.id &&
    entry?.routeId &&
    (entry?.frameUrl || entry?.externalUrl) &&
    entry?.target?.displayName,
  );
  assert(entries.length >= 2, `P67 live gates require at least two ready route-pool entries: ${JSON.stringify(report)}`);
  const [entryA, entryB] = entries;
  assert(entryA.id !== entryB.id, `route entries must be distinct: ${JSON.stringify(entries)}`);
  assert(entryA.routeId !== entryB.routeId, `route ids must be distinct: ${JSON.stringify(entries)}`);
  return [entryA, entryB];
}

async function runSessionJson(context, session, args, label, timeoutMs = 120000) {
  const result = await runCli(context, ['--json', '--session', session, ...args], timeoutMs);
  const parsed = parseJsonOutput(result.stdout, label);
  assert(parsed.success === true, `${label} failed: ${result.stdout}${result.stderr}`);
  return parsed;
}

async function ensureStreamPort(context, session) {
  let status = await runSessionJson(context, session, ['stream', 'status'], `${session} stream status`, 60000);
  if (!status.data?.enabled) {
    status = await runSessionJson(context, session, ['stream', 'enable'], `${session} stream enable`, 60000);
  }
  const port = status.data?.port;
  assert(Number.isInteger(port) && port > 0, `${session} did not return a stream port: ${JSON.stringify(status)}`);
  return port;
}

async function serviceStatus(port, label) {
  const status = await httpJson(port, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

async function serviceRequest(port, body, label) {
  const response = await httpJson(port, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

async function serviceRequestExpectedFailure(port, body, label) {
  const response = await httpJson(port, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success !== true, `${label} unexpectedly succeeded: ${JSON.stringify(response)}`);
  return response;
}

async function reconcile(port, label) {
  const response = await httpJson(port, 'POST', '/api/service/reconcile');
  writeArtifact(`${label}-reconcile-response.json`, response);
  assert(response.success === true, `${label} reconcile failed: ${JSON.stringify(response)}`);
  assert(response.data?.reconciled === true, `${label} did not reconcile: ${JSON.stringify(response)}`);
  return response;
}

async function restartStream(context, sessionName, label) {
  const disabledResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'disable'], 60000);
  const disabled = parseJsonOutput(disabledResult.stdout, `${label} stream disable`);
  writeArtifact(`${label}-stream-disable-response.json`, disabled);
  assert(disabled.success === true, `${label} stream disable failed: ${disabledResult.stdout}${disabledResult.stderr}`);

  const enabledResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'enable'], 90000);
  const enabled = parseJsonOutput(enabledResult.stdout, `${label} stream enable`);
  writeArtifact(`${label}-stream-enable-response.json`, enabled);
  assert(enabled.success === true, `${label} stream enable failed: ${enabledResult.stdout}${enabledResult.stderr}`);
  const restartedPort = enabled.data?.port;
  assert(Number.isInteger(restartedPort) && restartedPort > 0, `${label} stream restart did not return a port: ${JSON.stringify(enabled)}`);
  return restartedPort;
}

function stateFromStatus(status) {
  return status.data?.service_state || status.data || {};
}

function remoteBrowserRows(status) {
  return Object.values(stateFromStatus(status).browsers || {}).filter((browser) =>
    browser?.host === 'remote_headed' ||
    browser?.viewStreams?.some((stream) => stream?.provider === 'rdp_gateway'),
  );
}

function assertBrowserPresent(status, browserId, label) {
  const browser = stateFromStatus(status).browsers?.[browserId];
  assert(browser, `${label} missing browser ${browserId}: ${JSON.stringify(stateFromStatus(status).browsers)}`);
  return browser;
}

function assertReattachable(status, browserId, label) {
  const browser = assertBrowserPresent(status, browserId, label);
  assert(
    !String(browser.attachability?.state || '').startsWith('not_reattachable'),
    `${label} browser is not reattachable: ${JSON.stringify(browser.attachability || browser)}`,
  );
  return browser;
}

function assertRouteOwnership(status, routeId, browserId, routePoolEntryId, label) {
  const state = stateFromStatus(status);
  const route = state.remoteViewRoutes?.[routeId];
  const entry = state.routePool?.[routePoolEntryId];
  assert(route, `${label} missing route ${routeId}: ${JSON.stringify(state.remoteViewRoutes)}`);
  assert(entry, `${label} missing route-pool entry ${routePoolEntryId}: ${JSON.stringify(state.routePool)}`);
  assert(route.browserId === browserId, `${label} route owner mismatch: ${JSON.stringify(route)}`);
  assert(route.state === 'ready', `${label} route is not ready: ${JSON.stringify(route)}`);
  assert(entry.state === 'checked_out', `${label} route-pool entry is not checked out: ${JSON.stringify(entry)}`);
  assert(entry.currentRouteAllocationId === routeId, `${label} route-pool allocation mismatch: ${JSON.stringify(entry)}`);
  const browser = assertBrowserPresent(status, browserId, label);
  assert(
    browser.viewStreams?.some((stream) => stream.routeId === routeId),
    `${label} browser does not have stream for ${routeId}: ${JSON.stringify(browser.viewStreams)}`,
  );
}

function checkedOutRouteCount(status) {
  return Object.values(stateFromStatus(status).routePool || {})
    .filter((entry) => entry?.provider === 'rdp_gateway' && entry?.state === 'checked_out')
    .length;
}

async function openRouteBoundBrowser(context, {
  session,
  runtimeProfile,
  routeEntry,
  title,
  taskName,
  profileId,
}) {
  const args = [
    'remote-view',
    'open',
    smokeDataUrl(title),
    '--runtime-profile',
    runtimeProfile,
    '--display',
    routeEntry.target?.displayName,
    '--display-isolation',
    routeEntry.target?.displayIsolation || 'shared_display',
    '--route-pool-entry-json',
    JSON.stringify(routeEntry),
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
  ];
  if (profileId) args.push('--profile-id', profileId);
  const response = await runSessionJson(context, session, args, `${session} remote-view open`, 180000);
  assert(response.data?.status === 'opened', `${session} did not open remote view: ${JSON.stringify(response)}`);
  assert(
    response.data?.operatorVisible?.state === 'ready' ||
      response.data?.verification?.visibleWindowProof?.displayContent?.state === 'browser_window_visible',
    `${session} did not prove operator-visible browser window: ${JSON.stringify(response.data?.operatorVisible || response.data?.verification)}`,
  );
  return {
    browserId: `session:${session}`,
    routeId: routeEntry.routeId,
    routePoolEntryId: routeEntry.id,
    sessionName: session,
    streamPort: await ensureStreamPort(context, session),
    runtimeProfile,
    title,
    openResponse: response,
  };
}

async function openParkedBrowser(context, port, {
  session,
  runtimeProfile = null,
  profileId = null,
  title,
}) {
  const streamPort = await ensureStreamPort(context, session);
  const browserId = `session:${session}`;
  const request = {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: `p67LaunchParked${title.replace(/[^A-Za-z0-9]/g, '')}`,
    browserId,
    sessionName: session,
    params: {
      browserHost: 'remote_headed',
      controlInputProvider: 'manual_attached_desktop',
      displayIsolation: 'private_virtual_display',
      headless: false,
      url: smokeDataUrl(title),
      viewStreamProvider: 'rdp_gateway',
      waitUntil: 'load',
    },
    jobTimeoutMs: 120000,
  };
  if (runtimeProfile) {
    request.runtimeProfile = runtimeProfile;
    request.params.runtimeProfile = runtimeProfile;
  }
  if (profileId) {
    request.profileId = profileId;
    request.params.profileId = profileId;
  }
  const response = await serviceRequest(
    port,
    request,
    `p67-launch-parked-${session}`,
  );
  return {
    browserId,
    sessionName: session,
    streamPort,
    runtimeProfile,
    profileId,
    title,
    launchResponse: response,
  };
}

async function routeSwitch(port, browser, extraParams, label) {
  const response = await serviceRequest(
    port,
    {
      action: 'service_remote_view_route_switch',
      serviceName,
      agentName,
      taskName: label,
      params: {
        browserId: browser.browserId,
        sessionName: browser.sessionName,
        ...extraParams,
      },
      jobTimeoutMs: 90000,
    },
    label,
  );
  assert(response.data?.status === 'route_switched', `${label} did not switch route: ${JSON.stringify(response)}`);
  return response;
}

async function closeBrowser(port, browser, label) {
  try {
    return await serviceRequest(
      browser.streamPort || port,
      {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: label,
        params: { browserId: browser.browserId },
        jobTimeoutMs: 60000,
      },
      label,
    );
  } catch (error) {
    if (!browser.sessionName) throw error;
    const result = spawnSync('agent-browser', ['--json', '--session', browser.sessionName, 'close'], {
      cwd: new URL('..', import.meta.url).pathname,
      env: process.env,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: 60000,
    });
    assert(result.status === 0, `${label} CLI close fallback failed: ${result.stdout}${result.stderr}`);
    const parsed = parseJsonOutput(result.stdout, `${label} CLI close fallback`);
    writeArtifact(`${label}-cli-close-fallback.json`, parsed);
    assert(parsed.success === true, `${label} CLI close fallback failed: ${result.stdout}${result.stderr}`);
    return parsed;
  }
}

async function waitForDashboardCredentials(context, timeoutMs = 60000) {
  const path = join(context.agentHome, 'dashboard-auth.env');
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (existsSync(path)) {
      const values = parseEnvText(readFileSync(path, 'utf8'));
      const username =
        values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ||
        values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME;
      const password =
        values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD ||
        values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
      if (username && password) return { path, username, password };
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Dashboard bootstrap credential file was not ready at ${path}`);
}

function requireExistingExecutable(name) {
  const value = envValue(name);
  if (!value) throw new Error(`${name} is required and must point to a browser executable.`);
  if (!existsSync(value)) throw new Error(`${name} does not exist: ${value}`);
  return value;
}

function dashboardUrl(baseUrl, browser) {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', `browser:${browser.browserId}`);
  url.searchParams.set('browser', browser.browserId);
  url.searchParams.set('session', browser.sessionName);
  return url.toString();
}

function dashboardStateScript() {
  return `
(() => {
  const text = document.body?.innerText || "";
  const viewport = document.querySelector(".workspace-remote-viewport");
  const frame = viewport?.querySelector("iframe");
  const buttons = Array.from(document.querySelectorAll("button"));
  const findAria = (label) => buttons.find((button) => button.getAttribute("aria-label") === label);
  return {
    url: location.href,
    hasViewport: Boolean(viewport),
    hasFrame: Boolean(frame),
    frameSrc: frame?.getAttribute("src") || null,
    browserParam: new URL(location.href).searchParams.get("browser"),
    sessionParam: new URL(location.href).searchParams.get("session"),
    hasExternalButton: Boolean(findAria("Open workspace stream externally")),
    hasInteractionButton: Boolean(findAria("Open Guacamole interaction settings")),
    hasRefreshButton: Boolean(findAria("Refresh workspace viewport")),
    textIncludesTakenOver: /taken over|replaced by another|another dashboard|another guacamole popout/i.test(text),
    textSample: text.replace(/\\s+/g, " ").slice(0, 1600),
  };
})()
`;
}

async function evalInClient(context, session, script, timeoutMs = 60000) {
  const result = await runCli(context, ['--json', '--session', session, 'eval', script], timeoutMs);
  const parsed = parseJsonOutput(result.stdout, `${session} eval`);
  assert(parsed.success === true, `${session} eval failed: ${result.stdout}${result.stderr}`);
  return parsed.data?.result;
}

async function openDashboardClient(context, { executable, profile, session, url, viewport }) {
  const openedResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--profile',
    profile,
    '--executable-path',
    executable,
    '--args',
    '--no-sandbox',
    'open',
    url,
  ], 180000);
  const opened = parseJsonOutput(openedResult.stdout, `${session} dashboard open`);
  assert(opened.success === true, `${session} dashboard open failed: ${openedResult.stdout}${openedResult.stderr}`);
  await runCli(context, ['--json', '--session', session, 'set', 'viewport', String(viewport.width), String(viewport.height)]);
}

async function loginDashboardClient(context, session, credentials) {
  const result = await evalInClient(context, session, `
(async () => {
  const response = await fetch("/api/dashboard-auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "same-origin",
    body: JSON.stringify({
      username: ${JSON.stringify(credentials.username)},
      password: ${JSON.stringify(credentials.password)},
    }),
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || payload.authenticated !== true) {
    return { ok: false, status: response.status, payload };
  }
  location.reload();
  return { ok: true, status: response.status };
})()
`, 30000);
  assert(result?.ok === true, `${session} dashboard login failed: ${JSON.stringify(result)}`);
}

async function waitForDashboardBrowser(context, session, browser, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastState = null;
  while (Date.now() - started < timeoutMs) {
    lastState = await evalInClient(context, session, dashboardStateScript(), 30000);
    if (
      lastState?.hasViewport &&
      lastState?.hasFrame &&
      lastState?.browserParam === browser.browserId &&
      lastState?.sessionParam === browser.sessionName
    ) {
      return lastState;
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  writeArtifact(`${label}-last-dashboard-state.json`, lastState);
  throw new Error(`${label} did not render browser workspace. Last state: ${JSON.stringify(lastState)}`);
}

async function screenshotClient(context, session, label) {
  const path = join(artifactDir, `${label}.png`);
  const result = await runCli(context, ['--json', '--session', session, 'screenshot', path], 60000);
  const parsed = parseJsonOutput(result.stdout, `${session} screenshot`);
  assert(parsed.success === true, `${session} screenshot failed: ${result.stdout}${result.stderr}`);
  return path;
}

async function navigateClient(context, session, url) {
  const result = await evalInClient(context, session, `
(() => {
  location.href = ${JSON.stringify(url)};
  return { navigating: true, url: location.href };
})()
`, 30000);
  assert(result?.navigating === true, `${session} did not navigate to ${url}: ${JSON.stringify(result)}`);
}

async function refreshClient(context, session) {
  await evalInClient(context, session, 'location.reload(); ({ reloading: true, url: location.href })', 30000);
}

function fixtureState() {
  const browsers = {};
  for (const label of ['a', 'b', 'c', 'd']) {
    const browserId = `browser-${label}`;
    const routeId = label === 'a' || label === 'b' ? `route-${label}` : null;
    browsers[browserId] = {
      id: browserId,
      host: 'remote_headed',
      health: label === 'd' ? 'process_exited' : 'ready',
      profileId: `profile-${label}`,
      activeSessionIds: [`session-${label}`],
      attachability: {
        state: label === 'd' ? 'not_reattachable_closed' : routeId ? 'attached_ready' : 'reattachable_route_occupied',
      },
      viewStreams: routeId ? [{
        id: `stream-${label}`,
        provider: 'rdp_gateway',
        routeId,
        routePoolEntryId: `pool-${label}`,
        attachability: { state: 'attached_ready' },
      }] : [],
    };
  }
  return {
    browsers,
    displayAllocations: {
      'display-a': { id: 'display-a', state: 'ready', ownerBrowserId: 'browser-a' },
      'display-b': { id: 'display-b', state: 'ready', ownerBrowserId: 'browser-b' },
    },
    remoteViewRoutes: {
      'route-a': { id: 'route-a', state: 'ready', browserId: 'browser-a', displayAllocationId: 'display-a' },
      'route-b': { id: 'route-b', state: 'ready', browserId: 'browser-b', displayAllocationId: 'display-b' },
      'route-stale': { id: 'route-stale', state: 'released', browserId: 'browser-c', displayAllocationId: 'display-stale' },
    },
    routePool: {
      'pool-a': { id: 'pool-a', provider: 'rdp_gateway', state: 'checked_out', currentRouteAllocationId: 'route-a' },
      'pool-b': { id: 'pool-b', provider: 'rdp_gateway', state: 'checked_out', currentRouteAllocationId: 'route-b' },
    },
    viewerLeases: {
      'viewer-stale': { id: 'viewer-stale', state: 'disconnected', routeId: 'route-stale', browserId: 'browser-c' },
    },
  };
}

function runFixtureValidation() {
  const state = fixtureState();
  writeArtifact('p67-fixture-state.json', state);
  const remoteRows = Object.values(state.browsers).filter((browser) =>
    browser.host === 'remote_headed' || browser.viewStreams.some((stream) => stream.provider === 'rdp_gateway'),
  );
  assert(remoteRows.length === 4, `fixture should retain four browser rows: ${JSON.stringify(remoteRows)}`);
  assert(
    Object.values(state.routePool).filter((entry) => entry.state === 'checked_out').length === 2,
    `fixture should have exactly two checked-out route slots: ${JSON.stringify(state.routePool)}`,
  );
  assert(
    state.browsers['browser-c'].attachability.state === 'reattachable_route_occupied',
    `parked browser fixture should be reattachable: ${JSON.stringify(state.browsers['browser-c'])}`,
  );
  assert(
    state.browsers['browser-d'].attachability.state === 'not_reattachable_closed',
    `closed browser fixture should be terminal: ${JSON.stringify(state.browsers['browser-d'])}`,
  );
  writeArtifact('p67-fixture-summary.json', { artifactDir, mode, fixtureOnly: true });
  console.log(`P67 ${mode} fixture validation passed; artifacts: ${artifactDir}`);
}

async function launchInitialBrowsers(context, entries) {
  const browserA = await openRouteBoundBrowser(context, {
    session: `p67-${mode}-a`,
    runtimeProfile: `p67-${mode}-profile-a`,
    routeEntry: entries[0],
    title: `P67 ${mode} Browser A`,
    taskName: `p67${mode}OpenA`,
  });
  const browserB = await openRouteBoundBrowser(context, {
    session: `p67-${mode}-b`,
    runtimeProfile: `p67-${mode}-profile-b`,
    routeEntry: entries[1],
    title: `P67 ${mode} Browser B`,
    taskName: `p67${mode}OpenB`,
  });
  return [browserA, browserB];
}

async function runRouteChurnLikeScenario(context, port, browsers, entries, {
  cycles = 30,
  closeMidway = false,
  reconcileMidway = false,
} = {}) {
  const sequence = [browsers[0], browsers[1], browsers[2], browsers[0], browsers[3], browsers[1], browsers[0]];
  for (let index = 0; index < cycles; index += 1) {
    const browser = sequence[index % sequence.length];
    if (closeMidway && index === Math.floor(cycles / 2) && !browsers[2].closed) {
      await closeBrowser(port, browsers[2], `p67-${mode}-close-c-midway`);
      browsers[2].closed = true;
    }
    if (browser.closed) continue;
    await routeSwitch(port, browser, {}, `p67-${mode}-switch-${index}-${browser.sessionName}`);
    if (reconcileMidway && index === Math.floor(cycles / 2)) {
      await reconcile(port, `p67-${mode}-midway`);
    }
    const status = await serviceStatus(port, `p67-${mode}-after-switch-${index}`);
    assert(checkedOutRouteCount(status) <= 2, `more than two route slots checked out after switch ${index}`);
    for (const candidate of browsers) {
      if (candidate.closed) continue;
      assertReattachable(status, candidate.browserId, `p67 ${mode} switch ${index}`);
    }
  }
}

async function runLiveScenario() {
  const context = createSmokeContext({
    prefix: 'agent-browser-p67-rdp-live-',
    session: `p67-live-controller-${process.pid}`,
  });
  context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
  context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = 'rdp_gateway';
  context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER = 'manual_attached_desktop';
  context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');
  delete context.env.AGENT_BROWSER_AUTO_CONNECT;

  let port = null;
  const browsers = [];

  try {
    const readiness = routePoolReadiness(context.env);
    writeArtifact('route-pool-readiness.json', readiness);
    const entries = selectTwoRouteEntries(readiness);
    writeArtifact('selected-route-entries.json', { routeA: entries[0], routeB: entries[1] });

    const [browserA, browserB] = await launchInitialBrowsers(context, entries);
    browsers.push(browserA, browserB);
    port = await ensureStreamPort(context, browserA.sessionName);
    let status = await serviceStatus(port, 'after-initial-open');
    assertRouteOwnership(status, entries[0].routeId, browserA.browserId, entries[0].id, 'after initial A');
    assertRouteOwnership(status, entries[1].routeId, browserB.browserId, entries[1].id, 'after initial B');

    if (mode === 'profile-identity') {
      assert(
        stateFromStatus(status).browsers?.[browserA.browserId]?.profileId !== stateFromStatus(status).browsers?.[browserB.browserId]?.profileId,
        `profile identity collapsed between A and B: ${JSON.stringify(stateFromStatus(status).browsers)}`,
      );
      const profileIdBrowserC = await openParkedBrowser(context, port, {
        session: `p67-${mode}-profile-id-c`,
        profileId: `p67-${mode}-explicit-profile-id-c`,
        title: `P67 ${mode} Explicit ProfileId C`,
      });
      const profileIdBrowserD = await openParkedBrowser(context, port, {
        session: `p67-${mode}-profile-id-d`,
        profileId: `p67-${mode}-explicit-profile-id-d`,
        title: `P67 ${mode} Explicit ProfileId D`,
      });
      browsers.push(profileIdBrowserC, profileIdBrowserD);
      status = await serviceStatus(port, 'profile-identity-after-profile-id-variant');
      assert(
        stateFromStatus(status).browsers?.[profileIdBrowserC.browserId]?.profileId === profileIdBrowserC.profileId,
        `profileId variant C did not retain explicit profile id: ${JSON.stringify(stateFromStatus(status).browsers?.[profileIdBrowserC.browserId])}`,
      );
      assert(
        stateFromStatus(status).browsers?.[profileIdBrowserD.browserId]?.profileId === profileIdBrowserD.profileId,
        `profileId variant D did not retain explicit profile id: ${JSON.stringify(stateFromStatus(status).browsers?.[profileIdBrowserD.browserId])}`,
      );
      writeArtifact('profile-identity-summary.json', {
        runtimeProfileBrowserA: stateFromStatus(status).browsers?.[browserA.browserId]?.profileId,
        runtimeProfileBrowserB: stateFromStatus(status).browsers?.[browserB.browserId]?.profileId,
        profileIdBrowserC: stateFromStatus(status).browsers?.[profileIdBrowserC.browserId]?.profileId,
        profileIdBrowserD: stateFromStatus(status).browsers?.[profileIdBrowserD.browserId]?.profileId,
      });
    }

    if (['route-churn-soak', 'restart-reconcile', 'rollback-and-close', 'dashboard-rail-persistence', 'viewer-contention'].includes(mode)) {
      const browserC = await openParkedBrowser(context, port, {
        session: `p67-${mode}-c`,
        runtimeProfile: `p67-${mode}-profile-c`,
        title: `P67 ${mode} Browser C`,
      });
      const browserD = await openParkedBrowser(context, port, {
        session: `p67-${mode}-d`,
        runtimeProfile: `p67-${mode}-profile-d`,
        title: `P67 ${mode} Browser D`,
      });
      browsers.push(browserC, browserD);
      status = await serviceStatus(port, 'after-four-browser-launch');
      assert(
        remoteBrowserRows(status).filter((browser) => browsers.some((candidate) => candidate.browserId === browser.id)).length === 4,
        `owned rail source state does not retain four P67 browsers: ${JSON.stringify(remoteBrowserRows(status))}`,
      );
    }

    if (mode === 'route-churn-soak') {
      await runRouteChurnLikeScenario(context, port, browsers, entries, { closeMidway: true });
    } else if (mode === 'restart-reconcile') {
      await routeSwitch(port, browsers[2], {}, 'p67-restart-reconcile-parked-browser-reattach');
      port = await restartStream(context, browserA.sessionName, 'p67-restart-reconcile');
      await reconcile(port, 'p67-restart-reconcile-before-status');
      status = await serviceStatus(port, 'p67-restart-reconcile-after-reconcile');
      for (const browser of browsers) assertReattachable(status, browser.browserId, 'restart reconcile');
    } else if (mode === 'rollback-and-close') {
      await serviceRequestExpectedFailure(
        port,
        {
          action: 'service_remote_view_route_switch',
          serviceName,
          agentName,
          taskName: 'p67-forced-proof-failure',
          params: {
            browserId: browsers[2].browserId,
            sessionName: browsers[2].sessionName,
            routePoolEntryId: 'p67-missing-route-pool-entry',
          },
          jobTimeoutMs: 60000,
        },
        'p67-forced-proof-failure',
      );
      status = await serviceStatus(port, 'p67-after-forced-proof-failure');
      assertReattachable(status, browserA.browserId, 'rollback pre-existing browser A');
      await closeBrowser(port, browserA, 'p67-close-browser-a');
      status = await serviceStatus(port, 'p67-after-close-browser-a');
      const closedA = stateFromStatus(status).browsers?.[browserA.browserId];
      assert(
        !closedA || String(closedA.attachability?.state || closedA.health || '').includes('closed') ||
          ['closing', 'process_exited', 'not_started'].includes(String(closedA.health || '')),
        `Browser A remained reattachable after explicit close: ${JSON.stringify(closedA)}`,
      );
      assertReattachable(status, browserB.browserId, 'rollback browser B after close A');
    } else if (mode === 'dashboard-rail-persistence') {
      status = await serviceStatus(port, 'p67-dashboard-rail-source');
      const owned = remoteBrowserRows(status);
      assert(
        browsers.every((browser) => owned.some((row) => row.id === browser.browserId)),
        `dashboard rail source lost a P67 browser: ${JSON.stringify(owned)}`,
      );
      writeArtifact('dashboard-rail-persistence-source-proof.json', { owned });
    } else if (mode === 'viewer-contention') {
      const clientAExecutable = requireExistingExecutable('AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE');
      const clientBExecutable = requireExistingExecutable('AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE');
      assert(clientAExecutable !== clientBExecutable, 'P67 viewer contention requires two different client executables.');
      const baseDashboardUrl = envValue('AGENT_BROWSER_RDP_TEST_PUBLIC_URL') || `http://127.0.0.1:${port}/`;
      const clientA = `p67-${mode}-client-a`;
      const clientB = `p67-${mode}-client-b`;
      await openDashboardClient(context, {
        executable: clientAExecutable,
        profile: join(context.tempHome, 'client-a-profile'),
        session: clientA,
        url: dashboardUrl(baseDashboardUrl, browserA),
        viewport: { width: 1440, height: 900 },
      });
      const credentials = await waitForDashboardCredentials(context);
      await loginDashboardClient(context, clientA, credentials);
      const clientAOnA = await waitForDashboardBrowser(context, clientA, browserA, 'client-a-on-browser-a');
      await screenshotClient(context, clientA, 'client-a-on-browser-a');
      await openDashboardClient(context, {
        executable: clientBExecutable,
        profile: join(context.tempHome, 'client-b-profile'),
        session: clientB,
        url: dashboardUrl(baseDashboardUrl, browserB),
        viewport: { width: 1280, height: 820 },
      });
      await loginDashboardClient(context, clientB, credentials);
      const clientBOnB = await waitForDashboardBrowser(context, clientB, browserB, 'client-b-on-browser-b');
      await screenshotClient(context, clientB, 'client-b-on-browser-b');
      await navigateClient(context, clientA, dashboardUrl(baseDashboardUrl, browserB));
      const clientAOnB = await waitForDashboardBrowser(context, clientA, browserB, 'client-a-on-browser-b');
      await refreshClient(context, clientA);
      await refreshClient(context, clientB);
      writeArtifact('viewer-contention-summary.json', { clientAOnA, clientBOnB, clientAOnB });
    }

    status = await serviceStatus(port, 'p67-final-status');
    writeArtifact('p67-live-summary.json', {
      artifactDir,
      mode,
      browsers,
      checkedOutRoutes: checkedOutRouteCount(status),
    });
    console.log(`P67 ${mode} passed; artifacts: ${artifactDir}`);
  } finally {
    if (!keepBrowsers && port) {
      for (const browser of browsers.reverse()) {
        await closeBrowser(port, browser, `p67-cleanup-close-${browser.sessionName}`).catch(() => {});
      }
    }
    cleanupSmokeHome(context);
  }
}

if (fixtureOnly) {
  runFixtureValidation();
  process.exit(0);
}

const timeout = setTimeout(() => {
  console.error(`Timed out waiting for P67 ${mode} live gate`);
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, Number(process.env.AGENT_BROWSER_P67_LIVE_TIMEOUT_MS || 900000));

runLiveScenario()
  .then(() => clearTimeout(timeout))
  .catch((err) => {
    clearTimeout(timeout);
    console.error(err.stack || err.message);
    console.error(`Artifacts: ${artifactDir}`);
    process.exit(1);
  });
