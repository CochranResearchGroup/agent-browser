#!/usr/bin/env node

import { spawnSync } from 'node:child_process';

const tests = [
  'service_request_command_rejects_cdp_free_without_non_cdp_execution',
  'service_request_command_accepts_cdp_free_launch',
];

for (const testName of tests) {
  const result = spawnSync(
    'cargo',
    ['test', '--manifest-path', 'cli/Cargo.toml', testName, '--', '--test-threads=1'],
    {
      encoding: 'utf8',
      stdio: 'pipe',
    },
  );

  if (result.status !== 0) {
    process.stdout.write(result.stdout ?? '');
    process.stderr.write(result.stderr ?? '');
    process.exit(result.status ?? 1);
  }

  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  if (!output.includes(testName)) {
    console.error(`Expected cargo test output to include ${testName}`);
    process.exit(1);
  }
}

console.log('Service request CDP-free no-launch smoke passed');
