# Plan 0137 Slice J No-Effect Candidate Preflight

Date: 2026-08-29

Status: PREPARED | PRODUCTION EFFECT GATE CLOSED

Scope: freeze the reviewable evidence available before a Plan 0137 Slice J
production candidate transaction, identify every unresolved precondition, and
propose the bounded consumer cases without applying any production effect.

## Decision

Do not stage, install, select, commit, roll back, or close the candidate in
production.

The exact candidate binary is qualified in the isolated development runtime,
but the current production preflight is blocked on two independent conditions:

1. the candidate migration reader reports
   `service_state_display_browser_missing` for one current display-to-browser
   relation and refuses to rewrite the state; and
2. the candidate presentation prerequisite reports 52
   `route_not_ready` blockers, zero eligible handoffs, and `ready=false`.

No repair, route creation, process cleanup, Service State rewrite, backup, or
production install transaction was performed while preparing this packet.

## Candidate Identity

- Review commit: `d06faec5e23c327e6ef1b907a9e0973563885df6`.
- Last binary-affecting commit:
  `802eb068e9186fcff0d938d8c8eac8623849e438`.
- Every later path through the review commit is documentation or the active
  lane catalog. The dashboard source tree is identical at both commits.
- Package version: `0.28.0`.
- Candidate binary: `cli/target/release/agent-browser`.
- Candidate binary SHA-256:
  `7294a8ccdf49a3862b47834789ca986630b0620da4779ebf5fd94615e862b3b1`.
- Dashboard source-tree Git object:
  `09fe60a0450fd327ae087f86bdf0e3058fb20737`.
- Development selected generation: `0.28.0-7294a8ccdf49`.
- Development status and doctor passed for the candidate runtime, dashboard,
  backend, isolated provider, lane, and published development skill.

The production candidate generation is `NOT_CREATED`. Generation identity
includes the staged support-manifest digest, and production staging is an
effect that this packet does not authorize. An isolated disposable staging
run produced generation
`0.28.0-7294a8ccdf49-dff07cb68d22`, but that is fixture evidence rather than a
production generation identifier.

## Isolated Support Payload Receipt

The disposable isolated staging run mutated only its temporary root. It did
not inspect or modify production state.

- Runtime-generation schema: `agent-browser.runtime-generation.v1`.
- Support-payload schema: `agent-browser.workstation-payload.v1`.
- Support-manifest SHA-256:
  `dff07cb68d22655278b08195abbc173934ebfecbb379981e2383e8b886e50a38`.
- Guacamole-bundle manifest SHA-256:
  `ec9745a9de61ff26896e3481d7877c7194958accda473016d15dc97a0e19fb95`.
- Dashboard-backend unit SHA-256:
  `dc52f16473e601f159c2cb9f9a69d19105eea18a335a52be0be3db62754c46ac`.
- Dashboard unit SHA-256:
  `fd77ec212bbe7c618bb39b5c03a761163b5ab07019d1e6f7ece5357fd9bcc57c`.
- Runtime-interlock service SHA-256:
  `5bebf1df0bea7da94d7590a80a11663da8bc2746b4413c9ab813c7960034c0c6`.
- Runtime-interlock timer SHA-256:
  `5ec446e061d250678f6fa6c8c3c267817f225b0234b64697bc5682f9db40796c`.
- The payload is source-free and binds the installed binary as its runtime
  controller.

The isolated migration preview over an empty fixture root required only the
initial schema materialization, preserved all protected classes, and created
no backup. It does not qualify current production state.

## Current Production Baseline

Readback at approximately `2026-08-29T22:02Z` established:

- installed version `0.28.0`;
- selected generation `0.28.0-2851117fd877-04e7cf4c8b54`;
- installed binary SHA-256
  `2851117fd8778d18ef05cadfb999a2bed82ed16e7d56206188f5bd753467f9c9`;
- Service State schema `agent-browser.service-state.v2`;
- runtime-owner-registry revision `1611`;
- service-principals revision `15`;
- one dashboard, one runtime host, one executable generation, and zero legacy
  daemons in the install convergence projection; and
- install doctor `success=false`, including the candidate drift, duplicate
  profile pressure, runtime-monitor readiness, and loopback port-conflict
  findings.

The installed workspace bundle is not the candidate. Its SHA-256 begins
`932cc4d924`, while the exact candidate SHA-256 begins `7294a8ccdf49`.

## Candidate Dry Run

The exact candidate binary ran `install workstation --dry-run --json` against
the current production root. It returned `success=true`, `state=planned`,
`mode=dry-run`, `mutated=false`, and `ready=false`.

Migration preview:

- schema `agent-browser.service-state-migration-preview.v1`;
- `mutation=false`;
- status `blocked`;
- blocker class `service_state_display_browser_missing`; and
- next action requires a compatible reader or restoration of the selected
  generation and explicitly forbids rewriting unknown state.

Because the preview is blocked, the production migration digest is
`NOT_AVAILABLE`. Producing or bypassing it requires a separate source repair or
an explicitly authorized state reconciliation packet.

Presentation prerequisite:

- schema `agent-browser.candidate-presentation-prerequisite.v1`;
- required `true`;
- ready `false`;
- eligible handoff count `0`; and
- blocker counts `{route_not_ready: 52}`.

A fresh candidate presentation route and durable handoff are therefore not
ready. URL presence, provider readiness, and the global many-to-many projection
do not substitute for this candidate-specific receipt.

## Backup And Rollback Evidence

No backup receipt or rollback transaction exists for this uncreated candidate.
Both values are `NOT_CREATED_PRE_EFFECT`.

The current selected generation remains backed by accepted transaction
`upgrade-ff0b9cc4-5c9a-4124-963e-389f4b1e97ff`, revision `14`:

- state `old_generation_retirable`;
- terminal result `accepted`;
- migration status `committed_no_change`;
- source and target Service State schema `agent-browser.service-state.v2`;
- source and target lease schema `agent-browser.profile-lease.v1`;
- snapshot SHA-256
  `cf4977fb011c007b29563b504715e0ccd89e523e41dba90d3824494164adcc3f`;
- staged SHA-256
  `a3f0e4316b05d8a6a3f46d77f0882a2f9657720d5afdfba0208fb8ad4f8b317e`;
- stable runtime-census digest
  `9c1bdfc1b6b3c978589e4dac043a2a8b4b270a263ee587ee61ac862a917b4ba9`;
- one retained runtime handoff and zero outstanding owner obligations; and
- `rollbackReady=false`, with only `inspect` and `review_gc` currently safe.

This accepted historical receipt is baseline evidence. It is not rollback
proof for the proposed candidate and grants no cleanup authority.

## Runtime, Lease, And Route Readback

Fresh Service resource census:

- 167 scoped processes;
- 101 correlated, 135 protected, 32 observed, and zero cleanup candidates;
- 12,353,245,184 total RSS bytes;
- seven owned cleanup obligations, zero transferring, zero unknown, and 46
  satisfied; and
- duplicate active-profile-lease and duplicate live-browser warnings remain.

Fresh operating-system census:

- `agent-browser`: 22 processes, 3,063,984 KiB RSS;
- `chrome`: 131 processes, 8,949,352 KiB RSS;
- `Xorg`: 7 processes, 117,696 KiB RSS;
- `xrdp-sesman`: 8 processes, 33,364 KiB RSS;
- `xrdp`: 1 process, 2,116 KiB RSS;
- `guacd`: 1 process, 2,952 KiB RSS; and
- 52,054,224 KiB memory available, with 8,312,944 KiB swap free.

Lease doctor is unhealthy across 26 leases:

- 11 `legacy_principal_unproven` warnings;
- 8 `owner_generation_or_binding_mismatch` warnings;
- 2 `runtime_owner_principal_binding_missing` warnings;
- 21 leases in `identity_reconciliation_required`;
- 3 leases in `owned_idle`; and
- 2 leases in `stale`.

The global remote-view doctor reports its many-to-many substrate ready, but it
also reports install drift, runtime-monitor readiness, duplicate-profile
pressure, and port-conflict findings. This global substrate result does not
override the candidate-specific zero-eligible-handoff result.

## Proposed Consumer Acceptance

If every precondition is repaired and exact production effect authority is
later granted, the candidate packet proposes these separate cases:

1. Last30days acquires or recovers `last30days-facebook` and reaches only
   `about:blank`.
2. Odollo acquires its own carrier profile and proves lane creation without
   opening a carrier site or submitting a tracking number.
3. SoyLei recovers one exact existing-session or owner-binding inconsistency
   and proves exact reuse without a duplicate launch.
4. Fictitious records receive an exact retirement preview, followed by apply
   only after separate operator review and authority.
5. Manual seeding uses one disposable profile and nonproduction target, proves
   a durable ready handoff with CDP absent, and closes exactly.
6. A foreign principal proves it cannot acquire or repair a currently owned
   profile.

Browser acquisition acceptance remains separate from real provider navigation,
credentials, consent, extraction, tracking lookup, and downstream scheduling.

## Hard Stops

Stop and obtain exact authority before:

- repairing or rewriting the blocked production Service State relation;
- creating, selecting, reconciling, or mutating a route or handoff;
- creating a production backup or installation transaction;
- staging, installing, selecting, committing, rolling back, or closing a
  production generation;
- changing production leases, owners, sessions, routes, displays, providers,
  profiles, processes, or runtime records;
- retiring fictitious records or applying orphan cleanup;
- acquiring or recovering any real tenant profile;
- launching a browser or opening a provider site;
- entering credentials, navigating a carrier site, submitting tracking data,
  or inspecting private page content; or
- forcing shutdown, process cleanup, route cleanup, profile cleanup, or
  provider mutation.

The next bounded packet is a source-level compatibility diagnosis of
`service_state_display_browser_missing` or an explicitly authorized exact
production reconciliation. Candidate installation remains out of scope until
the migration preview returns a reviewable digest, a candidate-specific fresh
handoff is ready, and candidate backup and rollback evidence can be created.
