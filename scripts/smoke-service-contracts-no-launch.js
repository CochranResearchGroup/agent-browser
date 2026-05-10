#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { getServiceContracts } from '../packages/client/src/service-observability.js';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  httpJsonResult,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-contracts-no-launch-',
  sessionPrefix: 'contracts-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session } = context;

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
  }
}

async function enableStream() {
  const streamStatusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'stream',
    'status',
  ]);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );

  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, [
      '--json',
      '--session',
      session,
      'stream',
      'enable',
    ]);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }

  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

function assertNoBrowserLaunchState() {
  const statePath = join(agentHome, 'service', 'state.json');
  if (!existsSync(statePath)) {
    return;
  }

  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  const jobs = Object.values(state.jobs ?? {});
  assert(
    jobs.every((job) => !['launch', 'navigate', 'tab_new'].includes(job.action)),
    `service contracts recorded browser-launching jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `service contracts persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  const port = await enableStream();
  const contracts = await httpJson(port, 'GET', '/api/service/contracts');
  const clientContracts = await getServiceContracts({
    baseUrl: `http://127.0.0.1:${port}`,
  });
  const status = await httpJson(port, 'GET', '/api/service/status');

  assert(contracts.success === true, `HTTP service contracts failed: ${JSON.stringify(contracts)}`);
  assert(contracts.data?.schemaVersion === 'v1', `contracts schemaVersion mismatch: ${JSON.stringify(contracts)}`);
  assert(
    contracts.data?.contracts?.serviceRequest?.schemaId ===
      'https://agent-browser.local/contracts/service-request.v1.schema.json',
    `serviceRequest schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data?.contracts?.serviceRequest?.http?.route === '/api/service/request',
    `serviceRequest HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data?.contracts?.serviceRequest?.mcp?.tool === 'service_request',
    `serviceRequest MCP tool mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    Array.isArray(contracts.data?.contracts?.serviceRequest?.actions),
    `serviceRequest actions missing: ${JSON.stringify(contracts.data?.contracts?.serviceRequest)}`,
  );
  assert(
    contracts.data.contracts.serviceRequest.actionCount ===
      contracts.data.contracts.serviceRequest.actions.length,
    `serviceRequest action count mismatch: ${JSON.stringify(contracts.data.contracts.serviceRequest)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.schemaId ===
      'https://agent-browser.local/contracts/service-profile-allocation-response.v1.schema.json',
    `serviceProfileAllocationResponse schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.schemaPath ===
      'docs/dev/contracts/service-profile-allocation-response.v1.schema.json',
    `serviceProfileAllocationResponse schema path mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.http?.method === 'GET',
    `serviceProfileAllocationResponse HTTP method mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.http?.route ===
      '/api/service/profiles/<id>/allocation',
    `serviceProfileAllocationResponse HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/allocation',
    `serviceProfileAllocationResponse MCP resource template mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileAllocationResponse?.client?.helpers?.includes(
      'getServiceProfileAllocation',
    ),
    `serviceProfileAllocationResponse client helpers mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.schemaId ===
      'https://agent-browser.local/contracts/service-profile-readiness-response.v1.schema.json',
    `serviceProfileReadinessResponse schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.schemaPath ===
      'docs/dev/contracts/service-profile-readiness-response.v1.schema.json',
    `serviceProfileReadinessResponse schema path mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.http?.method === 'GET',
    `serviceProfileReadinessResponse HTTP method mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.http?.route ===
      '/api/service/profiles/<id>/readiness',
    `serviceProfileReadinessResponse HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/readiness',
    `serviceProfileReadinessResponse MCP resource template mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.client?.package ===
      '@agent-browser/client/service-observability',
    `serviceProfileReadinessResponse client package mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileReadinessResponse?.client?.helpers?.includes(
      'getServiceProfileReadiness',
    ),
    `serviceProfileReadinessResponse client helpers mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.schemaId ===
      'https://agent-browser.local/contracts/service-profile-seeding-handoff-response.v1.schema.json',
    `serviceProfileSeedingHandoffResponse schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.schemaPath ===
      'docs/dev/contracts/service-profile-seeding-handoff-response.v1.schema.json',
    `serviceProfileSeedingHandoffResponse schema path mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.http?.route ===
      '/api/service/profiles/<id>/seeding-handoff',
    `serviceProfileSeedingHandoffResponse HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.mcp?.tool ===
      'service_profile_seeding_handoff_update',
    `serviceProfileSeedingHandoffResponse MCP tool mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
    `serviceProfileSeedingHandoffResponse MCP resource template mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.client?.helpers?.includes(
      'getServiceProfileSeedingHandoff',
    ),
    `serviceProfileSeedingHandoffResponse client helpers mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileSeedingHandoffResponse?.client?.helpers?.includes(
      'updateServiceProfileSeedingHandoff',
    ),
    `serviceProfileSeedingHandoffResponse update helper mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.schemaId ===
      'https://agent-browser.local/contracts/service-profile-lookup-response.v1.schema.json',
    `serviceProfileLookupResponse schema id mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.schemaPath ===
      'docs/dev/contracts/service-profile-lookup-response.v1.schema.json',
    `serviceProfileLookupResponse schema path mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.http?.method === 'GET',
    `serviceProfileLookupResponse HTTP method mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.http?.route ===
      '/api/service/profiles/lookup',
    `serviceProfileLookupResponse HTTP route mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.client?.package ===
      '@agent-browser/client/service-observability',
    `serviceProfileLookupResponse client package mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.contracts?.serviceProfileLookupResponse?.client?.helpers?.includes('lookupServiceProfile'),
    `serviceProfileLookupResponse client helpers mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse?.client?.selectionOrder) ===
      JSON.stringify(['authenticatedServiceIds', 'targetServiceIds', 'sharedServiceIds']),
    `serviceProfileLookupResponse selection order mismatch: ${JSON.stringify(contracts.data?.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileLookupResponse?.client?.helpers?.includes(
      'lookupServiceProfile',
    ),
    `service client could not discover lookupServiceProfile helper: ${JSON.stringify(clientContracts.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileReadinessResponse?.client?.helpers?.includes(
      'getServiceProfileReadiness',
    ),
    `service client could not discover getServiceProfileReadiness helper: ${JSON.stringify(clientContracts.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileReadinessResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/readiness',
    `service client could not discover readiness MCP resource template: ${JSON.stringify(clientContracts.contracts?.serviceProfileReadinessResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileAllocationResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/allocation',
    `service client could not discover allocation MCP resource template: ${JSON.stringify(clientContracts.contracts?.serviceProfileAllocationResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileSeedingHandoffResponse?.client?.helpers?.includes(
      'getServiceProfileSeedingHandoff',
    ),
    `service client could not discover getServiceProfileSeedingHandoff helper: ${JSON.stringify(clientContracts.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileSeedingHandoffResponse?.client?.helpers?.includes(
      'updateServiceProfileSeedingHandoff',
    ),
    `service client could not discover updateServiceProfileSeedingHandoff helper: ${JSON.stringify(clientContracts.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileSeedingHandoffResponse?.mcp?.tool ===
      'service_profile_seeding_handoff_update',
    `service client could not discover seeding handoff MCP tool: ${JSON.stringify(clientContracts.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    clientContracts.contracts?.serviceProfileSeedingHandoffResponse?.mcp?.resourceTemplate ===
      'agent-browser://profiles/{profile_id}/seeding-handoff{?targetServiceId,siteId,loginId}',
    `service client could not discover seeding handoff MCP resource template: ${JSON.stringify(clientContracts.contracts?.serviceProfileSeedingHandoffResponse)}`,
  );
  assert(
    JSON.stringify(clientContracts.contracts?.serviceProfileLookupResponse?.client?.selectionOrder) ===
      JSON.stringify(['authenticatedServiceIds', 'targetServiceIds', 'sharedServiceIds']),
    `service client could not discover profile lookup selection order: ${JSON.stringify(clientContracts.contracts?.serviceProfileLookupResponse)}`,
  );
  assert(
    contracts.data?.http?.serviceProfileAllocationRoute === '/api/service/profiles/<id>/allocation',
    `serviceProfileAllocationRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
  );
  assert(
    contracts.data?.http?.serviceProfileReadinessRoute === '/api/service/profiles/<id>/readiness',
    `serviceProfileReadinessRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
  );
  assert(
    contracts.data?.http?.serviceProfileSeedingHandoffRoute ===
      '/api/service/profiles/<id>/seeding-handoff',
    `serviceProfileSeedingHandoffRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
  );
  assert(
    contracts.data?.http?.serviceProfileLookupRoute === '/api/service/profiles/lookup',
    `serviceProfileLookupRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
  );
  const emptyProfileLookup = await httpJsonResult(
    port,
    'GET',
    '/api/service/profiles/lookup?service-name=JournalDownloader&login-id=acs',
  );
  assert(
    emptyProfileLookup.statusCode === 200,
    `empty profile lookup status mismatch: ${JSON.stringify(emptyProfileLookup)}`,
  );
  assert(
    emptyProfileLookup.body?.success === true,
    `empty profile lookup did not return success envelope: ${JSON.stringify(emptyProfileLookup)}`,
  );
  assert(
    emptyProfileLookup.body?.data?.selectedProfile === null,
    `empty profile lookup selected profile mismatch: ${JSON.stringify(emptyProfileLookup)}`,
  );
  assert(
    emptyProfileLookup.body?.data?.selectedProfileMatch === null,
    `empty profile lookup selected match mismatch: ${JSON.stringify(emptyProfileLookup)}`,
  );
  const missingProfileAllocation = await httpJsonResult(
    port,
    'GET',
    '/api/service/profiles/missing-profile/allocation',
  );
  assert(
    missingProfileAllocation.statusCode === 404,
    `missing profile allocation status mismatch: ${JSON.stringify(missingProfileAllocation)}`,
  );
  assert(
    missingProfileAllocation.body?.success === false,
    `missing profile allocation did not return failure envelope: ${JSON.stringify(missingProfileAllocation)}`,
  );
  assert(
    missingProfileAllocation.body?.error === 'Profile allocation not found: missing-profile',
    `missing profile allocation error mismatch: ${JSON.stringify(missingProfileAllocation)}`,
  );
  const missingProfileReadiness = await httpJsonResult(
    port,
    'GET',
    '/api/service/profiles/missing-profile/readiness',
  );
  assert(
    missingProfileReadiness.statusCode === 404,
    `missing profile readiness status mismatch: ${JSON.stringify(missingProfileReadiness)}`,
  );
  assert(
    missingProfileReadiness.body?.success === false,
    `missing profile readiness did not return failure envelope: ${JSON.stringify(missingProfileReadiness)}`,
  );
  assert(
    missingProfileReadiness.body?.error === 'Profile readiness not found: missing-profile',
    `missing profile readiness error mismatch: ${JSON.stringify(missingProfileReadiness)}`,
  );
  assert(status.success === true, `HTTP service status failed: ${JSON.stringify(status)}`);
  assert(
    Array.isArray(status.data?.profileAllocations),
    `dashboard service status profileAllocations missing: ${JSON.stringify(status.data)}`,
  );
  assert(
    status.data.profileAllocations.length === 0,
    `no-launch service status unexpectedly reported profile allocations: ${JSON.stringify(status.data.profileAllocations)}`,
  );

  assertNoBrowserLaunchState();

  await cleanup();
  console.log('Service contracts no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
