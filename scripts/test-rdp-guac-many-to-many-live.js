#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
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
  configureRemoteHeadedContext,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacManyToManySmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacManyToManyLaunch';
const checkoutTaskName = 'rdpGuacManyToManyCheckout';
const viewerTaskName = 'rdpGuacManyToManyViewer';
const controllerTaskName = 'rdpGuacManyToManyController';
const closeTaskName = 'rdpGuacManyToManyClose';
const clientASession = 'rdp-guac-many-viewer-a';
const clientBSession = 'rdp-guac-many-viewer-b';
const browserAMarker = `BINDING PROOF BROWSER A ${process.pid}`;
const browserBMarker = `BINDING PROOF BROWSER B ${process.pid}`;
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-many-to-many-${timestamp}`);

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

function writeTextArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, value);
  return path;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function bindingProofDataUrl(marker) {
  const words = marker.split(/\s+/).join('<br>');
  const html = [
    '<!doctype html>',
    '<html>',
    `<head><title>${marker}</title></head>`,
    '<body style="margin:0;background:#ffffff;color:#000000;font-family:Arial,sans-serif;">',
    `<div style="position:fixed;top:8px;left:12px;right:12px;text-align:center;font-size:24px;font-weight:700;">${marker}</div>`,
    '<main style="min-height:100vh;display:flex;align-items:center;justify-content:center;text-align:center;overflow:hidden;">',
    `<h1 id="ready" style="font-size:38px;line-height:1.05;max-width:900px;margin:0;">${words}</h1>`,
    '</main>',
    `<div style="position:fixed;bottom:8px;left:12px;right:12px;text-align:center;font-size:24px;font-weight:700;">${marker}</div>`,
    '</body>',
    '</html>',
  ].join('');
  return `data:text/html;charset=utf-8,${encodeURIComponent(html)}`;
}

function commandExists(command) {
  const result = spawnSync('sh', ['-lc', `command -v ${command}`], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  return result.status === 0 ? result.stdout.trim() : null;
}

function runChecked(command, args, label) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  assert(
    result.status === 0,
    `${label} failed: ${command} ${args.join(' ')}\n${result.stdout}${result.stderr}`,
  );
  return result.stdout;
}

function parseRoutePoolConfig() {
  if (envValue('AGENT_BROWSER_RDP_ROUTE_POOL_JSON')) {
    const parsed = JSON.parse(envValue('AGENT_BROWSER_RDP_ROUTE_POOL_JSON'));
    assert(Array.isArray(parsed), 'AGENT_BROWSER_RDP_ROUTE_POOL_JSON must be an array');
    return parsed;
  }
  const route = (label) => ({
    id: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_POOL_ENTRY_ID`) || `pool-${label.toLowerCase()}`,
    routeId: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_ID`),
    connectionId: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_CONNECTION_ID`),
    connectionName: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_CONNECTION_NAME`),
    frameUrl: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_FRAME_URL`),
    externalUrl: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_EXTERNAL_URL`),
    providerMode: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_PROVIDER_MODE`) || 'simultaneous_view',
    target: {
      displayName: envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_DISPLAY_NAME`),
    },
  });
  return [route('A'), route('B')].filter((item) => item.routeId || item.connectionId || item.frameUrl);
}

function routeIdentity(route) {
  return route.connectionId || route.routeId || route.frameUrl || route.externalUrl || '';
}

function normalizeRouteConfig(route, index) {
  const label = index === 0 ? 'A' : 'B';
  const routeId = route.routeId || (route.connectionId ? `guacamole:${route.connectionId}` : `route-${label.toLowerCase()}`);
  const envDisplayName = envValue(`AGENT_BROWSER_RDP_ROUTE_${label}_DISPLAY_NAME`);
  return {
    id: route.id || `pool-${label.toLowerCase()}`,
    routeId,
    connectionId: route.connectionId || null,
    connectionName: route.connectionName || null,
    frameUrl: route.frameUrl || route.externalUrl || null,
    externalUrl: route.externalUrl || route.frameUrl || null,
    providerMode: route.providerMode || 'simultaneous_view',
    target: {
      ...(route.target && typeof route.target === 'object' ? route.target : {}),
      ...(route.displayName ? { displayName: route.displayName } : {}),
      ...(route.targetDisplayName ? { displayName: route.targetDisplayName } : {}),
      ...(envDisplayName ? { displayName: envDisplayName } : {}),
    },
  };
}

function requireDistinctRoutePool() {
  const routePool = parseRoutePoolConfig().slice(0, 2).map(normalizeRouteConfig);
  assert(
    routePool.length >= 2,
    'Two distinct route-pool entries are required. Set AGENT_BROWSER_RDP_ROUTE_POOL_JSON or AGENT_BROWSER_RDP_ROUTE_A_* and AGENT_BROWSER_RDP_ROUTE_B_*.',
  );
  for (const route of routePool) {
    assert(route.routeId || route.connectionId, `Route entry is missing routeId or connectionId: ${JSON.stringify(route)}`);
    assert(route.frameUrl || route.externalUrl, `Route entry is missing frameUrl or externalUrl: ${JSON.stringify(route)}`);
  }
  assert(
    routeIdentity(routePool[0]) && routeIdentity(routePool[0]) !== routeIdentity(routePool[1]),
    `Route entries must have distinct connectionId, routeId, or URL identity: ${JSON.stringify(routePool)}`,
  );
  const displayA = routePool[0].target?.displayName;
  const displayB = routePool[1].target?.displayName;
  if (displayA || displayB) {
    assert(
      displayA && displayB && displayA !== displayB,
      `Route entries with displayName targets must declare two distinct displayName values: ${JSON.stringify(routePool)}`,
    );
  }
  return routePool;
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

function loadGuacamoleCredentials() {
  const secretFile = process.env.AGENT_BROWSER_GUACAMOLE_SECRET_FILE ||
    join(process.env.HOME || '', '.agent-browser', 'secrets', 'guacamole.env');
  if (existsSync(secretFile)) {
    const values = parseEnvText(readFileSync(secretFile, 'utf8'));
    for (const [key, value] of Object.entries(values)) {
      if (!process.env[key]) process.env[key] = value;
    }
  }
  const username = envValue('GUACAMOLE_ADMIN_USERNAME');
  const password = envValue('GUACAMOLE_ADMIN_PASSWORD');
  assert(username && password, `Guacamole credentials were not found in ${secretFile}`);
  return { username, password };
}

function guacamoleBaseUrl(route) {
  const rawUrl = route.frameUrl || route.externalUrl;
  assert(rawUrl, `Route is missing a Guacamole URL: ${JSON.stringify(route)}`);
  const url = new URL(rawUrl);
  const marker = '/guacamole/';
  const index = url.pathname.indexOf(marker);
  assert(index >= 0, `Route URL is not under /guacamole/: ${rawUrl}`);
  url.pathname = url.pathname.slice(0, index + marker.length);
  url.search = '';
  url.hash = '';
  return url.toString();
}

async function waitForDashboardCredentials(context, timeoutMs = 60000) {
  const path = join(context.agentHome, 'dashboard-auth.env');
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (existsSync(path)) {
      const values = parseEnvText(readFileSync(path, 'utf8'));
      const username = values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME || values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME;
      const password = values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD || values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
      if (username && password) return { username, password };
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Dashboard bootstrap credential file was not ready at ${path}`);
}

async function ensureStreamPortForSession(context, sessionName, timeoutMs = 60000) {
  const statusResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'status'], timeoutMs);
  let stream = parseJsonOutput(statusResult.stdout, `${sessionName} stream status`);
  assert(stream.success === true, `${sessionName} stream status failed: ${statusResult.stdout}${statusResult.stderr}`);
  if (!stream.data?.enabled) {
    const enableResult = await runCli(context, ['--json', '--session', sessionName, 'stream', 'enable'], timeoutMs);
    stream = parseJsonOutput(enableResult.stdout, `${sessionName} stream enable`);
    assert(stream.success === true, `${sessionName} stream enable failed: ${enableResult.stdout}${enableResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `No stream port for ${sessionName}`);
  return port;
}

async function serviceStatusArtifact(streamPort, label) {
  const status = await httpJson(streamPort, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

async function serviceRequest(streamPort, body, label) {
  const response = await httpJson(streamPort, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

function seedRoutePoolEntry(context, route, browser) {
  const statePath = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.routePool = state.routePool || {};
  state.routePool[route.id] = {
    id: route.id,
    provider: 'rdp_gateway',
    routeId: route.routeId,
    connectionId: route.connectionId,
    connectionName: route.connectionName,
    frameUrl: route.frameUrl,
    externalUrl: route.externalUrl,
    target: {
      displayAllocationId: browser.displayAllocationId,
      browserId: browser.id,
      sessionId: browser.activeSessionIds?.[0] || browser.sessionId || null,
      ...(route.target?.displayName ? { displayName: route.target.displayName } : {}),
    },
    providerMode: route.providerMode,
    state: 'available',
    currentRouteAllocationId: null,
    readiness: {
      state: 'ready',
      source: 'rdp_guac_many_to_many_live_smoke',
      updatedAt: new Date().toISOString(),
    },
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
  return state.routePool[route.id];
}

async function launchRemoteBrowser({ context, remoteConfig, route, sessionName, title }) {
  const streamPort = await ensureStreamPortForSession(context, sessionName, 180000);
  const routeDisplayName = route.target?.displayName || null;
  const displayIsolation = routeDisplayName ? 'shared_display' : 'private_virtual_display';
  const launch = await serviceRequest(
    streamPort,
    {
      action: 'navigate',
      serviceName,
      agentName,
      taskName: launchTaskName,
      params: {
        browserHost: 'remote_headed',
        displayIsolation,
        ...(routeDisplayName ? { remoteHeadedDisplay: routeDisplayName } : {}),
        headless: false,
        runtimeProfile: `${sessionName}-profile`,
        url: bindingProofDataUrl(title),
        waitUntil: 'load',
        viewStreamProvider: remoteConfig.viewStreamProvider,
        controlInputProvider: remoteConfig.controlInputProvider,
        viewStreamUrl: remoteConfig.viewStreamUrl,
      },
      jobTimeoutMs: 120000,
    },
    `${sessionName}-launch`,
  );
  const browserId = `session:${sessionName}`;
  const afterLaunch = await serviceStatusArtifact(streamPort, `${sessionName}-after-launch`);
  const browser = afterLaunch.data?.service_state?.browsers?.[browserId];
  assert(browser?.health === 'ready', `${title} browser not ready: ${JSON.stringify(browser)}`);
  assert(browser.displayAllocationId, `${title} did not record display allocation id: ${JSON.stringify(browser)}`);
  if (routeDisplayName) {
    assert(
      browser.displayName === routeDisplayName,
      `${title} did not launch on route target display ${routeDisplayName}: ${JSON.stringify(browser)}`,
    );
  }
  const poolEntry = seedRoutePoolEntry(context, route, browser);
  writeArtifact(`${sessionName}-seeded-route-pool-entry.json`, poolEntry);
  const checkout = await serviceRequest(
    streamPort,
    {
      action: 'service_remote_view_route_checkout',
      serviceName,
      agentName,
      taskName: checkoutTaskName,
      params: {
        displayAllocationId: browser.displayAllocationId,
        routePoolEntryId: route.id,
        routeId: route.routeId,
        browserId,
        sessionName,
        provider: 'rdp_gateway',
      },
      jobTimeoutMs: 30000,
    },
    `${sessionName}-route-checkout`,
  );
  assert(checkout.data?.routePoolEntryId === route.id, `${title} used wrong route pool: ${JSON.stringify(checkout)}`);
  return {
    browserId,
    displayAllocationId: browser.displayAllocationId,
    displayIsolation,
    displayName: browser.displayName || null,
    launch,
    route,
    sessionName,
    streamPort,
    title,
  };
}

function tileUrl(baseUrl) {
  const url = new URL(baseUrl);
  url.searchParams.set('view', 'workspace:tile');
  return url.toString();
}

function tileStateScript() {
  return `
(() => {
  const viewport = document.querySelector(".workspace-remote-viewport");
  const tiles = Array.from(document.querySelectorAll(".workspace-remote-viewport-tile-card"));
  return {
    url: location.href,
    hasViewport: Boolean(viewport),
    uxState: viewport?.getAttribute("data-ux-state") || null,
    tileCount: tiles.length,
    frames: tiles.map((tile) => ({
      title: tile.querySelector("h3")?.textContent?.trim() || null,
      summary: tile.querySelector("p")?.textContent?.trim() || null,
      src: tile.querySelector("iframe")?.getAttribute("src") || null,
      iframeRect: (() => {
        const rect = tile.querySelector("iframe")?.getBoundingClientRect();
        return rect ? {
          x: rect.x,
          y: rect.y,
          width: rect.width,
          height: rect.height,
        } : null;
      })(),
      text: tile.textContent?.replace(/\\s+/g, " ").trim().slice(0, 800) || "",
    })),
    viewportSize: {
      width: window.innerWidth,
      height: window.innerHeight,
      devicePixelRatio: window.devicePixelRatio,
    },
    text: document.body?.innerText?.replace(/\\s+/g, " ").slice(0, 2000) || "",
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
  return { ok: true, status: response.status };
})()
`, 30000);
  assert(result?.ok === true, `${session} dashboard login failed: ${JSON.stringify(result)}`);
}

async function loginGuacamoleClient(context, { executable, profile, session, route, viewport }) {
  const credentials = loadGuacamoleCredentials();
  const baseUrl = guacamoleBaseUrl(route);
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
    baseUrl,
  ], 180000);
  const opened = parseJsonOutput(openedResult.stdout, `${session} guacamole open`);
  assert(opened.success === true, `${session} guacamole open failed: ${openedResult.stdout}${openedResult.stderr}`);
  await runCli(context, ['--json', '--session', session, 'set', 'viewport', String(viewport.width), String(viewport.height)]);
  const login = await evalInClient(context, session, `
(async () => {
  const response = await fetch("api/tokens", {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      username: ${JSON.stringify(credentials.username)},
      password: ${JSON.stringify(credentials.password)},
    }).toString(),
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || !payload.authToken) {
    return { ok: false, status: response.status, payload };
  }
  localStorage.setItem("GUAC_AUTH", JSON.stringify(payload));
  return { ok: true, status: response.status, username: payload.username, dataSource: payload.dataSource };
})()
`, 30000);
  assert(login?.ok === true, `${session} Guacamole login failed: ${JSON.stringify(login)}`);
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

async function waitForTileState(context, session, routes, label, timeoutMs = 60000) {
  const started = Date.now();
  let lastState = null;
  while (Date.now() - started < timeoutMs) {
    lastState = await evalInClient(context, session, tileStateScript(), 30000);
    const srcs = (lastState?.frames || []).map((frame) => frame.src || '').join('\n');
    if (
      lastState?.hasViewport &&
      lastState?.tileCount >= routes.length &&
      routes.every((route) => srcs.includes(route.frameUrl || route.externalUrl))
    ) {
      return lastState;
    }
    await new Promise((resolve) => setTimeout(resolve, 750));
  }
  writeArtifact(`${label}-last-tile-state.json`, lastState);
  throw new Error(`${label} did not show both route tiles. Last state: ${JSON.stringify(lastState)}`);
}

async function screenshotClient(context, session, label) {
  const path = join(artifactDir, `${label}.png`);
  const result = await runCli(context, ['--json', '--session', session, 'screenshot', path], 60000);
  const parsed = parseJsonOutput(result.stdout, `${session} screenshot`);
  assert(parsed.success === true, `${session} screenshot failed: ${result.stdout}${result.stderr}`);
  return path;
}

function imageSize(path) {
  const output = runChecked('identify', ['-format', '%w %h', path], `identify ${path}`).trim();
  const [width, height] = output.split(/\s+/).map((value) => Number.parseInt(value, 10));
  assert(Number.isFinite(width) && Number.isFinite(height), `Could not read image dimensions from ${path}: ${output}`);
  return { width, height };
}

function normalizedWords(value) {
  return value
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);
}

function bindingProofSatisfied(text, expectedMarker) {
  const ocrWords = new Set(normalizedWords(text));
  const expectedWords = normalizedWords(expectedMarker);
  if (expectedWords.every((word) => ocrWords.has(word))) return true;

  const match = expectedMarker.match(/\bBROWSER\s+([AB])\s+([0-9]+)\b/i);
  if (!match) return false;
  const [, browserLetter, runNumber] = match;
  const compactText = text.toUpperCase().replace(/[^A-Z0-9]+/g, '');
  const hasBinding = ocrWords.has('BINDING') || compactText.includes('BINDING');
  const hasRouteRunNumber = (ocrWords.has(browserLetter.toUpperCase()) && ocrWords.has(runNumber)) ||
    compactText.includes(`${browserLetter.toUpperCase()}${runNumber}`);
  return hasBinding && hasRouteRunNumber;
}

function cropAndOcrIframe({ screenshotPath, tileState, route, expectedMarker, label }) {
  const frame = (tileState.frames || []).find((item) => {
    const src = item.src || '';
    return src.includes(route.frameUrl || route.externalUrl);
  });
  assert(frame?.iframeRect, `${label} missing iframe rectangle for route ${route.routeId}: ${JSON.stringify(tileState)}`);

  const dimensions = imageSize(screenshotPath);
  const viewport = tileState.viewportSize || { width: dimensions.width, height: dimensions.height };
  const scaleX = dimensions.width / viewport.width;
  const scaleY = dimensions.height / viewport.height;
  const crop = {
    x: Math.max(0, Math.floor(frame.iframeRect.x * scaleX)),
    y: Math.max(0, Math.floor(frame.iframeRect.y * scaleY)),
    width: Math.max(1, Math.floor(frame.iframeRect.width * scaleX)),
    height: Math.max(1, Math.floor(frame.iframeRect.height * scaleY)),
  };
  const cropPath = join(artifactDir, `${label}-${route.id}-iframe.png`);
  const ocrBasePath = join(artifactDir, `${label}-${route.id}-ocr`);
  const ocrTextPath = `${ocrBasePath}.txt`;
  runChecked(
    'convert',
    [
      screenshotPath,
      '-crop',
      `${crop.width}x${crop.height}+${crop.x}+${crop.y}`,
      '+repage',
      '-colorspace',
      'Gray',
      cropPath,
    ],
    `${label} iframe crop`,
  );
  runChecked('tesseract', [cropPath, ocrBasePath, '--psm', '6'], `${label} iframe OCR`);
  const text = existsSync(ocrTextPath) ? readFileSync(ocrTextPath, 'utf8') : '';
  writeTextArtifact(`${label}-${route.id}-ocr-normalized.txt`, text.replace(/\s+/g, ' ').trim());
  assert(
    bindingProofSatisfied(text, expectedMarker),
    `${label} did not prove route ${route.routeId} shows ${expectedMarker}. OCR text: ${text}`,
  );
  return { crop, cropPath, ocrTextPath, text };
}

function assertVisualBindingProof({ screenshotPath, tileState, bindings, label }) {
  const proof = bindings.map((binding) => cropAndOcrIframe({
    screenshotPath,
    tileState,
    route: binding.route,
    expectedMarker: binding.expectedMarker,
    label,
  }));
  writeArtifact(`${label}-target-binding-proof.json`, proof.map((item) => ({
    crop: item.crop,
    cropPath: item.cropPath,
    ocrTextPath: item.ocrTextPath,
    textPreview: item.text.replace(/\s+/g, ' ').trim().slice(0, 240),
  })));
}

async function waitForVisualBindingProof({ context, session, routes, bindings, label, timeoutMs = 90000 }) {
  const started = Date.now();
  let lastError = null;
  let attempt = 0;
  while (Date.now() - started < timeoutMs) {
    attempt += 1;
    const tileState = await waitForTileState(context, session, routes, `${label}-attempt-${attempt}`, 30000);
    writeArtifact(`${label}-attempt-${attempt}-tile-state.json`, tileState);
    const screenshotPath = await screenshotClient(context, session, `${label}-attempt-${attempt}`);
    try {
      assertVisualBindingProof({
        screenshotPath,
        tileState,
        bindings,
        label: `${label}-attempt-${attempt}`,
      });
      writeArtifact(`${label}-visual-binding-proof.json`, {
        attempt,
        screenshotPath,
      });
  return { attempt, screenshotPath, tileState };
    } catch (error) {
      lastError = error;
      await delay(3000);
    }
  }
  throw lastError || new Error(`${label} did not produce visual binding proof before timeout`);
}

async function requestViewerAndControllerLeases(streamPort, routeA, routeB) {
  const viewer1A = await serviceRequest(streamPort, {
    action: 'service_viewer_lease_request',
    serviceName,
    agentName,
    taskName: viewerTaskName,
    params: { routeId: routeA.routeId, viewerId: 'viewer-1-a', viewerName: 'Viewer 1', openMode: 'tile' },
    jobTimeoutMs: 30000,
  }, 'viewer-1-route-a');
  const viewer2A = await serviceRequest(streamPort, {
    action: 'service_viewer_lease_request',
    serviceName,
    agentName,
    taskName: viewerTaskName,
    params: { routeId: routeA.routeId, viewerId: 'viewer-2-a', viewerName: 'Viewer 2', openMode: 'tile' },
    jobTimeoutMs: 30000,
  }, 'viewer-2-route-a');
  const viewer1B = await serviceRequest(streamPort, {
    action: 'service_viewer_lease_request',
    serviceName,
    agentName,
    taskName: viewerTaskName,
    params: { routeId: routeB.routeId, viewerId: 'viewer-1-b', viewerName: 'Viewer 1', openMode: 'tile' },
    jobTimeoutMs: 30000,
  }, 'viewer-1-route-b');
  const viewer2B = await serviceRequest(streamPort, {
    action: 'service_viewer_lease_request',
    serviceName,
    agentName,
    taskName: viewerTaskName,
    params: { routeId: routeB.routeId, viewerId: 'viewer-2-b', viewerName: 'Viewer 2', openMode: 'tile' },
    jobTimeoutMs: 30000,
  }, 'viewer-2-route-b');
  const controllerA = await serviceRequest(streamPort, {
    action: 'service_controller_lease_takeover',
    serviceName,
    agentName,
    taskName: controllerTaskName,
    params: { routeId: routeA.routeId, viewerLeaseId: viewer1A.data.viewerLeaseId, viewerId: 'viewer-1' },
    jobTimeoutMs: 30000,
  }, 'controller-viewer-1-route-a');
  const controllerB = await serviceRequest(streamPort, {
    action: 'service_controller_lease_takeover',
    serviceName,
    agentName,
    taskName: controllerTaskName,
    params: { routeId: routeB.routeId, viewerLeaseId: viewer2B.data.viewerLeaseId, viewerId: 'viewer-2' },
    jobTimeoutMs: 30000,
  }, 'controller-viewer-2-route-b');
  return { viewer1A, viewer2A, viewer1B, viewer2B, controllerA, controllerB };
}

async function closeRemoteBrowser(context, workspace) {
  if (workspace?.streamPort && workspace?.browserId) {
    try {
      await serviceRequest(workspace.streamPort, {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: closeTaskName,
        params: { browserId: workspace.browserId },
        jobTimeoutMs: 30000,
      }, `${workspace.sessionName}-close`);
    } catch {
      // Session close below is the final cleanup path.
    }
  }
  if (workspace?.sessionName) {
    try {
      await runCli(context, ['--json', '--session', workspace.sessionName, 'close'], 30000);
    } catch {
      // Best-effort cleanup after failed launch or forced shutdown.
    }
  }
}

const timeout = setTimeout(() => {
  console.error('Timed out waiting for many-to-many live smoke to complete');
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, 720000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-many-to-many-',
  sessionPrefix: 'rdp-guac-many-a',
});
context.env.AGENT_BROWSER_DASHBOARD_AUTH_FILE = join(context.agentHome, 'dashboard-auth.json');
context.env.AGENT_BROWSER_ENGINE = 'chrome';
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
if (!process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE && existsSync('/usr/bin/google-chrome-stable')) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = '/usr/bin/google-chrome-stable';
} else if (process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE;
}
delete context.env.AGENT_BROWSER_CDP;
delete context.env.AGENT_BROWSER_AUTO_CONNECT;

let browserA = null;
let browserB = null;

async function cleanup() {
  clearTimeout(timeout);
  for (const session of [clientASession, clientBSession]) {
    try {
      await runCli(context, ['--json', '--session', session, 'close'], 30000);
    } catch {
      // Dashboard client cleanup is best effort.
    }
  }
  await closeRemoteBrowser(context, browserB);
  await closeRemoteBrowser(context, browserA);
  cleanupSmokeHome(context);
}

try {
  assert(commandExists('identify'), 'ImageMagick identify is required for target-binding screenshot proof');
  assert(commandExists('convert'), 'ImageMagick convert is required for target-binding screenshot proof');
  assert(commandExists('tesseract'), 'tesseract is required for target-binding OCR proof');
  const routes = requireDistinctRoutePool();
  const clientAExecutable = envValue('AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE');
  const clientBExecutable = envValue('AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE') || clientAExecutable;
  assert(clientAExecutable && existsSync(clientAExecutable), 'AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE must point to a browser executable');
  assert(clientBExecutable && existsSync(clientBExecutable), 'AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE must point to a browser executable');
  const profileA = envValue('AGENT_BROWSER_RDP_TEST_PROFILE_A') || join(context.tempHome, 'viewer-a-profile');
  const profileB = envValue('AGENT_BROWSER_RDP_TEST_PROFILE_B') || join(context.tempHome, 'viewer-b-profile');
  const remoteConfig = configureRemoteHeadedContext(context);
  const browserBSession = envValue('AGENT_BROWSER_RDP_TEST_BROWSER_B') || `rdp-guac-many-b-${process.pid}`;

  browserA = await launchRemoteBrowser({
    context,
    remoteConfig,
    route: routes[0],
    sessionName: context.session,
    title: browserAMarker,
  });
  browserB = await launchRemoteBrowser({
    context,
    remoteConfig,
    route: routes[1],
    sessionName: browserBSession,
    title: browserBMarker,
  });

  const afterLaunch = await serviceStatusArtifact(browserA.streamPort, 'after-two-route-launch');
  const serviceState = afterLaunch.data?.service_state || {};
  assert(browserA.displayAllocationId !== browserB.displayAllocationId, 'Browsers did not get distinct display allocations');
  if (browserA.displayName || browserB.displayName) {
    assert(
      browserA.displayName && browserB.displayName && browserA.displayName !== browserB.displayName,
      `Browsers did not launch on distinct display names: ${JSON.stringify({ browserA, browserB })}`,
    );
  }
  assert(
    serviceState.routePool?.[routes[0].id]?.currentRouteAllocationId === routes[0].routeId &&
      serviceState.routePool?.[routes[1].id]?.currentRouteAllocationId === routes[1].routeId,
    `Route pool entries were not checked out distinctly: ${JSON.stringify(serviceState.routePool)}`,
  );
  assert(
    serviceState.remoteViewRoutes?.[routes[0].routeId]?.state === 'ready' &&
      serviceState.remoteViewRoutes?.[routes[1].routeId]?.state === 'ready',
    `Remote view routes were not ready: ${JSON.stringify(serviceState.remoteViewRoutes)}`,
  );

  const leases = await requestViewerAndControllerLeases(browserA.streamPort, routes[0], routes[1]);
  const afterLeases = await serviceStatusArtifact(browserA.streamPort, 'after-viewer-controller-leases');
  writeArtifact('viewer-controller-lease-proof.json', leases);

  const baseDashboardUrl = envValue('AGENT_BROWSER_RDP_TEST_PUBLIC_URL') || `http://127.0.0.1:${browserA.streamPort}/`;
  const dashboardTileUrl = tileUrl(baseDashboardUrl);
  writeArtifact('fixture.json', {
    artifactDir,
    browserA,
    browserB,
    dashboardTileUrl,
    routePool: routes,
    viewerLeaseCount: Object.keys(afterLeases.data?.service_state?.viewerLeases || {}).length,
  });

  const clientAViewport = { width: 1500, height: 950 };
  await loginGuacamoleClient(context, {
    executable: clientAExecutable,
    profile: profileA,
    session: clientASession,
    route: routes[0],
    viewport: clientAViewport,
  });
  await openDashboardClient(context, {
    executable: clientAExecutable,
    profile: profileA,
    session: clientASession,
    url: dashboardTileUrl,
    viewport: clientAViewport,
  });
  const credentials = await waitForDashboardCredentials(context);
  await loginDashboardClient(context, clientASession, credentials);
  await waitForVisualBindingProof({
    context,
    session: clientASession,
    routes,
    bindings: [
      { route: routes[0], expectedMarker: browserAMarker },
      { route: routes[1], expectedMarker: browserBMarker },
    ],
    label: 'viewer-1-tile',
  });

  const clientBViewport = { width: 1366, height: 860 };
  await loginGuacamoleClient(context, {
    executable: clientBExecutable,
    profile: profileB,
    session: clientBSession,
    route: routes[1],
    viewport: clientBViewport,
  });
  await openDashboardClient(context, {
    executable: clientBExecutable,
    profile: profileB,
    session: clientBSession,
    url: dashboardTileUrl,
    viewport: clientBViewport,
  });
  await loginDashboardClient(context, clientBSession, credentials);
  await waitForVisualBindingProof({
    context,
    session: clientBSession,
    routes,
    bindings: [
      { route: routes[0], expectedMarker: browserAMarker },
      { route: routes[1], expectedMarker: browserBMarker },
    ],
    label: 'viewer-2-tile',
  });

  await evalInClient(context, clientASession, `
(() => {
  const button = Array.from(document.querySelectorAll("button")).find((item) => item.getAttribute("aria-label") === ${JSON.stringify(`Refresh ${browserA.browserId}`)});
  if (!button) return { clicked: false, reason: "missing" };
  button.click();
  return { clicked: true };
})()
`);
  await waitForVisualBindingProof({
    context,
    session: clientASession,
    routes,
    bindings: [
      { route: routes[0], expectedMarker: browserAMarker },
      { route: routes[1], expectedMarker: browserBMarker },
    ],
    label: 'viewer-1-after-browser-a-refresh',
  });

  await closeRemoteBrowser(context, browserA);
  const afterCloseA = await serviceStatusArtifact(browserB.streamPort, 'after-close-browser-a');
  const stateAfterCloseA = afterCloseA.data?.service_state || {};
  assert(
    stateAfterCloseA.displayAllocations?.[browserA.displayAllocationId]?.state === 'released',
    `Browser A display was not released: ${JSON.stringify(stateAfterCloseA.displayAllocations?.[browserA.displayAllocationId])}`,
  );
  assert(
    stateAfterCloseA.routePool?.[routes[0].id]?.state === 'available',
    `Browser A route was not released: ${JSON.stringify(stateAfterCloseA.routePool?.[routes[0].id])}`,
  );
  assert(
    stateAfterCloseA.browsers?.[browserB.browserId]?.health === 'ready' &&
      stateAfterCloseA.routePool?.[routes[1].id]?.state === 'checked_out',
    `Browser B did not remain ready after closing A: ${JSON.stringify({
      browser: stateAfterCloseA.browsers?.[browserB.browserId],
      routePool: stateAfterCloseA.routePool?.[routes[1].id],
    })}`,
  );
  await waitForVisualBindingProof({
    context,
    session: clientBSession,
    routes: [routes[1]],
    bindings: [
      { route: routes[1], expectedMarker: browserBMarker },
    ],
    label: 'viewer-2-after-close-browser-a',
  });

  writeArtifact('summary.json', {
    artifactDir,
    browserA: {
      browserId: browserA.browserId,
      displayAllocationId: browserA.displayAllocationId,
      visualBindingMarker: browserAMarker,
      route: routes[0],
    },
    browserB: {
      browserId: browserB.browserId,
      displayAllocationId: browserB.displayAllocationId,
      visualBindingMarker: browserBMarker,
      route: routes[1],
    },
    dashboardTileUrl,
  });

  browserA = null;
  await cleanup();
  console.log(`RDP Guacamole many-to-many live smoke passed; artifacts: ${artifactDir}`);
} catch (err) {
  writeArtifact('failure.json', {
    error: err.stack || err.message,
    routePoolConfigHint: {
      json: 'AGENT_BROWSER_RDP_ROUTE_POOL_JSON',
      routeA: 'AGENT_BROWSER_RDP_ROUTE_A_ID, AGENT_BROWSER_RDP_ROUTE_A_FRAME_URL, AGENT_BROWSER_RDP_ROUTE_A_CONNECTION_ID',
      routeB: 'AGENT_BROWSER_RDP_ROUTE_B_ID, AGENT_BROWSER_RDP_ROUTE_B_FRAME_URL, AGENT_BROWSER_RDP_ROUTE_B_CONNECTION_ID',
    },
  });
  console.error(err.stack || err.message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}
