#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  incidentSummaryGroupView,
  incidentSummaryGroupViews,
} from '../packages/dashboard/src/lib/service-incident-summary.ts';

const groups = [
  {
    escalation: 'browser_degraded',
    severity: 'warning',
    state: 'active',
    count: 7,
    latestTimestamp: '2026-04-25T12:04:00Z',
    recommendedAction: 'Retry the browser after checking CDP health.',
    incidentIds: ['warning-1', 'warning-2', 'warning-3', 'warning-4', 'warning-5'],
  },
  {
    escalation: 'os_degraded_possible',
    severity: 'critical',
    state: 'active',
    count: 1,
    latestTimestamp: '2026-04-25T12:03:00Z',
    recommendedAction: 'Inspect the host OS and process table before retrying browser automation.',
    incidentIds: ['critical-1'],
  },
  {
    escalation: 'job_attention',
    severity: 'error',
    state: 'service',
    count: 3,
    latestTimestamp: '2026-04-25T12:05:00Z',
    recommendedAction: 'Inspect cancelled or timed-out jobs.',
    incidentIds: ['error-1', 'error-2', 'error-3'],
  },
  {
    escalation: 'service_triage',
    severity: 'error',
    state: 'active',
    count: 4,
    latestTimestamp: '2026-04-25T12:02:00Z',
    recommendedAction: 'Inspect service reconciliation errors.',
    incidentIds: ['error-4'],
  },
];

const rows = incidentSummaryGroupViews(groups);

assert.deepEqual(
  rows.map((row) => row.key),
  [
    'os_degraded_possible-critical-active',
    'service_triage-error-active',
    'job_attention-error-service',
    'browser_degraded-warning-active',
  ],
);
assert.equal(rows[0].severityTone, 'critical');
assert.equal(rows[0].severityLabel, 'critical');
assert.equal(rows[0].escalationLabel, 'os degraded possible');
assert.equal(rows[0].stateLabel, 'active');
assert.equal(rows[0].count, 1);
assert.equal(
  rows[0].recommendedAction,
  'Inspect the host OS and process table before retrying browser automation.',
);
assert.equal(rows[3].incidentIdLabel, 'warning-1 / warning-2 / warning-3 / warning-4 +1');

const fallbackRow = incidentSummaryGroupView({
  severity: null,
  escalation: null,
  state: null,
});
assert.equal(fallbackRow.key, 'unknown-unknown-unknown');
assert.equal(fallbackRow.severityTone, 'info');
assert.equal(fallbackRow.severityLabel, 'unknown');
assert.equal(fallbackRow.escalationLabel, 'unknown');
assert.equal(fallbackRow.stateLabel, 'unknown');
assert.equal(fallbackRow.count, 0);
assert.equal(fallbackRow.recommendedAction, 'Inspect incident details.');
assert.equal(fallbackRow.incidentIdLabel, 'No incident IDs');

console.log('Dashboard incident summary contract smoke passed');
