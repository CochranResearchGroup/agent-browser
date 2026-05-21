#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

loadAgentBrowserEnvFromRealHome();

const context = createSmokeContext({
  prefix: 'ab-dashboard-remote-control-ui-',
  sessionPrefix: 'dashboard-remote-control',
});

const { session, tempHome } = context;
const uiSession = `${session}-ui`;
const uiProfile = join(tempHome, 'dashboard-ui-profile');
const serviceName = 'DashboardRemoteControlSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'dashboardRemoteHeadedLaunch';
const closeTaskName = 'dashboardRemoteHeadedClose';
const browserId = `session:${session}`;
const viewStreamProvider = process.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER || 'rdp_gateway';
const controlInputProvider = process.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER || 'manual_attached_desktop';
const viewStreamUrl = process.env.AGENT_BROWSER_REMOTE_VIEW_URL || 'http://agent-browser.localhost/guacamole/';

context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = viewStreamProvider;
context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER = controlInputProvider;
context.env.AGENT_BROWSER_REMOTE_VIEW_URL = viewStreamUrl;

const timeout = setTimeout(() => {
  fail('Timed out waiting for dashboard remote-control UI live smoke to complete');
}, 300000);

let streamPort;
let browserLaunched = false;

function loadAgentBrowserEnvFromRealHome() {
  const realHome = process.env.HOME || '';
  const agentHome = process.env.AGENT_BROWSER_HOME || join(realHome, '.agent-browser');
  const envPath = join(agentHome, '.env');
  if (!existsSync(envPath)) return;

  const text = readFileSync(envPath, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    const value = trimmed.slice(separatorIndex + 1).trim();
    if (!process.env[key]) process.env[key] = value;
  }
}

async function cleanup() {
  clearTimeout(timeout);
  if (streamPort && browserLaunched) {
    try {
      await httpJson(streamPort, 'POST', '/api/service/request', {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: closeTaskName,
        params: { browserId },
        jobTimeoutMs: 30000,
      });
    } catch {
      // The final session close below is the fallback cleanup path.
    }
  }
  try {
    await runCli(context, ['--json', '--session', uiSession, 'close']);
  } catch {
    // The UI browser may not have launched if the smoke failed early.
  }
  await closeSession(context);
  if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
    console.error(`Keeping smoke home: ${tempHome}`);
  } else {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
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

async function evalInDashboard(script, timeoutMs = 60000) {
  const result = await runCli(context, ['--json', '--session', uiSession, 'eval', script], timeoutMs);
  const parsed = parseJsonOutput(result.stdout, 'dashboard eval');
  assert(parsed.success === true, `dashboard eval failed: ${result.stdout}${result.stderr}`);
  return parsed.data?.result;
}

async function waitForDashboardState(predicate, label, timeoutMs = 45000) {
  const started = Date.now();
  let lastState = null;
  while (Date.now() - started < timeoutMs) {
    lastState = await evalInDashboard(dashboardStateScript(browserId), 30000);
    if (predicate(lastState)) return lastState;
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  throw new Error(`${label} did not become true. Last dashboard state: ${JSON.stringify(lastState)}`);
}

function dashboardStateScript(targetBrowserId) {
  return `
(() => {
  const text = document.body?.innerText || "";
  const buttons = Array.from(document.querySelectorAll("button"));
  const serviceButton = buttons.find((button) => button.textContent?.trim() === "Service");
  const rowButton = buttons.find((button) => button.getAttribute("aria-label") === "Inspect browser ${targetBrowserId}");
  const rightPane = document.querySelector(".dashboard-pane-right");
  const rightPaneButtons = rightPane ? Array.from(rightPane.querySelectorAll("button")) : [];
  const controlButton = rightPaneButtons.find((button) => button.textContent?.includes("Open remote control"));
  const readinessStrip = rightPane?.querySelector('[aria-label="Remote view readiness"]');
  const dialog = document.querySelector(".service-view-stream-dialog");
  const frame = dialog?.querySelector("iframe");
  return {
    url: location.href,
    hasServiceButton: Boolean(serviceButton),
    hasTargetBrowserRow: Boolean(rowButton),
    targetBrowserRowText: rowButton?.textContent || "",
    hasRightPane: Boolean(rightPane),
    hasReadinessStrip: Boolean(readinessStrip),
    readinessText: readinessStrip?.textContent || "",
    hasInspectorControlButton: Boolean(controlButton),
    inspectorControlDisabled: controlButton ? (controlButton.disabled || controlButton.getAttribute("aria-disabled") === "true") : null,
    inspectorControlTitle: controlButton?.getAttribute("title") || null,
    hasDialog: Boolean(dialog),
    hasDialogFrame: Boolean(frame),
    dialogFrameSrc: frame?.getAttribute("src") || null,
    hasRemoteViewText: text.includes("Remote view"),
    hasRemoteControlText: text.includes("Remote control"),
  };
})()
`;
}

async function clickDashboardServiceTab() {
  const result = await evalInDashboard(`
(() => {
  const button = Array.from(document.querySelectorAll("button")).find((item) => item.textContent?.trim() === "Service");
  if (!button) return { clicked: false };
  button.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Service tab was not clickable: ${JSON.stringify(result)}`);
}

async function clickTargetBrowserRow() {
  const result = await evalInDashboard(`
(() => {
  const button = Array.from(document.querySelectorAll("button"))
    .find((item) => item.getAttribute("aria-label") === "Inspect browser ${browserId}");
  if (!button) return { clicked: false };
  button.scrollIntoView({ block: "center", inline: "nearest" });
  button.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Target browser row was not clickable: ${JSON.stringify(result)}`);
}

async function clickInspectorRemoteControl() {
  const result = await evalInDashboard(`
(() => {
  const rightPane = document.querySelector(".dashboard-pane-right");
  const button = rightPane
    ? Array.from(rightPane.querySelectorAll("button")).find((item) => item.textContent?.includes("Open remote control"))
    : null;
  if (!button) return { clicked: false, reason: "missing" };
  if (button.disabled || button.getAttribute("aria-disabled") === "true") {
    return { clicked: false, reason: "disabled", title: button.getAttribute("title") };
  }
  button.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Inspector remote-control button was not clickable: ${JSON.stringify(result)}`);
}

async function waitForDashboardFocusJob() {
  const started = Date.now();
  let lastJobs = null;
  while (Date.now() - started < 30000) {
    const jobs = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=30');
    lastJobs = jobs.data?.jobs ?? [];
    const focusJob = lastJobs.find(
      (job) =>
        job.action === 'view_focus' &&
        job.serviceName === 'agent-browser-dashboard' &&
        job.taskName === 'focus-browser-row-view',
    );
    if (focusJob) return focusJob;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Dashboard did not queue a view_focus job. Last jobs: ${JSON.stringify(lastJobs)}`);
}

try {
  streamPort = await ensureStreamPort();

  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    params: {
      browserHost: 'remote_headed',
      headless: false,
      url: smokeDataUrl('Dashboard Remote Control UI Smoke', 'Dashboard Remote Control UI Smoke'),
      waitUntil: 'load',
      viewStreamProvider,
      controlInputProvider,
      viewStreamUrl,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `remote_headed service request failed: ${JSON.stringify(launchResponse)}`);
  browserLaunched = true;

  const dashboardUrl = `http://127.0.0.1:${streamPort}/`;
  const openResult = await runCli(
    context,
    [
      '--json',
      '--session',
      uiSession,
      '--profile',
      uiProfile,
      '--args',
      '--no-sandbox',
      'open',
      dashboardUrl,
    ],
    180000,
  );
  const opened = parseJsonOutput(openResult.stdout, 'dashboard open');
  assert(opened.success === true, `dashboard open failed: ${openResult.stdout}${openResult.stderr}`);
  await runCli(context, ['--json', '--session', uiSession, 'set', 'viewport', '1440', '900']);

  await waitForDashboardState((state) => state?.hasServiceButton, 'dashboard Service navigation');
  await clickDashboardServiceTab();
  await waitForDashboardState((state) => state?.hasTargetBrowserRow, `browser row ${browserId}`);
  await clickTargetBrowserRow();
  await waitForDashboardState(
    (state) =>
      state?.hasRightPane &&
      state?.hasReadinessStrip &&
      state?.hasInspectorControlButton &&
      state?.inspectorControlDisabled === false,
    'right-pane remote-control inspector action',
  );

  await clickInspectorRemoteControl();
  const dialogState = await waitForDashboardState(
    (state) => state?.hasDialog && state?.hasDialogFrame && state?.dialogFrameSrc === viewStreamUrl,
    'remote-control view stream dialog',
  );
  const focusJob = await waitForDashboardFocusJob();

  await cleanup();
  console.log(
    `Service dashboard remote-control UI live smoke passed (${browserId}, dialog=${dialogState.dialogFrameSrc}, job=${focusJob.id})`,
  );
} catch (err) {
  await fail(err.stack || err.message);
}
