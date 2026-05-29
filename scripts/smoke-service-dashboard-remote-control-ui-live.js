#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
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
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');

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
let targetTabId = null;

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

async function waitForDashboardCredentials(timeoutMs = 60000) {
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
      if (username && password) return { username, password };
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Dashboard bootstrap credential file was not ready at ${path}`);
}

async function loginDashboard(credentials) {
  const result = await evalInDashboard(`
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
  assert(result?.ok === true, `dashboard login failed: ${JSON.stringify(result)}`);
}

async function waitForDashboardState(predicate, label, timeoutMs = 45000) {
  const started = Date.now();
  let lastState = null;
  while (Date.now() - started < timeoutMs) {
    lastState = await evalInDashboard(dashboardStateScript(browserId, targetTabId), 30000);
    if (predicate(lastState)) return lastState;
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  throw new Error(`${label} did not become true. Last dashboard state: ${JSON.stringify(lastState)}`);
}

function dashboardStateScript(targetBrowserId, targetServiceTabId) {
  return `
(() => {
  const text = document.body?.innerText || "";
  const buttons = Array.from(document.querySelectorAll("button"));
  const controls = Array.from(document.querySelectorAll("button, a"));
  const serviceButton = controls.find((control) => control.textContent?.trim() === "Service");
  const rowButton = buttons.find((button) => button.getAttribute("aria-label") === "Inspect browser ${targetBrowserId}");
  const workspaceButtons = Array.from(document.querySelectorAll(".service-workspace-tabs button"));
  const sessionsWorkspaceButton = workspaceButtons.find((button) => button.textContent?.includes("Sessions"));
  const tabsWorkspaceButton = workspaceButtons.find((button) => button.textContent?.includes("Tabs"));
  const sessionsFilterInput = document.querySelector('input[placeholder="Filter sessions, profiles, services"], input[placeholder="Filter sessions, tabs, profiles, URLs"]');
  const tabsFilterInput = document.querySelector('input[placeholder="Filter tabs, browsers, URLs"]');
  const targetTabButton = ${JSON.stringify(targetServiceTabId)}
    ? buttons.find((button) => button.getAttribute("aria-label") === "Inspect tab ${targetServiceTabId}")
    : null;
  const targetTabRow = targetTabButton?.closest(".service-browser-row-composite") || null;
  const targetTabControlButton = targetTabRow
    ? Array.from(targetTabRow.querySelectorAll("button")).find((button) => button.textContent?.trim() === "Control")
    : null;
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
    hasSessionsWorkspaceButton: Boolean(sessionsWorkspaceButton),
    hasSessionsFilterInput: Boolean(sessionsFilterInput),
    hasTabsWorkspaceButton: Boolean(tabsWorkspaceButton),
    hasTabsFilterInput: Boolean(tabsFilterInput),
    hasTargetTabRow: Boolean(targetTabButton),
    targetTabRowText: targetTabRow?.textContent || "",
    hasTargetTabControlButton: Boolean(targetTabControlButton),
    targetTabControlDisabled: targetTabControlButton
      ? (targetTabControlButton.disabled || targetTabControlButton.getAttribute("aria-disabled") === "true")
      : null,
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
    textSample: text.replace(/\\s+/g, " ").slice(0, 1200),
  };
})()
`;
}

async function waitForServiceTabRecord(timeoutMs = 30000) {
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
  throw new Error(`Service state did not record a live tab for ${browserId}. Last tabs: ${JSON.stringify(lastTabs)}`);
}

async function clickDashboardServiceTab() {
  const result = await evalInDashboard(`
(() => {
  const button = Array.from(document.querySelectorAll("button, a")).find((item) => item.textContent?.trim() === "Service");
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

async function clickSessionsWorkspaceTab() {
  const result = await evalInDashboard(`
(() => {
  const button = document.querySelector('.service-workspace-tabs button[value="sessions"]')
    || Array.from(document.querySelectorAll(".service-workspace-tabs button"))
      .find((item) => item.textContent?.includes("Sessions"));
  if (!button) return { clicked: false };
  button.scrollIntoView({ block: "center", inline: "nearest" });
  button.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "mouse" }));
  button.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
  button.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
  button.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Sessions workspace tab was not clickable: ${JSON.stringify(result)}`);
}

async function clickTabsWorkspaceTab() {
  const result = await evalInDashboard(`
(() => {
  const button = document.querySelector('.service-workspace-tabs button[value="tabs"]')
    || Array.from(document.querySelectorAll(".service-workspace-tabs button"))
      .find((item) => item.textContent?.includes("Tabs"));
  if (!button) return { clicked: false };
  button.scrollIntoView({ block: "center", inline: "nearest" });
  button.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, pointerType: "mouse" }));
  button.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
  button.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
  button.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Tabs workspace tab was not clickable: ${JSON.stringify(result)}`);
}

async function filterSessionsWorkspace(query) {
  const result = await evalInDashboard(`
(() => {
  const input = document.querySelector('input[placeholder="Filter sessions, profiles, services"], input[placeholder="Filter sessions, tabs, profiles, URLs"]');
  if (!input) return { changed: false, reason: "missing" };
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  input.focus();
  setter?.call(input, ${JSON.stringify(query)});
  input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: ${JSON.stringify(query)} }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
  return { changed: true, value: input.value };
})()
`);
  assert(result?.changed === true, `Sessions workspace filter was not editable: ${JSON.stringify(result)}`);
}

async function filterTabsWorkspace(query) {
  const result = await evalInDashboard(`
(() => {
  const input = document.querySelector('input[placeholder="Filter tabs, browsers, URLs"]');
  if (!input) return { changed: false, reason: "missing" };
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
  input.focus();
  setter?.call(input, ${JSON.stringify(query)});
  input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: ${JSON.stringify(query)} }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
  return { changed: true, value: input.value };
})()
`);
  assert(result?.changed === true, `Tabs workspace filter was not editable: ${JSON.stringify(result)}`);
}

async function clickTargetTabControl() {
  const result = await evalInDashboard(`
(() => {
  const tabButton = Array.from(document.querySelectorAll("button"))
    .find((item) => item.getAttribute("aria-label") === "Inspect tab ${targetTabId}");
  const row = tabButton?.closest(".service-browser-row-composite");
  const controlButton = row
    ? Array.from(row.querySelectorAll("button")).find((item) => item.textContent?.trim() === "Control")
    : null;
  if (!controlButton) return { clicked: false, reason: "missing" };
  if (controlButton.disabled || controlButton.getAttribute("aria-disabled") === "true") {
    return { clicked: false, reason: "disabled", title: controlButton.getAttribute("title") };
  }
  controlButton.scrollIntoView({ block: "center", inline: "nearest" });
  controlButton.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Target tab Control button was not clickable: ${JSON.stringify(result)}`);
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

async function closeViewStreamDialog() {
  const result = await evalInDashboard(`
(() => {
  const dialog = document.querySelector(".service-view-stream-dialog");
  const closeButton = dialog?.querySelector('[data-slot="dialog-close"]');
  if (!closeButton) return { clicked: false };
  closeButton.click();
  return { clicked: true };
})()
`);
  assert(result?.clicked === true, `Remote-control dialog close button was not clickable: ${JSON.stringify(result)}`);
}

async function waitForDashboardFocusJob(taskName) {
  const started = Date.now();
  let lastJobs = null;
  while (Date.now() - started < 30000) {
    const jobs = await httpJson(streamPort, 'GET', '/api/service/jobs?limit=30');
    lastJobs = jobs.data?.jobs ?? [];
    const focusJob = lastJobs.find(
      (job) =>
        job.action === 'view_focus' &&
        job.serviceName === 'agent-browser-dashboard' &&
        job.taskName === taskName,
    );
    if (focusJob) return focusJob;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Dashboard did not queue ${taskName} view_focus job. Last jobs: ${JSON.stringify(lastJobs)}`);
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
  const serviceTab = await waitForServiceTabRecord();
  targetTabId = serviceTab.id;

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
  const dashboardCredentials = await waitForDashboardCredentials();
  await loginDashboard(dashboardCredentials);

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
  const focusJob = await waitForDashboardFocusJob('focus-browser-row-view');
  await closeViewStreamDialog();
  await waitForDashboardState((state) => !state?.hasDialog, 'closed browser remote-control dialog');

  await waitForDashboardState((state) => state?.hasSessionsWorkspaceButton, 'Sessions workspace navigation');
  await clickSessionsWorkspaceTab();
  await waitForDashboardState((state) => state?.hasSessionsFilterInput, 'Sessions workspace filter');
  await filterSessionsWorkspace(targetTabId);
  await waitForDashboardState((state) => state?.hasTabsWorkspaceButton, 'Tabs workspace navigation');
  await clickTabsWorkspaceTab();
  await waitForDashboardState((state) => state?.hasTabsFilterInput, 'Tabs workspace filter');
  await filterTabsWorkspace(targetTabId);
  await waitForDashboardState(
    (state) =>
      state?.hasTargetTabRow &&
      state?.hasTargetTabControlButton &&
      state?.targetTabControlDisabled === false,
    `tab row ${targetTabId} Control action`,
  );
  await clickTargetTabControl();
  const tabDialogState = await waitForDashboardState(
    (state) => state?.hasDialog && state?.hasDialogFrame && state?.dialogFrameSrc === remoteConfig.viewStreamUrl,
    'tab remote-control view stream dialog',
  );
  const tabFocusJob = await waitForDashboardFocusJob('inspect-hidden-rdp-tab');

  await cleanup();
  console.log(
    `Service dashboard remote-control UI live smoke passed (${browserId}, tab=${targetTabId}, dialog=${dialogState.dialogFrameSrc}, tabDialog=${tabDialogState.dialogFrameSrc}, jobs=${focusJob.id},${tabFocusJob.id})`,
  );
} catch (err) {
  await fail(err.stack || err.message);
}
