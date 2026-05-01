#!/usr/bin/env node

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

try {
  const port = await enableStream();

  const httpProfile = await httpJson(port, 'POST', '/api/service/profiles/journal-downloader', {
    name: 'Journal Downloader',
    allocation: 'per_service',
    keyring: 'basic_password_store',
    persistent: true,
    sharedServiceIds: [serviceName],
  });
  assert(httpProfile.success === true, `HTTP profile upsert failed: ${JSON.stringify(httpProfile)}`);
  assertServiceProfileUpsertResponseSchemaRecord(
    httpProfile.data,
    profileUpsertResponseSchema,
    'HTTP profile upsert response',
  );
  assert(
    httpProfile.data?.profile?.id === 'journal-downloader',
    `HTTP profile id mismatch: ${JSON.stringify(httpProfile)}`,
  );

  const httpPolicy = await httpJson(port, 'POST', '/api/service/site-policies/google', {
    originPattern: 'https://accounts.google.com',
    interactionMode: 'human_like_input',
    challengePolicy: 'avoid_first',
    manualLoginPreferred: true,
    profileRequired: true,
  });
  assert(httpPolicy.success === true, `HTTP site policy upsert failed: ${JSON.stringify(httpPolicy)}`);
  assertServiceSitePolicyUpsertResponseSchemaRecord(
    httpPolicy.data,
    sitePolicyUpsertResponseSchema,
    'HTTP site policy upsert response',
  );
  assert(httpPolicy.data?.sitePolicy?.id === 'google', `HTTP policy id mismatch: ${JSON.stringify(httpPolicy)}`);

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
        profile.sharedServiceIds?.includes(serviceName),
    ),
    `MCP profiles resource did not include HTTP-upserted profile: ${JSON.stringify(mcpProfiles)}`,
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

  const httpSessions = await httpJson(port, 'GET', '/api/service/sessions');
  assert(
    httpSessions.data?.sessions?.some(
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

  const httpProviders = await httpJson(port, 'GET', '/api/service/providers');
  assert(
    httpProviders.data?.providers?.some(
      (provider) => provider.id === 'manual' && provider.displayName === 'Dashboard approval',
    ),
    `HTTP providers did not include MCP-upserted provider: ${JSON.stringify(httpProviders)}`,
  );

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

  const httpPoliciesAfterDelete = await httpJson(port, 'GET', '/api/service/site-policies');
  assertCollectionMissing(httpPoliciesAfterDelete.data, 'sitePolicies', 'google', 'HTTP site-policies');

  const httpDeleteSession = await httpJson(port, 'DELETE', '/api/service/sessions/journal-run');
  assert(
    httpDeleteSession.success === true && httpDeleteSession.data?.deleted === true,
    `HTTP session delete failed: ${JSON.stringify(httpDeleteSession)}`,
  );
  assertServiceSessionDeleteResponseSchemaRecord(
    httpDeleteSession.data,
    sessionDeleteResponseSchema,
    'HTTP session delete response',
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

  const httpProfilesAfterDelete = await httpJson(port, 'GET', '/api/service/profiles');
  assertCollectionMissing(httpProfilesAfterDelete.data, 'profiles', 'journal-downloader', 'HTTP profiles');

  const httpDeleteProvider = await httpJson(port, 'DELETE', '/api/service/providers/manual');
  assert(
    httpDeleteProvider.success === true && httpDeleteProvider.data?.deleted === true,
    `HTTP provider delete failed: ${JSON.stringify(httpDeleteProvider)}`,
  );
  assertServiceProviderDeleteResponseSchemaRecord(
    httpDeleteProvider.data,
    providerDeleteResponseSchema,
    'HTTP provider delete response',
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
