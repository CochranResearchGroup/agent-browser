#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { createServer } from 'node:net';
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

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-host-supervisor-'));
const home = join(fixtureRoot, 'home');
const socketDir = join(fixtureRoot, 'socket');
const supervisorRoot = join(fixtureRoot, 'supervisor');
const manifestDir = join(supervisorRoot, 'manifests');
mkdirSync(home, { recursive: true });
// Reboot removes volatile socket directories before systemd starts the host.
mkdirSync(manifestDir, { recursive: true });

const env = {
  ...process.env,
  HOME: home,
  AGENT_BROWSER_HOME: join(home, '.agent-browser'),
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SESSION_SUPERVISOR_ROOT: supervisorRoot,
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
  AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'disabled',
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
};
const executableSha256 = createHash('sha256').update(readFileSync(binary)).digest('hex');
const ingressPath = join(home, '.agent-browser', 'runtime-host-ingress.json');

async function availableLoopbackPort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  assert(port > 0, 'could not allocate a loopback fixture port');
  return port;
}

const alphaPort = await availableLoopbackPort();
const betaPort = await availableLoopbackPort();

if (process.platform === 'linux') {
  mkdirSync(join(home, '.agent-browser'), { recursive: true });
  const bootId = readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim();
  writeFileSync(ingressPath, `${JSON.stringify({
    schemaVersion: 'agent-browser.runtime-host-ingress.v1',
    revision: 1,
    bootEpoch: `linux:${bootId}`,
    activeTransactionId: null,
    selectedBackend: {
      topology: 'single_host',
      generationId: 'supervisor-restart-fixture',
      socketDir: join(fixtureRoot, 'stale-selected-socket'),
      binarySha256: executableSha256,
      hostId: 'runtime-host:dead-selected',
      pid: 4294967295,
      socketIdentity: 'unix:dead:selected',
    },
    candidateBackend: null,
    fallbackBackend: null,
  }, null, 2)}\n`, { mode: 0o600 });
}

function manifest(session, streamPort) {
  return {
    schemaVersion: 'agent-browser.session-supervisor.v1',
    session,
    executablePath: binary,
    executableSha256,
    streamPort,
    runtimeProfile: session,
    provenance: {
      packageVersion: '0.28.0',
      installedAt: '2026-08-20T00:00:00Z',
      installedBy: 'runtime-host supervisor no-launch fixture',
    },
  };
}

for (const [session, port] of [['alpha', alphaPort], ['beta', betaPort]]) {
  writeFileSync(
    join(manifestDir, `${session}.json`),
    `${JSON.stringify(manifest(session, port), null, 2)}\n`,
    { mode: 0o600 },
  );
}

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

async function waitFor(predicate, label, timeoutMs = 30000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

function startHost() {
  const child = spawn(binary, ['session', 'supervisor', 'run-host'], {
    cwd: rootDir,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let output = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stdout.on('data', (chunk) => { output += chunk; });
  child.stderr.on('data', (chunk) => { output += chunk; });
  return { child, output: () => output };
}

async function stopHost(host) {
  if (host.child.exitCode !== null) return;
  host.child.kill('SIGTERM');
  await new Promise((resolve) => host.child.once('exit', resolve));
}

function assertHostTopology(expectedPid) {
  if (process.platform !== 'win32') {
    assert((statSync(socketDir).mode & 0o777) === 0o700,
      'supervisor socket directory must be private');
    assert((statSync(join(socketDir, 'runtime-host.token')).mode & 0o777) === 0o600,
      'supervisor token must be private');
  }
  const pidFiles = readdirSync(socketDir).filter((name) => name.endsWith('.pid'));
  const sockets = readdirSync(socketDir).filter((name) => name.endsWith('.sock'));
  assert(pidFiles.length === 1 && pidFiles[0] === 'runtime-host.pid', `unexpected PID files: ${pidFiles}`);
  assert(sockets.length === 1 && sockets[0] === 'runtime-host.sock', `unexpected sockets: ${sockets}`);
  assert(readFileSync(join(socketDir, 'alpha.stream'), 'utf8').trim() === String(alphaPort),
    'alpha fixed port drifted');
  assert(readFileSync(join(socketDir, 'beta.stream'), 'utf8').trim() === String(betaPort),
    'beta fixed port drifted');
  const pid = Number.parseInt(readFileSync(join(socketDir, 'runtime-host.pid'), 'utf8'), 10);
  assert(Number.isInteger(pid) && processIsLive(pid), 'runtime host PID is not live');
  if (expectedPid !== undefined) assert(pid !== expectedPid, 'restarted host reused the old PID');
  if (process.platform === 'linux') {
    const ingress = JSON.parse(readFileSync(ingressPath, 'utf8'));
    assert(ingress.selectedBackend.pid === pid, 'ingress did not select the restarted host PID');
    assert(ingress.selectedBackend.socketDir === socketDir,
      'ingress did not select the restarted host socket directory');
    assert(ingress.selectedBackend.binarySha256 === executableSha256,
      'ingress changed the selected binary identity');
  }
  return pid;
}

async function assertDashboardDiscovery() {
  const response = await fetch(`http://127.0.0.1:${alphaPort}/api/sessions`);
  assert(response.ok, `dashboard discovery returned HTTP ${response.status}`);
  const sessions = await response.json();
  const names = sessions.map((session) => session.session).sort();
  assert(names.includes('alpha') && names.includes('beta'), `dashboard discovery omitted host lanes: ${names}`);
}

let host = startHost();
try {
  await waitFor(
    () => {
      assert(host.child.exitCode === null, `host exited before readiness: ${host.output()}`);
      return existsSync(join(socketDir, 'alpha.stream')) && existsSync(join(socketDir, 'beta.stream'));
    },
    'supervised lanes',
  );
  const firstPid = assertHostTopology();
  await assertDashboardDiscovery();
  await stopHost(host);
  await waitFor(() => !processIsLive(firstPid), 'first host exit');
  rmSync(socketDir, { recursive: true });

  host = startHost();
  await waitFor(
    () => existsSync(join(socketDir, 'alpha.stream')) && existsSync(join(socketDir, 'beta.stream')),
    'restarted supervised lanes',
  );
  assertHostTopology(firstPid);
  await assertDashboardDiscovery();
  console.log('Runtime host supervisor no-launch smoke passed');
} catch (error) {
  const files = existsSync(socketDir) ? readdirSync(socketDir).sort() : [];
  throw new Error(
    `${error.message}\nhost pid: ${host.child.pid} exit: ${host.child.exitCode} signal: ${host.child.signalCode}`
    + `\nsocket files: ${files.join(', ')}\n${host.output()}`,
  );
} finally {
  await stopHost(host);
  rmSync(fixtureRoot, { recursive: true, force: true });
}
