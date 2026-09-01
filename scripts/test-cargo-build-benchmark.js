#!/usr/bin/env node

import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const script = resolve(repoRoot, 'scripts', 'benchmark-cargo-build-jobs.js');

const planned = spawnSync(process.execPath, [script, '--plan', '--jobs', '4,6,8'], {
  cwd: repoRoot,
  encoding: 'utf8',
});
assert.equal(planned.status, 0, planned.stderr);
assert.deepEqual(JSON.parse(planned.stdout), {
  jobs: [4, 6, 8],
  isolatedTargetDirectories: true,
  sharedTargetPreserved: true,
  cargoCache: 'off',
  fastLinker: 'off',
  memoryLimitKib: 24 * 1024 * 1024,
});

const invalid = spawnSync(process.execPath, [script, '--plan', '--jobs', '4,nope'], {
  cwd: repoRoot,
  encoding: 'utf8',
});
assert.equal(invalid.status, 2);
assert.match(invalid.stderr, /positive integers/);

console.log('Cargo build benchmark contract tests passed');
