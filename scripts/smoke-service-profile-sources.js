#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceAccessPlan,
  getServiceProfiles,
  lookupServiceProfile,
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
  prefix: 'ab-profile-sources-',
  sessionPrefix: 'profile-sources',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session, tempHome } = context;
const configPath = join(tempHome, 'agent-browser-config.json');
context.env.AGENT_BROWSER_CONFIG = configPath;

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
      console.error(`Keeping smoke home: ${tempHome}`);
    } else {
      context.cleanupTempHome();
    }
  }
}

function seedStateAndConfig() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          'persisted-only': {
            id: 'persisted-only',
            name: 'Persisted Only',
            targetServiceIds: ['acs'],
            authenticatedServiceIds: ['acs'],
            sharedServiceIds: ['JournalDownloader'],
            allocation: 'per_service',
            keyring: 'basic_password_store',
          },
          google: {
            id: 'google',
            name: 'Persisted Google',
            targetServiceIds: ['google'],
            authenticatedServiceIds: [],
            sharedServiceIds: ['JournalDownloader'],
            allocation: 'per_service',
            keyring: 'basic_password_store',
          },
        },
      },
      null,
      2,
    )}\n`,
  );

  writeFileSync(
    configPath,
    `${JSON.stringify(
      {
        service: {
          profiles: {
            google: {
              id: 'google',
              name: 'Configured Google',
              targetServiceIds: ['google'],
              authenticatedServiceIds: ['google'],
              sharedServiceIds: ['JournalDownloader'],
              allocation: 'per_service',
              keyring: 'basic_password_store',
            },
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function enableStream() {
  const streamStatusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'stream',
    'status',
  ]);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );

  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, [
      '--json',
      '--session',
      session,
      'stream',
      'enable',
    ]);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }

  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function sourceById(collection, id) {
  return collection.profileSources?.find((source) => source.id === id);
}

function profileById(collection, id) {
  return collection.profiles?.find((profile) => profile.id === id);
}

function assertProfileSources(collection, label) {
  assert(
    profileById(collection, 'google')?.name === 'Configured Google',
    `${label} did not apply config override for google: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'google')?.source === 'config',
    `${label} did not report config source for google: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'google')?.overrideable === false,
    `${label} marked config profile overrideable: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'persisted-only')?.source === 'persisted_state',
    `${label} did not report persisted source: ${JSON.stringify(collection)}`,
  );
  assert(
    JSON.stringify(sourceById(collection, 'google')?.precedence) ===
      JSON.stringify(['config', 'runtime_observed', 'persisted_state']),
    `${label} source precedence mismatch: ${JSON.stringify(collection)}`,
  );
}

function assertSelectedProfileSource(response, label) {
  assert(
    response.selectedProfile?.id === 'google',
    `${label} did not select google profile: ${JSON.stringify(response)}`,
  );
  assert(
    response.selectedProfileSource?.id === 'google',
    `${label} selected profile source id mismatch: ${JSON.stringify(response)}`,
  );
  assert(
    response.selectedProfileSource?.source === 'config',
    `${label} selected profile source mismatch: ${JSON.stringify(response)}`,
  );
}

function assertNoBrowserLaunchState() {
  const statePath = join(agentHome, 'service', 'state.json');
  if (!existsSync(statePath)) {
    return;
  }

  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => !['launch', 'navigate', 'tab_new'].includes(job.action)),
    `profile source smoke recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `profile source smoke persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  seedStateAndConfig();
  const port = await enableStream();

  const httpCollection = await httpJson(port, 'GET', '/api/service/profiles');
  assert(httpCollection.success === true, `HTTP profiles failed: ${JSON.stringify(httpCollection)}`);
  assertProfileSources(httpCollection.data, 'HTTP profiles');

  const httpLookup = await httpJson(
    port,
    'GET',
    '/api/service/profiles/lookup?serviceName=JournalDownloader&targetServiceId=google',
  );
  assert(httpLookup.success === true, `HTTP profile lookup failed: ${JSON.stringify(httpLookup)}`);
  assertSelectedProfileSource(httpLookup.data, 'HTTP profile lookup');

  const httpAccessPlan = await httpJson(
    port,
    'GET',
    '/api/service/access-plan?serviceName=JournalDownloader&targetServiceId=google',
  );
  assert(httpAccessPlan.success === true, `HTTP access-plan failed: ${JSON.stringify(httpAccessPlan)}`);
  assertSelectedProfileSource(httpAccessPlan.data, 'HTTP access-plan');

  const mcpResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    'agent-browser://profiles',
  ]);
  const mcpCollection = readResourceContents(
    parseJsonOutput(mcpResult.stdout, 'mcp profiles resource'),
    'profiles',
  );
  assertProfileSources(mcpCollection, 'MCP profiles');

  const clientCollection = await getServiceProfiles({
    baseUrl: `http://127.0.0.1:${port}`,
  });
  assertProfileSources(clientCollection, 'client profiles');

  const clientLookup = await lookupServiceProfile({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName: 'JournalDownloader',
    targetServiceId: 'google',
  });
  assertSelectedProfileSource(clientLookup, 'client profile lookup');

  const clientAccessPlan = await getServiceAccessPlan({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName: 'JournalDownloader',
    targetServiceId: 'google',
  });
  assertSelectedProfileSource(clientAccessPlan, 'client access-plan');

  assertNoBrowserLaunchState();

  await cleanup();
  console.log('Service profile sources no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
