#!/usr/bin/env node

import {
  assert,
  assertIncidentSummaryFilteredResponse,
  assertIncidentSummarySmokeShape,
  closeSession,
  createSmokeContext,
  httpJson,
  httpJsonResult,
  incidentSummarySmokeFilterCases,
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
  const refused = await httpJsonResult(port, 'POST', '/api/service/request', {
    action: 'synthetic_unsupported_action',
    serviceName, agentName, taskName,
  });
  assert(refused.statusCode === 400, `Expected pre-dispatch HTTP refusal: ${JSON.stringify(refused)}`);
  const rejection = refused.body;
  assert(rejection.success === false && !rejection.job, 'Refusal must not create a job');
  assert(rejection.failure?.effectState === 'no_effect', JSON.stringify(rejection));
  assert(rejection.failure?.code === 'unsupported_action', JSON.stringify(rejection));
  assert(rejection.failure?.executableNextAction, 'Refusal lost executable recourse');
  const occurrenceId = rejection.failure?.occurrenceId;
  assert(typeof occurrenceId === 'string', 'Refusal lost occurrence correlation');
  const logged = await httpJson(port, 'GET', `/api/service/failures?occurrenceId=${encodeURIComponent(occurrenceId)}`);
  assert(logged.data.records.length === 1 && logged.data.records[0].occurrenceId === occurrenceId,
    `HTTP failure lookup did not preserve the producer occurrence: ${JSON.stringify(logged)}`);
  const afterJobs = await httpJson(port, 'GET', '/api/service/jobs');
  assert(!afterJobs.data.jobs.some(job => job.id === rejection.id || job.action === 'synthetic_unsupported_action'),
    'Rejected request created a job for its original request ID');
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

  for (const filterCase of incidentSummarySmokeFilterCases({ serviceName, agentName, taskName, session })) {
    const filtered = await httpJson(port, 'GET', `/api/service/incidents?${filterCase.httpQuery}`);
    assert(filtered.success === true, `HTTP ${filterCase.label} failed: ${JSON.stringify(filtered)}`);
    assertServiceIncidentsResponseSchemaRecord(
      filtered.data,
      incidentsResponseSchema,
      `HTTP incidents ${filterCase.label} response`,
    );
    assertIncidentSummaryFilteredResponse(
      filtered.data,
      filterCase.expected,
      `HTTP incidents ${filterCase.label}`,
    );
  }

  await cleanup();
  console.log('Service incident HTTP summary smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
