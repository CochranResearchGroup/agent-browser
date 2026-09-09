export type DashboardAuthStatusFetchOptions = {
  attempts?: number;
  timeoutMs?: number;
  retryDelayMs?: number;
};

const DEFAULT_ATTEMPTS = 3;
const DEFAULT_TIMEOUT_MS = 5_000;
const DEFAULT_RETRY_DELAY_MS = 250;

/**
 * Read the signed dashboard session without allowing a starved transport
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
    throw new Error("Dashboard authentication status attempts must be a positive integer");
  }
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new Error("Dashboard authentication status timeout must be positive");
  }
  if (!Number.isFinite(retryDelayMs) || retryDelayMs < 0) {
    throw new Error("Dashboard authentication status retry delay cannot be negative");
  }

  let lastError: unknown = new Error("Dashboard authentication status request failed");
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    const controller = new AbortController();
    const timeout = globalThis.setTimeout(() => controller.abort(), timeoutMs);
    try {
      const response = await fetchImpl("/api/dashboard-auth/status", {
        cache: "no-store",
        credentials: "same-origin",
        signal: controller.signal,
      });
      if (response.ok) return response;
      lastError = new Error(`Dashboard authentication status returned HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    } finally {
      globalThis.clearTimeout(timeout);
    }
    if (attempt < attempts && retryDelayMs > 0) {
      await new Promise((resolve) => globalThis.setTimeout(resolve, retryDelayMs));
    }
  }

  throw lastError;
}
