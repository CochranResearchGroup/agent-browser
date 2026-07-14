export type WorkspaceViewportTargetMode = "view" | "control" | "tile";

export type WorkspaceViewportTarget = {
  browserId: string | null;
  streamId: string | null;
  streamUrl: string | null;
  routeId: string | null;
  mode: WorkspaceViewportTargetMode;
  browserAvailable: boolean;
};

export type WorkspaceViewportPreflightState = {
  status: "idle" | "checking" | "ready" | "login-required" | "error";
  message: string;
};

export type WorkspaceViewportFrameIssue = {
  kind: "remote-disconnected" | "taken-over";
  message: string;
} | null;

export type WorkspaceViewportFrameState = {
  issue: WorkspaceViewportFrameIssue;
};

export type WorkspaceViewportRecoveryState = {
  status: "idle" | "pending" | "accepted" | "failed";
  action: string | null;
  message: string;
};

export type WorkspaceViewportControllerState = {
  targetToken: string | null;
  target: WorkspaceViewportTarget | null;
  targetStatus: "none" | "available" | "browser-unavailable";
  preflight: WorkspaceViewportPreflightState;
  frame: WorkspaceViewportFrameState;
  recovery: WorkspaceViewportRecoveryState;
};

export type WorkspaceViewportControllerEvent =
  | { type: "target_changed"; target: WorkspaceViewportTarget | null }
  | { type: "preflight_started"; targetToken: string; message?: string }
  | { type: "preflight_succeeded"; targetToken: string; message?: string }
  | {
      type: "preflight_failed";
      targetToken: string;
      status: "login-required" | "error";
      message: string;
    }
  | { type: "frame_cleared"; targetToken: string }
  | {
      type: "frame_failed";
      targetToken: string;
      kind: "remote-disconnected" | "taken-over";
      message: string;
    }
  | { type: "recovery_started"; targetToken: string; action: string; message?: string }
  | { type: "recovery_accepted"; targetToken: string; message?: string }
  | { type: "recovery_failed"; targetToken: string; message: string };

export const INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE: WorkspaceViewportControllerState = {
  targetToken: null,
  target: null,
  targetStatus: "none",
  preflight: { status: "idle", message: "" },
  frame: { issue: null },
  recovery: { status: "idle", action: null, message: "" },
};

function tokenPart(value: string | null | undefined): string {
  return encodeURIComponent(value?.trim() || "-");
}

export function workspaceViewportTargetToken(target: WorkspaceViewportTarget | null): string | null {
  if (!target) return null;
  return [
    `browser=${tokenPart(target.browserId)}`,
    `stream=${tokenPart(target.streamId)}`,
    `url=${tokenPart(target.streamUrl)}`,
    `route=${tokenPart(target.routeId)}`,
    `mode=${tokenPart(target.mode)}`,
  ].join("|");
}

export function workspaceViewportControllerReducer(
  state: WorkspaceViewportControllerState,
  event: WorkspaceViewportControllerEvent,
): WorkspaceViewportControllerState {
  if (event.type === "target_changed") {
    const targetToken = workspaceViewportTargetToken(event.target);
    if (targetToken === state.targetToken && event.target?.browserAvailable === state.target?.browserAvailable) {
      return state;
    }
    return {
      ...INITIAL_WORKSPACE_VIEWPORT_CONTROLLER_STATE,
      targetToken,
      target: event.target,
      targetStatus: !event.target ? "none" : event.target.browserAvailable ? "available" : "browser-unavailable",
    };
  }

  if (event.targetToken !== state.targetToken) {
    return state;
  }

  switch (event.type) {
    case "preflight_started":
      return {
        ...state,
        preflight: { status: "checking", message: event.message ?? "Checking stream access." },
      };
    case "preflight_succeeded":
      return {
        ...state,
        preflight: { status: "ready", message: event.message ?? "" },
      };
    case "preflight_failed":
      return {
        ...state,
        preflight: { status: event.status, message: event.message },
      };
    case "frame_cleared":
      return {
        ...state,
        frame: { issue: null },
      };
    case "frame_failed":
      return {
        ...state,
        frame: {
          issue: {
            kind: event.kind,
            message: event.message,
          },
        },
      };
    case "recovery_started":
      return {
        ...state,
        recovery: {
          status: "pending",
          action: event.action,
          message: event.message ?? "",
        },
      };
    case "recovery_accepted":
      return {
        ...state,
        recovery: {
          status: "accepted",
          action: state.recovery.action,
          message: event.message ?? "",
        },
      };
    case "recovery_failed":
      return {
        ...state,
        recovery: {
          status: "failed",
          action: state.recovery.action,
          message: event.message,
        },
      };
  }
}
