#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  defaultProfileLeaseExpiry,
  profileLeaseActionAllowed,
  profileLeaseExpiryToIso,
  projectProfileLeases,
} from "../packages/dashboard/src/lib/service-profile-leases.ts";

const lease = {
  id: "lease-odollo-fedex",
  leaseRevision: "sha256:revision-one",
  principalId: "service:odollo-fulfillment",
  profileId: "odollo-fedex",
  authorizedActions: ["rejoin", "renew", "release", "reconcile"],
  observationOnly: false,
};
const finding = {
  code: "owner_generation_stale",
  severity: "warning",
  leaseId: lease.id,
  profileId: lease.profileId,
  message: "Lease owner generation is stale.",
  safeActions: ["rejoin", "reconcile"],
};

assert.deepEqual(
  projectProfileLeases(
    [{ profileId: "odollo-fedex", allocation: "exclusive" }, { profileId: "other" }],
    { profileLeases: [lease], doctor: { findings: [finding] } },
  ),
  [
    { profileId: "odollo-fedex", allocation: "exclusive", profileLease: lease, profileLeaseFindings: [finding] },
    { profileId: "other", profileLease: null, profileLeaseFindings: [] },
  ],
);
assert.equal(profileLeaseActionAllowed(lease, "rejoin"), true);
assert.equal(profileLeaseActionAllowed({ ...lease, observationOnly: true }, "rejoin"), false);
assert.equal(profileLeaseActionAllowed({ ...lease, authorizedActions: ["renew"] }, "release"), false);
assert.equal(defaultProfileLeaseExpiry(new Date("2026-08-27T12:00:00.000Z")), "2026-08-27T13:00");
assert.equal(profileLeaseExpiryToIso("2026-08-27T13:00:00.000Z"), "2026-08-27T13:00:00.000Z");
assert.throws(() => profileLeaseExpiryToIso("not-a-date"), /valid lease expiry/);

const servicePanel = readFileSync("packages/dashboard/src/components/service-panel.tsx", "utf8");
assert.match(servicePanel, /fetch\(`\$\{serviceBase\(activePort\)\}\/profile-leases`\)/);
assert.match(servicePanel, /type="password"[\s\S]*autoComplete="off"[\s\S]*Paste for this action only/);
assert.match(servicePanel, /rejoinServiceProfileLease[\s\S]*renewServiceProfileLease[\s\S]*releaseServiceProfileLease/);
assert.match(servicePanel, /planServiceProfileLeaseReconciliation[\s\S]*Apply sealed plan/);
assert.match(servicePanel, /reconcilePlan\?\.effectCapable/);
assert.doesNotMatch(servicePanel, /localStorage[\s\S]{0,200}capability|sessionStorage[\s\S]{0,200}capability/i);

console.log("Dashboard profile lease checks passed.");
