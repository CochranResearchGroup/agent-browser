export type WorkspaceViewportUxState =
  | "preparing_focus"
  | "connecting"
  | "connected"
  | "owned_elsewhere"
  | "takeover_ready"
  | "taken_over"
  | "reconnecting"
  | "stale_target_recovered"
  | "provider_unavailable"
  | "browser_unavailable";

export type WorkspaceViewportUxStateInput = {
  hasBrowser: boolean;
  browserHealth?: string | null;
  hasStream: boolean;
  canEmbed: boolean;
  canControl: boolean;
  mode: "view" | "control";
  preflightStatus: "idle" | "checking" | "ready" | "login-required" | "error" | string;
  frameIssueKind?: "remote-disconnected" | "taken-over" | string | null;
  focusPending?: boolean;
  takeoverPending?: boolean;
  recoveredStaleTarget?: boolean;
};

export type WorkspaceViewportReadinessStatus =
  | "ready"
  | "checking"
  | "action_required"
  | "blocked";

export type WorkspaceViewportReadinessAction =
  | "none"
  | "wait_for_focus"
  | "wait_for_takeover"
  | "wait_for_stream"
  | "take_over"
  | "sign_in_again"
  | "open_externally"
  | "relaunch_browser"
  | "inspect_readiness"
  | "open_view_only";

export type WorkspaceViewportReadinessComponent = {
  component: string;
  status?: string | null;
  evidence?: string | null;
  nextAction?: string | null;
  recovery?: string | null;
  message?: string | null;
};

export type WorkspaceViewportReadinessInput = WorkspaceViewportUxStateInput & {
  streamProvider?: string | null;
  streamUrl?: string | null;
  streamReadiness?: unknown;
  preflightMessage?: string | null;
  frameIssueMessage?: string | null;
  focusMessage?: string | null;
};

export type WorkspaceViewportReadinessResult = {
  component: string;
  status: WorkspaceViewportReadinessStatus;
  evidence: string;
  nextAction: WorkspaceViewportReadinessAction;
  title: string;
  recoveryCopy: string;
};

const DEAD_BROWSER_HEALTH = new Set([
  "closed",
  "faulted",
  "not_started",
  "process_exited",
  "unreachable",
]);

const FAILED_READINESS_STATES = new Set([
  "blocked",
  "down",
  "error",
  "failed",
  "missing",
  "refused",
  "unavailable",
  "unhealthy",
]);

const ACTION_READINESS_STATES = new Set([
  "action_required",
  "auth_required",
  "degraded",
  "expired",
  "stale",
  "unknown",
  "warning",
]);

export function deriveWorkspaceViewportUxState(input: WorkspaceViewportUxStateInput): WorkspaceViewportUxState {
  const health = normalized(input.browserHealth);
  if (!input.hasBrowser || DEAD_BROWSER_HEALTH.has(health)) return "browser_unavailable";
  if (input.takeoverPending) return "reconnecting";
  if (input.frameIssueKind === "taken-over") return "taken_over";
  if (input.frameIssueKind === "remote-disconnected") return "takeover_ready";
  if (input.recoveredStaleTarget) return "stale_target_recovered";
  if (input.focusPending) return "preparing_focus";
  if (input.preflightStatus === "checking") return "connecting";
  if (input.preflightStatus === "login-required" || input.preflightStatus === "error") return "provider_unavailable";
  if (!input.hasStream || !input.canEmbed) return "provider_unavailable";
  if (input.mode === "control" && !input.canControl) return "connected";
  if (input.preflightStatus === "ready") return "connected";
  return "connecting";
}

export function deriveWorkspaceViewportReadiness(input: WorkspaceViewportReadinessInput): WorkspaceViewportReadinessResult {
  const health = normalized(input.browserHealth);
  if (!input.hasBrowser || DEAD_BROWSER_HEALTH.has(health)) {
    return {
      component: "browser",
      status: "blocked",
      evidence: input.hasBrowser ? `browser health is ${input.browserHealth ?? "unknown"}` : "no selected browser",
      nextAction: "relaunch_browser",
      title: "Browser unavailable",
      recoveryCopy: "The selected browser process or CDP endpoint is unhealthy. Relaunch the browser or inspect browser health before opening the remote desktop stream.",
    };
  }

  if (input.takeoverPending) {
    return {
      component: "viewer_lease",
      status: "checking",
      evidence: "view_takeover request is pending",
      nextAction: "wait_for_takeover",
      title: "Reconnecting viewer",
      recoveryCopy: "Agent Browser is waiting for the service-owned viewer takeover to reconnect this workspace stream.",
    };
  }

  if (input.frameIssueKind === "taken-over") {
    return {
      component: "viewer_lease",
      status: "action_required",
      evidence: input.frameIssueMessage || "viewer was replaced by another dashboard or popout",
      nextAction: "take_over",
      title: "Viewer taken over",
      recoveryCopy: "This viewer was taken over by another dashboard or Guacamole popout. Take over to reconnect it here.",
    };
  }

  if (input.frameIssueKind === "remote-disconnected") {
    return {
      component: "viewer_lease",
      status: "action_required",
      evidence: input.frameIssueMessage || "Guacamole reported this viewer disconnected",
      nextAction: "take_over",
      title: "Viewer ownership changed",
      recoveryCopy: "Another dashboard or Guacamole popout is using this remote desktop. Take over to reconnect it here.",
    };
  }

  if (input.focusPending) {
    return {
      component: "browser_focus",
      status: "checking",
      evidence: "view_focus request is pending",
      nextAction: "wait_for_focus",
      title: "Focusing browser",
      recoveryCopy: "Agent Browser is focusing the selected tab and asking the native window manager to maximize the browser before the stream is used.",
    };
  }

  const componentIssue = firstBlockingReadinessComponent(input.streamReadiness, {
    streamReady: Boolean(input.hasStream && input.streamUrl?.trim() && input.preflightStatus === "ready"),
  });
  if (componentIssue) return componentIssue;

  if (!input.hasStream || !input.streamUrl?.trim()) {
    return {
      component: "selected_stream",
      status: "blocked",
      evidence: input.hasStream ? "selected stream has no URL" : "selected browser has no view-stream record",
      nextAction: "inspect_readiness",
      title: "No embeddable stream",
      recoveryCopy: "The selected workspace does not currently report a service-owned view stream. Inspect readiness for the browser, display, Guacamole connection, and stream URL.",
    };
  }

  if (!input.canEmbed) {
    return {
      component: "iframe_embedding",
      status: "action_required",
      evidence: `${input.streamProvider || "view stream"} cannot be embedded in the dashboard`,
      nextAction: "open_externally",
      title: "Stream cannot be embedded",
      recoveryCopy: "This stream cannot be embedded in the dashboard. Open it externally or inspect provider readiness and iframe policy.",
    };
  }

  if (input.preflightStatus === "login-required") {
    return {
      component: "dashboard_auth",
      status: "action_required",
      evidence: input.preflightMessage || "dashboard session is not accepted by the stream route",
      nextAction: "sign_in_again",
      title: "Stream sign-in expired",
      recoveryCopy: "The remote stream needs a fresh dashboard sign-in before it can be embedded. Sign in again, then refresh the workspace viewport.",
    };
  }

  if (input.preflightStatus === "error") {
    return {
      component: providerComponent(input.preflightMessage),
      status: "blocked",
      evidence: input.preflightMessage || "stream preflight failed",
      nextAction: "open_externally",
      title: "Stream unavailable",
      recoveryCopy: "The remote stream route could not be reached or accepted by the browser. Open externally, then inspect RDP, Guacamole, dashboard auth, and ingress readiness.",
    };
  }

  if (input.preflightStatus === "checking") {
    return {
      component: "stream_preflight",
      status: "checking",
      evidence: "stream route preflight is in progress",
      nextAction: "wait_for_stream",
      title: "Checking stream",
      recoveryCopy: "Agent Browser is checking the workspace stream route before embedding it.",
    };
  }

  if (input.mode === "control" && !input.canControl) {
    return {
      component: "control_input",
      status: "action_required",
      evidence: `${input.streamProvider || "stream"} is view-only or lacks a control input provider`,
      nextAction: "open_view_only",
      title: "Stream is view-only",
      recoveryCopy: "The service did not report a control input provider for this stream. Use view mode or inspect provider readiness before expecting manual desktop control.",
    };
  }

  if (input.recoveredStaleTarget) {
    return {
      component: "selected_target",
      status: "ready",
      evidence: "retained target identity was stale and a live tab was selected",
      nextAction: "none",
      title: "Recovered stale selected tab identity",
      recoveryCopy: "The retained target identity was stale, but Agent Browser selected a current live tab before opening the workspace viewport.",
    };
  }

  return {
    component: input.streamProvider || "view_stream",
    status: "ready",
    evidence: input.streamUrl ? "stream URL is present and preflight is ready" : "stream is ready",
    nextAction: "none",
    title: "Stream ready",
    recoveryCopy: "The selected browser and remote stream are ready.",
  };
}

export function workspaceViewportUxStateLabel(state: WorkspaceViewportUxState): string {
  return state.replaceAll("_", " ");
}

export function workspaceViewportReadinessStatusLabel(status: WorkspaceViewportReadinessStatus): string {
  return status.replaceAll("_", " ");
}

export function compactWorkspaceViewportReadinessComponents(readiness: unknown): WorkspaceViewportReadinessComponent[] {
  if (!readiness) return [];
  const components = recordField(readiness, "components");
  const checks = recordField(readiness, "checks");
  const results = recordField(readiness, "results");
  const array: unknown[] = Array.isArray(readiness)
    ? readiness
    : Array.isArray(components)
      ? components
      : Array.isArray(checks)
        ? checks
        : Array.isArray(results)
          ? results
          : [readiness];

  return array
    .filter((item): item is Record<string, unknown> => Boolean(item) && typeof item === "object" && !Array.isArray(item))
    .map((item) => ({
      component: stringField(item, ["component", "name", "id", "check"]) || "readiness",
      status: stringField(item, ["status", "state", "result"]),
      evidence: stringField(item, ["evidence", "reason", "details"]),
      nextAction: stringField(item, ["nextAction", "next_action", "recommendedAction", "recommended_action"]),
      recovery: stringField(item, ["recovery", "recoveryCopy", "copy", "operatorCopy"]),
      message: stringField(item, ["message", "summary", "title"]),
    }));
}

function firstBlockingReadinessComponent(
  readiness: unknown,
  options: { streamReady?: boolean } = {},
): WorkspaceViewportReadinessResult | null {
  for (const component of compactWorkspaceViewportReadinessComponents(readiness)) {
    const status = normalized(component.status);
    if (!status || status === "ready" || status === "ok" || status === "passed") continue;
    if (status === "stale" && options.streamReady && isRetainedJobComponent(component.component)) {
      continue;
    }
    const isFailed = FAILED_READINESS_STATES.has(status);
    if (!isFailed && !ACTION_READINESS_STATES.has(status)) continue;
    const label = componentLabel(component.component);
    return {
      component: component.component,
      status: isFailed ? "blocked" : "action_required",
      evidence: component.evidence || component.message || `${label} readiness is ${component.status ?? "not ready"}.`,
      nextAction: readinessNextAction(component),
      title: `${label} readiness ${isFailed ? "failed" : "needs attention"}`,
      recoveryCopy: component.recovery || component.message || component.nextAction || `Inspect ${label} readiness before opening the workspace stream.`,
    };
  }
  return null;
}

function isRetainedJobComponent(component: string): boolean {
  const normalizedComponent = normalized(component).replaceAll("-", "_");
  return normalizedComponent.includes("focus_job")
    || normalizedComponent.includes("takeover_job")
    || normalizedComponent.includes("view_focus")
    || normalizedComponent.includes("view_takeover");
}

function readinessNextAction(component: WorkspaceViewportReadinessComponent): WorkspaceViewportReadinessAction {
  const action = normalized(component.nextAction);
  if (action.includes("sign") || action.includes("auth")) return "sign_in_again";
  if (action.includes("takeover") || action.includes("take_over")) return "take_over";
  if (action.includes("external") || action.includes("popout")) return "open_externally";
  if (action.includes("relaunch") || action.includes("browser")) return "relaunch_browser";
  return "inspect_readiness";
}

function providerComponent(message?: string | null): string {
  const normalizedMessage = normalized(message);
  if (normalizedMessage.includes("401") || normalizedMessage.includes("403") || normalizedMessage.includes("sign")) {
    return "dashboard_auth";
  }
  if (normalizedMessage.includes("refused") || normalizedMessage.includes("timeout") || normalizedMessage.includes("fetch")) {
    return "public_ingress";
  }
  if (normalizedMessage.includes("guacamole") || normalizedMessage.includes("connection closed")) {
    return "guacamole_connection";
  }
  return "provider";
}

function componentLabel(component: string): string {
  return component.replaceAll("_", " ").replaceAll("-", " ");
}

function normalized(value?: string | null): string {
  return (value ?? "").trim().toLowerCase();
}

function recordField(value: unknown, key: string): unknown {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)[key]
    : undefined;
}

function stringField(value: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const field = value[key];
    if (typeof field === "string" && field.trim()) return field.trim();
    if (typeof field === "number") return String(field);
    if (typeof field === "boolean") return String(field);
  }
  return null;
}
