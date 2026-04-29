#!/usr/bin/env node

import {
  assert,
  assertServiceTracePayload,
  closeSession,
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

const timeout = setTimeout(() => {
  fail('Timed out waiting for service job naming smoke to complete');
}, 90000);

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  console.error(message);
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
  return response.data.job;
}

function assertNamingWarnings(job, expected, label) {
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

  console.log('Service job naming warning smoke passed');
  await cleanup();
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
