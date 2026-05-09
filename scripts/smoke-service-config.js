#!/usr/bin/env node

import {
  deleteServiceMonitor,
  deleteServiceProfile,
  deleteServiceProvider,
  deleteServiceSession,
  deleteServiceSitePolicy,
  getServiceMonitors,
  getServiceProfiles,
  getServiceProviders,
  getServiceSessions,
  getServiceSitePolicies,
  updateServiceProfileFreshness,
  upsertServiceProfileReadinessMonitor,
  upsertServiceMonitor,
  upsertServiceProfile,
  upsertServiceProvider,
  upsertServiceSession,
  upsertServiceSitePolicy,
} from '../packages/client/src/service-observability.js';
import {
  assert,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  parseMcpToolPayload,
  runCli,
} from './smoke-utils.js';
import {
  assertServiceMonitorDeleteResponseSchemaRecord,
  assertServiceMonitorUpsertResponseSchemaRecord,
  assertServiceProfileDeleteResponseSchemaRecord,
  assertServiceProfileUpsertResponseSchemaRecord,
  assertServiceProviderDeleteResponseSchemaRecord,
  assertServiceProviderUpsertResponseSchemaRecord,
  assertServiceSessionDeleteResponseSchemaRecord,
  assertServiceSessionUpsertResponseSchemaRecord,
  assertServiceSitePolicyDeleteResponseSchemaRecord,
  assertServiceSitePolicyUpsertResponseSchemaRecord,
  loadServiceRecordSchema,
  parseMcpJsonResource,
} from './smoke-schema-utils.js';

const context = createSmokeContext({ prefix: 'ab-scfg-', sessionPrefix: 'scfg' });
const { session } = context;
const serviceName = 'ServiceConfigSmoke';
const agentName = 'smoke-agent';
const taskName = 'configMutationParity';
const traceFields = { serviceName, agentName, taskName };
const genericProfileId = 'generic-journal-profile';
const genericTargetId = 'generic-journal-login';
const genericMonitorId = 'generic-journal-login-profile-readiness';
const profileUpsertResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-profile-upsert-response.v1.schema.json',
);
const profileDeleteResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-profile-delete-response.v1.schema.json',
);
const sessionUpsertResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-session-upsert-response.v1.schema.json',
);
const sessionDeleteResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-session-delete-response.v1.schema.json',
);
const sitePolicyUpsertResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-site-policy-upsert-response.v1.schema.json',
);
const sitePolicyDeleteResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-site-policy-delete-response.v1.schema.json',
);
const monitorUpsertResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-monitor-upsert-response.v1.schema.json',
);
const monitorDeleteResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-monitor-delete-response.v1.schema.json',
);
const providerUpsertResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-provider-upsert-response.v1.schema.json',
);
const providerDeleteResponseSchema = loadServiceRecordSchema(
  '../docs/dev/contracts/service-provider-delete-response.v1.schema.json',
);
let mcp;

const timeout = setTimeout(() => {
  fail('Timed out waiting for service config smoke to complete');
}, 120000);

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
  if (mcp) mcp.rejectPending(message);
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  await cleanup();
  process.exit(1);
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
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function assertCollectionMissing(collection, key, id, label) {
  assert(Array.isArray(collection?.[key]), `${label} missing ${key} array: ${JSON.stringify(collection)}`);
  assert(!collection[key].some((item) => item.id === id), `${label} unexpectedly retained ${id}`);
}

function assertSource(collection, key, id, expectedSource, label) {
  assert(Array.isArray(collection?.[key]), `${label} missing ${key} array: ${JSON.stringify(collection)}`);
  assert(
    collection[key].some((source) => source.id === id && source.source === expectedSource),
    `${label} did not report ${id} as ${expectedSource}: ${JSON.stringify(collection)}`,
  );
}

function assertGenericRegistrationAccessPlan(data, label) {
  assert(data?.query?.serviceName === serviceName, `${label} serviceName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.agentName === agentName, `${label} agentName mismatch: ${JSON.stringify(data)}`);
  assert(data?.query?.taskName === taskName, `${label} taskName mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.query?.targetServiceIds?.includes(genericTargetId),
    `${label} target identity mismatch: ${JSON.stringify(data)}`,
  );
  assert(data?.selectedProfile?.id === genericProfileId, `${label} selected profile mismatch: ${JSON.stringify(data)}`);
  assert(
    data?.selectedProfileMatch?.reason === 'authenticated_target',
    `${label} selected profile reason mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.monitorFindings?.profileReadinessAttentionRequired === false,
    `${label} monitor findings should not require attention: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.available === true,
    `${label} planned service request unavailable: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.action === 'tab_new',
    `${label} planned action mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.serviceName === serviceName &&
      data?.decision?.serviceRequest?.request?.agentName === agentName &&
      data?.decision?.serviceRequest?.request?.taskName === taskName,
    `${label} planned caller labels mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.request?.loginId === genericTargetId ||
      data?.decision?.serviceRequest?.request?.targetServiceId === genericTargetId ||
      data?.decision?.serviceRequest?.request?.targetServiceIds?.includes(genericTargetId),
    `${label} planned target identity mismatch: ${JSON.stringify(data)}`,
  );
  assert(
    data?.decision?.serviceRequest?.mcp?.tool === 'service_request',
    `${label} planned MCP tool mismatch: ${JSON.stringify(data)}`,
  );
}

try {
  const port = await enableStream();
  const serviceBaseUrl = `http://127.0.0.1:${port}`;

  const httpProfile = await upsertServiceProfile({
    baseUrl: serviceBaseUrl,
    id: 'journal-downloader',
    profile: {
      name: 'Journal Downloader',
      allocation: 'per_service',
      keyring: 'basic_password_store',
      persistent: true,
      targetServiceIds: ['acs'],
      authenticatedServiceIds: ['acs'],
      sharedServiceIds: [serviceName],
    },
  });
  assertServiceProfileUpsertResponseSchemaRecord(
    httpProfile,
    profileUpsertResponseSchema,
    'client profile upsert response',
  );
  assert(
    httpProfile.profile?.id === 'journal-downloader',
    `HTTP profile id mismatch: ${JSON.stringify(httpProfile)}`,
  );
  const clientProfile = await upsertServiceProfile({
    baseUrl: serviceBaseUrl,
    id: 'client-profile',
    profile: {
      name: 'Client Profile',
      allocation: 'per_service',
      keyring: 'basic_password_store',
      persistent: true,
      targetServiceIds: ['client-target'],
      sharedServiceIds: [serviceName],
    },
  });
  assertServiceProfileUpsertResponseSchemaRecord(
    clientProfile,
    profileUpsertResponseSchema,
    'client extra profile upsert response',
  );
  assert(clientProfile.profile?.id === 'client-profile', `client profile id mismatch: ${JSON.stringify(clientProfile)}`);
  const freshnessProfile = await updateServiceProfileFreshness({
    baseUrl: serviceBaseUrl,
    id: 'client-profile',
    loginId: 'client-target',
    readinessState: 'fresh',
    readinessEvidence: 'auth_probe_cookie_present',
    lastVerifiedAt: '2026-05-06T12:00:00Z',
    freshnessExpiresAt: '2026-05-06T13:00:00Z',
  });
  assertServiceProfileUpsertResponseSchemaRecord(
    freshnessProfile,
    profileUpsertResponseSchema,
    'client freshness profile update response',
  );
  assert(
    freshnessProfile.profile?.targetReadiness?.[0]?.state === 'fresh',
    `client profile freshness state mismatch: ${JSON.stringify(freshnessProfile)}`,
  );
  assert(
    freshnessProfile.profile?.authenticatedServiceIds?.includes('client-target'),
    `client profile freshness auth targets mismatch: ${JSON.stringify(freshnessProfile)}`,
  );

  const httpPolicy = await upsertServiceSitePolicy({
    baseUrl: serviceBaseUrl,
    id: 'google',
    sitePolicy: {
      originPattern: 'https://accounts.google.com',
      interactionMode: 'human_like_input',
      challengePolicy: 'avoid_first',
      manualLoginPreferred: true,
      profileRequired: true,
    },
  });
  assertServiceSitePolicyUpsertResponseSchemaRecord(
    httpPolicy,
    sitePolicyUpsertResponseSchema,
    'client site policy upsert response',
  );
  assert(httpPolicy.sitePolicy?.id === 'google', `HTTP policy id mismatch: ${JSON.stringify(httpPolicy)}`);
  const clientPolicy = await upsertServiceSitePolicy({
    baseUrl: serviceBaseUrl,
    id: 'client-google',
    sitePolicy: {
      originPattern: 'https://client.example.com',
      interactionMode: 'human_like_input',
      challengePolicy: 'avoid_first',
      manualLoginPreferred: false,
      profileRequired: false,
    },
  });
  assertServiceSitePolicyUpsertResponseSchemaRecord(
    clientPolicy,
    sitePolicyUpsertResponseSchema,
    'client extra site policy upsert response',
  );

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
  const initialize = await send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-service-config-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  notify('notifications/initialized');

  const mcpProfilesResource = await send('resources/read', { uri: 'agent-browser://profiles' });
  const mcpProfiles = parseMcpJsonResource(
    mcpProfilesResource,
    'agent-browser://profiles',
    'MCP profiles resource',
  );
  assert(
    mcpProfiles.profiles?.some(
      (profile) =>
        profile.id === 'journal-downloader' &&
        profile.allocation === 'per_service' &&
        profile.targetServiceIds?.includes('acs') &&
        profile.authenticatedServiceIds?.includes('acs') &&
        profile.sharedServiceIds?.includes(serviceName),
    ),
    `MCP profiles resource did not include HTTP-upserted profile: ${JSON.stringify(mcpProfiles)}`,
  );

  const genericHttpProfile = await httpJson(
    port,
    'POST',
    `/api/service/profiles/${genericProfileId}`,
    {
      name: 'Generic Journal Login',
      allocation: 'per_service',
      keyring: 'basic_password_store',
      persistent: true,
      targetServiceIds: [genericTargetId],
      authenticatedServiceIds: [genericTargetId],
      sharedServiceIds: [serviceName],
    },
  );
  assert(genericHttpProfile.success === true, `generic HTTP profile upsert failed: ${JSON.stringify(genericHttpProfile)}`);
  assertServiceProfileUpsertResponseSchemaRecord(
    genericHttpProfile.data,
    profileUpsertResponseSchema,
    'generic HTTP profile upsert response',
  );

  const mcpFreshnessResult = await send('tools/call', {
    name: 'service_profile_freshness_update',
    arguments: {
      id: 'journal-downloader',
      loginId: 'acs',
      readinessState: 'stale',
      readinessEvidence: 'auth_probe_cookie_missing',
      lastVerifiedAt: '2026-05-06T14:00:00Z',
      ...traceFields,
    },
  });
  const mcpFreshness = parseMcpToolPayload(
    mcpFreshnessResult,
    'MCP service_profile_freshness_update',
  );
  assert(mcpFreshness.success === true, `MCP profile freshness update failed: ${JSON.stringify(mcpFreshness)}`);
  assertServiceProfileUpsertResponseSchemaRecord(
    mcpFreshness.data,
    profileUpsertResponseSchema,
    'MCP profile freshness update response',
  );
  assert(
    mcpFreshness.data?.profile?.targetReadiness?.[0]?.state === 'stale',
    `MCP profile freshness state mismatch: ${JSON.stringify(mcpFreshness)}`,
  );

  const mcpSessionResult = await send('tools/call', {
    name: 'service_session_upsert',
    arguments: {
      id: 'journal-run',
      session: {
        serviceName,
        agentName,
        taskName,
        profileId: 'journal-downloader',
        lease: 'exclusive',
        cleanup: 'close_browser',
      },
      ...traceFields,
    },
  });
  const mcpSession = parseMcpToolPayload(mcpSessionResult, 'MCP service_session_upsert');
  assert(mcpSession.success === true, `MCP session upsert failed: ${JSON.stringify(mcpSession)}`);
  assertServiceSessionUpsertResponseSchemaRecord(
    mcpSession.data,
    sessionUpsertResponseSchema,
    'MCP session upsert response',
  );
  assert(
    mcpSession.data?.session?.owner?.agent === agentName,
    `MCP session owner was not inferred from agentName: ${JSON.stringify(mcpSession)}`,
  );

  const clientSession = await upsertServiceSession({
    baseUrl: serviceBaseUrl,
    id: 'client-run',
    session: {
      serviceName,
      agentName,
      taskName,
      profileId: 'journal-downloader',
      lease: 'shared',
      cleanup: 'release_only',
    },
  });
  assertServiceSessionUpsertResponseSchemaRecord(
    clientSession,
    sessionUpsertResponseSchema,
    'client session upsert response',
  );
  assert(clientSession.session?.id === 'client-run', `client session id mismatch: ${JSON.stringify(clientSession)}`);

  const httpSessions = await getServiceSessions({ baseUrl: serviceBaseUrl });
  assert(
    httpSessions.sessions?.some(
      (session) =>
        session.id === 'journal-run' &&
        session.profileId === 'journal-downloader' &&
        session.owner?.agent === agentName,
    ),
    `HTTP sessions did not include MCP-upserted session: ${JSON.stringify(httpSessions)}`,
  );

  const mcpPoliciesResource = await send('resources/read', { uri: 'agent-browser://site-policies' });
  const mcpPolicies = parseMcpJsonResource(
    mcpPoliciesResource,
    'agent-browser://site-policies',
    'MCP site-policies resource',
  );
  assert(
    mcpPolicies.sitePolicies?.some(
      (policy) =>
        policy.id === 'google' &&
        policy.originPattern === 'https://accounts.google.com' &&
        policy.interactionMode === 'human_like_input',
    ),
    `MCP site-policies resource did not include HTTP-upserted policy: ${JSON.stringify(mcpPolicies)}`,
  );
  assert(
    mcpPolicies.sitePolicySources?.some(
      (source) =>
        source.id === 'google' &&
        source.source === 'persisted_state' &&
        source.overrideable === false,
    ),
    `MCP site-policies resource did not include persisted source metadata: ${JSON.stringify(mcpPolicies)}`,
  );

  const mcpMonitorResult = await send('tools/call', {
    name: 'service_monitor_upsert',
    arguments: {
      id: 'google-login-freshness',
      monitor: {
        name: 'Google login freshness',
        target: { site_policy: 'google' },
        intervalMs: 60000,
        state: 'paused',
      },
      ...traceFields,
    },
  });
  const mcpMonitor = parseMcpToolPayload(mcpMonitorResult, 'MCP service_monitor_upsert');
  assert(mcpMonitor.success === true, `MCP monitor upsert failed: ${JSON.stringify(mcpMonitor)}`);
  assertServiceMonitorUpsertResponseSchemaRecord(
    mcpMonitor.data,
    monitorUpsertResponseSchema,
    'MCP monitor upsert response',
  );
  const mcpGenericMonitorResult = await send('tools/call', {
    name: 'service_monitor_upsert',
    arguments: {
      id: genericMonitorId,
      monitor: {
        name: 'Generic journal login profile readiness',
        target: { profile_readiness: genericTargetId },
        intervalMs: 900000,
        state: 'active',
      },
      ...traceFields,
    },
  });
  const mcpGenericMonitor = parseMcpToolPayload(
    mcpGenericMonitorResult,
    'MCP generic service_monitor_upsert',
  );
  assert(
    mcpGenericMonitor.success === true,
    `MCP generic monitor upsert failed: ${JSON.stringify(mcpGenericMonitor)}`,
  );
  assertServiceMonitorUpsertResponseSchemaRecord(
    mcpGenericMonitor.data,
    monitorUpsertResponseSchema,
    'MCP generic monitor upsert response',
  );

  const genericAccessPlanQuery =
    `service-name=${encodeURIComponent(serviceName)}` +
    `&agent-name=${encodeURIComponent(agentName)}` +
    `&task-name=${encodeURIComponent(taskName)}` +
    `&login-id=${encodeURIComponent(genericTargetId)}` +
    '&challenge-id=none';
  const genericHttpAccessPlan = await httpJson(
    port,
    'GET',
    `/api/service/access-plan?${genericAccessPlanQuery}`,
  );
  assert(
    genericHttpAccessPlan.success === true,
    `generic HTTP access-plan failed: ${JSON.stringify(genericHttpAccessPlan)}`,
  );
  assertGenericRegistrationAccessPlan(genericHttpAccessPlan.data, 'generic HTTP access-plan');

  const genericMcpAccessPlanUri =
    `agent-browser://access-plan?serviceName=${encodeURIComponent(serviceName)}` +
    `&agentName=${encodeURIComponent(agentName)}` +
    `&taskName=${encodeURIComponent(taskName)}` +
    `&loginId=${encodeURIComponent(genericTargetId)}` +
    '&challengeId=none';
  const genericMcpAccessPlanResource = await send('resources/read', {
    uri: genericMcpAccessPlanUri,
  });
  const genericMcpAccessPlan = parseMcpJsonResource(
    genericMcpAccessPlanResource,
    genericMcpAccessPlanUri,
    'MCP generic access-plan resource',
  );
  assertGenericRegistrationAccessPlan(genericMcpAccessPlan, 'generic MCP access-plan');

  const clientMonitor = await upsertServiceMonitor({
    baseUrl: serviceBaseUrl,
    id: 'client-google-login-freshness',
    monitor: {
      name: 'Client Google login freshness',
      target: { site_policy: 'client-google' },
      intervalMs: 120000,
      state: 'paused',
    },
  });
  assertServiceMonitorUpsertResponseSchemaRecord(
    clientMonitor,
    monitorUpsertResponseSchema,
    'client monitor upsert response',
  );
  const clientProfileReadinessMonitor = await upsertServiceProfileReadinessMonitor({
    baseUrl: serviceBaseUrl,
    serviceName,
    loginId: 'client-target',
    intervalMs: 1800000,
  });
  assertServiceMonitorUpsertResponseSchemaRecord(
    clientProfileReadinessMonitor,
    monitorUpsertResponseSchema,
    'client profile-readiness monitor upsert response',
  );
  assert(
    clientProfileReadinessMonitor.monitor?.id === 'serviceconfigsmoke-client-target-profile-readiness',
    `client profile-readiness monitor id mismatch: ${JSON.stringify(clientProfileReadinessMonitor)}`,
  );
  assert(
    clientProfileReadinessMonitor.monitor?.target?.profile_readiness === 'client-target',
    `client profile-readiness monitor target mismatch: ${JSON.stringify(clientProfileReadinessMonitor)}`,
  );

  const httpMonitors = await getServiceMonitors({ baseUrl: serviceBaseUrl });
  assert(
    httpMonitors.monitors?.some(
      (monitor) =>
        monitor.id === 'google-login-freshness' &&
        monitor.name === 'Google login freshness' &&
        monitor.target?.site_policy === 'google',
    ),
    `HTTP monitors did not include MCP-upserted monitor: ${JSON.stringify(httpMonitors)}`,
  );
  assert(
    httpMonitors.monitors?.some(
      (monitor) =>
        monitor.id === genericMonitorId &&
        monitor.target?.profile_readiness === genericTargetId &&
        monitor.state === 'active',
    ),
    `HTTP monitors did not include MCP-upserted generic profile-readiness monitor: ${JSON.stringify(httpMonitors)}`,
  );
  assert(
    httpMonitors.monitors?.some(
      (monitor) =>
        monitor.id === 'serviceconfigsmoke-client-target-profile-readiness' &&
        monitor.target?.profile_readiness === 'client-target',
    ),
    `HTTP monitors did not include client profile-readiness monitor: ${JSON.stringify(httpMonitors)}`,
  );

  const mcpProviderResult = await send('tools/call', {
    name: 'service_provider_upsert',
    arguments: {
      id: 'manual',
      provider: {
        kind: 'manual_approval',
        displayName: 'Dashboard approval',
        enabled: true,
        capabilities: ['human_approval'],
      },
      ...traceFields,
    },
  });
  const mcpProvider = parseMcpToolPayload(mcpProviderResult, 'MCP service_provider_upsert');
  assert(mcpProvider.success === true, `MCP provider upsert failed: ${JSON.stringify(mcpProvider)}`);
  assertServiceProviderUpsertResponseSchemaRecord(
    mcpProvider.data,
    providerUpsertResponseSchema,
    'MCP provider upsert response',
  );

  const clientProvider = await upsertServiceProvider({
    baseUrl: serviceBaseUrl,
    id: 'client-manual',
    provider: {
      kind: 'manual_approval',
      displayName: 'Client approval',
      enabled: true,
      capabilities: ['human_approval'],
    },
  });
  assertServiceProviderUpsertResponseSchemaRecord(
    clientProvider,
    providerUpsertResponseSchema,
    'client provider upsert response',
  );
  assert(
    clientProvider.provider?.id === 'client-manual',
    `client provider id mismatch: ${JSON.stringify(clientProvider)}`,
  );

  const httpProviders = await getServiceProviders({ baseUrl: serviceBaseUrl });
  assert(
    httpProviders.providers?.some(
      (provider) => provider.id === 'manual' && provider.displayName === 'Dashboard approval',
    ),
    `HTTP providers did not include MCP-upserted provider: ${JSON.stringify(httpProviders)}`,
  );

  const mcpDeleteMonitorResult = await send('tools/call', {
    name: 'service_monitor_delete',
    arguments: { id: 'google-login-freshness', ...traceFields },
  });
  const mcpDeleteMonitor = parseMcpToolPayload(mcpDeleteMonitorResult, 'MCP service_monitor_delete');
  assert(mcpDeleteMonitor.success === true, `MCP monitor delete failed: ${JSON.stringify(mcpDeleteMonitor)}`);
  assertServiceMonitorDeleteResponseSchemaRecord(
    mcpDeleteMonitor.data,
    monitorDeleteResponseSchema,
    'MCP monitor delete response',
  );

  const clientDeleteMonitor = await deleteServiceMonitor({
    baseUrl: serviceBaseUrl,
    id: 'client-google-login-freshness',
  });
  assertServiceMonitorDeleteResponseSchemaRecord(
    clientDeleteMonitor,
    monitorDeleteResponseSchema,
    'client monitor delete response',
  );
  const clientDeleteProfileReadinessMonitor = await deleteServiceMonitor({
    baseUrl: serviceBaseUrl,
    id: 'serviceconfigsmoke-client-target-profile-readiness',
  });
  assertServiceMonitorDeleteResponseSchemaRecord(
    clientDeleteProfileReadinessMonitor,
    monitorDeleteResponseSchema,
    'client profile-readiness monitor delete response',
  );
  const clientDeleteGenericMonitor = await deleteServiceMonitor({
    baseUrl: serviceBaseUrl,
    id: genericMonitorId,
  });
  assertServiceMonitorDeleteResponseSchemaRecord(
    clientDeleteGenericMonitor,
    monitorDeleteResponseSchema,
    'generic profile-readiness monitor delete response',
  );

  const httpMonitorsAfterDelete = await getServiceMonitors({ baseUrl: serviceBaseUrl });
  assertCollectionMissing(httpMonitorsAfterDelete, 'monitors', 'google-login-freshness', 'HTTP monitors');
  assertCollectionMissing(httpMonitorsAfterDelete, 'monitors', 'client-google-login-freshness', 'HTTP monitors');
  assertCollectionMissing(
    httpMonitorsAfterDelete,
    'monitors',
    'serviceconfigsmoke-client-target-profile-readiness',
    'HTTP monitors',
  );
  assertCollectionMissing(httpMonitorsAfterDelete, 'monitors', genericMonitorId, 'HTTP monitors');

  const mcpDeletePolicyResult = await send('tools/call', {
    name: 'service_site_policy_delete',
    arguments: { id: 'google', ...traceFields },
  });
  const mcpDeletePolicy = parseMcpToolPayload(
    mcpDeletePolicyResult,
    'MCP service_site_policy_delete',
  );
  assert(mcpDeletePolicy.success === true, `MCP policy delete failed: ${JSON.stringify(mcpDeletePolicy)}`);
  assertServiceSitePolicyDeleteResponseSchemaRecord(
    mcpDeletePolicy.data,
    sitePolicyDeleteResponseSchema,
    'MCP site policy delete response',
  );

  const clientDeletePolicy = await deleteServiceSitePolicy({
    baseUrl: serviceBaseUrl,
    id: 'client-google',
  });
  assertServiceSitePolicyDeleteResponseSchemaRecord(
    clientDeletePolicy,
    sitePolicyDeleteResponseSchema,
    'client site policy delete response',
  );

  const httpPoliciesAfterDelete = await getServiceSitePolicies({ baseUrl: serviceBaseUrl });
  assertSource(httpPoliciesAfterDelete, 'sitePolicySources', 'google', 'builtin', 'HTTP site-policy sources');
  assertCollectionMissing(httpPoliciesAfterDelete, 'sitePolicies', 'client-google', 'HTTP site-policies');

  const httpDeleteSession = await deleteServiceSession({
    baseUrl: serviceBaseUrl,
    id: 'journal-run',
  });
  assert(
    httpDeleteSession.deleted === true,
    `HTTP session delete failed: ${JSON.stringify(httpDeleteSession)}`,
  );
  assertServiceSessionDeleteResponseSchemaRecord(
    httpDeleteSession,
    sessionDeleteResponseSchema,
    'client session delete response',
  );

  const clientDeleteSession = await deleteServiceSession({
    baseUrl: serviceBaseUrl,
    id: 'client-run',
  });
  assertServiceSessionDeleteResponseSchemaRecord(
    clientDeleteSession,
    sessionDeleteResponseSchema,
    'client extra session delete response',
  );

  const mcpProfilesBeforeProfileDelete = await send('resources/read', { uri: 'agent-browser://profiles' });
  const profilesBeforeProfileDelete = parseMcpJsonResource(
    mcpProfilesBeforeProfileDelete,
    'agent-browser://profiles',
    'MCP profiles before profile delete',
  );
  assert(
    profilesBeforeProfileDelete.profiles?.some((profile) => profile.id === 'journal-downloader'),
    `MCP profiles lost profile before delete: ${JSON.stringify(profilesBeforeProfileDelete)}`,
  );

  const mcpDeleteProfileResult = await send('tools/call', {
    name: 'service_profile_delete',
    arguments: { id: 'journal-downloader', ...traceFields },
  });
  const mcpDeleteProfile = parseMcpToolPayload(
    mcpDeleteProfileResult,
    'MCP service_profile_delete',
  );
  assert(mcpDeleteProfile.success === true, `MCP profile delete failed: ${JSON.stringify(mcpDeleteProfile)}`);
  assertServiceProfileDeleteResponseSchemaRecord(
    mcpDeleteProfile.data,
    profileDeleteResponseSchema,
    'MCP profile delete response',
  );

  const clientDeleteProfile = await deleteServiceProfile({
    baseUrl: serviceBaseUrl,
    id: 'client-profile',
  });
  assertServiceProfileDeleteResponseSchemaRecord(
    clientDeleteProfile,
    profileDeleteResponseSchema,
    'client profile delete response',
  );
  const clientDeleteGenericProfile = await deleteServiceProfile({
    baseUrl: serviceBaseUrl,
    id: genericProfileId,
  });
  assertServiceProfileDeleteResponseSchemaRecord(
    clientDeleteGenericProfile,
    profileDeleteResponseSchema,
    'generic profile delete response',
  );

  const httpProfilesAfterDelete = await getServiceProfiles({ baseUrl: serviceBaseUrl });
  assertCollectionMissing(httpProfilesAfterDelete, 'profiles', 'journal-downloader', 'HTTP profiles');
  assertCollectionMissing(httpProfilesAfterDelete, 'profiles', 'client-profile', 'HTTP profiles');
  assertCollectionMissing(httpProfilesAfterDelete, 'profiles', genericProfileId, 'HTTP profiles');

  const httpDeleteProvider = await deleteServiceProvider({
    baseUrl: serviceBaseUrl,
    id: 'manual',
  });
  assert(
    httpDeleteProvider.deleted === true,
    `HTTP provider delete failed: ${JSON.stringify(httpDeleteProvider)}`,
  );
  assertServiceProviderDeleteResponseSchemaRecord(
    httpDeleteProvider,
    providerDeleteResponseSchema,
    'client provider delete response',
  );

  const clientDeleteProvider = await deleteServiceProvider({
    baseUrl: serviceBaseUrl,
    id: 'client-manual',
  });
  assertServiceProviderDeleteResponseSchemaRecord(
    clientDeleteProvider,
    providerDeleteResponseSchema,
    'client extra provider delete response',
  );

  const mcpProvidersResource = await send('resources/read', { uri: 'agent-browser://providers' });
  const mcpProviders = parseMcpJsonResource(
    mcpProvidersResource,
    'agent-browser://providers',
    'MCP providers resource',
  );
  assertCollectionMissing(mcpProviders, 'providers', 'manual', 'MCP providers');

  await cleanup();
  console.log('Service config HTTP/MCP mutation smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
