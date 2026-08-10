#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

import { getServiceStatus } from '../packages/client/src/service-observability.js';

const run = spawnSync(
  'cargo',
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
    encoding: 'utf8',
    env: {
      ...process.env,
      AGENT_BROWSER_EMIT_FIXED_STATUS_HARNESS: '1',
    },
    maxBuffer: 16 * 1024 * 1024,
  },
);

assert.equal(
  run.status,
  0,
  `fixed-input Rust harness failed\n${run.stdout}\n${run.stderr}`,
);
const output = `${run.stdout}\n${run.stderr}`;
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
