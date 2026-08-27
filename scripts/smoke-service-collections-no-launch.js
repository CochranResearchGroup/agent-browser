#!/usr/bin/env node

import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceBrowsers,
  getServiceChallenges,
  getServiceMonitors,
  getServiceProfileLeases,
  getServiceProfiles,
  getServiceProviders,
  getServiceSessions,
  getServiceSitePolicies,
  getServiceTabs,
} from '../packages/client/src/service-observability.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-collections-no-launch-',
  sessionPrefix: 'service-collections-no-launch',
});
context.env.AGENT_BROWSER_RUNTIME_HOST = '1';
const { agentHome, session, tempHome } = context;
const stateDir = join(agentHome, 'service');
const statePath = join(stateDir, 'state.json');
const launchMarker = join(tempHome, 'browser-launch-attempted');
const launchSentinel = join(tempHome, 'browser-launch-sentinel');
const CLI_TIMEOUT_MS = 180000;

const collections = [
  ['profiles', 'profiles', getServiceProfiles],
  ['leases', 'profileLeases', getServiceProfileLeases, 'profile-leases', 'service_profile_leases'],
  ['browsers', 'browsers', getServiceBrowsers],
  ['sessions', 'sessions', getServiceSessions],
  ['tabs', 'tabs', getServiceTabs],
  ['monitors', 'monitors', getServiceMonitors],
  ['site-policies', 'sitePolicies', getServiceSitePolicies],
  ['providers', 'providers', getServiceProviders],
  ['challenges', 'challenges', getServiceChallenges],
];

async function enableStream() {
  let result = await runCli(
    context,
    ['--json', '--session', session, 'stream', 'status'],
    CLI_TIMEOUT_MS,
  );
  let response = parseJsonOutput(result.stdout, 'stream status');
  if (!response.data?.enabled) {
    result = await runCli(
      context,
      ['--json', '--session', session, 'stream', 'enable'],
      CLI_TIMEOUT_MS,
    );
    response = parseJsonOutput(result.stdout, 'stream enable');
  }
  assert(response.success === true, `stream readiness failed: ${result.stdout}${result.stderr}`);
  assert(Number.isInteger(response.data?.port), `stream enable returned no port: ${result.stdout}`);
  return response.data.port;
}

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

try {
  mkdirSync(stateDir, { recursive: true });
  const initialState = `${JSON.stringify({}, null, 2)}\n`;
  writeFileSync(statePath, initialState);
  writeFileSync(launchSentinel, `#!/bin/sh\nprintf touched > ${JSON.stringify(launchMarker)}\nexit 97\n`);
  chmodSync(launchSentinel, 0o700);
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = launchSentinel;

  const port = await enableStream();
  const baseUrl = `http://127.0.0.1:${port}`;

  const rejectedEffect = await fetch(`${baseUrl}/api/service/request`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      action: 'navigate',
      params: { url: 'https://example.com' },
    }),
  });
  const rejectedEffectBody = await rejectedEffect.json();
  assert(
    rejectedEffect.status === 400 && rejectedEffectBody.success === false,
    `unattributed HTTP effect request was not rejected: ${JSON.stringify(rejectedEffectBody)}`,
  );
  assert(
    String(rejectedEffectBody.error).includes('requires serviceName, agentName, and taskName'),
    `unattributed HTTP rejection did not explain its attribution requirement: ${JSON.stringify(rejectedEffectBody)}`,
  );
  assert(!existsSync(launchMarker), 'an unattributed HTTP effect request reached the browser executable');

  for (const [command, key, clientRead, transportName = command] of collections) {
    const cliResult = await runCli(
      context,
      ['--json', '--session', session, 'service', command],
      CLI_TIMEOUT_MS,
    );
    const cli = parseJsonOutput(cliResult.stdout, `CLI service ${command}`);
    assert(cli.success === true, `CLI service ${command} failed: ${cliResult.stdout}${cliResult.stderr}`);
    assert(Array.isArray(cli.data?.[key]), `CLI ${command} response was not a collection`);

    const http = await httpJson(port, 'GET', `/api/service/${transportName}`);
    assert(http.success === true, `HTTP service ${command} failed: ${JSON.stringify(http)}`);

    const mcpResult = await runCli(
      context,
      [
        '--json',
        '--session',
        session,
        'mcp',
        'read',
        `agent-browser://${transportName}`,
      ],
      CLI_TIMEOUT_MS,
    );
    const mcp = readResourceContents(
      parseJsonOutput(mcpResult.stdout, `MCP service ${command}`),
      key,
    );
    assert(mcp && typeof mcp === 'object', `MCP service ${command} returned no object`);

    const client = await clientRead({ baseUrl });
    assert(client && typeof client === 'object', `client service ${command} returned no object`);
  }

  assert(!existsSync(launchMarker), 'a label-optional collection read invoked the browser executable');
  const finalState = JSON.parse(readFileSync(statePath, 'utf8'));
  const allowedLifecycleActions = new Set([
    'stream_status',
    ...collections.map(([command, , , , action]) => action ?? `service_${command.replaceAll('-', '_')}`),
  ]);
  const jobs = Object.values(finalState.jobs ?? {});
  assert(
    jobs.every((job) => job.lifecycleOnly === true || job.action === 'stream_status'),
    `collection reads recorded an effect-capable job: ${JSON.stringify(finalState.jobs)}`,
  );
  assert(
    jobs.every((job) => allowedLifecycleActions.has(job.action)),
    `collection reads recorded an unexpected action: ${JSON.stringify(finalState.jobs)}`,
  );
  assert(
    Object.keys(finalState.browsers ?? {}).length === 0 &&
      Object.keys(finalState.sessions ?? {}).length === 0 &&
      Object.keys(finalState.tabs ?? {}).length === 0,
    `collection reads created browser ownership: ${JSON.stringify({
      browsers: finalState.browsers,
      sessions: finalState.sessions,
      tabs: finalState.tabs,
    })}`,
  );

  await cleanup();
  console.log('Service collection no-launch parity smoke passed');
} catch (error) {
  await cleanup();
  console.error(error.stack || error.message);
  process.exit(1);
}
