#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createSmokeContext,
  createMcpStdioClient,
  parseJsonOutput,
  readResourceContents,
  runCli,
} from './smoke-utils.js';
import { parseMcpJsonResource } from './smoke-schema-utils.js';

const context = createSmokeContext({
  prefix: 'ab-mcp-read-no-launch-',
  sessionPrefix: 'mcp-read-no-launch',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';

const { agentHome, session } = context;
const profileId = `mcp-read-google-${process.pid}`;
const targetServiceId = 'google';
let mcp;

async function cleanup() {
  try {
    await closeSession(context);
  } finally {
    context.cleanupTempHome();
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
            name: 'MCP read Google profile',
            userDataDir: join(context.tempHome, 'google-profile-user-data'),
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [],
            sharedServiceIds: ['McpReadSmoke'],
            targetReadiness: [
              {
                targetServiceId,
                loginId: targetServiceId,
                state: 'needs_manual_seeding',
                manualSeedingRequired: true,
                evidence: 'manual_seed_required_without_authenticated_hint',
                recommendedAction:
                  'launch_detached_runtime_login_complete_signin_close_then_relaunch_attachable',
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

function assertNoLaunchSideEffects(statePath) {
  if (!existsSync(statePath)) return;
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  assert(
    Object.keys(state.jobs ?? {}).length === 0,
    `mcp read persisted jobs: ${JSON.stringify(state.jobs)}`,
  );
  assert(
    Object.keys(state.browsers ?? {}).length === 0,
    `mcp read persisted browsers: ${JSON.stringify(state.browsers)}`,
  );
}

try {
  seedServiceState();

  const sessionsResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'mcp',
    'read',
    'agent-browser://sessions',
  ]);
  const sessions = readResourceContents(
    parseJsonOutput(sessionsResult.stdout, 'mcp sessions resource'),
    'sessions',
  );

  assert(
    Array.isArray(sessions.sessions),
    `invalid sessions resource: ${sessionsResult.stdout}`,
  );
  assert(sessions.count === 0, `mcp read returned unexpected sessions: ${sessionsResult.stdout}`);

  const statePath = join(agentHome, 'service', 'state.json');
  assertNoLaunchSideEffects(statePath);

  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: (message, stderr) => {
      console.error(message);
      if (stderr.trim()) {
        console.error(stderr.trim());
      }
    },
  });
  try {
    const initialize = await mcp.send('initialize', {
      protocolVersion: '2025-06-18',
      capabilities: {},
      clientInfo: { name: 'agent-browser-mcp-read-no-launch-smoke', version: '0' },
    });
    assert(initialize.capabilities?.resources, 'MCP resources capability missing');
    mcp.notify('notifications/initialized');

    const readinessUri = `agent-browser://profiles/${profileId}/readiness`;
    const readiness = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: readinessUri }),
      readinessUri,
      'MCP profile readiness resource',
    );

    assert(readiness.profileId === profileId, `readiness profile mismatch: ${JSON.stringify(readiness)}`);
    assert(readiness.count === 1, `readiness count mismatch: ${JSON.stringify(readiness)}`);
    assert(
      readiness.targetReadiness?.[0]?.targetServiceId === targetServiceId,
      `readiness target mismatch: ${JSON.stringify(readiness)}`,
    );
    assert(
      readiness.targetReadiness?.[0]?.state === 'needs_manual_seeding',
      `readiness state mismatch: ${JSON.stringify(readiness)}`,
    );

    const allocationUri = `agent-browser://profiles/${profileId}/allocation`;
    const allocation = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: allocationUri }),
      allocationUri,
      'MCP profile allocation resource',
    );

    assert(allocation.profileId === profileId, `allocation profile mismatch: ${JSON.stringify(allocation)}`);
    assert(
      allocation.profileAllocation?.profileId === profileId,
      `allocation row profile mismatch: ${JSON.stringify(allocation)}`,
    );
    assert(
      allocation.profileAllocation?.targetReadiness?.[0]?.state === 'needs_manual_seeding',
      `allocation readiness mismatch: ${JSON.stringify(allocation)}`,
    );

    const lookupUri = `agent-browser://profiles/lookup?serviceName=McpReadSmoke&loginId=${targetServiceId}`;
    const lookup = parseMcpJsonResource(
      await mcp.send('resources/read', { uri: lookupUri }),
      lookupUri,
      'MCP profile lookup resource',
    );

    assert(
      lookup.selectedProfile?.id === profileId,
      `lookup selected profile mismatch: ${JSON.stringify(lookup)}`,
    );
    assert(
      lookup.selectedProfileMatch?.reason === 'target_match',
      `lookup match reason mismatch: ${JSON.stringify(lookup)}`,
    );
    assert(
      lookup.readiness?.profileId === profileId,
      `lookup readiness profile mismatch: ${JSON.stringify(lookup)}`,
    );

    const uri = `agent-browser://profiles/${profileId}/seeding-handoff?targetServiceId=${targetServiceId}`;
    const handoff = parseMcpJsonResource(
      await mcp.send('resources/read', { uri }),
      uri,
      'MCP profile seeding handoff resource',
    );

    assert(handoff.profileId === profileId, `handoff profile mismatch: ${JSON.stringify(handoff)}`);
    assert(
      handoff.targetServiceId === targetServiceId,
      `handoff target mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.command === `agent-browser --runtime-profile ${profileId} runtime login https://accounts.google.com`,
      `handoff command mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.lifecycle?.state === 'needs_manual_seeding',
      `handoff lifecycle mismatch: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.operatorIntervention?.defaultChannels?.includes('mcp'),
      `handoff intervention missing MCP channel: ${JSON.stringify(handoff)}`,
    );
    assert(
      handoff.operatorIntervention?.blocksProfileLease === true,
      `handoff intervention should block profile lease: ${JSON.stringify(handoff)}`,
    );
  } finally {
    mcp.close();
    mcp = null;
  }

  assertNoLaunchSideEffects(statePath);

  await cleanup();
  console.log('MCP read no-launch smoke passed');
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
