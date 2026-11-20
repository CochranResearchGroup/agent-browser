import { responseFailureMetadata } from "./response-failure.ts";

export type ForeignCdpScreenshotFormat = "jpeg" | "png";

export type ForeignCdpScreenshot = {
  dataUrl: string;
  format: ForeignCdpScreenshotFormat;
  targetId: string | null;
  title: string | null;
  url: string | null;
};

export type ForeignCdpBorrowStatus = {
  active: boolean;
  grantId: string | null;
  owner: string | null;
  reason: string | null;
  issuedAt: string | null;
  expiresAt: string | null;
  allowedOperations: string[];
  lifecycleOwnership: false;
};

export type ForeignCdpInput =
  | {
      kind: "mouse";
      eventType: "mousePressed" | "mouseReleased" | "mouseMoved";
      x: number;
      y: number;
      button?: "none" | "left" | "middle" | "right" | "back" | "forward";
      clickCount?: number;
      modifiers?: number;
    }
  | {
      kind: "wheel";
      x: number;
      y: number;
      deltaX: number;
      deltaY: number;
    }
  | {
      kind: "keyboard";
      eventType: "keyDown" | "keyUp" | "char";
      key: string;
      code: string;
      text?: string;
      modifiers?: number;
    };

/** Builds the authenticated dashboard capture route for one foreign page target. */
export function foreignCdpScreenshotUrl(
  port: number,
  targetId?: string | null,
  format: ForeignCdpScreenshotFormat = "jpeg",
): string {
  const params = new URLSearchParams({
    port: String(port),
    format,
  });
  if (targetId?.trim()) params.set("targetId", targetId.trim());
  return `/api/session-screenshot?${params.toString()}`;
}

/** Captures one page target through the authenticated dashboard proxy. */
export async function fetchForeignCdpScreenshot({
  port,
  targetId,
  format = "png",
  fetcher = globalThis.fetch,
}: {
  port: number;
  targetId?: string | null;
  format?: ForeignCdpScreenshotFormat;
  fetcher?: typeof globalThis.fetch;
}): Promise<ForeignCdpScreenshot> {
  const response = await fetcher(foreignCdpScreenshotUrl(port, targetId, format), {
    cache: "no-store",
    credentials: "include",
  });
  const payload = await response.json() as {
    success?: boolean;
    dataUrl?: string | null;
    data?: string | null;
    format?: string | null;
    targetId?: string | null;
    title?: string | null;
    url?: string | null;
    error?: string | null;
  };
  if (!response.ok || responseFailureMetadata(payload)) {
    throw foreignCdpFailure(payload, `Foreign CDP capture returned HTTP ${response.status}.`);
  }
  const resolvedFormat: ForeignCdpScreenshotFormat = payload.format === "jpeg" ? "jpeg" : format;
  const dataUrl = payload.dataUrl
    || (payload.data ? `data:image/${resolvedFormat};base64,${payload.data}` : null);
  if (!dataUrl) throw new Error("Foreign CDP capture did not include image data.");
  return {
    dataUrl,
    format: resolvedFormat,
    targetId: payload.targetId ?? null,
    title: payload.title ?? null,
    url: payload.url ?? null,
  };
}

async function foreignCdpJsonRequest(
  path: string,
  init: RequestInit,
  fetcher: typeof globalThis.fetch,
): Promise<Record<string, unknown>> {
  const response = await fetcher(path, {
    ...init,
    cache: "no-store",
    credentials: "include",
    headers: init.body
      ? { "Content-Type": "application/json", ...init.headers }
      : init.headers,
  });
  const payload = await response.json() as Record<string, unknown>;
  if (!response.ok || responseFailureMetadata(payload)) {
    throw foreignCdpFailure(payload,
      typeof payload.error === "string"
        ? payload.error
        : `Foreign CDP control returned HTTP ${response.status}.`,
    );
  }
  return payload;
}

function normalizeBorrowStatus(payload: Record<string, unknown>): ForeignCdpBorrowStatus {
  return {
    active: payload.active === true,
    grantId: typeof payload.grantId === "string" ? payload.grantId : null,
    owner: typeof payload.owner === "string" ? payload.owner : null,
    reason: typeof payload.reason === "string" ? payload.reason : null,
    issuedAt: typeof payload.issuedAt === "string" ? payload.issuedAt : null,
    expiresAt: typeof payload.expiresAt === "string" ? payload.expiresAt : null,
    allowedOperations: Array.isArray(payload.allowedOperations)
      ? payload.allowedOperations.filter((value): value is string => typeof value === "string")
      : [],
    lifecycleOwnership: false,
  };
}

/** Read the current time-bounded input grant for one foreign CDP page. */
export async function readForeignCdpControlStatus({
  port,
  targetId,
  fetcher = globalThis.fetch,
}: {
  port: number;
  targetId: string;
  fetcher?: typeof globalThis.fetch;
}): Promise<ForeignCdpBorrowStatus> {
  const params = new URLSearchParams({ port: String(port), targetId });
  return normalizeBorrowStatus(await foreignCdpJsonRequest(
    `/api/foreign-cdp/control?${params.toString()}`,
    { method: "GET" },
    fetcher,
  ));
}

/** Request temporary pointer, keyboard, and wheel authority without lifecycle ownership. */
export async function borrowForeignCdpControl({
  port,
  targetId,
  reason,
  ttlSeconds = 300,
  fetcher = globalThis.fetch,
}: {
  port: number;
  targetId: string;
  reason: string;
  ttlSeconds?: number;
  fetcher?: typeof globalThis.fetch;
}): Promise<ForeignCdpBorrowStatus> {
  return normalizeBorrowStatus(await foreignCdpJsonRequest(
    "/api/foreign-cdp/borrow",
    { method: "POST", body: JSON.stringify({ port, targetId, reason, ttlSeconds }) },
    fetcher,
  ));
}

/** Release a temporary foreign-CDP input grant before its expiry. */
export async function releaseForeignCdpControl({
  port,
  targetId,
  grantId,
  fetcher = globalThis.fetch,
}: {
  port: number;
  targetId: string;
  grantId: string;
  fetcher?: typeof globalThis.fetch;
}): Promise<ForeignCdpBorrowStatus> {
  return normalizeBorrowStatus(await foreignCdpJsonRequest(
    "/api/foreign-cdp/release",
    { method: "POST", body: JSON.stringify({ port, targetId, grantId }) },
    fetcher,
  ));
}

/** Dispatch one fixed, server-validated input event under an active Borrow grant. */
export async function dispatchForeignCdpInput({
  port,
  targetId,
  grantId,
  input,
  fetcher = globalThis.fetch,
}: {
  port: number;
  targetId: string;
  grantId: string;
  input: ForeignCdpInput;
  fetcher?: typeof globalThis.fetch;
}): Promise<void> {
  await foreignCdpJsonRequest(
    "/api/foreign-cdp/input",
    { method: "POST", body: JSON.stringify({ port, targetId, grantId, input }) },
    fetcher,
  );
}

/** Preserve owner recourse on errors exposed by dashboard control adapters. */
function foreignCdpFailure(payload: unknown, fallback: string): Error {
  const envelope = payload as {failure?: unknown; data?: {failure?: unknown}};
  const metadata = responseFailureMetadata(payload);
  return Object.assign(new Error(metadata?.code ?? fallback), {
    code: metadata?.code ?? "foreign_cdp_request_failed",
    failure: envelope.failure ?? envelope.data?.failure ?? null,
    occurrenceId: metadata?.observationId ?? null,
  });
}
