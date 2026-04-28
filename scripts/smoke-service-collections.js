#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  readResourceContents,
  runCli,
  sendRawCommand,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sc-', sessionPrefix: 'sc' });
const { session } = context;
const runtimeProfile = `collections-${process.pid}`;
const serviceName = 'ServiceCollectionsSmoke';
const agentName = 'smoke-agent';
const taskName = 'collectionParitySmoke';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service collections smoke to complete');
}, 120000);

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

async function serviceCliCollection(command, key) {
  const result = await runCli(context, ['--json', 'service', command]);
  const parsed = parseJsonOutput(result.stdout, `service ${command}`);
  assert(parsed.success === true, `service ${command} failed: ${result.stdout}${result.stderr}`);
  assert(Array.isArray(parsed.data?.[key]), `service ${command} missing ${key} array`);
  assert(Number.isInteger(parsed.data?.count), `service ${command} missing numeric count`);
  return parsed.data;
}

async function serviceMcpCollection(uri, label) {
  const result = await runCli(context, ['--json', 'mcp', 'read', uri]);
  return readResourceContents(parseJsonOutput(result.stdout, `${label} resource`), label);
}

function assertContains(collection, key, predicate, label) {
  assert(
    collection[key]?.some(predicate),
    `${label} missing expected item in ${key}: ${JSON.stringify(collection)}`,
  );
}

try {
  const pageUrl = smokeDataUrl('Service Collections Smoke', 'Service Collections Smoke');

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--runtime-profile',
    runtimeProfile,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const launchResult = await sendRawCommand(context, {
    id: 'service-collections-smoke-launch',
    action: 'launch',
    headless: true,
    runtimeProfile,
    args: ['--no-sandbox'],
    serviceName,
    agentName,
    taskName,
  });
  assert(launchResult.success === true, `Metadata launch failed: ${JSON.stringify(launchResult)}`);

  const streamStatusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);

  const reconcileResult = await runCli(context, ['--json', '--session', session, 'service', 'reconcile']);
  const reconciled = parseJsonOutput(reconcileResult.stdout, 'service reconcile');
  assert(reconciled.success === true, `service reconcile failed: ${reconcileResult.stdout}${reconcileResult.stderr}`);
  const state = reconciled.data?.service_state;
  assert(state && typeof state === 'object', 'service reconcile response missing service_state');

  const expectedProfileId = runtimeProfile;
  const expectedSessionId = session;
  const expectedBrowserId = `session:${session}`;
  const expectedTab = Object.values(state.tabs || {}).find(
    (tab) =>
      tab.browserId === expectedBrowserId &&
      tab.lifecycle === 'ready' &&
      tab.title === 'Service Collections Smoke',
  );
  assert(expectedTab, `service reconcile did not retain the smoke tab: ${JSON.stringify(state.tabs)}`);

  const checks = [
    {
      command: 'profiles',
      endpoint: '/api/service/profiles',
      key: 'profiles',
      mcpUri: 'agent-browser://profiles',
      predicate: (item) =>
        item.id === expectedProfileId &&
        item.sharedServiceIds?.includes(serviceName) &&
        item.keyring === 'basic_password_store',
    },
    {
      command: 'sessions',
      endpoint: '/api/service/sessions',
      key: 'sessions',
      mcpUri: 'agent-browser://sessions',
      predicate: (item) =>
        item.id === expectedSessionId &&
        item.profileId === expectedProfileId &&
        item.serviceName === serviceName,
    },
    {
      command: 'browsers',
      endpoint: '/api/service/browsers',
      key: 'browsers',
      mcpUri: 'agent-browser://browsers',
      predicate: (item) => item.id === expectedBrowserId && item.health === 'ready',
    },
    {
      command: 'tabs',
      endpoint: '/api/service/tabs',
      key: 'tabs',
      mcpUri: 'agent-browser://tabs',
      predicate: (item) =>
        item.id === expectedTab.id &&
        item.browserId === expectedBrowserId &&
        item.lifecycle === 'ready' &&
        item.title === expectedTab.title,
    },
  ];

  for (const check of checks) {
    const cliCollection = await serviceCliCollection(check.command, check.key);
    assertContains(cliCollection, check.key, check.predicate, `CLI service ${check.command}`);

    const httpCollection = await httpJson(port, 'GET', check.endpoint);
    assert(httpCollection.success === true, `HTTP ${check.endpoint} failed: ${JSON.stringify(httpCollection)}`);
    assertContains(httpCollection.data, check.key, check.predicate, `HTTP ${check.endpoint}`);

    const mcpCollection = await serviceMcpCollection(check.mcpUri, check.key);
    assertContains(mcpCollection, check.key, check.predicate, `MCP ${check.mcpUri}`);
  }

  await cleanup();
  console.log('Service collections parity smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
