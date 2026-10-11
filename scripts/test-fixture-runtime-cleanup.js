#!/usr/bin/env node
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-fixture-cleanup-'));
const home = join(root, 'home');
const nonce = randomUUID();
mkdirSync(home);
try {
  for (const test of [
    { home, nonce: randomUUID(), expectedStatus: 0 },
    { home: root, nonce, expectedStatus: 0 },
    { home, nonce, expectedStatus: 1 },
  ]) {
    const environment = Object.fromEntries(
      Object.entries(process.env).filter(([key]) => !key.startsWith('AGENT_BROWSER_')),
    );
    const child = spawn('/usr/bin/sleep', ['60'], {
      env: {
        ...environment,
        HOME: test.home,
        AGENT_BROWSER_FIXTURE_RUN_ID: test.nonce,
        AGENT_BROWSER_RUNTIME_HOST: '1',
      },
      stdio: 'ignore',
    });
    try {
      await once(child, 'spawn');
      const result = spawnSync('/usr/bin/python3', [
        join(import.meta.dirname, 'lib', 'fixture-runtime-cleanup.py'),
        home, nonce, '0'.repeat(64),
      ], { encoding: 'utf8', timeout: 15_000 });
      assert.equal(result.status, test.expectedStatus, `${result.stdout}${result.stderr}`);
      assert.doesNotThrow(() => process.kill(child.pid, 0), 'foreign or unproven executable must survive');
      assert.ok(existsSync(home), 'failed cleanup must preserve its evidence home');
      if (result.status === 0) {
        assert.deepEqual(JSON.parse(result.stdout), { retired: [], residue: [] });
      }
    } finally {
      const exited = once(child, 'exit');
      child.kill('SIGTERM');
      await exited;
      assert.throws(() => process.kill(child.pid, 0), { code: 'ESRCH' });
    }
  }
  console.log('Fixture runtime cleanup fencing checks passed');
} finally {
  rmSync(root, { recursive: true, force: true });
}
