#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-validation-selector-'));
try {
  const githubOutput = join(root, 'github-output');
  const report = JSON.parse(execFileSync('node', [
    'scripts/dev/select-validation.js',
    '--base', 'HEAD',
    '--head', 'HEAD',
    '--ci',
    '--json',
    '--github-output', githubOutput,
  ], { encoding: 'utf8' }));
  assert.equal(report.schemaVersion, 'agent-browser.validation-selection.v2');
  assert.equal(report.tier, 'none');
  assert.deepEqual(report.changedFiles, []);
  assert.equal(report.jobs.versionSync, true);
  assert.equal(Object.hasOwn(report.jobs, 'comprehensive'), false);
  const outputs = Object.fromEntries(
    readFileSync(githubOutput, 'utf8').trim().split('\n').map((line) => {
      const separator = line.indexOf('=');
      return [line.slice(0, separator), line.slice(separator + 1)];
    }),
  );
  assert.equal(outputs.tier, 'none');
  assert.equal(outputs.version_sync, 'true');
  assert.equal(outputs.rust, 'false');
  assert.equal(outputs.repository_tooling, 'false');
  assert.equal(outputs.service_smokes, 'false');
  assert.equal(Object.hasOwn(outputs, 'comprehensive'), false);
  assert.equal(outputs.rust_compartments, '[]');
  assert.equal(JSON.parse(outputs.selection).schemaVersion, report.schemaVersion);

  assert.throws(
    () => execFileSync('node', [
      'scripts/dev/select-validation.js',
      '--base', 'HEAD',
      '--qualification', 'comprehensive',
    ], { encoding: 'utf8', stdio: 'pipe' }),
    (error) => error.status === 2 && /no full-suite route/.test(String(error.stderr)),
  );

  const fixtureRepo = join(root, 'rename-repo');
  mkdirSync(join(fixtureRepo, 'docs'), { recursive: true });
  for (const gitArgs of [
    ['init'],
    ['config', 'user.email', 'validation-fixture@example.invalid'],
    ['config', 'user.name', 'Validation Fixture'],
  ]) execFileSync('git', gitArgs, { cwd: fixtureRepo, stdio: 'pipe' });
  writeFileSync(join(fixtureRepo, 'docs', 'before.md'), '# Before\n');
  execFileSync('git', ['add', '.'], { cwd: fixtureRepo });
  execFileSync('git', ['commit', '-m', 'fixture base'], { cwd: fixtureRepo, stdio: 'pipe' });
  const base = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: fixtureRepo, encoding: 'utf8' }).trim();
  renameSync(join(fixtureRepo, 'docs', 'before.md'), join(fixtureRepo, 'docs', 'after.md'));
  execFileSync('git', ['add', '.'], { cwd: fixtureRepo });
  execFileSync('git', ['commit', '-m', 'fixture rename'], { cwd: fixtureRepo, stdio: 'pipe' });
  const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: fixtureRepo, encoding: 'utf8' }).trim();
  const renameReport = JSON.parse(execFileSync('node', [
    join(process.cwd(), 'scripts/dev/select-validation.js'),
    '--base', base,
    '--head', head,
    '--ci',
    '--json',
  ], { cwd: fixtureRepo, encoding: 'utf8' }));
  assert.deepEqual(renameReport.changedFiles, ['docs/after.md', 'docs/before.md']);
  assert.equal(renameReport.tier, 'docs');
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Validation selector CLI checks passed');
