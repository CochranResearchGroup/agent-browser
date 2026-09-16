#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const wrapper = join(repoRoot, 'scripts', 'ci', 'cargo-safe.sh');
const sanitizedCacheWrapper = join(repoRoot, 'scripts', 'ci', 'sccache-sanitized.sh');
const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-cargo-capacity-'));
const admissionDir = join(fixtureRoot, 'admission');
const meminfo = join(fixtureRoot, 'meminfo');

function writeMeminfo({ available = 64 * 1024 * 1024, swapFree = 16 * 1024 * 1024 } = {}) {
  writeFileSync(meminfo, `MemAvailable: ${available} kB\nSwapFree: ${swapFree} kB\n`);
}

function probe({ hold = 0, noWait = false, overrides = {} } = {}) {
  const child = spawn(wrapper, ['check'], {
    cwd: repoRoot,
    env: {
      ...process.env,
      AGENT_BROWSER_CARGO_ADMISSION_DIR: admissionDir,
      AGENT_BROWSER_CARGO_CAPACITY_PROBE_ONLY: '1',
      AGENT_BROWSER_CARGO_CAPACITY_HOLD_SECONDS: String(hold),
      AGENT_BROWSER_CARGO_ADMISSION_POLL_SECONDS: '0.05',
      AGENT_BROWSER_CARGO_MEMINFO_FILE: meminfo,
      AGENT_BROWSER_CARGO_DISK_AVAILABLE_KIB: String(1024 * 1024 * 1024),
      AGENT_BROWSER_CARGO_CPU_COUNT: '20',
      AGENT_BROWSER_CARGO_MEMORY_RESERVE_KIB: String(16 * 1024 * 1024),
      AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB: String(14 * 1024 * 1024),
      AGENT_BROWSER_CARGO_MAX_CONCURRENT: '2',
      AGENT_BROWSER_CARGO_ADMISSION_NO_WAIT: noWait ? '1' : '0',
      ...overrides,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', (chunk) => { stdout += chunk; });
  child.stderr.on('data', (chunk) => { stderr += chunk; });
  const completed = new Promise((completionResolve) => {
    child.on('close', (status) => completionResolve({ status, stdout, stderr }));
  });
  return { child, completed };
}

function executeCargo({ overrides = {} } = {}) {
  return new Promise((completionResolve) => {
    const child = spawn(wrapper, ['check'], {
      cwd: repoRoot,
      env: {
        ...process.env,
        AGENT_BROWSER_CARGO_ADMISSION_DIR: admissionDir,
        AGENT_BROWSER_CARGO_MEMINFO_FILE: meminfo,
        AGENT_BROWSER_CARGO_DISK_AVAILABLE_KIB: String(1024 * 1024 * 1024),
        AGENT_BROWSER_CARGO_CPU_COUNT: '20',
        ...overrides,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => { stdout += chunk; });
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    child.on('close', (status) => completionResolve({ status, stdout, stderr }));
  });
}

async function waitForClaimCount(expected) {
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) {
    let count = 0;
    try {
      count = readdirSync(join(admissionDir, 'claims')).filter((name) => name.endsWith('.claim')).length;
    } catch {}
    if (count === expected) return;
    await new Promise((resolveWait) => setTimeout(resolveWait, 25));
  }
  assert.fail(`timed out waiting for ${expected} Cargo capacity claims`);
}

function processIsAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function processesUsingPath(path) {
  const matches = [];
  for (const entry of readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const commandLine = readFileSync(join('/proc', entry, 'cmdline'), 'utf8').replaceAll('\0', ' ');
      if (commandLine.includes(path)) matches.push(Number(entry));
    } catch {}
  }
  return matches;
}

async function runLingeringScopeAccountingFixture() {
  const root = join(fixtureRoot, 'lingering-scope-accounting');
  const bin = join(root, 'bin');
  const browserPidFile = join(root, 'browser.pid');
  mkdirSync(bin, { recursive: true });
  const browser = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)', 'lingering-scope'], {
    stdio: 'ignore',
  });
  writeFileSync(browserPidFile, `${browser.pid}\n`);

  const systemctlPath = join(bin, 'systemctl');
  writeFileSync(systemctlPath, `#!/usr/bin/env bash
set -euo pipefail
pid="$(cat "\${P200_BROWSER_PID_FILE}")"
state="$(awk '{print $3}' "/proc/\${pid}/stat" 2>/dev/null || true)"
if [[ "$pid" =~ ^[0-9]+$ && -n "$state" && "$state" != 'Z' ]]; then
  printf 'active\n'
  exit 0
fi
printf 'inactive\n'
exit 3
`);
  chmodSync(systemctlPath, 0o755);

  const claimsDir = join(admissionDir, 'claims');
  const lingeringClaim = join(claimsDir, 'lingering-scope.claim');
  writeFileSync(
    lingeringClaim,
    'pid=999999\nstart=1\nscope=agent-browser-cargo-p200-lingering.scope\njobs=8\nmemory_claim_kib=14680064\n',
  );
  try {
    const blocked = await probe({
      noWait: true,
      overrides: {
        PATH: `${bin}:${process.env.PATH}`,
        P200_BROWSER_PID_FILE: browserPidFile,
        AGENT_BROWSER_CARGO_MAX_CONCURRENT: '1',
      },
    }).completed;
    assert.equal(blocked.status, 75, blocked.stderr);
    assert.match(blocked.stderr, /reason=concurrency_limit/);
    assert.equal(readdirSync(claimsDir).includes('lingering-scope.claim'), true);
  } finally {
    browser.kill('SIGTERM');
    await new Promise((resolveClose) => browser.once('close', resolveClose));
  }

  const admitted = await probe({
    noWait: true,
    overrides: {
      PATH: `${bin}:${process.env.PATH}`,
      P200_BROWSER_PID_FILE: browserPidFile,
      AGENT_BROWSER_CARGO_MAX_CONCURRENT: '1',
    },
  }).completed;
  assert.equal(admitted.status, 0, admitted.stderr);
  assert.equal(JSON.parse(admitted.stdout).admitted, true);
  assert.equal(readdirSync(claimsDir).includes('lingering-scope.claim'), false);
}

async function runScopeDescendantFixture(exitStatus) {
  const label = exitStatus === 0 ? 'success' : 'failure';
  const root = join(fixtureRoot, `scope-${label}`);
  const bin = join(root, 'bin');
  const profile = join(root, 'profile');
  const browserPidFile = join(root, 'browser.pid');
  const unitFile = join(root, 'unit');
  const stopReceipt = join(root, 'stop.receipt');
  mkdirSync(bin, { recursive: true });
  mkdirSync(profile, { recursive: true });

  const cargoPath = join(bin, 'cargo');
  writeFileSync(cargoPath, `#!/usr/bin/env bash
set -euo pipefail
node -e 'setInterval(() => {}, 1000)' -- "--user-data-dir=\${P200_PROFILE_PATH}" </dev/null >/dev/null 2>&1 &
printf '%s\n' "$!" > "\${P200_BROWSER_PID_FILE}"
exit "\${P200_CARGO_EXIT_STATUS}"
`);
  chmodSync(cargoPath, 0o755);

  const systemdRunPath = join(bin, 'systemd-run');
  writeFileSync(systemdRunPath, `#!/usr/bin/env bash
set -euo pipefail
unit=''
while (( $# > 0 )); do
  case "$1" in
    --unit=*) unit="\${1#--unit=}"; shift ;;
    --user|--scope|--quiet|--slice=*|--property=*) shift ;;
    *) break ;;
  esac
done
printf '%s\n' "$unit" > "\${P200_UNIT_FILE}"
"$@"
`);
  chmodSync(systemdRunPath, 0o755);

  const systemctlPath = join(bin, 'systemctl');
  writeFileSync(systemctlPath, `#!/usr/bin/env bash
set -euo pipefail
case " $* " in
  *' show-environment '*|*' set-property '*) exit 0 ;;
  *' is-active '*)
    pid="$(cat "\${P200_BROWSER_PID_FILE}" 2>/dev/null || true)"
    state="$(awk '{print $3}' "/proc/\${pid}/stat" 2>/dev/null || true)"
    if [[ "$pid" =~ ^[0-9]+$ && -n "$state" && "$state" != 'Z' ]]; then
      printf 'active\n'
      exit 0
    fi
    printf 'inactive\n'
    exit 3
    ;;
  *' stop '*)
    unit="\${*: -1}"
    expected="$(cat "\${P200_UNIT_FILE}")"
    [[ -n "$expected" && "$unit" == "$expected" ]]
    claim_count="$(find "\${AGENT_BROWSER_CARGO_ADMISSION_DIR}/claims" -maxdepth 1 -name '*.claim' -print | wc -l)"
    [[ "$claim_count" == '1' ]]
    pid="$(cat "\${P200_BROWSER_PID_FILE}")"
    kill "$pid"
    printf 'unit=%s claim_present=true\n' "$unit" > "\${P200_STOP_RECEIPT}"
    ;;
  *) exit 1 ;;
esac
`);
  chmodSync(systemctlPath, 0o755);

  const foreign = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)', 'foreign-control'], {
    stdio: 'ignore',
  });
  let browserPid = null;
  try {
    const result = await executeCargo({
      overrides: {
        PATH: `${bin}:${process.env.PATH}`,
        AGENT_BROWSER_CARGO_FORCE_WSL: '1',
        AGENT_BROWSER_CARGO_CACHE: 'off',
        AGENT_BROWSER_CARGO_FAST_LINKER: 'off',
        AGENT_BROWSER_CARGO_SLICE: `agent-browser-cargo-p200-${process.pid}-${label}.slice`,
        P200_BROWSER_PID_FILE: browserPidFile,
        P200_PROFILE_PATH: profile,
        P200_CARGO_EXIT_STATUS: String(exitStatus),
        P200_UNIT_FILE: unitFile,
        P200_STOP_RECEIPT: stopReceipt,
      },
    });
    browserPid = Number(readFileSync(browserPidFile, 'utf8').trim());
    assert.equal(result.status, exitStatus, result.stderr);
    assert.equal(processIsAlive(browserPid), false, `${label} path left its browser-like descendant alive`);
    assert.deepEqual(processesUsingPath(profile), [], `${label} path retained a process using its profile`);
    assert.equal(processIsAlive(foreign.pid), true, `${label} path terminated a foreign process`);
    const unit = readFileSync(unitFile, 'utf8').trim();
    assert.match(unit, /^agent-browser-cargo-[a-zA-Z0-9_.-]+\.scope$/);
    assert.equal(
      readFileSync(stopReceipt, 'utf8').trim(),
      `unit=${unit} claim_present=true`,
      `${label} path released its claim before exact-scope teardown`,
    );
    await waitForClaimCount(0);
  } finally {
    foreign.kill('SIGTERM');
    if (browserPid && processIsAlive(browserPid)) process.kill(browserPid, 'SIGTERM');
  }
}

async function runRealScopeDescendantFixture() {
  const root = join(fixtureRoot, 'real-scope');
  const bin = join(root, 'bin');
  const profile = join(root, 'profile');
  const browserPidFile = join(root, 'browser.pid');
  const slice = `agent-browser-cargo-p200_${process.pid}.slice`;
  mkdirSync(bin, { recursive: true });
  mkdirSync(profile, { recursive: true });
  const cargoPath = join(bin, 'cargo');
  writeFileSync(cargoPath, `#!/usr/bin/env bash
set -euo pipefail
node -e 'setInterval(() => {}, 1000)' -- "--user-data-dir=\${P200_PROFILE_PATH}" </dev/null >/dev/null 2>&1 &
printf '%s\n' "$!" > "\${P200_BROWSER_PID_FILE}"
`);
  chmodSync(cargoPath, 0o755);

  let browserPid = null;
  let scopeUnit = null;
  try {
    const result = await executeCargo({
      overrides: {
        PATH: `${bin}:${process.env.PATH}`,
        AGENT_BROWSER_CARGO_FORCE_WSL: '1',
        AGENT_BROWSER_CARGO_CACHE: 'off',
        AGENT_BROWSER_CARGO_FAST_LINKER: 'off',
        AGENT_BROWSER_CARGO_SLICE: slice,
        P200_BROWSER_PID_FILE: browserPidFile,
        P200_PROFILE_PATH: profile,
      },
    });
    browserPid = Number(readFileSync(browserPidFile, 'utf8').trim());
    const scopeMatch = result.stderr.match(/unit=(agent-browser-cargo-[a-zA-Z0-9_.-]+\.scope)/);
    scopeUnit = scopeMatch?.[1] ?? null;
    assert.equal(result.status, 0, result.stderr);
    assert.ok(scopeUnit, `real scope identity missing from wrapper diagnostics: ${result.stderr}`);
    assert.equal(processIsAlive(browserPid), false, 'real scope left its browser-like descendant alive');
    assert.deepEqual(processesUsingPath(profile), [], 'real scope retained a process using its profile');
    await waitForClaimCount(0);
    const active = spawnSync('systemctl', ['--user', 'is-active', scopeUnit], { encoding: 'utf8' });
    assert.notEqual(active.status, 0, `real scope remained active: ${active.stdout}${active.stderr}`);
  } finally {
    if (scopeUnit) spawnSync('systemctl', ['--user', 'stop', scopeUnit], { stdio: 'ignore' });
    spawnSync('systemctl', ['--user', 'stop', slice], { stdio: 'ignore' });
    if (browserPid && processIsAlive(browserPid)) process.kill(browserPid, 'SIGTERM');
  }
}

async function runUnavailableSystemdFixture() {
  const root = join(fixtureRoot, 'systemd-unavailable');
  const bin = join(root, 'bin');
  mkdirSync(bin, { recursive: true });
  for (const tool of ['cargo', 'systemd-run']) {
    const toolPath = join(bin, tool);
    writeFileSync(toolPath, '#!/usr/bin/env bash\nexit 99\n');
    chmodSync(toolPath, 0o755);
  }
  const systemctlPath = join(bin, 'systemctl');
  writeFileSync(systemctlPath, '#!/usr/bin/env bash\nexit 1\n');
  chmodSync(systemctlPath, 0o755);

  const result = await executeCargo({
    overrides: {
      PATH: `${bin}:${process.env.PATH}`,
      AGENT_BROWSER_CARGO_FORCE_WSL: '1',
      AGENT_BROWSER_CARGO_CACHE: 'off',
      AGENT_BROWSER_CARGO_FAST_LINKER: 'off',
    },
  });
  assert.equal(result.status, 78, result.stderr);
  assert.match(result.stderr, /user systemd manager is unavailable/);
  await waitForClaimCount(0);
}

try {
  writeMeminfo();

  const first = probe({ hold: 1 });
  await waitForClaimCount(1);
  const second = probe();
  const secondResult = await second.completed;
  assert.equal(secondResult.status, 0, secondResult.stderr);
  assert.equal(JSON.parse(secondResult.stdout).admitted, true);
  assert.equal(first.child.exitCode, null, 'second admission should complete while first remains live');
  assert.equal((await first.completed).status, 0);
  await waitForClaimCount(0);

  const holders = [probe({ hold: 1 }), probe({ hold: 1 })];
  await waitForClaimCount(2);
  const thirdResult = await probe({ noWait: true }).completed;
  assert.equal(thirdResult.status, 75);
  assert.match(thirdResult.stderr, /reason=concurrency_limit/);
  await Promise.all(holders.map(({ completed }) => completed));
  await waitForClaimCount(0);

  writeMeminfo({ available: 20 * 1024 * 1024 });
  const pressure = await probe({ noWait: true }).completed;
  assert.equal(pressure.status, 75);
  assert.match(pressure.stderr, /reason=memory_pressure/);

  writeMeminfo({ available: 64 * 1024 * 1024, swapFree: 1 });
  const historicalSwap = await probe({ noWait: true }).completed;
  assert.equal(historicalSwap.status, 0, historicalSwap.stderr);
  assert.equal(JSON.parse(historicalSwap.stdout).admitted, true);

  writeMeminfo({ available: 31 * 1024 * 1024, swapFree: 1 });
  const currentSwapPressure = await probe({ noWait: true }).completed;
  assert.equal(currentSwapPressure.status, 75);
  assert.match(currentSwapPressure.stderr, /reason=swap_pressure/);

  writeMeminfo();
  const claimsDir = join(admissionDir, 'claims');
  writeFileSync(join(claimsDir, 'stale.claim'), 'pid=999999\nstart=1\n');
  const stale = await probe({ noWait: true }).completed;
  assert.equal(stale.status, 0, stale.stderr);
  assert.equal(JSON.parse(stale.stdout).admitted, true);

  const accelerationBin = join(fixtureRoot, 'acceleration-bin');
  mkdirSync(accelerationBin);
  for (const tool of ['sccache', 'mold']) {
    const toolPath = join(accelerationBin, tool);
    writeFileSync(toolPath, '#!/bin/sh\nexit 0\n');
    chmodSync(toolPath, 0o755);
  }
  const accelerated = await probe({
    noWait: true,
    overrides: { PATH: `${accelerationBin}:${process.env.PATH}` },
  }).completed;
  assert.equal(accelerated.status, 0, accelerated.stderr);
  assert.deepEqual(JSON.parse(accelerated.stdout).acceleration, {
    cache: 'sccache',
    linker: 'mold',
  });

  const cargoPath = join(accelerationBin, 'cargo');
  writeFileSync(cargoPath, '#!/bin/sh\nprintf \'%s|%s|%s\\n\' "$CARGO_BUILD_JOBS" "$RUSTC_WRAPPER" "$RUSTFLAGS"\n');
  chmodSync(cargoPath, 0o755);
  const unamePath = join(accelerationBin, 'uname');
  writeFileSync(unamePath, '#!/bin/sh\nprintf \'Linux\\n\'\n');
  chmodSync(unamePath, 0o755);

  const executed = await executeCargo({
    overrides: { PATH: `${accelerationBin}:${process.env.PATH}` },
  });
  assert.equal(executed.status, 0, executed.stderr);
  assert.equal(executed.stdout.trim(), `8|${sanitizedCacheWrapper}|-C link-arg=-fuse-ld=mold`);
  assert.match(executed.stderr, /Running Cargo without WSL admission: jobs=8 cache=sccache linker=mold/);

  const observingCache = join(accelerationBin, 'observing-sccache');
  writeFileSync(
    observingCache,
    '#!/bin/sh\nprintf \'%s|%s|%s\\n\' "${P158_TEST_API_KEY:-}" "${P158_TEST_REFRESH:-}" "${CARGO_BUILD_JOBS:-}"\n',
  );
  chmodSync(observingCache, 0o755);
  const sanitized = spawnSync(sanitizedCacheWrapper, ['rustc'], {
    encoding: 'utf8',
    env: {
      ...process.env,
      AGENT_BROWSER_SCCACHE_EXECUTABLE: observingCache,
      P158_TEST_API_KEY: 'must-not-reach-sccache',
      P158_TEST_REFRESH: 'must-also-be-removed',
      CARGO_BUILD_JOBS: '8',
    },
  });
  assert.equal(sanitized.status, 0, sanitized.stderr);
  assert.equal(sanitized.stdout.trim(), '||8');

  const optedOut = await executeCargo({
    overrides: {
      PATH: `${accelerationBin}:${process.env.PATH}`,
      AGENT_BROWSER_CARGO_CACHE: 'off',
      AGENT_BROWSER_CARGO_FAST_LINKER: 'off',
    },
  });
  assert.equal(optedOut.status, 0, optedOut.stderr);
  assert.equal(optedOut.stdout.trim(), '8||');
  assert.match(optedOut.stderr, /cache=none linker=none/);

  writeMeminfo();
  await runScopeDescendantFixture(0);
  await runScopeDescendantFixture(23);
  await runLingeringScopeAccountingFixture();
  await runUnavailableSystemdFixture();
  if (process.env.AGENT_BROWSER_CARGO_REAL_SCOPE_TEST === '1') {
    await runRealScopeDescendantFixture();
  }

  console.log('Cargo capacity admission tests passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}
