#!/usr/bin/env node

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceTrace,
} from '../packages/client/src/service-observability.js';
import {
  postServiceRequest,
} from '../packages/client/src/service-request.js';
import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-profile-lease-wait-',
  sessionPrefix: 'profile-lease-wait',
});

const { agentHome, session, tempHome } = context;
const serviceName = 'ProfileLeaseWaitSmoke';
const agentName = 'smoke-agent';
const taskName = 'waitForProfileLease';
const profileId = `profile-lease-wait-${process.pid}`;
const holderSessionId = `profile-lease-holder-${process.pid}`;
const statePath = join(agentHome, 'service', 'state.json');

const timeout = setTimeout(() => {
  fail('Timed out waiting for profile lease wait smoke to complete');
}, 90000);

async function cleanup() {
  clearTimeout(timeout);
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
  console.error(message);
  await cleanup();
  process.exit(1);
}

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    statePath,
    `${JSON.stringify(
      {
        profiles: {
          [profileId]: {
            id: profileId,
            name: 'Profile lease wait smoke profile',
            userDataDir: join(tempHome, 'profile-user-data'),
            targetServiceIds: ['acs'],
            authenticatedServiceIds: ['acs'],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
        },
        sessions: {
          [holderSessionId]: {
            id: holderSessionId,
            profileId,
            lease: 'exclusive',
            serviceName: 'LeaseHolder',
            agentName: 'holder-agent',
            taskName: 'holdProfileLease',
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

function releaseHolderLease() {
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  assert(state.sessions?.[holderSessionId], `seeded holder session missing: ${JSON.stringify(state.sessions)}`);
  state.sessions[holderSessionId].lease = 'released';
  writeFileSync(statePath, `${JSON.stringify(state, null, 2)}\n`);
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

try {
  seedServiceState();

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const port = await ensureStreamPort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const response = await postServiceRequest({
    baseUrl,
    request: {
      action: 'tab_list',
      serviceName,
      agentName,
      taskName,
      targetServiceId: 'acs',
      runtimeProfile: profileId,
      profileLeasePolicy: 'wait',
      profileLeaseWaitTimeoutMs: 600,
      jobTimeoutMs: 5_000,
    },
  });
  assert(response.success === false, `profile lease wait request should time out on held lease: ${JSON.stringify(response)}`);
  assert(
    response.error?.includes('timed out') || response.error?.includes('waiting'),
    `profile lease wait response did not explain timeout: ${JSON.stringify(response)}`,
  );
  releaseHolderLease();

  const trace = await getServiceTrace({
    baseUrl,
    query: {
      serviceName,
      agentName,
      taskName,
      limit: 50,
    },
  });
  const waits = trace.summary?.profileLeaseWaits?.waits ?? [];
  assert(trace.summary?.profileLeaseWaits?.count >= 1, `trace missing profile lease wait summary: ${JSON.stringify(trace.summary)}`);
  const waitRecord = waits.find((item) => item.jobId === response.id);
  assert(waitRecord, `trace missing wait record for job ${response.id}: ${JSON.stringify(waits)}`);
  assert(waitRecord.profileId === profileId, `wait record profile mismatch: ${JSON.stringify(waitRecord)}`);
  assert(waitRecord.outcome === 'timed_out', `wait record outcome mismatch: ${JSON.stringify(waitRecord)}`);
  assert(Number.isInteger(waitRecord.waitedMs), `wait record missing waitedMs: ${JSON.stringify(waitRecord)}`);
  assert(
    waitRecord.conflictSessionIds.includes(holderSessionId),
    `wait record missing conflict session: ${JSON.stringify(waitRecord)}`,
  );

  await cleanup();
  console.log('Service profile lease wait live smoke passed');
} catch (err) {
  await fail(err.stack || err.message || String(err));
}
