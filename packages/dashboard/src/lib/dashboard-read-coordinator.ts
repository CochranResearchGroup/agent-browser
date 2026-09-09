"use client";

export type DashboardReadPolicy = {
  cacheGroup?: string;
  freshForMs?: number;
};

type ResponseSnapshot = {
  body: Blob;
  byteCount: number;
  cacheGroup?: string;
  cacheGroupRevision: number;
  completedAt: number;
  headers: [string, string][];
  status: number;
  statusText: string;
};

type DashboardReadFlight = {
  cacheGroup?: string;
  cacheGroupRevision: number;
  promise: Promise<ResponseSnapshot>;
};

type DashboardReadEnvironment = {
  fetch: typeof globalThis.fetch;
  isVisible: () => boolean;
  maxSnapshotBytes: number;
  maxSnapshotEntries: number;
  monotonicNow: () => number;
};

const DEFAULT_MAX_SNAPSHOT_BYTES = 24 * 1024 * 1024;
const DEFAULT_MAX_SNAPSHOT_ENTRIES = 16;

export class DashboardReadPausedError extends Error {
  constructor() {
    super("Dashboard reads are paused while this document is hidden.");
    this.name = "DashboardReadPausedError";
  }
}

/**
 * Coordinates idempotent dashboard reads inside one browser document.
 * Concurrent consumers share one request, successful responses may be reused
 * for a bounded freshness window, and failures are returned without retry or
 * caching so the original response remains observable.
 */
export class DashboardReadCoordinator {
  private readonly environment: DashboardReadEnvironment;
  private readonly flights = new Map<string, DashboardReadFlight>();
  private readonly cacheGroupRevisions = new Map<string, number>();
  private readonly snapshots = new Map<string, ResponseSnapshot>();
  private snapshotBytes = 0;

  constructor(environment: Partial<DashboardReadEnvironment> = {}) {
    this.environment = {
      fetch: environment.fetch ?? ((input, init) => globalThis.fetch(input, init)),
      isVisible: environment.isVisible ?? (() => (
        typeof document === "undefined" || document.visibilityState === "visible"
      )),
      maxSnapshotBytes: Math.max(
        0,
        Math.floor(environment.maxSnapshotBytes ?? DEFAULT_MAX_SNAPSHOT_BYTES),
      ),
      maxSnapshotEntries: Math.max(
        0,
        Math.floor(environment.maxSnapshotEntries ?? DEFAULT_MAX_SNAPSHOT_ENTRIES),
      ),
      monotonicNow: environment.monotonicNow ?? (() => globalThis.performance.now()),
    };
  }

  async read(input: RequestInfo | URL, policy: DashboardReadPolicy = {}): Promise<Response> {
    const key = input instanceof Request ? input.url : String(input);
    const cacheGroupRevision = this.cacheGroupRevision(policy.cacheGroup);
    const cached = this.snapshots.get(key);
    const freshForMs = Math.max(0, policy.freshForMs ?? 0);
    if (cached) {
      const ageMs = this.environment.monotonicNow() - cached.completedAt;
      if (
        cached.cacheGroup === policy.cacheGroup
        && cached.cacheGroupRevision === cacheGroupRevision
        && ageMs >= 0
        && ageMs < freshForMs
      ) {
        this.touchSnapshot(key, cached);
        return responseFromSnapshot(cached);
      }
      this.deleteSnapshot(key);
    }
    if (!this.environment.isVisible()) throw new DashboardReadPausedError();

    let flight = this.flights.get(key);
    if (
      flight
      && (flight.cacheGroup !== policy.cacheGroup
        || flight.cacheGroupRevision !== cacheGroupRevision)
    ) {
      flight = undefined;
    }
    if (!flight) {
      const promise = this.environment.fetch(input, {
        method: "GET",
        cache: "no-store",
        credentials: "same-origin",
      }).then(async (response) => {
        const body = await response.blob();
        const snapshot: ResponseSnapshot = {
          body,
          byteCount: body.size,
          cacheGroup: policy.cacheGroup,
          cacheGroupRevision,
          completedAt: this.environment.monotonicNow(),
          headers: Array.from(response.headers.entries()),
          status: response.status,
          statusText: response.statusText,
        };
        if (
          response.ok
          && freshForMs > 0
          && this.cacheGroupRevision(policy.cacheGroup) === cacheGroupRevision
        ) {
          this.storeSnapshot(key, snapshot);
        }
        return snapshot;
      }).finally(() => {
        if (this.flights.get(key)?.promise === promise) this.flights.delete(key);
      });
      flight = { cacheGroup: policy.cacheGroup, cacheGroupRevision, promise };
      this.flights.set(key, flight);
    }
    return responseFromSnapshot(await flight.promise);
  }

  invalidateCacheGroup(cacheGroup: string): void {
    this.cacheGroupRevisions.set(cacheGroup, this.cacheGroupRevision(cacheGroup) + 1);
    for (const [key, snapshot] of this.snapshots) {
      if (snapshot.cacheGroup === cacheGroup) this.deleteSnapshot(key);
    }
    for (const [key, flight] of this.flights) {
      if (flight.cacheGroup === cacheGroup) this.flights.delete(key);
    }
  }

  private cacheGroupRevision(cacheGroup?: string): number {
    return cacheGroup ? (this.cacheGroupRevisions.get(cacheGroup) ?? 0) : 0;
  }

  private deleteSnapshot(key: string): void {
    const snapshot = this.snapshots.get(key);
    if (!snapshot) return;
    this.snapshots.delete(key);
    this.snapshotBytes = Math.max(0, this.snapshotBytes - snapshot.byteCount);
  }

  private storeSnapshot(key: string, snapshot: ResponseSnapshot): void {
    this.deleteSnapshot(key);
    if (
      this.environment.maxSnapshotEntries === 0
      || snapshot.byteCount > this.environment.maxSnapshotBytes
    ) return;
    while (
      this.snapshots.size >= this.environment.maxSnapshotEntries
      || this.snapshotBytes + snapshot.byteCount > this.environment.maxSnapshotBytes
    ) {
      const oldestKey = this.snapshots.keys().next().value as string | undefined;
      if (oldestKey === undefined) return;
      this.deleteSnapshot(oldestKey);
    }
    this.snapshots.set(key, snapshot);
    this.snapshotBytes += snapshot.byteCount;
  }

  private touchSnapshot(key: string, snapshot: ResponseSnapshot): void {
    this.snapshots.delete(key);
    this.snapshots.set(key, snapshot);
  }
}

function responseFromSnapshot(snapshot: ResponseSnapshot): Response {
  return new Response(snapshot.body, {
    headers: snapshot.headers,
    status: snapshot.status,
    statusText: snapshot.statusText,
  });
}

export const dashboardReadCoordinator = new DashboardReadCoordinator();

export function fetchCoordinatedDashboardRead(
  input: RequestInfo | URL,
  policy: DashboardReadPolicy = {},
): Promise<Response> {
  return dashboardReadCoordinator.read(input, policy);
}

export function invalidateCoordinatedDashboardReadGroup(cacheGroup: string): void {
  dashboardReadCoordinator.invalidateCacheGroup(cacheGroup);
}

type DashboardPollEnvironment = {
  isVisible: () => boolean;
  setTimeout: (callback: () => void, delayMs: number) => unknown;
  clearTimeout: (timer: unknown) => void;
  subscribeVisibility: (listener: () => void) => () => void;
  onError?: (error: unknown) => void;
};

function browserPollEnvironment(): DashboardPollEnvironment {
  return {
    isVisible: () => typeof document === "undefined" || document.visibilityState === "visible",
    setTimeout: (callback, delayMs) => globalThis.setTimeout(callback, delayMs),
    clearTimeout: (timer) => globalThis.clearTimeout(timer as ReturnType<typeof setTimeout>),
    subscribeVisibility: (listener) => {
      if (typeof document === "undefined") return () => undefined;
      document.addEventListener("visibilitychange", listener);
      return () => document.removeEventListener("visibilitychange", listener);
    },
  };
}

/** Start one visibility-aware poll loop whose delay begins after completion. */
export function startCompletionDrivenDashboardPoll(
  task: () => void | Promise<void>,
  intervalMs: number,
  overrides: Partial<DashboardPollEnvironment> = {},
): () => void {
  const environment = { ...browserPollEnvironment(), ...overrides };
  let stopped = false;
  let running = false;
  let timer: unknown = null;

  const clearScheduled = () => {
    if (timer !== null) environment.clearTimeout(timer);
    timer = null;
  };
  const schedule = () => {
    clearScheduled();
    if (stopped || !environment.isVisible()) return;
    timer = environment.setTimeout(() => {
      timer = null;
      void run();
    }, Math.max(0, intervalMs));
  };
  const run = async () => {
    if (stopped || running || !environment.isVisible()) return;
    running = true;
    try {
      await task();
    } catch (error) {
      environment.onError?.(error);
    } finally {
      running = false;
      schedule();
    }
  };
  const unsubscribe = environment.subscribeVisibility(() => {
    clearScheduled();
    if (environment.isVisible()) void run();
  });
  void run();

  return () => {
    stopped = true;
    clearScheduled();
    unsubscribe();
  };
}

export type SessionTabPollCandidate = {
  session: string;
  port: number;
  pid?: number;
  engine?: string;
  provider?: string;
  cdpPort?: number;
  ownership?: string;
  addressability?: string;
};

export type SessionTabPollPlanner = {
  select: <T extends SessionTabPollCandidate>(sessions: T[], activePort: number) => T[];
};

/**
 * Keeps the selected session current while rotating a bounded tab-read budget
 * across new, changed, and inactive live sessions for eventual rail accuracy.
 */
export function createSessionTabPollPlanner(maximumPerCycle = 4): SessionTabPollPlanner {
  const limit = Math.max(1, Math.floor(maximumPerCycle));
  const fingerprints = new Map<number, string>();
  let cursor = 0;

  return {
    select<T extends SessionTabPollCandidate>(sessions: T[], activePort: number): T[] {
      const live = sessions.filter((session) => session.port > 0);
      const livePorts = new Set(live.map((session) => session.port));
      for (const port of fingerprints.keys()) {
        if (!livePorts.has(port)) fingerprints.delete(port);
      }
      if (live.length === 0) return [];

      const selected: T[] = [];
      const selectedPorts = new Set<number>();
      const add = (session: T) => {
        if (selected.length >= limit || selectedPorts.has(session.port)) return;
        selected.push(session);
        selectedPorts.add(session.port);
        fingerprints.set(session.port, sessionFingerprint(session));
      };
      const active = live.find((session) => session.port === activePort);
      if (active) add(active);

      for (const session of live) {
        if (selected.length >= limit) break;
        if (fingerprints.get(session.port) !== sessionFingerprint(session)) add(session);
      }

      let visited = 0;
      while (selected.length < limit && visited < live.length) {
        const session = live[cursor % live.length];
        cursor = (cursor + 1) % live.length;
        visited += 1;
        add(session);
      }
      return selected;
    },
  };
}

function sessionFingerprint(session: SessionTabPollCandidate): string {
  return JSON.stringify([
    session.session,
    session.port,
    session.pid ?? null,
    session.engine ?? null,
    session.provider ?? null,
    session.cdpPort ?? null,
    session.ownership ?? null,
    session.addressability ?? null,
  ]);
}
