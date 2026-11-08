import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

// Execute the maintained public entrypoint in a fresh checkout. Tool doubles
// retain Cargo's real fallback behavior; no compiler or browser is launched.
const command = JSON.parse(readFileSync(new URL('../package.json', import.meta.url))).scripts['build:development-candidate'];
const root = mkdtempSync(join(tmpdir(), 'development-candidate-build-'));
try {
  mkdirSync(join(root, 'scripts/ci'), { recursive: true });
  writeFileSync(join(root, 'package.json'), JSON.stringify({ scripts: {
    'build:development-candidate': command,
    'version:sync': 'node stage.cjs version',
    'build:dashboard': 'node stage.cjs dashboard',
  } }));
  writeFileSync(join(root, 'scripts/ci/cargo-safe.sh'), '#!/bin/sh\nexec node stage.cjs cargo\n', { mode: 0o755 });
  writeFileSync(join(root, 'stage.cjs'), `
const fs = require('node:fs');
const stage = process.argv[2];
fs.appendFileSync('trace', stage + '\\n');
fs.mkdirSync('packages/dashboard/out', {recursive:true});
if (stage === 'dashboard') {
  if (process.env.FAIL_DASHBOARD === '1') process.exit(1);
  fs.writeFileSync('packages/dashboard/out/index.html', '<script src="/_next/static/app.js"></script>');
}
if (stage === 'cargo' && !fs.existsSync('packages/dashboard/out/index.html'))
  fs.writeFileSync('packages/dashboard/out/index.html', 'Dashboard not built. Run: pnpm build');
`);
  const run = spawnSync('pnpm', ['run', 'build:development-candidate'], { cwd: root, encoding: 'utf8' });
  assert.equal(run.status, 0, run.stderr);
  assert.doesNotMatch(readFileSync(join(root, 'packages/dashboard/out/index.html'), 'utf8'), /Dashboard not built/, 'fresh candidate must embed the frontend, not Cargo fallback');
  assert.deepEqual(readFileSync(join(root, 'trace'), 'utf8').trim().split('\n'), ['version', 'dashboard', 'cargo']);
  rmSync(join(root, 'trace'));
  const failed = spawnSync('pnpm', ['run', 'build:development-candidate'], { cwd: root, encoding: 'utf8', env: { ...process.env, FAIL_DASHBOARD: '1' } });
  assert.notEqual(failed.status, 0, 'frontend failure must stop the candidate build');
  assert.deepEqual(readFileSync(join(root, 'trace'), 'utf8').trim().split('\n'), ['version', 'dashboard']);
  console.log('Fresh development candidate embeds dashboard; frontend failure prevents Cargo');
} finally { rmSync(root, { recursive: true, force: true }); }
