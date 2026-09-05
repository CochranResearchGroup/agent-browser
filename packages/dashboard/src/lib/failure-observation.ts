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
// Plan 0158's largest frozen dashboard-action epoch contains 500 operations.
// Preserve two complete epochs before explicit loss accounting begins.
export const MAX_PENDING_DASHBOARD_READ_FAILURES = 4096;

type RequiredObservationId = DashboardFailureObservation & { observationId: string };

type FailureDeliveryEnvironment = {
  fetcher: FetchImplementation;
  isVisible: () => boolean;
  maxPending: number;
};

/**
 * Retain browser-only read failures while the same-origin runtime is
 * unavailable. Delivery is driven only by a later successful observed read,
 * so this queue never retries the failed operation or creates a probe loop.
 */
export class DashboardFailureDeliveryQueue {
  private readonly environment: FailureDeliveryEnvironment;
  private readonly pending: RequiredObservationId[] = [];
  private readonly pendingIds = new Set<string>();
  private droppedCount = 0;
  private gapObservationId: string | null = null;
  private flushPromise: Promise<void> | null = null;

  constructor(environment: Partial<FailureDeliveryEnvironment> = {}) {
    this.environment = {
      fetcher: environment.fetcher ?? globalThis.fetch,
      isVisible: environment.isVisible ?? isDocumentVisible,
      maxPending: Math.max(
        0,
        Math.floor(environment.maxPending ?? MAX_PENDING_DASHBOARD_READ_FAILURES),
      ),
    };
  }

  enqueue(observation: DashboardFailureObservation): void {
    const occurrence = {
      ...observation,
      observationId: observation.observationId || newObservationId(),
    };
    if (this.pendingIds.has(occurrence.observationId)) return;
    if (this.environment.maxPending === 0) {
      this.recordGap();
      return;
    }
    while (this.pending.length >= this.environment.maxPending) {
      const dropped = this.pending.shift();
      if (dropped) this.pendingIds.delete(dropped.observationId);
      this.recordGap();
    }
    this.pending.push(occurrence);
    this.pendingIds.add(occurrence.observationId);
  }

  /** Deliver retained evidence once, stopping at the first delivery failure. */
  flushAfterRecovery(): Promise<void> {
    if (!this.environment.isVisible()) return Promise.resolve();
    if (this.flushPromise) return this.flushPromise;
    this.flushPromise = this.flush().finally(() => {
      this.flushPromise = null;
    });
    return this.flushPromise;
  }

  /** Privacy-safe queue counters for diagnostics and provider-free tests. */
  counts(): { dropped: number; pending: number } {
    return { dropped: this.droppedCount, pending: this.pending.length };
  }

  private recordGap(): void {
    this.droppedCount += 1;
    this.gapObservationId ||= newObservationId();
  }

  private async flush(): Promise<void> {
    if (this.droppedCount > 0 && this.gapObservationId) {
      const reportedCount = this.droppedCount;
      const reportedId = this.gapObservationId;
      const delivered = await sendDashboardFailure({
        category: "dashboard_action",
        stage: "failure_delivery",
        code: "dashboard_read_failure_delivery_gap",
        summary: `Dashboard read failure delivery queue dropped ${reportedCount} occurrence${reportedCount === 1 ? "" : "s"}.`,
        action: "dashboard_read_failure_delivery",
        observationId: reportedId,
      }, this.environment.fetcher);
      if (!delivered) return;
      this.droppedCount = Math.max(0, this.droppedCount - reportedCount);
      if (this.droppedCount === 0) this.gapObservationId = null;
    }

    while (this.pending.length > 0) {
      const occurrence = this.pending[0];
      if (!await sendDashboardFailure(occurrence, this.environment.fetcher)) return;
      if (this.pending[0] === occurrence) this.pending.shift();
      this.pendingIds.delete(occurrence.observationId);
    }
  }
}

/** Submit privacy-bounded client evidence without disrupting the failing action. */
export async function reportDashboardFailure(
  observation: DashboardFailureObservation,
  fetcher: FetchImplementation = globalThis.fetch,
): Promise<void> {
  await sendDashboardFailure(observation, fetcher);
}

// Module lifetime spans dashboard SPA transitions and runtime recovery. The
// dynamic fetch indirection keeps the queue attached to the currently installed
// window wrapper without recursively observing its own allowlisted endpoint.
const dashboardReadFailureDeliveryQueue = new DashboardFailureDeliveryQueue({
  fetcher: (input, init) => globalThis.fetch(input, init),
});

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
      if (metadata?.deferUntilRecovery && !response.ok) {
        dashboardReadFailureDeliveryQueue.enqueue({
          ...metadata.observation,
          code: `http_${response.status}`,
          summary: "Dashboard read received a non-success HTTP response.",
        });
      } else if (metadata?.deferUntilRecovery && !isJsonResponse(response)) {
        dashboardReadFailureDeliveryQueue.enqueue({
          ...metadata.observation,
          code: "response_non_json",
          summary: "Dashboard read received a non-JSON response.",
        });
      } else if (metadata?.deferUntilRecovery) {
        void response.clone().json().then((payload: unknown) => {
          if (payload && typeof payload === "object" && (payload as { success?: unknown }).success === false) {
            dashboardReadFailureDeliveryQueue.enqueue({
              ...metadata.observation,
              code: "application_failure",
              summary: "Dashboard read returned a non-success application result.",
            });
            return;
          }
          return dashboardReadFailureDeliveryQueue.flushAfterRecovery();
        }).catch(() => {
          dashboardReadFailureDeliveryQueue.enqueue({
            ...metadata.observation,
            code: "response_invalid_json",
            summary: "Dashboard read received malformed JSON.",
          });
        });
      } else if (metadata && !response.ok) {
        void reportDashboardFailure({
          ...metadata.observation,
          code: `http_${response.status}`,
          summary: "Dashboard action received a non-success HTTP response.",
        }, current);
      } else if (metadata && isJsonResponse(response)) {
        void response.clone().json().then((payload: unknown) => {
          if (payload && typeof payload === "object" && (payload as { success?: unknown }).success === false) {
            return reportDashboardFailure({
              ...metadata.observation,
              code: "application_failure",
              summary: "Dashboard action returned a non-success application result.",
            }, current);
          }
        }).catch(() => undefined);
      }
      return response;
    } catch (error) {
      if (metadata) {
        const observation = {
          ...metadata.observation,
          code: error instanceof DOMException && error.name === "AbortError" ? "request_aborted" : "fetch_rejected",
          summary: metadata.deferUntilRecovery
            ? "Dashboard read failed before receiving an HTTP response."
            : "Dashboard action failed before receiving an HTTP response.",
        };
        if (metadata.deferUntilRecovery) dashboardReadFailureDeliveryQueue.enqueue(observation);
        else void reportDashboardFailure(observation, current);
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
): {
  observation: Omit<DashboardFailureObservation, "code" | "summary">;
  deferUntilRecovery: boolean;
} | null {
  const raw = typeof input === "string" || input instanceof URL ? String(input) : input.url;
  let parsed: URL;
  try {
    parsed = new URL(raw, window.location.href);
  } catch {
    return null;
  }
  if (parsed.origin !== window.location.origin) return null;
  const path = parsed.pathname;
  if (path === FAILURE_OBSERVATION_ROUTE) return null;
  const method = (init?.method || (input instanceof Request ? input.method : "GET")).toUpperCase();
  const action = actionFromBody(init?.body);
  if (method === "GET" && isDocumentVisible()) {
    const readAction = monitoredReadAction(path);
    if (readAction) {
      return {
        deferUntilRecovery: true,
        observation: {
          category: "dashboard_action",
          stage: "http_read",
          action: readAction,
        },
      };
    }
  }
  if (path.includes("/stream/") && path.endsWith("/frame")) {
    return {
      deferUntilRecovery: false,
      observation: { category: "cdp_stream", stage: "frame_fetch", action: action || "stream_frame" },
    };
  }
  if (path.includes("guacamole")) {
    return {
      deferUntilRecovery: false,
      observation: { category: "guacamole_load", stage: "http_load", action: action || "remote_view_load" },
    };
  }
  if (!["POST", "PUT", "PATCH", "DELETE"].includes(method)) return null;
  return {
    deferUntilRecovery: false,
    observation: {
      category: action === "service_remote_view_handoff_resolve" ? "handoff_link" : "dashboard_action",
      stage: "http_action",
      action: action || method.toLowerCase(),
    },
  };
}

function monitoredReadAction(path: string): string | null {
  if (path === "/api/service/status") return "service_status_read";
  if (path === "/api/service/resources") return "service_resources_read";
  if (path === "/api/session-tabs") return "session_tabs_read";
  if (path === "/api/runtime/health") return "runtime_health_read";
  return null;
}

function isJsonResponse(response: Response): boolean {
  const contentType = response.headers.get("content-type")?.toLowerCase() || "";
  return contentType.includes("application/json") || contentType.includes("+json");
}

function isDocumentVisible(): boolean {
  return typeof document === "undefined" || document.visibilityState === "visible";
}

async function sendDashboardFailure(
  observation: DashboardFailureObservation,
  fetcher: FetchImplementation,
): Promise<boolean> {
  try {
    const response = await fetcher(FAILURE_OBSERVATION_ROUTE, {
      method: "POST",
      credentials: "same-origin",
      keepalive: true,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ...observation,
        observationId: observation.observationId || newObservationId(),
      }),
    });
    return response.ok;
  } catch {
    // Failure reporting is strictly subordinate to the original action.
    return false;
  }
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
