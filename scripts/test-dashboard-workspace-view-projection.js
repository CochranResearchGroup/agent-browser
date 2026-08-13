#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  applyStatusObservationsToWorkspaceSources,
  projectWorkspaceViews,
} from '../packages/dashboard/src/lib/workspace-view-projection.ts';
import {
  deriveWorkspaceNodes,
  deriveWorkspaceViewAuthorityLedger,
  projectServiceWorkspaceViews,
} from '../packages/dashboard/src/lib/service-workspaces.ts';
import { buildSelectedWorkspaceContext } from '../packages/dashboard/src/lib/selected-workspace-context.ts';
import { deriveWorkspaceViewportReadiness } from '../packages/dashboard/src/lib/workspace-viewport-state.ts';

const selection = (values = {}) => ({
  workspaceId: null,
  browserId: null,
  sessionId: null,
  tabId: null,
  profileId: null,
  jobId: null,
  ...values,
});

const authority = (subjectKey, values = {}) => ({
  subjectKey,
  authoritySource: 'service-status-compatibility',
  browserId: subjectKey.replace(/^(?:browser|daemon):/, ''),
  workspaceId: subjectKey,
  inventoryClass: 'service-owned-controllable-browser',
  lifecycle: { state: 'controllable', live: true, retained: false, health: 'ready' },
  routeBoundOwnership: { state: 'finalized', reason: null },
  operatorVisibleProof: { state: 'ready', reason: null, routeId: 'route-a', displayAllocationId: 'display-a' },
  lifecycleActions: [{ id: 'close', enabled: true }],
  presentationActionCeilings: {
    view: { allowed: true, reason: null },
    control: { allowed: true, reason: null },
    stream: { allowed: true, reason: null },
    screenshot: { allowed: true, reason: null },
  },
  diagnostics: [],
  ...values,
});

const rdp = {
  id: 'rdp-a',
  provider: 'rdp_gateway',
  controlInput: 'manual_attached_desktop',
  url: 'http://127.0.0.1:8080/guacamole/#/client/a',
  frameUrl: 'http://127.0.0.1:8080/guacamole/#/client/a',
  externalUrl: 'https://operator.example.test/guacamole/#/client/a',
  publicOperatorUrl: 'https://operator.example.test/guacamole/#/client/a',
  routeId: 'route-a',
  displayAllocationId: 'display-private-a',
  routeSource: 'pool',
  providerMode: 'simultaneous_view',
  readiness: { state: 'ready' },
};

const cdp = {
  id: 'cdp-a',
  provider: 'cdp_screencast',
  controlInput: 'cdp_input',
  url: 'http://127.0.0.1:9223/',
  readiness: { state: 'ready' },
};

const canonicalInput = {
  sources: {
    serviceBrowsers: [{ id: 'browser-a', activeSessionIds: ['session-a'], viewStreams: [rdp] }],
    daemonSessions: [{ session: 'session-a', port: 9223, engine: 'chrome' }],
    serviceTabs: [
      { id: 'stale', browserId: 'browser-a', lifecycle: 'closed', title: 'about:blank', url: 'about:blank' },
      { id: 'live', browserId: 'browser-a', targetId: 'target-live', lifecycle: 'active', title: 'Live', url: 'https://example.test/' },
    ],
  },
  authorityLedger: { 'browser:browser-a': authority('browser:browser-a') },
  intent: {
    selection: selection({ browserId: 'browser-a', tabId: 'stale' }),
    mode: 'view',
    dashboardHref: 'https://dashboard.example.test/',
  },
};

const canonical = projectWorkspaceViews(canonicalInput);
assert.equal(canonical.selected.stream.provider, 'rdp_gateway');
assert.equal(canonical.selected.frameUrl, 'https://operator.example.test/guacamole/#/client/a');
assert.equal(canonical.selected.tabSelection.tab.id, 'live');
assert.equal(canonical.selected.tabSelection.recoveredFromStaleSelection, true);
assert.equal(canonical.selected.authority.inventoryClass, 'service-owned-controllable-browser');

const unrouted = projectWorkspaceViews({
  ...canonicalInput,
  sources: {
    ...canonicalInput.sources,
    serviceBrowsers: [{
      id: 'browser-a',
      activeSessionIds: ['session-a'],
      viewStreams: [{
        ...rdp,
        routeId: null,
        attachability: {
          state: 'reattachable_no_route',
          recommendedAction: 'service_remote_view_browser_reattach',
          reason: 'browser is live but no remote-view route is selected',
        },
      }],
    }],
  },
});
assert.equal(unrouted.selected.canView, false);
assert.equal(unrouted.selected.canControl, false);
assert.equal(unrouted.selected.readiness.state, 'reattachable_no_route');
assert.equal(unrouted.selected.readiness.recoveryAction, 'service_remote_view_browser_reattach');
assert.match(unrouted.selected.readiness.reason ?? '', /no remote-view route/i);

const observedSources = applyStatusObservationsToWorkspaceSources({
  serviceBrowsers: [{ id: 'browser-observed', viewStreams: [{ id: 'stream-observed', provider: 'rdp_gateway' }] }],
}, {
  schemaVersion: 1,
  observations: {
    viewStreams: [{
      browserId: 'browser-observed',
      streamId: 'stream-observed',
      state: 'observed',
      validUntil: '2026-08-09T21:00:09.000Z',
      routePresentation: { frameUrl: '/fresh-frame', externalUrl: '/fresh-external' },
    }],
  },
}, Date.parse('2026-08-09T21:00:05.000Z'));
assert.equal(observedSources.serviceBrowsers[0].viewStreams[0].frameUrl, '/fresh-frame');
const staleSources = applyStatusObservationsToWorkspaceSources({
  serviceBrowsers: [{ id: 'browser-observed', viewStreams: [{ id: 'stream-observed', provider: 'rdp_gateway' }] }],
}, {
  schemaVersion: 1,
  observations: {
    viewStreams: [{
      browserId: 'browser-observed',
      streamId: 'stream-observed',
      state: 'observed',
      validUntil: '2026-08-09T21:00:09.000Z',
      routePresentation: { frameUrl: '/stale-frame', externalUrl: '/stale-external' },
    }],
  },
}, Date.parse('2026-08-09T21:00:10.000Z'));
assert.equal(staleSources.serviceBrowsers[0].viewStreams[0].frameUrl, undefined);
const unknownVersionSources = applyStatusObservationsToWorkspaceSources(canonicalInput.sources, {
  schemaVersion: 2,
  observations: { viewStreams: [] },
});
assert.equal(unknownVersionSources, canonicalInput.sources);

const liveBlank = projectWorkspaceViews({
  ...canonicalInput,
  sources: {
    ...canonicalInput.sources,
    serviceTabs: [
      { id: 'blank', browserId: 'browser-a', lifecycle: 'active', title: 'about:blank', url: 'about:blank' },
      { id: 'content', browserId: 'browser-a', lifecycle: 'active', title: 'Content', url: 'https://example.test/content' },
    ],
  },
  intent: {
    ...canonicalInput.intent,
    selection: selection({ browserId: 'browser-a', tabId: 'blank' }),
  },
});
assert.equal(liveBlank.selected.tabSelection.tab.id, 'blank');
assert.equal(liveBlank.selected.tabSelection.selectionEvidence, 'selected-live-blank');
assert.equal(liveBlank.selected.tabSelection.recoveredFromStaleSelection, false);
assert.equal(liveBlank.selected.tabSelection.staleSelectionId, 'blank');

const explicitCdp = projectWorkspaceViews({
  ...canonicalInput,
  intent: {
    ...canonicalInput.intent,
    preferences: {
      selected: { subjectKey: 'browser:browser-a', provider: 'cdp_screencast' },
    },
  },
});
assert.equal(explicitCdp.selected.stream.provider, 'cdp_screencast');
assert.equal(explicitCdp.selected.selectionReason, 'explicit-provider');
assert.equal(explicitCdp.selected.authority.inventoryClass, 'service-owned-controllable-browser');

const blockedAuthority = projectWorkspaceViews({
  ...canonicalInput,
  authorityLedger: {
    'browser:browser-a': authority('browser:browser-a', {
      inventoryClass: 'service-owned-diagnostic-browser',
      operatorVisibleProof: { state: 'wrong_tab', reason: 'Selected tab is not visible.', routeId: 'route-a', displayAllocationId: 'display-a' },
      presentationActionCeilings: {
        view: { allowed: false, reason: 'Canonical operator proof is diagnostic.' },
        control: { allowed: false, reason: 'Canonical operator proof is diagnostic.' },
        stream: { allowed: false, reason: 'Canonical operator proof is diagnostic.' },
        screenshot: { allowed: false, reason: 'Canonical operator proof is diagnostic.' },
      },
    }),
  },
});
assert.equal(blockedAuthority.selected.stream.provider, 'rdp_gateway');
assert.equal(blockedAuthority.selected.canView, false);
assert.equal(blockedAuthority.selected.canControl, false);
assert.equal(blockedAuthority.selected.authority.inventoryClass, 'service-owned-diagnostic-browser');
assert.equal(blockedAuthority.selected.readiness.source, 'authority');

const missingAuthority = projectWorkspaceViews({ ...canonicalInput, authorityLedger: {} });
assert.equal(missingAuthority.selected.authorityPreservation, 'missing');
assert.equal(missingAuthority.selected.canView, false);
assert.equal(missingAuthority.selected.canControl, false);

const manualSubject = 'manual-runtime:browser-b';
const manual = projectWorkspaceViews({
  sources: {
    selectedContext: {
      node: { id: manualSubject, label: 'Manual browser' },
      stream: { provider: 'rdp_gateway', routeId: 'route-b', url: 'http://127.0.0.1:8092/route-b', embeddable: true },
    },
    remoteViewRoutes: {
      'route-b': {
        id: 'route-b',
        provider: 'rdp_gateway',
        localEmbedUrl: 'http://127.0.0.1:8092/route-b',
        publicOperatorUrl: 'https://operator.example.test/route-b',
        controlInput: 'manual_attached_desktop',
        readiness: { state: 'ready' },
      },
    },
  },
  authorityLedger: {
    [manualSubject]: authority(manualSubject, {
      authoritySource: 'daemon-detection',
      inventoryClass: 'manual-runtime-browser',
    }),
  },
  intent: {
    selection: selection({ workspaceId: manualSubject }),
    mode: 'view',
    dashboardHref: 'https://dashboard.example.test/',
  },
});
assert.equal(manual.selected.frameUrl, 'https://operator.example.test/route-b');
assert.equal(manual.selected.authority.inventoryClass, 'manual-runtime-browser');
const manualNode = deriveWorkspaceNodes({
  manualBrowsers: [{
    id: manualSubject,
    runtimeProfile: 'manual-profile',
    pid: 8811,
    browserFamily: 'chromium',
    launchMode: 'manual',
    targetUrl: 'https://example.test/manual',
    remoteViewRouteId: 'route-b',
    remoteViewUrl: 'https://operator.example.test/route-b',
    remoteControlAvailable: true,
  }],
}).find((node) => node.id === manualSubject);
assert.ok(manualNode);
assert.equal(manualNode.inventoryClass, 'manual-runtime-browser');
assert.equal(manualNode.actions.find((item) => item.id === 'view')?.enabled, true);
assert.equal(manualNode.actions.find((item) => item.id === 'control')?.enabled, true);

const foreignSubject = 'daemon:foreign';
const foreign = projectWorkspaceViews({
  sources: { daemonSessions: [{ session: 'foreign', port: 45011, detected: true, ownership: 'foreign_cdp' }] },
  authorityLedger: {
    [foreignSubject]: authority(foreignSubject, {
      authoritySource: 'daemon-detection',
      inventoryClass: 'detected-non-owned-browser',
      lifecycle: { state: 'view-only', live: true, retained: false, health: 'ready' },
      presentationActionCeilings: {
        view: { allowed: true, reason: null },
        control: { allowed: false, reason: 'Borrow is required.' },
        stream: { allowed: true, reason: null },
        screenshot: { allowed: true, reason: null },
      },
    }),
  },
  intent: { selection: selection({ sessionId: 'foreign' }), mode: 'control' },
});
assert.equal(foreign.selected.stream.provider, 'cdp_snapshot');
assert.equal(foreign.selected.authority.inventoryClass, 'detected-non-owned-browser');
assert.equal(foreign.selected.canControl, false);
const foreignBeforeBorrow = foreign.selected.authority.lifecycle;
const foreignBorrowReadiness = deriveWorkspaceViewportReadiness({
  hasBrowser: true,
  browserHealth: 'ready',
  hasStream: true,
  streamProvider: foreign.selected.stream.provider,
  streamUrl: foreign.selected.frameUrl,
  streamReadiness: foreign.selected.stream.readiness,
  canEmbed: foreign.selected.canEmbed,
  canControl: foreign.selected.canControl,
  mode: 'control',
  preflightStatus: 'ready',
  frameIssueKind: 'taken-over',
});
assert.equal(foreignBorrowReadiness.nextAction, 'take_over');
assert.deepEqual(foreign.selected.authority.lifecycle, foreignBeforeBorrow);
assert.equal(foreign.selected.authority.lifecycle.state, 'view-only');

const crossedBase = {
  serviceBrowsers: [{
    id: 'crossed-browser',
    profileId: 'crossed-profile',
    host: 'local_headed',
    health: 'ready',
    pid: 7701,
    activeSessionIds: ['crossed-session'],
    viewStreams: [{
      id: 'crossed-stream',
      provider: 'cdp_screencast',
      url: 'http://127.0.0.1:47701/',
      controlInput: 'cdp_input',
      readiness: { state: 'ready' },
    }],
  }],
  serviceSessions: [{ id: 'crossed-session', browserIds: ['crossed-browser'], tabIds: ['crossed-tab'] }],
  serviceTabs: [{ id: 'crossed-tab', browserId: 'crossed-browser', lifecycle: 'active', url: 'https://example.test/crossed' }],
};
const crossedBlocked = {
  ...crossedBase,
  serviceBrowsers: [{
    ...crossedBase.serviceBrowsers[0],
    viewStreams: [{
      ...crossedBase.serviceBrowsers[0].viewStreams[0],
      readiness: { state: 'unreachable', reason: 'projection transport unavailable' },
    }],
  }],
};
const crossedReadyLedger = deriveWorkspaceViewAuthorityLedger(crossedBase);
const crossedBlockedLedger = deriveWorkspaceViewAuthorityLedger(crossedBlocked);
assert.deepEqual(crossedBlockedLedger['browser:crossed-browser'], crossedReadyLedger['browser:crossed-browser']);
const crossedReadyNode = deriveWorkspaceNodes(crossedBase).find((node) => node.id === 'browser:crossed-browser');
const crossedBlockedNode = deriveWorkspaceNodes(crossedBlocked).find((node) => node.id === 'browser:crossed-browser');
assert.ok(crossedReadyNode && crossedBlockedNode);
assert.equal(crossedReadyNode.inventoryClass, crossedBlockedNode.inventoryClass);
assert.equal(crossedReadyNode.state, crossedBlockedNode.state);
assert.equal(crossedReadyNode.actions.find((item) => item.id === 'view')?.enabled, true);
assert.equal(crossedBlockedNode.actions.find((item) => item.id === 'view')?.enabled, false);

const inspectorProjection = projectServiceWorkspaceViews(crossedBase, { mode: 'inspect' });
const inspectorView = inspectorProjection.candidates.find((candidate) => candidate.browser.id === 'crossed-browser');
assert.ok(inspectorView);
assert.deepEqual(inspectorView.authority, crossedReadyLedger['browser:crossed-browser']);
assert.equal(inspectorView.stream?.id, crossedReadyNode.viewStream?.provider === 'cdp_screencast' ? 'crossed-stream' : null);
assert.equal(inspectorView.canView, crossedReadyNode.actions.find((item) => item.id === 'view')?.enabled);
assert.equal(inspectorView.canControl, crossedReadyNode.actions.find((item) => item.id === 'control')?.enabled);

const contextSnapshot = {
  serviceBrowsers: [{
    id: 'context-browser',
    health: 'ready',
    activeSessionIds: ['context-session'],
    viewStreams: [
      { id: 'context-rdp', provider: 'rdp_gateway', routeId: 'context-route', controlInput: 'manual_attached_desktop', readiness: { state: 'ready' } },
      { id: 'context-cdp', provider: 'cdp_screencast', url: 'http://127.0.0.1:47702/', controlInput: 'cdp_input', readiness: { state: 'ready' } },
    ],
  }],
  serviceSessions: [{ id: 'context-session', browserIds: ['context-browser'], tabIds: ['context-tab'] }],
  serviceTabs: [{ id: 'context-tab', browserId: 'context-browser', lifecycle: 'active', url: 'https://example.test/context' }],
  remoteViewRoutes: {
    'context-route': {
      id: 'context-route',
      provider: 'rdp_gateway',
      routeId: 'context-route',
      localEmbedUrl: 'http://127.0.0.1:8092/context-route',
      publicOperatorUrl: 'https://operator.example.test/context-route',
      controlInput: 'manual_attached_desktop',
      readiness: { state: 'ready' },
    },
  },
};
const contextSelection = selection({ browserId: 'context-browser', tabId: 'context-tab' });
const contextNodes = deriveWorkspaceNodes({ ...contextSnapshot, includeRetained: true, includeHidden: true });
const baseContext = buildSelectedWorkspaceContext({ ...contextSnapshot, selection: contextSelection, nodes: contextNodes, refreshedAt: 42 });
const contextAuthority = deriveWorkspaceViewAuthorityLedger(contextSnapshot);
const contextSources = {
  serviceBrowsers: contextSnapshot.serviceBrowsers,
  serviceTabs: contextSnapshot.serviceTabs,
  remoteViewRoutes: contextSnapshot.remoteViewRoutes,
  selectedContext: { node: baseContext.node, stream: contextSnapshot.serviceBrowsers[0].viewStreams[0] },
};
const contextAutomatic = projectWorkspaceViews({
  sources: contextSources,
  authorityLedger: contextAuthority,
  intent: { selection: contextSelection, mode: 'view', dashboardHref: 'https://dashboard.example.test/' },
});
const contextPreferred = projectWorkspaceViews({
  sources: contextSources,
  authorityLedger: contextAuthority,
  intent: {
    selection: contextSelection,
    mode: 'view',
    dashboardHref: 'https://dashboard.example.test/',
    preferences: { selected: { subjectKey: 'browser:context-browser', provider: 'rdp_gateway' } },
  },
});
assert.equal(contextAutomatic.selected.stream.provider, 'cdp_screencast');
assert.equal(contextPreferred.selected.stream.provider, 'rdp_gateway');
assert.equal(contextPreferred.selected.frameUrl, 'https://operator.example.test/context-route');
assert.equal(contextPreferred.selected.authority, contextAutomatic.selected.authority);
const viewportReady = deriveWorkspaceViewportReadiness({
  hasBrowser: true,
  browserHealth: baseContext.browser?.health,
  hasStream: Boolean(contextPreferred.selected.stream),
  streamProvider: contextPreferred.selected.stream?.provider,
  streamUrl: contextPreferred.selected.frameUrl,
  streamReadiness: contextPreferred.selected.stream?.readiness,
  canEmbed: contextPreferred.selected.canEmbed,
  canControl: contextPreferred.selected.canControl,
  mode: 'view',
  preflightStatus: 'ready',
});
assert.equal(viewportReady.status, 'ready');
assert.match(viewportReady.evidence, /stream URL is present/);

const genericReadinessFailure = deriveWorkspaceViewportReadiness({
  hasBrowser: true,
  browserHealth: 'ready',
  hasStream: true,
  streamProvider: 'rdp_gateway',
  streamUrl: 'https://operator.example.test/context-route',
  streamReadiness: { component: 'readiness', state: 'failed' },
  canEmbed: true,
  canControl: true,
  mode: 'view',
  preflightStatus: 'ready',
});
assert.equal(genericReadinessFailure.title, 'readiness failed');
assert.equal(
  genericReadinessFailure.recoveryCopy,
  'Inspect readiness before opening the workspace stream.',
);
assert.doesNotMatch(
  `${genericReadinessFailure.title} ${genericReadinessFailure.recoveryCopy}`,
  /readiness readiness/i,
);

const browserBStream = { ...rdp, id: 'rdp-b', routeId: 'route-b', displayAllocationId: 'display-b' };
const tiles = projectWorkspaceViews({
  sources: {
    serviceBrowsers: [
      { id: 'browser-a', viewStreams: [rdp, cdp] },
      { id: 'browser-b', viewStreams: [browserBStream, { ...cdp, id: 'cdp-b' }] },
    ],
  },
  authorityLedger: {
    'browser:browser-a': authority('browser:browser-a'),
    'browser:browser-b': authority('browser:browser-b'),
  },
  intent: {
    selection: selection({ browserId: 'browser-a' }),
    mode: 'tile',
    preferences: {
      selected: { subjectKey: 'browser:browser-a', provider: 'cdp_screencast' },
      byBrowserId: {
        'browser-a': { streamKey: 'id:rdp-a' },
        'browser-b': { streamKey: 'id:cdp-b' },
      },
    },
  },
});
assert.equal(tiles.selected.stream.id, 'cdp-a');
assert.equal(tiles.tiles.find((tile) => tile.browser.id === 'browser-a').stream.id, 'rdp-a');
assert.equal(tiles.tiles.find((tile) => tile.browser.id === 'browser-b').stream.id, 'cdp-b');

console.log('dashboard workspace view projection tests passed');
