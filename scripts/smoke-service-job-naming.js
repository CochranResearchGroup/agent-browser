#!/usr/bin/env node

import { readFileSync } from 'node:fs';

import {
  assert,
  assertServiceTracePayload,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sjn-', sessionPrefix: 'sjn' });
const { session } = context;
const serviceName = 'JobNamingSmoke';
const agentName = 'smoke-agent';
const taskName = 'jobNamingSmoke';
const unnamedJobId = `job-naming-unnamed-${process.pid}`;
const namedJobId = `job-naming-named-${process.pid}`;
const jobRecordSchema = JSON.parse(
  readFileSync(new URL('../docs/dev/contracts/service-job-record.v1.schema.json', import.meta.url), 'utf8'),
);

let mcp;

const timeout = setTimeout(() => {
  fail('Timed out waiting for service job naming smoke to complete');
}, 90000);

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
}

async function serviceCommand(port, body) {
  const response = await httpJson(port, 'POST', '/api/command', body);
  assert(response.success === true, `HTTP command ${body.id} failed: ${JSON.stringify(response)}`);
  return response;
}

async function serviceJob(port, jobId) {
  const response = await httpJson(port, 'GET', `/api/service/jobs/${encodeURIComponent(jobId)}`);
  assert(response.success === true, `HTTP service job ${jobId} failed: ${JSON.stringify(response)}`);
  assert(response.data?.job?.id === jobId, `HTTP service job ${jobId} returned wrong job`);
  assertServiceJobSchemaRecord(response.data.job, `HTTP service job ${jobId}`);
  return response.data.job;
}

function parseMcpJsonResource(result, uri, label) {
  const content = result.contents?.[0];
  assert(
    content?.mimeType === 'application/json',
    `${label} content MIME mismatch: ${JSON.stringify(content)}`,
  );
  assert(content?.uri === uri, `${label} content URI mismatch: ${JSON.stringify(content)}`);
  assert(typeof content.text === 'string', `${label} content missing JSON text`);
  return JSON.parse(content.text);
}

function schemaEnum(property) {
  return jobRecordSchema.properties[property].enum;
}

function assertServiceJobSchemaRecord(job, label) {
  assert(job && typeof job === 'object', `${label} is not an object: ${JSON.stringify(job)}`);
  for (const field of jobRecordSchema.required) {
    assert(Object.hasOwn(job, field), `${label} missing schema field ${field}: ${JSON.stringify(job)}`);
  }
  for (const field of [
    'service_name',
    'agent_name',
    'task_name',
    'naming_warnings',
    'has_naming_warning',
    'submitted_at',
    'started_at',
    'completed_at',
    'timeout_ms',
  ]) {
    assert(!Object.hasOwn(job, field), `${label} leaked snake_case field ${field}`);
  }
  assert(schemaEnum('state').includes(job.state), `${label} state is outside schema enum`);
  assert(schemaEnum('priority').includes(job.priority), `${label} priority is outside schema enum`);
  assert(Array.isArray(job.namingWarnings), `${label} missing namingWarnings array`);
  for (const warning of job.namingWarnings) {
    assert(
      jobRecordSchema.properties.namingWarnings.items.enum.includes(warning),
      `${label} warning is outside schema enum: ${warning}`,
    );
  }
  assert(typeof job.hasNamingWarning === 'boolean', `${label} missing hasNamingWarning boolean`);
}

function assertNamingWarnings(job, expected, label) {
  assertServiceJobSchemaRecord(job, label);
  assert(Array.isArray(job.namingWarnings), `${label} missing namingWarnings array: ${JSON.stringify(job)}`);
  assert(job.hasNamingWarning === expected.length > 0, `${label} hasNamingWarning mismatch: ${JSON.stringify(job)}`);
  assert(
    JSON.stringify(job.namingWarnings) === JSON.stringify(expected),
    `${label} warnings were ${JSON.stringify(job.namingWarnings)}, expected ${JSON.stringify(expected)}`,
  );
}

try {
  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--args',
    '--no-sandbox',
    'open',
    smokeDataUrl('Service Job Naming Smoke', 'Service Job Naming Smoke'),
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

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
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);

  await serviceCommand(port, {
    id: unnamedJobId,
    action: 'state_list',
  });
  await serviceCommand(port, {
    id: namedJobId,
    action: 'state_list',
    serviceName,
    agentName,
    taskName,
  });

  const unnamedJob = await serviceJob(port, unnamedJobId);
  const namedJob = await serviceJob(port, namedJobId);
  const jobs = await httpJson(port, 'GET', '/api/service/jobs?limit=50');
  assert(jobs.success === true, `HTTP service jobs failed: ${JSON.stringify(jobs)}`);
  const collectionUnnamedJob = jobs.data?.jobs?.find((job) => job.id === unnamedJobId);
  const collectionNamedJob = jobs.data?.jobs?.find((job) => job.id === namedJobId);
  assert(collectionUnnamedJob, `HTTP jobs collection missing unnamed job ${unnamedJobId}`);
  assert(collectionNamedJob, `HTTP jobs collection missing named job ${namedJobId}`);
  assertServiceJobSchemaRecord(collectionUnnamedJob, 'HTTP collection unnamed job');
  assertServiceJobSchemaRecord(collectionNamedJob, 'HTTP collection named job');
  assertNamingWarnings(
    unnamedJob,
    ['missing_service_name', 'missing_agent_name', 'missing_task_name'],
    'Unnamed HTTP service job',
  );
  assertNamingWarnings(namedJob, [], 'Named HTTP service job');

  const trace = await httpJson(port, 'GET', '/api/service/trace?limit=50');
  assertServiceTracePayload(trace, 'HTTP service trace');
  const traceUnnamedJob = trace.data.jobs.find((job) => job.id === unnamedJobId);
  const traceNamedJob = trace.data.jobs.find((job) => job.id === namedJobId);
  assert(traceUnnamedJob, `Trace missing unnamed job ${unnamedJobId}: ${JSON.stringify(trace.data.jobs)}`);
  assert(traceNamedJob, `Trace missing named job ${namedJobId}: ${JSON.stringify(trace.data.jobs)}`);
  assertNamingWarnings(traceUnnamedJob, unnamedJob.namingWarnings, 'Trace unnamed job');
  assertNamingWarnings(traceNamedJob, [], 'Trace named job');
  assert(
    trace.data.summary.namingWarningCount >= 1,
    `Trace summary did not report naming warnings: ${JSON.stringify(trace.data.summary)}`,
  );

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-job-naming-smoke', version: '0' },
  });
  assert(initialize.capabilities?.resources, 'MCP resources capability missing');
  mcp.notify('notifications/initialized');
  const mcpJobsResource = await mcp.send('resources/read', { uri: 'agent-browser://jobs' });
  const mcpJobs = parseMcpJsonResource(mcpJobsResource, 'agent-browser://jobs', 'MCP jobs resource');
  const mcpUnnamedJob = mcpJobs.jobs?.find((job) => job.id === unnamedJobId);
  const mcpNamedJob = mcpJobs.jobs?.find((job) => job.id === namedJobId);
  assert(mcpUnnamedJob, `MCP jobs resource missing unnamed job ${unnamedJobId}`);
  assert(mcpNamedJob, `MCP jobs resource missing named job ${namedJobId}`);
  assertNamingWarnings(mcpUnnamedJob, unnamedJob.namingWarnings, 'MCP unnamed job');
  assertNamingWarnings(mcpNamedJob, [], 'MCP named job');

  console.log('Service job naming warning smoke passed');
  await cleanup();
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
