#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  requestServiceRemoteViewOpen,
  requestServiceRoutePoolRepair,
} from '../packages/client/src/service-request.js';
import { buildAudit } from './audit-route-handoff.js';
import { assert, parseJsonOutput } from './smoke-utils.js';
import { loadAgentBrowserEnvFromRealHome } from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_SERVICE_NAME || 'RemoteViewOpenLiveSmoke';
const agentName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_AGENT_NAME || 'smoke-agent';
const daemonSession = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_DAEMON_SESSION || `remote-view-open-live-${process.pid}`;
const runtimeProfile = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_RUNTIME_PROFILE || `${daemonSession}-profile`;
const useFixture = process.argv.includes('--fixture') || process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_FIXTURE === '1';
const forceProofFailure = process.argv.includes('--force-proof-failure') ||
  process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_FORCE_PROOF_FAILURE === '1';
const displayName = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_DISPLAY || ':10';
const displayIsolation = process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_DISPLAY_ISOLATION || 'shared_display';
const remoteViewOpenTimeoutMs = Number(process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_TIMEOUT_MS || 300000);
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
  const ocrMarker = 'REMOTE VIEW OPEN FIXTURE';
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
        ocrMarker,
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
    maxBuffer: 64 * 1024 * 1024,
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
  let result = null;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    result = run(agentBrowser, ['--session', daemonSession, '--json', ...args], label, {
      allowFailure: true,
      timeoutMs,
    });
    if (result.status === 0) break;
    const output = `${result.stdout}${result.stderr}`;
    if (!output.includes('Daemon failed to start') || attempt === 3) break;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1500 * attempt);
  }
  assert(
    result?.status === 0,
    `${label} failed: ${agentBrowser} --session ${daemonSession} --json ${args.join(' ')}\n${result?.stdout ?? ''}${result?.stderr ?? ''}`,
  );
  return parseJsonOutput(result.stdout, label);
}

function runAgentJsonExpectFailure(args, label, timeoutMs = 120000) {
  const previous = process.env.AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE;
  process.env.AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE = '1';
  try {
    const result = run(agentBrowser, ['--session', daemonSession, '--json', ...args], label, {
      allowFailure: true,
      timeoutMs,
    });
    assert(
      result.status !== 0,
      `${label} unexpectedly succeeded: ${result.stdout}${result.stderr}`,
    );
    const parsed = parseJsonOutput(result.stdout, label);
    assert(parsed.success === false, `${label} did not return success=false: ${JSON.stringify(parsed)}`);
    return parsed;
  } finally {
    if (previous === undefined) {
      delete process.env.AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE;
    } else {
      process.env.AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE = previous;
    }
  }
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

function selectRouteEntry(report, serviceStatus = null) {
  assert(report.success === true, `route pool readiness is not green: ${JSON.stringify(report)}`);
  const entries = report.routePoolJson;
  assert(Array.isArray(entries) && entries.length > 0, `route pool readiness did not return entries: ${JSON.stringify(report)}`);
  const liveRoutePool = serviceStatus?.data?.service_state?.routePool || serviceStatus?.data?.routePool || {};
  const liveEntries = Object.values(liveRoutePool);
  const entriesWithLiveState = entries.map((entry) => {
    const live = liveRoutePool[entry?.id] || liveEntries.find((candidate) => candidate?.routeId === entry?.routeId);
    if (!live) return entry;
    const sameRoute = live.routeId && entry?.routeId && live.routeId === entry.routeId;
    const sameConnection = live.connectionId && entry?.connectionId && String(live.connectionId) === String(entry.connectionId);
    if (!sameRoute && !sameConnection) {
      return {
        ...entry,
        stalePersistedRoutePoolEntry: {
          id: live.id,
          routeId: live.routeId,
          connectionId: live.connectionId,
          state: live.state,
          currentRouteAllocationId: live.currentRouteAllocationId,
        },
      };
    }
    return {
      ...entry,
      ...live,
      routeDescriptor: live.routeDescriptor || entry.routeDescriptor,
      target: {
        ...(entry.target || {}),
        ...(live.target || {}),
      },
      readiness: live.readiness || entry.readiness,
    };
  });
  const selectedId = envValue('AGENT_BROWSER_REMOTE_VIEW_OPEN_ROUTE_POOL_ENTRY_ID');
  const isAvailable = (entry) => {
    const live = liveRoutePool[entry?.id];
    if (entry?.stalePersistedRoutePoolEntry) return true;
    if (live) {
      const sameRoute = live.routeId && entry?.routeId && live.routeId === entry.routeId;
      const sameConnection = live.connectionId && entry?.connectionId && String(live.connectionId) === String(entry.connectionId);
      if (!sameRoute && !sameConnection) return true;
    }
    return !live || !live.state || live.state === 'available';
  };
  const selected = selectedId
    ? entriesWithLiveState.find((entry) => entry?.id === selectedId || entry?.routeId === selectedId)
    : entriesWithLiveState.find(isAvailable) ?? entriesWithLiveState[0];
  assert(selected, `selected route-pool entry ${selectedId} was not found in readiness output`);
  assert(
    isAvailable(selected),
    `selected route-pool entry ${selected.id} is not available in service state: ${JSON.stringify(liveRoutePool[selected.id])}`,
  );
  assert(selected.routeId || selected.connectionId, `selected route-pool entry is missing route identity: ${JSON.stringify(selected)}`);
  assert(selected.frameUrl || selected.externalUrl, `selected route-pool entry is missing route URL: ${JSON.stringify(selected)}`);
  return { entries: entriesWithLiveState, selected, liveRoutePool };
}

async function repairRoutePoolState(baseUrl, label) {
  const repaired = await requestServiceRoutePoolRepair({
    baseUrl,
    serviceName,
    agentName,
    taskName: label,
    apply: true,
    staleCheckouts: true,
    params: {
      stalePendingAcquisitions: true,
    },
  });
  assert(repaired.success === true, `${label} failed: ${JSON.stringify(repaired)}`);
  return repaired;
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

function targetIdFromOpenResponse(response, label) {
  const targetId = response.data?.operatorVisible?.target?.targetId || response.data?.tab?.targetId;
  assert(typeof targetId === 'string' && targetId.length > 0, `${label} did not return targetId: ${JSON.stringify(response)}`);
  return targetId;
}

function normalizeUrlForCompare(value) {
  return String(value || '').replace(/\/$/, '');
}

function assertRepeatOpenSingleActiveTarget(serviceStatus, records, targetUrl, responses) {
  const state = serviceStatus.data?.service_state || serviceStatus.data || {};
  const responseTargetIds = responses.map(({ response, label }) => targetIdFromOpenResponse(response, label));
  const uniqueResponseTargets = new Set(responseTargetIds);
  assert(
    uniqueResponseTargets.size === 1,
    `repeat remote-view open did not converge to one response target: ${JSON.stringify(responseTargetIds)}`,
  );
  const selectedTargetId = responseTargetIds[0];
  const intendedUrl = normalizeUrlForCompare(targetUrl);
  const retainedTabs = Object.values(state.tabs || {});
  const activeTargetTabs = retainedTabs.filter((tab) => {
    const lifecycle = tab?.lifecycle || 'unknown';
    const tabTargetId = tab?.targetId || String(tab?.id || '').replace(/^target:/, '');
    const tabUrl = normalizeUrlForCompare(tab?.url || tab?.serviceTabHandle?.url);
    return tab?.browserId === records.browser.id &&
      tabTargetId === selectedTargetId &&
      tabUrl === intendedUrl &&
      !['closed', 'released', 'stale'].includes(lifecycle);
  });
  assert(
    activeTargetTabs.length === 1,
    `service state did not retain exactly one active intended target: ${JSON.stringify({ selectedTargetId, intendedUrl, activeTargetTabs, retainedTabs })}`,
  );
  const duplicateIntentTabs = retainedTabs.filter((tab) => {
    const lifecycle = tab?.lifecycle || 'unknown';
    const tabUrl = normalizeUrlForCompare(tab?.url || tab?.serviceTabHandle?.url);
    return tab?.browserId === records.browser.id &&
      tabUrl === intendedUrl &&
      !['closed', 'released', 'stale'].includes(lifecycle);
  });
  assert(
    duplicateIntentTabs.length === 1,
    `repeat remote-view open retained duplicate active intended tabs: ${JSON.stringify(duplicateIntentTabs)}`,
  );
  return {
    selectedTargetId,
    responseTargetIds,
    activeTargetTab: activeTargetTabs[0],
    duplicateIntentTabCount: duplicateIntentTabs.length,
  };
}

function cleanupPayloadFromError(error) {
  const match = String(error || '').match(/cleanup=(\{.*\})$/s);
  assert(match, `forced failure error did not include cleanup JSON: ${error}`);
  return JSON.parse(match[1]);
}

function assertForcedProofFailureResponse(response) {
  assert(response.success === false, `forced proof response unexpectedly succeeded: ${JSON.stringify(response)}`);
  assert(
    String(response.error || '').includes('forced_visible_window_proof_failure'),
    `forced proof response has wrong error: ${JSON.stringify(response)}`,
  );
  const cleanup = cleanupPayloadFromError(response.error);
  const acceptedCleanupStates = new Set(['closed_new_browser', 'closed_opened_tab']);
  assert(acceptedCleanupStates.has(cleanup.state), `forced proof cleanup did not close browser or tab: ${JSON.stringify(cleanup)}`);
  assert(acceptedCleanupStates.has(cleanup.cleanup?.state), `forced proof cleanup payload mismatch: ${JSON.stringify(cleanup)}`);
  assert(cleanup.leaseRollback?.state === 'rolled_back', `forced proof lease rollback missing: ${JSON.stringify(cleanup)}`);
  assert(cleanup.leaseRollback?.phase === 'proof_failed', `forced proof rollback phase mismatch: ${JSON.stringify(cleanup)}`);
  return cleanup;
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

function remoteViewOpenRepeatCliArgs(routeEntry, openedIds, taskName, targetUrl) {
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
    '--route-id',
    openedIds.routeId,
    '--display-allocation-id',
    openedIds.displayAllocationId,
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

function normalizedWords(value) {
  return value
    .toUpperCase()
    .replace(/[^A-Z0-9]+/g, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);
}

function ocrMarkerSatisfied(text, expectedMarker) {
  const ocrWords = new Set(normalizedWords(text));
  const expectedWords = normalizedWords(expectedMarker);
  return expectedWords.every((word) => ocrWords.has(word));
}

function assertRouteDisplayOcr(display, expectedMarker) {
  assert(commandExists('import'), 'ImageMagick import is required for route-display OCR proof');
  assert(commandExists('convert'), 'ImageMagick convert is required for route-display OCR proof');
  assert(commandExists('tesseract'), 'tesseract is required for route-display OCR proof');
  const screenshotPath = join(artifactDir, 'route-display-root.png');
  const grayPath = join(artifactDir, 'route-display-root-gray.png');
  const ocrBasePath = join(artifactDir, 'route-display-root-ocr');
  const ocrTextPath = `${ocrBasePath}.txt`;
  run('import', ['-display', display, '-window', 'root', screenshotPath], 'route display screenshot', {
    timeoutMs: 60000,
  });
  run('convert', [screenshotPath, '-colorspace', 'Gray', grayPath], 'route display screenshot normalize', {
    timeoutMs: 60000,
  });
  run('tesseract', [grayPath, ocrBasePath, '--psm', '6'], 'route display OCR', {
    timeoutMs: 60000,
  });
  const text = existsSync(ocrTextPath) ? readFileSync(ocrTextPath, 'utf8') : '';
  writeTextArtifact('route-display-root-ocr-normalized.txt', text.replace(/\s+/g, ' ').trim());
  assert(
    ocrMarkerSatisfied(text, expectedMarker),
    `route display OCR did not prove fixture marker ${expectedMarker}. OCR text: ${text}`,
  );
  return {
    screenshotPath,
    grayPath,
    ocrTextPath,
    textPreview: text.replace(/\s+/g, ' ').trim().slice(0, 240),
  };
}

function assertRouteHandoffAudit(serviceStatus, expected, label) {
  const audit = buildAudit({
    source: { kind: 'live-smoke', path: 'scripts/smoke-remote-view-open-live.js' },
    serviceStatus,
    remoteViewDoctor: null,
    collectionErrors: [],
  });
  writeArtifact(`${label}-route-handoff-audit.json`, audit);
  const rows = audit.rows.filter((row) =>
    row.routeId === expected.routeId &&
    row.displayAllocationId === expected.displayAllocationId,
  );
  assert(rows.length > 0, `${label} route-handoff audit missed route/display ${JSON.stringify(expected)}`);
  const readyRow = rows.find((row) => row.classification === 'route_bound_ready');
  assert(readyRow, `${label} route-handoff audit did not report route_bound_ready: ${JSON.stringify(rows)}`);
  assert(
    readyRow.visualState === 'browser_window_visible',
    `${label} route-handoff audit did not retain browser-window proof: ${JSON.stringify(readyRow)}`,
  );
  assert(
    readyRow.streamProvider === 'rdp_gateway',
    `${label} route-handoff audit row is not bound to rdp_gateway: ${JSON.stringify(readyRow)}`,
  );
  return readyRow;
}

function assertForcedProofRollback(serviceStatus, selected, cleanup) {
  assert(serviceStatus.success === true, `forced proof service status failed: ${JSON.stringify(serviceStatus)}`);
  const state = serviceStatus.data?.service_state || serviceStatus.data || {};
  const rollback = cleanup.leaseRollback;
  const routePoolEntry = state.routePool?.[selected.id];
  assert(routePoolEntry, `forced proof rollback did not retain route-pool entry ${selected.id}: ${JSON.stringify(state.routePool || {})}`);
  assert(routePoolEntry.state === 'available', `forced proof rollback did not restore route-pool availability: ${JSON.stringify(routePoolEntry)}`);
  assert(
    routePoolEntry.currentRouteAllocationId == null,
    `forced proof rollback left route allocation claimed: ${JSON.stringify(routePoolEntry)}`,
  );
  const display = state.displayAllocations?.[rollback.displayAllocationId];
  assert(!display || display.state !== 'pending', `forced proof rollback left display pending: ${JSON.stringify(display)}`);
  const route = state.remoteViewRoutes?.[rollback.routeId];
  assert(!route || route.state !== 'pending', `forced proof rollback left route pending: ${JSON.stringify(route)}`);
  const lease = state.remoteViewAcquisitionLeases?.[rollback.leaseId];
  assert(lease, `forced proof rollback did not retain acquisition lease ${rollback.leaseId}: ${JSON.stringify(state.remoteViewAcquisitionLeases || {})}`);
  assert(lease.state === 'failed', `forced proof lease state mismatch: ${JSON.stringify(lease)}`);
  assert(lease.phase === 'rollback_complete', `forced proof lease phase mismatch: ${JSON.stringify(lease)}`);
  assert(
    String(lease.failureReason || '').includes('proof_failed'),
    `forced proof lease failure reason mismatch: ${JSON.stringify(lease)}`,
  );
  return { routePoolEntry, display, route, lease };
}

function cleanupLiveSession() {
  if (process.env.AGENT_BROWSER_REMOTE_VIEW_OPEN_PRESERVE === '1') return;
  run(agentBrowser, ['--session', daemonSession, '--json', 'close'], 'cleanup live session', {
    allowFailure: true,
    timeoutMs: 60000,
  });
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
  const baseUrl = await ensureStreamBaseUrl();
  const preRepairStatus = runAgentJson(['service', 'status'], 'pre-repair service status');
  const preRepairRoutePool = preRepairStatus?.data?.service_state?.routePool || preRepairStatus?.data?.routePool || {};
  const hasBlockedRoute = Object.values(preRepairRoutePool).some(
    (entry) => entry?.state && entry.state !== 'available',
  );
  const preOpenRepair = hasBlockedRoute ? await repairRoutePoolState(baseUrl, 'pre-open route-pool repair') : null;
  const preOpenStatus = runAgentJson(['service', 'status'], 'pre-open service status');
  const { entries, selected, liveRoutePool } = selectRouteEntry(readiness, preOpenStatus);
  const routeDisplayName = displayNameForRoute(selected);
  const routeDisplayIsolation = displayIsolationForRoute(selected);
  writeArtifact('route-pool-readiness.json', readiness);
  writeArtifact('pre-open-route-pool-repair.json', {
    ran: Boolean(preOpenRepair),
    response: preOpenRepair,
  });
  writeArtifact('pre-open-route-pool-state.json', {
    selectedRoutePoolEntryId: selected.id,
    liveRoutePool,
  });

  try {
    if (forceProofFailure) {
      const failed = runAgentJsonExpectFailure(
        remoteViewOpenCliArgs(selected, 'remoteViewOpenLiveForcedProofFailure', targetUrl),
        'remote-view open forced proof failure',
        remoteViewOpenTimeoutMs,
      );
      const cleanup = assertForcedProofFailureResponse(failed);
      const postFailureStatus = runAgentJson(['service', 'status'], 'post-forced-proof-failure service status');
      const rollbackState = assertForcedProofRollback(postFailureStatus, selected, cleanup);
      writeArtifact('forced-proof-failure.json', failed);
      writeArtifact('forced-proof-rollback-state.json', rollbackState);
      const summary = {
        success: true,
        mode: 'forced_proof_failure',
        artifactDir,
        command: agentBrowser,
        daemonSession,
        runtimeProfile,
        remoteViewOpenTimeoutMs,
        displayName: routeDisplayName,
        fixture: Boolean(fixture),
        routePoolEntryId: selected.id,
        routeId: cleanup.leaseRollback.routeId,
        displayAllocationId: cleanup.leaseRollback.displayAllocationId,
        acquisitionLeaseId: cleanup.leaseRollback.leaseId,
        cleanupState: cleanup.state,
        rollbackState: cleanup.leaseRollback.state,
      };
      writeArtifact('summary.json', summary);
      console.log(JSON.stringify(summary, null, 2));
      return;
    }

    const first = runAgentJson(
      remoteViewOpenCliArgs(selected, 'remoteViewOpenLiveCliFirst', targetUrl),
      'remote-view open CLI first',
      remoteViewOpenTimeoutMs,
    );
    const firstIds = assertOpenResponse(first, 'CLI first open');
    writeArtifact('cli-first.json', first);

    const repeat = runAgentJson(
      remoteViewOpenRepeatCliArgs(selected, firstIds, 'remoteViewOpenLiveCliRepeat', targetUrl),
      'remote-view open CLI repeat',
      remoteViewOpenTimeoutMs,
    );
    const repeatIds = assertOpenResponse(repeat, 'CLI repeat open');
    writeArtifact('cli-repeat.json', repeat);
    assert(repeatIds.routeId === firstIds.routeId, `CLI repeat route changed: ${JSON.stringify({ firstIds, repeatIds })}`);
    assert(
      repeatIds.displayAllocationId === firstIds.displayAllocationId,
      `CLI repeat display allocation changed: ${JSON.stringify({ firstIds, repeatIds })}`,
    );

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
      viewStreamProvider: 'rdp_gateway',
      providerMode: selected.providerMode || selected.routeDescriptor?.providerMode || 'simultaneous_view',
      frameUrl: selected.frameUrl,
      externalUrl: selected.externalUrl,
      connectionId: selected.connectionId,
      connectionName: selected.connectionName,
      url: targetUrl,
      jobTimeoutMs: remoteViewOpenTimeoutMs,
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
    const repeatOpenTargetProof = assertRepeatOpenSingleActiveTarget(serviceStatus, records, targetUrl, [
      { response: first, label: 'CLI first open' },
      { response: repeat, label: 'CLI repeat open' },
      { response: http, label: 'HTTP helper open' },
    ]);
    const auditRow = assertRouteHandoffAudit(serviceStatus, httpIds, 'post-open');
    writeArtifact('service-state-proof.json', {
      route: records.route,
      browser: records.browser,
      allocation: records.allocation,
      stream: records.stream,
      auditRow,
      repeatOpenTargetProof,
    });

    assertXwininfo(routeDisplayName);
    const matchingWindow = assertX11BrowserWindowPid(routeDisplayName, records.browser.pid);
    const ocrProof = fixture ? assertRouteDisplayOcr(routeDisplayName, fixture.ocrMarker) : null;

    const summary = {
      success: true,
      artifactDir,
      command: agentBrowser,
      daemonSession,
      runtimeProfile,
      remoteViewOpenTimeoutMs,
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
      routeHandoffClassification: auditRow.classification,
      routeHandoffVisualState: auditRow.visualState,
      selectedTargetId: repeatOpenTargetProof.selectedTargetId,
      duplicateIntentTabCount: repeatOpenTargetProof.duplicateIntentTabCount,
      ocrProof,
    };
    writeArtifact('summary.json', summary);
    console.log(JSON.stringify(summary, null, 2));
  } finally {
    await fixture?.close();
    cleanupLiveSession();
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
