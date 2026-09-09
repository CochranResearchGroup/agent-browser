#!/usr/bin/env node

import { spawnSync } from 'node:child_process';

const tests = [
  'test_cdp_free_launch_plan_is_no_devtools_headed_lifecycle_only',
  'test_cdp_free_launch_response_reports_unsupported_cdp_operations',
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
