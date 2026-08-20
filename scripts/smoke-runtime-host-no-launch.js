#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
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

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-host-'));
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

function run(session, args = ['stream', 'status']) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, ['--json', '--session', session, ...args], {
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
      let payload;
      try {
        payload = JSON.parse(stdout.trim());
      } catch (error) {
        reject(new Error(`${session} returned invalid JSON: ${error.message}\n${stdout}${stderr}`));
        return;
      }
      if (payload.success !== true) {
        reject(new Error(`${session} returned failure: ${stdout}${stderr}`));
        return;
      }
      resolve(payload);
    });
  });
}

function filesWithSuffix(suffix) {
  return readdirSync(socketDir).filter((name) => name.endsWith(suffix)).sort();
}

function processIsLive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitFor(predicate, label, timeoutMs = 5000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

let hostPid = null;
try {
  const results = await Promise.all([
    run('alpha'),
    run('alpha'),
    run('beta'),
    run('gamma'),
  ]);
  assert(results.every((result) => Number.isInteger(result.data?.port)), 'lane stream ports are missing');

  hostPid = Number.parseInt(readFileSync(join(socketDir, 'runtime-host.pid'), 'utf8'), 10);
  assert(Number.isInteger(hostPid) && processIsLive(hostPid), 'runtime host PID is not live');
  assert(filesWithSuffix('.pid').length === 1, `expected one PID file: ${filesWithSuffix('.pid')}`);
  assert(filesWithSuffix('.sock').length === 1, `expected one socket: ${filesWithSuffix('.sock')}`);
  assert(filesWithSuffix('.stream').length === 3, `expected three logical streams: ${filesWithSuffix('.stream')}`);

  const manifest = JSON.parse(readFileSync(join(socketDir, 'runtime-host.json'), 'utf8'));
  assert(manifest.schemaVersion === 'agent-browser.runtime-host.v1', 'runtime host manifest schema mismatch');
  assert(manifest.pid === hostPid, 'runtime host manifest PID mismatch');
  assert(manifest.maxLanes === 64, 'runtime host lane bound mismatch');

  await run('alpha', ['close']);
  assert(!existsSync(join(socketDir, 'alpha.stream')), 'closed alpha lane retained stream metadata');
  await Promise.all([run('beta'), run('gamma')]);
  assert(processIsLive(hostPid), 'closing one lane terminated the shared runtime host');

  await Promise.all([run('beta', ['close']), run('gamma', ['close'])]);
  await waitFor(() => !processIsLive(hostPid), 'idle runtime host exit');
  assert(!existsSync(join(socketDir, 'runtime-host.json')), 'owned runtime host manifest survived exit');
  console.log('Runtime host no-launch smoke passed');
} finally {
  if (hostPid && processIsLive(hostPid)) {
    process.kill(hostPid, 'SIGTERM');
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}
