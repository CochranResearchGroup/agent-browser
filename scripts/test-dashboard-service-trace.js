#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  normalizeServiceTraceData,
  traceBrowserCapabilityLaunches,
  traceFilterSummary,
  traceProfileLeaseWaits,
  traceSummaryCards,
  traceSummaryContexts,
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
  summary: {
    contextCount: 2,
    hasTraceContext: true,
    namingWarningCount: 1,
    browserCapabilityLaunches: {
      count: 2,
      appliedCount: 1,
      skippedCount: 1,
      launches: [
        {
          source: 'session',
          timestamp: '2026-04-25T12:00:20Z',
          serviceName: 'JournalDownloader',
          agentName: 'agent-a',
          taskName: 'probeACSwebsite',
          browserId: 'browser-1',
          profileId: 'profile-1',
          sessionId: 'session-1',
          applied: true,
          reason: 'validated_binding_applied',
          browserBuild: 'stealthcdp_chromium',
          bindingId: 'binding-1',
          hostId: 'local',
          executableId: 'stealth-current',
          capabilityId: 'stealth-cdp',
          executablePath: '/opt/chromium-stealthcdp/chrome',
        },
        {
          source: 'event',
          timestamp: '2026-04-25T12:00:30Z',
          serviceName: 'JournalDownloader',
          agentName: 'agent-a',
          taskName: 'probeACSwebsite',
          browserId: 'browser-1',
          profileId: 'profile-1',
          sessionId: 'session-2',
          applied: false,
          reason: 'profile_compatibility_missing_or_blocked',
          browserBuild: 'stealthcdp_chromium',
          bindingId: null,
          hostId: null,
          executableId: null,
          capabilityId: null,
          executablePath: null,
        },
      ],
    },
    profileLeaseWaits: {
      count: 2,
      activeCount: 1,
      completedCount: 1,
      waits: [
        {
          jobId: 'job-wait-complete',
          profileId: 'profile-1',
          outcome: 'ready',
          startedAt: '2026-04-25T12:00:10Z',
          endedAt: '2026-04-25T12:00:15Z',
          waitedMs: 5000,
          retryAfterMs: 250,
          conflictSessionIds: ['session-conflict'],
          serviceName: 'JournalDownloader',
          agentName: 'agent-a',
          taskName: 'probeACSwebsite',
        },
        {
          jobId: 'job-wait-active',
          profileId: 'profile-1',
          outcome: 'started',
          startedAt: '2026-04-25T12:05:00Z',
          endedAt: null,
          waitedMs: null,
          retryAfterMs: 500,
          conflictSessionIds: ['session-1', 'session-2'],
          serviceName: 'JournalDownloader',
          agentName: 'agent-b',
          taskName: 'downloadIssue',
        },
      ],
    },
    contexts: [
      {
        serviceName: 'SmallService',
        taskName: 'quickProbe',
        targetIdentityCount: 0,
        targetServiceIds: [],
        hasNamingWarning: true,
        namingWarnings: ['missing_agent_name'],
        eventCount: 1,
        jobCount: 0,
        incidentCount: 0,
        activityCount: 0,
        latestTimestamp: '2026-04-25T12:01:00Z',
      },
      {
        serviceName: 'JournalDownloader',
        agentName: 'agent-a',
        taskName: 'probeACSwebsite',
        browserId: 'browser-1',
        profileId: 'profile-1',
        sessionId: 'session-1',
        targetIdentityCount: 3,
        targetServiceIds: ['acs', 'google', 'acs'],
        hasNamingWarning: false,
        namingWarnings: [],
        eventCount: 2,
        jobCount: 2,
        incidentCount: 0,
        activityCount: 2,
        latestTimestamp: '2026-04-25T12:04:00Z',
      },
    ],
  },
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

const summaryContexts = traceSummaryContexts(traceData);
assert.equal(summaryContexts[0].serviceName, 'JournalDownloader');
assert.equal(summaryContexts[0].browserId, 'browser-1');
assert.equal(summaryContexts[1].serviceName, 'SmallService');

const summaryCards = traceSummaryCards(traceData);
assert.equal(summaryCards[0].title, 'JournalDownloader');
assert.equal(summaryCards[0].subtitle, 'probeACSwebsite');
assert.equal(summaryCards[0].total, 6);
assert.equal(summaryCards[0].warning, null);
assert.deepEqual(summaryCards[0].meta, [
  'agent agent-a',
  'browser browser-1',
  'profile profile-1',
  'session session-1',
]);
assert.deepEqual(summaryCards[0].targetServiceIds, ['acs', 'google']);
assert.deepEqual(summaryCards[0].counts, ['2 ev', '2 jobs', '0 inc', '2 act']);
assert.equal(summaryCards[1].warning, 'Missing agent name');
assert.deepEqual(summaryCards[1].targetServiceIds, []);
assert.equal(traceSummaryCards(null).length, 0);

const browserCapabilityLaunches = traceBrowserCapabilityLaunches(traceData);
assert.deepEqual(
  browserCapabilityLaunches.map((launch) => launch.sessionId),
  ['session-1', 'session-2'],
  'Applied browser capability launches should sort before skipped decisions',
);
assert.equal(browserCapabilityLaunches[0].applied, true);
assert.equal(browserCapabilityLaunches[0].bindingId, 'binding-1');
assert.equal(browserCapabilityLaunches[1].reason, 'profile_compatibility_missing_or_blocked');
assert.equal(traceBrowserCapabilityLaunches(null).length, 0);

const profileLeaseWaits = traceProfileLeaseWaits(traceData);
assert.deepEqual(
  profileLeaseWaits.map((wait) => wait.jobId),
  ['job-wait-active', 'job-wait-complete'],
  'Active profile lease waits should sort before completed waits',
);
assert.equal(profileLeaseWaits[0].profileId, 'profile-1');
assert.equal(profileLeaseWaits[0].conflictSessionIds.length, 2);
assert.equal(profileLeaseWaits[1].waitedMs, 5000);
assert.equal(traceProfileLeaseWaits(null).length, 0);

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
