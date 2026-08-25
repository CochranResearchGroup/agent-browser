# Plan 0124 Retained Browser And Final Episode Live Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Status: PARTIAL ACCEPTED | UNRELATED ELASTIC CYCLE PRESSURE-DEFERRED

## Accepted Retained Browser Boundary

The development runtime retained one authenticated browser through occupied
route parking, reassignment from route 1 to route 2, reconciliation, and an
authenticated dashboard refresh. Browser `session:p124-auth-live2` retained:

- PID `26642`;
- process start token
  `linux:bbb70da7-8203-4b25-a172-e5d5b1b701b5:43118584`;
- profile `p124-auth-live2`;
- target `34561B17DC2C1BD6A3A3D4D5F6FE8F6E`;
- the local fixture result `authenticated` read directly from the recorded CDP
  target.

Peer browser `session:p124-peer-live2` moved from route 2 into occupied route
1. Recovery capacity was granted and released on slot 1, and the authenticated
browser was parked as `reattachable_stale_route` without process or target
replacement. The authenticated browser then moved into route 2 through a
second granted and released recovery reservation. Reconciliation retained both
browsers and exact active slot ownership.

The authenticated development dashboard smoke returned HTTP 200 for service
status and bound the readback to development generation
`0.28.0-08de92737c24`, binary SHA-256
`08de92737c241aa819b3fb0ef1557bc456521feb577b61c025980be274ffa728`.
This required commit `cd2967f9`, which gives only
`GET /api/service/status` a ten-second first-response allowance. Ordinary
dashboard reads retain the two-second failover window, and service mutations
retain their existing bounded job-aware timeouts.

## Final Generation Browser External Episode

Browser `session:p124-passkey-final` launched directly into route 3, display
`development-display-3`, and slot 3. One configured `passkey_chooser` episode
completed with `outcome=desktop` on the same final installed generation.

The episode issued:

- admission receipt
  `presentation-admission:p124-passkey-final-generation:slot:development-slot-3`;
- scene-stage receipt `scene-stage:p124-passkey-final-generation`;
- trigger receipt
  `cdp-page-trigger:7c11b2e43efa1afa05599b8ef982d40a69988a05c161441dea2b2bd83dd9fec1`;
- paired page-absence receipt
  `paired-cdp-absence:b47c8a530449967af4474925ccbb34c151dc50d8bc75eab081b42905f1bdb2e6`;
- fresh ephemeral frame receipt `desktop-frame-5a04dcd5d14d7b3930df1955`;
- verification, restoration, slot-release, and cleanup receipts.

The 1280 by 633 frame contained 20,633 bytes, was not persisted, and had
SHA-256 `9021a19f379068ebdae21a0b9390ffa9894d6c0ba11c7be00c9bfec293864348`.
The browser remained healthy through restoration and was then closed through
its exact service-owned identity. Reconciliation returned the runtime to the
two retained acceptance browsers.

A preliminary attempt on the route-moved peer failed safely with
`desktop_scene_binding_unavailable` because the process display and assigned
display names disagreed. No capture occurred. The accepted retry used a fresh
native route and display binding rather than weakening scene identity checks.

## Legacy Lifecycle Disposition

The single remaining closing lifecycle row is
`session:development-presentation-provider-v1-2`. Fresh OS and provider
readback proves:

- recorded PID and process group `70849` are absent;
- the legacy profile directory is absent;
- its Chrome singleton lock is absent;
- canonical provider inventory references only v5 route profiles;
- no authoritative profile mapping exists for the v1 digest.

This row is explicitly retained as quarantined migration residue. It is not a
live resource-pressure source, is not eligible for reuse, and must not be
deleted by guessing an identity relationship. This is the plan's protected
disposition for the unprovable legacy row.

## Validation

The dashboard timeout repair passed:

- its red then green focused regression;
- all 15 dashboard-ingress tests;
- Rust clippy with warnings denied;
- Rust formatting and diff hygiene;
- the authenticated installed dashboard smoke.

The development doctor passed with all three development units selected on
the final generation. Production remained read-only and retained selected
generation `0.28.0-c128349c482f-d9745dc2e128`, binary SHA-256
`c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.

## Remaining Boundary

The unrelated elastic cycle is not yet accepted in this packet. Repeated
scale-out requests correctly deferred at four slots with
`reason=pressure_admission`, `reasons=[cpu_load]`, and
`productionUnchanged=true`. The retained authenticated browser remains alive
while current host pressure is allowed to decay. Plan 0124 closes only after
one admitted fifth route is reclaimed after cooldown and the same PID, process
token, profile, target, and authenticated result survive that unrelated
scale-in.
