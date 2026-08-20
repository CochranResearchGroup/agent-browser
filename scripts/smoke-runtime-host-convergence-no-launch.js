#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
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

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-host-convergence-'));
const home = join(fixtureRoot, 'home');
const oldDir = join(fixtureRoot, 'old-host');
const candidateDir = join(fixtureRoot, 'candidate-host');
const ingressPath = join(home, '.agent-browser', 'runtime-host-ingress.json');
mkdirSync(home, { recursive: true });
const binarySha256 = createHash('sha256').update(readFileSync(binary)).digest('hex');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function hostEnv(socketDir) {
  return {
    ...process.env,
    HOME: home,
    AGENT_BROWSER_HOME: join(home, '.agent-browser'),
    AGENT_BROWSER_SOCKET_DIR: socketDir,
    AGENT_BROWSER_RUNTIME_HOST: '1',
    AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  };
}

function selectedEnv() {
  const env = {
    ...process.env,
    HOME: home,
    AGENT_BROWSER_HOME: join(home, '.agent-browser'),
    AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE: ingressPath,
    AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  };
  delete env.AGENT_BROWSER_SOCKET_DIR;
  delete env.AGENT_BROWSER_RUNTIME_HOST;
  return env;
}

function run(session, args, env) {
  const result = spawnSync(binary, ['--json', '--session', session, ...args], {
    cwd: rootDir,
    env,
    encoding: 'utf8',
  });
  assert(result.status === 0, `command failed: ${result.stderr || result.stdout}`);
  const payload = JSON.parse(result.stdout);
  assert(payload.success === true, `command returned failure: ${result.stdout}`);
  return payload;
}

async function waitFor(predicate, label, timeoutMs = 10000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

function processIsLive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitForHost(socketDir) {
  await waitFor(
    () => existsSync(join(socketDir, 'runtime-host.json'))
      && existsSync(join(socketDir, 'runtime-host.identity.json'))
      && existsSync(join(socketDir, 'runtime-host.sha256'))
      && readFileSync(join(socketDir, 'runtime-host.sha256'), 'utf8').trim() === binarySha256,
    `runtime host identity in ${socketDir}`,
  );
}

async function startHost(socketDir, generationId) {
  mkdirSync(socketDir, { recursive: true });
  const env = hostEnv(socketDir);
  run('alpha', ['stream', 'status'], env);
  run('beta', ['stream', 'status'], env);
  await waitForHost(socketDir);
  const manifest = JSON.parse(readFileSync(join(socketDir, 'runtime-host.json'), 'utf8'));
  assert(processIsLive(manifest.pid), `${generationId} runtime host is not live`);
  return {
    topology: 'single_host',
    generationId,
    socketDir,
    binarySha256,
    hostId: manifest.hostId,
    pid: manifest.pid,
    socketIdentity: manifest.socketIdentity,
  };
}

function writeRegistry(registry) {
  mkdirSync(join(home, '.agent-browser'), { recursive: true });
  const staged = `${ingressPath}.tmp`;
  writeFileSync(staged, `${JSON.stringify(registry, null, 2)}\n`, { mode: 0o600 });
  renameSync(staged, ingressPath);
}

function stagedRegistry(oldBackend, candidateBackend, revision) {
  return {
    schemaVersion: 'agent-browser.runtime-host-ingress.v1',
    revision,
    activeTransactionId: 'upgrade-convergence-smoke',
    selectedBackend: oldBackend,
    candidateBackend,
    fallbackBackend: null,
  };
}

function committedRegistry(oldBackend, candidateBackend, revision) {
  return {
    schemaVersion: 'agent-browser.runtime-host-ingress.v1',
    revision,
    activeTransactionId: null,
    selectedBackend: candidateBackend,
    candidateBackend: null,
    fallbackBackend: oldBackend,
  };
}

function selectedPort() {
  return Number(run('alpha', ['stream', 'status'], selectedEnv()).data.port);
}

async function stopHost(backend, signal = 'SIGTERM') {
  if (processIsLive(backend.pid)) process.kill(backend.pid, signal);
  await waitFor(() => !processIsLive(backend.pid), `${backend.generationId} host exit`);
}

let oldBackend;
let candidateBackend;
try {
  oldBackend = await startHost(oldDir, 'generation-old');
  candidateBackend = await startHost(candidateDir, 'generation-candidate-crash');
  assert(oldBackend.pid !== candidateBackend.pid, 'transaction did not create two bounded hosts');
  writeRegistry(stagedRegistry(oldBackend, candidateBackend, 2));
  const oldPort = Number(readFileSync(join(oldDir, 'alpha.stream'), 'utf8'));
  assert(selectedPort() === oldPort, 'staged candidate changed selected ingress');

  await stopHost(candidateBackend, 'SIGKILL');
  assert(selectedPort() === oldPort, 'candidate crash disrupted selected old host');
  rmSync(candidateDir, { recursive: true, force: true });

  candidateBackend = await startHost(candidateDir, 'generation-candidate');
  writeRegistry(stagedRegistry(oldBackend, candidateBackend, 3));
  writeRegistry(committedRegistry(oldBackend, candidateBackend, 4));
  const candidatePort = Number(readFileSync(join(candidateDir, 'alpha.stream'), 'utf8'));
  assert(selectedPort() === candidatePort, 'committed ingress did not select candidate host');

  writeRegistry({
    ...committedRegistry(candidateBackend, oldBackend, 5),
    selectedBackend: oldBackend,
    fallbackBackend: candidateBackend,
  });
  await stopHost(candidateBackend);
  assert(selectedPort() === oldPort, 'post-commit rollback did not restore old host');
  rmSync(candidateDir, { recursive: true, force: true });

  candidateBackend = await startHost(candidateDir, 'generation-candidate-final');
  writeRegistry(stagedRegistry(oldBackend, candidateBackend, 6));
  writeRegistry(committedRegistry(oldBackend, candidateBackend, 7));
  await stopHost(oldBackend);
  assert(selectedPort() === Number(readFileSync(join(candidateDir, 'alpha.stream'), 'utf8')),
    'final candidate did not remain reachable after old-host exit');
  assert(processIsLive(candidateBackend.pid), 'final candidate host is not live');
  console.log('Runtime host convergence no-launch smoke passed');
} finally {
  if (candidateBackend && processIsLive(candidateBackend.pid)) {
    await stopHost(candidateBackend).catch(() => {});
  }
  if (oldBackend && processIsLive(oldBackend.pid)) {
    await stopHost(oldBackend).catch(() => {});
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}
