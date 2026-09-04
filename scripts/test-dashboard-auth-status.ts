#!/usr/bin/env node

import assert from "node:assert/strict";
import { fetchDashboardAuthStatus } from "../packages/dashboard/src/lib/dashboard-auth-status.ts";

function hangingRequest(signal: AbortSignal | null | undefined): Promise<Response> {
  return new Promise((_resolve, reject) => {
    signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), {
      once: true,
    });
  });
}

let recoveringAttempts = 0;
const recoveringFetch: typeof fetch = async (_input, init) => {
  recoveringAttempts += 1;
  if (recoveringAttempts === 1) return hangingRequest(init?.signal);
  return new Response(JSON.stringify({ authenticated: true }), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
};

const recovered = await fetchDashboardAuthStatus(recoveringFetch, {
  attempts: 2,
  timeoutMs: 10,
  retryDelayMs: 0,
});
assert.equal(recovered.status, 200);
assert.equal(recoveringAttempts, 2);

let exhaustedAttempts = 0;
const exhaustedFetch: typeof fetch = async (_input, init) => {
  exhaustedAttempts += 1;
  return hangingRequest(init?.signal);
};
await assert.rejects(
  fetchDashboardAuthStatus(exhaustedFetch, { attempts: 2, timeoutMs: 10, retryDelayMs: 0 }),
  (error: unknown) => error instanceof DOMException && error.name === "AbortError",
);
assert.equal(exhaustedAttempts, 2);

let nonOkAttempts = 0;
const nonOkFetch: typeof fetch = async () => {
  nonOkAttempts += 1;
  return new Response("unavailable", { status: 503 });
};
await assert.rejects(
  fetchDashboardAuthStatus(nonOkFetch, { attempts: 2, timeoutMs: 10, retryDelayMs: 0 }),
  /HTTP 503/,
);
assert.equal(nonOkAttempts, 2);

console.log("Dashboard authentication status retry checks passed");
