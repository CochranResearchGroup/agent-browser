#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const wrapper = join(repoRoot, 'scripts', 'ci', 'cargo-safe.sh');
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

  console.log('Cargo capacity admission tests passed');
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}
