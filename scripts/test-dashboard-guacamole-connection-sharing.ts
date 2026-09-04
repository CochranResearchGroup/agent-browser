#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolveGuacamoleViewerFrame } from "../packages/dashboard/src/lib/guacamole-connection-sharing.ts";

const viewportSource = readFileSync(
  "packages/dashboard/src/components/workspace-remote-viewport.tsx",
  "utf8",
);

assert.match(
  viewportSource,
  /const guacamoleSharingStream = useMemo<ServiceViewStream \| null>/,
  "the sharing effect must receive a stream descriptor stabilized across projection rerenders",
);
assert.match(
  viewportSource,
  /\[stream\?\.connectionId, stream\?\.displayAllocationId, stream\?\.provider, stream\?\.providerMode, stream\?\.routeId\]/,
  "only semantic Guacamole route fields may invalidate the sharing descriptor",
);
assert.match(
  viewportSource,
  /\}, \[browser\?\.id, browser\?\.profileId, guacamoleSharingStream, sharingResolutionNonce, streamUrl, viewportSelection\?\.selection\.sessionId, viewportTargetToken\]\);/,
  "projection object churn must not regenerate a share credential or remount the iframe",
);
assert.match(
  viewportSource,
  /stream\?\.providerMode === "simultaneous_view"[\s\S]*setSharingResolutionNonce\(\(current\) => current \+ 1\)/,
  "a disconnected simultaneous viewer must re-enter primary election instead of requiring takeover",
);
assert.match(
  viewportSource,
  /sharingRecoveryRetryRef\.current < 3/,
  "automatic simultaneous-view re-election must remain bounded",
);

const direct = "https://dashboard.example.test/guacamole/#/client/direct-id";
const stream = {
  connectionId: "17",
  provider: "rdp_gateway",
  providerMode: "simultaneous_view",
  routeId: "route-1",
};
const requests: Array<{ url: string; init?: RequestInit }> = [];
const response = (value: unknown) => new Response(JSON.stringify(value), {
  status: 200,
  headers: { "content-type": "application/json" },
});
const fetchImpl = async (input: string | URL | Request, init?: RequestInit) => {
  const url = String(input);
  requests.push({ url, init });
  if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
  if (url.endsWith("/activeConnections")) {
    return response({
      "shared-child-uuid": {
        connectionIdentifier: "17",
        sharingProfileIdentifier: "29",
      },
      "active-uuid": {
        connectionIdentifier: "17",
        sharingProfileIdentifier: null,
      },
    });
  }
  if (url.endsWith("/connections/17/sharingProfiles")) {
    return response({
      "29": {
        identifier: "29",
        name: "Agent Browser Shared Session route-1",
        primaryConnectionIdentifier: "17",
      },
    });
  }
  if (url.endsWith("/activeConnections/active-uuid/sharingCredentials/29")) {
    return response({
      expected: [{ name: "key", type: "QUERY_PARAMETER" }],
      values: { key: "share-secret" },
    });
  }
  throw new Error(`Unexpected request: ${url}`);
};

const shared = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  fetchImpl: fetchImpl as typeof globalThis.fetch,
  frameUrl: direct,
  stream,
});
assert.equal(shared.mode, "shared");
assert.equal(shared.url, "https://dashboard-share.example.test/guacamole/#/?key=share-secret");
assert.equal(requests.length, 4);
assert.equal(requests[0].init?.method, "POST");
assert.equal(new Headers(requests[1].init?.headers).get("Guacamole-Token"), "auth-secret");

const sharedOnly = await resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/",
    frameUrl: direct,
    stream,
    fetchImpl: (async (input: string | URL | Request) => {
      const url = String(input);
      if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
      if (url.endsWith("/activeConnections")) {
        return response({
          "shared-child-uuid": {
            connectionIdentifier: "17",
            sharingProfileIdentifier: "29",
          },
        });
      }
      if (url.endsWith("/api/guacamole-primary-claim")) {
        return response({ success: true, granted: true, retryAfterMs: 10_000 });
      }
      throw new Error(`Unexpected request: ${url}`);
    }) as typeof globalThis.fetch,
  });
assert.deepEqual(sharedOnly, { mode: "direct", url: direct });

let vanishedDiscoveryCount = 0;
const vanishedPrimary = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      vanishedDiscoveryCount += 1;
      if (vanishedDiscoveryCount > 1) return response({});
      return response({
        "active-uuid": {
          connectionIdentifier: "17",
          sharingProfileIdentifier: null,
        },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) {
      return response({
        "29": {
          identifier: "29",
          name: "Agent Browser Shared Session route-1",
          primaryConnectionIdentifier: "17",
        },
      });
    }
    if (url.endsWith("/activeConnections/active-uuid/sharingCredentials/29")) {
      return new Response("not found", { status: 404 });
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assert.deepEqual(vanishedPrimary, { mode: "direct", url: direct });

let stalePrimaryClaimCount = 0;
const stalePrimary = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "stale-primary": { connectionIdentifier: "17", sharingProfileIdentifier: null },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) {
      return response({
        "29": {
          identifier: "29",
          name: "Agent Browser Shared Session route-1",
          primaryConnectionIdentifier: "17",
        },
      });
    }
    if (url.endsWith("/activeConnections/stale-primary/sharingCredentials/29")) {
      return new Response("not found", { status: 404 });
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      stalePrimaryClaimCount += 1;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assert.deepEqual(stalePrimary, { mode: "direct", url: direct });
assert.equal(stalePrimaryClaimCount, 1);

const noActive = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) return response({});
    if (url.endsWith("/api/guacamole-primary-claim")) {
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
});
assert.deepEqual(noActive, { mode: "direct", url: direct });

let discoveryCount = 0;
let claimHeld = false;
const concurrentFetch = async (input: string | URL | Request) => {
  const url = String(input);
  if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
  if (url.endsWith("/activeConnections")) {
    discoveryCount += 1;
    if (discoveryCount <= 2 || !claimHeld) return response({});
    return response({
      "elected-primary": {
        connectionIdentifier: "17",
        sharingProfileIdentifier: null,
      },
    });
  }
  if (url.endsWith("/api/guacamole-primary-claim")) {
    if (!claimHeld) {
      claimHeld = true;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    return response({ success: true, granted: false, retryAfterMs: 10_000 });
  }
  if (url.endsWith("/connections/17/sharingProfiles")) {
    return response({
      "29": {
        identifier: "29",
        name: "Agent Browser Shared Session route-1",
        primaryConnectionIdentifier: "17",
      },
    });
  }
  if (url.endsWith("/activeConnections/elected-primary/sharingCredentials/29")) {
    return response({
      expected: [{ name: "key", type: "QUERY_PARAMETER" }],
      values: { key: "share-secret" },
    });
  }
  throw new Error(`Unexpected request: ${url}`);
};
const concurrent = await Promise.all([
  resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/remote-view/opaque",
    frameUrl: direct,
    stream,
    fetchImpl: concurrentFetch as typeof globalThis.fetch,
    waitImpl: async () => {},
  }),
  resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/remote-view/opaque",
    frameUrl: direct,
    stream,
    fetchImpl: concurrentFetch as typeof globalThis.fetch,
    waitImpl: async () => {},
  }),
]);
assert.deepEqual(concurrent.map((entry) => entry.mode).sort(), ["direct", "shared"]);

const localCrossOrigin = await resolveGuacamoleViewerFrame({
  dashboardHref: "http://127.0.0.1:4949/",
  frameUrl: "http://127.0.0.1:8093/guacamole/#/client/direct-id",
  stream,
  fetchImpl: (() => { throw new Error("must not fetch"); }) as typeof globalThis.fetch,
});
assert.deepEqual(localCrossOrigin, {
  mode: "direct",
  url: "http://127.0.0.1:8093/guacamole/#/client/direct-id",
});

console.log("Dashboard Guacamole connection-sharing behavior passed");
