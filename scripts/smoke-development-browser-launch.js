#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import {
  assertProductionUnchanged,
  developmentRuntimeDescriptor,
  productionSnapshot,
} from './lib/development-runtime.js';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const iterations = Number(takeOption(args, '--iterations') || 3);
const json = removeFlag(args, '--json');
if (!Number.isInteger(iterations) || iterations < 1 || iterations > 10) {
  throw new Error('--iterations must be an integer from 1 through 10');
}
if (args.length) throw new Error(`Unknown arguments: ${args.join(' ')}`);

const descriptor = developmentRuntimeDescriptor();
const productionBefore = productionSnapshot();
const runId = `${Date.now()}-${process.pid}`;
const results = [];

waitForServiceStateReady();

for (let index = 1; index <= iterations; index += 1) {
  const identity = `p126-fresh-${runId}-${index}`;
  const profileRoot = resolve(descriptor.stateDir, 'runtime-profiles', identity);
  assert.ok(profileRoot.startsWith(`${resolve(descriptor.stateDir)}/`));
  try {
    const opened = runJson([
      '--session', identity,
      '--runtime-profile', identity,
      '--json',
      'open',
      'about:blank',
    ]);
    assert.equal(opened.success, true);
    assert.equal(opened.data?.url, 'about:blank');
    const url = runJson([
      '--session', identity,
      '--runtime-profile', identity,
      '--json',
      'get',
      'url',
    ]);
    assert.equal(url.success, true);
    assert.equal(url.data?.url, 'about:blank');
    const closed = runJson([
      '--session', identity,
      '--runtime-profile', identity,
      '--json',
      'close',
    ]);
    assert.equal(closed.success, true);
    assert.equal(closed.data?.closed, true);
    waitForNoProcess(identity);
    results.push({ identity, opened: true, url: 'about:blank', closed: true });
  } finally {
    try {
      runJson(['--session', identity, '--runtime-profile', identity, '--json', 'close']);
    } catch {
      // The exact session may already be absent after a successful close.
    }
    waitForNoProcess(identity);
    if (existsSync(profileRoot)) execFileSync('gio', ['trash', profileRoot]);
  }
}

const productionAfter = productionSnapshot();
assertProductionUnchanged(productionBefore, productionAfter);
const report = {
  success: true,
  runtimeEnvironment: descriptor.environment,
  browserExecutable: descriptor.browserExecutable,
  iterations,
  results,
  productionUnchanged: true,
};
if (json) console.log(JSON.stringify(report, null, 2));
else console.log(`Development browser launch smoke passed: iterations=${iterations}`);

function runJson(commandArgs) {
  const output = execFileSync(descriptor.executable, [
    '--service-state-lock-timeout-ms',
    '10000',
    ...commandArgs,
  ], {
    encoding: 'utf8',
    timeout: 45_000,
  });
  return JSON.parse(output.trim());
}

function waitForNoProcess(identity) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const output = execFileSync('ps', ['-eo', 'args='], { encoding: 'utf8' });
    if (!output.includes(identity)) return;
    execFileSync('sleep', ['0.1']);
  }
  throw new Error(`Development browser process remained after close: ${identity}`);
}

function waitForServiceStateReady() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    const result = spawnSync(descriptor.executable, [
      '--service-state-lock-timeout-ms',
      '10000',
      '--json',
      'service',
      'status',
    ], {
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
      timeout: 15_000,
    });
    let response = null;
    try {
      response = JSON.parse(result.stdout.trim());
    } catch {
      // The structured failure below distinguishes transport errors from lock contention.
    }
    if (result.status === 0) {
      if (response.success === true) return;
    }
    const error = typeof response?.error === 'string'
      ? response.error
      : result.error?.message || result.stderr || 'unknown preflight failure';
    if (!error.startsWith('service_state_lock_timeout:')) {
      throw new Error(`Development Service State preflight failed: ${error}`);
    }
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
  }
  throw new Error('Development Service State did not become readable within 30 seconds');
}

function takeOption(values, option) {
  const index = values.indexOf(option);
  if (index < 0) return null;
  if (!values[index + 1]) throw new Error(`${option} requires a value`);
  return values.splice(index, 2)[1];
}

function removeFlag(values, flag) {
  const index = values.indexOf(flag);
  if (index < 0) return false;
  values.splice(index, 1);
  return true;
}
