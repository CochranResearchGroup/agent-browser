#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  rootDir,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  closeRemoteHeadedBrowser,
  configureRemoteHeadedContext,
  ensureStreamPort,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacReadinessFailureSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacReadinessLiveLaunch';
const closeTaskName = 'rdpGuacReadinessLiveClose';
const clientSession = 'rdp-guac-readiness-client';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-readiness-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function envValue(name) {
  const value = process.env[name]?.trim();
  return value || null;
}

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function redactUrl(value) {
  if (typeof value !== 'string') return value;
  if (!/^https?:\/\//i.test(value)) return value;
  try {
    const url = new URL(value);
    return `${url.protocol}//${url.host}${url.pathname ? '/...' : ''}`;
  } catch {
    return '[redacted-url]';
  }
}

function redactReadiness(value) {
  if (Array.isArray(value)) return value.map(redactReadiness);
  if (!value || typeof value !== 'object') return redactUrl(value);
  const redacted = {};
  for (const [key, item] of Object.entries(value)) {
    redacted[key] = key.toLowerCase().includes('url') ? redactUrl(item) : redactReadiness(item);
  }
  return redacted;
}

function firstExistingExecutable(names) {
  for (const name of names) {
    if (name && existsSync(name)) return name;
  }
  return null;
}

function discoverRemoteDisplay() {
  const result = spawnSync('sh', ['-lc', "ps -eo user,args | awk '/Xorg :|Xvfb :/ && !/awk/ {print}'"], {
    cwd: rootDir,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.status !== 0) return null;
  const lines = result.stdout.split(/\r?\n/);
  const preferred = lines.find((line) => /Xorg :\d+/.test(line) && /xrdp|agent-browser-rdp|agent-b/.test(line));
  const xorg = lines.find((line) => /Xorg :\d+/.test(line));
  const any = preferred || xorg || lines.find((line) => /Xvfb :\d+/.test(line));
  return any?.match(/(?:Xorg|Xvfb)\s+(:\d+)/)?.[1] ?? null;
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

function dashboardUrl(baseUrl, browserId, tabId = null, sessionName = null, mode = 'control') {
  const url = new URL(baseUrl);
  url.searchParams.set('view', `workspace:${mode}`);
  url.searchParams.set('workspace', `browser:${browserId}`);
  url.searchParams.set('browser', browserId);
  if (sessionName) url.searchParams.set('session', sessionName);
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
  const buttons = Array.from(document.querySelectorAll("button"));
  const links = Array.from(document.querySelectorAll("a"));
  const findButton = (label) => buttons.find((button) => button.textContent?.trim() === label);
  const findAria = (label) => buttons.find((button) => button.getAttribute("aria-label") === label);
  return {
    url: location.href,
    title: document.title,
    uxState: viewport?.getAttribute("data-ux-state") || null,
    readinessStatus: viewport?.getAttribute("data-readiness-status") || null,
    readinessAction: viewport?.getAttribute("data-readiness-action") || null,
    hasViewport: Boolean(viewport),
    hasFrame: Boolean(frame),
    frameSrc: frame?.getAttribute("src") || null,
    heading: header?.querySelector("h2")?.textContent?.replace(/\\s+/g, " ").trim() || null,
    badgeTexts: Array.from(viewport?.querySelectorAll(".workspace-remote-viewport-badge-text") || [])
      .map((item) => item.textContent?.trim())
      .filter(Boolean),
    noticeText: Array.from(viewport?.querySelectorAll(".workspace-remote-viewport-notice") || [])
      .map((item) => item.textContent?.replace(/\\s+/g, " ").trim())
      .filter(Boolean),
    stageText: stage?.textContent?.replace(/\\s+/g, " ").trim().slice(0, 1000) || null,
    hasTakeoverButton: Boolean(findButton("Take over")),
    hasSignInAgainLink: links.some((link) => link.textContent?.trim() === "Sign in again"),
    hasOpenExternallyButton: Boolean(findButton("Open externally") || findAria("Open workspace stream externally")),
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
  writeArtifact(`${label}-service-status.json`, redactReadiness(status));
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

function runHealthyReadinessBaseline() {
  const result = spawnSync(process.execPath, ['scripts/smoke-rdp-gateway-readiness.js', '--require-html5-client'], {
    cwd: rootDir,
    env: process.env,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  let parsed;
  try {
    parsed = JSON.parse(result.stdout.trim());
  } catch (err) {
    throw new Error(`RDP gateway readiness output was not JSON: ${err.message}\n${result.stdout}${result.stderr}`);
  }
  writeArtifact('healthy-readiness-live-redacted.json', redactReadiness(parsed));
  assert(
    result.status === 0 && parsed.success === true && parsed.readiness?.status === 'ready',
    `RDP gateway readiness baseline failed: ${JSON.stringify(redactReadiness(parsed))}\n${result.stderr}`,
  );
  return parsed;
}

function createFixtureServer() {
  const server = createServer((req, res) => {
    const path = new URL(req.url || '/', 'http://127.0.0.1').pathname;
    const title = path.includes('blank') ? 'Fixture Blank' : 'RDP Guac Readiness Fixture';
    const body = path.includes('taken-over')
      ? 'This viewer was replaced by another dashboard.'
      : 'RDP Guac readiness fixture stream rendered.';
    res.writeHead(200, {
      'content-type': 'text/html; charset=utf-8',
      'cache-control': 'no-store',
    });
    res.end(`<!doctype html><title>${title}</title><main style="font:16px sans-serif;padding:24px"><h1>${title}</h1><p>${body}</p></main>`);
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      resolve({
        baseUrl: `http://127.0.0.1:${address.port}`,
        close: () => new Promise((done) => server.close(done)),
      });
    });
  });
}

function statePath(context) {
  return join(context.agentHome, 'service', 'state.json');
}

function readState(context) {
  return JSON.parse(readFileSync(statePath(context), 'utf8'));
}

function writeState(context, state) {
  writeFileSync(statePath(context), `${JSON.stringify(state, null, 2)}\n`);
}

function readiness(component, status, evidence, nextAction, recovery) {
  return {
    components: [
      {
        component,
        status,
        evidence,
        nextAction,
        recovery,
      },
    ],
  };
}

function seedFixtureBrowser(context, {
  browserId,
  displayName,
  health = 'ready',
  lastError = null,
  sessionName,
  stream = null,
  tabs = [],
}) {
  const state = readState(context);
  state.browsers = state.browsers || {};
  state.tabs = state.tabs || {};
  state.browsers[browserId] = {
    id: browserId,
    profileId: `${sessionName}-profile`,
    host: 'remote_headed',
    health,
    browserBuild: 'stealthcdp_chromium',
    displayIsolation: 'shared_display',
    displayName,
    pid: null,
    cdpEndpoint: null,
    viewStreams: stream ? [stream] : [],
    activeSessionIds: [sessionName],
    lastError,
    lastHealthObservation: lastError
      ? {
          observedAt: new Date().toISOString(),
          health,
          reasonKind: 'fixture',
          failureClass: 'slice_d_readiness_fixture',
          processExitCause: health === 'process_exited' ? 'fixture_process_exit' : null,
          message: lastError,
          details: null,
        }
      : null,
  };
  for (const tab of tabs) {
    state.tabs[tab.id] = {
      id: tab.id,
      browserId,
      targetId: tab.targetId ?? null,
      sessionId: sessionName,
      lifecycle: tab.lifecycle ?? 'ready',
      url: tab.url ?? smokeDataUrl(tab.title ?? tab.id, tab.title ?? tab.id),
      title: tab.title ?? tab.id,
      ownerSessionId: sessionName,
      latestSnapshotId: null,
      latestScreenshotId: null,
      challengeId: null,
    };
  }
  writeState(context, state);
}

async function assertFixtureScenario({
  baseDashboardUrl,
  context,
  evidenceClass,
  expected,
  id,
  matrix,
  sessionName,
  streamPort,
  tabId = null,
}) {
  const url = dashboardUrl(baseDashboardUrl, `session:${sessionName}`, tabId, sessionName, 'control');
  await navigateDashboardClient(context, clientSession, url);
  const state = await waitForClientState(
    context,
    clientSession,
    (item) => {
      if (!item?.hasViewport) return false;
      if (expected.uxState && item.uxState !== expected.uxState) return false;
      if (expected.readinessStatus && item.readinessStatus !== expected.readinessStatus) return false;
      if (expected.readinessAction && item.readinessAction !== expected.readinessAction) return false;
      if (expected.hasFrame !== undefined && item.hasFrame !== expected.hasFrame) return false;
      if (expected.hasTakeoverButton !== undefined && item.hasTakeoverButton !== expected.hasTakeoverButton) return false;
      if (expected.hasSignInAgainLink !== undefined && item.hasSignInAgainLink !== expected.hasSignInAgainLink) return false;
      if (expected.hasOpenExternallyButton !== undefined && item.hasOpenExternallyButton !== expected.hasOpenExternallyButton) return false;
      if (expected.textPattern && !expected.textPattern.test(`${item.noticeText?.join(' ')} ${item.stageText} ${item.textSample}`)) return false;
      return true;
    },
    `${id} readiness fixture`,
    60000,
  );
  const screenshot = await screenshotClient(context, clientSession, `fixture-${id}`);
  const status = await serviceStatusArtifact(streamPort, `fixture-${id}`);
  const artifact = writeArtifact(`fixture-${id}.json`, { state, status: redactReadiness(status) });
  matrix.push({
    id,
    evidenceClass,
    readinessStatus: state.readinessStatus,
    readinessAction: state.readinessAction,
    uxState: state.uxState,
    screenshot,
    artifact,
  });
  return state;
}

const timeout = setTimeout(() => {
  fail('Timed out waiting for RDP and Guacamole readiness failure live smoke to complete');
}, 540000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-readiness-',
  sessionPrefix: 'rdp-guac-readiness',
});
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');

let streamPort;
let browserLaunched = false;
let fixtureServer = null;
let liveBrowserId = null;

async function cleanup() {
  clearTimeout(timeout);
  try {
    await runCli(context, ['--json', '--session', clientSession, 'close'], 30000);
  } catch {
    // Client cleanup is best-effort because failed launches leave no session.
  }
  await closeRemoteHeadedBrowser({
    agentName,
    browserId: browserLaunched ? liveBrowserId : null,
    context,
    serviceName,
    streamPort,
    taskName: closeTaskName,
  });
  if (fixtureServer) {
    await fixtureServer.close();
  }
  cleanupSmokeHome(context);
}

async function fail(message) {
  console.error(message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}

try {
  const readinessBaseline = runHealthyReadinessBaseline();
  const remoteViewUrl = envValue('AGENT_BROWSER_REMOTE_VIEW_URL');
  assert(remoteViewUrl, 'AGENT_BROWSER_REMOTE_VIEW_URL is required for the live healthy RDP and Guacamole baseline.');
  const displayName = envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY') || discoverRemoteDisplay();
  assert(displayName, 'AGENT_BROWSER_REMOTE_HEADED_DISPLAY is required or an active Xorg/Xvfb remote display must be discoverable.');
  const clientExecutable = envValue('AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE')
    || firstExistingExecutable(['/usr/bin/google-chrome', '/usr/bin/chromium-browser', '/usr/bin/brave-browser', '/usr/bin/chromium']);
  assert(clientExecutable, 'AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE is required or a common Chrome/Chromium executable must exist.');

  context.env.AGENT_BROWSER_REMOTE_VIEW_URL = remoteViewUrl;
  context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = envValue('AGENT_BROWSER_REMOTE_VIEW_PROVIDER') || 'rdp_gateway';
  context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER =
    envValue('AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER') || 'manual_attached_desktop';
  context.env.AGENT_BROWSER_REMOTE_HEADED_DISPLAY = displayName;

  const remoteConfig = configureRemoteHeadedContext(context);
  assert(
    remoteConfig.viewStreamProvider === 'rdp_gateway',
    `AGENT_BROWSER_REMOTE_VIEW_PROVIDER must be rdp_gateway for this smoke, got ${remoteConfig.viewStreamProvider}.`,
  );

  fixtureServer = await createFixtureServer();
  streamPort = await ensureStreamPort(context, 180000);
  await runCli(context, ['--json', '--session', context.session, 'service', 'status'], 60000);

  const liveTitle = 'RDP Guac Readiness Healthy';
  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    params: {
      browserHost: 'remote_headed',
      displayIsolation: envValue('AGENT_BROWSER_RDP_TEST_DISPLAY_ISOLATION') || 'shared_display',
      headless: false,
      remoteHeadedDisplay: displayName,
      url: smokeDataUrl(liveTitle, liveTitle),
      waitUntil: 'load',
      viewStreamProvider: remoteConfig.viewStreamProvider,
      controlInputProvider: remoteConfig.controlInputProvider,
      viewStreamUrl: remoteConfig.viewStreamUrl,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `remote_headed RDP readiness launch failed: ${JSON.stringify(launchResponse)}`);
  browserLaunched = true;
  liveBrowserId = `session:${context.session}`;
  writeArtifact('live-launch-response.json', redactReadiness(launchResponse));

  const liveTab = await waitForServiceTabRecord(streamPort, liveBrowserId, liveTitle, 'healthy RDP Guac readiness launch');
  const baseDashboardUrl = envValue('AGENT_BROWSER_RDP_TEST_PUBLIC_URL') || `http://127.0.0.1:${streamPort}/`;
  const liveWorkspaceUrl = dashboardUrl(baseDashboardUrl, liveBrowserId, liveTab.id, context.session, 'control');
  writeArtifact('fixture.json', {
    artifactDir,
    clientExecutable,
    displayName,
    liveBrowserId,
    liveTabId: liveTab.id,
    readinessBaseline: redactReadiness(readinessBaseline),
    remoteViewUrl: redactUrl(remoteConfig.viewStreamUrl),
    streamPort,
    liveWorkspaceUrl: redactUrl(liveWorkspaceUrl),
  });

  await openDashboardClient(context, {
    executable: clientExecutable,
    profile: envValue('AGENT_BROWSER_RDP_TEST_PROFILE_A') || join(context.tempHome, 'client-profile'),
    session: clientSession,
    url: liveWorkspaceUrl,
    viewport: { width: 1440, height: 900 },
  });
  const dashboardCredentials = await waitForDashboardCredentials(context);
  await loginDashboardClient(context, clientSession, dashboardCredentials);
  const matrix = [];
  const liveState = await waitForClientState(
    context,
    clientSession,
    (state) =>
      state?.hasViewport &&
      state?.hasFrame &&
      state?.readinessStatus === 'ready' &&
      (state?.uxState === 'connected' || state?.uxState === 'stale_target_recovered'),
    'healthy live RDP Guac workspace viewport',
    90000,
  );
  const liveScreenshot = await screenshotClient(context, clientSession, 'live-healthy-rdp-guac');
  await serviceStatusArtifact(streamPort, 'live-healthy-rdp-guac');
  matrix.push({
    id: 'healthy_rdp_guac',
    evidenceClass: 'live',
    readinessStatus: liveState.readinessStatus,
    readinessAction: liveState.readinessAction,
    uxState: liveState.uxState,
    screenshot: liveScreenshot,
  });

  const fixtureStream = (id, streamReadiness, urlPath = '/healthy') => ({
    id: `${id}-stream`,
    provider: 'rdp_gateway',
    controlInput: 'manual_attached_desktop',
    url: `${fixtureServer.baseUrl}${urlPath}`,
    readOnly: false,
    readiness: streamReadiness,
  });
  const scenarioSession = (id) => `rdp-guac-readiness-${id}`;

  const scenarios = [
    {
      id: 'auth_failure',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('auth');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: fixtureStream('auth', readiness(
            'dashboard_auth',
            'auth_required',
            'fixture dashboard session rejected by stream route',
            'sign_in_again',
            'Sign in again, then refresh the workspace viewport.',
          )),
        });
        return { sessionName };
      },
      expected: {
        readinessStatus: 'action_required',
        readinessAction: 'sign_in_again',
        hasSignInAgainLink: true,
        textPattern: /sign in again|fresh dashboard sign-in/i,
      },
    },
    {
      id: 'guacamole_connection_missing',
      evidenceClass: 'isolated-live',
      seed() {
        const sessionName = scenarioSession('missing-connection');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: fixtureStream('missing-connection', readiness(
            'guacamole_connection',
            'failed',
            'invalid test Guacamole connection id',
            'inspect_readiness',
            'Create or grant the Guacamole connection before opening the workspace stream.',
          )),
        });
        return { sessionName };
      },
      expected: {
        readinessStatus: 'blocked',
        readinessAction: 'inspect_readiness',
        textPattern: /guacamole connection|create or grant/i,
      },
    },
    {
      id: 'provider_ingress_refused',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('provider');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: fixtureStream('provider', readiness(
            'public_ingress',
            'failed',
            'fixture route refused connection',
            'open_externally',
            'Open externally, then inspect DNS, tunnel, proxy, RDP, and Guacamole readiness.',
          )),
        });
        return { sessionName };
      },
      expected: {
        readinessStatus: 'blocked',
        readinessAction: 'open_externally',
        hasOpenExternallyButton: true,
        textPattern: /open externally|ingress|proxy/i,
      },
    },
    {
      id: 'viewer_ownership_changed',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('viewer');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: fixtureStream('viewer', readiness(
            'viewer_lease',
            'stale',
            'fixture viewer was replaced by another dashboard',
            'take_over',
            'Another dashboard or Guacamole popout is using this remote desktop. Take over to reconnect it here.',
          ), '/taken-over'),
        });
        return { sessionName };
      },
      expected: {
        readinessStatus: 'action_required',
        readinessAction: 'take_over',
        hasTakeoverButton: true,
        textPattern: /take over|another dashboard|guacamole popout/i,
      },
    },
    {
      id: 'browser_unavailable',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('browser');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          health: 'process_exited',
          lastError: 'fixture browser process exited',
          sessionName,
          stream: fixtureStream('browser', null),
        });
        return { sessionName };
      },
      expected: {
        uxState: 'browser_unavailable',
        readinessStatus: 'blocked',
        readinessAction: 'relaunch_browser',
        hasFrame: false,
        textPattern: /browser unavailable|relaunch/i,
      },
    },
    {
      id: 'missing_stream',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('missing-stream');
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: null,
        });
        return { sessionName };
      },
      expected: {
        readinessStatus: 'blocked',
        readinessAction: 'inspect_readiness',
        hasFrame: false,
        textPattern: /no embeddable stream|does not currently report/i,
      },
    },
    {
      id: 'stale_focus_job_recovered',
      evidenceClass: 'fixture-backed',
      seed() {
        const sessionName = scenarioSession('stale-job');
        const staleTabId = `${sessionName}-stale-tab`;
        const liveTabId = `${sessionName}-live-tab`;
        seedFixtureBrowser(context, {
          browserId: `session:${sessionName}`,
          displayName,
          sessionName,
          stream: fixtureStream('stale-job', readiness(
            'focus_job',
            'stale',
            'older view_focus job is still running after a later focus succeeded',
            'inspect_readiness',
            'Inspect retained job history if the stale job remains after the stream is visible.',
          )),
          tabs: [
            {
              id: staleTabId,
              lifecycle: 'closed',
              title: 'about:blank',
              url: 'about:blank',
            },
            {
              id: liveTabId,
              lifecycle: 'ready',
              targetId: 'target-live-stale-job',
              title: 'Recovered Live Tab',
              url: smokeDataUrl('Recovered Live Tab', 'Recovered Live Tab'),
            },
          ],
        });
        return { sessionName, tabId: staleTabId };
      },
      expected: {
        uxState: 'stale_target_recovered',
        readinessStatus: 'ready',
        readinessAction: 'none',
        hasFrame: true,
        textPattern: /recovered stale selected tab identity|queued view focus|failed/i,
      },
    },
  ];

  for (const scenario of scenarios) {
    const { sessionName, tabId = null } = scenario.seed();
    await assertFixtureScenario({
      baseDashboardUrl,
      context,
      evidenceClass: scenario.evidenceClass,
      expected: scenario.expected,
      id: scenario.id,
      matrix,
      sessionName,
      streamPort,
      tabId,
    });
  }

  const summaryPath = writeArtifact('summary.json', {
    artifactDir,
    displayName,
    matrix,
    readinessBaseline: redactReadiness(readinessBaseline),
    remoteViewUrl: redactUrl(remoteConfig.viewStreamUrl),
  });

  await cleanup();
  console.log(`RDP Guacamole readiness failure live smoke passed; summary: ${summaryPath}`);
} catch (err) {
  await fail(err.stack || err.message);
}
