import type { ServiceViewStream } from "@/lib/service-view-streams";

type GuacamoleAuthToken = {
  authToken?: string;
};

type GuacamoleActiveConnection = {
  connectionIdentifier?: string;
  connectable?: boolean;
  startDate?: number;
};

type GuacamoleSharingProfile = {
  identifier?: string;
  name?: string;
  primaryConnectionIdentifier?: string;
};

type GuacamoleSharingCredentials = {
  expected?: Array<{ name?: string; type?: string }>;
  values?: Record<string, string>;
};

type GuacamolePrimaryClaim = {
  granted?: boolean;
  retryAfterMs?: number;
};

export type GuacamoleViewerFrameResolution = {
  mode: "direct" | "shared";
  url: string;
};

function guacamoleRoot(frameUrl: string, dashboardHref: string): URL | null {
  try {
    const resolved = new URL(frameUrl, dashboardHref);
    if (resolved.origin !== new URL(dashboardHref).origin) return null;
    if (!/\/guacamole\/?$/i.test(resolved.pathname) || !resolved.hash.startsWith("#/client/")) return null;
    resolved.hash = "";
    resolved.search = "";
    if (!resolved.pathname.endsWith("/")) resolved.pathname += "/";
    return resolved;
  } catch {
    return null;
  }
}

function guacamoleSharingRoot(root: URL): URL {
  const labels = root.hostname.split(".");
  if (labels.length < 2 || !labels[0] || labels[0].endsWith("-share")) {
    throw new Error("Guacamole connection-sharing origin is unavailable");
  }
  const shared = new URL(root);
  labels[0] = `${labels[0]}-share`;
  shared.hostname = labels.join(".");
  return shared;
}

async function readJson<T>(response: Response, operation: string): Promise<T> {
  if (!response.ok) throw new Error(`${operation} returned HTTP ${response.status}`);
  return response.json() as Promise<T>;
}

function waitForElection(delayMs: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(signal.reason ?? new DOMException("Aborted", "AbortError"));
      return;
    }
    const onAbort = () => {
      clearTimeout(timer);
      reject(signal?.reason ?? new DOMException("Aborted", "AbortError"));
    };
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, delayMs);
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}

/**
 * Resolves a simultaneous-view Guacamole route into a connection-sharing URL.
 * The transient share key remains inside the iframe URL and is never returned
 * by the Agent Browser service API or persisted as an operator handoff.
 */
export async function resolveGuacamoleViewerFrame({
  dashboardHref,
  fetchImpl = globalThis.fetch,
  frameUrl,
  signal,
  stream,
  waitImpl = waitForElection,
  nowImpl = Date.now,
}: {
  dashboardHref: string;
  fetchImpl?: typeof globalThis.fetch;
  frameUrl: string;
  signal?: AbortSignal;
  stream: ServiceViewStream;
  waitImpl?: (delayMs: number, signal?: AbortSignal) => Promise<void>;
  nowImpl?: () => number;
}): Promise<GuacamoleViewerFrameResolution> {
  const root = guacamoleRoot(frameUrl, dashboardHref);
  const connectionId = stream.connectionId?.trim();
  const routeId = stream.routeId?.trim();
  if (stream.providerMode !== "simultaneous_view" || !root || !connectionId || !routeId) {
    return { mode: "direct", url: frameUrl };
  }

  const tokenResponse = await fetchImpl(new URL("api/tokens", root), {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: "",
    cache: "no-store",
    signal,
  });
  const token = await readJson<GuacamoleAuthToken>(tokenResponse, "Guacamole header authentication");
  if (!token.authToken) throw new Error("Guacamole header authentication returned no token");
  const authenticatedFetch = async <T>(path: string, operation: string): Promise<T> => readJson<T>(
    await fetchImpl(new URL(path, root), {
      credentials: "include",
      headers: { "Guacamole-Token": token.authToken! },
      cache: "no-store",
      signal,
    }),
    operation,
  );
  const claimPrimary = async (): Promise<GuacamolePrimaryClaim> => readJson<GuacamolePrimaryClaim>(
    await fetchImpl(new URL("/api/guacamole-primary-claim", dashboardHref), {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ routeId, connectionId }),
      cache: "no-store",
      signal,
    }),
    "Guacamole primary election",
  );

  const electionDeadline = nowImpl() + 15_000;
  let consecutiveEmptySnapshots = 0;
  while (nowImpl() < electionDeadline) {
    const activeConnections = await authenticatedFetch<Record<string, GuacamoleActiveConnection>>(
      "api/session/data/postgresql/activeConnections",
      "Guacamole active-connection discovery",
    );
    // Guacamole 1.5.5 does not identify which REST rows are primary versus
    // shared children. Probe every matching row through the restricted
    // sharing-profile endpoint instead of guessing from row shape.
    const activeCandidates = Object.entries(activeConnections)
      .filter(([, active]) => active.connectionIdentifier === connectionId)
      .sort(([leftId, left], [rightId, right]) => {
        const leftStart = Number.isFinite(left.startDate) ? left.startDate! : Number.NEGATIVE_INFINITY;
        const rightStart = Number.isFinite(right.startDate) ? right.startDate! : Number.NEGATIVE_INFINITY;
        return rightStart - leftStart || leftId.localeCompare(rightId);
      });
    if (activeCandidates.length === 0) {
      consecutiveEmptySnapshots += 1;
      if (consecutiveEmptySnapshots < 2) {
        await waitImpl(100, signal);
        continue;
      }
      const claim = await claimPrimary();
      if (claim.granted) return { mode: "direct", url: frameUrl };
      const retryAfterMs = Number.isFinite(claim.retryAfterMs) ? claim.retryAfterMs! : 250;
      await waitImpl(Math.max(50, Math.min(250, retryAfterMs)), signal);
      continue;
    }
    consecutiveEmptySnapshots = 0;

    const sharingProfiles = await authenticatedFetch<Record<string, GuacamoleSharingProfile>>(
      `api/session/data/postgresql/connections/${encodeURIComponent(connectionId)}/sharingProfiles`,
      "Guacamole sharing-profile discovery",
    );
    const expectedName = `Agent Browser Shared Session ${routeId}`;
    const sharingProfile = Object.values(sharingProfiles).find((profile) =>
      profile.name === expectedName && profile.primaryConnectionIdentifier === connectionId,
    );
    if (!sharingProfile?.identifier) {
      throw new Error("Guacamole simultaneous-view sharing profile is unavailable");
    }

    for (const [activeConnectionId] of activeCandidates) {
      const credentialsResponse = await fetchImpl(new URL(
        `api/session/data/postgresql/activeConnections/${encodeURIComponent(activeConnectionId)}/sharingCredentials/${encodeURIComponent(sharingProfile.identifier)}`,
        root,
      ), {
        credentials: "include",
        headers: { "Guacamole-Token": token.authToken },
        cache: "no-store",
        signal,
      });
      // A 404 invalidates only this active-row candidate. It is not evidence
      // that every matching provider connection disappeared or that direct
      // primary election is safe.
      if (credentialsResponse.status === 404) continue;
      const credentials = await readJson<GuacamoleSharingCredentials>(
        credentialsResponse,
        "Guacamole connection-sharing credential creation",
      );
      const keyExpected = credentials.expected?.some(
        (field) => field.name === "key" && field.type === "QUERY_PARAMETER",
      );
      const key = credentials.values?.key;
      if (!keyExpected || !key) throw new Error("Guacamole connection-sharing returned no usable key");

      const shared = guacamoleSharingRoot(root);
      shared.hash = `/?key=${encodeURIComponent(key)}`;
      return { mode: "shared", url: shared.toString() };
    }
    await waitImpl(100, signal);
  }
  throw new Error("Guacamole primary election timed out");
}
