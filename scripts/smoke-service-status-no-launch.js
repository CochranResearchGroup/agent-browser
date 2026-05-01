#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-status-no-launch-',
  sessionPrefix: 'status-no-launch',
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

try {
  const statusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'status',
  ]);
  const status = parseJsonOutput(statusResult.stdout, 'service status');

  assert(status.success === true, `service status failed: ${statusResult.stdout}`);
  assert(
    status.data?.control_plane?.browser_health === 'NotStarted',
    `service status launched browser: ${JSON.stringify(status.data?.control_plane)}`,
  );

  const statePath = join(agentHome, 'service', 'state.json');
  assert(existsSync(statePath), `service state was not written: ${statePath}`);
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  assert(
    Object.keys(state.jobs ?? {}).length === 0,
    `service status persisted jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `service status persisted browsers: ${JSON.stringify(state.browsers)}`,
  );

  await cleanup();
  console.log('Service status no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
