#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

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
    contracts.data?.http?.serviceProfileAllocationRoute === '/api/service/profiles/<id>/allocation',
    `serviceProfileAllocationRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
  );
  assert(
    contracts.data?.http?.serviceProfileReadinessRoute === '/api/service/profiles/<id>/readiness',
    `serviceProfileReadinessRoute mismatch: ${JSON.stringify(contracts.data?.http)}`,
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
