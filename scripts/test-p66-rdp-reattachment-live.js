#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
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
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const mode = process.argv.find((arg) => arg.startsWith('--mode='))?.slice('--mode='.length) ||
  process.env.AGENT_BROWSER_P66_LIVE_MODE ||
  'two-route-switching';
const keepBrowsers = process.env.AGENT_BROWSER_P66_KEEP_BROWSERS === '1';
const serviceName = 'P66RdpReattachmentLive';
const agentName = 'p66-live-smoke';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-p66-rdp-reattachment-${mode}-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

function smokeDataUrl(title, heading) {
  return `data:text/html,${encodeURIComponent([
    '<!doctype html>',
    '<meta charset="utf-8">',
    `<title>${title}</title>`,
    '<body style="margin:0;font-family:Arial,sans-serif;background:white;color:#111;">',
    '<main style="min-height:100vh;display:grid;place-items:center;text-align:center;">',
    `<h1 style="font-size:42px;">${heading}</h1>`,
    '</main>',
    '</body>',
  ].join(''))}`;
}

function runNode(args, label, env) {
  const result = spawnSync(process.execPath, args, {
    cwd: new URL('..', import.meta.url).pathname,
    env,
    encoding: 'utf8',
    stdio: 'pipe',
    timeout: 120000,
  });
  assert(result.status === 0, `${label} failed: ${result.stdout}${result.stderr}`);
  return parseJsonOutput(result.stdout, label);
}

function routePoolReadiness(env) {
  if (process.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON?.trim()) {
    return {
      success: true,
      routePoolJson: JSON.parse(process.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON),
      source: 'AGENT_BROWSER_RDP_ROUTE_POOL_JSON',
    };
  }
  return runNode(['scripts/smoke-rdp-guac-route-pool-readiness.js', '--report-only'], 'route-pool readiness', env);
}

function selectTwoRouteEntries(report) {
  assert(report.success === true, `route-pool readiness is not green: ${JSON.stringify(report)}`);
  const entries = (report.routePoolJson || []).filter((entry) =>
    entry?.id &&
    entry?.routeId &&
    (entry?.frameUrl || entry?.externalUrl) &&
    entry?.target?.displayName,
  );
  assert(entries.length >= 2, `P66 live gates require at least two ready route-pool entries: ${JSON.stringify(report)}`);
  const [entryA, entryB] = entries;
  assert(entryA.id !== entryB.id, `route entries must be distinct: ${JSON.stringify(entries)}`);
  assert(entryA.routeId !== entryB.routeId, `route ids must be distinct: ${JSON.stringify(entries)}`);
  return [entryA, entryB];
}

async function runSessionJson(context, session, args, label, timeoutMs = 120000) {
  const result = await runCli(context, ['--json', '--session', session, ...args], timeoutMs);
  const parsed = parseJsonOutput(result.stdout, label);
  assert(parsed.success === true, `${label} failed: ${result.stdout}${result.stderr}`);
  return parsed;
}

async function ensureStreamPort(context, session) {
  let status = await runSessionJson(context, session, ['stream', 'status'], `${session} stream status`, 60000);
  if (!status.data?.enabled) {
    status = await runSessionJson(context, session, ['stream', 'enable'], `${session} stream enable`, 60000);
  }
  const port = status.data?.port;
  assert(Number.isInteger(port) && port > 0, `${session} did not return a stream port: ${JSON.stringify(status)}`);
  return port;
}

async function serviceStatus(port, label) {
  const status = await httpJson(port, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

async function serviceRequest(port, body, label) {
  const response = await httpJson(port, 'POST', '/api/service/request', body);
  writeArtifact(`${label}-response.json`, response);
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
  return response;
}

async function reconcile(port, label) {
  const response = await httpJson(port, 'POST', '/api/service/reconcile');
  writeArtifact(`${label}-reconcile-response.json`, response);
  assert(response.success === true, `${label} reconcile failed: ${JSON.stringify(response)}`);
  assert(response.data?.reconciled === true, `${label} did not reconcile: ${JSON.stringify(response)}`);
  return response;
}

async function openBrowser(context, session, runtimeProfile, routeEntry, title) {
  const display = routeEntry.target?.displayName;
  const response = await runSessionJson(
    context,
    session,
    [
      'remote-view',
      'open',
      smokeDataUrl(title, title),
      '--runtime-profile',
      runtimeProfile,
      '--display',
      display,
      '--display-isolation',
      routeEntry.target?.displayIsolation || 'shared_display',
      '--route-pool-entry-json',
      JSON.stringify(routeEntry),
      '--service-name',
      serviceName,
      '--agent-name',
      agentName,
      '--task-name',
      `p66Open${title.replace(/[^A-Za-z0-9]/g, '')}`,
    ],
    `${session} remote-view open`,
    180000,
  );
  assert(response.data?.status === 'opened', `${session} did not open remote view: ${JSON.stringify(response)}`);
  assert(
    response.data?.operatorVisible?.state === 'ready' ||
      response.data?.verification?.visibleWindowProof?.displayContent?.state === 'browser_window_visible',
    `${session} did not prove operator-visible browser window: ${JSON.stringify(response.data?.operatorVisible || response.data?.verification)}`,
  );
  return {
    browserId: `session:${session}`,
    routeId: routeEntry.routeId,
    routePoolEntryId: routeEntry.id,
    sessionName: session,
    runtimeProfile,
    openResponse: response,
  };
}

function stateFromStatus(status) {
  return status.data?.service_state || status.data || {};
}

function assertBrowserPresent(status, browserId, label) {
  const state = stateFromStatus(status);
  const browser = state.browsers?.[browserId];
  assert(browser, `${label} missing browser ${browserId}: ${JSON.stringify(state.browsers)}`);
  assert(browser.health === 'ready', `${label} browser is not ready: ${JSON.stringify(browser)}`);
  assert(browser.attachability?.state, `${label} browser has no attachability: ${JSON.stringify(browser)}`);
  return browser;
}

function assertRouteOwnership(status, routeId, browserId, routePoolEntryId, label) {
  const state = stateFromStatus(status);
  const route = state.remoteViewRoutes?.[routeId];
  const entry = state.routePool?.[routePoolEntryId];
  assert(route, `${label} missing route ${routeId}: ${JSON.stringify(state.remoteViewRoutes)}`);
  assert(entry, `${label} missing route-pool entry ${routePoolEntryId}: ${JSON.stringify(state.routePool)}`);
  assert(route.browserId === browserId, `${label} route owner mismatch: ${JSON.stringify(route)}`);
  assert(route.state === 'ready', `${label} route is not ready: ${JSON.stringify(route)}`);
  assert(entry.state === 'checked_out', `${label} route-pool entry is not checked out: ${JSON.stringify(entry)}`);
  assert(entry.currentRouteAllocationId === routeId, `${label} route-pool allocation mismatch: ${JSON.stringify(entry)}`);
  const browser = assertBrowserPresent(status, browserId, label);
  assert(
    browser.viewStreams?.some((stream) => stream.routeId === routeId && stream.attachability?.state === 'attached_ready'),
    `${label} browser does not have attached-ready stream for ${routeId}: ${JSON.stringify(browser.viewStreams)}`,
  );
}

function assertReattachable(status, browserId, label) {
  const browser = assertBrowserPresent(status, browserId, label);
  assert(
    !String(browser.attachability?.state || '').startsWith('not_reattachable'),
    `${label} browser is not reattachable: ${JSON.stringify(browser.attachability)}`,
  );
  return browser;
}

async function routeSwitch(port, browser, extraParams, label) {
  const response = await serviceRequest(
    port,
    {
      action: 'service_remote_view_route_switch',
      serviceName,
      agentName,
      taskName: label,
      params: {
        browserId: browser.browserId,
        sessionName: browser.sessionName,
        ...extraParams,
      },
      jobTimeoutMs: 60000,
    },
    label,
  );
  assert(response.data?.status === 'route_switched', `${label} did not switch route: ${JSON.stringify(response)}`);
  return response;
}

async function closeBrowser(port, browser, label) {
  return serviceRequest(
    port,
    {
      action: 'service_browser_close',
      serviceName,
      agentName,
      taskName: label,
      params: { browserId: browser.browserId },
      jobTimeoutMs: 60000,
    },
    label,
  );
}

async function runScenario() {
  const context = createSmokeContext({
    prefix: 'agent-browser-p66-rdp-live-',
    session: `p66-live-controller-${process.pid}`,
  });
  context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
  context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = 'rdp_gateway';
  context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER = 'manual_attached_desktop';
  delete context.env.AGENT_BROWSER_AUTO_CONNECT;

  let port = null;
  let browserA = null;
  let browserB = null;
  try {
    const readiness = routePoolReadiness(context.env);
    writeArtifact('route-pool-readiness.json', readiness);
    const [entryA, entryB] = selectTwoRouteEntries(readiness);
    writeArtifact('selected-route-entries.json', { entryA, entryB });

    browserA = await openBrowser(context, 'p66-live-a', 'p66-live-profile-a', entryA, 'P66 Browser A');
    browserB = await openBrowser(context, 'p66-live-b', 'p66-live-profile-b', entryB, 'P66 Browser B');
    port = await ensureStreamPort(context, browserA.sessionName);

    const afterOpen = await serviceStatus(port, 'after-open');
    assertRouteOwnership(afterOpen, entryA.routeId, browserA.browserId, entryA.id, 'after-open A');
    assertRouteOwnership(afterOpen, entryB.routeId, browserB.browserId, entryB.id, 'after-open B');

    const switchBToA = await routeSwitch(
      port,
      browserB,
      {
        routePoolEntryId: entryA.id,
        routeId: entryA.routeId,
        remoteViewRouteId: entryA.routeId,
      },
      'p66-switch-b-to-a-park-a',
    );
    assert(
      switchBToA.data?.routeSwitchParking?.browserId === browserA.browserId,
      `switch B to A did not park browser A: ${JSON.stringify(switchBToA)}`,
    );
    let status = await serviceStatus(port, 'after-switch-b-to-a');
    assertRouteOwnership(status, entryA.routeId, browserB.browserId, entryA.id, 'after-switch-b-to-a');
    assertReattachable(status, browserA.browserId, 'after-switch-b-to-a');

    if (mode === 'remote-view-reconcile-reattach-live') {
      const reconciled = await reconcile(port, 'after-switch-b-to-a');
      assert(reconciled.data?.remoteViewRepair, `reconcile response missing remoteViewRepair: ${JSON.stringify(reconciled)}`);
      status = await serviceStatus(port, 'after-reconcile');
      assertReattachable(status, browserA.browserId, 'after-reconcile');
    }

    const switchAToAvailable = await routeSwitch(port, browserA, {}, 'p66-switch-a-to-available');
    assert(
      switchAToAvailable.data?.routePoolEntryId === entryB.id ||
        switchAToAvailable.data?.newRoutePoolEntryId === entryB.id,
      `switch A back did not use the available second route: ${JSON.stringify(switchAToAvailable)}`,
    );
    status = await serviceStatus(port, 'after-switch-a-back');
    assertRouteOwnership(status, entryA.routeId, browserB.browserId, entryA.id, 'after-switch-a-back B');
    assertRouteOwnership(status, entryB.routeId, browserA.browserId, entryB.id, 'after-switch-a-back A');

    if (mode === 'dashboard-rdp-reattachable-rail-live') {
      const state = stateFromStatus(status);
      const ownedBrowsers = Object.values(state.browsers || {}).filter((browser) =>
        browser?.host === 'remote_headed' &&
        browser?.attachability &&
        !String(browser.attachability.state || '').startsWith('not_reattachable'),
      );
      assert(
        ownedBrowsers.some((browser) => browser.id === browserA.browserId) &&
          ownedBrowsers.some((browser) => browser.id === browserB.browserId),
        `dashboard rail source state does not retain both reattachable browsers: ${JSON.stringify(ownedBrowsers)}`,
      );
      writeArtifact('dashboard-rail-source-proof.json', { ownedBrowsers });
    }

    if (mode === 'rdp-browser-reattach-until-close-live') {
      await closeBrowser(port, browserA, 'p66-close-browser-a');
      status = await serviceStatus(port, 'after-close-a');
      const closedA = stateFromStatus(status).browsers?.[browserA.browserId];
      assert(
        !closedA || String(closedA.attachability?.state || closedA.health || '').includes('closed') ||
          ['closing', 'process_exited', 'not_started'].includes(String(closedA.health || '')),
        `Browser A remained reattachable after close: ${JSON.stringify(closedA)}`,
      );
      assertRouteOwnership(status, entryA.routeId, browserB.browserId, entryA.id, 'after-close-a B');
    }

    writeArtifact('p66-live-summary.json', {
      artifactDir,
      mode,
      browserA,
      browserB,
      routeA: entryA.id,
      routeB: entryB.id,
    });
    console.log(`P66 ${mode} passed; artifacts: ${artifactDir}`);
  } finally {
    if (!keepBrowsers && port) {
      if (browserA) await closeBrowser(port, browserA, 'p66-cleanup-close-a').catch(() => {});
      if (browserB) await closeBrowser(port, browserB, 'p66-cleanup-close-b').catch(() => {});
    }
    cleanupSmokeHome(context);
  }
}

const timeout = setTimeout(() => {
  console.error(`Timed out waiting for P66 ${mode} live gate`);
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, Number(process.env.AGENT_BROWSER_P66_LIVE_TIMEOUT_MS || 600000));

runScenario()
  .then(() => clearTimeout(timeout))
  .catch((err) => {
    clearTimeout(timeout);
    console.error(err.stack || err.message);
    console.error(`Artifacts: ${artifactDir}`);
    process.exit(1);
  });
