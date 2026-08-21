#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { createServiceStatusMcpToolCall } from '../packages/client/src/service-observability.js';

const context = createSmokeContext({
  prefix: 'ab-status-no-launch-',
  sessionPrefix: 'status-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_RUNTIME_HOST = '1';

const { agentHome, session } = context;
let mcp;

async function cleanup() {
  try {
    mcp?.close();
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

try {
  const streamResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'stream',
    'status',
  ]);
  const stream = parseJsonOutput(streamResult.stdout, 'stream status');
  assert(stream.success === true, `stream status failed: ${streamResult.stdout}`);

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

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: (message) => {
      throw new Error(message);
    },
  });
  await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'service-status-no-launch', version: '0' },
  });
  mcp.notify('notifications/initialized');
  const mcpResult = await mcp.send('tools/call', createServiceStatusMcpToolCall());
  const mcpStatus = JSON.parse(mcpResult.content?.[0]?.text || '{}');
  assert(mcpStatus.success === true, `MCP service_status failed: ${JSON.stringify(mcpStatus)}`);
  assert(
    mcpStatus.data?.runtimeLifecycle?.schemaVersion ===
      'agent-browser.runtime-lifecycle-status.v1',
    `MCP service_status omitted runtimeLifecycle: ${JSON.stringify(mcpStatus.data)}`,
  );
  assert(
    mcpStatus.data?.control_plane?.browser_health === 'NotStarted',
    `MCP service_status launched browser: ${JSON.stringify(mcpStatus.data?.control_plane)}`,
  );

  const statePath = join(agentHome, 'service', 'state.json');
  assert(existsSync(statePath), `service state was not written: ${statePath}`);
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => ['stream_status', 'service_status'].includes(job.action)),
    `service status persisted an unexpected job: ${JSON.stringify(state.jobs)}`,
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
