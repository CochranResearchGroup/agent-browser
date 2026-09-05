#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  classifyGuacamoleShareAuthMessage,
  confirmGuacamolePrimaryWhenConnected,
  isConnectedGuacamolePrimaryFrame,
  resolveGuacamoleViewerFrame,
} from "../packages/dashboard/src/lib/guacamole-connection-sharing.ts";

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
assert.match(
  viewportSource,
  /setViewerFrameUrl\(null\);[\s\S]{0,800}resolveGuacamoleViewerFrame/,
  "reconnect must remove the old iframe before resolving a fresh sharing key",
);
assert.match(
  viewportSource,
  /resolveGuacamoleViewerFrame\([\s\S]{0,500}\.then\(\(resolution\)[\s\S]{0,700}setViewerFrameUrl\(resolution\.url\)/,
  "only a completed fresh resolution may install an iframe URL",
);
assert.match(
  viewportSource,
  /\.catch\(\(cause\)[\s\S]{0,600}type: "preflight_failed"[\s\S]{0,600}code: "guacamole_connection_sharing_failed"/,
  "terminal reconnect failure must remain explicit in UI state and the failure journal",
);
assert.match(
  viewportSource,
  /classifyGuacamoleShareAuthMessage/,
  "a shared resolution must validate the sibling-origin authentication signal",
);
assert.match(
  viewportSource,
  /outcome === "ready"[\s\S]{0,1000}setViewerFrameUrl\(null\)[\s\S]{0,800}code: "guacamole_share_key_rejected"[\s\S]{0,800}setSharingResolutionNonce\(\(current\) => current \+ 1\)/,
  "a rejected key must be removed, recorded, and replaced through bounded fresh election",
);
assert.match(
  viewportSource,
  /name=\{viewerShareAttempt \? `\$\{GUACAMOLE_SHARE_FRAME_NAME_PREFIX\}\$\{viewerShareAttempt\.attemptId\}` : undefined\}/,
  "the sibling frame must receive only its nonce-bound attempt identity",
);

const expectedShareFrame = {};
const unrelatedShareFrame = {};
const shareAuthMessage = (outcome: string, attemptId = "attempt-expected") => ({
  type: "agent-browser-guacamole-share-auth",
  attemptId,
  outcome,
});
const classifyShareAuth = ({
  data = shareAuthMessage("ready"),
  eventOrigin = "https://dashboard-share.example.test",
  eventSource = expectedShareFrame,
}: {
  data?: unknown;
  eventOrigin?: string;
  eventSource?: unknown;
} = {}) => classifyGuacamoleShareAuthMessage({
  attemptId: "attempt-expected",
  data,
  eventOrigin,
  eventSource,
  expectedOrigin: "https://dashboard-share.example.test",
  expectedSource: expectedShareFrame,
});
assert.equal(classifyShareAuth(), "ready");
assert.equal(classifyShareAuth({ data: shareAuthMessage("share_key_rejected") }), "share_key_rejected");
assert.equal(classifyShareAuth({ eventOrigin: "https://attacker.example.test" }), null);
assert.equal(classifyShareAuth({ eventSource: unrelatedShareFrame }), null);
assert.equal(classifyShareAuth({ data: shareAuthMessage("ready", "attempt-stale") }), null);
assert.equal(classifyShareAuth({
  data: { type: "agent-browser-guacamole-other", attemptId: "attempt-expected", outcome: "ready" },
}), null);
assert.equal(classifyShareAuth({ data: shareAuthMessage("unexpected") }), null);

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

const primaryFrame = (state: string) => ({
  src: direct, dataset: { guacamolePrimaryRevision: "rendered-revision" },
  contentDocument: { querySelectorAll: () => [{}] },
  contentWindow: {
    location: { origin: "https://dashboard.example.test" },
    angular: { element: () => ({ scope: () => ({ client: { clientState: { connectionState: state } } }) }) },
  },
}) as unknown as HTMLIFrameElement;
assert.equal(isConnectedGuacamolePrimaryFrame(primaryFrame("CONNECTED"), direct, "rendered-revision"), true);
for (const state of ["IDLE", "CONNECTING", "WAITING", "DISCONNECTED", "CLIENT_ERROR", "TUNNEL_ERROR"]) {
  assert.equal(isConnectedGuacamolePrimaryFrame(primaryFrame(state), direct, "rendered-revision"), false);
}
assert.equal(isConnectedGuacamolePrimaryFrame(primaryFrame("CONNECTED"), direct, "replaced-revision"), false);
assert.equal(isConnectedGuacamolePrimaryFrame(primaryFrame("CONNECTED"), `${direct}-other`, "rendered-revision"), false);
assert.equal(isConnectedGuacamolePrimaryFrame(null, direct, "rendered-revision"), false);
const inaccessibleFrame = primaryFrame("CONNECTED");
Object.defineProperty(inaccessibleFrame, "contentWindow", { get() { throw new Error("cross-origin"); } });
assert.equal(isConnectedGuacamolePrimaryFrame(inaccessibleFrame, direct, "rendered-revision"), false);

// Reproduce a confirmed primary closing within the original 30-second startup
// TTL. A stale revision must force two new empty observations before admission.
let fencedNow = 0;
const fencedSteps: string[] = [];
let fencedClaims = 0;
const fencedResolution = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct, stream, nowImpl: () => fencedNow,
  waitImpl: async (delay) => { fencedNow += delay; },
  fetchImpl: (async (input, init) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "private-auth" });
    if (url.endsWith("/activeConnections")) { fencedSteps.push("empty"); return response({}); }
    assert.ok(url.endsWith("/api/guacamole-primary-claim"));
    fencedSteps.push("claim");
    fencedClaims += 1;
    const body = JSON.parse(String(init?.body));
    assert.equal(body.observedRevision, fencedClaims === 1 ? null : "connected-revision");
    return response(fencedClaims === 1
      ? { granted: false, retryAfterMs: 0, revision: "connected-revision" }
      : { granted: true, retryAfterMs: 30_000, claimId: "private-claim", revision: "next-revision" });
  }) as typeof globalThis.fetch,
});
assert.deepEqual(fencedSteps, ["empty", "empty", "claim", "empty", "empty", "claim"]);
assert.equal(fencedResolution.mode, "direct");
assert.ok(fencedResolution.mode === "direct" && fencedResolution.primaryReservation);
const reservation = fencedResolution.primaryReservation;
assert.deepEqual(reservation, { claimId: "private-claim", revision: "next-revision", routeId: "route-1", connectionId: "17" });
assert.ok(fencedNow < 15_000, "the original election budget is not extended");
let confirmationNow = 0;
let confirmationRequests = 0;
assert.equal(await confirmGuacamolePrimaryWhenConnected({
  reservation, dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  signal: new AbortController().signal, nowImpl: () => confirmationNow,
  waitImpl: async (delay) => { confirmationNow += delay; },
  isConnected: () => confirmationNow >= 300,
  fetchImpl: (async (url, init) => {
    confirmationRequests += 1;
    assert.equal(String(url), "https://dashboard.example.test/api/guacamole-primary-claim");
    assert.equal(init?.credentials, "include");
    assert.deepEqual(JSON.parse(String(init?.body)), { operation: "connected", ...reservation });
    return response({ confirmed: true });
  }) as typeof globalThis.fetch,
}), true);
assert.equal(confirmationRequests, 1);
assert.equal(confirmationNow, 300);
for (const aborted of [false, true]) {
  let now = 0;
  const controller = new AbortController();
  if (aborted) controller.abort();
  assert.equal(await confirmGuacamolePrimaryWhenConnected({
    reservation, dashboardHref: "https://dashboard.example.test/", signal: controller.signal,
    nowImpl: () => now, waitImpl: async (delay) => { now += delay; },
    isConnected: () => aborted,
    fetchImpl: (async () => { throw new Error("an unready or cancelled frame cannot confirm"); }) as typeof globalThis.fetch,
  }), false);
  assert.equal(now, aborted ? 0 : 30_000);
}
function assertSharedResolution(
  actual: Awaited<ReturnType<typeof resolveGuacamoleViewerFrame>>,
  expectedUrl: string,
  expectedDonor: string,
): void {
  assert.equal(actual.mode, "shared");
  assert.equal(actual.url, expectedUrl);
  assert.equal(actual.primaryActiveConnectionId, expectedDonor);
  assert.match(actual.attemptId, /^[A-Za-z0-9._:-]{8,256}$/);
}
const fetchImpl = async (input: string | URL | Request, init?: RequestInit) => {
  const url = String(input);
  requests.push({ url, init });
  if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
  if (url.endsWith("/activeConnections")) {
    return response({
      "shared-child-uuid": {
        connectionIdentifier: "17",
        connectable: true,
        sharingProfileIdentifier: "29",
        startDate: 200,
      },
      "active-uuid": {
        connectionIdentifier: "17",
        connectable: true,
        startDate: 100,
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
assertSharedResolution(
  shared,
  "https://dashboard-share.example.test/guacamole/#/?key=share-secret",
  "active-uuid",
);
assert.equal(requests.length, 6);
assert.equal(requests[0].init?.method, "POST");
assert.equal(new Headers(requests[1].init?.headers).get("Guacamole-Token"), "auth-secret");
const activeConnectionSnapshots = requests.filter(({ url }) => url.endsWith("/activeConnections"));
assert.equal(activeConnectionSnapshots.length, 3);
assert.ok(activeConnectionSnapshots.every(({ init }) => init?.cache === "no-store"));

let sharedOnlyNow = 0;
let sharedOnlyMintCount = 0;
let sharedOnlyClaimCount = 0;
await assert.rejects(() => resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/",
    frameUrl: direct,
    stream,
    nowImpl: () => sharedOnlyNow,
    waitImpl: async (delayMs) => { sharedOnlyNow += delayMs; },
    fetchImpl: (async (input: string | URL | Request) => {
      const url = String(input);
      if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
      if (url.endsWith("/activeConnections")) {
        return response({
          "shared-child-uuid": {
            connectionIdentifier: "17",
            connectable: true,
            sharingProfileIdentifier: "29",
            startDate: 100,
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
      if (url.endsWith("/activeConnections/shared-child-uuid/sharingCredentials/29")) {
        sharedOnlyMintCount += 1;
        return response({
          expected: [{ name: "key", type: "QUERY_PARAMETER" }],
          values: { key: "shared-child-secret" },
        });
      }
      if (url.endsWith("/api/guacamole-primary-claim")) {
        sharedOnlyClaimCount += 1;
        return response({ success: true, granted: true, retryAfterMs: 10_000 });
      }
      throw new Error(`Unexpected request: ${url}`);
    }) as typeof globalThis.fetch,
  }), /Guacamole primary election timed out/);
assert.equal(sharedOnlyClaimCount, 0);
assert.equal(sharedOnlyMintCount, 0, "a shared child must never donate another sharing key");

const profileResponse = () => response({
  "29": {
    identifier: "29",
    name: "Agent Browser Shared Session route-1",
    primaryConnectionIdentifier: "17",
  },
});
const credentialResponse = (key: string) => response({
  expected: [{ name: "key", type: "QUERY_PARAMETER" }],
  values: { key },
});

const staleCandidateRequests: string[] = [];
let staleCandidateClaimed = false;
const staleCandidateRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "stale-former-primary": { connectionIdentifier: "17", connectable: true, startDate: 100 },
        "live-shared-child": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      staleCandidateRequests.push(url);
      if (url.includes("/stale-former-primary/")) return new Response("not found", { status: 404 });
      if (url.includes("/live-shared-child/")) return credentialResponse("surviving-child-secret");
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      staleCandidateClaimed = true;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
});
assertSharedResolution(
  staleCandidateRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=surviving-child-secret",
  "live-shared-child",
);
assert.deepEqual(staleCandidateRequests.map((url) =>
  url.match(/activeConnections\/([^/]+)/)?.[1]), ["stale-former-primary", "live-shared-child"]);
assert.equal(staleCandidateClaimed, false);

let postMintDiscoveryCount = 0;
const postMintCredentialRequests: string[] = [];
const postMintRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      postMintDiscoveryCount += 1;
      return response({
        ...(postMintDiscoveryCount === 1 ? {
          "closing-oldest": { connectionIdentifier: "17", connectable: true, startDate: 100 },
        } : {}),
        "stable-successor": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      const candidateId = url.match(/activeConnections\/([^/]+)/)?.[1];
      assert.ok(candidateId);
      postMintCredentialRequests.push(candidateId);
      return credentialResponse(`${candidateId}-secret`);
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assertSharedResolution(
  postMintRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=stable-successor-secret",
  "stable-successor",
);
assert.deepEqual(postMintCredentialRequests, ["closing-oldest", "stable-successor"]);
assert.equal(postMintDiscoveryCount, 5, "the returned key requires two stable post-mint relists");

let changedRepresentationDiscoveryCount = 0;
const changedRepresentationRequests: string[] = [];
const changedRepresentationRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      changedRepresentationDiscoveryCount += 1;
      return response({
        "changing-row": {
          connectionIdentifier: "17",
          connectable: true,
          sharingProfileIdentifier: changedRepresentationDiscoveryCount === 1 ? undefined : "29",
          startDate: 100,
        },
        "stable-row": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      const candidateId = url.match(/activeConnections\/([^/]+)/)?.[1];
      assert.ok(candidateId);
      changedRepresentationRequests.push(candidateId);
      return credentialResponse(`${candidateId}-secret`);
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assertSharedResolution(
  changedRepresentationRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=stable-row-secret",
  "stable-row",
);
assert.deepEqual(changedRepresentationRequests, ["changing-row", "stable-row"]);

let missingDateSnapshotCount = 0;
const missingDateRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      missingDateSnapshotCount += 1;
      return response({ "missing-date-row": { connectionIdentifier: "17", connectable: true } });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) return credentialResponse("missing-date-secret");
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assertSharedResolution(
  missingDateRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=missing-date-secret",
  "missing-date-row",
);
assert.equal(missingDateSnapshotCount, 3);

const nonConnectableCredentialRequests: string[] = [];
const connectableRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "older-not-connectable": { connectionIdentifier: "17", connectable: false, startDate: 100 },
        "stable-connectable": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      const candidateId = url.match(/activeConnections\/([^/]+)/)?.[1];
      assert.ok(candidateId);
      nonConnectableCredentialRequests.push(candidateId);
      return credentialResponse(`${candidateId}-secret`);
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assertSharedResolution(
  connectableRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=stable-connectable-secret",
  "stable-connectable",
);
assert.deepEqual(nonConnectableCredentialRequests, ["stable-connectable"]);

let nonConnectableNow = 0;
let nonConnectableClaimCount = 0;
let nonConnectableMintCount = 0;
await assert.rejects(() => resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  nowImpl: () => nonConnectableNow,
  waitImpl: async (delayMs) => { nonConnectableNow += delayMs; },
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "closing-row": { connectionIdentifier: "17", connectable: false, startDate: 100 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      nonConnectableMintCount += 1;
      return credentialResponse("must-not-be-used");
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      nonConnectableClaimCount += 1;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
}), /Guacamole primary election timed out/);
assert.equal(nonConnectableMintCount, 0);
assert.equal(nonConnectableClaimCount, 0);

const primaryRoleCredentialRequests: string[] = [];
const primaryRoleRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "older-shared-child": {
          connectionIdentifier: "17",
          connectable: true,
          sharingProfileIdentifier: "29",
          startDate: 100,
        },
        "direct-primary": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      const candidateId = url.match(/activeConnections\/([^/]+)/)?.[1];
      assert.ok(candidateId);
      primaryRoleCredentialRequests.push(candidateId);
      return credentialResponse(`${candidateId}-secret`);
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
});
assertSharedResolution(
  primaryRoleRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=direct-primary-secret",
  "direct-primary",
);
assert.deepEqual(primaryRoleCredentialRequests, ["direct-primary"]);

let youngPrimaryNow = 100_000;
let youngPrimaryMintedAt: number | null = null;
const youngPrimaryStartDate = youngPrimaryNow - 100;
const maturePrimaryRecovery = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  nowImpl: () => youngPrimaryNow,
  waitImpl: async (delayMs) => { youngPrimaryNow += delayMs; },
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "starting-primary": {
          connectionIdentifier: "17",
          connectable: true,
          startDate: youngPrimaryStartDate,
        },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.endsWith("/activeConnections/starting-primary/sharingCredentials/29")) {
      youngPrimaryMintedAt = youngPrimaryNow;
      return credentialResponse("mature-primary-secret");
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
});
assertSharedResolution(
  maturePrimaryRecovery,
  "https://dashboard-share.example.test/guacamole/#/?key=mature-primary-secret",
  "starting-primary",
);
assert.ok(youngPrimaryMintedAt !== null);
assert.ok(
  youngPrimaryMintedAt - youngPrimaryStartDate >= 3_000,
  "a newly connectable primary must survive its startup window before donating a share key",
);

let deadlineNow = 4_000;
let deadlineValidationCount = 0;
await assert.rejects(() => resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  nowImpl: () => deadlineNow,
  waitImpl: async (delayMs) => { deadlineNow += delayMs; },
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      deadlineValidationCount += 1;
      return response({
        "deadline-row": { connectionIdentifier: "17", connectable: true, startDate: 100 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      deadlineNow = 18_950;
      return credentialResponse("too-late-secret");
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
}), /Guacamole primary election timed out/);
assert.equal(deadlineValidationCount, 1, "post-mint waits must not start a snapshot after the deadline");
assert.equal(deadlineNow, 19_000);

const deterministicCandidateRequests: string[] = [];
await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "inserted-first-but-older": { connectionIdentifier: "17", connectable: true, startDate: 100 },
        "z-newer": { connectionIdentifier: "17", connectable: true, startDate: 200 },
        "a-newer": { connectionIdentifier: "17", connectable: true, startDate: 200 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      deterministicCandidateRequests.push(url);
      return credentialResponse("newest-secret");
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
});
assert.match(deterministicCandidateRequests[0], /inserted-first-but-older/);

let allStaleNow = 4_000;
let allStaleClaimCount = 0;
let allStaleCredentialCount = 0;
await assert.rejects(() => resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  nowImpl: () => allStaleNow,
  waitImpl: async (delayMs) => { allStaleNow += delayMs; },
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      return response({
        "stale-a": { connectionIdentifier: "17", connectable: true, startDate: 200 },
        "stale-b": { connectionIdentifier: "17", connectable: true, startDate: 100 },
      });
    }
    if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
    if (url.includes("/sharingCredentials/29")) {
      allStaleCredentialCount += 1;
      return new Response("not found", { status: 404 });
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      allStaleClaimCount += 1;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
}), /Guacamole primary election timed out/);
assert.ok(allStaleCredentialCount > 2);
assert.equal(allStaleClaimCount, 0);

for (const [label, terminalResponse, expected] of [
  ["401", new Response("unauthorized", { status: 401 }), /returned HTTP 401/],
  ["403", new Response("forbidden", { status: 403 }), /returned HTTP 403/],
  ["500", new Response("failed", { status: 500 }), /returned HTTP 500/],
  ["malformed", response({ expected: [], values: {} }), /returned no usable key/],
] as const) {
  let claimCount = 0;
  await assert.rejects(() => resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/remote-view/opaque",
    frameUrl: direct,
    stream,
    fetchImpl: (async (input: string | URL | Request) => {
      const url = String(input);
      if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
      if (url.endsWith("/activeConnections")) {
        return response({ "live-row": { connectionIdentifier: "17", connectable: true, startDate: 100 } });
      }
      if (url.endsWith("/connections/17/sharingProfiles")) return profileResponse();
      if (url.includes("/sharingCredentials/29")) return terminalResponse;
      if (url.endsWith("/api/guacamole-primary-claim")) {
        claimCount += 1;
        return response({ success: true, granted: true, retryAfterMs: 10_000 });
      }
      throw new Error(`Unexpected request in ${label} case: ${url}`);
    }) as typeof globalThis.fetch,
  }), expected);
  assert.equal(claimCount, 0, `${label} must remain terminal without primary election`);
}

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
          connectable: true,
          startDate: 100,
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

let noActiveDiscoveryCount = 0;
let noActiveClaimCount = 0;
const noActive = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/",
  frameUrl: direct,
  stream,
  fetchImpl: (async (input: string | URL | Request) => {
    const url = String(input);
    if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
    if (url.endsWith("/activeConnections")) {
      noActiveDiscoveryCount += 1;
      return response({});
    }
    if (url.endsWith("/api/guacamole-primary-claim")) {
      noActiveClaimCount += 1;
      return response({ success: true, granted: true, retryAfterMs: 10_000 });
    }
    throw new Error(`Unexpected request: ${url}`);
  }) as typeof globalThis.fetch,
  waitImpl: async () => {},
});
assert.deepEqual(noActive, { mode: "direct", url: direct });
assert.equal(noActiveDiscoveryCount, 2);
assert.equal(noActiveClaimCount, 1);

// The service claim lease outlives one resolver's entire election deadline.
// If the elected primary is slow to appear in the provider projection, a
// second resolver must time out rather than win another unrestricted claim.
let delayedProviderNow = 0;
let delayedProviderLeaseExpiresAt = 0;
let delayedProviderGrantCount = 0;
let delayedProviderDeniedCount = 0;
const delayedProviderFetch = async (input: string | URL | Request) => {
  const url = String(input);
  if (url.endsWith("/api/tokens")) return response({ authToken: "auth-secret" });
  if (url.endsWith("/activeConnections")) return response({});
  if (url.endsWith("/api/guacamole-primary-claim")) {
    if (delayedProviderNow >= delayedProviderLeaseExpiresAt) {
      delayedProviderGrantCount += 1;
      delayedProviderLeaseExpiresAt = delayedProviderNow + 30_000;
      return response({ success: true, granted: true, retryAfterMs: 30_000 });
    }
    delayedProviderDeniedCount += 1;
    return response({
      success: true,
      granted: false,
      retryAfterMs: delayedProviderLeaseExpiresAt - delayedProviderNow,
    });
  }
  throw new Error(`Unexpected delayed-provider request: ${url}`);
};
const delayedProviderWait = async (delayMs: number) => {
  delayedProviderNow += delayMs;
};
const delayedProviderWinner = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: delayedProviderFetch as typeof globalThis.fetch,
  nowImpl: () => delayedProviderNow,
  waitImpl: delayedProviderWait,
});
assert.deepEqual(delayedProviderWinner, { mode: "direct", url: direct });
await assert.rejects(() => resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: delayedProviderFetch as typeof globalThis.fetch,
  nowImpl: () => delayedProviderNow,
  waitImpl: delayedProviderWait,
}), /Guacamole primary election timed out/);
assert.equal(delayedProviderGrantCount, 1);
assert.ok(delayedProviderDeniedCount > 0);
assert.ok(delayedProviderNow < delayedProviderLeaseExpiresAt);

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
        connectable: true,
        startDate: 100,
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
assert.equal(
  concurrent.find((entry) => entry.mode === "shared")?.primaryActiveConnectionId,
  "elected-primary",
);
assert.match(
  concurrent.find((entry) => entry.mode === "shared")?.attemptId ?? "",
  /^[A-Za-z0-9._:-]{8,256}$/,
);

// Both viewers reconnect while the primary elected above is asynchronously
// closing. Each may mint a restricted key from the closing row, but the
// post-mint donor check must discard it. Only keys bound to the surviving
// primary may be returned to an iframe for redemption.
let electedPrimaryVisible = true;
let reconnectKeySequence = 0;
const reconnectShareKeys = new Map<string, { donor: string; valid: boolean }>();
const reconnectCredentialDonors: string[] = [];
const reconnectRedeemedKeys: string[] = [];
const reconnectFetch = async (input: string | URL | Request, init?: RequestInit) => {
  const url = new URL(String(input));
  if (url.pathname.endsWith("/api/tokens") && init?.method === "POST") {
    const key = new URLSearchParams(String(init.body ?? "")).get("key");
    if (!key) return response({ authToken: "auth-secret" });
    reconnectRedeemedKeys.push(key);
    const record = reconnectShareKeys.get(key);
    return record?.valid
      ? response({ authToken: `shared-${record.donor}` })
      : new Response("forbidden", { status: 403 });
  }
  if (url.pathname.endsWith("/activeConnections")) {
    return response({
      ...(electedPrimaryVisible ? {
        "elected-primary": { connectionIdentifier: "17", connectable: true, startDate: 100 },
      } : {}),
      "surviving-primary": { connectionIdentifier: "17", connectable: true, startDate: 200 },
    });
  }
  if (url.pathname.endsWith("/connections/17/sharingProfiles")) return profileResponse();
  const donor = url.pathname.match(/activeConnections\/([^/]+)\/sharingCredentials/)?.[1];
  if (donor) {
    reconnectCredentialDonors.push(donor);
    reconnectKeySequence += 1;
    const key = `${donor}-reconnect-${reconnectKeySequence}`;
    const valid = donor === "surviving-primary";
    reconnectShareKeys.set(key, { donor, valid });
    if (donor === "elected-primary") electedPrimaryVisible = false;
    return credentialResponse(key);
  }
  throw new Error(`Unexpected reconnect request: ${url}`);
};
const simultaneousReconnects = await Promise.all([
  resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/remote-view/opaque",
    frameUrl: direct,
    stream,
    fetchImpl: reconnectFetch as typeof globalThis.fetch,
    waitImpl: async () => {},
  }),
  resolveGuacamoleViewerFrame({
    dashboardHref: "https://dashboard.example.test/remote-view/opaque",
    frameUrl: direct,
    stream,
    fetchImpl: reconnectFetch as typeof globalThis.fetch,
    waitImpl: async () => {},
  }),
]);
assert.deepEqual(
  simultaneousReconnects.map((entry) => entry.primaryActiveConnectionId),
  ["surviving-primary", "surviving-primary"],
);
assert.equal(
  new Set(simultaneousReconnects.map((entry) => entry.attemptId)).size,
  2,
  "simultaneous reconnects must have distinct message-fencing attempt ids",
);
const reconnectKeys = simultaneousReconnects.map((entry) =>
  new URL(entry.url).hash.match(/[?&]key=([^&]+)/)?.[1]);
assert.ok(reconnectKeys.every((key) => key?.startsWith("surviving-primary-reconnect-")));
assert.ok(
  [...reconnectShareKeys.entries()]
    .filter(([, record]) => !record.valid)
    .every(([key]) => !reconnectKeys.includes(key)),
  "a key minted from the closing elected primary must never reach an iframe reload",
);
for (const key of reconnectKeys) {
  assert.ok(key);
  const redemptionResponse = await reconnectFetch(
    "https://dashboard-share.example.test/guacamole/api/tokens",
    { method: "POST", body: `key=${key}` },
  );
  assert.equal(redemptionResponse.status, 200);
}
assert.equal(reconnectCredentialDonors.filter((donor) => donor === "elected-primary").length, 2);
assert.equal(reconnectCredentialDonors.filter((donor) => donor === "surviving-primary").length, 2);
assert.deepEqual(reconnectRedeemedKeys, reconnectKeys);

const shareKeys = new Map<string, { activeConnectionId: string; valid: boolean }>();
const redeemedKeys: string[] = [];
const closingNewestRequests: string[] = [];
const closingNewestFetch = async (input: string | URL | Request, init?: RequestInit) => {
  const url = new URL(String(input));
  if (url.pathname.endsWith("/api/tokens") && init?.method === "POST") {
    const key = new URLSearchParams(String(init.body ?? "")).get("key");
    if (!key) return response({ authToken: "auth-secret" });
    redeemedKeys.push(key);
    const shareKey = shareKeys.get(key);
    return shareKey?.valid
      ? response({ authToken: `shared-${shareKey.activeConnectionId}` })
      : new Response("forbidden", { status: 403 });
  }
  if (url.pathname.endsWith("/activeConnections")) {
    return response({
      "stable-primary": { connectionIdentifier: "17", connectable: true, startDate: 100 },
      "closing-newest": { connectionIdentifier: "17", connectable: true, startDate: 200 },
    });
  }
  if (url.pathname.endsWith("/connections/17/sharingProfiles")) return profileResponse();
  const activeConnectionId = url.pathname.match(/activeConnections\/([^/]+)\/sharingCredentials/)?.[1];
  if (activeConnectionId) {
    closingNewestRequests.push(activeConnectionId);
    const key = `${activeConnectionId}-key`;
    shareKeys.set(key, { activeConnectionId, valid: activeConnectionId !== "closing-newest" });
    return credentialResponse(key);
  }
  throw new Error(`Unexpected request: ${url}`);
};
const stableReconnect = await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/remote-view/opaque",
  frameUrl: direct,
  stream,
  fetchImpl: closingNewestFetch as typeof globalThis.fetch,
  waitImpl: async () => {},
});
const selectedKey = new URL(stableReconnect.url).hash.match(/[?&]key=([^&]+)/)?.[1];
assert.ok(selectedKey, "a stable restricted sharing key must be returned");
const redemption = await closingNewestFetch(
  "https://dashboard-share.example.test/guacamole/api/tokens",
  { method: "POST", body: `key=${selectedKey}` },
);
assert.equal(redemption.status, 200, "the returned key must remain redeemable after candidate selection");
assert.equal(selectedKey, "stable-primary-key");
assert.equal(stableReconnect.primaryActiveConnectionId, "stable-primary");
assert.match(stableReconnect.attemptId, /^[A-Za-z0-9._:-]{8,256}$/);
assert.deepEqual(closingNewestRequests, ["stable-primary"]);
assert.deepEqual(redeemedKeys, ["stable-primary-key"]);

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
