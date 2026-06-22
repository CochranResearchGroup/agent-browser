#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { requestServiceRemoteViewOpen } from '../packages/client/src/service-request.js';
import { assert, parseJsonOutput } from './smoke-utils.js';
import { loadAgentBrowserEnvFromRealHome } from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_SERVICE_NAME || 'RemoteViewOpenLiveSmoke';
const agentName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_AGENT_NAME || 'smoke-agent';
const runtimeProfile = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_RUNTIME_PROFILE || 'stealthcdp-default';
const useFixture = process.argv.includes('--fixture') || process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_FIXTURE === '1';
const displayName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_DISPLAY || ':10';
const displayIsolation = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_DISPLAY_ISOLATION || 'shared_display';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-remote-view-open-live-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

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

function createFixtureServer() {
  const marker = `REMOTE VIEW OPEN FIXTURE ${process.pid}`;
  const html = [
    '<!doctype html>',
    '<html>',
    `<head><title>${marker}</title></head>`,
    '<body style="margin:0;background:#fff;color:#111;font-family:Arial,sans-serif;">',
    '<main style="min-height:100vh;display:grid;place-items:center;text-align:center;">',
    `<h1 id="ready" style="font-size:42px;">${marker}</h1>`,
    '</main>',
    '</body>',
    '</html>',
  ].join('');
  const server = createServer((_req, res) => {
    res.writeHead(200, {
      'content-type': 'text/html; charset=utf-8',
      'cache-control': 'no-store',
    });
    res.end(html);
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      resolve({
        marker,
        targetUrl: `http://127.0.0.1:${address.port}/`,
        close: () => new Promise((done) => server.close(done)),
      });
    });
  });
}

function envValue(name) {
  const value = process.env[name]?.trim();
  return value || null;
}

function commandExists(command) {
  const result = spawnSync('sh', ['-lc', `command -v ${command}`], {
    encoding: 'utf8',
    stdio: 'pipe',
  });
  return result.status === 0 ? result.stdout.trim() : null;
}

function agentBrowserCommand() {
  return envValue('AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD') ||
    envValue('AGENT_BROWSER_COMMAND') ||
    commandExists('agent-browser') ||
    'agent-browser';
}

const agentBrowser = agentBrowserCommand();

function run(command, args, label, { allowFailure = false, timeoutMs = 120000 } = {}) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
    timeout: timeoutMs,
  });
  if (!allowFailure) {
    assert(
      result.status === 0,
      `${label} failed: ${command} ${args.join(' ')}\n${result.stdout}${result.stderr}`,
    );
  }
  return result;
}

function runAgentJson(args, label, timeoutMs = 120000) {
  const result = run(agentBrowser, ['--json', ...args], label, { timeoutMs });
  return parseJsonOutput(result.stdout, label);
}

function routePoolReadiness() {
  if (envValue('AGENT_BROWSER_RDP_ROUTE_POOL_JSON')) {
    return {
      success: true,
      status: 'ready',
      routePoolJson: JSON.parse(envValue('AGENT_BROWSER_RDP_ROUTE_POOL_JSON')),
      source: 'AGENT_BROWSER_RDP_ROUTE_POOL_JSON',
    };
  }
  const result = run(process.execPath, ['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], 'route pool readiness');
  const parsed = parseJsonOutput(result.stdout, 'route pool readiness');
  parsed.source = 'scripts/smoke-rdp-guac-route-pool-readiness.js';
  return parsed;
}

function selectRouteEntry(report) {
  assert(report.success === true, `route pool readiness is not green: ${JSON.stringify(report)}`);
  const entries = report.routePoolJson;
  assert(Array.isArray(entries) && entries.length > 0, `route pool readiness did not return entries: ${JSON.stringify(report)}`);
  const selectedId = envValue('AGENT_BROWSER_REMOTE_VIEW_OPEN_ROUTE_POOL_ENTRY_ID');
  const selected = selectedId ? entries.find((entry) => entry?.id === selectedId || entry?.routeId === selectedId) : entries[0];
  assert(selected, `selected route-pool entry ${selectedId} was not found in readiness output`);
  assert(selected.routeId || selected.connectionId, `selected route-pool entry is missing route identity: ${JSON.stringify(selected)}`);
  assert(selected.frameUrl || selected.externalUrl, `selected route-pool entry is missing route URL: ${JSON.stringify(selected)}`);
  return { entries, selected };
}

function displayNameForRoute(routeEntry) {
  return routeEntry?.target?.displayName || displayName;
}

function displayIsolationForRoute(routeEntry) {
  return routeEntry?.target?.displayIsolation || displayIsolation;
}

function visibleWindowProof(response, label) {
  const proof = response.data?.verification?.visibleWindowProof;
  assert(proof?.state === 'ready', `${label} visible window proof is not ready: ${JSON.stringify(response)}`);
  const displayContent = proof.displayContent;
  assert(
    displayContent?.state === 'browser_window_visible',
    `${label} did not prove a visible browser window: ${JSON.stringify(displayContent || proof)}`,
  );
  return proof;
}

function routeIdFromResponse(response, label) {
  const routeId = response.data?.route?.routeId || response.data?.routeId;
  assert(typeof routeId === 'string' && routeId.length > 0, `${label} did not return routeId: ${JSON.stringify(response)}`);
  return routeId;
}

function displayAllocationIdFromResponse(response, label) {
  const displayAllocationId = response.data?.route?.displayAllocationId || response.data?.displayAllocationId;
  assert(
    typeof displayAllocationId === 'string' && displayAllocationId.length > 0,
    `${label} did not return displayAllocationId: ${JSON.stringify(response)}`,
  );
  return displayAllocationId;
}

function assertOpenResponse(response, label) {
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  assert(response.data?.status === 'opened', `${label} did not return opened status: ${JSON.stringify(response)}`);
  visibleWindowProof(response, label);
  return {
    routeId: routeIdFromResponse(response, label),
    displayAllocationId: displayAllocationIdFromResponse(response, label),
  };
}

function remoteViewOpenCliArgs(routeEntry, taskName, targetUrl) {
  const routeDisplayName = displayNameForRoute(routeEntry);
  return [
    'remote-view',
    'open',
    targetUrl,
    '--runtime-profile',
    runtimeProfile,
    '--display',
    routeDisplayName,
    '--display-isolation',
    displayIsolationForRoute(routeEntry),
    '--route-pool-entry-json',
    JSON.stringify(routeEntry),
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
  ];
}

function stateRecords(status, routeId) {
  const state = status.data?.service_state || status.data || {};
  const route = state.remoteViewRoutes?.[routeId];
  assert(route, `service status is missing route ${routeId}`);
  const browser = state.browsers?.[route.browserId];
  assert(browser, `service status is missing browser ${route.browserId}`);
  const allocation = state.displayAllocations?.[route.displayAllocationId] ||
    state.remoteViewDisplayAllocations?.[route.displayAllocationId];
  assert(allocation, `service status is missing display allocation ${route.displayAllocationId}`);
  const stream = [
    ...Object.values(state.viewStreams || {}),
    ...Object.values(state.browsers || {}).flatMap((candidate) => candidate?.viewStreams || []),
  ].find((candidate) => candidate?.routeId === routeId);
  assert(stream, `service status is missing retained stream for ${routeId}`);
  return { allocation, browser, route, stream };
}

function assertServiceState(status, expected, label) {
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  const { allocation, browser, route, stream } = stateRecords(status, expected.routeId);
  assert(route.state === 'ready', `${label} route is not ready: ${JSON.stringify(route)}`);
  assert(route.displayAllocationId === expected.displayAllocationId, `${label} route display allocation mismatch: ${JSON.stringify(route)}`);
  assert(route.browserId === browser.id, `${label} route browser mismatch: ${JSON.stringify({ route, browser })}`);
  assert(browser.host === 'remote_headed', `${label} browser is not remote_headed: ${JSON.stringify(browser)}`);
  assert(browser.profileId === runtimeProfile, `${label} browser profile mismatch: ${JSON.stringify(browser)}`);
  assert(browser.displayName === expected.displayName, `${label} browser display mismatch: ${JSON.stringify(browser)}`);
  assert(allocation.state === 'ready', `${label} allocation is not ready: ${JSON.stringify(allocation)}`);
  assert(allocation.ownerBrowserId === browser.id, `${label} allocation browser mismatch: ${JSON.stringify({ allocation, browser })}`);
  assert(Array.isArray(allocation.routeIds) && allocation.routeIds.includes(expected.routeId), `${label} allocation route mismatch: ${JSON.stringify(allocation)}`);
  assert(stream.displayAllocationId === expected.displayAllocationId, `${label} stream allocation mismatch: ${JSON.stringify(stream)}`);
  assert(stream.remoteReadiness?.state === 'ready', `${label} stream readiness is not ready: ${JSON.stringify(stream)}`);
  assert(
    stream.remoteReadiness?.displayContent?.state === 'browser_window_visible',
    `${label} stream does not retain browser-window proof: ${JSON.stringify(stream)}`,
  );
  return { allocation, browser, route, stream };
}

function assertXwininfo(display) {
  const result = run('xwininfo', ['-display', display, '-root', '-tree'], 'xwininfo route display', {
    allowFailure: true,
    timeoutMs: 30000,
  });
  writeTextArtifact('xwininfo.txt', `${result.stdout}${result.stderr}`);
  assert(result.status === 0, `xwininfo failed for ${display}: ${result.stdout}${result.stderr}`);
  assert(
    /Chrom(e|ium)/i.test(result.stdout),
    `xwininfo for ${display} did not show Chrome or Chromium windows`,
  );
}

function assertX11BrowserWindowPid(display, browserPid) {
  assert(Number.isInteger(browserPid) && browserPid > 0, `browser PID is missing: ${browserPid}`);
  const root = run('xprop', ['-display', display, '-root', '_NET_CLIENT_LIST'], 'xprop route client list', {
    allowFailure: true,
    timeoutMs: 30000,
  });
  assert(root.status === 0, `xprop failed for ${display}: ${root.stdout}${root.stderr}`);
  const [, rawIds = ''] = root.stdout.split('#');
  const windowIds = rawIds
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
  assert(windowIds.length > 0, `xprop did not return client windows for ${display}: ${root.stdout}`);

  const windows = windowIds.map((id) => {
    const result = run('xprop', ['-display', display, '-id', id, 'WM_CLASS', 'WM_NAME', '_NET_WM_PID'], `xprop ${id}`, {
      allowFailure: true,
      timeoutMs: 30000,
    });
    const pid = Number(result.stdout.match(/_NET_WM_PID\(CARDINAL\) = (\d+)/)?.[1] || 0);
    return {
      id,
      ok: result.status === 0,
      pid: Number.isInteger(pid) && pid > 0 ? pid : null,
      raw: result.stdout.trim(),
    };
  });
  writeArtifact('x11-window-pids.json', { display, browserPid, windows });
  const matchingWindow = windows.find((window) => window.pid === browserPid && /Chrom(e|ium)/i.test(window.raw));
  assert(
    matchingWindow,
    `no Chrome or Chromium X11 window on ${display} matched browser PID ${browserPid}: ${JSON.stringify(windows)}`,
  );
  return matchingWindow;
}

async function ensureStreamBaseUrl() {
  const status = runAgentJson(['stream', 'status'], 'stream status');
  if (!status.data?.enabled) {
    const enabled = runAgentJson(['stream', 'enable'], 'stream enable');
    assert(enabled.success === true, `stream enable failed: ${JSON.stringify(enabled)}`);
    const port = enabled.data?.port;
    assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(enabled)}`);
    return `http://127.0.0.1:${port}`;
  }
  const port = status.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(status)}`);
  return `http://127.0.0.1:${port}`;
}

async function main() {
  const fixture = useFixture ? await createFixtureServer() : null;
  const targetUrl = fixture?.targetUrl || process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_URL || 'https://www.linkedin.com/';
  const readiness = routePoolReadiness();
  const { entries, selected } = selectRouteEntry(readiness);
  const routeDisplayName = displayNameForRoute(selected);
  const routeDisplayIsolation = displayIsolationForRoute(selected);
  writeArtifact('route-pool-readiness.json', readiness);

  try {
    const first = runAgentJson(
      remoteViewOpenCliArgs(selected, 'remoteViewOpenLiveCliFirst', targetUrl),
      'remote-view open CLI first',
      180000,
    );
    const firstIds = assertOpenResponse(first, 'CLI first open');
    writeArtifact('cli-first.json', first);

    const repeat = runAgentJson(
      remoteViewOpenCliArgs(selected, 'remoteViewOpenLiveCliRepeat', targetUrl),
      'remote-view open CLI repeat',
      180000,
    );
    const repeatIds = assertOpenResponse(repeat, 'CLI repeat open');
    writeArtifact('cli-repeat.json', repeat);
    assert(repeatIds.routeId === firstIds.routeId, `CLI repeat route changed: ${JSON.stringify({ firstIds, repeatIds })}`);
    assert(
      repeatIds.displayAllocationId === firstIds.displayAllocationId,
      `CLI repeat display allocation changed: ${JSON.stringify({ firstIds, repeatIds })}`,
    );

    const baseUrl = await ensureStreamBaseUrl();
    const http = await requestServiceRemoteViewOpen({
      baseUrl,
      serviceName,
      agentName,
      taskName: 'remoteViewOpenLiveHttpHelper',
      runtimeProfile,
      display: routeDisplayName,
      displayName: routeDisplayName,
      remoteHeadedDisplay: routeDisplayName,
      displayIsolation: routeDisplayIsolation,
      routeId: selected.routeId,
      remoteViewRouteId: selected.routeId,
      routePoolEntryId: selected.id,
      routePoolEntry: selected,
      routePool: entries,
      routeDescriptor: selected.routeDescriptor,
      provider: 'rdp_gateway',
      providerMode: selected.providerMode || selected.routeDescriptor?.providerMode || 'simultaneous_view',
      frameUrl: selected.frameUrl,
      externalUrl: selected.externalUrl,
      connectionId: selected.connectionId,
      connectionName: selected.connectionName,
      url: targetUrl,
      jobTimeoutMs: 180000,
    });
    const httpIds = {
      ...assertOpenResponse(http, 'HTTP helper open'),
      displayName: routeDisplayName,
    };
    writeArtifact('http-helper.json', http);
    assert(httpIds.routeId === firstIds.routeId, `HTTP helper route changed: ${JSON.stringify({ firstIds, httpIds })}`);
    assert(
      httpIds.displayAllocationId === firstIds.displayAllocationId,
      `HTTP helper display allocation changed: ${JSON.stringify({ firstIds, httpIds })}`,
    );

    const url = runAgentJson(['get', 'url'], 'get url');
    const title = runAgentJson(['get', 'title'], 'get title');
    writeArtifact('cdp-readback.json', { url, title });
    if (fixture) {
      assert(url.data?.url === fixture.targetUrl, `CDP URL readback is not fixture URL: ${JSON.stringify(url)}`);
      assert(title.data?.title === fixture.marker, `CDP title readback is not fixture title: ${JSON.stringify(title)}`);
    } else {
      assert(url.data?.url?.includes('linkedin.com'), `CDP URL readback is not LinkedIn: ${JSON.stringify(url)}`);
      assert(typeof title.data?.title === 'string' && title.data.title.length > 0, `CDP title readback missing: ${JSON.stringify(title)}`);
    }

    const serviceStatus = runAgentJson(['service', 'status'], 'service status');
    const records = assertServiceState(serviceStatus, httpIds, 'post-open');
    writeArtifact('service-state-proof.json', {
      route: records.route,
      browser: records.browser,
      allocation: records.allocation,
      stream: records.stream,
    });

    assertXwininfo(routeDisplayName);
    const matchingWindow = assertX11BrowserWindowPid(routeDisplayName, records.browser.pid);

    const summary = {
      success: true,
      artifactDir,
      command: agentBrowser,
      runtimeProfile,
      displayName: routeDisplayName,
      fixture: Boolean(fixture),
      fixtureMarker: fixture?.marker || null,
      routeId: httpIds.routeId,
      displayAllocationId: httpIds.displayAllocationId,
      frameUrl: records.route.frameUrl,
      externalUrl: records.route.externalUrl,
      title: title.data.title,
      url: url.data.url,
      x11WindowId: matchingWindow.id,
      x11WindowPid: matchingWindow.pid,
    };
    writeArtifact('summary.json', summary);
    console.log(JSON.stringify(summary, null, 2));
  } finally {
    await fixture?.close();
  }
}

main().then(() => {
  process.exit(0);
}).catch((error) => {
  const failure = {
    success: false,
    artifactDir,
    message: error instanceof Error ? error.message : String(error),
    stack: error instanceof Error ? error.stack : null,
  };
  writeArtifact('failure.json', failure);
  console.error(JSON.stringify(failure, null, 2));
  process.exit(1);
});
