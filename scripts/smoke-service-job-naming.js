#!/usr/bin/env node

import { setTimeout as delay } from 'node:timers/promises';

import {
  cancelServiceJob,
} from '../packages/client/src/service-observability.js';
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
import {
  assertServiceJobCancelResponseSchemaRecord,
  assertServiceJobsResponseSchemaRecord,
  assertServiceJobSchemaRecord,
  loadServiceRecordSchema,
  parseMcpJsonResource,
} from './smoke-schema-utils.js';

const context = createSmokeContext({ prefix: 'ab-sjn-', sessionPrefix: 'sjn' });
const { session } = context;
const serviceName = 'JobNamingSmoke';
const agentName = 'smoke-agent';
const taskName = 'jobNamingSmoke';
const unnamedJobId = `job-naming-unnamed-${process.pid}`;
const namedJobId = `job-naming-named-${process.pid}`;
const runningJobId = `job-naming-running-${process.pid}`;
const cancelJobId = `job-naming-cancel-${process.pid}`;
const jobRecordSchema = loadServiceRecordSchema('../docs/dev/contracts/service-job-record.v1.schema.json');
const jobsResponseSchema = loadServiceRecordSchema('../docs/dev/contracts/service-jobs-response.v1.schema.json');
const jobCancelResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-job-cancel-response.v1.schema.json',
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
  assertServiceJobsResponseSchemaRecord(response.data, jobsResponseSchema, `HTTP service job ${jobId} response`);
  assert(response.data?.job?.id === jobId, `HTTP service job ${jobId} returned wrong job`);
  assertServiceJobSchemaRecord(response.data.job, jobRecordSchema, `HTTP service job ${jobId}`);
  return response.data.job;
}

function assertNamingWarnings(job, expected, label) {
  assertServiceJobSchemaRecord(job, jobRecordSchema, label);
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
  const serviceBaseUrl = `http://127.0.0.1:${port}`;

  const runningWaitResponsePromise = fetch(`${serviceBaseUrl}/api/browser/wait`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: runningJobId,
      selector: '#this-element-never-appears',
      state: 'visible',
      timeoutMs: 5000,
      serviceName,
      agentName,
      taskName,
    }),
  }).then((response) => response.json());
  await delay(250);
  const queuedWaitResponsePromise = fetch(`${serviceBaseUrl}/api/browser/wait`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: cancelJobId,
      selector: '#this-element-also-never-appears',
      state: 'visible',
      timeoutMs: 30000,
      serviceName,
      agentName,
      taskName,
    }),
  }).then((response) => response.json());
  await delay(250);
  const cancelled = await cancelServiceJob({
    baseUrl: serviceBaseUrl,
    jobId: cancelJobId,
  });
  assertServiceJobCancelResponseSchemaRecord(
    cancelled,
    jobCancelResponseSchema,
    'client service job cancel response',
  );
  assert(cancelled.cancelled === true, `client service job cancel did not cancel: ${JSON.stringify(cancelled)}`);
  assert(cancelled.job?.state === 'cancelled', `cancelled job state mismatch: ${JSON.stringify(cancelled)}`);
  const queuedWaitResponse = await queuedWaitResponsePromise;
  assert(
    queuedWaitResponse.success === false &&
      typeof queuedWaitResponse.error === 'string' &&
      queuedWaitResponse.error.includes('cancelled before dispatch'),
    `cancelled queued wait did not report cancellation: ${JSON.stringify(queuedWaitResponse)}`,
  );
  const runningWaitResponse = await runningWaitResponsePromise;
  assert(
    runningWaitResponse.success === false,
    `front-running wait unexpectedly succeeded: ${JSON.stringify(runningWaitResponse)}`,
  );

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
  assertServiceJobsResponseSchemaRecord(jobs.data, jobsResponseSchema, 'HTTP service jobs response');
  const collectionUnnamedJob = jobs.data?.jobs?.find((job) => job.id === unnamedJobId);
  const collectionNamedJob = jobs.data?.jobs?.find((job) => job.id === namedJobId);
  assert(collectionUnnamedJob, `HTTP jobs collection missing unnamed job ${unnamedJobId}`);
  assert(collectionNamedJob, `HTTP jobs collection missing named job ${namedJobId}`);
  assertServiceJobSchemaRecord(collectionUnnamedJob, jobRecordSchema, 'HTTP collection unnamed job');
  assertServiceJobSchemaRecord(collectionNamedJob, jobRecordSchema, 'HTTP collection named job');
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
