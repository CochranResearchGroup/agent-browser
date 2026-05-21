#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamCapabilityLabel,
  viewStreamControlTitle,
  viewStreamOpenTitle,
} from '../packages/dashboard/src/lib/service-view-streams.ts';
import {
  assert,
  closeSession,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

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
const viewStreamProvider = process.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER || 'rdp_gateway';
const controlInputProvider = process.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER || 'manual_attached_desktop';
const viewStreamUrl = process.env.AGENT_BROWSER_REMOTE_VIEW_URL || 'http://agent-browser.localhost/guacamole/';

context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = viewStreamProvider;
context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER = controlInputProvider;
context.env.AGENT_BROWSER_REMOTE_VIEW_URL = viewStreamUrl;

const timeout = setTimeout(() => {
  fail('Timed out waiting for remote view control live smoke to complete');
}, 240000);

let streamPort;
let browserLaunched = false;

function loadAgentBrowserEnvFromRealHome() {
  const realHome = process.env.HOME || '';
  const agentHome = process.env.AGENT_BROWSER_HOME || join(realHome, '.agent-browser');
  const envPath = join(agentHome, '.env');
  if (!existsSync(envPath)) return;

  const text = readFileSync(envPath, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    const value = trimmed.slice(separatorIndex + 1).trim();
    if (!process.env[key]) process.env[key] = value;
  }
}

async function cleanup() {
  clearTimeout(timeout);
  if (streamPort && browserLaunched) {
    try {
      await httpJson(streamPort, 'POST', '/api/service/request', {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: closeTaskName,
        params: { browserId },
        jobTimeoutMs: 30000,
      });
    } catch {
      // closeSession is the final cleanup path for failed launch or shutdown cases.
    }
  }
  await closeSession(context);
  if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
    console.error(`Keeping smoke home: ${context.tempHome}`);
  } else {
    context.cleanupTempHome();
  }
}

async function fail(message) {
  console.error(message);
  await cleanup();
  process.exit(1);
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

function findPrimaryViewStream(browser) {
  return browser?.viewStreams?.find(canOpenViewStream) ?? browser?.viewStreams?.[0] ?? null;
}

try {
  streamPort = await ensureStreamPort();
  const targetUrl = smokeDataUrl('Remote View Control Smoke', 'Remote View Control Smoke');

  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    params: {
      browserHost: 'remote_headed',
      headless: false,
      url: targetUrl,
      waitUntil: 'load',
      viewStreamProvider,
      controlInputProvider,
      viewStreamUrl,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `remote_headed service request failed: ${JSON.stringify(launchResponse)}`);
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
  assert(stream.provider === viewStreamProvider, `View stream provider mismatch: ${JSON.stringify(stream)}`);
  assert(stream.url === viewStreamUrl, `View stream URL mismatch: ${JSON.stringify(stream)}`);
  assert(stream.controlInput === controlInputProvider, `Control input provider mismatch: ${JSON.stringify(stream)}`);
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
    `Service remote view control live smoke passed (${viewStreamCapabilityLabel(stream)} via ${viewStreamUrl})`,
  );
} catch (err) {
  await fail(err.stack || err.message);
}
