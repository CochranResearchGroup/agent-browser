#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { lookupServiceProfile } from '../packages/client/src/service-observability.js';

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-profile-lookup-',
  sessionPrefix: 'profile-lookup',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session, tempHome } = context;
const serviceName = 'ProfileLookupSmoke';
const targetServiceId = 'acs';
const authenticatedProfileId = `profile-lookup-authenticated-${process.pid}`;
const targetOnlyProfileId = `profile-lookup-target-${process.pid}`;
const otherServiceProfileId = `profile-lookup-other-service-${process.pid}`;

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
          [targetOnlyProfileId]: {
            id: targetOnlyProfileId,
            name: 'Target-only ACS profile',
            userDataDir: join(tempHome, 'target-profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
          [authenticatedProfileId]: {
            id: authenticatedProfileId,
            name: 'Authenticated ACS profile',
            userDataDir: join(tempHome, 'authenticated-profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
          [otherServiceProfileId]: {
            id: otherServiceProfileId,
            name: 'Other service authenticated ACS profile',
            userDataDir: join(tempHome, 'other-service-profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: ['OtherService'],
            persistent: true,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
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

try {
  seedServiceState();
  const port = await enableStream();
  const lookup = await httpJson(
    port,
    'GET',
    `/api/service/profiles/lookup?service-name=${encodeURIComponent(serviceName)}&login-id=${encodeURIComponent(
      targetServiceId,
    )}`,
  );

  assert(lookup.success === true, `profile lookup failed: ${JSON.stringify(lookup)}`);
  assert(
    lookup.data?.selectedProfile?.id === authenticatedProfileId,
    `profile lookup did not select authenticated profile: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfileMatch?.profileId === authenticatedProfileId,
    `profile lookup selected match id mismatch: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfileMatch?.reason === 'authenticated_target',
    `profile lookup did not report authenticated_target: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfileMatch?.matchedField === 'authenticatedServiceIds',
    `profile lookup did not report authenticated matched field: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfileMatch?.matchedIdentity === targetServiceId,
    `profile lookup did not report authenticated matched identity: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfile?.id !== targetOnlyProfileId,
    `profile lookup selected target-only profile over authenticated profile: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.selectedProfile?.id !== otherServiceProfileId,
    `profile lookup ignored service allow-list: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.readiness?.profileId === authenticatedProfileId,
    `profile lookup readiness did not use selected profile: ${JSON.stringify(lookup.data)}`,
  );
  assert(
    lookup.data?.readinessSummary?.needsManualSeeding === false,
    `profile lookup readiness summary unexpectedly requires manual seeding: ${JSON.stringify(lookup.data)}`,
  );

  const clientLookup = await lookupServiceProfile({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName,
    loginId: targetServiceId,
  });
  assert(
    clientLookup.selectedProfile?.id === authenticatedProfileId,
    `client lookup did not select authenticated profile: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.selectedProfileMatch?.profileId === authenticatedProfileId,
    `client lookup selected match id mismatch: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.selectedProfileMatch?.reason === 'authenticated_target',
    `client lookup did not report authenticated_target: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.selectedProfileMatch?.matchedField === 'authenticatedServiceIds',
    `client lookup did not report authenticated matched field: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.selectedProfileMatch?.matchedIdentity === targetServiceId,
    `client lookup did not report authenticated matched identity: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.readiness?.profileId === authenticatedProfileId,
    `client lookup readiness did not use selected profile: ${JSON.stringify(clientLookup)}`,
  );
  assert(
    clientLookup.readinessSummary?.needsManualSeeding === false,
    `client lookup readiness summary unexpectedly requires manual seeding: ${JSON.stringify(clientLookup)}`,
  );

  await cleanup();
  console.log('Service profile lookup no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
