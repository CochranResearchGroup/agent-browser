#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  classifyGuacamoleShareAuthMessage,
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

// Backend ownership replaces viewer election. Retain key custody, role,
// maturity, cancellation and deadline coverage at the resolver boundary.
const ownedId = "00000000-0000-4000-8000-000000000001";
function fixture(options: {
  missing?: boolean; vanished?: boolean; immature?: boolean; child?: boolean;
  denied?: boolean; invalidOwner?: boolean; authDenied?: boolean; keyMissing?: boolean;
} = {}) {
  let now = 10_000;
  let minted = false;
  const calls: string[] = [];
  const donors: string[] = [];
  const fetchImpl = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    calls.push(url);
    if (url.endsWith("/api/guacamole-primary-claim")) {
      assert.deepEqual(JSON.parse(String(init?.body)), { operation: "ensure", routeId: "route-1", connectionId: "17" });
      if (options.denied) return new Response(JSON.stringify({ code: "guacamole_primary_owner_stale" }), { status: 503 });
      return response(options.invalidOwner ? { granted: true } : {
        granted: false, primaryOwned: true, activeConnectionId: ownedId,
      });
    }
    if (url.endsWith("/api/tokens")) {
      if (options.authDenied) return new Response("denied", { status: 401 });
      return response({ authToken: "test-only-token" });
    }
    if (url.endsWith("/activeConnections")) return response({
      // An older connectable viewer-owned row must never donate a key.
      foreign: { connectionIdentifier: "17", connectable: true, startDate: 0 },
      ...(!options.missing && !(options.vanished && minted) ? {
        [ownedId]: { connectionIdentifier: "17", connectable: true,
          startDate: options.immature ? 9_000 : 100,
          sharingProfileIdentifier: options.child ? "child" : null },
      } : {}),
    });
    if (url.endsWith("/sharingProfiles")) return response({
      profile: { identifier: "29", name: "Agent Browser Shared Session route-1", primaryConnectionIdentifier: "17" },
    });
    if (url.includes("/sharingCredentials/")) {
      const donor = url.match(/activeConnections\/([^/]+)/)?.[1];
      assert.equal(donor, ownedId);
      donors.push(donor!);
      minted = true;
      if (options.keyMissing) return new Response("gone", { status: 404 });
      assert.ok(now >= 12_000 || !options.immature, "primary must mature before donating a key");
      return response({ expected: [{ name: "key", type: "QUERY_PARAMETER" }], values: { key: "synthetic-share-key" } });
    }
    throw new Error(`Unexpected request ${url}`);
  };
  return {
    calls, donors,
    resolve: (signal?: AbortSignal) => resolveGuacamoleViewerFrame({
      dashboardHref: "https://dashboard.example.test/service", frameUrl: direct, stream,
      fetchImpl: fetchImpl as typeof fetch, signal, nowImpl: () => now,
      waitImpl: async (delay, signal) => {
        if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
        now += delay;
      }, attemptIdImpl: () => "synthetic-attempt",
    }),
  };
}
const stable = fixture();
const peers = await Promise.all([stable.resolve(), stable.resolve()]);
for (const result of peers) {
  assert.equal(result.mode, "shared");
  assert.equal(result.primaryActiveConnectionId, ownedId);
  assert.equal(result.url, "https://dashboard-share.example.test/guacamole/#/?key=synthetic-share-key");
}
assert.deepEqual(stable.donors, [ownedId, ownedId]);
assert.equal((await stable.resolve()).mode, "shared", "a reopened viewer keeps using the backend owner");
assert.equal((await fixture({ immature: true }).resolve()).mode, "shared");
for (const options of [{ missing: true }, { vanished: true }, { child: true }, { keyMissing: true }]) {
  const selected = fixture(options);
  await assert.rejects(selected.resolve(), /backend primary sharing timed out/);
  assert.ok(selected.donors.every(id => id === ownedId));
  assert.equal(selected.calls.filter(url => url.endsWith("/api/guacamole-primary-claim")).length, 1,
    "a missing provider row cannot trigger primary re-election or restart");
}
for (const options of [{ denied: true }, { invalidOwner: true }]) {
  const selected = fixture(options);
  await assert.rejects(selected.resolve(), options.denied ? /guacamole_primary_owner_stale/ : /identity is unavailable/);
  assert.equal(selected.calls.length, 1, "failed ownership admission must precede provider authentication");
}
await assert.rejects(fixture({ authDenied: true }).resolve());
const abort = new AbortController(); abort.abort();
await assert.rejects(fixture({ missing: true }).resolve(abort.signal), /Aborted/);
await assert.rejects(resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/service", stream,
  frameUrl: "http://127.0.0.1:8193/guacamole/#/client/1",
  fetchImpl: async () => { throw new Error("must not fetch"); },
}), /primary binding is incomplete/);
assert.deepEqual(await resolveGuacamoleViewerFrame({
  dashboardHref: "https://dashboard.example.test/service", frameUrl: direct,
  stream: { ...stream, providerMode: "exclusive" },
}), { mode: "direct", url: direct });
console.log("Guacamole backend ownership and exact sharing custody passed");
