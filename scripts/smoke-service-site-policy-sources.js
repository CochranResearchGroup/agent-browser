#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { getServiceSitePolicies } from '../packages/client/src/service-observability.js';

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
  prefix: 'ab-site-policy-sources-',
  sessionPrefix: 'site-policy-sources',
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
        sitePolicies: {
          'persisted-only': {
            id: 'persisted-only',
            originPattern: 'https://persisted.example',
            browserHost: 'local_headed',
            interactionMode: 'human_like_input',
            challengePolicy: 'avoid_first',
          },
          google: {
            id: 'google',
            originPattern: 'https://persisted-google.example',
            browserHost: 'remote_headed',
            interactionMode: 'browser_input',
            challengePolicy: 'provider_allowed',
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
          sitePolicies: {
            google: {
              id: 'google',
              originPattern: 'https://configured-google.example',
              browserHost: 'docker_headed',
              interactionMode: 'human_like_input',
              challengePolicy: 'manual_only',
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
  return collection.sitePolicySources?.find((source) => source.id === id);
}

function policyById(collection, id) {
  return collection.sitePolicies?.find((policy) => policy.id === id);
}

function assertSources(collection, label) {
  assert(
    policyById(collection, 'google')?.originPattern === 'https://configured-google.example',
    `${label} did not apply config override for google: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'google')?.source === 'config',
    `${label} did not report config source for google: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'google')?.overrideable === false,
    `${label} marked config policy overrideable: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'persisted-only')?.source === 'persisted_state',
    `${label} did not report persisted source: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'microsoft')?.source === 'builtin',
    `${label} did not report builtin source for microsoft: ${JSON.stringify(collection)}`,
  );
  assert(
    sourceById(collection, 'microsoft')?.overrideable === true,
    `${label} did not mark builtin policy overrideable: ${JSON.stringify(collection)}`,
  );
  assert(
    JSON.stringify(sourceById(collection, 'microsoft')?.precedence) ===
      JSON.stringify(['config', 'persisted_state', 'builtin']),
    `${label} source precedence mismatch: ${JSON.stringify(collection)}`,
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
    `site-policy source smoke recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `site-policy source smoke persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  seedStateAndConfig();
  const port = await enableStream();

  const httpCollection = await httpJson(port, 'GET', '/api/service/site-policies');
  assert(httpCollection.success === true, `HTTP site-policies failed: ${JSON.stringify(httpCollection)}`);
  assertSources(httpCollection.data, 'HTTP site-policies');

  const mcpResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    'agent-browser://site-policies',
  ]);
  const mcpCollection = readResourceContents(
    parseJsonOutput(mcpResult.stdout, 'mcp site-policies resource'),
    'site-policies',
  );
  assertSources(mcpCollection, 'MCP site-policies');

  const clientCollection = await getServiceSitePolicies({
    baseUrl: `http://127.0.0.1:${port}`,
  });
  assertSources(clientCollection, 'client site-policies');

  assertNoBrowserLaunchState();

  await cleanup();
  console.log('Service site-policy sources no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
