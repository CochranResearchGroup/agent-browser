#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  normalizeServiceTraceData,
  traceFilterSummary,
  traceTimelineItems,
} from '../packages/dashboard/src/lib/service-trace.ts';
import {
  incidentPriorityView,
} from '../packages/dashboard/src/lib/service-incidents.ts';

const traceData = {
  filters: {
    serviceName: 'JournalDownloader',
    agentName: 'agent-a',
    taskName: 'probeACSwebsite',
    browserId: 'browser-1',
    profileId: 'profile-1',
    sessionId: 'session-1',
    since: '2026-04-25T12:00:00Z',
    limit: 20,
  },
  events: [
    {
      id: 'event-seen',
      timestamp: '2026-04-25T12:01:00Z',
      kind: 'browser_health_changed',
      message: 'Browser recovered',
      browserId: 'browser-1',
      profileId: 'profile-1',
      sessionId: 'session-1',
      serviceName: 'JournalDownloader',
      agentName: 'agent-a',
      taskName: 'probeACSwebsite',
    },
    {
      id: 'event-standalone',
      timestamp: '2026-04-25T12:04:00Z',
      kind: 'tab_lifecycle_changed',
      message: 'Tab title changed',
      browserId: 'browser-1',
      serviceName: 'JournalDownloader',
      taskName: 'probeACSwebsite',
    },
  ],
  jobs: [
    {
      id: 'job-seen',
      action: 'snapshot',
      state: 'succeeded',
      completedAt: '2026-04-25T12:02:00Z',
      serviceName: 'JournalDownloader',
      agentName: 'agent-a',
      taskName: 'probeACSwebsite',
    },
    {
      id: 'job-standalone',
      action: 'wait',
      state: 'failed',
      submittedAt: '2026-04-25T12:03:00Z',
      error: 'Wait timed out',
      serviceName: 'JournalDownloader',
      agentName: 'agent-a',
      taskName: 'probeACSwebsite',
    },
  ],
  incidents: [],
  activity: [
    {
      id: 'activity-event-seen',
      source: 'event',
      eventId: 'event-seen',
      timestamp: '2026-04-25T12:01:00Z',
      kind: 'browser_health_changed',
      title: 'unreachable to ready',
      message: 'Browser recovered',
      browserId: 'browser-1',
      serviceName: 'JournalDownloader',
      taskName: 'probeACSwebsite',
    },
    {
      id: 'activity-job-seen',
      source: 'job',
      jobId: 'job-seen',
      timestamp: '2026-04-25T12:02:00Z',
      kind: 'succeeded',
      title: 'snapshot',
      message: 'job-seen',
      serviceName: 'JournalDownloader',
      agentName: 'agent-a',
      taskName: 'probeACSwebsite',
    },
  ],
};

const mcpToolPayload = {
  tool: 'service_trace',
  success: true,
  data: traceData,
  error: null,
};

assert.equal(normalizeServiceTraceData(traceData), traceData);
assert.equal(normalizeServiceTraceData(mcpToolPayload), traceData);
assert.equal(normalizeServiceTraceData(null), null);

const summary = traceFilterSummary(traceData.filters);
for (const expected of [
  'service JournalDownloader',
  'agent agent-a',
  'task probeACSwebsite',
  'browser browser-1',
  'profile profile-1',
  'session session-1',
  'since 2026-04-25T12:00:00Z',
  'limit 20',
]) {
  assert(summary.includes(expected), `Trace filter summary missing ${expected}`);
}

const timeline = traceTimelineItems(traceData);
assert.deepEqual(
  timeline.map((item) => item.id),
  ['trace-event-event-standalone', 'trace-job-job-standalone', 'activity-job-seen', 'activity-event-seen'],
);
assert.equal(
  timeline.some((item) => item.id === 'trace-event-event-seen'),
  false,
  'Activity-backed events should not be duplicated',
);
assert.equal(
  timeline.some((item) => item.id === 'trace-job-job-seen'),
  false,
  'Activity-backed jobs should not be duplicated',
);
assert.equal(timeline[0].source, 'event');
assert.equal(timeline[1].source, 'job');
assert.equal(timeline[1].message, 'Wait timed out');

const criticalIncidentPriority = incidentPriorityView({
  label: 'browser-1',
  severity: 'critical',
  escalation: 'os_degraded_possible',
  recommendedAction: 'Inspect the host OS and process table before retrying browser automation.',
});
assert.equal(criticalIncidentPriority.severityTone, 'critical');
assert.equal(criticalIncidentPriority.severityLabel, 'critical');
assert.equal(criticalIncidentPriority.escalationLabel, 'os degraded possible');
assert.equal(
  criticalIncidentPriority.recommendedAction,
  'Inspect the host OS and process table before retrying browser automation.',
);
assert.equal(
  criticalIncidentPriority.ariaLabel,
  'Inspect critical incident for browser-1',
);

console.log('Dashboard service trace contract smoke passed');
