#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-mcp-read-no-launch-',
  sessionPrefix: 'mcp-read-no-launch',
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
  const sessionsResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    'agent-browser://sessions',
  ]);
  const sessions = readResourceContents(
    parseJsonOutput(sessionsResult.stdout, 'mcp sessions resource'),
    'sessions',
  );

  assert(
    Array.isArray(sessions.sessions),
    `invalid sessions resource: ${sessionsResult.stdout}`,
  );
  assert(sessions.count === 0, `mcp read returned unexpected sessions: ${sessionsResult.stdout}`);

  const statePath = join(agentHome, 'service', 'state.json');
  if (existsSync(statePath)) {
    const state = JSON.parse(readFileSync(statePath, 'utf8'));
    assert(
      Object.keys(state.jobs ?? {}).length === 0,
      `mcp read persisted jobs: ${JSON.stringify(state.jobs)}`,
    );
    assert(
      Object.keys(state.browsers ?? {}).length === 0,
      `mcp read persisted browsers: ${JSON.stringify(state.browsers)}`,
    );
  }

  await cleanup();
  console.log('MCP read no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
