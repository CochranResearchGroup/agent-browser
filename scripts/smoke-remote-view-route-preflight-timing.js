#!/usr/bin/env node

import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';

import { getServiceRemoteViewRoutePreflight } from '../packages/client/src/service-observability.js';

import {
  assert,
  closeSession,
  createSmokeContext,
  parseJsonOutput,
  runCli,
} from './smoke-utils.js';

const context = createSmokeContext({
  prefix: 'ab-route-preflight-timing-',
  sessionPrefix: 'route-preflight-timing',
});
context.env.AGENT_BROWSER_ARGS = '--no-sandbox';
context.env.AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS = '0';

const { agentHome, session, tempHome } = context;
const fixtureDisplayNumber = '199';
const fixtureDisplaySocketPath = `/tmp/.X11-unix/X${fixtureDisplayNumber}`;
const fixtureAbstractDisplaySocket = `\0/tmp/.X11-unix/X${fixtureDisplayNumber}`;
let createdFixtureDisplaySocket = false;
let fixtureAbstractServer = null;
const maxElapsedMs = Number.parseInt(
  process.env.AGENT_BROWSER_REMOTE_VIEW_PREFLIGHT_TIMING_MAX_MS || '4500',
  10,
);

function routePoolEntry() {
  return {
    id: 'timing-route-a',
    routeId: 'guacamole:timing-a',
    connectionId: 'timing-a',
    connectionName: 'Timing route A',
    frameUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
    externalUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
    routeDescriptor: {
      localEmbedUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
      dashboardEmbedUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
      publicOperatorUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
      healthUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
      externalUrl: 'http://127.0.0.1:8080/guacamole/#/client/timing-a',
      embeddingPolicy: 'local_embed_preferred',
      providerMode: 'simultaneous_view',
    },
    providerMode: 'simultaneous_view',
    state: 'available',
    target: {
      hostname: '127.0.0.1',
      port: '3389',
      displayName: `:${fixtureDisplayNumber}`,
      targetIdentityKey: 'timing-route-a',
    },
    readiness: {
      observedAt: '2999-01-01T00:00:00Z',
      components: [
        ready('guacamole_web'),
        ready('guacamole_login'),
        ready('guacamole_connection_permissions'),
        ready('rdp_backend_tcp'),
      ],
    },
  };
}

async function ensureFixtureDisplaySocket() {
  mkdirSync('/tmp/.X11-unix', { recursive: true });
  if (!existsSync(fixtureDisplaySocketPath)) {
    try {
      writeFileSync(fixtureDisplaySocketPath, '');
      createdFixtureDisplaySocket = true;
    } catch {
      fixtureAbstractServer = createServer((socket) => socket.end());
      await new Promise((resolve, reject) => {
        fixtureAbstractServer.once('error', reject);
        fixtureAbstractServer.listen(fixtureAbstractDisplaySocket, resolve);
      });
      fixtureAbstractServer.off('error', () => {});
    }
  }
}

function ready(component) {
  return {
    component,
    status: 'ready',
    evidence: `${component} retained ready fixture`,
    observedAt: '2999-01-01T00:00:00Z',
    nextAction: 'none',
  };
}

function seedServiceState(entry) {
  const serviceDir = join(agentHome, 'service');
  mkdirSync(serviceDir, { recursive: true });
  writeFileSync(
    join(serviceDir, 'state.json'),
    `${JSON.stringify(
      {
        routePool: {
          [entry.id]: entry,
        },
      },
      null,
      2,
    )}\n`,
  );
}

async function streamBaseUrl() {
  const statusResult = await runCli(context, [
    '--json',
    '--session',
    session,
    'stream',
    'status',
  ]);
  let stream = parseJsonOutput(statusResult.stdout, 'stream status');
  assert(stream.success === true, `stream status failed: ${statusResult.stdout}${statusResult.stderr}`);
  if (!stream.data?.enabled) {
    const enableResult = await runCli(context, [
      '--json',
      '--session',
      session,
      'stream',
      'enable',
    ]);
    stream = parseJsonOutput(enableResult.stdout, 'stream enable');
    assert(stream.success === true, `stream enable failed: ${enableResult.stdout}${enableResult.stderr}`);
  }
  const port = stream.data?.port;
  assert(Number.isInteger(port) && port > 0, `stream did not return a port: ${JSON.stringify(stream)}`);
  return `http://127.0.0.1:${port}`;
}

async function cleanup() {
  if (createdFixtureDisplaySocket) {
    rmSync(fixtureDisplaySocketPath, { force: true });
    createdFixtureDisplaySocket = false;
  }
  if (fixtureAbstractServer) {
    try {
      fixtureAbstractServer.close();
    } catch {
      // Best-effort cleanup; this server only exists to publish an abstract socket fixture.
    }
    fixtureAbstractServer = null;
  }
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

try {
  assert(Number.isInteger(maxElapsedMs) && maxElapsedMs > 0, 'timing max must be a positive integer');
  await ensureFixtureDisplaySocket();
  const entry = routePoolEntry();
  seedServiceState(entry);
  const baseUrl = await streamBaseUrl();

  const start = performance.now();
  const response = await getServiceRemoteViewRoutePreflight({
    baseUrl,
    routePoolEntryId: entry.id,
    routeId: entry.routeId,
    remoteViewRouteId: entry.routeId,
    viewStreamProvider: 'rdp_gateway',
    serviceName: 'RemoteViewPreflightTiming',
    agentName: 'codex',
    taskName: 'fastRoutePreflightTiming',
    jobTimeoutMs: maxElapsedMs,
  });
  const elapsedMs = Math.round(performance.now() - start);

  assert(elapsedMs <= maxElapsedMs, `route preflight exceeded ${maxElapsedMs}ms: ${elapsedMs}ms`);
  assert(response.status === 'preflight_ready', `unexpected preflight status: ${JSON.stringify(response)}`);
  assert(response.fastPreflight?.noLaunch === true, `preflight did not report noLaunch: ${JSON.stringify(response)}`);
  assert(
    ['ready', 'partial', 'blocked', 'stale'].includes(response.fastPreflight?.status),
    `unexpected fastPreflight status: ${JSON.stringify(response.fastPreflight)}`,
  );
  assert(
    response.fastPreflight.components?.some((component) => component.component === 'display_access'),
    `preflight missing display_access component: ${JSON.stringify(response.fastPreflight)}`,
  );
  assert(
    response.fastPreflight.components?.some((component) => component.component === 'route_desktop'),
    `preflight missing route_desktop component: ${JSON.stringify(response.fastPreflight)}`,
  );
  for (const component of [
    'guacamole_web',
    'guacamole_login',
    'guacamole_connection_permissions',
    'rdp_backend_tcp',
  ]) {
    assert(
      response.fastPreflight.components?.some(
        (entry) => entry.component === component && entry.status === 'ready',
      ),
      `preflight missing retained ready ${component}: ${JSON.stringify(response.fastPreflight)}`,
    );
  }

  await cleanup();
  console.log(
    `Remote-view route preflight timing smoke passed in ${elapsedMs}ms with status ${response.fastPreflight.status}`,
  );
} catch (err) {
  await cleanup();
  console.error(err.stack || err.message);
  process.exit(1);
}
