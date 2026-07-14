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
  configureRemoteHeadedContext,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacBrowserSwitchSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacBrowserSwitchLaunch';
const closeTaskName = 'rdpGuacBrowserSwitchClose';
const dashboardFocusTaskName = 'workspace-viewport-control';
const dashboardTakeoverTaskName = 'workspace-viewport-takeover';
const clientASession = 'rdp-guac-switch-client-a';
const clientBSession = 'rdp-guac-switch-client-b';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-browser-switch-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function envValue(name) {
  const value = process.env[name]?.trim();
  return value || null;
}

function requireExistingExecutable(name) {
  const value = envValue(name);
  if (!value) {
    throw new Error(`${name} is required and must point to a browser executable.`);
  }
  if (!existsSync(value)) {
    throw new Error(`${name} does not exist: ${value}`);
  }
  return value;
}

function configuredBrowserSession(name, fallback) {
  const value = envValue(name);
  if (!value) return fallback;
  return value.startsWith('session:') ? value.slice('session:'.length) : value;
}

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
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

async function ensureStreamPortForSession(context, sessionName, timeoutMs = 60000) {
  const streamStatusResult = await runCli(
    context,
    ['--json', '--session', sessionName, 'stream', 'status'],
    timeoutMs,
  );
  let stream = parseJsonOutput(streamStatusResult.stdout, `${sessionName} stream status`);
  assert(
    stream.success === true,
    `${sessionName} stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(
      context,
      ['--json', '--session', sessionName, 'stream', 'enable'],
      timeoutMs,
    );
    stream = parseJsonOutput(streamResult.stdout, `${sessionName} stream enable`);
    assert(
      stream.success === true,
      `${sessionName} stream enable failed: ${streamResult.stdout}${streamResult.stderr}`,
    );
  }
  const port = stream.data?.port;
  assert(
    Number.isInteger(port) && port > 0,
    `${sessionName} stream status did not return a port: ${JSON.stringify(stream)}`,
  );
  return port;
}

function dashboardUrl(baseUrl, browserId, tabId, sessionName) {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', `browser:${browserId}`);
  url.searchParams.set('browser', browserId);
  url.searchParams.set('session', sessionName);
  if (tabId) url.searchParams.set('tab', tabId);
  return url.toString();
}

function dashboardStateScript() {
  return `
(() => {
  const text = document.body?.innerText || "";
  const viewport = document.querySelector(".workspace-remote-viewport");
  const header = viewport?.querySelector(".workspace-remote-viewport-header");
  const stage = viewport?.querySelector(".workspace-remote-viewport-stage");
  const frame = viewport?.querySelector("iframe");
  const url = new URL(location.href);
  const buttons = Array.from(document.querySelectorAll("button"));
  const findButton = (label) => buttons.find((button) => button.textContent?.trim() === label);
  const findAria = (label) => buttons.find((button) => button.getAttribute("aria-label") === label);
  const takeoverButton = findButton("Take over");
  const refreshButton = findAria("Refresh workspace viewport");
  const externalButton = findAria("Open workspace stream externally");
  const interactionButton = findAria("Open Guacamole interaction settings");
  const fullscreenButton = buttons.find((button) => {
    const label = button.getAttribute("aria-label") || "";
    return label.includes("workspace viewport fullscreen") || label.includes("workspace viewport to window");
  });
  return {
    url: location.href,
    title: document.title,
    workspaceParam: url.searchParams.get("workspace"),
    browserParam: url.searchParams.get("browser"),
    sessionParam: url.searchParams.get("session"),
    tabParam: url.searchParams.get("tab"),
    uxState: viewport?.getAttribute("data-ux-state") || null,
    hasViewport: Boolean(viewport),
    hasFrame: Boolean(frame),
    frameSrc: frame?.getAttribute("src") || null,
    heading: header?.querySelector("h2")?.textContent?.replace(/\\s+/g, " ").trim() || null,
    subtitle: Array.from(header?.querySelectorAll("p") || [])
      .map((item) => item.textContent?.replace(/\\s+/g, " ").trim())
      .filter(Boolean)
      .join(" | "),
    badgeTexts: Array.from(viewport?.querySelectorAll(".workspace-remote-viewport-badge-text") || [])
      .map((item) => item.textContent?.trim())
      .filter(Boolean),
    noticeText: Array.from(viewport?.querySelectorAll(".workspace-remote-viewport-notice") || [])
      .map((item) => item.textContent?.replace(/\\s+/g, " ").trim())
      .filter(Boolean),
    stageText: stage?.textContent?.replace(/\\s+/g, " ").trim().slice(0, 1000) || null,
    hasTakeoverButton: Boolean(takeoverButton),
    takeoverDisabled: takeoverButton ? (takeoverButton.disabled || takeoverButton.getAttribute("aria-disabled") === "true") : null,
    hasRefreshButton: Boolean(refreshButton),
    hasExternalButton: Boolean(externalButton),
    hasInteractionButton: Boolean(interactionButton),
    interactionDisabled: interactionButton ? (interactionButton.disabled || interactionButton.getAttribute("aria-disabled") === "true") : null,
    hasFullscreenButton: Boolean(fullscreenButton),
    textIncludesTakenOver: /taken over|replaced by another|another dashboard|another guacamole popout/i.test(text),
    textIncludesReconnect: /take over to reconnect|waiting for a fresh stream|reconnect/i.test(text),
    textSample: text.replace(/\\s+/g, " ").slice(0, 1800),
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
  return { ok: true, status: response.status, username: payload.user?.username || null };
})()
`, 30000);
  assert(result?.ok === true, `${session} dashboard login failed: ${JSON.stringify(result)}`);
}

async function waitForClientState(context, session, predicate, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastState = null;
  while (Date.now() - started < timeoutMs) {
    lastState = await evalInClient(context, session, dashboardStateScript(), 30000);
    if (predicate(lastState)) return lastState;
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  const artifactLabel = label.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || session;
  writeArtifact(`${artifactLabel}-last-state.json`, lastState);
  try {
    await screenshotClient(context, session, `${artifactLabel}-timeout`);
  } catch {
    // Timeout artifacts are best-effort because the client may already be gone.
  }
  throw new Error(`${label} did not become true. Last ${session} state: ${JSON.stringify(lastState)}`);
}

async function waitForClientWorkspace(context, session, workspace, label, timeoutMs = 60000) {
  return waitForClientState(
    context,
    session,
    (state) =>
      state?.hasViewport &&
      state?.hasFrame &&
      state?.hasInteractionButton &&
      state?.hasFullscreenButton &&
      state?.browserParam === workspace.browserId &&
      state?.sessionParam === workspace.sessionName &&
      state?.tabParam === workspace.tabId &&
      state?.heading?.includes(workspace.title),
    label,
    timeoutMs,
  );
}

async function openDashboardClient(context, {
  executable,
  profile,
  session,
  url,
  viewport,
}) {
  const args = [
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
  ];
  const openedResult = await runCli(context, args, 180000);
  const opened = parseJsonOutput(openedResult.stdout, `${session} dashboard open`);
  assert(opened.success === true, `${session} dashboard open failed: ${openedResult.stdout}${openedResult.stderr}`);
  await runCli(context, ['--json', '--session', session, 'set', 'viewport', String(viewport.width), String(viewport.height)]);
}

async function navigateDashboardClient(context, session, url) {
  const result = await evalInClient(context, session, `
(() => {
  location.href = ${JSON.stringify(url)};
  return { navigating: true, url: location.href };
})()
`, 30000);
  assert(result?.navigating === true, `${session} did not start navigation to ${url}: ${JSON.stringify(result)}`);
}

async function refreshClient(context, session) {
  await evalInClient(context, session, 'location.reload(); ({ reloading: true, url: location.href })', 30000);
}

async function screenshotClient(context, session, label) {
  const path = join(artifactDir, `${label}.png`);
  const result = await runCli(context, ['--json', '--session', session, 'screenshot', path], 60000);
  const parsed = parseJsonOutput(result.stdout, `${session} screenshot`);
  assert(parsed.success === true, `${session} screenshot failed: ${result.stdout}${result.stderr}`);
  return path;
}

async function serviceStatusArtifact(streamPort, label) {
  const status = await httpJson(streamPort, 'GET', '/api/service/status');
  assert(status.success === true, `service status failed for ${label}: ${JSON.stringify(status)}`);
  writeArtifact(`${label}-service-status.json`, status);
  return status;
}

async function waitForServiceTabRecord(streamPort, browserId, title, label, timeoutMs = 45000) {
  const started = Date.now();
  let lastTabs = null;
  while (Date.now() - started < timeoutMs) {
    const status = await httpJson(streamPort, 'GET', '/api/service/status');
    assert(status.success === true, `HTTP service status failed: ${JSON.stringify(status)}`);
    lastTabs = Object.values(status.data?.service_state?.tabs || {});
    const tab = lastTabs.find(
      (item) =>
        item?.browserId === browserId &&
        item?.lifecycle !== 'closed' &&
        (!title || item?.title === title),
    );
    if (tab?.id) return tab;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not record a live tab for ${browserId}. Last tabs: ${JSON.stringify(lastTabs)}`);
}

async function currentJobIds(streamPort, action) {
  const jobsResponse = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=100');
  const jobs = jobsResponse.data?.jobs ?? [];
  return new Set(jobs.filter((item) => item.action === action).map((item) => item.id));
}

async function waitForJob(streamPort, {
  action,
  ignoredIds,
  label,
  states = ['succeeded'],
  taskName,
  timeoutMs = 30000,
}) {
  const started = Date.now();
  let lastJobs = null;
  while (Date.now() - started < timeoutMs) {
    const jobsResponse = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=100');
    lastJobs = jobsResponse.data?.jobs ?? [];
    const job = lastJobs.find(
      (item) =>
        item.action === action &&
        (!taskName || item.taskName === taskName) &&
        !ignoredIds.has(item.id) &&
        states.includes(item.state),
    );
    if (job) return job;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not queue ${action} with state ${states.join(' or ')}. Last jobs: ${JSON.stringify(lastJobs)}`);
}

async function waitForOptionalJob(streamPort, options) {
  try {
    return {
      observed: true,
      job: await waitForJob(streamPort, options),
    };
  } catch (error) {
    return {
      observed: false,
      error: String(error?.message || error),
    };
  }
}

async function clickExternalOpen(context, session) {
  return evalInClient(context, session, `
(() => {
  const button = Array.from(document.querySelectorAll("button"))
    .find((item) => item.getAttribute("aria-label") === "Open workspace stream externally");
  if (!button) return { clicked: false, reason: "missing" };
  if (button.disabled || button.getAttribute("aria-disabled") === "true") {
    return { clicked: false, reason: "disabled" };
  }
  const originalOpen = window.open;
  const opened = [];
  window.open = (...args) => {
    opened.push(args.map((item) => String(item)));
    return null;
  };
  button.scrollIntoView({ block: "center", inline: "nearest" });
  try {
    button.click();
  } finally {
    window.open = originalOpen;
  }
  return { clicked: true, opened };
})()
`);
}

async function launchRemoteBrowser({
  context,
  displayIsolation,
  remoteConfig,
  sessionName,
  title,
}) {
  const streamPort = await ensureStreamPortForSession(context, sessionName, 180000);
  const runtimeProfile = `${sessionName}-profile`;
  const browserId = `session:${sessionName}`;
  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    browserId,
    sessionName,
    runtimeProfile,
    params: {
      browserHost: 'remote_headed',
      displayIsolation,
      headless: false,
      remoteHeadedDisplay: envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY'),
      runtimeProfile,
      url: smokeDataUrl(title, title),
      waitUntil: 'load',
      viewStreamProvider: remoteConfig.viewStreamProvider,
      controlInputProvider: remoteConfig.controlInputProvider,
      viewStreamUrl: remoteConfig.viewStreamUrl,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `${title} remote_headed RDP launch failed: ${JSON.stringify(launchResponse)}`);
  const serviceTab = await waitForServiceTabRecord(streamPort, browserId, title, `${title} launch`);
  return {
    browserId,
    launched: true,
    launchResponse,
    sessionName,
    streamPort,
    tabId: serviceTab.id,
    title,
    runtimeProfile,
  };
}

function assertFinalBrowserState(status, workspace) {
  const browser = status.data?.service_state?.browsers?.[workspace.browserId];
  assert(browser, `Final service state missing browser ${workspace.browserId}: ${JSON.stringify(status.data)}`);
  assert(
    browser.health === 'ready',
    `${workspace.title} browser did not remain ready: ${JSON.stringify(browser)}`,
  );
  const tab = status.data?.service_state?.tabs?.[workspace.tabId];
  assert(tab, `Final service state missing tab ${workspace.tabId}: ${JSON.stringify(status.data)}`);
  assert(
    tab.browserId === workspace.browserId && tab.title === workspace.title && tab.lifecycle !== 'closed',
    `${workspace.title} tab identity drifted: ${JSON.stringify(tab)}`,
  );
}

async function closeRemoteBrowser(context, workspace) {
  if (workspace?.streamPort && workspace?.launched) {
    try {
      await httpJson(workspace.streamPort, 'POST', '/api/service/request', {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: closeTaskName,
        params: { browserId: workspace.browserId },
        jobTimeoutMs: 30000,
      });
    } catch {
      // Session close is the final cleanup path for failed launch or shutdown cases.
    }
  }
  if (workspace?.sessionName) {
    try {
      await runCli(context, ['--json', '--session', workspace.sessionName, 'close'], 30000);
    } catch {
      // Remote browser cleanup remains best-effort after failures.
    }
  }
}

function classifyViewerOutcome(client1, client2) {
  if (client1?.hasTakeoverButton || client1?.textIncludesTakenOver) return 'client_2_took_over';
  if (client2?.hasTakeoverButton || client2?.textIncludesTakenOver) return 'client_1_retained_ownership';
  if (client1?.hasFrame && client2?.hasFrame) return 'simultaneous_view';
  return 'unclassified';
}

const timeout = setTimeout(() => {
  fail('Timed out waiting for RDP and Guacamole browser-switch live smoke to complete');
}, 540000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-switch-',
  session: configuredBrowserSession('AGENT_BROWSER_RDP_TEST_BROWSER_A', undefined),
  sessionPrefix: 'rdp-guac-switch-a',
});
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');

let browserASession;
let browserBSession;
let profileA;
let profileB;
let clientAExecutable;
let clientBExecutable;
let displayIsolation;
let remoteConfig;

try {
  browserASession = context.session;
  browserBSession = configuredBrowserSession(
    'AGENT_BROWSER_RDP_TEST_BROWSER_B',
    `rdp-guac-switch-b-${process.pid}`,
  );
  assert(
    browserASession !== browserBSession,
    `AGENT_BROWSER_RDP_TEST_BROWSER_A and AGENT_BROWSER_RDP_TEST_BROWSER_B must identify different daemon sessions: ${browserASession}`,
  );
  profileA = envValue('AGENT_BROWSER_RDP_TEST_PROFILE_A') || join(context.tempHome, 'client-a-profile');
  profileB = envValue('AGENT_BROWSER_RDP_TEST_PROFILE_B') || join(context.tempHome, 'client-b-profile');
  clientAExecutable = requireExistingExecutable('AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE');
  clientBExecutable = requireExistingExecutable('AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE');
  assert(
    clientAExecutable !== clientBExecutable,
    'AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE and AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE must be different executables.',
  );

  const explicitRemoteViewUrl = envValue('AGENT_BROWSER_REMOTE_VIEW_URL');
  assert(
    explicitRemoteViewUrl,
    'AGENT_BROWSER_REMOTE_VIEW_URL is required so the smoke can open a real Guacamole or RDP gateway stream.',
  );
  displayIsolation = envValue('AGENT_BROWSER_RDP_TEST_DISPLAY_ISOLATION') || 'shared_display';
  if (displayIsolation === 'shared_display') {
    assert(
      envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY'),
      'AGENT_BROWSER_REMOTE_HEADED_DISPLAY is required when AGENT_BROWSER_RDP_TEST_DISPLAY_ISOLATION is shared_display.',
    );
  }

  context.env.AGENT_BROWSER_REMOTE_VIEW_URL = explicitRemoteViewUrl;
  context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = envValue('AGENT_BROWSER_REMOTE_VIEW_PROVIDER') || 'rdp_gateway';
  context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER =
    envValue('AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER') || 'manual_attached_desktop';
  if (envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY')) {
    context.env.AGENT_BROWSER_REMOTE_HEADED_DISPLAY = envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY');
  }

  remoteConfig = configureRemoteHeadedContext(context);
  assert(
    remoteConfig.viewStreamProvider === 'rdp_gateway',
    `AGENT_BROWSER_REMOTE_VIEW_PROVIDER must be rdp_gateway for this smoke, got ${remoteConfig.viewStreamProvider}.`,
  );
} catch (err) {
  clearTimeout(timeout);
  console.error(err.stack || err.message);
  console.error(`Artifacts: ${artifactDir}`);
  cleanupSmokeHome(context);
  process.exit(1);
}

let browserA = null;
let browserB = null;

async function cleanup() {
  clearTimeout(timeout);
  for (const clientSession of [clientASession, clientBSession]) {
    try {
      await runCli(context, ['--json', '--session', clientSession, 'close'], 30000);
    } catch {
      // Client cleanup is best-effort because failed launches leave no session.
    }
  }
  await closeRemoteBrowser(context, browserB);
  await closeRemoteBrowser(context, browserA);
  cleanupSmokeHome(context);
}

async function fail(message) {
  console.error(message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}

try {
  browserA = await launchRemoteBrowser({
    context,
    displayIsolation,
    remoteConfig,
    sessionName: browserASession,
    title: 'RDP Guac Browser Switch A',
  });
  writeArtifact('browser-a-launch-response.json', browserA.launchResponse);

  browserB = await launchRemoteBrowser({
    context,
    displayIsolation,
    remoteConfig,
    sessionName: browserBSession,
    title: 'RDP Guac Browser Switch B',
  });
  writeArtifact('browser-b-launch-response.json', browserB.launchResponse);

  const baseDashboardUrl = envValue('AGENT_BROWSER_RDP_TEST_PUBLIC_URL') || `http://127.0.0.1:${browserA.streamPort}/`;
  browserA.workspaceUrl = dashboardUrl(baseDashboardUrl, browserA.browserId, browserA.tabId, browserA.sessionName);
  browserB.workspaceUrl = dashboardUrl(baseDashboardUrl, browserB.browserId, browserB.tabId, browserB.sessionName);
  await serviceStatusArtifact(browserA.streamPort, 'after-browser-launches');

  writeArtifact('fixture.json', {
    artifactDir,
    browserA,
    browserB,
    clientAExecutable,
    clientBExecutable,
    clientASession,
    clientBSession,
    displayIsolation,
    profileA,
    profileB,
    route: {
      url: remoteConfig.viewStreamUrl,
      frameUrl: remoteConfig.frameUrl,
      externalUrl: remoteConfig.externalUrl,
      routeId: remoteConfig.routeId,
      connectionId: remoteConfig.connectionId,
      connectionName: remoteConfig.connectionName,
    },
    streamPort: browserA.streamPort,
  });

  const beforeClientAFocusJobs = await currentJobIds(browserA.streamPort, 'view_focus');
  await openDashboardClient(context, {
    executable: clientAExecutable,
    profile: profileA,
    session: clientASession,
    url: browserA.workspaceUrl,
    viewport: { width: 1440, height: 900 },
  });
  const dashboardCredentials = await waitForDashboardCredentials(context);
  await loginDashboardClient(context, clientASession, dashboardCredentials);
  const clientAOnA = await waitForClientWorkspace(
    context,
    clientASession,
    browserA,
    'client 1 connected to browser A workspace viewport',
  );
  const clientAInitialFocusJob = await waitForJob(browserA.streamPort, {
    action: 'view_focus',
    ignoredIds: beforeClientAFocusJobs,
    label: 'client 1 browser A focus',
    states: ['succeeded', 'running', 'queued'],
    taskName: dashboardFocusTaskName,
  });
  writeArtifact('client-1-browser-a-connected.json', { state: clientAOnA, focusJob: clientAInitialFocusJob });
  await screenshotClient(context, clientASession, 'client-1-browser-a-connected');

  const beforeClientBFocusJobs = await currentJobIds(browserB.streamPort, 'view_focus');
  await navigateDashboardClient(context, clientASession, browserB.workspaceUrl);
  const clientAOnB = await waitForClientWorkspace(
    context,
    clientASession,
    browserB,
    'client 1 switched to browser B workspace viewport',
  );
  const clientBFocusJob = await waitForJob(browserB.streamPort, {
    action: 'view_focus',
    ignoredIds: beforeClientBFocusJobs,
    label: 'client 1 browser B focus',
    states: ['succeeded', 'running', 'queued'],
    taskName: dashboardFocusTaskName,
  });
  writeArtifact('client-1-browser-b-switched.json', { state: clientAOnB, focusJob: clientBFocusJob });
  await screenshotClient(context, clientASession, 'client-1-browser-b-switched');
  await serviceStatusArtifact(browserA.streamPort, 'after-client-1-switch-to-b');

  await refreshClient(context, clientASession);
  const clientAAfterRefreshB = await waitForClientWorkspace(
    context,
    clientASession,
    browserB,
    'client 1 refresh recovery on browser B',
  );
  writeArtifact('client-1-browser-b-after-refresh.json', clientAAfterRefreshB);
  await screenshotClient(context, clientASession, 'client-1-browser-b-after-refresh');
  await serviceStatusArtifact(browserA.streamPort, 'after-client-1-refresh-b');

  await openDashboardClient(context, {
    executable: clientBExecutable,
    profile: profileB,
    session: clientBSession,
    url: browserA.workspaceUrl,
    viewport: { width: 1366, height: 860 },
  });
  await loginDashboardClient(context, clientBSession, dashboardCredentials);
  const clientBOnA = await waitForClientWorkspace(
    context,
    clientBSession,
    browserA,
    'client 2 connected to browser A while client 1 remains routed to browser B',
  );
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const clientAAfterClientB = await evalInClient(context, clientASession, dashboardStateScript());
  const clientBAfterClientB = await evalInClient(context, clientBSession, dashboardStateScript());
  const twoBrowserViewerOutcome = classifyViewerOutcome(clientAAfterClientB, clientBAfterClientB);
  assert(
    twoBrowserViewerOutcome !== 'unclassified',
    `Could not classify two-client cross-browser viewer behavior: ${JSON.stringify({ clientAAfterClientB, clientBAfterClientB })}`,
  );
  assert(
    clientAAfterClientB.browserParam === browserB.browserId && clientAAfterClientB.heading?.includes(browserB.title),
    `Client 1 did not preserve browser B route after client 2 opened A: ${JSON.stringify(clientAAfterClientB)}`,
  );
  writeArtifact('after-client-2-opens-browser-a.json', {
    client1: clientAAfterClientB,
    client2: clientBAfterClientB,
    client2Connected: clientBOnA,
    outcome: twoBrowserViewerOutcome,
  });
  await screenshotClient(context, clientASession, 'client-1-after-client-2-opens-browser-a');
  await screenshotClient(context, clientBSession, 'client-2-browser-a-connected');
  await serviceStatusArtifact(browserA.streamPort, 'after-client-2-opens-browser-a');

  const alternation = [browserA, browserB, browserA, browserB];
  const alternationResults = [];
  for (let index = 0; index < alternation.length; index += 1) {
    const workspace = alternation[index];
    const beforeFocusJobs = await currentJobIds(workspace.streamPort, 'view_focus');
    await navigateDashboardClient(context, clientASession, workspace.workspaceUrl);
    const state = await waitForClientWorkspace(
      context,
      clientASession,
      workspace,
      `client 1 alternation ${index + 1} to ${workspace.title}`,
    );
    const focusJob = await waitForOptionalJob(workspace.streamPort, {
      action: 'view_focus',
      ignoredIds: beforeFocusJobs,
      label: `client 1 alternation ${index + 1} focus`,
      states: ['succeeded', 'running', 'queued'],
      taskName: dashboardFocusTaskName,
    });
    const screenshot = await screenshotClient(
      context,
      clientASession,
      `client-1-alternation-${index + 1}-${workspace === browserA ? 'a' : 'b'}`,
    );
    alternationResults.push({
      browserId: workspace.browserId,
      focusJob,
      screenshot,
      state,
      title: workspace.title,
    });
  }
  writeArtifact('client-1-alternation-results.json', alternationResults);

  const beforeExternalJobIds = await currentJobIds(browserB.streamPort, 'view_takeover');
  const externalResult = await clickExternalOpen(context, clientASession);
  assert(externalResult?.clicked === true, `External open button was not clickable: ${JSON.stringify(externalResult)}`);
  const externalJob = await waitForJob(browserB.streamPort, {
    action: 'view_takeover',
    ignoredIds: beforeExternalJobIds,
    label: 'browser-switch external open takeover',
    taskName: dashboardTakeoverTaskName,
  });
  writeArtifact('external-open-takeover-job.json', { externalResult, externalJob });
  await screenshotClient(context, clientASession, 'client-1-after-external-open');

  const finalStatus = await serviceStatusArtifact(browserA.streamPort, 'final');
  assertFinalBrowserState(finalStatus, browserA);
  assertFinalBrowserState(finalStatus, browserB);

  writeArtifact('summary.json', {
    artifactDir,
    browserA: {
      browserId: browserA.browserId,
      serviceTabId: browserA.tabId,
      sessionName: browserA.sessionName,
      workspaceUrl: browserA.workspaceUrl,
    },
    browserB: {
      browserId: browserB.browserId,
      serviceTabId: browserB.tabId,
      sessionName: browserB.sessionName,
      workspaceUrl: browserB.workspaceUrl,
    },
    displayIsolation,
    externalOpenTakeoverJobId: externalJob.id,
    outcome: twoBrowserViewerOutcome,
    route: {
      url: remoteConfig.viewStreamUrl,
      frameUrl: remoteConfig.frameUrl,
      externalUrl: remoteConfig.externalUrl,
      routeId: remoteConfig.routeId,
      connectionId: remoteConfig.connectionId,
      connectionName: remoteConfig.connectionName,
    },
  });

  await cleanup();
  console.log(`RDP Guacamole browser-switch live smoke passed (${twoBrowserViewerOutcome}); artifacts: ${artifactDir}`);
} catch (err) {
  await fail(err.stack || err.message);
}
