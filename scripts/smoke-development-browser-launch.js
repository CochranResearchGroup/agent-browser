#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
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
