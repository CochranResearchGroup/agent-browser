#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

const generator = new URL('./build-policy-capsule.mjs', import.meta.url).pathname;
const root = mkdtempSync(join(tmpdir(), 'policy-capsule-'));
mkdirSync(join(root, 'policies'));
writeFileSync(join(root, 'policies/a.md'), '# A\n\nCanonical rule.\n');

const base = {
  schemaVersion: 'agent-browser.policy-capsule-profile.v1',
  scope: 'fixture scope',
  authority: 'fixture authority',
  policies: ['policies/a.md'],
  operativeRules: ['Apply the fixture rule.'],
  requiredChecks: ['fixture-check'],
  hardStops: ['Stop on fixture drift.'],
  rereadWhen: ['A selected hash changes.'],
};
const profile = join(root, 'profile.json');
const output = join(root, 'capsule.md');

function run(args = [], expected = 0) {
  const result = spawnSync(process.execPath, [generator, '--root', root, '--profile', 'profile.json', '--output', 'capsule.md', ...args], { encoding: 'utf8' });
  assert.equal(result.status, expected, result.stderr || result.stdout);
  return result;
}

writeFileSync(profile, JSON.stringify(base));
run();
const first = readFileSync(output, 'utf8');
run();
assert.equal(readFileSync(output, 'utf8'), first, 'generation must be deterministic');
run(['--check']);

writeFileSync(join(root, 'policies/a.md'), '# A\n\nChanged canonical rule.\n');
assert.match(run(['--check'], 1).stderr, /policy capsule stale/);
run();
run(['--check']);

writeFileSync(profile, JSON.stringify({ ...base, policies: ['policies/missing.md'] }));
assert.notEqual(run([], 1).stderr.indexOf('ENOENT'), -1);

for (const field of ['policies', 'operativeRules', 'requiredChecks', 'hardStops', 'rereadWhen']) {
  writeFileSync(profile, JSON.stringify({ ...base, [field]: [] }));
  assert.match(run([], 1).stderr, new RegExp(`empty policy capsule field: ${field}`));
}

writeFileSync(profile, JSON.stringify({ ...base, schemaVersion: 'future' }));
assert.match(run([], 1).stderr, /unsupported policy capsule profile/);

console.log('Policy capsule tests passed');
