#!/usr/bin/env node

import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { createServer } from 'node:net';
import { isAbsolute, join, resolve } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const binaryArgument = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD
  || join(rootDir, 'cli', 'target', 'debug', 'agent-browser');
const binary = isAbsolute(binaryArgument) ? binaryArgument : resolve(rootDir, binaryArgument);

if (!process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD) {
  const build = spawnSync(join(rootDir, 'scripts', 'ci', 'cargo-safe.sh'),
    ['build', '--manifest-path', 'cli/Cargo.toml'],
    { cwd: rootDir, encoding: 'utf8', stdio: 'inherit' });
  if (build.status !== 0) process.exit(build.status ?? 1);
}

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-supervisor-takeover-'));
const home = join(fixtureRoot, 'home');
const socketDir = join(fixtureRoot, 'socket');
const supervisorRoot = join(fixtureRoot, 'supervisor');
const manifestDir = join(supervisorRoot, 'manifests');
const systemctlState = join(fixtureRoot, 'systemctl.pid');
const systemctlLog = join(fixtureRoot, 'supervisor.log');
const fakeSystemctl = join(fixtureRoot, 'systemctl-fixture');
mkdirSync(home, { recursive: true });
mkdirSync(socketDir, { recursive: true });
mkdirSync(manifestDir, { recursive: true });

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

async function availableLoopbackPort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  assert(port > 0, 'could not allocate a fixture port');
  return port;
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

function runJson(args) {
  const result = spawnSync(binary, [...args, '--json'], {
    cwd: rootDir,
    env,
    encoding: 'utf8',
  });
  let body;
  try {
    body = JSON.parse(result.stdout.trim());
  } catch {
    throw new Error(`invalid JSON for ${args.join(' ')}: ${result.stdout}\n${result.stderr}`);
  }
  assert(result.status === 0, `${args.join(' ')} failed: ${JSON.stringify(body)}\n${result.stderr}`);
  assert(body.success === true && body.data, `${args.join(' ')} returned no data: ${JSON.stringify(body)}`);
  return body.data;
}

const streamPort = await availableLoopbackPort();
const executableSha256 = createHash('sha256').update(readFileSync(binary)).digest('hex');
writeFileSync(join(manifestDir, 'takeover.json'), `${JSON.stringify({
  schemaVersion: 'agent-browser.session-supervisor.v1',
  session: 'takeover',
  executablePath: binary,
  executableSha256,
  streamPort,
  runtimeProfile: 'takeover',
  provenance: {
    packageVersion: '0.28.0',
    installedAt: '2026-09-01T00:00:00Z',
    installedBy: 'runtime host supervisor takeover no-launch fixture',
  },
}, null, 2)}\n`, { mode: 0o600 });

writeFileSync(fakeSystemctl, `#!/bin/sh
set -eu
state="$AGENT_BROWSER_FAKE_SYSTEMCTL_STATE"
if [ "\${2:-}" = "show" ]; then
  pid=0
  active=inactive
  sub=dead
  if [ -f "$state" ]; then
    candidate=$(cat "$state")
    if kill -0 "$candidate" 2>/dev/null; then
      pid=$candidate
      active=active
      sub=running
    fi
  fi
  printf 'LoadState=loaded\\nUnitFileState=enabled\\nActiveState=%s\\nSubState=%s\\nResult=success\\nNRestarts=0\\nMainPID=%s\\n' "$active" "$sub" "$pid"
  exit 0
fi
if [ "\${2:-}" = "start" ]; then
  setsid "$AGENT_BROWSER_FAKE_HOST_BINARY" session supervisor run-host >>"$AGENT_BROWSER_FAKE_SYSTEMCTL_LOG" 2>&1 &
  echo $! >"$state"
  exit 0
fi
if [ "\${2:-}" = "reset-failed" ]; then
  exit 0
fi
exit 1
`, { mode: 0o700 });
chmodSync(fakeSystemctl, 0o700);

const env = {
  ...process.env,
  HOME: home,
  AGENT_BROWSER_HOME: join(home, '.agent-browser'),
  AGENT_BROWSER_SOCKET_DIR: socketDir,
  AGENT_BROWSER_SESSION_SUPERVISOR_ROOT: supervisorRoot,
  AGENT_BROWSER_SESSION_SUPERVISOR_SYSTEMCTL: fakeSystemctl,
  AGENT_BROWSER_RUNTIME_HOST_SUPERVISOR_TAKEOVER_STATE: join(fixtureRoot, 'takeover.json'),
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0',
  AGENT_BROWSER_FAKE_SYSTEMCTL_STATE: systemctlState,
  AGENT_BROWSER_FAKE_SYSTEMCTL_LOG: systemctlLog,
  AGENT_BROWSER_FAKE_HOST_BINARY: binary,
};

if (process.platform === 'linux') {
  mkdirSync(join(home, '.agent-browser'), { recursive: true });
  const bootId = readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim();
  writeFileSync(join(home, '.agent-browser', 'runtime-host-ingress.json'), `${JSON.stringify({
    schemaVersion: 'agent-browser.runtime-host-ingress.v1',
    revision: 1,
    bootEpoch: `linux:${bootId}`,
    activeTransactionId: null,
    selectedBackend: {
      topology: 'single_host',
      generationId: 'supervisor-takeover-fixture',
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

const source = spawn(binary, ['session', 'supervisor', 'run-host'], {
  cwd: rootDir,
  env,
  stdio: ['ignore', 'pipe', 'pipe'],
});
let sourceOutput = '';
let legacyWriter;
let fifoWriter;

source.stdout.on('data', (chunk) => { sourceOutput += chunk; });
source.stderr.on('data', (chunk) => { sourceOutput += chunk; });

try {
  await waitFor(
    () => existsSync(join(socketDir, 'runtime-host.identity.json'))
      && existsSync(join(socketDir, 'takeover.stream')),
    'source runtime host',
  );
  const sourcePid = source.pid;
  const conflict = spawnSync(binary, ['session', 'supervisor', 'run-host'], {
    cwd: rootDir,
    env,
    encoding: 'utf8',
  });
  assert(conflict.status !== 0, 'a second unsupervised host unexpectedly started');
  assert(`${conflict.stdout}${conflict.stderr}`.includes('port_conflict'),
    `the fixture did not reproduce the original port conflict: ${conflict.stdout}${conflict.stderr}`);
  const legacyMarker = join(home, '.agent-browser', 'runtime-adoption', 'admission-drain.json');
  mkdirSync(join(home, '.agent-browser', 'runtime-adoption'), { recursive: true });
  writeFileSync(legacyMarker, 'malformed retained upgrade metadata');
  // A crashed writer's PID may now belong to an unrelated live process.
  // Its retained diagnostics must not veto the operator's host transfer.
  const staleWriterPath = join(home, '.agent-browser/convergence/workstation.lock');
  mkdirSync(join(home, '.agent-browser/convergence'), { recursive: true });
  writeFileSync(staleWriterPath, `${process.pid}\n`);
  // A guarded public transaction command registers custody before reading
  // its supplied transaction. Pause that OS read and present the older wire
  // format to qualify cross-version exclusion without a private test API.
  const legacyTransaction = join(home, '.agent-browser/runtime-adoption/transactions/upgrade-legacy-writer.json');
  mkdirSync(join(home, '.agent-browser/runtime-adoption/transactions'), {recursive:true});
  const fifo = spawnSync('mkfifo', [legacyTransaction], {encoding:'utf8'});
  assert(fifo.status === 0, `fixture FIFO creation failed: ${fifo.stderr}`);
  fifoWriter = spawn('python3', ['-c', `import os,sys
fd=os.open(sys.argv[1],os.O_RDWR)
print('fifo-ready',flush=True)
sys.stdin.readline()
os.write(fd,b'{}')
os.close(fd)
`, legacyTransaction], {stdio:['pipe','pipe','pipe']});
  let fifoReady = false;
  fifoWriter.stdout.on('data', chunk => { fifoReady ||= chunk.toString().includes('fifo-ready'); });
  await waitFor(() => fifoReady || fifoWriter.exitCode !== null, 'transaction input boundary');
  assert(fifoReady, 'fixture FIFO writer did not start');
  let legacyOutput = '';
  legacyWriter = spawn(binary, ['install', 'transactions', 'close',
    '--transaction-id', 'upgrade-legacy-writer', '--expected-revision', '1',
    '--candidate-generation', 'fixture', '--census-digest', 'none', '--json'],
  {cwd:rootDir,env,stdio:['ignore','pipe','pipe']});
  legacyWriter.stdout.on('data', chunk => { legacyOutput += chunk; });
  legacyWriter.stderr.on('data', chunk => { legacyOutput += chunk; });
  await waitFor(() => legacyWriter.exitCode !== null || (() => {
    try {
      return readdirSync(`/proc/${legacyWriter.pid}/fd`).some(fd =>
        readlinkSync(`/proc/${legacyWriter.pid}/fd/${fd}`) === legacyTransaction);
    } catch { return false; }
  })(), 'executing transaction writer');
  assert(legacyWriter.exitCode === null, `transaction exited before input boundary: ${legacyOutput}`);
  // The open descriptor is the synchronization boundary; remove the pathname
  // so unrelated diagnostic inventory cannot wait on the prepared FIFO.
  rmSync(legacyTransaction);
  const busy = runJson(['session', 'supervisor', 'recover-host', '--dry-run']);
  assert(busy.blockers.some(blocker => blocker.code === 'workstation_transaction_busy'),
    'an executing writer must exclude takeover effects');
  const competing = spawnSync(binary, ['install', 'workstation', '--apply', '--json'], {
    cwd:rootDir, env:{...env, AGENT_BROWSER_WORKSTATION_ROOT:home}, encoding:'utf8',
  });
  assert(competing.status !== 0 && `${competing.stdout}${competing.stderr}`.includes('already active'),
    'a competing apply must stop before effects while writer custody is held');
  assert(!existsSync(join(home, '.local/lib/agent-browser/generations')),
    'a competing apply must not stage a candidate generation');
  const preview = spawnSync(binary, ['install', 'workstation', '--dry-run',
    '--runtime-replacement-policy', 'full-shutdown', '--json'], {cwd:rootDir,env,encoding:'utf8'});
  assert(preview.status === 0, `replacement preview failed: ${preview.stdout} ${preview.stderr}`);
  const replacement = JSON.parse(preview.stdout).runtimeReplacementPlan;
  assert(replacement.disposition === 'blocked'
    && replacement.blockers.some(blocker => blocker.code === 'workstation_transaction_busy'),
  `replacement preview must report executing writer contention: ${JSON.stringify(replacement)}`);
  const claimedBusy = runJson(['session', 'supervisor', 'recover-host', '--dry-run']);
  assert(claimedBusy.blockers.some(blocker => blocker.code === 'workstation_transaction_busy'),
    'an executing transaction must retain its process-instance custody between physical locks');
  assert(readFileSync(staleWriterPath, 'utf8').trim() === String(legacyWriter.pid),
    'the legacy coordination protocol must retain PID-only writer text for older binaries');
  rmSync(join(home, '.agent-browser/convergence/workstation.owner.json'));
  writeFileSync(staleWriterPath, `${legacyWriter.pid}\n`);
  const legacyBusy = runJson(['session', 'supervisor', 'recover-host', '--dry-run']);
  assert(legacyBusy.blockers.some(blocker => blocker.code === 'workstation_transaction_busy'),
    'an executing legacy transaction without an apply flag must retain custody');
  fifoWriter.stdin.end('release\n');
  await waitFor(() => legacyWriter.exitCode !== null, 'legacy transaction completion');
  assert(legacyWriter.exitCode !== 0, 'the deliberately invalid transaction must not mutate state');
  writeFileSync(staleWriterPath, `${process.pid}\n`);
  const plan = runJson(['session', 'supervisor', 'recover-host', '--dry-run']);
  assert(plan.disposition === 'ready_for_takeover', `stale writer metadata must not veto takeover: ${JSON.stringify(plan)}`);
  assert(plan.selectedBackend.pid === sourcePid, 'plan selected a different process');
  assert(plan.selectedListenerPorts.includes(streamPort), 'plan did not prove the fixed port owner');
  assert(plan.p147CapabilityReady === true, 'P147 capability evidence was not ready');

  const outcome = runJson([
    'session', 'supervisor', 'recover-host', '--apply',
    '--expected-plan-digest', plan.planDigest,
  ]);
  assert(outcome.state === 'accepted', `takeover was not accepted: ${JSON.stringify(outcome)}`);
  assert(outcome.sourcePid === sourcePid, 'receipt source PID drifted');
  assert(outcome.replacementPid !== sourcePid, 'supervisor did not replace the source PID');
  assert(outcome.browserLaunched === false, 'recovery reported a browser launch');
  await waitFor(() => !processIsLive(sourcePid), 'source host retirement');
  assert(processIsLive(outcome.replacementPid), 'replacement host is not live');
  const transaction = JSON.parse(readFileSync(join(fixtureRoot, 'takeover.json'), 'utf8'));
  assert(transaction.state === 'accepted', 'durable transaction was not accepted');
  assert(readFileSync(legacyMarker, 'utf8') === 'malformed retained upgrade metadata',
    'takeover must ignore retained upgrade markers without rewriting them');
  const steadyPlan = runJson(['session', 'supervisor', 'recover-host', '--dry-run']);
  assert(steadyPlan.disposition === 'already_supervised',
    `accepted recovery did not reach supervised steady state: ${JSON.stringify(steadyPlan)}`);
  const steadyOutcome = runJson([
    'session', 'supervisor', 'recover-host', '--apply',
    '--expected-plan-digest', steadyPlan.planDigest,
  ]);
  assert(steadyOutcome.state === 'already_supervised', 'steady-state apply was not zero-effect');
  assert(steadyOutcome.replacementPid === outcome.replacementPid,
    'steady-state receipt selected a different replacement');
  console.log('Runtime host supervisor takeover no-launch smoke passed');
} catch (error) {
  const supervisorOutput = existsSync(systemctlLog) ? readFileSync(systemctlLog, 'utf8') : '';
  throw new Error(`${error.message}\nsource output:\n${sourceOutput}\nsupervisor output:\n${supervisorOutput}`);
} finally {
  if (fifoWriter && fifoWriter.exitCode === null) fifoWriter.stdin.end('release\n');
  if (legacyWriter && legacyWriter.exitCode === null) {
    await waitFor(() => legacyWriter.exitCode !== null, 'legacy fixture writer cleanup').catch(() => {});
    if (legacyWriter.exitCode === null) legacyWriter.kill('SIGTERM');
  }
  if (fifoWriter && fifoWriter.exitCode === null) {
    await waitFor(() => fifoWriter.exitCode !== null, 'FIFO fixture cleanup').catch(() => {});
    if (fifoWriter.exitCode === null) fifoWriter.kill('SIGTERM');
  }
  if (source.exitCode === null && processIsLive(source.pid)) source.kill('SIGTERM');
  if (existsSync(systemctlState)) {
    const pid = Number.parseInt(readFileSync(systemctlState, 'utf8'), 10);
    if (Number.isInteger(pid) && processIsLive(pid)) process.kill(pid, 'SIGTERM');
  }
  rmSync(fixtureRoot, { recursive: true, force: true });
}
