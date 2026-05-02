#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-contracts-no-launch-',
  sessionPrefix: 'contracts-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session } = context;

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
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

function assertNoBrowserLaunchState() {
  const statePath = join(agentHome, 'service', 'state.json');
  if (!existsSync(statePath)) {
    return;
  }

  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => !['launch', 'navigate', 'tab_new'].includes(job.action)),
    `service contracts recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `service contracts persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  const port = await enableStream();
  const contracts = await httpJson(port, 'GET', '/api/service/contracts');

  assert(contracts.success === true, `HTTP service contracts failed: ${JSON.stringify(contracts)}`);
  assert(contracts.data?.schemaVersion === 'v1', `contracts schemaVersion mismatch: ${JSON.stringify(contracts)}`);
  assert(
    contracts.data?.contracts?.serviceRequest?.schemaId ===
      'https://agent-browser.local/contracts/service-request.v1.schema.json',
    `serviceRequest schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data?.contracts?.serviceRequest?.http?.route === '/api/service/request',
    `serviceRequest HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data?.contracts?.serviceRequest?.mcp?.tool === 'service_request',
    `serviceRequest MCP tool mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    Array.isArray(contracts.data?.contracts?.serviceRequest?.actions),
    `serviceRequest actions missing: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data.contracts.serviceRequest.actionCount ===
      contracts.data.contracts.serviceRequest.actions.length,
    `serviceRequest action count mismatch: ${JSON.stringify(contracts.data.contracts.serviceRequest)}`,
  );

  assertNoBrowserLaunchState();

  await cleanup();
  console.log('Service contracts no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
