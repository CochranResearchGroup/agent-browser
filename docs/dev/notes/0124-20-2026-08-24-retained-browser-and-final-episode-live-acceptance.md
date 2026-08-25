# Plan 0124 Retained Browser And Final Episode Live Acceptance

Date: 2026-08-24

Completed: 2026-08-25

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Status: ACCEPTED

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
the final generation. At the first checkpoint, production remained read-only
on generation `0.28.0-c128349c482f-d9745dc2e128`, binary SHA-256
`c128349c482fc049b70fe5f3dbfeadd3a9336cdd3ad5f81731dc2cb6b3d5cd63`.
Concurrent Plan 0130 acceptance later selected production generation
`0.28.0-80d87ab7be0d-5926db67f48a`, binary SHA-256
`80d87ab7be0d2b3a1c8241a6bc5865fe556a00dcf700c2489759ab1a947af97a`.
Plan 0124 did not perform that update. Both its scale-out and scale-in receipts
proved the then-current production snapshot unchanged across their own
transactions.

## Accepted Pressure Admission And Elastic Cycle

Initial unrelated scale-out requests correctly deferred at four slots with
`reason=pressure_admission`, `reasons=[cpu_load]`, and
`productionUnchanged=true` while the retained authenticated browser remained
alive.

The original CPU gate used one-minute load average as its only CPU-pressure
signal. Commit `eec463d6` deepened pressure admission into a typed snapshot
evaluator and a bounded Linux sampler. A fresh one-second `/proc/stat` delta now
controls CPU admission when available. The evaluator requires at least ten
percent idle capacity with a one-core floor, rejects I/O wait above ten
percent, preserves memory, swap, and file-handle reserves, and uses load
average only as a fail-closed fallback. Fixtures prove that lagging high load
with five idle-core equivalents is admitted, zero idle capacity is rejected
even with low load, high I/O wait is rejected, and missing CPU samples retain
the conservative fallback.

An initial 250 ms tracer caught one brief idle scheduler interval between
otherwise saturated samples. The following scale command resampled at zero
idle capacity and made no change, but the interval was too noisy to remain an
admission authority. The final sampler therefore aggregates one second before
making the same typed decision.

The first corrected live request returned `reasons=[cpu_capacity]`, zero
idle-core equivalents against a required two on the 20-CPU host, four slots
before and after, and `productionUnchanged=true`. This proves the current
deferral was real CPU saturation rather than a stale load-average false
positive.

The accepted scale-out then measured `15.158648925281476` idle-core
equivalents against a required two while one-minute load still lagged at
`29.13`. It admitted six slots and provisioned exactly route
`development-route-5`, slot `development-slot-5`, and display reservation
`development-display-5`, moving capacity from four to five. Receipt:
`lifecycle-1787663403038-67989-294f0da5-539d-49d4-b0cf-2e7e255efab6.json`.

After scale-out, browser `session:p124-auth-live2` still had PID `26642`, the
same process start token, profile, route 2 binding, and target
`34561B17DC2C1BD6A3A3D4D5F6FE8F6E`; direct CDP readback remained
`authenticated`. After `41,774.4287109375` ms of cooldown, exact reference
checking returned no blockers or ambiguities and reclaimed route 5, moving
capacity from five to four. Receipt:
`lifecycle-1787663450079-80415-7cbf52f7-8ad1-4e21-a987-2ff616e84c81.json`.
The same browser identity and authenticated result survived scale-in and a
following reconciliation.

Exact service-owned close then removed the acceptance browser. Final
reconciliation reported zero browsers, four `warm_idle` slots, no route 5
processes, no route 5 or authenticated-profile singleton lock, four provider
displays, zero resource candidates, and one owned cleanup obligation for the
explicitly protected legacy row. The development skill was synchronized after
concurrent Plan 0130 source drift, and development doctor returned success
without restarting development units. The local authentication fixture was
stopped after the final browser close.

Current-main release build completed through bounded Cargo admission in
12 minutes 38 seconds and copied matching SHA-256
`451690603f76392cf6f5654cd5e06cb42e55161a3d6370181105b33da3646933`
to `cli/target/release/agent-browser` and `bin/agent-browser-linux-x64`. This
build did not restart either runtime or replace the accepted installed
development generation.

Final source closeout also repaired two pre-existing validator gaps exposed by
the current runtime-host topology. The fixed-input status harness now invokes
Cargo through the bounded WSL wrapper and captures output in regular files, so
an inherited pipe cannot keep the Node process alive after Cargo exits. Shared
smoke fixtures now opt into the supported single runtime host, and daemon
startup allows 60 seconds for executable provenance hashing under pressure.
The owner effect-admission transaction was also moved behind the runtime
lifecycle deep module, restoring the final dispatcher architecture boundary.

The final source validation passed:

- generated service-client contracts, JavaScript types, package exports,
  request and observability helpers, the fixed-input producer-to-client
  harness, managed-profile flow, and all no-launch client examples;
- service API and MCP parity, the no-launch service contracts smoke, dashboard
  view-stream tests, dashboard inspector action tests, and the final actions
  architecture check;
- route-confusion gates, three CDP screencast view-stream tests, and the
  isolated live CDP tab-streaming smoke;
- Rust formatting, clippy with warnings denied, 26 desktop-capture tests, three
  desktop-control coordinator tests, and the remote-view reattachment stress
  test;
- documentation build and repository diff hygiene.
