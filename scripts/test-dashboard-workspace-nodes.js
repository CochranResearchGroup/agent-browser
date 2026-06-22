#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { deriveWorkspaceNodes, deriveWorkspaceOwnershipDiagnostics } from '../packages/dashboard/src/lib/service-workspaces.ts';

const workspaceSource = readFileSync('packages/dashboard/src/lib/service-workspaces.ts', 'utf8');

assert.match(
  workspaceSource,
  /serviceTabsBySessionId = groupByLinkedId[\s\S]*serviceSessionById = new Map[\s\S]*serviceBrowserByActiveSessionId = new Map[\s\S]*profileAllocationById = new Map/,
  'Workspace derivation must pre-index session, browser, tab, and profile records before building rows',
);

assert.doesNotMatch(
  workspaceSource,
  /serviceTabs\.filter\(\(tab\) => tab\.sessionId === session\.id \|\| tab\.ownerSessionId === session\.id\)/,
  'Workspace derivation must not rescan all service tabs for each service session',
);

function byId(nodes, id) {
  const node = nodes.find((item) => item.id === id);
  assert.ok(node, `Missing workspace node ${id}`);
  return node;
}

function missingId(nodes, id) {
  assert.equal(nodes.some((item) => item.id === id), false, `Unexpected workspace node ${id}`);
}

function action(node, id) {
  const found = node.actions.find((item) => item.id === id);
  assert.ok(found, `Missing action ${id} on ${node.id}`);
  return found;
}

const diagnosticFixture = {
  serviceBrowsers: [
    {
      id: 'rdp-a',
      profileId: 'rdp-profile-a',
      host: 'remote_headed',
      health: 'ready',
      pid: 6101,
      cdpEndpoint: 'ws://127.0.0.1:9222/devtools/browser/shared',
      displayName: ':10',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/shared',
          routeId: 'shared-route',
          displayAllocationId: 'display-shared',
          providerMode: 'single_controller',
          viewerLeaseIds: ['viewer-a'],
          controllerLeaseId: 'viewer-a',
          remoteReadiness: { state: 'ready' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-a-session'],
    },
    {
      id: 'rdp-b',
      profileId: 'rdp-profile-b',
      host: 'remote_headed',
      health: 'ready',
      pid: 6102,
      cdpEndpoint: 'ws://127.0.0.1:9222/devtools/browser/shared',
      displayName: ':10',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/shared',
          routeId: 'shared-route',
          displayAllocationId: 'display-shared',
          providerMode: 'single_controller',
          viewerLeaseIds: ['viewer-b'],
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-b-session'],
    },
    {
      id: 'rdp-stale-target',
      profileId: 'rdp-profile-c',
      host: 'remote_headed',
      health: 'ready',
      pid: 6103,
      cdpEndpoint: 'ws://127.0.0.1:9223/devtools/browser/stale',
      displayName: ':11',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/stale',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-stale-session'],
    },
    {
      id: 'rdp-idle-display',
      profileId: 'rdp-profile-idle',
      host: 'remote_headed',
      health: 'ready',
      pid: 6104,
      displayName: ':12',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/idle',
          routeId: 'idle-route',
          displayAllocationId: 'display-idle',
          remoteReadiness: { state: 'ready' },
          displayContent: {
            state: 'terminal_only',
            windows: [
              { title: 'agent-browser-rdp-a@cooper: ~', className: 'XTerm' },
            ],
          },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-idle-session'],
    },
    {
      id: 'rdp-unbound-display',
      profileId: 'rdp-profile-unbound',
      host: 'remote_headed',
      health: 'ready',
      pid: 6105,
      displayName: ':109',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-unbound-session'],
    },
  ],
  serviceSessions: [
    { id: 'rdp-a-session', browserIds: ['rdp-a'], tabIds: ['rdp-a-tab'], profileId: 'rdp-profile-a' },
    { id: 'rdp-b-session', browserIds: ['rdp-b'], tabIds: ['rdp-b-tab'], profileId: 'rdp-profile-b' },
    { id: 'rdp-stale-session', browserIds: ['rdp-stale-target'], tabIds: ['rdp-stale-old', 'rdp-stale-live'], profileId: 'rdp-profile-c' },
    { id: 'rdp-idle-session', browserIds: ['rdp-idle-display'], tabIds: ['rdp-idle-tab'], profileId: 'rdp-profile-idle' },
    { id: 'rdp-unbound-session', browserIds: ['rdp-unbound-display'], tabIds: ['rdp-unbound-tab'], profileId: 'rdp-profile-unbound' },
  ],
  serviceTabs: [
    { id: 'rdp-a-tab', browserId: 'rdp-a', sessionId: 'rdp-a-session', targetId: 'target-shared', lifecycle: 'active', title: 'A', url: 'https://example.test/a' },
    { id: 'rdp-b-tab', browserId: 'rdp-b', sessionId: 'rdp-b-session', targetId: 'target-shared', lifecycle: 'active', title: 'B', url: 'https://example.test/b' },
    { id: 'rdp-stale-old', browserId: 'rdp-stale-target', sessionId: 'rdp-stale-session', targetId: 'target-stale-old', lifecycle: 'closed', title: 'about:blank', url: 'about:blank' },
    { id: 'rdp-stale-live', browserId: 'rdp-stale-target', sessionId: 'rdp-stale-session', targetId: 'target-stale-live', lifecycle: 'active', title: 'Live fallback', url: 'https://example.test/live' },
    { id: 'rdp-idle-tab', browserId: 'rdp-idle-display', sessionId: 'rdp-idle-session', targetId: 'target-idle', lifecycle: 'active', title: 'Expected target', url: 'https://example.test/target' },
    { id: 'rdp-unbound-tab', browserId: 'rdp-unbound-display', sessionId: 'rdp-unbound-session', targetId: 'target-unbound', lifecycle: 'active', title: 'Expected target on :109', url: 'https://example.test/unbound' },
  ],
};

const ownershipDiagnostics = deriveWorkspaceOwnershipDiagnostics(diagnosticFixture);
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-cdp-endpoint' && diagnostic.relatedIds.includes('browser:rdp-a') && diagnostic.relatedIds.includes('browser:rdp-b')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-display' && diagnostic.message.includes(':10')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-guacamole-route' && diagnostic.message.includes('shared-route')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-target' && diagnostic.relatedIds.includes('tab:rdp-a-tab') && diagnostic.relatedIds.includes('tab:rdp-b-tab')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'stale-retained-target' && diagnostic.relatedIds.includes('browser:rdp-stale-target') && diagnostic.message.includes('fall back to a current live tab')));

const diagnosticNodes = deriveWorkspaceNodes(diagnosticFixture);
const rdpA = byId(diagnosticNodes, 'browser:rdp-a');
assert.ok(rdpA.diagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-cdp-endpoint'));
assert.match(rdpA.secondaryLabel, /Duplicate CDP endpoint/);
const staleDiagnosticNode = byId(diagnosticNodes, 'browser:rdp-stale-target');
assert.ok(staleDiagnosticNode.diagnostics.some((diagnostic) => diagnostic.kind === 'stale-retained-target'));
assert.equal(staleDiagnosticNode.primaryTab?.id, 'rdp-stale-live');
const idleDisplayNode = byId(diagnosticNodes, 'browser:rdp-idle-display');
assert.ok(idleDisplayNode.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(idleDisplayNode.group, 'needs-attention');
assert.match(idleDisplayNode.attentionReason ?? '', /terminal/i);
assert.equal(idleDisplayNode.viewStream?.controllable, false);
assert.equal(action(idleDisplayNode, 'control').enabled, false);

const unboundDisplayNode = byId(diagnosticNodes, 'browser:rdp-unbound-display');
assert.ok(unboundDisplayNode.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(unboundDisplayNode.group, 'needs-attention');
assert.match(unboundDisplayNode.attentionReason ?? '', /no service-owned Guacamole route/);
assert.match(unboundDisplayNode.attentionReason ?? '', /:109/);
assert.equal(unboundDisplayNode.viewStream?.url, null);
assert.equal(unboundDisplayNode.viewStream?.controllable, false);
assert.equal(action(unboundDisplayNode, 'control').enabled, false);

const nodes = deriveWorkspaceNodes({
  daemonSessions: [
    {
      session: 'daemon-only',
      port: 4101,
      engine: 'chrome',
    },
    {
      session: 'dashboard-viewer-plan0025',
      port: 37273,
      engine: 'chrome',
    },
    {
      session: 'detected-chatgpt-45011',
      port: 45011,
      engine: 'chrome',
      provider: 'detected-cdp',
      detected: true,
      cdpPort: 45011,
      profilePath: '/home/example/.auracall/browser-profiles/default/chatgpt',
      pid: 45011,
    },
  ],
  daemonTabsByPort: {
    4101: [
      {
        index: 0,
        title: 'Standalone tab',
        url: 'https://example.test/standalone',
        type: 'page',
        active: true,
      },
    ],
    37273: [
      {
        index: 0,
        title: 'Agent Browser',
        url: 'https://agent-browser.example.test/?workspace=browser%3Asession%3Adefault&view=workspace%3Acontrol',
        type: 'page',
        active: true,
      },
    ],
    45011: [
      {
        index: 0,
        title: 'ChatGPT',
        url: 'https://chatgpt.com/',
        type: 'page',
        active: true,
      },
    ],
  },
  serviceBrowsers: [
    {
      id: 'browser-live',
      profileId: 'profile-ready',
      host: 'local',
      health: 'ready',
      browserBuild: 'stealthcdp_chromium',
      pid: 1234,
      cdpEndpoint: 'ws://127.0.0.1:4101/devtools/browser/live',
      processStats: {
        pid: 1234,
        running: true,
        rssBytes: 134217728,
        cpuSeconds: 42,
      },
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:44841/',
          frameUrl: 'http://127.0.0.1:44841/',
          controlInput: 'cdp_input',
          readOnly: false,
          readiness: { state: 'ready' },
        },
      ],
      activeSessionIds: ['session-live'],
    },
    {
      id: 'browser-retained',
      profileId: 'profile-retained',
      host: 'local',
      health: 'process_exited',
      browserBuild: 'stock_chrome',
    },
    {
      id: 'browser-disconnected',
      profileId: 'profile-disconnected',
      host: 'local',
      health: 'disconnected',
      lastError: 'DevTools endpoint is unreachable.',
      activeSessionIds: ['session-disconnected'],
    },
    {
      id: 'browser-cdp-disconnected',
      profileId: 'profile-cdp-disconnected',
      host: 'local_headless',
      health: 'cdp_disconnected',
      pid: 30734,
      cdpEndpoint: 'ws://127.0.0.1:37883/devtools/browser/f1dec9ac',
      lastError: 'CDP endpoint is unreachable.',
      activeSessionIds: ['session-cdp-disconnected'],
      viewStreams: [],
    },
    {
      id: 'browser-control',
      profileId: 'profile-control',
      host: 'remote',
      health: 'ready',
      pid: 2234,
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'http://127.0.0.1:8080/rdp/browser-control',
          routeId: 'route-control',
          displayAllocationId: 'display-control',
          providerMode: 'simultaneous_view',
          viewerLeaseIds: ['viewer-control-observer'],
          controllerLeaseId: 'viewer-control-controller',
          remoteReadiness: { state: 'ready' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-control'],
    },
    {
      id: 'browser-private-preferred',
      profileId: 'profile-control',
      host: 'remote_headed',
      health: 'ready',
      pid: 2235,
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'http://127.0.0.1:8080/rdp/shared-fallback',
          routeId: 'route-shared',
          displayAllocationId: 'display-shared',
          routeSource: 'config',
          providerMode: 'single_controller',
          remoteReadiness: { state: 'ready' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
        {
          provider: 'rdp_gateway',
          url: 'http://127.0.0.1:8080/rdp/private-route',
          routeId: 'route-private',
          displayAllocationId: 'display-private-a',
          routeSource: 'pool',
          providerMode: 'simultaneous_view',
          viewerLeaseIds: ['viewer-private-a', 'viewer-private-b'],
          remoteReadiness: { state: 'ready' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-private-preferred'],
    },
    {
      id: 'browser-takeover',
      profileId: 'profile-takeover',
      host: 'remote',
      health: 'ready',
      browserBuild: 'stealthcdp_chromium',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'http://127.0.0.1:8080/rdp/browser-takeover',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-takeover'],
    },
    {
      id: 'session:odollo-carrier-ups',
      profileId: 'stealthcdp-default',
      host: 'remote_headed',
      health: 'ready',
      browserBuild: 'stealthcdp_chromium',
      displayName: ':10',
      pid: 3234,
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/MQBjAHBvc3RncmVzcWw=',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['odollo-carrier-ups'],
    },
    {
      id: 'session:session-2',
      profileId: 'stealthcdp-default',
      host: 'remote_headed',
      health: 'process_exited',
      browserBuild: 'stealthcdp_chromium',
      displayName: ':10',
      lastError: 'Recorded browser PID 3941149 is no longer running',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/MQBjAHBvc3RncmVzcWw=',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-2'],
    },
    {
      id: 'session:dashboard-local-viewer-plan0016',
      profileId: 'profile-dashboard-viewer',
      host: 'remote_headed',
      health: 'ready',
      browserBuild: 'stealthcdp_chromium',
      displayName: ':109',
      pid: 97113,
      cdpEndpoint: 'ws://127.0.0.1:36877/devtools/browser/viewer',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['dashboard-local-viewer-plan0016'],
    },
  ],
  serviceSessions: [
    {
      id: 'session-live',
      serviceName: 'JournalDownloader',
      agentName: 'article-probe-agent',
      taskName: 'probeACSwebsite',
      profileId: 'profile-ready',
      browserIds: ['browser-live'],
      tabIds: ['tab-live'],
    },
    {
      id: 'session-disconnected',
      serviceName: 'MonitorService',
      profileId: 'profile-disconnected',
      browserIds: ['browser-disconnected'],
    },
    {
      id: 'session-cdp-disconnected',
      serviceName: 'TranscriptReview',
      taskName: 'contextualReadout',
      profileId: 'profile-cdp-disconnected',
      browserIds: ['browser-cdp-disconnected'],
    },
    {
      id: 'session-control',
      serviceName: 'DashboardSmoke',
      taskName: 'remoteViewControl',
      profileId: 'profile-control',
      browserIds: ['browser-control'],
      tabIds: ['tab-control'],
    },
    {
      id: 'session-private-preferred',
      serviceName: 'DashboardSmoke',
      taskName: 'privateRoutePreference',
      profileId: 'profile-control',
      browserIds: ['browser-private-preferred'],
      tabIds: ['tab-private-preferred'],
    },
    {
      id: 'session-takeover',
      owner: { operator: 'Alex' },
      lease: 'human_takeover',
      cleanup: 'release_only',
      profileLeaseDisposition: 'exclusive',
      profileLeaseConflictSessionIds: ['session-agent-waiting'],
      createdAt: '2026-05-23T13:00:00.000Z',
      lastLeaseObservedAt: '2026-05-23T13:05:00.000Z',
      serviceName: 'DashboardSmoke',
      taskName: 'manualControl',
      profileId: 'profile-takeover',
      browserIds: ['browser-takeover'],
      tabIds: ['tab-takeover'],
    },
    {
      id: 'session-standalone-takeover',
      owner: { operator: 'Morgan' },
      lease: 'human_takeover',
      cleanup: 'release_only',
      profileLeaseDisposition: 'exclusive',
      profileLeaseConflictSessionIds: ['session-background-worker'],
      serviceName: 'QueueRunner',
      taskName: 'manualResume',
      profileId: 'profile-standalone-takeover',
    },
    {
      id: 'odollo-carrier-ups',
      serviceName: 'AgentBrowserDashboard',
      taskName: 'restoreGuacRemoteViewportFinal',
      profileId: 'stealthcdp-default',
      browserIds: ['session:odollo-carrier-ups'],
      tabIds: ['tab-odollo-ups'],
    },
    {
      id: 'session-2',
      serviceName: 'AgentBrowserDashboard',
      taskName: 'restoreGuacRemoteViewportFinal',
      profileId: 'stealthcdp-default',
      browserIds: ['session:session-2'],
      tabIds: ['tab-stale-session-2'],
    },
    {
      id: 'dashboard-local-viewer-plan0016',
      profileId: 'profile-dashboard-viewer',
      browserIds: ['session:dashboard-local-viewer-plan0016'],
      tabIds: ['tab-dashboard-local-viewer-plan0016'],
    },
  ],
  serviceTabs: [
    {
      id: 'tab-live',
      browserId: 'browser-live',
      sessionId: 'session-live',
      lifecycle: 'active',
      title: 'ACS Publications',
      url: 'https://pubs.example.test/article',
    },
    {
      id: 'tab-control',
      browserId: 'browser-control',
      sessionId: 'session-control',
      lifecycle: 'active',
      title: 'Remote headed browser',
      url: 'https://dashboard.example.test/control',
    },
    {
      id: 'tab-private-preferred',
      browserId: 'browser-private-preferred',
      sessionId: 'session-private-preferred',
      lifecycle: 'active',
      title: 'Private route browser',
      url: 'https://dashboard.example.test/private-route',
    },
    {
      id: 'tab-takeover',
      browserId: 'browser-takeover',
      sessionId: 'session-takeover',
      lifecycle: 'active',
      title: 'Manual control target',
      url: 'https://dashboard.example.test/takeover',
    },
    {
      id: 'tab-odollo-ups',
      browserId: 'session:odollo-carrier-ups',
      sessionId: 'odollo-carrier-ups',
      lifecycle: 'active',
      targetId: 'target-odollo-ups',
      title: 'Tracking | UPS - United States',
      url: 'https://www.ups.com/track?tracknum=1Z2G26X60300020412',
    },
    {
      id: 'tab-stale-session-2',
      browserId: 'session:session-2',
      sessionId: 'session-2',
      lifecycle: 'closed',
      title: 'about:blank',
      url: 'about:blank',
    },
    {
      id: 'tab-dashboard-local-viewer-plan0016',
      browserId: 'session:dashboard-local-viewer-plan0016',
      ownerSessionId: 'dashboard-local-viewer-plan0016',
      lifecycle: 'ready',
      targetId: 'target-dashboard-local-viewer-plan0016',
      title: 'agent-browser',
      url: 'http://127.0.0.1:4848/?view=workspace%3Acontrol&workspace=daemon-session%3Adefault-posture-smoke&session=default-posture-smoke&tab=0',
    },
  ],
  profileAllocations: [
    {
      profileId: 'profile-ready',
      profileName: 'Research profile',
      browserBuild: 'stealthcdp_chromium',
      serviceNames: ['JournalDownloader'],
      targetReadiness: [
        {
          targetServiceId: 'acs',
          loginId: 'research-login',
          state: 'ready',
        },
      ],
    },
    {
      profileId: 'profile-retained',
      profileName: 'Retained profile',
      browserBuild: 'stock_chrome',
    },
    {
      profileId: 'profile-disconnected',
      profileName: 'Monitor profile',
      serviceNames: ['MonitorService'],
    },
    {
      profileId: 'profile-cdp-disconnected',
      profileName: 'Transcript review profile',
      serviceNames: ['TranscriptReview'],
    },
    {
      profileId: 'profile-control',
      profileName: 'Remote control profile',
      serviceNames: ['DashboardSmoke'],
    },
    {
      profileId: 'profile-dashboard-viewer',
      profileName: 'Dashboard viewer profile',
      browserBuild: 'stealthcdp_chromium',
    },
    {
      profileId: 'profile-takeover',
      profileName: 'Manual takeover profile',
      browserBuild: 'stealthcdp_chromium',
      serviceNames: ['DashboardSmoke'],
      waitingJobIds: ['job-takeover-wait'],
      conflictSessionIds: ['session-agent-waiting'],
      leaseState: 'exclusive',
    },
    {
      profileId: 'stealthcdp-default',
      profileName: 'Stealth CDP default',
      browserBuild: 'stealthcdp_chromium',
      serviceNames: ['AgentBrowserDashboard'],
      taskNames: ['restoreGuacRemoteViewportFinal'],
      browserIds: ['session:odollo-carrier-ups', 'session:session-2'],
      exclusiveHolderSessionIds: ['odollo-carrier-ups', 'session-2'],
      conflictSessionIds: ['session-2'],
      leaseState: 'exclusive',
      recommendedAction: 'reuse_holder_or_release_profile',
    },
    {
      profileId: 'profile-standalone-takeover',
      profileName: 'Standalone takeover profile',
      serviceNames: ['QueueRunner'],
      waitingJobIds: ['job-standalone-takeover-wait'],
      conflictSessionIds: ['session-background-worker'],
      leaseState: 'exclusive',
    },
    {
      profileId: 'profile-conflict',
      profileName: 'Conflicted profile',
      leaseState: 'exclusive_conflict',
      conflictSessionIds: ['session-a', 'session-b'],
      recommendedAction: 'Wait for the exclusive profile lease to clear.',
    },
    {
      profileId: 'profile-auth-ready',
      profileName: 'Auth ready profile',
      targetReadiness: [
        {
          targetServiceId: 'gmail',
          loginId: 'primary',
          state: 'ready',
          evidence: 'fresh-cookie-probe',
        },
      ],
    },
    {
      profileId: 'profile-manual-seeding',
      profileName: 'Needs manual login',
      targetReadiness: [
        {
          targetServiceId: 'canva',
          loginId: 'design',
          state: 'needs_manual_seeding',
          manualSeedingRequired: true,
          recommendedAction: 'Open a detached headed browser for manual login.',
        },
      ],
    },
  ],
  jobs: [
    {
      id: 'job-live',
      state: 'running',
      serviceName: 'JournalDownloader',
      target: { browserId: 'browser-live' },
    },
    {
      id: 'job-takeover-wait',
      state: 'waiting_profile_lease',
      serviceName: 'DashboardSmoke',
      target: { profileId: 'profile-takeover' },
    },
    {
      id: 'job-standalone-takeover-wait',
      state: 'waiting_profile_lease',
      serviceName: 'QueueRunner',
      target: { profileId: 'profile-standalone-takeover' },
    },
  ],
  incidents: [
    {
      id: 'incident-disconnected',
      browserId: 'browser-disconnected',
      label: 'Browser disconnected',
      latestMessage: 'DevTools endpoint is unreachable.',
      recommendedAction: 'Repair the retained browser record.',
    },
  ],
});

const live = byId(nodes, 'browser:browser-live');
assert.equal(live.source, 'service-browser');
assert.equal(live.group, 'active');
assert.equal(live.state, 'controllable');
assert.equal(live.label, 'JournalDownloader');
assert.equal(live.profileId, 'profile-ready');
assert.equal(live.primaryTab?.id, 'tab-live');
assert.deepEqual(live.relatedIds.serviceSessionIds, ['session-live']);
assert.deepEqual(live.relatedIds.jobIds, ['job-live']);
assert.equal(live.viewStream?.provider, 'cdp_screencast');
assert.equal(live.viewStream?.controllable, true);
assert.equal(live.process?.pid, 1234);
assert.equal(live.process?.rssBytes, 134217728);
assert.equal(live.process?.cpuSeconds, 42);
assert.equal(live.process?.cdpPort, 4101);
assert.equal(live.process?.streamPort, 44841);
assert.equal(action(live, 'focus').enabled, true);
assert.equal(action(live, 'control').enabled, true);
assert.equal(action(live, 'close').enabled, true);

missingId(nodes, 'browser:browser-retained');
missingId(nodes, 'profile:profile-retained');

const disconnected = byId(nodes, 'browser:browser-disconnected');
assert.equal(disconnected.group, 'needs-attention');
assert.equal(disconnected.state, 'needs-attention');
assert.equal(disconnected.attentionReason, 'Repair the retained browser record.');
assert.equal(action(disconnected, 'repair').enabled, true);

missingId(nodes, 'browser:browser-cdp-disconnected');

const control = byId(nodes, 'browser:browser-control');
assert.equal(control.group, 'active');
assert.equal(control.state, 'controllable');
assert.equal(control.viewStream?.provider, 'rdp_gateway');
assert.equal(control.viewStream?.controllable, true);
assert.equal(control.viewStream?.routeId, 'route-control');
assert.equal(control.viewStream?.displayAllocationId, 'display-control');
assert.equal(control.viewStream?.providerMode, 'simultaneous_view');
assert.deepEqual(control.viewStream?.viewerLeaseIds, ['viewer-control-observer']);
assert.equal(control.viewStream?.controllerLeaseId, 'viewer-control-controller');
assert.match(control.viewStream?.routeSummary ?? '', /route-control \/ display display-control \/ simultaneous view \/ 1 viewer, controller leased \/ ready/);
assert.match(control.secondaryLabel, /route-control \/ display display-control/);
assert.equal(action(control, 'control').enabled, true);
assert.equal(action(control, 'external-open').enabled, true);

const privatePreferred = byId(nodes, 'browser:browser-private-preferred');
assert.equal(privatePreferred.viewStream?.routeId, 'route-private');
assert.equal(privatePreferred.viewStream?.displayAllocationId, 'display-private-a');
assert.equal(privatePreferred.viewStream?.routeSource, 'pool');

const serviceDashboardViewer = byId(nodes, 'browser:session:dashboard-local-viewer-plan0016');
assert.equal(serviceDashboardViewer.role, 'viewer-client');
assert.equal(serviceDashboardViewer.group, 'needs-attention');
assert.equal(serviceDashboardViewer.viewStream?.url, null);
assert.equal(serviceDashboardViewer.viewStream?.controllable, false);
assert.match(serviceDashboardViewer.attentionReason ?? '', /Agent Browser/);
assert.ok(serviceDashboardViewer.diagnostics.some((diagnostic) => diagnostic.kind === 'viewer-client-target'));
assert.ok(serviceDashboardViewer.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(action(serviceDashboardViewer, 'control').enabled, false);
assert.equal(privatePreferred.viewStream?.providerMode, 'simultaneous_view');
assert.deepEqual(privatePreferred.viewStream?.viewerLeaseIds, ['viewer-private-a', 'viewer-private-b']);
assert.match(privatePreferred.viewStream?.routeSummary ?? '', /route-private \/ display display-private-a \/ simultaneous view \/ 2 viewers \/ ready/);

const odolloUps = byId(nodes, 'browser:session:odollo-carrier-ups');
assert.equal(odolloUps.group, 'active');
assert.equal(odolloUps.state, 'controllable');
assert.equal(odolloUps.label, 'Odollo UPS: 1Z2G26X60300020412');
assert.match(odolloUps.secondaryLabel, /Odollo UPS \/ remote_headed \/ stealthcdp_chromium/);
assert.equal(odolloUps.attentionReason, null);
assert.match(odolloUps.viewStream?.url ?? '', /\/guacamole\/#\/client\//);
assert.equal(odolloUps.viewStream?.controllable, true);
assert.equal(action(odolloUps, 'control').enabled, true);
assert.equal(action(odolloUps, 'external-open').enabled, true);

missingId(nodes, 'browser:session:session-2');
missingId(nodes, 'service-session:session-2');

const takeover = byId(nodes, 'browser:browser-takeover');
assert.equal(takeover.source, 'service-browser');
assert.equal(takeover.group, 'active');
assert.equal(takeover.state, 'controllable');
assert.equal(takeover.takeover?.active, true);
assert.equal(takeover.takeover?.sessionId, 'session-takeover');
assert.equal(takeover.takeover?.ownerLabel, 'operator: Alex');
assert.equal(takeover.takeover?.startedAt, '2026-05-23T13:00:00.000Z');
assert.deepEqual(takeover.takeover?.conflictSessionIds, ['session-agent-waiting']);
assert.deepEqual(takeover.takeover?.waitingJobIds, ['job-takeover-wait']);
assert.equal(takeover.primaryTab?.id, 'tab-takeover');
assert.match(takeover.attentionReason ?? '', /Human takeover holds the profile lease/);
assert.equal(takeover.viewStream?.controllable, true);
assert.equal(action(takeover, 'control').enabled, true);
assert.equal(action(takeover, 'view').enabled, true);
assert.equal(action(takeover, 'resume').enabled, false);
assert.match(action(takeover, 'resume').reason ?? '', /do not yet expose a service-owned resume action/);

const standaloneTakeover = byId(nodes, 'service-session:session-standalone-takeover');
assert.equal(standaloneTakeover.source, 'service-session');
assert.equal(standaloneTakeover.group, 'needs-attention');
assert.equal(standaloneTakeover.state, 'blocked');
assert.equal(standaloneTakeover.takeover?.ownerLabel, 'operator: Morgan');
assert.deepEqual(standaloneTakeover.takeover?.waitingJobIds, ['job-standalone-takeover-wait']);
assert.equal(standaloneTakeover.actions.filter((item) => item.id === 'resume').length, 1);
assert.equal(action(standaloneTakeover, 'resume').enabled, false);

const daemon = byId(nodes, 'daemon-session:daemon-only');
assert.equal(daemon.source, 'daemon-session');
assert.equal(daemon.role, 'target-browser');
assert.equal(daemon.group, 'detected');
assert.equal(daemon.label, 'Standalone tab');
assert.match(daemon.secondaryLabel, /not agent-browser service-owned/);
assert.equal(daemon.primaryTab?.url, 'https://example.test/standalone');
assert.equal(daemon.viewStream?.provider, 'cdp_screencast');
assert.equal(daemon.viewStream?.url, 'http://127.0.0.1:4101/');
assert.equal(daemon.process?.streamPort, 4101);
assert.equal(action(daemon, 'view').enabled, true);
assert.equal(action(daemon, 'control').enabled, true);
assert.equal(action(daemon, 'add-tab').enabled, true);

const detectedExternal = byId(nodes, 'daemon-session:detected-chatgpt-45011');
assert.equal(detectedExternal.source, 'daemon-session');
assert.equal(detectedExternal.group, 'detected');
assert.equal(detectedExternal.label, 'ChatGPT');
assert.equal(detectedExternal.viewStream, null);
assert.equal(detectedExternal.process?.cdpPort, 45011);
assert.equal(detectedExternal.process?.streamPort, null);
assert.match(detectedExternal.secondaryLabel, /detected external Chrome/);
assert.match(detectedExternal.secondaryLabel, /not agent-browser service-owned/);
assert.equal(action(detectedExternal, 'control').enabled, false);
assert.match(action(detectedExternal, 'control').reason ?? '', /no agent-browser-owned stream/);
assert.equal(action(detectedExternal, 'kill').enabled, false);

const dashboardViewer = byId(nodes, 'daemon-session:dashboard-viewer-plan0025');
assert.equal(dashboardViewer.source, 'daemon-session');
assert.equal(dashboardViewer.role, 'viewer-client');
assert.equal(dashboardViewer.group, 'needs-attention');
assert.equal(dashboardViewer.state, 'needs-attention');
assert.match(dashboardViewer.roleReason ?? '', /dashboard viewer client/);
assert.match(dashboardViewer.secondaryLabel, /viewer client/);
assert.ok(dashboardViewer.diagnostics.some((diagnostic) => diagnostic.kind === 'viewer-client-target'));
assert.equal(action(dashboardViewer, 'view').enabled, false);
assert.match(action(dashboardViewer, 'view').reason ?? '', /dashboard viewer client/);
assert.equal(action(dashboardViewer, 'control').enabled, false);
assert.equal(action(dashboardViewer, 'add-tab').enabled, false);
assert.equal(action(dashboardViewer, 'kill').enabled, true);

const conflict = byId(nodes, 'profile:profile-conflict');
assert.equal(conflict.source, 'profile');
assert.equal(conflict.group, 'needs-attention');
assert.equal(conflict.state, 'blocked');
assert.equal(conflict.attentionReason, 'Wait for the exclusive profile lease to clear.');
assert.equal(action(conflict, 'launch').enabled, false);

missingId(nodes, 'profile:profile-auth-ready');

const manualSeeding = byId(nodes, 'profile:profile-manual-seeding');
assert.equal(manualSeeding.group, 'needs-attention');
assert.equal(manualSeeding.state, 'needs-attention');
assert.equal(manualSeeding.attentionReason, 'Open a detached headed browser for manual login.');
assert.equal(action(manualSeeding, 'launch').enabled, false);
assert.equal(action(manualSeeding, 'seed').enabled, true);

assert.deepEqual(
  nodes.map((node) => node.group),
  [...nodes.map((node) => node.group)].sort((left, right) => {
    const order = { 'needs-attention': 0, active: 1, detected: 2, retained: 3 };
    return order[left] - order[right];
  }),
  'Workspace nodes should be grouped by attention, owned active, then detected order',
);
assert.ok(!nodes.some((node) => node.group === 'retained'), 'Retained rows belong outside the live workspace rail');

console.log('Dashboard workspace node contract smoke passed');
