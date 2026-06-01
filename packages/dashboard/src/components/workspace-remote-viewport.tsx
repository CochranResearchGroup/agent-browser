"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent as ReactMouseEvent, type ReactNode, type WheelEvent as ReactWheelEvent } from "react";
import { createPortal } from "react-dom";
import { useAtomValue } from "jotai/react";
import { AlertTriangle, ExternalLink, LogIn, Maximize2, Minimize2, MousePointer2, PlugZap, RefreshCw, Settings2, SquareArrowOutUpRight, Unplug } from "lucide-react";
import {
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  controlInputLabel,
  viewStreamCapabilityLabel,
  viewStreamExternalUrl,
  viewStreamFrameUrl,
  viewStreamLabel,
  viewStreamOpenTitle,
  viewStreamReadinessLabel,
  viewStreamRouteSummary,
  type ServiceViewStream,
} from "@/lib/service-view-streams";
import {
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  dashboardWorkspaceSelectionHasValue,
  readDashboardWorkspaceUrlSelection,
  type DashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import type { SelectedWorkspaceContext } from "@/lib/selected-workspace-context";
import { activePortAtom, activeSessionNameAtom, sessionsAtom } from "@/store/sessions";
import type { SessionInfo } from "@/types";
import { cn } from "@/lib/utils";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import {
  deriveWorkspaceViewportReadiness,
  deriveWorkspaceViewportUxState,
  workspaceViewportReadinessStatusLabel,
  workspaceViewportUxStateLabel,
} from "@/lib/workspace-viewport-state";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { StreamMessage } from "@/types";

type WorkspaceViewportMode = "view" | "control" | "tile";

type WorkspaceViewportBrowser = {
  id: string;
  displayName?: string | null;
  profileId?: string | null;
  host?: string | null;
  health?: string | null;
  browserBuild?: string | null;
  displayAllocationId?: string | null;
  viewStreams?: ServiceViewStream[];
  activeSessionIds?: string[];
};

type WorkspaceViewportTab = {
  id: string;
  browserId?: string | null;
  targetId?: string | null;
  title?: string | null;
  url?: string | null;
  lifecycle?: string | null;
};

type ServiceStatusData = {
  service_state?: {
    browsers?: Record<string, WorkspaceViewportBrowser>;
    tabs?: Record<string, WorkspaceViewportTab>;
  };
};

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: string | null;
};

type ServiceRequestAction =
  | "service_remote_view_route_checkout"
  | "service_viewer_lease_request"
  | "service_viewer_lease_release"
  | "service_controller_lease_takeover";

type WorkspaceViewportSelection = {
  mode: WorkspaceViewportMode;
  selection: DashboardWorkspaceUrlSelection;
};

type WorkspaceViewportTile = {
  browser: WorkspaceViewportBrowser;
  stream: ServiceViewStream;
  frameUrl: string;
  externalUrl: string | null;
  routeKey: string;
  sharedRoute: boolean;
};

type StreamPreflightState = {
  status: "idle" | "checking" | "ready" | "login-required" | "error";
  message: string;
};

type WorkspaceFrameFailure = "login-required" | "fatal-error" | "browser-error" | "remote-disconnected" | "taken-over";

type WorkspaceFrameIssue = {
  kind: "remote-disconnected" | "taken-over";
  message: string;
} | null;

type CdpStreamState = {
  connected: boolean;
  browserConnected: boolean;
  screencasting: boolean;
  viewportWidth: number;
  viewportHeight: number;
  frameReceived: boolean;
  httpFallback: boolean;
  message: string;
};

type GuacamoleMouseState = {
  x?: number;
  y?: number;
  left?: boolean;
  middle?: boolean;
  right?: boolean;
  up?: boolean;
  down?: boolean;
};

type GuacamoleClient = {
  sendMouseState?: (state: GuacamoleMouseState, flush?: boolean) => void;
};

type GuacamoleAngularScope = {
  client?: {
    client?: GuacamoleClient;
  };
};

type GuacamoleMenuScope = GuacamoleAngularScope & {
  menu?: {
    shown?: boolean;
  };
  $apply?: (fn: () => void) => void;
  $evalAsync?: (fn: () => void) => void;
  $parent?: GuacamoleMenuScope;
};

type GuacamoleFrameWindow = Window & typeof globalThis & {
  Guacamole?: {
    Mouse?: {
      State?: new (template?: GuacamoleMouseState) => GuacamoleMouseState;
    };
    Position?: {
      fromClientPosition?: (element: Element, clientX: number, clientY: number) => { x: number; y: number };
    };
  };
  angular?: {
    element?: (element: Element) => {
      scope?: () => GuacamoleMenuScope | undefined;
      isolateScope?: () => GuacamoleMenuScope | undefined;
    };
  };
  __agentBrowserTouchClickBridgeCleanup?: () => void;
};

const GUACAMOLE_TOUCH_BRIDGE_STYLE = "agent-browser-touch-click-bridge";
const GUACAMOLE_TOUCH_BRIDGE_TAP_MS = 700;
const WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH = new Set([
  "cdp_disconnected",
  "closed",
  "disconnected",
  "faulted",
  "not_started",
  "process_exited",
  "unreachable",
]);

const SCREENCAST_ENGINES = new Set(["chrome"]);

const KEY_INFO: Record<string, { text?: string; keyCode: number }> = {
  Enter: { text: "\r", keyCode: 13 },
  Tab: { text: "\t", keyCode: 9 },
  Backspace: { text: "\b", keyCode: 8 },
  Escape: { keyCode: 27 },
  ArrowLeft: { keyCode: 37 },
  ArrowUp: { keyCode: 38 },
  ArrowRight: { keyCode: 39 },
  ArrowDown: { keyCode: 40 },
  Delete: { keyCode: 46 },
  Home: { keyCode: 36 },
  End: { keyCode: 35 },
  PageUp: { keyCode: 33 },
  PageDown: { keyCode: 34 },
};

function serviceBase(_activePort: number): string {
  return SERVICE_API_BASE;
}

function cdpModifiers(e: ReactMouseEvent | ReactWheelEvent | KeyboardEvent): number {
  let m = 0;
  if (e.altKey) m |= 1;
  if (e.ctrlKey) m |= 2;
  if (e.metaKey) m |= 4;
  if (e.shiftKey) m |= 8;
  return m;
}

function cdpButton(btn: number): string {
  switch (btn) {
    case 0: return "left";
    case 1: return "middle";
    case 2: return "right";
    default: return "none";
  }
}

function isCdpScreencastStream(stream?: ServiceViewStream | null): boolean {
  return stream?.provider?.trim().toLowerCase() === "cdp_screencast";
}

function workspaceCdpWebSocketUrl(streamUrl: string | null): string | null {
  if (!streamUrl || typeof window === "undefined") return null;
  try {
    const resolved = new URL(streamUrl, window.location.href);
    if (window.location.protocol === "https:" && resolved.port) {
      const proxied = new URL(`/api/stream/${encodeURIComponent(resolved.port)}`, window.location.href);
      proxied.protocol = "wss:";
      return proxied.toString();
    }
    resolved.protocol = resolved.protocol === "https:" ? "wss:" : "ws:";
    resolved.pathname = "/";
    resolved.search = "";
    resolved.hash = "";
    return resolved.toString();
  } catch {
    return null;
  }
}

function workspaceCdpStreamPort(streamUrl: string | null): string | null {
  if (!streamUrl || typeof window === "undefined") return null;
  try {
    return new URL(streamUrl, window.location.href).port || null;
  } catch {
    return null;
  }
}

function touchByIdentifier(touches: TouchList, identifier: number): Touch | null {
  for (let index = 0; index < touches.length; index += 1) {
    const touch = touches.item(index);
    if (touch?.identifier === identifier) return touch;
  }
  return null;
}

function installGuacamoleTouchClickBridge(frame: HTMLIFrameElement | null): (() => void) | null {
  if (!frame) return null;

  let win: GuacamoleFrameWindow | null = null;
  let doc: Document | null = null;
  try {
    win = frame.contentWindow as GuacamoleFrameWindow | null;
    doc = frame.contentDocument;
  } catch {
    return null;
  }
  if (!win || !doc || win.__agentBrowserTouchClickBridgeCleanup) return null;

  const display = doc.querySelector<HTMLElement>(".display");
  if (!display || !win.Guacamole?.Mouse?.State) return null;

  const findClient = (): GuacamoleClient | null => {
    const angularElement = win?.angular?.element;
    if (!angularElement) return null;
    const candidates = [
      display,
      doc.querySelector<HTMLElement>(".client-tile"),
      doc.querySelector<HTMLElement>("guac-client"),
    ].filter((element): element is HTMLElement => Boolean(element));

    for (const element of candidates) {
      const client = angularElement(element).scope?.()?.client?.client;
      if (client?.sendMouseState) return client;
    }
    return null;
  };

  const positionForTouch = (touch: Touch): { x: number; y: number } => {
    const position = win?.Guacamole?.Position?.fromClientPosition?.(display, touch.clientX, touch.clientY);
    if (position) return position;
    const rect = display.getBoundingClientRect();
    return {
      x: Math.max(0, touch.clientX - rect.left),
      y: Math.max(0, touch.clientY - rect.top),
    };
  };

  const sendMouse = (touch: Touch, left: boolean): boolean => {
    const client = findClient();
    const State = win?.Guacamole?.Mouse?.State;
    if (!client?.sendMouseState || !State) return false;
    const position = positionForTouch(touch);
    const state = new State({
      x: position.x,
      y: position.y,
      left,
      middle: false,
      right: false,
      up: false,
      down: false,
    });
    client.sendMouseState(state, true);
    return true;
  };

  const stopTouch = (event: TouchEvent): void => {
    if (event.cancelable) event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
  };

  const style = doc.createElement("style");
  style.dataset.agentBrowser = GUACAMOLE_TOUCH_BRIDGE_STYLE;
  style.textContent = `
html,
body,
.client,
.client-view,
.client-tile,
.display {
  touch-action: none !important;
  overscroll-behavior: none !important;
  -webkit-user-select: none !important;
  user-select: none !important;
}
`;
  doc.head?.appendChild(style);

  let activeTouchIdentifier: number | null = null;
  let startX = 0;
  let startY = 0;
  let lastX = 0;
  let lastY = 0;
  let startedAt = 0;
  let moved = false;
  let releaseTimer: number | null = null;

  const clearReleaseTimer = () => {
    if (releaseTimer === null) return;
    win?.clearTimeout(releaseTimer);
    releaseTimer = null;
  };

  const resetTouch = () => {
    activeTouchIdentifier = null;
    moved = false;
    clearReleaseTimer();
  };

  const movementThreshold = () => Math.max(18, 18 * (win?.devicePixelRatio || 1));

  const onTouchStart = (event: TouchEvent) => {
    if (event.touches.length !== 1) {
      resetTouch();
      return;
    }

    const touch = event.touches.item(0);
    if (!touch) return;
    stopTouch(event);
    clearReleaseTimer();
    activeTouchIdentifier = touch.identifier;
    startX = touch.clientX;
    startY = touch.clientY;
    lastX = touch.clientX;
    lastY = touch.clientY;
    startedAt = Date.now();
    moved = false;
    sendMouse(touch, false);
  };

  const onTouchMove = (event: TouchEvent) => {
    if (activeTouchIdentifier === null) return;
    const touch = touchByIdentifier(event.touches, activeTouchIdentifier)
      ?? touchByIdentifier(event.changedTouches, activeTouchIdentifier);
    if (!touch) return;
    stopTouch(event);
    lastX = touch.clientX;
    lastY = touch.clientY;
    const dx = lastX - startX;
    const dy = lastY - startY;
    if (Math.hypot(dx, dy) > movementThreshold()) moved = true;
    sendMouse(touch, false);
  };

  const onTouchEnd = (event: TouchEvent) => {
    if (activeTouchIdentifier === null) return;
    const touch = touchByIdentifier(event.changedTouches, activeTouchIdentifier);
    if (!touch) return;
    stopTouch(event);
    lastX = touch.clientX;
    lastY = touch.clientY;
    const elapsed = Date.now() - startedAt;
    const dx = lastX - startX;
    const dy = lastY - startY;
    const isTap = !moved && Math.hypot(dx, dy) <= movementThreshold() && elapsed <= GUACAMOLE_TOUCH_BRIDGE_TAP_MS;
    sendMouse(touch, false);
    if (isTap && sendMouse(touch, true)) {
      releaseTimer = win?.setTimeout(() => {
        sendMouse(touch, false);
        releaseTimer = null;
      }, 45) ?? null;
    }
    activeTouchIdentifier = null;
    moved = false;
  };

  const onTouchCancel = (event: TouchEvent) => {
    if (activeTouchIdentifier !== null) stopTouch(event);
    resetTouch();
  };

  display.addEventListener("touchstart", onTouchStart, { capture: true, passive: false });
  display.addEventListener("touchmove", onTouchMove, { capture: true, passive: false });
  display.addEventListener("touchend", onTouchEnd, { capture: true, passive: false });
  display.addEventListener("touchcancel", onTouchCancel, { capture: true, passive: false });

  const cleanup = () => {
    display.removeEventListener("touchstart", onTouchStart, true);
    display.removeEventListener("touchmove", onTouchMove, true);
    display.removeEventListener("touchend", onTouchEnd, true);
    display.removeEventListener("touchcancel", onTouchCancel, true);
    style.remove();
    resetTouch();
    if (win?.__agentBrowserTouchClickBridgeCleanup === cleanup) {
      delete win.__agentBrowserTouchClickBridgeCleanup;
    }
  };
  win.__agentBrowserTouchClickBridgeCleanup = cleanup;
  return cleanup;
}

function guacamoleScopeWithMenu(scope?: GuacamoleMenuScope): GuacamoleMenuScope | null {
  let current = scope;
  while (current) {
    if (current.menu) return current;
    current = current.$parent;
  }
  return null;
}

function openGuacamoleInteractionSettings(frame: HTMLIFrameElement | null): boolean {
  if (!frame) return false;

  let win: GuacamoleFrameWindow | null = null;
  let doc: Document | null = null;
  try {
    win = frame.contentWindow as GuacamoleFrameWindow | null;
    doc = frame.contentDocument;
  } catch {
    return false;
  }
  if (!win || !doc) return false;

  const menu = doc.querySelector<HTMLElement>("#guac-menu");
  const scope = guacamoleScopeWithMenu(win.angular?.element?.(menu ?? doc.body)?.scope?.());
  if (!scope?.menu) return false;

  const openMenu = () => {
    scope.menu!.shown = true;
  };

  if (scope.$apply) {
    try {
      scope.$apply(openMenu);
    } catch {
      openMenu();
      scope.$evalAsync?.(() => undefined);
    }
  } else {
    openMenu();
    scope.$evalAsync?.(() => undefined);
  }

  win.setTimeout(() => {
    const keyboardSettings = doc.querySelector<HTMLElement>("#keyboard-settings");
    const mouseSettings = doc.querySelector<HTMLElement>("#mouse-settings");
    keyboardSettings?.scrollIntoView({ block: "start", inline: "nearest" });
    (keyboardSettings ?? mouseSettings ?? menu)?.focus?.();
  }, 80);

  return true;
}

function readWorkspaceViewportSelection(): WorkspaceViewportSelection | null {
  if (typeof window === "undefined") return null;
  const params = new URLSearchParams(window.location.search);
  const view = params.get("view");
  const mode = view === "workspace:control"
    ? "control"
    : view === "workspace:view"
      ? "view"
      : view === "workspace:tile"
        ? "tile"
        : null;
  if (!mode) return null;
  const selection = readDashboardWorkspaceUrlSelection();
  return mode === "tile" || dashboardWorkspaceSelectionHasValue(selection) ? { mode, selection } : null;
}

function browserIdFromSelection(selection: DashboardWorkspaceUrlSelection): string | null {
  if (selection.browserId) return selection.browserId;
  if (selection.workspaceId?.startsWith("browser:")) return selection.workspaceId.slice("browser:".length);
  return null;
}

function daemonSessionFromSelection(
  sessions: SessionInfo[],
  selection?: DashboardWorkspaceUrlSelection | null,
): SessionInfo | null {
  const selectedSession = stripSessionBrowserPrefix(selection?.sessionId);
  const workspaceSession = selection?.workspaceId?.startsWith("daemon-session:")
    ? selection.workspaceId.slice("daemon-session:".length)
    : null;
  const selected = selectedSession || workspaceSession;
  if (!selected) return null;
  return sessions.find((session) => session.session === selected) ?? null;
}

function daemonBrowserFromSession(session: SessionInfo | null): WorkspaceViewportBrowser | null {
  if (!session || session.pending || session.closing || session.port <= 0) return null;
  const streamUrl = `http://127.0.0.1:${session.port}/`;
  return {
    id: `daemon:${session.session}`,
    displayName: session.session,
    host: "daemon-session",
    health: "ready",
    browserBuild: session.provider ?? session.engine ?? null,
    viewStreams: [
      {
        id: `daemon-stream:${session.session}`,
        provider: "cdp_screencast",
        controlInput: "cdp_input",
        url: streamUrl,
        frameUrl: streamUrl,
        externalUrl: streamUrl,
        routeId: `daemon:${session.session}`,
        connectionName: session.session,
        routeSource: "daemon-session",
        providerMode: "single_controller",
        readOnly: false,
        readiness: { state: "ready", reason: `daemon stream ${session.port}` },
      },
    ],
    activeSessionIds: [session.session],
  };
}

function primaryViewStream(browser?: WorkspaceViewportBrowser | null): ServiceViewStream | null {
  const streams = browser?.viewStreams ?? [];
  if (streams.length === 0) return null;
  return [...streams].sort((left, right) => workspaceViewportStreamScore(right) - workspaceViewportStreamScore(left))[0] ?? null;
}

function hasOpenWorkspaceViewportStream(browser?: WorkspaceViewportBrowser | null): boolean {
  return canOpenViewStream(primaryViewStream(browser));
}

function chooseWorkspaceViewportBrowser(
  serviceBrowser: WorkspaceViewportBrowser | null,
  daemonBrowser: WorkspaceViewportBrowser | null,
): WorkspaceViewportBrowser | null {
  if (hasOpenWorkspaceViewportStream(serviceBrowser)) return serviceBrowser;
  if (hasOpenWorkspaceViewportStream(daemonBrowser)) return daemonBrowser;
  return serviceBrowser ?? daemonBrowser;
}

function browserCanRenderWorkspaceViewport(browser?: WorkspaceViewportBrowser | null): boolean {
  const health = browser?.health?.trim().toLowerCase() ?? "";
  return Boolean(browser) && !WORKSPACE_VIEWPORT_TERMINAL_BROWSER_HEALTH.has(health);
}

function workspaceViewportStreamScore(stream: ServiceViewStream): number {
  const provider = stream.provider?.trim().toLowerCase() ?? "";
  const routeSource = stream.routeSource?.trim().toLowerCase() ?? "";
  const providerMode = stream.providerMode?.trim().toLowerCase() ?? "";
  const displayAllocationId = stream.displayAllocationId?.trim().toLowerCase() ?? "";
  let score = 0;
  if (canOpenViewStream(stream)) score += 80;
  if (provider === "rdp_gateway") score += 20;
  if (canOpenControlViewStream(stream)) score += 15;
  if (stream.routeId || stream.connectionId || stream.connectionName) score += 20;
  if (displayAllocationId) score += 10;
  if (displayAllocationId && !displayAllocationId.includes("shared")) score += 35;
  if (routeSource === "pool" || routeSource === "generated" || routeSource === "discovered") score += 40;
  if (providerMode === "simultaneous_view") score += 20;
  if (providerMode === "single_controller") score += 10;
  if (viewStreamReadinessLabel(stream) === "ready") score += 10;
  return score;
}

function workspaceViewportRouteKey(stream: ServiceViewStream): string {
  return stream.routeId || stream.connectionId || stream.frameUrl || stream.externalUrl || stream.url || "unrouted";
}

function workspaceViewportTiles(serviceStatus: ServiceStatusData | null): WorkspaceViewportTile[] {
  const browsers = Object.values(serviceStatus?.service_state?.browsers ?? {});
  const candidates = browsers
    .map((browser) => {
      if (!browserCanRenderWorkspaceViewport(browser)) return null;
      const stream = primaryViewStream(browser);
      const frameUrl = resolveWorkspaceStreamUrl(stream);
      if (!stream || !frameUrl || !canOpenViewStream(stream)) return null;
      return {
        browser,
        stream,
        frameUrl,
        externalUrl: resolveWorkspaceStreamUrl(stream, "external"),
        routeKey: workspaceViewportRouteKey(stream),
        sharedRoute: false,
      };
    })
    .filter((tile): tile is WorkspaceViewportTile => Boolean(tile))
    .sort((left, right) => {
      const score = workspaceViewportStreamScore(right.stream) - workspaceViewportStreamScore(left.stream);
      if (score !== 0) return score;
      return left.browser.id.localeCompare(right.browser.id);
    });

  const routeCounts = new Map<string, number>();
  for (const candidate of candidates) {
    routeCounts.set(candidate.routeKey, (routeCounts.get(candidate.routeKey) ?? 0) + 1);
  }
  return candidates.slice(0, 2).map((candidate) => ({
    ...candidate,
    sharedRoute: (routeCounts.get(candidate.routeKey) ?? 0) > 1,
  }));
}

function browserTabs(tabs: WorkspaceViewportTab[], browserId: string): WorkspaceViewportTab[] {
  return tabs.filter((tab) => tab.browserId === browserId);
}

function isLiveWorkspaceViewportTab(tab: WorkspaceViewportTab): boolean {
  const lifecycle = (tab.lifecycle ?? "").toLowerCase();
  return lifecycle === "active" || lifecycle === "ready" || lifecycle === "loading";
}

function isBlankWorkspaceViewportTab(tab: WorkspaceViewportTab): boolean {
  const url = (tab.url ?? "").trim().toLowerCase();
  const title = (tab.title ?? "").trim().toLowerCase();
  const blankUrl = !url || url === "about:blank" || url === "chrome://newtab/";
  const blankTitle = !title || title === "about:blank" || title === "new tab";
  return blankUrl && blankTitle;
}

function workspaceViewportTabScore(tab: WorkspaceViewportTab): number {
  if (!isLiveWorkspaceViewportTab(tab)) return -1000;
  const lifecycle = (tab.lifecycle ?? "").toLowerCase();
  let score = lifecycle === "active" ? 400 : lifecycle === "loading" ? 320 : 300;
  if (!isBlankWorkspaceViewportTab(tab)) score += 200;
  if (tab.targetId) score += 25;
  return score;
}

function selectedTabForBrowser(
  tabs: WorkspaceViewportTab[],
  browserId: string,
  selection: DashboardWorkspaceUrlSelection,
): { tab: WorkspaceViewportTab | null; tabIndex: number | null; recoveredFromStaleSelection: boolean } {
  const rows = browserTabs(tabs, browserId);
  if (rows.length === 0) return { tab: null, tabIndex: null, recoveredFromStaleSelection: false };
  const selected = selection.tabId ? rows.find((tab) => tab.id === selection.tabId) : undefined;
  const focusableRows = rows.filter(isLiveWorkspaceViewportTab);
  const selectedFocusable = selected && isLiveWorkspaceViewportTab(selected) && !isBlankWorkspaceViewportTab(selected) ? selected : undefined;
  const tab = selectedFocusable
    ?? [...focusableRows].sort((left, right) => workspaceViewportTabScore(right) - workspaceViewportTabScore(left))[0]
    ?? rows[0];
  const indexRows = focusableRows.length > 0 ? focusableRows : rows;
  const tabIndex = indexRows.findIndex((item) => item.id === tab.id);
  const selectedWasStale = Boolean(selected && (!isLiveWorkspaceViewportTab(selected) || isBlankWorkspaceViewportTab(selected)));
  return {
    tab,
    tabIndex: tabIndex >= 0 ? tabIndex : null,
    recoveredFromStaleSelection: Boolean(selectedWasStale && tab.id !== selected?.id),
  };
}

function stripSessionBrowserPrefix(value?: string | null): string | null {
  const trimmed = value?.trim();
  if (!trimmed) return null;
  return trimmed.startsWith("session:") ? trimmed.slice("session:".length) : trimmed;
}

function daemonSessionNameForBrowser(
  browser: WorkspaceViewportBrowser,
  selection?: DashboardWorkspaceUrlSelection | null,
): string | null {
  const selectedSession = stripSessionBrowserPrefix(selection?.sessionId);
  if (selectedSession) return selectedSession;
  const activeSession = browser.activeSessionIds?.find((sessionId) => sessionId.trim());
  if (activeSession) return activeSession;
  return stripSessionBrowserPrefix(browser.id);
}

function workspaceViewportTitle(browser: WorkspaceViewportBrowser, tab?: WorkspaceViewportTab | null): string {
  return tab?.title || browser.displayName || browser.id;
}

function workspaceViewportSubtitle(browser: WorkspaceViewportBrowser, tab?: WorkspaceViewportTab | null): string {
  return [
    browser.host,
    browser.browserBuild,
    browser.profileId,
    tab?.url,
  ].filter(Boolean).join(" / ");
}

function dashboardLoginPath(): string {
  if (typeof window === "undefined") return "/login";
  return `/login?next=${encodeURIComponent(`${window.location.pathname}${window.location.search}`)}`;
}

function responseLooksLikeDashboardLogin(response: Response, fallbackUrl: URL): boolean {
  const responseUrl = response.url ? new URL(response.url, fallbackUrl) : fallbackUrl;
  const xFrameOptions = response.headers.get("x-frame-options")?.trim().toLowerCase() ?? "";
  return responseUrl.pathname === "/login" || xFrameOptions === "deny";
}

function resolveWorkspaceStreamUrl(stream?: ServiceViewStream | null, mode: "frame" | "external" = "frame"): string | null {
  const streamUrl = mode === "external" ? viewStreamExternalUrl(stream) : viewStreamFrameUrl(stream);
  if (!streamUrl) return null;
  if (typeof window === "undefined") return streamUrl;
  try {
    return new URL(streamUrl, window.location.href).toString();
  } catch {
    return streamUrl;
  }
}

function buildWorkspaceFrameUrl(streamUrl: string | null, refreshNonce: number): string | null {
  if (!streamUrl || typeof window === "undefined") return streamUrl;
  try {
    const resolved = new URL(streamUrl, window.location.href);
    if (resolved.origin === window.location.origin) {
      resolved.searchParams.set("agentBrowserViewport", "workspace");
      resolved.searchParams.set("agentBrowserRefresh", String(refreshNonce));
    }
    return resolved.toString();
  } catch {
    return streamUrl;
  }
}

function detectWorkspaceFrameFailure(frame: HTMLIFrameElement | null): WorkspaceFrameFailure | null {
  if (!frame) return null;
  try {
    const href = frame.contentWindow?.location.href ?? "";
    const title = frame.contentDocument?.title ?? "";
    const bodyText = frame.contentDocument?.body?.innerText ?? "";
    const combined = `${href}\n${title}\n${bodyText}`.toLowerCase();
    if (href.includes("/login") || combined.includes("login required")) {
      return "login-required";
    }
    if (
      combined.includes("taken over")
      || combined.includes("another user")
      || combined.includes("another connection")
      || combined.includes("replaced by another")
    ) {
      return "taken-over";
    }
    if (combined.includes("you have been disconnected")) {
      return "remote-disconnected";
    }
    if (combined.includes("fatal error") || combined.includes("connection closed")) {
      return "fatal-error";
    }
    if (href.startsWith("chrome-error://") || title.includes("refused to connect") || title.includes("not available")) {
      return "browser-error";
    }
  } catch {
    return null;
  }
  return null;
}

function WorkspaceCdpStreamCanvas({
  streamUrl,
  canControl,
  refreshNonce,
}: {
  streamUrl: string;
  canControl: boolean;
  refreshNonce: number;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryCountRef = useRef(0);
  const frameSizeRef = useRef({ width: 1280, height: 720 });
  const [state, setState] = useState<CdpStreamState>({
    connected: false,
    browserConnected: false,
    screencasting: false,
    viewportWidth: 1280,
    viewportHeight: 720,
    frameReceived: false,
    httpFallback: false,
    message: "Connecting to CDP stream.",
  });
  const websocketUrl = useMemo(() => workspaceCdpWebSocketUrl(streamUrl), [streamUrl]);
  const streamPort = useMemo(() => workspaceCdpStreamPort(streamUrl), [streamUrl]);

  const sendInput = useCallback((msg: Record<string, unknown>) => {
    const ws = wsRef.current;
    if (!canControl) return;
    if (state.httpFallback && streamPort) {
      void fetch(`/api/stream/${encodeURIComponent(streamPort)}/input`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(msg),
      });
      return;
    }
    if (ws?.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(msg));
  }, [canControl, state.httpFallback, streamPort]);

  const drawFrame = useCallback((base64: string) => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const bin = atob(base64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i += 1) bytes[i] = bin.charCodeAt(i);

    createImageBitmap(new Blob([bytes], { type: "image/jpeg" })).then((bmp) => {
      canvas.width = bmp.width;
      canvas.height = bmp.height;
      frameSizeRef.current = { width: bmp.width, height: bmp.height };
      const ctx = canvas.getContext("2d");
      if (ctx) ctx.drawImage(bmp, 0, 0);
      bmp.close();
    }).catch(() => {
      setState((current) => ({
        ...current,
        message: "The CDP stream sent a frame that could not be decoded.",
      }));
    });
  }, []);

  useEffect(() => {
    if (!websocketUrl) {
      setState((current) => ({
        ...current,
        connected: false,
        browserConnected: false,
        screencasting: false,
        message: "The CDP stream did not include a usable WebSocket port.",
      }));
      return;
    }

    let disposed = false;

    const connect = () => {
      if (disposed || wsRef.current?.readyState === WebSocket.OPEN) return;
      const ws = new WebSocket(websocketUrl);
      wsRef.current = ws;
      setState((current) => ({
        ...current,
        message: "Connecting to CDP stream.",
      }));

      ws.onopen = () => {
        if (disposed) return;
        retryCountRef.current = 0;
        setState((current) => ({
          ...current,
          connected: true,
          message: "Waiting for CDP frames.",
        }));
      };

      ws.onclose = () => {
        if (disposed) return;
        setState((current) => ({
          ...current,
          connected: false,
          browserConnected: false,
          screencasting: false,
          message: "CDP stream disconnected; reconnecting.",
        }));
        const delay = Math.min(1000 * 2 ** retryCountRef.current, 10000);
        retryCountRef.current += 1;
        reconnectTimerRef.current = setTimeout(connect, delay);
      };

      ws.onerror = () => {
        if (window.location.protocol === "https:") {
          setState((current) => ({
            ...current,
            connected: false,
            httpFallback: true,
            message: "WebSocket stream unavailable; using HTTPS frame polling.",
          }));
        }
        ws.close();
      };

      ws.onmessage = (event) => {
        let msg: StreamMessage;
        try {
          msg = JSON.parse(event.data) as StreamMessage;
        } catch {
          return;
        }

        switch (msg.type) {
          case "frame":
            drawFrame(msg.data);
            setState((current) => ({
              ...current,
              frameReceived: true,
              message: "",
            }));
            break;
          case "status": {
            const supported = !msg.engine || SCREENCAST_ENGINES.has(msg.engine);
            frameSizeRef.current = {
              width: msg.viewportWidth || frameSizeRef.current.width,
              height: msg.viewportHeight || frameSizeRef.current.height,
            };
            setState((current) => ({
              ...current,
              browserConnected: msg.connected,
              screencasting: msg.screencasting,
              viewportWidth: msg.viewportWidth || current.viewportWidth,
              viewportHeight: msg.viewportHeight || current.viewportHeight,
              message: msg.connected
                ? supported
                  ? msg.screencasting
                    ? current.message
                    : "CDP stream connected; waiting for screencast frames."
                  : `CDP screencast is not available for ${msg.engine}.`
                : "CDP stream is connected, but the browser is not ready.",
            }));
            break;
          }
          case "error":
            setState((current) => ({ ...current, message: msg.message }));
            break;
          default:
            break;
        }
      };
    };

    connect();

    return () => {
      disposed = true;
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [drawFrame, websocketUrl, refreshNonce]);

  useEffect(() => {
    if (!state.httpFallback || !streamPort) return;
    let disposed = false;

    const poll = async () => {
      try {
        const response = await fetch(`/api/stream/${encodeURIComponent(streamPort)}/frame`, {
          cache: "no-store",
          credentials: "include",
        });
        const json = await response.json() as {
          success?: boolean;
          frame?: string | null;
          status?: {
            connected?: boolean;
            screencasting?: boolean;
            viewportWidth?: number;
            viewportHeight?: number;
          } | null;
          error?: string | null;
        };
        if (disposed) return;
        if (json.frame) {
          drawFrame(json.frame);
        }
        setState((current) => ({
          ...current,
          connected: Boolean(json.success),
          browserConnected: Boolean(json.status?.connected),
          screencasting: Boolean(json.status?.screencasting),
          viewportWidth: json.status?.viewportWidth || current.viewportWidth,
          viewportHeight: json.status?.viewportHeight || current.viewportHeight,
          frameReceived: Boolean(json.frame) || current.frameReceived,
          message: json.frame ? "" : json.error || "Waiting for CDP frames through HTTPS polling.",
        }));
      } catch (err) {
        if (disposed) return;
        setState((current) => ({
          ...current,
          connected: false,
          message: err instanceof Error ? err.message : "HTTPS frame polling failed.",
        }));
      }
    };

    void poll();
    const timer = window.setInterval(poll, 900);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [drawFrame, state.httpFallback, streamPort]);

  const toViewport = useCallback((e: ReactMouseEvent): { x: number; y: number } | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const width = canvas.width || frameSizeRef.current.width || state.viewportWidth;
    const height = canvas.height || frameSizeRef.current.height || state.viewportHeight;
    const scaleX = width / Math.max(rect.width, 1);
    const scaleY = height / Math.max(rect.height, 1);
    return {
      x: Math.round((e.clientX - rect.left) * scaleX),
      y: Math.round((e.clientY - rect.top) * scaleY),
    };
  }, [state.viewportHeight, state.viewportWidth]);

  const handleMouseEvent = useCallback((e: ReactMouseEvent, eventType: string) => {
    const pos = toViewport(e);
    if (!pos) return;
    sendInput({
      type: "input_mouse",
      eventType,
      x: pos.x,
      y: pos.y,
      button: cdpButton(e.button),
      clickCount: eventType === "mousePressed" ? 1 : 0,
      modifiers: cdpModifiers(e),
    });
  }, [sendInput, toViewport]);

  const handleWheel = useCallback((e: ReactWheelEvent) => {
    const pos = toViewport(e);
    if (!pos) return;
    sendInput({
      type: "input_mouse",
      eventType: "mouseWheel",
      x: pos.x,
      y: pos.y,
      button: "none",
      clickCount: 0,
      deltaX: e.deltaX,
      deltaY: e.deltaY,
      modifiers: cdpModifiers(e),
    });
  }, [sendInput, toViewport]);

  const dispatchKey = useCallback((e: KeyboardEvent, eventType: string) => {
    const info = KEY_INFO[e.key];
    const text = eventType === "keyDown"
      ? (info?.text ?? (e.key.length === 1 ? e.key : undefined))
      : undefined;
    const keyCode = info?.keyCode ?? (e.key.length === 1 ? e.key.charCodeAt(0) : 0);
    sendInput({
      type: "input_keyboard",
      eventType,
      key: e.key,
      code: e.code,
      text,
      windowsVirtualKeyCode: keyCode,
      modifiers: cdpModifiers(e),
    });
  }, [sendInput]);

  useEffect(() => {
    if (!canControl) return;
    const handler = (event: KeyboardEvent) => {
      if (document.activeElement !== canvasRef.current) return;
      event.preventDefault();
      event.stopPropagation();
      dispatchKey(event, event.type === "keydown" ? "keyDown" : "keyUp");
    };
    window.addEventListener("keydown", handler, true);
    window.addEventListener("keyup", handler, true);
    return () => {
      window.removeEventListener("keydown", handler, true);
      window.removeEventListener("keyup", handler, true);
    };
  }, [canControl, dispatchKey]);

  const hasFrame = state.frameReceived;

  return (
    <div className="workspace-cdp-stream" data-provider="cdp_screencast" data-websocket-url={websocketUrl ?? ""}>
      <canvas
        ref={canvasRef}
        tabIndex={canControl ? 0 : -1}
        className="workspace-cdp-stream-canvas"
        aria-label={canControl ? "Interactive CDP workspace stream" : "CDP workspace stream"}
        onMouseMove={(event) => handleMouseEvent(event, "mouseMoved")}
        onMouseDown={(event) => {
          if (canControl) canvasRef.current?.focus();
          handleMouseEvent(event, "mousePressed");
        }}
        onMouseUp={(event) => handleMouseEvent(event, "mouseReleased")}
        onWheel={handleWheel}
        onContextMenu={(event) => event.preventDefault()}
      />
      {!hasFrame && (
        <div className="workspace-cdp-stream-status">
          <RefreshCw className={cn("size-4", state.connected && "animate-spin")} />
          <span>
            {state.message || "Waiting for CDP frames."}
          </span>
        </div>
      )}
      <div className="workspace-cdp-stream-footer">
        <span className={cn("workspace-cdp-stream-dot", state.connected && "workspace-cdp-stream-dot-ready")} />
        <span>{state.browserConnected ? state.screencasting || state.frameReceived ? "CDP stream live" : "CDP stream waiting" : "CDP browser idle"}</span>
        <span className="workspace-cdp-stream-port">{websocketUrl ?? streamUrl}</span>
      </div>
    </div>
  );
}

export function WorkspaceRemoteViewport({
  fallback,
  selectedWorkspaceContext,
}: {
  fallback: ReactNode;
  selectedWorkspaceContext?: SelectedWorkspaceContext | null;
}) {
  const activePort = useAtomValue(activePortAtom);
  const activeSessionName = useAtomValue(activeSessionNameAtom);
  const sessions = useAtomValue(sessionsAtom);
  const [viewportSelection, setViewportSelection] = useState<WorkspaceViewportSelection | null>(() => readWorkspaceViewportSelection());
  const [serviceStatus, setServiceStatus] = useState<ServiceStatusData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [focusMessage, setFocusMessage] = useState("");
  const [focusPending, setFocusPending] = useState(false);
  const [takeoverPending, setTakeoverPending] = useState(false);
  const [recoveryPending, setRecoveryPending] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);
  const [fullscreenFallback, setFullscreenFallback] = useState(false);
  const [streamRefreshNonce, setStreamRefreshNonce] = useState(() => Date.now());
  const [tileRefreshNonces, setTileRefreshNonces] = useState<Record<string, number>>({});
  const [frameIssue, setFrameIssue] = useState<WorkspaceFrameIssue>(null);
  const [streamPreflight, setStreamPreflight] = useState<StreamPreflightState>({
    status: "idle",
    message: "",
  });
  const viewportRef = useRef<HTMLElement | null>(null);
  const viewportFrameRef = useRef<HTMLIFrameElement | null>(null);
  const focusedKeyRef = useRef("");
  const streamFrameRetryRef = useRef(0);
  const touchClickBridgeCleanupRef = useRef<(() => void) | null>(null);

  const clearFullscreenFallbackOffset = useCallback(() => {
    const root = viewportRef.current;
    if (!root) return;
    root.style.removeProperty("--workspace-remote-viewport-offset-x");
    root.style.removeProperty("--workspace-remote-viewport-offset-y");
  }, []);

  const enableFullscreenFallback = useCallback(() => {
    clearFullscreenFallbackOffset();
    setFullscreenFallback(true);
    setFullscreen(true);
  }, [clearFullscreenFallbackOffset]);

  const fetchServiceStatus = useCallback(async () => {
    if (typeof window === "undefined") return;
    setLoading(true);
    try {
      const resp = await fetch(`${serviceBase(activePort)}/status`);
      const json = (await resp.json()) as ApiResponse<ServiceStatusData>;
      if (!json.success) throw new Error(json.error || "Service status failed");
      setServiceStatus(json.data ?? null);
      setError("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service status unavailable");
    } finally {
      setLoading(false);
    }
  }, [activePort]);

  const refreshWorkspaceViewport = useCallback(() => {
    streamFrameRetryRef.current = 0;
    setFrameIssue(null);
    setStreamRefreshNonce(Date.now());
    void fetchServiceStatus();
  }, [fetchServiceStatus]);

  useEffect(() => {
    const onSelection = () => setViewportSelection(readWorkspaceViewportSelection());
    window.addEventListener("popstate", onSelection);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onSelection);
    return () => {
      window.removeEventListener("popstate", onSelection);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onSelection);
    };
  }, []);

  useEffect(() => {
    if (!viewportSelection) return;
    void fetchServiceStatus();
    const timer = window.setInterval(fetchServiceStatus, 7000);
    return () => window.clearInterval(timer);
  }, [fetchServiceStatus, viewportSelection]);

  const browserId = viewportSelection ? browserIdFromSelection(viewportSelection.selection) : null;
  const serviceBrowser = browserId ? serviceStatus?.service_state?.browsers?.[browserId] ?? null : null;
  const daemonBrowser = daemonBrowserFromSession(daemonSessionFromSelection(sessions, viewportSelection?.selection));
  const browser = chooseWorkspaceViewportBrowser(serviceBrowser, daemonBrowser);
  const tabs = useMemo(() => Object.values(serviceStatus?.service_state?.tabs ?? {}), [serviceStatus]);
  const tabSelection = browser?.id && viewportSelection
    ? selectedTabForBrowser(tabs, browser.id, viewportSelection.selection)
    : { tab: null, tabIndex: null, recoveredFromStaleSelection: false };
  const stream = primaryViewStream(browser);
  const tileStreams = viewportSelection?.mode === "tile" ? workspaceViewportTiles(serviceStatus) : [];
  const streamUrl = resolveWorkspaceStreamUrl(stream);
  const externalStreamUrl = resolveWorkspaceStreamUrl(stream, "external");
  const frameUrl = buildWorkspaceFrameUrl(streamUrl, streamRefreshNonce);
  const canEmbed = stream ? canEmbedViewStream(stream) : false;
  const canControl = stream ? canOpenControlViewStream(stream) : false;
  const canRenderSelectedBrowser = browserCanRenderWorkspaceViewport(browser);
  const canRenderCdpStream = canRenderSelectedBrowser && isCdpScreencastStream(stream) && Boolean(streamUrl) && streamPreflight.status === "ready";
  const canRenderFrame = canRenderSelectedBrowser && !isCdpScreencastStream(stream) && canEmbed && streamPreflight.status === "ready";
  const singleWorkspaceMode = viewportSelection?.mode === "control" ? "control" : "view";
  const viewportUxState = deriveWorkspaceViewportUxState({
    hasBrowser: Boolean(browser),
    browserHealth: browser?.health,
    hasStream: Boolean(stream),
    canEmbed,
    canControl,
    mode: singleWorkspaceMode,
    preflightStatus: streamPreflight.status,
    frameIssueKind: frameIssue?.kind ?? null,
    focusPending,
    takeoverPending,
    recoveredStaleTarget: tabSelection.recoveredFromStaleSelection,
  });
  const viewportReadiness = deriveWorkspaceViewportReadiness({
    hasBrowser: Boolean(browser),
    browserHealth: browser?.health,
    hasStream: Boolean(stream),
    canEmbed,
    canControl,
    mode: singleWorkspaceMode,
    preflightStatus: streamPreflight.status,
    preflightMessage: streamPreflight.message,
    frameIssueKind: frameIssue?.kind ?? null,
    frameIssueMessage: frameIssue?.message ?? null,
    focusPending,
    takeoverPending,
    recoveredStaleTarget: tabSelection.recoveredFromStaleSelection,
    streamProvider: stream?.provider,
    streamUrl,
    streamReadiness: stream?.readiness ?? stream?.remoteReadiness,
    focusMessage,
  });

  useEffect(() => {
    streamFrameRetryRef.current = 0;
    setFrameIssue(null);
  }, [streamUrl]);

  useEffect(() => {
    if (!frameUrl || !canEmbed) {
      setStreamPreflight({ status: "idle", message: "" });
      return;
    }

    let disposed = false;
    const preflightStreamUrl = frameUrl;
    setStreamPreflight({ status: "checking", message: "Checking stream access." });

    async function checkStreamAccess() {
      try {
        const resolved = new URL(preflightStreamUrl, window.location.href);
        if (resolved.origin !== window.location.origin) {
          setStreamPreflight({ status: "ready", message: "" });
          return;
        }
        const response = await fetch(resolved.toString(), {
          cache: "no-store",
          credentials: "include",
          redirect: "follow",
        });
        if (disposed) return;
        if (responseLooksLikeDashboardLogin(response, resolved)) {
          setStreamPreflight({
            status: "login-required",
            message: "The remote stream needs a fresh dashboard sign-in before it can be embedded.",
          });
          return;
        }
        if (response.status === 401 || response.status === 403) {
          setStreamPreflight({
            status: "login-required",
            message: "The remote stream rejected the current dashboard session.",
          });
          return;
        }
        if (!response.ok) {
          setStreamPreflight({
            status: "error",
            message: `The remote stream returned HTTP ${response.status}.`,
          });
          return;
        }
        setStreamPreflight({ status: "ready", message: "" });
      } catch (err) {
        if (disposed) return;
        setStreamPreflight({
          status: "error",
          message: err instanceof Error ? err.message : "The remote stream could not be reached.",
        });
      }
    }

    void checkStreamAccess();
    return () => {
      disposed = true;
    };
  }, [canEmbed, frameUrl]);

  const handleFrameLoadIssue = useCallback((failure: WorkspaceFrameFailure) => {
    if (failure === "login-required") {
      setFrameIssue(null);
      setStreamPreflight({
        status: "login-required",
        message: "The remote stream needs a fresh dashboard sign-in before it can be embedded.",
      });
      return;
    }

    if (failure === "remote-disconnected" || failure === "taken-over") {
      setFrameIssue({
        kind: failure,
        message: failure === "taken-over"
          ? "This viewer was taken over by another dashboard or Guacamole popout. Take over to reconnect it here."
          : "Another dashboard or Guacamole popout is using this remote desktop. Take over to reconnect it here.",
      });
      return;
    }

    if (streamFrameRetryRef.current < 2) {
      streamFrameRetryRef.current += 1;
      setFrameIssue(null);
      setFocusMessage("Remote stream load failed; retrying the Guacamole viewport.");
      setStreamRefreshNonce(Date.now());
      return;
    }

    setFrameIssue(null);
    setStreamPreflight({
      status: "error",
      message: failure === "fatal-error"
        ? "Guacamole reported that the remote desktop connection closed."
        : "The embedded remote stream failed to load. Refresh the workspace viewport or open the stream externally.",
    });
  }, []);

  const onFrameLoad = useCallback(() => {
    const failure = detectWorkspaceFrameFailure(viewportFrameRef.current);
    if (failure) {
      handleFrameLoadIssue(failure);
      return;
    }
    setFrameIssue(null);
    streamFrameRetryRef.current = 0;
  }, [handleFrameLoadIssue]);

  const onFrameError = useCallback(() => {
    handleFrameLoadIssue("browser-error");
  }, [handleFrameLoadIssue]);

  useEffect(() => {
    if (!frameUrl || !canRenderFrame) return;
    const timer = window.setInterval(() => {
      const failure = detectWorkspaceFrameFailure(viewportFrameRef.current);
      if (failure === "remote-disconnected" || failure === "taken-over") {
        handleFrameLoadIssue(failure);
        return;
      }
      if (!failure) setFrameIssue(null);
    }, 1000);
    return () => window.clearInterval(timer);
  }, [canRenderFrame, frameUrl, handleFrameLoadIssue]);

  useEffect(() => {
    if (!viewportSelection || viewportSelection.mode !== "control") return;
    if (!browser || !stream || !canControl) return;
    const tabIndex = tabSelection.tabIndex;
    const targetId = tabSelection.tab?.targetId?.trim();
    const focusKey = [browser.id, tabSelection.tab?.id ?? "", targetId ?? "", tabIndex ?? "", streamUrl ?? ""].join("|");
    if (focusedKeyRef.current === focusKey) return;
    focusedKeyRef.current = focusKey;
    const browserForFocus = browser;
    const selectionForFocus = viewportSelection.selection;
    if (!targetId && tabIndex === null) {
      setFocusMessage("No stable tab index was available; showing the stream without a queued focus request.");
      return;
    }

    async function queueFocus() {
      const sessionName = daemonSessionNameForBrowser(browserForFocus, selectionForFocus);
      const params = targetId
        ? { targetId, ...(tabIndex !== null ? { index: tabIndex } : {}), maximize: true, ...(sessionName ? { sessionName } : {}) }
        : { index: tabIndex, maximize: true, ...(sessionName ? { sessionName } : {}) };
      setFocusPending(true);
      try {
        const resp = await fetch(`${serviceBase(activePort)}/request`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            action: "view_focus",
            serviceName: "agent-browser-dashboard",
            agentName: activeSessionName || "operator",
            taskName: "workspace-viewport-control",
            params,
            jobTimeoutMs: 5000,
          }),
        });
        const json = (await resp.json()) as ApiResponse<unknown>;
        if (!json.success) {
          setFocusMessage(json.error || "Remote-view focus request was not accepted; showing the stream anyway.");
          return;
        }
        setFocusMessage(tabSelection.recoveredFromStaleSelection
          ? "Recovered stale selected tab identity and queued view focus against the current live target."
          : "Queued view focus and maximize before opening the workspace viewport.");
      } catch (err) {
        setFocusMessage(err instanceof Error
          ? `Remote-view focus request failed: ${err.message}`
          : "Remote-view focus request failed; showing the stream anyway.");
      } finally {
        setFocusPending(false);
      }
    }

    void queueFocus();
  }, [activePort, activeSessionName, browser, canControl, stream, streamUrl, tabSelection.recoveredFromStaleSelection, tabSelection.tab?.id, tabSelection.tabIndex, viewportSelection]);

  useEffect(() => {
    if (!frameUrl || !canRenderFrame || !canControl) return;
    const frame = viewportFrameRef.current;
    if (!frame) return;

    let disposed = false;
    let attempts = 0;
    const install = () => {
      if (disposed) return;
      const cleanup = installGuacamoleTouchClickBridge(frame);
      if (!cleanup) return;
      touchClickBridgeCleanupRef.current?.();
      touchClickBridgeCleanupRef.current = cleanup;
    };

    const onLoad = () => {
      attempts = 0;
      install();
    };

    frame.addEventListener("load", onLoad);
    install();
    const timer = window.setInterval(() => {
      attempts += 1;
      install();
      if (attempts >= 24 && touchClickBridgeCleanupRef.current) {
        window.clearInterval(timer);
      }
    }, 500);

    return () => {
      disposed = true;
      window.clearInterval(timer);
      frame.removeEventListener("load", onLoad);
      touchClickBridgeCleanupRef.current?.();
      touchClickBridgeCleanupRef.current = null;
    };
  }, [canControl, canRenderFrame, frameUrl]);

  useEffect(() => {
    if (typeof document === "undefined") return;
    const onFullscreenChange = () => {
      clearFullscreenFallbackOffset();
      setFullscreenFallback(false);
      setFullscreen(document.fullscreenElement === viewportRef.current);
    };
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, [clearFullscreenFallbackOffset]);

  const toggleFullscreen = useCallback(async () => {
    if (typeof document === "undefined") {
      setFullscreen((current) => !current);
      return;
    }

    const root = viewportRef.current;
    const isCurrentFullscreen = document.fullscreenElement === root;
    try {
      if (isCurrentFullscreen) {
        await document.exitFullscreen();
        return;
      }
      if (fullscreen || fullscreenFallback) {
        clearFullscreenFallbackOffset();
        setFullscreenFallback(false);
        setFullscreen(false);
        return;
      }
      if (root?.requestFullscreen) {
        await root.requestFullscreen();
        return;
      }
    } catch {
      // Keep the CSS fullscreen fallback available if the browser rejects native fullscreen.
    }
    enableFullscreenFallback();
  }, [clearFullscreenFallbackOffset, enableFullscreenFallback, fullscreen, fullscreenFallback]);

  const openInteractionSettings = useCallback(() => {
    const opened = openGuacamoleInteractionSettings(viewportFrameRef.current);
    setFocusMessage(opened
      ? "Opened Guacamole interaction settings for keyboard and mouse mode."
      : "Guacamole interaction settings are not available until the stream finishes loading.");
  }, []);

  const postWorkspaceRecoveryRequest = useCallback(async (
    action: ServiceRequestAction,
    taskName: string,
    params: Record<string, unknown>,
  ) => {
    const resp = await fetch(`${serviceBase(activePort)}/request`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        action,
        serviceName: "agent-browser-dashboard",
        agentName: activeSessionName || "operator",
        taskName,
        params,
        jobTimeoutMs: 5000,
      }),
    });
    const json = (await resp.json()) as ApiResponse<unknown>;
    if (!json.success) throw new Error(json.error || `${action} was not accepted`);
    return json;
  }, [activePort, activeSessionName]);

  const workspaceRouteId = stream?.routeId?.trim() || null;
  const workspaceViewerLeaseIds = useMemo(() => Array.from(new Set([
    ...(stream?.viewerLeaseIds ?? []),
    ...(stream?.controllerLeaseId ? [stream.controllerLeaseId] : []),
  ].filter((id): id is string => Boolean(id?.trim())))), [stream?.controllerLeaseId, stream?.viewerLeaseIds]);
  const workspaceSessionName = browser && viewportSelection ? daemonSessionNameForBrowser(browser, viewportSelection.selection) : null;
  const workspaceViewerId = activeSessionName || "operator";

  const refreshWorkspaceRoute = useCallback(async () => {
    if (!browser || !stream) return;
    const displayAllocationId = stream.displayAllocationId || browser.displayAllocationId;
    if (!displayAllocationId) {
      setFocusMessage("Route refresh requires a service-owned display allocation.");
      return;
    }
    setRecoveryPending("route-refresh");
    setFocusMessage("Refreshing the service-owned remote route lease.");
    try {
      await postWorkspaceRecoveryRequest("service_remote_view_route_checkout", "workspace-viewport-route-refresh", {
        displayAllocationId,
        browserId: browser.id,
        ...(workspaceSessionName ? { sessionName: workspaceSessionName } : {}),
        ...(stream.id ? { streamId: stream.id } : {}),
        ...(workspaceRouteId ? { routeId: workspaceRouteId } : {}),
        ...(stream.provider ? { provider: stream.provider } : {}),
        ...(stream.providerMode ? { providerMode: stream.providerMode } : {}),
        ...(stream.frameUrl ? { frameUrl: stream.frameUrl } : {}),
        ...(stream.externalUrl ? { externalUrl: stream.externalUrl } : {}),
        ...(stream.connectionId ? { connectionId: stream.connectionId } : {}),
        ...(stream.connectionName ? { connectionName: stream.connectionName } : {}),
      });
      streamFrameRetryRef.current = 0;
      setFrameIssue(null);
      setStreamRefreshNonce(Date.now());
      setFocusMessage("Refreshed the service-owned remote route lease.");
      void fetchServiceStatus();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Route refresh failed: ${err.message}` : "Route refresh failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [browser, fetchServiceStatus, postWorkspaceRecoveryRequest, stream, workspaceRouteId, workspaceSessionName]);

  const reconnectWorkspaceViewer = useCallback(async () => {
    if (!browser || !workspaceRouteId) return;
    setRecoveryPending("viewer-reconnect");
    setFocusMessage("Requesting a fresh observer lease for this workspace route.");
    try {
      await postWorkspaceRecoveryRequest("service_viewer_lease_request", "workspace-viewport-viewer-reconnect", {
        routeId: workspaceRouteId,
        browserId: browser.id,
        viewerId: workspaceViewerId,
        viewerName: workspaceViewerId,
        viewerRole: "observer",
        openMode: "embedded",
      });
      streamFrameRetryRef.current = 0;
      setFrameIssue(null);
      setStreamRefreshNonce(Date.now());
      setFocusMessage("Reconnected the service-owned observer lease.");
      void fetchServiceStatus();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Viewer reconnect failed: ${err.message}` : "Viewer reconnect failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [browser, fetchServiceStatus, postWorkspaceRecoveryRequest, workspaceRouteId, workspaceViewerId]);

  const takeoverWorkspaceController = useCallback(async () => {
    if (!browser || !workspaceRouteId) return;
    setRecoveryPending("controller-takeover");
    setFrameIssue(null);
    setFocusMessage("Requesting explicit controller takeover for this workspace route.");
    try {
      await postWorkspaceRecoveryRequest("service_controller_lease_takeover", "workspace-viewport-controller-takeover", {
        routeId: workspaceRouteId,
        browserId: browser.id,
        viewerId: workspaceViewerId,
        viewerName: workspaceViewerId,
        viewerRole: "controller",
        openMode: "embedded",
      });
      streamFrameRetryRef.current = 0;
      setStreamRefreshNonce(Date.now());
      setFocusMessage("Controller lease takeover was accepted and the viewport is reconnecting.");
      void fetchServiceStatus();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Controller takeover failed: ${err.message}` : "Controller takeover failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [browser, fetchServiceStatus, postWorkspaceRecoveryRequest, workspaceRouteId, workspaceViewerId]);

  const releaseWorkspaceViewers = useCallback(async () => {
    if (workspaceViewerLeaseIds.length === 0) {
      setFocusMessage("No retained viewer leases are attached to this workspace route.");
      return;
    }
    setRecoveryPending("viewer-release");
    setFocusMessage(`Releasing ${workspaceViewerLeaseIds.length} retained viewer lease${workspaceViewerLeaseIds.length === 1 ? "" : "s"}.`);
    try {
      for (const viewerLeaseId of workspaceViewerLeaseIds) {
        await postWorkspaceRecoveryRequest("service_viewer_lease_release", "workspace-viewport-viewer-release", {
          viewerLeaseId,
        });
      }
      setFrameIssue(null);
      setStreamRefreshNonce(Date.now());
      setFocusMessage("Released retained viewer leases for this workspace route.");
      void fetchServiceStatus();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Viewer release failed: ${err.message}` : "Viewer release failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [fetchServiceStatus, postWorkspaceRecoveryRequest, workspaceViewerLeaseIds]);

  const requestWorkspaceTakeover = useCallback(async (openMode: "iframe" | "external") => {
    if (!browser || !stream) return false;
    const sessionName = viewportSelection ? daemonSessionNameForBrowser(browser, viewportSelection.selection) : null;
    const params = {
      browserId: browser.id,
      ...(sessionName ? { sessionName } : {}),
      ...(stream.id ? { streamId: stream.id } : {}),
      ...(stream.provider ? { provider: stream.provider } : {}),
      ...(tabSelection.tab?.targetId ? { targetId: tabSelection.tab.targetId } : {}),
      ...(tabSelection.tabIndex !== null ? { index: tabSelection.tabIndex } : {}),
      openMode,
      reason: frameIssue?.kind ?? "operator_request",
    };

    setTakeoverPending(true);
    setFrameIssue(null);
    setFocusMessage("Requesting service-owned viewer takeover and reconnect.");
    try {
      const resp = await fetch(`${serviceBase(activePort)}/request`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "view_takeover",
          serviceName: "agent-browser-dashboard",
          agentName: activeSessionName || "operator",
          taskName: "workspace-viewport-takeover",
          params,
          jobTimeoutMs: 5000,
        }),
      });
      const json = (await resp.json()) as ApiResponse<unknown>;
      if (!json.success) {
        setFocusMessage(json.error || "Viewer takeover request was not accepted; refresh the workspace viewport or inspect readiness.");
        return false;
      }
      streamFrameRetryRef.current = 0;
      setStreamRefreshNonce(Date.now());
      setFocusMessage(openMode === "external"
        ? "Queued viewer takeover before opening the external workspace stream."
        : "Queued viewer takeover and reconnect for this workspace viewport.");
      void fetchServiceStatus();
      return true;
    } catch (err) {
      setFocusMessage(err instanceof Error
        ? `Viewer takeover request failed: ${err.message}`
        : "Viewer takeover request failed; refresh the workspace viewport or inspect readiness.");
      return false;
    } finally {
      setTakeoverPending(false);
    }
  }, [activePort, activeSessionName, browser, fetchServiceStatus, frameIssue?.kind, stream, tabSelection.tab?.targetId, tabSelection.tabIndex, viewportSelection]);

  const openWorkspaceStreamExternally = useCallback(async () => {
    if (!externalStreamUrl) return;
    const accepted = await requestWorkspaceTakeover("external");
    if (!accepted) return;
    window.open(externalStreamUrl, "_blank", "noopener,noreferrer");
  }, [externalStreamUrl, requestWorkspaceTakeover]);

  if (!viewportSelection) return <>{fallback}</>;

  if (viewportSelection.mode === "tile") {
    return (
      <section
        className="workspace-remote-viewport workspace-remote-viewport-tile"
        data-ux-state={tileStreams.length > 0 ? "connected" : "missing_stream"}
        data-readiness-status={tileStreams.length > 0 ? "ready" : "missing"}
        aria-label="Tiled workspace remote view"
      >
        <header className="workspace-remote-viewport-header">
          <div className="min-w-0">
            <p className="workspace-remote-viewport-kicker">Workspace viewport / tile</p>
            <h2>Live remote workspaces</h2>
            <p>Service-owned remote routes shown side by side.</p>
          </div>
          <div className="workspace-remote-viewport-actions">
            <Badge variant="secondary" className="workspace-remote-viewport-badge">
              <span className="workspace-remote-viewport-badge-text">
                {tileStreams.length} live route{tileStreams.length === 1 ? "" : "s"}
              </span>
            </Badge>
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Refresh tiled workspace view"
              title="Refresh tiled workspace view"
              onClick={() => {
                setTileRefreshNonces({});
                void fetchServiceStatus();
              }}
            >
              <RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
            </Button>
          </div>
        </header>

        {error && (
          <div className="workspace-remote-viewport-notices">
            <p className="workspace-remote-viewport-notice workspace-remote-viewport-notice-bad">
              <AlertTriangle className="size-3.5" />
              {error}
            </p>
          </div>
        )}

        {tileStreams.length > 0 ? (
          <div className="workspace-remote-viewport-tile-grid">
            {tileStreams.map((tile) => {
              const nonce = tileRefreshNonces[tile.browser.id] ?? streamRefreshNonce;
              const tileFrameUrl = buildWorkspaceFrameUrl(tile.frameUrl, nonce);
              return (
                <article
                  key={tile.browser.id}
                  className={cn("workspace-remote-viewport-tile-card", tile.sharedRoute && "workspace-remote-viewport-tile-card-shared")}
                >
                  <header className="workspace-remote-viewport-tile-header">
                    <div className="min-w-0">
                      <h3>{workspaceViewportTitle(tile.browser)}</h3>
                      <p>{viewStreamRouteSummary(tile.stream)}</p>
                    </div>
                    <div className="workspace-remote-viewport-tile-actions">
                      {tile.sharedRoute && (
                        <Badge variant="destructive" className="workspace-remote-viewport-badge">
                          <span className="workspace-remote-viewport-badge-text">shared route</span>
                        </Badge>
                      )}
                      <Button
                        type="button"
                        size="icon"
                        variant="outline"
                        aria-label={`Refresh ${tile.browser.id}`}
                        title={`Refresh ${tile.browser.id}`}
                        onClick={() => {
                          setTileRefreshNonces((current) => ({
                            ...current,
                            [tile.browser.id]: Date.now(),
                          }));
                        }}
                      >
                        <RefreshCw className="size-3.5" />
                      </Button>
                      {tile.externalUrl && (
                        <Button size="icon" variant="outline" asChild>
                          <a href={tile.externalUrl} target="_blank" rel="noreferrer" aria-label={`Open ${tile.browser.id} externally`}>
                            <ExternalLink className="size-3.5" />
                          </a>
                        </Button>
                      )}
                    </div>
                  </header>
                  {tile.sharedRoute && (
                    <p className="workspace-remote-viewport-notice workspace-remote-viewport-notice-bad">
                      <AlertTriangle className="size-3.5" />
                      This route is shared by multiple workspaces; simultaneous viewing may fall back to provider takeover behavior.
                    </p>
                  )}
                  <div className="workspace-remote-viewport-tile-stage">
                    <iframe
                      key={`${tile.browser.id}:${tileFrameUrl ?? ""}`}
                      title={`${viewStreamLabel(tile.stream)} ${tile.browser.id}`}
                      src={tileFrameUrl ?? undefined}
                      className="workspace-remote-viewport-frame"
                      allow="clipboard-read; clipboard-write; fullscreen; pointer-lock"
                      allowFullScreen
                    />
                  </div>
                </article>
              );
            })}
          </div>
        ) : (
          <div className="workspace-remote-viewport-empty">
            <SquareArrowOutUpRight className="size-6" />
            <h3>No live remote routes</h3>
            <p>No service-owned embeddable remote-view routes are ready to tile.</p>
          </div>
        )}
      </section>
    );
  }

  const viewport = (
    <section
      ref={viewportRef}
      className={cn("workspace-remote-viewport", fullscreen && "workspace-remote-viewport-fullscreen")}
      data-ux-state={viewportUxState}
      data-readiness-status={viewportReadiness.status}
      data-readiness-action={viewportReadiness.nextAction}
      data-selected-workspace-id={selectedWorkspaceContext?.node?.id ?? ""}
      data-selected-workspace-state={selectedWorkspaceContext?.state ?? ""}
      aria-label="Workspace remote viewport"
    >
      <header className="workspace-remote-viewport-header">
        <div className="min-w-0">
          <p className="workspace-remote-viewport-kicker">
            Workspace viewport
            {viewportSelection.mode === "control" ? " / control" : " / view"}
          </p>
          <h2>{browser ? workspaceViewportTitle(browser, tabSelection.tab) : "No selected browser stream"}</h2>
          <p>{browser ? workspaceViewportSubtitle(browser, tabSelection.tab) : "Select a workspace with service-owned view-stream evidence."}</p>
        </div>
        <div className="workspace-remote-viewport-actions">
          {stream && (
            <Badge
              variant={canControl ? "default" : canEmbed ? "secondary" : "outline"}
              className="workspace-remote-viewport-badge"
            >
              <span className="workspace-remote-viewport-badge-text">
                {viewStreamCapabilityLabel(stream)}
              </span>
            </Badge>
          )}
          {stream && (
            <Badge variant="outline" className="workspace-remote-viewport-badge">
              <span className="workspace-remote-viewport-badge-text">
                {viewStreamRouteSummary(stream)}
              </span>
            </Badge>
          )}
          {stream && (
            <Badge
              variant={viewStreamReadinessLabel(stream) === "ready" ? "secondary" : "outline"}
              className="workspace-remote-viewport-badge"
            >
              <span className="workspace-remote-viewport-badge-text">
                {viewStreamReadinessLabel(stream)}
              </span>
            </Badge>
          )}
          <Badge variant="outline" className="workspace-remote-viewport-badge">
            <span className="workspace-remote-viewport-badge-text">
              {workspaceViewportUxStateLabel(viewportUxState)}
            </span>
          </Badge>
          <Badge
            variant={viewportReadiness.status === "ready" ? "secondary" : viewportReadiness.status === "blocked" ? "destructive" : "outline"}
            className="workspace-remote-viewport-badge"
          >
            <span className="workspace-remote-viewport-badge-text">
              {workspaceViewportReadinessStatusLabel(viewportReadiness.status)}
            </span>
          </Badge>
          <Button
            type="button"
            size="icon"
            variant="outline"
            aria-label="Refresh workspace viewport"
            title="Refresh workspace viewport"
            onClick={() => {
              refreshWorkspaceViewport();
            }}
          >
            <RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
          </Button>
          {stream && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Refresh remote route lease"
              title="Refresh remote route lease"
              disabled={Boolean(recoveryPending) || !(stream.displayAllocationId || browser?.displayAllocationId)}
              onClick={() => {
                void refreshWorkspaceRoute();
              }}
            >
              <PlugZap className={cn("size-3.5", recoveryPending === "route-refresh" && "animate-spin")} />
            </Button>
          )}
          {workspaceRouteId && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Reconnect viewer lease"
              title="Reconnect viewer lease"
              disabled={Boolean(recoveryPending)}
              onClick={() => {
                void reconnectWorkspaceViewer();
              }}
            >
              <RefreshCw className={cn("size-3.5", recoveryPending === "viewer-reconnect" && "animate-spin")} />
            </Button>
          )}
          {workspaceRouteId && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Take controller lease"
              title="Take controller lease"
              disabled={Boolean(recoveryPending)}
              onClick={() => {
                void takeoverWorkspaceController();
              }}
            >
              <MousePointer2 className={cn("size-3.5", recoveryPending === "controller-takeover" && "animate-spin")} />
            </Button>
          )}
          {workspaceViewerLeaseIds.length > 0 && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Release viewer leases"
              title="Release viewer leases"
              disabled={Boolean(recoveryPending)}
              onClick={() => {
                void releaseWorkspaceViewers();
              }}
            >
              <Unplug className={cn("size-3.5", recoveryPending === "viewer-release" && "animate-spin")} />
            </Button>
          )}
          {externalStreamUrl && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Open workspace stream externally"
              title="Open workspace stream externally"
              onClick={openWorkspaceStreamExternally}
            >
              <ExternalLink className="size-3.5" />
            </Button>
          )}
          {stream?.provider === "rdp_gateway" && (
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label="Open Guacamole interaction settings"
              title="Open Guacamole keyboard and mouse settings"
              disabled={!canRenderFrame}
              onClick={openInteractionSettings}
            >
              <Settings2 className="size-3.5" />
            </Button>
          )}
          <Button
            type="button"
            size="icon"
            variant="outline"
            aria-label={fullscreen ? "Return workspace viewport to window" : "Open workspace viewport fullscreen"}
            title={fullscreen ? "Return workspace viewport to window" : "Open workspace viewport fullscreen"}
            onClick={() => void toggleFullscreen()}
          >
            {fullscreen ? <Minimize2 className="size-3.5" /> : <Maximize2 className="size-3.5" />}
          </Button>
        </div>
      </header>

      {(error || viewportReadiness.status !== "ready" || focusMessage || takeoverPending || recoveryPending || (stream && !canControl && viewportSelection.mode === "control")) && (
        <div className="workspace-remote-viewport-notices">
          {error && (
            <p className="workspace-remote-viewport-notice workspace-remote-viewport-notice-bad">
              <AlertTriangle className="size-3.5" />
              {error}
            </p>
          )}
          {viewportReadiness.status !== "ready" && (
            <div className={cn(
              "workspace-remote-viewport-notice",
              viewportReadiness.status === "blocked" || viewportReadiness.nextAction === "take_over"
                ? "workspace-remote-viewport-notice-bad"
                : undefined,
            )}>
              <AlertTriangle className="size-3.5" />
              <span className="workspace-remote-viewport-notice-text">
                <strong>{viewportReadiness.title}.</strong> {viewportReadiness.recoveryCopy}
              </span>
              {viewportReadiness.nextAction === "take_over" && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="workspace-remote-viewport-notice-action"
                  disabled={takeoverPending}
                  onClick={() => {
                    void requestWorkspaceTakeover("iframe");
                  }}
                >
                  <RefreshCw className={cn("size-3.5", takeoverPending && "animate-spin")} />
                  Take over
                </Button>
              )}
              {viewportReadiness.nextAction === "sign_in_again" && (
                <Button size="sm" variant="default" className="workspace-remote-viewport-notice-action" asChild>
                  <a href={dashboardLoginPath()}>
                    <LogIn className="size-3.5" />
                    Sign in again
                  </a>
                </Button>
              )}
              {viewportReadiness.nextAction === "open_externally" && externalStreamUrl && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="workspace-remote-viewport-notice-action"
                  onClick={openWorkspaceStreamExternally}
                >
                  <ExternalLink className="size-3.5" />
                  Open externally
                </Button>
              )}
            </div>
          )}
          {takeoverPending && (
            <p className="workspace-remote-viewport-notice">
              <RefreshCw className="size-3.5 animate-spin" />
              Reconnecting the service-owned viewer lease.
            </p>
          )}
          {recoveryPending && (
            <p className="workspace-remote-viewport-notice">
              <RefreshCw className="size-3.5 animate-spin" />
              Running service-owned remote-view recovery action.
            </p>
          )}
          {focusMessage && (
            <p className="workspace-remote-viewport-notice">
              <MousePointer2 className="size-3.5" />
              {focusMessage}
            </p>
          )}
          {stream && !canControl && viewportSelection.mode === "control" && (
            <p className="workspace-remote-viewport-notice">
              <AlertTriangle className="size-3.5" />
              The service marked this stream as {controlInputLabel(stream)}, so the viewport is view-only.
            </p>
          )}
        </div>
      )}

      <div className="workspace-remote-viewport-stage">
        {stream && canRenderCdpStream && streamUrl ? (
          <WorkspaceCdpStreamCanvas
            key={`${streamUrl}:${streamRefreshNonce}`}
            streamUrl={streamUrl}
            canControl={canControl}
            refreshNonce={streamRefreshNonce}
          />
        ) : stream && canRenderFrame ? (
          <iframe
            key={`${streamUrl ?? ""}:${streamRefreshNonce}`}
            ref={viewportFrameRef}
            title={`${viewStreamLabel(stream)} ${stream.id ?? ""}`.trim()}
            src={frameUrl ?? undefined}
            className="workspace-remote-viewport-frame"
            allow="clipboard-read; clipboard-write; fullscreen; pointer-lock"
            allowFullScreen
            onLoad={onFrameLoad}
            onError={onFrameError}
          />
        ) : (
          <div className="workspace-remote-viewport-empty">
            {streamPreflight.status === "login-required" ? (
              <LogIn className="size-6" />
            ) : (
              <SquareArrowOutUpRight className="size-6" />
            )}
            <h3>
              {viewportReadiness.title}
            </h3>
            <p>
              {viewportReadiness.recoveryCopy
                || streamPreflight.message
                || (stream ? viewStreamOpenTitle(stream) : "The selected workspace does not currently report a service-owned view stream.")}
            </p>
            <div className="workspace-remote-viewport-empty-actions">
              {viewportReadiness.nextAction === "sign_in_again" && (
                <Button size="sm" variant="default" asChild>
                  <a href={dashboardLoginPath()}>
                    <LogIn className="size-3.5" />
                    Sign in again
                  </a>
                </Button>
              )}
              {externalStreamUrl && (
                <Button size="sm" variant="outline" onClick={openWorkspaceStreamExternally}>
                  <ExternalLink className="size-3.5" />
                  Open externally
                </Button>
              )}
            </div>
          </div>
        )}
      </div>
    </section>
  );

  if (fullscreenFallback && typeof document !== "undefined") {
    return createPortal(viewport, document.body);
  }

  return viewport;
}
