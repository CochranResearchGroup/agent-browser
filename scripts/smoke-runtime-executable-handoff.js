#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const args = process.argv.slice(2);
const options = {
  agentBrowserBin: process.env.AGENT_BROWSER_BIN || resolve(rootDir, 'cli/target/debug/agent-browser'),
  json: false,
  publish: false,
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--agent-browser-bin') {
    options.agentBrowserBin = requiredValue(args, ++index, arg);
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--publish') {
    options.publish = true;
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    throw new Error(`Unknown argument: ${arg}`);
  }
}

const tempDir = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-handoff-'));
const sessionName = `runtime-handoff-${process.pid}`;
let browserPid = null;

try {
  const profilePath = join(tempDir, 'profile');
  const smokeUrl = 'data:text/html,<title>Runtime Handoff Smoke</title><h1>Runtime Handoff Smoke</h1>';
  runAgent(['--profile', profilePath, 'open', smokeUrl]);
  const before = runtimeReadback();

  let transition;
  if (options.publish) {
    const published = runJson('pnpm', [
      'publish:local-dashboard',
      '--',
      '--skip-browser',
      '--json',
    ], { timeoutMs: 600_000 });
    const handoff = published.handoffs?.resumed?.find(
      (entry) => entry.sessionName === sessionName,
    );
    assert(handoff, `Publish did not report resumed session '${sessionName}'`);
    transition = {
      mode: 'publish',
      publishedExecutable: published.installBin,
      handoff,
    };
  } else {
    const prepared = runAgent(['handoff', 'prepare']);
    assert(prepared.data?.prepared === true, `Handoff was not prepared: ${JSON.stringify(prepared)}`);
    waitFor(
      () => !existsSync(runtimePidPath()),
      `Daemon '${sessionName}' did not exit after handoff prepare`,
    );
    assert(processIsLive(before.browserPid), 'Browser exited during handoff prepare');
    const resumed = runAgent(['handoff', 'resume']);
    assert(resumed.data?.resumed === true, `Handoff was not resumed: ${JSON.stringify(resumed)}`);
    transition = {
      mode: 'direct',
      prepared: prepared.data,
      resumed: resumed.data,
    };
  }

  const after = runtimeReadback();
  assert(before.daemonPid !== after.daemonPid, 'Daemon PID did not change');
  assert(before.browserPid === after.browserPid, 'Browser PID changed across handoff');
  assert(before.cdpUrl === after.cdpUrl, 'Browser CDP endpoint changed across handoff');
  assert(before.title === after.title, 'Active tab title changed across handoff');
  assert(processIsLive(after.browserPid), 'Browser is not running after handoff');
  assert(
    after.streamFilePort === after.streamPort,
    `Runtime stream record did not refresh to the replacement daemon: ${JSON.stringify(after)}`,
  );
  if (after.serviceStreamUrl !== null) {
    assert(
      after.serviceStreamUrl === `http://127.0.0.1:${after.streamPort}/`,
      `Service stream record did not refresh to the replacement daemon: ${JSON.stringify(after)}`,
    );
  }
  browserPid = after.browserPid;

  const result = {
    success: true,
    sessionName,
    before,
    after,
    transition,
    daemonChanged: true,
    browserPidPreserved: true,
    cdpEndpointPreserved: true,
    tabPreserved: true,
    streamRecordRefreshed: true,
  };
  if (options.json) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    console.log('Runtime executable handoff smoke passed');
  }
} finally {
  try {
    runAgent(['close']);
  } catch {
    // Continue to the bounded process cleanup assertion below.
  }
  if (browserPid !== null) {
    waitFor(() => !processIsLive(browserPid), 'Browser remained alive after normal session close');
  }
  rmSync(tempDir, { recursive: true, force: true });
}

function runtimeReadback() {
  const browserPidResponse = runAgent(['get', 'browser-pid']);
  const cdpUrlResponse = runAgent(['get', 'cdp-url']);
  let titleResponse;
  waitFor(() => {
    titleResponse = runAgent(['get', 'title']);
    return titleResponse.data?.title === 'Runtime Handoff Smoke';
  }, 'Runtime handoff smoke page did not finish loading');
  const streamResponse = runAgent(['stream', 'status']);
  const serviceResponse = runAgent(['service', 'browsers']);
  const daemonPid = Number.parseInt(readFileSync(runtimePidPath(), 'utf8').trim(), 10);
  const currentBrowserPid = browserPidResponse.data?.pid;
  const cdpUrl = cdpUrlResponse.data?.cdpUrl;
  const title = titleResponse.data?.title;
  const streamPort = streamResponse.data?.port;
  const serviceBrowser = serviceResponse.data?.browsers?.find(
    (browser) => browser?.id === `session:${sessionName}`,
  );
  const serviceStreamUrl = serviceBrowser?.viewStreams?.find(
    (stream) => stream?.provider === 'cdp_screencast',
  )?.url ?? null;
  const streamFilePort = Number.parseInt(readFileSync(runtimeStreamPath(), 'utf8').trim(), 10);
  assert(Number.isInteger(daemonPid) && daemonPid > 0, 'Daemon PID is missing');
  assert(Number.isInteger(currentBrowserPid) && currentBrowserPid > 0, 'Browser PID is missing');
  assert(typeof cdpUrl === 'string' && cdpUrl.length > 0, 'CDP endpoint is missing');
  assert(title === 'Runtime Handoff Smoke', `Unexpected tab title '${title}'`);
  assert(Number.isInteger(streamPort) && streamPort > 0, 'Runtime stream port is missing');
  assert(streamFilePort === streamPort, 'Runtime stream metadata does not match the daemon');
  return {
    daemonPid,
    browserPid: currentBrowserPid,
    cdpUrl,
    title,
    streamPort,
    streamFilePort,
    serviceStreamUrl,
  };
}

function runtimePidPath() {
  const socketDir = process.env.AGENT_BROWSER_SOCKET_DIR
    || (process.env.XDG_RUNTIME_DIR
      ? join(process.env.XDG_RUNTIME_DIR, 'agent-browser')
      : join(process.env.HOME, '.agent-browser'));
  return join(socketDir, `${sessionName}.pid`);
}

function runtimeStreamPath() {
  const socketDir = process.env.AGENT_BROWSER_SOCKET_DIR
    || (process.env.XDG_RUNTIME_DIR
      ? join(process.env.XDG_RUNTIME_DIR, 'agent-browser')
      : join(process.env.HOME, '.agent-browser'));
  return join(socketDir, `${sessionName}.stream`);
}

function runAgent(commandArgs) {
  return runJson(options.agentBrowserBin, [
    '--json',
    '--session',
    sessionName,
    ...commandArgs,
  ]);
}

function runJson(command, commandArgs, { timeoutMs = 120_000 } = {}) {
  const result = spawnSync(command, commandArgs, {
    cwd: rootDir,
    env: process.env,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: timeoutMs,
    maxBuffer: 16 * 1024 * 1024,
  });
  let payload;
  try {
    payload = JSON.parse(String(result.stdout || '').trim());
  } catch (error) {
    throw new Error(
      `${command} ${commandArgs.join(' ')} returned invalid JSON: ${error.message}\n` +
      `${result.stdout || ''}${result.stderr || ''}`,
    );
  }
  if (result.status !== 0 || payload.success !== true) {
    throw new Error(
      `${command} ${commandArgs.join(' ')} failed: ` +
      `${payload.error || result.error?.message || result.stderr || result.stdout}`,
    );
  }
  return payload;
}

function processIsLive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function waitFor(predicate, message) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    if (predicate()) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
  }
  throw new Error(message);
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) throw new Error(`Missing value for ${flag}`);
  return value;
}

function printHelp() {
  console.log(`Usage: node scripts/smoke-runtime-executable-handoff.js [options]

Prove that replacing or restarting an agent-browser daemon preserves its live
browser PID, CDP endpoint, and active tab, then prove normal close still cleans
up the handed-off browser.

Options:
  --agent-browser-bin <path>  Binary used for the session.
  --publish                   Exercise the full publish:local-dashboard path.
  --json                      Print structured evidence.
`);
}
