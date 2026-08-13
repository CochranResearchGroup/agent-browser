import type { ServiceViewStream } from "./service-view-streams.ts";

const VIEWER_ROUTE_PROVIDER = "rdp_gateway";
const SYNTHETIC_ROUTE_PREFIXES = ["daemon:", "foreign-cdp:", "service-cdp-snapshot:"];

type WorkspaceRecoveryFailure = {
  code?: string | null;
  error?: string | null;
};

/**
 * Select the service-owned RDP route authorized for viewer lease operations.
 * Presentation-only daemon and CDP streams are never route authority.
 */
export function selectWorkspaceViewerRoute(
  streams: readonly ServiceViewStream[],
  selected?: ServiceViewStream | null,
): ServiceViewStream | null {
  if (isWorkspaceViewerRoute(selected)) return selected;
  return streams.find(isWorkspaceViewerRoute) ?? null;
}

function isWorkspaceViewerRoute(stream?: ServiceViewStream | null): stream is ServiceViewStream {
  const routeId = stream?.routeId?.trim();
  if (!routeId || stream?.provider?.trim().toLowerCase() !== VIEWER_ROUTE_PROVIDER) return false;
  return !SYNTHETIC_ROUTE_PREFIXES.some((prefix) => routeId.startsWith(prefix));
}

/** Preserve the backend's typed failure code when a recovery request fails. */
export function workspaceRecoveryFailureMessage(
  response: WorkspaceRecoveryFailure,
  action: string,
): string {
  const code = response.code?.trim() || "";
  const error = response.error?.trim() || "";
  if (!code) return error || `${action} was not accepted`;
  if (!error || error === code || error.startsWith(`${code}:`) || error.includes(`[${code}]`)) {
    return error || code;
  }
  return `${code}: ${error}`;
}
