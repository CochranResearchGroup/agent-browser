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

const daemonStream = {
  id: 'daemon-stream:qbo-soylei',
  provider: 'cdp_screencast',
  routeId: 'daemon:qbo-soylei',
  url: 'http://127.0.0.1:38285/',
  readiness: { state: 'ready' },
};
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

console.log('dashboard view-stream tests passed');
