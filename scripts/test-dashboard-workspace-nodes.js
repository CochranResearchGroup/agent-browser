#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  deriveLiveWorkspaceNodes,
  deriveWorkspaceNodes,
  deriveWorkspaceOwnershipDiagnostics,
  workspaceInventoryPlacementForNode,
  workspaceNodeLiveControlEligibility,
} from '../packages/dashboard/src/lib/service-workspaces.ts';

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
          displayContent: { state: 'browser_window_visible' },
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
          displayContent: { state: 'browser_window_visible' },
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
    {
      id: 'rdp-proof-missing',
      profileId: 'rdp-profile-proof-missing',
      host: 'remote_headed',
      health: 'ready',
      pid: 6106,
      displayName: ':13',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/proof-missing',
          routeId: 'proof-missing-route',
          displayAllocationId: 'display-proof-missing',
          remoteReadiness: { state: 'ready' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-proof-missing-session'],
    },
    {
      id: 'rdp-readiness-display-proof',
      profileId: 'rdp-profile-readiness-display-proof',
      host: 'remote_headed',
      health: 'ready',
      pid: 6107,
      displayName: ':14',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          frameUrl: 'http://127.0.0.1:8092/guacamole/#/client/readiness-proof',
          externalUrl: 'https://agent-browser.example.test/guacamole/#/client/readiness-proof',
          routeId: 'readiness-proof-route',
          displayAllocationId: 'display-readiness-proof',
          connectionId: '4',
          connectionName: 'Agent Browser RDP Route B',
          routeSource: 'pool',
          providerMode: 'simultaneous_view',
          readiness: {
            state: 'ready',
            displayContent: {
              state: 'browser_window_visible',
              windows: [
                { title: '(20+) OpenAI - Search Results | Facebook - Chromium', className: 'Chrome' },
              ],
            },
          },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-readiness-display-proof-session'],
    },
  ],
  serviceSessions: [
    { id: 'rdp-a-session', browserIds: ['rdp-a'], tabIds: ['rdp-a-tab'], profileId: 'rdp-profile-a' },
    { id: 'rdp-b-session', browserIds: ['rdp-b'], tabIds: ['rdp-b-tab'], profileId: 'rdp-profile-b' },
    { id: 'rdp-stale-session', browserIds: ['rdp-stale-target'], tabIds: ['rdp-stale-old', 'rdp-stale-live'], profileId: 'rdp-profile-c' },
    { id: 'rdp-idle-session', browserIds: ['rdp-idle-display'], tabIds: ['rdp-idle-tab'], profileId: 'rdp-profile-idle' },
    { id: 'rdp-unbound-session', browserIds: ['rdp-unbound-display'], tabIds: ['rdp-unbound-tab'], profileId: 'rdp-profile-unbound' },
    { id: 'rdp-proof-missing-session', browserIds: ['rdp-proof-missing'], tabIds: ['rdp-proof-missing-tab'], profileId: 'rdp-profile-proof-missing' },
    { id: 'rdp-readiness-display-proof-session', browserIds: ['rdp-readiness-display-proof'], tabIds: ['rdp-readiness-display-proof-tab'], profileId: 'rdp-profile-readiness-display-proof' },
  ],
  serviceTabs: [
    { id: 'rdp-a-tab', browserId: 'rdp-a', sessionId: 'rdp-a-session', targetId: 'target-shared', lifecycle: 'active', title: 'A', url: 'https://example.test/a' },
    { id: 'rdp-b-tab', browserId: 'rdp-b', sessionId: 'rdp-b-session', targetId: 'target-shared', lifecycle: 'active', title: 'B', url: 'https://example.test/b' },
    { id: 'rdp-stale-old', browserId: 'rdp-stale-target', sessionId: 'rdp-stale-session', targetId: 'target-stale-old', lifecycle: 'closed', title: 'about:blank', url: 'about:blank' },
    { id: 'rdp-stale-live', browserId: 'rdp-stale-target', sessionId: 'rdp-stale-session', targetId: 'target-stale-live', lifecycle: 'active', title: 'Live fallback', url: 'https://example.test/live' },
    { id: 'rdp-idle-tab', browserId: 'rdp-idle-display', sessionId: 'rdp-idle-session', targetId: 'target-idle', lifecycle: 'active', title: 'Expected target', url: 'https://example.test/target' },
    { id: 'rdp-unbound-tab', browserId: 'rdp-unbound-display', sessionId: 'rdp-unbound-session', targetId: 'target-unbound', lifecycle: 'active', title: 'Expected target on :109', url: 'https://example.test/unbound' },
    { id: 'rdp-proof-missing-tab', browserId: 'rdp-proof-missing', sessionId: 'rdp-proof-missing-session', targetId: 'target-proof-missing', lifecycle: 'active', title: 'Expected target proof missing', url: 'https://example.test/proof-missing' },
    { id: 'rdp-readiness-display-proof-tab', browserId: 'rdp-readiness-display-proof', sessionId: 'rdp-readiness-display-proof-session', targetId: 'target-readiness-display-proof', lifecycle: 'active', title: '(20+) OpenAI - Search Results | Facebook', url: 'https://www.facebook.com/search/posts?q=OpenAI' },
  ],
};

const ownershipDiagnostics = deriveWorkspaceOwnershipDiagnostics(diagnosticFixture);
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-cdp-endpoint' && diagnostic.relatedIds.includes('browser:rdp-a') && diagnostic.relatedIds.includes('browser:rdp-b')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-display' && diagnostic.message.includes(':10')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-guacamole-route' && diagnostic.message.includes('shared-route')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-target' && diagnostic.relatedIds.includes('tab:rdp-a-tab') && diagnostic.relatedIds.includes('tab:rdp-b-tab')));
assert.ok(ownershipDiagnostics.some((diagnostic) => diagnostic.kind === 'stale-retained-target' && diagnostic.relatedIds.includes('browser:rdp-stale-target') && diagnostic.message.includes('fall back to a current live tab')));

const diagnosticNodes = deriveWorkspaceNodes(diagnosticFixture);
const liveDiagnosticNodes = deriveLiveWorkspaceNodes(diagnosticFixture);
assert.ok(
  liveDiagnosticNodes.every((node) =>
    node.role !== 'viewer-client' &&
      node.inventoryClass !== 'viewer-client' &&
      (node.group === 'active' || node.group === 'detected' || (node.live && node.group === 'needs-attention'))
  ),
  'Live workspace projection must keep viable live targets and live needs-attention rows visible while excluding retained history and viewer clients',
);
const rdpA = byId(diagnosticNodes, 'browser:rdp-a');
assert.ok(rdpA.diagnostics.some((diagnostic) => diagnostic.kind === 'duplicate-cdp-endpoint'));
assert.match(rdpA.secondaryLabel, /Duplicate CDP endpoint/);
const staleDiagnosticNode = byId(diagnosticNodes, 'browser:rdp-stale-target');
assert.ok(staleDiagnosticNode.diagnostics.some((diagnostic) => diagnostic.kind === 'stale-retained-target'));
assert.equal(staleDiagnosticNode.primaryTab?.id, 'rdp-stale-live');
const idleDisplayNode = byId(diagnosticNodes, 'browser:rdp-idle-display');
assert.ok(idleDisplayNode.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(idleDisplayNode.group, 'needs-attention');
assert.ok(
  liveDiagnosticNodes.some((node) => node.id === 'browser:rdp-idle-display'),
  'Live route-proof diagnostic rows must stay in the rail instead of disappearing after status reconciliation',
);
assert.equal(idleDisplayNode.inventoryClass, 'service-owned-diagnostic-browser');
assert.equal(idleDisplayNode.state, 'needs-attention');
assert.match(idleDisplayNode.attentionReason ?? '', /terminal/i);

const authorityFixture = {
  serviceBrowsers: [
    {
      id: 'authority-ready',
      profileId: 'authority-profile-ready',
      host: 'local_headed',
      health: 'ready',
      pid: 7201,
      cdpEndpoint: 'ws://127.0.0.1:9721/devtools/browser/ready',
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:8721/',
          controlInput: 'cdp_input',
          readOnly: false,
        },
      ],
      activeSessionIds: ['authority-ready-session'],
    },
    {
      id: 'authority-non-viable',
      profileId: 'authority-profile-non-viable',
      host: 'local_headed',
      health: 'ready',
      pid: 7202,
      cdpEndpoint: 'ws://127.0.0.1:9722/devtools/browser/non-viable',
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:8722/',
          controlInput: 'cdp_input',
          readOnly: false,
        },
      ],
      activeSessionIds: ['authority-non-viable-session'],
    },
    {
      id: 'authority-attention',
      profileId: 'authority-profile-attention',
      host: 'local_headed',
      health: 'ready',
      pid: 7203,
      cdpEndpoint: 'ws://127.0.0.1:9723/devtools/browser/attention',
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:8723/',
          controlInput: 'cdp_input',
          readOnly: false,
        },
      ],
      activeSessionIds: ['authority-attention-session'],
    },
  ],
  serviceSessions: [
    { id: 'authority-ready-session', browserIds: ['authority-ready'], profileId: 'authority-profile-ready' },
    { id: 'authority-non-viable-session', browserIds: ['authority-non-viable'], profileId: 'authority-profile-non-viable' },
    { id: 'authority-attention-session', browserIds: ['authority-attention'], profileId: 'authority-profile-attention' },
  ],
  browserSessionAuthority: {
    schemaVersion: 1,
    browserVerdicts: [
      {
        key: 'authority-ready',
        browserId: 'authority-ready',
        state: 'viable',
        viable: true,
        needsAttention: false,
        reasons: [],
      },
      {
        key: 'authority-non-viable',
        browserId: 'authority-non-viable',
        state: 'non_viable',
        viable: false,
        needsAttention: true,
        reasons: ['cleanup_candidate_process_correlates_to_browser'],
      },
      {
        key: 'authority-attention',
        browserId: 'authority-attention',
        state: 'attention',
        viable: false,
        needsAttention: true,
        reasons: ['live_browser_missing_pid'],
      },
    ],
  },
};
const authorityNodes = deriveWorkspaceNodes(authorityFixture);
const authorityLiveNodes = deriveLiveWorkspaceNodes(authorityFixture);
assert.equal(byId(authorityNodes, 'browser:authority-ready').group, 'active');
const nonViableAuthorityNode = byId(authorityNodes, 'browser:authority-non-viable');
assert.equal(nonViableAuthorityNode.live, false);
assert.equal(nonViableAuthorityNode.group, 'needs-attention');
assert.match(nonViableAuthorityNode.attentionReason ?? '', /cleanup_candidate_process_correlates_to_browser/);
missingId(authorityLiveNodes, 'browser:authority-non-viable');
const attentionAuthorityNode = byId(authorityNodes, 'browser:authority-attention');
assert.equal(attentionAuthorityNode.live, true);
assert.equal(attentionAuthorityNode.group, 'needs-attention');
assert.ok(
  authorityLiveNodes.some((node) => node.id === 'browser:authority-attention'),
  'Authority attention rows with live evidence must remain visible at the bottom of the live rail',
);
assert.equal(idleDisplayNode.viewStream?.operatorVisibleState, 'route_bound_terminal_only');
assert.match(idleDisplayNode.viewStream?.operatorVisibleReason ?? '', /terminal/i);
assert.equal(idleDisplayNode.viewStream?.embeddable, false);
assert.equal(idleDisplayNode.viewStream?.controllable, false);
assert.equal(action(idleDisplayNode, 'view').enabled, false);
assert.match(action(idleDisplayNode, 'view').reason ?? '', /terminal/i);
assert.equal(action(idleDisplayNode, 'control').enabled, false);
assert.match(action(idleDisplayNode, 'control').reason ?? '', /terminal/i);
assert.equal(action(idleDisplayNode, 'repair').enabled, true);
assert.equal(workspaceNodeLiveControlEligibility(idleDisplayNode).state, 'not-controllable');
assert.equal(workspaceNodeLiveControlEligibility(idleDisplayNode).canControl, false);

const readinessDisplayProofNode = byId(diagnosticNodes, 'browser:rdp-readiness-display-proof');
assert.equal(readinessDisplayProofNode.group, 'active');
assert.equal(readinessDisplayProofNode.inventoryClass, 'service-owned-controllable-browser');
assert.equal(readinessDisplayProofNode.viewStream?.operatorVisibleState, 'ready');
assert.equal(readinessDisplayProofNode.viewStream?.embeddable, true);
assert.equal(action(readinessDisplayProofNode, 'control').enabled, true);
assert.ok(
  liveDiagnosticNodes.some((node) => node.id === 'browser:rdp-readiness-display-proof'),
  'Browser streams with visible-window proof nested under readiness.displayContent must stay in the owned rail',
);

const unboundDisplayNode = byId(diagnosticNodes, 'browser:rdp-unbound-display');
assert.ok(unboundDisplayNode.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(unboundDisplayNode.group, 'needs-attention');
assert.ok(
  liveDiagnosticNodes.some((node) => node.id === 'browser:rdp-unbound-display'),
  'Live unbound route rows must stay visible in the rail as needs-attention rows',
);
assert.equal(unboundDisplayNode.inventoryClass, 'service-owned-diagnostic-browser');
assert.equal(unboundDisplayNode.state, 'needs-attention');
assert.match(unboundDisplayNode.attentionReason ?? '', /no service-owned Guacamole route/);
assert.equal(unboundDisplayNode.viewStream?.operatorVisibleState, 'route_bound_proof_missing');
assert.match(unboundDisplayNode.viewStream?.operatorVisibleReason ?? '', /no service-owned Guacamole route/);
assert.match(unboundDisplayNode.viewStream?.operatorVisibleReason ?? '', /:109/);
assert.equal(unboundDisplayNode.viewStream?.url, null);
assert.equal(unboundDisplayNode.viewStream?.embeddable, false);
assert.equal(unboundDisplayNode.viewStream?.controllable, false);
assert.equal(action(unboundDisplayNode, 'view').enabled, false);
assert.match(action(unboundDisplayNode, 'view').reason ?? '', /no service-owned Guacamole route/);
assert.equal(action(unboundDisplayNode, 'control').enabled, false);
assert.match(action(unboundDisplayNode, 'control').reason ?? '', /no service-owned Guacamole route/);
assert.equal(action(unboundDisplayNode, 'repair').enabled, true);

const proofMissingNode = byId(diagnosticNodes, 'browser:rdp-proof-missing');
assert.ok(proofMissingNode.diagnostics.some((diagnostic) => diagnostic.kind === 'idle-route-display'));
assert.equal(proofMissingNode.group, 'needs-attention');
assert.ok(
  liveDiagnosticNodes.some((node) => node.id === 'browser:rdp-proof-missing'),
  'Live proof-missing rows must stay visible in the rail as needs-attention rows',
);
assert.equal(proofMissingNode.inventoryClass, 'service-owned-diagnostic-browser');
assert.equal(proofMissingNode.state, 'needs-attention');
assert.match(proofMissingNode.attentionReason ?? '', /operator-visible proof missing/);
assert.equal(proofMissingNode.viewStream?.operatorVisibleState, 'route_bound_proof_missing');
assert.match(proofMissingNode.viewStream?.routeSummary ?? '', /operator-visible proof missing/);
assert.equal(proofMissingNode.viewStream?.embeddable, false);
assert.equal(proofMissingNode.viewStream?.controllable, false);
assert.equal(action(proofMissingNode, 'view').enabled, false);
assert.match(action(proofMissingNode, 'view').reason ?? '', /operator-visible proof missing/);
assert.equal(action(proofMissingNode, 'control').enabled, false);
assert.match(action(proofMissingNode, 'control').reason ?? '', /operator-visible proof missing/);
assert.equal(action(proofMissingNode, 'repair').enabled, true);

const structuredProofNodes = deriveWorkspaceNodes({
  serviceBrowsers: [
    {
      id: 'rdp-wrong-tab',
      profileId: 'rdp-profile-proof',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/wrong-tab',
          routeId: 'route-wrong-tab',
          displayAllocationId: 'display-wrong-tab',
          remoteReadiness: {
            operatorVisible: {
              state: 'wrong_tab',
              components: {
                display: { state: 'ready' },
                tab: { state: 'wrong_tab' },
              },
            },
          },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-wrong-tab-session'],
    },
    {
      id: 'rdp-guacamole-unavailable',
      profileId: 'rdp-profile-proof',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/guac-unavailable',
          routeId: 'route-guac-unavailable',
          displayAllocationId: 'display-guac-unavailable',
          remoteReadiness: {
            operatorVisible: {
              state: 'guacamole_route_unavailable',
              components: {
                display: { state: 'ready' },
                tab: { state: 'ready' },
                guacamole: { state: 'guacamole_route_unavailable' },
              },
            },
          },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-guacamole-unavailable-session'],
    },
    {
      id: 'rdp-cdp-target-unavailable',
      profileId: 'rdp-profile-proof',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/cdp-target',
          routeId: 'route-cdp-target',
          displayAllocationId: 'display-cdp-target',
          remoteReadiness: {
            operatorVisible: {
              state: 'cdp_target_unavailable',
              components: {
                display: { state: 'ready' },
                tab: { state: 'cdp_target_unavailable' },
              },
            },
          },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-cdp-target-unavailable-session'],
    },
    {
      id: 'rdp-stale-route-record',
      profileId: 'rdp-profile-proof',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/stale-route',
          routeId: 'route-stale',
          displayAllocationId: 'display-stale',
          remoteReadiness: {
            operatorVisible: {
              state: 'stale_route_record',
              components: {
                route: {
                  state: 'stale_route_record',
                  routePoolEntryState: 'checked_out',
                  currentRouteAllocationId: 'route-missing',
                },
                display: { state: 'ready' },
                tab: { state: 'ready' },
              },
            },
          },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['rdp-stale-route-record-session'],
    },
  ],
});

for (const [id, expectedState, reasonPattern] of [
  ['browser:rdp-wrong-tab', 'wrong_tab', /selected tab URL/i],
  ['browser:rdp-guacamole-unavailable', 'guacamole_route_unavailable', /Guacamole operator route/i],
  ['browser:rdp-cdp-target-unavailable', 'cdp_target_unavailable', /CDP target id/i],
  ['browser:rdp-stale-route-record', 'stale_route_record', /stale route allocation/i],
]) {
  const node = byId(structuredProofNodes, id);
  assert.equal(node.group, 'needs-attention');
  assert.equal(node.inventoryClass, 'service-owned-diagnostic-browser');
  assert.equal(node.state, 'needs-attention');
  assert.match(node.attentionReason ?? '', reasonPattern);
  assert.equal(node.viewStream?.operatorVisibleState, expectedState);
  assert.match(node.viewStream?.operatorVisibleReason ?? '', reasonPattern);
  assert.equal(node.viewStream?.embeddable, false);
  assert.equal(node.viewStream?.controllable, false);
  assert.equal(action(node, 'view').enabled, false);
  assert.match(action(node, 'view').reason ?? '', reasonPattern);
  assert.equal(action(node, 'control').enabled, false);
  assert.match(action(node, 'control').reason ?? '', reasonPattern);
  assert.equal(action(node, 'repair').enabled, true);
  assert.equal(workspaceNodeLiveControlEligibility(node).state, 'not-controllable');
  assert.equal(workspaceNodeLiveControlEligibility(node).canControl, false);
}

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
    {
      session: 'detected-empty-but-reachable',
      port: 45013,
      engine: 'chrome',
      provider: 'detected-cdp',
      detected: true,
      ownership: 'foreign_cdp',
      addressability: 'cdp_reachable',
      cdpPort: 45013,
      profilePath: '/home/example/.auracall/browser-profiles/default/facebook',
      pid: 45013,
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
    45013: [],
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
      id: 'browser-cdp-missing-stream',
      profileId: 'profile-cdp-missing-stream',
      host: 'attached_existing',
      health: 'ready',
      browserBuild: 'stock_chrome',
      cdpEndpoint: 'ws://127.0.0.1:37365/devtools/browser/missing-stream',
      viewStreams: [
        {
          id: 'cdp-screencast',
          provider: 'cdp_screencast',
          url: null,
          controlInput: null,
          readOnly: true,
          readiness: {
            state: 'unavailable',
            reason: 'missing_stream_server',
          },
          routeSource: 'daemon_stream_server',
        },
      ],
      activeSessionIds: ['session-cdp-missing-stream'],
    },
    {
      id: 'browser-readiness-blocked',
      profileId: 'profile-readiness-blocked',
      host: 'local',
      health: 'ready',
      browserBuild: 'stock_chrome',
      pid: 1235,
      cdpEndpoint: 'ws://127.0.0.1:4102/devtools/browser/readiness-blocked',
      viewStreams: [
        {
          provider: 'cdp_screencast',
          url: 'http://127.0.0.1:44842/',
          frameUrl: 'http://127.0.0.1:44842/',
          controlInput: 'cdp_input',
          readOnly: false,
          readiness: { state: 'unreachable', reason: 'stream proxy timed out' },
        },
      ],
      activeSessionIds: ['session-readiness-blocked'],
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
          displayContent: { state: 'browser_window_visible' },
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
          displayContent: { state: 'browser_window_visible' },
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
          displayContent: { state: 'browser_window_visible' },
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
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
      activeSessionIds: ['session-takeover'],
    },
    {
      id: 'browser-route-switch',
      profileId: 'profile-route-switch',
      host: 'remote_headed',
      health: 'ready',
      browserBuild: 'stealthcdp_chromium',
      pid: 4234,
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/route-switch-current',
          routeId: 'route-switch-current',
          displayAllocationId: 'display-route-switch-current',
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
          attachability: {
            recommendedAction: 'service_remote_view_route_switch',
            reason: 'A fresher route-pool entry is available for this retained browser.',
          },
        },
      ],
      activeSessionIds: ['session-route-switch'],
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
          displayContent: { state: 'browser_window_visible' },
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
      id: 'session-cdp-missing-stream',
      serviceName: 'AttachedChrome',
      profileId: 'profile-cdp-missing-stream',
      browserIds: ['browser-cdp-missing-stream'],
      tabIds: ['tab-cdp-missing-stream'],
    },
    {
      id: 'session-readiness-blocked',
      serviceName: 'StreamReadiness',
      profileId: 'profile-readiness-blocked',
      browserIds: ['browser-readiness-blocked'],
      tabIds: ['tab-readiness-blocked'],
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
      id: 'session-route-switch',
      serviceName: 'DashboardSmoke',
      taskName: 'routeSwitch',
      profileId: 'profile-route-switch',
      browserIds: ['browser-route-switch'],
      tabIds: ['tab-route-switch'],
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
      id: 'tab-cdp-missing-stream',
      browserId: 'browser-cdp-missing-stream',
      sessionId: 'session-cdp-missing-stream',
      lifecycle: 'active',
      targetId: 'target-cdp-missing-stream',
      title: 'Attached CDP page',
      url: 'https://example.test/attached-cdp',
    },
    {
      id: 'tab-readiness-blocked',
      browserId: 'browser-readiness-blocked',
      sessionId: 'session-readiness-blocked',
      lifecycle: 'active',
      targetId: 'target-readiness-blocked',
      title: 'Readiness blocked page',
      url: 'https://example.test/readiness-blocked',
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
      id: 'tab-route-switch',
      browserId: 'browser-route-switch',
      sessionId: 'session-route-switch',
      lifecycle: 'active',
      title: 'Route switch target',
      url: 'https://dashboard.example.test/route-switch',
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
      profileId: 'profile-route-switch',
      profileName: 'Route switch profile',
      browserBuild: 'stealthcdp_chromium',
      serviceNames: ['DashboardSmoke'],
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
assert.deepEqual(live.inventoryPlacement, {
  lane: 'primary',
  reason: 'Live browser authority is viable.',
  rank: 0,
});
assert.deepEqual(workspaceInventoryPlacementForNode(live), live.inventoryPlacement);
assert.equal(live.inventoryClass, 'service-owned-controllable-browser');
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
assert.deepEqual(workspaceNodeLiveControlEligibility(live), {
  state: 'controllable',
  canView: true,
  canControl: true,
  reason: null,
});

const cdpMissingStream = byId(nodes, 'browser:browser-cdp-missing-stream');
assert.equal(cdpMissingStream.source, 'service-browser');
assert.equal(cdpMissingStream.group, 'active');
assert.equal(cdpMissingStream.inventoryClass, 'service-owned-view-only-browser');
assert.equal(cdpMissingStream.state, 'view-only');
assert.equal(cdpMissingStream.viewStream?.provider, 'cdp_snapshot');
assert.equal(cdpMissingStream.viewStream?.url, '/api/session-screenshot?port=37365');
assert.equal(cdpMissingStream.viewStream?.embeddable, true);
assert.equal(cdpMissingStream.viewStream?.controllable, false);
assert.equal(cdpMissingStream.viewStream?.readOnly, true);
assert.match(cdpMissingStream.viewStream?.routeSummary ?? '', /missing_stream_server/);
assert.equal(cdpMissingStream.process?.cdpPort, 37365);
assert.equal(cdpMissingStream.process?.streamPort, null);
assert.equal(action(cdpMissingStream, 'view').enabled, true);
assert.equal(action(cdpMissingStream, 'control').enabled, false);
assert.equal(workspaceNodeLiveControlEligibility(cdpMissingStream).state, 'view-only');

const readinessBlocked = byId(nodes, 'browser:browser-readiness-blocked');
assert.equal(readinessBlocked.live, true);
assert.equal(readinessBlocked.group, 'needs-attention');
assert.equal(readinessBlocked.state, 'needs-attention');
assert.equal(readinessBlocked.viewStream?.embeddable, false);
assert.equal(readinessBlocked.viewStream?.controllable, false);
assert.match(readinessBlocked.viewStream?.routeSummary ?? '', /unreachable/i);
assert.equal(action(readinessBlocked, 'view').enabled, false);
assert.equal(action(readinessBlocked, 'control').enabled, false);
assert.match(action(readinessBlocked, 'view').reason ?? '', /stream proxy timed out|unreachable/i);
assert.equal(workspaceNodeLiveControlEligibility(readinessBlocked).state, 'not-controllable');

missingId(nodes, 'browser:browser-retained');
const retainedProfileLauncher = byId(nodes, 'profile:profile-retained');
assert.equal(retainedProfileLauncher.source, 'profile');
assert.equal(retainedProfileLauncher.inventoryPlacement?.lane, 'launcher');
assert.equal(action(retainedProfileLauncher, 'launch').enabled, true);

const disconnected = byId(nodes, 'browser:browser-disconnected');
assert.equal(disconnected.group, 'needs-attention');
assert.equal(disconnected.state, 'needs-attention');
assert.equal(disconnected.attentionReason, 'Repair the retained browser record.');
assert.equal(action(disconnected, 'repair').enabled, true);

missingId(nodes, 'browser:browser-cdp-disconnected');

const control = byId(nodes, 'browser:browser-control');
assert.equal(control.group, 'active');
assert.equal(control.inventoryClass, 'service-owned-controllable-browser');
assert.equal(control.state, 'controllable');
assert.equal(control.viewStream?.provider, 'rdp_gateway');
assert.equal(control.viewStream?.controllable, true);
assert.equal(control.viewStream?.routeId, 'route-control');
assert.equal(control.viewStream?.displayAllocationId, 'display-control');
assert.equal(control.viewStream?.providerMode, 'simultaneous_view');
assert.deepEqual(control.viewStream?.viewerLeaseIds, ['viewer-control-observer']);
assert.equal(control.viewStream?.controllerLeaseId, 'viewer-control-controller');
assert.equal(control.viewStream?.operatorVisibleState, 'ready');
assert.equal(control.viewStream?.operatorVisibleReason, null);
assert.equal(control.routeBoundOwnership?.state, 'finalized');
assert.match(control.viewStream?.routeSummary ?? '', /route-control \/ display display-control \/ simultaneous view \/ 1 viewer, controller leased \/ operator visible \/ ready/);
assert.match(control.secondaryLabel, /route-control \/ display display-control/);
assert.equal(control.profileActionability?.recommendedAction, 'takeOverViewer');
assert.equal(control.profileActionability?.enabled, true);
assert.equal(control.profileActionability?.ownerBrowserId, 'browser-control');
assert.deepEqual(control.profileActionability?.ownerSessionIds, ['session-control']);
assert.match(control.profileActionability?.reason ?? '', /controller lease viewer-control-controller is active/);
assert.equal(action(control, 'add-tab').enabled, false);
assert.match(action(control, 'add-tab').reason ?? '', /take over the viewer before control/);
assert.equal(action(control, 'control').enabled, true);
assert.equal(action(control, 'external-open').enabled, true);

const privatePreferred = byId(nodes, 'browser:browser-private-preferred');
assert.equal(privatePreferred.viewStream?.routeId, 'route-private');
assert.equal(privatePreferred.viewStream?.displayAllocationId, 'display-private-a');
assert.equal(privatePreferred.viewStream?.routeSource, 'pool');

missingId(nodes, 'browser:session:dashboard-local-viewer-plan0016');
assert.equal(privatePreferred.viewStream?.providerMode, 'simultaneous_view');
assert.deepEqual(privatePreferred.viewStream?.viewerLeaseIds, ['viewer-private-a', 'viewer-private-b']);
assert.equal(privatePreferred.viewStream?.operatorVisibleState, 'ready');
assert.match(privatePreferred.viewStream?.routeSummary ?? '', /route-private \/ display display-private-a \/ simultaneous view \/ 2 viewers \/ operator visible \/ ready/);

const odolloUps = byId(nodes, 'browser:session:odollo-carrier-ups');
assert.equal(odolloUps.group, 'active');
assert.equal(odolloUps.state, 'controllable');
assert.equal(odolloUps.label, 'Odollo UPS: 1Z2G26X60300020412');
assert.match(odolloUps.secondaryLabel, /Odollo UPS \/ remote_headed \/ stealthcdp_chromium/);
assert.equal(odolloUps.attentionReason, null);
assert.match(odolloUps.viewStream?.url ?? '', /\/guacamole\/#\/client\//);
assert.equal(odolloUps.viewStream?.controllable, true);
assert.equal(odolloUps.profileActionability?.recommendedAction, 'openSharedProfileTab');
assert.equal(odolloUps.profileActionability?.enabled, true);
assert.equal(odolloUps.profileActionability?.profileId, 'stealthcdp-default');
assert.equal(odolloUps.profileActionability?.ownerBrowserId, 'session:odollo-carrier-ups');
assert.deepEqual(odolloUps.profileActionability?.ownerSessionIds, ['odollo-carrier-ups']);
assert.deepEqual(odolloUps.profileActionability?.activeTabIds, ['tab-odollo-ups']);
assert.match(odolloUps.profileActionability?.reason ?? '', /open the next operation as a tab through this owner/);
assert.equal(action(odolloUps, 'add-tab').enabled, true);
assert.equal(action(odolloUps, 'control').enabled, true);
assert.equal(action(odolloUps, 'external-open').enabled, true);

missingId(nodes, 'browser:session:session-2');
missingId(nodes, 'service-session:session-2');

const takeover = byId(nodes, 'browser:browser-takeover');
assert.equal(takeover.source, 'service-browser');
assert.equal(takeover.group, 'active');
assert.equal(takeover.inventoryClass, 'service-owned-controllable-browser');
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

const routeSwitch = byId(nodes, 'browser:browser-route-switch');
assert.equal(routeSwitch.source, 'service-browser');
assert.equal(routeSwitch.group, 'active');
assert.equal(routeSwitch.state, 'controllable');
assert.equal(routeSwitch.profileActionability?.recommendedAction, 'routeSwitch');
assert.equal(routeSwitch.profileActionability?.enabled, true);
assert.equal(routeSwitch.profileActionability?.routeId, 'route-switch-current');
assert.equal(routeSwitch.profileActionability?.displayAllocationId, 'display-route-switch-current');
assert.match(routeSwitch.profileActionability?.reason ?? '', /fresher route-pool entry/);
assert.equal(action(routeSwitch, 'add-tab').enabled, false);
assert.match(action(routeSwitch, 'add-tab').reason ?? '', /fresher route-pool entry/);

const standaloneTakeover = byId(nodes, 'service-session:session-standalone-takeover');
assert.equal(standaloneTakeover.source, 'service-session');
assert.equal(standaloneTakeover.group, 'needs-attention');
assert.equal(standaloneTakeover.inventoryClass, 'service-owned-session');
assert.equal(standaloneTakeover.state, 'blocked');
assert.equal(standaloneTakeover.takeover?.ownerLabel, 'operator: Morgan');
assert.deepEqual(standaloneTakeover.takeover?.waitingJobIds, ['job-standalone-takeover-wait']);
assert.equal(standaloneTakeover.actions.filter((item) => item.id === 'resume').length, 1);
assert.equal(action(standaloneTakeover, 'resume').enabled, false);

const daemon = byId(nodes, 'daemon-session:daemon-only');
assert.equal(daemon.source, 'daemon-session');
assert.equal(daemon.role, 'target-browser');
assert.equal(daemon.group, 'detected');
assert.equal(daemon.inventoryClass, 'detected-non-owned-browser');
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
assert.deepEqual(detectedExternal.inventoryPlacement, {
  lane: 'detected',
  reason: 'Detected non-owned browser has viable read-only evidence.',
  rank: 100,
});
assert.equal(detectedExternal.inventoryClass, 'detected-non-owned-browser');
assert.equal(detectedExternal.label, 'ChatGPT');
assert.equal(detectedExternal.viewStream?.provider, 'cdp_snapshot');
assert.equal(detectedExternal.viewStream?.url, '/api/session-screenshot?port=45011');
assert.equal(detectedExternal.viewStream?.embeddable, true);
assert.equal(detectedExternal.viewStream?.controllable, false);
assert.equal(detectedExternal.viewStream?.readOnly, true);
assert.equal(detectedExternal.process?.cdpPort, 45011);
assert.equal(detectedExternal.process?.streamPort, null);
assert.match(detectedExternal.secondaryLabel, /detected external Chrome/);
assert.match(detectedExternal.secondaryLabel, /not agent-browser service-owned/);
assert.equal(action(detectedExternal, 'inspect').enabled, true);
assert.equal(action(detectedExternal, 'view').enabled, true);
assert.equal(action(detectedExternal, 'stream').enabled, false);
assert.match(action(detectedExternal, 'stream').reason ?? '', /explicitly adopted/);
assert.equal(action(detectedExternal, 'screenshot').enabled, true);
assert.equal(action(detectedExternal, 'control').enabled, false);
assert.match(action(detectedExternal, 'control').reason ?? '', /borrow-control/);
assert.equal(action(detectedExternal, 'add-tab').enabled, false);
assert.match(action(detectedExternal, 'add-tab').reason ?? '', /borrow-control/);
assert.equal(action(detectedExternal, 'borrow-control').enabled, false);
assert.match(action(detectedExternal, 'borrow-control').reason ?? '', /not active/);
assert.equal(action(detectedExternal, 'repair').enabled, false);
assert.match(action(detectedExternal, 'repair').reason ?? '', /Non-owned browsers do not use service-owned route repair/);
assert.equal(action(detectedExternal, 'close').enabled, false);
assert.equal(action(detectedExternal, 'kill').enabled, false);
assert.match(action(detectedExternal, 'kill').reason ?? '', /explicit adoption/);

const detectedReachableNoTabs = byId(nodes, 'daemon-session:detected-empty-but-reachable');
assert.equal(detectedReachableNoTabs.source, 'daemon-session');
assert.equal(detectedReachableNoTabs.group, 'detected');
assert.equal(detectedReachableNoTabs.inventoryPlacement?.lane, 'detected');
assert.equal(detectedReachableNoTabs.inventoryClass, 'detected-non-owned-browser');
assert.equal(detectedReachableNoTabs.live, true);
assert.equal(detectedReachableNoTabs.retained, false);
assert.equal(detectedReachableNoTabs.health, 'live');
assert.equal(detectedReachableNoTabs.label, 'detected-empty-but-reachable');
assert.equal(detectedReachableNoTabs.viewStream?.provider, 'cdp_snapshot');
assert.equal(detectedReachableNoTabs.viewStream?.url, '/api/session-screenshot?port=45013');
assert.equal(detectedReachableNoTabs.viewStream?.embeddable, true);
assert.equal(detectedReachableNoTabs.process?.cdpPort, 45013);
assert.equal(detectedReachableNoTabs.process?.running, true);
assert.equal(action(detectedReachableNoTabs, 'view').enabled, true);
assert.equal(action(detectedReachableNoTabs, 'screenshot').enabled, true);

missingId(nodes, 'daemon-session:dashboard-viewer-plan0025');
assert.ok(!deriveLiveWorkspaceNodes({
  daemonSessions: [
    {
      session: 'dashboard-viewer-plan0025',
      port: 37273,
      engine: 'chrome',
    },
  ],
  daemonTabsByPort: {
    37273: [
      {
        index: 0,
        title: 'Agent Browser',
        url: 'https://agent-browser.example.test/?workspace=browser%3Asession%3Adefault&view=workspace%3Acontrol',
        type: 'page',
        active: true,
      },
    ],
  },
}).some((node) => node.id === 'daemon-session:dashboard-viewer-plan0025'));

const conflict = byId(nodes, 'profile:profile-conflict');
assert.equal(conflict.source, 'profile');
assert.equal(conflict.group, 'needs-attention');
assert.equal(conflict.inventoryClass, 'service-profile-action');
assert.equal(conflict.state, 'blocked');
assert.equal(conflict.attentionReason, 'Wait for the exclusive profile lease to clear.');
assert.equal(conflict.profileActionability?.recommendedAction, 'waitForProfileHolder');
assert.equal(conflict.profileActionability?.enabled, false);
assert.deepEqual(conflict.profileActionability?.ownerSessionIds, ['session-a', 'session-b']);
assert.match(conflict.profileActionability?.reason ?? '', /no compatible live browser row/);
assert.equal(action(conflict, 'launch').enabled, false);

missingId(nodes, 'profile:profile-auth-ready');

const manualSeeding = byId(nodes, 'profile:profile-manual-seeding');
assert.equal(manualSeeding.group, 'needs-attention');
assert.equal(manualSeeding.inventoryPlacement?.lane, 'launcher');
assert.equal(manualSeeding.state, 'needs-attention');
assert.equal(manualSeeding.attentionReason, 'Open a detached headed browser for manual login.');
assert.equal(action(manualSeeding, 'launch').enabled, false);
assert.equal(action(manualSeeding, 'seed').enabled, true);

assert.deepEqual(
  nodes.map((node) => node.inventoryPlacement?.rank),
  [...nodes.map((node) => node.inventoryPlacement?.rank)].sort((left, right) => left - right),
  'Workspace nodes should be sorted by inventory placement rank',
);
const firstAttentionIndex = nodes.findIndex((node) => node.inventoryPlacement?.lane === 'attention');
assert.ok(firstAttentionIndex > 0, 'Needs Attention rows should not lead the inventory');
assert.ok(
  nodes.slice(firstAttentionIndex).every((node) => node.inventoryPlacement?.lane === 'attention'),
  'Needs Attention rows should sort last after viable and launcher rows',
);
assert.ok(!nodes.some((node) => node.inventoryPlacement?.lane === 'retained'), 'Retained rows belong outside the live workspace rail');

const retainedNodes = deriveWorkspaceNodes({
  includeRetained: true,
  serviceBrowsers: [
    {
      id: 'retained-with-stream-url',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          url: 'https://agent-browser.example.test/guacamole/#/client/retained',
          routeId: 'retained-route',
          displayAllocationId: 'retained-display',
          controlInput: 'manual_attached_desktop',
          readOnly: false,
        },
      ],
    },
  ],
});
const retainedWithStreamUrl = byId(retainedNodes, 'browser:retained-with-stream-url');
assert.equal(retainedWithStreamUrl.inventoryPlacement?.lane, 'retained');
assert.equal(retainedWithStreamUrl.inventoryClass, 'retained-history');
assert.equal(retainedWithStreamUrl.routeBoundOwnership?.state, 'retained');
assert.equal(workspaceNodeLiveControlEligibility(retainedWithStreamUrl).state, 'not-controllable');
assert.equal(workspaceNodeLiveControlEligibility(retainedWithStreamUrl).canView, false);
assert.equal(workspaceNodeLiveControlEligibility(retainedWithStreamUrl).canControl, false);

const hiddenInventoryNodes = deriveWorkspaceNodes({
  includeRetained: true,
  daemonSessions: [
    {
      session: 'registered-empty-stream',
      port: 47001,
      engine: 'chrome',
    },
    {
      session: 'closed-unactionable-viewer',
      port: 47002,
      engine: 'chrome',
      closing: true,
    },
  ],
  profileAllocations: [
    {
      profileId: 'profile-no-action',
      profileName: 'No action profile',
      targetReadiness: [{ state: 'ready' }],
    },
  ],
});
missingId(hiddenInventoryNodes, 'daemon-session:registered-empty-stream');
missingId(hiddenInventoryNodes, 'daemon-session:closed-unactionable-viewer');
missingId(hiddenInventoryNodes, 'profile:profile-no-action');

const routeBoundOwnershipNodes = deriveWorkspaceNodes({
  serviceBrowsers: [
    {
      id: 'route-finalized',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          frameUrl: 'https://agent-browser.example.test/guacamole/#/client/finalized',
          routeId: 'route-finalized',
          displayAllocationId: 'display-finalized',
          routePoolEntryId: 'pool-finalized',
          remoteReadiness: { state: 'ready' },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
          routeBoundOwnership: { state: 'finalized' },
        },
      ],
      activeSessionIds: ['route-finalized-session'],
    },
    {
      id: 'route-pending',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          frameUrl: 'https://agent-browser.example.test/guacamole/#/client/pending',
          routeId: 'route-pending',
          displayAllocationId: 'display-pending',
          routePoolEntryId: 'pool-pending',
          remoteReadiness: { state: 'ready' },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
          routeBoundOwnership: { state: 'pending' },
        },
      ],
      activeSessionIds: ['route-pending-session'],
    },
    {
      id: 'route-rolled-back',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          frameUrl: 'https://agent-browser.example.test/guacamole/#/client/rolled-back',
          routeId: 'route-rolled-back',
          displayAllocationId: 'display-rolled-back',
          routePoolEntryId: 'pool-rolled-back',
          remoteReadiness: { state: 'ready' },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
          routeBoundOwnership: { state: 'rolled-back' },
        },
      ],
      activeSessionIds: ['route-rolled-back-session'],
    },
    {
      id: 'route-diagnostic',
      host: 'remote_headed',
      health: 'ready',
      viewStreams: [
        {
          provider: 'rdp_gateway',
          frameUrl: 'https://agent-browser.example.test/guacamole/#/client/diagnostic',
          routeId: 'route-diagnostic',
          displayAllocationId: 'display-diagnostic',
          routePoolEntryId: 'pool-diagnostic',
          remoteReadiness: { state: 'ready' },
          displayContent: { state: 'browser_window_visible' },
          controlInput: 'manual_attached_desktop',
          readOnly: false,
          routeBoundOwnership: { state: 'diagnostic' },
        },
      ],
      activeSessionIds: ['route-diagnostic-session'],
    },
  ],
  serviceSessions: [
    { id: 'route-finalized-session', browserIds: ['route-finalized'], tabIds: ['route-finalized-tab'] },
    { id: 'route-pending-session', browserIds: ['route-pending'], tabIds: ['route-pending-tab'] },
    { id: 'route-rolled-back-session', browserIds: ['route-rolled-back'], tabIds: ['route-rolled-back-tab'] },
    { id: 'route-diagnostic-session', browserIds: ['route-diagnostic'], tabIds: ['route-diagnostic-tab'] },
  ],
  serviceTabs: [
    { id: 'route-finalized-tab', browserId: 'route-finalized', sessionId: 'route-finalized-session', lifecycle: 'active', url: 'https://example.test/finalized' },
    { id: 'route-pending-tab', browserId: 'route-pending', sessionId: 'route-pending-session', lifecycle: 'active', url: 'https://example.test/pending' },
    { id: 'route-rolled-back-tab', browserId: 'route-rolled-back', sessionId: 'route-rolled-back-session', lifecycle: 'active', url: 'https://example.test/rolled-back' },
    { id: 'route-diagnostic-tab', browserId: 'route-diagnostic', sessionId: 'route-diagnostic-session', lifecycle: 'active', url: 'https://example.test/diagnostic' },
  ],
});

const finalizedOwnership = byId(routeBoundOwnershipNodes, 'browser:route-finalized');
assert.equal(finalizedOwnership.routeBoundOwnership?.state, 'finalized');
assert.equal(action(finalizedOwnership, 'control').enabled, true);
assert.equal(workspaceNodeLiveControlEligibility(finalizedOwnership).canControl, true);

for (const [id, state] of [
  ['route-pending', 'pending'],
  ['route-rolled-back', 'rolled-back'],
  ['route-diagnostic', 'diagnostic'],
]) {
  const node = byId(routeBoundOwnershipNodes, `browser:${id}`);
  assert.equal(node.routeBoundOwnership?.state, state);
  assert.equal(action(node, 'control').enabled, false);
  assert.equal(workspaceNodeLiveControlEligibility(node).canControl, false);
}

console.log('Dashboard workspace node contract smoke passed');
