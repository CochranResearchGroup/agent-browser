#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD
  || join(rootDir, 'cli', 'target', 'debug', 'agent-browser');
const chromeExecutable = [
  process.env.AGENT_BROWSER_SMOKE_CHROME_EXECUTABLE,
  '/usr/bin/google-chrome',
  '/usr/bin/google-chrome-stable',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
].find((candidate) => candidate && existsSync(candidate));

if (!process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD) {
  const build = spawnSync(
    join(rootDir, 'scripts', 'ci', 'cargo-safe.sh'),
    ['build', '--manifest-path', 'cli/Cargo.toml'],
    { cwd: rootDir, encoding: 'utf8', stdio: 'inherit' },
  );
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-runtime-host-handoff-'));
const home = join(fixtureRoot, 'home');
const oldDir = join(fixtureRoot, 'old-host');
const candidateDir = join(fixtureRoot, 'candidate-host');
mkdirSync(home, { recursive: true });
const lanes = [
  {
    sourceSession: 'source-alpha',
    runtimeProfile: 'hot-handoff-alpha',
    initialTitle: 'Hot handoff alpha',
    rollbackTitle: 'Rollback restored alpha',
  },
  {
    sourceSession: 'source-beta',
    runtimeProfile: 'hot-handoff-beta',
    initialTitle: 'Hot handoff beta',
    rollbackTitle: 'Rollback restored beta',
  },
];
const candidateProfiles = new Map();

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function runtimeProfileForSession(session) {
  return candidateProfiles.get(session)
    || lanes.find((lane) => lane.sourceSession === session)?.runtimeProfile
    || lanes.find((lane) => session.endsWith(lane.runtimeProfile))?.runtimeProfile
    || lanes[0].runtimeProfile;
}

function runtimeEnv(socketDir, session) {
  assert(chromeExecutable, 'hot handoff smoke requires a native Linux Chrome executable');
  return {
    ...process.env,
    HOME: home,
    AGENT_BROWSER_HOME: join(home, '.agent-browser'),
    AGENT_BROWSER_SOCKET_DIR: socketDir,
    AGENT_BROWSER_RUNTIME_HOST: '1',
    AGENT_BROWSER_RUNTIME_PROFILE: runtimeProfileForSession(session),
    AGENT_BROWSER_EXECUTABLE_PATH: chromeExecutable,
    AGENT_BROWSER_EXECUTABLE_PATH_SOURCE: 'runtime-host-hot-handoff-smoke',
    AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  };
}

function invoke(session, args, socketDir, expectSuccess = true) {
  return new Promise((resolve, reject) => {
    const child = spawn(
      binary,
      ['--json', '--session', session, ...args],
      {
        cwd: rootDir,
        env: runtimeEnv(socketDir, session),
        stdio: ['ignore', 'pipe', 'pipe'],
      },
    );
    let stdout = '';
    let stderr = '';
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject);
    child.on('exit', (code, signal) => {
      let payload;
      try {
        payload = JSON.parse(stdout.trim());
      } catch (error) {
        reject(new Error(
          `${session} returned invalid JSON: ${error.message}; code=${code} signal=${signal}\n${stdout}${stderr}`,
        ));
        return;
      }
      const succeeded = code === 0 && payload.success !== false;
      if (succeeded !== expectSuccess) {
        reject(new Error(
          `${session} success mismatch: expected=${expectSuccess} code=${code} signal=${signal}\n${stdout}${stderr}`,
        ));
        return;
      }
      resolve(payload);
    });
  });
}

function processIsLive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function waitFor(predicate, label, timeoutMs = 15000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`Timed out waiting for ${label}`);
}

function hostPid(socketDir) {
  const path = join(socketDir, 'runtime-host.pid');
  if (!existsSync(path)) return null;
  const pid = Number.parseInt(readFileSync(path, 'utf8'), 10);
  return Number.isInteger(pid) ? pid : null;
}

function transferPreparedHandoff(sourceSession) {
  mkdirSync(candidateDir, { recursive: true });
  copyFileSync(
    join(oldDir, `${sourceSession}.handoff.json`),
    join(candidateDir, `${sourceSession}.handoff.json`),
  );
}

function clearReturnedHandoff(sourceSession) {
  rmSync(join(oldDir, `${sourceSession}.handoff.json`), { force: true });
}

async function stopHost(socketDir) {
  const pid = hostPid(socketDir);
  if (!pid || !processIsLive(pid)) return;
  process.kill(pid, 'SIGTERM');
  await waitFor(() => !processIsLive(pid), `host ${pid} exit`);
}

const candidateSessions = new Set();
try {
  for (const lane of lanes) {
    await invoke(
      `profile-setup-${lane.runtimeProfile}`,
      ['runtime', 'create', lane.runtimeProfile, '--browser-family', 'chrome'],
      oldDir,
    );
    const initial = await invoke(
      lane.sourceSession,
      ['open', `data:text/html,<title>${lane.initialTitle}</title>`],
      oldDir,
    );
    assert(initial.data?.title === lane.initialTitle,
      `${lane.sourceSession} browser did not launch: ${JSON.stringify(initial)}`);
  }

  const oldHostPid = hostPid(oldDir);
  assert(oldHostPid && processIsLive(oldHostPid), 'source runtime host is not live');

  for (const lane of lanes) {
    const prepared = await invoke(lane.sourceSession, ['handoff', 'prepare'], oldDir);
    lane.sourcePid = prepared.data?.browserPid;
    lane.candidateSession = prepared.data?.candidateSessionName;
    assert(Number.isInteger(lane.sourcePid),
      `${lane.sourceSession} prepare omitted browser PID: ${JSON.stringify(prepared)}`);
    assert(lane.candidateSession,
      `${lane.sourceSession} prepare omitted candidate session: ${JSON.stringify(prepared)}`);
    candidateProfiles.set(lane.candidateSession, lane.runtimeProfile);
    candidateSessions.add(lane.candidateSession);
    transferPreparedHandoff(lane.sourceSession);
    const resumed = await invoke(
      lane.candidateSession,
      ['handoff', 'resume', '--source-session', lane.sourceSession],
      candidateDir,
    );
    assert(resumed.data?.ownerTransferReceipt?.receiptId,
      `${lane.sourceSession} resume omitted owner receipt: ${JSON.stringify(resumed)}`);
  }

  const candidateHostPid = hostPid(candidateDir);
  assert(candidateHostPid && candidateHostPid !== oldHostPid,
    'handoff did not remain within the two-host convergence window');
  for (const lane of lanes) {
    const candidateStatus = await invoke(lane.candidateSession, ['get', 'title'], candidateDir);
    assert(candidateStatus.data === lane.initialTitle
      || candidateStatus.data?.title === lane.initialTitle,
    `${lane.sourceSession} candidate lost its active page: ${JSON.stringify(candidateStatus)}`);
    const rejected = await invoke(
      lane.sourceSession,
      ['open', `data:text/html,<title>Old ${lane.sourceSession} must not mutate</title>`],
      oldDir,
      false,
    );
    assert(
      String(rejected.error).includes('observation')
        || String(rejected.error).includes('runtime_owner_generation_stale'),
      `${lane.sourceSession} retained old-host effect authority: ${JSON.stringify(rejected)}`,
    );
  }

  for (const lane of lanes) {
    const rolledBack = await invoke(
      lane.candidateSession,
      ['handoff', 'rollback', '--source-session', lane.sourceSession],
      candidateDir,
    );
    assert(rolledBack.data?.ownerTransferReceipt?.receiptId,
      `${lane.sourceSession} rollback omitted owner receipt: ${JSON.stringify(rolledBack)}`);
    clearReturnedHandoff(lane.sourceSession);
    await invoke(
      lane.sourceSession,
      ['open', `data:text/html,<title>${lane.rollbackTitle}</title>`],
      oldDir,
    );
  }
  await waitFor(() => !processIsLive(candidateHostPid), 'rolled-back candidate host exit');

  for (const lane of lanes) {
    const preparedAgain = await invoke(lane.sourceSession, ['handoff', 'prepare'], oldDir);
    candidateSessions.delete(lane.candidateSession);
    lane.candidateSession = preparedAgain.data?.candidateSessionName;
    candidateProfiles.set(lane.candidateSession, lane.runtimeProfile);
    candidateSessions.add(lane.candidateSession);
    transferPreparedHandoff(lane.sourceSession);
    lane.resumedAgain = await invoke(
      lane.candidateSession,
      ['handoff', 'resume', '--source-session', lane.sourceSession],
      candidateDir,
    );
    assert(lane.resumedAgain.data?.ownerTransferReceipt?.processInstanceDigest,
      `${lane.sourceSession} second resume omitted browser identity: ${JSON.stringify(lane.resumedAgain)}`);
  }
  assert(hostPid(candidateDir) !== oldHostPid, 'candidate retry reused the old host process');
  for (const lane of lanes) {
    await invoke(lane.sourceSession, ['handoff', 'finalize'], oldDir);
  }
  await waitFor(() => !processIsLive(oldHostPid), 'finalized old host exit');
  for (const lane of lanes) {
    const finalTitle = await invoke(lane.candidateSession, ['get', 'title'], candidateDir);
    assert(finalTitle.data === lane.rollbackTitle
      || finalTitle.data?.title === lane.rollbackTitle,
    `${lane.sourceSession} candidate lost the browser after finalize: ${JSON.stringify(finalTitle)}`);
    assert(
      lane.resumedAgain.data?.browserPid === lane.sourcePid,
      `${lane.sourceSession} browser PID changed during hot handoff: ${JSON.stringify(lane.resumedAgain)}`,
    );
  }
  assert(processIsLive(hostPid(candidateDir)), 'candidate host exited after successful finalize');
  console.log('Runtime host multi-lane hot handoff smoke passed');
} finally {
  for (const candidateSession of candidateSessions) {
    await invoke(candidateSession, ['close'], candidateDir).catch(() => {});
  }
  await stopHost(candidateDir).catch(() => {});
  await stopHost(oldDir).catch(() => {});
  rmSync(fixtureRoot, { recursive: true, force: true });
}
