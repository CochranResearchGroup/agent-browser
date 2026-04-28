#!/usr/bin/env node

import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

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

function seedConfigCollections(context, expectedTabId) {
  const statePath = join(context.agentHome, 'service', 'state.json');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  state.sitePolicies = {
    ...(state.sitePolicies || {}),
    google: {
      id: 'google',
      originPattern: 'https://accounts.google.com',
      browserHost: 'local_headed',
      interactionMode: 'human_like_input',
      manualLoginPreferred: true,
      profileRequired: true,
      challengePolicy: 'avoid_first',
      allowedChallengeProviders: ['manual'],
    },
  };
  state.providers = {
    ...(state.providers || {}),
    manual: {
      id: 'manual',
      kind: 'manual_approval',
      displayName: 'Dashboard approval',
      enabled: true,
      configRef: 'service.providers.manual',
      capabilities: ['human_approval'],
    },
  };
  state.challenges = {
    ...(state.challenges || {}),
    'challenge-1': {
      id: 'challenge-1',
      tabId: expectedTabId,
      kind: 'captcha',
      state: 'waiting_for_human',
      providerId: 'manual',
      policyDecision: 'manual_approval_required',
      humanApproved: false,
      result: 'pending',
    },
  };
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
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
  seedConfigCollections(context, expectedTab.id);

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
    {
      command: 'site-policies',
      endpoint: '/api/service/site-policies',
      key: 'sitePolicies',
      mcpUri: 'agent-browser://site-policies',
      predicate: (item) =>
        item.id === 'google' &&
        item.originPattern === 'https://accounts.google.com' &&
        item.interactionMode === 'human_like_input' &&
        item.challengePolicy === 'avoid_first',
    },
    {
      command: 'providers',
      endpoint: '/api/service/providers',
      key: 'providers',
      mcpUri: 'agent-browser://providers',
      predicate: (item) =>
        item.id === 'manual' &&
        item.kind === 'manual_approval' &&
        item.displayName === 'Dashboard approval' &&
        item.capabilities?.includes('human_approval'),
    },
    {
      command: 'challenges',
      endpoint: '/api/service/challenges',
      key: 'challenges',
      mcpUri: 'agent-browser://challenges',
      predicate: (item) =>
        item.id === 'challenge-1' &&
        item.tabId === expectedTab.id &&
        item.kind === 'captcha' &&
        item.providerId === 'manual',
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
