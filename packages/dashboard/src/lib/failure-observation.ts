export type DashboardFailureCategory =
  | "guacamole_load"
  | "handoff_link"
  | "cdp_stream"
  | "dashboard_action";

export type DashboardFailureObservation = {
  category: DashboardFailureCategory;
  stage: string;
  code: string;
  summary: string;
  action?: string | null;
  observationId?: string;
  browserId?: string | null;
  profileId?: string | null;
  sessionId?: string | null;
  routeId?: string | null;
  displayId?: string | null;
  handoffIdHash?: string | null;
  streamProvider?: string | null;
  elapsedMs?: number | null;
};

type FetchImplementation = typeof globalThis.fetch;

const FAILURE_OBSERVATION_ROUTE = "/api/service/failure-observation";

/** Submit privacy-bounded client evidence without disrupting the failing action. */
export async function reportDashboardFailure(
  observation: DashboardFailureObservation,
  fetcher: FetchImplementation = globalThis.fetch,
): Promise<void> {
  try {
    await fetcher(FAILURE_OBSERVATION_ROUTE, {
      method: "POST",
      credentials: "same-origin",
      keepalive: true,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ...observation,
        observationId: observation.observationId || newObservationId(),
      }),
    });
  } catch {
    // Failure reporting is strictly subordinate to the original action.
  }
}

export async function hashOpaqueIdentifier(value: string): Promise<string | null> {
  if (!globalThis.crypto?.subtle || !value.trim()) return null;
  const encoded = new TextEncoder().encode(value.trim());
  const digest = await globalThis.crypto.subtle.digest("SHA-256", encoded);
  return `sha256:${Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

/**
 * Observe dashboard fetch failures globally. The wrapper never consumes or
 * changes the response returned to the caller and never records request URLs,
 * bodies, headers, or credentials.
 */
export function installDashboardFetchFailureInstrumentation(): () => void {
  if (typeof window === "undefined") return () => undefined;
  const current = window.fetch.bind(window);
  const instrumented: FetchImplementation = async (input, init) => {
    const metadata = observableFetchMetadata(input, init);
    try {
      const response = await current(input, init);
      if (metadata && !response.ok) {
        void reportDashboardFailure({
          ...metadata,
          code: `http_${response.status}`,
          summary: "Dashboard action received a non-success HTTP response.",
        }, current);
      } else if (metadata && response.headers.get("content-type")?.includes("application/json")) {
        void response.clone().json().then((payload: unknown) => {
          if (payload && typeof payload === "object" && (payload as { success?: unknown }).success === false) {
            return reportDashboardFailure({
              ...metadata,
              code: "application_failure",
              summary: "Dashboard action returned a non-success application result.",
            }, current);
          }
        }).catch(() => undefined);
      }
      return response;
    } catch (error) {
      if (metadata) {
        void reportDashboardFailure({
          ...metadata,
          code: error instanceof DOMException && error.name === "AbortError" ? "request_aborted" : "fetch_rejected",
          summary: "Dashboard action failed before receiving an HTTP response.",
        }, current);
      }
      throw error;
    }
  };
  window.fetch = instrumented;
  return () => {
    if (window.fetch === instrumented) window.fetch = current;
  };
}

function observableFetchMetadata(
  input: RequestInfo | URL,
  init?: RequestInit,
): Omit<DashboardFailureObservation, "code" | "summary"> | null {
  const raw = typeof input === "string" || input instanceof URL ? String(input) : input.url;
  let path = "";
  try {
    path = new URL(raw, window.location.href).pathname;
  } catch {
    return null;
  }
  if (path === FAILURE_OBSERVATION_ROUTE) return null;
  const method = (init?.method || (input instanceof Request ? input.method : "GET")).toUpperCase();
  const action = actionFromBody(init?.body);
  if (path.includes("/stream/") && path.endsWith("/frame")) {
    return { category: "cdp_stream", stage: "frame_fetch", action: action || "stream_frame" };
  }
  if (path.includes("guacamole")) {
    return { category: "guacamole_load", stage: "http_load", action: action || "remote_view_load" };
  }
  if (!["POST", "PUT", "PATCH", "DELETE"].includes(method)) return null;
  return {
    category: action === "service_remote_view_handoff_resolve" ? "handoff_link" : "dashboard_action",
    stage: "http_action",
    action: action || method.toLowerCase(),
  };
}

function actionFromBody(body: BodyInit | null | undefined): string | null {
  if (typeof body !== "string") return null;
  try {
    const value = JSON.parse(body) as { action?: unknown };
    return typeof value.action === "string" && value.action.trim() ? value.action.trim().slice(0, 128) : null;
  } catch {
    return null;
  }
}

function newObservationId(): string {
  return globalThis.crypto?.randomUUID?.() || `dashboard-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}
