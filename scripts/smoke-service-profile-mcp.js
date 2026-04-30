#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  readResourceContents,
  runCli,
  sendRawCommand,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sp-', sessionPrefix: 'sp' });
const { session } = context;
const runtimeProfile = `smoke-${process.pid}`;
const serviceName = 'RuntimeProfileSmoke';
const agentName = 'smoke-agent';
const taskName = 'profileSessionResourceSmoke';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service profile MCP smoke to complete');
}, 90000);

async function cleanup() {
  clearTimeout(timeout);
  await closeSession(context);
  context.cleanupTempHome();
}

async function fail(message) {
  await cleanup();
  console.error(message);
  process.exit(1);
}

try {
  const pageHtml = [
    '<!doctype html>',
    '<html>',
    '<head><title>Service Profile MCP Smoke</title></head>',
    '<body><h1 id="ready">Service Profile MCP Smoke</h1></body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

  const openResult = await runCli(context, [
    '--json',
    '--session',
    session,
    '--runtime-profile',
    runtimeProfile,
    '--args',
    '--no-sandbox',
    'open',
    pageUrl,
  ]);
  const opened = parseJsonOutput(openResult.stdout, 'open');
  assert(opened.success === true, `Open command failed: ${openResult.stdout}${openResult.stderr}`);

  const launchResult = await sendRawCommand(context, {
    id: 'service-profile-smoke-launch',
    action: 'launch',
    headless: true,
    runtimeProfile,
    args: ['--no-sandbox'],
    serviceName,
    agentName,
    taskName,
  });
  assert(
    launchResult.success === true,
    `Metadata launch command failed: ${JSON.stringify(launchResult)}`,
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
  const profile = profilesResource.profiles?.find((profile) => profile.id === runtimeProfile);
  assert(
    profile,
    `MCP profiles resource did not include runtime profile ${runtimeProfile}: ${JSON.stringify(
      profilesResource,
    )}`,
  );
  assert(profile.name === runtimeProfile, `Profile name was ${profile.name}`);
  assert(profile.persistent === true, 'Profile was not marked persistent');
  assert(profile.allocation === 'per_service', `Profile allocation was ${profile.allocation}`);
  assert(profile.keyring === 'basic_password_store', `Profile keyring was ${profile.keyring}`);
  assert(
    profile.sharedServiceIds?.includes(serviceName),
    `Profile sharedServiceIds missing ${serviceName}: ${JSON.stringify(profile)}`,
  );
  assert(
    typeof profile.userDataDir === 'string' && profile.userDataDir.includes(runtimeProfile),
    `Profile userDataDir did not include runtime profile name: ${JSON.stringify(profile)}`,
  );

  const cliProfilesResult = await runCli(context, ['--json', 'service', 'profiles']);
  const cliProfiles = parseJsonOutput(cliProfilesResult.stdout, 'service profiles');
  assert(
    cliProfiles.success === true,
    `Service profiles failed: ${cliProfilesResult.stdout}${cliProfilesResult.stderr}`,
  );
  assert(
    cliProfiles.data?.profiles?.some((item) => item.id === runtimeProfile),
    `Service profiles did not include runtime profile ${runtimeProfile}: ${JSON.stringify(cliProfiles.data)}`,
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
  const persistedSession = sessionsResource.sessions?.find((item) => item.id === session);
  assert(
    persistedSession,
    `MCP sessions resource did not include active session ${session}: ${JSON.stringify(
      sessionsResource,
    )}`,
  );
  assert(
    persistedSession.serviceName === serviceName,
    `Session serviceName was ${persistedSession.serviceName}`,
  );
  assert(persistedSession.agentName === agentName, `Session agentName was ${persistedSession.agentName}`);
  assert(persistedSession.taskName === taskName, `Session taskName was ${persistedSession.taskName}`);
  assert(
    persistedSession.profileId === runtimeProfile,
    `Session profileId was ${persistedSession.profileId}`,
  );
  assert(persistedSession.lease === 'exclusive', `Session lease was ${persistedSession.lease}`);
  assert(
    persistedSession.cleanup === 'close_browser',
    `Session cleanup was ${persistedSession.cleanup}`,
  );
  assert(
    persistedSession.browserIds?.includes(`session:${session}`),
    `Session browserIds missing active browser: ${JSON.stringify(persistedSession)}`,
  );

  const cliSessionsResult = await runCli(context, ['--json', 'service', 'sessions']);
  const cliSessions = parseJsonOutput(cliSessionsResult.stdout, 'service sessions');
  assert(
    cliSessions.success === true,
    `Service sessions failed: ${cliSessionsResult.stdout}${cliSessionsResult.stderr}`,
  );
  assert(
    cliSessions.data?.sessions?.some(
      (item) =>
        item.id === session &&
        item.serviceName === serviceName &&
        item.profileId === runtimeProfile,
    ),
    `Service sessions did not include active session metadata: ${JSON.stringify(cliSessions.data)}`,
  );

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
      event.profileId === runtimeProfile &&
      event.serviceName === serviceName &&
      event.agentName === agentName &&
      event.taskName === taskName,
  );
  assert(
    launchEvent,
    `MCP events resource missing launch event context: ${JSON.stringify(eventsResource)}`,
  );
  assert(launchEvent.serviceName === serviceName, `Event serviceName was ${launchEvent.serviceName}`);
  assert(launchEvent.agentName === agentName, `Event agentName was ${launchEvent.agentName}`);
  assert(launchEvent.taskName === taskName, `Event taskName was ${launchEvent.taskName}`);
  assert(
    launchEvent.browserId === `session:${session}`,
    `Event browserId was ${launchEvent.browserId}`,
  );

  const filteredEventsResult = await runCli(context, [
    '--json',
    'service',
    'events',
    '--kind',
    'browser_launch_recorded',
    '--profile-id',
    runtimeProfile,
    '--session-id',
    session,
    '--service-name',
    serviceName,
    '--agent-name',
    agentName,
    '--task-name',
    taskName,
  ]);
  const filteredEvents = parseJsonOutput(filteredEventsResult.stdout, 'filtered service events');
  assert(
    filteredEvents.success === true,
    `Filtered service events failed: ${filteredEventsResult.stdout}${filteredEventsResult.stderr}`,
  );
  assert(
    filteredEvents.data?.events?.some((event) => event.id === launchEvent.id),
    `Filtered service events did not include launch event: ${JSON.stringify(filteredEvents)}`,
  );

  await cleanup();
  console.log('Service profile MCP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
