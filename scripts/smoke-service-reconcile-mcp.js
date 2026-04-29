#!/usr/bin/env node

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  assertServiceOwnershipHandoff,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  readResourceContents,
  runCli,
  seedServiceOwnershipHandoff,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sr-', sessionPrefix: 'sr' });
const { session } = context;
const profileDir = join(context.tempHome, 'chrome-profile');
const handoffSession = `${session}-handoff`;
const legacySession = `${session}-legacy`;
const staleTabId = 'target:service-reconcile-stale-target';
const handoffScenario = {
  handoffSession,
  legacySession,
  staleTabId,
  staleTargetId: 'service-reconcile-stale-target',
  staleTitle: 'Stale Service Reconcile Target',
};

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

  seedServiceOwnershipHandoff(context, {
    browserId: liveBrowser.id,
    liveTabId: liveTab.id,
    ...handoffScenario,
  });

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
  assertServiceOwnershipHandoff({
    sessions: Object.values(handoffReconciled.data?.service_state?.sessions || {}),
    tabs: Object.values(handoffReconciled.data?.service_state?.tabs || {}),
  }, 'service reconcile handoff response', {
    browserId: liveBrowser.id,
    liveTabId: liveTab.id,
    ...handoffScenario,
  });

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);
  assertServiceOwnershipHandoff({
    sessions: Object.values(status.data?.service_state?.sessions || {}),
    tabs: Object.values(status.data?.service_state?.tabs || {}),
  }, 'service status', {
    browserId: liveBrowser.id,
    liveTabId: liveTab.id,
    ...handoffScenario,
  });

  const sessionsData = await serviceCliCollection('sessions', 'sessions');
  assertServiceOwnershipHandoff(
    {
      sessions: sessionsData.sessions,
      tabs: Object.values(handoffReconciled.data?.service_state?.tabs || {}),
    },
    'service sessions',
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      ...handoffScenario,
    },
  );

  const tabsData = await serviceCliCollection('tabs', 'tabs');
  assertServiceOwnershipHandoff(
    {
      sessions: Object.values(handoffReconciled.data?.service_state?.sessions || {}),
      tabs: tabsData.tabs,
    },
    'service tabs',
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      ...handoffScenario,
    },
  );

  const sessionsResource = await serviceMcpCollection('agent-browser://sessions', 'sessions');
  assertServiceOwnershipHandoff(
    {
      sessions: sessionsResource.sessions,
      tabs: Object.values(handoffReconciled.data?.service_state?.tabs || {}),
    },
    'MCP sessions resource',
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      ...handoffScenario,
    },
  );

  const handoffTabsResource = await serviceMcpCollection('agent-browser://tabs', 'tabs');
  assertServiceOwnershipHandoff(
    {
      sessions: Object.values(handoffReconciled.data?.service_state?.sessions || {}),
      tabs: handoffTabsResource.tabs,
    },
    'MCP tabs resource',
    {
      browserId: liveBrowser.id,
      liveTabId: liveTab.id,
      ...handoffScenario,
    },
  );

  await cleanup();
  console.log('Service reconcile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
