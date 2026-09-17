#!/usr/bin/env node

import assert from 'node:assert/strict';

import { economicsMarkdown, summarizeValidationEconomics } from './lib/validation-economics.js';

const summary = summarizeValidationEconomics({
  selection: {
    tier: 'focused',
    jobs: { docs: false, rustQuality: true, rust: true },
    exclusions: ['dashboard not selected'],
  },
  jobs: [
    { name: 'Validation Selection', started_at: '2026-09-16T11:59:00Z', completed_at: '2026-09-16T12:00:00Z' },
    { name: 'Rust Quality', started_at: '2026-09-16T12:00:00Z', completed_at: '2026-09-16T12:02:00Z' },
    { name: 'Rust', started_at: '2026-09-16T12:02:00Z', completed_at: '2026-09-16T12:07:00Z' },
    { name: 'Dashboard', started_at: '2026-09-16T12:00:00Z', completed_at: null },
  ],
});
assert.equal(summary.wallSeconds, 480);
assert.equal(summary.observedRunnerSeconds, 480);
assert.equal(summary.observedRunnerMinutes, 8);
assert.deepEqual(summary.selectedJobs, ['Validation Selection', 'Rust Quality', 'Rust']);
assert.match(economicsMarkdown(summary), /Tier: `focused`/);
assert.match(economicsMarkdown(summary), /not billing-exact/);
assert.match(economicsMarkdown(summary), /excludes Presubmit itself/);

assert.throws(
  () => summarizeValidationEconomics({ selection: { jobs: { docs: true } }, jobs: [] }),
  /Validation Selection, Documentation/,
);

console.log('Validation economics checks passed');
