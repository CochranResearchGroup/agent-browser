#!/usr/bin/env node

import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sr-', sessionPrefix: 'sr' });
const { session } = context;
const profileDir = join(context.tempHome, 'chrome-profile');

mkdirSync(profileDir, { recursive: true });

const timeout = setTimeout(() => {
  fail('Timed out waiting for service reconcile MCP smoke to complete');
}, 90000);

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

try {
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>Service Reconcile MCP Smoke</title></head>',
    '<body><h1 id="ready">Service Reconcile MCP Smoke</h1></body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

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

  await cleanup();
  console.log('Service reconcile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
