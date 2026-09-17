#!/usr/bin/env node

import assert from 'node:assert/strict';

import { PRESUBMIT_JOB_KEYS, verifyPresubmit } from './lib/presubmit-verifier.js';

function selection(overrides = {}) {
  return {
    schemaVersion: 'agent-browser.validation-selection.v2',
    tier: 'docs',
    jobs: Object.fromEntries(PRESUBMIT_JOB_KEYS.map((key) => [key, key === 'docs'])),
    unknownFiles: [],
    ...overrides,
  };
}

function results(overrides = {}) {
  return {
    ...Object.fromEntries(PRESUBMIT_JOB_KEYS.map((key) => [key, key === 'docs' ? 'success' : 'skipped'])),
    ...overrides,
  };
}

const passed = verifyPresubmit({ selection: selection(), results: results() });
assert.deepEqual(passed.selected, ['docs']);
assert.deepEqual(passed.excluded, PRESUBMIT_JOB_KEYS.filter((key) => key !== 'docs'));

assert.throws(
  () => verifyPresubmit({ selection: selection(), results: results({ docs: 'skipped' }) }),
  /selected presubmit job docs ended with skipped/,
);
assert.throws(
  () => verifyPresubmit({ selection: selection(), results: results({ docs: 'cancelled' }) }),
  /selected presubmit job docs ended with cancelled/,
);
assert.throws(
  () => verifyPresubmit({ selection: selection(), results: results({ rust: 'success' }) }),
  /unselected presubmit job rust ended with success/,
);
assert.throws(
  () => verifyPresubmit({ selection: selection({ schemaVersion: 'unknown' }), results: results() }),
  /unsupported validation selection schema/,
);
assert.throws(
  () => verifyPresubmit({ selection: selection({ jobs: { ...selection().jobs, surprise: false } }), results: results() }),
  /unknown jobs: surprise/,
);
assert.throws(
  () => verifyPresubmit({ selection: selection(), results: { ...results(), workstation: 'queued' } }),
  /unexpected result: queued/,
);

console.log('Presubmit aggregate verifier checks passed');
