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
  ensureStreamPort,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacViewerTransferSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacViewerTransferLaunch';
const closeTaskName = 'rdpGuacViewerTransferClose';
const dashboardTakeoverTaskName = 'workspace-viewport-takeover';
const clientASession = 'rdp-guac-client-a';
const clientBSession = 'rdp-guac-client-b';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-hardening-${timestamp}`);

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

function configuredSessionName() {
  const browser = envValue('AGENT_BROWSER_RDP_TEST_BROWSER_A');
  if (!browser) return undefined;
  return browser.startsWith('session:') ? browser.slice('session:'.length) : browser;
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

function dashboardUrl(baseUrl, browserId, tabId) {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', `browser:${browserId}`);
  url.searchParams.set('browser', browserId);
  if (tabId) url.searchParams.set('tab', tabId);
  return url.toString();
}

function dashboardStateScript() {
  return `
(() => {
  const text = document.body?.innerText || "";
  const viewport = document.querySelector(".workspace-remote-viewport");
  const stage = viewport?.querySelector(".workspace-remote-viewport-stage");
  const frame = viewport?.querySelector("iframe");
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
    uxState: viewport?.getAttribute("data-ux-state") || null,
    hasViewport: Boolean(viewport),
    hasFrame: Boolean(frame),
    frameSrc: frame?.getAttribute("src") || null,
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

async function waitForServiceTabRecord(streamPort, browserId, label, timeoutMs = 30000) {
  const started = Date.now();
  let lastTabs = null;
  while (Date.now() - started < timeoutMs) {
    const status = await httpJson(streamPort, 'GET', '/api/service/status');
    assert(status.success === true, `HTTP service status failed: ${JSON.stringify(status)}`);
    lastTabs = Object.values(status.data?.service_state?.tabs || {});
    const tab = lastTabs.find((item) => item?.browserId === browserId && item?.lifecycle !== 'closed');
    if (tab?.id) return tab;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not record a live tab for ${browserId}. Last tabs: ${JSON.stringify(lastTabs)}`);
}

async function clickTakeoverIfAvailable(context, session) {
  return evalInClient(context, session, `
(() => {
  const button = Array.from(document.querySelectorAll("button"))
    .find((item) => item.textContent?.trim() === "Take over");
  if (!button) return { clicked: false, reason: "missing" };
  if (button.disabled || button.getAttribute("aria-disabled") === "true") {
    return { clicked: false, reason: "disabled" };
  }
  button.scrollIntoView({ block: "center", inline: "nearest" });
  button.click();
  return { clicked: true };
})()
`);
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

async function refreshClient(context, session) {
  await evalInClient(context, session, 'location.reload(); ({ reloading: true, url: location.href })', 30000);
}

async function waitForTakeoverJob(streamPort, ignoredIds, label, timeoutMs = 30000) {
  const started = Date.now();
  let lastJobs = null;
  while (Date.now() - started < timeoutMs) {
    const jobsResponse = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=50');
    lastJobs = jobsResponse.data?.jobs ?? [];
    const job = lastJobs.find(
      (item) =>
        item.action === 'view_takeover' &&
        item.taskName === dashboardTakeoverTaskName &&
        !ignoredIds.has(item.id),
    );
    if (job) return job;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not queue a new view_takeover job. Last jobs: ${JSON.stringify(lastJobs)}`);
}

async function currentTakeoverJobIds(streamPort) {
  const jobsResponse = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=50');
  const jobs = jobsResponse.data?.jobs ?? [];
  return new Set(jobs.filter((item) => item.action === 'view_takeover').map((item) => item.id));
}

function classifyViewerOutcome(client1, client2) {
  if (client1?.hasTakeoverButton || client1?.textIncludesTakenOver) return 'client_2_took_over';
  if (client2?.hasTakeoverButton || client2?.textIncludesTakenOver) return 'client_1_retained_ownership';
  if (client1?.hasFrame && client2?.hasFrame) return 'simultaneous_view';
  return 'unclassified';
}

const timeout = setTimeout(() => {
  fail('Timed out waiting for RDP and Guacamole viewer-transfer live smoke to complete');
}, 420000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-transfer-',
  session: configuredSessionName(),
  sessionPrefix: 'rdp-guac-transfer',
});
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');

let profileA;
let profileB;
let clientAExecutable;
let clientBExecutable;
let displayIsolation;
let remoteConfig;

try {
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

const { session } = context;
const browserId = `session:${session}`;
let streamPort;
let browserLaunched = false;

async function cleanup() {
  clearTimeout(timeout);
  for (const clientSession of [clientASession, clientBSession]) {
    try {
      await runCli(context, ['--json', '--session', clientSession, 'close'], 30000);
    } catch {
      // Client cleanup is best-effort because failed launches leave no session.
    }
  }
  await closeRemoteHeadedBrowser({
    agentName,
    browserId: browserLaunched ? browserId : null,
    context,
    serviceName,
    streamPort,
    taskName: closeTaskName,
  });
  cleanupSmokeHome(context);
}

async function fail(message) {
  console.error(message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}

try {
  streamPort = await ensureStreamPort(context, 180000);
  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    params: {
      browserHost: 'remote_headed',
      displayIsolation,
      headless: false,
      remoteHeadedDisplay: envValue('AGENT_BROWSER_REMOTE_HEADED_DISPLAY'),
      url: smokeDataUrl('RDP Guac Viewer Transfer A', 'RDP Guac Viewer Transfer A'),
      waitUntil: 'load',
      viewStreamProvider: remoteConfig.viewStreamProvider,
      controlInputProvider: remoteConfig.controlInputProvider,
      viewStreamUrl: remoteConfig.viewStreamUrl,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `remote_headed RDP launch failed: ${JSON.stringify(launchResponse)}`);
  browserLaunched = true;
  writeArtifact('launch-response.json', launchResponse);

  const serviceTab = await waitForServiceTabRecord(streamPort, browserId, 'RDP Guac launch');
  await serviceStatusArtifact(streamPort, 'before-client-1');

  const baseDashboardUrl = envValue('AGENT_BROWSER_RDP_TEST_PUBLIC_URL') || `http://127.0.0.1:${streamPort}/`;
  const workspaceUrl = dashboardUrl(baseDashboardUrl, browserId, serviceTab.id);
  writeArtifact('fixture.json', {
    artifactDir,
    browserId,
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
    serviceSession: session,
    serviceTabId: serviceTab.id,
    streamPort,
    workspaceUrl,
  });

  await openDashboardClient(context, {
    executable: clientAExecutable,
    profile: profileA,
    session: clientASession,
    url: workspaceUrl,
    viewport: { width: 1440, height: 900 },
  });
  const dashboardCredentials = await waitForDashboardCredentials(context);
  await loginDashboardClient(context, clientASession, dashboardCredentials);
  const client1Connected = await waitForClientState(
    context,
    clientASession,
    (state) => state?.hasViewport && state?.hasFrame && state?.hasInteractionButton && state?.hasFullscreenButton,
    'client 1 connected workspace viewport',
  );
  writeArtifact('client-1-connected.json', client1Connected);
  await screenshotClient(context, clientASession, 'client-1-connected');

  await openDashboardClient(context, {
    executable: clientBExecutable,
    profile: profileB,
    session: clientBSession,
    url: workspaceUrl,
    viewport: { width: 1366, height: 860 },
  });
  await loginDashboardClient(context, clientBSession, dashboardCredentials);
  const client2Connected = await waitForClientState(
    context,
    clientBSession,
    (state) => state?.hasViewport && state?.hasFrame,
    'client 2 connected workspace viewport',
  );
  await new Promise((resolve) => setTimeout(resolve, 3000));
  const client1AfterClient2 = await evalInClient(context, clientASession, dashboardStateScript());
  const client2AfterClient2 = await evalInClient(context, clientBSession, dashboardStateScript());
  const firstOutcome = classifyViewerOutcome(client1AfterClient2, client2AfterClient2);
  assert(firstOutcome !== 'unclassified', `Could not classify two-client viewer behavior: ${JSON.stringify({ client1AfterClient2, client2AfterClient2 })}`);
  writeArtifact('after-client-2-open.json', {
    client1: client1AfterClient2,
    client2: client2AfterClient2,
    outcome: firstOutcome,
  });
  await screenshotClient(context, clientASession, 'client-1-after-client-2-open');
  await screenshotClient(context, clientBSession, 'client-2-connected');
  await serviceStatusArtifact(streamPort, 'after-client-2-open');

  let client1TakeoverJob = null;
  if (client1AfterClient2.hasTakeoverButton) {
    const beforeJobIds = await currentTakeoverJobIds(streamPort);
    const clicked = await clickTakeoverIfAvailable(context, clientASession);
    assert(clicked?.clicked === true, `Client 1 Take over button was not clickable: ${JSON.stringify(clicked)}`);
    client1TakeoverJob = await waitForTakeoverJob(streamPort, beforeJobIds, 'client 1 takeover');
    await waitForClientState(context, clientASession, (state) => state?.hasViewport && state?.hasFrame, 'client 1 after takeover');
    await new Promise((resolve) => setTimeout(resolve, 3000));
  }

  const client1AfterTakeover = await evalInClient(context, clientASession, dashboardStateScript());
  const client2AfterTakeover = await evalInClient(context, clientBSession, dashboardStateScript());
  writeArtifact('after-client-1-takeover.json', {
    client1: client1AfterTakeover,
    client2: client2AfterTakeover,
    takeoverJob: client1TakeoverJob,
  });
  await screenshotClient(context, clientASession, 'client-1-after-takeover');
  await screenshotClient(context, clientBSession, 'client-2-after-client-1-takeover');
  await serviceStatusArtifact(streamPort, 'after-client-1-takeover');

  const beforeExternalJobIds = await currentTakeoverJobIds(streamPort);
  const externalResult = await clickExternalOpen(context, clientASession);
  assert(externalResult?.clicked === true, `External open button was not clickable: ${JSON.stringify(externalResult)}`);
  const externalJob = await waitForTakeoverJob(streamPort, beforeExternalJobIds, 'external open takeover');
  writeArtifact('external-open-takeover-job.json', externalJob);

  await runCli(context, ['--json', '--session', clientASession, 'set', 'viewport', '390', '844']);
  await waitForClientState(
    context,
    clientASession,
    (state) => state?.hasViewport && state?.hasFrame && state?.hasInteractionButton,
    'mobile-size client 1 workspace viewport',
  );
  await screenshotClient(context, clientASession, 'client-1-mobile-viewport');

  await refreshClient(context, clientASession);
  await waitForClientState(context, clientASession, (state) => state?.hasViewport && state?.hasFrame, 'client 1 refresh recovery');
  await screenshotClient(context, clientASession, 'client-1-after-refresh');
  await serviceStatusArtifact(streamPort, 'after-client-1-refresh');

  await refreshClient(context, clientBSession);
  await waitForClientState(context, clientBSession, (state) => state?.hasViewport && state?.hasFrame, 'client 2 refresh recovery');
  await screenshotClient(context, clientBSession, 'client-2-after-refresh');
  const finalStatus = await serviceStatusArtifact(streamPort, 'after-client-2-refresh');

  const finalBrowser = finalStatus.data?.service_state?.browsers?.[browserId];
  assert(finalBrowser, `Final service state missing browser ${browserId}: ${JSON.stringify(finalStatus.data)}`);
  assert(
    finalBrowser.health === 'ready',
    `Browser did not remain ready after viewer transfer and refresh: ${JSON.stringify(finalBrowser)}`,
  );

  writeArtifact('summary.json', {
    artifactDir,
    browserId,
    client1TakeoverJobId: client1TakeoverJob?.id ?? null,
    displayIsolation,
    externalOpenTakeoverJobId: externalJob.id,
    outcome: firstOutcome,
    route: {
      url: remoteConfig.viewStreamUrl,
      frameUrl: remoteConfig.frameUrl,
      externalUrl: remoteConfig.externalUrl,
      routeId: remoteConfig.routeId,
      connectionId: remoteConfig.connectionId,
      connectionName: remoteConfig.connectionName,
    },
    serviceTabId: serviceTab.id,
    workspaceUrl,
  });

  await cleanup();
  console.log(`RDP Guacamole viewer-transfer live smoke passed (${firstOutcome}); artifacts: ${artifactDir}`);
} catch (err) {
  await fail(err.stack || err.message);
}
