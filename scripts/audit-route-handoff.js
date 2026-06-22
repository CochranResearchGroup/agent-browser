#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const isMain = process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
const args = isMain ? process.argv.slice(2) : [];
if (args[0] === '--') args.shift();

function usage() {
  return [
    'Usage: node scripts/audit-route-handoff.js [--json] [--fixture <path>] [--agent-browser <cmd>] [--skip-doctor]',
    '',
    'Read-only audit that joins service browser rows, tabs, route pool entries,',
    'display allocations, remote-view routes, viewer leases, runtime identity,',
    'view streams, and retained route-visible proof.',
  ].join('\n');
}

function takeFlag(name) {
  const index = args.indexOf(name);
  if (index === -1) return null;
  const value = args[index + 1];
  if (!value || value.startsWith('--')) {
    throw new Error(`${name} requires a value`);
  }
  args.splice(index, 2);
  return value;
}

function hasFlag(name) {
  const index = args.indexOf(name);
  if (index === -1) return false;
  args.splice(index, 1);
  return true;
}

const jsonOutput = hasFlag('--json');
const skipDoctor = hasFlag('--skip-doctor');
const fixturePath = takeFlag('--fixture');
const agentBrowserCommand = takeFlag('--agent-browser') || process.env.AGENT_BROWSER_COMMAND || 'agent-browser';
const help = hasFlag('--help') || hasFlag('-h');

if (help) {
  console.log(usage());
  process.exit(0);
}

if (args.length > 0) {
  console.error(`Unknown argument(s): ${args.join(' ')}`);
  console.error(usage());
  process.exit(2);
}

function runJson(command, commandArgs, label) {
  const result = spawnSync(command, commandArgs, {
    encoding: 'utf8',
    stdio: 'pipe',
    timeout: 120000,
  });
  if (result.status !== 0) {
    throw new Error(`${label} failed: ${command} ${commandArgs.join(' ')}\n${result.stdout}${result.stderr}`);
  }
  try {
    return JSON.parse(result.stdout.trim());
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON: ${err.message}\n${result.stdout}`);
  }
}

function loadInputs() {
  if (fixturePath) {
    const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
    return {
      source: { kind: 'fixture', path: fixturePath },
      serviceStatus: fixture.serviceStatus,
      remoteViewDoctor: fixture.remoteViewDoctor ?? null,
      collectionErrors: [],
    };
  }
  const serviceStatus = runJson(agentBrowserCommand, ['service', 'status', '--json'], 'agent-browser service status');
  let remoteViewDoctor = null;
  const collectionErrors = [];
  if (!skipDoctor) {
    try {
      remoteViewDoctor = runJson(agentBrowserCommand, ['doctor', 'remote-view', '--json'], 'agent-browser doctor remote-view');
    } catch (err) {
      collectionErrors.push({
        source: 'agent-browser doctor remote-view --json',
        error: err.message,
      });
    }
  }
  return {
    source: { kind: 'live', command: agentBrowserCommand },
    serviceStatus,
    remoteViewDoctor,
    collectionErrors,
  };
}

function recordMap(value) {
  if (!value) return {};
  if (Array.isArray(value)) {
    return Object.fromEntries(
      value
        .filter((item) => item && typeof item === 'object')
        .map((item, index) => [String(item.id ?? item.browserId ?? item.tabId ?? index), item]),
    );
  }
  if (typeof value === 'object') return value;
  return {};
}

function values(value) {
  return Object.values(recordMap(value));
}

function stateFromServiceStatus(serviceStatus) {
  const data = serviceStatus?.data ?? {};
  return data.service_state ?? data;
}

function firstString(...valuesToCheck) {
  for (const value of valuesToCheck) {
    if (typeof value === 'string' && value.trim()) return value.trim();
  }
  return null;
}

function readinessState(readiness) {
  if (!readiness) return null;
  if (typeof readiness === 'string') return readiness.trim() || null;
  if (Array.isArray(readiness)) {
    const failed = readiness.map(readinessState).find((state) => state && state !== 'ready');
    return failed ?? null;
  }
  if (typeof readiness !== 'object') return null;
  return firstString(
    readiness.state,
    readiness.status,
    readiness.readiness,
    readiness.lastProviderEvent,
    readiness.reason,
  );
}

function displayContentState(proof) {
  return firstString(
    proof?.displayContent?.state,
    proof?.readiness?.displayContent?.state,
    proof?.state,
  );
}

function displayContentWindows(proof) {
  const windows = proof?.displayContent?.windows ?? proof?.readiness?.displayContent?.windows;
  return Array.isArray(windows) ? windows : [];
}

function windowSummary(proof) {
  return displayContentWindows(proof)
    .map((window) => firstString(window.className, window.title, window.id))
    .filter(Boolean)
    .slice(0, 5);
}

function hasTerminalOnlyProof(proof) {
  const state = displayContentState(proof);
  if (state && ['terminal_only', 'desktop_only', 'no_browser_window_visible'].includes(state)) return true;
  const windows = displayContentWindows(proof);
  if (windows.length === 0) return false;
  const terminalWindows = windows.filter((window) => {
    const label = `${window.className ?? ''} ${window.title ?? ''}`.toLowerCase();
    return label.includes('xterm') || label.includes('terminal') || label.includes('shell');
  });
  const browserWindows = windows.filter((window) => {
    const label = `${window.className ?? ''} ${window.title ?? ''}`.toLowerCase();
    return label.includes('chrome') || label.includes('chromium') || label.includes('browser');
  });
  return terminalWindows.length > 0 && browserWindows.length === 0;
}

function routeForBrowser({ browser, routes, streams }) {
  const direct = routes.find((route) => route?.browserId === browser.id && route?.state === 'ready');
  if (direct) return direct;
  const byDisplay = routes.find((route) =>
    route?.state === 'ready' &&
    firstString(route.displayAllocationId) &&
    route.displayAllocationId === browser.displayAllocationId,
  );
  if (byDisplay) return byDisplay;
  const streamRouteIds = new Set(streams.map((stream) => stream?.routeId).filter(Boolean));
  return routes.find((route) => streamRouteIds.has(route?.id)) ?? null;
}

function routePoolEntryForRoute(route, routePool) {
  if (!route) return null;
  return routePool.find((entry) =>
    entry?.id === route.routePoolEntryId ||
    entry?.routeId === route.id ||
    entry?.currentRouteAllocationId === route.id ||
    entry?.connectionId === route.connectionId,
  ) ?? null;
}

function streamForBrowser(browser, route, state) {
  const browserStreams = Array.isArray(browser.viewStreams) ? browser.viewStreams : [];
  const stateStreams = values(state.viewStreams);
  return (
    browserStreams.find((stream) => route?.id && stream?.routeId === route.id) ??
    browserStreams.find((stream) => browser.displayAllocationId && stream?.displayAllocationId === browser.displayAllocationId) ??
    stateStreams.find((stream) => route?.id && stream?.routeId === route.id) ??
    stateStreams.find((stream) => browser.displayAllocationId && stream?.displayAllocationId === browser.displayAllocationId) ??
    browserStreams[0] ??
    null
  );
}

function viewerLeasesFor({ browser, route, displayAllocationId, viewerLeases }) {
  return viewerLeases.filter((lease) =>
    lease?.browserId === browser.id ||
    (route?.id && lease?.routeId === route.id) ||
    (displayAllocationId && lease?.displayAllocationId === displayAllocationId),
  );
}

function tabsForBrowser(browser, tabsById) {
  const fromHandles = Array.isArray(browser.tabHandles) ? browser.tabHandles : [];
  const fromIds = Array.isArray(browser.tabIds)
    ? browser.tabIds.map((id) => tabsById[id]).filter(Boolean)
    : [];
  const merged = [...fromHandles, ...fromIds];
  const seen = new Set();
  return merged.filter((tab) => {
    const id = firstString(tab.tabId, tab.id, tab.targetId, tab?.serviceTabHandle?.tabId);
    if (!id || seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

function tabId(tab) {
  return firstString(tab.tabId, tab.id, tab.targetId, tab?.serviceTabHandle?.tabId);
}

function tabTitle(tab) {
  return firstString(tab.title, tab?.serviceTabHandle?.title);
}

function tabUrl(tab) {
  return firstString(tab.url, tab?.serviceTabHandle?.url);
}

function isRetainedOrStale(browser, route, allocation) {
  const health = firstString(browser.health);
  if (health && !['ready', 'healthy'].includes(health)) return true;
  if (route && route.state && !['ready', 'checked_out'].includes(route.state)) return true;
  if (allocation && allocation.state && !['ready', 'checked_out'].includes(allocation.state)) return true;
  return false;
}

function browserOwnership(browser) {
  return firstString(browser.ownership, browser.profileOrigin, browser.ownerKind) ?? 'agent_browser_owned';
}

function classify({ browser, route, allocation, proof }) {
  if (browser.detected === true || browserOwnership(browser) === 'foreign_cdp') return 'foreign_cdp';
  if (isRetainedOrStale(browser, route, allocation)) return 'stale_or_retained';
  if (route?.state === 'ready') {
    if (hasTerminalOnlyProof(proof)) return 'route_bound_terminal_only';
    if (readinessState(proof) === 'ready' && displayContentState(proof) === 'browser_window_visible') {
      return 'route_bound_ready';
    }
    return 'route_bound_proof_missing';
  }
  if (browser.host === 'remote_headed') return 'direct_remote_headed';
  return 'route_bound_proof_missing';
}

function buildAudit(inputs) {
  const state = stateFromServiceStatus(inputs.serviceStatus);
  const browsers = values(state.browsers);
  const tabsById = recordMap(state.tabs);
  const routes = values(state.remoteViewRoutes);
  const routePool = values(state.routePool);
  const displayAllocations = recordMap(state.displayAllocations);
  const viewerLeases = values(state.viewerLeases);
  const runtimeInventory = inputs.remoteViewDoctor?.data?.runtimeInventory ?? null;
  const runtimeConvergence = inputs.remoteViewDoctor?.data?.runtimeConvergence ?? null;
  const remoteControl = inputs.remoteViewDoctor?.data?.remoteControl ?? null;

  const rows = [];
  for (const browser of browsers) {
    const route = routeForBrowser({
      browser,
      routes,
      streams: Array.isArray(browser.viewStreams) ? browser.viewStreams : [],
    });
    const stream = streamForBrowser(browser, route, state);
    const displayAllocationId = firstString(browser.displayAllocationId, route?.displayAllocationId, stream?.displayAllocationId);
    const allocation = displayAllocationId ? displayAllocations[displayAllocationId] : null;
    const routePoolEntry = routePoolEntryForRoute(route, routePool);
    const proof = route?.readiness ?? stream?.remoteReadiness ?? stream?.readiness ?? allocation?.readiness ?? null;
    const leases = viewerLeasesFor({ browser, route, displayAllocationId, viewerLeases });
    const tabs = tabsForBrowser(browser, tabsById);
    const classification = classify({ browser, route, allocation, proof });
    const proofState = readinessState(proof);
    const visualState = displayContentState(proof);
    const streamUrl = firstString(
      stream?.dashboardEmbedUrl,
      stream?.frameUrl,
      stream?.url,
      stream?.externalUrl,
      stream?.routeDescriptor?.dashboardEmbedUrl,
      stream?.routeDescriptor?.publicOperatorUrl,
      route?.frameUrl,
      route?.externalUrl,
    );

    const base = {
      classification,
      browserId: browser.id,
      profileId: firstString(browser.profileId, browser.runtimeProfile),
      ownership: browserOwnership(browser),
      host: firstString(browser.host),
      health: firstString(browser.health),
      pid: browser.pid ?? null,
      displayName: firstString(browser.displayName, allocation?.displayName, routePoolEntry?.target?.displayName),
      displayAllocationId,
      routeId: route?.id ?? stream?.routeId ?? null,
      routeState: firstString(route?.state),
      routePoolEntryId: routePoolEntry?.id ?? route?.routePoolEntryId ?? null,
      streamProvider: firstString(stream?.provider, route?.provider),
      streamControlInput: firstString(stream?.controlInput, route?.controlInput),
      streamUrl,
      proofState,
      visualState,
      proofWindowSummary: windowSummary(proof),
      viewerLeaseIds: leases.map((lease) => lease.id).filter(Boolean),
      explanation: explanationFor({ classification, route, stream, proof, browser }),
    };

    if (tabs.length === 0) {
      rows.push({ ...base, tabId: null, title: null, url: null });
    } else {
      for (const tab of tabs) {
        rows.push({
          ...base,
          tabId: tabId(tab),
          title: tabTitle(tab),
          url: tabUrl(tab),
        });
      }
    }
  }

  const summary = rows.reduce((acc, row) => {
    acc[row.classification] = (acc[row.classification] ?? 0) + 1;
    return acc;
  }, {});

  return {
    schemaVersion: 'agent-browser.route-handoff-audit.v1',
    generatedAt: new Date().toISOString(),
    source: inputs.source,
    collectionErrors: inputs.collectionErrors,
    summary,
    runtime: {
      convergenceStatus: runtimeConvergence?.status ?? null,
      inventoryStatus: runtimeInventory?.status ?? null,
      runtimeCount: runtimeInventory?.runtimeCount ?? null,
      staleRuntimeCount: runtimeInventory?.staleCount ?? null,
      remoteControlStatus: remoteControl?.status ?? null,
      remoteControlReady: remoteControl?.ready ?? null,
    },
    collections: {
      browsers: browsers.length,
      tabs: Object.keys(tabsById).length,
      displayAllocations: Object.keys(displayAllocations).length,
      remoteViewRoutes: routes.length,
      routePool: routePool.length,
      viewerLeases: viewerLeases.length,
    },
    rows,
  };
}

function explanationFor({ classification, route, stream, proof, browser }) {
  if (classification === 'route_bound_ready') {
    return 'route is ready and retained proof reports a visible browser window';
  }
  if (classification === 'route_bound_terminal_only') {
    return 'route is ready but retained visual proof indicates terminal-only or no browser window content';
  }
  if (classification === 'route_bound_proof_missing') {
    if (route?.state === 'ready') {
      return 'route is ready but no row-bound browser-window proof is retained';
    }
    if (stream?.provider === 'rdp_gateway') {
      return 'browser exposes a Guacamole stream URL without a ready route-bound visual proof';
    }
    return 'browser row has no current route-bound visual proof';
  }
  if (classification === 'direct_remote_headed') {
    return 'remote-headed browser has no ready remote-view route binding for this row';
  }
  if (classification === 'foreign_cdp') {
    return 'browser is detected as non-owned CDP and must not be treated as lifecycle-owned';
  }
  if (classification === 'stale_or_retained') {
    return 'browser, route, or display allocation is not in a live ready state';
  }
  return `classification ${classification} for browser ${browser.id ?? 'unknown'} with proof ${readinessState(proof) ?? 'none'}`;
}

function printText(audit) {
  console.log(`Route handoff audit (${audit.schemaVersion})`);
  console.log(`Source: ${audit.source.kind}${audit.source.path ? ` ${audit.source.path}` : ''}`);
  console.log(
    `Runtime: convergence=${audit.runtime.convergenceStatus ?? 'unknown'} inventory=${audit.runtime.inventoryStatus ?? 'unknown'} remoteControl=${audit.runtime.remoteControlStatus ?? 'unknown'}`,
  );
  if (audit.collectionErrors.length > 0) {
    console.log(`Collection warnings: ${audit.collectionErrors.length}`);
  }
  console.log(`Collections: browsers=${audit.collections.browsers} tabs=${audit.collections.tabs} routes=${audit.collections.remoteViewRoutes} routePool=${audit.collections.routePool} displays=${audit.collections.displayAllocations} leases=${audit.collections.viewerLeases}`);
  console.log('');
  console.log([
    'classification'.padEnd(27),
    'browser'.padEnd(34),
    'profile'.padEnd(24),
    'display'.padEnd(9),
    'route'.padEnd(18),
    'pool'.padEnd(18),
    'proof'.padEnd(20),
    'tab/title',
  ].join('  '));
  for (const row of audit.rows) {
    const proof = `${row.proofState ?? 'none'}/${row.visualState ?? 'none'}`;
    const tab = `${row.tabId ?? 'no-tab'} ${row.title ?? ''}`.trim();
    console.log([
      row.classification.padEnd(27),
      String(row.browserId ?? '').slice(0, 34).padEnd(34),
      String(row.profileId ?? '').slice(0, 24).padEnd(24),
      String(row.displayName ?? '').slice(0, 9).padEnd(9),
      String(row.routeId ?? '').slice(0, 18).padEnd(18),
      String(row.routePoolEntryId ?? '').slice(0, 18).padEnd(18),
      proof.slice(0, 20).padEnd(20),
      tab,
    ].join('  '));
  }
}

if (isMain) {
  try {
    const audit = buildAudit(loadInputs());
    if (jsonOutput) {
      console.log(JSON.stringify({ success: true, data: audit }, null, 2));
    } else {
      printText(audit);
    }
  } catch (err) {
    if (jsonOutput) {
      console.log(JSON.stringify({ success: false, error: err.message }, null, 2));
    } else {
      console.error(err.message);
    }
    process.exit(1);
  }
}

export {
  buildAudit,
  classify,
};
