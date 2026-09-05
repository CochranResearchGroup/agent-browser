#!/usr/bin/env node

import assert from "node:assert/strict";

import {
  DashboardReadCoordinator,
  DashboardReadPausedError,
  createSessionTabPollPlanner,
  startCompletionDrivenDashboardPoll,
} from "../packages/dashboard/src/lib/dashboard-read-coordinator.ts";

let now = 1_000;
let visible = true;
let fetchCount = 0;
let transferredBytes = 0;
let releaseFetch: (() => void) | null = null;

const coordinator = new DashboardReadCoordinator({
  fetch: async () => {
    fetchCount += 1;
    await new Promise<void>((resolve) => {
      releaseFetch = resolve;
    });
    const body = JSON.stringify({ success: true, sequence: fetchCount });
    transferredBytes += Buffer.byteLength(body);
    return new Response(body, {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  },
  isVisible: () => visible,
  monotonicNow: () => now,
});

const first = coordinator.read("/api/service/status", { freshForMs: 10_000 });
const second = coordinator.read("/api/service/status", { freshForMs: 10_000 });
assert.equal(fetchCount, 1, "concurrent consumers must share one network request");
releaseFetch?.();
const [firstResponse, secondResponse] = await Promise.all([first, second]);
assert.deepEqual(await firstResponse.json(), await secondResponse.json());
assert.equal(fetchCount, 1);
assert.equal(
  transferredBytes,
  Buffer.byteLength(JSON.stringify({ success: true, sequence: 1 })),
  "shared consumers must transfer one response body",
);

await coordinator.read("/api/service/status", { freshForMs: 10_000 });
assert.equal(fetchCount, 1, "fresh snapshots must be reused");
now += 10_001;
const refreshed = coordinator.read("/api/service/status", { freshForMs: 10_000 });
assert.equal(fetchCount, 2, "expired snapshots must refresh once");
releaseFetch?.();
await refreshed;

now -= 20_000;
const correctedBackward = coordinator.read("/api/service/status", { freshForMs: 10_000 });
assert.equal(fetchCount, 3, "a backward clock observation must invalidate rather than pin stale state");
releaseFetch?.();
await correctedBackward;

const contracts = coordinator.read("/api/service/contracts", {
  cacheGroup: "runtime-static",
  freshForMs: 300_000,
});
releaseFetch?.();
await contracts;
now += 60_000;
await coordinator.read("/api/service/contracts", {
  cacheGroup: "runtime-static",
  freshForMs: 300_000,
});
assert.equal(fetchCount, 4, "static reads must stay shared within their bounded lifetime");
coordinator.invalidateCacheGroup("runtime-static");
const refreshedContracts = coordinator.read("/api/service/contracts", {
  cacheGroup: "runtime-static",
  freshForMs: 300_000,
});
assert.equal(fetchCount, 5, "a runtime identity change must invalidate static reads immediately");
releaseFetch?.();
await refreshedContracts;

visible = false;
await assert.rejects(
  coordinator.read("/api/service/resources", { freshForMs: 10_000 }),
  DashboardReadPausedError,
);
assert.equal(fetchCount, 5, "hidden documents must not start network reads");

visible = true;
let failureFetchCount = 0;
const failing = new DashboardReadCoordinator({
  fetch: async () => {
    failureFetchCount += 1;
    return new Response("gateway timeout", { status: 504 });
  },
  isVisible: () => true,
  monotonicNow: () => now,
});
const [failedA, failedB] = await Promise.all([
  failing.read("/api/service/resources", { freshForMs: 10_000 }),
  failing.read("/api/service/resources", { freshForMs: 10_000 }),
]);
assert.equal(failureFetchCount, 1, "one genuine failed flight must be shared, not retried");
assert.equal(failedA.status, 504, "the genuine failure must remain visible to every consumer");
assert.equal(failedB.status, 504);
await failing.read("/api/service/resources", { freshForMs: 10_000 });
assert.equal(failureFetchCount, 2, "failed responses must never enter the freshness cache");

let boundedFetchCount = 0;
const bounded = new DashboardReadCoordinator({
  fetch: async (input) => {
    boundedFetchCount += 1;
    const url = String(input);
    const body = url.endsWith("large") ? "123456789" : url.slice(-4).padEnd(4, "_");
    return new Response(body, { status: 200 });
  },
  isVisible: () => true,
  maxSnapshotBytes: 8,
  maxSnapshotEntries: 2,
  monotonicNow: () => now,
});
await bounded.read("/zero-ttl");
await bounded.read("/zero-ttl", { freshForMs: 10_000 });
assert.equal(
  boundedFetchCount,
  2,
  "a zero-TTL read must share only its active flight and must not seed persistent cache state",
);
await bounded.read("/one", { freshForMs: 10_000 });
await bounded.read("/two", { freshForMs: 10_000 });
await bounded.read("/three", { freshForMs: 10_000 });
await bounded.read("/one", { freshForMs: 10_000 });
assert.equal(boundedFetchCount, 6, "entry-budget eviction must remove the least recently used key");
await bounded.read("/large", { freshForMs: 10_000 });
await bounded.read("/large", { freshForMs: 10_000 });
assert.equal(boundedFetchCount, 8, "a response larger than the byte budget must never be retained");

let invalidationFetchCount = 0;
let releaseOldGeneration: (() => void) | null = null;
const invalidated = new DashboardReadCoordinator({
  fetch: async () => {
    invalidationFetchCount += 1;
    const generation = invalidationFetchCount;
    if (generation === 1) {
      await new Promise<void>((resolve) => {
        releaseOldGeneration = resolve;
      });
    }
    return new Response(`generation-${generation}`, { status: 200 });
  },
  isVisible: () => true,
  monotonicNow: () => now,
});
const oldGeneration = invalidated.read("/static", {
  cacheGroup: "runtime-static",
  freshForMs: 300_000,
});
invalidated.invalidateCacheGroup("runtime-static");
const newGeneration = invalidated.read("/static", {
  cacheGroup: "runtime-static",
  freshForMs: 300_000,
});
assert.equal(invalidationFetchCount, 2, "invalidation must not join an obsolete active flight");
releaseOldGeneration?.();
assert.equal(await newGeneration.then((response) => response.text()), "generation-2");
await oldGeneration;
assert.equal(
  await invalidated.read("/static", {
    cacheGroup: "runtime-static",
    freshForMs: 300_000,
  }).then((response) => response.text()),
  "generation-2",
  "an obsolete flight must not overwrite post-invalidation cache state",
);

const sessions = Array.from({ length: 20 }, (_, index) => ({
  session: `session-${index + 1}`,
  port: 9_200 + index,
  pid: 10_000 + index,
}));
const planner = createSessionTabPollPlanner(4);
const observed = new Set<number>();
let tabRequestCount = 0;
for (let cycle = 0; cycle < 7; cycle += 1) {
  const batch = planner.select(sessions, 9_213);
  assert.ok(batch.length <= 4, "each tab refresh cycle must have a fixed request bound");
  assert.ok(batch.some((session) => session.port === 9_213), "the selected session must stay fresh");
  tabRequestCount += batch.length;
  for (const session of batch) observed.add(session.port);
}
assert.equal(observed.size, 20, "bounded rotation must preserve eventual left-rail coverage");
assert.ok(
  tabRequestCount <= 28,
  "seven cycles over twenty sessions must stay bounded instead of issuing 140 tab requests",
);

type Timer = { callback: () => void; delay: number };
let pollVisible = true;
let visibilityListener: (() => void) | null = null;
const timers: Timer[] = [];
let pollRuns = 0;
let concurrentPolls = 0;
let maximumConcurrentPolls = 0;
let releasePoll: (() => void) | null = null;
const stop = startCompletionDrivenDashboardPoll(
  async () => {
    pollRuns += 1;
    concurrentPolls += 1;
    maximumConcurrentPolls = Math.max(maximumConcurrentPolls, concurrentPolls);
    await new Promise<void>((resolve) => {
      releasePoll = resolve;
    });
    concurrentPolls -= 1;
  },
  7_000,
  {
    isVisible: () => pollVisible,
    setTimeout: (callback, delay) => {
      timers.push({ callback, delay });
      return timers.length;
    },
    clearTimeout: () => undefined,
    subscribeVisibility: (listener) => {
      visibilityListener = listener;
      return () => {
        visibilityListener = null;
      };
    },
  },
);
assert.equal(pollRuns, 1);
assert.equal(timers.length, 0, "the next cycle must not schedule before completion");
releasePoll?.();
await new Promise((resolve) => setImmediate(resolve));
assert.equal(timers.length, 1);
assert.equal(timers[0].delay, 7_000);

pollVisible = false;
timers.shift()?.callback();
await new Promise((resolve) => setImmediate(resolve));
assert.equal(pollRuns, 1, "hidden pages must remain silent");
assert.equal(timers.length, 0);

pollVisible = true;
visibilityListener?.();
assert.equal(pollRuns, 2, "visibility restoration must request one fresh cycle");
visibilityListener?.();
assert.equal(pollRuns, 2, "visibility events must not overlap an active flight");
releasePoll?.();
await new Promise((resolve) => setImmediate(resolve));
assert.equal(maximumConcurrentPolls, 1);
stop();

console.log("dashboard read coordinator tests passed");
