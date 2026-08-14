"use client";

import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type ReactNode, type WheelEvent as ReactWheelEvent } from "react";
import { createPortal } from "react-dom";
import { useAtomValue, useSetAtom } from "jotai/react";
import { AlertTriangle, ChevronDown, Download, ExternalLink, LogIn, Maximize2, Minimize2, MoreHorizontal, MousePointer2, PlugZap, RefreshCw, Settings2, SquareArrowOutUpRight, Unplug } from "lucide-react";
import {
  canEmbedViewStream,
  canOpenControlViewStream,
  canOpenViewStream,
  controlInputLabel,
  viewStreamDashboardFrameUrl,
  viewStreamExternalUrl,
  viewStreamLabel,
  viewStreamOpenTitle,
  viewStreamRouteSummary,
  type ServiceViewStream,
} from "@/lib/service-view-streams";
import {
  DASHBOARD_WORKSPACE_SELECTION_EVENT,
  dashboardWorkspaceSelectionHasValue,
  readDashboardWorkspaceUrlSelection,
  writeDashboardWorkspaceUrlSelection,
  type DashboardWorkspaceUrlSelection,
} from "@/lib/workspace-url-selection";
import type { SelectedWorkspaceContext } from "@/lib/selected-workspace-context";
import type { WorkspaceViewProjection } from "@/lib/workspace-view-projection";
import {
  selectWorkspaceViewerRoute,
  workspaceRecoveryFailureMessage,
} from "@/lib/workspace-recovery";
import {
  planAutomaticWorkspaceConnection,
  resolveWorkspaceViewSources,
  workspaceConnectionReadinessGeneration,
  workspaceViewerRouteIsAttached,
  type WorkspaceViewSource,
} from "@/lib/workspace-view-connection";
import { activePortAtom, activeSessionNameAtom, sessionsAtom } from "@/store/sessions";
import { appendConsoleLogsAtom } from "@/store/stream";
import type { SessionInfo } from "@/types";
import { cn } from "@/lib/utils";
import { SERVICE_API_BASE } from "@/lib/dashboard-api";
import {
  deriveWorkspaceViewportReadiness,
  deriveWorkspaceViewportUxState,
} from "@/lib/workspace-viewport-state";
import {
  INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE,
  workspaceViewportControllerReducer,
  workspaceViewportTargetToken,
  type WorkspaceViewportPreflightState,
  type WorkspaceViewportTarget,
} from "@/lib/workspace-viewport-controller";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { StreamMessage } from "@/types";
import {
  borrowForeignCdpControl,
  dispatchForeignCdpInput,
  fetchForeignCdpScreenshot,
  readForeignCdpControlStatus,
  releaseForeignCdpControl,
  type ForeignCdpBorrowStatus,
  type ForeignCdpInput,
} from "@/lib/foreign-cdp-control";

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
  attachability?: unknown;
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

type ApiResponse<T> = {
  success: boolean;
  data?: T;
  code?: string | null;
  error?: string | null;
};

async function readWorkspaceApiResponse<T extends { error?: string | null }>(response: Response): Promise<T> {
  const body = await response.text();
  try {
    return JSON.parse(body) as T;
  } catch {
    const detail = body.trim().slice(0, 240) || response.statusText || "empty response";
    throw new Error(`HTTP ${response.status}: ${detail}`);
  }
}

function viewportRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : null;
}

function viewportString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

type ServiceRequestAction =
  | "service_remote_view_browser_reattach"
  | "service_remote_view_route_switch"
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
  stream: ServiceViewStream | null;
  frameUrl: string | null;
  externalUrl: string | null;
  routeKey: string | null;
  sharedRoute: boolean;
  streamChoices: readonly ServiceViewStream[];
  streamChoiceKeys: readonly string[];
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

function isCdpScreencastStream(stream?: { provider?: string | null } | null): boolean {
  return stream?.provider?.trim().toLowerCase() === "cdp_screencast";
}

function isCdpSnapshotStream(stream?: { provider?: string | null } | null): boolean {
  return stream?.provider?.trim().toLowerCase() === "cdp_snapshot";
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

function workspaceCdpStreamPort(streamUrl: string | null): number | null {
  if (!streamUrl || typeof window === "undefined") return null;
  try {
    const port = new URL(streamUrl, window.location.href).port;
    return port ? Number(port) : null;
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
  const dashboardHref = typeof window === "undefined" ? null : window.location.href;
  const streamUrl = mode === "external" ? viewStreamExternalUrl(stream) : viewStreamDashboardFrameUrl(stream, dashboardHref);
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
    if (isGuacamoleClientFrameUrl(resolved)) return resolved.toString();
    if (resolved.origin === window.location.origin) {
      resolved.searchParams.set("agentBrowserViewport", "workspace");
      resolved.searchParams.set("agentBrowserRefresh", String(refreshNonce));
    }
    return resolved.toString();
  } catch {
    return streamUrl;
  }
}

function buildCdpSnapshotUrl(streamUrl: string | null, targetId?: string | null): string | null {
  if (!streamUrl || typeof window === "undefined") return streamUrl;
  try {
    const resolved = new URL(streamUrl, window.location.href);
    const target = targetId?.trim();
    if (target) resolved.searchParams.set("targetId", target);
    return resolved.toString();
  } catch {
    return streamUrl;
  }
}

function isGuacamoleClientFrameUrl(url: URL): boolean {
  return /\/guacamole\/?$/i.test(url.pathname) && url.hash.startsWith("#/client/");
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
  const appendConsoleLogs = useSetAtom(appendConsoleLogsAtom);
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
    if (state.httpFallback) {
      if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
      wsRef.current?.close();
      wsRef.current = null;
      return;
    }
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
          case "console":
            appendConsoleLogs({ ...msg, streamPort });
            break;
          case "page_error":
            appendConsoleLogs({ ...msg, streamPort });
            break;
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
  }, [appendConsoleLogs, drawFrame, state.httpFallback, streamPort, websocketUrl, refreshNonce]);

  useEffect(() => {
    if (!state.httpFallback || !streamPort) return;
    let disposed = false;
    let timer: number | null = null;
    let controller: AbortController | null = null;

    const poll = async () => {
      controller = new AbortController();
      try {
        const response = await fetch(`/api/stream/${encodeURIComponent(streamPort)}/frame`, {
          cache: "no-store",
          credentials: "include",
          signal: controller.signal,
        });
        const json = await readWorkspaceApiResponse<{
          success?: boolean;
          frame?: string | null;
          status?: {
            connected?: boolean;
            screencasting?: boolean;
            viewportWidth?: number;
            viewportHeight?: number;
          } | null;
          error?: string | null;
        }>(response);
        if (disposed) return;
        if (!response.ok) {
          throw new Error(json.error || `Frame request returned HTTP ${response.status}.`);
        }
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
      } finally {
        controller = null;
        if (!disposed) timer = window.setTimeout(() => void poll(), 900);
      }
    };

    void poll();
    return () => {
      disposed = true;
      controller?.abort();
      if (timer !== null) window.clearTimeout(timer);
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

function WorkspaceCdpSnapshotViewer({
  snapshotUrl,
  refreshNonce,
  canControl,
  onInput,
  onTargetResolved,
}: {
  snapshotUrl: string;
  refreshNonce: number;
  canControl: boolean;
  onInput: (input: ForeignCdpInput) => void;
  onTargetResolved: (targetId: string) => void;
}) {
  const imageRef = useRef<HTMLImageElement | null>(null);
  const [state, setState] = useState<{
    dataUrl: string | null;
    connected: boolean;
    message: string;
    targetLabel: string;
  }>({
    dataUrl: null,
    connected: false,
    message: "Fetching read-only CDP screenshot.",
    targetLabel: "",
  });

  useEffect(() => {
    let disposed = false;
    let controller: AbortController | null = null;

    const fetchSnapshot = async () => {
      controller?.abort();
      controller = new AbortController();
      try {
        const response = await fetch(snapshotUrl, {
          cache: "no-store",
          credentials: "include",
          signal: controller.signal,
        });
        const json = await response.json() as {
          success?: boolean;
          dataUrl?: string | null;
          data?: string | null;
          format?: string | null;
          title?: string | null;
          url?: string | null;
          targetId?: string | null;
          error?: string | null;
        };
        if (disposed) return;
        if (!response.ok || json.success === false) {
          throw new Error(json.error || `Snapshot request returned HTTP ${response.status}.`);
        }
        const format = json.format || "jpeg";
        const dataUrl = json.dataUrl || (json.data ? `data:image/${format};base64,${json.data}` : null);
        if (!dataUrl) throw new Error("Snapshot response did not include image data.");
        setState({
          dataUrl,
          connected: true,
          message: "",
          targetLabel: [json.title, json.url, json.targetId].filter(Boolean).join(" / "),
        });
        if (json.targetId) onTargetResolved(json.targetId);
      } catch (err) {
        if (disposed || (err instanceof DOMException && err.name === "AbortError")) return;
        setState((current) => ({
          ...current,
          connected: false,
          message: err instanceof Error ? err.message : "Read-only CDP screenshot polling failed.",
        }));
      }
    };

    void fetchSnapshot();
    const timer = window.setInterval(fetchSnapshot, 750);
    return () => {
      disposed = true;
      controller?.abort();
      window.clearInterval(timer);
    };
  }, [onTargetResolved, snapshotUrl, refreshNonce]);

  const imageCoordinates = useCallback((clientX: number, clientY: number) => {
    const image = imageRef.current;
    if (!image?.naturalWidth || !image.naturalHeight) return null;
    const bounds = image.getBoundingClientRect();
    const scale = Math.min(bounds.width / image.naturalWidth, bounds.height / image.naturalHeight);
    const renderedWidth = image.naturalWidth * scale;
    const renderedHeight = image.naturalHeight * scale;
    const left = bounds.left + (bounds.width - renderedWidth) / 2;
    const top = bounds.top + (bounds.height - renderedHeight) / 2;
    if (clientX < left || clientX > left + renderedWidth || clientY < top || clientY > top + renderedHeight) {
      return null;
    }
    return {
      x: (clientX - left) / scale,
      y: (clientY - top) / scale,
    };
  }, []);

  const dispatchMouse = useCallback((
    event: ReactMouseEvent<HTMLDivElement>,
    eventType: "mousePressed" | "mouseReleased" | "mouseMoved",
  ) => {
    if (!canControl) return;
    const point = imageCoordinates(event.clientX, event.clientY);
    if (!point) return;
    const button = event.button === 1 ? "middle" : event.button === 2 ? "right" : "left";
    onInput({ kind: "mouse", eventType, ...point, button, clickCount: 1 });
  }, [canControl, imageCoordinates, onInput]);

  const dispatchKeyboard = useCallback((
    event: ReactKeyboardEvent<HTMLDivElement>,
    eventType: "keyDown" | "keyUp",
  ) => {
    if (!canControl) return;
    event.preventDefault();
    onInput({
      kind: "keyboard",
      eventType,
      key: event.key,
      code: event.code,
      text: eventType === "keyDown" && event.key.length === 1 ? event.key : undefined,
      modifiers: (event.altKey ? 1 : 0) | (event.ctrlKey ? 2 : 0) | (event.metaKey ? 4 : 0) | (event.shiftKey ? 8 : 0),
    });
  }, [canControl, onInput]);

  const dispatchWheel = useCallback((event: ReactWheelEvent<HTMLDivElement>) => {
    if (!canControl) return;
    const point = imageCoordinates(event.clientX, event.clientY);
    if (!point) return;
    event.preventDefault();
    onInput({ kind: "wheel", ...point, deltaX: event.deltaX, deltaY: event.deltaY });
  }, [canControl, imageCoordinates, onInput]);

  return (
    <div
      className={cn("workspace-cdp-stream", canControl && "workspace-cdp-stream-controllable")}
      data-provider="cdp_snapshot"
      data-snapshot-url={snapshotUrl}
      tabIndex={canControl ? 0 : -1}
      aria-label={canControl ? "Interactive borrowed foreign CDP browser" : "Read-only foreign CDP browser watch"}
      onMouseDown={(event) => {
        if (canControl) event.currentTarget.focus();
        dispatchMouse(event, "mousePressed");
      }}
      onMouseUp={(event) => dispatchMouse(event, "mouseReleased")}
      onMouseMove={(event) => {
        if (event.buttons !== 0) dispatchMouse(event, "mouseMoved");
      }}
      onKeyDown={(event) => dispatchKeyboard(event, "keyDown")}
      onKeyUp={(event) => dispatchKeyboard(event, "keyUp")}
      onWheel={dispatchWheel}
      onContextMenu={(event) => event.preventDefault()}
    >
      {state.dataUrl && (
        <img
          ref={imageRef}
          className="workspace-cdp-snapshot-image"
          src={state.dataUrl}
          alt={canControl ? "Borrowed interactive browser snapshot" : "Read-only browser snapshot"}
          draggable={false}
        />
      )}
      {(!state.dataUrl || state.message) && (
        <div className="workspace-cdp-stream-status">
          <RefreshCw className={cn("size-4", !state.message && "animate-spin")} />
          <span>{state.message || "Waiting for read-only CDP screenshot."}</span>
        </div>
      )}
      <div className="workspace-cdp-stream-footer" aria-live="polite">
        <span className={cn("workspace-cdp-stream-dot", state.connected && "workspace-cdp-stream-dot-ready")} />
        <span>{canControl ? "Foreign CDP Borrow control active" : state.connected ? "Foreign CDP watch live" : "Foreign CDP watch waiting"}</span>
        <span className="workspace-cdp-stream-port">{state.targetLabel || snapshotUrl}</span>
      </div>
    </div>
  );
}

function WorkspaceSourceMenu({
  sources,
  selected,
  onSelect,
  label = "View",
}: {
  sources: readonly WorkspaceViewSource[];
  selected: WorkspaceViewSource | null;
  onSelect: (source: WorkspaceViewSource) => void;
  label?: string;
}) {
  if (sources.length <= 1) return null;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button type="button" size="sm" variant="outline" aria-label={`${label} source`}>
          {label}: {selected?.label ?? "Unavailable"}
          <ChevronDown className="size-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-56">
        <DropdownMenuLabel>Browser view</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={selected?.identity ?? ""}
          onValueChange={(identity) => {
            const source = sources.find((candidate) => candidate.identity === identity);
            if (source) onSelect(source);
          }}
        >
          {sources.map((source) => (
            <DropdownMenuRadioItem key={source.identity} value={source.identity}>
              <span className="flex min-w-0 flex-col">
                <span>{source.label}</span>
                <span className="truncate text-xs text-muted-foreground">{source.detail}</span>
              </span>
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export function WorkspaceRemoteViewport({
  fallback,
  selectedWorkspaceContext,
  projection,
  onRefresh,
  onSelectStream,
}: {
  fallback: ReactNode;
  selectedWorkspaceContext?: SelectedWorkspaceContext | null;
  projection: WorkspaceViewProjection;
  onRefresh: () => Promise<void>;
  onSelectStream: (browserId: string, streamKey: string) => void;
}) {
  const activePort = useAtomValue(activePortAtom);
  const activeSessionName = useAtomValue(activeSessionNameAtom);
  const sessions = useAtomValue(sessionsAtom);
  const [viewportSelection, setViewportSelection] = useState<WorkspaceViewportSelection | null>(() => readWorkspaceViewportSelection());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [focusMessage, setFocusMessage] = useState("");
  const [focusPending, setFocusPending] = useState(false);
  const [takeoverPending, setTakeoverPending] = useState(false);
  const [recoveryPending, setRecoveryPending] = useState<string | null>(null);
  const [foreignBorrow, setForeignBorrow] = useState<ForeignCdpBorrowStatus | null>(null);
  const [foreignBorrowPending, setForeignBorrowPending] = useState(false);
  const [foreignTargetId, setForeignTargetId] = useState<string | null>(null);
  const [fullscreen, setFullscreen] = useState(false);
  const [fullscreenFallback, setFullscreenFallback] = useState(false);
  const [streamRefreshNonce, setStreamRefreshNonce] = useState(() => Date.now());
  const [tileRefreshNonces, setTileRefreshNonces] = useState<Record<string, number>>({});
  const [automaticAttemptKeys, setAutomaticAttemptKeys] = useState<string[]>([]);
  const [connectionRetryNonce, setConnectionRetryNonce] = useState(0);
  const [frameIssue, setFrameIssue] = useState<WorkspaceFrameIssue>(null);
  const [viewportController, dispatchViewportController] = useReducer(
    workspaceViewportControllerReducer,
    INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE,
  );
  const viewportRef = useRef<HTMLElement | null>(null);
  const viewportFrameRef = useRef<HTMLIFrameElement | null>(null);
  const focusedKeyRef = useRef("");
  const streamFrameRetryRef = useRef(0);
  const automaticAttemptKeyRef = useRef(new Set<string>());
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

  const refreshProjection = useCallback(async () => {
    setLoading(true);
    try {
      await onRefresh();
      setError("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Service status unavailable");
    } finally {
      setLoading(false);
    }
  }, [onRefresh]);

  const refreshWorkspaceViewport = useCallback(() => {
    streamFrameRetryRef.current = 0;
    setFrameIssue(null);
    setStreamRefreshNonce(Date.now());
    void refreshProjection();
  }, [refreshProjection]);

  const selectWorkspaceStream = useCallback((browserId: string, option: ServiceViewStream, index: number) => {
    const candidate = projection.candidates.find((item) => item.browser.id === browserId);
    const streamKey = candidate?.streamChoiceKeys[index];
    if (!streamKey) return;
    onSelectStream(browserId, streamKey);
    streamFrameRetryRef.current = 0;
    setFrameIssue(null);
    setFocusMessage(`Selected ${viewStreamLabel(option)} for this browser.`);
    setStreamRefreshNonce(Date.now());
  }, [onSelectStream, projection.candidates]);

  useEffect(() => {
    const onSelection = () => setViewportSelection(readWorkspaceViewportSelection());
    window.addEventListener("popstate", onSelection);
    window.addEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onSelection);
    return () => {
      window.removeEventListener("popstate", onSelection);
      window.removeEventListener(DASHBOARD_WORKSPACE_SELECTION_EVENT, onSelection);
    };
  }, []);

  const selectedDaemonSession = daemonSessionFromSelection(sessions, viewportSelection?.selection);
  const selectedProjection = projection.selected;
  const browser = selectedProjection?.browser ?? null;
  const tabSelection = selectedProjection?.tabSelection
    ?? { tab: null, tabIndex: null, recoveredFromStaleSelection: false, staleSelectionId: null };
  const streamChoices = selectedProjection?.streamChoices ?? [];
  const stream = selectedProjection?.stream ?? null;
  const singleWorkspaceMode = viewportSelection?.mode === "control" ? "control" : "view";
  const sourceResolution = useMemo(
    () => resolveWorkspaceViewSources({ streams: streamChoices, selected: stream, mode: singleWorkspaceMode }),
    [singleWorkspaceMode, stream, streamChoices],
  );
  const tileStreams: WorkspaceViewportTile[] = viewportSelection?.mode === "tile"
    ? projection.tiles.map((tile) => ({
        browser: tile.browser,
        stream: tile.stream,
        frameUrl: tile.canView ? tile.frameUrl : null,
        externalUrl: tile.externalUrl,
        routeKey: tile.routeKey,
        sharedRoute: tile.sharedRoute,
        streamChoices: tile.streamChoices,
        streamChoiceKeys: tile.streamChoiceKeys,
      }))
    : [];
  const liveTileStreamCount = tileStreams.filter((tile) => Boolean(tile.frameUrl)).length;
  const streamUrl = resolveWorkspaceStreamUrl(stream);
  const snapshotStream = isCdpSnapshotStream(stream);
  const externalStreamUrl = snapshotStream ? null : resolveWorkspaceStreamUrl(stream, "external");
  const frameUrl = buildWorkspaceFrameUrl(streamUrl, streamRefreshNonce);
  const snapshotUrl = snapshotStream ? buildCdpSnapshotUrl(streamUrl, tabSelection.tab?.targetId) : null;
  const foreignCdpPort = snapshotStream
    ? selectedDaemonSession?.cdpPort ?? selectedDaemonSession?.port ?? null
    : null;
  const foreignControlTargetId = tabSelection.tab?.targetId ?? foreignTargetId;
  const canEmbed = selectedProjection?.canView ?? false;
  const canControl = selectedProjection?.canControl ?? false;
  const canRenderSelectedBrowser = selectedProjection?.authority.lifecycle.live ?? false;
  const viewportTarget = useMemo<WorkspaceViewportTarget | null>(() => {
    if (!browser && !streamUrl) return null;
    return {
      browserId: browser?.id ?? null,
      streamId: stream?.id ?? null,
      streamUrl: streamUrl ?? snapshotUrl ?? null,
      routeId: snapshotStream ? null : stream?.routeId ?? null,
      mode: viewportSelection?.mode === "tile" ? "tile" : viewportSelection?.mode === "control" ? "control" : "view",
      browserAvailable: canRenderSelectedBrowser,
    };
  }, [browser, canRenderSelectedBrowser, snapshotStream, snapshotUrl, stream, streamUrl, viewportSelection?.mode]);
  const viewportTargetToken = workspaceViewportTargetToken(viewportTarget);
  const streamPreflight: WorkspaceViewportPreflightState =
    viewportController.targetToken === viewportTargetToken
      ? viewportController.preflight
      : { status: "idle", message: "" };
  const canRenderCdpStream = canRenderSelectedBrowser && isCdpScreencastStream(stream) && Boolean(streamUrl) && streamPreflight.status === "ready";
  const canRenderSnapshotStream = canRenderSelectedBrowser && snapshotStream && Boolean(snapshotUrl) && streamPreflight.status === "ready";
  const canRenderFrame = canRenderSelectedBrowser && !isCdpScreencastStream(stream) && !snapshotStream && canEmbed && streamPreflight.status === "ready";
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
    streamReadiness: selectedProjection?.readiness ?? null,
    focusMessage,
  });

  const captureForeignCdpScreenshot = useCallback(async () => {
    if (!foreignCdpPort) return;
    setRecoveryPending("foreign-cdp-capture");
    setFocusMessage("Capturing the selected foreign browser target as PNG.");
    try {
      const captured = await fetchForeignCdpScreenshot({
        port: foreignCdpPort,
        targetId: tabSelection.tab?.targetId,
        format: "png",
      });
      const safeTitle = (captured.title || "foreign-browser")
        .replace(/[^a-z0-9]+/gi, "-")
        .replace(/^-+|-+$/g, "")
        .slice(0, 64) || "foreign-browser";
      const download = document.createElement("a");
      download.href = captured.dataUrl;
      download.download = `${safeTitle}-${Date.now()}.png`;
      document.body.appendChild(download);
      download.click();
      download.remove();
      setFocusMessage("Downloaded a PNG capture from the selected foreign browser target.");
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Foreign browser capture failed: ${err.message}` : "Foreign browser capture failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [foreignCdpPort, tabSelection.tab?.targetId]);

  const resolveForeignTarget = useCallback((targetId: string) => {
    setForeignTargetId((current) => current === targetId ? current : targetId);
  }, []);

  useEffect(() => {
    setForeignBorrow(null);
    setForeignTargetId(null);
  }, [foreignCdpPort, snapshotUrl]);

  useEffect(() => {
    if (!foreignCdpPort || !foreignControlTargetId) return;
    let disposed = false;
    void readForeignCdpControlStatus({
      port: foreignCdpPort,
      targetId: foreignControlTargetId,
    }).then((status) => {
      if (!disposed) setForeignBorrow(status);
    }).catch((err) => {
      if (!disposed) {
        setForeignBorrow(null);
        setFocusMessage(err instanceof Error ? `Could not read Borrow status: ${err.message}` : "Could not read Borrow status.");
      }
    });
    return () => {
      disposed = true;
    };
  }, [foreignCdpPort, foreignControlTargetId]);

  useEffect(() => {
    if (!foreignBorrow?.active || !foreignBorrow.expiresAt) return;
    const expiresAt = new Date(foreignBorrow.expiresAt).getTime();
    const delay = Math.max(0, expiresAt - Date.now());
    const timer = window.setTimeout(() => {
      setForeignBorrow(null);
      setFocusMessage("Borrow control expired. The foreign browser remains running and non-owned.");
    }, Math.min(delay + 50, 900_050));
    return () => window.clearTimeout(timer);
  }, [foreignBorrow?.active, foreignBorrow?.expiresAt]);

  const borrowForeignControl = useCallback(async () => {
    if (!foreignCdpPort || !foreignControlTargetId) return;
    setForeignBorrowPending(true);
    setFocusMessage("Requesting five minutes of foreign-browser input control.");
    try {
      const status = await borrowForeignCdpControl({
        port: foreignCdpPort,
        targetId: foreignControlTargetId,
        reason: "Dashboard operator interactive diagnosis",
        ttlSeconds: 300,
      });
      setForeignBorrow(status);
      setFocusMessage(`Borrow control is active until ${status.expiresAt ? new Date(status.expiresAt).toLocaleTimeString() : "grant expiry"}. Browser lifecycle ownership did not change.`);
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Borrow control failed: ${err.message}` : "Borrow control failed.");
    } finally {
      setForeignBorrowPending(false);
    }
  }, [foreignCdpPort, foreignControlTargetId]);

  const releaseForeignControl = useCallback(async () => {
    if (!foreignCdpPort || !foreignControlTargetId || !foreignBorrow?.grantId) return;
    setForeignBorrowPending(true);
    try {
      const status = await releaseForeignCdpControl({
        port: foreignCdpPort,
        targetId: foreignControlTargetId,
        grantId: foreignBorrow.grantId,
      });
      setForeignBorrow(status);
      setFocusMessage("Borrow control released. The foreign browser remains running and non-owned.");
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Release control failed: ${err.message}` : "Release control failed.");
    } finally {
      setForeignBorrowPending(false);
    }
  }, [foreignBorrow?.grantId, foreignCdpPort, foreignControlTargetId]);

  const sendForeignInput = useCallback((input: ForeignCdpInput) => {
    if (!foreignBorrow?.active || !foreignBorrow.grantId || !foreignCdpPort || !foreignControlTargetId) return;
    void dispatchForeignCdpInput({
      port: foreignCdpPort,
      targetId: foreignControlTargetId,
      grantId: foreignBorrow.grantId,
      input,
    }).catch((err) => {
      setFocusMessage(err instanceof Error ? `Foreign browser input failed: ${err.message}` : "Foreign browser input failed.");
      if (err instanceof Error && /No active Borrow|does not authorize/i.test(err.message)) {
        setForeignBorrow(null);
      }
    });
  }, [foreignBorrow?.active, foreignBorrow?.grantId, foreignCdpPort, foreignControlTargetId]);

  useEffect(() => {
    if (!viewportSelection || !tabSelection.recoveredFromStaleSelection || !tabSelection.tab?.id) return;
    if (viewportSelection.selection.tabId === tabSelection.tab.id) return;
    const nextSelection = {
      ...viewportSelection.selection,
      tabId: tabSelection.tab.id,
    };
    const writtenSelection = writeDashboardWorkspaceUrlSelection(nextSelection, "replace");
    setViewportSelection({ mode: viewportSelection.mode, selection: writtenSelection });
    setFocusMessage(tabSelection.staleSelectionId
      ? `Recovered stale selected tab identity ${tabSelection.staleSelectionId}; using current live target ${tabSelection.tab.id}.`
      : `Recovered stale selected tab identity; using current live target ${tabSelection.tab.id}.`);
  }, [tabSelection.recoveredFromStaleSelection, tabSelection.staleSelectionId, tabSelection.tab?.id, viewportSelection]);

  useEffect(() => {
    streamFrameRetryRef.current = 0;
    setFrameIssue(null);
  }, [streamUrl]);

  useEffect(() => {
    dispatchViewportController({ type: "target_changed", target: viewportTarget });
  }, [viewportTarget, viewportTargetToken]);

  useEffect(() => {
    if (!frameUrl || !canEmbed || !viewportTargetToken) {
      return;
    }

    let disposed = false;
    const preflightStreamUrl = frameUrl;
    const preflightTargetToken = viewportTargetToken;
    dispatchViewportController({
      type: "preflight_started",
      targetToken: preflightTargetToken,
      message: "Checking stream access.",
    });

    async function checkStreamAccess() {
      try {
        const resolved = new URL(preflightStreamUrl, window.location.href);
        if (resolved.origin !== window.location.origin) {
          dispatchViewportController({ type: "preflight_succeeded", targetToken: preflightTargetToken });
          return;
        }
        const response = await fetch(resolved.toString(), {
          cache: "no-store",
          credentials: "include",
          redirect: "follow",
        });
        if (disposed) return;
        if (responseLooksLikeDashboardLogin(response, resolved)) {
          dispatchViewportController({
            type: "preflight_failed",
            targetToken: preflightTargetToken,
            status: "login-required",
            message: "The remote stream needs a fresh dashboard sign-in before it can be embedded.",
          });
          return;
        }
        if (response.status === 401 || response.status === 403) {
          dispatchViewportController({
            type: "preflight_failed",
            targetToken: preflightTargetToken,
            status: "login-required",
            message: "The remote stream rejected the current dashboard session.",
          });
          return;
        }
        if (!response.ok) {
          dispatchViewportController({
            type: "preflight_failed",
            targetToken: preflightTargetToken,
            status: "error",
            message: `The remote stream returned HTTP ${response.status}.`,
          });
          return;
        }
        dispatchViewportController({ type: "preflight_succeeded", targetToken: preflightTargetToken });
      } catch (err) {
        if (disposed) return;
        dispatchViewportController({
          type: "preflight_failed",
          targetToken: preflightTargetToken,
          status: "error",
          message: err instanceof Error ? err.message : "The remote stream could not be reached.",
        });
      }
    }

    void checkStreamAccess();
    return () => {
      disposed = true;
    };
  }, [canEmbed, frameUrl, viewportTargetToken]);

  const handleFrameLoadIssue = useCallback((failure: WorkspaceFrameFailure) => {
    if (failure === "login-required") {
      setFrameIssue(null);
      if (viewportTargetToken) dispatchViewportController({
        type: "preflight_failed",
        targetToken: viewportTargetToken,
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
    if (viewportTargetToken) dispatchViewportController({
      type: "preflight_failed",
      targetToken: viewportTargetToken,
      status: "error",
      message: failure === "fatal-error"
        ? "Guacamole reported that the remote desktop connection closed."
        : "The embedded remote stream failed to load. Refresh the workspace viewport or open the stream externally.",
    });
  }, [viewportTargetToken]);

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
    if (!viewportSelection || viewportSelection.mode !== "control" || snapshotStream) return;
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
  }, [activePort, activeSessionName, browser, canControl, snapshotStream, stream, streamUrl, tabSelection.recoveredFromStaleSelection, tabSelection.tab?.id, tabSelection.tabIndex, viewportSelection]);

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
    const json = await readWorkspaceApiResponse<ApiResponse<unknown>>(resp);
    if (!resp.ok || !json.success) {
      throw new Error(workspaceRecoveryFailureMessage(json, action));
    }
    return json;
  }, [activePort, activeSessionName]);

  const workspaceViewerRoute = useMemo(
    () => snapshotStream ? null : selectWorkspaceViewerRoute(streamChoices, stream),
    [snapshotStream, stream, streamChoices],
  );
  const workspaceRouteId = workspaceViewerRoute?.routeId?.trim() || null;
  const workspaceViewerLeaseIds = useMemo(() => Array.from(new Set([
    ...(workspaceViewerRoute?.viewerLeaseIds ?? []),
    ...(workspaceViewerRoute?.controllerLeaseId ? [workspaceViewerRoute.controllerLeaseId] : []),
  ].filter((id): id is string => Boolean(id?.trim())))), [workspaceViewerRoute?.controllerLeaseId, workspaceViewerRoute?.viewerLeaseIds]);
  const workspaceViewerId = activeSessionName || "operator";
  const routeRecoveryAction = selectedProjection?.readiness.recoveryAction ?? null;
  const connectionReadinessGeneration = workspaceConnectionReadinessGeneration(
    workspaceViewerRoute ?? stream,
    routeRecoveryAction,
  );
  const connectionPlan = planAutomaticWorkspaceConnection({
    browserId: browser?.id ?? "unselected",
    browserLive: selectedProjection?.authority.lifecycle.live ?? false,
    mode: singleWorkspaceMode,
    sourceResolution,
    currentStream: stream,
    routeRecoveryAction,
    readinessGeneration: connectionReadinessGeneration,
    viewerRoute: workspaceViewerRoute,
    viewerRouteReady: workspaceViewerRouteIsAttached(workspaceViewerRoute),
    viewerLeaseIds: workspaceViewerLeaseIds,
    attemptedActionKeys: automaticAttemptKeys,
  });
  const connectionInProgress = Boolean(recoveryPending)
    || loading
    || connectionPlan.status === "connecting"
    || streamPreflight.status === "checking";
  const connectionStatusLabel = connectionInProgress
    ? "Connecting…"
    : viewportReadiness.status === "ready"
      ? "Ready"
      : "Action required";

  const recoverWorkspaceBrowser = useCallback(async (
    targetBrowser: WorkspaceViewportBrowser | null,
    targetStream: ServiceViewStream | null,
  ) => {
    if (!targetBrowser) return;
    const displayAllocationId = targetStream?.displayAllocationId || targetBrowser.displayAllocationId;
    const recoveryAction: ServiceRequestAction = projection.candidates
      .find((candidate) => candidate.browser.id === targetBrowser.id)
      ?.readiness.recoveryAction ?? "service_remote_view_browser_reattach";
    const switchingRoute = recoveryAction === "service_remote_view_route_switch";
    setRecoveryPending("route-refresh");
    setFocusMessage(switchingRoute
      ? "Switching the retained remote browser to an available route."
      : "Reattaching the retained remote browser route.");
    try {
      await postWorkspaceRecoveryRequest(recoveryAction, switchingRoute ? "workspace-viewport-route-switch" : "workspace-viewport-browser-reattach", {
        browserId: targetBrowser.id,
        ...(daemonSessionNameForBrowser(targetBrowser, viewportSelection?.selection) ? {
          sessionName: daemonSessionNameForBrowser(targetBrowser, viewportSelection?.selection),
        } : {}),
        ...(targetStream?.id ? { streamId: targetStream.id } : {}),
        ...(displayAllocationId ? { displayAllocationId } : {}),
        ...(!switchingRoute && targetStream?.routeId ? { routeId: targetStream.routeId } : {}),
        ...(targetStream?.provider ? { provider: targetStream.provider } : {}),
        ...(targetStream?.providerMode ? { providerMode: targetStream.providerMode } : {}),
        ...(targetStream?.frameUrl ? { frameUrl: targetStream.frameUrl } : {}),
        ...(targetStream?.externalUrl ? { externalUrl: targetStream.externalUrl } : {}),
        ...(targetStream?.connectionId ? { connectionId: targetStream.connectionId } : {}),
        ...(targetStream?.connectionName ? { connectionName: targetStream.connectionName } : {}),
      });
      streamFrameRetryRef.current = 0;
      setFrameIssue(null);
      setStreamRefreshNonce(Date.now());
      setFocusMessage(switchingRoute
        ? "Switched the retained remote browser route."
        : "Reattached the retained remote browser route.");
      void refreshProjection();
    } catch (err) {
      setFocusMessage(err instanceof Error
        ? `Browser route recovery failed: ${err.message}`
        : "Browser route recovery failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [postWorkspaceRecoveryRequest, projection.candidates, refreshProjection, viewportSelection?.selection]);

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
      void refreshProjection();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Viewer reconnect failed: ${err.message}` : "Viewer reconnect failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [browser, postWorkspaceRecoveryRequest, refreshProjection, workspaceRouteId, workspaceViewerId]);

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
      void refreshProjection();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Controller takeover failed: ${err.message}` : "Controller takeover failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [browser, postWorkspaceRecoveryRequest, refreshProjection, workspaceRouteId, workspaceViewerId]);

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
      void refreshProjection();
    } catch (err) {
      setFocusMessage(err instanceof Error ? `Viewer release failed: ${err.message}` : "Viewer release failed.");
    } finally {
      setRecoveryPending(null);
    }
  }, [postWorkspaceRecoveryRequest, refreshProjection, workspaceViewerLeaseIds]);

  const retryWorkspaceConnection = useCallback(() => {
    automaticAttemptKeyRef.current.clear();
    setAutomaticAttemptKeys([]);
    setConnectionRetryNonce((current) => current + 1);
    setFocusMessage("Retrying the browser connection.");
    refreshWorkspaceViewport();
  }, [refreshWorkspaceViewport]);

  useEffect(() => {
    automaticAttemptKeyRef.current.clear();
    setAutomaticAttemptKeys([]);
  }, [browser?.id]);

  useEffect(() => {
    if (viewportSelection?.mode === "tile" || !browser || loading || recoveryPending || takeoverPending) return;
    const action = connectionPlan.action;
    if (!action || automaticAttemptKeyRef.current.has(action.attemptKey)) return;
    automaticAttemptKeyRef.current.add(action.attemptKey);
    setAutomaticAttemptKeys((current) => current.includes(action.attemptKey)
      ? current
      : [...current, action.attemptKey]);

    if (action.kind === "select-source") {
      const index = streamChoices.indexOf(action.source.stream);
      if (index >= 0) selectWorkspaceStream(browser.id, action.source.stream, index);
      return;
    }
    if (action.kind === "recover-route") {
      void recoverWorkspaceBrowser(browser, workspaceViewerRoute ?? stream);
      return;
    }
    void reconnectWorkspaceViewer();
  }, [
    browser,
    connectionPlan.action,
    connectionRetryNonce,
    loading,
    reconnectWorkspaceViewer,
    recoverWorkspaceBrowser,
    recoveryPending,
    selectWorkspaceStream,
    stream,
    streamChoices,
    takeoverPending,
    viewportSelection?.mode,
    workspaceViewerRoute,
  ]);

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
      void refreshProjection();
      return true;
    } catch (err) {
      setFocusMessage(err instanceof Error
        ? `Viewer takeover request failed: ${err.message}`
        : "Viewer takeover request failed; refresh the workspace viewport or inspect readiness.");
      return false;
    } finally {
      setTakeoverPending(false);
    }
  }, [activePort, activeSessionName, browser, frameIssue?.kind, refreshProjection, stream, tabSelection.tab?.targetId, tabSelection.tabIndex, viewportSelection]);

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
        data-ux-state={liveTileStreamCount > 0 ? "connected" : "missing_stream"}
        data-readiness-status={liveTileStreamCount > 0 ? "ready" : "missing"}
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
                {tileStreams.length} workspace{tileStreams.length === 1 ? "" : "s"} / {liveTileStreamCount} live stream{liveTileStreamCount === 1 ? "" : "s"}
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
                void refreshProjection();
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
              const tileFrameUrl = tile.frameUrl ? buildWorkspaceFrameUrl(tile.frameUrl, nonce) : null;
              const tileChoices = tile.streamChoices;
              const tileSourceResolution = resolveWorkspaceViewSources({
                streams: tileChoices,
                selected: tile.stream,
                mode: "view",
              });
              return (
                <article
                  key={tile.browser.id}
                  className={cn("workspace-remote-viewport-tile-card", tile.sharedRoute && "workspace-remote-viewport-tile-card-shared")}
                >
                  <header className="workspace-remote-viewport-tile-header">
                    <div className="min-w-0">
                      <h3>{workspaceViewportTitle(tile.browser)}</h3>
                      <p>{tile.stream ? viewStreamRouteSummary(tile.stream) : "No usable view stream reported."}</p>
                    </div>
                    <div className="workspace-remote-viewport-tile-actions">
                      {tile.sharedRoute && (
                        <Badge variant="destructive" className="workspace-remote-viewport-badge">
                          <span className="workspace-remote-viewport-badge-text">shared route</span>
                        </Badge>
                      )}
                      {tileFrameUrl && (
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
                      )}
                      {tile.externalUrl && (
                        <Button size="icon" variant="outline" asChild>
                          <a href={tile.externalUrl} target="_blank" rel="noreferrer" aria-label={`Open ${tile.browser.id} externally`}>
                            <ExternalLink className="size-3.5" />
                          </a>
                        </Button>
                      )}
                    </div>
                  </header>
                  <div className="workspace-remote-viewport-stream-picker">
                    <WorkspaceSourceMenu
                      sources={tileSourceResolution.choices}
                      selected={tileSourceResolution.selected}
                      onSelect={(source) => {
                        const index = tileChoices.indexOf(source.stream);
                        if (index >= 0) selectWorkspaceStream(tile.browser.id, source.stream, index);
                      }}
                    />
                  </div>
                  {tile.sharedRoute && (
                    <p className="workspace-remote-viewport-notice workspace-remote-viewport-notice-bad">
                      <AlertTriangle className="size-3.5" />
                      This route is shared by multiple workspaces; simultaneous viewing may fall back to provider takeover behavior.
                    </p>
                  )}
                  <div className="workspace-remote-viewport-tile-stage">
                    {tile.stream && tileFrameUrl ? (
                      <iframe
                        key={`${tile.browser.id}:${tileFrameUrl}`}
                        title={`${viewStreamLabel(tile.stream)} ${tile.browser.id}`}
                        src={tileFrameUrl}
                        className="workspace-remote-viewport-frame"
                        allow="clipboard-read; clipboard-write; fullscreen; pointer-lock"
                        allowFullScreen
                      />
                    ) : (
                      <div className="workspace-remote-viewport-empty workspace-remote-viewport-tile-empty">
                        <PlugZap className="size-6" />
                        <h3>Connecting browser view</h3>
                        <p>The browser is still running. Retry its presentation connection without restarting the browser or profile.</p>
                        <div className="workspace-remote-viewport-empty-actions">
                          <Button
                            size="sm"
                            variant="default"
                            disabled={Boolean(recoveryPending)}
                            onClick={() => {
                              void recoverWorkspaceBrowser(tile.browser, tile.stream);
                            }}
                          >
                            <PlugZap className={cn("size-3.5", recoveryPending === "route-refresh" && "animate-spin")} />
                            Retry connection
                          </Button>
                        </div>
                      </div>
                    )}
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
          {browser && (
            <WorkspaceSourceMenu
              sources={sourceResolution.choices}
              selected={sourceResolution.selected}
              onSelect={(source) => {
                const index = streamChoices.indexOf(source.stream);
                if (index >= 0) selectWorkspaceStream(browser.id, source.stream, index);
              }}
            />
          )}
          <Badge
            variant={viewportReadiness.status === "ready" ? "secondary" : viewportReadiness.status === "blocked" ? "destructive" : "outline"}
            className="workspace-remote-viewport-badge"
            aria-live="polite"
          >
            <span className="workspace-remote-viewport-badge-text">{connectionStatusLabel}</span>
          </Badge>
          {snapshotStream && foreignCdpPort && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              aria-label="Capture foreign browser screenshot"
              title="Download the selected foreign browser target as a PNG"
              disabled={Boolean(recoveryPending)}
              onClick={() => {
                void captureForeignCdpScreenshot();
              }}
            >
              <Download className="size-3.5" />
              Capture PNG
            </Button>
          )}
          {snapshotStream && foreignCdpPort && !foreignBorrow?.active && (
            <Button
              type="button"
              size="sm"
              variant="default"
              aria-label="Borrow foreign browser control for five minutes"
              title="Temporarily enable pointer, keyboard, and wheel input. Close and Kill remain unavailable."
              disabled={foreignBorrowPending || !foreignControlTargetId}
              onClick={() => {
                void borrowForeignControl();
              }}
            >
              <MousePointer2 className="size-3.5" />
              Borrow control
            </Button>
          )}
          {snapshotStream && foreignCdpPort && foreignBorrow?.active && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              aria-label="Release foreign browser control"
              title="Stop sending input while leaving the foreign browser running"
              disabled={foreignBorrowPending}
              onClick={() => {
                void releaseForeignControl();
              }}
            >
              <Unplug className="size-3.5" />
              Release control
            </Button>
          )}
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button
                type="button"
                size="sm"
                variant="outline"
                aria-label="Advanced connection controls"
              >
                <MoreHorizontal className="size-3.5" />
                Advanced
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="min-w-56">
              <DropdownMenuLabel>Advanced connection controls</DropdownMenuLabel>
              <DropdownMenuItem onSelect={refreshWorkspaceViewport}>
                <RefreshCw className="size-3.5" />
                Reload view
              </DropdownMenuItem>
              {stream && !snapshotStream && (
                <DropdownMenuItem
                  disabled={Boolean(recoveryPending)}
                  onSelect={() => { void recoverWorkspaceBrowser(browser, stream); }}
                >
                  <PlugZap className="size-3.5" />
                  Reattach desktop route
                </DropdownMenuItem>
              )}
              {workspaceRouteId && (
                <DropdownMenuItem
                  disabled={Boolean(recoveryPending)}
                  onSelect={() => { void reconnectWorkspaceViewer(); }}
                >
                  <RefreshCw className="size-3.5" />
                  Reconnect viewer
                </DropdownMenuItem>
              )}
              {workspaceRouteId && (
                <DropdownMenuItem
                  disabled={Boolean(recoveryPending)}
                  onSelect={() => { void takeoverWorkspaceController(); }}
                >
                  <MousePointer2 className="size-3.5" />
                  Take control
                </DropdownMenuItem>
              )}
              {workspaceViewerLeaseIds.length > 0 && (
                <DropdownMenuItem
                  disabled={Boolean(recoveryPending)}
                  onSelect={() => { void releaseWorkspaceViewers(); }}
                >
                  <Unplug className="size-3.5" />
                  Release viewer
                </DropdownMenuItem>
              )}
              {stream?.provider === "rdp_gateway" && (
                <>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem disabled={!canRenderFrame} onSelect={openInteractionSettings}>
                    <Settings2 className="size-3.5" />
                    Mouse and keyboard settings
                  </DropdownMenuItem>
                </>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
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
        <div className="workspace-remote-viewport-notices" aria-live="polite">
          {error && (
            <p className="workspace-remote-viewport-notice workspace-remote-viewport-notice-bad">
              <AlertTriangle className="size-3.5" />
              {error}
            </p>
          )}
          {viewportReadiness.status !== "ready" && (
            <div className={cn(
              "workspace-remote-viewport-notice",
              !connectionInProgress && (viewportReadiness.status === "blocked" || viewportReadiness.nextAction === "take_over")
                ? "workspace-remote-viewport-notice-bad"
                : undefined,
            )}>
              {connectionInProgress
                ? <RefreshCw className="size-3.5 animate-spin" />
                : <AlertTriangle className="size-3.5" />}
              <span className="workspace-remote-viewport-notice-text">
                {connectionInProgress ? (
                  <><strong>{connectionPlan.message}.</strong> The browser process and profile remain active.</>
                ) : (
                  <><strong>{viewportReadiness.title}.</strong> {viewportReadiness.recoveryCopy}</>
                )}
              </span>
              {!connectionInProgress && viewportReadiness.nextAction === "take_over" && (
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
                  <MousePointer2 className={cn("size-3.5", takeoverPending && "animate-spin")} />
                  Take control
                </Button>
              )}
              {!connectionInProgress && viewportReadiness.nextAction === "sign_in_again" && (
                <Button size="sm" variant="default" className="workspace-remote-viewport-notice-action" asChild>
                  <a href={dashboardLoginPath()}>
                    <LogIn className="size-3.5" />
                    Sign in again
                  </a>
                </Button>
              )}
              {!connectionInProgress && viewportReadiness.nextAction !== "take_over" && viewportReadiness.nextAction !== "sign_in_again" && (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="workspace-remote-viewport-notice-action"
                  onClick={retryWorkspaceConnection}
                >
                  <RefreshCw className="size-3.5" />
                  Retry connection
                </Button>
              )}
            </div>
          )}
          {focusMessage && !connectionInProgress && (
            <p className="workspace-remote-viewport-notice">
              <MousePointer2 className="size-3.5" />
              {focusMessage}
            </p>
          )}
          {stream && !canControl && !(snapshotStream && foreignBorrow?.active) && viewportSelection.mode === "control" && (
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
        ) : stream && canRenderSnapshotStream && snapshotUrl ? (
          <WorkspaceCdpSnapshotViewer
            key={`${snapshotUrl}:${streamRefreshNonce}`}
            snapshotUrl={snapshotUrl}
            refreshNonce={streamRefreshNonce}
            canControl={foreignBorrow?.active === true}
            onInput={sendForeignInput}
            onTargetResolved={resolveForeignTarget}
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
