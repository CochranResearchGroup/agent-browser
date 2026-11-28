#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { basename } from 'node:path';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  captureDashboardScreenshot,
  cdpEvaluate,
  clickDashboardRefresh as clickViewerClientDashboardRefresh,
  closeViewerClients,
  dashboardWorkspaceUrl as viewerClientDashboardWorkspaceUrl,
  launchDashboardViewerClient,
  navigateDashboardViewerClient,
  reconnectDashboardViewerClient,
  resolveViewerClientExecutable,
  waitForDevToolsActivePort,
  waitForDashboardViewerClientPageUrl,
  waitForDashboardState as waitForViewerClientDashboardState,
  waitForJson,
} from './lib/p47-viewer-client.js';
import {
  classifyScenarioFailure,
  routeBoundFinalizationEvidence,
  scenarioSpec,
  supportedScenarioIds,
  validateScenarioSpec,
} from './lib/p46-scenario-harness.js';

const args = process.argv.slice(2);
const scenario = valueFor('--scenario') || 's0';
const resetBefore = args.includes('--reset-before');
const resetAfter = args.includes('--reset-after');
const artifactDir = valueFor('--artifact-dir') ||
  join(tmpdir(), `agent-browser-p46-${scenario}-${new Date().toISOString().replace(/[:.]/g, '-')}`);
const agentBrowserCommandArg = valueFor('--agent-browser-command');
const explicitAgentBrowserCommand = agentBrowserCommandArg || process.env.AGENT_BROWSER_COMMAND || null;
const requireExplicitAgentBrowserCommand = args.includes('--require-explicit-agent-browser-command');
const requireAgentBrowserDaemonCommandMatch = args.includes('--require-agent-browser-daemon-command-match');
const allowS4DuplicateProfileLane = args.includes('--allow-s4-duplicate-profile-lane');
const agentBrowser = explicitAgentBrowserCommand || 'agent-browser';
const defaultCommandMaxBuffer = 32 * 1024 * 1024;
const s12CycleCount = Math.max(10, Number.parseInt(valueFor('--cycles') || '10', 10) || 10);

mkdirSync(artifactDir, { recursive: true });

function valueFor(name) {
  const index = args.indexOf(name);
  if (index === -1) return null;
  return args[index + 1] || null;
}

function writeJson(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function writeText(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, value);
  return path;
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
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
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    values[key] = value.replace(/\\"/g, '"');
  }
  return values;
}

function loadDashboardCredentials() {
  const authEnvPath = process.env.AGENT_BROWSER_DASHBOARD_AUTH_ENV ||
    join(process.env.HOME || '', '.agent-browser', 'dashboard-auth.env');
  if (!existsSync(authEnvPath)) {
    return {
      ok: false,
      path: authEnvPath,
      reason: 'dashboard auth env file is missing',
    };
  }
  const values = parseEnvText(readFileSync(authEnvPath, 'utf8'));
  const username = values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME;
  const password = values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
  return {
    ok: Boolean(username && password),
    path: authEnvPath,
    reason: username && password ? null : 'dashboard auth env file does not contain usable credentials',
    username,
    password,
  };
}

function run(command, commandArgs, label, {
  timeoutMs = 120000,
  allowFailure = true,
  env = {},
  maxBuffer = defaultCommandMaxBuffer,
} = {}) {
  const startedAt = new Date().toISOString();
  const result = spawnSync(command, commandArgs, {
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env,
    },
    stdio: 'pipe',
    timeout: timeoutMs,
    maxBuffer,
  });
  const record = {
    command,
    args: commandArgs,
    label,
    status: result.status,
    signal: result.signal,
    error: result.error ? String(result.error.message || result.error) : null,
    stdout: result.stdout,
    stderr: result.stderr,
    startedAt,
    finishedAt: new Date().toISOString(),
  };
  if (!allowFailure && result.status !== 0) {
    throw new Error(`${label} failed: ${command} ${commandArgs.join(' ')}\n${result.stdout}${result.stderr}`);
  }
  return record;
}

function runJson(command, commandArgs, label, options = {}) {
  const record = run(command, commandArgs, label, options);
  const text = record.stdout.trim();
  try {
    record.json = text ? JSON.parse(text) : null;
  } catch (error) {
    record.parseError = String(error.message || error);
    record.json = null;
  }
  return record;
}

function agentBrowserCommandInfo() {
  const source = agentBrowserCommandArg
    ? 'flag'
    : process.env.AGENT_BROWSER_COMMAND
      ? 'environment'
      : 'path-default';
  const which = agentBrowser.includes('/')
    ? null
    : run('sh', ['-lc', `command -v ${shellQuote(agentBrowser)}`], 'resolve agent-browser command', {
      timeoutMs: 10000,
    });
  const commandPath = which?.stdout?.trim() || agentBrowser;
  let realpath = null;
  let realpathError = null;
  try {
    if (commandPath && existsSync(commandPath)) realpath = realpathSync(commandPath);
  } catch (error) {
    realpathError = String(error.message || error);
  }
  const version = run(agentBrowser, ['--version'], 'agent-browser version', {
    timeoutMs: 10000,
  });
  const daemon = agentBrowserDaemonInfo(realpath);
  return {
    command: agentBrowser,
    commandPath,
    daemon,
    explicit: Boolean(explicitAgentBrowserCommand),
    realpath,
    realpathError,
    requireDaemonCommandMatch: requireAgentBrowserDaemonCommandMatch,
    requireExplicit: requireExplicitAgentBrowserCommand,
    source,
    version: {
      status: version.status,
      stdout: version.stdout.trim(),
      stderr: version.stderr.trim(),
      error: version.error,
    },
    which: which
      ? {
        status: which.status,
        stdout: which.stdout.trim(),
        stderr: which.stderr.trim(),
        error: which.error,
      }
      : null,
  };
}

function agentBrowserDaemonInfo(expectedRealpath) {
  const uid = typeof process.getuid === 'function' ? process.getuid() : null;
  const socketDir = uid === null ? null : `/run/user/${uid}/agent-browser`;
  const ss = socketDir
    ? run('sh', ['-lc', `ss -xlpn | grep ${shellQuote(socketDir)} || true`], 'agent-browser daemon socket listeners', {
      timeoutMs: 10000,
    })
    : null;
  const listenerKeys = new Set();
  const listenerRows = [];
  for (const line of (ss?.stdout || '').split(/\r?\n/)) {
    const socketPath = line.split(/\s+/).find((part) => part.startsWith(socketDir));
    if (!socketPath) continue;
    for (const match of line.matchAll(/pid=(\d+)/g)) {
      const pid = match[1];
      const key = `${pid}:${socketPath}`;
      if (listenerKeys.has(key)) continue;
      listenerKeys.add(key);
      listenerRows.push({ pid, socketPath });
    }
  }
  listenerRows.sort((a, b) => Number(a.pid) - Number(b.pid) || a.socketPath.localeCompare(b.socketPath));
  const listeners = listenerRows.map(({ pid, socketPath }) => {
    const exe = run('readlink', ['-f', `/proc/${pid}/exe`], `daemon ${pid} exe`, { timeoutMs: 5000 });
    const cwd = run('readlink', ['-f', `/proc/${pid}/cwd`], `daemon ${pid} cwd`, { timeoutMs: 5000 });
    const cmdline = run('sh', ['-lc', `tr '\\0' ' ' </proc/${pid}/cmdline`], `daemon ${pid} cmdline`, {
      timeoutMs: 5000,
    });
    return {
      cmdline: cmdline.stdout.trim(),
      cwd: cwd.stdout.trim(),
      exe: exe.stdout.trim(),
      matchesExpectedRealpath: Boolean(expectedRealpath && exe.stdout.trim() === expectedRealpath),
      pid: Number(pid),
      socketPath,
    };
  });
  const matchingListeners = listeners.filter((listener) => listener.matchesExpectedRealpath);
  const defaultSocketPath = socketDir ? `${socketDir}/default.sock` : null;
  const defaultSocketListeners = listeners.filter((listener) => listener.socketPath === defaultSocketPath);
  const matchingDefaultSocketListeners = defaultSocketListeners.filter((listener) => listener.matchesExpectedRealpath);
  return {
    defaultSocketListenerCount: defaultSocketListeners.length,
    defaultSocketMatchingListenerCount: matchingDefaultSocketListeners.length,
    defaultSocketPath,
    expectedRealpath,
    listenerCount: listeners.length,
    listeners,
    matchingListenerCount: matchingListeners.length,
    singleMatchingListener:
      defaultSocketListeners.length === 1 && matchingDefaultSocketListeners.length === 1,
    noListeners: listeners.length === 0,
    socketDir,
    ss: ss
      ? {
        status: ss.status,
        stdout: ss.stdout.trim(),
        stderr: ss.stderr.trim(),
        error: ss.error,
      }
      : null,
  };
}

function runAgentJson(commandArgs, label, options = {}) {
  return runJson(agentBrowser, ['--json', ...commandArgs], label, options);
}

function runAgentSessionJson(sessionName, commandArgs, label, options = {}) {
  return runJson(agentBrowser, ['--json', '--session', sessionName, ...commandArgs], label, options);
}

function curlRecord(url, label) {
  return run('curl', [
    '--insecure',
    '--location',
    '--silent',
    '--show-error',
    '--output',
    '/dev/null',
    '--write-out',
    '%{http_code} %{url_effective}',
    '--max-time',
    '12',
    url,
  ], label, { timeoutMs: 20000 });
}

function commandExists(command) {
  const result = run('sh', ['-lc', `command -v ${command}`], `find ${command}`);
  return result.status === 0 ? result.stdout.trim() : null;
}

function readServiceSessions(statusJson) {
  const sessions = statusJson?.data?.service_state?.sessions || {};
  const sessionIds = Object.values(sessions)
    .map((session) => session?.id)
    .filter((id) => typeof id === 'string' && id.length > 0);
  const browsers = statusJson?.data?.service_state?.browsers || {};
  const browserSessionIds = Object.values(browsers)
    .flatMap((browser) => {
      const activeSessionIds = Array.isArray(browser?.activeSessionIds) ? browser.activeSessionIds : [];
      const idSession = typeof browser?.id === 'string' && browser.id.startsWith('session:')
        ? [browser.id.slice('session:'.length)]
        : [];
      return [...activeSessionIds, ...idSession];
    })
    .filter((id) => typeof id === 'string' && id.length > 0);
  return Array.from(new Set([...sessionIds, ...browserSessionIds])).sort();
}

function activeIncidentCount(statusJson) {
  const incidents = statusJson?.data?.service_state?.incidents || {};
  return Object.values(incidents).filter((incident) => incident?.state === 'active').length;
}

function displayHasTerminal(display) {
  const windows = display?.displayContent?.windows || [];
  return windows.some((window) => /xterm|terminal|shell/i.test(`${window.title || ''}\n${window.raw || ''}`));
}

function displayStates(displayReport) {
  return ['A', 'B'].map((route) => {
    const display = displayReport?.routes?.[route];
    return {
      route,
      displayName: display?.displayName || null,
      state: display?.displayContent?.state || null,
      terminalVisible: displayHasTerminal(display),
      windowTitles: (display?.displayContent?.windows || []).map((window) => window.title),
    };
  });
}

function responseUrl(record) {
  const data = record?.json?.data;
  if (typeof data === 'string') return data;
  if (typeof data?.url === 'string') return data.url;
  if (typeof data?.value === 'string') return data.value;
  return null;
}

function serviceTabIdFromCommandData(data) {
  const tab = data?.tab || data;
  const handleTabId = tab?.serviceTabHandle?.tabId;
  if (typeof handleTabId === 'string' && handleTabId.length > 0) return handleTabId;
  const tabId = tab?.tabId || tab?.targetId || tab?.id;
  return typeof tabId === 'string' && tabId.length > 0 ? tabId : null;
}

function tabMatchesCommandData(tab, data) {
  const commandTab = data?.tab || data;
  if (!tab || !commandTab) return false;
  const tabId = serviceTabIdFromTab(tab);
  const commandTabId = serviceTabIdFromCommandData(commandTab);
  if (tabId && commandTabId && tabId === commandTabId) return true;
  const sameIndex = Number.isInteger(tab?.index) && tab.index === commandTab.index;
  const sameUrl = typeof tab?.url === 'string' && tab.url === commandTab.url;
  return sameIndex && sameUrl;
}

function serviceTabIdFromOpen(openJson) {
  return serviceTabIdFromCommandData(openJson?.data);
}

function tabsFromRecord(record) {
  const data = record?.json?.data;
  if (Array.isArray(data)) return data;
  if (Array.isArray(data?.tabs)) return data.tabs;
  return [];
}

function serviceTabIdFromTab(tab) {
  const candidates = [
    tab?.serviceTabHandle?.tabId,
    tab?.serviceTabId,
    tab?.tabId,
    tab?.targetId,
    tab?.id,
  ];
  return candidates.find((value) => typeof value === 'string' && value.length > 0) || null;
}

function tabSelector(tab, fallbackIndex) {
  const index = tab?.index ?? tab?.position ?? fallbackIndex;
  return String(index);
}

function routePoolEntries(routePoolReport) {
  const entries = routePoolReport?.routePoolJson;
  if (Array.isArray(entries)) return entries;
  if (typeof entries === 'string') {
    try {
      return JSON.parse(entries);
    } catch {
      return [];
    }
  }
  return [];
}

function resetRuntime(phase) {
  const artifacts = {};
  const status = runAgentJson(['service', 'status'], `${phase} service status before reset`, {
    timeoutMs: 60000,
  });
  artifacts[`${phase}-service-status-before-reset.json`] = writeJson(`${phase}-service-status-before-reset.json`, status);
  const sessions = readServiceSessions(status.json);
  const closeResults = [];
  for (const sessionId of sessions) {
    closeResults.push(runJson(agentBrowser, ['--json', '--session', sessionId, 'close'], `${phase} close ${sessionId}`, {
      timeoutMs: 60000,
    }));
  }
  artifacts[`${phase}-close-results.json`] = writeJson(`${phase}-close-results.json`, closeResults);
  const after = runAgentJson(['service', 'status'], `${phase} service status after reset`, {
    timeoutMs: 60000,
  });
  artifacts[`${phase}-service-status-after-reset.json`] = writeJson(`${phase}-service-status-after-reset.json`, after);
  return {
    sessionsClosed: sessions,
    activeIncidentsBefore: activeIncidentCount(status.json),
    activeIncidentsAfter: activeIncidentCount(after.json),
    artifacts,
  };
}

function captureBaseline(label) {
  const artifacts = {};
  const installDoctor = runAgentJson(['install', 'doctor'], `${label} install doctor`, {
    timeoutMs: 60000,
  });
  artifacts['install-doctor.json'] = writeJson('install-doctor.json', installDoctor);

  const remoteViewDoctor = runAgentJson(['doctor', 'remote-view'], `${label} remote-view doctor`, {
    timeoutMs: 60000,
  });
  artifacts['remote-view-doctor.json'] = writeJson('remote-view-doctor.json', remoteViewDoctor);

  const serviceStatus = runAgentJson(['service', 'status'], `${label} service status`, {
    timeoutMs: 60000,
  });
  artifacts['service-status.json'] = writeJson('service-status.json', serviceStatus);

  const incidents = runAgentJson(['service', 'incidents', '--summary'], `${label} incident summary`, {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-summary.json'] = writeJson('service-incidents-summary.json', incidents);

  const routePool = runJson(process.execPath, ['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], `${label} route pool`, {
    timeoutMs: 60000,
  });
  artifacts['route-pool-readiness.json'] = writeJson('route-pool-readiness.json', routePool);

  const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], `${label} route display content`, {
    timeoutMs: 60000,
  });
  artifacts['display-content.json'] = writeJson('display-content.json', displayContent);

  const gcDryRun = runAgentJson(['service', 'gc', '--dry-run'], `${label} service gc dry run`, {
    timeoutMs: 60000,
  });
  artifacts['service-gc-dry-run.json'] = writeJson('service-gc-dry-run.json', gcDryRun);

  const pruneDryRun = runAgentJson(['service', 'prune-retained', '--dry-run'], `${label} prune retained dry run`, {
    timeoutMs: 60000,
  });
  artifacts['service-prune-retained-dry-run.json'] = writeJson('service-prune-retained-dry-run.json', pruneDryRun);

  const dashboard = curlRecord('http://127.0.0.1:4848/', `${label} dashboard http`);
  artifacts['dashboard-http.txt'] = writeText('dashboard-http.txt', `${dashboard.stdout}\n${dashboard.stderr}`);

  const guacamole = curlRecord('https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=', `${label} public guacamole route`);
  artifacts['guacamole-route-http.txt'] = writeText('guacamole-route-http.txt', `${guacamole.stdout}\n${guacamole.stderr}`);

  const importCommand = commandExists('import');
  const screenshots = [];
  if (importCommand && displayContent.json?.success === true) {
    for (const route of ['A', 'B']) {
      const displayName = displayContent.json?.routes?.[route]?.displayName;
      if (!displayName) continue;
      const path = join(artifactDir, `route-${route.toLowerCase()}-root.png`);
      const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], `${label} route ${route} screenshot`, {
        timeoutMs: 60000,
      });
      screenshots.push({
        route,
        displayName,
        path,
        status: screenshot.status,
        stderr: screenshot.stderr,
      });
    }
  }
  artifacts['route-display-screenshots.json'] = writeJson('route-display-screenshots.json', screenshots);

  return {
    artifacts,
    dashboard,
    displayContent,
    gcDryRun,
    guacamole,
    incidents,
    installDoctor,
    pruneDryRun,
    remoteViewDoctor,
    routePool,
    screenshots,
    serviceStatus,
  };
}

function routePoolEnv(routePoolReport) {
  const entries = routePoolEntries(routePoolReport);
  return {
    AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(entries),
  };
}

function routePoolEnvFromServiceStatus(statusJson) {
  const entries = Object.values(statusJson?.data?.service_state?.routePool || {});
  return {
    AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(entries),
  };
}

function pressureSnapshot(statusJson) {
  const serviceState = statusJson?.data?.service_state || {};
  const sessions = Object.values(serviceState.sessions || {});
  const browsers = Object.values(serviceState.browsers || {});
  const tabs = browsers.flatMap((browser) => {
    if (Array.isArray(browser?.tabs)) return browser.tabs;
    if (Array.isArray(browser?.targets)) return browser.targets;
    return [];
  });
  const routePool = Object.values(serviceState.routePool || {});
  return {
    browsers: browsers.length,
    checkedOutRoutePool: routePool.filter((entry) =>
      entry?.state === 'checked_out' || Boolean(entry?.currentRouteAllocationId)
    ).length,
    displayAllocations: Object.values(serviceState.displayAllocations || {})
      .filter((allocation) => !['released', 'failed'].includes(allocation?.state)).length,
    remoteViewAcquisitionLeases: Object.values(serviceState.remoteViewAcquisitionLeases || {})
      .filter((lease) =>
        !['completed', 'failed', 'released'].includes(lease?.state) &&
        !['checked_out', 'rollback_complete'].includes(lease?.phase)
      ).length,
    remoteViewRoutes: Object.values(serviceState.remoteViewRoutes || {})
      .filter((route) => !['released', 'failed'].includes(route?.state)).length,
    sessions: sessions.length,
    tabs: tabs.length,
  };
}

function pressureWithinBaseline(after, before) {
  return Object.entries(after).filter(([key, value]) => value > (before?.[key] ?? 0));
}

function routePoolReturnedToBaseline(statusJson) {
  const entries = Object.values(statusJson?.data?.service_state?.routePool || {});
  return entries.length > 0 && entries.every((entry) =>
    ['available', 'ready'].includes(entry?.state) && !entry?.currentRouteAllocationId
  );
}

function captureS12Boundary(label, artifacts) {
  const installDoctor = runAgentJson(['install', 'doctor'], `${label} install doctor`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-install-doctor.json`] = writeJson(`${label}-install-doctor.json`, installDoctor);
  const remoteViewDoctor = runAgentJson(['doctor', 'remote-view'], `${label} remote-view doctor`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-remote-view-doctor.json`] = writeJson(`${label}-remote-view-doctor.json`, remoteViewDoctor);
  const serviceStatus = runAgentJson(['service', 'status'], `${label} service status`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-service-status.json`] = writeJson(`${label}-service-status.json`, serviceStatus);
  const incidents = runAgentJson(['service', 'incidents', '--summary'], `${label} incident summary`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-service-incidents-summary.json`] = writeJson(`${label}-service-incidents-summary.json`, incidents);
  const routePool = runJson(process.execPath, ['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], `${label} route pool`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-route-pool-readiness.json`] = writeJson(`${label}-route-pool-readiness.json`, routePool);
  return {
    activeIncidents: activeIncidentCount(serviceStatus.json),
    artifacts: {
      incidents: artifacts[`${label}-service-incidents-summary.json`],
      installDoctor: artifacts[`${label}-install-doctor.json`],
      remoteViewDoctor: artifacts[`${label}-remote-view-doctor.json`],
      routePool: artifacts[`${label}-route-pool-readiness.json`],
      serviceStatus: artifacts[`${label}-service-status.json`],
    },
    installDoctor,
    incidents,
    pressure: pressureSnapshot(serviceStatus.json),
    remoteViewDoctor,
    routePoolBaseline: routePoolReturnedToBaseline(serviceStatus.json),
    routePool,
    serviceStatus,
  };
}

function displayNameFromOpen(openRecord) {
  const data = openRecord?.json?.data || {};
  return data.routeBinding?.displayName ||
    data.verification?.visibleWindowProof?.displayName ||
    null;
}

function displayStateForName(displayContentJson, displayName) {
  if (!displayName) return null;
  const routes = displayContentJson?.routes || {};
  return Object.values(routes)
    .find((route) => route?.displayName === displayName)
    ?.displayContent || null;
}

function dashboardWorkspaceUrlForDaemonSession(sessionName, baseUrl = 'http://127.0.0.1:4848/') {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', `daemon-session:${sessionName}`);
  url.searchParams.set('session', sessionName);
  return url.toString();
}

function selectedWorkspacePanelScript() {
  return `
(() => {
  const panel = document.querySelector(".workspace-selection-panel");
  const viewport = document.querySelector(".workspace-remote-viewport");
  const contextHost = panel || document.querySelector("[data-selected-workspace-context]");
  const title = panel?.querySelector(".workspace-selection-title-cell h2");
  const page = panel?.querySelector(".workspace-selection-page");
  const status = panel?.querySelector(".workspace-selection-header-status");
  const actionButtons = Array.from(panel?.querySelectorAll(".workspace-selection-action[data-action-id]") || []);
  const facts = {};
  for (const row of Array.from(panel?.querySelectorAll(".workspace-selection-details > div") || [])) {
    const key = row.querySelector("dt")?.textContent?.replace(/\\s+/g, " ").trim();
    const value = row.querySelector("dd")?.textContent?.replace(/\\s+/g, " ").trim();
    if (key) facts[key] = value || "";
  }
  const selectedWorkspaceId = panel?.getAttribute("data-selected-workspace-id") ||
    contextHost?.getAttribute("data-selected-workspace-id") ||
    viewport?.getAttribute("data-selected-workspace-id") ||
    null;
  const selectedWorkspaceSource = panel?.getAttribute("data-selected-workspace-source") ||
    contextHost?.getAttribute("data-selected-workspace-source") ||
    (selectedWorkspaceId?.startsWith("browser:") ? "service-browser" : null) ||
    (selectedWorkspaceId?.startsWith("daemon-session:") ? "daemon-session" : null);
  const selectedWorkspaceState = panel?.getAttribute("data-selected-workspace-state") ||
    contextHost?.getAttribute("data-selected-workspace-state") ||
    viewport?.getAttribute("data-selected-workspace-state") ||
    null;
  return {
    url: location.href,
    panelReady: Boolean(panel),
    contextReady: Boolean(selectedWorkspaceId),
    selectedWorkspaceId,
    selectedWorkspaceState,
    selectedWorkspaceSource,
    title: title?.textContent?.replace(/\\s+/g, " ").trim() || null,
    page: page?.textContent?.replace(/\\s+/g, " ").trim() || null,
    statusText: status?.textContent?.replace(/\\s+/g, " ").trim() || null,
    facts,
    actions: actionButtons.map((button) => ({
      id: button.getAttribute("data-action-id"),
      enabled: button.getAttribute("data-action-enabled") === "true" && !button.disabled,
      reason: button.getAttribute("data-action-reason") || button.getAttribute("title") || null,
      text: button.textContent?.replace(/\\s+/g, " ").trim() || null,
    })),
    viewport: {
      selectedWorkspaceId: viewport?.getAttribute("data-selected-workspace-id") || null,
      selectedWorkspaceState: viewport?.getAttribute("data-selected-workspace-state") || null,
      uxState: viewport?.getAttribute("data-ux-state") || null,
      hasFrame: Boolean(viewport?.querySelector("iframe")),
      hasRefreshButton: Boolean(Array.from(document.querySelectorAll("button")).find((button) => button.getAttribute("aria-label") === "Refresh workspace viewport")),
      frameSrc: viewport?.querySelector("iframe")?.getAttribute("src") || null,
      text: viewport?.textContent?.replace(/\\s+/g, " ").trim().slice(0, 1400) || null,
    },
    textSample: document.body?.innerText?.replace(/\\s+/g, " ").slice(0, 2200) || "",
  };
})()
`;
}

async function waitForSelectedWorkspacePanel(cdp, label, predicate, timeoutMs = 60000, artifactName = null) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await cdpEvaluate(cdp, selectedWorkspacePanelScript());
    if (last?.contextReady && predicate(last)) {
      if (artifactName) writeJson(artifactName, last);
      return last;
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  if (artifactName) writeJson(artifactName, last);
  throw new Error(`${label} did not reach expected selected workspace panel state: ${JSON.stringify(last)}`);
}

async function clickSelectedWorkspaceContextRefresh(viewerClient) {
  return cdpEvaluate(viewerClient.cdp, `
(() => {
  const button = document.querySelector('button[aria-label="Refresh workspace context"]');
  const viewportButton = document.querySelector('button[aria-label="Refresh workspace viewport"]');
  const target = button || viewportButton;
  if (!target) return { clicked: false, reason: "missing" };
  if (target.disabled || target.getAttribute("aria-disabled") === "true") {
    return { clicked: false, reason: "disabled" };
  }
  target.click();
  return { clicked: true, target: button ? "context" : "viewport" };
})()
`);
}

async function reloadDashboardViewerClient(viewerClient, label) {
  const before = await cdpEvaluate(viewerClient.cdp, 'location.href');
  await viewerClient.cdp.send('Page.reload', { ignoreCache: true });
  return {
    before,
    label,
    reloaded: true,
  };
}

async function fetchDashboardJson(viewerClient, path) {
  return cdpEvaluate(viewerClient.cdp, `
(async () => {
  const response = await fetch(${JSON.stringify(path)}, {
    cache: "no-store",
    credentials: "same-origin",
  });
  const text = await response.text();
  let json = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch (error) {
    return { ok: false, status: response.status, parseError: String(error.message || error), text: text.slice(0, 1000) };
  }
  return { ok: response.ok, status: response.status, json };
})()
`);
}

async function waitForDashboardJson(viewerClient, path, label, predicate = () => true, timeoutMs = 60000) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await fetchDashboardJson(viewerClient, path);
    if (last?.ok && predicate(last.json)) return last.json;
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  throw new Error(`${label} was not ready at ${path}: ${JSON.stringify(last)}`);
}

function launchForeignCdpBrowser({
  installDoctorJson,
  label,
  targetUrl,
}) {
  const { executable, verifiedChromium } = resolveViewerClientExecutable({
    commandExists,
    installDoctorJson,
  });
  if (!executable) throw new Error('No Chromium executable found for S10 foreign CDP proof');
  const profileDir = mkdtempSync(join(artifactDir, `${label}-foreign-cdp-profile-`));
  const stdoutPath = join(artifactDir, `${label}-foreign-cdp-stdout.log`);
  const stderrPath = join(artifactDir, `${label}-foreign-cdp-stderr.log`);
  const launchPath = join(artifactDir, `${label}-foreign-cdp-launch.json`);
  const launchArgs = [
    '--headless=new',
    '--disable-gpu',
    '--no-sandbox',
    `--user-data-dir=${profileDir}`,
    '--remote-debugging-port=0',
    targetUrl,
  ];
  let stdout = '';
  let stderr = '';
  let exited = null;
  let lastReadinessError = null;
  let activePort = null;
  let resolvedPort = null;
  writeFileSync(stdoutPath, '');
  writeFileSync(stderrPath, '');
  const writeLaunch = (extra = {}) => {
    writeFileSync(launchPath, `${JSON.stringify({
      executable,
      launchArgs,
      label,
      pid: proc?.pid || null,
      profileDir,
      profileBasename: basename(profileDir),
      role: 'foreign-cdp-browser',
      stdoutPath,
      stderrPath,
      targetUrl,
      verifiedChromium,
      activePort,
      exited,
      lastReadinessError,
      resolvedPort,
      ...extra,
    }, null, 2)}\n`);
  };
  const proc = spawn(executable, launchArgs, {
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  proc.stdout.setEncoding('utf8');
  proc.stdout.on('data', (chunk) => {
    stdout += chunk;
    writeFileSync(stdoutPath, stdout);
  });
  proc.stderr.setEncoding('utf8');
  proc.stderr.on('data', (chunk) => {
    stderr += chunk;
    writeFileSync(stderrPath, stderr);
  });
  proc.on('exit', (code, signal) => {
    exited = { code, signal };
    writeLaunch();
  });
  writeLaunch({ started: true });
  return {
    executable,
    launchArgs,
    launchPath,
    label,
    proc,
    profileDir,
    profileBasename: basename(profileDir),
    stderrPath,
    stdoutPath,
    targetUrl,
    verifiedChromium,
    async waitUntilReady() {
      activePort = await waitForDevToolsActivePort(profileDir, `${label} foreign CDP browser`, 30000, (error) => {
        lastReadinessError = error;
      });
      resolvedPort = activePort.port;
      const version = await waitForJson(`http://127.0.0.1:${resolvedPort}/json/version`, `${label} foreign CDP browser`, 30000, (error) => {
        lastReadinessError = error;
      });
      const pages = await waitForJson(`http://127.0.0.1:${resolvedPort}/json`, `${label} foreign CDP browser pages`, 30000, (error) => {
        lastReadinessError = error;
      });
      writeLaunch({ ready: true, version, pages });
      return { activePort, pages, port: resolvedPort, version };
    },
    close() {
      try {
        proc.kill('SIGTERM');
      } catch {
        // Best effort.
      }
      let cleanupError = null;
      try {
        rmSync(profileDir, {
          recursive: true,
          force: true,
          maxRetries: 8,
          retryDelay: 125,
        });
      } catch (error) {
        cleanupError = String(error.stack || error.message || error);
      }
      writeLaunch({ closed: true, cleanupError });
    },
  };
}

function evaluateS0(capture) {
  const failures = [];
  const warnings = [];
  const installSuccess = capture.installDoctor.status === 0 && capture.installDoctor.json?.success === true;
  const remoteViewSuccess = capture.remoteViewDoctor.status === 0 && capture.remoteViewDoctor.json?.success === true;
  const serviceSuccess = capture.serviceStatus.status === 0 && capture.serviceStatus.json?.success === true;
  const activeIncidents = activeIncidentCount(capture.serviceStatus.json);
  const incidentActiveRows = (capture.incidents.json?.data?.incidents || []).filter((incident) => incident?.state === 'active');
  const routePoolSuccess = capture.routePool.status === 0 && capture.routePool.json?.success === true;
  const routeEntries = routePoolEntries(capture.routePool.json);
  const displaySuccess = capture.displayContent.status === 0 && capture.displayContent.json?.success === true;
  const states = displayStates(capture.displayContent.json);
  const terminalDisplays = states.filter((state) => state.terminalVisible || state.state === 'terminal_only');
  const guacamoleOk = capture.guacamole.status === 0 && /^2\d\d\b/.test(capture.guacamole.stdout.trim());
  const dashboardOk = capture.dashboard.status === 0 && /^[23]\d\d\b/.test(capture.dashboard.stdout.trim());

  if (!installSuccess) failures.push('install doctor is not green');
  if (!remoteViewSuccess) failures.push('remote-view doctor is not green');
  if (!serviceSuccess) failures.push('service status did not return success');
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (incidentActiveRows.length !== 0) failures.push(`incident summary contains ${incidentActiveRows.length} active incident row(s)`);
  if (!routePoolSuccess || routeEntries.length < 2) failures.push('route-pool readiness did not return two ready entries');
  if (!displaySuccess) failures.push('route display inspection is not green');
  if (terminalDisplays.length > 0) failures.push(`terminal content visible on route display(s): ${terminalDisplays.map((item) => item.route).join(', ')}`);
  if (!guacamoleOk) failures.push('public Guacamole route did not return an HTTP 2xx final status');
  if (!dashboardOk) failures.push('dashboard root did not return HTTP 2xx or 3xx');
  if (capture.screenshots.length < 2 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    warnings.push('route display screenshots were not captured for both routes');
  }
  warnings.push('dashboard authenticated live-rail visual proof is not implemented in the S0 runner yet');

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      dashboardHttp: capture.dashboard.stdout.trim(),
      displayStates: states,
      guacamoleHttp: capture.guacamole.stdout.trim(),
      installDoctorSuccess: installSuccess,
      remoteViewDoctorSuccess: remoteViewSuccess,
      routePoolEntries: routeEntries.map((entry) => ({
        id: entry.id,
        routeId: entry.routeId,
        displayName: entry.target?.displayName,
        state: entry.readiness?.state,
      })),
      screenshotCount: capture.screenshots.length,
    },
  };
}

function captureS1() {
  const artifacts = {};
  const before = captureBaseline('s1-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const poolEnv = routePoolEnv(before.routePool.json);
  const open = runAgentJson([
    'remote-view',
    'open',
    'https://example.com/',
    '--runtime-profile',
    'p46-s1-profile',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's1 remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);

  const getUrlAfterOpen = runAgentJson(['get', 'url'], 's1 get url after open', {
    timeoutMs: 60000,
  });
  artifacts['get-url-after-open.json'] = writeJson('get-url-after-open.json', getUrlAfterOpen);

  const navigate = runAgentJson(['open', 'https://www.iana.org/domains/reserved'], 's1 navigate current tab', {
    timeoutMs: 120000,
  });
  artifacts['navigate-current-tab.json'] = writeJson('navigate-current-tab.json', navigate);

  const getUrlAfterNavigate = runAgentJson(['get', 'url'], 's1 get url after navigate', {
    timeoutMs: 60000,
  });
  artifacts['get-url-after-navigate.json'] = writeJson('get-url-after-navigate.json', getUrlAfterNavigate);

  const tabNew = runAgentJson(['tab', 'new', 'https://example.com/?p46-tab=new'], 's1 new tab', {
    timeoutMs: 120000,
  });
  artifacts['tab-new.json'] = writeJson('tab-new.json', tabNew);

  const tabList = runAgentJson(['tab', 'list'], 's1 tab list', {
    timeoutMs: 60000,
  });
  artifacts['tab-list.json'] = writeJson('tab-list.json', tabList);

  const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's1 route display content after controls', {
    timeoutMs: 60000,
  });
  artifacts['display-content-after-controls.json'] = writeJson('display-content-after-controls.json', displayContent);

  const importCommand = commandExists('import');
  const screenshots = [];
  if (importCommand) {
    const displayName = open.json?.data?.routeBinding?.displayName ||
      open.json?.data?.verification?.visibleWindowProof?.displayName ||
      ':13';
    const path = join(artifactDir, 's1-route-display-root.png');
    const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], 's1 route display screenshot', {
      timeoutMs: 60000,
    });
    screenshots.push({
      displayName,
      path,
      status: screenshot.status,
      stderr: screenshot.stderr,
    });
  }
  artifacts['s1-screenshots.json'] = writeJson('s1-screenshots.json', screenshots);

  const serviceStatus = runAgentJson(['service', 'status'], 's1 service status after controls', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-controls.json'] = writeJson('service-status-after-controls.json', serviceStatus);

  const incidents = runAgentJson(['service', 'incidents', '--summary'], 's1 incident summary after controls', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-after-controls.json'] = writeJson('service-incidents-after-controls.json', incidents);

  return {
    artifacts,
    before,
    displayContent,
    getUrlAfterNavigate,
    getUrlAfterOpen,
    incidents,
    navigate,
    open,
    screenshots,
    serviceStatus,
    tabList,
    tabNew,
  };
}

async function captureS2() {
  const artifacts = {};
  const externalOperators = [];
  const before = captureBaseline('s2-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const open = runAgentJson([
    'remote-view',
    'open',
    'https://example.com/?p46=s2',
    '--runtime-profile',
    'p46-s2-profile',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's2 remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);

  const browserId = open.json?.data?.browserId || open.json?.data?.tab?.browserId || 'session:default';
  const sessionName = open.json?.data?.sessionName || 'default';
  const tabId = serviceTabIdFromOpen(open.json);
  const routeUrl = open.json?.data?.routeDescriptor?.publicOperatorUrl ||
    open.json?.data?.routeBinding?.externalUrl ||
    null;
  const expected = { browserId, sessionName, tabId };
  const dashboardUrl = viewerClientDashboardWorkspaceUrl(expected);
  artifacts['s2-targets.json'] = writeJson('s2-targets.json', {
    browserId,
    dashboardUrl,
    routeUrl,
    sessionName,
    tabId,
  });

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  try {
    const operatorA = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl,
      expected,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-a',
      writeJson,
    });
    externalOperators.push(operatorA);
    artifacts['operator-a-dashboard-state.json'] = writeJson('operator-a-dashboard-state.json', operatorA.state);
    const operatorB = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl,
      expected,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-b',
      writeJson,
    });
    externalOperators.push(operatorB);
    artifacts['operator-b-dashboard-state.json'] = writeJson('operator-b-dashboard-state.json', operatorB.state);
    const refreshB = await clickViewerClientDashboardRefresh(operatorB);
    artifacts['operator-b-refresh-click.json'] = writeJson('operator-b-refresh-click.json', refreshB);

    const dashboardScreenshotA = await captureDashboardScreenshot(operatorA, join(artifactDir, 'operator-a-dashboard.png'));
    artifacts['operator-a-dashboard-screenshot.json'] = writeJson('operator-a-dashboard-screenshot.json', {
      path: dashboardScreenshotA,
      status: existsSync(dashboardScreenshotA) ? 0 : 1,
    });
    const dashboardScreenshotB = await captureDashboardScreenshot(operatorB, join(artifactDir, 'operator-b-dashboard.png'));
    artifacts['operator-b-dashboard-screenshot.json'] = writeJson('operator-b-dashboard-screenshot.json', {
      path: dashboardScreenshotB,
      status: existsSync(dashboardScreenshotB) ? 0 : 1,
    });

    const navigate = runAgentJson(['open', 'https://www.iana.org/domains/reserved'], 's2 operator A navigates controlled browser', {
      timeoutMs: 120000,
    });
    artifacts['operator-a-navigate-controlled-browser.json'] = writeJson('operator-a-navigate-controlled-browser.json', navigate);
    const getUrlAfterNavigate = runAgentJson(['get', 'url'], 's2 get url after operator A navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-url-after-operator-a-navigate.json'] = writeJson('get-url-after-operator-a-navigate.json', getUrlAfterNavigate);

    const stateBAfterNavigate = await waitForViewerClientDashboardState(
      operatorB.cdp,
      expected,
      'operator B after navigate',
      60000,
      writeJson,
    );
    artifacts['operator-b-dashboard-state-after-navigate.json'] = writeJson('operator-b-dashboard-state-after-navigate.json', stateBAfterNavigate);

  const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's2 route display content after navigate', {
    timeoutMs: 60000,
  });
  artifacts['display-content-after-navigate.json'] = writeJson('display-content-after-navigate.json', displayContent);

  const importCommand = commandExists('import');
  const screenshots = [];
  if (importCommand) {
    const displayName = open.json?.data?.routeBinding?.displayName ||
      open.json?.data?.verification?.visibleWindowProof?.displayName ||
      ':13';
    const path = join(artifactDir, 's2-route-display-root.png');
    const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], 's2 route display screenshot', {
      timeoutMs: 60000,
    });
    screenshots.push({
      displayName,
      path,
      status: screenshot.status,
      stderr: screenshot.stderr,
    });
  }
  artifacts['s2-route-display-screenshots.json'] = writeJson('s2-route-display-screenshots.json', screenshots);

  const serviceStatus = runAgentJson(['service', 'status'], 's2 service status after two operators', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-two-operators.json'] = writeJson('service-status-after-two-operators.json', serviceStatus);

  const incidents = runAgentJson(['service', 'incidents', '--summary'], 's2 incident summary after two operators', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-after-two-operators.json'] = writeJson('service-incidents-after-two-operators.json', incidents);
  const finalization = routeBoundFinalizationEvidence({
    incidentsJson: incidents.json,
    openJson: open.json,
    statusJson: serviceStatus.json,
  });
  artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

    return {
      artifacts,
      before,
      credentials,
      displayContent,
      getUrlAfterNavigate,
      finalization,
      incidents,
      navigate,
      open,
      refreshB: { json: { data: { result: refreshB } } },
      screenshots,
      screenshotA: { status: existsSync(dashboardScreenshotA) ? 0 : 1 },
      screenshotB: { status: existsSync(dashboardScreenshotB) ? 0 : 1 },
      serviceStatus,
      stateA: { json: { data: { result: operatorA.state } } },
      stateB: { json: { data: { result: operatorB.state } } },
      stateBAfterNavigate: { json: { data: { result: stateBAfterNavigate } } },
      targets: {
        browserId,
        dashboardScreenshotA,
        dashboardScreenshotB,
        dashboardUrl,
        routeUrl,
        sessionName,
        tabId,
      },
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

function remoteViewOpenReady(record) {
  const data = record?.json?.data || {};
  return record?.status === 0 &&
    record?.json?.success === true &&
    data.status === 'opened' &&
    data.operatorVisible?.state === 'ready';
}

function sameProfileWindowReady(record) {
  const data = record?.json?.data || {};
  return record?.status === 0 &&
    record?.json?.success === true &&
    data.sameProfile === true &&
    typeof data.targetId === 'string' &&
    data.targetId.length > 0;
}

function captureS3Diagnostics(label, artifacts, open) {
  const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], `${label} route display content`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-display-content.json`] = writeJson(`${label}-display-content.json`, displayContent);

  const serviceStatus = runAgentJson(['service', 'status'], `${label} service status`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-service-status.json`] = writeJson(`${label}-service-status.json`, serviceStatus);

  const incidents = runAgentJson(['service', 'incidents', '--summary'], `${label} incident summary`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-service-incidents.json`] = writeJson(`${label}-service-incidents.json`, incidents);

  const routePool = runJson(process.execPath, ['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], `${label} route-pool readiness`, {
    timeoutMs: 60000,
  });
  artifacts[`${label}-route-pool-readiness.json`] = writeJson(`${label}-route-pool-readiness.json`, routePool);

  const finalization = routeBoundFinalizationEvidence({
    incidentsJson: incidents.json,
    openJson: open?.json,
    statusJson: serviceStatus.json,
  });
  artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

  return {
    displayContent,
    finalization,
    incidents,
    routePool,
    serviceStatus,
  };
}

function s3EarlyFailureCapture({
  artifacts,
  before,
  credentials,
  diagnostics,
  failedStage,
  failureReason,
  open,
  tabList = null,
  tabNew = null,
  targets = null,
}) {
  return {
    artifacts,
    before,
    credentials,
    displayContent: diagnostics.displayContent,
    failedStage,
    failureReason,
    finalization: diagnostics.finalization,
    getUrlAAfterBNavigate: null,
    getUrlAAfterNavigate: null,
    getUrlBAfterANavigate: null,
    getUrlBAfterNavigate: null,
    incidents: diagnostics.incidents,
    navigateA: null,
    navigateB: null,
    open,
    refreshA: null,
    refreshB: null,
    routePool: diagnostics.routePool,
    screenshots: [],
    screenshotA: { status: 1 },
    screenshotB: { status: 1 },
    serviceStatus: diagnostics.serviceStatus,
    stateA: null,
    stateAAfterControls: null,
    stateB: null,
    stateBAfterControls: null,
    switchA: null,
    switchABack: null,
    switchB: null,
    tabList,
    tabNew,
    targets: targets || {
      browserId: null,
      dashboardScreenshotA: null,
      dashboardScreenshotB: null,
      dashboardUrlA: null,
      dashboardUrlB: null,
      routeUrl: null,
      sessionName: null,
      tabA: null,
      tabAId: null,
      tabB: null,
      tabBId: null,
    },
  };
}

async function captureS3() {
  const artifacts = {};
  const externalOperators = [];
  const before = captureBaseline('s3-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const open = runAgentJson([
    'remote-view',
    'open',
    'https://example.com/?p46=s3-tab-a',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's3 default-profile remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);
  if (!remoteViewOpenReady(open)) {
    const diagnostics = captureS3Diagnostics('s3-after-failed-open', artifacts, open);
    return s3EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnostics,
      failedStage: 'remote_view_open',
      failureReason: open.json?.error || 'remote-view open did not produce operatorVisible.state=ready',
      open,
    });
  }

  const tabNew = runAgentJson(['tab', 'new', 'https://example.com/?p46=s3-tab-b'], 's3 new tab B', {
    timeoutMs: 120000,
  });
  artifacts['tab-new-b.json'] = writeJson('tab-new-b.json', tabNew);

  const tabList = runAgentJson(['tab', 'list', '--verbose'], 's3 tab list after tab B', {
    timeoutMs: 60000,
  });
  artifacts['tab-list-after-tab-b.json'] = writeJson('tab-list-after-tab-b.json', tabList);
  const tabs = tabsFromRecord(tabList);
  const tabA = tabs.find((tab) => String(tab?.url || '').includes('p46=s3-tab-a')) || tabs[0] || null;
  const tabB = tabs.find((tab) => String(tab?.url || '').includes('p46=s3-tab-b')) || tabs[1] || null;
  const tabAId = serviceTabIdFromOpen(open.json) || serviceTabIdFromTab(tabA);
  const tabBId = serviceTabIdFromCommandData(tabNew.json?.data) || serviceTabIdFromTab(tabB);
  const browserId = open.json?.data?.browserId || open.json?.data?.tab?.browserId || 'session:default';
  const sessionName = open.json?.data?.sessionName || 'default';
  const routeUrl = open.json?.data?.routeDescriptor?.publicOperatorUrl ||
    open.json?.data?.routeBinding?.externalUrl ||
    null;
  const expectedA = { browserId, sessionName, tabId: tabAId };
  const expectedB = { browserId, sessionName, tabId: tabBId };
  const dashboardUrlA = viewerClientDashboardWorkspaceUrl(expectedA);
  const dashboardUrlB = viewerClientDashboardWorkspaceUrl(expectedB);
  artifacts['s3-targets.json'] = writeJson('s3-targets.json', {
    browserId,
    dashboardUrlA,
    dashboardUrlB,
    routeUrl,
    sessionName,
    tabA: { id: tabAId, selector: tabSelector(open.json?.data?.tab || tabA, 1), source: tabA },
    tabB: { id: tabBId, selector: tabSelector(tabNew.json?.data || tabB, 2), source: tabB },
  });

  if (!tabAId || !tabBId || tabAId === tabBId || tabNew.status !== 0 || tabNew.json?.success !== true) {
    const diagnostics = captureS3Diagnostics('s3-after-tab-handle-failure', artifacts, open);
    return s3EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnostics,
      failedStage: 'tab_handles',
      failureReason: 'S3 did not obtain two distinct service tab handles before dashboard launch',
      open,
      tabList,
      tabNew,
      targets: {
        browserId,
        dashboardScreenshotA: null,
        dashboardScreenshotB: null,
        dashboardUrlA,
        dashboardUrlB,
        routeUrl,
        sessionName,
        tabA,
        tabAId,
        tabB,
        tabBId,
      },
    });
  }

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  try {
    const operatorA = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlA,
      expected: expectedA,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-a',
      writeJson,
    });
    externalOperators.push(operatorA);
    artifacts['operator-a-dashboard-state.json'] = writeJson('operator-a-dashboard-state.json', operatorA.state);
    const operatorB = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlB,
      expected: expectedB,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-b',
      writeJson,
    });
    externalOperators.push(operatorB);
    artifacts['operator-b-dashboard-state.json'] = writeJson('operator-b-dashboard-state.json', operatorB.state);
    const refreshA = await clickViewerClientDashboardRefresh(operatorA);
    artifacts['operator-a-refresh-click.json'] = writeJson('operator-a-refresh-click.json', refreshA);
    const refreshB = await clickViewerClientDashboardRefresh(operatorB);
    artifacts['operator-b-refresh-click.json'] = writeJson('operator-b-refresh-click.json', refreshB);

    const dashboardScreenshotA = await captureDashboardScreenshot(operatorA, join(artifactDir, 'operator-a-dashboard.png'));
    artifacts['operator-a-dashboard-screenshot.json'] = writeJson('operator-a-dashboard-screenshot.json', {
      path: dashboardScreenshotA,
      status: existsSync(dashboardScreenshotA) ? 0 : 1,
    });
    const dashboardScreenshotB = await captureDashboardScreenshot(operatorB, join(artifactDir, 'operator-b-dashboard.png'));
    artifacts['operator-b-dashboard-screenshot.json'] = writeJson('operator-b-dashboard-screenshot.json', {
      path: dashboardScreenshotB,
      status: existsSync(dashboardScreenshotB) ? 0 : 1,
    });

    const switchA = runAgentJson(['tab', tabSelector(open.json?.data?.tab || tabA, 1)], 's3 switch to tab A', {
      timeoutMs: 60000,
    });
    artifacts['switch-tab-a.json'] = writeJson('switch-tab-a.json', switchA);
    const navigateA = runAgentJson(['open', 'https://www.iana.org/domains/reserved?p46=s3-tab-a'], 's3 navigate tab A', {
      timeoutMs: 120000,
    });
    artifacts['navigate-tab-a.json'] = writeJson('navigate-tab-a.json', navigateA);
    const getUrlAAfterNavigate = runAgentJson(['get', 'url'], 's3 get tab A URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-tab-a-url-after-navigate.json'] = writeJson('get-tab-a-url-after-navigate.json', getUrlAAfterNavigate);

    const switchB = runAgentJson(['tab', tabSelector(tabNew.json?.data || tabB, 2)], 's3 switch to tab B', {
      timeoutMs: 60000,
    });
    artifacts['switch-tab-b.json'] = writeJson('switch-tab-b.json', switchB);
    const getUrlBAfterANavigate = runAgentJson(['get', 'url'], 's3 get tab B URL after tab A navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-tab-b-url-after-tab-a-navigate.json'] = writeJson('get-tab-b-url-after-tab-a-navigate.json', getUrlBAfterANavigate);
    const navigateB = runAgentJson(['open', 'https://example.org/?p46=s3-tab-b'], 's3 navigate tab B', {
      timeoutMs: 120000,
    });
    artifacts['navigate-tab-b.json'] = writeJson('navigate-tab-b.json', navigateB);
    const getUrlBAfterNavigate = runAgentJson(['get', 'url'], 's3 get tab B URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-tab-b-url-after-navigate.json'] = writeJson('get-tab-b-url-after-navigate.json', getUrlBAfterNavigate);
    const switchABack = runAgentJson(['tab', tabSelector(open.json?.data?.tab || tabA, 1)], 's3 switch back to tab A', {
      timeoutMs: 60000,
    });
    artifacts['switch-tab-a-back.json'] = writeJson('switch-tab-a-back.json', switchABack);
    const getUrlAAfterBNavigate = runAgentJson(['get', 'url'], 's3 get tab A URL after tab B navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-tab-a-url-after-tab-b-navigate.json'] = writeJson('get-tab-a-url-after-tab-b-navigate.json', getUrlAAfterBNavigate);

    const stateAAfterControls = await waitForViewerClientDashboardState(
      operatorA.cdp,
      expectedA,
      'operator A after S3 tab controls',
      60000,
      writeJson,
    );
    artifacts['operator-a-dashboard-state-after-controls.json'] = writeJson('operator-a-dashboard-state-after-controls.json', stateAAfterControls);
    const stateBAfterControls = await waitForViewerClientDashboardState(
      operatorB.cdp,
      expectedB,
      'operator B after S3 tab controls',
      60000,
      writeJson,
    );
    artifacts['operator-b-dashboard-state-after-controls.json'] = writeJson('operator-b-dashboard-state-after-controls.json', stateBAfterControls);

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's3 route display content after controls', {
      timeoutMs: 60000,
    });
    artifacts['display-content-after-controls.json'] = writeJson('display-content-after-controls.json', displayContent);

    const importCommand = commandExists('import');
    const screenshots = [];
    if (importCommand) {
      const displayName = open.json?.data?.routeBinding?.displayName ||
        open.json?.data?.verification?.visibleWindowProof?.displayName ||
        ':13';
      const path = join(artifactDir, 's3-route-display-root.png');
      const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], 's3 route display screenshot', {
        timeoutMs: 60000,
      });
      screenshots.push({
        displayName,
        path,
        status: screenshot.status,
        stderr: screenshot.stderr,
      });
    }
    artifacts['s3-route-display-screenshots.json'] = writeJson('s3-route-display-screenshots.json', screenshots);

    const serviceStatus = runAgentJson(['service', 'status'], 's3 service status after tab controls', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-tab-controls.json'] = writeJson('service-status-after-tab-controls.json', serviceStatus);

    const incidents = runAgentJson(['service', 'incidents', '--summary'], 's3 incident summary after tab controls', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-tab-controls.json'] = writeJson('service-incidents-after-tab-controls.json', incidents);
    const finalization = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: open.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

    return {
      artifacts,
      before,
      credentials,
      displayContent,
      finalization,
      getUrlAAfterBNavigate,
      getUrlAAfterNavigate,
      getUrlBAfterANavigate,
      getUrlBAfterNavigate,
      incidents,
      navigateA,
      navigateB,
      open,
      refreshA: { json: { data: { result: refreshA } } },
      refreshB: { json: { data: { result: refreshB } } },
      screenshots,
      screenshotA: { status: existsSync(dashboardScreenshotA) ? 0 : 1 },
      screenshotB: { status: existsSync(dashboardScreenshotB) ? 0 : 1 },
      serviceStatus,
      stateA: { json: { data: { result: operatorA.state } } },
      stateAAfterControls: { json: { data: { result: stateAAfterControls } } },
      stateB: { json: { data: { result: operatorB.state } } },
      stateBAfterControls: { json: { data: { result: stateBAfterControls } } },
      switchA,
      switchABack,
      switchB,
      tabList,
      tabNew,
      targets: {
        browserId,
        dashboardScreenshotA,
        dashboardScreenshotB,
        dashboardUrlA,
        dashboardUrlB,
        routeUrl,
        sessionName,
        tabA,
        tabAId,
        tabB,
        tabBId,
      },
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

function s4EarlyFailureCapture({
  artifacts,
  before,
  credentials,
  diagnosticsA = null,
  diagnosticsB = null,
  failedStage,
  failureReason,
  openA,
  openB = null,
  targets = null,
  topologyPreflight = null,
}) {
  return {
    artifacts,
    before,
    closeA: null,
    credentials,
    displayContent: diagnosticsB?.displayContent || diagnosticsA?.displayContent || null,
    failedStage,
    failureReason,
    finalizationA: diagnosticsA?.finalization || null,
    finalizationB: diagnosticsB?.finalization || null,
    getUrlAAfterNavigate: null,
    getUrlBAfterCloseA: null,
    getUrlBAfterNavigate: null,
    incidents: diagnosticsB?.incidents || diagnosticsA?.incidents || null,
    incidentsAfterCloseA: null,
    navigateA: null,
    navigateB: null,
    openA,
    openB,
    refreshA: null,
    refreshB: null,
    screenshots: [],
    screenshotA: { status: 1 },
    screenshotB: { status: 1 },
    serviceStatus: diagnosticsB?.serviceStatus || diagnosticsA?.serviceStatus || null,
    serviceStatusAfterCloseA: null,
    stateA: null,
    stateAAfterControls: null,
    stateB: null,
    stateBAfterControls: null,
    tabNewA: null,
    tabNewB: null,
    topologyPreflight,
    targets: targets || {
      browserAId: openA?.json?.data?.browserId || null,
      browserBId: openB?.json?.data?.browserId || null,
      dashboardScreenshotA: null,
      dashboardScreenshotB: null,
      dashboardUrlA: null,
      dashboardUrlB: null,
      displayA: displayNameFromOpen(openA),
      displayB: displayNameFromOpen(openB),
      routeAId: openA?.json?.data?.routeId || openA?.json?.data?.routeBinding?.routeId || null,
      routeBId: openB?.json?.data?.routeId || openB?.json?.data?.routeBinding?.routeId || null,
      sessionA: 'p46-s4-window-a',
      sessionB: 'p46-s4-window-b',
      tabAId: serviceTabIdFromOpen(openA?.json),
      tabBId: serviceTabIdFromOpen(openB?.json),
    },
  };
}

function s4SameProfileTopologyPreflight({ profileId, sessionA, sessionB, routePoolEntryA, routePoolEntryB }) {
  const sameRuntimeProfile = Boolean(profileId && sessionA && sessionB && sessionA !== sessionB);
  const distinctRoutePoolEntries = Boolean(routePoolEntryA && routePoolEntryB && routePoolEntryA !== routePoolEntryB);
  const independentBrowserProcesses = Boolean(sameRuntimeProfile && distinctRoutePoolEntries);
  const blocker = independentBrowserProcesses && !allowS4DuplicateProfileLane
    ? {
      code: 'same_profile_multi_process_unsupported',
      message: [
        `S4 would launch runtime profile '${profileId}' in two independent browser sessions`,
        `(${sessionA} and ${sessionB}) across route-pool entries ${routePoolEntryA} and ${routePoolEntryB}.`,
        'Shared authenticated profiles must use the retained browser lane unless reviewed duplicate-profile-lane intent is explicit.',
      ].join(' '),
      remediation: 'Choose a retained-browser same-profile topology or rerun only after reviewed duplicate-lane behavior is explicitly implemented.',
    }
    : null;
  return {
    allowed: !blocker,
    allowDuplicateProfileLane: allowS4DuplicateProfileLane,
    blocker,
    profileId,
    routePoolEntries: [routePoolEntryA, routePoolEntryB].filter(Boolean),
    sessions: [sessionA, sessionB],
    topology: independentBrowserProcesses
      ? 'one_profile_two_sessions_two_route_pool_entries'
      : 'one_profile_one_session_one_route_two_windows',
  };
}

async function captureS4() {
  const artifacts = {};
  const externalOperators = [];
  const profileId = 'p46-s4-profile';
  const runId = new Date().toISOString().replace(/[:.]/g, '-');
  const sessionA = `p46-s4-window-${runId}`;
  const sessionB = sessionA;
  const routePoolEntryA = 'guacamole-rdp-a';
  const routePoolEntryB = null;
  const before = captureBaseline('s4-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const commonOpenArgs = [
    '--runtime-profile',
    profileId,
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ];
  const openA = runAgentSessionJson(sessionA, [
    'remote-view',
    'open',
    'https://example.com/?p46=s4-window-a',
    '--session-name',
    sessionA,
    '--route-pool-entry-id',
    routePoolEntryA,
    ...commonOpenArgs,
  ], 's4 window A remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['window-a-remote-view-open.json'] = writeJson('window-a-remote-view-open.json', openA);
  if (!remoteViewOpenReady(openA)) {
    const diagnosticsA = captureS3Diagnostics('s4-after-window-a-failed-open', artifacts, openA);
    return s4EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnosticsA,
      failedStage: 'window_a_remote_view_open',
      failureReason: openA.json?.error || 'window A remote-view open did not produce operatorVisible.state=ready',
      openA,
    });
  }

  const topologyPreflight = s4SameProfileTopologyPreflight({
    profileId,
    routePoolEntryA,
    routePoolEntryB,
    sessionA,
    sessionB,
  });
  artifacts['s4-topology-preflight.json'] = writeJson('s4-topology-preflight.json', topologyPreflight);
  if (!topologyPreflight.allowed) {
    const diagnosticsA = captureS3Diagnostics('s4-after-topology-blocker', artifacts, openA);
    return s4EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnosticsA,
      failedStage: 'same_profile_multi_process_preflight',
      failureReason: `${topologyPreflight.blocker.code}: ${topologyPreflight.blocker.message}`,
      openA,
      topologyPreflight,
    });
  }

  const openB = runAgentSessionJson(sessionB, [
    'window',
    'new',
    'https://example.org/?p46=s4-window-b',
    '--same-profile',
  ], 's4 window B same-profile window open', {
    timeoutMs: 120000,
  });
  artifacts['window-b-same-profile-window-open.json'] = writeJson('window-b-same-profile-window-open.json', openB);
  if (!sameProfileWindowReady(openB)) {
    const diagnosticsA = captureS3Diagnostics('s4-after-window-b-failed-open-a', artifacts, openA);
    const diagnosticsB = captureS3Diagnostics('s4-after-window-b-failed-open-b', artifacts, openB);
    return s4EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnosticsA,
      diagnosticsB,
      failedStage: 'window_b_remote_view_open',
      failureReason: openB.json?.error || 'window B same-profile window open did not return a targetId',
      openA,
      openB,
    });
  }

  const browserAId = openA.json?.data?.browserId || openA.json?.data?.tab?.browserId || `session:${sessionA}`;
  const browserBId = browserAId;
  const tabAId = serviceTabIdFromOpen(openA.json);
  const tabBId = serviceTabIdFromOpen(openB.json);
  const expectedA = { browserId: browserAId, sessionName: sessionA, tabId: tabAId };
  const expectedB = { browserId: browserBId, sessionName: sessionB, tabId: tabBId };
  const dashboardUrlA = viewerClientDashboardWorkspaceUrl(expectedA);
  const dashboardUrlB = viewerClientDashboardWorkspaceUrl(expectedB);
  const targets = {
    browserAId,
    browserBId,
    dashboardScreenshotA: null,
    dashboardScreenshotB: null,
    dashboardUrlA,
    dashboardUrlB,
    displayA: displayNameFromOpen(openA),
    displayB: displayNameFromOpen(openA),
    profileId,
    routeAId: openA.json?.data?.routeId || openA.json?.data?.routeBinding?.routeId || null,
    routeBId: openA.json?.data?.routeId || openA.json?.data?.routeBinding?.routeId || null,
    sessionA,
    sessionB,
    tabAId,
    tabBId,
  };
  artifacts['s4-targets.json'] = writeJson('s4-targets.json', targets);

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  try {
    const operatorA = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlA,
      expected: expectedA,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-a',
      writeJson,
    });
    externalOperators.push(operatorA);
    artifacts['operator-a-dashboard-state.json'] = writeJson('operator-a-dashboard-state.json', operatorA.state);
    const operatorB = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlB,
      expected: expectedB,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-b',
      writeJson,
    });
    externalOperators.push(operatorB);
    artifacts['operator-b-dashboard-state.json'] = writeJson('operator-b-dashboard-state.json', operatorB.state);
    const refreshA = await clickViewerClientDashboardRefresh(operatorA);
    artifacts['operator-a-refresh-click.json'] = writeJson('operator-a-refresh-click.json', refreshA);
    const refreshB = await clickViewerClientDashboardRefresh(operatorB);
    artifacts['operator-b-refresh-click.json'] = writeJson('operator-b-refresh-click.json', refreshB);

    const dashboardScreenshotA = await captureDashboardScreenshot(operatorA, join(artifactDir, 'operator-a-dashboard.png'));
    targets.dashboardScreenshotA = dashboardScreenshotA;
    artifacts['operator-a-dashboard-screenshot.json'] = writeJson('operator-a-dashboard-screenshot.json', {
      path: dashboardScreenshotA,
      status: existsSync(dashboardScreenshotA) ? 0 : 1,
    });
    const dashboardScreenshotB = await captureDashboardScreenshot(operatorB, join(artifactDir, 'operator-b-dashboard.png'));
    targets.dashboardScreenshotB = dashboardScreenshotB;
    artifacts['operator-b-dashboard-screenshot.json'] = writeJson('operator-b-dashboard-screenshot.json', {
      path: dashboardScreenshotB,
      status: existsSync(dashboardScreenshotB) ? 0 : 1,
    });
    artifacts['s4-targets-after-dashboard.json'] = writeJson('s4-targets-after-dashboard.json', targets);

    const navigateA = runAgentSessionJson(sessionA, ['open', 'https://www.iana.org/domains/reserved?p46=s4-window-a'], 's4 navigate window A', {
      timeoutMs: 120000,
    });
    artifacts['window-a-navigate.json'] = writeJson('window-a-navigate.json', navigateA);
    const getUrlAAfterNavigate = runAgentSessionJson(sessionA, ['get', 'url'], 's4 get window A URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['window-a-get-url-after-navigate.json'] = writeJson('window-a-get-url-after-navigate.json', getUrlAAfterNavigate);
    const tabNewA = runAgentSessionJson(sessionA, ['tab', 'new', 'https://example.com/?p46=s4-window-a-tab'], 's4 new tab in window A', {
      timeoutMs: 120000,
    });
    artifacts['window-a-tab-new.json'] = writeJson('window-a-tab-new.json', tabNewA);

    const navigateB = runAgentSessionJson(sessionB, ['open', 'https://example.org/?p46=s4-window-b-navigate'], 's4 navigate window B', {
      timeoutMs: 120000,
    });
    artifacts['window-b-navigate.json'] = writeJson('window-b-navigate.json', navigateB);
    const getUrlBAfterNavigate = runAgentSessionJson(sessionB, ['get', 'url'], 's4 get window B URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['window-b-get-url-after-navigate.json'] = writeJson('window-b-get-url-after-navigate.json', getUrlBAfterNavigate);
    const tabNewB = runAgentSessionJson(sessionB, ['tab', 'new', 'https://example.org/?p46=s4-window-b-tab'], 's4 new tab in window B', {
      timeoutMs: 120000,
    });
    artifacts['window-b-tab-new.json'] = writeJson('window-b-tab-new.json', tabNewB);

    const stateAAfterControls = await waitForViewerClientDashboardState(
      operatorA.cdp,
      expectedA,
      'operator A after S4 window controls',
      60000,
      writeJson,
    );
    artifacts['operator-a-dashboard-state-after-controls.json'] = writeJson('operator-a-dashboard-state-after-controls.json', stateAAfterControls);
    const stateBAfterControls = await waitForViewerClientDashboardState(
      operatorB.cdp,
      expectedB,
      'operator B after S4 window controls',
      60000,
      writeJson,
    );
    artifacts['operator-b-dashboard-state-after-controls.json'] = writeJson('operator-b-dashboard-state-after-controls.json', stateBAfterControls);

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's4 route display content before close', {
      timeoutMs: 60000,
    });
    artifacts['display-content-before-close.json'] = writeJson('display-content-before-close.json', displayContent);

    const importCommand = commandExists('import');
    const screenshots = [];
    if (importCommand) {
      for (const [label, displayName] of [['window-a', targets.displayA], ['window-b', targets.displayB]]) {
        if (!displayName) continue;
        const path = join(artifactDir, `${label}-route-display-root.png`);
        const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], `s4 ${label} route display screenshot`, {
          timeoutMs: 60000,
        });
        screenshots.push({
          displayName,
          label,
          path,
          status: screenshot.status,
          stderr: screenshot.stderr,
        });
      }
    }
    artifacts['s4-route-display-screenshots.json'] = writeJson('s4-route-display-screenshots.json', screenshots);

    const serviceStatus = runAgentJson(['service', 'status'], 's4 service status before closing window A', {
      timeoutMs: 60000,
    });
    artifacts['service-status-before-close-a.json'] = writeJson('service-status-before-close-a.json', serviceStatus);
    const incidents = runAgentJson(['service', 'incidents', '--summary'], 's4 incident summary before closing window A', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-before-close-a.json'] = writeJson('service-incidents-before-close-a.json', incidents);
    const finalizationA = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: openA.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence-window-a.json'] = writeJson('route-bound-finalization-evidence-window-a.json', finalizationA);
    const finalizationB = finalizationA;
    artifacts['route-bound-finalization-evidence-window-b.json'] = writeJson('route-bound-finalization-evidence-window-b.json', finalizationB);

    const closeA = runAgentSessionJson(sessionA, ['tab', 'close', '0'], 's4 close window A target', {
      timeoutMs: 60000,
    });
    artifacts['window-a-close.json'] = writeJson('window-a-close.json', closeA);
    const getUrlBAfterCloseA = runAgentSessionJson(sessionB, ['get', 'url'], 's4 get window B URL after closing A', {
      timeoutMs: 60000,
    });
    artifacts['window-b-get-url-after-close-a.json'] = writeJson('window-b-get-url-after-close-a.json', getUrlBAfterCloseA);
    const serviceStatusAfterCloseA = runAgentJson(['service', 'status'], 's4 service status after closing window A', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-close-a.json'] = writeJson('service-status-after-close-a.json', serviceStatusAfterCloseA);
    const incidentsAfterCloseA = runAgentJson(['service', 'incidents', '--summary'], 's4 incident summary after closing window A', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-close-a.json'] = writeJson('service-incidents-after-close-a.json', incidentsAfterCloseA);

    return {
      artifacts,
      before,
      closeA,
      credentials,
      displayContent,
      finalizationA,
      finalizationB,
      getUrlAAfterNavigate,
      getUrlBAfterCloseA,
      getUrlBAfterNavigate,
      incidents,
      incidentsAfterCloseA,
      navigateA,
      navigateB,
      openA,
      openB,
      refreshA: { json: { data: { result: refreshA } } },
      refreshB: { json: { data: { result: refreshB } } },
      screenshots,
      screenshotA: { status: existsSync(dashboardScreenshotA) ? 0 : 1 },
      screenshotB: { status: existsSync(dashboardScreenshotB) ? 0 : 1 },
      serviceStatus,
      serviceStatusAfterCloseA,
      stateA: { json: { data: { result: operatorA.state } } },
      stateAAfterControls: { json: { data: { result: stateAAfterControls } } },
      stateB: { json: { data: { result: operatorB.state } } },
      stateBAfterControls: { json: { data: { result: stateBAfterControls } } },
      tabNewA,
      tabNewB,
      targets,
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

function s5EarlyFailureCapture({
  artifacts,
  before,
  credentials,
  diagnosticsA = null,
  diagnosticsB = null,
  failedStage,
  failureReason,
  openA,
  openB = null,
  targets = null,
}) {
  return {
    artifacts,
    before,
    closeA: null,
    credentials,
    displayContent: diagnosticsB?.displayContent || diagnosticsA?.displayContent || null,
    failedStage,
    failureReason,
    finalizationA: diagnosticsA?.finalization || null,
    finalizationB: diagnosticsB?.finalization || null,
    getUrlAAfterNavigate: null,
    getUrlBAfterCloseA: null,
    getUrlBAfterNavigate: null,
    incidents: diagnosticsB?.incidents || diagnosticsA?.incidents || null,
    incidentsAfterCloseA: null,
    navigateA: null,
    navigateB: null,
    openA,
    openB,
    refreshA: null,
    refreshB: null,
    screenshots: [],
    screenshotA: { status: 1 },
    screenshotB: { status: 1 },
    serviceStatus: diagnosticsB?.serviceStatus || diagnosticsA?.serviceStatus || null,
    serviceStatusAfterCloseA: null,
    stateA: null,
    stateAAfterControls: null,
    stateB: null,
    stateBAfterControls: null,
    tabNewA: null,
    tabNewB: null,
    targets: targets || {
      browserAId: openA?.json?.data?.browserId || null,
      browserBId: openB?.json?.data?.browserId || null,
      dashboardScreenshotA: null,
      dashboardScreenshotB: null,
      dashboardUrlA: null,
      dashboardUrlB: null,
      displayA: displayNameFromOpen(openA),
      displayB: displayNameFromOpen(openB),
      profileAId: 'p46-s5-profile-a',
      profileBId: 'p46-s5-profile-b',
      routeAId: openA?.json?.data?.routeId || openA?.json?.data?.routeBinding?.routeId || null,
      routeBId: openB?.json?.data?.routeId || openB?.json?.data?.routeBinding?.routeId || null,
      sessionA: 'p46-s5-profile-a',
      sessionB: 'p46-s5-profile-b',
      tabAId: serviceTabIdFromOpen(openA?.json),
      tabBId: serviceTabIdFromOpen(openB?.json),
    },
  };
}

async function captureS5(mode = 's5') {
  const artifacts = {};
  const externalOperators = [];
  const runId = new Date().toISOString().replace(/[:.]/g, '-');
  const scenarioId = mode === 's6' ? 's6' : 's5';
  const isS6 = scenarioId === 's6';
  const profileAId = `p46-${scenarioId}-profile-a`;
  const profileBId = `p46-${scenarioId}-profile-b`;
  const sessionA = `p46-${scenarioId}-profile-a-${runId}`;
  const sessionB = `p46-${scenarioId}-profile-b-${runId}`;
  const routePoolEntryA = 'guacamole-rdp-a';
  const routePoolEntryB = 'guacamole-rdp-b';
  const before = captureBaseline(`${scenarioId}-before`);
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const commonOpenArgs = [
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ];
  const openA = runAgentSessionJson(sessionA, [
    'remote-view',
    'open',
    `https://example.com/?p46=${scenarioId}-profile-a`,
    '--session-name',
    sessionA,
    '--runtime-profile',
    profileAId,
    '--route-pool-entry-id',
    routePoolEntryA,
    ...commonOpenArgs,
  ], `${scenarioId} profile A remote-view open`, {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['profile-a-remote-view-open.json'] = writeJson('profile-a-remote-view-open.json', openA);
  if (!remoteViewOpenReady(openA)) {
    const diagnosticsA = captureS3Diagnostics(`${scenarioId}-after-profile-a-failed-open`, artifacts, openA);
    return s5EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnosticsA,
      failedStage: 'profile_a_remote_view_open',
      failureReason: openA.json?.error || 'profile A remote-view open did not produce operatorVisible.state=ready',
      openA,
    });
  }

  const openB = runAgentSessionJson(sessionB, [
    'remote-view',
    'open',
    `https://example.org/?p46=${scenarioId}-profile-b`,
    '--session-name',
    sessionB,
    '--runtime-profile',
    profileBId,
    '--route-pool-entry-id',
    routePoolEntryB,
    ...commonOpenArgs,
  ], `${scenarioId} profile B remote-view open`, {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['profile-b-remote-view-open.json'] = writeJson('profile-b-remote-view-open.json', openB);
  if (!remoteViewOpenReady(openB)) {
    const diagnosticsA = captureS3Diagnostics(`${scenarioId}-after-profile-b-failed-open-a`, artifacts, openA);
    const diagnosticsB = captureS3Diagnostics(`${scenarioId}-after-profile-b-failed-open-b`, artifacts, openB);
    return s5EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnosticsA,
      diagnosticsB,
      failedStage: 'profile_b_remote_view_open',
      failureReason: openB.json?.error || 'profile B remote-view open did not produce operatorVisible.state=ready',
      openA,
      openB,
    });
  }

  const browserAId = openA.json?.data?.browserId || openA.json?.data?.tab?.browserId || `session:${sessionA}`;
  const browserBId = openB.json?.data?.browserId || openB.json?.data?.tab?.browserId || `session:${sessionB}`;
  const tabAId = serviceTabIdFromOpen(openA.json);
  const tabBId = serviceTabIdFromOpen(openB.json);
  const expectedA = { browserId: browserAId, sessionName: sessionA, tabId: tabAId };
  const expectedB = { browserId: browserBId, sessionName: sessionB, tabId: tabBId };
  const dashboardUrlA = viewerClientDashboardWorkspaceUrl(expectedA);
  const dashboardUrlB = viewerClientDashboardWorkspaceUrl(expectedB);
  const targets = {
    browserAId,
    browserBId,
    dashboardScreenshotA: null,
    dashboardScreenshotB: null,
    dashboardUrlA,
    dashboardUrlB,
    displayA: displayNameFromOpen(openA),
    displayB: displayNameFromOpen(openB),
    profileAId,
    profileBId,
    routeAId: openA.json?.data?.routeId || openA.json?.data?.routeBinding?.routeId || null,
    routeBId: openB.json?.data?.routeId || openB.json?.data?.routeBinding?.routeId || null,
    sessionA,
    sessionB,
    tabAId,
    tabBId,
  };
  artifacts[`${scenarioId}-targets.json`] = writeJson(`${scenarioId}-targets.json`, targets);

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  try {
    const operatorA = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlA,
      expected: expectedA,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-a',
      writeJson,
    });
    externalOperators.push(operatorA);
    artifacts['operator-a-dashboard-state.json'] = writeJson('operator-a-dashboard-state.json', operatorA.state);
    const operatorB = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlB,
      expected: expectedB,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-b',
      writeJson,
    });
    externalOperators.push(operatorB);
    artifacts['operator-b-dashboard-state.json'] = writeJson('operator-b-dashboard-state.json', operatorB.state);
    const refreshA = await clickViewerClientDashboardRefresh(operatorA);
    artifacts['operator-a-refresh-click.json'] = writeJson('operator-a-refresh-click.json', refreshA);
    const refreshB = await clickViewerClientDashboardRefresh(operatorB);
    artifacts['operator-b-refresh-click.json'] = writeJson('operator-b-refresh-click.json', refreshB);

    const dashboardScreenshotA = await captureDashboardScreenshot(operatorA, join(artifactDir, 'operator-a-dashboard.png'));
    targets.dashboardScreenshotA = dashboardScreenshotA;
    artifacts['operator-a-dashboard-screenshot.json'] = writeJson('operator-a-dashboard-screenshot.json', {
      path: dashboardScreenshotA,
      status: existsSync(dashboardScreenshotA) ? 0 : 1,
    });
    const dashboardScreenshotB = await captureDashboardScreenshot(operatorB, join(artifactDir, 'operator-b-dashboard.png'));
    targets.dashboardScreenshotB = dashboardScreenshotB;
    artifacts['operator-b-dashboard-screenshot.json'] = writeJson('operator-b-dashboard-screenshot.json', {
      path: dashboardScreenshotB,
      status: existsSync(dashboardScreenshotB) ? 0 : 1,
    });
    artifacts[`${scenarioId}-targets-after-dashboard.json`] = writeJson(`${scenarioId}-targets-after-dashboard.json`, targets);

    const navigateA = runAgentSessionJson(sessionA, ['open', `https://www.iana.org/domains/reserved?p46=${scenarioId}-profile-a`], `${scenarioId} navigate profile A`, {
      timeoutMs: 120000,
    });
    artifacts['profile-a-navigate.json'] = writeJson('profile-a-navigate.json', navigateA);
    const getUrlAAfterNavigate = runAgentSessionJson(sessionA, ['get', 'url'], `${scenarioId} get profile A URL after navigate`, {
      timeoutMs: 60000,
    });
    artifacts['profile-a-get-url-after-navigate.json'] = writeJson('profile-a-get-url-after-navigate.json', getUrlAAfterNavigate);
    const tabNewA = runAgentSessionJson(sessionA, ['tab', 'new', `https://example.com/?p46=${scenarioId}-profile-a-tab`], `${scenarioId} new tab in profile A`, {
      timeoutMs: 120000,
    });
    artifacts['profile-a-tab-new.json'] = writeJson('profile-a-tab-new.json', tabNewA);

    const navigateB = runAgentSessionJson(sessionB, ['open', `https://example.org/?p46=${scenarioId}-profile-b-navigate`], `${scenarioId} navigate profile B`, {
      timeoutMs: 120000,
    });
    artifacts['profile-b-navigate.json'] = writeJson('profile-b-navigate.json', navigateB);
    const getUrlBAfterNavigate = runAgentSessionJson(sessionB, ['get', 'url'], `${scenarioId} get profile B URL after navigate`, {
      timeoutMs: 60000,
    });
    artifacts['profile-b-get-url-after-navigate.json'] = writeJson('profile-b-get-url-after-navigate.json', getUrlBAfterNavigate);
    const tabNewB = runAgentSessionJson(sessionB, ['tab', 'new', `https://example.org/?p46=${scenarioId}-profile-b-tab`], `${scenarioId} new tab in profile B`, {
      timeoutMs: 120000,
    });
    artifacts['profile-b-tab-new.json'] = writeJson('profile-b-tab-new.json', tabNewB);

    const stateAAfterControls = await waitForViewerClientDashboardState(
      operatorA.cdp,
      expectedA,
      `operator A after ${scenarioId.toUpperCase()} profile controls`,
      60000,
      writeJson,
    );
    artifacts['operator-a-dashboard-state-after-controls.json'] = writeJson('operator-a-dashboard-state-after-controls.json', stateAAfterControls);
    const stateBAfterControls = await waitForViewerClientDashboardState(
      operatorB.cdp,
      expectedB,
      `operator B after ${scenarioId.toUpperCase()} profile controls`,
      60000,
      writeJson,
    );
    artifacts['operator-b-dashboard-state-after-controls.json'] = writeJson('operator-b-dashboard-state-after-controls.json', stateBAfterControls);

    let stateAAfterSwap = null;
    let stateBAfterSwap = null;
    let refreshAAfterSwap = null;
    let refreshBAfterSwap = null;
    let dashboardScreenshotAAfterSwap = null;
    let dashboardScreenshotBAfterSwap = null;
    if (isS6) {
      const navigateOperatorAToB = await navigateDashboardViewerClient(operatorA, dashboardUrlB);
      artifacts['operator-a-swapped-to-profile-b-navigate.json'] = writeJson('operator-a-swapped-to-profile-b-navigate.json', navigateOperatorAToB);
      await waitForDashboardViewerClientPageUrl(operatorA, dashboardUrlB, 'operator A swapped to profile B', {
        artifactName: 'operator-a-swapped-to-profile-b-page-url.json',
        writeJson,
      });
      artifacts['operator-a-swapped-to-profile-b-page-url.json'] = join(artifactDir, 'operator-a-swapped-to-profile-b-page-url.json');
      const reconnectOperatorA = await reconnectDashboardViewerClient(operatorA, 'operator A swapped to profile B', {
        artifactName: 'operator-a-swapped-to-profile-b-reconnect-discovery.json',
        writeJson,
      });
      artifacts['operator-a-swapped-to-profile-b-reconnect.json'] = writeJson('operator-a-swapped-to-profile-b-reconnect.json', reconnectOperatorA);
      stateAAfterSwap = await waitForViewerClientDashboardState(
        operatorA.cdp,
        expectedB,
        'operator A swapped to profile B',
        60000,
        writeJson,
      );
      artifacts['operator-a-swapped-to-profile-b-dashboard-state.json'] = writeJson('operator-a-swapped-to-profile-b-dashboard-state.json', stateAAfterSwap);
      const navigateOperatorBToA = await navigateDashboardViewerClient(operatorB, dashboardUrlA);
      artifacts['operator-b-swapped-to-profile-a-navigate.json'] = writeJson('operator-b-swapped-to-profile-a-navigate.json', navigateOperatorBToA);
      await waitForDashboardViewerClientPageUrl(operatorB, dashboardUrlA, 'operator B swapped to profile A', {
        artifactName: 'operator-b-swapped-to-profile-a-page-url.json',
        writeJson,
      });
      artifacts['operator-b-swapped-to-profile-a-page-url.json'] = join(artifactDir, 'operator-b-swapped-to-profile-a-page-url.json');
      const reconnectOperatorB = await reconnectDashboardViewerClient(operatorB, 'operator B swapped to profile A', {
        artifactName: 'operator-b-swapped-to-profile-a-reconnect-discovery.json',
        writeJson,
      });
      artifacts['operator-b-swapped-to-profile-a-reconnect.json'] = writeJson('operator-b-swapped-to-profile-a-reconnect.json', reconnectOperatorB);
      stateBAfterSwap = await waitForViewerClientDashboardState(
        operatorB.cdp,
        expectedA,
        'operator B swapped to profile A',
        60000,
        writeJson,
      );
      artifacts['operator-b-swapped-to-profile-a-dashboard-state.json'] = writeJson('operator-b-swapped-to-profile-a-dashboard-state.json', stateBAfterSwap);
      refreshAAfterSwap = await clickViewerClientDashboardRefresh(operatorA);
      artifacts['operator-a-swapped-refresh-click.json'] = writeJson('operator-a-swapped-refresh-click.json', refreshAAfterSwap);
      refreshBAfterSwap = await clickViewerClientDashboardRefresh(operatorB);
      artifacts['operator-b-swapped-refresh-click.json'] = writeJson('operator-b-swapped-refresh-click.json', refreshBAfterSwap);
      dashboardScreenshotAAfterSwap = await captureDashboardScreenshot(operatorA, join(artifactDir, 'operator-a-swapped-dashboard.png'));
      artifacts['operator-a-swapped-dashboard-screenshot.json'] = writeJson('operator-a-swapped-dashboard-screenshot.json', {
        path: dashboardScreenshotAAfterSwap,
        status: existsSync(dashboardScreenshotAAfterSwap) ? 0 : 1,
      });
      dashboardScreenshotBAfterSwap = await captureDashboardScreenshot(operatorB, join(artifactDir, 'operator-b-swapped-dashboard.png'));
      artifacts['operator-b-swapped-dashboard-screenshot.json'] = writeJson('operator-b-swapped-dashboard-screenshot.json', {
        path: dashboardScreenshotBAfterSwap,
        status: existsSync(dashboardScreenshotBAfterSwap) ? 0 : 1,
      });
    }

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], `${scenarioId} route display content before close`, {
      timeoutMs: 60000,
    });
    artifacts['display-content-before-close.json'] = writeJson('display-content-before-close.json', displayContent);

    const importCommand = commandExists('import');
    const screenshots = [];
    if (importCommand) {
      for (const [label, displayName] of [['profile-a', targets.displayA], ['profile-b', targets.displayB]]) {
        if (!displayName) continue;
        const path = join(artifactDir, `${label}-route-display-root.png`);
        const screenshot = run(importCommand, ['-display', displayName, '-window', 'root', path], `${scenarioId} ${label} route display screenshot`, {
          timeoutMs: 60000,
        });
        screenshots.push({
          displayName,
          label,
          path,
          status: screenshot.status,
          stderr: screenshot.stderr,
        });
      }
    }
    artifacts[`${scenarioId}-route-display-screenshots.json`] = writeJson(`${scenarioId}-route-display-screenshots.json`, screenshots);

    const serviceStatus = runAgentJson(['service', 'status'], `${scenarioId} service status before closing profile A`, {
      timeoutMs: 60000,
    });
    artifacts['service-status-before-close-a.json'] = writeJson('service-status-before-close-a.json', serviceStatus);
    const incidents = runAgentJson(['service', 'incidents', '--summary'], `${scenarioId} incident summary before closing profile A`, {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-before-close-a.json'] = writeJson('service-incidents-before-close-a.json', incidents);
    const finalizationA = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: openA.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence-profile-a.json'] = writeJson('route-bound-finalization-evidence-profile-a.json', finalizationA);
    const finalizationB = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: openB.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence-profile-b.json'] = writeJson('route-bound-finalization-evidence-profile-b.json', finalizationB);

    const closeA = runAgentSessionJson(sessionA, ['close'], `${scenarioId} close profile A browser`, {
      timeoutMs: 60000,
    });
    artifacts['profile-a-close.json'] = writeJson('profile-a-close.json', closeA);
    const getUrlBAfterCloseA = runAgentSessionJson(sessionB, ['get', 'url'], `${scenarioId} get profile B URL after closing profile A`, {
      timeoutMs: 60000,
    });
    artifacts['profile-b-get-url-after-close-a.json'] = writeJson('profile-b-get-url-after-close-a.json', getUrlBAfterCloseA);
    const serviceStatusAfterCloseA = runAgentJson(['service', 'status'], `${scenarioId} service status after closing profile A`, {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-close-a.json'] = writeJson('service-status-after-close-a.json', serviceStatusAfterCloseA);
    const incidentsAfterCloseA = runAgentJson(['service', 'incidents', '--summary'], `${scenarioId} incident summary after closing profile A`, {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-close-a.json'] = writeJson('service-incidents-after-close-a.json', incidentsAfterCloseA);

    return {
      artifacts,
      before,
      closeA,
      credentials,
      displayContent,
      finalizationA,
      finalizationB,
      getUrlAAfterNavigate,
      getUrlBAfterCloseA,
      getUrlBAfterNavigate,
      incidents,
      incidentsAfterCloseA,
      navigateA,
      navigateB,
      openA,
      openB,
      refreshA: { json: { data: { result: refreshA } } },
      refreshAAfterSwap: { json: { data: { result: refreshAAfterSwap } } },
      refreshB: { json: { data: { result: refreshB } } },
      refreshBAfterSwap: { json: { data: { result: refreshBAfterSwap } } },
      screenshots,
      screenshotA: { status: existsSync(dashboardScreenshotA) ? 0 : 1 },
      screenshotB: { status: existsSync(dashboardScreenshotB) ? 0 : 1 },
      serviceStatus,
      serviceStatusAfterCloseA,
      stateA: { json: { data: { result: operatorA.state } } },
      stateAAfterControls: { json: { data: { result: stateAAfterControls } } },
      stateAAfterSwap: { json: { data: { result: stateAAfterSwap } } },
      stateB: { json: { data: { result: operatorB.state } } },
      stateBAfterControls: { json: { data: { result: stateBAfterControls } } },
      stateBAfterSwap: { json: { data: { result: stateBAfterSwap } } },
      swappedScreenshotA: { status: dashboardScreenshotAAfterSwap && existsSync(dashboardScreenshotAAfterSwap) ? 0 : 1 },
      swappedScreenshotB: { status: dashboardScreenshotBAfterSwap && existsSync(dashboardScreenshotBAfterSwap) ? 0 : 1 },
      tabNewA,
      tabNewB,
      targets,
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

function routeCapacityBlocker(record) {
  const text = `${record?.json?.error || ''}\n${record?.stdout || ''}\n${record?.stderr || ''}`.toLowerCase();
  return text.includes('route') && (
    text.includes('capacity') ||
    text.includes('exhaust') ||
    text.includes('route_pool_exhausted') ||
    text.includes('no route-pool') ||
    text.includes('no route pool') ||
    text.includes('no available route')
  );
}

function displayAccessBlocker(record) {
  const text = `${record?.json?.error || ''}\n${record?.stdout || ''}\n${record?.stderr || ''}`.toLowerCase();
  return (
    text.includes('display_access_grant_failed') ||
    text.includes('display_access_grant_timeout') ||
    text.includes('display_access_failed') ||
    text.includes('x11_auth_denied') ||
    text.includes('route_display_unavailable') ||
    text.includes('route_display_missing')
  );
}

function createDisplayAccessDeniedFixture() {
  const fixtureDir = join(artifactDir, 's8-display-access-denied-bin');
  mkdirSync(fixtureDir, { recursive: true });
  const timeoutPath = join(fixtureDir, 'timeout');
  writeFileSync(timeoutPath, [
    '#!/usr/bin/env sh',
    'echo "p46-s8 display access denied fixture: $*" >&2',
    'exit 1',
    '',
  ].join('\n'), { mode: 0o755 });
  return {
    env: {
      PATH: `${fixtureDir}:${process.env.PATH || ''}`,
    },
    fixtureDir,
    timeoutPath,
  };
}

async function captureS7() {
  const artifacts = {};
  const runId = new Date().toISOString().replace(/[:.]/g, '-');
  const sessionA = `p46-s7-profile-a-${runId}`;
  const sessionB = `p46-s7-profile-b-${runId}`;
  const sessionC = `p46-s7-profile-c-${runId}`;
  const before = captureBaseline('s7-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const poolEnv = routePoolEnv(before.routePool.json);
  const commonOpenArgs = [
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ];
  const openA = runAgentSessionJson(sessionA, [
    'remote-view',
    'open',
    'https://example.com/?p46=s7-profile-a',
    '--session-name',
    sessionA,
    '--runtime-profile',
    'p46-s7-profile-a',
    '--route-pool-entry-id',
    'guacamole-rdp-a',
    ...commonOpenArgs,
  ], 's7 profile A remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['profile-a-remote-view-open.json'] = writeJson('profile-a-remote-view-open.json', openA);
  const openB = runAgentSessionJson(sessionB, [
    'remote-view',
    'open',
    'https://example.org/?p46=s7-profile-b',
    '--session-name',
    sessionB,
    '--runtime-profile',
    'p46-s7-profile-b',
    '--route-pool-entry-id',
    'guacamole-rdp-b',
    ...commonOpenArgs,
  ], 's7 profile B remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['profile-b-remote-view-open.json'] = writeJson('profile-b-remote-view-open.json', openB);

  const statusOccupied = runAgentJson(['service', 'status'], 's7 service status with route pool occupied', {
    timeoutMs: 60000,
  });
  artifacts['service-status-occupied.json'] = writeJson('service-status-occupied.json', statusOccupied);
  const incidentsOccupied = runAgentJson(['service', 'incidents', '--summary'], 's7 incident summary with route pool occupied', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-occupied.json'] = writeJson('service-incidents-occupied.json', incidentsOccupied);

  const thirdOpen = runAgentSessionJson(sessionC, [
    'remote-view',
    'open',
    'https://www.iana.org/domains/reserved?p46=s7-profile-c',
    '--session-name',
    sessionC,
    '--runtime-profile',
    'p46-s7-profile-c',
    ...commonOpenArgs,
  ], 's7 third profile remote-view open while routes occupied', {
    timeoutMs: 120000,
    env: routePoolEnvFromServiceStatus(statusOccupied.json),
  });
  artifacts['profile-c-third-open-while-occupied.json'] = writeJson('profile-c-third-open-while-occupied.json', thirdOpen);
  const statusAfterThird = runAgentJson(['service', 'status'], 's7 service status after third open attempt', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-third-open.json'] = writeJson('service-status-after-third-open.json', statusAfterThird);
  const displayAfterThird = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's7 route display content after third open attempt', {
    timeoutMs: 60000,
  });
  artifacts['display-content-after-third-open.json'] = writeJson('display-content-after-third-open.json', displayAfterThird);

  const closeA = runAgentSessionJson(sessionA, ['close'], 's7 close profile A to release one route', {
    timeoutMs: 60000,
  });
  artifacts['profile-a-close-before-retry.json'] = writeJson('profile-a-close-before-retry.json', closeA);
  const retryC = runAgentSessionJson(sessionC, [
    'remote-view',
    'open',
    'https://www.iana.org/domains/reserved?p46=s7-profile-c-retry',
    '--session-name',
    sessionC,
    '--runtime-profile',
    'p46-s7-profile-c',
    '--route-pool-entry-id',
    'guacamole-rdp-a',
    ...commonOpenArgs,
  ], 's7 retry third profile after releasing route A', {
    timeoutMs: 120000,
  });
  artifacts['profile-c-retry-after-release.json'] = writeJson('profile-c-retry-after-release.json', retryC);
  const statusAfterRetry = runAgentJson(['service', 'status'], 's7 service status after retry', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-retry.json'] = writeJson('service-status-after-retry.json', statusAfterRetry);
  const incidentsAfterRetry = runAgentJson(['service', 'incidents', '--summary'], 's7 incident summary after retry', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-after-retry.json'] = writeJson('service-incidents-after-retry.json', incidentsAfterRetry);
  const finalizationC = routeBoundFinalizationEvidence({
    incidentsJson: incidentsAfterRetry.json,
    openJson: retryC.json,
    statusJson: statusAfterRetry.json,
  });
  artifacts['route-bound-finalization-evidence-profile-c-retry.json'] = writeJson('route-bound-finalization-evidence-profile-c-retry.json', finalizationC);

  return {
    artifacts,
    before,
    closeA,
    displayAfterThird,
    finalizationC,
    incidentsAfterRetry,
    incidentsOccupied,
    openA,
    openB,
    retryC,
    sessions: { sessionA, sessionB, sessionC },
    statusAfterRetry,
    statusAfterThird,
    statusOccupied,
    thirdOpen,
  };
}

async function captureS8() {
  const artifacts = {};
  const runId = new Date().toISOString().replace(/[:.]/g, '-');
  const failureSession = `p46-s8-denied-${runId}`;
  const repairSession = `p46-s8-repair-${runId}`;
  const before = captureBaseline('s8-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const poolEnv = routePoolEnv(before.routePool.json);
  const deniedFixture = createDisplayAccessDeniedFixture();
  artifacts['display-access-denied-fixture.json'] = writeJson('display-access-denied-fixture.json', {
    fixtureDir: deniedFixture.fixtureDir,
    pathPrepended: deniedFixture.fixtureDir,
    timeoutPath: deniedFixture.timeoutPath,
  });
  artifacts['display-access-denied-timeout-shim'] = deniedFixture.timeoutPath;
  const commonOpenArgs = [
    'remote-view',
    'open',
    'https://example.com/?p46=s8-display-access',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
    '--route-pool-entry-id',
    'guacamole-rdp-a',
  ];
  const deniedOpen = runAgentSessionJson(failureSession, [
    ...commonOpenArgs,
    '--session-name',
    failureSession,
    '--runtime-profile',
    'p46-s8-denied-profile',
  ], 's8 route-bound open with simulated display access denial', {
    timeoutMs: 120000,
    env: {
      ...poolEnv,
      ...deniedFixture.env,
    },
  });
  artifacts['display-access-denied-open.json'] = writeJson('display-access-denied-open.json', deniedOpen);
  const statusAfterDenied = runAgentJson(['service', 'status'], 's8 service status after display access denial', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-display-access-denied.json'] = writeJson('service-status-after-display-access-denied.json', statusAfterDenied);
  const incidentsAfterDenied = runAgentJson(['service', 'incidents', '--summary'], 's8 incident summary after display access denial', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-after-display-access-denied.json'] = writeJson('service-incidents-after-display-access-denied.json', incidentsAfterDenied);
  const displayAfterDenied = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's8 route display content after display access denial', {
    timeoutMs: 60000,
  });
  artifacts['display-content-after-display-access-denied.json'] = writeJson('display-content-after-display-access-denied.json', displayAfterDenied);
  const doctorAfterDenied = runAgentJson(['doctor', 'remote-view'], 's8 remote-view doctor after display access denial', {
    timeoutMs: 60000,
  });
  artifacts['remote-view-doctor-after-display-access-denied.json'] = writeJson('remote-view-doctor-after-display-access-denied.json', doctorAfterDenied);

  const repairOpen = runAgentSessionJson(repairSession, [
    ...commonOpenArgs,
    '--session-name',
    repairSession,
    '--runtime-profile',
    'p46-s8-repair-profile',
  ], 's8 route-bound open after display access recovery', {
    timeoutMs: 120000,
    env: routePoolEnvFromServiceStatus(statusAfterDenied.json),
  });
  artifacts['display-access-repair-open.json'] = writeJson('display-access-repair-open.json', repairOpen);
  const statusAfterRepair = runAgentJson(['service', 'status'], 's8 service status after display access repair', {
    timeoutMs: 60000,
  });
  artifacts['service-status-after-display-access-repair.json'] = writeJson('service-status-after-display-access-repair.json', statusAfterRepair);
  const incidentsAfterRepair = runAgentJson(['service', 'incidents', '--summary'], 's8 incident summary after display access repair', {
    timeoutMs: 60000,
  });
  artifacts['service-incidents-after-display-access-repair.json'] = writeJson('service-incidents-after-display-access-repair.json', incidentsAfterRepair);
  const displayAfterRepair = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's8 route display content after display access repair', {
    timeoutMs: 60000,
  });
  artifacts['display-content-after-display-access-repair.json'] = writeJson('display-content-after-display-access-repair.json', displayAfterRepair);
  const doctorAfterRepair = runAgentJson(['doctor', 'remote-view'], 's8 remote-view doctor after display access repair', {
    timeoutMs: 60000,
  });
  artifacts['remote-view-doctor-after-display-access-repair.json'] = writeJson('remote-view-doctor-after-display-access-repair.json', doctorAfterRepair);
  const finalizationRepair = routeBoundFinalizationEvidence({
    incidentsJson: incidentsAfterRepair.json,
    openJson: repairOpen.json,
    statusJson: statusAfterRepair.json,
  });
  artifacts['route-bound-finalization-evidence-repair.json'] = writeJson('route-bound-finalization-evidence-repair.json', finalizationRepair);

  return {
    artifacts,
    before,
    deniedFixture,
    deniedOpen,
    displayAfterDenied,
    displayAfterRepair,
    doctorAfterDenied,
    doctorAfterRepair,
    finalizationRepair,
    incidentsAfterDenied,
    incidentsAfterRepair,
    repairOpen,
    sessions: { failureSession, repairSession },
    statusAfterDenied,
    statusAfterRepair,
  };
}

async function captureS9() {
  const artifacts = {};
  const externalOperators = [];
  const before = captureBaseline('s9-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const duplicateUrl = 'https://example.com/?p46=s9-duplicate';
  const open = runAgentJson([
    'remote-view',
    'open',
    duplicateUrl,
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's9 route-bound open duplicate tab A', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);
  if (!remoteViewOpenReady(open)) {
    const diagnostics = captureS3Diagnostics('s9-after-failed-open', artifacts, open);
    return s3EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnostics,
      failedStage: 'remote_view_open',
      failureReason: open.json?.error || 'remote-view open did not produce operatorVisible.state=ready',
      open,
    });
  }

  const duplicateB = runAgentJson(['tab', 'new', duplicateUrl], 's9 new duplicate same-origin tab B', {
    timeoutMs: 120000,
  });
  artifacts['tab-new-duplicate-b.json'] = writeJson('tab-new-duplicate-b.json', duplicateB);
  const blankTab = runAgentJson(['tab', 'new', 'about:blank'], 's9 new blank tab', {
    timeoutMs: 120000,
  });
  artifacts['tab-new-blank.json'] = writeJson('tab-new-blank.json', blankTab);
  const tabList = runAgentJson(['tab', 'list', '--verbose'], 's9 tab list after duplicate and blank tabs', {
    timeoutMs: 60000,
  });
  artifacts['tab-list-after-setup.json'] = writeJson('tab-list-after-setup.json', tabList);

  const tabs = tabsFromRecord(tabList);
  const duplicateTabs = tabs.filter((tab) => String(tab?.url || '') === duplicateUrl);
  const blankTabs = tabs.filter((tab) => String(tab?.url || '') === 'about:blank' || String(tab?.url || '').startsWith('about:blank'));
  const duplicateA = duplicateTabs[0] || tabs.find((tab) => String(tab?.url || '').includes('p46=s9-duplicate')) || tabs[0] || null;
  const duplicateBTab = duplicateTabs[1] || tabs.find((tab) => serviceTabIdFromTab(tab) === serviceTabIdFromCommandData(duplicateB.json?.data)) || tabs[1] || null;
  const blank = blankTabs[0] || tabs.find((tab) => serviceTabIdFromTab(tab) === serviceTabIdFromCommandData(blankTab.json?.data)) || tabs[2] || null;
  const duplicateAId = serviceTabIdFromOpen(open.json) || serviceTabIdFromTab(duplicateA);
  const duplicateBId = serviceTabIdFromCommandData(duplicateB.json?.data) || serviceTabIdFromTab(duplicateBTab);
  const blankId = serviceTabIdFromCommandData(blankTab.json?.data) || serviceTabIdFromTab(blank);
  const browserId = open.json?.data?.browserId || open.json?.data?.tab?.browserId || 'session:default';
  const sessionName = open.json?.data?.sessionName || 'default';
  const expectedA = { browserId, sessionName, tabId: duplicateAId };
  const expectedB = { browserId, sessionName, tabId: duplicateBId };
  const expectedBlank = { browserId, sessionName, tabId: blankId };
  const dashboardUrlA = viewerClientDashboardWorkspaceUrl(expectedA);
  const dashboardUrlB = viewerClientDashboardWorkspaceUrl(expectedB);
  const dashboardUrlBlank = viewerClientDashboardWorkspaceUrl(expectedBlank);
  artifacts['s9-targets.json'] = writeJson('s9-targets.json', {
    browserId,
    duplicateUrl,
    sessionName,
    tabs: {
      blank: { id: blankId, selector: tabSelector(blankTab.json?.data || blank, 3), source: blank },
      duplicateA: { id: duplicateAId, selector: tabSelector(open.json?.data?.tab || duplicateA, 1), source: duplicateA },
      duplicateB: { id: duplicateBId, selector: tabSelector(duplicateB.json?.data || duplicateBTab, 2), source: duplicateBTab },
    },
    dashboardUrls: {
      blank: dashboardUrlBlank,
      duplicateA: dashboardUrlA,
      duplicateB: dashboardUrlB,
    },
  });

  if (!duplicateAId || !duplicateBId || !blankId || new Set([duplicateAId, duplicateBId, blankId]).size !== 3) {
    const diagnostics = captureS3Diagnostics('s9-after-tab-handle-failure', artifacts, open);
    return {
      ...s3EarlyFailureCapture({
        artifacts,
        before,
        credentials,
        diagnostics,
        failedStage: 'tab_handles',
        failureReason: 'S9 did not obtain three distinct duplicate/blank service tab handles before dashboard launch',
        open,
        tabList,
        tabNew: duplicateB,
        targets: {
          browserId,
          dashboardUrlA,
          dashboardUrlB,
          dashboardUrlBlank,
          sessionName,
          tabAId: duplicateAId,
          tabBId: duplicateBId,
          tabBlankId: blankId,
        },
      }),
      blankTab,
      duplicateB,
    };
  }

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  try {
    const operatorA = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlA,
      expected: expectedA,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-a',
      writeJson,
    });
    externalOperators.push(operatorA);
    artifacts['operator-a-duplicate-a-dashboard-state.json'] = writeJson('operator-a-duplicate-a-dashboard-state.json', operatorA.state);
    const operatorB = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlB,
      expected: expectedB,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-b',
      writeJson,
    });
    externalOperators.push(operatorB);
    artifacts['operator-b-duplicate-b-dashboard-state.json'] = writeJson('operator-b-duplicate-b-dashboard-state.json', operatorB.state);
    const operatorBlank = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: dashboardUrlBlank,
      expected: expectedBlank,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-c',
      stateOptions: { allowRecoveredStaleTab: true },
      writeJson,
    });
    externalOperators.push(operatorBlank);
    artifacts['operator-c-blank-dashboard-state.json'] = writeJson('operator-c-blank-dashboard-state.json', operatorBlank.state);

    const switchBlank = runAgentJson(['tab', tabSelector(blankTab.json?.data || blank, 3)], 's9 switch to blank tab', {
      timeoutMs: 60000,
    });
    artifacts['switch-blank-tab.json'] = writeJson('switch-blank-tab.json', switchBlank);
    const getBlankBeforeNavigate = runAgentJson(['get', 'url'], 's9 get blank URL before navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-blank-url-before-navigate.json'] = writeJson('get-blank-url-before-navigate.json', getBlankBeforeNavigate);
    const navigateBlank = runAgentJson(['open', 'https://www.iana.org/domains/reserved?p46=s9-blank-recovered'], 's9 navigate blank tab to recovered URL', {
      timeoutMs: 120000,
    });
    artifacts['navigate-blank-tab.json'] = writeJson('navigate-blank-tab.json', navigateBlank);
    const getBlankAfterNavigate = runAgentJson(['get', 'url'], 's9 get blank URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-blank-url-after-navigate.json'] = writeJson('get-blank-url-after-navigate.json', getBlankAfterNavigate);
    const tabListAfterBlankNavigate = runAgentJson(['tab', 'list', '--verbose'], 's9 tab list after blank navigate', {
      timeoutMs: 60000,
    });
    artifacts['tab-list-after-blank-navigate.json'] = writeJson('tab-list-after-blank-navigate.json', tabListAfterBlankNavigate);
    const operatorBlankNavigate = await navigateDashboardViewerClient(operatorBlank, dashboardUrlBlank);
    artifacts['operator-c-blank-recovered-navigate.json'] = writeJson('operator-c-blank-recovered-navigate.json', operatorBlankNavigate);
    await waitForDashboardViewerClientPageUrl(
      operatorBlank,
      dashboardUrlBlank,
      'operator C blank recovered dashboard page URL',
      {
        artifactName: 'operator-c-blank-recovered-page-url.json',
        writeJson,
      },
    );
    artifacts['operator-c-blank-recovered-page-url.json'] = join(artifactDir, 'operator-c-blank-recovered-page-url.json');
    const operatorBlankReconnect = await reconnectDashboardViewerClient(operatorBlank, 'operator C blank recovered dashboard', {
      artifactName: 'operator-c-blank-recovered-reconnect-discovery.json',
      writeJson,
    });
    artifacts['operator-c-blank-recovered-reconnect.json'] = writeJson('operator-c-blank-recovered-reconnect.json', operatorBlankReconnect);

    const switchDuplicateA = runAgentJson(['tab', tabSelector(open.json?.data?.tab || duplicateA, 1)], 's9 switch to duplicate tab A', {
      timeoutMs: 60000,
    });
    artifacts['switch-duplicate-a.json'] = writeJson('switch-duplicate-a.json', switchDuplicateA);
    const navigateDuplicateA = runAgentJson(['open', 'https://www.iana.org/domains/reserved?p46=s9-duplicate-a'], 's9 navigate duplicate tab A', {
      timeoutMs: 120000,
    });
    artifacts['navigate-duplicate-a.json'] = writeJson('navigate-duplicate-a.json', navigateDuplicateA);
    const getDuplicateAAfterNavigate = runAgentJson(['get', 'url'], 's9 get duplicate A URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-duplicate-a-url-after-navigate.json'] = writeJson('get-duplicate-a-url-after-navigate.json', getDuplicateAAfterNavigate);

    const switchDuplicateB = runAgentJson(['tab', tabSelector(duplicateB.json?.data || duplicateBTab, 2)], 's9 switch to duplicate tab B', {
      timeoutMs: 60000,
    });
    artifacts['switch-duplicate-b.json'] = writeJson('switch-duplicate-b.json', switchDuplicateB);
    const getDuplicateBAfterANavigate = runAgentJson(['get', 'url'], 's9 get duplicate B URL after duplicate A navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-duplicate-b-url-after-a-navigate.json'] = writeJson('get-duplicate-b-url-after-a-navigate.json', getDuplicateBAfterANavigate);
    const navigateDuplicateB = runAgentJson(['open', 'https://example.org/?p46=s9-duplicate-b'], 's9 navigate duplicate tab B', {
      timeoutMs: 120000,
    });
    artifacts['navigate-duplicate-b.json'] = writeJson('navigate-duplicate-b.json', navigateDuplicateB);
    const getDuplicateBAfterNavigate = runAgentJson(['get', 'url'], 's9 get duplicate B URL after navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-duplicate-b-url-after-navigate.json'] = writeJson('get-duplicate-b-url-after-navigate.json', getDuplicateBAfterNavigate);
    const switchDuplicateABack = runAgentJson(['tab', tabSelector(open.json?.data?.tab || duplicateA, 1)], 's9 switch back to duplicate tab A', {
      timeoutMs: 60000,
    });
    artifacts['switch-duplicate-a-back.json'] = writeJson('switch-duplicate-a-back.json', switchDuplicateABack);
    const getDuplicateAAfterBNavigate = runAgentJson(['get', 'url'], 's9 get duplicate A URL after duplicate B navigate', {
      timeoutMs: 60000,
    });
    artifacts['get-duplicate-a-url-after-b-navigate.json'] = writeJson('get-duplicate-a-url-after-b-navigate.json', getDuplicateAAfterBNavigate);

    const stateAAfterControls = await waitForViewerClientDashboardState(
      operatorA.cdp,
      expectedA,
      'operator A duplicate tab A after S9 controls',
      60000,
      writeJson,
    );
    artifacts['operator-a-duplicate-a-dashboard-state-after-controls.json'] = writeJson('operator-a-duplicate-a-dashboard-state-after-controls.json', stateAAfterControls);
    const stateBAfterControls = await waitForViewerClientDashboardState(
      operatorB.cdp,
      expectedB,
      'operator B duplicate tab B after S9 controls',
      60000,
      writeJson,
    );
    artifacts['operator-b-duplicate-b-dashboard-state-after-controls.json'] = writeJson('operator-b-duplicate-b-dashboard-state-after-controls.json', stateBAfterControls);
    const stateBlankAfterControls = await waitForViewerClientDashboardState(
      operatorBlank.cdp,
      expectedBlank,
      'operator C blank tab after S9 controls',
      60000,
      writeJson,
    );
    artifacts['operator-c-blank-dashboard-state-after-controls.json'] = writeJson('operator-c-blank-dashboard-state-after-controls.json', stateBlankAfterControls);

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's9 route display content after controls', {
      timeoutMs: 60000,
    });
    artifacts['display-content-after-controls.json'] = writeJson('display-content-after-controls.json', displayContent);
    const serviceStatus = runAgentJson(['service', 'status'], 's9 service status after stale-target controls', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-controls.json'] = writeJson('service-status-after-controls.json', serviceStatus);
    const incidents = runAgentJson(['service', 'incidents', '--summary'], 's9 incident summary after stale-target controls', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-controls.json'] = writeJson('service-incidents-after-controls.json', incidents);
    const finalization = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: open.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

    return {
      artifacts,
      before,
      blankTab,
      credentials,
      displayContent,
      duplicateB,
      finalization,
      getBlankAfterNavigate,
      getBlankBeforeNavigate,
      getDuplicateAAfterBNavigate,
      getDuplicateAAfterNavigate,
      getDuplicateBAfterANavigate,
      getDuplicateBAfterNavigate,
      incidents,
      navigateBlank,
      navigateDuplicateA,
      navigateDuplicateB,
      open,
      operatorBlankNavigate: { json: { data: { result: operatorBlankNavigate } } },
      operatorBlankReconnect: { json: { data: { result: operatorBlankReconnect } } },
      serviceStatus,
      stateA: { json: { data: { result: operatorA.state } } },
      stateAAfterControls: { json: { data: { result: stateAAfterControls } } },
      stateB: { json: { data: { result: operatorB.state } } },
      stateBAfterControls: { json: { data: { result: stateBAfterControls } } },
      stateBlank: { json: { data: { result: operatorBlank.state } } },
      stateBlankAfterControls: { json: { data: { result: stateBlankAfterControls } } },
      switchBlank,
      switchDuplicateA,
      switchDuplicateABack,
      switchDuplicateB,
      tabList,
      tabListAfterBlankNavigate,
      targets: {
        blank,
        blankId,
        browserId,
        duplicateA,
        duplicateAId,
        duplicateB: duplicateBTab,
        duplicateBId,
        duplicateUrl,
        sessionName,
      },
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

async function captureS10() {
  const artifacts = {};
  const externalOperators = [];
  let foreignBrowser = null;
  const before = captureBaseline('s10-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const serviceOwnedUrl = 'https://example.com/?p46=s10-service-owned';
  const foreignUrl = 'https://example.org/?p46=s10-foreign-cdp';
  const open = runAgentJson([
    'remote-view',
    'open',
    serviceOwnedUrl,
    '--runtime-profile',
    'p46-s10-service-owned',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's10 route-bound service-owned browser open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open-service-owned.json'] = writeJson('remote-view-open-service-owned.json', open);
  if (!remoteViewOpenReady(open)) {
    const diagnostics = captureS3Diagnostics('s10-after-failed-open', artifacts, open);
    return s3EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnostics,
      failedStage: 'remote_view_open',
      failureReason: open.json?.error || 'remote-view open did not produce operatorVisible.state=ready',
      open,
    });
  }

  foreignBrowser = launchForeignCdpBrowser({
    installDoctorJson: before.installDoctor.json,
    label: 's10',
    targetUrl: foreignUrl,
  });
  try {
    const foreignReady = await foreignBrowser.waitUntilReady();
    artifacts['foreign-cdp-ready.json'] = writeJson('foreign-cdp-ready.json', foreignReady);

    if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
    const browserId = open.json?.data?.browserId || 'session:default';
    const sessionName = open.json?.data?.sessionName || 'default';
    const tabId = serviceTabIdFromOpen(open.json) || null;
    const serviceExpected = { browserId, sessionName, tabId };
    const serviceDashboardUrl = viewerClientDashboardWorkspaceUrl(serviceExpected);
    const operator = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl: serviceDashboardUrl,
      expected: serviceExpected,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-s10',
      writeJson,
    });
    externalOperators.push(operator);
    artifacts['operator-service-owned-dashboard-state.json'] = writeJson('operator-service-owned-dashboard-state.json', operator.state);

    const dashboardSessions = await waitForDashboardJson(
      operator,
      '/api/sessions',
      's10 authenticated dashboard sessions inventory',
      (json) => Array.isArray(json) && json.some((session) =>
        session?.ownership === 'foreign_cdp' &&
          session?.provider === 'detected-cdp' &&
          session?.cdpPort === foreignReady.port &&
          String(session?.profilePath || '') === foreignBrowser.profileDir
      ),
      60000,
    );
    artifacts['dashboard-sessions-inventory.json'] = writeJson('dashboard-sessions-inventory.json', dashboardSessions);
    const foreignSession = (Array.isArray(dashboardSessions) ? dashboardSessions : []).find((session) =>
      session?.ownership === 'foreign_cdp' &&
        session?.provider === 'detected-cdp' &&
        session?.cdpPort === foreignReady.port &&
        String(session?.profilePath || '') === foreignBrowser.profileDir
    ) || null;
    artifacts['foreign-cdp-session.json'] = writeJson('foreign-cdp-session.json', foreignSession);

    const dashboardTabs = foreignReady.port
      ? await waitForDashboardJson(
        operator,
        `/api/session-tabs?port=${foreignReady.port}`,
        's10 authenticated foreign dashboard tab inventory',
        (json) => Array.isArray(json) && json.length > 0,
        60000,
      )
      : [];
    artifacts['foreign-cdp-tabs.json'] = writeJson('foreign-cdp-tabs.json', dashboardTabs);

    const statusAfterForeign = runAgentJson(['service', 'status'], 's10 service status after foreign CDP discovery', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-foreign-cdp.json'] = writeJson('service-status-after-foreign-cdp.json', statusAfterForeign);
    const incidentsAfterForeign = runAgentJson(['service', 'incidents', '--summary'], 's10 incident summary after foreign CDP discovery', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-foreign-cdp.json'] = writeJson('service-incidents-after-foreign-cdp.json', incidentsAfterForeign);
    const finalization = routeBoundFinalizationEvidence({
      incidentsJson: incidentsAfterForeign.json,
      openJson: open.json,
      statusJson: statusAfterForeign.json,
    });
    artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

    const servicePanel = await waitForSelectedWorkspacePanel(
      operator.cdp,
      's10 service-owned selected workspace panel',
      (panel) => panel.selectedWorkspaceId === `browser:${browserId}` && panel.selectedWorkspaceSource === 'service-browser',
      60000,
      'operator-service-owned-selected-workspace-panel.json',
    );
    artifacts['operator-service-owned-selected-workspace-panel.json'] = join(artifactDir, 'operator-service-owned-selected-workspace-panel.json');
    const serviceRefresh = await clickSelectedWorkspaceContextRefresh(operator);
    artifacts['operator-service-owned-context-refresh.json'] = writeJson('operator-service-owned-context-refresh.json', serviceRefresh);
    const servicePanelAfterRefresh = await waitForSelectedWorkspacePanel(
      operator.cdp,
      's10 service-owned selected workspace panel after refresh',
      (panel) => panel.selectedWorkspaceId === `browser:${browserId}` && panel.selectedWorkspaceSource === 'service-browser',
      60000,
      'operator-service-owned-selected-workspace-panel-after-refresh.json',
    );
    artifacts['operator-service-owned-selected-workspace-panel-after-refresh.json'] = join(artifactDir, 'operator-service-owned-selected-workspace-panel-after-refresh.json');

    const foreignDashboardUrl = foreignSession?.session
      ? dashboardWorkspaceUrlForDaemonSession(foreignSession.session)
      : null;
    let foreignNavigate = null;
    let foreignPanel = null;
    let foreignPanelAfterRefresh = null;
    let foreignRefresh = null;
    if (foreignDashboardUrl) {
      foreignNavigate = await navigateDashboardViewerClient(operator, foreignDashboardUrl);
      artifacts['operator-foreign-cdp-dashboard-navigate.json'] = writeJson('operator-foreign-cdp-dashboard-navigate.json', foreignNavigate);
      foreignPanel = await waitForSelectedWorkspacePanel(
        operator.cdp,
        's10 foreign CDP selected workspace panel',
        (panel) => panel.selectedWorkspaceId === `daemon-session:${foreignSession.session}` && panel.selectedWorkspaceSource === 'daemon-session',
        60000,
        'operator-foreign-cdp-selected-workspace-panel.json',
      );
      artifacts['operator-foreign-cdp-selected-workspace-panel.json'] = join(artifactDir, 'operator-foreign-cdp-selected-workspace-panel.json');
      foreignRefresh = await clickSelectedWorkspaceContextRefresh(operator);
      artifacts['operator-foreign-cdp-context-refresh.json'] = writeJson('operator-foreign-cdp-context-refresh.json', foreignRefresh);
      foreignPanelAfterRefresh = await waitForSelectedWorkspacePanel(
        operator.cdp,
        's10 foreign CDP selected workspace panel after refresh',
        (panel) => panel.selectedWorkspaceId === `daemon-session:${foreignSession.session}` && panel.selectedWorkspaceSource === 'daemon-session',
        60000,
        'operator-foreign-cdp-selected-workspace-panel-after-refresh.json',
      );
      artifacts['operator-foreign-cdp-selected-workspace-panel-after-refresh.json'] = join(artifactDir, 'operator-foreign-cdp-selected-workspace-panel-after-refresh.json');
    }

    const serviceNavigateBack = await navigateDashboardViewerClient(operator, serviceDashboardUrl);
    artifacts['operator-service-owned-dashboard-navigate-back.json'] = writeJson('operator-service-owned-dashboard-navigate-back.json', serviceNavigateBack);
    const servicePanelAfterSwitchBack = await waitForSelectedWorkspacePanel(
      operator.cdp,
      's10 service-owned selected workspace panel after switch back',
      (panel) => panel.selectedWorkspaceId === `browser:${browserId}` && panel.selectedWorkspaceSource === 'service-browser',
      60000,
      'operator-service-owned-selected-workspace-panel-after-switch-back.json',
    );
    artifacts['operator-service-owned-selected-workspace-panel-after-switch-back.json'] = join(artifactDir, 'operator-service-owned-selected-workspace-panel-after-switch-back.json');

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's10 route display content after foreign CDP inventory', {
      timeoutMs: 60000,
    });
    artifacts['display-content-after-foreign-cdp.json'] = writeJson('display-content-after-foreign-cdp.json', displayContent);
    const statusAfterDashboard = runAgentJson(['service', 'status'], 's10 service status after dashboard selection switches', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-dashboard-switches.json'] = writeJson('service-status-after-dashboard-switches.json', statusAfterDashboard);
    const incidentsAfterDashboard = runAgentJson(['service', 'incidents', '--summary'], 's10 incident summary after dashboard selection switches', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-dashboard-switches.json'] = writeJson('service-incidents-after-dashboard-switches.json', incidentsAfterDashboard);

    return {
      artifacts,
      before,
      credentials,
      dashboardSessions,
      dashboardTabs,
      displayContent,
      finalization,
      foreignBrowser: {
        launchPath: foreignBrowser.launchPath,
        profileDir: foreignBrowser.profileDir,
        profileBasename: foreignBrowser.profileBasename,
        targetUrl: foreignBrowser.targetUrl,
      },
      foreignNavigate: { json: { data: { result: foreignNavigate } } },
      foreignPanel: { json: { data: { result: foreignPanel } } },
      foreignPanelAfterRefresh: { json: { data: { result: foreignPanelAfterRefresh } } },
      foreignReady,
      foreignRefresh: { json: { data: { result: foreignRefresh } } },
      foreignSession,
      incidentsAfterDashboard,
      incidentsAfterForeign,
      open,
      serviceNavigateBack: { json: { data: { result: serviceNavigateBack } } },
      servicePanel: { json: { data: { result: servicePanel } } },
      servicePanelAfterRefresh: { json: { data: { result: servicePanelAfterRefresh } } },
      servicePanelAfterSwitchBack: { json: { data: { result: servicePanelAfterSwitchBack } } },
      serviceRefresh: { json: { data: { result: serviceRefresh } } },
      serviceStatusAfterDashboard: statusAfterDashboard,
      statusAfterForeign,
      targets: {
        browserId,
        foreignDashboardUrl,
        foreignUrl,
        serviceDashboardUrl,
        serviceOwnedUrl,
        sessionName,
        tabId,
      },
    };
  } finally {
    closeViewerClients(externalOperators);
    if (foreignBrowser) foreignBrowser.close();
  }
}

async function captureS11() {
  const artifacts = {};
  const externalOperators = [];
  const before = captureBaseline('s11-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  const poolEnv = routePoolEnv(before.routePool.json);
  const serviceOwnedUrl = 'https://example.com/?p46=s11-live';
  const open = runAgentJson([
    'remote-view',
    'open',
    serviceOwnedUrl,
    '--runtime-profile',
    'p46-s11-profile',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 's11 route-bound browser open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);
  if (!remoteViewOpenReady(open)) {
    const diagnostics = captureS3Diagnostics('s11-after-failed-open', artifacts, open);
    return s3EarlyFailureCapture({
      artifacts,
      before,
      credentials,
      diagnostics,
      failedStage: 'remote_view_open',
      failureReason: open.json?.error || 'remote-view open did not produce operatorVisible.state=ready',
      open,
    });
  }

  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);
  const browserId = open.json?.data?.browserId || 'session:default';
  const sessionName = open.json?.data?.sessionName || 'default';
  const tabId = serviceTabIdFromOpen(open.json) || null;
  const expected = { browserId, sessionName, tabId };
  const dashboardUrl = viewerClientDashboardWorkspaceUrl(expected);
  const staleTabId = 'target:p46-s11-stale-target';
  const staleExpected = { browserId, sessionName, tabId: staleTabId };
  const staleDashboardUrl = viewerClientDashboardWorkspaceUrl(staleExpected);
  artifacts['s11-targets.json'] = writeJson('s11-targets.json', {
    browserId,
    dashboardUrl,
    serviceOwnedUrl,
    sessionName,
    staleDashboardUrl,
    staleTabId,
    tabId,
  });

  try {
    const operator = await launchDashboardViewerClient({
      artifactDir,
      commandExists,
      credentials,
      dashboardUrl,
      expected,
      installDoctorJson: before.installDoctor.json,
      label: 'operator-s11',
      writeJson,
    });
    externalOperators.push(operator);
    artifacts['operator-initial-dashboard-state.json'] = writeJson('operator-initial-dashboard-state.json', operator.state);
    artifacts['operator-initial-screenshot.png'] = await captureDashboardScreenshot(operator, join(artifactDir, 'operator-initial-screenshot.png'));

    const initialFrameSrc = operator.state?.frameSrc || null;
    const initialGuacamole = initialFrameSrc
      ? curlRecord(initialFrameSrc, 's11 initial direct Guacamole frame URL')
      : { status: null, stdout: '', stderr: 'missing frameSrc', error: 'missing frameSrc' };
    artifacts['direct-guacamole-initial.json'] = writeJson('direct-guacamole-initial.json', initialGuacamole);

    const reload = await reloadDashboardViewerClient(operator, 's11 dashboard reload');
    artifacts['operator-dashboard-reload.json'] = writeJson('operator-dashboard-reload.json', reload);
    const stateAfterReload = await waitForViewerClientDashboardState(
      operator.cdp,
      expected,
      'operator S11 after dashboard reload',
      60000,
      writeJson,
    );
    artifacts['operator-dashboard-state-after-reload.json'] = writeJson('operator-dashboard-state-after-reload.json', stateAfterReload);

    const staleNavigate = await navigateDashboardViewerClient(operator, staleDashboardUrl);
    artifacts['operator-stale-url-navigate.json'] = writeJson('operator-stale-url-navigate.json', staleNavigate);
    const stateAfterStale = await waitForViewerClientDashboardState(
      operator.cdp,
      staleExpected,
      'operator S11 stale dashboard recovery',
      60000,
      writeJson,
      { allowRecoveredLiveTab: true, allowRecoveredStaleTab: true },
    );
    artifacts['operator-dashboard-state-after-stale-url.json'] = writeJson('operator-dashboard-state-after-stale-url.json', stateAfterStale);
    const staleUrlReadback = await cdpEvaluate(operator.cdp, 'location.href');
    artifacts['operator-stale-url-readback.json'] = writeJson('operator-stale-url-readback.json', {
      locationHref: staleUrlReadback,
      requestedUrl: staleDashboardUrl,
      recovered: staleUrlReadback !== staleDashboardUrl,
    });
    const staleReconnect = await reconnectDashboardViewerClient(operator, 'operator S11 stale dashboard', {
      artifactName: 'operator-stale-url-reconnect-discovery.json',
      writeJson,
    });
    artifacts['operator-stale-url-reconnect.json'] = writeJson('operator-stale-url-reconnect.json', staleReconnect);

    const viewportRefresh = await clickViewerClientDashboardRefresh(operator);
    artifacts['operator-viewport-refresh-after-stale-url.json'] = writeJson('operator-viewport-refresh-after-stale-url.json', viewportRefresh);
    const stateAfterViewportRefresh = await waitForViewerClientDashboardState(
      operator.cdp,
      staleExpected,
      'operator S11 after viewport refresh',
      60000,
      writeJson,
      { allowRecoveredLiveTab: true, allowRecoveredStaleTab: true },
    );
    artifacts['operator-dashboard-state-after-viewport-refresh.json'] = writeJson('operator-dashboard-state-after-viewport-refresh.json', stateAfterViewportRefresh);
    artifacts['operator-after-refresh-screenshot.png'] = await captureDashboardScreenshot(operator, join(artifactDir, 'operator-after-refresh-screenshot.png'));

    const directFrameSrc = stateAfterViewportRefresh?.frameSrc || stateAfterStale?.frameSrc || stateAfterReload?.frameSrc || initialFrameSrc || null;
    const directGuacamole = directFrameSrc
      ? curlRecord(directFrameSrc, 's11 direct Guacamole frame URL after stale recovery')
      : { status: null, stdout: '', stderr: 'missing frameSrc', error: 'missing frameSrc' };
    artifacts['direct-guacamole-after-stale-recovery.json'] = writeJson('direct-guacamole-after-stale-recovery.json', directGuacamole);

    const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], 's11 route display content after reload and stale URL recovery', {
      timeoutMs: 60000,
    });
    artifacts['display-content-after-stale-recovery.json'] = writeJson('display-content-after-stale-recovery.json', displayContent);
    const serviceStatus = runAgentJson(['service', 'status'], 's11 service status after reload and stale URL recovery', {
      timeoutMs: 60000,
    });
    artifacts['service-status-after-stale-recovery.json'] = writeJson('service-status-after-stale-recovery.json', serviceStatus);
    const incidents = runAgentJson(['service', 'incidents', '--summary'], 's11 incident summary after reload and stale URL recovery', {
      timeoutMs: 60000,
    });
    artifacts['service-incidents-after-stale-recovery.json'] = writeJson('service-incidents-after-stale-recovery.json', incidents);
    const finalization = routeBoundFinalizationEvidence({
      incidentsJson: incidents.json,
      openJson: open.json,
      statusJson: serviceStatus.json,
    });
    artifacts['route-bound-finalization-evidence.json'] = writeJson('route-bound-finalization-evidence.json', finalization);

    return {
      artifacts,
      before,
      credentials,
      directFrameSrc,
      directGuacamole,
      finalization,
      incidents,
      initialGuacamole,
      open,
      reload: { json: { data: { result: reload } } },
      serviceStatus,
      stateAfterReload: { json: { data: { result: stateAfterReload } } },
      stateAfterStale: { json: { data: { result: stateAfterStale } } },
      stateAfterViewportRefresh: { json: { data: { result: stateAfterViewportRefresh } } },
      staleNavigate: { json: { data: { result: staleNavigate } } },
      staleReconnect: { json: { data: { result: staleReconnect } } },
      targets: {
        browserId,
        dashboardUrl,
        serviceOwnedUrl,
        sessionName,
        staleDashboardUrl,
        staleTabId,
        tabId,
      },
      viewportRefresh: { json: { data: { result: viewportRefresh } } },
      displayContent,
    };
  } finally {
    closeViewerClients(externalOperators);
  }
}

async function captureS12() {
  const artifacts = {};
  const cycles = [];
  const credentials = loadDashboardCredentials();
  artifacts['dashboard-credentials-source.json'] = writeJson('dashboard-credentials-source.json', {
    ok: credentials.ok,
    path: credentials.path,
    reason: credentials.reason,
  });
  if (!credentials.ok) throw new Error(`dashboard credentials unavailable: ${credentials.reason}`);

  for (let index = 1; index <= s12CycleCount; index += 1) {
    const label = `s12-cycle-${String(index).padStart(2, '0')}`;
    const externalOperators = [];
    const cycle = {
      index,
      label,
      artifacts: {},
      failedStage: null,
      failureReason: null,
    };
    const boundaryBefore = captureS12Boundary(`${label}-before`, artifacts);
    cycle.before = boundaryBefore;
    const sessionName = `${label}-${new Date().toISOString().replace(/[:.]/g, '-')}`;
    const runtimeProfile = `${label}-profile`;
    try {
      const open = runAgentSessionJson(sessionName, [
        'remote-view',
        'open',
        `https://example.com/?p46=s12&cycle=${index}`,
        '--session-name',
        sessionName,
        '--runtime-profile',
        runtimeProfile,
        '--browser-build',
        'stealthcdp_chromium',
        '--view-stream-provider',
        'rdp_gateway',
      ], `${label} route-bound browser open`, {
        timeoutMs: 120000,
        env: routePoolEnvFromServiceStatus(boundaryBefore.serviceStatus.json),
      });
      cycle.open = open;
      artifacts[`${label}-remote-view-open.json`] = writeJson(`${label}-remote-view-open.json`, open);
      if (!remoteViewOpenReady(open)) {
        cycle.failedStage = 'remote_view_open';
        cycle.failureReason = open.json?.error || 'remote-view open did not produce operatorVisible.state=ready';
      } else {
        const browserId = open.json?.data?.browserId || open.json?.data?.tab?.browserId || `session:${sessionName}`;
        const tabId = serviceTabIdFromOpen(open.json) || null;
        const expected = { browserId, sessionName, tabId };
        const dashboardUrl = viewerClientDashboardWorkspaceUrl(expected);
        cycle.targets = {
          browserId,
          dashboardUrl,
          displayName: displayNameFromOpen(open),
          runtimeProfile,
          sessionName,
          tabId,
        };
        artifacts[`${label}-targets.json`] = writeJson(`${label}-targets.json`, cycle.targets);

        const operator = await launchDashboardViewerClient({
          artifactDir,
          commandExists,
          credentials,
          dashboardUrl,
          expected,
          installDoctorJson: boundaryBefore.installDoctor?.json,
          label,
          writeJson,
        });
        externalOperators.push(operator);
        artifacts[`${label}-operator-dashboard-state.json`] = writeJson(`${label}-operator-dashboard-state.json`, operator.state);

        const reload = await reloadDashboardViewerClient(operator, `${label} dashboard reload`);
        cycle.reload = { json: { data: { result: reload } } };
        artifacts[`${label}-operator-dashboard-reload.json`] = writeJson(`${label}-operator-dashboard-reload.json`, reload);
        const stateAfterReload = await waitForViewerClientDashboardState(
          operator.cdp,
          expected,
          `${label} after dashboard reload`,
          60000,
          writeJson,
        );
        cycle.stateAfterReload = { json: { data: { result: stateAfterReload } } };
        artifacts[`${label}-operator-dashboard-state-after-reload.json`] = writeJson(`${label}-operator-dashboard-state-after-reload.json`, stateAfterReload);

        const reconnect = await reconnectDashboardViewerClient(operator, `${label} dashboard reconnect`, {
          artifactName: `${label}-operator-reconnect-discovery.json`,
          writeJson,
        });
        cycle.reconnect = { json: { data: { result: reconnect } } };
        artifacts[`${label}-operator-reconnect.json`] = writeJson(`${label}-operator-reconnect.json`, reconnect);
        const viewportRefresh = await clickViewerClientDashboardRefresh(operator);
        cycle.viewportRefresh = { json: { data: { result: viewportRefresh } } };
        artifacts[`${label}-operator-viewport-refresh.json`] = writeJson(`${label}-operator-viewport-refresh.json`, viewportRefresh);

        const navigate = runAgentSessionJson(sessionName, ['open', `https://www.iana.org/domains/reserved?p46=s12&cycle=${index}`], `${label} navigate`, {
          timeoutMs: 120000,
        });
        cycle.navigate = navigate;
        artifacts[`${label}-navigate.json`] = writeJson(`${label}-navigate.json`, navigate);
        const tabNew = runAgentSessionJson(sessionName, ['tab', 'new', `https://example.org/?p46=s12-tab&cycle=${index}`], `${label} new tab`, {
          timeoutMs: 120000,
        });
        cycle.tabNew = tabNew;
        artifacts[`${label}-tab-new.json`] = writeJson(`${label}-tab-new.json`, tabNew);
        const tabList = runAgentSessionJson(sessionName, ['tab', 'list', '--verbose'], `${label} tab list`, {
          timeoutMs: 60000,
        });
        cycle.tabList = tabList;
        artifacts[`${label}-tab-list.json`] = writeJson(`${label}-tab-list.json`, tabList);
        const tabs = tabsFromRecord(tabList);
        const newTab = tabs.find((tab) => tabMatchesCommandData(tab, tabNew.json?.data)) || tabNew.json?.data || tabs[1] || null;
        const switchTab = runAgentSessionJson(sessionName, ['tab', tabSelector(newTab, 2)], `${label} switch tab`, {
          timeoutMs: 60000,
        });
        cycle.switchTab = switchTab;
        artifacts[`${label}-switch-tab.json`] = writeJson(`${label}-switch-tab.json`, switchTab);
        const getUrlAfterSwitch = runAgentSessionJson(sessionName, ['get', 'url'], `${label} get URL after tab switch`, {
          timeoutMs: 60000,
        });
        cycle.getUrlAfterSwitch = getUrlAfterSwitch;
        artifacts[`${label}-get-url-after-switch.json`] = writeJson(`${label}-get-url-after-switch.json`, getUrlAfterSwitch);

        const stateAfterControls = await waitForViewerClientDashboardState(
          operator.cdp,
          expected,
          `${label} after controls`,
          60000,
          writeJson,
          { allowRecoveredLiveTab: true },
        );
        cycle.stateAfterControls = { json: { data: { result: stateAfterControls } } };
        artifacts[`${label}-operator-dashboard-state-after-controls.json`] = writeJson(`${label}-operator-dashboard-state-after-controls.json`, stateAfterControls);
        const frameSrc = stateAfterControls?.frameSrc || stateAfterReload?.frameSrc || null;
        const directGuacamole = frameSrc
          ? curlRecord(frameSrc, `${label} direct Guacamole frame URL`)
          : { status: null, stdout: '', stderr: 'missing frameSrc', error: 'missing frameSrc' };
        cycle.directGuacamole = directGuacamole;
        artifacts[`${label}-direct-guacamole.json`] = writeJson(`${label}-direct-guacamole.json`, directGuacamole);

        const serviceStatusBeforeClose = runAgentJson(['service', 'status'], `${label} service status before close`, {
          timeoutMs: 60000,
        });
        cycle.serviceStatusBeforeClose = serviceStatusBeforeClose;
        artifacts[`${label}-service-status-before-close.json`] = writeJson(`${label}-service-status-before-close.json`, serviceStatusBeforeClose);
        const incidentsBeforeClose = runAgentJson(['service', 'incidents', '--summary'], `${label} incident summary before close`, {
          timeoutMs: 60000,
        });
        cycle.incidentsBeforeClose = incidentsBeforeClose;
        artifacts[`${label}-service-incidents-before-close.json`] = writeJson(`${label}-service-incidents-before-close.json`, incidentsBeforeClose);
        cycle.finalization = routeBoundFinalizationEvidence({
          incidentsJson: incidentsBeforeClose.json,
          openJson: open.json,
          statusJson: serviceStatusBeforeClose.json,
        });
        artifacts[`${label}-route-bound-finalization-evidence.json`] = writeJson(`${label}-route-bound-finalization-evidence.json`, cycle.finalization);
        const displayContent = runJson(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content'], `${label} route display content before close`, {
          timeoutMs: 60000,
        });
        cycle.displayContent = displayContent;
        artifacts[`${label}-display-content-before-close.json`] = writeJson(`${label}-display-content-before-close.json`, displayContent);

        const close = runAgentSessionJson(sessionName, ['close'], `${label} close`, {
          timeoutMs: 60000,
        });
        cycle.close = close;
        artifacts[`${label}-close.json`] = writeJson(`${label}-close.json`, close);
      }
    } finally {
      closeViewerClients(externalOperators);
      cycle.reset = resetRuntime(label);
      const boundaryAfter = captureS12Boundary(`${label}-after`, artifacts);
      cycle.after = boundaryAfter;
      cycle.pressureIncrease = pressureWithinBaseline(boundaryAfter.pressure, boundaryBefore.pressure);
      cycles.push(cycle);
      artifacts[`${label}-cycle-summary.json`] = writeJson(`${label}-cycle-summary.json`, cycle);
    }
  }

  return {
    artifacts,
    credentials,
    cycleCount: s12CycleCount,
    cycles,
  };
}

function captureS3OpenProof() {
  const artifacts = {};
  const before = captureBaseline('s3-open-before');
  for (const [name, path] of Object.entries(before.artifacts)) {
    artifacts[`before-${name}`] = path;
  }
  const poolEnv = routePoolEnv(before.routePool.json);
  const open = runAgentJson([
    'remote-view',
    'open',
    'https://example.com/?p50=s3-open',
    '--browser-build',
    'stealthcdp_chromium',
    '--view-stream-provider',
    'rdp_gateway',
  ], 'p50 s3-open route-bound remote-view open', {
    timeoutMs: 120000,
    env: poolEnv,
  });
  artifacts['remote-view-open.json'] = writeJson('remote-view-open.json', open);
  const diagnostics = captureS3Diagnostics(
    remoteViewOpenReady(open) ? 's3-open-after-open' : 's3-open-after-failed-open',
    artifacts,
    open,
  );
  return {
    artifacts,
    before,
    displayContent: diagnostics.displayContent,
    finalization: diagnostics.finalization,
    incidents: diagnostics.incidents,
    open,
    routePool: diagnostics.routePool,
    serviceStatus: diagnostics.serviceStatus,
  };
}

function evaluateS1(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = capture.open.status === 0 &&
    capture.open.json?.success === true &&
    openData.status === 'opened' &&
    openData.operatorVisible?.state === 'ready';
  const routeId = openData.routeId || openData.routeBinding?.routeId;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const afterOpenUrl = responseUrl(capture.getUrlAfterOpen);
  const afterNavigateUrl = responseUrl(capture.getUrlAfterNavigate);
  const tabListData = capture.tabList.json?.data;
  const tabs = Array.isArray(tabListData) ? tabListData : tabListData?.tabs || [];
  const displayState = displayName === ':14'
    ? capture.displayContent.json?.routes?.B?.displayContent
    : capture.displayContent.json?.routes?.A?.displayContent;
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const activeIncidents = activeIncidentCount(capture.serviceStatus.json);

  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (routeId !== 'guacamole:3' && routeId !== 'guacamole:4') failures.push(`unexpected route ID ${routeId || 'missing'}`);
  if (!displayName) failures.push('remote-view open did not report a display name');
  if (!String(afterOpenUrl || '').startsWith('https://example.com/')) failures.push(`URL after open was ${afterOpenUrl || 'missing'}`);
  if (!String(afterNavigateUrl || '').startsWith('https://www.iana.org/domains/reserved')) failures.push(`URL after navigate was ${afterNavigateUrl || 'missing'}`);
  if (capture.navigate.status !== 0 || capture.navigate.json?.success !== true) failures.push('navigate control failed');
  if (capture.tabNew.status !== 0 || capture.tabNew.json?.success !== true) failures.push('new-tab control failed');
  if (!Array.isArray(tabs) || tabs.length < 2) failures.push('tab list did not show at least two tabs after new-tab control');
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (capture.screenshots.length < 1 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    failures.push('route display screenshot was not captured after S1 controls');
  }
  warnings.push('S1 exercises browser controls through the route-bound session; authenticated dashboard button-click UX remains for a later runner slice');

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      afterNavigateUrl,
      afterOpenUrl,
      displayName,
      displayState: displayState?.state || null,
      routeId,
      screenshotCount: capture.screenshots.length,
      tabCount: Array.isArray(tabs) ? tabs.length : null,
      title: openData.tab?.title || null,
      visibleWindows: (displayState?.windows || []).map((window) => window.title),
    },
  };
}

function evaluateS2(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = capture.open.status === 0 &&
    capture.open.json?.success === true &&
    openData.status === 'opened' &&
    openData.operatorVisible?.state === 'ready';
  const routeId = openData.routeId || openData.routeBinding?.routeId;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const stateA = capture.stateA.json?.data?.result;
  const stateB = capture.stateB.json?.data?.result;
  const stateBAfterNavigate = capture.stateBAfterNavigate.json?.data?.result;
  const urlAfterNavigate = responseUrl(capture.getUrlAfterNavigate);
  const displayState = displayName === ':14'
    ? capture.displayContent.json?.routes?.B?.displayContent
    : capture.displayContent.json?.routes?.A?.displayContent;
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const activeIncidents = activeIncidentCount(capture.serviceStatus.json);
  const browserRows = Object.values(capture.serviceStatus.json?.data?.service_state?.browsers || {})
    .filter((browser) => browser?.profileId === 'p46-s2-profile' || browser?.runtimeProfile === 'p46-s2-profile');

  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  for (const [label, state] of [['operator A', stateA], ['operator B', stateB], ['operator B after navigate', stateBAfterNavigate]]) {
    if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
    if (state?.browserParam !== capture.targets.browserId) failures.push(`${label} dashboard browser param mismatch`);
    if (state?.sessionParam !== capture.targets.sessionName) failures.push(`${label} dashboard session param mismatch`);
    if (capture.targets.tabId && state?.tabParam !== capture.targets.tabId) failures.push(`${label} dashboard tab param mismatch`);
    if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
    if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
  }
  if (stateA?.frameSrc && stateB?.frameSrc && stateA.frameSrc !== stateB.frameSrc) {
    failures.push('operator A and operator B dashboard frames point to different route URLs');
  }
  const refreshResult = capture.refreshB.json?.data?.result;
  if (refreshResult?.clicked !== true) failures.push(`operator B refresh control was not functional: ${JSON.stringify(refreshResult)}`);
  if (!String(urlAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`controlled browser URL after operator A navigate was ${urlAfterNavigate || 'missing'}`);
  }
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (browserRows.length > 1) failures.push(`S2 created ${browserRows.length} p46-s2-profile browser rows instead of one`);
  if (capture.screenshots.length < 1 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    failures.push('route display screenshot was not captured after S2 navigate');
  }
  if (capture.screenshotA.status !== 0 || !existsSync(capture.targets.dashboardScreenshotA)) {
    failures.push('operator A dashboard screenshot was not captured');
  }
  if (capture.screenshotB.status !== 0 || !existsSync(capture.targets.dashboardScreenshotB)) {
    failures.push('operator B dashboard screenshot was not captured');
  }
  warnings.push('S2 dashboard screenshots prove route iframe and controls, while direct Guacamole visual readback remains covered by route-display screenshot and route URL evidence');

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      browserRows: browserRows.map((browser) => ({
        health: browser.health,
        id: browser.id,
        profileId: browser.profileId,
        runtimeProfile: browser.runtimeProfile,
      })),
      displayName,
      displayState: displayState?.state || null,
      frameSrcA: stateA?.frameSrc || null,
      frameSrcB: stateB?.frameSrc || null,
      finalization: capture.finalization,
      operatorAHasViewport: Boolean(stateA?.hasViewport),
      operatorBHasViewport: Boolean(stateB?.hasViewport),
      refreshClicked: refreshResult?.clicked === true,
      routeId,
      screenshotCount: capture.screenshots.length,
      urlAfterNavigate,
      visibleWindows: (displayState?.windows || []).map((window) => window.title),
    },
  };
}

function evaluateS3(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = capture.open.status === 0 &&
    capture.open.json?.success === true &&
    openData.status === 'opened' &&
    openData.operatorVisible?.state === 'ready';
  const routeId = openData.routeId || openData.routeBinding?.routeId;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const stateA = capture.stateA?.json?.data?.result;
  const stateB = capture.stateB?.json?.data?.result;
  const stateAAfterControls = capture.stateAAfterControls?.json?.data?.result;
  const stateBAfterControls = capture.stateBAfterControls?.json?.data?.result;
  const tabAAfterNavigate = responseUrl(capture.getUrlAAfterNavigate || {});
  const tabBAfterANavigate = responseUrl(capture.getUrlBAfterANavigate || {});
  const tabBAfterNavigate = responseUrl(capture.getUrlBAfterNavigate || {});
  const tabAAfterBNavigate = responseUrl(capture.getUrlAAfterBNavigate || {});
  const tabs = tabsFromRecord(capture.tabList);
  const distinctTabIds = Boolean(
    capture.targets.tabAId &&
    capture.targets.tabBId &&
    capture.targets.tabAId !== capture.targets.tabBId
  );
  const displayState = displayName === ':14'
    ? capture.displayContent.json?.routes?.B?.displayContent
    : capture.displayContent.json?.routes?.A?.displayContent;
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const activeIncidents = activeIncidentCount(capture.serviceStatus.json);
  const browserRows = Object.values(capture.serviceStatus.json?.data?.service_state?.browsers || {})
    .filter((browser) => browser?.id === capture.targets.browserId || browser?.profileId === 'default' || browser?.runtimeProfile === 'default');

  if (capture.failedStage) {
    if (capture.failedStage === 'remote_view_open') {
      failures.push(`S3 stopped before dashboard launch because remote-view open failed: ${capture.failureReason}`);
    } else if (capture.failedStage === 'tab_handles') {
      failures.push(`S3 stopped before dashboard launch because tab targeting was unsafe: ${capture.failureReason}`);
    } else {
      failures.push(`S3 stopped before dashboard launch at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    }
    if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
    if (!distinctTabIds) failures.push('S3 did not record two distinct tab IDs');
    if (capture.finalization?.finalized !== true) {
      failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
    warnings.push('S3 failed closed before launching dashboard viewers or tab controls, so this run is valid failure evidence but not a UX pass.');

    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents,
        browserRows: browserRows.map((browser) => ({
          health: browser.health,
          id: browser.id,
          profileId: browser.profileId,
          runtimeProfile: browser.runtimeProfile,
        })),
        displayName,
        displayState: displayState?.state || null,
        distinctTabIds,
        failedStage: capture.failedStage,
        failureReason: capture.failureReason || null,
        finalization: capture.finalization,
        routeId,
        tabCount: tabs.length,
        visibleWindows: (displayState?.windows || []).map((window) => window.title),
      },
    };
  }
  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!distinctTabIds) failures.push('S3 did not record two distinct tab IDs');
  if (tabs.length < 2) failures.push('tab list did not show at least two tabs');
  for (const [label, state, expectedTabId] of [
    ['operator A', stateA, capture.targets.tabAId],
    ['operator B', stateB, capture.targets.tabBId],
    ['operator A after controls', stateAAfterControls, capture.targets.tabAId],
    ['operator B after controls', stateBAfterControls, capture.targets.tabBId],
  ]) {
    if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
    if (state?.browserParam !== capture.targets.browserId) failures.push(`${label} dashboard browser param mismatch`);
    if (state?.sessionParam !== capture.targets.sessionName) failures.push(`${label} dashboard session param mismatch`);
    if (expectedTabId && state?.tabParam !== expectedTabId) failures.push(`${label} dashboard tab param mismatch`);
    if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
    if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
  }
  const refreshA = capture.refreshA.json?.data?.result;
  const refreshB = capture.refreshB.json?.data?.result;
  if (refreshA?.clicked !== true) failures.push(`operator A refresh control was not functional: ${JSON.stringify(refreshA)}`);
  if (refreshB?.clicked !== true) failures.push(`operator B refresh control was not functional: ${JSON.stringify(refreshB)}`);
  if (capture.switchA.status !== 0 || capture.switchA.json?.success !== true) failures.push('switch to tab A failed');
  if (capture.switchB.status !== 0 || capture.switchB.json?.success !== true) failures.push('switch to tab B failed');
  if (capture.switchABack.status !== 0 || capture.switchABack.json?.success !== true) failures.push('switch back to tab A failed');
  if (!String(tabAAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`tab A URL after tab A navigate was ${tabAAfterNavigate || 'missing'}`);
  }
  if (!String(tabBAfterANavigate || '').startsWith('https://example.com/')) {
    failures.push(`tab B URL changed after tab A navigate: ${tabBAfterANavigate || 'missing'}`);
  }
  if (!String(tabBAfterNavigate || '').startsWith('https://example.org/')) {
    failures.push(`tab B URL after tab B navigate was ${tabBAfterNavigate || 'missing'}`);
  }
  if (!String(tabAAfterBNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`tab A URL changed after tab B navigate: ${tabAAfterBNavigate || 'missing'}`);
  }
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (browserRows.length > 1) failures.push(`S3 shows ${browserRows.length} default-profile browser rows instead of one`);
  if (capture.screenshots.length < 1 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    failures.push('route display screenshot was not captured after S3 controls');
  }
  if (capture.screenshotA.status !== 0 || !existsSync(capture.targets.dashboardScreenshotA)) {
    failures.push('operator A dashboard screenshot was not captured');
  }
  if (capture.screenshotB.status !== 0 || !existsSync(capture.targets.dashboardScreenshotB)) {
    failures.push('operator B dashboard screenshot was not captured');
  }
  warnings.push('S3 proves dashboard tab parameters and route visual proof; product behavior for simultaneous independent tab control remains serialized through the shared browser session.');

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      browserRows: browserRows.map((browser) => ({
        health: browser.health,
        id: browser.id,
        profileId: browser.profileId,
        runtimeProfile: browser.runtimeProfile,
      })),
      displayName,
      displayState: displayState?.state || null,
      distinctTabIds,
      finalization: capture.finalization,
      operatorAHasViewport: Boolean(stateA?.hasViewport),
      operatorBHasViewport: Boolean(stateB?.hasViewport),
      refreshAClicked: refreshA?.clicked === true,
      refreshBClicked: refreshB?.clicked === true,
      routeId,
      screenshotCount: capture.screenshots.length,
      tabAAfterBNavigate,
      tabAAfterNavigate,
      tabBAfterANavigate,
      tabBAfterNavigate,
      tabCount: tabs.length,
      visibleWindows: (displayState?.windows || []).map((window) => window.title),
    },
  };
}

function evaluateS4(capture) {
  const failures = [];
  const warnings = [];
  const openAData = capture.openA?.json?.data || {};
  const openBData = capture.openB?.json?.data || {};
  const openASuccess = remoteViewOpenReady(capture.openA);
  const openBSuccess = sameProfileWindowReady(capture.openB);
  const stateA = capture.stateA?.json?.data?.result;
  const stateB = capture.stateB?.json?.data?.result;
  const stateAAfterControls = capture.stateAAfterControls?.json?.data?.result;
  const stateBAfterControls = capture.stateBAfterControls?.json?.data?.result;
  const routeAId = openAData.routeId || openAData.routeBinding?.routeId || capture.targets.routeAId;
  const routeBId = openBData.routeId || openBData.routeBinding?.routeId || capture.targets.routeBId;
  const displayA = displayNameFromOpen(capture.openA) || capture.targets.displayA;
  const displayB = displayNameFromOpen(capture.openB) || capture.targets.displayB;
  const displayAState = displayStateForName(capture.displayContent?.json, displayA);
  const displayBState = displayStateForName(capture.displayContent?.json, displayB);
  const terminalAVisible = displayHasTerminal({ displayContent: displayAState }) || displayAState?.state === 'terminal_only';
  const terminalBVisible = displayHasTerminal({ displayContent: displayBState }) || displayBState?.state === 'terminal_only';
  const urlAAfterNavigate = responseUrl(capture.getUrlAAfterNavigate || {});
  const urlBAfterNavigate = responseUrl(capture.getUrlBAfterNavigate || {});
  const urlBAfterCloseA = responseUrl(capture.getUrlBAfterCloseA || {});
  const activeIncidents = activeIncidentCount(capture.serviceStatus?.json);
  const activeIncidentsAfterCloseA = activeIncidentCount(capture.serviceStatusAfterCloseA?.json);
  const browserRowsBeforeClose = Object.values(capture.serviceStatus?.json?.data?.service_state?.browsers || {})
    .filter((browser) =>
      browser?.id === capture.targets.browserAId ||
      browser?.id === capture.targets.browserBId ||
      browser?.profileId === capture.targets.profileId ||
      browser?.runtimeProfile === capture.targets.profileId
    );
  const browserBAfterClose = capture.serviceStatusAfterCloseA?.json?.data?.service_state?.browsers?.[capture.targets.browserBId] || null;

  if (capture.failedStage) {
    failures.push(`S4 stopped at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    if (!openASuccess) failures.push('window A remote-view open did not produce operatorVisible.state=ready');
    if (!openBSuccess && capture.failedStage !== 'same_profile_multi_process_preflight') {
      failures.push('window B same-profile window open did not produce a targetId');
    }
    if (capture.finalizationA && capture.finalizationA.finalized !== true) {
      failures.push(`window A route-bound finalization incomplete: ${(capture.finalizationA.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (
      capture.finalizationB &&
      capture.finalizationB.finalized !== true &&
      capture.failedStage !== 'same_profile_multi_process_preflight'
    ) {
      failures.push(`window B route-bound finalization incomplete: ${(capture.finalizationB.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (Number.isFinite(activeIncidents) && activeIncidents !== 0) {
      failures.push(`service status reports ${activeIncidents} active incident(s)`);
    }
    warnings.push('S4 failed closed before the full two-window UX proof, so this run is valid failure evidence but not a scenario pass.');
    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents: Number.isFinite(activeIncidents) ? activeIncidents : null,
        displayA,
        displayAState: displayAState?.state || null,
        displayB,
        displayBState: displayBState?.state || null,
        failedStage: capture.failedStage,
        failureReason: capture.failureReason || null,
        finalizationA: capture.finalizationA,
        finalizationB: capture.finalizationB,
        openASuccess,
        openBSuccess,
        routeAId,
        routeBId,
        topologyPreflight: capture.topologyPreflight,
        visibleWindowsA: (displayAState?.windows || []).map((window) => window.title),
        visibleWindowsB: (displayBState?.windows || []).map((window) => window.title),
      },
    };
  }

  if (!openASuccess) failures.push('window A remote-view open did not produce operatorVisible.state=ready');
  if (!openBSuccess) failures.push('window B same-profile window open did not produce a targetId');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!capture.targets.browserAId || !capture.targets.browserBId) failures.push('S4 did not record both browser IDs');
  if (capture.targets.browserAId && capture.targets.browserBId && capture.targets.browserAId !== capture.targets.browserBId) {
    failures.push('S4 same-profile windows did not share one browser ID');
  }
  if (!routeAId || !routeBId) failures.push('S4 did not record both route IDs');
  if (routeAId && routeBId && routeAId !== routeBId) failures.push('S4 same-profile windows did not share one route ID');
  if (!displayA || !displayB) failures.push('S4 did not record both display names');
  if (displayA && displayB && displayA !== displayB) failures.push('S4 same-profile windows did not share one display');
  for (const [label, state, browserId, sessionName, tabId] of [
    ['operator A', stateA, capture.targets.browserAId, capture.targets.sessionA, capture.targets.tabAId],
    ['operator B', stateB, capture.targets.browserBId, capture.targets.sessionB, capture.targets.tabBId],
    ['operator A after controls', stateAAfterControls, capture.targets.browserAId, capture.targets.sessionA, capture.targets.tabAId],
    ['operator B after controls', stateBAfterControls, capture.targets.browserBId, capture.targets.sessionB, capture.targets.tabBId],
  ]) {
    if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
    if (state?.browserParam !== browserId) failures.push(`${label} dashboard browser param mismatch`);
    if (state?.sessionParam !== sessionName) failures.push(`${label} dashboard session param mismatch`);
    if (tabId && state?.tabParam !== tabId) failures.push(`${label} dashboard tab param mismatch`);
    if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
    if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
  }
  const refreshA = capture.refreshA?.json?.data?.result;
  const refreshB = capture.refreshB?.json?.data?.result;
  if (refreshA?.clicked !== true) failures.push(`operator A refresh control was not functional: ${JSON.stringify(refreshA)}`);
  if (refreshB?.clicked !== true) failures.push(`operator B refresh control was not functional: ${JSON.stringify(refreshB)}`);
  if (capture.navigateA.status !== 0 || capture.navigateA.json?.success !== true) failures.push('navigate window A failed');
  if (capture.navigateB.status !== 0 || capture.navigateB.json?.success !== true) failures.push('navigate window B failed');
  if (capture.tabNewA.status !== 0 || capture.tabNewA.json?.success !== true) failures.push('new-tab control in window A failed');
  if (capture.tabNewB.status !== 0 || capture.tabNewB.json?.success !== true) failures.push('new-tab control in window B failed');
  if (!String(urlAAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`window A URL after navigate was ${urlAAfterNavigate || 'missing'}`);
  }
  if (!String(urlBAfterNavigate || '').startsWith('https://example.org/')) {
    failures.push(`window B URL after navigate was ${urlBAfterNavigate || 'missing'}`);
  }
  if (!String(urlBAfterCloseA || '').startsWith('https://example.org/')) {
    failures.push(`window B URL after closing window A was ${urlBAfterCloseA || 'missing'}`);
  }
  if (displayAState?.state !== 'browser_window_visible') {
    failures.push(`window A route display content was ${displayAState?.state || 'missing'}`);
  }
  if (displayBState?.state !== 'browser_window_visible') {
    failures.push(`window B route display content was ${displayBState?.state || 'missing'}`);
  }
  if (terminalAVisible) failures.push(`terminal content visible on window A route display ${displayA}`);
  if (terminalBVisible) failures.push(`terminal content visible on window B route display ${displayB}`);
  if (capture.finalizationA?.finalized !== true) {
    failures.push(`window A route-bound finalization incomplete: ${(capture.finalizationA?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (capture.finalizationB?.finalized !== true) {
    failures.push(`window B route-bound finalization incomplete: ${(capture.finalizationB?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status before close reports ${activeIncidents} active incident(s)`);
  if (activeIncidentsAfterCloseA !== 0) failures.push(`service status after close A reports ${activeIncidentsAfterCloseA} active incident(s)`);
  if (browserRowsBeforeClose.length !== 1) {
    failures.push(`S4 saw ${browserRowsBeforeClose.length} same-profile browser row(s) before close instead of one retained browser process`);
  }
  if (capture.closeA.status !== 0 || capture.closeA.json?.success !== true) failures.push('closing window A failed');
  if (!browserBAfterClose || browserBAfterClose.health !== 'ready') {
    failures.push(`window B browser health after closing A was ${browserBAfterClose?.health || 'missing'}`);
  }
  if (capture.screenshots.length < 2 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    failures.push('route display screenshots were not captured for both S4 windows');
  }
  if (capture.screenshotA.status !== 0 || !existsSync(capture.targets.dashboardScreenshotA)) {
    failures.push('operator A dashboard screenshot was not captured');
  }
  if (capture.screenshotB.status !== 0 || !existsSync(capture.targets.dashboardScreenshotB)) {
    failures.push('operator B dashboard screenshot was not captured');
  }

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      activeIncidentsAfterCloseA,
      browserBAfterClose: browserBAfterClose
        ? {
          health: browserBAfterClose.health,
          id: browserBAfterClose.id,
          profileId: browserBAfterClose.profileId,
          runtimeProfile: browserBAfterClose.runtimeProfile,
        }
        : null,
      browserRowsBeforeClose: browserRowsBeforeClose.map((browser) => ({
        health: browser.health,
        id: browser.id,
        profileId: browser.profileId,
        runtimeProfile: browser.runtimeProfile,
      })),
      displayA,
      displayAState: displayAState?.state || null,
      displayB,
      displayBState: displayBState?.state || null,
      finalizationA: capture.finalizationA,
      finalizationB: capture.finalizationB,
      openASuccess,
      openBSuccess,
      refreshAClicked: refreshA?.clicked === true,
      refreshBClicked: refreshB?.clicked === true,
      routeAId,
      routeBId,
      screenshotCount: capture.screenshots.length,
      urlAAfterNavigate,
      urlBAfterCloseA,
      urlBAfterNavigate,
      visibleWindowsA: (displayAState?.windows || []).map((window) => window.title),
      visibleWindowsB: (displayBState?.windows || []).map((window) => window.title),
    },
  };
}

function evaluateS5(capture) {
  const failures = [];
  const warnings = [];
  const openASuccess = remoteViewOpenReady(capture.openA);
  const openBSuccess = remoteViewOpenReady(capture.openB);
  const stateA = capture.stateA?.json?.data?.result;
  const stateB = capture.stateB?.json?.data?.result;
  const stateAAfterControls = capture.stateAAfterControls?.json?.data?.result;
  const stateBAfterControls = capture.stateBAfterControls?.json?.data?.result;
  const routeAId = capture.openA?.json?.data?.routeId || capture.openA?.json?.data?.routeBinding?.routeId || capture.targets.routeAId;
  const routeBId = capture.openB?.json?.data?.routeId || capture.openB?.json?.data?.routeBinding?.routeId || capture.targets.routeBId;
  const displayA = displayNameFromOpen(capture.openA) || capture.targets.displayA;
  const displayB = displayNameFromOpen(capture.openB) || capture.targets.displayB;
  const displayAState = displayStateForName(capture.displayContent?.json, displayA);
  const displayBState = displayStateForName(capture.displayContent?.json, displayB);
  const terminalAVisible = displayHasTerminal({ displayContent: displayAState }) || displayAState?.state === 'terminal_only';
  const terminalBVisible = displayHasTerminal({ displayContent: displayBState }) || displayBState?.state === 'terminal_only';
  const urlAAfterNavigate = responseUrl(capture.getUrlAAfterNavigate || {});
  const urlBAfterNavigate = responseUrl(capture.getUrlBAfterNavigate || {});
  const urlBAfterCloseA = responseUrl(capture.getUrlBAfterCloseA || {});
  const activeIncidents = activeIncidentCount(capture.serviceStatus?.json);
  const activeIncidentsAfterCloseA = activeIncidentCount(capture.serviceStatusAfterCloseA?.json);
  const browserRowsBeforeClose = Object.values(capture.serviceStatus?.json?.data?.service_state?.browsers || {})
    .filter((browser) =>
      browser?.id === capture.targets.browserAId ||
      browser?.id === capture.targets.browserBId ||
      browser?.profileId === capture.targets.profileAId ||
      browser?.profileId === capture.targets.profileBId ||
      browser?.runtimeProfile === capture.targets.profileAId ||
      browser?.runtimeProfile === capture.targets.profileBId
    );
  const browserBAfterClose = capture.serviceStatusAfterCloseA?.json?.data?.service_state?.browsers?.[capture.targets.browserBId] || null;
  const routePoolBeforeClose = capture.serviceStatus?.json?.data?.service_state?.routePool || {};
  const routePoolAfterCloseA = capture.serviceStatusAfterCloseA?.json?.data?.service_state?.routePool || {};

  if (capture.failedStage) {
    failures.push(`S5 stopped at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    if (!openASuccess) failures.push('profile A remote-view open did not produce operatorVisible.state=ready');
    if (!openBSuccess) failures.push('profile B remote-view open did not produce operatorVisible.state=ready');
    if (capture.finalizationA && capture.finalizationA.finalized !== true) {
      failures.push(`profile A route-bound finalization incomplete: ${(capture.finalizationA.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (capture.finalizationB && capture.finalizationB.finalized !== true) {
      failures.push(`profile B route-bound finalization incomplete: ${(capture.finalizationB.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (Number.isFinite(activeIncidents) && activeIncidents !== 0) {
      failures.push(`service status reports ${activeIncidents} active incident(s)`);
    }
    warnings.push('S5 failed closed before the full two-profile UX proof, so this run is valid failure evidence but not a scenario pass.');
    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents: Number.isFinite(activeIncidents) ? activeIncidents : null,
        displayA,
        displayAState: displayAState?.state || null,
        displayB,
        displayBState: displayBState?.state || null,
        failedStage: capture.failedStage,
        failureReason: capture.failureReason || null,
        finalizationA: capture.finalizationA,
        finalizationB: capture.finalizationB,
        openASuccess,
        openBSuccess,
        routeAId,
        routeBId,
        visibleWindowsA: (displayAState?.windows || []).map((window) => window.title),
        visibleWindowsB: (displayBState?.windows || []).map((window) => window.title),
      },
    };
  }

  if (!openASuccess) failures.push('profile A remote-view open did not produce operatorVisible.state=ready');
  if (!openBSuccess) failures.push('profile B remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!capture.targets.browserAId || !capture.targets.browserBId) failures.push('S5 did not record both browser IDs');
  if (capture.targets.browserAId && capture.targets.browserBId && capture.targets.browserAId === capture.targets.browserBId) {
    failures.push('S5 profile A and profile B shared one browser ID');
  }
  if (!routeAId || !routeBId) failures.push('S5 did not record both route IDs');
  if (routeAId && routeBId && routeAId === routeBId) failures.push('S5 profile A and profile B shared one route ID');
  if (!displayA || !displayB) failures.push('S5 did not record both display names');
  if (displayA && displayB && displayA === displayB) failures.push('S5 profile A and profile B shared one display');
  for (const [label, state, browserId, sessionName, tabId] of [
    ['operator A', stateA, capture.targets.browserAId, capture.targets.sessionA, capture.targets.tabAId],
    ['operator B', stateB, capture.targets.browserBId, capture.targets.sessionB, capture.targets.tabBId],
    ['operator A after controls', stateAAfterControls, capture.targets.browserAId, capture.targets.sessionA, capture.targets.tabAId],
    ['operator B after controls', stateBAfterControls, capture.targets.browserBId, capture.targets.sessionB, capture.targets.tabBId],
  ]) {
    if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
    if (state?.browserParam !== browserId) failures.push(`${label} dashboard browser param mismatch`);
    if (state?.sessionParam !== sessionName) failures.push(`${label} dashboard session param mismatch`);
    if (tabId && state?.tabParam !== tabId) failures.push(`${label} dashboard tab param mismatch`);
    if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
    if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
  }
  const refreshA = capture.refreshA?.json?.data?.result;
  const refreshB = capture.refreshB?.json?.data?.result;
  if (refreshA?.clicked !== true) failures.push(`operator A refresh control was not functional: ${JSON.stringify(refreshA)}`);
  if (refreshB?.clicked !== true) failures.push(`operator B refresh control was not functional: ${JSON.stringify(refreshB)}`);
  if (capture.navigateA.status !== 0 || capture.navigateA.json?.success !== true) failures.push('navigate profile A failed');
  if (capture.navigateB.status !== 0 || capture.navigateB.json?.success !== true) failures.push('navigate profile B failed');
  if (capture.tabNewA.status !== 0 || capture.tabNewA.json?.success !== true) failures.push('new-tab control in profile A failed');
  if (capture.tabNewB.status !== 0 || capture.tabNewB.json?.success !== true) failures.push('new-tab control in profile B failed');
  if (!String(urlAAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`profile A URL after navigate was ${urlAAfterNavigate || 'missing'}`);
  }
  if (!String(urlBAfterNavigate || '').startsWith('https://example.org/')) {
    failures.push(`profile B URL after navigate was ${urlBAfterNavigate || 'missing'}`);
  }
  if (!String(urlBAfterCloseA || '').startsWith('https://example.org/')) {
    failures.push(`profile B URL after closing profile A was ${urlBAfterCloseA || 'missing'}`);
  }
  if (displayAState?.state !== 'browser_window_visible') {
    failures.push(`profile A route display content was ${displayAState?.state || 'missing'}`);
  }
  if (displayBState?.state !== 'browser_window_visible') {
    failures.push(`profile B route display content was ${displayBState?.state || 'missing'}`);
  }
  if (terminalAVisible) failures.push(`terminal content visible on profile A route display ${displayA}`);
  if (terminalBVisible) failures.push(`terminal content visible on profile B route display ${displayB}`);
  if (capture.finalizationA?.finalized !== true) {
    failures.push(`profile A route-bound finalization incomplete: ${(capture.finalizationA?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (capture.finalizationB?.finalized !== true) {
    failures.push(`profile B route-bound finalization incomplete: ${(capture.finalizationB?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status before close reports ${activeIncidents} active incident(s)`);
  if (activeIncidentsAfterCloseA !== 0) failures.push(`service status after close A reports ${activeIncidentsAfterCloseA} active incident(s)`);
  if (browserRowsBeforeClose.length !== 2) failures.push(`S5 saw ${browserRowsBeforeClose.length} profile browser row(s) before close instead of two`);
  if (capture.closeA.status !== 0 || capture.closeA.json?.success !== true) failures.push('closing profile A failed');
  if (!browserBAfterClose || browserBAfterClose.health !== 'ready') {
    failures.push(`profile B browser health after closing A was ${browserBAfterClose?.health || 'missing'}`);
  }
  if (routeAId && routePoolAfterCloseA['guacamole-rdp-a']?.currentRouteAllocationId === routeAId) {
    failures.push('route A remained checked out after closing profile A');
  }
  if (routeBId && routePoolAfterCloseA['guacamole-rdp-b']?.currentRouteAllocationId !== routeBId) {
    failures.push('route B was disturbed after closing profile A');
  }
  if (capture.screenshots.length < 2 || capture.screenshots.some((shot) => shot.status !== 0 || !existsSync(shot.path))) {
    failures.push('route display screenshots were not captured for both S5 profiles');
  }
  if (capture.screenshotA.status !== 0 || !existsSync(capture.targets.dashboardScreenshotA)) {
    failures.push('operator A dashboard screenshot was not captured');
  }
  if (capture.screenshotB.status !== 0 || !existsSync(capture.targets.dashboardScreenshotB)) {
    failures.push('operator B dashboard screenshot was not captured');
  }

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      activeIncidentsAfterCloseA,
      browserBAfterClose: browserBAfterClose
        ? {
          health: browserBAfterClose.health,
          id: browserBAfterClose.id,
          profileId: browserBAfterClose.profileId,
          runtimeProfile: browserBAfterClose.runtimeProfile,
        }
        : null,
      browserRowsBeforeClose: browserRowsBeforeClose.map((browser) => ({
        health: browser.health,
        id: browser.id,
        profileId: browser.profileId,
        runtimeProfile: browser.runtimeProfile,
      })),
      displayA,
      displayAState: displayAState?.state || null,
      displayB,
      displayBState: displayBState?.state || null,
      finalizationA: capture.finalizationA,
      finalizationB: capture.finalizationB,
      openASuccess,
      openBSuccess,
      refreshAClicked: refreshA?.clicked === true,
      refreshBClicked: refreshB?.clicked === true,
      routeAId,
      routeBId,
      routePoolAfterCloseA: {
        a: routePoolAfterCloseA['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolAfterCloseA['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolAfterCloseA['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolAfterCloseA['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
      routePoolBeforeClose: {
        a: routePoolBeforeClose['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolBeforeClose['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolBeforeClose['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolBeforeClose['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
      screenshotCount: capture.screenshots.length,
      urlAAfterNavigate,
      urlBAfterCloseA,
      urlBAfterNavigate,
      visibleWindowsA: (displayAState?.windows || []).map((window) => window.title),
      visibleWindowsB: (displayBState?.windows || []).map((window) => window.title),
    },
  };
}

function evaluateS6(capture) {
  const base = evaluateS5(capture);
  const failures = [...base.failures];
  const warnings = [...base.warnings];
  const stateAAfterSwap = capture.stateAAfterSwap?.json?.data?.result;
  const stateBAfterSwap = capture.stateBAfterSwap?.json?.data?.result;
  const refreshAAfterSwap = capture.refreshAAfterSwap?.json?.data?.result;
  const refreshBAfterSwap = capture.refreshBAfterSwap?.json?.data?.result;

  if (!capture.failedStage) {
    for (const [label, state, browserId, sessionName, tabId] of [
      ['operator A swapped to profile B', stateAAfterSwap, capture.targets.browserBId, capture.targets.sessionB, capture.targets.tabBId],
      ['operator B swapped to profile A', stateBAfterSwap, capture.targets.browserAId, capture.targets.sessionA, capture.targets.tabAId],
    ]) {
      if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
      if (state?.browserParam !== browserId) failures.push(`${label} dashboard browser param mismatch`);
      if (state?.sessionParam !== sessionName) failures.push(`${label} dashboard session param mismatch`);
      if (tabId && state?.tabParam !== tabId) failures.push(`${label} dashboard tab param mismatch`);
      if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
      if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
    }
    if (refreshAAfterSwap?.clicked !== true) {
      failures.push(`operator A swapped refresh control was not functional: ${JSON.stringify(refreshAAfterSwap)}`);
    }
    if (refreshBAfterSwap?.clicked !== true) {
      failures.push(`operator B swapped refresh control was not functional: ${JSON.stringify(refreshBAfterSwap)}`);
    }
    if (capture.swappedScreenshotA.status !== 0) failures.push('operator A swapped dashboard screenshot was not captured');
    if (capture.swappedScreenshotB.status !== 0) failures.push('operator B swapped dashboard screenshot was not captured');
  }

  warnings.push('S6 reuses the S5 two-profile route-bound proof and adds dashboard selection swap verification before profile A cleanup.');
  return {
    ...base,
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      ...base.evidence,
      operatorASwappedBrowserParam: stateAAfterSwap?.browserParam || null,
      operatorASwappedRefreshClicked: refreshAAfterSwap?.clicked === true,
      operatorASwappedSessionParam: stateAAfterSwap?.sessionParam || null,
      operatorBSwappedBrowserParam: stateBAfterSwap?.browserParam || null,
      operatorBSwappedRefreshClicked: refreshBAfterSwap?.clicked === true,
      operatorBSwappedSessionParam: stateBAfterSwap?.sessionParam || null,
      swappedScreenshotA: capture.swappedScreenshotA.status,
      swappedScreenshotB: capture.swappedScreenshotB.status,
    },
  };
}

function evaluateS7(capture) {
  const failures = [];
  const warnings = [];
  const openASuccess = remoteViewOpenReady(capture.openA);
  const openBSuccess = remoteViewOpenReady(capture.openB);
  const thirdFailed = capture.thirdOpen.status !== 0 || capture.thirdOpen.json?.success !== true;
  const thirdCapacityBlocker = routeCapacityBlocker(capture.thirdOpen);
  const retrySucceeded = remoteViewOpenReady(capture.retryC);
  const retryExplicitBlocker = !retrySucceeded && Boolean(capture.retryC.json?.error);
  const activeIncidentsOccupied = activeIncidentCount(capture.statusOccupied.json);
  const activeIncidentsAfterThird = activeIncidentCount(capture.statusAfterThird.json);
  const activeIncidentsAfterRetry = activeIncidentCount(capture.statusAfterRetry.json);
  const browsersAfterThird = Object.values(capture.statusAfterThird.json?.data?.service_state?.browsers || {});
  const profileCBrowserAfterThird = browsersAfterThird.find((browser) =>
    browser?.profileId === 'p46-s7-profile-c' ||
    browser?.runtimeProfile === 'p46-s7-profile-c' ||
    browser?.id === `session:${capture.sessions.sessionC}`
  );
  const routePoolOccupied = capture.statusOccupied.json?.data?.service_state?.routePool || {};
  const routePoolAfterRetry = capture.statusAfterRetry.json?.data?.service_state?.routePool || {};
  const displayStatesAfterThird = displayStates(capture.displayAfterThird.json);
  const terminalDisplays = displayStatesAfterThird.filter((state) => state.terminalVisible || state.state === 'terminal_only');

  if (!openASuccess) failures.push('S7 profile A did not open ready while occupying route A');
  if (!openBSuccess) failures.push('S7 profile B did not open ready while occupying route B');
  if (routePoolOccupied['guacamole-rdp-a']?.state !== 'checked_out') failures.push('route A was not checked out while S7 capacity was occupied');
  if (routePoolOccupied['guacamole-rdp-b']?.state !== 'checked_out') failures.push('route B was not checked out while S7 capacity was occupied');
  if (!thirdFailed) failures.push('third route-bound request succeeded while both route-pool entries were occupied');
  if (!thirdCapacityBlocker) failures.push(`third route-bound request did not return a route-capacity blocker: ${capture.thirdOpen.json?.error || 'missing error'}`);
  if (profileCBrowserAfterThird) failures.push('failed third demand created a retained profile C browser row');
  if (terminalDisplays.length > 0) failures.push(`terminal content visible after third demand on route display(s): ${terminalDisplays.map((item) => item.route).join(', ')}`);
  if (capture.closeA.status !== 0 || capture.closeA.json?.success !== true) failures.push('closing profile A before retry failed');
  if (!retrySucceeded && !retryExplicitBlocker) failures.push('retry after release neither succeeded nor returned an explicit blocker');
  if (retrySucceeded && capture.finalizationC?.finalized !== true) {
    failures.push(`profile C retry route-bound finalization incomplete: ${(capture.finalizationC?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidentsOccupied !== 0) failures.push(`occupied baseline reports ${activeIncidentsOccupied} active incident(s)`);
  if (activeIncidentsAfterThird !== 0) failures.push(`third-demand status reports ${activeIncidentsAfterThird} active incident(s)`);
  if (activeIncidentsAfterRetry !== 0) failures.push(`retry status reports ${activeIncidentsAfterRetry} active incident(s)`);
  if (retrySucceeded && routePoolAfterRetry['guacamole-rdp-a']?.currentRouteAllocationId !== (capture.retryC.json?.data?.routeId || capture.retryC.json?.data?.routeBinding?.routeId)) {
    failures.push('route A was not checked out by the successful profile C retry');
  }

  warnings.push('S7 proves route-pool capacity behavior through service state and display inspection; dashboard fake-row UX proof remains limited to retained-row absence in service status.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidentsAfterRetry,
      activeIncidentsAfterThird,
      activeIncidentsOccupied,
      displayStatesAfterThird,
      openASuccess,
      openBSuccess,
      profileCBrowserAfterThird: profileCBrowserAfterThird
        ? {
          health: profileCBrowserAfterThird.health,
          id: profileCBrowserAfterThird.id,
          profileId: profileCBrowserAfterThird.profileId,
          runtimeProfile: profileCBrowserAfterThird.runtimeProfile,
        }
        : null,
      retryExplicitBlocker: retryExplicitBlocker ? capture.retryC.json?.error : null,
      retrySucceeded,
      routePoolAfterRetry: {
        a: routePoolAfterRetry['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolAfterRetry['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolAfterRetry['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolAfterRetry['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
      routePoolOccupied: {
        a: routePoolOccupied['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolOccupied['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolOccupied['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolOccupied['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
      thirdCapacityBlocker,
      thirdError: capture.thirdOpen.json?.error || null,
      thirdFailed,
    },
  };
}

function evaluateS8(capture) {
  const failures = [];
  const warnings = [];
  const deniedFailed = capture.deniedOpen.status !== 0 || capture.deniedOpen.json?.success !== true;
  const deniedDisplayBlocker = displayAccessBlocker(capture.deniedOpen);
  const repairSucceeded = remoteViewOpenReady(capture.repairOpen);
  const deniedError = capture.deniedOpen.json?.error || null;
  const statusAfterDenied = capture.statusAfterDenied.json?.data?.service_state || {};
  const statusAfterRepair = capture.statusAfterRepair.json?.data?.service_state || {};
  const activeIncidentsAfterDenied = activeIncidentCount(capture.statusAfterDenied.json);
  const activeIncidentsAfterRepair = activeIncidentCount(capture.statusAfterRepair.json);
  const browsersAfterDenied = Object.values(statusAfterDenied.browsers || {});
  const deniedBrowserRow = browsersAfterDenied.find((browser) =>
    browser?.profileId === 'p46-s8-denied-profile' ||
    browser?.runtimeProfile === 'p46-s8-denied-profile' ||
    browser?.id === `session:${capture.sessions.failureSession}`
  );
  const routePoolAfterDenied = statusAfterDenied.routePool || {};
  const routePoolAfterRepair = statusAfterRepair.routePool || {};
  const displayStatesAfterDenied = displayStates(capture.displayAfterDenied.json);
  const displayStatesAfterRepair = displayStates(capture.displayAfterRepair.json);
  const terminalDisplaysAfterDenied = displayStatesAfterDenied.filter((state) => state.terminalVisible || state.state === 'terminal_only');
  const repairOpenData = capture.repairOpen.json?.data || {};
  const repairDisplayName = repairOpenData.routeBinding?.displayName ||
    repairOpenData.verification?.visibleWindowProof?.displayName ||
    null;
  const repairDisplayState = displayStateForName(capture.displayAfterRepair.json, repairDisplayName);
  const repairTerminalVisible = displayHasTerminal({ displayContent: repairDisplayState }) || repairDisplayState?.state === 'terminal_only';
  const displayAccessGrantState = repairOpenData.verification?.displayAccessGrant?.state || null;

  if (!deniedFailed) failures.push('simulated display-access denial unexpectedly succeeded');
  if (!deniedDisplayBlocker) failures.push(`display-access denial did not return a typed display-access blocker: ${deniedError || 'missing error'}`);
  if (!String(deniedError || '').includes('cleanup=')) failures.push('display-access denial did not report cleanup evidence');
  if (deniedBrowserRow) failures.push('failed display-access demand created a retained denied-profile browser row');
  if (routePoolAfterDenied['guacamole-rdp-a']?.state !== 'available') {
    failures.push(`route A was ${routePoolAfterDenied['guacamole-rdp-a']?.state || 'missing'} after display-access denial instead of available`);
  }
  if (routePoolAfterDenied['guacamole-rdp-a']?.currentRouteAllocationId) {
    failures.push('route A retained an allocation after display-access denial');
  }
  if (terminalDisplaysAfterDenied.length > 0) {
    failures.push(`terminal content visible after display-access denial on route display(s): ${terminalDisplaysAfterDenied.map((item) => item.route).join(', ')}`);
  }
  if (activeIncidentsAfterDenied !== 0) failures.push(`display-access denial status reports ${activeIncidentsAfterDenied} active incident(s)`);
  if (!repairSucceeded) failures.push(`display-access repair open did not produce operatorVisible.state=ready: ${capture.repairOpen.json?.error || 'missing error'}`);
  if (repairSucceeded && capture.finalizationRepair?.finalized !== true) {
    failures.push(`display-access repair route-bound finalization incomplete: ${(capture.finalizationRepair?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (repairDisplayState?.state !== 'browser_window_visible') {
    failures.push(`route display content after display-access repair was ${repairDisplayState?.state || 'missing'}`);
  }
  if (repairTerminalVisible) failures.push(`terminal content visible after display-access repair on display ${repairDisplayName}`);
  if (activeIncidentsAfterRepair !== 0) failures.push(`display-access repair status reports ${activeIncidentsAfterRepair} active incident(s)`);
  if (repairSucceeded && routePoolAfterRepair['guacamole-rdp-a']?.currentRouteAllocationId !== (repairOpenData.routeId || repairOpenData.routeBinding?.routeId)) {
    failures.push('route A was not checked out by the successful display-access repair open');
  }
  if (!displayAccessGrantState) failures.push('display-access repair open did not report displayAccessGrant state');

  warnings.push('S8 uses a local timeout shim to simulate display access denial safely; it does not mutate host X11 permissions.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidentsAfterDenied,
      activeIncidentsAfterRepair,
      deniedBrowserRow: deniedBrowserRow
        ? {
          health: deniedBrowserRow.health,
          id: deniedBrowserRow.id,
          profileId: deniedBrowserRow.profileId,
          runtimeProfile: deniedBrowserRow.runtimeProfile,
        }
        : null,
      deniedDisplayBlocker,
      deniedError,
      deniedFailed,
      displayAccessGrantState,
      displayStatesAfterDenied,
      displayStatesAfterRepair,
      repairDisplayName,
      repairDisplayState: repairDisplayState?.state || null,
      repairSucceeded,
      routePoolAfterDenied: {
        a: routePoolAfterDenied['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolAfterDenied['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolAfterDenied['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolAfterDenied['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
      routePoolAfterRepair: {
        a: routePoolAfterRepair['guacamole-rdp-a']?.state || null,
        aAllocation: routePoolAfterRepair['guacamole-rdp-a']?.currentRouteAllocationId || null,
        b: routePoolAfterRepair['guacamole-rdp-b']?.state || null,
        bAllocation: routePoolAfterRepair['guacamole-rdp-b']?.currentRouteAllocationId || null,
      },
    },
  };
}

function evaluateS9(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = remoteViewOpenReady(capture.open);
  const routeId = openData.routeId || openData.routeBinding?.routeId;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const tabs = tabsFromRecord(capture.tabList);
  const tabIds = [
    capture.targets?.duplicateAId,
    capture.targets?.duplicateBId,
    capture.targets?.blankId,
  ].filter(Boolean);
  const distinctTabIds = tabIds.length === 3 && new Set(tabIds).size === 3;
  const duplicateSetupCount = tabs.filter((tab) => String(tab?.url || '') === capture.targets?.duplicateUrl).length;
  const blankBeforeNavigate = responseUrl(capture.getBlankBeforeNavigate || {});
  const blankAfterNavigate = responseUrl(capture.getBlankAfterNavigate || {});
  const duplicateAAfterNavigate = responseUrl(capture.getDuplicateAAfterNavigate || {});
  const duplicateBAfterANavigate = responseUrl(capture.getDuplicateBAfterANavigate || {});
  const duplicateBAfterNavigate = responseUrl(capture.getDuplicateBAfterNavigate || {});
  const duplicateAAfterBNavigate = responseUrl(capture.getDuplicateAAfterBNavigate || {});
  const stateA = capture.stateA?.json?.data?.result;
  const stateB = capture.stateB?.json?.data?.result;
  const stateBlank = capture.stateBlank?.json?.data?.result;
  const stateAAfterControls = capture.stateAAfterControls?.json?.data?.result;
  const stateBAfterControls = capture.stateBAfterControls?.json?.data?.result;
  const stateBlankAfterControls = capture.stateBlankAfterControls?.json?.data?.result;
  const blankInitialRecoveryNotice = Array.isArray(stateBlank?.noticeText) &&
    stateBlank.noticeText.some((text) => /Recovered stale selected tab identity/.test(text));
  const blankInitialExactSelection = Boolean(
    stateBlank?.tabParam &&
      stateBlank.tabParam === capture.targets.blankId &&
      stateBlank.expectedTabId === capture.targets.blankId,
  );
  const blankInitialRecovered = Boolean(
    blankInitialExactSelection ||
      (
        stateBlank?.recoveredStaleTab === true &&
        blankInitialRecoveryNotice &&
        stateBlank?.tabParam
      ),
  );
  const displayState = displayStateForName(capture.displayContent?.json, displayName);
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const activeIncidents = activeIncidentCount(capture.serviceStatus?.json);
  const browserRows = Object.values(capture.serviceStatus?.json?.data?.service_state?.browsers || {})
    .filter((browser) => browser?.id === capture.targets?.browserId || browser?.profileId === 'default' || browser?.runtimeProfile === 'default');

  if (capture.failedStage) {
    failures.push(`S9 stopped before stale-target controls at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
    if (!distinctTabIds) failures.push('S9 did not record three distinct duplicate and blank tab IDs');
    if (capture.finalization?.finalized !== true) {
      failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
    warnings.push('S9 failed closed before dashboard stale-target controls; this is failure evidence, not a scenario pass.');
    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents,
        distinctTabIds,
        failedStage: capture.failedStage,
        failureReason: capture.failureReason || null,
        routeId,
        tabCount: tabs.length,
      },
    };
  }

  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!distinctTabIds) failures.push('S9 did not record three distinct duplicate and blank tab IDs');
  if (duplicateSetupCount < 2) failures.push(`S9 setup found ${duplicateSetupCount} duplicate same-origin tab(s) instead of at least two`);
  if (!String(blankBeforeNavigate || '').startsWith('about:blank')) {
    failures.push(`blank tab initial URL was ${blankBeforeNavigate || 'missing'}`);
  }
  if (!String(blankAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`blank tab did not recover to the navigated URL: ${blankAfterNavigate || 'missing'}`);
  }
  if (!blankInitialRecovered) {
    failures.push('operator C initial blank tab state neither preserved the requested blank target nor proved stale selected-tab recovery before live readiness');
  }
  if (!String(duplicateAAfterNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`duplicate tab A URL after navigate was ${duplicateAAfterNavigate || 'missing'}`);
  }
  if (!String(duplicateBAfterANavigate || '').startsWith('https://example.com/')) {
    failures.push(`duplicate tab B changed after duplicate A navigate: ${duplicateBAfterANavigate || 'missing'}`);
  }
  if (!String(duplicateBAfterNavigate || '').startsWith('https://example.org/')) {
    failures.push(`duplicate tab B URL after navigate was ${duplicateBAfterNavigate || 'missing'}`);
  }
  if (!String(duplicateAAfterBNavigate || '').startsWith('https://www.iana.org/domains/reserved')) {
    failures.push(`duplicate tab A changed after duplicate B navigate: ${duplicateAAfterBNavigate || 'missing'}`);
  }
  for (const [label, state, expectedTabId] of [
    ['operator A duplicate tab A', stateA, capture.targets.duplicateAId],
    ['operator B duplicate tab B', stateB, capture.targets.duplicateBId],
    ['operator A duplicate tab A after controls', stateAAfterControls, capture.targets.duplicateAId],
    ['operator B duplicate tab B after controls', stateBAfterControls, capture.targets.duplicateBId],
    ['operator C blank tab after controls', stateBlankAfterControls, capture.targets.blankId],
  ]) {
    if (!state?.hasViewport || !state?.hasFrame) failures.push(`${label} dashboard did not show a remote viewport iframe`);
    if (state?.browserParam !== capture.targets.browserId) failures.push(`${label} dashboard browser param mismatch`);
    if (state?.sessionParam !== capture.targets.sessionName) failures.push(`${label} dashboard session param mismatch`);
    if (expectedTabId && state?.tabParam !== expectedTabId) failures.push(`${label} dashboard tab param mismatch`);
    if (!state?.hasRefreshButton) failures.push(`${label} dashboard missing refresh control`);
    if (state?.hasPasswordInput) failures.push(`${label} dashboard still shows password input after login`);
  }
  if (!stateBlank?.hasViewport || !stateBlank?.hasFrame) failures.push('operator C initial blank tab dashboard did not show a remote viewport iframe');
  if (stateBlank?.browserParam !== capture.targets.browserId) failures.push('operator C initial blank tab dashboard browser param mismatch');
  if (stateBlank?.sessionParam !== capture.targets.sessionName) failures.push('operator C initial blank tab dashboard session param mismatch');
  if (!stateBlank?.hasRefreshButton) failures.push('operator C initial blank tab dashboard missing refresh control');
  if (stateBlank?.hasPasswordInput) failures.push('operator C initial blank tab dashboard still shows password input after login');
  if (!blankInitialExactSelection && stateBlank?.recoveredStaleTab !== true) {
    failures.push('operator C initial blank tab dashboard did not mark stale blank tab identity as recovered');
  }
  if (capture.operatorBlankNavigate?.json?.data?.result?.ok !== true) failures.push('operator C blank recovered dashboard navigation failed');
  if (capture.operatorBlankReconnect?.json?.data?.result?.reconnected !== true) failures.push('operator C blank recovered dashboard reconnect failed');
  if (capture.switchBlank.status !== 0 || capture.switchBlank.json?.success !== true) failures.push('switch to blank tab failed');
  if (capture.switchDuplicateA.status !== 0 || capture.switchDuplicateA.json?.success !== true) failures.push('switch to duplicate tab A failed');
  if (capture.switchDuplicateB.status !== 0 || capture.switchDuplicateB.json?.success !== true) failures.push('switch to duplicate tab B failed');
  if (capture.switchDuplicateABack.status !== 0 || capture.switchDuplicateABack.json?.success !== true) failures.push('switch back to duplicate tab A failed');
  if (capture.navigateBlank.status !== 0 || capture.navigateBlank.json?.success !== true) failures.push('navigate blank tab failed');
  if (capture.navigateDuplicateA.status !== 0 || capture.navigateDuplicateA.json?.success !== true) failures.push('navigate duplicate tab A failed');
  if (capture.navigateDuplicateB.status !== 0 || capture.navigateDuplicateB.json?.success !== true) failures.push('navigate duplicate tab B failed');
  if (displayState?.state !== 'browser_window_visible') {
    failures.push(`route display content after S9 controls was ${displayState?.state || 'missing'}`);
  }
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (browserRows.length > 1) failures.push(`S9 shows ${browserRows.length} default-profile browser rows instead of one`);

  warnings.push('S9 proves duplicate and blank tab target recovery through dashboard tab params plus serialized browser tab controls.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      blankAfterNavigate,
      blankBeforeNavigate,
      blankInitialRecovered,
      blankInitialExactSelection,
      blankInitialRecoveredTabParam: stateBlank?.tabParam || null,
      browserRows: browserRows.map((browser) => ({
        health: browser.health,
        id: browser.id,
        profileId: browser.profileId,
        runtimeProfile: browser.runtimeProfile,
      })),
      displayName,
      displayState: displayState?.state || null,
      distinctTabIds,
      duplicateAAfterBNavigate,
      duplicateAAfterNavigate,
      duplicateBAfterANavigate,
      duplicateBAfterNavigate,
      duplicateSetupCount,
      finalization: capture.finalization,
      routeId,
      tabIds,
      visibleWindows: (displayState?.windows || []).map((window) => window.title),
    },
  };
}

function actionById(panel, id) {
  return (panel?.actions || []).find((action) => action?.id === id) || null;
}

function evaluateS10(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = remoteViewOpenReady(capture.open);
  const routeId = openData.routeId || openData.routeBinding?.routeId || null;
  const displayAllocationId = openData.displayAllocationId || openData.routeBinding?.displayAllocationId || null;
  const routePoolEntryId = openData.routePoolEntryId || openData.routeBinding?.routePoolEntryId || null;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const foreignSession = capture.foreignSession;
  const foreignPanel = capture.foreignPanel?.json?.data?.result;
  const foreignPanelAfterRefresh = capture.foreignPanelAfterRefresh?.json?.data?.result;
  const servicePanel = capture.servicePanel?.json?.data?.result;
  const servicePanelAfterRefresh = capture.servicePanelAfterRefresh?.json?.data?.result;
  const servicePanelAfterSwitchBack = capture.servicePanelAfterSwitchBack?.json?.data?.result;
  const foreignActions = foreignPanel?.actions || [];
  const serviceActions = servicePanel?.actions || [];
  const foreignDisabledMutationActions = ['focus', 'view', 'control', 'add-tab', 'repair', 'close', 'kill', 'borrow-control']
    .every((id) => {
      const action = actionById(foreignPanel, id);
      return !action || action.enabled === false;
    });
  const foreignReadOnlyCapabilitiesPresent = Boolean(
    foreignSession?.capabilities?.inspect === true &&
      foreignSession?.capabilities?.stream === true &&
      foreignSession?.capabilities?.screenshot === true,
  );
  const foreignReadOnlyActionsPresent = ['inspect', 'stream', 'screenshot']
    .every((id) => foreignActions.some((action) => action?.id === id)) ||
      (!foreignPanel?.panelReady && foreignReadOnlyCapabilitiesPresent);
  const foreignReadOnlyFrontendDisabled = ['inspect', 'stream', 'screenshot']
    .every((id) => {
      const action = actionById(foreignPanel, id);
      return action && action.enabled === false && /not wired|inspect|stream|screenshot/i.test(action.reason || action.text || '');
    }) || !foreignPanel?.panelReady;
  const serviceControlReady = Boolean(
    (
      !servicePanel?.panelReady ||
      (
        actionById(servicePanel, 'view')?.enabled === true &&
        actionById(servicePanel, 'control')?.enabled === true
      )
    ) &&
      servicePanel?.viewport?.hasFrame === true &&
      servicePanel?.viewport?.hasRefreshButton === true,
  );
  const displayState = displayStateForName(capture.displayContent?.json, displayName);
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const activeIncidents = activeIncidentCount(capture.serviceStatusAfterDashboard?.json || capture.statusAfterForeign?.json);
  const serviceState = capture.serviceStatusAfterDashboard?.json?.data?.service_state ||
    capture.statusAfterForeign?.json?.data?.service_state ||
    {};
  const routePoolEntry = routePoolEntryId ? serviceState.routePool?.[routePoolEntryId] : null;
  const route = routeId ? serviceState.remoteViewRoutes?.[routeId] : null;
  const serviceBrowser = capture.targets?.browserId ? serviceState.browsers?.[capture.targets.browserId] : null;
  const serviceBrowserHasRoute = Boolean(
    serviceBrowser?.displayAllocationId === displayAllocationId &&
      serviceBrowser?.viewStreams?.some((stream) => stream?.routeId === routeId && stream?.displayAllocationId === displayAllocationId),
  );
  const foreignRouteBorrowed = Boolean(
    foreignSession?.session &&
      (
        foreignPanel?.facts?.Route ||
        foreignPanel?.facts?.Stream && foreignPanel.facts.Stream !== 'not reported' ||
        foreignPanel?.viewport?.hasFrame ||
        foreignPanel?.viewport?.frameSrc ||
        String(foreignPanel?.viewport?.text || '').includes(String(routeId || 'route-id-not-set')) ||
        String(foreignPanel?.viewport?.text || '').includes(String(displayAllocationId || 'display-allocation-not-set'))
      )
  );
  const foreignContextStable = Boolean(
    foreignPanel?.selectedWorkspaceId &&
      foreignPanelAfterRefresh?.selectedWorkspaceId === foreignPanel.selectedWorkspaceId &&
      foreignPanelAfterRefresh?.selectedWorkspaceSource === 'daemon-session'
  );
  const serviceContextStable = Boolean(
    servicePanel?.selectedWorkspaceId === `browser:${capture.targets?.browserId}` &&
      servicePanelAfterRefresh?.selectedWorkspaceId === servicePanel.selectedWorkspaceId &&
      servicePanelAfterSwitchBack?.selectedWorkspaceId === servicePanel.selectedWorkspaceId &&
      servicePanelAfterSwitchBack?.selectedWorkspaceSource === 'service-browser'
  );

  if (capture.failedStage) {
    failures.push(`S10 stopped before foreign-CDP dashboard inventory at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
    if (capture.finalization?.finalized !== true) {
      failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents,
        failedStage: capture.failedStage,
        foreignDetected: false,
        routeId,
      },
    };
  }

  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!foreignSession) failures.push('dashboard sessions inventory did not contain the launched foreign CDP browser');
  if (foreignSession?.ownership !== 'foreign_cdp') failures.push(`foreign row ownership was ${foreignSession?.ownership || 'missing'}`);
  if (foreignSession?.provider !== 'detected-cdp') failures.push(`foreign row provider was ${foreignSession?.provider || 'missing'}`);
  if (foreignSession?.detected !== true) failures.push('foreign row was not marked detected');
  if (foreignSession?.addressability !== 'cdp_reachable') failures.push(`foreign row addressability was ${foreignSession?.addressability || 'missing'}`);
  if (foreignSession?.capabilities?.lifecycle !== false) failures.push('foreign row lifecycle capability was not disabled');
  if (foreignSession?.capabilities?.mutateRequiresBorrow !== true) failures.push('foreign row did not require borrow before mutation');
  if (foreignSession?.cdpPort !== capture.foreignReady?.port) failures.push('foreign row CDP port did not match launched browser');
  if (!Array.isArray(capture.dashboardTabs) || capture.dashboardTabs.length === 0) failures.push('dashboard tab inventory did not read the foreign CDP tab list');
  if (!foreignPanel?.contextReady) failures.push('foreign selected workspace context did not render');
  if (!foreignPanel?.panelReady) warnings.push('foreign selected workspace detail panel was not mounted; S10 used viewport-route context and session capabilities as evidence');
  if (foreignPanel?.selectedWorkspaceSource !== 'daemon-session') failures.push(`foreign selected workspace source was ${foreignPanel?.selectedWorkspaceSource || 'missing'}`);
  if (foreignPanel?.selectedWorkspaceId !== `daemon-session:${foreignSession?.session}`) failures.push('foreign selected workspace id did not match detected daemon session');
  if (foreignPanel?.panelReady && !/detected-non-owned-browser/i.test(foreignPanel?.facts?.['Inventory class'] || foreignPanel?.textSample || '')) {
    failures.push('foreign selected workspace panel did not identify detected non-owned inventory class');
  }
  if (!/not agent-browser service-owned|foreign CDP|detected external Chrome/i.test(foreignPanel?.textSample || '')) {
    failures.push('foreign selected workspace panel did not visibly mark the row as non-owned foreign CDP');
  }
  if (!foreignDisabledMutationActions) failures.push('foreign selected workspace exposed a mutation, lifecycle, route, or borrow control as runnable');
  if (!foreignReadOnlyActionsPresent) failures.push('foreign selected workspace did not list read-only inspect, stream, and screenshot actions');
  if (!foreignReadOnlyFrontendDisabled) {
    warnings.push('foreign read-only actions are advertised by the node model but disabled in the compact panel because they are not frontend-wired yet');
  }
  if (foreignRouteBorrowed) failures.push('foreign selected workspace borrowed service-owned route, stream, or display state');
  if (!foreignContextStable) failures.push('foreign selected workspace context did not remain stable across refresh');
  if (!servicePanel?.contextReady) failures.push('service-owned selected workspace context did not render');
  if (!servicePanel?.panelReady) warnings.push('service-owned selected workspace detail panel was not mounted; S10 used viewport-route context as evidence');
  if (servicePanel?.selectedWorkspaceSource !== 'service-browser') failures.push(`service-owned selected workspace source was ${servicePanel?.selectedWorkspaceSource || 'missing'}`);
  if (!serviceControlReady) failures.push('service-owned row did not remain view/control ready with a remote viewport frame');
  if (!serviceContextStable) failures.push('service-owned selected workspace context did not survive refresh and foreign row switch-back');
  if (capture.foreignNavigate?.json?.data?.result?.ok !== true) failures.push('foreign dashboard navigation failed');
  if (capture.serviceNavigateBack?.json?.data?.result?.ok !== true) failures.push('service-owned dashboard navigation back failed');
  if (capture.serviceRefresh?.json?.data?.result?.clicked !== true) failures.push('service-owned selected workspace refresh was not clickable');
  if (capture.foreignRefresh?.json?.data?.result?.clicked !== true) failures.push('foreign selected workspace refresh was not clickable');
  if (displayState?.state !== 'browser_window_visible') {
    failures.push(`route display content after S10 selection switches was ${displayState?.state || 'missing'}`);
  }
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (route?.browserId !== capture.targets?.browserId) failures.push('route is no longer bound to the service-owned browser');
  if (routePoolEntry?.currentRouteAllocationId !== routeId) failures.push('route-pool entry is no longer checked out to the service-owned route');
  if (!serviceBrowserHasRoute) failures.push('service-owned browser no longer carries the route-bound view stream');
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);

  warnings.push('S10 uses a real process-scanned foreign CDP browser; mutation/adoption remains intentionally disabled in the dashboard compact action surface.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      displayName,
      foreignActionIds: foreignActions.map((action) => ({
        enabled: action.enabled,
        id: action.id,
        reason: action.reason,
      })),
      foreignCdpPort: foreignSession?.cdpPort || null,
      foreignContextStable,
      foreignDetected: Boolean(foreignSession),
      foreignProfileDir: capture.foreignBrowser?.profileDir || null,
      foreignRouteBorrowed,
      foreignSession: foreignSession?.session || null,
      routeId,
      routePoolEntryId,
      serviceActionIds: serviceActions.map((action) => ({
        enabled: action.enabled,
        id: action.id,
        reason: action.reason,
      })),
      serviceBrowserHasRoute,
      serviceContextStable,
      serviceControlReady,
    },
  };
}

function evaluateS11(capture) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = remoteViewOpenReady(capture.open);
  const routeId = openData.routeId || openData.routeBinding?.routeId || null;
  const displayAllocationId = openData.displayAllocationId || openData.routeBinding?.displayAllocationId || null;
  const routePoolEntryId = openData.routePoolEntryId || openData.routeBinding?.routePoolEntryId || null;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const stateAfterReload = capture.stateAfterReload?.json?.data?.result;
  const stateAfterStale = capture.stateAfterStale?.json?.data?.result;
  const stateAfterViewportRefresh = capture.stateAfterViewportRefresh?.json?.data?.result;
  const staleReconnect = capture.staleReconnect?.json?.data?.result;
  const viewportRefresh = capture.viewportRefresh?.json?.data?.result;
  const activeIncidents = activeIncidentCount(capture.serviceStatus?.json);
  const serviceState = capture.serviceStatus?.json?.data?.service_state || {};
  const routePoolEntry = routePoolEntryId ? serviceState.routePool?.[routePoolEntryId] : null;
  const route = routeId ? serviceState.remoteViewRoutes?.[routeId] : null;
  const serviceBrowser = capture.targets?.browserId ? serviceState.browsers?.[capture.targets.browserId] : null;
  const serviceBrowserHasRoute = Boolean(
    serviceBrowser?.displayAllocationId === displayAllocationId &&
      serviceBrowser?.viewStreams?.some((stream) => stream?.routeId === routeId && stream?.displayAllocationId === displayAllocationId),
  );
  const displayState = displayStateForName(capture.displayContent?.json, displayName);
  const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
  const initialGuacamoleOk = capture.initialGuacamole?.status === 0 && /^[23]\d\d\b/.test(String(capture.initialGuacamole.stdout || '').trim());
  const directGuacamoleOk = capture.directGuacamole?.status === 0 && /^[23]\d\d\b/.test(String(capture.directGuacamole.stdout || '').trim());
  const staleRecovered = Boolean(
    stateAfterStale?.recoveredStaleTab ||
      stateAfterStale?.recoveredLiveTab ||
      stateAfterViewportRefresh?.recoveredStaleTab ||
      stateAfterViewportRefresh?.recoveredLiveTab ||
      (stateAfterStale?.noticeText || []).some((text) => /Recovered stale selected tab identity/.test(text)) ||
      (stateAfterViewportRefresh?.noticeText || []).some((text) => /Recovered stale selected tab identity/.test(text))
  );

  if (capture.failedStage) {
    failures.push(`S11 stopped before dashboard refresh/stale URL controls at ${capture.failedStage}: ${capture.failureReason || 'unknown failure'}`);
    if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
    return {
      passed: false,
      failures,
      warnings,
      evidence: {
        activeIncidents,
        failedStage: capture.failedStage,
        routeId,
      },
    };
  }

  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);
  if (!stateAfterReload?.hasViewport || !stateAfterReload?.hasFrame) failures.push('dashboard reload did not restore a remote viewport iframe');
  if (stateAfterReload?.browserParam !== capture.targets?.browserId) failures.push('dashboard reload browser param mismatch');
  if (stateAfterReload?.sessionParam !== capture.targets?.sessionName) failures.push('dashboard reload session param mismatch');
  if (!stateAfterReload?.hasRefreshButton) failures.push('dashboard reload state is missing refresh control');
  if (stateAfterReload?.hasPasswordInput) failures.push('dashboard reload exposed a password input');
  if (!staleRecovered) failures.push('stale dashboard tab URL did not report selected-target recovery');
  if (!stateAfterStale?.hasViewport || !stateAfterStale?.hasFrame) failures.push('stale dashboard URL recovery did not keep a remote viewport iframe');
  if (stateAfterStale?.browserParam !== capture.targets?.browserId) failures.push('stale dashboard URL recovery browser param mismatch');
  if (stateAfterStale?.sessionParam !== capture.targets?.sessionName) failures.push('stale dashboard URL recovery session param mismatch');
  if (stateAfterStale?.hasPasswordInput) failures.push('stale dashboard URL recovery exposed a password input');
  if (staleReconnect?.reconnected !== true) failures.push('viewer-client reconnect after stale dashboard URL failed');
  if (viewportRefresh?.clicked !== true) failures.push(`viewport refresh after stale dashboard URL was not clickable: ${JSON.stringify(viewportRefresh)}`);
  if (!stateAfterViewportRefresh?.hasViewport || !stateAfterViewportRefresh?.hasFrame) failures.push('viewport refresh after stale URL did not preserve the iframe');
  if (stateAfterViewportRefresh?.browserParam !== capture.targets?.browserId) failures.push('viewport refresh after stale URL browser param mismatch');
  if (stateAfterViewportRefresh?.sessionParam !== capture.targets?.sessionName) failures.push('viewport refresh after stale URL session param mismatch');
  if (!initialGuacamoleOk) failures.push(`initial direct Guacamole frame URL was not HTTP 2xx/3xx: ${capture.initialGuacamole?.stdout || capture.initialGuacamole?.stderr || 'missing result'}`);
  if (!directGuacamoleOk) failures.push(`direct Guacamole frame URL after stale recovery was not HTTP 2xx/3xx: ${capture.directGuacamole?.stdout || capture.directGuacamole?.stderr || 'missing result'}`);
  if (stateAfterReload?.frameSrc && stateAfterViewportRefresh?.frameSrc && stateAfterReload.frameSrc !== stateAfterViewportRefresh.frameSrc) {
    failures.push('Guacamole frame URL changed across reload and stale URL recovery');
  }
  if (displayState?.state !== 'browser_window_visible') {
    failures.push(`route display content after S11 controls was ${displayState?.state || 'missing'}`);
  }
  if (terminalVisible) failures.push(`terminal content visible on route display ${displayName}`);
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (route?.browserId !== capture.targets?.browserId) failures.push('route is no longer bound to the service-owned browser after S11 controls');
  if (routePoolEntry?.currentRouteAllocationId !== routeId) failures.push('route-pool entry is no longer checked out to the service-owned route after S11 controls');
  if (!serviceBrowserHasRoute) failures.push('service-owned browser no longer carries the route-bound view stream after S11 controls');
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);

  warnings.push('S11 uses one route-bound browser and one viewer-client; multi-browser reload soak remains reserved for S12.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      directGuacamoleHttp: String(capture.directGuacamole?.stdout || '').trim(),
      displayName,
      displayState: displayState?.state || null,
      frameSrcAfterRefresh: stateAfterViewportRefresh?.frameSrc || null,
      frameSrcAfterReload: stateAfterReload?.frameSrc || null,
      initialGuacamoleHttp: String(capture.initialGuacamole?.stdout || '').trim(),
      routeId,
      routePoolEntryId,
      serviceBrowserHasRoute,
      staleRecovered,
      staleRecoveredLiveTab: Boolean(stateAfterStale?.recoveredLiveTab || stateAfterViewportRefresh?.recoveredLiveTab),
      staleRecoveredWithNotice: Boolean(stateAfterStale?.recoveredStaleTab || stateAfterViewportRefresh?.recoveredStaleTab),
      staleReconnect: staleReconnect?.reconnected === true,
      viewportRefreshClicked: viewportRefresh?.clicked === true,
    },
  };
}

function evaluateS12(capture) {
  const failures = [];
  const warnings = [];
  const cycles = Array.isArray(capture.cycles) ? capture.cycles : [];
  if (cycles.length < 10) failures.push(`S12 ran ${cycles.length} cycle(s), expected at least 10`);
  if (!capture.credentials.ok) failures.push(`dashboard credentials unavailable: ${capture.credentials.reason}`);

  for (const cycle of cycles) {
    const label = cycle.label || `cycle-${cycle.index}`;
    const openSuccess = remoteViewOpenReady(cycle.open);
    const stateAfterReload = cycle.stateAfterReload?.json?.data?.result;
    const stateAfterControls = cycle.stateAfterControls?.json?.data?.result;
    const reconnect = cycle.reconnect?.json?.data?.result;
    const viewportRefresh = cycle.viewportRefresh?.json?.data?.result;
    const directGuacamoleOk = cycle.directGuacamole?.status === 0 && /^[23]\d\d\b/.test(String(cycle.directGuacamole.stdout || '').trim());
    const activeBefore = cycle.before?.activeIncidents;
    const activeBeforeClose = activeIncidentCount(cycle.serviceStatusBeforeClose?.json);
    const activeAfterReset = cycle.reset?.activeIncidentsAfter;
    const activeAfter = cycle.after?.activeIncidents;
    const displayState = displayStateForName(cycle.displayContent?.json, cycle.targets?.displayName);
    const terminalVisible = displayHasTerminal({ displayContent: displayState }) || displayState?.state === 'terminal_only';
    const tabList = tabsFromRecord(cycle.tabList);
    const switchedUrl = responseUrl(cycle.getUrlAfterSwitch || {});

    if (cycle.failedStage) failures.push(`${label} stopped at ${cycle.failedStage}: ${cycle.failureReason || 'unknown failure'}`);
    if (!openSuccess) failures.push(`${label} remote-view open did not produce operatorVisible.state=ready`);
    if (!stateAfterReload?.hasViewport || !stateAfterReload?.hasFrame) failures.push(`${label} dashboard reload did not restore a remote viewport iframe`);
    if (stateAfterReload?.browserParam !== cycle.targets?.browserId) failures.push(`${label} dashboard reload browser param mismatch`);
    if (stateAfterReload?.sessionParam !== cycle.targets?.sessionName) failures.push(`${label} dashboard reload session param mismatch`);
    if (reconnect?.reconnected !== true) failures.push(`${label} viewer-client reconnect failed`);
    if (viewportRefresh?.clicked !== true) failures.push(`${label} viewport refresh was not clickable`);
    if (cycle.navigate?.status !== 0 || cycle.navigate?.json?.success !== true) failures.push(`${label} navigate command failed`);
    if (cycle.tabNew?.status !== 0 || cycle.tabNew?.json?.success !== true) failures.push(`${label} tab new command failed`);
    if (!Array.isArray(tabList) || tabList.length < 2) failures.push(`${label} tab list did not show at least two tabs`);
    if (cycle.switchTab?.status !== 0 || cycle.switchTab?.json?.success !== true) failures.push(`${label} tab switch command failed`);
    if (!String(switchedUrl || '').startsWith('https://example.org/')) failures.push(`${label} switched-tab URL was ${switchedUrl || 'missing'}`);
    if (!stateAfterControls?.hasViewport || !stateAfterControls?.hasFrame) failures.push(`${label} dashboard did not preserve remote viewport after controls`);
    if (stateAfterControls?.browserParam !== cycle.targets?.browserId) failures.push(`${label} dashboard after controls browser param mismatch`);
    if (stateAfterControls?.sessionParam !== cycle.targets?.sessionName) failures.push(`${label} dashboard after controls session param mismatch`);
    if (!directGuacamoleOk) failures.push(`${label} direct Guacamole frame URL was not HTTP 2xx/3xx: ${cycle.directGuacamole?.stdout || cycle.directGuacamole?.stderr || 'missing result'}`);
    if (displayState?.state !== 'browser_window_visible') failures.push(`${label} route display content was ${displayState?.state || 'missing'}`);
    if (terminalVisible) failures.push(`${label} terminal content visible on route display ${cycle.targets?.displayName || 'unknown'}`);
    if (cycle.finalization?.finalized !== true) {
      failures.push(`${label} route-bound finalization incomplete: ${(cycle.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
    }
    if (activeBefore !== 0) failures.push(`${label} boundary before reports ${activeBefore} active incident(s)`);
    if (activeBeforeClose !== 0) failures.push(`${label} before close reports ${activeBeforeClose} active incident(s)`);
    if (activeAfterReset !== 0) failures.push(`${label} reset reports ${activeAfterReset} active incident(s) after reset`);
    if (activeAfter !== 0) failures.push(`${label} boundary after reports ${activeAfter} active incident(s)`);
    if (cycle.close?.status !== 0 || cycle.close?.json?.success !== true) failures.push(`${label} close command failed`);
    if (!cycle.after?.routePoolBaseline) failures.push(`${label} route-pool state did not return to baseline after reset`);
    if ((cycle.pressureIncrease || []).length > 0) {
      failures.push(`${label} pressure increased after reset: ${cycle.pressureIncrease.map(([key, value]) => `${key}=${value}`).join(', ')}`);
    }
  }

  warnings.push('S12 repeats the normal-use reset contract with one route-bound browser per cycle; S6 cross-observation remains covered by the S6 scenario.');
  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      cycleCount: cycles.length,
      cycles: cycles.map((cycle) => ({
        activeAfter: cycle.after?.activeIncidents ?? null,
        activeAfterReset: cycle.reset?.activeIncidentsAfter ?? null,
        activeBefore: cycle.before?.activeIncidents ?? null,
        directGuacamoleHttp: String(cycle.directGuacamole?.stdout || '').trim(),
        index: cycle.index,
        pressureAfter: cycle.after?.pressure || null,
        pressureBefore: cycle.before?.pressure || null,
        pressureIncrease: cycle.pressureIncrease || [],
        routePoolBaselineAfter: Boolean(cycle.after?.routePoolBaseline),
      })),
    },
  };
}

function evaluateS3OpenProof(capture, commandInfo) {
  const failures = [];
  const warnings = [];
  const openData = capture.open.json?.data || {};
  const openSuccess = remoteViewOpenReady(capture.open);
  const routeId = openData.routeId || openData.routeBinding?.routeId;
  const displayName = openData.routeBinding?.displayName ||
    openData.verification?.visibleWindowProof?.displayName ||
    null;
  const proofDisplayState = openData.verification?.visibleWindowProof?.displayContent?.state ||
    openData.operatorVisible?.proof?.displayContent?.state ||
    null;
  const displayState = displayName === ':14'
    ? capture.displayContent.json?.routes?.B?.displayContent
    : capture.displayContent.json?.routes?.A?.displayContent;
  const activeIncidents = activeIncidentCount(capture.serviceStatus.json);

  if (!commandInfo?.explicit) failures.push('agent-browser command was not explicit');
  if (commandInfo?.version?.status !== 0) failures.push('agent-browser command version check failed');
  if (!openSuccess) failures.push('remote-view open did not produce operatorVisible.state=ready');
  if (proofDisplayState !== 'browser_window_visible') {
    failures.push(`visible-window proof state was ${proofDisplayState || 'missing'}`);
  }
  if (displayState?.state !== 'browser_window_visible') {
    failures.push(`route display content after open was ${displayState?.state || 'missing'}`);
  }
  if (capture.finalization?.finalized !== true) {
    failures.push(`route-bound finalization incomplete: ${(capture.finalization?.blockers || ['missing finalization evidence']).join('; ')}`);
  }
  if (activeIncidents !== 0) failures.push(`service status reports ${activeIncidents} active incident(s)`);
  if (!routeId) failures.push('remote-view open did not report a route ID');
  if (!displayName) failures.push('remote-view open did not report a display name');

  if (!openSuccess) {
    warnings.push('S3-open stopped at route-bound open proof; dashboard viewers and tab controls were intentionally not run.');
  }

  return {
    passed: failures.length === 0,
    failures,
    warnings,
    evidence: {
      activeIncidents,
      command: commandInfo,
      displayName,
      displayState: displayState?.state || null,
      finalization: capture.finalization,
      openStatus: capture.open.status,
      openSuccess,
      proofDisplayState,
      routeId,
      visibleWindows: (displayState?.windows || []).map((window) => window.title),
    },
  };
}

function writeAudit(result) {
  if (result.passed) return null;
  const classification = classifyScenarioFailure(result.failures);
  const audit = [
    `# P46 ${scenario.toUpperCase()} Failure Audit`,
    '',
    `Artifact directory: ${artifactDir}`,
    '',
    '## Classification',
    '',
    `- ${classification}`,
    '',
    '## Failures',
    '',
    ...result.failures.map((failure) => `- ${failure}`),
    '',
    '## Required Planning Before Retry',
    '',
    'Inspect the JSON artifacts in this directory, classify the concrete',
    'runtime or harness defect, and update the P46 plan or a dated note before',
    'retrying this scenario. If this is the second consecutive failure for the',
    'same scenario, stop execution and return to chat planning with the',
    'maintainer.',
    '',
  ].join('\n');
  return writeText('FAILURE_AUDIT.md', audit);
}

async function main() {
  const commandInfo = agentBrowserCommandInfo();
  writeJson('agent-browser-command.json', commandInfo);
  const normalized = scenario.toLowerCase();
  const spec = scenarioSpec(normalized);
  if (!spec) {
    const result = {
      scenario: normalized,
      status: 'unsupported',
      artifactDir,
      message: `Only ${supportedScenarioIds().map((id) => id.toUpperCase()).join(', ')} are implemented in the current P46 runner slice.`,
    };
    writeJson('summary.json', result);
    console.error(JSON.stringify(result, null, 2));
    process.exit(2);
  }
  if (requireExplicitAgentBrowserCommand && !commandInfo.explicit) {
    const result = {
      scenario: normalized,
      status: 'missing_explicit_agent_browser_command',
      artifactDir,
      agentBrowserCommand: commandInfo,
      message: 'Pass --agent-browser-command <path> or set AGENT_BROWSER_COMMAND for remediation runs.',
    };
    writeJson('summary.json', result);
    console.error(JSON.stringify(result, null, 2));
    process.exit(2);
  }
  if (
    requireAgentBrowserDaemonCommandMatch &&
    !commandInfo.daemon?.singleMatchingListener &&
    !commandInfo.daemon?.noListeners
  ) {
    const result = {
      scenario: normalized,
      status: 'agent_browser_daemon_command_mismatch',
      artifactDir,
      agentBrowserCommand: commandInfo,
      message: 'The default agent-browser daemon listener is duplicated or not running the explicit command realpath.',
    };
    writeJson('summary.json', result);
    console.error(JSON.stringify(result, null, 2));
    process.exit(2);
  }
  const specValidation = validateScenarioSpec(spec);
  if (!specValidation.ok) {
    const result = {
      scenario: normalized,
      status: 'invalid_spec',
      artifactDir,
      spec,
      failures: specValidation.failures,
    };
    writeJson('summary.json', result);
    console.error(JSON.stringify(result, null, 2));
    process.exit(2);
  }

  const summary = {
    scenario: normalized,
    scenarioSpec: spec,
    agentBrowserCommand: commandInfo,
    artifactDir,
    resetBefore,
    resetAfter,
    startedAt: new Date().toISOString(),
    reset: {},
    result: null,
  };

  try {
    if (resetBefore) summary.reset.before = resetRuntime('before');
    const capture = normalized === 's0'
      ? captureBaseline(normalized)
      : normalized === 's1'
        ? captureS1()
        : normalized === 's2'
          ? await captureS2()
          : normalized === 's3-open'
            ? captureS3OpenProof()
            : normalized === 's4'
              ? await captureS4()
              : normalized === 's5'
                ? await captureS5()
                : normalized === 's6'
                  ? await captureS5('s6')
                  : normalized === 's7'
                    ? await captureS7()
                    : normalized === 's8'
                      ? await captureS8()
                      : normalized === 's9'
                        ? await captureS9()
                        : normalized === 's10'
                          ? await captureS10()
                          : normalized === 's11'
                            ? await captureS11()
                            : normalized === 's12'
                              ? await captureS12()
                              : await captureS3();
    const result = normalized === 's0'
      ? evaluateS0(capture)
      : normalized === 's1'
        ? evaluateS1(capture)
        : normalized === 's2'
          ? evaluateS2(capture)
          : normalized === 's3-open'
            ? evaluateS3OpenProof(capture, commandInfo)
            : normalized === 's4'
              ? evaluateS4(capture)
              : normalized === 's5'
                ? evaluateS5(capture)
                : normalized === 's6'
                  ? evaluateS6(capture)
                  : normalized === 's7'
                    ? evaluateS7(capture)
                    : normalized === 's8'
                      ? evaluateS8(capture)
                      : normalized === 's9'
                        ? evaluateS9(capture)
                        : normalized === 's10'
                          ? evaluateS10(capture)
                          : normalized === 's11'
                            ? evaluateS11(capture)
                            : normalized === 's12'
                              ? evaluateS12(capture)
                              : evaluateS3(capture);
    const auditPath = writeAudit(result);
    summary.result = {
      ...result,
      auditPath,
    };
    if (resetAfter) summary.reset.after = resetRuntime('after');
    summary.finishedAt = new Date().toISOString();
    writeJson('summary.json', summary);
    console.log(JSON.stringify(summary, null, 2));
    process.exit(result.passed ? 0 : 1);
  } catch (error) {
    if (resetAfter && !summary.reset.after) {
      try {
        summary.reset.after = resetRuntime('after-error');
      } catch (resetError) {
        summary.reset.afterError = String(resetError.stack || resetError.message || resetError);
      }
    }
    summary.finishedAt = new Date().toISOString();
    summary.result = {
      passed: false,
      failures: [String(error.stack || error.message || error)],
      warnings: [],
      auditPath: null,
    };
    summary.result.auditPath = writeAudit(summary.result);
    writeJson('summary.json', summary);
    console.error(JSON.stringify(summary, null, 2));
    process.exit(1);
  }
}

main();
