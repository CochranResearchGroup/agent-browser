export type ServiceRouteDescriptor = {
  localEmbedUrl?: string | null;
  publicOperatorUrl?: string | null;
  dashboardEmbedUrl?: string | null;
  externalUrl?: string | null;
  healthUrl?: string | null;
};

export type ServiceViewStream = {
  id?: string;
  provider?: string;
  controlInput?: string | null;
  url?: string | null;
  frameUrl?: string | null;
  externalUrl?: string | null;
  localEmbedUrl?: string | null;
  publicOperatorUrl?: string | null;
  dashboardEmbedUrl?: string | null;
  routeDescriptor?: ServiceRouteDescriptor | null;
  routeId?: string | null;
  displayAllocationId?: string | null;
  connectionId?: string | null;
  connectionName?: string | null;
  routeSource?: string | null;
  providerMode?: string | null;
  viewerLeaseIds?: string[];
  controllerLeaseId?: string | null;
  readOnly?: boolean;
  readiness?: unknown;
  remoteReadiness?: unknown;
  attachability?: unknown;
  displayContent?: unknown;
};

export type ServiceViewStreamReadinessEvidence = {
  state: string | null;
  reason: string | null;
  blocked: boolean;
};

const EMBEDDABLE_VIEW_STREAM_PROVIDERS = new Set([
  "cdp_screencast",
  "cdp_snapshot",
  "external_url",
  "novnc",
  "rdp_gateway",
  "virtual_display_webrtc",
  "chrome_tab_webrtc",
]);

const BLOCKING_VIEW_STREAM_READINESS_STATES = new Set([
  "unreachable",
  "auth_expired",
  "stale_target",
  "invalid_payload",
  "unsupported_provider",
  "disabled",
  "unavailable",
  "terminal_only",
  "terminal_only_route",
  "idle_display",
  "display_idle",
  "no_browser_window",
  "route_bound_terminal_only",
  "route_bound_display_idle",
  "route_bound_browser_not_visible",
  "route_bound_proof_missing",
  "public_operator_not_checked",
  "public_operator_unavailable",
  "invalid_operator_route",
  "dashboard_unavailable",
  "proxy_failed",
  "timed_out",
  "wrong_tab",
  "guacamole_route_unavailable",
  "cdp_target_unavailable",
  "stale_route_record",
]);

export function viewStreamLabel(stream: ServiceViewStream): string {
  return stream.provider?.replaceAll("_", " ") || "view stream";
}

export function controlInputLabel(stream: ServiceViewStream): string {
  return stream.controlInput?.replaceAll("_", " ") || (stream.readOnly ? "view only" : "interactive");
}

export function canEmbedViewStream(stream: ServiceViewStream): boolean {
  return Boolean(viewStreamFrameUrl(stream) && EMBEDDABLE_VIEW_STREAM_PROVIDERS.has(stream.provider ?? ""));
}

export function viewStreamUrl(stream?: ServiceViewStream | null): string | null {
  return stream?.url || stream?.frameUrl || stream?.externalUrl || null;
}

export function viewStreamFrameUrl(stream?: ServiceViewStream | null): string | null {
  return stream?.frameUrl || stream?.url || null;
}

export function viewStreamExternalUrl(stream?: ServiceViewStream | null): string | null {
  return stream?.externalUrl || stream?.frameUrl || stream?.url || null;
}

export function viewStreamDashboardFrameUrl(stream?: ServiceViewStream | null, dashboardHref?: string | null): string | null {
  const frameUrl = viewStreamFrameUrl(stream);
  const localEmbedUrl = stream?.localEmbedUrl || stream?.routeDescriptor?.localEmbedUrl || frameUrl;
  const publicUrl = (
    stream?.publicOperatorUrl
    || stream?.routeDescriptor?.publicOperatorUrl
    || stream?.externalUrl
    || stream?.routeDescriptor?.externalUrl
    || null
  );
  const dashboardEmbedUrl = stream?.dashboardEmbedUrl || stream?.routeDescriptor?.dashboardEmbedUrl || null;
  if (!dashboardHref) return dashboardEmbedUrl || frameUrl || publicUrl || null;

  const dashboardIsLocal = isLoopbackUrl(dashboardHref);
  if (dashboardIsLocal) return localEmbedUrl || dashboardEmbedUrl || publicUrl || null;

  if (dashboardEmbedUrl && !isLoopbackUrl(dashboardEmbedUrl, dashboardHref)) return dashboardEmbedUrl;
  if (frameUrl && isLoopbackUrl(frameUrl, dashboardHref) && publicUrl) return publicUrl;
  return frameUrl || publicUrl || dashboardEmbedUrl || null;
}

export function canControlViewStream(stream: ServiceViewStream): boolean {
  return stream.readOnly !== true && Boolean(stream.controlInput);
}

export function canOpenViewStream(stream?: ServiceViewStream | null): boolean {
  return Boolean(stream && canEmbedViewStream(stream) && !hasBlockingViewStreamReadiness(stream));
}

export function canOpenControlViewStream(stream?: ServiceViewStream | null): boolean {
  return Boolean(stream && canEmbedViewStream(stream) && canControlViewStream(stream) && !hasBlockingViewStreamReadiness(stream));
}

export function viewStreamOpenTitle(stream?: ServiceViewStream | null): string {
  if (!stream) return "No service-owned view stream is registered for this browser.";
  if (hasBlockingViewStreamReadiness(stream)) {
    const reason = viewStreamReadinessEvidence(stream).reason;
    return reason
      ? `${viewStreamLabel(stream)} is unavailable: ${reason}.`
      : `${viewStreamLabel(stream)} is unavailable: ${viewStreamReadinessLabel(stream)}.`;
  }
  if (!canEmbedViewStream(stream)) {
    const reason = readinessReason(stream.remoteReadiness ?? stream.readiness);
    if (!stream.url) {
      return reason
        ? `${viewStreamLabel(stream)} is unavailable: ${reason}.`
        : "This stream has no embeddable URL yet.";
    }
    return "This stream provider is not embeddable in the dashboard.";
  }
  return `Open ${viewStreamLabel(stream)} in the dashboard.`;
}

export function viewStreamControlTitle(stream?: ServiceViewStream | null): string {
  if (!stream) return "No service-owned view stream is registered for this browser.";
  if (hasBlockingViewStreamReadiness(stream)) return viewStreamOpenTitle(stream);
  if (!canEmbedViewStream(stream)) return viewStreamOpenTitle(stream);
  if (!canControlViewStream(stream)) return "The service marked this stream as view-only or did not report a control input provider.";
  return `Focus the browser and open ${controlInputLabel(stream)} control.`;
}

export function viewStreamCapabilityLabel(stream: ServiceViewStream): string {
  const view = viewStreamLabel(stream);
  const control = controlInputLabel(stream);
  return `${view} / ${control}`;
}

export function viewStreamRouteLabel(stream?: ServiceViewStream | null): string {
  if (!stream) return "no route";
  return stream.routeId || stream.connectionName || stream.connectionId || stream.displayAllocationId || "unrouted";
}

export function viewStreamLeaseLabel(stream?: ServiceViewStream | null): string {
  if (!stream) return "no viewers";
  const viewerCount = stream.viewerLeaseIds?.length ?? 0;
  const viewerLabel = `${viewerCount} viewer${viewerCount === 1 ? "" : "s"}`;
  return stream.controllerLeaseId ? `${viewerLabel}, controller leased` : viewerLabel;
}

export function viewStreamReadinessLabel(stream?: ServiceViewStream | null): string {
  if (!stream) return "readiness unknown";
  const state = viewStreamReadinessEvidence(stream).state;
  if (state) return state.replaceAll("_", " ");
  if (canOpenViewStream(stream)) return "ready";
  return "readiness unknown";
}

/** Returns the source-reported readiness evidence without interpreting browser lifecycle authority. */
export function viewStreamReadinessEvidence(stream?: ServiceViewStream | null): ServiceViewStreamReadinessEvidence {
  if (!stream) return { state: null, reason: null, blocked: false };
  const readiness = stream.remoteReadiness ?? stream.readiness;
  const readinessState = normalizeReadinessState(viewStreamReadinessState(readiness));
  const displayContent = displayContentValue(stream);
  const displayState = normalizeReadinessState(viewStreamReadinessState(displayContent));
  const blockingState = [readinessState, displayState]
    .find((state): state is string => Boolean(state && BLOCKING_VIEW_STREAM_READINESS_STATES.has(state)))
    ?? null;
  const reason = readinessReason(blockingState === displayState ? displayContent : readiness)
    ?? readinessReason(readiness)
    ?? readinessReason(displayContent);
  return {
    state: blockingState ?? readinessState ?? displayState,
    reason,
    blocked: Boolean(blockingState),
  };
}

export function viewStreamRouteSummary(stream?: ServiceViewStream | null): string {
  if (!stream) return "no route";
  return [
    viewStreamRouteLabel(stream),
    stream.displayAllocationId ? `display ${stream.displayAllocationId}` : null,
    stream.providerMode?.replaceAll("_", " ") ?? null,
    viewStreamLeaseLabel(stream),
    viewStreamReadinessLabel(stream),
  ].filter(Boolean).join(" / ");
}

export function viewStreamReadinessState(readiness: unknown): string | null {
  if (!readiness) return null;
  if (typeof readiness === "string") return readiness.trim() || null;
  if (typeof readiness !== "object") return null;
  if (Array.isArray(readiness)) {
    const failed = readiness.find((item) => viewStreamReadinessState(item) && viewStreamReadinessState(item) !== "ready");
    return failed ? viewStreamReadinessState(failed) : null;
  }
  const record = readiness as Record<string, unknown>;
  for (const key of ["state", "status", "readiness", "lastProviderEvent"]) {
    const value = record[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }
  const operatorVisible = viewStreamReadinessState(record.operatorVisible);
  if (operatorVisible && normalizeReadinessState(operatorVisible) !== "ready") return operatorVisible;
  const components = record.components ?? record.checks ?? record.results;
  if (Array.isArray(components)) {
    const failed = components.find((item) => viewStreamReadinessState(item) && viewStreamReadinessState(item) !== "ready");
    return failed ? viewStreamReadinessState(failed) : null;
  }
  if (components && typeof components === "object") {
    for (const component of Object.values(components as Record<string, unknown>)) {
      const state = viewStreamReadinessState(component);
      if (state && normalizeReadinessState(state) !== "ready") return state;
    }
  }
  return null;
}

function hasBlockingViewStreamReadiness(stream: ServiceViewStream): boolean {
  return viewStreamReadinessEvidence(stream).blocked;
}

function readinessReason(readiness: unknown): string | null {
  if (!readiness || typeof readiness !== "object") return null;
  if (Array.isArray(readiness)) {
    for (const item of readiness) {
      const reason = readinessReason(item);
      if (reason) return reason;
    }
    return null;
  }
  const record = readiness as Record<string, unknown>;
  const reason = record.reason ?? record.message ?? record.lastProviderEvent;
  if (typeof reason === "string" && reason.trim()) return reason.trim().replaceAll("_", " ");
  for (const nested of [record.operatorVisible, record.components, record.checks, record.results]) {
    if (!nested || typeof nested !== "object") continue;
    const values = Array.isArray(nested) ? nested : Object.values(nested as Record<string, unknown>);
    for (const value of values) {
      const nestedReason = readinessReason(value);
      if (nestedReason) return nestedReason;
    }
  }
  return null;
}

function displayContentValue(stream: ServiceViewStream): unknown {
  return stream.displayContent
    ?? recordValue(stream.remoteReadiness, "displayContent")
    ?? recordValue(stream.readiness, "displayContent")
    ?? null;
}

function recordValue(value: unknown, key: string): unknown {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return (value as Record<string, unknown>)[key];
}

function normalizeReadinessState(value: string | null): string | null {
  const normalized = value?.trim().toLowerCase().replaceAll("-", "_");
  return normalized || null;
}

function isLoopbackUrl(value: string | null | undefined, base?: string | null): boolean {
  if (!value) return false;
  try {
    const url = base ? new URL(value, base) : new URL(value);
    const hostname = url.hostname.toLowerCase();
    return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1" || hostname === "[::1]";
  } catch {
    return false;
  }
}
