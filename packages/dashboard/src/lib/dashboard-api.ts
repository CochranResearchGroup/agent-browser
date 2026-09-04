"use client";

export const SERVICE_API_BASE = "/api/service";
export const CHAT_STATUS_API_URL = "/api/chat/status";
export const CHAT_API_URL = "/api/chat";
export const MODELS_API_URL = "/api/models";
export const APP_INTELLIGENCE_STATUS_API_URL = "/api/app-intelligence/status";
export const APP_INTELLIGENCE_INSPECT_API_URL = "/api/app-intelligence/inspect-workspace";
export const APP_INTELLIGENCE_OPERATOR_STATUS_API_URL = "/api/app-intelligence/operator/status";
export const APP_INTELLIGENCE_OPERATOR_TURN_API_URL = "/api/app-intelligence/operator/turn";
export const APP_INTELLIGENCE_OPERATOR_CONFIRM_API_URL = "/api/app-intelligence/operator/confirm";

const SHARED_SERVICE_STATUS_TTL_MS = 10_000;

type SharedServiceStatusSnapshot = {
  body: string;
  completedAt: number;
  headers: [string, string][];
  status: number;
  statusText: string;
};

let sharedServiceStatusSnapshot: SharedServiceStatusSnapshot | null = null;
let sharedServiceStatusFlight: Promise<SharedServiceStatusSnapshot> | null = null;

function serviceStatusResponse(snapshot: SharedServiceStatusSnapshot): Response {
  return new Response(snapshot.body, {
    headers: snapshot.headers,
    status: snapshot.status,
    statusText: snapshot.statusText,
  });
}

/**
 * Share the large Service Status projection across dashboard components.
 * The ten-second freshness bound spans the five-second and seven-second UI
 * poll intervals while preserving a finite, frequently refreshed left rail.
 */
export async function fetchSharedServiceStatus(): Promise<Response> {
  if (
    sharedServiceStatusSnapshot
    && Date.now() - sharedServiceStatusSnapshot.completedAt < SHARED_SERVICE_STATUS_TTL_MS
  ) {
    return serviceStatusResponse(sharedServiceStatusSnapshot);
  }
  if (!sharedServiceStatusFlight) {
    sharedServiceStatusFlight = fetch(`${SERVICE_API_BASE}/status`, { cache: "no-store" })
      .then(async (response) => {
        const snapshot = {
          body: await response.text(),
          completedAt: Date.now(),
          headers: Array.from(response.headers.entries()),
          status: response.status,
          statusText: response.statusText,
        } satisfies SharedServiceStatusSnapshot;
        if (response.ok) sharedServiceStatusSnapshot = snapshot;
        return snapshot;
      })
      .finally(() => {
        sharedServiceStatusFlight = null;
      });
  }
  return serviceStatusResponse(await sharedServiceStatusFlight);
}

export function sessionTabsApiUrl(port: number): string {
  return `/api/session-tabs?port=${encodeURIComponent(String(port))}`;
}

export function sessionScreenshotApiUrl(port: number, targetId?: string | null): string {
  const params = new URLSearchParams({ port: String(port) });
  if (targetId) params.set("targetId", targetId);
  return `/api/session-screenshot?${params.toString()}`;
}

export function sessionConsoleApiUrl(port: number, session?: string | null): string {
  const params = new URLSearchParams({ port: String(port) });
  if (session) params.set("session", session);
  return `/api/session-console?${params.toString()}`;
}
