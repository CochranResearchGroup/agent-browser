import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import {
  assert,
  closeSession,
  httpJson,
  parseJsonOutput,
  runCli,
  smokeDataUrl,
} from './smoke-utils.js';

function parseEnvValue(value) {
  let parsed = value.trim();
  if (
    (parsed.startsWith('"') && parsed.endsWith('"')) ||
    (parsed.startsWith("'") && parsed.endsWith("'"))
  ) {
    parsed = parsed.slice(1, -1);
  }
  return parsed.replace(/\\"/g, '"');
}

export function loadAgentBrowserEnvFromRealHome() {
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
    const value = parseEnvValue(trimmed.slice(separatorIndex + 1));
    if (!process.env[key]) process.env[key] = value;
  }

  const configPath = join(agentHome, 'config.json');
  if (!process.env.AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH && existsSync(configPath)) {
    const config = JSON.parse(readFileSync(configPath, 'utf8'));
    const manifestPath = config?.service?.browserBuildManifests?.stealthcdp_chromium?.manifestPath;
    if (typeof manifestPath === 'string' && manifestPath.trim()) {
      process.env.AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH = manifestPath.trim();
    }
  }
}

export function configureRemoteHeadedContext(context) {
  const config = {
    viewStreamProvider: process.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER || 'rdp_gateway',
    controlInputProvider: process.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER || 'manual_attached_desktop',
    viewStreamUrl: process.env.AGENT_BROWSER_REMOTE_VIEW_URL || 'http://agent-browser.localhost/guacamole/',
    frameUrl: process.env.AGENT_BROWSER_REMOTE_VIEW_FRAME_URL || process.env.AGENT_BROWSER_REMOTE_VIEW_URL || null,
    externalUrl: process.env.AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL || process.env.AGENT_BROWSER_REMOTE_VIEW_URL || null,
    routeId: process.env.AGENT_BROWSER_REMOTE_VIEW_ROUTE_ID || null,
    connectionId: process.env.AGENT_BROWSER_GUACAMOLE_CONNECTION_ID || null,
    connectionName: process.env.AGENT_BROWSER_GUACAMOLE_CONNECTION_NAME || null,
  };

  context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
  context.env.AGENT_BROWSER_REMOTE_VIEW_PROVIDER = config.viewStreamProvider;
  context.env.AGENT_BROWSER_REMOTE_CONTROL_INPUT_PROVIDER = config.controlInputProvider;
  context.env.AGENT_BROWSER_REMOTE_VIEW_URL = config.viewStreamUrl;
  if (config.frameUrl) context.env.AGENT_BROWSER_REMOTE_VIEW_FRAME_URL = config.frameUrl;
  if (config.externalUrl) context.env.AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL = config.externalUrl;
  if (config.routeId) context.env.AGENT_BROWSER_REMOTE_VIEW_ROUTE_ID = config.routeId;
  if (config.connectionId) context.env.AGENT_BROWSER_GUACAMOLE_CONNECTION_ID = config.connectionId;
  if (config.connectionName) context.env.AGENT_BROWSER_GUACAMOLE_CONNECTION_NAME = config.connectionName;

  return config;
}

export async function ensureStreamPort(context, timeoutMs = 60000) {
  const streamStatusResult = await runCliWithDaemonStartRetry(
    context,
    ['--json', '--session', context.session, 'stream', 'status'],
    timeoutMs,
  );
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(
      context,
      ['--json', '--session', context.session, 'stream', 'enable'],
      timeoutMs,
    );
    stream = parseJsonOutput(streamResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${streamResult.stdout}${streamResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream status did not return a port: ${JSON.stringify(stream)}`);
  return port;
}

async function runCliWithDaemonStartRetry(context, args, timeoutMs) {
  let lastError = null;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      return await runCli(context, args, timeoutMs);
    } catch (err) {
      lastError = err;
      if (!String(err?.message || err).includes('Daemon failed to start') || attempt === 3) {
        throw err;
      }
      await new Promise((resolve) => setTimeout(resolve, 750 * attempt));
    }
  }
  throw lastError;
}

export async function launchRemoteHeadedBrowser({
  agentName,
  config,
  context,
  heading,
  serviceName,
  streamPort,
  taskName,
  title,
}) {
  const launchResponse = await httpJson(streamPort, 'POST', '/api/service/request', {
    action: 'navigate',
    serviceName,
    agentName,
    taskName,
    params: {
      browserHost: 'remote_headed',
      displayIsolation: 'private_virtual_display',
      headless: false,
      url: smokeDataUrl(title, heading),
      waitUntil: 'load',
      viewStreamProvider: config.viewStreamProvider,
      controlInputProvider: config.controlInputProvider,
      viewStreamUrl: config.viewStreamUrl,
      frameUrl: config.frameUrl,
      externalUrl: config.externalUrl,
      routeId: config.routeId || `route:${context.session}`,
      connectionId: config.connectionId,
      connectionName: config.connectionName,
    },
    jobTimeoutMs: 120000,
  });
  assert(launchResponse.success === true, `remote_headed service request failed: ${JSON.stringify(launchResponse)}`);

  const browserId = `session:${context.session}`;
  return { browserId, launchResponse };
}

export async function closeRemoteHeadedBrowser({
  agentName,
  browserId,
  context,
  serviceName,
  streamPort,
  taskName,
}) {
  if (streamPort && browserId) {
    try {
      await httpJson(streamPort, 'POST', '/api/service/request', {
        action: 'service_browser_close',
        serviceName,
        agentName,
        taskName,
        params: { browserId },
        jobTimeoutMs: 30000,
      });
    } catch {
      // closeSession is the final cleanup path for failed launch or shutdown cases.
    }
  }
  await closeSession(context);
}

export function cleanupSmokeHome(context) {
  if (process.env.AGENT_BROWSER_SMOKE_KEEP_HOME === '1') {
    console.error(`Keeping smoke home: ${context.tempHome}`);
  } else {
    context.cleanupTempHome();
  }
}
