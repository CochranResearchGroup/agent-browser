#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  buildSelectedWorkspaceContext,
  selectedWorkspaceChatSummary,
  selectedWorkspaceDiagnosticBundle,
} from '../packages/dashboard/src/lib/selected-workspace-context.ts';

const emptySelection = {
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
};

const fixture = {
  daemonSessions: [
    { session: 'daemon-a', port: 38409, engine: 'chrome' },
    { session: 'daemon-stale', port: 38410, engine: 'chrome' },
  ],
  daemonTabsByPort: {
    38409: [
      { index: 0, title: 'Daemon Page', url: 'https://daemon.example.test/', type: 'page', active: true },
    ],
  },
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'default',
      health: 'ready',
      pid: 95444,
      processStats: { pid: 95444, running: true, rssBytes: 104857600, cpuSeconds: 12.25 },
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
  ],
  serviceSessions: [
    { id: 'session-live', browserIds: ['browser-live'], tabIds: ['tab-live'], profileId: 'default', serviceName: 'svc' },
  ],
  serviceTabs: [
    { id: 'tab-live', browserId: 'browser-live', sessionId: 'session-live', targetId: 'target-live', lifecycle: 'active', title: 'Live', url: 'https://live.example.test/' },
  ],
  profileAllocations: [
    { profileId: 'default', profileName: 'Default', browserIds: ['browser-live'], holderSessionIds: ['session-live'] },
    { profileId: 'profile-retained', profileName: 'Retained only', leaseState: 'available' },
  ],
  jobs: [
    { id: 'job-live', state: 'running', target: { browserId: 'browser-live' } },
  ],
  incidents: [
    { id: 'incident-live', browserId: 'browser-live', severity: 'warning', latestMessage: 'needs inspection' },
  ],
  resources: [
    {
      pid: 95444,
      kind: 'browser',
      disposition: 'candidate',
      reasons: ['old_temporary_profile_process'],
      correlation: {
        browserId: 'browser-live',
        profileId: 'default',
        sessionIds: ['session-live'],
      },
    },
    {
      pid: 95445,
      kind: 'browser',
      disposition: 'protected',
      reasons: ['service_state_browser'],
      correlation: {
        browserId: 'browser-live',
        profileId: 'default',
      },
    },
    {
      pid: 95555,
      kind: 'browser',
      disposition: 'candidate',
      reasons: ['unrelated'],
      correlation: {
        browserId: 'browser-other',
      },
    },
  ],
};

function context(selection) {
  return buildSelectedWorkspaceContext({
    ...fixture,
    selection: { ...emptySelection, ...selection },
    refreshedAt: 1710000000000,
  });
}

const byBrowser = context({ browserId: 'browser-live' });
assert.equal(byBrowser.node.id, 'browser:browser-live');
assert.equal(byBrowser.source, 'service-browser');
assert.equal(byBrowser.inventoryClass, 'service-owned-controllable-browser');
assert.equal(byBrowser.runtime.pid, 95444);
assert.equal(byBrowser.runtime.streamPort, 38395);
assert.equal(byBrowser.primaryTab.targetId, 'target-live');
assert.equal(byBrowser.controllable, true);
assert.equal(byBrowser.profileAllocation.profileId, 'default');
assert.equal(byBrowser.resources.length, 2);
assert.deepEqual(
  byBrowser.evidence.rows
    .filter((row) => ['Resource candidates', 'Protected resources', 'Resource reasons'].includes(row.label))
    .map((row) => [row.label, row.value]),
  [
    ['Resource candidates', '1'],
    ['Protected resources', '1'],
    ['Resource reasons', 'old_temporary_profile_process, service_state_browser'],
  ],
);
assert.equal(selectedWorkspaceDiagnosticBundle(byBrowser).resources.length, 2);
assert.equal(selectedWorkspaceDiagnosticBundle(byBrowser).workspace.inventoryClass, 'service-owned-controllable-browser');
assert.deepEqual(
  byBrowser.evidence.rows
    .filter((row) => row.label === 'Inventory class')
    .map((row) => row.value),
  ['service-owned-controllable-browser'],
);
assert.ok(selectedWorkspaceChatSummary(byBrowser).includes('browser browser-live'));
assert.equal(selectedWorkspaceDiagnosticBundle(byBrowser).ids.browserId, 'browser-live');

const terminalOnlySelection = buildSelectedWorkspaceContext({
  ...fixture,
  serviceBrowsers: [
    {
      id: 'browser-terminal-only',
      profileId: 'default',
      host: 'remote_headed',
      health: 'ready',
      pid: 95446,
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/terminal-only',
          routeId: 'terminal-only-route',
          displayAllocationId: 'display-terminal-only',
          remoteReadiness: { state: 'ready' },
          displayContent: {
            state: 'terminal_only',
            windows: [{ title: 'agent-browser-rdp@host: ~', className: 'XTerm' }],
          },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-terminal-only'],
    },
  ],
  serviceSessions: [
    { id: 'session-terminal-only', browserIds: ['browser-terminal-only'], tabIds: ['tab-terminal-only'], profileId: 'default', serviceName: 'svc' },
  ],
  serviceTabs: [
    { id: 'tab-terminal-only', browserId: 'browser-terminal-only', sessionId: 'session-terminal-only', targetId: 'target-terminal-only', lifecycle: 'active', title: 'Route-bound target tab', url: 'https://example.test/route-bound-target' },
  ],
  selection: { ...emptySelection, browserId: 'browser-terminal-only' },
  refreshedAt: 1710000000000,
});
assert.equal(terminalOnlySelection.node.id, 'browser:browser-terminal-only');
assert.equal(terminalOnlySelection.primaryTab.title, 'Route-bound target tab');
assert.equal(terminalOnlySelection.state, 'needs-attention');
assert.equal(terminalOnlySelection.viewable, false);
assert.equal(terminalOnlySelection.controllable, false);
assert.equal(terminalOnlySelection.stream.url, null);
assert.equal(terminalOnlySelection.stream.embeddable, false);
assert.match(terminalOnlySelection.stream.operatorVisibleReason, /terminal/i);

const byWorkspaceBrowser = context({ workspaceId: 'browser:browser-live' });
assert.equal(byWorkspaceBrowser.node.id, 'browser:browser-live');

const bySession = context({ sessionId: 'session-live' });
assert.equal(bySession.node.id, 'browser:browser-live');

const byServiceBrowserWithDaemonSession = buildSelectedWorkspaceContext({
  ...fixture,
  selection: { ...emptySelection, browserId: 'browser-live', sessionId: 'daemon-a' },
  refreshedAt: 1710000000000,
});
assert.equal(byServiceBrowserWithDaemonSession.node.id, 'browser:browser-live');
assert.equal(byServiceBrowserWithDaemonSession.daemonSession.session, 'daemon-a');
assert.equal(byServiceBrowserWithDaemonSession.runtime.streamPort, 38395);

const byServiceBrowserWithDaemonFallbackStream = buildSelectedWorkspaceContext({
  ...fixture,
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'default',
      health: 'ready',
      pid: 95444,
      processStats: { pid: 95444, running: true, rssBytes: 104857600, cpuSeconds: 12.25 },
      activeSessionIds: ['session-live'],
    },
  ],
  selection: { ...emptySelection, browserId: 'browser-live', sessionId: 'daemon-a' },
  refreshedAt: 1710000000000,
});
assert.equal(byServiceBrowserWithDaemonFallbackStream.node.id, 'browser:browser-live');
assert.equal(byServiceBrowserWithDaemonFallbackStream.daemonSession.session, 'daemon-a');
assert.equal(byServiceBrowserWithDaemonFallbackStream.runtime.streamPort, 38409);
assert.equal(byServiceBrowserWithDaemonFallbackStream.runtime.running, true);

const byTab = context({ tabId: 'target-live' });
assert.equal(byTab.node.id, 'browser:browser-live');

const byJob = context({ jobId: 'job-live' });
assert.equal(byJob.node.id, 'browser:browser-live');

const byDaemon = context({ workspaceId: 'daemon-session:daemon-a' });
assert.equal(byDaemon.node.id, 'daemon-session:daemon-a');
assert.equal(byDaemon.source, 'daemon-session');
assert.equal(byDaemon.runtime.streamPort, 38409);
assert.equal(byDaemon.runtime.running, true);
assert.equal(byDaemon.stream.provider, 'cdp_screencast');

const byStaleDaemon = context({ workspaceId: 'daemon-session:daemon-stale' });
assert.equal(byStaleDaemon.node.id, 'daemon-session:daemon-stale');
assert.equal(byStaleDaemon.source, 'daemon-session');
assert.equal(byStaleDaemon.state, 'needs-attention');
assert.equal(byStaleDaemon.node.health, 'stale-stream');
assert.equal(byStaleDaemon.runtime.streamPort, 38410);
assert.equal(byStaleDaemon.runtime.running, false);
assert.equal(byStaleDaemon.stream, null);
assert.ok(byStaleDaemon.node.attentionReason.includes('no CDP tab evidence'));

const byExplicitDaemonWithConflictingSessionParam = context({ workspaceId: 'daemon-session:daemon-a', sessionId: 'session-live' });
assert.equal(byExplicitDaemonWithConflictingSessionParam.node.id, 'daemon-session:daemon-a');
assert.equal(byExplicitDaemonWithConflictingSessionParam.source, 'daemon-session');

const byProfileOnly = context({ workspaceId: 'profile:profile-retained' });
assert.equal(byProfileOnly.node.id, 'profile:profile-retained');
assert.equal(byProfileOnly.source, 'profile');
assert.equal(byProfileOnly.retained, true);

const stale = context({ browserId: 'missing-browser' });
assert.equal(stale.node, null);
assert.equal(stale.state, 'missing');
assert.ok(stale.missingReason.includes('stale'));

const none = context({});
assert.equal(none.node, null);
assert.equal(none.state, 'none');

console.log('dashboard selected workspace context tests passed');
