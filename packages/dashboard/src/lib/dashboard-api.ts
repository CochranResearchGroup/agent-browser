"use client";

import {
  fetchCoordinatedDashboardRead,
  invalidateCoordinatedDashboardReadGroup,
} from "@/lib/dashboard-read-coordinator";

export const SERVICE_API_BASE = "/api/service";
export const CHAT_STATUS_API_URL = "/api/chat/status";
export const CHAT_API_URL = "/api/chat";
export const MODELS_API_URL = "/api/models";
export const APP_INTELLIGENCE_STATUS_API_URL = "/api/app-intelligence/status";
export const APP_INTELLIGENCE_INSPECT_API_URL = "/api/app-intelligence/inspect-workspace";
export const APP_INTELLIGENCE_OPERATOR_STATUS_API_URL = "/api/app-intelligence/operator/status";
export const APP_INTELLIGENCE_OPERATOR_TURN_API_URL = "/api/app-intelligence/operator/turn";
export const APP_INTELLIGENCE_OPERATOR_CONFIRM_API_URL = "/api/app-intelligence/operator/confirm";

const SHARED_SERVICE_READ_TTL_MS = 10_000;
const RUNTIME_STATIC_READ_TTL_MS = 5 * 60_000;
const RUNTIME_STATIC_READ_GROUP = "runtime-static";
let observedRuntimeGeneration: string | null = null;

/**
 * Share the large Service Status projection across dashboard components.
 * The ten-second freshness bound spans the five-second and seven-second UI
 * poll intervals while preserving a finite, frequently refreshed left rail.
 */
export async function fetchSharedServiceStatus(): Promise<Response> {
  return fetchCoordinatedDashboardRead(`${SERVICE_API_BASE}/status?projection=dashboard-summary`, {
    freshForMs: SHARED_SERVICE_READ_TTL_MS,
  });
}

export function fetchSharedServiceResources(): Promise<Response> {
  return fetchCoordinatedDashboardRead(`${SERVICE_API_BASE}/resources`, {
    freshForMs: SHARED_SERVICE_READ_TTL_MS,
  });
}

export function fetchSharedServiceContracts(): Promise<Response> {
  return fetchCoordinatedDashboardRead(`${SERVICE_API_BASE}/contracts`, {
    cacheGroup: RUNTIME_STATIC_READ_GROUP,
    freshForMs: RUNTIME_STATIC_READ_TTL_MS,
  });
}

export function fetchSharedBrowserCapabilityRegistry(): Promise<Response> {
  return fetchCoordinatedDashboardRead(`${SERVICE_API_BASE}/browser-capability-registry`, {
    cacheGroup: RUNTIME_STATIC_READ_GROUP,
    freshForMs: RUNTIME_STATIC_READ_TTL_MS,
  });
}

export async function fetchSharedRuntimeHealth(): Promise<Response> {
  const response = await fetchCoordinatedDashboardRead("/api/runtime/health", {
    freshForMs: SHARED_SERVICE_READ_TTL_MS,
  });
  if (response.ok) {
    const health = await response.clone().json().catch(() => null) as {
      workstationUpgrade?: { selectedGenerationId?: unknown };
    } | null;
    const generation = health?.workstationUpgrade?.selectedGenerationId;
    if (typeof generation === "string" && generation.length > 0) {
      if (observedRuntimeGeneration !== generation) {
        invalidateCoordinatedDashboardReadGroup(RUNTIME_STATIC_READ_GROUP);
      }
      observedRuntimeGeneration = generation;
    }
  }
  return response;
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
