#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  getServiceJobs,
  getServiceStatus,
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
  prefix: 'ab-service-request-lease-reject-',
  sessionPrefix: 'service-request-lease-reject',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session, tempHome } = context;
const serviceName = 'ServiceRequestLeaseRejectSmoke';
const agentName = 'smoke-agent';
const taskName = 'rejectHeldProfileLease';
const targetServiceId = 'acs';
const profileId = `service-request-lease-reject-${process.pid}`;
const holderSessionId = `service-request-lease-holder-${process.pid}`;

async function cleanup() {
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

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [profileId]: {
            id: profileId,
            name: 'Service request lease reject profile',
            userDataDir: join(tempHome, 'profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
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
  assert(Number.isInteger(port) && port > 0, `stream enable did not return a port: ${JSON.stringify(stream)}`);
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
      targetServiceId,
      profileLeasePolicy: 'reject',
      jobTimeoutMs: 5_000,
    },
  });

  assert(response.success === false, `lease reject request should fail: ${JSON.stringify(response)}`);
  assert(
    response.error?.includes('Service profile lease conflict') &&
      response.error.includes(profileId) &&
      response.error.includes(holderSessionId),
    `lease reject response did not report profile conflict: ${JSON.stringify(response)}`,
  );

  const jobs = await getServiceJobs({
    baseUrl,
    query: { serviceName, agentName, taskName, limit: 20 },
  });
  const job = jobs.jobs?.find((item) => item.id === response.id);
  assert(job, `lease reject job missing from service jobs: ${JSON.stringify(jobs)}`);
  assert(job.state === 'failed', `lease reject job state mismatch: ${JSON.stringify(job)}`);
  assert(job.action === 'tab_list', `lease reject job action mismatch: ${JSON.stringify(job)}`);
  assert(job.targetServiceId === targetServiceId, `lease reject job target mismatch: ${JSON.stringify(job)}`);
  assert(
    Array.isArray(job.targetServiceIds) && job.targetServiceIds.includes(targetServiceId),
    `lease reject job missing normalized target IDs: ${JSON.stringify(job)}`,
  );
  assert(
    typeof job.error === 'string' && job.error.includes('Service profile lease conflict'),
    `lease reject job error mismatch: ${JSON.stringify(job)}`,
  );

  const serviceStatus = await getServiceStatus({ baseUrl });
  assert(
    (serviceStatus.browsers ? Object.keys(serviceStatus.browsers).length : 0) === 0,
    `lease reject smoke should not launch browsers: ${JSON.stringify(serviceStatus.browsers)}`,
  );

  await cleanup();
  console.log('Service request lease reject no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
