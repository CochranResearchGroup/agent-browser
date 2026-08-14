import {
  canOpenControlViewStream,
  canOpenViewStream,
  viewStreamRouteSummary,
  type ServiceViewStream,
} from "./service-view-streams.ts";

export type WorkspaceViewMode = "view" | "control";

export type WorkspaceViewSource = {
  stream: ServiceViewStream;
  label: string;
  detail: string;
  identity: string;
};

export type WorkspaceViewSourceResolution = {
  selected: WorkspaceViewSource | null;
  choices: readonly WorkspaceViewSource[];
  selectionReason: "selected-ready" | "automatic-ready-fallback" | "selected-unavailable" | "unavailable";
};

export type WorkspaceRouteRecoveryAction =
  | "service_remote_view_browser_reattach"
  | "service_remote_view_route_switch";

export type WorkspaceConnectionAutomaticAction =
  | {
      kind: "select-source";
      source: WorkspaceViewSource;
      attemptKey: string;
    }
  | {
      kind: "recover-route";
      serviceAction: WorkspaceRouteRecoveryAction;
      attemptKey: string;
    }
  | {
      kind: "request-viewer-lease";
      routeId: string;
      attemptKey: string;
    };

export type WorkspaceConnectionPlan = {
  status: "connecting" | "ready" | "action-required" | "unavailable";
  action: WorkspaceConnectionAutomaticAction | null;
  message: string;
};

/**
 * Resolves one operator-facing view source while keeping provider and route
 * mechanics behind a compact semantic interface.
 */
export function resolveWorkspaceViewSources({
  streams,
  selected,
  mode,
}: {
  streams: readonly ServiceViewStream[];
  selected?: ServiceViewStream | null;
  mode: WorkspaceViewMode;
}): WorkspaceViewSourceResolution {
  const choices = uniqueWorkspaceViewSources(streams);
  const selectedSource = choices.find((choice) => choice.stream === selected)
    ?? (selected ? sourceFor(selected) : null);
  if (selectedSource && sourceIsUsable(selectedSource.stream, mode)) {
    return { selected: selectedSource, choices, selectionReason: "selected-ready" };
  }

  const fallback = choices
    .filter((choice) => sourceIsUsable(choice.stream, mode))
    .sort((left, right) => sourcePreference(right.stream, mode) - sourcePreference(left.stream, mode))[0]
    ?? null;
  if (fallback) {
    return { selected: fallback, choices, selectionReason: "automatic-ready-fallback" };
  }
  if (selectedSource) {
    return { selected: selectedSource, choices, selectionReason: "selected-unavailable" };
  }
  return { selected: null, choices, selectionReason: "unavailable" };
}

/**
 * Plans at most one bounded, non-controlling connection step. Callers persist
 * attempt keys for the current binding so a failed effect is never looped.
 */
export function planAutomaticWorkspaceConnection({
  browserId,
  browserLive,
  sourceResolution,
  currentStream,
  routeRecoveryAction,
  readinessGeneration,
  viewerRoute,
  viewerRouteReady = false,
  viewerLeaseIds,
  attemptedActionKeys,
}: {
  browserId: string;
  browserLive: boolean;
  mode: WorkspaceViewMode;
  sourceResolution: WorkspaceViewSourceResolution;
  currentStream?: ServiceViewStream | null;
  routeRecoveryAction?: string | null;
  readinessGeneration?: string | null;
  viewerRoute?: ServiceViewStream | null;
  viewerRouteReady?: boolean;
  viewerLeaseIds: readonly string[];
  attemptedActionKeys: readonly (string | undefined)[];
}): WorkspaceConnectionPlan {
  if (!browserLive) {
    return { status: "unavailable", action: null, message: "Browser unavailable" };
  }

  const attempted = new Set(attemptedActionKeys.filter((key): key is string => Boolean(key)));
  if (
    sourceResolution.selected
    && sourceResolution.selected.stream !== currentStream
    && sourceResolution.selectionReason === "automatic-ready-fallback"
  ) {
    const attemptKey = connectionAttemptKey("select-source", browserId, sourceResolution.selected.identity, readinessGeneration);
    if (!attempted.has(attemptKey)) {
      return {
        status: "connecting",
        action: { kind: "select-source", source: sourceResolution.selected, attemptKey },
        message: `Opening ${sourceResolution.selected.label}`,
      };
    }
  }

  if (isSafeRouteRecoveryAction(routeRecoveryAction)) {
    const routeIdentity = viewerRoute ? semanticSourceIdentity(viewerRoute) : "unrouted";
    const attemptKey = connectionAttemptKey(routeRecoveryAction, browserId, routeIdentity, readinessGeneration);
    if (!attempted.has(attemptKey)) {
      return {
        status: "connecting",
        action: {
          kind: "recover-route",
          serviceAction: routeRecoveryAction,
          attemptKey,
        },
        message: "Reconnecting desktop",
      };
    }
    return {
      status: "action-required",
      action: null,
      message: "Automatic desktop recovery did not complete",
    };
  }

  const routeId = viewerRoute?.routeId?.trim();
  if (viewerRouteReady && routeId && viewerLeaseIds.length === 0) {
    const attemptKey = connectionAttemptKey("request-viewer-lease", browserId, routeId, readinessGeneration);
    if (!attempted.has(attemptKey)) {
      return {
        status: "connecting",
        action: { kind: "request-viewer-lease", routeId, attemptKey },
        message: "Connecting desktop viewer",
      };
    }
    return {
      status: "action-required",
      action: null,
      message: "Desktop viewer could not reconnect automatically",
    };
  }

  if (sourceResolution.selected && sourceResolution.selectionReason !== "selected-unavailable") {
    return { status: "ready", action: null, message: `${sourceResolution.selected.label} ready` };
  }
  return { status: "unavailable", action: null, message: "No usable browser view" };
}

export function workspaceViewerRouteIsAttached(stream?: ServiceViewStream | null): boolean {
  return Boolean(
    stream?.routeId?.trim()
    && readinessState(stream.attachability) === "attached_ready"
  );
}

export function workspaceConnectionReadinessGeneration(
  stream?: ServiceViewStream | null,
  recoveryAction?: string | null,
): string {
  return [
    stream?.id?.trim() || "stream",
    stream?.routeId?.trim() || "unrouted",
    stream?.displayAllocationId?.trim() || "display",
    readinessState(stream?.attachability),
    readinessState(stream?.remoteReadiness),
    recoveryAction?.trim() || "none",
  ].join("|");
}

function uniqueWorkspaceViewSources(streams: readonly ServiceViewStream[]): WorkspaceViewSource[] {
  const seen = new Set<string>();
  const choices: WorkspaceViewSource[] = [];
  for (const stream of streams) {
    const source = sourceFor(stream);
    if (seen.has(source.identity)) continue;
    seen.add(source.identity);
    choices.push(source);
  }
  const labelCounts = new Map<string, number>();
  for (const choice of choices) {
    labelCounts.set(choice.label, (labelCounts.get(choice.label) ?? 0) + 1);
  }
  return choices.map((choice, index) => labelCounts.get(choice.label) === 1
    ? choice
    : {
        ...choice,
        label: `${choice.label} — ${sourceDisambiguator(choice.stream, index)}`,
      });
}

function sourceFor(stream: ServiceViewStream): WorkspaceViewSource {
  return {
    stream,
    label: semanticSourceLabel(stream),
    detail: viewStreamRouteSummary(stream),
    identity: semanticSourceIdentity(stream),
  };
}

function semanticSourceLabel(stream: ServiceViewStream): string {
  switch (normalized(stream.provider)) {
    case "rdp_gateway":
      return "Desktop";
    case "cdp_screencast":
      return "Live page";
    case "cdp_snapshot":
      return "Snapshot";
    default:
      return "Browser view";
  }
}

function semanticSourceIdentity(stream: ServiceViewStream): string {
  return [
    normalized(stream.provider),
    normalized(stream.routeId),
    normalized(stream.displayAllocationId),
    normalized(stream.providerMode),
  ].join("|");
}

function sourceDisambiguator(stream: ServiceViewStream, index: number): string {
  return stream.connectionName?.trim()
    || stream.routeId?.trim()
    || stream.displayAllocationId?.trim()
    || `source ${index + 1}`;
}

function sourceIsUsable(stream: ServiceViewStream, mode: WorkspaceViewMode): boolean {
  return mode === "control" ? canOpenControlViewStream(stream) : canOpenViewStream(stream);
}

function sourcePreference(stream: ServiceViewStream, mode: WorkspaceViewMode): number {
  const provider = normalized(stream.provider);
  if (mode === "control" && provider === "rdp_gateway") return 30;
  if (provider === "rdp_gateway") return 20;
  if (provider === "cdp_screencast") return 10;
  if (provider === "cdp_snapshot") return 5;
  return 0;
}

function isSafeRouteRecoveryAction(value?: string | null): value is WorkspaceRouteRecoveryAction {
  return value === "service_remote_view_browser_reattach"
    || value === "service_remote_view_route_switch";
}

function connectionAttemptKey(
  action: string,
  browserId: string,
  sourceIdentity: string,
  readinessGeneration?: string | null,
): string {
  return [action, browserId, sourceIdentity, readinessGeneration?.trim() || "current"].join("|");
}

function normalized(value?: string | null): string {
  return value?.trim().toLowerCase() ?? "";
}

function readinessState(value: unknown): string {
  if (!value || typeof value !== "object") return "";
  const state = (value as { state?: unknown }).state;
  return typeof state === "string" ? normalized(state) : "";
}
