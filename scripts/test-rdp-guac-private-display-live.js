#!/usr/bin/env node

import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  assert,
  createSmokeContext,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';
import {
  cleanupSmokeHome,
  configureRemoteHeadedContext,
  loadAgentBrowserEnvFromRealHome,
} from './smoke-remote-headed-utils.js';

loadAgentBrowserEnvFromRealHome();

const serviceName = 'RdpGuacPrivateDisplaySmoke';
const agentName = 'smoke-agent';
const launchTaskName = 'rdpGuacPrivateDisplayLaunch';
const closeTaskName = 'rdpGuacPrivateDisplayClose';
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const artifactDir = join(tmpdir(), `agent-browser-rdp-guac-private-display-${timestamp}`);

mkdirSync(artifactDir, { recursive: true });

function writeArtifact(name, value) {
  const path = join(artifactDir, name);
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
  return path;
}

async function ensureStreamPortForSession(context, sessionName, timeoutMs = 60000) {
  const statusResult = await runCli(
    context,
    ['--json', '--session', sessionName, 'stream', 'status'],
    timeoutMs,
  );
  let stream = parseJsonOutput(statusResult.stdout, `${sessionName} stream status`);
  assert(
    stream.success === true,
    `${sessionName} stream status failed: ${statusResult.stdout}${statusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const enableResult = await runCli(
      context,
      ['--json', '--session', sessionName, 'stream', 'enable'],
      timeoutMs,
    );
    stream = parseJsonOutput(enableResult.stdout, `${sessionName} stream enable`);
    assert(
      stream.success === true,
      `${sessionName} stream enable failed: ${enableResult.stdout}${enableResult.stderr}`,
    );
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `No stream port for ${sessionName}`);
  return port;
}

async function launchPrivateBrowser(context, remoteConfig, sessionName, title) {
  const streamPort = await ensureStreamPortForSession(context, sessionName, 180000);
  const response = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName: launchTaskName,
    params: {
      browserHost: 'remote_headed',
      displayIsolation: 'private_virtual_display',
      headless: false,
      runtimeProfile: `${sessionName}-profile`,
      url: smokeDataUrl(title, title),
      waitUntil: 'load',
      viewStreamProvider: remoteConfig.viewStreamProvider,
      controlInputProvider: remoteConfig.controlInputProvider,
      viewStreamUrl: remoteConfig.viewStreamUrl,
      frameUrl: remoteConfig.frameUrl,
      externalUrl: remoteConfig.externalUrl,
      routeId: remoteConfig.routeId || `route:${sessionName}`,
      connectionId: remoteConfig.connectionId,
      connectionName: remoteConfig.connectionName,
    },
    jobTimeoutMs: 120000,
  });
  assert(response.success === true, `${title} launch failed: ${JSON.stringify(response)}`);
  return {
    browserId: `session:${sessionName}`,
    launchResponse: response,
    sessionName,
    streamPort,
    title,
  };
}

async function serviceStatus(streamPort, label) {
  const status = await httpJson(streamPort, 'GET', '/api/service/status');
  writeArtifact(`${label}-service-status.json`, status);
  assert(status.success === true, `${label} service status failed: ${JSON.stringify(status)}`);
  return status;
}

function browserRecord(status, workspace) {
  const browser = status.data?.service_state?.browsers?.[workspace.browserId];
  assert(browser, `Missing browser ${workspace.browserId}: ${JSON.stringify(status.data)}`);
  return browser;
}

function allocationRecord(status, allocationId) {
  const allocation = status.data?.service_state?.displayAllocations?.[allocationId];
  assert(allocation, `Missing display allocation ${allocationId}: ${JSON.stringify(status.data)}`);
  return allocation;
}

async function closePrivateBrowser(context, workspace) {
  if (workspace?.streamPort && workspace?.browserId) {
    try {
      await httpJson(workspace.streamPort, 'POST', '/api/service/request', {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName: closeTaskName,
        params: { browserId: workspace.browserId },
        jobTimeoutMs: 30000,
      });
    } catch {
      // The session close below is the final cleanup path after launch failures.
    }
  }
  if (workspace?.sessionName) {
    try {
      await runCli(context, ['--json', '--session', workspace.sessionName, 'close'], 30000);
    } catch {
      // Cleanup is best effort after failed launch or shutdown.
    }
  }
}

const timeout = setTimeout(() => {
  console.error('Timed out waiting for private display live smoke to complete');
  console.error(`Artifacts: ${artifactDir}`);
  process.exit(1);
}, 300000);

const context = createSmokeContext({
  prefix: 'ab-rdp-guac-private-display-',
  sessionPrefix: 'rdp-guac-private-a',
});
context.env.AGENT_BROWSER_ENGINE = 'chrome';
if (!process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE && existsSync('/usr/bin/google-chrome-stable')) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = '/usr/bin/google-chrome-stable';
} else if (process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE) {
  context.env.AGENT_BROWSER_EXECUTABLE_PATH = process.env.AGENT_BROWSER_PRIVATE_DISPLAY_EXECUTABLE;
}
delete context.env.AGENT_BROWSER_CDP;
delete context.env.AGENT_BROWSER_AUTO_CONNECT;
const browserASession = context.session;
const browserBSession = `rdp-guac-private-b-${process.pid}`;
const remoteConfig = configureRemoteHeadedContext(context);

let browserA = null;
let browserB = null;

async function cleanup() {
  clearTimeout(timeout);
  await closePrivateBrowser(context, browserB);
  await closePrivateBrowser(context, browserA);
  cleanupSmokeHome(context);
}

try {
  browserA = await launchPrivateBrowser(
    context,
    remoteConfig,
    browserASession,
    'RDP Guac Private Display A',
  );
  writeArtifact('browser-a-launch-response.json', browserA.launchResponse);

  browserB = await launchPrivateBrowser(
    context,
    remoteConfig,
    browserBSession,
    'RDP Guac Private Display B',
  );
  writeArtifact('browser-b-launch-response.json', browserB.launchResponse);

  const launched = await serviceStatus(browserA.streamPort, 'after-launch');
  const browserARecord = browserRecord(launched, browserA);
  const browserBRecord = browserRecord(launched, browserB);
  assert(browserARecord.health === 'ready', `Browser A not ready: ${JSON.stringify(browserARecord)}`);
  assert(browserBRecord.health === 'ready', `Browser B not ready: ${JSON.stringify(browserBRecord)}`);
  assert(
    browserARecord.displayIsolation === 'private_virtual_display',
    `Browser A display isolation mismatch: ${JSON.stringify(browserARecord)}`,
  );
  assert(
    browserBRecord.displayIsolation === 'private_virtual_display',
    `Browser B display isolation mismatch: ${JSON.stringify(browserBRecord)}`,
  );
  assert(
    browserARecord.displayAllocationId && browserBRecord.displayAllocationId,
    `Browsers did not record display allocation ids: ${JSON.stringify({ browserARecord, browserBRecord })}`,
  );
  assert(
    browserARecord.displayAllocationId !== browserBRecord.displayAllocationId,
    `Private browsers shared display allocation id ${browserARecord.displayAllocationId}`,
  );
  assert(
    browserARecord.displayName && browserBRecord.displayName,
    `Browsers did not record display names: ${JSON.stringify({ browserARecord, browserBRecord })}`,
  );
  assert(
    browserARecord.displayName !== browserBRecord.displayName,
    `Private browsers shared display name ${browserARecord.displayName}`,
  );

  const allocationA = allocationRecord(launched, browserARecord.displayAllocationId);
  const allocationB = allocationRecord(launched, browserBRecord.displayAllocationId);
  assert(allocationA.state === 'ready', `Allocation A not ready: ${JSON.stringify(allocationA)}`);
  assert(allocationB.state === 'ready', `Allocation B not ready: ${JSON.stringify(allocationB)}`);
  writeArtifact('display-allocation-proof.json', {
    browserA: browserARecord,
    browserB: browserBRecord,
    allocationA,
    allocationB,
  });

  await closePrivateBrowser(context, browserA);
  const afterCloseA = await serviceStatus(browserB.streamPort, 'after-close-a');
  const closedA = browserRecord(afterCloseA, browserA);
  const stillReadyB = browserRecord(afterCloseA, browserB);
  const releasedA = allocationRecord(afterCloseA, browserARecord.displayAllocationId);
  const retainedB = allocationRecord(afterCloseA, browserBRecord.displayAllocationId);
  assert(closedA.health === 'not_started', `Browser A not closed: ${JSON.stringify(closedA)}`);
  assert(stillReadyB.health === 'ready', `Browser B did not remain ready: ${JSON.stringify(stillReadyB)}`);
  assert(releasedA.state === 'released', `Allocation A not released: ${JSON.stringify(releasedA)}`);
  assert(retainedB.state === 'ready', `Allocation B was affected by closing A: ${JSON.stringify(retainedB)}`);
  writeArtifact('close-a-release-proof.json', {
    closedA,
    stillReadyB,
    releasedA,
    retainedB,
  });

  await cleanup();
  console.log(`RDP Guacamole private-display live smoke passed; artifacts: ${artifactDir}`);
} catch (err) {
  console.error(err.stack || err.message);
  console.error(`Artifacts: ${artifactDir}`);
  await cleanup();
  process.exit(1);
}
