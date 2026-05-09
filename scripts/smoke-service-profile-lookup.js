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
const targetMatchServiceName = 'ProfileLookupTargetOnlySmoke';
const fallbackServiceName = 'ProfileLookupFallbackSmoke';
const manualSeedingServiceName = 'ProfileLookupManualSeedingSmoke';
const targetServiceId = 'acs';
const manualSeedingTargetServiceId = 'google';
const authenticatedProfileId = `profile-lookup-authenticated-${process.pid}`;
const targetOnlyProfileId = `profile-lookup-target-${process.pid}`;
const otherServiceProfileId = `profile-lookup-other-service-${process.pid}`;
const serviceSharedProfileId = `profile-lookup-service-shared-${process.pid}`;
const manualSeedingProfileId = `profile-lookup-google-${process.pid}`;

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
            sharedServiceIds: [serviceName, targetMatchServiceName],
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
          [serviceSharedProfileId]: {
            id: serviceSharedProfileId,
            name: 'Service-shared profile',
            userDataDir: join(tempHome, 'service-shared-profile-user-data'),
            targetServiceIds: [],
            authenticatedServiceIds: [],
            sharedServiceIds: [fallbackServiceName],
            persistent: true,
          },
          [manualSeedingProfileId]: {
            id: manualSeedingProfileId,
            name: 'Manual seeding Google profile',
            userDataDir: join(tempHome, 'manual-seeding-profile-user-data'),
            targetServiceIds: [manualSeedingTargetServiceId],
            authenticatedServiceIds: [],
            sharedServiceIds: [manualSeedingServiceName],
            targetReadiness: [
              {
                targetServiceId: manualSeedingTargetServiceId,
                loginId: manualSeedingTargetServiceId,
                state: 'needs_manual_seeding',
                manualSeedingRequired: true,
                evidence: 'manual_seed_required_without_authenticated_hint',
                recommendedAction: 'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
                seedingMode: 'detached_headed_no_cdp',
                cdpAttachmentAllowedDuringSeeding: false,
                preferredKeyring: 'basic_password_store',
                setupScopes: ['signin', 'chrome_sync', 'passkeys', 'browser_plugins'],
              },
            ],
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
  assert(
    lookup.data?.seedingHandoff === null,
    `profile lookup unexpectedly returned seeding handoff: ${JSON.stringify(lookup.data)}`,
  );

  const targetMatchLookup = await httpJson(
    port,
    'GET',
    `/api/service/profiles/lookup?service-name=${encodeURIComponent(
      targetMatchServiceName,
    )}&login-id=${encodeURIComponent(targetServiceId)}`,
  );
  assert(targetMatchLookup.success === true, `target profile lookup failed: ${JSON.stringify(targetMatchLookup)}`);
  assert(
    targetMatchLookup.data?.selectedProfile?.id === targetOnlyProfileId,
    `target profile lookup did not select target-only profile: ${JSON.stringify(targetMatchLookup.data)}`,
  );
  assert(
    targetMatchLookup.data?.selectedProfileMatch?.reason === 'target_match',
    `target profile lookup did not report target_match: ${JSON.stringify(targetMatchLookup.data)}`,
  );
  assert(
    targetMatchLookup.data?.selectedProfileMatch?.matchedField === 'targetServiceIds',
    `target profile lookup did not report target matched field: ${JSON.stringify(targetMatchLookup.data)}`,
  );
  assert(
    targetMatchLookup.data?.selectedProfileMatch?.matchedIdentity === targetServiceId,
    `target profile lookup did not report target matched identity: ${JSON.stringify(targetMatchLookup.data)}`,
  );

  const fallbackLookup = await httpJson(
    port,
    'GET',
    `/api/service/profiles/lookup?service-name=${encodeURIComponent(fallbackServiceName)}&login-id=unknown`,
  );
  assert(fallbackLookup.success === true, `fallback profile lookup failed: ${JSON.stringify(fallbackLookup)}`);
  assert(
    fallbackLookup.data?.selectedProfile?.id === serviceSharedProfileId,
    `fallback profile lookup did not select service-shared profile: ${JSON.stringify(fallbackLookup.data)}`,
  );
  assert(
    fallbackLookup.data?.selectedProfileMatch?.reason === 'service_allow_list',
    `fallback profile lookup did not report service_allow_list: ${JSON.stringify(fallbackLookup.data)}`,
  );
  assert(
    fallbackLookup.data?.selectedProfileMatch?.matchedField === 'sharedServiceIds',
    `fallback profile lookup did not report service matched field: ${JSON.stringify(fallbackLookup.data)}`,
  );
  assert(
    fallbackLookup.data?.selectedProfileMatch?.matchedIdentity === fallbackServiceName,
    `fallback profile lookup did not report service matched identity: ${JSON.stringify(fallbackLookup.data)}`,
  );

  const manualSeedingLookup = await httpJson(
    port,
    'GET',
    `/api/service/profiles/lookup?service-name=${encodeURIComponent(
      manualSeedingServiceName,
    )}&login-id=${encodeURIComponent(manualSeedingTargetServiceId)}`,
  );
  assert(
    manualSeedingLookup.success === true,
    `manual seeding profile lookup failed: ${JSON.stringify(manualSeedingLookup)}`,
  );
  assert(
    manualSeedingLookup.data?.selectedProfile?.id === manualSeedingProfileId,
    `manual seeding profile lookup did not select Google profile: ${JSON.stringify(manualSeedingLookup.data)}`,
  );
  assert(
    manualSeedingLookup.data?.readinessSummary?.needsManualSeeding === true,
    `manual seeding profile lookup did not report seeding need: ${JSON.stringify(manualSeedingLookup.data)}`,
  );
  assert(
    manualSeedingLookup.data?.seedingHandoff?.profileId === manualSeedingProfileId,
    `manual seeding profile lookup handoff profile mismatch: ${JSON.stringify(manualSeedingLookup.data)}`,
  );
  assert(
    manualSeedingLookup.data?.seedingHandoff?.targetServiceId === manualSeedingTargetServiceId,
    `manual seeding profile lookup handoff target mismatch: ${JSON.stringify(manualSeedingLookup.data)}`,
  );
  assert(
    manualSeedingLookup.data?.seedingHandoff?.seedingMode === 'detached_headed_no_cdp',
    `manual seeding profile lookup handoff mode mismatch: ${JSON.stringify(manualSeedingLookup.data)}`,
  );
  assert(
    manualSeedingLookup.data?.seedingHandoff?.command ===
      `agent-browser --runtime-profile ${manualSeedingProfileId} runtime login https://accounts.google.com`,
    `manual seeding profile lookup handoff command mismatch: ${JSON.stringify(manualSeedingLookup.data)}`,
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
  assert(
    clientLookup.seedingHandoff === null,
    `client lookup unexpectedly returned seeding handoff: ${JSON.stringify(clientLookup)}`,
  );

  const clientManualSeedingLookup = await lookupServiceProfile({
    baseUrl: `http://127.0.0.1:${port}`,
    serviceName: manualSeedingServiceName,
    loginId: manualSeedingTargetServiceId,
  });
  assert(
    clientManualSeedingLookup.selectedProfile?.id === manualSeedingProfileId,
    `client manual seeding lookup did not select Google profile: ${JSON.stringify(clientManualSeedingLookup)}`,
  );
  assert(
    clientManualSeedingLookup.readinessSummary?.needsManualSeeding === true,
    `client manual seeding lookup did not report seeding need: ${JSON.stringify(clientManualSeedingLookup)}`,
  );
  assert(
    clientManualSeedingLookup.seedingHandoff?.profileId === manualSeedingProfileId,
    `client manual seeding lookup handoff profile mismatch: ${JSON.stringify(clientManualSeedingLookup)}`,
  );
  assert(
    clientManualSeedingLookup.seedingHandoff?.command ===
      `agent-browser --runtime-profile ${manualSeedingProfileId} runtime login https://accounts.google.com`,
    `client manual seeding lookup handoff command mismatch: ${JSON.stringify(clientManualSeedingLookup)}`,
  );

  await cleanup();
  console.log('Service profile lookup no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
