#!/usr/bin/env node

import {
  assert,
  assertIncidentSummarySmokeShape,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  seedIncidentSummarySmokeEvents,
} from './smoke-utils.js';
import {
  assertServiceIncidentsResponseSchemaRecord,
  loadServiceRecordSchema,
} from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-incident-summary-http-',
  sessionPrefix: 'incident-summary-http',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { session } = context;
const serviceName = 'IncidentSummaryHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'groupHttpIncidentRemedies';
const incidentsResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-incidents-response.v1.schema.json',
);

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function enableStream() {
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
  return port;
}

function seedIncidentEvents() {
  const statusResult = runCli(context, ['--json', '--session', session, 'service', 'status']);
  return statusResult.then((result) => {
    const status = parseJsonOutput(result.stdout, 'service status');
    assert(status.success === true, `service status failed before seed: ${result.stdout}${result.stderr}`);
    seedIncidentSummarySmokeEvents(context, { serviceName, agentName, taskName });
  });
}

try {
  await seedIncidentEvents();
  const port = await enableStream();
  const incidents = await httpJson(
    port,
    'GET',
    `/api/service/incidents?summary=true&limit=20&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(taskName)}`,
  );

  assert(incidents.success === true, `HTTP service incidents failed: ${JSON.stringify(incidents)}`);
  assertServiceIncidentsResponseSchemaRecord(
    incidents.data,
    incidentsResponseSchema,
    'HTTP incidents summary response',
  );
  assert(incidents.data.count === 3, `HTTP incidents summary response count mismatch: ${JSON.stringify(incidents)}`);
  assertIncidentSummarySmokeShape(incidents.data.summary, 'HTTP incidents');

  await cleanup();
  console.log('Service incident HTTP summary smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
