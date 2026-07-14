#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { assertServiceStatusResponseSchemaRecord, loadServiceRecordSchema } from './smoke-schema-utils.js';
import { buildSelectedWorkspaceContext } from '../packages/dashboard/src/lib/selected-workspace-context.ts';
import {
  deriveLiveWorkspaceNodes,
  deriveWorkspaceNodes,
  workspaceNodeLiveControlEligibility,
} from '../packages/dashboard/src/lib/service-workspaces.ts';
import {
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamReadinessLabel,
} from '../packages/dashboard/src/lib/service-view-streams.ts';

const serviceStatusSchema = loadServiceRecordSchema('../docs/dev/contracts/service-status-response.v1.schema.json');
const observabilityTypes = readFileSync('packages/client/src/service-observability.generated.d.ts', 'utf8');

assert.match(
  observabilityTypes,
  /browserSessionAuthority\??: ServiceBrowserSessionAuthoritySnapshot/,
  'Generated service observability types must expose browserSessionAuthority from service status.',
);

assert.ok(
  serviceStatusSchema.properties.browserSessionAuthority?.properties?.browserVerdicts,
  'Service status schema must describe browserSessionAuthority browser verdicts.',
);

const readyStream = {
  id: 'ready-stream',
  provider: 'cdp_screencast',
  url: 'http://127.0.0.1:44841/',
  frameUrl: 'http://127.0.0.1:44841/',
  controlInput: 'cdp_input',
  readiness: { state: 'ready', reason: 'stream_server_ready' },
  readOnly: false,
};

const knownBadStream = {
  id: 'known-bad-stream',
  provider: 'cdp_screencast',
  url: 'http://127.0.0.1:44842/',
  frameUrl: 'http://127.0.0.1:44842/',
  controlInput: 'cdp_input',
  readiness: { state: 'unreachable', reason: 'stream proxy timed out' },
  readOnly: false,
};

const fixture = {
  control_plane: {
    waiting_profile_lease_job_count: 0,
    service_monitor_interval_ms: 60000,
  },
  profileAllocations: [
    {
      profileId: 'profile-owned-ready',
      profileName: 'Owned Ready',
      browserIds: ['owned-ready'],
      holderSessionIds: ['owned-ready-session'],
    },
    {
      profileId: 'profile-owned-non-viable',
      profileName: 'Owned Non Viable',
      browserIds: ['owned-non-viable'],
      holderSessionIds: ['owned-non-viable-session'],
    },
    {
      profileId: 'profile-owned-attention',
      profileName: 'Owned Attention',
      browserIds: ['owned-attention'],
      holderSessionIds: ['owned-attention-session'],
    },
  ],
  browserSessionAuthority: {
    schemaVersion: 1,
    summary: {
      modeledBrowserCount: 3,
      viableBrowserCount: 1,
      attentionBrowserCount: 1,
      nonViableBrowserCount: 1,
    },
    resourcePressure: {
      state: 'pressure',
      totalProcessCount: 4,
      correlatedProcessCount: 3,
      candidateCount: 1,
      protectedCount: 2,
      observedCount: 1,
      observedUnownedAgentBrowserProcessCount: 1,
      candidateRssBytes: 1048576,
      totalRssBytes: 4194304,
      reasons: ['cleanup_candidates_present'],
    },
    browserVerdicts: [
      {
        key: 'owned-ready',
        browserId: 'owned-ready',
        state: 'viable',
        viable: true,
        needsAttention: false,
        reasons: [],
      },
      {
        key: 'owned-non-viable',
        browserId: 'owned-non-viable',
        state: 'non_viable',
        viable: false,
        needsAttention: true,
        reasons: ['cleanup_candidate_process_correlates_to_browser'],
      },
      {
        key: 'owned-attention',
        browserId: 'owned-attention',
        state: 'attention',
        viable: false,
        needsAttention: true,
        reasons: ['live_browser_missing_pid'],
      },
    ],
  },
  service_state: {
    browsers: {
      'owned-ready': {
        id: 'owned-ready',
        profileId: 'profile-owned-ready',
        host: 'local_headed',
        health: 'ready',
        browserBuild: 'stealthcdp_chromium',
        pid: 7201,
        cdpEndpoint: 'ws://127.0.0.1:9721/devtools/browser/ready',
        viewStreams: [readyStream],
        activeSessionIds: ['owned-ready-session'],
      },
      'owned-non-viable': {
        id: 'owned-non-viable',
        profileId: 'profile-owned-non-viable',
        host: 'local_headed',
        health: 'ready',
        browserBuild: 'stealthcdp_chromium',
        pid: 7202,
        cdpEndpoint: 'ws://127.0.0.1:9722/devtools/browser/non-viable',
        viewStreams: [readyStream],
        activeSessionIds: ['owned-non-viable-session'],
      },
      'owned-attention': {
        id: 'owned-attention',
        profileId: 'profile-owned-attention',
        host: 'local_headed',
        health: 'ready',
        browserBuild: 'stealthcdp_chromium',
        pid: 7203,
        cdpEndpoint: 'ws://127.0.0.1:9723/devtools/browser/attention',
        viewStreams: [readyStream],
        activeSessionIds: ['owned-attention-session'],
      },
    },
    sessions: {
      'owned-ready-session': {
        id: 'owned-ready-session',
        browserIds: ['owned-ready'],
        tabIds: ['owned-ready-tab'],
        profileId: 'profile-owned-ready',
      },
      'owned-non-viable-session': {
        id: 'owned-non-viable-session',
        browserIds: ['owned-non-viable'],
        tabIds: ['owned-non-viable-tab'],
        profileId: 'profile-owned-non-viable',
      },
      'owned-attention-session': {
        id: 'owned-attention-session',
        browserIds: ['owned-attention'],
        tabIds: ['owned-attention-tab'],
        profileId: 'profile-owned-attention',
      },
    },
    tabs: {
      'owned-ready-tab': {
        id: 'owned-ready-tab',
        browserId: 'owned-ready',
        sessionId: 'owned-ready-session',
        targetId: 'target-owned-ready',
        lifecycle: 'active',
        title: 'Owned Ready',
        url: 'https://ready.example.test/',
      },
      'owned-non-viable-tab': {
        id: 'owned-non-viable-tab',
        browserId: 'owned-non-viable',
        sessionId: 'owned-non-viable-session',
        targetId: 'target-owned-non-viable',
        lifecycle: 'active',
        title: 'Owned Non Viable',
        url: 'https://non-viable.example.test/',
      },
      'owned-attention-tab': {
        id: 'owned-attention-tab',
        browserId: 'owned-attention',
        sessionId: 'owned-attention-session',
        targetId: 'target-owned-attention',
        lifecycle: 'active',
        title: 'Owned Attention',
        url: 'https://attention.example.test/',
      },
    },
  },
};

assertServiceStatusResponseSchemaRecord(fixture, serviceStatusSchema, 'cross-seam service status fixture');

const workspaceInput = {
  serviceBrowsers: Object.values(fixture.service_state.browsers),
  serviceSessions: Object.values(fixture.service_state.sessions),
  serviceTabs: Object.values(fixture.service_state.tabs),
  profileAllocations: fixture.profileAllocations,
  browserSessionAuthority: fixture.browserSessionAuthority,
  daemonSessions: [
    {
      session: 'detected-non-owned-cdp',
      port: 45011,
      engine: 'chrome',
      provider: 'detected-cdp',
      detected: true,
      ownership: 'foreign_cdp',
      addressability: 'cdp_reachable',
      cdpPort: 45011,
      profilePath: '/home/example/.agent-browser/foreign/profile',
      pid: 45011,
    },
  ],
  daemonTabsByPort: {
    45011: [
      {
        index: 0,
        title: 'Detected Non Owned CDP',
        url: 'https://detected.example.test/',
        type: 'page',
        active: true,
      },
    ],
  },
};

function byId(nodes, id) {
  const node = nodes.find((candidate) => candidate.id === id);
  assert.ok(node, `Missing workspace node ${id}`);
  return node;
}

function missingId(nodes, id) {
  assert.equal(nodes.some((node) => node.id === id), false, `Unexpected workspace node ${id}`);
}

const nodes = deriveWorkspaceNodes(workspaceInput);
const liveNodes = deriveLiveWorkspaceNodes(workspaceInput);

const readyNode = byId(nodes, 'browser:owned-ready');
assert.equal(readyNode.live, true);
assert.equal(readyNode.group, 'active');
assert.equal(readyNode.inventoryPlacement.lane, 'primary');
assert.equal(readyNode.viewStream?.embeddable, true);
assert.equal(readyNode.viewStream?.controllable, true);

const nonViableNode = byId(nodes, 'browser:owned-non-viable');
assert.equal(nonViableNode.live, false);
assert.equal(nonViableNode.retained, true);
assert.equal(nonViableNode.group, 'needs-attention');
assert.equal(nonViableNode.viewStream?.embeddable, false);
assert.equal(nonViableNode.viewStream?.controllable, false);
assert.match(nonViableNode.attentionReason ?? '', /cleanup_candidate_process_correlates_to_browser/);
missingId(liveNodes, 'browser:owned-non-viable');

const attentionNode = byId(nodes, 'browser:owned-attention');
assert.equal(attentionNode.live, true);
assert.equal(attentionNode.group, 'needs-attention');
assert.equal(attentionNode.inventoryPlacement.lane, 'attention');
assert.match(attentionNode.attentionReason ?? '', /live_browser_missing_pid/);
assert.ok(
  liveNodes.some((node) => node.id === 'browser:owned-attention'),
  'Authority attention browser with live evidence must remain in the live rail.',
);

const detectedNode = byId(nodes, 'daemon-session:detected-non-owned-cdp');
assert.equal(detectedNode.inventoryClass, 'detected-non-owned-browser');
assert.equal(detectedNode.group, 'detected');
assert.equal(detectedNode.live, true);
assert.ok(
  liveNodes.some((node) => node.id === 'daemon-session:detected-non-owned-cdp'),
  'Viable detected non-owned CDP browser must remain in the live rail.',
);

const readyIndex = liveNodes.findIndex((node) => node.id === 'browser:owned-ready');
const detectedIndex = liveNodes.findIndex((node) => node.id === 'daemon-session:detected-non-owned-cdp');
const attentionIndex = liveNodes.findIndex((node) => node.id === 'browser:owned-attention');
assert.ok(readyIndex > -1 && detectedIndex > -1 && attentionIndex > -1, 'Expected all visible live rail rows.');
assert.ok(
  readyIndex < attentionIndex && detectedIndex < attentionIndex,
  'Attention-worthy authority rows must sort after viable and detected rows.',
);

const emptySelection = {
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
};
const nonViableContext = buildSelectedWorkspaceContext({
  ...workspaceInput,
  selection: { ...emptySelection, browserId: 'owned-non-viable' },
  nodes,
  refreshedAt: 1710000000000,
});
assert.equal(nonViableContext.node?.id, 'browser:owned-non-viable');
assert.equal(nonViableContext.live, false);
assert.equal(nonViableContext.viewable, false);
assert.equal(nonViableContext.controllable, false);
assert.equal(workspaceNodeLiveControlEligibility(nonViableNode).canView, false);
assert.equal(workspaceNodeLiveControlEligibility(nonViableNode).canControl, false);

const readyContext = buildSelectedWorkspaceContext({
  ...workspaceInput,
  selection: { ...emptySelection, browserId: 'owned-ready' },
  nodes,
  refreshedAt: 1710000000000,
});
assert.equal(readyContext.viewable, true);
assert.equal(readyContext.controllable, true);

assert.equal(canOpenViewStream(knownBadStream), false);
assert.equal(canOpenControlViewStream(knownBadStream), false);
assert.equal(viewStreamReadinessLabel(knownBadStream), 'unreachable');
assert.equal(canOpenViewStream(readyStream), true);
assert.equal(canOpenControlViewStream(readyStream), true);
assert.equal(viewStreamReadinessLabel(readyStream), 'ready');

console.log('Cross-seam interlock tests passed');
