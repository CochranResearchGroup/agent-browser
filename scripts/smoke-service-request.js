#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceAccessPlan,
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
  createServiceTabRequestFromAccessPlan,
  evaluateServiceTab,
  postServiceRequest,
  releaseServiceTabHandle,
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
  configureRemoteHeadedContext,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';
import {
  assertServiceJobsResponseSchemaRecord,
  assertServiceJobSchemaRecord,
  assertServiceRequestMcpToolCallSchemaRecord,
  assertServiceRequestPayloadSchemaRecord,
  loadServiceRecordSchema,
  parseMcpJsonResource,
} from './smoke-schema-utils.js';

loadAgentBrowserEnvFromRealHome();

const context = createSmokeContext({
  prefix: 'ab-service-request-',
  sessionPrefix: 'service-request',
});
const remoteHeadedConfig = configureRemoteHeadedContext(context);

const { agentHome, session, tempHome } = context;
const duplicateSession = `${session}-duplicate`;
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
const plannedTabTaskName = 'plannedServiceTabRequestSmoke';
const jobRecordSchema = loadServiceRecordSchema('../docs/dev/contracts/service-job-record.v1.schema.json');
const jobsResponseSchema = loadServiceRecordSchema('../docs/dev/contracts/service-jobs-response.v1.schema.json');
const serviceRequestSchema = loadServiceRecordSchema('../docs/dev/contracts/service-request.v1.schema.json');
const serviceRequestMcpToolCallSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json',
);

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for service request smoke to complete');
}, 150000);

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
            browserBuild: 'stealthcdp_chromium',
            defaultBrowserHost: 'remote_headed',
            persistent: true,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

function remoteHeadedHints() {
  return {
    browserBuild: 'stealthcdp_chromium',
    displayIsolation: 'private_virtual_display',
  };
}

function remoteHeadedParams() {
  return {
    browserHost: 'remote_headed',
    viewStreamProvider: remoteHeadedConfig.viewStreamProvider,
    controlInputProvider: remoteHeadedConfig.controlInputProvider,
    displayIsolation: 'private_virtual_display',
    viewStreamUrl: remoteHeadedConfig.viewStreamUrl,
    ...(remoteHeadedConfig.frameUrl ? { frameUrl: remoteHeadedConfig.frameUrl } : {}),
    ...(remoteHeadedConfig.externalUrl ? { externalUrl: remoteHeadedConfig.externalUrl } : {}),
    ...(remoteHeadedConfig.routeId ? { routeId: remoteHeadedConfig.routeId } : {}),
    ...(remoteHeadedConfig.connectionId ? { connectionId: remoteHeadedConfig.connectionId } : {}),
    ...(remoteHeadedConfig.connectionName ? { connectionName: remoteHeadedConfig.connectionName } : {}),
    headless: false,
  };
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
  try {
    await closeSession({ ...context, session: duplicateSession });
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

async function ensureStreamPort(targetSession = session) {
  const streamStatusResult = await runCli(context, ['--json', '--session', targetSession, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', targetSession, 'stream', 'enable']);
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
    ...remoteHeadedHints(),
    action: 'navigate',
    params: {
      ...remoteHeadedParams(),
      url: httpUrl,
      waitUntil: 'load',
      id: 'ignored-by-service-request',
      action: 'ignored-by-service-request',
    },
    jobTimeoutMs: 120000,
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
    ['authenticated_target', 'explicit_profile'].includes(activeSession.profileSelectionReason),
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
    ...remoteHeadedHints(),
    params: remoteHeadedParams(),
    jobTimeoutMs: 120000,
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
    ...remoteHeadedHints(),
    params: remoteHeadedParams(),
    jobTimeoutMs: 120000,
  });
  assert(tabResponse.success === true, `client service tab request failed: ${JSON.stringify(tabResponse)}`);

  const accessPlan = await getServiceAccessPlan({
    baseUrl: serviceBaseUrl,
    serviceName,
    agentName,
    taskName: plannedTabTaskName,
    loginId: targetServiceId,
    ...remoteHeadedHints(),
  });
  assert(accessPlan.selectedProfile?.id === selectedProfileId, `access plan selected wrong profile: ${JSON.stringify(accessPlan)}`);
  assert(
    accessPlan.selectedProfileMatch?.reason === 'authenticated_target',
    `access plan selected profile for wrong reason: ${JSON.stringify(accessPlan)}`,
  );
  assert(accessPlan.decision?.serviceRequest?.available === true, `access plan service request was unavailable: ${JSON.stringify(accessPlan)}`);
  assert(
    accessPlan.decision.serviceRequest.request?.serviceName === serviceName &&
      accessPlan.decision.serviceRequest.request.agentName === agentName &&
      accessPlan.decision.serviceRequest.request.taskName === plannedTabTaskName,
    `access plan service request missing caller labels: ${JSON.stringify(accessPlan)}`,
  );
  assert(
    JSON.stringify(accessPlan.decision.serviceRequest.request?.targetServiceIds) ===
      JSON.stringify([targetServiceId]),
    `access plan service request target IDs mismatch: ${JSON.stringify(accessPlan)}`,
  );
  assert(
    accessPlan.decision.serviceRequest.request?.profileLeasePolicy === 'wait',
    `access plan service request lease policy mismatch: ${JSON.stringify(accessPlan)}`,
  );
  assert(
    accessPlan.decision.profileReuse?.recommendedAction === 'reuse_existing_browser',
    `access plan did not recommend retained browser reuse: ${JSON.stringify(accessPlan.decision.profileReuse)}`,
  );
  assert(
    accessPlan.decision.profileReuse?.sharedAcquisition?.mode === 'tab_new',
    `access plan did not recommend shared tab acquisition: ${JSON.stringify(accessPlan.decision.profileReuse)}`,
  );
  assert(
    typeof accessPlan.decision.profileReuse?.sharedAcquisition?.browserId === 'string' &&
      typeof accessPlan.decision.profileReuse?.sharedAcquisition?.sessionName === 'string',
    `access plan did not include retained browser route hints: ${JSON.stringify(accessPlan.decision.profileReuse)}`,
  );
  const plannedTabUrl = smokeDataUrl('Planned Service Tab Request Smoke', 'Planned Service Tab Request Smoke');
  const plannedTabRequest = createServiceTabRequestFromAccessPlan(accessPlan, {
    url: plannedTabUrl,
    ...remoteHeadedHints(),
    params: remoteHeadedParams(),
    jobTimeoutMs: 120000,
  });
  assert(
    plannedTabRequest.browserId === accessPlan.decision.profileReuse.sharedAcquisition.browserId &&
      plannedTabRequest.sessionName === accessPlan.decision.profileReuse.sharedAcquisition.sessionName,
    `planned tab request did not copy retained browser route hints: ${JSON.stringify(plannedTabRequest)}`,
  );
  assertServiceRequestPayloadSchemaRecord(
    plannedTabRequest,
    serviceRequestSchema,
    'planned access-plan service tab request payload',
  );
  const plannedTabResponse = await requestServiceTab({
    baseUrl: serviceBaseUrl,
    accessPlan,
    url: plannedTabUrl,
    ...remoteHeadedHints(),
    params: remoteHeadedParams(),
    jobTimeoutMs: 120000,
  });
  assert(
    plannedTabResponse.success === true,
    `planned client service tab request failed: ${JSON.stringify(plannedTabResponse)}`,
  );
  assert(
    plannedTabResponse.data?.sharedAcquisition?.browserReused === true &&
      plannedTabResponse.data?.sharedAcquisition?.tabOpened === true,
    `planned tab response did not report retained browser tab acquisition: ${JSON.stringify(plannedTabResponse)}`,
  );
  const duplicatePort = await ensureStreamPort(duplicateSession);
  const duplicateLaunchResponse = await postServiceRequest({
    baseUrl: `http://127.0.0.1:${duplicatePort}`,
    request: createServiceRequest({
      serviceName,
      agentName,
      taskName: 'duplicateProfileLaunchRejectedSmoke',
      action: 'navigate',
      runtimeProfile: selectedProfileId,
      ...remoteHeadedHints(),
      params: {
        ...remoteHeadedParams(),
        url: smokeDataUrl('Duplicate Profile Launch Rejected', 'Duplicate Profile Launch Rejected'),
        waitUntil: 'load',
      },
      profileLeasePolicy: 'wait',
      profileLeaseWaitTimeoutMs: 1000,
      jobTimeoutMs: 120000,
    }),
  });
  assert(
    duplicateLaunchResponse.success === false,
    `duplicate profile launch unexpectedly succeeded: ${JSON.stringify(duplicateLaunchResponse)}`,
  );
  assert(
    duplicateLaunchResponse.error?.includes('Duplicate service profile lane blocked') &&
      duplicateLaunchResponse.error.includes(selectedProfileId),
    `duplicate profile launch did not report duplicate-lane rejection: ${JSON.stringify(duplicateLaunchResponse)}`,
  );
  const firstSharedHandle = tabResponse.data?.serviceTabHandle;
  const plannedSharedHandle = plannedTabResponse.data?.serviceTabHandle;
  assert(firstSharedHandle?.valid === true, `first shared tab missing valid handle: ${JSON.stringify(tabResponse)}`);
  assert(plannedSharedHandle?.valid === true, `planned shared tab missing valid handle: ${JSON.stringify(plannedTabResponse)}`);
  assert(
    firstSharedHandle.browserId === plannedSharedHandle.browserId &&
      firstSharedHandle.sessionName === plannedSharedHandle.sessionName,
    `shared tab handles did not route through one retained browser: ${JSON.stringify({ firstSharedHandle, plannedSharedHandle })}`,
  );
  assert(
    firstSharedHandle.targetId !== plannedSharedHandle.targetId,
    `shared tab handles unexpectedly reused one target: ${JSON.stringify({ firstSharedHandle, plannedSharedHandle })}`,
  );
  const releaseResponse = await releaseServiceTabHandle({
    baseUrl: serviceBaseUrl,
    serviceName,
    agentName,
    taskName: 'releaseFirstSharedTabSmoke',
    serviceTabHandle: firstSharedHandle,
    jobTimeoutMs: 120000,
  });
  assert(releaseResponse.success === true, `shared tab release failed: ${JSON.stringify(releaseResponse)}`);
  assert(releaseResponse.data?.tabReleased === true, `shared tab release did not update state: ${JSON.stringify(releaseResponse)}`);
  assert(
    releaseResponse.data?.browserProcessPreserved === true &&
      releaseResponse.data?.sessionRoutePreserved === true &&
      releaseResponse.data?.closeBrowserOnRelease === false,
    `shared tab release did not preserve retained browser route: ${JSON.stringify(releaseResponse)}`,
  );
  assert(
    releaseResponse.data?.physicalTabCloseAttempted === true &&
      releaseResponse.data?.physicalTabClosed === true,
    `shared tab release did not physically close the selected target: ${JSON.stringify(releaseResponse)}`,
  );
  assert(
    releaseResponse.data?.serviceTabHandle?.valid === false &&
      releaseResponse.data?.serviceTabHandle?.staleReason === 'tab_closed',
    `shared tab release did not return stale tab evidence: ${JSON.stringify(releaseResponse)}`,
  );
  const plannedStillUsable = await evaluateServiceTab({
    baseUrl: serviceBaseUrl,
    serviceName,
    agentName,
    taskName: 'plannedSharedTabSurvivesReleaseSmoke',
    serviceTabHandle: plannedSharedHandle,
    script: 'document.title',
    returnByValue: true,
    timeoutMs: 5000,
    maxReturnBytes: 128,
    jobTimeoutMs: 120000,
  });
  assert(
    plannedStillUsable.success === true && plannedStillUsable.data?.ok === true,
    `planned shared tab was not usable after release: ${JSON.stringify(plannedStillUsable)}`,
  );
  assert(
    plannedStillUsable.data?.targetId === plannedSharedHandle.targetId,
    `post-release evaluate used the wrong target: ${JSON.stringify(plannedStillUsable)}`,
  );
  assert(
    plannedStillUsable.data?.result?.result?.value === 'Planned Service Tab Request Smoke' ||
      plannedStillUsable.data?.result === 'Planned Service Tab Request Smoke',
    `planned shared tab title changed after release: ${JSON.stringify(plannedStillUsable)}`,
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
    jobTimeoutMs: 120000,
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

  const plannedTabJob = findJob(jobs, {
    prefix: 'http-service-request-tab_new-',
    taskName: plannedTabTaskName,
  });
  assert(plannedTabJob, `planned client service tab request job missing: ${JSON.stringify(jobs)}`);
  assertServiceJobSchemaRecord(plannedTabJob, jobRecordSchema, 'planned client service tab request job');
  assert(
    plannedTabJob.action === 'tab_new',
    `planned client service tab request action mismatch: ${JSON.stringify(plannedTabJob)}`,
  );
  assert(
    plannedTabJob.state === 'succeeded',
    `planned client service tab request did not succeed: ${JSON.stringify(plannedTabJob)}`,
  );
  assertJobIdentityHints(
    plannedTabJob,
    {
      siteId: null,
      loginId: null,
      targetServiceId: null,
      targetServiceIds: [targetServiceId],
    },
    'planned client service tab request job',
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
  const traceIdentityContext = traceResponse.summary?.contexts?.find(
    (context) =>
      context.serviceName === serviceName &&
      context.agentName === agentName &&
      context.taskName === httpTaskName &&
      JSON.stringify(context.targetServiceIds) === JSON.stringify(fallbackTargetServiceIds),
  );
  assert(
    traceIdentityContext?.targetIdentityCount === fallbackTargetServiceIds.length,
    `service trace summary missing retained target identities: ${JSON.stringify(traceResponse.summary)}`,
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
