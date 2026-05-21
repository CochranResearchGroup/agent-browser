#!/usr/bin/env node

import { join } from 'node:path';

import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  closeRemoteHeadedBrowser,
  configureRemoteHeadedContext,
  ensureStreamPort,
  launchRemoteHeadedBrowser,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

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
const remoteConfig = configureRemoteHeadedContext(context);

const timeout = setTimeout(() => {
  fail('Timed out waiting for dashboard remote-control UI live smoke to complete');
}, 300000);

let streamPort;
let browserLaunched = false;

async function cleanup() {
  clearTimeout(timeout);
  try {
    await runCli(context, ['--json', '--session', uiSession, 'close']);
  } catch {
    // The UI browser may not have launched if the smoke failed early.
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
  await cleanup();
  process.exit(1);
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
  streamPort = await ensureStreamPort(context);
  await launchRemoteHeadedBrowser({
    agentName,
    config: remoteConfig,
    context,
    heading: 'Dashboard Remote Control UI Smoke',
    serviceName,
    streamPort,
    taskName: launchTaskName,
    title: 'Dashboard Remote Control UI Smoke',
  });
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
    (state) => state?.hasDialog && state?.hasDialogFrame && state?.dialogFrameSrc === remoteConfig.viewStreamUrl,
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
