#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  readResourceContents,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sr-', sessionPrefix: 'sr' });
const { session } = context;
const profileDir = join(context.tempHome, 'chrome-profile');
const handoffSession = `${session}-handoff`;
const legacySession = `${session}-legacy`;
const staleTabId = 'target:service-reconcile-stale-target';

mkdirSync(profileDir, { recursive: true });

const timeout = setTimeout(() => {
  fail('Timed out waiting for service reconcile MCP smoke to complete');
}, 120000);

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

function statePath() {
  return join(context.agentHome, 'service', 'state.json');
}

function seedOwnershipHandoff({ browserId, liveTabId }) {
  const path = statePath();
  const state = JSON.parse(readFileSync(path, 'utf8'));
  assert(state.browsers?.[browserId], `Cannot seed handoff; missing browser ${browserId}`);
  state.browsers[browserId].activeSessionIds = [handoffSession];
  state.sessions = {
    ...(state.sessions || {}),
    [legacySession]: {
      id: legacySession,
      serviceName: 'LegacyService',
      agentName: 'legacy-agent',
      taskName: 'staleOwner',
      lease: 'shared',
      cleanup: 'detach',
      browserIds: [browserId],
      tabIds: [liveTabId, staleTabId],
    },
  };
  state.tabs = {
    ...(state.tabs || {}),
    [liveTabId]: {
      ...(state.tabs?.[liveTabId] || {}),
      ownerSessionId: legacySession,
    },
    [staleTabId]: {
      id: staleTabId,
      browserId,
      targetId: 'service-reconcile-stale-target',
      lifecycle: 'ready',
      ownerSessionId: legacySession,
      url: 'https://stale.example.invalid/',
      title: 'Stale Service Reconcile Target',
    },
  };
  writeFileSync(path, `${JSON.stringify(state, null, 2)}\n`);
}

async function serviceCliCollection(command, key) {
  const result = await runCli(context, ['--json', 'service', command]);
  const parsed = parseJsonOutput(result.stdout, `service ${command}`);
  assert(parsed.success === true, `service ${command} failed: ${result.stdout}${result.stderr}`);
  assert(Array.isArray(parsed.data?.[key]), `service ${command} missing ${key} array`);
  return parsed.data;
}

async function serviceMcpCollection(uri, label) {
  const result = await runCli(context, ['--json', 'mcp', 'read', uri]);
  return readResourceContents(parseJsonOutput(result.stdout, `${label} resource`), label);
}

function assertHandoffState(state, { browserId, liveTabId, label }) {
  const tabs = Object.values(state.tabs || {});
  const sessions = Object.values(state.sessions || {});
  const liveTab = tabs.find((tab) => tab.id === liveTabId);
  const staleTab = tabs.find((tab) => tab.id === staleTabId);
  const newOwner = sessions.find((item) => item.id === handoffSession);
  const oldOwner = sessions.find((item) => item.id === legacySession);

  assert(liveTab, `${label} missing live tab ${liveTabId}: ${JSON.stringify(tabs)}`);
  assert(staleTab, `${label} missing stale tab ${staleTabId}: ${JSON.stringify(tabs)}`);
  assert(newOwner, `${label} missing handoff session ${handoffSession}: ${JSON.stringify(sessions)}`);
  assert(oldOwner, `${label} missing legacy session ${legacySession}: ${JSON.stringify(sessions)}`);
  assert(liveTab.browserId === browserId, `${label} live tab browser mismatch: ${JSON.stringify(liveTab)}`);
  assert(liveTab.lifecycle === 'ready', `${label} live tab was not ready: ${JSON.stringify(liveTab)}`);
  assert(
    liveTab.ownerSessionId === handoffSession,
    `${label} live tab owner was not reassigned: ${JSON.stringify(liveTab)}`,
  );
  assert(
    staleTab.lifecycle === 'closed',
    `${label} stale tab was not closed: ${JSON.stringify(staleTab)}`,
  );
  assert(
    newOwner.tabIds?.includes(liveTabId),
    `${label} handoff session did not receive live tab: ${JSON.stringify(newOwner)}`,
  );
  assert(
    !oldOwner.tabIds?.includes(liveTabId) && !oldOwner.tabIds?.includes(staleTabId),
    `${label} legacy session retained browser tabs: ${JSON.stringify(oldOwner)}`,
  );
}

try {
  const pageUrl = smokeDataUrl('Service Reconcile MCP Smoke', 'Service Reconcile MCP Smoke');

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--profile',
    profileDir,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const reconcileResult = await runCli(context, ['--json', '--session', session, 'service', 'reconcile']);
  const reconciled = parseJsonOutput(reconcileResult.stdout, 'service reconcile');
  assert(reconciled.success === true, `service reconcile failed: ${reconcileResult.stdout}`);
  assert(reconciled.data?.reconciled === true, 'service reconcile did not report reconciled=true');

  const state = reconciled.data?.service_state;
  assert(state && typeof state === 'object', 'service reconcile response missing service_state');
  const stateBrowsers = Object.values(state.browsers || {});
  const stateTabs = Object.values(state.tabs || {});
  const liveBrowser = stateBrowsers.find(
    (browser) => browser.health === 'ready' && typeof browser.cdpEndpoint === 'string',
  );
  assert(
    liveBrowser,
    `service reconcile did not retain a ready browser: ${JSON.stringify(stateBrowsers)}`,
  );
  const liveTab = stateTabs.find(
    (tab) =>
      tab.browserId === liveBrowser.id &&
      tab.lifecycle === 'ready' &&
      tab.title === 'Service Reconcile MCP Smoke' &&
      typeof tab.targetId === 'string',
  );
  assert(
    liveTab,
    `service reconcile did not retain the live smoke tab: ${JSON.stringify(stateTabs)}`,
  );
  assert(
    state.reconciliation?.browserCount >= 1,
    'service reconcile did not update reconciliation browserCount',
  );
  assert(
    state.reconciliation?.lastReconciledAt,
    'service reconcile did not update lastReconciledAt',
  );

  const browsersResourceResult = await runCli(context, [
    '--json',
    'mcp',
    'read',
    'agent-browser://browsers',
  ]);
  const browsersResource = readResourceContents(
    parseJsonOutput(browsersResourceResult.stdout, 'mcp browsers resource'),
    'browsers',
  );
  const resourceBrowser = browsersResource.browsers?.find(
    (browser) =>
      browser.id === liveBrowser.id &&
      browser.health === liveBrowser.health &&
      browser.cdpEndpoint === liveBrowser.cdpEndpoint,
  );
  assert(
    resourceBrowser,
    `MCP browsers resource did not match service reconcile browser ${liveBrowser.id}: ${JSON.stringify(
      browsersResource,
    )}`,
  );

  const tabsResourceResult = await runCli(context, ['--json', 'mcp', 'read', 'agent-browser://tabs']);
  const tabsResource = readResourceContents(
    parseJsonOutput(tabsResourceResult.stdout, 'mcp tabs resource'),
    'tabs',
  );
  const resourceTab = tabsResource.tabs?.find(
    (tab) =>
      tab.id === liveTab.id &&
      tab.browserId === liveBrowser.id &&
      tab.lifecycle === liveTab.lifecycle &&
      tab.title === liveTab.title &&
      tab.targetId === liveTab.targetId,
  );
  assert(
    resourceTab,
    `MCP tabs resource did not match service reconcile tab ${liveTab.id}: ${JSON.stringify(
      tabsResource,
    )}`,
  );

  const cliTabsResult = await runCli(context, ['--json', 'service', 'tabs']);
  const cliTabs = parseJsonOutput(cliTabsResult.stdout, 'service tabs');
  assert(
    cliTabs.success === true,
    `Service tabs failed: ${cliTabsResult.stdout}${cliTabsResult.stderr}`,
  );
  const cliTab = cliTabs.data?.tabs?.find(
    (tab) =>
      tab.id === liveTab.id &&
      tab.browserId === liveBrowser.id &&
      tab.lifecycle === liveTab.lifecycle &&
      tab.title === liveTab.title &&
      tab.targetId === liveTab.targetId,
  );
  assert(
    cliTab,
    `Service tabs did not match service reconcile tab ${liveTab.id}: ${JSON.stringify(cliTabs.data)}`,
  );

  seedOwnershipHandoff({ browserId: liveBrowser.id, liveTabId: liveTab.id });

  const handoffReconcileResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'reconcile',
  ]);
  const handoffReconciled = parseJsonOutput(handoffReconcileResult.stdout, 'handoff service reconcile');
  assert(
    handoffReconciled.success === true,
    `handoff service reconcile failed: ${handoffReconcileResult.stdout}${handoffReconcileResult.stderr}`,
  );
  assertHandoffState(handoffReconciled.data?.service_state || {}, {
    browserId: liveBrowser.id,
    liveTabId: liveTab.id,
    label: 'service reconcile handoff response',
  });

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);
  assertHandoffState(status.data?.service_state || {}, {
    browserId: liveBrowser.id,
    liveTabId: liveTab.id,
    label: 'service status',
  });

  const sessionsData = await serviceCliCollection('sessions', 'sessions');
  assertHandoffState(
    {
      tabs: handoffReconciled.data?.service_state?.tabs,
      sessions: Object.fromEntries(sessionsData.sessions.map((item) => [item.id, item])),
    },
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      label: 'service sessions',
    },
  );

  const tabsData = await serviceCliCollection('tabs', 'tabs');
  assertHandoffState(
    {
      tabs: Object.fromEntries(tabsData.tabs.map((item) => [item.id, item])),
      sessions: handoffReconciled.data?.service_state?.sessions,
    },
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      label: 'service tabs',
    },
  );

  const sessionsResource = await serviceMcpCollection('agent-browser://sessions', 'sessions');
  assertHandoffState(
    {
      tabs: handoffReconciled.data?.service_state?.tabs,
      sessions: Object.fromEntries(sessionsResource.sessions.map((item) => [item.id, item])),
    },
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      label: 'MCP sessions resource',
    },
  );

  const handoffTabsResource = await serviceMcpCollection('agent-browser://tabs', 'tabs');
  assertHandoffState(
    {
      tabs: Object.fromEntries(handoffTabsResource.tabs.map((item) => [item.id, item])),
      sessions: handoffReconciled.data?.service_state?.sessions,
    },
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      label: 'MCP tabs resource',
    },
  );

  await cleanup();
  console.log('Service reconcile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
