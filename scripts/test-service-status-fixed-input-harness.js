#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  closeSync,
  mkdtempSync,
  openSync,
  readFileSync,
  rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { getServiceStatus } from '../packages/client/src/service-observability.js';

const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-fixed-status-'));
try {
  const stdoutPath = join(fixture, 'cargo.stdout');
  const stderrPath = join(fixture, 'cargo.stderr');
  const stdoutFd = openSync(stdoutPath, 'w');
  const stderrFd = openSync(stderrPath, 'w');
  let run;
  try {
    run = spawnSync(
      'scripts/ci/cargo-safe.sh',
      [
        'test',
        '--manifest-path',
        'cli/Cargo.toml',
        'fixed_input_harness_crosses_real_status_entries_and_transports',
        '--',
        '--nocapture',
        '--test-threads=1',
      ],
      {
        cwd: new URL('..', import.meta.url),
        env: {
          ...process.env,
          AGENT_BROWSER_EMIT_FIXED_STATUS_HARNESS: '1',
        },
        stdio: ['ignore', stdoutFd, stderrFd],
      },
    );
  } finally {
    closeSync(stdoutFd);
    closeSync(stderrFd);
  }

  const stdout = readFileSync(stdoutPath, 'utf8');
  const stderr = readFileSync(stderrPath, 'utf8');
  assert.equal(
    run.status,
    0,
    `fixed-input Rust harness failed\n${stdout}\n${stderr}`,
  );
  const output = `${stdout}\n${stderr}`;
  const marker = 'AGENT_BROWSER_FIXED_STATUS_DATA=';
  const line = output.split('\n').find((candidate) => candidate.includes(marker));
  assert(line, `fixed-input Rust harness did not emit canonical data\n${output}`);
  const canonical = JSON.parse(line.slice(line.indexOf(marker) + marker.length));

  const decoded = await getServiceStatus({
    baseUrl: 'http://fixed-status.invalid',
    fetch: async () => ({
      ok: true,
      json: async () => ({ success: true, data: canonical }),
    }),
  });

  assert.deepEqual(decoded, canonical);
  assert.equal(decoded.statusProjection?.schemaVersion, 1);
  assert.equal(
    decoded.statusProjection?.authority?.projectedAt,
    '2026-08-10T12:00:05.000Z',
  );

  console.log('Service Status fixed-input producer and generated-client harness passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
