#!/usr/bin/env node

import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import { existsSync } from 'node:fs';

const context = createSmokeContext({ prefix: 'ab-sph-', sessionPrefix: 'sph' });
const { session } = context;
const runtimeProfile = `smoke-http-${process.pid}`;
const serviceName = 'RuntimeProfileHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'profileSessionHttpStatusSmoke';
const traceFields = { serviceName, agentName, taskName };

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

async function browserPost(port, endpoint, body = {}) {
  return httpJson(port, 'POST', `/api/browser/${endpoint}`, { ...body, ...traceFields });
}

function assertHttpSuccess(response, label) {
  assert(response.success === true, `${label} failed: ${JSON.stringify(response)}`);
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
    '<input id="disabled-name" value="locked" disabled>',
    '<button id="mark-ready" onclick="document.getElementById(\'click-output\').textContent = \'Clicked\'">Mark ready</button>',
    '<p id="click-output">Not clicked</p>',
    '<button id="hover-target" onmouseover="document.getElementById(\'hover-output\').textContent = \'Hovered\'">Hover target</button>',
    '<p id="hover-output">Not hovered</p>',
    '<p id="press-output"></p>',
    '<label for="choice">Choice</label>',
    '<select id="choice"><option value="a">Alpha</option><option value="b">Beta</option></select>',
    '<label for="remember"><input id="remember" type="checkbox">Remember me</label>',
    '<ul><li class="item">One</li><li class="item">Two</li><li class="item">Three</li></ul>',
    '<div id="box" data-role="sample" style="display: block; width: 200px; height: 100px;">Box target</div>',
    '<div style="height: 1200px;"></div>',
    '<p id="bottom-target">Bottom target</p>',
    '</main>',
    '</body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;
  const secondPageHtml = '<!doctype html><title>Second HTTP Page</title><h1 id="second-ready">Second HTTP Page</h1>';
  const secondPageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(secondPageHtml)}`;
  const tabPageHtml = '<!doctype html><title>HTTP Tab Page</title><h1 id="tab-ready">HTTP Tab Page</h1>';
  const tabPageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(tabPageHtml)}`;

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

  const browserNavigate = await browserPost(port, 'navigate', {
    url: secondPageUrl,
    waitUntil: 'load',
  });
  assertHttpSuccess(browserNavigate, 'HTTP browser navigate');
  assert(
    browserNavigate.data?.url === secondPageUrl,
    `HTTP browser navigate returned ${browserNavigate.data?.url}`,
  );

  const browserBack = await browserPost(port, 'back');
  assertHttpSuccess(browserBack, 'HTTP browser back');
  assert(browserBack.data?.url === pageUrl, `HTTP browser back returned ${browserBack.data?.url}`);

  const browserForward = await browserPost(port, 'forward');
  assertHttpSuccess(browserForward, 'HTTP browser forward');
  assert(
    browserForward.data?.url === secondPageUrl,
    `HTTP browser forward returned ${browserForward.data?.url}`,
  );

  const browserReload = await browserPost(port, 'reload');
  assertHttpSuccess(browserReload, 'HTTP browser reload');
  assert(browserReload.data?.url === secondPageUrl, `HTTP browser reload returned ${browserReload.data?.url}`);

  const browserNavigateHome = await browserPost(port, 'navigate', {
    url: pageUrl,
    waitUntil: 'load',
  });
  assertHttpSuccess(browserNavigateHome, 'HTTP browser navigate home');
  assert(
    browserNavigateHome.data?.url === pageUrl,
    `HTTP browser navigate home returned ${browserNavigateHome.data?.url}`,
  );

  const browserNewTab = await browserPost(port, 'new-tab', {
    url: tabPageUrl,
  });
  assertHttpSuccess(browserNewTab, 'HTTP browser new-tab');
  assert(browserNewTab.data?.url === tabPageUrl, `HTTP browser new-tab returned ${browserNewTab.data?.url}`);
  assert(browserNewTab.data?.index === 1, `HTTP browser new-tab index was ${browserNewTab.data?.index}`);

  const browserSwitchTab = await browserPost(port, 'switch-tab', {
    index: 0,
  });
  assertHttpSuccess(browserSwitchTab, 'HTTP browser switch-tab');
  assert(browserSwitchTab.data?.index === 0, `HTTP browser switch-tab index was ${browserSwitchTab.data?.index}`);
  assert(
    browserSwitchTab.data?.url === pageUrl,
    `HTTP browser switch-tab returned ${browserSwitchTab.data?.url}`,
  );

  const browserCloseTab = await browserPost(port, 'close-tab', {
    index: 1,
  });
  assertHttpSuccess(browserCloseTab, 'HTTP browser close-tab');
  assert(browserCloseTab.data?.closed === 1, `HTTP browser close-tab closed ${browserCloseTab.data?.closed}`);

  const browserSnapshot = await browserPost(port, 'snapshot', {
    selector: '#main',
    interactive: true,
  });
  assertHttpSuccess(browserSnapshot, 'HTTP browser snapshot');
  assert(
    browserSnapshot.data?.snapshot?.includes('Service Profile HTTP Smoke'),
    `HTTP browser snapshot missing page content: ${JSON.stringify(browserSnapshot)}`,
  );

  const screenshotPath = `${context.tempHome}/http-browser-screenshot.png`;
  const browserScreenshot = await browserPost(port, 'screenshot', {
    selector: '#main',
    path: screenshotPath,
  });
  assertHttpSuccess(browserScreenshot, 'HTTP browser screenshot');
  assert(
    browserScreenshot.data?.path === screenshotPath && existsSync(screenshotPath),
    `HTTP browser screenshot did not write expected path: ${JSON.stringify(browserScreenshot)}`,
  );

  const browserFill = await browserPost(port, 'fill', {
    selector: '#name',
    value: 'Ada Lovelace',
  });
  assertHttpSuccess(browserFill, 'HTTP browser fill');
  assert(browserFill.data?.filled === '#name', 'HTTP browser fill did not report filled selector');

  const browserType = await browserPost(port, 'type', {
    selector: '#name',
    text: ' Jr',
    delayMs: 1,
  });
  assertHttpSuccess(browserType, 'HTTP browser type');
  assert(browserType.data?.typed === ' Jr', 'HTTP browser type did not report typed text');

  const browserPress = await browserPost(port, 'press', {
    key: 'Enter',
  });
  assertHttpSuccess(browserPress, 'HTTP browser press');
  assert(browserPress.data?.pressed === 'Enter', 'HTTP browser press did not report pressed key');

  const browserFocus = await browserPost(port, 'focus', {
    selector: '#name',
  });
  assertHttpSuccess(browserFocus, 'HTTP browser focus');
  assert(browserFocus.data?.focused === '#name', 'HTTP browser focus did not report focused selector');

  const browserValue = await browserPost(port, 'get-value', {
    selector: '#name',
  });
  assertHttpSuccess(browserValue, 'HTTP browser get-value');
  assert(
    browserValue.data?.value === 'Ada Lovelace Jr',
    `HTTP browser get-value returned ${browserValue.data?.value}`,
  );

  const browserClear = await browserPost(port, 'clear', {
    selector: '#name',
  });
  assertHttpSuccess(browserClear, 'HTTP browser clear');
  assert(browserClear.data?.cleared === '#name', 'HTTP browser clear did not report cleared selector');

  const browserClearedValue = await browserPost(port, 'get-value', {
    selector: '#name',
  });
  assertHttpSuccess(browserClearedValue, 'HTTP browser cleared get-value');
  assert(
    browserClearedValue.data?.value === '',
    `HTTP browser clear left value ${browserClearedValue.data?.value}`,
  );

  const browserSelect = await browserPost(port, 'select', {
    selector: '#choice',
    values: ['b'],
  });
  assertHttpSuccess(browserSelect, 'HTTP browser select');
  assert(
    browserSelect.data?.selected?.[0] === 'b',
    `HTTP browser select returned ${JSON.stringify(browserSelect.data)}`,
  );

  const browserSelectedValue = await browserPost(port, 'get-value', {
    selector: '#choice',
  });
  assertHttpSuccess(browserSelectedValue, 'HTTP browser selected get-value');
  assert(
    browserSelectedValue.data?.value === 'b',
    `HTTP browser select left value ${browserSelectedValue.data?.value}`,
  );

  const browserHover = await browserPost(port, 'hover', {
    selector: '#hover-target',
  });
  assertHttpSuccess(browserHover, 'HTTP browser hover');
  assert(browserHover.data?.hovered === '#hover-target', 'HTTP browser hover did not report hovered selector');

  const browserHoverText = await browserPost(port, 'get-text', {
    selector: '#hover-output',
  });
  assertHttpSuccess(browserHoverText, 'HTTP browser hover get-text');
  assert(
    browserHoverText.data?.text === 'Hovered',
    `HTTP browser hover did not update page text: ${browserHoverText.data?.text}`,
  );

  const browserClick = await browserPost(port, 'click', {
    selector: '#mark-ready',
  });
  assertHttpSuccess(browserClick, 'HTTP browser click');
  assert(
    browserClick.data?.clicked === '#mark-ready',
    'HTTP browser click did not report clicked selector',
  );

  const browserWait = await browserPost(port, 'wait', {
    selector: '#click-output',
    text: 'Clicked',
    timeoutMs: 2000,
  });
  assertHttpSuccess(browserWait, 'HTTP browser wait');
  assert(browserWait.data?.waited === 'text', 'HTTP browser wait did not report text wait');
  assert(browserWait.data?.text === 'Clicked', 'HTTP browser wait did not report waited text');

  const browserText = await browserPost(port, 'get-text', {
    selector: '#click-output',
  });
  assertHttpSuccess(browserText, 'HTTP browser get-text');
  assert(browserText.data?.text === 'Clicked', `HTTP browser get-text returned ${browserText.data?.text}`);

  const browserVisible = await browserPost(port, 'is-visible', {
    selector: '#ready',
  });
  assertHttpSuccess(browserVisible, 'HTTP browser is-visible');
  assert(browserVisible.data?.visible === true, 'HTTP browser is-visible did not report visible');

  const browserEnabled = await browserPost(port, 'is-enabled', {
    selector: '#name',
  });
  assertHttpSuccess(browserEnabled, 'HTTP browser is-enabled');
  assert(browserEnabled.data?.enabled === true, 'HTTP browser is-enabled did not report enabled');

  const browserDisabled = await browserPost(port, 'is-enabled', {
    selector: '#disabled-name',
  });
  assertHttpSuccess(browserDisabled, 'HTTP browser disabled is-enabled');
  assert(browserDisabled.data?.enabled === false, 'HTTP browser is-enabled did not report disabled');

  const browserAttribute = await browserPost(port, 'get-attribute', {
    selector: '#box',
    attribute: 'data-role',
  });
  assertHttpSuccess(browserAttribute, 'HTTP browser get-attribute');
  assert(
    browserAttribute.data?.value === 'sample',
    `HTTP browser get-attribute returned ${browserAttribute.data?.value}`,
  );

  const browserHtml = await browserPost(port, 'get-html', {
    selector: '#box',
  });
  assertHttpSuccess(browserHtml, 'HTTP browser get-html');
  assert(browserHtml.data?.html === 'Box target', `HTTP browser get-html returned ${browserHtml.data?.html}`);

  const browserStyles = await browserPost(port, 'get-styles', {
    selector: '#box',
    properties: ['display', 'width', 'height'],
  });
  assertHttpSuccess(browserStyles, 'HTTP browser get-styles');
  assert(browserStyles.data?.styles?.display === 'block', 'HTTP browser get-styles did not return display');
  assert(browserStyles.data?.styles?.width === '200px', 'HTTP browser get-styles did not return width');
  assert(browserStyles.data?.styles?.height === '100px', 'HTTP browser get-styles did not return height');

  const browserCount = await browserPost(port, 'count', {
    selector: '.item',
  });
  assertHttpSuccess(browserCount, 'HTTP browser count');
  assert(browserCount.data?.count === 3, `HTTP browser count returned ${browserCount.data?.count}`);

  const browserBox = await browserPost(port, 'get-box', {
    selector: '#box',
  });
  assertHttpSuccess(browserBox, 'HTTP browser get-box');
  assert(browserBox.data?.width === 200, 'HTTP browser get-box did not return expected width');
  assert(browserBox.data?.height === 100, 'HTTP browser get-box did not return expected height');

  const browserCheck = await browserPost(port, 'check', {
    selector: '#remember',
  });
  assertHttpSuccess(browserCheck, 'HTTP browser check');
  assert(browserCheck.data?.checked === '#remember', 'HTTP browser check did not report checked selector');

  const browserChecked = await browserPost(port, 'is-checked', {
    selector: '#remember',
  });
  assertHttpSuccess(browserChecked, 'HTTP browser is-checked');
  assert(browserChecked.data?.checked === true, 'HTTP browser is-checked did not report checked');

  const browserUncheck = await browserPost(port, 'uncheck', {
    selector: '#remember',
  });
  assertHttpSuccess(browserUncheck, 'HTTP browser uncheck');
  assert(
    browserUncheck.data?.unchecked === '#remember',
    'HTTP browser uncheck did not report unchecked selector',
  );

  const browserUnchecked = await browserPost(port, 'is-checked', {
    selector: '#remember',
  });
  assertHttpSuccess(browserUnchecked, 'HTTP browser unchecked is-checked');
  assert(browserUnchecked.data?.checked === false, 'HTTP browser is-checked did not report unchecked');

  const browserScroll = await browserPost(port, 'scroll', {
    direction: 'down',
    amount: 200,
  });
  assertHttpSuccess(browserScroll, 'HTTP browser scroll');
  assert(browserScroll.data?.scrolled === true, 'HTTP browser scroll did not report success');

  const browserScrollIntoView = await browserPost(port, 'scroll-into-view', {
    selector: '#bottom-target',
  });
  assertHttpSuccess(browserScrollIntoView, 'HTTP browser scroll-into-view');
  assert(
    browserScrollIntoView.data?.scrolled === '#bottom-target',
    'HTTP browser scroll-into-view did not report target selector',
  );

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
