import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

export function dashboardWorkspaceUrl({ browserId, sessionName, tabId }, baseUrl = 'http://127.0.0.1:4848/') {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:control');
  url.searchParams.set('workspace', `browser:${browserId}`);
  url.searchParams.set('browser', browserId);
  url.searchParams.set('session', sessionName);
  if (tabId) url.searchParams.set('tab', tabId);
  return url.toString();
}

export function dashboardStateScript({ browserId, sessionName, tabId }) {
  return `
(() => {
  const text = document.body?.innerText || "";
  const url = new URL(location.href);
  const viewport = document.querySelector(".workspace-remote-viewport");
  const header = viewport?.querySelector(".workspace-remote-viewport-header");
  const stage = viewport?.querySelector(".workspace-remote-viewport-stage");
  const frame = viewport?.querySelector("iframe");
  const buttons = Array.from(document.querySelectorAll("button"));
  const findAria = (label) => buttons.find((button) => button.getAttribute("aria-label") === label);
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
    browserParam: url.searchParams.get("browser"),
    sessionParam: url.searchParams.get("session"),
    tabParam: url.searchParams.get("tab"),
    expectedBrowserId: ${JSON.stringify(browserId)},
    expectedSessionName: ${JSON.stringify(sessionName)},
    expectedTabId: ${JSON.stringify(tabId)},
    hasViewport: Boolean(viewport),
    uxState: viewport?.getAttribute("data-ux-state") || null,
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
    stageText: stage?.textContent?.replace(/\\s+/g, " ").trim().slice(0, 1200) || null,
    hasRefreshButton: Boolean(refreshButton),
    refreshDisabled: refreshButton ? (refreshButton.disabled || refreshButton.getAttribute("aria-disabled") === "true") : null,
    hasExternalButton: Boolean(externalButton),
    hasInteractionButton: Boolean(interactionButton),
    interactionDisabled: interactionButton ? (interactionButton.disabled || interactionButton.getAttribute("aria-disabled") === "true") : null,
    hasFullscreenButton: Boolean(fullscreenButton),
    hasPasswordInput: Boolean(document.querySelector('input[type="password"]')),
    textSample: text.replace(/\\s+/g, " ").slice(0, 1800),
  };
})()
`;
}

export function verifiedChromiumFromInstallDoctor(installDoctorJson) {
  const manifest = installDoctorJson?.data?.launchConfig?.browserBuildManifests?.stealthcdp_chromium;
  if (manifest?.ready === true && manifest?.executablePathExists === true && existsSync(manifest.executablePath)) {
    return manifest.executablePath;
  }
  return null;
}

export function parseEnvText(text) {
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

export function dashboardCredentialsFromEnv(values, sourcePath = null) {
  const username = values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME;
  const password = values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD ||
    values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
  return {
    ok: Boolean(username && password),
    path: sourcePath,
    reason: username && password ? null : 'dashboard auth env file does not contain usable credentials',
    username,
    password,
  };
}

export function commandVectorUsesServiceOwnership(command, args = []) {
  const vector = [command, ...args].filter(Boolean).map(String);
  const commandName = vector[0]?.split('/').pop();
  if (commandName === 'agent-browser') return true;
  return vector.some((part, index) => {
    if (part === 'remote-view' && vector[index + 1] === 'open') return true;
    if (part === '--session') return true;
    if (part === 'service' && ['request', 'route', 'checkout'].includes(vector[index + 1])) return true;
    if (part === 'route' || part === 'checkout') return true;
    return false;
  });
}

export function resolveViewerClientExecutable({
  commandExists,
  env = process.env,
  installDoctorJson,
} = {}) {
  const verifiedChromium = verifiedChromiumFromInstallDoctor(installDoctorJson);
  const candidates = [
    env.P47_VIEWER_CLIENT_CHROMIUM,
    env.P46_CHROMIUM,
    verifiedChromium,
    'google-chrome',
    'chromium',
    'chromium-browser',
  ].filter(Boolean);
  for (const candidate of candidates) {
    const found = commandExists ? commandExists(candidate) : candidate;
    if (found) return { executable: found, verifiedChromium };
  }
  return { executable: null, verifiedChromium };
}

export function createViewerClientLaunchDescriptor({
  artifactDir,
  dashboardUrl,
  executable,
  label,
  port,
  profileDir,
  verifiedChromium = null,
}) {
  const launchArgs = [
    '--headless=new',
    '--disable-gpu',
    '--no-sandbox',
    `--user-data-dir=${profileDir}`,
    `--remote-debugging-port=${port}`,
    'about:blank',
  ];
  if (commandVectorUsesServiceOwnership(executable, launchArgs)) {
    throw new Error(`viewer-client launch vector uses service ownership: ${executable} ${launchArgs.join(' ')}`);
  }
  return {
    artifactDir,
    dashboardUrl,
    executable,
    forbiddenServiceOwnership: true,
    label,
    launchArgs,
    launchPath: join(artifactDir, `${label}-chromium-launch.json`),
    port,
    profileDir,
    readinessUrl: port > 0 ? `http://127.0.0.1:${port}/json/version` : null,
    remoteDebuggingPortMode: port > 0 ? 'explicit' : 'chromium_dynamic',
    role: 'viewer-client',
    stderrPath: join(artifactDir, `${label}-chromium-stderr.log`),
    stdoutPath: join(artifactDir, `${label}-chromium-stdout.log`),
    verifiedChromium,
  };
}

export function resolveViewerClientDebuggingPort(env = process.env) {
  const raw = env.P47_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT ||
    env.P46_VIEWER_CLIENT_REMOTE_DEBUGGING_PORT;
  if (!raw) return 0;
  const port = Number.parseInt(raw, 10);
  if (!Number.isInteger(port) || port < 0 || port > 65535) {
    throw new Error(`Invalid viewer-client DevTools port override: ${raw}`);
  }
  return port;
}

export function readDevToolsActivePort(profileDir) {
  const activePortPath = join(profileDir, 'DevToolsActivePort');
  if (!existsSync(activePortPath)) return null;
  const [portLine, browserPath = null] = readFileSync(activePortPath, 'utf8').split(/\r?\n/);
  const port = Number.parseInt(portLine, 10);
  if (!Number.isInteger(port) || port <= 0 || port > 65535) return null;
  return { browserPath, path: activePortPath, port };
}

export async function waitForDevToolsActivePort(profileDir, label, timeoutMs = 30000, onAttempt = null) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < timeoutMs) {
    try {
      const activePort = readDevToolsActivePort(profileDir);
      if (activePort) return activePort;
      lastError = 'DevToolsActivePort not written yet';
    } catch (error) {
      lastError = String(error.message || error);
    }
    if (onAttempt) onAttempt(lastError);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} did not write DevToolsActivePort in ${profileDir}: ${lastError || 'timed out'}`);
}

export async function waitForJson(url, label, timeoutMs = 30000, onAttempt = null) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return await response.json();
      lastError = `${label} returned HTTP ${response.status}`;
    } catch (error) {
      lastError = String(error.message || error);
    }
    if (onAttempt) onAttempt(lastError);
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} was not ready at ${url}: ${lastError || 'timed out'}`);
}

export function cdpConnect(webSocketDebuggerUrl) {
  let nextId = 1;
  const pending = new Map();
  const socket = new WebSocket(webSocketDebuggerUrl);
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    if (!message.id) return;
    const waiter = pending.get(message.id);
    if (!waiter) return;
    pending.delete(message.id);
    if (message.error) {
      waiter.reject(new Error(JSON.stringify(message.error)));
    } else {
      waiter.resolve(message.result || {});
    }
  });
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error(`CDP websocket did not open: ${webSocketDebuggerUrl}`)), 15000);
    socket.addEventListener('open', () => {
      clearTimeout(timeout);
      resolve({
        close() {
          socket.close();
        },
        send(method, params = {}) {
          const id = nextId;
          nextId += 1;
          return new Promise((resolveSend, rejectSend) => {
            const timeout = setTimeout(() => {
              pending.delete(id);
              rejectSend(new Error(`CDP command ${method} timed out after 30000ms`));
            }, 30000);
            pending.set(id, {
              reject(error) {
                clearTimeout(timeout);
                rejectSend(error);
              },
              resolve(value) {
                clearTimeout(timeout);
                resolveSend(value);
              },
            });
            socket.send(JSON.stringify({ id, method, params }));
          });
        },
      });
    }, { once: true });
    socket.addEventListener('error', (event) => {
      clearTimeout(timeout);
      reject(new Error(`CDP websocket error: ${event.message || 'unknown'}`));
    }, { once: true });
  });
}

export async function cdpEvaluate(cdp, expression) {
  const result = await cdp.send('Runtime.evaluate', {
    awaitPromise: true,
    expression,
    returnByValue: true,
  });
  if (result.exceptionDetails) {
    throw new Error(`CDP evaluation failed: ${JSON.stringify(result.exceptionDetails)}`);
  }
  return result.result?.value;
}

function pageDiscoverySnapshot(page) {
  return {
    id: page?.id || null,
    title: page?.title || null,
    type: page?.type || null,
    url: page?.url || null,
    webSocketDebuggerUrl: page?.webSocketDebuggerUrl || null,
  };
}

function chooseDashboardPage(pages) {
  return pages.find((item) => item.type === 'page' && item.url && item.url !== 'about:blank') ||
    pages.find((item) => item.type === 'page') ||
    pages[0];
}

export async function waitForDashboardViewerClientPageUrl(
  viewerClient,
  expectedUrl,
  label = viewerClient.label || 'dashboard viewer-client',
  {
    artifactName = null,
    timeoutMs = 30000,
    writeJson = null,
  } = {},
) {
  const resolvedPort = viewerClient.resolvedPort || viewerClient.port;
  if (!resolvedPort || resolvedPort <= 0) {
    throw new Error(`${label} cannot wait for page URL without a resolved DevTools port`);
  }
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    const pages = await waitForJson(`http://127.0.0.1:${resolvedPort}/json`, `${label} pages`, 5000);
    const page = chooseDashboardPage(pages);
    last = {
      chosenPage: pageDiscoverySnapshot(page),
      expectedUrl,
      label,
      pageCount: pages.length,
      pages: pages.map(pageDiscoverySnapshot),
      resolvedPort,
      urlMatched: page?.url === expectedUrl,
    };
    if (last.urlMatched) {
      if (writeJson && artifactName) writeJson(artifactName, last);
      return last;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  if (writeJson && artifactName) writeJson(artifactName, last);
  throw new Error(`${label} did not reach expected page URL ${expectedUrl}: ${JSON.stringify(last)}`);
}

export async function reconnectDashboardViewerClient(
  viewerClient,
  label = viewerClient.label || 'dashboard viewer-client',
  {
    artifactName = null,
    writeJson = null,
  } = {},
) {
  const resolvedPort = viewerClient.resolvedPort || viewerClient.port;
  if (!resolvedPort || resolvedPort <= 0) {
    throw new Error(`${label} cannot reconnect without a resolved DevTools port`);
  }
  const pages = await waitForJson(`http://127.0.0.1:${resolvedPort}/json`, `${label} pages after navigation`, 30000);
  const page = chooseDashboardPage(pages);
  if (!page?.webSocketDebuggerUrl) throw new Error(`${label} did not expose a page websocket after navigation`);
  const discovery = {
    chosenPage: pageDiscoverySnapshot(page),
    label,
    pageCount: pages.length,
    pages: pages.map(pageDiscoverySnapshot),
    previousPage: {
      id: viewerClient.pageId || null,
      url: viewerClient.pageUrl || null,
      webSocketDebuggerUrl: viewerClient.websocketUrl || null,
    },
    samePageId: Boolean(viewerClient.pageId && page.id && viewerClient.pageId === page.id),
    sameWebSocketDebuggerUrl: Boolean(viewerClient.websocketUrl && page.webSocketDebuggerUrl === viewerClient.websocketUrl),
    resolvedPort,
  };
  if (writeJson && artifactName) writeJson(artifactName, discovery);
  const nextCdp = await cdpConnect(page.webSocketDebuggerUrl);
  try {
    viewerClient.cdp.close();
  } catch {
    // Best effort.
  }
  viewerClient.cdp = nextCdp;
  viewerClient.pageId = page.id || null;
  viewerClient.pageUrl = page.url || null;
  viewerClient.websocketUrl = page.webSocketDebuggerUrl;
  return {
    discovery,
    domainEnableSkipped: true,
    pageId: page.id || null,
    pageTitle: page.title || null,
    pageUrl: page.url || null,
    reconnected: true,
    resolvedPort,
    websocketUrl: page.webSocketDebuggerUrl,
  };
}

function recoveredStaleTabState(last, expected) {
  return Boolean(
    last?.hasViewport &&
      last?.hasFrame &&
      last?.browserParam === expected.browserId &&
      last?.sessionParam === expected.sessionName &&
      expected.tabId &&
      last?.tabParam &&
      last?.hasRefreshButton &&
      !last?.hasPasswordInput &&
      Array.isArray(last?.noticeText) &&
      last.noticeText.some((text) => /Recovered stale selected tab identity/.test(text)),
  );
}

function recoveredLiveTabState(last, expected) {
  return Boolean(
    last?.hasViewport &&
      last?.hasFrame &&
      last?.browserParam === expected.browserId &&
      last?.sessionParam === expected.sessionName &&
      expected.tabId &&
      last?.tabParam &&
      last.tabParam !== expected.tabId &&
      last?.hasRefreshButton &&
      !last?.hasPasswordInput
  );
}

export async function waitForDashboardState(
  cdp,
  expected,
  label,
  timeoutMs = 60000,
  writeJson = null,
  { allowRecoveredLiveTab = false, allowRecoveredStaleTab = false } = {},
) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await cdpEvaluate(cdp, dashboardStateScript(expected));
    if (
      last?.hasViewport &&
      last?.hasFrame &&
      last?.browserParam === expected.browserId &&
      last?.sessionParam === expected.sessionName &&
      (!expected.tabId || last?.tabParam === expected.tabId) &&
      last?.hasRefreshButton &&
      !last?.hasPasswordInput
    ) {
      return last;
    }
    if (allowRecoveredStaleTab && recoveredStaleTabState(last, expected)) {
      return { ...last, recoveredStaleTab: true };
    }
    if (allowRecoveredLiveTab && recoveredLiveTabState(last, expected)) {
      return { ...last, recoveredLiveTab: true };
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  if (writeJson) writeJson(`${label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}-last-dashboard-state.json`, last);
  throw new Error(`${label} did not reach expected state: ${JSON.stringify(last)}`);
}

export async function clickDashboardRefresh(viewerClient) {
  return cdpEvaluate(viewerClient.cdp, `
(() => {
  const button = Array.from(document.querySelectorAll("button"))
    .find((item) => item.getAttribute("aria-label") === "Refresh workspace viewport");
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

export async function navigateDashboardViewerClient(viewerClient, url) {
  return cdpEvaluate(viewerClient.cdp, `
(() => {
  const nextUrl = ${JSON.stringify(url)};
  const from = location.href;
  const current = new URL(location.href);
  const next = new URL(nextUrl);
  let method = "location.assign";
  if (current.origin === next.origin) {
    history.pushState(null, "", nextUrl);
    window.dispatchEvent(new PopStateEvent("popstate", { state: null }));
    method = "history.pushState";
  } else {
    window.location.assign(nextUrl);
  }
  return {
    from,
    locationHref: location.href,
    method,
    ok: true,
    requestedUrl: nextUrl,
  };
})()
`);
}

export async function captureDashboardScreenshot(viewerClient, path) {
  const result = await viewerClient.cdp.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  writeFileSync(path, Buffer.from(result.data, 'base64'));
  return path;
}

export async function launchDashboardViewerClient({
  artifactDir,
  commandExists,
  credentials,
  dashboardUrl,
  env = process.env,
  expected,
  installDoctorJson,
  label,
  stateOptions = {},
  writeJson,
}) {
  const { executable, verifiedChromium } = resolveViewerClientExecutable({
    commandExists,
    env,
    installDoctorJson,
  });
  if (!executable) throw new Error('No external Chromium executable found for dashboard viewer-client proof');
  const profileDir = mkdtempSync(join(artifactDir, `${label}-viewer-client-profile-`));
  const port = resolveViewerClientDebuggingPort(env);
  const descriptor = createViewerClientLaunchDescriptor({
    artifactDir,
    dashboardUrl,
    executable,
    label,
    port,
    profileDir,
    verifiedChromium,
  });
  let stdout = '';
  let stderr = '';
  let lastReadinessError = null;
  let exited = null;
  let activePortEvidence = null;
  let resolvedPort = port > 0 ? port : null;
  let readinessUrl = descriptor.readinessUrl;
  writeFileSync(descriptor.stdoutPath, '');
  writeFileSync(descriptor.stderrPath, '');
  const writeLaunch = (extra = {}) => {
    writeFileSync(descriptor.launchPath, `${JSON.stringify({
      ...descriptor,
      lastReadinessError,
      pid: proc?.pid || null,
      activePort: activePortEvidence,
      readinessUrl,
      resolvedPort,
      ...extra,
    }, null, 2)}\n`);
  };
  const proc = spawn(executable, descriptor.launchArgs, {
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  proc.stdout.setEncoding('utf8');
  proc.stdout.on('data', (chunk) => {
    stdout += chunk;
    writeFileSync(descriptor.stdoutPath, stdout);
  });
  proc.stderr.setEncoding('utf8');
  proc.stderr.on('data', (chunk) => {
    stderr += chunk;
    writeFileSync(descriptor.stderrPath, stderr);
  });
  proc.on('exit', (code, signal) => {
    exited = { code, signal };
    writeLaunch({ exited });
  });
  writeLaunch({ started: true });
  try {
    let activePort = null;
    if (port === 0) {
      activePort = await waitForDevToolsActivePort(profileDir, `${label} dashboard viewer-client`, 30000, (error) => {
        lastReadinessError = error;
      });
      activePortEvidence = activePort;
      resolvedPort = activePort.port;
      readinessUrl = `http://127.0.0.1:${resolvedPort}/json/version`;
      writeLaunch({ activePort });
    }
    const browser = await waitForJson(readinessUrl, `${label} dashboard viewer-client`, 30000, (error) => {
      lastReadinessError = error;
    });
    const pages = await waitForJson(`http://127.0.0.1:${resolvedPort}/json`, `${label} dashboard viewer-client pages`, 30000, (error) => {
      lastReadinessError = error;
    });
    const page = pages.find((item) => item.type === 'page') || pages[0];
    if (!page?.webSocketDebuggerUrl) throw new Error(`${label} dashboard viewer-client did not expose a page websocket`);
    const cdp = await cdpConnect(page.webSocketDebuggerUrl);
    await cdp.send('Page.enable');
    await cdp.send('Runtime.enable');
    await cdp.send('Page.navigate', { url: dashboardUrl });
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const login = await cdpEvaluate(cdp, `
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
    return { ok: false, status: response.status };
  }
  return { ok: true, status: response.status };
})()
`);
    if (login?.ok !== true) throw new Error(`${label} dashboard login failed: ${JSON.stringify(login)}`);
    await cdp.send('Page.navigate', { url: dashboardUrl });
    const state = await waitForDashboardState(cdp, expected, `${label} dashboard state`, 60000, writeJson, stateOptions);
    writeLaunch({
      browser,
      pageUrl: page.url || null,
      ready: true,
      resolvedPort,
      websocketUrl: page.webSocketDebuggerUrl,
    });
    return {
      ...descriptor,
      readinessUrl,
      resolvedPort,
      cdp,
      close() {
        try {
          cdp.close();
        } catch {
          // Best effort.
        }
        proc.kill('SIGTERM');
        rmSync(profileDir, { recursive: true, force: true });
        writeLaunch({ closed: true, exited });
      },
      login,
      pageId: page.id || null,
      pageUrl: page.url || null,
      proc,
      state,
      stderr: () => stderr,
      stdout: () => stdout,
    };
  } catch (error) {
    writeLaunch({
      error: String(error.stack || error.message || error),
      exited,
      ready: false,
    });
    if (!proc.killed) proc.kill('SIGTERM');
    throw error;
  }
}

export function closeViewerClients(viewerClients) {
  for (const viewerClient of viewerClients.reverse()) {
    try {
      viewerClient.close();
    } catch {
      // Best effort.
    }
  }
}
