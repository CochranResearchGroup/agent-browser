#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({ prefix: 'ab-sph-', sessionPrefix: 'sph' });
const { session } = context;
const runtimeProfile = `smoke-http-${process.pid}`;
const serviceName = 'RuntimeProfileHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'profileSessionHttpStatusSmoke';

const timeout = setTimeout(() => {
  fail('Timed out waiting for service profile HTTP smoke to complete');
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
    '<head><title>Service Profile HTTP Smoke</title></head>',
    '<body>',
    '<main id="main">',
    '<h1 id="ready">Service Profile HTTP Smoke</h1>',
    '<label for="name">Name</label>',
    '<input id="name" value="stale" onkeydown="if (event.key === \'Enter\') { document.getElementById(\'press-output\').textContent = \'Pressed Enter\'; }">',
    '<button id="mark-ready" onclick="document.getElementById(\'click-output\').textContent = \'Clicked\'">Mark ready</button>',
    '<p id="click-output">Not clicked</p>',
    '<p id="press-output"></p>',
    '</main>',
    '</body>',
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

  const streamStatusResult = await runCli(context, ['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(context, ['--json', '--session', session, 'stream', 'enable']);
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(
      stream.success === true,
      `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`,
    );
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);

  const launchResult = await httpJson(port, 'POST', '/api/command', {
    id: 'service-profile-http-smoke-launch',
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
    `HTTP metadata launch command failed: ${JSON.stringify(launchResult)}`,
  );

  const browserUrl = await httpJson(port, 'GET', '/api/browser/url');
  assert(browserUrl.success === true, `HTTP browser URL failed: ${JSON.stringify(browserUrl)}`);
  assert(browserUrl.data?.url === pageUrl, `HTTP browser URL was ${browserUrl.data?.url}`);

  const browserTitle = await httpJson(port, 'GET', '/api/browser/title');
  assert(
    browserTitle.success === true,
    `HTTP browser title failed: ${JSON.stringify(browserTitle)}`,
  );
  assert(
    browserTitle.data?.title === 'Service Profile HTTP Smoke',
    `HTTP browser title was ${browserTitle.data?.title}`,
  );

  const browserTabs = await httpJson(port, 'GET', '/api/browser/tabs?verbose=true');
  assert(browserTabs.success === true, `HTTP browser tabs failed: ${JSON.stringify(browserTabs)}`);
  assert(
    browserTabs.data?.tabs?.some((tab) => tab.url === pageUrl && tab.active === true),
    `HTTP browser tabs did not include active smoke page: ${JSON.stringify(browserTabs)}`,
  );

  const browserSnapshot = await httpJson(port, 'POST', '/api/browser/snapshot', {
    selector: '#main',
    interactive: true,
    serviceName,
    agentName,
    taskName,
  });
  assert(
    browserSnapshot.success === true,
    `HTTP browser snapshot failed: ${JSON.stringify(browserSnapshot)}`,
  );
  assert(
    browserSnapshot.data?.snapshot?.includes('Service Profile HTTP Smoke'),
    `HTTP browser snapshot missing page content: ${JSON.stringify(browserSnapshot)}`,
  );

  const browserFill = await httpJson(port, 'POST', '/api/browser/fill', {
    selector: '#name',
    value: 'Ada Lovelace',
    serviceName,
    agentName,
    taskName,
  });
  assert(browserFill.success === true, `HTTP browser fill failed: ${JSON.stringify(browserFill)}`);
  assert(browserFill.data?.filled === '#name', 'HTTP browser fill did not report filled selector');

  const browserType = await httpJson(port, 'POST', '/api/browser/type', {
    selector: '#name',
    text: ' Jr',
    delayMs: 1,
    serviceName,
    agentName,
    taskName,
  });
  assert(browserType.success === true, `HTTP browser type failed: ${JSON.stringify(browserType)}`);
  assert(browserType.data?.typed === ' Jr', 'HTTP browser type did not report typed text');

  const browserPress = await httpJson(port, 'POST', '/api/browser/press', {
    key: 'Enter',
    serviceName,
    agentName,
    taskName,
  });
  assert(browserPress.success === true, `HTTP browser press failed: ${JSON.stringify(browserPress)}`);
  assert(browserPress.data?.pressed === 'Enter', 'HTTP browser press did not report pressed key');

  const browserValue = await httpJson(port, 'POST', '/api/browser/get-value', {
    selector: '#name',
    serviceName,
    agentName,
    taskName,
  });
  assert(
    browserValue.success === true,
    `HTTP browser get-value failed: ${JSON.stringify(browserValue)}`,
  );
  assert(
    browserValue.data?.value === 'Ada Lovelace Jr',
    `HTTP browser get-value returned ${browserValue.data?.value}`,
  );

  const browserClick = await httpJson(port, 'POST', '/api/browser/click', {
    selector: '#mark-ready',
    serviceName,
    agentName,
    taskName,
  });
  assert(
    browserClick.success === true,
    `HTTP browser click failed: ${JSON.stringify(browserClick)}`,
  );
  assert(
    browserClick.data?.clicked === '#mark-ready',
    'HTTP browser click did not report clicked selector',
  );

  const browserWait = await httpJson(port, 'POST', '/api/browser/wait', {
    selector: '#click-output',
    text: 'Clicked',
    timeoutMs: 2000,
    serviceName,
    agentName,
    taskName,
  });
  assert(browserWait.success === true, `HTTP browser wait failed: ${JSON.stringify(browserWait)}`);
  assert(browserWait.data?.waited === 'text', 'HTTP browser wait did not report text wait');
  assert(browserWait.data?.text === 'Clicked', 'HTTP browser wait did not report waited text');

  const browserText = await httpJson(port, 'POST', '/api/browser/get-text', {
    selector: '#click-output',
    serviceName,
    agentName,
    taskName,
  });
  assert(
    browserText.success === true,
    `HTTP browser get-text failed: ${JSON.stringify(browserText)}`,
  );
  assert(browserText.data?.text === 'Clicked', `HTTP browser get-text returned ${browserText.data?.text}`);

  const browserVisible = await httpJson(port, 'POST', '/api/browser/is-visible', {
    selector: '#ready',
    serviceName,
    agentName,
    taskName,
  });
  assert(
    browserVisible.success === true,
    `HTTP browser is-visible failed: ${JSON.stringify(browserVisible)}`,
  );
  assert(browserVisible.data?.visible === true, 'HTTP browser is-visible did not report visible');

  const status = await httpJson(port, 'GET', '/api/service/status');
  assert(status.success === true, `HTTP service status failed: ${JSON.stringify(status)}`);
  const serviceState = status.data?.service_state;
  assert(serviceState && typeof serviceState === 'object', 'HTTP service status missing service_state');

  const profileCollection = await httpJson(port, 'GET', '/api/service/profiles');
  assert(
    profileCollection.success === true,
    `HTTP service profiles failed: ${JSON.stringify(profileCollection)}`,
  );
  assert(
    Array.isArray(profileCollection.data?.profiles),
    'HTTP service profiles missing profiles array',
  );

  const profile = Object.values(serviceState.profiles || {}).find(
    (profile) => profile.id === runtimeProfile,
  );
  assert(
    profile,
    `HTTP service status did not include runtime profile ${runtimeProfile}: ${JSON.stringify(
      serviceState.profiles,
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
  assert(
    profileCollection.data.profiles.some((item) => item.id === runtimeProfile),
    `HTTP service profiles did not include runtime profile ${runtimeProfile}: ${JSON.stringify(
      profileCollection,
    )}`,
  );

  const sessionCollection = await httpJson(port, 'GET', '/api/service/sessions');
  assert(
    sessionCollection.success === true,
    `HTTP service sessions failed: ${JSON.stringify(sessionCollection)}`,
  );
  assert(
    Array.isArray(sessionCollection.data?.sessions),
    'HTTP service sessions missing sessions array',
  );
  const persistedSession = Object.values(serviceState.sessions || {}).find(
    (item) => item.id === session,
  );
  assert(
    persistedSession,
    `HTTP service status did not include active session ${session}: ${JSON.stringify(
      serviceState.sessions,
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
  assert(
    sessionCollection.data.sessions.some((item) => item.id === session),
    `HTTP service sessions did not include active session ${session}: ${JSON.stringify(
      sessionCollection,
    )}`,
  );

  for (const [path, key] of [
    ['/api/service/browsers', 'browsers'],
    ['/api/service/tabs', 'tabs'],
    ['/api/service/site-policies', 'sitePolicies'],
    ['/api/service/providers', 'providers'],
    ['/api/service/challenges', 'challenges'],
  ]) {
    const collection = await httpJson(port, 'GET', path);
    assert(collection.success === true, `HTTP ${path} failed: ${JSON.stringify(collection)}`);
    assert(Array.isArray(collection.data?.[key]), `HTTP ${path} missing ${key} array`);
    assert(
      Number.isInteger(collection.data?.count),
      `HTTP ${path} missing numeric count: ${JSON.stringify(collection)}`,
    );
  }

  const events = await httpJson(
    port,
    'GET',
    `/api/service/events?kind=browser_launch_recorded&profile-id=${encodeURIComponent(
      runtimeProfile,
    )}&session-id=${encodeURIComponent(session)}&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(
      taskName,
    )}&limit=20`,
  );
  assert(events.success === true, `HTTP service events failed: ${JSON.stringify(events)}`);
  const launchEvent = events.data?.events?.find(
    (event) =>
      event.sessionId === session &&
      event.profileId === runtimeProfile &&
      event.serviceName === serviceName &&
      event.agentName === agentName &&
      event.taskName === taskName,
  );
  assert(
    launchEvent,
    `HTTP service events missing launch event context: ${JSON.stringify(events)}`,
  );
  assert(launchEvent.serviceName === serviceName, `Event serviceName was ${launchEvent.serviceName}`);
  assert(launchEvent.agentName === agentName, `Event agentName was ${launchEvent.agentName}`);
  assert(launchEvent.taskName === taskName, `Event taskName was ${launchEvent.taskName}`);
  assert(
    launchEvent.browserId === `session:${session}`,
    `Event browserId was ${launchEvent.browserId}`,
  );

  const trace = await httpJson(
    port,
    'GET',
    `/api/service/trace?profile-id=${encodeURIComponent(
      runtimeProfile,
    )}&session-id=${encodeURIComponent(session)}&service-name=${encodeURIComponent(
      serviceName,
    )}&agent-name=${encodeURIComponent(agentName)}&task-name=${encodeURIComponent(
      taskName,
    )}&limit=20`,
  );
  assert(trace.success === true, `HTTP service trace failed: ${JSON.stringify(trace)}`);
  assert(
    trace.data?.filters?.profileId === runtimeProfile,
    `Trace profile filter was ${trace.data?.filters?.profileId}`,
  );
  assert(
    trace.data?.filters?.sessionId === session,
    `Trace session filter was ${trace.data?.filters?.sessionId}`,
  );
  assert(
    trace.data?.filters?.serviceName === serviceName,
    `Trace service filter was ${trace.data?.filters?.serviceName}`,
  );
  assert(
    trace.data?.filters?.agentName === agentName,
    `Trace agent filter was ${trace.data?.filters?.agentName}`,
  );
  assert(
    trace.data?.filters?.taskName === taskName,
    `Trace task filter was ${trace.data?.filters?.taskName}`,
  );
  assert(Array.isArray(trace.data?.events), 'HTTP service trace missing events array');
  assert(Array.isArray(trace.data?.jobs), 'HTTP service trace missing jobs array');
  assert(Array.isArray(trace.data?.incidents), 'HTTP service trace missing incidents array');
  assert(Array.isArray(trace.data?.activity), 'HTTP service trace missing activity array');
  assert(
    trace.data.events.some((event) => event.id === launchEvent.id),
    `HTTP service trace did not include launch event ${launchEvent.id}: ${JSON.stringify(trace)}`,
  );
  assert(
    trace.data.matched?.events >= trace.data.events.length,
    'HTTP service trace matched event count is inconsistent with returned events',
  );
  assert(
    trace.data.counts?.events === trace.data.events.length,
    'HTTP service trace event count does not match returned events',
  );

  await cleanup();
  console.log('Service profile HTTP smoke passed');
} catch (err) {
  await fail(err.stack || err.message);
}
