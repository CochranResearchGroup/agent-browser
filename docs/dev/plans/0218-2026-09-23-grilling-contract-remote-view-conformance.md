# Plan 0218 | Grilling-Contract Remote View Conformance

Date: 2026-09-23

Plan version: 12

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P218

Predecessor: [Plan 0217](0217-2026-09-22-availability-first-remote-view-reliability.md), superseded because it covered only availability and visual reliability rather than the complete accepted grilling contract

Original design authority: Codex thread `01a0b65d-47f9-7b51-a17b-791ee87769b3`, grilling exchange on 2026-09-19

Audit basis: accepted user and assistant design turns 207 through 349 in the
original design authority, first reconciled against branch head `c627fd4b`,
then structurally audited with an up-to-date CodeGraph index at `6800163f` on
2026-09-23, and re-audited against checkpoint `d0c2a4cf` on 2026-09-24

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, and `CochranResearchGroup/agent-browser#195`

Branch: `platform/p211-simple-cold-upgrade` with inherited P211 and P217 custody

Pull request: draft PR #191

Target: `main`

Overall effort ceiling: the inherited Plan 0217 cumulative 2,000,000-token
ceiling was exhausted. The durable pre-M0 readback was 595,176 tokens and the
M0 control readback was 340,551 tokens including worker effort, for 935,727
cumulative at that checkpoint. The completed downstream CLI closure goal then
used 1,446,348 tokens, bringing the reconciled Plan 0218 cumulative total to
2,382,075 tokens. On 2026-09-23 the operator explicitly directed Plan 0218 to
continue with a new checkpoint at 500,000 tokens. That instruction renews one
bounded execution window and revises the cumulative ceiling to 2,882,075
tokens. It does not reset prior usage or authorize effects excluded elsewhere
in this plan. The new goal's live counter is authoritative for this window;
stop for operator review when it reaches 500,000 tokens.

## Objective

Make Agent Browser implement the complete architecture accepted during the
September 19 grilling session. In the trusted single-user runtime, a valid
instruction to open or recover a browser must produce a usable browser and
authenticated remote view unless a concrete current resource, configuration,
authentication, or execution failure makes that impossible.

Historical uncertainty is diagnostic evidence, not permission authority.
Lease claims, fencing, custody, quarantine, cleanup obligations, retained owner
records, stale inventory, and equivalent renamed concepts must not deny an
ordinary open, reserve phantom capacity, substitute an unauthenticated profile,
or prevent creation of a separate working route. Exact uncertainty may prevent
destructive cleanup of the exact uncertain resource.

The runtime must use one provider-owned presentation authority, one
transactional SQLite authority, one durable opaque handoff contract, and one
Desktop Services control authority. It must reconstruct usable presentation
capacity after a cold start without hidden infrastructure browsers, mutable
route inventories, operator-managed display numbers, or runtime fallback to
legacy JSON.

## Current State

P211 implemented substantial SQLite, operation-journal, route-keeper,
capacity, handoff, desktop-control, and migration foundations. P217 commit
`9bdcfbe3` removed `remote_view_open` from generic profile-lease classification,
resolved an explicit ordinary profile before legacy owner evidence, and added
an architecture guard against restoring that coupling.

The complete grilling contract is not implemented or accepted.
`BrowserSessionHost` commits its manager state through
`BrowserRuntimeSqliteStore`, but adjacent ordinary remote-view and dashboard
handoff resolution, browser lifecycle, monitor, desktop, and Service request
paths still consume JSON Service State. Runtime route and provider inventory
environment variables remain live, the development provider keeps the
route-keeper readiness interlock hard-coded false, history budgets are
configuration without enforced compaction, verified database backup and
restore are absent, and joined cold-start plus external visual recovery
acceptance has not passed. Explicit administrative and adversarial Lease
Authority contracts remain supported, but they are not ordinary runtime
authority.

The 2026-09-23 source audit found that the branch already contains useful
provider-free implementations for SQLite defaults, forward-only migration,
operation and host-generation fencing, route-keeper supervision, capacity
growth, least-loaded placement, durable admission queueing, cooldown scale-in,
navigation and handoff recovery, and atomic Desktop Services control
publication. Those are implementation foundations, not whole-contract
acceptance.

The same audit found five classes of remaining conformance work:

1. adjacent ordinary remote-view, dashboard handoff, lifecycle, monitor,
   desktop, and Service request paths still read JSON Service State even though
   Browser Session Manager state is SQLite-backed, so the authority boundary
   is incomplete;
2. the development provider still represents route-keeper runtime readiness as
   false, and provider-backed joined startup has not qualified the source
   foundations;
3. history compaction, integrity checking, verified online backup and restore,
   provider-credential access, and full disposable-profile quota mutation are
   schema or configuration foundations without the accepted operational
   behavior; `BrowserRuntimeConfigPatch` currently mutates only the seven
   presentation capacity, queue, deadline, and cooldown fields rather than all
   accepted live settings;
4. no one frozen installed candidate has passed the joined cold-upgrade,
   cold-start, failure-injection, external visual, responsive-input, and fresh
   process-residue matrix;
5. the old Lease Authority and runtime-owner system is blocked from ordinary
   `remote_view_open` admission by a test and source guard, but it remains a
   direct dependency of both `agent-browser-service-model` and the CLI. Its
   state and operations are still embedded throughout `ServiceState`, so it is
   not yet quarantined out of the trusted single-user product build.

The 2026-09-23 CodeGraph initialization indexed 961 files into 36,717 symbols
and 139,589 edges and reported the index up to date. Structural exploration
confirmed the SQLite-backed Browser Session Manager seam, the remaining
Service State consumers, route-keeper cold recovery and host-generation
fencing, stable capacity selection, and the partial configuration mutation
surface.

M0 is complete at the version 7 checkpoint. The repository-owned coverage
manifest contains exactly G01 through G45 with 26 partial, 8 failed, and 11
missing rows. The frozen ordinary-open-through-handoff closure records the
three required M1 cuts from default host to legacy state, handoff finalization
to legacy state, and legacy Service State to Lease Authority. The standalone
architecture gate reports 8 definite violations and leaves the other 11
prohibitions explicitly unverified rather than treating detector absence as a
pass. Its enforcement mode intentionally exits nonzero until M1 repairs the
red set. No product behavior or runtime state changed in M0.

## M0 Checkpoint

- Source baseline: `e7c27f7810b9961a9223807a83d8c40bb22260b1` on
  `platform/p211-simple-cold-upgrade`, five commits ahead of its remote when M0
  started. Installed `agent-browser` and `agent-browser-dev` both reported
  `0.28.0`; no installed candidate or service was changed.
- Coverage authority:
  `docs/dev/contracts/p218-grilling-contract-coverage.v1.json`, validated by
  `scripts/dev/test-p218-grilling-contract-coverage.mjs` for exact ordered
  G01 through G45 closure, required fields, statuses, and prohibition IDs.
- Dependency authority:
  `docs/dev/architecture/p218-ordinary-open-handoff-closure.v1.json`, validated
  by `scripts/dev/test-p218-architecture.mjs` for the frozen nodes, edges, and
  three required M1 cuts.
- Red gate: `scripts/dev/check-p218-architecture.mjs`. Report mode preserves
  the M0 baseline; `--enforce` exits 1 for P02, P03, P05, P09, P12, P15, P16,
  and P19. The other 11 prohibitions remain explicitly unverified rather than
  inferred from static source or detector absence.
- Compiler protocol: `scripts/dev/p218-compiler-diagnostics.mjs` consumes an
  explicit retained Cargo JSON-lines artifact, filters first-party errors,
  deterministically normalizes, deduplicates, classifies, diffs waves, and
  emits compact JSON plus a worklist. Its hermetic test covers all five
  classes and all four prior-wave dispositions without invoking Cargo.
- Worker receipt `/root/m0_compiler_helper`: requested `gpt-6-luna` at medium
  effort; runtime model report unavailable; accepted two disjoint helper files
  after 5 focused tests passed.
- Worker receipt `/root/m0_coverage_manifest`: requested `gpt-6-luna` at medium
  effort; worker reported GPT-6 medium; accepted its schema and validator, then
  primary reconciliation replaced the overly conservative 40-missing-row
  evidence depth with the frozen 26 partial, 8 fail, and 11 missing inventory.
- Progress classification: `outcome_progress`. M0 made omissions and current
  violations mechanically visible and intentionally performed no behavior
repair. M1 is the next milestone; its first cut is removal of the default
product Cargo edge to Lease Authority using only the deterministic compiler
helper and the three frozen closure edges.

M1 cut 1 began on 2026-09-23 by freezing
`docs/dev/contracts/p218-m1-replacement-interfaces.v1.json`. The contract
allows only the Browser Session Manager and SQLite host interfaces for
ordinary authority, one data-only principal-provenance value type, and a
non-default migration diagnostic boundary. It explicitly forbids moving or
renaming lease, runtime-owner, custody, quarantine, cleanup-admission, JSON
fallback, or dual-write concepts into a replacement API. The
`agent-browser-service-model -> agent-browser-lease-authority` Cargo edge was
then removed to start the compiler-driven excision wave; the package is
expected to remain uncompilable until the classified first-cut groups are
resolved.

The first retained compiler wave is
`/tmp/agent-browser-p218-m1-wave-1`. Cargo exited 101 as expected after the
edge removal. The compact manifest contains 7 groups and 76 occurrences: one
group each in `abandoned_browser_retirement.rs`, `profile_lease.rs`,
`runtime_owner_projection.rs`, and `session_tab.rs`; two in
`principal_continuity.rs`; and one 70-occurrence `ServiceState` group. The raw
JSON SHA-256 is `2d9d687ec6291a8fb3593648c202762be5439e7bc6ae987921b0ae8a71dcbeda`.
The data-only principal-provenance type now belongs to the Service model, and
the `profile_lease.rs` and `session_tab.rs` leaf imports no longer reference
Lease Authority. The other groups remain open because they carry operational
lease, runtime-owner, or cleanup-admission behavior and cannot be mechanically
renamed into the default product.

M1 compiler wave 2 followed source checkpoint `aca80104`. Abandoned-browser
retirement no longer imports Lease Authority, serializes runtime-owner terminal
states, or fences profile-claim acquisition from a pending historical cleanup
record. The compact wave at `/tmp/agent-browser-p218-m1-wave-2` reports 4
groups and 71 occurrences, with 3 resolved, 0 new, 4 repeated, and 0 regressed
against wave 1. Its raw JSON SHA-256 is
`4d018828989d82bc51ff791a82e4cab51a0ab1101f7c0cb8a70831bb11f5e044`.
The cut-specific guard now names only `principal_continuity.rs`,
`runtime_owner_projection.rs`, and `service_state.rs`. The package remains
intentionally uncompilable; the next packet must remove one of those coherent
authority surfaces rather than translate it into a renamed product type.

Source checkpoint `c326a161` deletes the complete Service-model
`runtime_owner_projection` module and its `ServiceState` projection methods.
This removes 703 lines that exposed owner, attestation, lifecycle, resource-lane,
and session-binding authority to default-product consumers. The cut-specific
guard now names only `principal_continuity.rs` and `service_state.rs`. CLI
callers are intentionally unresolved until their corresponding legacy product
surfaces are deleted; no third compiler wave was started in this packet.

Source checkpoint `d05060ec` deletes the 1,101-line principal-continuity module
and its `ServiceState` work-lease wrappers. Default-product Service-model code
no longer derives runtime-owner principal recourse, binds subordinate work
leases, or projects legacy principal migration as live behavior. The
cut-specific guard now names only `service_state.rs`, which remains the final
primary-owned Service-model authority cut. Downstream errors remain
unclassified until the next retained compiler wave.

Compiler wave 3 at `/tmp/agent-browser-p218-m1-wave-3` exited 101 and reports
2 groups with 62 occurrences: one new mechanical `profile_lease.rs` reference
to the deliberately deleted principal-continuity recourse enum, and one
repeated 61-occurrence `service_state.rs` Lease Authority group. Compared with
wave 2, 3 groups resolved, 1 is new, 1 repeated, and 0 regressed. The raw JSON
SHA-256 is `0c2250defab91cf770478c9d370c6ca230efb9ad12ae0f42c63f10b0250bcdef`.
The new downstream error is a deleted-product-surface consequence that the
generic helper classifies mechanically; it must be resolved by deleting the
remaining profile-lease surface, not restoring or renaming the recourse enum.

Source checkpoint `2ce55e07` deletes the 296-line profile-lease record module
and its exports rather than restoring the deleted continuity-recourse type.
The cut-specific guard still names only `service_state.rs`. No compiler wave
followed this deletion; `ServiceState` is the sole remaining direct source
group and owns the next packet's architectural decision.

The next primary-owned source cut removes the remaining typed Lease Authority,
principal-registry, and runtime-owner state, methods, persistence wrappers, and
authority-only tests from `service_state.rs`. The cut removes 2,193 lines and
adds no replacement authority surface. The cut-specific architecture gate now
passes with no findings. Compiler wave 4 at
`/tmp/agent-browser-p218-m1-wave-4` exited 101 with 4 groups: one mechanical
stale export of `RuntimeOwnerPersistenceParts` and three primary architecture
groups for deleted profile-lease schema and receipt surfaces. Compared with
wave 3, 2 groups resolved, 4 are new, none repeated, and none regressed. Its
raw JSON SHA-256 is
`9cd2d9d7e5c5b3d82f941bae1822291e0b28466c48e3e02db3b125f100f5178a`.
The packet stops at the second causal group as required; the Service model does
not yet compile and M1 is not accepted.

The immediate cleanup packet removes all four wave-4 groups: the stale
runtime-owner persistence exports, the obsolete profile-lease schema marker,
and the reconciliation receipt field, methods, and tests. Compiler wave 5 at
`/tmp/agent-browser-p218-m1-wave-5` exits 0 with no groups; all 4 prior groups
resolved with no new, repeated, or regressed group. Its raw JSON SHA-256 is
`793a9d24c7b916e8dc38616e43e0ddc221d70655e81dd81bf81cec560629da1c`.
The focused `agent-browser-service-model` library suite passes all 195 tests
after synchronizing the abandoned-retirement fixture and frozen digest with
the already-removed terminal owner fields. This satisfies the Service-model
portion of M1 cut 1; downstream CLI closure remains open.

CLI compiler wave 6 at `/tmp/agent-browser-p218-m1-wave-6` established the
downstream baseline: 142 groups and 298 occurrences after the Service-model
cut, with raw JSON SHA-256
`612b366525ce188c403bdc8816719ebf378db3da842db2cdf59e57c5aeb546a4`.
Checkpoint `135e3b51` then removes the CLI Lease Authority Cargo edge and the
legacy authority, principal, profile-lease, recovery, and runtime-owner
subsystem. Checkpoint `df7eaf64` removes the associated adoption,
reconciliation, retirement, boot-epoch, retained-state, and state-migration
operations. Together these checkpoints delete 28,824 lines without adding a
replacement authority surface.

Compiler wave 7 at `/tmp/agent-browser-p218-m1-wave-7` exits 101 with 157
groups and 355 occurrences: 80 wave-6 groups resolved, 95 new downstream
integration groups surfaced, 62 repeated, and none regressed. Its raw JSON
SHA-256 is
`6d0ea9ed2e272d4bec41383453eba4ec25313e3e249834d6667d6d25a967a6b1`.
The remaining causal boundary is command dispatch plus action-runtime,
health/status, workstation, HTTP, and MCP integration; the CLI does not yet
compile and M1 remains open.

The first capsule-pilot admission measured the 200-word capsule at 2,289 bytes
and approximately 573 tokens, versus 9,244 words, 66,200 bytes, and
approximately 16,550 tokens for the eight canonical policies. Using the
declared UTF-8-bytes-divided-by-four proxy, one verified reuse avoided an
estimated 15,977 ingestion tokens. No canonical fallback or capsule ambiguity
occurred. These are ingestion estimates, not exact context or billing data.

The second capsule-pilot admission also passed `pnpm test:policy-capsule` and
reused the capsule without a canonical fallback, re-read trigger, or observed
behavioral defect. Across two verified loads, the pilot loaded an estimated
1,146 capsule tokens instead of 33,100 canonical-policy tokens, for estimated
gross ingestion savings of 31,954 tokens. This remains a byte-based estimate,
not exact model-context or billing measurement.

The downstream CLI closure continued through custody checkpoint `c1a99cf8`,
which removed the hot workstation upgrade authority and deleted 21,775 lines.
The following uncommitted closure packet removed the protected authority
process entry points, runtime-admission command injection, authority-selected
profile routing, reviewed legacy process-tree shutdown, retained-owner launch
repair, runtime-owner Service State sidecar loading and writes, and more stale
command and daemon integration. It also restored owner-free close and snapshot
behavior and replaced the retained resource tree with a diagnostic-only
surface.

Compiler waves 8 through 14 remain retained under
`/tmp/agent-browser-p218-m1-wave-<n>`. Wave 13 exited 101 with 58 groups and 85
occurrences after the entry-point cut; its raw JSON SHA-256 is
`52ba5ea3500f8367be22ff31c17f30b5fe01f36a526a61d2dd0ad662718988ca`.
Wave 14 exited 101 with 80 groups and 114 occurrences after the process-tree,
retained-launch, and Service State sidecar cut. It resolved 15 wave-13 groups,
surfaced 37 downstream groups, repeated 43, and regressed none. Its raw JSON
SHA-256 is
`3c4095db4381800f548c65e0e77f4461618a4fb8868e5e6288d41084d9c2f9bb`.
The 80 groups classify as 43 mechanical removals, 8 neutral type extractions,
and 29 primary architecture decisions. The CLI therefore remains
intentionally uncompilable, and the M1 downstream closure milestone is not
accepted.

The goal-level runtime reported 507,008 tokens before closeout, exceeding the
450,000-token M1 allocation and consuming the requested 15 percent closeout
reserve. Work stopped at that explicit budget boundary. The next bounded
packet must start from the wave-14 worklist, remove the remaining launch,
remote-headed, health, HTTP, MCP, and dashboard authority surfaces, and avoid
restoring placeholder lease or runtime-owner APIs merely to make the compiler
green.

The successor goal selected the coherent downstream CLI dependency-closure
milestone rather than claiming all of M1. Custody checkpoint `d60d7b16` and
the following closure batch remove the remaining default-product calls into
Lease Authority and runtime-owner behavior from launch, remote-headed,
health, status, Service request, HTTP, MCP, dashboard, workstation, and
handoff paths. The retained final compiler artifact is
`/tmp/agent-browser-p218-m1-wave-20`: Cargo exits 0, the compact manifest has
zero groups, all 42 wave-16 groups resolve, and no group is new, repeated, or
regressed. Its raw JSON SHA-256 is
`06b3ef88f2e692c1e7a96d312d0bf65cea2787ef8628fc5ca2dc68abc60b1567`.
The aggregate architecture gate remains intentionally red for P02, P03, P05,
P09, P12, and P19, but P15 now reports zero findings. This accepts only the
default-product Lease Authority closure milestone; SQLite authority, durable
handoff, viewer authority, and operation-closure work remain separate M1
packets.

Downstream reconciliation restores invariant-aware Service State decoding,
updates the Lease Authority architecture test from requiring the CLI edge to
forbidding it, removes stale profile-lease actions and collection probes, and
makes the optional presentation provider non-blocking when it is genuinely
unconfigured. The no-launch collection fixture now performs the required
forward SQLite migration before daemon startup. A stale workstation fixture
assertion against the deleted Service State migration preview was replaced by
the current source-free workstation plan contract. Thirty-six authority-bound
legacy test modules are excluded from the default test build and four retained
inspection or recovery modules carry module-level dead-code allowances. This
is explicit quarantine and reduced legacy coverage, not evidence that those
deleted-product behaviors remain supported; subsequent packets must replace
coverage only for retained product contracts rather than re-enable Lease
Authority behavior.

Validation for this milestone includes workspace check, workspace test
compilation, strict workspace Clippy, formatting, the focused native action,
service, and stream compartments, the CLI integration compartment, Service
State validation, route-confusion gates, Service API and MCP parity, Service
client contract and type checks, Lease Authority architecture and self-tests,
the P218 compiler and coverage helpers, the P218 architecture report, the
no-launch Service collection smoke, and workstation install and Guacamole
asset fixtures. The first comprehensive Rust run exposed over-pruned fixture
exports and invariant-blind Service State parsing; both were repaired and the
affected compartments passed. The comprehensive runner was not replayed after
those exact focused repairs. One `sccache` incompatibility was retried with the
repository-supported cache opt-out and passed; one invalid compartment name
was command rework only.

The third capsule-pilot admission passed `pnpm test:policy-capsule` and reused
the capsule without a canonical fallback, re-read trigger, material ambiguity,
or observed policy-behavior defect. Across three admissions, the pilot loaded
an estimated 1,719 capsule tokens instead of 49,650 canonical-policy tokens,
for estimated gross ingestion savings of 47,931 tokens. The source measurements
remain 2,289 bytes and 200 words for the capsule versus 66,200 bytes and 9,244
words for the eight policies. These are byte-based ingestion estimates, not
exact model-context or billing measurements. No canonical policy fallback
occurred; the capsule should be retained for matching P218 M1 turns while its
hash verification and explicit trigger contract remain current.

The successor goal's pre-closeout runtime readback is 1,441,501 tokens; the
final goal receipt remains authoritative because closeout itself adds usage.
The goal produced four custody commits (`c1a99cf8`, `d60d7b16`, `5d932f3a`,
and this evidence closeout), six retained compiler waves numbered 15 through
20, and 24 distinct successful validation gates. One comprehensive Rust batch
failed before the focused fixture-export and Service State parser repairs and
was not replayed in full; every affected focused compartment passed afterward.
Rework comprised four implementation or fixture clusters: over-pruned test
exports, invariant-aware State parsing, fresh-daemon no-launch migration and
optional-provider handling, and the stale workstation preview assertion.
Operational rework comprised one unsupported `sccache` invocation, one invalid
compartment name, and one redundant Cargo launch attempt. No installed runtime,
provider, tenant, ingress, production, push, merge, or release effect occurred.

The operator-renewed execution window next selected the bounded P12 and P19
stored-viewer-authority cut. The primary removed the persisted `ViewerLease`
collection, route and stream viewer-lease ID lists, viewer request, heartbeat,
release, and takeover dispatch, and the dashboard controls that manufactured
viewer presence. Session heartbeat remains unchanged. Capacity and
attachability now project no live viewer evidence, and configured desktop
interaction fails closed until M3 supplies an authenticated live Guacamole
observer. The separately fenced Desktop Services controller ID and epoch still
exist as stored control state. They are not viewer-presence proof, and the
2026-09-24 re-audit found that their use by operator focus is not yet sufficient
G45 authority because no live viewer heartbeat currently governs how long that
control remains held.

Retained compiler wave 24 at `/tmp/agent-browser-p218-m1-wave-24` exits 0 with
zero first-party groups after resolving all 52 wave-21 production groups. Its
raw JSON SHA-256 is
`101fdc6d15c04628ab05ae0b19b166af1d3e98a1fa33581c63aa0589edc95ee4`.
The Service Model package passes 195 library tests and all 71 package
integration tests. The aggregate architecture report now has four definite
violations, P02, P03, P05, and P09; P12, P15, P16, and P19 have zero findings
but remain detector gaps rather than accepted rows. The policy capsule,
formatting, diff check, and Service API and MCP parity gates pass.

This is an incomplete custody checkpoint, not P12 or P19 acceptance. Retained
workspace test-build wave 26 at `/tmp/agent-browser-p218-m1-wave-26` has 19
groups and 23 occurrences, all in legacy or replacement test fixtures that
still construct or assert persisted viewer records. The public viewer-lease
HTTP and MCP collection and service-request action metadata were removed, but
generated client and complete documentation reconciliation remain open. A
Luna import-only worker completed 19 disjoint removals. A second Luna fixture
worker was interrupted for the operator-requested checkpoint and made no
accepted fixture claim. The next packet must finish the exact wave-26 worklist,
regenerate and verify clients, reconcile all user-facing viewer-lease docs,
then run focused CLI compartments and strict Clippy before deciding P12 and
P19 acceptance.

## 2026-09-24 Bob/Alice Re-audit Disposition

The re-audit at `d0c2a4cf` does not change the plan objective, milestone order,
or primary topic. The current direction remains the accepted direction, but the
checkpoint is not G42 through G45 acceptance. Findings are incorporated into
the existing milestones as follows:

1. The provider-free Browser Session Manager core is directionally correct.
   All 26 focused `agent-browser-service-model` integration tests pass for
   exact-session refresh, same-profile browser reuse, distinct session and tab
   identity, attributed-tab cleanup, Alice-first browser preservation, expiry,
   and final-session cleanup. The stronger workstation Alice/Bob fixture also
   expresses separate targets and opaque handoffs, independent navigation and
   clicks, Alice-first cleanup preserving Bob, and final browser termination,
   but it is ignored and cannot become acceptance evidence while the complete
   CLI test build is broken.
2. M1 must finish the retained wave-26 fixture reconciliation and restore a
   compiling complete workspace test surface. It must also remove the deleted
   viewer-lease actions and records from generated clients, HTTP and dashboard
   allowlists, CLI help, README, skills, docs, schemas, tests, and generators.
   Internal API/MCP parity without those complete public surfaces is not
   sufficient.
3. G42 remains partial until the Browser Session Manager owns the complete
   ordinary lifecycle, including handoff identities and exact handoff-access
   heartbeat refresh. The separate host handoff map and JSON finalization path
   remain evidence for P03 and P05, not an accepted parallel authority.
4. G43 and G44 are substantially implemented but remain unaccepted until the
   provider-free Alice/Bob fixture runs in the normal validation surface and
   proves distinct session, tab, target, heartbeat, expiry, and handoff
   identities plus independent command and cleanup effects from one candidate.
5. G35 and G45 remain failed. M3 must introduce bounded authenticated
   Guacamole connection observation and disconnect detection, make that live
   heartbeat the only active-viewer evidence, and make it govern eager recovery
   and continued Desktop Services control. A stored controller ID or epoch may
   fence a control grant but cannot establish or indefinitely retain it without
   the current authenticated connection heartbeat. Until then, no stored
   controller record may be promoted as G45 proof.
6. The G01 through G45 coverage manifest must be reconciled before the next
   conformance verdict. Its G35 and G45 evidence still names removed persisted
   viewer state; those rows remain failed for the replacement gap and stored
   controller-retention issue instead. G42 through G44 must cite the current
   focused tests while retaining their incomplete acceptance status.
7. The aggregate architecture result remains four definite violations, P02,
   P03, P05, and P09; four detector gaps, P12, P15, P16, and P19; and eleven
   unverified rows. Zero findings from a detector-gap row are not acceptance.
   These findings stay within the existing M1 through M3 work and do not open a
   separate repair program.

The next bounded packet remains the wave-26 fixture and public-surface
reconciliation already selected at the version 11 checkpoint. After that packet
compiles, the next G42 through G44 packet consolidates handoff ownership and
runs the provider-free Alice/Bob acceptance fixture. M3 then supplies the live
viewer and heartbeat-governed control authority required for G35 and G45. This
ordering addresses every accepted re-audit finding without switching topics or
prematurely pulling provider-backed M3 work into the incomplete M1 excision.

The 2026-09-24 execution continuation stopped when the live goal counter first
became available above the operator's 500,000-token checkpoint cap; the readback
was 841,393 tokens. Active workers were interrupted immediately and no compiler
wave, commit, push, install, browser, provider, ingress, or production effect
followed that readback. The retained uncommitted packet removes obsolete viewer
fixtures, HTTP and dashboard routing special cases, generated client APIs,
dashboard viewer-lease planning and projection, contract schemas, and
user-facing guidance. Focused Browser Session Manager and dashboard stream
checks passed; the dashboard observer-frame fixture was under bounded repair
when interrupted. Remaining continuation work includes interrupted worker-diff
reconciliation, residual Rust and operational-script viewer references, the
stale actions responsibility inventory, the explicit P158-to-M3 dependency,
one new compiler diagnostic wave, selected validation, and a custody commit.

Worker receipt `/root/m1_service_model_inventory`: requested `gpt-6-luna` at
medium effort; effective runtime model was unavailable. Its read-only six-file
inventory was accepted, but its suggestion to extract authority-bearing lease
and runtime-owner DTOs into a shared product crate was rejected as contrary to
G41. A bounded follow-up edited only `profile_lease.rs` and `session_tab.rs`;
file-local formatting and no-reference checks passed. Worker receipt
`/root/m1_guard_validation_audit`: requested `gpt-6-luna` at medium effort;
effective runtime model was unavailable. Its read-only finding was accepted by
adding a cut-specific service-model dependency and source guard while leaving
aggregate P15 red until the CLI closure is removed.

The next operator-renewed continuation closed the retained wave-26 compiler
breakage and produced a bounded custody packet. Obsolete viewer-lease state was
removed from reconciliation fixtures and counters, compact status projections,
Guacamole binding tests, route-handoff audit output, development presentation
reference checks, and the actions responsibility inventory. The interrupted
dashboard observer fixture was repaired without restoring viewer actions. Its
lifecycle test and the dashboard view-stream test pass, as do the route-handoff
audit and development presentation provider fixtures. Workspace test
compilation at `/tmp/agent-browser-p218-m1-wave-27-no-cache` exits 0. The first
attempt at `/tmp/agent-browser-p218-m1-wave-27` failed in the optional sccache
wrapper before a source diagnostic; the supported cache-off retry is the code
verdict. Formatting, the coverage-manifest validator, and diff validation pass.
The aggregate architecture result remains intentionally red with four definite
violations, four detector gaps, and eleven unverified rows. P12 and P19 are not
accepted merely because compilation is restored: one disabled legacy viewer
policy fixture remains for deletion, P158 remains an explicit M3 dependency,
and the G42 through G45 authority work remains open. No push, install, browser,
provider, ingress, production, or other live effect occurred.

The following 500,000-token continuation first hardened the M1 policy capsule
for aggregate multi-agent efficiency. The capsule now selects subagent workflow
optimization and parallel-plan design, defaults workers to compact fresh
packets, admits them sequentially, polls aggregate usage at worker boundaries,
and reserves the final 30 percent for primary reconciliation and custody. One
fresh-context worker then implemented the bounded G42 handoff-membership packet.
`ManagedBrowserSession` now owns its opaque handoff IDs; journaled open and
navigation bind those IDs before their SQLite commit; successful handoff
resolution validates membership and persists activity and expiry for only the
exact session. Load-time migration binds an older ready manager handoff only
when its session, browser, profile, name, tab, target, and optional intent
attribution all agree. Focused Alice/Bob model and host tests, the SQLite keeper
reload test, complete workspace test compilation, strict workspace Clippy,
formatting, capsule validation, coverage validation, and diff validation pass.
The final disabled viewer-policy fixture was deleted, and Clippy fallout from
the preceding viewer excision was repaired without restoring stored viewer
authority. The remaining nonjournaled keeper-attach compatibility path now
publishes session membership and its handoff in one SQLite transaction. G42
remains partial until every remaining ordinary command and handoff-finalization
path is proven to use the manager-owned transaction boundary and the P03/P05
compatibility projections are removed. P158 remains an explicit M3 dependency,
and G35/G45 still require authenticated live Guacamole observation. No push,
install, browser, provider, ingress, production, or other live effect occurred.

## Grilling Contract Ledger

Every row is normative. Later implementation notes, safety language, renamed
types, or partial tests cannot weaken a row without explicit operator revision
of this plan.

| ID | Accepted decision | Required invariant |
| --- | --- | --- |
| G01 | Existing runtime host owns presentation authority | Browser Session Manager calls one typed in-process provider interface. No second daemon or provider database becomes authority. |
| G02 | No operator-maintained route inventory | The installer stores policy only. The provider deterministically owns route slots, users, credentials, Guacamole connections, and live display observations. |
| G03 | Legacy policy and runtime JSON are migration-only | One idempotent forward-only cold-upgrade migration imports valid user configuration, profile, session, handoff, and history state, records hashes and typed rejects, archives sources read-only, removes old runtime and unit authority plus exact Agent Browser-owned old processes, removes legacy variables, and never dual-writes, restores the old binary, or falls back. Provider route rows, route IDs, display numbers, leases, and connection rows are rebuilt rather than imported as authority. |
| G04 | One user-private SQLite authority | Ordinary browser, profile, session, tab, handoff, operation, presentation intent, configuration, history, cleanup-obligation, and provider-credential state have one transactional authority. JSON is limited to exports, diagnostic receipts, and migration archives outside the ordinary runtime dependency closure. |
| G05 | Stale history cannot veto work | Malformed, contradictory, ambiguous, or stale history is preserved diagnostically but cannot prevent a fresh valid browser request. |
| G06 | Presentation warms before browser launch | Startup reconstructs provider capacity. `minimumReady=1` unlocks service and `warmTarget=4` continues in the background. Readiness is false until one complete Guacamole, XRDP, and display path is usable. |
| G07 | Ordinary open waits for real capacity | A request waits up to the configured deadline, provisionally 90 seconds. Timeout or real exhaustion returns a typed failure before Chrome launches. |
| G08 | No hidden infrastructure browser | In-process protocol-level route keepers traverse Guacamole and retain XRDP sessions without Chrome, profiles, tabs, handoffs, or Browser Session Manager records. |
| G09 | Displays and browsers have deterministic placement | Prefer one browser per display while capacity can grow. At the provisional default maximum of six displays, overflow browsers may share the least-loaded healthy display up to the provisional density of four. Both limits are live configuration and remain subject to G33. |
| G10 | Capacity has bounded queuing | After display and density limits, a configurable bounded queue, provisionally 32, prioritizes active recovery, durable handoff access, then aged FIFO new opens. Duplicate browser/profile requests coalesce. |
| G11 | Desktop Services owns control | One fenced per-display Desktop Services lease governs handoff activation, focus, maximize, capture, pointer, keyboard, and future desktop automation. Opening another browser transfers control visibly; prior viewers become view-only. |
| G12 | Handoff identity is durable | One opaque authenticated `/remote-view/<handoff-id>` survives runtime, provider, Guacamole, display, browser, and tab recovery for the same logical session. It ends only by explicit close, configured retention, or a typed unrecoverable condition. |
| G13 | Handoff resolution performs bounded recovery | Resolution restores capacity, chooses a display, reuses or launches exactly one browser with the same profile, recreates the tab at its last committed URL, focuses and maximizes it, and returns the viewer. It never replays clicks, submissions, downloads, forms, or other page effects. |
| G14 | Recovery is eager only when useful | Baseline presentation capacity is eager. Active viewers recover eagerly. Dormant browsers recover lazily when their handoff or named session is accessed. |
| G15 | Recovery is bounded and singular | Active recovery retains the same handoff and retries with bounded backoff for up to the configured deadline. It proves an old browser unusable before launching at most one replacement and never creates competing replacements. |
| G16 | History is useful and bounded | Store compact structured lifecycle, navigation, display-binding, browser-replacement, recovery, and compaction events. Do not store raw logs, screenshots, bodies, heartbeats, or repeated polls. |
| G17 | Exact and summarized history coexist | Retain material lifecycle and recovery events indefinitely. Keep exact URL transitions within 64 MiB, then summarize oldest navigation by day with first URL, last URL, count, identities, incidents, and an auditable compaction event. |
| G18 | SQLite is recoverable | Use durable WAL transactions, integrity checks, one rotating verified online backup, a 96 MiB live database target, and a 128 MiB routine database, WAL, and backup budget. Preserve corrupt databases and record restoration gaps. |
| G19 | Operations are crash-consistent | Session admission, browser identity, presentation reservation, and handoff identity share one durable operation. Waiting operations do not surprise-launch after restart; begun effects reconcile or compensate; idempotent clients resume by operation ID. |
| G20 | Privilege is installed once and used only as needed | Initial install may establish one narrowly scoped noninteractive helper. Healthy cold starts make zero privileged calls. Each exact repair records resource, prior observation, action, and postcondition. Shared XRDP is never broadly restarted. |
| G21 | Disposable retention is bounded | Defaults are 24 inactive hours, 20 retained profiles, and 10 GiB, all live user settings. Oldest inactive disposable sessions expire first. Active-viewer, controlled, pending-operation, and named-profile sessions are not evicted. A disposable session has no separate pin state. |
| G22 | Configuration is typed and inspectable | Typed Service configuration and `agent-browser config get/set` own every accepted capacity, deadline, cooldown, retention, quota, history, and database-budget setting. Dashboard configuration remains read-only in this plan. Mutable environment policy is not runtime authority; environment variables are limited to immutable process bootstrap such as runtime identity and database location. |
| G23 | Valid user instructions are availability-first | Ordinary named-profile open cannot consult or be denied by legacy lease admission, historical owner proof, quarantine, cleanup obligations, or equivalent denial-first metadata. Only current concrete failures may deny it. |
| G24 | Development acceptance precedes production | Implement and validate in the isolated development runtime. Production, external ingress publication, formal release, and merge require separate authority after accepted development evidence. |
| G25 | Provider credentials follow the SQLite authority | Generated provider credentials are stored only in the user-private SQLite authority and are projected transiently into the user runtime directory or container secret mount. Transient secret files are removed on shutdown; rebuilt route-user credentials may rotate without changing handoff identity. |
| G26 | Runtime generations fence every effect | Each runtime-host start claims a monotonically increasing generation. Provider effects, browser effects, callbacks, operation completion, and recovery commits require the current generation plus operation ID; stale generations cannot commit. |
| G27 | Browser placement is stable | Prefer unused healthy displays before sharing the least-loaded healthy display. A browser keeps its display binding during ordinary operation and is not routinely migrated for load balancing; only bounded failure recovery may rebind it. |
| G28 | Extra capacity scales in safely | Only an empty, exact-reference-free display above `warmTarget` may retire after the configurable cooldown, provisionally 10 minutes. A final browser, handoff, viewer, operation, and keeper reference check precedes the effect. |
| G29 | Limit changes converge without eviction | Valid configuration updates commit transactionally, enter history, and take effect without reinstall or restart. Lowering a limit below current usage reports a visible over-target state, admits no worsening work, and converges through ordinary release without killing browsers or viewers. |
| G30 | Status and doctor expose reconciliation | Report configured versus observed capacity, keeper health, display generations, browser counts per display, pending operations, queue depth, Desktop Services control ownership, handoff recovery, migration receipt, database integrity and size, resource pressure, and privileged repair receipts. Credentials and raw provider URLs remain redacted. |
| G31 | Repeated opens are idempotent | Repeated or concurrent `remote_view_open` for the same logical session and tab returns and activates one existing handoff and coalesces under one operation ID. A genuinely new logical tab gets a distinct handoff; explicit session closure makes prior handoffs terminal. |
| G32 | Recovery URL means committed top-level navigation | Persist a URL only after Chrome reports committed top-level navigation, including operator-driven navigation. Ignore provisional redirects, subframes, and bootstrap pages; retain bounded redirect history and recover the final committed top-level URL. |
| G33 | Launch obeys live resource pressure | Even below display and density limits, each browser launch passes current memory, process, and disk admission. Pressure rejection is typed and does not disturb existing browsers, displays, or viewers. Development stress evidence must validate or lower the provisional density of four. |
| G34 | Provider route state is derived | Agent Browser-owned Guacamole database rows, users, connections, sharing profiles, route IDs, display numbers, and leases are disposable derived infrastructure. Cold upgrade rebuilds the owned namespace from SQLite policy and retained operator inputs rather than migrating provider rows. |
| G35 | Live viewer authority is observational | An authenticated remote-view client connection and its live Guacamole tunnel, with bounded heartbeat and disconnect detection, are the only authority that a viewer is active. Stored URLs, tabs, database flags, and previously opened pages are not active-viewer proof. |
| G36 | Named and disposable handoffs differ | Named-profile handoffs have no default time expiry. Disposable-profile handoffs expire after the configured inactivity period, provisionally 24 hours; expiry closes the logical session, makes its handoffs terminal, reference-checks and deletes its disposable profile, and retains compact history. Browser reclamation alone preserves either kind of recoverable handoff and profile. |
| G37 | Disposable quotas fail cleanly | Count and byte limits are enforced with oldest-inactive-first cleanup. If protected sessions prevent sufficient cleanup, reject the new disposable request with a typed quota or low-disk result. Never delete named profiles or protected disposable sessions to admit new work. |
| G38 | Disposable promotion is deferred | There is no disposable-session pinning or profile-promotion implementation in this plan. Until a future explicit promotion feature exists, every disposable profile remains subject to G36 and G37. |
| G39 | Doctor is read-only | `doctor` observes and recommends but never repairs. Startup and requests may invoke the same narrowly typed automatic reconciler, and an explicit `repair` command may request it, but neither path gains arbitrary commands or broad restart authority. |
| G40 | Waiting work does not surprise-launch | Queued operation identity and outcome persist for idempotency, but work that was only waiting becomes retryable after restart and launches nothing until the client resumes it. Effects already begun reconcile or compensate under G19 and G26. |
| G41 | Legacy lease denial is physically quarantined | The trusted single-user `agent-browser` CLI, runtime binary, and `agent-browser-service-model` do not compile, link, embed, deserialize, expose, or dispatch Lease Authority, runtime-owner, lease-recovery, lease-dashboard, or lease-MCP code. The extracted `agent-browser-lease-authority` crate may remain an independently buildable workspace member with its own tests, but no default product package depends on it. Historical lease and owner data are migration diagnostics only and cannot become live runtime state. |
| G42 | Session management is heartbeat-based | One Browser Session Manager owns the complete ordinary lifecycle. A named session contains its profile, browser, tab, last activity, expiry, and handoff identities. A successful command or handoff access refreshes only that exact session. Current heartbeat time and direct browser/process observation determine liveness; historical ownership, lease, custody, quarantine, and cleanup records do not participate. |
| G43 | Same-profile sessions share without aliasing | Alice and Bob may reuse one healthy browser when they request the same exact profile, but each receives a distinct session, tab, target, heartbeat, expiry, and opaque handoff. Every command resolves through the named session to its exact tab. Alice's navigation, clicks, closure, or expiry cannot mutate Bob's tab or heartbeat. |
| G44 | Cleanup follows active-session references | Closing or expiring Alice removes only Alice's session and tab while Bob keeps the shared browser alive. The browser closes only after Bob, the final active session, closes or expires. Reopening a healthy named session is idempotent and reuses its existing session, browser, tab, target, and handoff rather than creating parallel ownership state. |
| G45 | Session and viewer heartbeats stay distinct | A session heartbeat governs logical browser-session retention. An authenticated Guacamole connection heartbeat governs whether a remote viewer is active, whether recovery is eager, and whether Desktop Services control remains held. Neither heartbeat creates a lease or admission claim, and neither may be reconstructed from a stored flag, URL, tab, or historical event. |

## Simple Session State Machine

The ordinary trusted single-user runtime implements this model directly:

```text
open Alice on profile work
  -> create or reuse browser(work)
  -> create Alice session and Alice tab
  -> issue Alice handoff
  -> record Alice heartbeat

open Bob on profile work
  -> reuse healthy browser(work)
  -> create Bob session and Bob tab
  -> issue Bob handoff
  -> record Bob heartbeat

command Alice
  -> resolve Alice session -> Alice tab
  -> perform command
  -> refresh Alice heartbeat only

close or expire Alice
  -> remove Alice session and Alice tab
  -> preserve browser(work) because Bob remains

close or expire Bob
  -> remove Bob session and Bob tab
  -> close browser(work) because no active session remains
```

The durable ordinary model contains no lease claim, owner generation,
quarantine state, cleanup obligation, recovery authorization, acquisition
receipt, or equivalent renamed denial record. Operation IDs and runtime-host
generations provide crash consistency for effects; they do not grant admission
authority or outlive their exact operation.

## Architectural Prohibitions

The following are compile-time or deterministic source-contract failures:

1. `remote_view_open` depends on legacy profile-lease admission, Lease Authority,
   historical runtime-owner proof, quarantine, or cleanup-obligation modules.
2. Browser Session Manager reads route inventory, display numbers, provider
   inventory, or policy from environment variables or generated JSON files.
3. Ordinary runtime code falls back to legacy JSON after the SQLite authority
   exists, including when SQLite is corrupt.
4. A hidden Chrome process is used to establish or retain XRDP or Guacamole
   presentation capacity.
5. A second provider database, daemon, scheduler, controller, or desktop-input
   lease competes with the runtime host, SQLite authority, presentation
   provider, or Desktop Services.
6. Status reports ready before `minimumReady` complete provider paths are live.
7. Browser launch can precede capacity reservation and readiness proof.
8. Safety, custody, reconciliation, or ownership terminology is renamed while
   preserving an ordinary-path veto.
9. Provider credentials become authoritative in environment variables,
   editable JSON, generated units, or persistent secret files outside SQLite.
10. `doctor` performs mutation, or any repair path can execute arbitrary
    privileged commands or broadly restart shared XRDP.
11. Routine balancing moves a healthy browser between displays, lowering a
    configured limit kills existing work, or scale-in skips the final exact
    reference check.
12. A stored page, tab, URL, or database flag substitutes for a live
    authenticated viewer connection.
13. A merely queued pre-restart request launches a browser without client
    resumption.
14. A disposable handoff survives its configured expiry or quota cleanup by
    acquiring an undeclared pin or promotion state.
15. The CLI, runtime, Service model, dashboard, generated client, HTTP or MCP
    surface compiles, links, embeds, exposes, or dispatches the legacy Lease
    Authority or runtime-owner system.
16. Ordinary session admission, command routing, heartbeat refresh, expiry, or
    cleanup consults a lease, owner, custody, quarantine, recovery-plan, or
    cleanup-obligation record.
17. Two same-profile named sessions share a tab, target, heartbeat, expiry, or
    handoff identity, or a command addressed to one session can mutate the
    other's tab or liveness.
18. Closing or expiring one session closes a browser that still has another
    active session, or closing the final session leaves its browser running.
19. A stored session heartbeat substitutes for a live viewer heartbeat, or a
    viewer heartbeat becomes browser-session admission authority.

Keep the extracted Lease Authority crate independently buildable only in its
own package scope. It must not be a dependency of the trusted single-user CLI,
runtime, Service model, dashboard, generated client, HTTP, or MCP product
surfaces. Quarantined historical implementations stay outside every compiled
default-product dependency closure and have an owner, reason, and deletion or
archival decision.

## Consolidated Batch

- Freeze the executable ordinary-open, provider, persistence, handoff,
  recovery, and desktop-control dependency closures and install architecture
  guards for G01 through G45 before further behavior patches.
- Remove runtime JSON and environment authority through one forward-only
  migration, then join Browser Session Manager and presentation provider to one
  SQLite-backed runtime-host interface.
- Complete route-keeper startup, readiness, capacity, queueing, browser launch,
  durable handoff recovery, Desktop Services control, history compaction, and
  database recovery as one coherent runtime outcome.
- Qualify one source candidate provider-free, publish it once to the isolated
  development runtime, and run the complete cold-start, failure-injection, and
  external visual-operational acceptance matrix.
- Reconcile PR #191, roadmap, runbook, active lane, user documentation, and
  generated contracts only from the frozen accepted candidate.

## Scope And Non-Goals

Included:

- every G01 through G45 invariant;
- removal or compile-time quarantine of conflicting ordinary-path code;
- typed runtime, provider, persistence, configuration, handoff, capacity,
  recovery, desktop-control, history, and migration interfaces;
- provider-free fixtures, isolated development effects, and synthetic external
  desktop and mobile evidence;
- required help, README, skill, docs-site, inline comments, generated clients,
  plan, roadmap, runbook, and active-lane parity.

Excluded:

- production or staging mutation, public ingress publication, formal release,
  merge, or upstream contribution;
- new multi-tenant admission policy or adversarial user model;
- replay of page interactions beyond the last committed top-level URL;
- dashboard configuration editing;
- durable-profile promotion for disposable profiles;
- private-site content or credentials in acceptance artifacts.

## Delivery Sequence And Budget

### M0 | Closed-world conformance map and red architecture gates | 200,000 tokens

Reconcile inherited P217 usage, branch and installed identities, then map every
G01 through G45 row to current symbols, state stores, tests, and evidence. Add
failing architecture checks for every currently violated prohibition and a
machine-readable G01 through G45 coverage manifest. Freeze one dependency graph
for ordinary open through handoff recovery. Exit only when omissions and
violations are mechanically visible; do not repair behavior in this milestone.

### M1 | Authority and persistence cutover | 450,000 tokens

Remove legacy denial and JSON or environment authority from the compiled
ordinary dependency closure. Complete one SQLite-backed runtime-host interface,
one forward-only migration, typed configuration, operation journaling,
provider-credential custody, integrity and backup handling, restore-gap
reporting, history budgets, and compaction. Expose every accepted mutable
setting, not only presentation capacity fields. Preserve exact cleanup
protection outside admission. Exit when architectural guards and provider-free
authority, migration, corruption, backup, restore, configuration, quota, and
compaction fixtures pass.

Remove the Lease Authority and runtime-owner dependency from
`agent-browser-service-model`, the CLI, dashboard, generated client, HTTP, and
MCP surfaces. Preserve only migration-time diagnostic decoding behind a tool
that is not linked into the default product. Exit only when a deterministic
Cargo and source-graph guard proves the default product closure cannot reach or
serialize the quarantined crate.

Implement the G42 through G45 state machine as the only ordinary session
lifecycle. Remove parallel owner, lease, and cleanup aggregates rather than
adapting them behind the new API. Exit with provider-free Alice/Bob fixtures
that prove shared-browser reuse, separate tabs and handoffs, exact heartbeat
refresh, independent command effects, Alice-first cleanup preserving Bob, and
final-session cleanup closing the browser.

Use a compiler-driven excision protocol rather than manually reading the full
dependency surface. First freeze the allowed replacement interfaces and remove
one top-level dependency edge. Then run bounded compile waves through the WSL
Cargo wrapper with Cargo JSON messages. A deterministic repository helper must:

- preserve the complete raw compiler output as an artifact while keeping it
  out of model context by default;
- retain first-party errors and suppress dependency build chatter;
- normalize and deduplicate diagnostics by package, file, symbol, error code,
  and causal root;
- group failures into mechanical leaf removals, neutral-type extraction,
  migration-only decoding, deleted product surfaces, and primary-owned
  architectural decisions;
- emit a compact machine-readable manifest plus a short ordered worklist with
  exact files, representative diagnostics, counts, and the next compile gate;
- compare each wave with the prior manifest so resolved, new, repeated, and
  regressed groups are visible without replaying the raw log.

The primary owns Cargo manifests, the `ServiceState` cut, replacement
interfaces, migration boundaries, worker packet selection, integration, and
the final closure verdict. Run at most one compile wave after each coherent
batch of accepted leaf edits. A second causal error group in the same surface
triggers reclassification from mechanical removal to primary-owned design; it
does not start an unbounded edit and compile loop.

#### M1 Execution Efficiency Controls

The policy-ingestion pilot uses
`docs/dev/policy-capsules/p218-m1.json` and the deterministic
`scripts/dev/build-policy-capsule.mjs` generator. The generated capsule records
the eight canonical policy hashes, operative rules, checks, hard stops, and
re-read triggers. A session may reuse that capsule while its hashes and triggers
remain unchanged; canonical policy remains authoritative when a trigger fires.
`pnpm test:policy-capsule` now provides hermetic coverage for deterministic
generation, current-capsule verification, policy hash drift, missing source
files, empty required fields, and unsupported schema versions. The repo loading
contract routes matching work through the checked capsule and falls back to
canonical policy on drift, a trigger, or unresolved ambiguity.

- Treat the remaining `ServiceState` excision as one primary-owned packet with
  one outcome: remove its Lease Authority and runtime-owner state and
  operations, run one compact compiler wave, and leave one clean custody
  checkpoint. Do not fragment that packet into per-method commits or
  documentation-only continuations.
- Admit the packet only when the live cumulative allowance can cover the
  implementation, one compiler wave, reconciliation, and a 20 percent reserve
  for focused validation, documentation, and clean checkpointing. A successor
  session, worker, renamed packet, or revised prompt does not reset usage.
- Read the full required policy set once at packet start. Carry a short
  source-linked checklist in subsequent handoffs and re-read full policy only
  when scope, effect class, authority, or validation obligations change.
- Use CodeGraph for the initial structural cut and sync it immediately after a
  deletion checkpoint. Do not repeat structural discovery with grep or broad
  file reads; reserve literal search for forbidden names, configuration, and
  guard evidence.
- Between compiler waves, use the cut-specific architecture guard, formatting,
  and literal no-reference checks. Run Cargo only after one substantial
  coherent batch, not after each leaf deletion. Preserve the complete raw JSON
  outside model context and review only the compact manifest plus, when
  necessary, one exact retained diagnostic.
- Preserve compact manifests, worklists, hashes, exit status, and reproduction
  commands in the plan or another tracked evidence index. Raw compiler JSON
  may remain in an untracked artifact directory, but a temporary locator alone
  is not durable evidence and must not be the only continuation record.
- Extend `p218-compiler-diagnostics.mjs` when a known deleted-product-surface
  consequence is misclassified as a mechanical import. Do not spend primary
  reasoning repeatedly correcting a deterministic classification gap.
- Spawn workers only after the primary freezes exact disjoint diagnostic
  groups and write scopes. Give economical workers the compact manifest,
  replacement contract, exact files, focused verifier, and stop rule. Do not
  delegate another broad inventory, `ServiceState`, Cargo manifests, migration
  boundaries, or acceptance judgment.
- Complete the default-product Lease Authority closure before starting SQLite
  configuration, backup, compaction, provider, or session-lifecycle behavior
  batches. Those remain in M1 but are separate causal packets with their own
  acceptance evidence.

### M2 | Provider, capacity, and launch integration | 400,000 tokens

Remove the development route-keeper interlock and join the in-process
Guacamole keeper to readiness, minimum and warm capacity, bounded queueing,
placement, stable bindings, density, live resource-pressure admission,
over-target convergence, safe scale-in, privilege receipts, and browser launch.
Prove no Chrome launch on capacity timeout and no hidden infrastructure browser. Exit when
three isolated zero-process cold starts each produce one ready route within the
measured provisional deadline and the injected timeout launches no Chrome.

### M3 | Durable handoff, recovery, and desktop control | 350,000 tokens

Complete stable handoff resolution, last-committed-URL recovery, eager active
and lazy dormant recovery, single replacement fencing, and the shared Desktop
Services control lease. Add repeated-open coalescing, authenticated live-viewer
authority, named versus disposable expiry, quota cleanup, and restart-safe
waiting-operation behavior. Keep session heartbeat, viewer heartbeat, and
Desktop Services control as three explicit non-interchangeable concepts. Prove runtime, provider, Guacamole, route, display,
browser, tab, viewer-disconnect, quota, and restart failure cases preserve the
required identity and never replay page effects. Exit with provider-free
failure injection plus one installed joined recovery pass.

### M4 | Frozen visual-operational acceptance | 400,000 tokens

Freeze source, binary, installed generation, SQLite schema, provider manifest,
route IDs, displays, configuration, and acceptance denominator. Test every
frozen route from authenticated external desktop and mobile viewports using
synthetic content. Require current complete pixels, clean desktop, correct
browser and z-order, focus, pointer, keyboard, scroll, resize, control transfer,
reconnect, one real forward-only cold upgrade, and every M3 restart case.
Measure the provisional 90-second readiness deadline and density of four under
development pressure. Preserve first failures. Permit one consolidated repair
and one bounded repeat of affected cases.

### M5 | Qualification and integration handoff | 200,000 tokens

Run validation selected from the complete P218 diff, verify every G01 through
G45 row and prohibition against the frozen candidate, update all public and governing
documentation, and perform one closed-world review limited to the contract and
repair regressions. Publish a remote checkpoint and update draft PR #191. Do
not merge, install production, release, or remove the worktree.

## Worker Assignments

The primary agent owns the critical path, contract ledger, branch, candidate,
runtime custody, evidence adjudication, and verdict. No worker owns or blocks
the critical path.
For the lease excision, optimize for token efficiency with deterministic tools
first and admit at most two concurrent `gpt-5.6-luna` workers at low or medium
reasoning for mechanical, readily verified leaf batches. Give each worker only
the compact compiler manifest, frozen replacement contract, exact disjoint
files, required edit class, focused check, and stop condition. Suitable batches
include deleting unreachable lease endpoints, projections, client types,
dashboard components, tests, and imports or replacing a preclassified neutral
value type. Workers must return a patch, diagnostic-group IDs addressed, and
focused validation evidence; they stop on an unclassified dependency, shared
manifest or `ServiceState` edit, architectural choice, or second causal error.

The primary inspects and integrates worker diffs without repeating their full
mechanical investigation. It may also admit one later read-only
acceptance-evidence reviewer after the closure is frozen. Workers cannot edit
Cargo manifests, `ServiceState`, migration authority, the G01 through G45
ledger, or shared runtime; revise architecture or acceptance; mutate any
runtime; or declare acceptance. Do not use full-history forks or ask workers to
parse raw compiler output. Record each worker handle, effective model and
effort, assigned diagnostic groups, terminal status, accepted edits, and
reconciliation decision.

## Controls And Stop Rules

- Start execution only after current cumulative P217 usage is reconciled
  against the inherited ceiling.
- At every packet start and material checkpoint, read the live cumulative goal
  counter and reserve 20 percent of the packet allowance for validation and
  closeout. Do not start an indivisible critical-path packet when it cannot fit
  with that reserve.
- One implementation attempt and one consolidated repair are allowed per
  milestone. A filename, exception, renamed authority, or passing narrow test
  does not reset an attempt.
- A second related defect triggers batch reconciliation before another build or
  installed candidate.
- During lease excision, invoke the compiler only through the deterministic
  diagnostic helper and `scripts/ci/cargo-safe.sh`; model context receives the
  compact manifest unless the primary opens one exact retained diagnostic.
- Bound each compiler wave to one coherent cut and each Luna worker to one
  disjoint mechanical batch. Failed or ambiguous worker output returns the
  diagnostic group to the primary without automatic retry or model escalation.
- No runtime candidate is built until all known source and architecture
  violations for its milestone are repaired and cheaper checks pass.
- Use one development candidate publication for the completed implementation
  batch. Another publication requires a demonstrated source defect and enough
  remaining allowance for full acceptance.
- Do not weaken an architectural prohibition to make existing code pass.
- Do not use protocol status, process presence, stored URLs, doctor output, or
  unit tests as substitutes for visible pixels and responsive input.
- Do not treat preservation of an uncertain exact resource as permission to
  reserve capacity or refuse a separate working route.
- Stop production, staging, ingress, release, merge, destructive profile
  cleanup, and private-site effects unless separately authorized.

## Evidence And Exit

Maintain one evidence table keyed by G01 through G45. Each row records source
commit, test or artifact, installed generation when applicable, result,
failure preservation, and reviewer disposition. Evidence from different
candidates cannot be combined for final acceptance.

P218 completes only when:

1. every G-row has current evidence from one frozen candidate;
2. every architectural prohibition is enforced by a deterministic gate;
3. the ordinary-open dependency closure contains no denial-first authority;
4. three cold starts and the injected no-Chrome timeout pass;
5. every frozen route passes desktop and mobile pixels, input, reconnect, and
   recovery;
6. fresh OS process and resource readback shows no unexplained browser,
   daemon, keeper, XRDP, or Guacamole residue;
7. exact uncertain resources remain untouched and diagnostic without blocking
   service;
8. changed-surface validation and closed-world review pass;
9. local and remote source identity, plan, roadmap, runbook, lane catalog, and
   draft PR agree.

A partial pass remains OPEN with the exact failed G-row. Completion does not
authorize merge, production installation, release, ingress publication,
profile deletion, branch deletion, or worktree removal.
