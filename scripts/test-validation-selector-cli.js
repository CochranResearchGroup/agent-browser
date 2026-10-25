#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

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

  // Exercise the public CLI against a real changed-file diff, not private selectors.
  let selectionFixture = 0;
  function reportForChangedPaths(paths) {
    const before = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: fixtureRepo, encoding: 'utf8' }).trim();
    selectionFixture += 1;
    for (const file of paths) {
      mkdirSync(dirname(join(fixtureRepo, file)), { recursive: true });
      writeFileSync(join(fixtureRepo, file), `// selection fixture ${selectionFixture}\n`);
    }
    execFileSync('git', ['add', ...paths], { cwd: fixtureRepo });
    execFileSync('git', ['commit', '-m', 'selection fixture'], { cwd: fixtureRepo, stdio: 'pipe' });
    return JSON.parse(execFileSync('node', [
      join(process.cwd(), 'scripts/dev/select-validation.js'),
      '--base', before, '--head', 'HEAD', '--ci', '--json',
    ], { cwd: fixtureRepo, encoding: 'utf8' }));
  }
  const startup = reportForChangedPaths(['packages/dashboard/src/lib/dashboard-read-coordinator.ts']);
  const commands = startup.recommendations.map(check => check.command);
  assert.ok(commands.includes('pnpm test:dashboard-workspace-startup'),
    'polling changes must recommend the existing mounted hook/cache regression');
  const startupChecks = [
    'pnpm test:dashboard-read-coordinator',
    'pnpm test:dashboard-workspace-startup',
    'pnpm test:dashboard-selected-workspace-context',
  ];
  function assertStartupChecks(report, label) {
    const selected = report.recommendations.map(check => check.command);
    for (const command of startupChecks) {
      assert.equal(selected.filter(value => value === command).length, 1,
        `${label} must recommend ${command} exactly once`);
      const buildIndex = selected.indexOf('pnpm build:dashboard');
      if (buildIndex >= 0) assert.ok(selected.indexOf(command) < buildIndex,
        `${label} must recommend focused startup checks before the dashboard build`);
    }
  }
  assertStartupChecks(startup, 'poll coordinator');
  assert.equal(startup.tier, 'dashboard');
  for (const file of [
    'packages/dashboard/src/lib/dashboard-api.ts',
    'packages/dashboard/src/lib/selected-workspace-context.ts',
    'packages/dashboard/src/hooks/use-selected-workspace-context.ts',
    'scripts/test-dashboard-read-coordinator.ts',
    'scripts/test-dashboard-workspace-startup.cjs',
    'scripts/test-dashboard-selected-workspace-context.js',
    'scripts/fixtures/workspace-startup.tsx',
  ]) assertStartupChecks(reportForChangedPaths([file]), file);
  assertStartupChecks(reportForChangedPaths([
    'packages/dashboard/src/lib/dashboard-read-coordinator.ts',
    'packages/dashboard/src/lib/selected-workspace-context.ts',
    'scripts/fixtures/workspace-startup.tsx',
  ]), 'combined startup changes');
  const unrelated = reportForChangedPaths(['packages/dashboard/src/lib/unrelated.ts']);
  const unrelatedCommands = unrelated.recommendations.map(check => check.command);
  assert.equal(unrelated.tier, 'dashboard');
  assert.ok(unrelatedCommands.includes('pnpm build:dashboard'), 'ordinary dashboard checks remain');
  for (const command of startupChecks) assert.ok(!unrelatedCommands.includes(command),
    'an unrelated dashboard file must not select the startup suites');
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Validation selector CLI checks passed');
