#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  buildSelectedWorkspaceChatPacket,
  CONTEXTUAL_CHAT_PROVIDER_ID,
  SELECTED_WORKSPACE_CHAT_PACKET_VERSION,
  selectedWorkspaceChatPacketSummary,
  validateSelectedWorkspaceChatPacket,
} from '../packages/dashboard/src/lib/selected-workspace-chat-packet.ts';
import { buildSelectedWorkspaceContext } from '../packages/dashboard/src/lib/selected-workspace-context.ts';
import { buildSelectedWorkspaceConsoleEvidence } from '../packages/dashboard/src/lib/selected-workspace-console.ts';

const emptySelection = {
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
};

const fixture = {
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'default',
      health: 'ready',
      pid: 1234,
      processStats: {
        pid: 1234,
        running: true,
        rssBytes: 52428800,
        cpuSeconds: 9.25,
      },
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:38395/',
          routeId: 'route-live',
          providerMode: 'single_controller',
          controlInput: 'cdp_input',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-live'],
    },
    {
      id: 'browser-retained',
      profileId: 'retained',
      health: 'process_exited',
      activeSessionIds: ['session-retained'],
    },
  ],
  serviceSessions: [
    { id: 'session-live', browserIds: ['browser-live'], tabIds: ['tab-live'], profileId: 'default', serviceName: 'svc', agentName: 'agent', taskName: 'task' },
    { id: 'session-retained', browserIds: ['browser-retained'], tabIds: [], profileId: 'retained', serviceName: 'svc' },
  ],
  serviceTabs: [
    { id: 'tab-live', browserId: 'browser-live', sessionId: 'session-live', targetId: 'target-live', lifecycle: 'active', title: 'Live Page', url: 'https://example.test/app' },
  ],
  profileAllocations: [
    { profileId: 'default', profileName: 'Default', browserIds: ['browser-live'], holderSessionIds: ['session-live'] },
    { profileId: 'profile-only', profileName: 'Profile only' },
  ],
  jobs: [
    { id: 'job-live', state: 'running', target: { browserId: 'browser-live' } },
  ],
  incidents: [
    { id: 'incident-live', browserId: 'browser-live', severity: 'warning', latestMessage: 'inspect' },
  ],
};

function packet(selection, contextPatch = {}) {
  const context = buildSelectedWorkspaceContext({
    ...fixture,
    selection: { ...emptySelection, ...selection },
    refreshedAt: Date.now(),
  });
  return buildSelectedWorkspaceChatPacket(
    {
      ...context,
      ...contextPatch,
    },
    { createdAt: '2026-05-31T14:00:00.000Z' },
  );
}

const live = packet({ browserId: 'browser-live' });
assert.equal(live.version, SELECTED_WORKSPACE_CHAT_PACKET_VERSION);
assert.equal(live.provider, CONTEXTUAL_CHAT_PROVIDER_ID);
assert.equal(live.workspace.id, 'browser:browser-live');
assert.equal(live.workspace.state, 'controllable');
assert.equal(live.runtime.pid, 1234);
assert.equal(live.page.targetId, 'target-live');
assert.equal(live.stream.provider, 'cdp_screencast');
assert.equal(live.ownership.serviceName, 'svc');
const workspaceEvidence = live.evidence.find((item) => item.id === 'workspace.summary');
const activityEvidence = live.evidence.find((item) => item.id === 'activity.summary');
const streamEvidence = live.evidence.find((item) => item.id === 'stream.readiness');
const networkUnavailable = live.evidence.find((item) => item.source === 'network');
const consoleUnavailable = live.evidence.find((item) => item.source === 'console');
assert.ok(workspaceEvidence?.included);
assert.equal(workspaceEvidence.sourceLabel, 'Workspace');
assert.equal(workspaceEvidence.available, true);
assert.equal(workspaceEvidence.unavailableReason, null);
assert.ok(activityEvidence?.included);
assert.equal(activityEvidence.sourceLabel, 'Activity summary');
assert.equal(activityEvidence.available, true);
assert.equal(activityEvidence.facts.jobCount, 1);
assert.deepEqual(activityEvidence.facts.jobStates, { running: 1 });
assert.equal(activityEvidence.facts.incidentCount, 1);
assert.ok(streamEvidence?.included);
assert.equal(streamEvidence.sourceLabel, 'Stream readiness');
assert.equal(streamEvidence.available, true);
assert.equal(streamEvidence.facts.provider, 'cdp_screencast');
assert.equal(streamEvidence.facts.controlInput, 'cdp_input');
assert.equal(streamEvidence.facts.cdpPort, live.runtime.cdpPort);
assert.equal(streamEvidence.facts.streamPort, live.runtime.streamPort);
assert.equal(streamEvidence.facts.embeddable, true);
assert.equal(streamEvidence.facts.controllable, true);
assert.equal(networkUnavailable?.id, 'network.unavailable');
assert.equal(networkUnavailable?.available, false);
assert.equal(networkUnavailable?.included, false);
assert.match(networkUnavailable?.unavailableReason ?? '', /not implemented/);
assert.equal(consoleUnavailable?.id, 'console.unavailable');
assert.equal(consoleUnavailable?.available, false);
assert.ok(selectedWorkspaceChatPacketSummary(live).includes('browser:browser-live'));
assert.deepEqual(validateSelectedWorkspaceChatPacket(live), []);

const liveContext = buildSelectedWorkspaceContext({
  ...fixture,
  selection: { ...emptySelection, browserId: 'browser-live' },
  refreshedAt: Date.now(),
});
const liveConsoleEvidence = buildSelectedWorkspaceConsoleEvidence(liveContext, [
  { type: 'console', level: 'error', text: 'console failure token=secret', timestamp: Date.now(), streamPort: 38395 },
]);
const withConsole = buildSelectedWorkspaceChatPacket(
  liveContext,
  {
    createdAt: '2026-05-31T14:00:00.000Z',
    include: { console: true },
    consoleEvidence: liveConsoleEvidence,
  },
);
const consoleEvidence = withConsole.evidence.find((item) => item.source === 'console');
assert.equal(consoleEvidence?.id, 'console.summary');
assert.equal(consoleEvidence?.sourceLabel, 'Console');
assert.equal(consoleEvidence?.available, true);
assert.equal(consoleEvidence?.included, true);
assert.equal(consoleEvidence?.facts.counts.scoped, 1);
assert.doesNotMatch(JSON.stringify(withConsole), /token=secret/);

const streamExcluded = buildSelectedWorkspaceChatPacket(
  buildSelectedWorkspaceContext({
    ...fixture,
    selection: { ...emptySelection, browserId: 'browser-live' },
    refreshedAt: Date.now(),
  }),
  { createdAt: '2026-05-31T14:00:00.000Z', include: { stream: false } },
);
const excludedStreamEvidence = streamExcluded.evidence.find((item) => item.id === 'stream.readiness');
assert.equal(excludedStreamEvidence?.available, true);
assert.equal(excludedStreamEvidence?.included, false);

const retained = packet({ browserId: 'browser-retained' });
assert.equal(retained.workspace.id, 'browser:browser-retained');
assert.equal(retained.workspace.retained, true);
assert.equal(retained.workspace.live, false);

const profileOnly = packet({ workspaceId: 'profile:profile-only' });
assert.equal(profileOnly.workspace.id, 'profile:profile-only');
assert.equal(profileOnly.workspace.source, 'profile');

const missing = packet({ browserId: 'missing-browser' });
assert.equal(missing.workspace.id, null);
assert.equal(missing.workspace.state, 'missing');
assert.ok(missing.workspace.missingReason.includes('stale'));

const sensitive = packet(
  { browserId: 'browser-live' },
  {
    evidence: {
      summary: 'unsafe',
      rows: [
        { label: 'Cookie', value: 'session=secret' },
        { label: 'Authorization', value: 'Bearer token' },
        { label: 'localStorage', value: 'password=secret' },
      ],
    },
  },
);
const serializedSensitive = JSON.stringify(sensitive);
assert.doesNotMatch(serializedSensitive, /session=secret|Bearer token|password=secret/);
assert.match(serializedSensitive, /"label":"Cookie","value":"\[redacted\]"/);

const badProvider = { ...live, provider: 'openai' };
assert.ok(validateSelectedWorkspaceChatPacket(badProvider).some((error) => error.includes('codex-app-server')));

console.log('dashboard selected workspace chat packet tests passed');
