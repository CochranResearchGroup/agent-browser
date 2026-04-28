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

const context = createSmokeContext({ prefix: 'ab-scfg-', sessionPrefix: 'scfg' });
const { session } = context;
const serviceName = 'ServiceConfigSmoke';
const agentName = 'smoke-agent';
const taskName = 'configMutationParity';
const traceFields = { serviceName, agentName, taskName };
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

  const httpPolicy = await httpJson(port, 'POST', '/api/service/site-policies/google', {
    originPattern: 'https://accounts.google.com',
    interactionMode: 'human_like_input',
    challengePolicy: 'avoid_first',
    manualLoginPreferred: true,
    profileRequired: true,
  });
  assert(httpPolicy.success === true, `HTTP site policy upsert failed: ${JSON.stringify(httpPolicy)}`);
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

  const mcpPoliciesResource = await send('resources/read', { uri: 'agent-browser://site-policies' });
  const mcpPolicies = JSON.parse(mcpPoliciesResource.contents?.[0]?.text || '{}');
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

  const httpPoliciesAfterDelete = await httpJson(port, 'GET', '/api/service/site-policies');
  assertCollectionMissing(httpPoliciesAfterDelete.data, 'sitePolicies', 'google', 'HTTP site-policies');

  const httpDeleteProvider = await httpJson(port, 'DELETE', '/api/service/providers/manual');
  assert(
    httpDeleteProvider.success === true && httpDeleteProvider.data?.deleted === true,
    `HTTP provider delete failed: ${JSON.stringify(httpDeleteProvider)}`,
  );

  const mcpProvidersResource = await send('resources/read', { uri: 'agent-browser://providers' });
  const mcpProviders = JSON.parse(mcpProvidersResource.contents?.[0]?.text || '{}');
  assertCollectionMissing(mcpProviders, 'providers', 'manual', 'MCP providers');

  await cleanup();
  console.log('Service config HTTP/MCP mutation smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
