#!/usr/bin/env node

import {
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamCapabilityLabel,
  viewStreamControlTitle,
  viewStreamOpenTitle,
} from '../packages/dashboard/src/lib/service-view-streams.ts';
import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  closeRemoteHeadedBrowser,
  configureRemoteHeadedContext,
  ensureStreamPort,
  launchRemoteHeadedBrowser,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const context = createSmokeContext({
  prefix: 'ab-remote-view-control-',
  sessionPrefix: 'remote-view-control',
});

const { session } = context;
const serviceName = 'RemoteViewControlSmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'remoteHeadedLaunch';
const focusTaskName = 'remoteHeadedControlFocus';
const closeTaskName = 'remoteHeadedClose';
const browserId = `session:${session}`;
const remoteConfig = configureRemoteHeadedContext(context);

const timeout = setTimeout(() => {
  fail('Timed out waiting for remote view control live smoke to complete');
}, 240000);

let streamPort;
let browserLaunched = false;

async function cleanup() {
  clearTimeout(timeout);
  await closeRemoteHeadedBrowser({
    agentName,
    browserId: browserLaunched ? browserId : null,
    context,
    serviceName,
    streamPort,
    taskName: closeTaskName,
  });
  cleanupSmokeHome(context);
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
}

function findPrimaryViewStream(browser) {
  return browser?.viewStreams?.find(canOpenViewStream) ?? browser?.viewStreams?.[0] ?? null;
}

try {
  streamPort = await ensureStreamPort(context);
  await launchRemoteHeadedBrowser({
    agentName,
    config: remoteConfig,
    context,
    heading: 'Remote View Control Smoke',
    serviceName,
    streamPort,
    taskName: launchTaskName,
    title: 'Remote View Control Smoke',
  });
  browserLaunched = true;

  const statusResult = await runCli(context, ['--json', '--session', session, 'service', 'status']);
  const status = parseJsonOutput(statusResult.stdout, 'service status after remote headed launch');
  assert(status.success === true, `service status failed: ${statusResult.stdout}${statusResult.stderr}`);

  const browser = status.data?.service_state?.browsers?.[browserId];
  assert(browser, `Service state missing browser ${browserId}: ${JSON.stringify(status.data)}`);
  assert(browser.host === 'remote_headed', `Browser host mismatch: ${JSON.stringify(browser)}`);
  assert(browser.health === 'ready', `Remote-headed browser is not ready: ${JSON.stringify(browser)}`);

  const stream = findPrimaryViewStream(browser);
  assert(stream, `Remote-headed browser did not record a view stream: ${JSON.stringify(browser)}`);
  assert(stream.provider === remoteConfig.viewStreamProvider, `View stream provider mismatch: ${JSON.stringify(stream)}`);
  assert(stream.url === remoteConfig.viewStreamUrl, `View stream URL mismatch: ${JSON.stringify(stream)}`);
  assert(stream.controlInput === remoteConfig.controlInputProvider, `Control input provider mismatch: ${JSON.stringify(stream)}`);
  assert(canOpenViewStream(stream), `Dashboard would not enable View: ${viewStreamOpenTitle(stream)}`);
  assert(canOpenControlViewStream(stream), `Dashboard would not enable Control: ${viewStreamControlTitle(stream)}`);

  const focusResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'view_focus',
    serviceName,
    agentName,
    taskName: focusTaskName,
    params: {
      index: 0,
      maximize: true,
    },
    jobTimeoutMs: 30000,
  });
  assert(focusResponse.success === true, `view_focus failed: ${JSON.stringify(focusResponse)}`);
  assert(focusResponse.data?.broughtToFront === true, `view_focus did not bring the browser forward: ${JSON.stringify(focusResponse)}`);
  assert(focusResponse.data?.maximizeRequested === true, `view_focus did not request maximize: ${JSON.stringify(focusResponse)}`);

  await cleanup();
  console.log(
    `Service remote view control live smoke passed (${viewStreamCapabilityLabel(stream)} via ${remoteConfig.viewStreamUrl})`,
  );
} catch (err) {
  await fail(err.stack || err.message);
}
