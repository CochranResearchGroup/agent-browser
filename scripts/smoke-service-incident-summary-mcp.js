#!/usr/bin/env node

import {
  assert,
  assertIncidentSummarySmokeShape,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  parseMcpToolPayload,
  runCli,
  seedIncidentSummarySmokeEvents,
} from './smoke-utils.js';
import {
  assertServiceIncidentsResponseSchemaRecord,
  loadServiceRecordSchema,
} from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-incident-summary-mcp-',
  sessionPrefix: 'incident-summary-mcp',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'IncidentSummaryMcpSmoke';
const agentName = 'smoke-agent';
const taskName = 'groupMcpIncidentRemedies';
const incidentsResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-incidents-response.v1.schema.json',
);

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service incident MCP summary smoke to complete');
}, 45000);

function send(method, params) {
  return mcp.send(method, params);
}

function notify(method, params) {
  mcp.notify(method, params);
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  if (mcp) {
    mcp.rejectPending(message);
  }
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
}

async function seedIncidentEvents() {
  const result = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(result.stdout, 'service status');
  assert(status.success === true, `service status failed before seed: ${result.stdout}${result.stderr}`);
  seedIncidentSummarySmokeEvents(context, { serviceName, agentName, taskName });
}

try {
  await seedIncidentEvents();
  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });

  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-incident-summary-mcp-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const incidentsResult = await send('tools/call', {
    name: 'service_incidents',
    arguments: {
      summary: true,
      serviceName,
      agentName,
      taskName,
      limit: 20,
    },
  });
  const incidents = parseMcpToolPayload(incidentsResult, 'MCP service_incidents');
  assert(incidents.success === true, `MCP service_incidents failed: ${JSON.stringify(incidents)}`);
  assertServiceIncidentsResponseSchemaRecord(
    incidents.data,
    incidentsResponseSchema,
    'MCP incidents summary response',
  );
  assert(incidents.data.count === 3, `MCP incidents summary response count mismatch: ${JSON.stringify(incidents)}`);
  assertIncidentSummarySmokeShape(incidents.data.summary, 'MCP incidents');

  await cleanup();
  console.log('Service incident MCP summary smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
