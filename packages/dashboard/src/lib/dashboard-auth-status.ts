import {dashboardLocalFailure} from "./response-failure.ts";
export type DashboardAuthStatusFetchOptions = {
  attempts?: number;
  timeoutMs?: number;
  retryDelayMs?: number;
};

const DEFAULT_ATTEMPTS = 3;
const DEFAULT_TIMEOUT_MS = 5_000;
const DEFAULT_RETRY_DELAY_MS = 250;

/**
 * Read the shared Authelia session without allowing a starved transport
 * request to hold the authentication gate open indefinitely.
 */
export async function fetchDashboardAuthStatus(
  fetchImpl: typeof fetch = globalThis.fetch,
  options: DashboardAuthStatusFetchOptions = {},
): Promise<Response> {
  const attempts = options.attempts ?? DEFAULT_ATTEMPTS;
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const retryDelayMs = options.retryDelayMs ?? DEFAULT_RETRY_DELAY_MS;
  if (!Number.isInteger(attempts) || attempts < 1) {
    throw dashboardLocalFailure(new Error("Dashboard authentication status attempts must be a positive integer"),"dashboard_argument_invalid","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
  }
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw dashboardLocalFailure(new Error("Dashboard authentication status timeout must be positive"),"dashboard_argument_invalid","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
  }
  if (!Number.isFinite(retryDelayMs) || retryDelayMs < 0) {
    throw dashboardLocalFailure(new Error("Dashboard authentication status retry delay cannot be negative"),"dashboard_argument_invalid","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
  }

  let lastError: unknown = dashboardLocalFailure(new Error("Dashboard authentication status request failed"),"dashboard_auth_status_unavailable","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    const controller = new AbortController();
    const timeout = globalThis.setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetchImpl("/api/dashboard-auth/status", {
        cache: "no-store",
        credentials: "same-origin",
        signal: controller.signal,
        redirect: "manual",
      });
      if (response.type === "opaqueredirect" || (response.status >= 300 && response.status < 400)
        || response.headers.get("content-type")?.includes("text/html")) {
        throw dashboardLocalFailure(new Error("Workspace sign-in redirect: reopen this workspace through its authenticated public URL."),"dashboard_auth_redirect","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
      }
      if (response.ok || response.status === 401 || response.status === 403) return response;
      lastError = dashboardLocalFailure(new Error(`Dashboard authentication status returned HTTP ${response.status}`),"dashboard_auth_status_unavailable","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
    } catch (error) {
      lastError = error;
    } finally {
      globalThis.clearTimeout(timeout);
    }
    if (attempt < attempts && retryDelayMs > 0) {
      await new Promise((resolve) => globalThis.setTimeout(resolve, retryDelayMs));
    }
  }

  throw dashboardLocalFailure(lastError as Error,"dashboard_auth_status_unavailable","packages/dashboard/src/lib/dashboard-auth-status.ts","fetchDashboardAuthStatus");
}
