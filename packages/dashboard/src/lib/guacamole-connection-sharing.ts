import type { ServiceViewStream } from "@/lib/service-view-streams";

type GuacamoleAuthToken = {
  authToken?: string;
};

type GuacamoleActiveConnection = {
  connectionIdentifier?: string;
  connectable?: boolean;
  sharingProfileIdentifier?: string | null;
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
  claimId?: string | null;
  revision?: string | null;
};

export type GuacamolePrimaryReservation = { claimId: string; revision: string; routeId: string; connectionId: string };

/** Read Guacamole 1.5.5's ManagedClient state, not iframe load or authentication.
 * The public revision binds this exact rendered frame without exposing its claim.
 */
export function isConnectedGuacamolePrimaryFrame(
  frame: HTMLIFrameElement | null, expectedUrl: string, revision: string,
): boolean {
  if (!frame || frame.dataset.guacamolePrimaryRevision !== revision || frame.src !== expectedUrl) return false;
  try {
    const win = frame.contentWindow as (Window & {
      angular?: { element?: (element: Element) => {
        scope?: () => { client?: { clientState?: { connectionState?: string } } } | undefined;
      } };
    }) | null;
    const doc = frame.contentDocument;
    if (!doc || win?.location.origin !== new URL(expectedUrl).origin) return false;
    for (const element of doc.querySelectorAll(".display, .client-tile, guac-client")) {
      if (win?.angular?.element?.(element).scope?.()?.client?.clientState?.connectionState === "CONNECTED") return true;
    }
  } catch { /* Cross-origin or detached frames cannot confirm a primary. */ }
  return false;
}

type GuacamoleActiveCandidate = readonly [string, GuacamoleActiveConnection];

const GUACAMOLE_PRIMARY_MINIMUM_AGE_MS = 3_000;
export const GUACAMOLE_SHARE_FRAME_NAME_PREFIX = "agent-browser-guacamole-share:";

export type GuacamoleShareAuthOutcome = "ready" | "share_key_rejected";

export type GuacamoleViewerFrameResolution =
  | { mode: "direct"; url: string; primaryReservation?: GuacamolePrimaryReservation }
  | {
    mode: "shared";
    url: string;
    primaryActiveConnectionId: string;
    attemptId: string;
  };

export function classifyGuacamoleShareAuthMessage({
  attemptId,
  data,
  eventOrigin,
  eventSource,
  expectedOrigin,
  expectedSource,
}: {
  attemptId: string;
  data: unknown;
  eventOrigin: string;
  eventSource: unknown;
  expectedOrigin: string;
  expectedSource: unknown;
}): GuacamoleShareAuthOutcome | null {
  if (!attemptId || eventOrigin !== expectedOrigin || !expectedSource || eventSource !== expectedSource) return null;
  if (!data || typeof data !== "object" || Array.isArray(data)) return null;
  const record = data as Record<string, unknown>;
  if (record.type !== "agent-browser-guacamole-share-auth" || record.attemptId !== attemptId) return null;
  return record.outcome === "ready" || record.outcome === "share_key_rejected"
    ? record.outcome
    : null;
}

function newShareAttemptId(): string {
  if (typeof globalThis.crypto?.randomUUID === "function") return globalThis.crypto.randomUUID();
  if (typeof globalThis.crypto?.getRandomValues !== "function") {
    throw new Error("Guacamole connection-sharing attempt identity cannot be generated securely");
  }
  const bytes = new Uint8Array(16);
  globalThis.crypto.getRandomValues(bytes);
  return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
}

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

async function waitWithinDeadline({
  deadline,
  delayMs,
  nowImpl,
  signal,
  waitImpl,
}: {
  deadline: number;
  delayMs: number;
  nowImpl: () => number;
  signal?: AbortSignal;
  waitImpl: (delayMs: number, signal?: AbortSignal) => Promise<void>;
}): Promise<boolean> {
  const remainingMs = deadline - nowImpl();
  if (remainingMs <= 0) return false;
  await waitImpl(Math.min(delayMs, remainingMs), signal);
  return nowImpl() < deadline;
}

/** Retire the exact startup claim only after the owned direct frame is connected.
 * Missing, replaced, cross-origin, or unready frames retain the original TTL.
 * This is not a viewer retry and never extends the election deadline.
 */
export async function confirmGuacamolePrimaryWhenConnected({
  reservation, dashboardHref, isConnected, signal,
  fetchImpl = globalThis.fetch, nowImpl = Date.now, waitImpl = waitForElection,
}: {
  reservation: GuacamolePrimaryReservation;
  dashboardHref: string;
  isConnected: () => boolean;
  signal: AbortSignal;
  fetchImpl?: typeof globalThis.fetch;
  nowImpl?: () => number;
  waitImpl?: (delayMs: number, signal?: AbortSignal) => Promise<void>;
}): Promise<boolean> {
  const deadline = nowImpl() + 30_000;
  while (!signal.aborted && nowImpl() < deadline) {
    if (isConnected()) {
      if (signal.aborted) return false;
      const response = await readJson<{ confirmed?: boolean }>(await fetchImpl(
        new URL("/api/guacamole-primary-claim", dashboardHref), {
          method: "POST", credentials: "include", cache: "no-store", signal,
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ operation: "connected", ...reservation }),
        },
      ), "Guacamole primary connection confirmation");
      return response.confirmed === true;
    }
    if (!await waitWithinDeadline({ deadline, delayMs: 100, nowImpl, signal, waitImpl })) break;
  }
  return false;
}

function matchingActiveConnections(
  activeConnections: Record<string, GuacamoleActiveConnection>,
  connectionId: string,
): GuacamoleActiveCandidate[] {
  return Object.entries(activeConnections)
    .filter(([, active]) => active.connectionIdentifier === connectionId);
}

function connectableCandidates(
  activeConnections: Record<string, GuacamoleActiveConnection>,
  connectionId: string,
  nowMs: number,
): GuacamoleActiveCandidate[] {
  return matchingActiveConnections(activeConnections, connectionId)
    .filter(([, active]) => active.connectable === true
      && active.sharingProfileIdentifier == null
      && (!Number.isFinite(active.startDate)
        || nowMs - active.startDate! >= GUACAMOLE_PRIMARY_MINIMUM_AGE_MS))
    .sort(([leftId, left], [rightId, right]) => {
      const leftStart = Number.isFinite(left.startDate) ? left.startDate! : Number.POSITIVE_INFINITY;
      const rightStart = Number.isFinite(right.startDate) ? right.startDate! : Number.POSITIVE_INFINITY;
      return leftStart - rightStart || leftId.localeCompare(rightId);
    });
}

function hasExactConnectableCandidate(
  activeConnections: Record<string, GuacamoleActiveConnection>,
  candidate: GuacamoleActiveCandidate,
): boolean {
  const [candidateId, expected] = candidate;
  const observed = activeConnections[candidateId];
  return observed?.connectionIdentifier === expected.connectionIdentifier
    && observed.connectable === true
    && observed.sharingProfileIdentifier == null
    && expected.sharingProfileIdentifier == null
    && observed.startDate === expected.startDate;
}

/**
 * Resolves a simultaneous-view Guacamole route into a connection-sharing URL.
 * The transient share key remains inside the iframe URL and is never returned
 * by the Agent Browser service API or persisted as an operator handoff.
 */
export async function resolveGuacamoleViewerFrame({
  attemptIdImpl = newShareAttemptId,
  dashboardHref,
  fetchImpl = globalThis.fetch,
  frameUrl,
  signal,
  stream,
  waitImpl = waitForElection,
  nowImpl = Date.now,
}: {
  attemptIdImpl?: () => string;
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
  let observedRevision: string | null = null;
  const claimPrimary = async (): Promise<GuacamolePrimaryClaim> => readJson<GuacamolePrimaryClaim>(
    await fetchImpl(new URL("/api/guacamole-primary-claim", dashboardHref), {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ routeId, connectionId, observedRevision }),
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
    // shared children. Prefer the oldest connectable row, then prove it remains
    // stable after key creation so a closing viewer cannot donate a dead key.
    const matchingRows = matchingActiveConnections(activeConnections, connectionId);
    // Guacamole includes shared child sessions in activeConnections. A key
    // minted from one of those rows dies with that viewer and strands the next
    // viewer on Guacamole's login screen. Only a direct primary may donate a
    // sharing credential. Some Guacamole APIs expose
    // sharingProfileIdentifier for shared children, while the installed 1.5.5
    // projection may omit it. The connectable, role, age, and post-mint
    // identity checks therefore act together rather than trusting one field.
    const activeCandidates = connectableCandidates(activeConnections, connectionId, nowImpl());
    if (matchingRows.length === 0) {
      consecutiveEmptySnapshots += 1;
      if (consecutiveEmptySnapshots < 2) {
        if (!await waitWithinDeadline({
          deadline: electionDeadline, delayMs: 100, nowImpl, signal, waitImpl,
        })) break;
        continue;
      }
      const claim = await claimPrimary();
      if (claim.granted) return {
        mode: "direct", url: frameUrl,
        ...(claim.claimId && claim.revision ? { primaryReservation: {
          claimId: claim.claimId, revision: claim.revision, routeId, connectionId,
        } } : {}),
      };
      observedRevision = claim.revision ?? null;
      // A revision is admission metadata, not provider-absence proof. Collect
      // both empty snapshots again after every denied or stale claim.
      consecutiveEmptySnapshots = 0;
      const retryAfterMs = Number.isFinite(claim.retryAfterMs) ? claim.retryAfterMs! : 250;
      if (!await waitWithinDeadline({
        deadline: electionDeadline,
        delayMs: Math.max(50, Math.min(250, retryAfterMs)),
        nowImpl,
        signal,
        waitImpl,
      })) break;
      continue;
    }
    consecutiveEmptySnapshots = 0;

    // A just-created direct primary is visible before it is old enough to
    // donate a reliable sharing key. Wait for maturity rather than electing a
    // second unrestricted primary.
    if (activeCandidates.length === 0) {
      if (!await waitWithinDeadline({
        deadline: electionDeadline, delayMs: 100, nowImpl, signal, waitImpl,
      })) break;
      continue;
    }

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

    for (const candidate of activeCandidates) {
      const [activeConnectionId] = candidate;
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

      let stable = true;
      for (let snapshot = 0; snapshot < 2; snapshot += 1) {
        if (!await waitWithinDeadline({
          deadline: electionDeadline, delayMs: 100, nowImpl, signal, waitImpl,
        })) {
          stable = false;
          break;
        }
        const validation = await authenticatedFetch<Record<string, GuacamoleActiveConnection>>(
          "api/session/data/postgresql/activeConnections",
          "Guacamole active-connection stability validation",
        );
        if (!hasExactConnectableCandidate(validation, candidate)) {
          stable = false;
          break;
        }
      }
      if (!stable) break;

      const shared = guacamoleSharingRoot(root);
      shared.hash = `/?key=${encodeURIComponent(key)}`;
      const attemptId = attemptIdImpl().trim();
      if (!attemptId) throw new Error("Guacamole connection-sharing attempt identity is unavailable");
      return {
        mode: "shared",
        url: shared.toString(),
        primaryActiveConnectionId: activeConnectionId,
        attemptId,
      };
    }
    if (!await waitWithinDeadline({
      deadline: electionDeadline, delayMs: 100, nowImpl, signal, waitImpl,
    })) break;
  }
  throw new Error("Guacamole primary election timed out");
}
