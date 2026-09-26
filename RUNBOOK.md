# Runbook

Current index. [Turns 369 through 417](RUNBOOK-history-2026-09-16-turn369-through-2026-09-19-turn417.md) preserve the prior active runbook. [The September 13 archive](RUNBOOK-history-2026-09-13-through-turn312.md) preserves checkpoints through Turn 312; [Turn 313](RUNBOOK-history-2026-09-13-turn313.md), [Turns 314 through 320](RUNBOOK-history-2026-09-14-turn314-through-turn320.md), [P169 history through Turn 319](RUNBOOK-history-2026-09-14-p169-through-turn319.md), [superseded P186 checkpoints through Turn 328](RUNBOOK-history-2026-09-14-p186-through-turn328.md), [P194 through P196 checkpoints from Turns 340 through 343](RUNBOOK-history-2026-09-15-p194-through-turn343.md), and [Turns 323 through 368](RUNBOOK-history-2026-09-14-turn323-through-2026-09-16-turn368.md) remain separately preserved.

## Active Plan Locator Index

- [P12](docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md), [P78](docs/dev/plans/0078-2026-07-27-guacamole-route-fixture-recovery-interlock-plan.md), [P111](docs/dev/plans/0111-2026-08-13-multi-agent-shared-browser-profile-authority-plan.md), [P116](docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md), [P144](docs/dev/plans/0144-2026-08-31-lease-authority-coordination-and-revocation-plan.md), and [P158](docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md)
- [P157 production identity](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md), [P157 desktop slots](docs/dev/plans/0162-2026-09-10-production-desktop-slot-and-jit-viewer-allocation.md), [P157 profile reset](docs/dev/plans/0163-2026-09-10-profile-data-reset-backup-and-restore.md), [P165](docs/dev/plans/0165-2026-09-11-bill-identifier-form-drift-repair.md), [P169 Turnstile leaf](docs/dev/plans/0169-2026-09-11-cloudflare-turnstile-desktop-challenge-plan.md), and [P178](docs/dev/plans/0178-2026-09-13-browserless-runtime-lane-quiescence.md)
- [P182](docs/dev/plans/0182-2026-09-13-authentication-resume-state-reconciliation.md), [P187](docs/dev/plans/0187-2026-09-14-challenge-countermeasure-control-plane-blueprint.md), [P169 hCaptcha leaf](docs/dev/plans/0189-2026-09-13-hcaptcha-fixture-checkbox-acceptance.md), [P190](docs/dev/plans/0190-2026-09-14-advisory-candidate-build-and-promotion-orchestrator.md), [P197](docs/dev/plans/0197-2026-09-16-challenge-consumer-integration.md), and [P204](docs/dev/plans/0204-2026-09-16-ci-validation-economics-and-tiering.md)
- [P218](docs/dev/plans/0218-2026-09-23-grilling-contract-remote-view-conformance.md) and [P214](docs/dev/plans/0214-2026-09-17-candidate-permit-event-plan.md)

## Current P218 status | 2026-09-25

Plan 0218 version 38 remains open on `platform/p211-simple-cold-upgrade`.
Source checkpoint `0d994c7a` reconciles a PID-only manual-seeding launch
only after the recorded PID is absent. It atomically closes the old SQLite
operation without a handoff or launch retry; the next launch requires a new
operation ID. A live or unreadable PID remains reserved. The focused Rust
fixture, strict workspace Clippy, formatting, route-confusion gates, handoff
documentation check, and docs build pass. Launch-issued outcomes without a
PID, live-PID identity reconciliation, route rebinding, and installed
visual/input acceptance remain open. Architecture P03, P05, and P09 are
violated; coverage remains zero pass, 27 partial, seven fail, and 11 missing.

Source checkpoint `2b7e866e` dispatches SQLite manual-seeding acquire,
verified-process close, and current-proof durable resolution. It journals the
launch issue before effects, avoids the parallel runtime JSON writer, and
retains the provider-owned keeper after close. The dashboard accepts the
process-bound receipt. Five focused Rust cases, strict workspace Clippy,
formatting, route-confusion and dashboard handoff checks, remote-view docs
checks, dashboard build, and docs build pass. This is source-only evidence:
no installed browser, pixels, or input were accepted. Route rebinding after a
keeper change, uncertain-launch reconciliation, and the live keeper probe
remain open. The current architecture audit reports P03, P05, and P09
violated, five detector gaps, and 11 unverified rows. The G01–G45 coverage
inventory remains zero pass, 27 partial, seven fail, and 11 missing.

### Earlier P218 source checkpoints

Source checkpoint `12025924` added a keeper-derived presentation proof bridge for
SQLite manual seeding. It checks the exact process around display access,
window staging, process-owned X11 visibility, and public operator reachability.
One focused provider-free fixture, strict workspace Clippy, and formatting
passed through `cargo-signal`. At that checkpoint the effectful handler did
not call this bridge; P03/P05 and installed visual acceptance remain open.
An earlier source cut journaled a detached manual-seeding launch with an
uncertain process identity as `recovery_required`, preserving its PID with the
SQLite operation observation and blocking profile reuse or ready handoff
publication. One focused fixture, strict workspace Clippy, and formatting pass
through `cargo-signal`. That adapter did not call this path at the time;
exact process reconciliation and installed acceptance remain open.
Source checkpoint `ef194bfc` atomically publishes a manual-seeding
ready handoff, lifecycle state, and operation result after exact route and
process observation plus a ready operator visibility proof. Its result exposes
the opaque handoff URL and a redacted proof digest; the focused fixture confirms
a raw provider URL in the supplied proof is omitted. The fixture, strict
workspace Clippy, and formatting pass through `cargo-signal`. Dispatch,
durable resolution, live process verification, and installed visual acceptance
remain open, and P03/P05 remain violated.
Source checkpoint `17883ee9` adds a CDP-free launch helper that
uses the SQLite named-profile planner and returns the exact PID plus optional
captured process identity without writing a legacy Service Browser record.
The effectful handler does not call this helper yet; no launch or ready handoff
was exercised. Its focused planner fixture, strict workspace Clippy, and
formatting pass through `cargo-signal`.
Source checkpoint `e613dbb6` binds a ready keeper slot to
manual seeding before launch and rejects ordinary browser-open reservations
for that same slot. Its focused fixture passes both race orders, active-browser
occupancy refusal, exact replay, and launch observation. Strict workspace
Clippy and formatting pass through `cargo-signal`. No effectful adapter or
ready handoff has moved from JSON; P03/P05 remain violated.
The prior source checkpoint `6a240621` adds an explicit CDP-free launch
planner for the reserved SQLite named profile. It rejects competing profile
selectors, requires an absolute
executable path, and skips the legacy JSON profile and browser-capability
selectors. Its active provider-free fixture passes one test; strict workspace
Clippy and formatting checks pass through `cargo-signal`. The effectful
launcher has not adopted this planner, so this is not installed acceptance.
The preceding SQLite manual-seeding source foundation reserved the profile
atomically with its operation generation. Its exact
process and provider-route observation is journaled in a second SQLite
transaction. The focused reservation and observation fixtures, formatting, and
strict workspace Clippy pass through `cargo-signal`. No ready handoff is
published by this foundation. The effectful manual-seeding adapter, ready
publication, durable resolution, and close path remain open; P03/P05 are still
violated.
The G03 importer now filters malformed active records, rejects orphaned
sessions and tabs, and repairs browser session membership while retaining valid
active state. Its SQLite fixture proves archive-byte preservation and replay;
installed recovery and handoff import remain open.
The new G03/G05 source cut imports valid active sessions while filtering
malformed or conflicting legacy history rows into typed migration rejects. Its
focused fixture passes through `cargo-signal`; installed cold migration and the
remaining source classes are unverified. The original legacy source remains in
the read-only migration archive. P03/P05 manual-seeding JSON authority remains
open.
The G42/P03/P05 ordinary-open source cut has no installed
acceptance. Ordinary `remote_view_open` is routed toward the SQLite manager
journal for the default RDP request; URL navigation, disposable-profile
intent, process-bound visibility proof, and exact failure observations were
added. JSON-only handoff resolution is rejected. The read-only dry-run
projection now uses SQLite manager state and live keeper status; the CLI no
longer injects the legacy route-pool environment. The legacy JSON manual-seeding
and route code still compiles, advanced effectful open options currently fail closed,
and P03/P05 remain violated. Do not publish this candidate as a completed
remote-view feature. The initial ordinary-open checkpoint was `2065265b`.
Eight focused journal tests and the ordinary-open adapter fixture pass through
`cargo-signal`, including a crash after the navigation issue fence. The
broader `remote_view_open` filter passes 18 retained focused cases after the
obsolete JSON route-pool dry-run fixtures were removed. The SQLite dry-run
store fixture passes with its legacy JSON source removed. Strict workspace
Clippy, formatting, the coverage validator, the architecture-detector self-test,
and the no-launch route-confusion gate pass on this candidate. Its dashboard
fixture now expects the current projection without stored viewer counts. The
selected live CDP tab-streaming smoke failed during daemon startup while Cargo
admission reported memory pressure; it needs an isolated rerun. The P03/P05
detector now reports the manual-seeding JSON path and its mismatch with
SQLite-only durable handoff resolution explicitly.
A disposable-home daemon-router dry-run fixture passes after deleting the
legacy Service State source; it reads the SQLite profile catalog and publishes
no handoff. Effectful open, manual seeding, and installed acceptance remain open.
The effectful ordinary-open adapter now rejects an unknown named profile or
missing default disposable policy from the SQLite catalog before presentation
admission. The disposable-home router fixture confirms the unknown profile
creates neither a queue entry nor a handoff after legacy JSON removal. Focused
Rust, strict Clippy, formatting, remote-view docs checks, and the docs build
pass. The remaining legacy route instructions across the user-facing docs still
need reconciliation before publication.
The ordinary remote-view adapter now carries positive `jobTimeoutMs` into its
SQLite presentation queue admission. The two focused adapter and deadline
fixtures, strict Clippy, final formatting, remote-view documentation check,
and docs build pass. The full effectful path remains unqualified.
The CLI now rejects explicit route and display selectors before dispatch,
including a global display-isolation choice. Focused `remote_view_open` Rust
tests pass all 18 cases, including a replacement parser rejection fixture.
The main help, README, agent skill, and remote-view docs show the ordinary
SQLite-managed command. Legacy service guidance elsewhere still needs an
audit, and the P03/P05 manual-seeding JSON path remains open. Strict Clippy,
formatting, route-confusion gates, the remote-view documentation check, and the
docs build pass for this cut.

At that checkpoint, the provider-free SQLite host fixture proves Alice and
Bob share one browser with separate sessions, tabs, targets, handoffs, and
heartbeats; Alice-first cleanup
preserves Bob and final-session cleanup closes the browser. Session or tab
closure terminalizes its exact handoff in the same SQLite transaction. A new
G40 SQLite restart fixture proves waiting work becomes retryable and runs only
after an exact client resume. G40, G42, G43, and G44 remain partial because
installed effects and the full ordinary `remote_view_open` path are not yet
proven. The next primary source batch is the G42/P03/P05 cut of JSON
acquisition and handoff finalization onto the SQLite session authority.

The architecture gate reports three violations (P03, P05, P09), five detector
gaps, and eleven unverified rows. The G40 focused test, strict workspace
Clippy, formatting, coverage check, and selected service client checks pass;
the full raw Cargo output is retained through `cargo-signal`. No installed
runtime, provider, or production acceptance is claimed. The previous committed
branch checkpoint matched its remote; the version 34 source checkpoint is
currently local only. Use fresh Git readback, the plan, and the coverage
manifest for exact evidence and
remaining requirements.

## Turn 450 | 2026-09-23

Plan 0218 M1 removes the CLI Lease Authority Cargo edge and 28,824 lines of
legacy authority, principal, lease, recovery, runtime-owner, adoption,
reconciliation, retirement, retained-state, and state-migration implementation
at checkpoints `135e3b51` and `df7eaf64`. Compiler wave 7 retains the next
integration boundary with 157 groups; the CLI remains noncompiling and M1 is
open.

The verified P218 capsule was reused once with no canonical fallback. The
bytes-divided-by-four proxy estimates 573 capsule tokens versus 16,550
canonical-policy tokens, or 15,977 tokens of avoided ingestion. This is not
exact context or billing telemetry.

## Turn 449 | 2026-09-23

The Plan 0218 policy-ingestion pilot now has hermetic positive and negative
tests plus repository loading-contract wiring. Tests cover deterministic output,
current-capsule verification, canonical-policy hash drift, missing source files,
empty required fields, and unsupported schemas. Matching future work checks and
reuses the capsule; drift, listed triggers, and unresolved ambiguity return the
agent to canonical policy.

## Turn 448 | 2026-09-23

Plan 0218 begins a policy-ingestion pilot. A deterministic profile selects the
eight applicable canonical policies and generates a 200-word execution capsule
with source hashes, operative rules, checks, hard stops, and re-read triggers.
The selected canonical files total 9,444 words. Rebuilding the capsule is
byte-identical, preserving reproducibility while avoiding unchanged full-policy
reloads. Canonical policy remains authoritative whenever a trigger or hash
changes.

## Turn 447 | 2026-09-23

Plan 0218 M1 compiler wave 5 exits successfully with zero diagnostic groups
after removing the four wave-4 Service-model remnants. The focused
`agent-browser-service-model` library suite passes all 195 tests. The
cut-specific architecture gate remains green, completing the Service-model
portion of cut 1 while downstream CLI closure remains open.

No runtime, provider, browser, installation, publication, push, or merge
effect occurred.

## Turn 446 | 2026-09-23

Plan 0218 M1 removes the remaining typed Lease Authority, principal-registry,
and runtime-owner family from the Service State aggregate, including its
authority-only tests and persistence wrappers. The cut-specific architecture
gate now passes with no findings.

Compiler wave 4 exited 101 with four downstream groups: one mechanical stale
export and three primary architecture groups for deleted profile-lease schema
and receipt surfaces. The packet stops at the second causal group. The Service
model remains noncompiling, M1 remains open, and no runtime, provider, browser,
installation, publication, push, or merge effect occurred.

## Turn 445 | 2026-09-23

Plan 0218 version 9 adds execution-efficiency controls without changing G01
through G45 or milestone scope. The remaining Service State excision is one
primary-owned outcome packet. Packet admission now requires implementation,
one compiler wave, reconciliation, and a 20 percent validation and closeout
reserve to fit the live cumulative allowance. Compiler cadence, CodeGraph
sync, durable compact evidence, policy re-read triggers, deterministic
classifier repair, and worker admission are narrowed accordingly.

This is planning hardening, not implementation progress or renewed execution
budget. The active 500,000-token goal cap remains cumulative and blocked.

## Turn 444 | 2026-09-23

Plan 0218 M1 source checkpoint `2ce55e07` deletes the legacy profile-lease
record module and exports instead of restoring the removed principal-continuity
recourse enum. This removes another 303 lines from the default Service model.

The cut-specific guard names only `service_state.rs`. No further compiler wave
ran, and Service State remains the final primary-owned cut. No installed
runtime, provider, browser, ingress, production, release, push, or merge effect
occurred.

## Turn 443 | 2026-09-23

Plan 0218 M1 compiler wave 3 follows checkpoint `d05060ec`. It exited 101 and
compacted to 2 groups with 62 occurrences: the repeated 61-occurrence Service
State authority group and one expected new `profile_lease.rs` reference to the
deleted principal-continuity recourse enum. Three wave-2 groups resolved and
none regressed.

The next packet must delete the remaining profile-lease product surface and
perform the primary-owned Service State cut. It must not restore or rename the
recourse enum. No installed runtime, provider, browser, ingress, production,
release, push, or merge effect occurred.

## Turn 442 | 2026-09-23

Plan 0218 M1 source checkpoint `d05060ec` deletes the legacy principal
continuity module and its `ServiceState` work-lease wrappers. This removes
1,154 lines from the default Service model. It no longer derives runtime-owner
principal recourse, binds subordinate work leases, or treats legacy principal
migration as live product behavior.

The cut-specific guard now names only `service_state.rs`. That final
primary-owned authority cut remains open, downstream compilation is still
intentionally broken, and no additional Cargo wave ran. No installed runtime,
provider, browser, ingress, production, release, push, or merge effect
occurred.

## Turn 441 | 2026-09-23

Plan 0218 M1 source checkpoint `c326a161` deletes the Service-model
runtime-owner projection module and all associated `ServiceState` projection
methods. The default Service model no longer exports owner, attestation,
lifecycle, resource-lane, or session-binding projections. This removed 703
lines and reduced the cut-specific guard from three source files to two.

Principal continuity and Service State remain the final Service-model source
groups. CLI callers are intentionally unresolved until their legacy product
surfaces are deleted, so the package and CLI are not compilation-qualified and
no third Cargo wave ran. No installed runtime, provider, browser, ingress,
production, release, push, or merge effect occurred.

## Turn 440 | 2026-09-23

Plan 0218 M1 source checkpoint `aca80104` removes abandoned-retirement's
Lease Authority dependency. Pending historical retirement no longer denies a
profile claim, and retirement terminal projections no longer serialize
runtime-owner lifecycle or cleanup-obligation states. This advances G41 and
the availability-first G23 boundary without introducing a replacement denial
concept.

Compiler wave 2 exited 101 as expected and compacted to 4 groups and 71
occurrences, compared with 7 groups and 76 occurrences in wave 1. Three groups
resolved, none were new or regressed, and the cut-specific guard dropped from
four source files to three. Principal continuity, runtime-owner projection, and
Service State remain open; the Service model does not yet compile. No installed
runtime, provider, browser, ingress, production, release, push, or merge effect
occurred.

## Turn 439 | 2026-09-23

Plan 0218 M1 cut 1 is now compiler-driven. The frozen replacement contract
allows the SQLite Browser Session Manager authority, one data-only principal
provenance enum, and a non-default migration diagnostic boundary; it forbids
moving or renaming lease, runtime-owner, custody, quarantine, cleanup-admission,
JSON fallback, or dual-write concepts into the replacement API. The Service
model Cargo edge to Lease Authority is removed.

The first retained WSL-safe Cargo JSON wave exited 101 and compacted to 7
groups with 76 occurrences. Two leaf groups now use the Service model's local
principal-provenance value. Five coupled groups remain in retirement,
principal continuity, runtime-owner projection, and Service State. The
cut-specific architecture guard remains red until all Service model source
references are gone, while aggregate P15 correctly remains red for the wider
CLI closure. This is outcome progress but not a compiling candidate or M1
acceptance. No installed runtime, provider, browser, ingress, production,
release, push, or merge effect occurred.

## Turn 438 | 2026-09-23

Plan 0218 M0 is complete as a source-only checkpoint. The exact G01 through
G45 manifest reports 26 partial, 8 failed, and 11 missing rows. The frozen
ordinary-open-through-handoff graph records split SQLite and JSON authority and
the three required M1 cuts. The red architecture gate deterministically fails
P02, P03, P05, P09, P12, P15, P16, and P19 while leaving the other 11
prohibitions explicitly unverified. A deterministic Cargo JSON diagnostic
helper and hermetic tests are ready for compiler-driven M1 lease excision.

Inherited usage was 595,176 tokens. The M0 control readback before closeout was
340,551 including worker effort, already exceeding its 200,000 allocation by
140,551. Cumulative usage was 935,727 of 2,000,000, leaving 1,064,273 at that
checkpoint. No product behavior, build, installed binary,
service, provider, browser, ingress, production, or release effect occurred.
The next action is M1's default-product Lease Authority dependency cut; no M1
work began in this checkpoint.

## Turn 437 | 2026-09-23

Plan 0218 supersedes P217 because availability-first open and visual reliability
were only part of the accepted September 19 grilling contract. P218 freezes 45
normative decisions covering runtime authority, SQLite migration and recovery,
provider topology, capacity, hidden-browser removal, durable handoff recovery,
Desktop Services control, history, retention, privilege, configuration, and
development acceptance. Deterministic architectural prohibitions must make
ordinary-path denial authority and legacy runtime inputs visible as build or
contract failures rather than review-dependent intent.

Commit `9bdcfbe3` is inherited evidence that ordinary `remote_view_open` no
longer participates in legacy profile-lease classification. It does not prove
the remaining contract. M0 must reconcile cumulative P217 usage, map G01
through G45 to source and evidence, and add red architecture gates before any
further behavior repair or runtime candidate. No runtime, provider, browser,
ingress, production, or release effect occurred while creating the plan.

## Turn 434 | 2026-09-22

The first Plan 0217 candidate installed as development generation
`0.28.0-2bd55895e03c` with production unchanged, current isolated skill, and
three passing disposable browser cycles. Its ordinary no-effect remote-view
reproducer retained the original profile denial because the first regression
carried identity fields absent from the real CLI request. The one allowed M1
repair now derives the daemon-session local principal and evaluates it through
the existing shared-local policy; the corrected field-free regression, all
nine shared-local tests, formatting, strict Clippy, and diff hygiene pass.

Doctor separately fails because fresh observation sees only route 1 on `:24`
while durable inventory retains four ready routes on `:13` through `:16`.
Provider plan and staging completed, but preflight correctly refuses mutation
at the hard-coded `route-keeper-runtime` interlock. No provider apply occurred.
That provider-control join belongs to unfinished Plan 0211 architecture and is
not widened into Plan 0217. One replacement candidate remains authorized for
the local-attribution repair; external visual acceptance remains gated by the
provider authority split.

## Turn 435 | 2026-09-22

Replacement checkpoint `9a03297b` installed as development generation
`0.28.0-cd33a6aa46d9`, binary SHA-256
`cd33a6aa46d90f93b8d7788840a38db781edffd50e8351ffec05917ffe0bf47b`,
with production unchanged. The exact ordinary no-effect reproducer still fails
`existing_session_profile_identity_unproven`. The passing unit seam did not
model the request shape at generic profile-lease admission, which precedes the
remote-view coordinator's later launch and attribution flow.

Plan 0217's one implementation plus one repair are consumed. A third repair or
candidate is not authorized. M1 remains incomplete, and M3 is independently
gated by the durable-inventory versus fresh-display split plus the deliberate
`route-keeper-runtime` provider interlock. No browser open or provider apply
occurred. Integration remains prohibited.

## Turn 436 | 2026-09-22

Read-only diagnosis identified the exact M1 miss. Installed failure provenance
preserves action `remote_view_open`, lane `default`, and profile `default`; the
active shared-local policy grants profile use. Development Service State has no
`default` browser or PID. Its historical `default` session resolves runtime
owner binding as `Ok(None)`. The availability-first helper was wired only for
binding-error and binding-present paths, so the absent-binding path reaches
`existing_session_profile_identity_unproven` without consulting it. The green
fixture constructed an owner binding and covered the wrong branch.

This materially narrows the next repair to the binding-absent branch and its
true regression shape. No source, runtime, browser, provider, or ingress effect
was performed because Plan 0217's implementation and repair allowance remains
consumed. The provider route-keeper interlock remains the second independent
acceptance gate.

## Turn 432 | 2026-09-22

P217 M0 isolated the ordinary-path denial before browser or provider effects.
Installed generation `0.28.0-1d808efe0797` has four ready warm provider routes
and three projected warm-idle Service slots, but an exact named-profile dry run
fails `existing_session_profile_identity_unproven` because released historical
session metadata overrides the active shared-local `default` profile. The
[M0 cut line](docs/dev/notes/0217-2-2026-09-22-m0-cut-line.md) binds source,
binary, provider and inventory identities, records the red/green focused
reproducer, freezes the restart recovery contract, and identifies the M1 source
cut. The source repair now lets explicit shared-local remote-view selection
ignore unproved historical custody only when no current profile process is
proved; exact uncertain cleanup remains untouched. The visual baseline is still
incomplete because the installed generation returns no handoff and direct X
capture was correctly refused. No runtime or provider mutation occurred.

## Turn 431 | 2026-09-22

Plan 0217 supersedes Plan 0211 and inherits its branch, draft PR #191, evidence,
and retry history. P217 freezes two unmet requirements: ordinary operation must
serve a fresh working route despite stale ownership history, and Guacamole/XRDP
must repeatedly render and respond correctly on external desktop and mobile
clients across every warm route. One `/goal` campaign is capped at 2,000,000
cumulative tokens across five sub-milestones. It forbids another lease,
scheduler, allocator, or plan-revision loop. Plan 0211 is CANCELLED as
superseded; its narrower source and installed receipts remain evidence.

## Turn 430 | 2026-09-22

The P211 integration-ready verdict is withdrawn after comparison with the
pre-plan design interview. Removing denial-first lease behavior and delivering
simple, reliable Guacamole/XRDP viewing are mandatory acceptance requirements.
Current evidence proves protocol readiness and one installed opaque handoff,
but not repeated external desktop/mobile rendering, clean desktop state,
correct z-order, responsive input and resize, or recovery from blank, stale,
partial, terminal-only, hidden-browser, and unresponsive-window failures.
Uncertain ownership may preserve an exact resource from destructive cleanup;
it may not deny creation or selection of another working route. Draft PR #191
remains a review vehicle. Plan 0211 is OPEN for one consolidated correction and
visual-operational acceptance campaign.

## Turn 429 | 2026-09-22

P211 M3 and M4 are complete at source `9e7c5719` and installed development
generation `0.28.0-1d808efe0797`, binary SHA-256
`1d808efe0797dabdd1da2725d70947ae435a1cbb0a0a2a41fe1ee1ad839b5514`.
Exact quarantine reconciliation cleared the retained obligation without manual
SQLite mutation. Doctor and three disposable cycles passed using
`/opt/google/chrome/chrome`. The ordinary named `default` profile returned a
ready authenticated opaque handoff, then close removed the exact Chrome PID,
browser, and active session while retaining explicit-close history and four
ready obligation-free XRDP scopes. The final repair grants the service user
access to the exact selected route display before managed Chrome launch; 89
affected tests, the display-access contract, formatting, and strict Clippy
pass. Draft PR #191 is current and branch divergence is zero. The candidate is
ready for protected integration; merge, production install, release, branch
deletion, and worktree removal remain separate maintainer actions.

## Turn 428 | 2026-09-22

P211 version 65 M1 and M2 are complete. Source `cf75aac1` makes desktop-control
activation atomic. Frozen candidate `8bc9a19b` passes every comprehensive Rust
compartment after the first run found one missing runtime-config normalizer
fixture; the repaired service compartment passes all 599 tests. Strict Clippy,
formatting, client, API/MCP, dashboard, documentation, policy and handoff gates
also pass. The first failure is retained and unaffected comprehensive evidence
is reused by impact.

Goal usage is 595,176 cumulative tokens, below M2's 850,000 ceiling. The plan
now distinguishes the version 65 completion boundary from its historical
expanded architecture table. M3 is next and requires fresh development-runtime
effect custody before one isolated installed cold workflow. Production,
release and integration effects remain excluded. P211 remains OPEN.

## Turn 427 | 2026-09-22

P211 version 64 accepts two blocking review findings against the uncommitted
version 63 checkpoint. Desktop-control activation currently publishes the
successor lease before its separately transacted focus effect, so a failed
focus can fence the prior controller without completing activation. Repair must
give lease publication and focus one commit outcome or use an exact rollback
that cannot overwrite an intervening transfer. Failure, success and concurrent
transfer regressions are required before qualification.

The 16-file recovery manifest matches the dirty P211 tree and the bundle hashes
verify, but `/tmp` is temporary evidence rather than durable custody. Preserve
the exact unqualified checkpoint in durable storage or an explicitly
unqualified custody commit before depending on it for resumption. The execution
allowance remains exhausted; this documentation-only review ran no Rust build,
runtime or provider effect. P211 remains OPEN. The next sequence is durable
custody, atomic activation repair, focused qualification, complete
changed-surface gates, then viewer and agent input enforcement.

## Turn 426 | 2026-09-21

P211 version 63 is an operator-requested, uncommitted checkpoint on HEAD
`9d3bd816`. SQLite per-display control and journaled handoff focus integration
are written; Rust compilation, tests and strict Clippy remain pending. The
[checkpoint](docs/dev/plans/0211-2026-09-17-simple-cold-upgrade.md#version-63-operator-checkpoint)
records scope, verification and the saved patch/archive. Client and narrow docs
checks pass. All workers are terminal; no installed/provider mutation occurred.

The previous qualified source remains `6526ccef`; it does not qualify v63 edits.
P211 stays OPEN. Fresh 10:30 UTC readback is 464 elapsed minutes since 02:46,
104 beyond the inherited 360-minute ceiling on prior elapsed accounting; active
versus idle attribution is unknown. No new build starts at this checkpoint.
Reconcile the allowance before resumed qualification. Actual viewer/agent input
fencing, persistence, quarantine and full isolated acceptance remain open.

## Turn 425 | 2026-09-21

P211 checkpoint `45124f99` completes the provider-free recovery proof and
adoption-terminal packet. Exact predecessor-exit proof now reconstructs only
from retained `Ready`, `Degraded`, or deterministic interrupted `Adopting`
state. A current adoption terminal event persists `RecoveryFailed` while
retaining predecessor evidence; stale fences and predecessor occurrences do
not mutate the successor.

The combined 50-test route-keeper lane, formatting, strict workspace Clippy,
and diff check pass. Configured startup recovery and multi-route orchestration
remain the next bounded packet. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

## Turn 424 | 2026-09-21

P211 checkpoint `8f60434f` implements configured Guacamole adoption while
leaving runtime startup fail-closed. The adapter validates the exact durable
catalog and `Adopting` fence before provider access, retains one fresh tunnel
task for same-process replay, reobserves XRDP under the configured route user,
and emits the schema-v4 predecessor/current occurrence receipt.

The 18-test focused Guacamole keeper lane, formatting, strict workspace Clippy,
and diff check pass. The next bounded packet must reconstruct exact
predecessor-exit proof for retained `Ready`, `Degraded`, and interrupted
`Adopting` records and persist a failed-adoption terminal transition before
configured startup recovery is enabled. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

## Turn 423 | 2026-09-21

P211 checkpoint `11f606a4` completes the version 50 provider-free route
identity packet. Authority schema v4 separates the durable configured route
and exact XRDP witness from predecessor and successor Guacamole tunnel
occurrences. Historical v3 adoption receipts migrate deterministically from
their former same-UUID invariant. New adoption requires the exact catalog
digest, route user, complete XRDP witness, predecessor occurrence, and successor
fence. Stale predecessor disconnect evidence cannot degrade the successor.

The service-model unit and integration suites pass, as do the 44-test focused
CLI route-keeper lane, the SQLite v1-to-v4 migration test, formatting, and
strict workspace Clippy. Configured connector adoption joined to the exact
predecessor-exit capability is the next bounded packet. No provider, browser,
Service State, installed-runtime, production, ingress, or release effect
occurred.

## Turn 422 | 2026-09-21

P211 version 50 corrects a configured-adoption identity contradiction before
provider code is written. The configured Guacamole path can create only a new
websocket tunnel, whose protocol UUID is newly generated. The version 49 model
required a cold successor to preserve the predecessor UUID, so no truthful
configured adapter could satisfy it.

The corrected contract keeps the digest-fenced catalog connection and complete
XRDP ownership witness as durable route identity. A Guacamole UUID identifies
one transport occurrence under one keeper fence. A successor occurrence may be
published only after exact predecessor process exit, fresh catalog validation,
and unchanged XRDP witness readback. Provider-free model and stale-event tests
come first; configured connector and runtime-host recovery remain later gates.
No provider, browser, Service State, installed-runtime, production, ingress, or
release effect occurred.

## Turn 421 | 2026-09-21

P211 checkpoint `536d58a9` clears the broad provider-free support-lane gate.
The failure reproduced in six browser handoff and host fixtures whose v3
route-keeper authorities omitted the exact host-process claim for their active
generation. Production validation correctly failed closed. The repaired
fixtures register the complete process identity before starting a keeper.

The isolated workstation compartment passes all 215 tests, the browser
compartment passes 140 active tests with two browser-launch tests ignored,
strict workspace Clippy passes, and the complete provider-free runner passes
both lanes in 720 seconds. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next:
implement the provider-free configured Guacamole connector adoption join while
preserving the exact predecessor-exit proof and retained route identity.

## Turn 433 | 2026-09-22

Plan 0217 M0 and M1 establish the ordinary named-profile failure at source
baseline `8995ee0d` and move availability-first admission to the narrow shared
local `remote_view_open` seam. Historical session and owner records no longer
reserve an explicitly selected shared-local profile when no current process is
proved for that profile. Proved current processes, exact uncertain resources,
and cleanup-owned state remain protected. The regression failed before the
repair with `existing_session_profile_identity_unproven` and now passes.

Focused profile and remote-view groups, formatting, strict workspace Clippy,
documentation contracts, links, policy wiring, and the docs build pass. The
729-second comprehensive Rust run retained one non-green support result in an
untouched Lease Authority process-observation test. The exact test passed on
rerun, and the entire Lease Authority compartment then passed 118 tests. The
M0 cut-line note preserves the original result and the bounded reruns. This is
source qualification only. No browser, route, provider, Service State,
installed-runtime, production, ingress, or release effect occurred.

## Turn 420 | 2026-09-20

P211 source checkpoint `1ebfa757` advances the route-keeper authority to v3.
Every host generation now carries an append-only exact host-process claim: boot
epoch plus PID, start token, and executable identity. An active predecessor can
remain bound to its original claim while an absent slot is rebased for a newer
successor. A predecessor-exit proof is constructed only from those persisted
claims: a different boot proves exit, while a same-boot observation must prove
the exact predecessor missing or reused by an unrelated process. Ambiguous,
failed, or exact-live observations do not construct proof. Configured startup
registers its own exact claim, but continues to refuse cold recovery because the
configured Guacamole connector cannot adopt a retained primary task.

Diff hygiene, formatting, strict workspace Clippy, and all 44 focused
route-keeper tests pass. The broad provider-free runner ended nonzero after 815
seconds in its support lane; its slow workstation diagnostic rerun was stopped
without a failure diagnosis, so that broader receipt remains an explicit gate.
This is a source checkpoint only. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next: make
the broad support-lane failure reproducible or clear it, then design the
separate configured connector-adoption and runtime-evidence joins.

## Turn 419 | 2026-09-19

P211 source checkpoint `b0a82574` adds one proof-bound, provider-free
cold-process recovery seam for a retained route. Persistence cannot construct
the private predecessor-exit proof. The proof binds the complete prior ready
receipt and a strictly newer successor host generation. Recovery first persists
the exact disconnect, then prepares or replays one adoption. Compare-and-swap
conflicts, route rebound, and a foreign operation ID fail before connector
adoption. An interrupted connector call leaves replayable `Adopting` state, and
a missing exact receipt returns `Pending` instead of claiming readiness.

The first unsafe tracer was rejected because it manufactured disconnect proof,
could strand partial multi-route recovery, and blurred whole-authority startup.
The replacement passed four focused red-green cycles, all 42 route-keeper
tests, formatting, strict workspace Clippy, and two closed-world reviews. This
checkpoint does not construct predecessor process proof, rebase absent slots,
clear quarantine, implement configured connector adoption, or remove the
configured startup refusal. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred. Next:
bind exact predecessor process-exit evidence to this private proof without
enabling live provider recovery.

## Turn 418 | 2026-09-19

P211 implementation checkpoint `da9438a7` resolves Browser Session Manager
handoffs only through the current SQLite route-keeper authority. The keeper
catalog digest now binds the reviewed public operator origin with each exact
connection binding. Journaled opens validate their persisted slot, display,
handoff identity, and opaque URL before browser effects, then atomically commit
manager state and the handoff. Navigation validates the complete keeper-backed
route vector, and resolution reloads both the SQLite handoff registry and
current keeper authority before focus. Public responses retain only the durable
`/remote-view/<handoff-id>` URL and provider-neutral presentation semantics.

Qualification checkpoint `3922f138` adds the missing daemon-boundary regression
for SQLite lookup, current keeper reload, and failure before focus. The focused
test, formatting, strict workspace Clippy, the development-provider fixture,
the prior focused keeper, host, store, and model gates, and the complete
provider-free Rust runner pass. The comprehensive run completed in 818 seconds
with both lanes at zero. Source and remote match at `3922f138`. This remains
provider-free source qualification. No provider, browser, Service State,
installed-runtime, production, ingress, or release effect occurred.

The documentation closeout reconciles CLI help, README, the repository skill,
the dashboard and remote-view guides, inline comments, planning projections,
and the active-lane catalog. The remote-view documentation contract, link
checker, production docs build, six selected workstation and Guacamole
fixtures, planning audit, and goal audit pass. The shared installed skill was
not mutated; experimental guidance remains repository-scoped pending its
governed development-runtime publication step.

Plan 0211 remains `OPEN`. Cold-process adoption or recovery for durable
non-`Absent` keeper state is the next bounded implementation packet. Live
supervisor health and readiness, public capacity use, remaining SQLite domains,
shared Desktop Services control, the frozen development candidate, and the
isolated cold-start matrix remain open. Production and external ingress remain
excluded.
