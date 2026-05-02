#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  parseMcpToolPayload,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';
import { parseMcpJsonResource } from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-request-',
  sessionPrefix: 'service-request',
});

const { agentHome, session, tempHome } = context;
const serviceName = 'ServiceRequestSmoke';
const agentName = 'smoke-agent';
const targetServiceId = 'acs';
const selectedProfileId = `service-request-selected-${process.pid}`;
const fallbackProfileId = `service-request-fallback-${process.pid}`;
const selectedUserDataDir = join(tempHome, 'selected-profile-user-data');
const fallbackUserDataDir = join(tempHome, 'fallback-profile-user-data');
const httpTaskName = 'httpServiceRequestSmoke';
const mcpTaskName = 'mcpServiceRequestSmoke';

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service request smoke to complete');
}, 90000);

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [fallbackProfileId]: {
            id: fallbackProfileId,
            name: 'Fallback service request profile',
            userDataDir: fallbackUserDataDir,
            targetServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
          [selectedProfileId]: {
            id: selectedProfileId,
            name: 'Authenticated service request profile',
            userDataDir: selectedUserDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession(context);
  } finally {
    if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
      console.error(`Keeping smoke home: ${tempHome}`);
    } else {
      context.cleanupTempHome();
    }
  }
}

async function fail(message) {
  if (mcp) mcp.rejectPending(message);
  await cleanup();
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

async function ensureStreamPort() {
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
  return port;
}

function findJob(jobs, { prefix, taskName }) {
  return jobs.find(
    (job) =>
      typeof job.id === 'string' &&
      job.id.startsWith(prefix) &&
      job.serviceName === serviceName &&
      job.agentName === agentName &&
      job.taskName === taskName,
  );
}

try {
  seedServiceState();

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const port = await ensureStreamPort();
  const httpUrl = smokeDataUrl('HTTP Service Request Smoke', 'HTTP Service Request Smoke');
  const httpResponse = await httpJson(port, 'POST', '/api/service/request', {
    serviceName,
    agentName,
    taskName: httpTaskName,
    siteId: targetServiceId,
    action: 'navigate',
    params: {
      url: httpUrl,
      waitUntil: 'load',
      id: 'ignored-by-service-request',
      action: 'ignored-by-service-request',
    },
    jobTimeoutMs: 30000,
  });
  assert(httpResponse.success === true, `HTTP service request failed: ${JSON.stringify(httpResponse)}`);
  assert(
    httpResponse.data?.url?.startsWith('data:text/html'),
    `HTTP service request did not navigate to data URL: ${JSON.stringify(httpResponse.data)}`,
  );

  const sessionsResponse = await httpJson(port, 'GET', '/api/service/sessions');
  assert(sessionsResponse.success === true, `HTTP service sessions failed: ${JSON.stringify(sessionsResponse)}`);
  const activeSession = sessionsResponse.data?.sessions?.find((item) => item.id === session);
  assert(activeSession, `HTTP service sessions missing active session: ${JSON.stringify(sessionsResponse.data)}`);
  assert(
    activeSession.profileId === selectedProfileId,
    `HTTP service request selected wrong profile: ${JSON.stringify(activeSession)}`,
  );

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-request-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  assert(initialize.capabilities?.resources, 'MCP resources capability missing');
  mcp.notify('notifications/initialized');

  const mcpUrl = smokeDataUrl('MCP Service Request Smoke', 'MCP Service Request Smoke');
  const mcpResult = await mcp.send('tools/call', {
    name: 'service_request',
    arguments: {
      serviceName,
      agentName,
      taskName: mcpTaskName,
      siteId: targetServiceId,
      loginIds: ['orcid'],
      action: 'navigate',
      params: {
        url: mcpUrl,
        waitUntil: 'load',
      },
      jobTimeoutMs: 30000,
    },
  });
  const mcpPayload = parseMcpToolPayload(mcpResult, 'service_request');
  assert(mcpPayload.success === true, `MCP service_request failed: ${JSON.stringify(mcpPayload)}`);
  assert(mcpPayload.tool === 'service_request', `MCP payload tool mismatch: ${JSON.stringify(mcpPayload)}`);
  assert(mcpPayload.trace?.serviceName === serviceName, 'MCP trace missing serviceName');
  assert(mcpPayload.trace?.siteId === targetServiceId, 'MCP trace missing siteId');
  assert(mcpPayload.trace?.loginIds?.[0] === 'orcid', 'MCP trace missing loginIds');
  assert(
    mcpPayload.data?.url?.startsWith('data:text/html'),
    `MCP service_request did not navigate to data URL: ${JSON.stringify(mcpPayload.data)}`,
  );

  const jobsResponse = await httpJson(port, 'GET', '/api/service/jobs?limit=50');
  assert(jobsResponse.success === true, `HTTP service jobs failed: ${JSON.stringify(jobsResponse)}`);
  const jobs = jobsResponse.data?.jobs ?? [];
  const httpJob = findJob(jobs, {
    prefix: 'http-service-request-navigate-',
    taskName: httpTaskName,
  });
  assert(httpJob, `HTTP service request job missing: ${JSON.stringify(jobs)}`);
  assert(httpJob.action === 'navigate', `HTTP service request job action mismatch: ${JSON.stringify(httpJob)}`);
  assert(httpJob.state === 'succeeded', `HTTP service request job did not succeed: ${JSON.stringify(httpJob)}`);

  const mcpJob = findJob(jobs, {
    prefix: 'mcp-service-request-navigate-',
    taskName: mcpTaskName,
  });
  assert(mcpJob, `MCP service_request job missing: ${JSON.stringify(jobs)}`);
  assert(mcpJob.action === 'navigate', `MCP service_request job action mismatch: ${JSON.stringify(mcpJob)}`);
  assert(mcpJob.state === 'succeeded', `MCP service_request job did not succeed: ${JSON.stringify(mcpJob)}`);

  const mcpJobsResource = await mcp.send('resources/read', { uri: 'agent-browser://jobs' });
  const mcpJobs = parseMcpJsonResource(mcpJobsResource, 'agent-browser://jobs', 'MCP jobs resource');
  assert(
    findJob(mcpJobs.jobs ?? [], {
      prefix: 'mcp-service-request-navigate-',
      taskName: mcpTaskName,
    }),
    `MCP jobs resource missing service_request job: ${JSON.stringify(mcpJobs)}`,
  );

  await cleanup();
  console.log('Service request HTTP/MCP live smoke passed');
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
