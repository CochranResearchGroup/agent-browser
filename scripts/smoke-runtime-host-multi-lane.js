#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import { createServer } from 'node:http';
import { createConnection } from 'node:net';
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD
  || join(rootDir, 'cli', 'target', 'debug', 'agent-browser');

if (!process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD) {
  const build = spawnSync(
    join(rootDir, 'scripts', 'ci', 'cargo-safe.sh'),
    ['build', '--manifest-path', 'cli/Cargo.toml'],
    { cwd: rootDir, encoding: 'utf8', stdio: 'inherit' },
  );
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-host-multi-lane-'));
const home = join(fixtureRoot, 'home');
const socketDir = join(fixtureRoot, 'socket');
mkdirSync(home, { recursive: true });
mkdirSync(socketDir, { recursive: true });
const env = {
  ...process.env,
  HOME: home,
  AGENT_BROWSER_HOME: join(home, '.agent-browser'),
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_IDLE_TIMEOUT_MS: '1500',
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
};

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function processIsLive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitFor(predicate, label, timeoutMs = 10000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

function run(session, args, options = {}) {
  return new Promise((resolve, reject) => {
    const profileArgs = options.profile ? ['--profile', options.profile] : [];
    const child = spawn(binary, ['--json', '--session', session, ...profileArgs, ...args], {
      cwd: rootDir,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code !== 0) {
        reject(new Error(`${session} failed: code=${code} signal=${signal}\n${stdout}${stderr}`));
        return;
      }
      try {
        const payload = JSON.parse(stdout.trim());
        assert(payload.success === true, `${session} returned failure: ${stdout}${stderr}`);
        resolve(payload);
      } catch (error) {
        reject(new Error(`${session} returned invalid JSON: ${error.message}\n${stdout}${stderr}`));
      }
    });
  });
}

function sendHostCommand(session, command) {
  const token = readFileSync(join(socketDir, 'runtime-host.token'), 'utf8').trim();
  const payload = JSON.stringify({
    ...command,
    _agentBrowserAuthToken: token,
    _agentBrowserRuntimeLane: session,
  });
  return new Promise((resolve, reject) => {
    const socket = createConnection(join(socketDir, 'runtime-host.sock'));
    let response = '';
    socket.setEncoding('utf8');
    socket.on('connect', () => socket.write(`${payload}\n`));
    socket.on('data', (chunk) => {
      response += chunk;
      const newline = response.indexOf('\n');
      if (newline < 0) return;
      socket.end();
      try {
        resolve(JSON.parse(response.slice(0, newline)));
      } catch (error) {
        reject(new Error(`invalid host response: ${error.message}\n${response}`));
      }
    });
    socket.on('error', reject);
  });
}

let slowRequestSeen;
let releaseSlowRequest;
const slowRequest = new Promise((resolve) => { slowRequestSeen = resolve; });
const slowRelease = new Promise((resolve) => { releaseSlowRequest = resolve; });
const server = createServer(async (request, response) => {
  if (request.url === '/slow') {
    slowRequestSeen();
    await slowRelease;
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end('<title>Slow navigation released</title>');
    return;
  }
  response.writeHead(200, { 'content-type': 'text/html' });
  response.end('<title>Runtime host fixture</title>');
});
await new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(0, '127.0.0.1', resolve);
});
const port = server.address().port;

let hostPid = null;
try {
  await run('alpha', ['open', `http://127.0.0.1:${port}/`], {
    profile: join(fixtureRoot, 'profile-alpha'),
  });
  await Promise.all([
    run('beta', ['stream', 'status']),
    run('beta', ['stream', 'status']),
    run('gamma', ['stream', 'status']),
  ]);

  hostPid = Number.parseInt(readFileSync(join(socketDir, 'runtime-host.pid'), 'utf8'), 10);
  assert(Number.isInteger(hostPid) && processIsLive(hostPid), 'runtime host PID is not live');
  assert(readdirSync(socketDir).filter((name) => name.endsWith('.pid')).length === 1, 'expected one PID file');
  assert(readdirSync(socketDir).filter((name) => name.endsWith('.sock')).length === 1, 'expected one socket');
  assert(readdirSync(socketDir).filter((name) => name.endsWith('.stream')).length === 3, 'expected three lanes');

  const navigation = sendHostCommand('alpha', {
    id: 'p117-slow-navigation',
    action: 'navigate',
    url: `http://127.0.0.1:${port}/slow`,
  });
  await slowRequest;

  const responsivenessStarted = Date.now();
  await Promise.all([
    run('beta', ['stream', 'status']),
    run('gamma', ['stream', 'status']),
  ]);
  assert(Date.now() - responsivenessStarted < 3000, 'a stalled alpha lane blocked unrelated lanes');

  const cancellation = await sendHostCommand('alpha', {
    id: 'p117-cancel-slow-navigation',
    action: 'service_job_cancel',
    jobId: 'p117-slow-navigation',
    reason: 'runtime host stress fixture',
  });
  assert(cancellation.success === true, `cancellation failed: ${JSON.stringify(cancellation)}`);
  assert(cancellation.data?.cancellationRequested === true, 'running cancellation was not requested');
  const cancelled = await navigation;
  assert(cancelled.data?.cancelled === true, `navigation did not cancel: ${JSON.stringify(cancelled)}`);
  releaseSlowRequest();

  const recovered = await run('alpha', ['open', 'data:text/html,<title>Recovered lane</title>']);
  assert(
    recovered.data?.title === 'Recovered lane',
    `cancelled lane did not recover: ${JSON.stringify(recovered)}`,
  );

  await Promise.all([
    run('alpha', ['close']),
    run('beta', ['close']),
    run('gamma', ['close']),
  ]);
  await waitFor(() => !processIsLive(hostPid), 'first runtime host exit');

  await run('beta', ['stream', 'status']);
  const restartedPid = Number.parseInt(readFileSync(join(socketDir, 'runtime-host.pid'), 'utf8'), 10);
  assert(processIsLive(restartedPid) && restartedPid !== hostPid, 'runtime host did not restart cleanly');
  await run('beta', ['close']);
  console.log('Runtime host multi-lane stress smoke passed');
} finally {
  releaseSlowRequest();
  server.close();
  if (hostPid && processIsLive(hostPid)) process.kill(hostPid, 'SIGTERM');
  if (existsSync(fixtureRoot)) rmSync(fixtureRoot, { recursive: true, force: true });
}
