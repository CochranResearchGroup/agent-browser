#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  canControlViewStream,
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  controlInputLabel,
  viewStreamCapabilityLabel,
  viewStreamControlTitle,
  viewStreamDashboardFrameUrl,
  viewStreamLabel,
  viewStreamOpenTitle,
  viewStreamReadinessLabel,
  viewStreamRouteSummary,
} from '../packages/dashboard/src/lib/service-view-streams.ts';
import {
  selectWorkspaceViewerRoute,
  workspaceRecoveryFailureMessage,
} from '../packages/dashboard/src/lib/workspace-recovery.ts';
import {
  planAutomaticWorkspaceConnection,
  resolveWorkspaceViewSources,
  workspaceConnectionReadinessGeneration,
  workspaceViewerRouteIsAttached,
} from '../packages/dashboard/src/lib/workspace-view-connection.ts';
import {
  borrowForeignCdpControl,
  dispatchForeignCdpInput,
  fetchForeignCdpScreenshot,
  foreignCdpScreenshotUrl,
  readForeignCdpControlStatus,
  releaseForeignCdpControl,
} from '../packages/dashboard/src/lib/foreign-cdp-control.ts';

const route = {
  id: 'remote-headed-view',
  provider: 'rdp_gateway',
  controlInput: 'manual_attached_desktop',
  url: 'http://127.0.0.1:8080/rdp/session',
  frameUrl: 'http://127.0.0.1:8080/guacamole/#/client/route-a',
  externalUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
  routeDescriptor: {
    localEmbedUrl: 'http://127.0.0.1:8080/guacamole/#/client/route-a',
    publicOperatorUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
    dashboardEmbedUrl: 'https://agent-browser.example.test/guacamole/#/client/route-a',
  },
  routeId: 'route-a',
  displayAllocationId: 'display-a',
  connectionName: 'Browser A',
  routeSource: 'pool',
  providerMode: 'simultaneous_view',
  readiness: { state: 'ready' },
  readOnly: false,
};

assert.equal(viewStreamLabel(route), 'rdp gateway');
assert.equal(controlInputLabel(route), 'manual attached desktop');
assert.equal(viewStreamCapabilityLabel(route), 'rdp gateway / manual attached desktop');
assert.equal(canEmbedViewStream(route), true);
assert.equal(canControlViewStream(route), true);
assert.equal(canOpenViewStream(route), true);
assert.equal(canOpenControlViewStream(route), true);
assert.equal(viewStreamOpenTitle(route), 'Open rdp gateway in the dashboard.');
assert.equal(viewStreamReadinessLabel(route), 'ready');
assert.match(viewStreamRouteSummary(route), /route-a.*display display-a.*ready/);
assert.equal(
  viewStreamDashboardFrameUrl(route, 'http://127.0.0.1:4848/'),
  route.routeDescriptor.localEmbedUrl,
);
assert.equal(
  viewStreamDashboardFrameUrl(route, 'https://dashboard.example.test/'),
  route.routeDescriptor.dashboardEmbedUrl,
);
const loopbackOnlyExternalRoute = {
  ...route,
  frameUrl: 'http://127.0.0.1:8093/guacamole/#/client/route-a',
  externalUrl: null,
  routeDescriptor: {
    localEmbedUrl: 'http://127.0.0.1:8093/guacamole/#/client/route-a',
    publicOperatorUrl: 'https://agent-browser.example.test',
    dashboardEmbedUrl: 'http://127.0.0.1:8093/guacamole/#/client/route-a',
  },
};
assert.equal(
  viewStreamDashboardFrameUrl(loopbackOnlyExternalRoute, 'https://agent-browser.example.test/remote-view/opaque'),
  'https://agent-browser.example.test/guacamole/#/client/route-a',
  'external dashboards must rebase the internal Guacamole path onto the public origin',
);
assert.equal(
  viewStreamDashboardFrameUrl({
    ...loopbackOnlyExternalRoute,
    frameUrl: 'http://127.0.0.1:8093/not-guacamole',
    routeDescriptor: {
      ...loopbackOnlyExternalRoute.routeDescriptor,
      localEmbedUrl: 'http://127.0.0.1:8093/not-guacamole',
      dashboardEmbedUrl: 'http://127.0.0.1:8093/not-guacamole',
    },
  }, 'https://agent-browser.example.test/remote-view/opaque'),
  null,
  'arbitrary loopback paths must never be projected through public ingress',
);

const daemonStream = {
  id: 'daemon-stream:qbo-soylei',
  provider: 'cdp_screencast',
  controlInput: 'cdp_input',
  routeId: 'daemon:qbo-soylei',
  url: 'http://127.0.0.1:38285/',
  readiness: { state: 'ready' },
};
const blockedDaemonStream = {
  ...daemonStream,
  readiness: { state: 'stale_target', reason: 'Selected target is stale' },
};
const automaticDesktop = resolveWorkspaceViewSources({
  streams: [blockedDaemonStream, route],
  selected: blockedDaemonStream,
  mode: 'control',
});
assert.equal(automaticDesktop.selected?.stream, route);
assert.equal(automaticDesktop.selected?.label, 'Desktop');
assert.equal(automaticDesktop.selectionReason, 'automatic-ready-fallback');
const explicitLivePage = resolveWorkspaceViewSources({
  streams: [daemonStream, route],
  selected: daemonStream,
  mode: 'control',
});
assert.equal(explicitLivePage.selected?.stream, daemonStream);
assert.equal(explicitLivePage.selectionReason, 'selected-ready');
const duplicateDesktopChoices = resolveWorkspaceViewSources({
  streams: [
    route,
    { ...route, id: 'remote-headed-view-duplicate' },
    {
      ...route,
      id: 'remote-headed-view-b',
      routeId: 'route-b',
      connectionName: 'Browser B',
      displayAllocationId: 'display-b',
    },
  ],
  selected: route,
  mode: 'control',
});
assert.equal(duplicateDesktopChoices.choices.length, 2);
assert.deepEqual(
  duplicateDesktopChoices.choices.map((choice) => choice.label),
  ['Desktop — Browser A', 'Desktop — Browser B'],
);
const recoveryPlan = planAutomaticWorkspaceConnection({
  browserId: 'browser-a',
  browserLive: true,
  mode: 'control',
  sourceResolution: resolveWorkspaceViewSources({ streams: [route], selected: route, mode: 'control' }),
  currentStream: route,
  routeRecoveryAction: 'service_remote_view_browser_reattach',
  readinessGeneration: 'reattachable-stale-route:2026-08-14T12:00:00Z',
  viewerRoute: route,
  viewerLeaseIds: [],
  attemptedActionKeys: [],
});
assert.equal(recoveryPlan.action?.kind, 'recover-route');
assert.equal(recoveryPlan.action?.serviceAction, 'service_remote_view_browser_reattach');
const repeatedRecoveryPlan = planAutomaticWorkspaceConnection({
  browserId: 'browser-a',
  browserLive: true,
  mode: 'control',
  sourceResolution: resolveWorkspaceViewSources({ streams: [route], selected: route, mode: 'control' }),
  currentStream: route,
  routeRecoveryAction: 'service_remote_view_browser_reattach',
  readinessGeneration: 'reattachable-stale-route:2026-08-14T12:00:00Z',
  viewerRoute: route,
  viewerLeaseIds: [],
  attemptedActionKeys: [recoveryPlan.action?.attemptKey],
});
assert.equal(repeatedRecoveryPlan.action, null);
assert.equal(repeatedRecoveryPlan.status, 'action-required');
const attachedRoute = {
  ...route,
  attachability: { state: 'attached_ready' },
  remoteReadiness: { state: 'ready' },
};
assert.equal(workspaceViewerRouteIsAttached(attachedRoute), true);
const readinessGeneration = workspaceConnectionReadinessGeneration(attachedRoute, null);
const viewerPlan = planAutomaticWorkspaceConnection({
  browserId: 'browser-a',
  browserLive: true,
  mode: 'control',
  sourceResolution: resolveWorkspaceViewSources({ streams: [attachedRoute], selected: attachedRoute, mode: 'control' }),
  currentStream: attachedRoute,
  routeRecoveryAction: null,
  readinessGeneration,
  viewerRoute: attachedRoute,
  viewerRouteReady: true,
  viewerLeaseIds: [],
  attemptedActionKeys: [],
});
assert.equal(viewerPlan.action?.kind, 'request-viewer-lease');
const repeatedViewerPlan = planAutomaticWorkspaceConnection({
  browserId: 'browser-a',
  browserLive: true,
  mode: 'control',
  sourceResolution: resolveWorkspaceViewSources({ streams: [attachedRoute], selected: attachedRoute, mode: 'control' }),
  currentStream: attachedRoute,
  routeRecoveryAction: null,
  readinessGeneration,
  viewerRoute: attachedRoute,
  viewerRouteReady: true,
  viewerLeaseIds: [],
  attemptedActionKeys: [viewerPlan.action?.attemptKey],
});
assert.equal(repeatedViewerPlan.action, null);
assert.equal(repeatedViewerPlan.status, 'action-required');
assert.equal(
  selectWorkspaceViewerRoute([daemonStream, route], daemonStream)?.routeId,
  'route-a',
);
assert.equal(selectWorkspaceViewerRoute([daemonStream], daemonStream), null);
assert.equal(
  workspaceRecoveryFailureMessage(
    { code: 'remote_view_route_not_found', error: "remote view route 'route-a' not found" },
    'service_viewer_lease_request',
  ),
  "remote_view_route_not_found: remote view route 'route-a' not found",
);
assert.equal(
  workspaceRecoveryFailureMessage(
    { code: 'remote_view_route_not_found', error: "remote_view_route_not_found: route 'route-a' not found" },
    'service_viewer_lease_request',
  ),
  "remote_view_route_not_found: route 'route-a' not found",
);
assert.equal(
  workspaceRecoveryFailureMessage({ error: 'backend unavailable' }, 'service_viewer_lease_request'),
  'backend unavailable',
);
assert.equal(
  workspaceRecoveryFailureMessage({}, 'service_viewer_lease_request'),
  'service_viewer_lease_request was not accepted',
);

const blocked = { ...route, readiness: { state: 'stale_target', reason: 'Selected tab is not visible' } };
assert.equal(canOpenViewStream(blocked), false);
assert.equal(canOpenControlViewStream(blocked), false);
assert.match(viewStreamOpenTitle(blocked), /Selected tab is not visible/);
assert.equal(
  viewStreamControlTitle({ ...route, readOnly: true, controlInput: null }),
  'The service marked this stream as view-only or did not report a control input provider.',
);

assert.equal(
  foreignCdpScreenshotUrl(45011, 'target-a', 'png'),
  '/api/session-screenshot?port=45011&format=png&targetId=target-a',
);
let captureRequest = null;
const captured = await fetchForeignCdpScreenshot({
  port: 45011,
  targetId: 'target-a',
  format: 'png',
  fetcher: async (input, init) => {
    captureRequest = { input, init };
    return new Response(JSON.stringify({
      success: true,
      targetId: 'target-a',
      title: 'Detected page',
      format: 'png',
      dataUrl: 'data:image/png;base64,AA==',
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  },
});
assert.equal(captureRequest.input, '/api/session-screenshot?port=45011&format=png&targetId=target-a');
assert.equal(captured.dataUrl, 'data:image/png;base64,AA==');

const requests = [];
const fetcher = async (input, init) => {
  requests.push({ input, init });
  if (input.startsWith('/api/foreign-cdp/control')) {
    return new Response(JSON.stringify({ active: false, lifecycleOwnership: false }), { status: 200 });
  }
  if (input === '/api/foreign-cdp/borrow') {
    return new Response(JSON.stringify({
      active: true,
      grantId: 'grant-a',
      expiresAt: '2026-08-05T12:05:00Z',
      allowedOperations: ['pointer', 'keyboard', 'wheel'],
      lifecycleOwnership: false,
    }), { status: 200 });
  }
  return new Response(JSON.stringify({ success: true, active: false, lifecycleOwnership: false }), { status: 200 });
};
await readForeignCdpControlStatus({ port: 45011, targetId: 'target-a', fetcher });
await borrowForeignCdpControl({ port: 45011, targetId: 'target-a', reason: 'Diagnosis', ttlSeconds: 300, fetcher });
await dispatchForeignCdpInput({
  port: 45011,
  targetId: 'target-a',
  grantId: 'grant-a',
  input: { kind: 'wheel', deltaX: 0, deltaY: 120 },
  fetcher,
});
await releaseForeignCdpControl({ port: 45011, targetId: 'target-a', grantId: 'grant-a', fetcher });
assert.deepEqual(requests.map((request) => request.input), [
  '/api/foreign-cdp/control?port=45011&targetId=target-a',
  '/api/foreign-cdp/borrow',
  '/api/foreign-cdp/input',
  '/api/foreign-cdp/release',
]);

const page = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
assert.match(page, /useWorkspaceViewPreferences[\s\S]*projection=\{selectedWorkspace\.projection\}/);
assert.match(viewport, /Borrow control[\s\S]*Release control/);
assert.match(viewport, /dispatchForeignCdpInput[\s\S]*canControl=\{foreignBorrow\?\.active === true\}/);
assert.match(viewport, /projection\.selected[\s\S]*projection\.tiles/);
assert.doesNotMatch(viewport, /fetch\(`\$\{serviceBase\(activePort\)\}\/status`\)/);
assert.match(viewport, /selectWorkspaceViewerRoute\(streamChoices, stream\)/);
assert.match(viewport, /workspaceViewerRoute\?\.routeId/);
assert.match(viewport, /WorkspaceSourceMenu/);
assert.match(viewport, /Advanced connection controls/);
assert.match(viewport, /Retry connection/);
assert.match(viewport, /Take control/);
assert.doesNotMatch(viewport, /Use \{viewStreamLabel/);
assert.doesNotMatch(viewport, /Wake stream/);
assert.doesNotMatch(viewport, /aria-label="Refresh workspace viewport"/);
assert.doesNotMatch(viewport, /aria-label="Reconnect viewer lease"/);

console.log('dashboard view-stream tests passed');
