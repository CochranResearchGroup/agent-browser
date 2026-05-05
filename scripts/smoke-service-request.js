#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceEvents,
  getServiceJob,
  getServiceJobs,
  getServiceTrace,
} from '../packages/client/src/service-observability.js';
import {
  SERVICE_REQUEST_ACTIONS,
  createServiceRequest,
  createServiceRequestMcpToolCall,
  createServiceTabRequest,
  postServiceRequest,
  requestServiceTab,
} from '../packages/client/src/service-request.js';
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
import {
  assertServiceJobsResponseSchemaRecord,
  assertServiceJobSchemaRecord,
  assertServiceRequestMcpToolCallSchemaRecord,
  assertServiceRequestPayloadSchemaRecord,
  loadServiceRecordSchema,
  parseMcpJsonResource,
} from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-service-request-',
  sessionPrefix: 'service-request',
});

const { agentHome, session, tempHome } = context;
const serviceName = 'ServiceRequestSmoke';
const agentName = 'smoke-agent';
const targetServiceId = 'acs';
const fallbackTargetServiceIds = [
  targetServiceId,
  'google',
  'microsoft',
  'orcid',
  'nih',
  'pubmed',
  'crossref',
  'scopus',
  'wos',
  'canvas',
  'github',
  'gmail',
  'outlook',
];
const selectedProfileId = `service-request-selected-${process.pid}`;
const fallbackProfileId = `service-request-fallback-${process.pid}`;
const selectedUserDataDir = join(tempHome, 'selected-profile-user-data');
const fallbackUserDataDir = join(tempHome, 'fallback-profile-user-data');
const httpTaskName = 'httpServiceRequestSmoke';
const mcpTaskName = 'mcpServiceRequestSmoke';
const tabTaskName = 'serviceTabRequestSmoke';
const jobRecordSchema = loadServiceRecordSchema('../docs/dev/contracts/service-job-record.v1.schema.json');
const jobsResponseSchema = loadServiceRecordSchema('../docs/dev/contracts/service-jobs-response.v1.schema.json');
const serviceRequestSchema = loadServiceRecordSchema('../docs/dev/contracts/service-request.v1.schema.json');
const serviceRequestMcpToolCallSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json',
);

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
            targetServiceIds: fallbackTargetServiceIds,
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

function assertJobIdentityHints(job, expected, label) {
  for (const [field, value] of Object.entries(expected)) {
    if (field === 'targetServiceIds') {
      assert(
        JSON.stringify(job.targetServiceIds) === JSON.stringify(value),
        `${label} ${field} mismatch: ${JSON.stringify(job)}`,
      );
    } else {
      assert(job[field] === value, `${label} ${field} mismatch: ${JSON.stringify(job)}`);
    }
  }
}

try {
  seedServiceState();

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const port = await ensureStreamPort();
  assert(
    SERVICE_REQUEST_ACTIONS.length === serviceRequestSchema.properties.action.enum.length,
    'generated service request action list drifted from schema',
  );
  const httpUrl = smokeDataUrl('HTTP Service Request Smoke', 'HTTP Service Request Smoke');
  const httpRequest = createServiceRequest({
    serviceName,
    agentName,
    taskName: httpTaskName,
    siteId: targetServiceId,
    targetServices: fallbackTargetServiceIds.slice(1),
    action: 'navigate',
    params: {
      url: httpUrl,
      waitUntil: 'load',
      id: 'ignored-by-service-request',
      action: 'ignored-by-service-request',
    },
    jobTimeoutMs: 30000,
  });
  assertServiceRequestPayloadSchemaRecord(httpRequest, serviceRequestSchema, 'HTTP service request payload');
  const explicitProfileRequest = createServiceRequest({
    serviceName,
    agentName,
    taskName: 'explicitProfileContractSmoke',
    action: 'tab_list',
    profile: selectedUserDataDir,
    runtimeProfile: selectedProfileId,
  });
  assertServiceRequestPayloadSchemaRecord(
    explicitProfileRequest,
    serviceRequestSchema,
    'explicit profile service request payload',
  );
  const httpResponse = await postServiceRequest({
    baseUrl: `http://127.0.0.1:${port}`,
    request: httpRequest,
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
  assert(
    activeSession.profileSelectionReason === 'authenticated_target',
    `HTTP service request selected profile for wrong reason: ${JSON.stringify(activeSession)}`,
  );
  assert(
    activeSession.profileLeaseDisposition === 'new_browser',
    `HTTP service request recorded wrong profile lease disposition: ${JSON.stringify(activeSession)}`,
  );
  assert(
    Array.isArray(activeSession.profileLeaseConflictSessionIds) &&
      activeSession.profileLeaseConflictSessionIds.length === 0,
    `HTTP service request recorded unexpected profile lease conflicts: ${JSON.stringify(activeSession)}`,
  );

  const serviceBaseUrl = `http://127.0.0.1:${port}`;
  const tabUrl = smokeDataUrl('Service Tab Request Smoke', 'Service Tab Request Smoke');
  const tabRequest = createServiceTabRequest({
    serviceName,
    agentName,
    taskName: tabTaskName,
    siteId: targetServiceId,
    loginId: targetServiceId,
    targetServices: fallbackTargetServiceIds.slice(1),
    url: tabUrl,
    jobTimeoutMs: 30000,
  });
  assertServiceRequestPayloadSchemaRecord(tabRequest, serviceRequestSchema, 'client service tab request payload');
  const tabResponse = await requestServiceTab({
    baseUrl: serviceBaseUrl,
    serviceName,
    agentName,
    taskName: tabTaskName,
    siteId: targetServiceId,
    loginId: targetServiceId,
    targetServices: fallbackTargetServiceIds.slice(1),
    url: tabUrl,
    jobTimeoutMs: 30000,
  });
  assert(tabResponse.success === true, `client service tab request failed: ${JSON.stringify(tabResponse)}`);

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
  const mcpToolCall = createServiceRequestMcpToolCall({
    serviceName,
    agentName,
    taskName: mcpTaskName,
    siteId: targetServiceId,
    targetServices: fallbackTargetServiceIds.slice(1),
    loginIds: ['orcid'],
    action: 'navigate',
    params: {
      url: mcpUrl,
      waitUntil: 'load',
    },
    jobTimeoutMs: 30000,
  });
  assertServiceRequestMcpToolCallSchemaRecord(
    mcpToolCall,
    serviceRequestMcpToolCallSchema,
    serviceRequestSchema,
    'MCP service_request tool call',
  );
  const mcpResult = await mcp.send('tools/call', mcpToolCall);
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

  const jobsResponseData = await getServiceJobs({
    baseUrl: serviceBaseUrl,
    query: { limit: 50 },
  });
  assertServiceJobsResponseSchemaRecord(
    jobsResponseData,
    jobsResponseSchema,
    'HTTP service request jobs response',
  );
  const jobs = jobsResponseData?.jobs ?? [];
  const httpJob = findJob(jobs, {
    prefix: 'http-service-request-navigate-',
    taskName: httpTaskName,
  });
  assert(httpJob, `HTTP service request job missing: ${JSON.stringify(jobs)}`);
  assertServiceJobSchemaRecord(httpJob, jobRecordSchema, 'HTTP service request job');
  assert(httpJob.action === 'navigate', `HTTP service request job action mismatch: ${JSON.stringify(httpJob)}`);
  assert(httpJob.state === 'succeeded', `HTTP service request job did not succeed: ${JSON.stringify(httpJob)}`);
  assertJobIdentityHints(
    httpJob,
    {
      siteId: targetServiceId,
      loginId: null,
      targetServiceId: null,
      targetServiceIds: fallbackTargetServiceIds,
    },
    'HTTP service request job',
  );

  const tabJob = findJob(jobs, {
    prefix: 'http-service-request-tab_new-',
    taskName: tabTaskName,
  });
  assert(tabJob, `client service tab request job missing: ${JSON.stringify(jobs)}`);
  assertServiceJobSchemaRecord(tabJob, jobRecordSchema, 'client service tab request job');
  assert(tabJob.action === 'tab_new', `client service tab request action mismatch: ${JSON.stringify(tabJob)}`);
  assert(tabJob.state === 'succeeded', `client service tab request did not succeed: ${JSON.stringify(tabJob)}`);
  assertJobIdentityHints(
    tabJob,
    {
      siteId: targetServiceId,
      loginId: targetServiceId,
      targetServiceId: null,
      targetServiceIds: fallbackTargetServiceIds,
    },
    'client service tab request job',
  );

  const httpJobDetail = await getServiceJob({
    baseUrl: serviceBaseUrl,
    id: httpJob.id,
  });
  assertServiceJobsResponseSchemaRecord(
    httpJobDetail,
    jobsResponseSchema,
    'HTTP service request job detail response',
  );
  assertServiceJobSchemaRecord(httpJobDetail.job, jobRecordSchema, 'HTTP service request job detail');
  assert(httpJobDetail.job.id === httpJob.id, 'HTTP service request job detail returned wrong job');

  const mcpJob = findJob(jobs, {
    prefix: 'mcp-service-request-navigate-',
    taskName: mcpTaskName,
  });
  assert(mcpJob, `MCP service_request job missing: ${JSON.stringify(jobs)}`);
  assertServiceJobSchemaRecord(mcpJob, jobRecordSchema, 'MCP service_request job');
  assert(mcpJob.action === 'navigate', `MCP service_request job action mismatch: ${JSON.stringify(mcpJob)}`);
  assert(mcpJob.state === 'succeeded', `MCP service_request job did not succeed: ${JSON.stringify(mcpJob)}`);
  assertJobIdentityHints(
    mcpJob,
    {
      siteId: targetServiceId,
      loginId: null,
      targetServiceId: null,
      targetServiceIds: fallbackTargetServiceIds,
    },
    'MCP service_request job',
  );

  const eventsResponse = await getServiceEvents({
    baseUrl: serviceBaseUrl,
    query: { limit: 20 },
  });
  assert(Array.isArray(eventsResponse.events), `service events client missing events: ${JSON.stringify(eventsResponse)}`);
  assert(Number.isInteger(eventsResponse.count), `service events client missing count: ${JSON.stringify(eventsResponse)}`);

  const traceResponse = await getServiceTrace({
    baseUrl: serviceBaseUrl,
    query: { serviceName, limit: 50 },
  });
  assert(
    traceResponse.jobs.some((job) => job.id === httpJob.id),
    `service trace client missing HTTP job: ${JSON.stringify(traceResponse.jobs)}`,
  );
  assertJobIdentityHints(
    traceResponse.jobs.find((job) => job.id === httpJob.id),
    {
      siteId: targetServiceId,
      loginId: null,
      targetServiceId: null,
      targetServiceIds: fallbackTargetServiceIds,
    },
    'service trace HTTP job',
  );
  assert(
    traceResponse.jobs.some((job) => job.id === mcpJob.id),
    `service trace client missing MCP job: ${JSON.stringify(traceResponse.jobs)}`,
  );

  const mcpJobsResource = await mcp.send('resources/read', { uri: 'agent-browser://jobs' });
  const mcpJobs = parseMcpJsonResource(mcpJobsResource, 'agent-browser://jobs', 'MCP jobs resource');
  assert(Array.isArray(mcpJobs.jobs), `MCP jobs resource missing jobs array: ${JSON.stringify(mcpJobs)}`);
  assert(Number.isInteger(mcpJobs.count), `MCP jobs resource missing count integer: ${JSON.stringify(mcpJobs)}`);
  assert(mcpJobs.count === mcpJobs.jobs.length, `MCP jobs resource count mismatch: ${JSON.stringify(mcpJobs)}`);
  for (const [index, job] of mcpJobs.jobs.entries()) {
    assertServiceJobSchemaRecord(job, jobRecordSchema, `MCP jobs resource job ${index}`);
  }
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
