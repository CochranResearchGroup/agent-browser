#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { request } from 'node:http';
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const tempHome = mkdtempSync(join(tmpdir(), 'ab-sph-'));
const realHome = process.env.HOME;
const cargoHome = process.env.CARGO_HOME || (realHome ? join(realHome, '.cargo') : undefined);
const rustupHome = process.env.RUSTUP_HOME || (realHome ? join(realHome, '.rustup') : undefined);
const session = `sph-${process.pid}`;
const runtimeProfile = `smoke-http-${process.pid}`;
const serviceName = 'RuntimeProfileHttpSmoke';
const agentName = 'smoke-agent';
const taskName = 'profileSessionHttpStatusSmoke';
const agentHome = join(tempHome, '.agent-browser');
const socketDir = join(tempHome, 's');

mkdirSync(socketDir, { recursive: true });

const env = {
  ...process.env,
  HOME: tempHome,
  AGENT_BROWSER_HOME: agentHome,
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  ...(cargoHome ? { CARGO_HOME: cargoHome } : {}),
  ...(rustupHome ? { RUSTUP_HOME: rustupHome } : {}),
};

const timeout = setTimeout(() => {
  fail('Timed out waiting for service profile HTTP smoke to complete');
}, 90000);

function cargoArgs(args) {
  return ['run', '--quiet', '--manifest-path', 'cli/Cargo.toml', '--', ...args];
}

function runCli(args) {
  return new Promise((resolve, reject) => {
    const proc = spawn('cargo', cargoArgs(args), {
      cwd: rootDir,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const procTimeout = setTimeout(() => {
      proc.kill('SIGTERM');
      reject(new Error(`agent-browser ${args.join(' ')} timed out`));
    }, 60000);
    let out = '';
    let err = '';
    proc.stdout.setEncoding('utf8');
    proc.stderr.setEncoding('utf8');
    proc.stdout.on('data', (chunk) => {
      out += chunk;
    });
    proc.stderr.on('data', (chunk) => {
      err += chunk;
    });
    proc.on('error', (err) => {
      clearTimeout(procTimeout);
      reject(err);
    });
    proc.on('exit', (code, signal) => {
      clearTimeout(procTimeout);
      if (code === 0) {
        resolve({ stdout: out, stderr: err });
      } else {
        reject(
          new Error(
            `agent-browser ${args.join(' ')} failed: code=${code} signal=${signal}\n${out}${err}`,
          ),
        );
      }
    });
  });
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function parseJsonOutput(output, label) {
  const text = output.trim();
  try {
    return JSON.parse(text);
  } catch (err) {
    throw new Error(`Failed to parse ${label} JSON output: ${err.message}\n${output}`);
  }
}

function httpJson(port, method, path, body) {
  return new Promise((resolve, reject) => {
    const rawBody = body === undefined ? undefined : JSON.stringify(body);
    const req = request(
      {
        host: '127.0.0.1',
        port,
        method,
        path,
        headers: rawBody
          ? {
              'content-type': 'application/json',
              'content-length': Buffer.byteLength(rawBody),
            }
          : undefined,
      },
      (res) => {
        let text = '';
        res.setEncoding('utf8');
        res.on('data', (chunk) => {
          text += chunk;
        });
        res.on('end', () => {
          let parsed;
          try {
            parsed = JSON.parse(text);
          } catch (err) {
            reject(new Error(`Failed to parse HTTP ${method} ${path}: ${err.message}\n${text}`));
            return;
          }
          if (!res.statusCode || res.statusCode < 200 || res.statusCode >= 300) {
            reject(new Error(`HTTP ${method} ${path} returned ${res.statusCode}: ${text}`));
            return;
          }
          resolve(parsed);
        });
      },
    );
    req.setTimeout(30000, () => {
      req.destroy(new Error(`HTTP ${method} ${path} timed out`));
    });
    req.on('error', reject);
    if (rawBody) req.write(rawBody);
    req.end();
  });
}

async function cleanup() {
  clearTimeout(timeout);
  try {
    await runCli(['--json', '--session', session, 'close']);
  } catch {
    // The smoke may fail before the daemon starts; cleanup must stay best-effort.
  }
  rmSync(tempHome, { recursive: true, force: true });
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
    '<body><h1 id="ready">Service Profile HTTP Smoke</h1></body>',
    '</html>',
  ].join('');
  const pageUrl = `data:text/html;charset=utf-8,${encodeURIComponent(pageHtml)}`;

  const openResult = await runCli([
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

  const streamStatusResult = await runCli(['--json', '--session', session, 'stream', 'status']);
  let stream = parseJsonOutput(streamStatusResult.stdout, 'stream status');
  assert(
    stream.success === true,
    `stream status failed: ${streamStatusResult.stdout}${streamStatusResult.stderr}`,
  );
  if (!stream.data?.enabled) {
    const streamResult = await runCli(['--json', '--session', session, 'stream', 'enable']);
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

  const status = await httpJson(port, 'GET', '/api/service/status');
  assert(status.success === true, `HTTP service status failed: ${JSON.stringify(status)}`);
  const serviceState = status.data?.service_state;
  assert(serviceState && typeof serviceState === 'object', 'HTTP service status missing service_state');

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
