#!/usr/bin/env node

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  createMcpStdioClient,
  createSmokeContext,
  parseJsonOutput,
  parseMcpToolPayload,
  readResourceContents,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-profile-target-mcp-',
  sessionPrefix: 'profile-target-mcp',
});

const { agentHome, session, tempHome } = context;
const serviceName = 'ProfileTargetMcpSmoke';
const agentName = 'smoke-agent';
const taskName = 'typedNavigateProfileTargetSmoke';
const targetServiceId = 'acs';
const selectedProfileId = `profile-target-${process.pid}`;
const fallbackProfileId = `profile-fallback-${process.pid}`;
const selectedUserDataDir = join(tempHome, 'selected-profile-user-data');
const fallbackUserDataDir = join(tempHome, 'fallback-profile-user-data');

let mcp;
const timeout = setTimeout(() => {
  fail('Timed out waiting for MCP target-profile smoke to complete');
}, 90000);

function startMcpServer() {
  mcp = createMcpStdioClient({
    context,
    args: ['--session', session, 'mcp', 'serve'],
    onFatal: fail,
  });
}

function seedServiceState() {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        profiles: {
          [fallbackProfileId]: {
            id: fallbackProfileId,
            name: 'Fallback target profile',
            userDataDir: fallbackUserDataDir,
            targetServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
          [selectedProfileId]: {
            id: selectedProfileId,
            name: 'Authenticated target profile',
            userDataDir: selectedUserDataDir,
            targetServiceIds: [targetServiceId],
            authenticatedServiceIds: [targetServiceId],
            sharedServiceIds: [serviceName],
            persistent: true,
          },
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function cleanup() {
  clearTimeout(timeout);
  if (mcp) mcp.close();
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
  if (mcp) mcp.rejectPending(message);
  await cleanup();
  console.error(message);
  const stderr = mcp?.stderr() ?? '';
  if (stderr.trim()) console.error(stderr.trim());
  process.exit(1);
}

try {
  seedServiceState();
  const statusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'service',
    'status',
  ]);
  const status = parseJsonOutput(statusResult.stdout, 'service status');
  assert(
    status.success === true,
    `service status failed: ${statusResult.stdout}${statusResult.stderr}`,
  );

  startMcpServer();

  const initialize = await mcp.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'agent-browser-profile-target-mcp-smoke', version: '0' },
  });
  assert(initialize.capabilities?.tools, 'MCP tools capability missing');
  mcp.notify('notifications/initialized');

  const pageUrl = smokeDataUrl('Profile Target MCP Smoke', 'Profile Target MCP Smoke');
  const navigateResult = await mcp.send('tools/call', {
    name: 'browser_navigate',
    arguments: {
      url: pageUrl,
      waitUntil: 'load',
      serviceName,
      agentName,
      taskName,
      targetServiceId,
    },
  });
  const navigatePayload = parseMcpToolPayload(navigateResult, 'browser_navigate');
  assert(
    navigatePayload.success === true,
    `browser_navigate failed: ${JSON.stringify(navigatePayload)}`,
  );
  assert(navigatePayload.tool === 'browser_navigate', 'browser_navigate payload tool mismatch');
  assert(navigatePayload.trace?.serviceName === serviceName, 'trace missing serviceName');
  assert(
    navigatePayload.trace?.targetServiceId === targetServiceId,
    'trace missing targetServiceId',
  );
  assert(
    navigatePayload.data?.url?.startsWith('data:text/html'),
    `browser_navigate did not report data URL: ${JSON.stringify(navigatePayload.data)}`,
  );

  const profilesResourceResult = await runCli(context, [
    '--json',
    'mcp',
    'read',
    'agent-browser://profiles',
  ]);
  const profilesResource = readResourceContents(
    parseJsonOutput(profilesResourceResult.stdout, 'mcp profiles resource'),
    'profiles',
  );
  const selectedProfile = profilesResource.profiles?.find(
    (profile) => profile.id === selectedProfileId,
  );
  assert(
    selectedProfile,
    `Selected profile missing from MCP profiles: ${JSON.stringify(profilesResource)}`,
  );
  assert(
    selectedProfile.authenticatedServiceIds?.includes(targetServiceId),
    `Selected profile lost authenticated target scope: ${JSON.stringify(selectedProfile)}`,
  );
  assert(
    selectedProfile.userDataDir === selectedUserDataDir,
    `Selected profile userDataDir changed: ${JSON.stringify(selectedProfile)}`,
  );

  const sessionsResourceResult = await runCli(context, [
    '--json',
    'mcp',
    'read',
    'agent-browser://sessions',
  ]);
  const sessionsResource = readResourceContents(
    parseJsonOutput(sessionsResourceResult.stdout, 'mcp sessions resource'),
    'sessions',
  );
  const activeSession = sessionsResource.sessions?.find((item) => item.id === session);
  assert(activeSession, `Active session missing: ${JSON.stringify(sessionsResource)}`);
  assert(
    activeSession.profileId === selectedProfileId,
    `Session selected wrong profile: ${JSON.stringify(activeSession)}`,
  );
  assert(
    activeSession.serviceName === serviceName,
    `Session serviceName was ${activeSession.serviceName}`,
  );
  assert(activeSession.agentName === agentName, `Session agentName was ${activeSession.agentName}`);
  assert(activeSession.taskName === taskName, `Session taskName was ${activeSession.taskName}`);

  const eventsResourceResult = await runCli(context, [
    '--json',
    'mcp',
    'read',
    'agent-browser://events',
  ]);
  const eventsResource = readResourceContents(
    parseJsonOutput(eventsResourceResult.stdout, 'mcp events resource'),
    'events',
  );
  const launchEvent = eventsResource.events?.find(
    (event) =>
      event.kind === 'browser_launch_recorded' &&
      event.sessionId === session &&
      event.profileId === selectedProfileId &&
      event.serviceName === serviceName &&
      event.agentName === agentName &&
      event.taskName === taskName,
  );
  assert(
    launchEvent,
    `Launch event did not record selected target profile: ${JSON.stringify(eventsResource)}`,
  );

  await cleanup();
  console.log('MCP target-profile smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
