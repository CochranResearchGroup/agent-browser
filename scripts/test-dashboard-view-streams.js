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
  viewStreamLabel,
  viewStreamOpenTitle,
  viewStreamReadinessLabel,
  viewStreamRouteLabel,
  viewStreamRouteSummary,
  viewStreamLeaseLabel,
} from '../packages/dashboard/src/lib/service-view-streams.ts';
import {
  compactWorkspaceViewportReadinessComponents,
  deriveWorkspaceViewportReadiness,
  deriveWorkspaceViewportUxState,
  workspaceViewportReadinessStatusLabel,
  workspaceViewportUxStateLabel,
} from '../packages/dashboard/src/lib/workspace-viewport-state.ts';

const dashboardPage = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
const workspaceNavigator = readFileSync('packages/dashboard/src/components/workspace-navigator.tsx', 'utf8');
const workspaceViewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const css = readFileSync('packages/dashboard/src/app/globals.css', 'utf8');
const rdpAutologinSetup = readFileSync('scripts/setup-rdp-autologin-user.sh', 'utf8');

const rdpGatewayStream = {
  id: 'remote-headed-view',
  provider: 'rdp_gateway',
  controlInput: 'manual_attached_desktop',
  url: 'http://127.0.0.1:8080/rdp/session',
  frameUrl: 'http://127.0.0.1:8080/guacamole/#/client/route-a',
  routeId: 'route-a',
  displayAllocationId: 'display-a',
  connectionId: 'guac-a',
  connectionName: 'Browser A',
  providerMode: 'simultaneous_view',
  viewerLeaseIds: ['viewer-a', 'viewer-b'],
  controllerLeaseId: 'viewer-a',
  remoteReadiness: { state: 'ready' },
  readOnly: false,
};

assert.equal(viewStreamLabel(rdpGatewayStream), 'rdp gateway');
assert.equal(controlInputLabel(rdpGatewayStream), 'manual attached desktop');
assert.equal(viewStreamCapabilityLabel(rdpGatewayStream), 'rdp gateway / manual attached desktop');
assert.equal(canEmbedViewStream(rdpGatewayStream), true);
assert.equal(canControlViewStream(rdpGatewayStream), true);
assert.equal(canOpenViewStream(rdpGatewayStream), true);
assert.equal(canOpenControlViewStream(rdpGatewayStream), true);
assert.equal(viewStreamOpenTitle(rdpGatewayStream), 'Open rdp gateway in the dashboard.');
assert.equal(viewStreamControlTitle(rdpGatewayStream), 'Focus the browser and open manual attached desktop control.');
assert.equal(viewStreamRouteLabel(rdpGatewayStream), 'route-a');
assert.equal(viewStreamLeaseLabel(rdpGatewayStream), '2 viewers, controller leased');
assert.equal(viewStreamReadinessLabel(rdpGatewayStream), 'ready');
assert.equal(
  viewStreamRouteSummary(rdpGatewayStream),
  'route-a / display display-a / simultaneous view / 2 viewers, controller leased / ready',
);

assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
  }),
  'connected',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    recoveredStaleTarget: true,
  }),
  'stale_target_recovered',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
  }),
  'takeover_ready',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'taken-over',
  }),
  'taken_over',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
    takeoverPending: true,
  }),
  'reconnecting',
);
assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'process_exited',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
  }),
  'browser_unavailable',
);
assert.equal(workspaceViewportUxStateLabel('stale_target_recovered'), 'stale target recovered');
assert.equal(workspaceViewportReadinessStatusLabel('action_required'), 'action required');

assert.equal(
  deriveWorkspaceViewportUxState({
    hasBrowser: true,
    browserHealth: 'cdp_disconnected',
    hasStream: false,
    canEmbed: false,
    canControl: false,
    mode: 'control',
    preflightStatus: 'idle',
  }),
  'browser_unavailable',
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'cdp_disconnected',
    hasStream: false,
    canEmbed: false,
    canControl: false,
    mode: 'control',
    preflightStatus: 'idle',
  }),
  {
    component: 'browser',
    status: 'blocked',
    evidence: 'browser health is cdp_disconnected',
    nextAction: 'relaunch_browser',
    title: 'Browser unavailable',
    recoveryCopy: 'The selected browser process or CDP endpoint is unhealthy. Relaunch the browser or inspect browser health before opening the remote desktop stream.',
  },
);

assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }),
  {
    component: 'rdp_gateway',
    status: 'ready',
    evidence: 'stream URL is present and preflight is ready',
    nextAction: 'none',
    title: 'Stream ready',
    recoveryCopy: 'The selected browser and remote stream are ready.',
  },
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'login-required',
    preflightMessage: 'The remote stream rejected the current dashboard session.',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }).nextAction,
  'sign_in_again',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    frameIssueKind: 'remote-disconnected',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
  }).nextAction,
  'take_over',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: {
      components: [
        {
          component: 'guacamole_connection',
          status: 'failed',
          evidence: 'connection missing',
          nextAction: 'inspect_readiness',
          recovery: 'Create or grant the Guacamole connection before opening the workspace stream.',
        },
      ],
    },
  }).component,
  'guacamole_connection',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running after a later focus succeeded',
        nextAction: 'inspect_readiness',
      },
    ],
  }).status,
  'ready',
);
assert.equal(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'checking',
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running before stream readiness is proven',
        nextAction: 'inspect_readiness',
      },
    ],
  }).status,
  'action_required',
);
assert.deepEqual(
  deriveWorkspaceViewportReadiness({
    hasBrowser: true,
    browserHealth: 'ready',
    hasStream: true,
    canEmbed: true,
    canControl: true,
    mode: 'control',
    preflightStatus: 'ready',
    recoveredStaleTarget: true,
    streamProvider: 'rdp_gateway',
    streamUrl: 'http://127.0.0.1:8080/guacamole',
    streamReadiness: [
      {
        component: 'focus_job',
        status: 'stale',
        evidence: 'older view_focus job is still running after a later focus succeeded',
        nextAction: 'inspect_readiness',
      },
    ],
  }),
  {
    component: 'selected_target',
    status: 'ready',
    evidence: 'retained target identity was stale and a live tab was selected',
    nextAction: 'none',
    title: 'Recovered stale selected tab identity',
    recoveryCopy: 'The retained target identity was stale, but Agent Browser selected a current live tab before opening the workspace viewport.',
  },
);
assert.deepEqual(
  compactWorkspaceViewportReadinessComponents({
    components: [{ component: 'public_ingress', status: 'failed', evidence: 'timeout' }],
  }),
  [{ component: 'public_ingress', status: 'failed', evidence: 'timeout', nextAction: null, recovery: null, message: null }],
);

assert.equal(
  canEmbedViewStream({
    provider: 'rdp_gateway',
    url: null,
  }),
  false,
);
assert.equal(
  canEmbedViewStream({
    provider: 'cdp_screencast',
    url: 'http://127.0.0.1:8080/cdp/session',
  }),
  true,
);
const cdpScreencastStream = {
  provider: 'cdp_screencast',
  controlInput: 'cdp_input',
  url: 'http://127.0.0.1:44841/',
  frameUrl: 'http://127.0.0.1:44841/',
  readiness: { state: 'ready', reason: 'stream_server_ready' },
  readOnly: false,
};
assert.equal(canEmbedViewStream(cdpScreencastStream), true);
assert.equal(canOpenControlViewStream(cdpScreencastStream), true);
assert.equal(viewStreamOpenTitle(cdpScreencastStream), 'Open cdp screencast in the dashboard.');
assert.equal(viewStreamControlTitle(cdpScreencastStream), 'Focus the browser and open cdp input control.');
assert.equal(
  viewStreamOpenTitle({
    provider: 'cdp_screencast',
    url: null,
    readiness: { state: 'unavailable', reason: 'missing_stream_server' },
    readOnly: true,
  }),
  'cdp screencast is unavailable: missing stream server.',
);
assert.equal(
  canControlViewStream({
    provider: 'rdp_gateway',
    readOnly: true,
    controlInput: 'manual_attached_desktop',
  }),
  false,
);
assert.equal(
  canOpenControlViewStream({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/session',
    readOnly: true,
    controlInput: 'manual_attached_desktop',
  }),
  false,
);
assert.equal(
  viewStreamControlTitle({
    provider: 'rdp_gateway',
    url: 'http://127.0.0.1:8080/rdp/session',
    readOnly: true,
  }),
  'The service marked this stream as view-only or did not report a control input provider.',
);
assert.equal(controlInputLabel({ readOnly: true }), 'view only');
assert.equal(viewStreamLabel({}), 'view stream');

assert.match(
  dashboardPage,
  /import \{ WorkspaceRemoteViewport \} from "@\/components\/workspace-remote-viewport";[\s\S]*<WorkspaceRemoteViewport fallback=\{<Viewport \/>\} selectedWorkspaceContext=\{selectedWorkspace\.context\} \/>/,
  'Dashboard viewport route must render the workspace remote viewport wrapper with selected workspace context before falling back to CDP screencast',
);

assert.match(
  dashboardPage,
  /readWorkspaceViewportRoute[\s\S]*view === "workspace:tile"[\s\S]*DASHBOARD_WORKSPACE_SELECTION_EVENT[\s\S]*!hasSessions && activeSection !== "service" && !hasWorkspaceViewportRoute/,
  'Dashboard empty state must yield to service-owned workspace viewport URLs even when no daemon sessions are active',
);

assert.match(
  workspaceNavigator,
  /function pushWorkspaceViewportUrl\(node: WorkspaceNode, mode: "view" \| "control"\)[\s\S]*url\.pathname = "\/"[\s\S]*url\.searchParams\.set\("view", `workspace:\$\{mode\}`\)[\s\S]*DASHBOARD_WORKSPACE_QUERY_KEYS[\s\S]*new PopStateEvent\("popstate"/,
  'Workspace navigator View and Control actions must push a stable workspace viewport URL and notify route listeners',
);

assert.match(
  workspaceNavigator,
  /function pushWorkspaceTileUrl[\s\S]*url\.searchParams\.set\("view", "workspace:tile"\)[\s\S]*DASHBOARD_WORKSPACE_QUERY_KEYS[\s\S]*aria-label="Open tiled workspace view"/,
  'Workspace navigator must expose a tiled remote workspace route that does not depend on one selected browser',
);

assert.match(
  workspaceNavigator,
  /action\.id === "control" && node\.viewStream\?\.controllable[\s\S]*pushWorkspaceViewportUrl\(node, "control"\)[\s\S]*action\.id === "view" && node\.viewStream\?\.embeddable[\s\S]*pushWorkspaceViewportUrl\(node, "view"\)/,
  'Workspace navigator primary View and Control actions must open the dashboard-owned workspace viewport',
);

assert.match(
  workspaceViewport,
  /view === "workspace:control"[\s\S]*view === "workspace:view"[\s\S]*browserIdFromSelection[\s\S]*daemonSessionFromSelection[\s\S]*daemonBrowserFromSession[\s\S]*primaryViewStream[\s\S]*chooseWorkspaceViewportBrowser[\s\S]*isBlankWorkspaceViewportTab[\s\S]*workspaceViewportTabScore[\s\S]*daemonSessionNameForBrowser[\s\S]*const params = targetId[\s\S]*sessionName[\s\S]*action: "view_focus"[\s\S]*taskName: "workspace-viewport-control"[\s\S]*params,/,
  'Workspace remote viewport must restore URL selection, synthesize daemon streams, choose a recoverable browser, choose a live non-blank service-owned target, and queue view_focus before control embedding',
);

assert.match(
  workspaceViewport,
  /function chooseWorkspaceViewportBrowser[\s\S]*hasOpenWorkspaceViewportStream\(serviceBrowser\)[\s\S]*return serviceBrowser[\s\S]*hasOpenWorkspaceViewportStream\(daemonBrowser\)[\s\S]*return daemonBrowser[\s\S]*return serviceBrowser \?\? daemonBrowser/,
  'Workspace remote viewport must prefer an openable daemon stream when a selected service browser is stale or has no embeddable stream',
);

assert.match(
  workspaceViewport,
  /view === "workspace:tile"[\s\S]*workspaceViewportTiles[\s\S]*tileStreams = viewportSelection\?\.mode === "tile"/,
  'Workspace remote viewport must derive tile mode from the URL and service-owned route state',
);
assert.match(
  workspaceViewport,
  /workspace-remote-viewport-tile-grid[\s\S]*tileStreams\.map\(\(tile\)[\s\S]*workspace-remote-viewport-tile-card[\s\S]*tile\.sharedRoute[\s\S]*shared route[\s\S]*<iframe/,
  'Workspace remote viewport must render a tiled view with two service-owned remote routes and visible shared-route warnings',
);

assert.match(
  workspaceViewport,
  /recoveredFromStaleSelection[\s\S]*deriveWorkspaceViewportUxState[\s\S]*Recovered stale selected tab identity/,
  'Workspace remote viewport must expose the Slice A UX state vocabulary and recover stale retained tab identity as a state, not as browser failure',
);
assert.match(
  workspaceViewport,
  /deriveWorkspaceViewportReadiness[\s\S]*streamReadiness: stream\?\.readiness \?\? stream\?\.remoteReadiness[\s\S]*data-readiness-status=\{viewportReadiness\.status\}[\s\S]*viewStreamRouteSummary\(stream\)[\s\S]*viewportReadiness\.recoveryCopy/,
  'Workspace remote viewport must derive compact readiness and render actionable recovery copy for auth, provider, browser, viewer, and retained-job states',
);
assert.match(
  workspaceViewport,
  /data-ux-state=\{viewportUxState\}/,
  'Workspace remote viewport must expose the derived UX state on the viewport shell',
);
assert.match(
  workspaceViewport,
  /workspaceViewportUxStateLabel\(viewportUxState\)/,
  'Workspace remote viewport must render the service-derived UX state vocabulary',
);

assert.match(
  workspaceViewport,
  /function resolveWorkspaceStreamUrl[\s\S]*viewStreamExternalUrl\(stream\)[\s\S]*viewStreamFrameUrl\(stream\)[\s\S]*new URL\(streamUrl, window\.location\.href\)\.toString\(\)[\s\S]*resolved\.origin === window\.location\.origin[\s\S]*setStreamPreflight\(\{ status: "ready", message: "" \}\)/,
  'Workspace remote viewport must resolve service-owned frame and external stream URLs and allow cross-origin iframe rendering instead of treating CORS preflight failure as stream unavailability',
);
assert.match(
  workspaceViewport,
  /function detectWorkspaceFrameFailure[\s\S]*catch \{\s*return null;\s*\}[\s\S]*return null;/,
  'Workspace remote viewport must not classify cross-origin Guacamole frame inspection limits as browser-error failures',
);

assert.match(
  workspaceViewport,
  /WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH[\s\S]*process_exited[\s\S]*function browserCanRenderWorkspaceViewport[\s\S]*!WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH\.has\(health\)[\s\S]*const canRenderSelectedBrowser = browserCanRenderWorkspaceViewport\(browser\)[\s\S]*const canRenderCdpStream = canRenderSelectedBrowser[\s\S]*const canRenderFrame = canRenderSelectedBrowser/,
  'Workspace remote viewport must not embed retained Guacamole routes for browsers whose process or CDP endpoint is terminal',
);

assert.match(
  workspaceViewport,
  /function workspaceViewportTiles[\s\S]*if \(!browserCanRenderWorkspaceViewport\(browser\)\) return null[\s\S]*canOpenViewStream\(stream\)/,
  'Workspace tile mode must exclude retained terminal browser records even when they still have Guacamole URLs',
);

assert.doesNotMatch(
  workspaceViewport,
  /params: \{ index: tabIndex, maximize: true \}/,
  'Workspace remote viewport must not rely only on retained tab indexes when target IDs are available',
);

assert.match(
  workspaceViewport,
  /installGuacamoleTouchClickBridge[\s\S]*sendMouse\(touch, true\)[\s\S]*sendMouse\(touch, false\)[\s\S]*<iframe[\s\S]*ref=\{viewportFrameRef\}[\s\S]*className="workspace-remote-viewport-frame"[\s\S]*allow="clipboard-read; clipboard-write; fullscreen; pointer-lock"/,
  'Workspace remote viewport must embed service-owned streams behind dashboard chrome with input capabilities enabled',
);

assert.match(
  workspaceViewport,
  /function isCdpScreencastStream[\s\S]*provider\?\.trim\(\)\.toLowerCase\(\) === "cdp_screencast"[\s\S]*function workspaceCdpWebSocketUrl[\s\S]*\/api\/stream\/\$\{encodeURIComponent\(resolved\.port\)\}[\s\S]*resolved\.protocol = resolved\.protocol === "https:" \? "wss:" : "ws:"[\s\S]*function WorkspaceCdpStreamCanvas[\s\S]*new WebSocket\(websocketUrl\)[\s\S]*case "frame":[\s\S]*drawFrame\(msg\.data\)[\s\S]*type: "input_mouse"[\s\S]*type: "input_keyboard"/,
  'Workspace remote viewport must render CDP screencast streams through a native WebSocket canvas instead of iframing the stream server HTTP root',
);

assert.match(
  workspaceViewport,
  /const canRenderCdpStream = canRenderSelectedBrowser && isCdpScreencastStream\(stream\)[\s\S]*const canRenderFrame = canRenderSelectedBrowser && !isCdpScreencastStream\(stream\)[\s\S]*<WorkspaceCdpStreamCanvas[\s\S]*: stream && canRenderFrame \? \(/,
  'Workspace remote viewport must route CDP screencasts to the native canvas before considering the iframe path',
);

assert.match(
  css,
  /\.workspace-cdp-stream[\s\S]*\.workspace-cdp-stream-canvas[\s\S]*\.workspace-cdp-stream-footer/,
  'Workspace remote viewport must style the native CDP canvas so stream readiness is visible without embedding the dashboard login shell',
);

assert.match(
  workspaceViewport,
  /function openGuacamoleInteractionSettings[\s\S]*#guac-menu[\s\S]*scope\.menu!\.shown = true[\s\S]*#keyboard-settings[\s\S]*#mouse-settings[\s\S]*aria-label="Open Guacamole interaction settings"/,
  'Workspace remote viewport must expose a control that opens Guacamole keyboard and mouse interaction settings',
);

assert.match(
  workspaceViewport,
  /remote-disconnected[\s\S]*you have been disconnected[\s\S]*Another dashboard or Guacamole popout is using this remote desktop[\s\S]*Take over/,
  'Workspace remote viewport must identify Guacamole single-viewer disconnects and expose a takeover action',
);

assert.match(
  workspaceViewport,
  /requestWorkspaceTakeover[\s\S]*browserId: browser\.id[\s\S]*streamId: stream\.id[\s\S]*openMode[\s\S]*action: "view_takeover"[\s\S]*taskName: "workspace-viewport-takeover"[\s\S]*setStreamRefreshNonce\(Date\.now\(\)\)/,
  'Workspace remote viewport Take over must queue a service-owned view_takeover request and reconnect the iframe',
);

assert.match(
  workspaceViewport,
  /postWorkspaceRecoveryRequest[\s\S]*action: ServiceRequestAction[\s\S]*service_remote_view_route_checkout[\s\S]*workspace-viewport-route-refresh[\s\S]*service_viewer_lease_request[\s\S]*workspace-viewport-viewer-reconnect[\s\S]*service_controller_lease_takeover[\s\S]*workspace-viewport-controller-takeover[\s\S]*service_viewer_lease_release[\s\S]*workspace-viewport-viewer-release/,
  'Workspace remote viewport must expose explicit route refresh, viewer reconnect, controller takeover, and viewer release recovery actions',
);

assert.match(
  workspaceViewport,
  /aria-label="Refresh remote route lease"[\s\S]*aria-label="Reconnect viewer lease"[\s\S]*aria-label="Take controller lease"[\s\S]*aria-label="Release viewer leases"/,
  'Workspace remote viewport recovery actions must be visible as stable icon-button controls',
);

assert.match(
  workspaceViewport,
  /openWorkspaceStreamExternally[\s\S]*const accepted = await requestWorkspaceTakeover\("external"\)[\s\S]*if \(!accepted\) return[\s\S]*window\.open\(externalStreamUrl, "_blank", "noopener,noreferrer"\)/,
  'Workspace remote viewport external open must await service-owned takeover acceptance before opening the external route',
);

assert.doesNotMatch(
  workspaceViewport,
  /onClick=\{refreshWorkspaceViewport\}[\s\S]*Take over/,
  'Workspace remote viewport Take over must not be a local iframe-only refresh',
);

assert.doesNotMatch(
  workspaceViewport,
  /sandbox=/,
  'Workspace remote viewport must not sandbox first-party Guacamole streams because that breaks operator input capture',
);

assert.match(
  rdpAutologinSetup,
  /INSERT INTO guacamole_connection_permission[\s\S]*SELECT entity_id, \{connection_id\}, 'READ'::guacamole_object_permission_type[\s\S]*WHERE type = 'USER'[\s\S]*ON CONFLICT DO NOTHING/,
  'XRDP autologin setup must grant current Guacamole users READ on the configured remote desktop connection',
);

assert.match(
  css,
  /\.workspace-remote-viewport[\s\S]*grid-template-rows: auto auto minmax\(0, 1fr\)[\s\S]*\.workspace-remote-viewport-stage[\s\S]*min-height: 0[\s\S]*touch-action: none[\s\S]*\.workspace-remote-viewport-frame[\s\S]*height: 100%[\s\S]*touch-action: none[\s\S]*\.workspace-remote-viewport-tile-grid[\s\S]*grid-template-columns: repeat\(2, minmax\(0, 1fr\)\)[\s\S]*\.workspace-remote-viewport-tile-stage/,
  'Workspace remote viewport CSS must keep compact chrome and a stable iframe stage',
);

assert.match(
  css,
  /\.service-view-stream-route-strip[\s\S]*grid-template-columns: repeat\(auto-fit, minmax\(7rem, 1fr\)\)/,
  'Service stream cards must render route metadata in stable responsive columns',
);

console.log('Dashboard view stream contract smoke passed');
