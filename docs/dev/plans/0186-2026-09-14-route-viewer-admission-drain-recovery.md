# Plan 0186 | Route Viewer Admission Drain Recovery

Date: 2026-09-14

State: OPEN

Lane: P186

Product lane: PL-BUGFIX

Branch: `fix/plan-0186-route-profile-record-sync`

Target: `main`

Integration: merge

Work item: [issue #112](https://github.com/CochranResearchGroup/agent-browser/issues/112)

Consolidation: required

## Objective

Let exact transaction-owned post-commit reconciliation recreate missing
canonical Guacamole route viewers while the admission drain remains active,
including viewers whose terminal owner history names a generation-relative
profile path, without admitting ordinary browser effects or tenant profiles.

## Current State

P185 merged through PR #111 as `ffc6e510`; its exact integrated candidate is
SHA-256 `bce36a4c7cab42f64601c94e40cf4c7c45b2d9d814779fa4baf4f1d2dce10c97`.
The existing forward-only transaction remains at revision 17 with candidate
generation `0.28.0-a28994570dd3-9d43d7f4e826` selected and the admission drain
active.

PR #113 merged the first scoped-admission repair as `3c7d29da`. Exact merged
candidate `34318d21bf7104c982a0d73a33c24875538c9b309cd459273559a08b47fc121a`
admitted route A's initial launch, but the next header command failed before
effect as `runtime_admission_draining`. Unlike navigation, `set headers` did
not carry the global managed profile when the CLI attached its claim, so its
generated prestart launch also lacked that claim. The task-owned temporary
host was terminated after its scoped cleanup left no route display.

PR #114 merged the exact secondary-command shaping repair as `bba8b7a3`.
Candidate SHA-256
`5610a704203b70f3410b5cb1d1240c14eac75891447f856b5da15a4fc8dde3a1`
passes the strengthened source-free fixture, the 28-test admission sweep,
format, clippy, and the selected workstation checks. Runtime acceptance then
proved the admission repair: route A launched, applied its header, and
navigated. Route B stopped before effect as
`existing_session_profile_identity_unproven`. Service State has no route-B or
route-C browser, session, tab, or process-identity row, but retains a ready
generation-1 owner for each former generation-relative profile identity. The
current managed route profiles resolve to stable user-data paths, so the stale
owners now block their own replacement.

Fresh status projection after the reboot correction shows a stronger terminal
join than the earlier readback: B and C each retain a matching runtime lifecycle
record in `terminal` state with `satisfied` cleanup and exact
`exact_process_exited` plus `profile_lock_released` evidence. The stale ready
owner is history under the existing lifecycle contract, but the raw registry
session matcher still returns it as live. The launch profile-selection path uses
that raw matcher instead of the lifecycle-aware resolution contract.

PR #116 merged the terminal-history and canonical legacy-path repair as
`e9a9496a`. Exact merged candidate SHA-256
`3bb31e5ceb7bf5fa5d42c2bc2f8b0c62260e8bb46b4102297fe1babd86f78179`
passes the production-shaped source-free workstation fixture. Its first live
route-A open still stopped before effect as
`existing_session_profile_identity_unproven`. The guarded migration required
`runtimeProfile` to be present in both normalized launch representations, but
the real CLI can retain the explicit global `--runtime-profile` selection in
only one. The source-free fixture did not distinguish those normalization
shapes.

PR #117 merged the normalized-representation repair as `acf5d4c6`. Exact
merged candidate SHA-256
`95d9509f2da8e002cb5f55fd8334399de8d3e93a9b523e2cd922b6de58d9d1ba`
passes the source-free fixture, but its first live route-A open still stopped
before effect with `existing_session_profile_identity_unproven`. No candidate
daemon or browser was created. The live CLI differs from the fixture before
daemon startup: explicit `--runtime-profile rdp-guac-route-a-viewer` loads the
runtime profile's obsolete configured `userDataDir` into the generic profile
field before CLI provenance is recorded. Main preflight then misclassifies
that inherited startup default as an explicit caller path and rejects the
guarded stable-path migration.

PR #118 merged the CLI provenance repair as `851fcebf`. Exact merged candidate
SHA-256
`9becdc951f7ae2a58fedda4965a1d3abb4e023d5a4c658298dd29a54da782e24`
passes the source-free fixture and advances live route A through browser
launch. Post-launch owner registration then stops as
`runtime_lifecycle_profile_identity_mismatch`. Exact cleanup proves the
launched process exited and the stable profile lock released. Registration
looks up the new profile digest, finds no owner, then collides with the same
logical browser's terminal lifecycle record under the historical digest. The
lifecycle transition therefore needs the same exact canonical profile
migration already admitted by prelaunch selection.

PR #119 merged the lifecycle migration as `0a2c8800`. Exact merged candidate
SHA-256
`2b0fd4f270db39dd041c68c545493d2bf4ea9be124deb155c48189d2d9d89a5e`
passes the pinned source-free fixture. Live route A then created a stable-path
browser and atomically advanced its owner and lifecycle to generation 4, but
the following navigation was denied because the retained `BrowserProfile`
record still named the legacy path. Exact cleanup closed the browser, removed
its session projection, returned the stable lifecycle to terminal with
satisfied cleanup, and terminated the task-owned candidate host. The remaining
repair must update the canonical profile record in the same repository
transaction as terminal replacement and accept this already-migrated owner
with stale profile metadata on the next guarded preflight.

## Consolidated Batch

- Recognize only canonical managed `rdp-guac-route-*-viewer` profiles as the
  transaction-owned route-viewer scope.
- Attach the exact environment-derived transaction ID and revision to the
  route viewer's launch, navigation, header, and close commands.
- Propagate that already-scoped claim to the generated prestart launch.
- Materialize the managed route scope for secondary commands only when the CLI
  session and runtime profile are exactly equal.
- Exclude an owner from session binding only when its exact same-generation
  lifecycle is terminal with cleanup satisfied and another current match
  exists. Apply that filter before multi-owner ambiguity is evaluated so an
  old profile identity cannot shadow the new stable-profile owner, while a
  sole terminal owner remains available to the guarded relaunch path.
- Let that guarded relaunch move from a historical profile digest to the stable
  managed runtime-profile path only when the session, explicit runtime profile,
  and exact canonical single-letter route-viewer ID agree.
- Treat the exact profile in either normalized launch representation as the
  explicit CLI evidence; continue to reject a conflicting command-payload
  profile.
- When an explicit canonical route runtime profile has an exact retained owner
  binding, distinguish its configuration-derived `userDataDir` from a
  caller-authored `--profile`. Let continuity replace only the inherited path;
  preserve an explicit profile as a hard conflict.
- Atomically move the exact terminal route owner, lifecycle record, and
  `BrowserProfile.userDataDir` from the historical profile identity to the
  stable identity as one next-generation owner.
  Require canonical route and browser identity, complete terminal cleanup
  evidence, no destination owner, no colliding lifecycle, and no principal
  binding on either digest.
- Preserve active, retained, closing, transferring, unknown, mismatched, and
  cleanup-unsatisfied owners, along with registered-principal authority and
  every ambiguous nonterminal state.
- Keep missing claims, stale revisions, ordinary profiles, and lookalike route
  profile names denied.
- Integrate one candidate, resume the existing transaction, then install the
  exact integrated P186 generation through one fresh preserving transaction.

## Scope

- `cli/src/runtime_adoption.rs` admission scope and focused tests.
- `cli/src/main.rs` claim attachment, launch propagation, and focused tests.
- Runtime-owner session binding, guarded route-profile selection and lifecycle
  migration, and focused fail-closed tests; no new public contract is required.
- Source-free workstation fixture proof for the embedded support workflow.
- Canonical plan, roadmap, runbook, and active-lane projections.

## Non-Goals

- No general transaction bypass, admission-drain removal, tenant browser
  launch, authentication retry, profile deletion, credential change, provider
  mutation, broad cleanup, or formal release.
- No relaxation of transaction ID, revision, candidate generation, census,
  managed-profile, owner, or immutable-generation checks.
- No second writer for the P181 lease-authority extraction surfaces.

## Test Plan

First prove the unmodified admission contract fails the canonical route-viewer
claim. Green tests must cover every required action and prove that an ordinary
profile, missing claim, stale revision, and lookalike route name remain denied.
The generated launch must inherit only the already-scoped claim. The added
owner tests must prove terminal-and-satisfied history cannot make a current
replacement for the same session ambiguous, while a sole terminal owner still
reaches the guarded relaunch path. Migration coverage must prove that only an
exact canonical route viewer can replace its generation-relative profile path.
Existing negative coverage must continue to preserve missing, mismatched,
nonterminal, and cleanup-unsatisfied lifecycle states. Run the
focused admission tests, relevant workstation fixture, Rust format, workspace
clippy with warnings denied, patch hygiene, and validation selection from
`ffc6e510`.

## Source Validation Evidence

- Both focused regressions failed red on the old admission contract.
- The final admission-focused Rust sweep passed 28 tests with zero failures.
- Preliminary candidate `18af9f54f67a51d9e2ba0e6d5b379b8b22f3e11cd1d3c3f30ce15f402637bf20`
  and exact PR #113 merge candidate `34318d21bf7104c982a0d73a33c24875538c9b309cd459273559a08b47fc121a`
  passed the source-free workstation fixture. The previously built P185 binary
  failed that fixture with the exact `runtime_admission_draining` signature.
- The follow-up header-shaping regression and the 28-test admission sweep pass;
  exact session-profile equality is required before secondary commands receive
  the route scope. Optimized follow-up candidate
  `4b4ca13a6b1241d689b13f07d162ee9d6da312b66cd03c8b07fa8818acab5bdb`
  passes the strengthened source-free fixture.
- Rust format, workspace clippy with warnings denied, validation selection,
  workstation host provision, fresh VM harness, Guacamole assets, PostgreSQL
  durability, route-user synchronization, and patch hygiene all pass.
- The stale-owner regression failed red on the pre-repair matcher with
  `runtime_owner_session_ambiguous`. The repaired matcher passes all 24
  `runtime_owner_transfer::tests`, including the replacement-session case.
- The production-shaped terminal legacy route-owner regression, the canonical
  route-name boundary test, both existing exact-terminal-owner tests, and the
  existing custom-profile relaunch test pass.
- Optimized candidate SHA-256
  `3f398a578987998d2605edd30a5e7fb371fc0515576e4148d7049b6abaa238a9`
  passes the source-free workstation fixture with a terminal legacy route-B
  owner under the active transaction drain. Rust format, workspace clippy with
  warnings denied, and patch hygiene pass on source checkpoint `c68e1ea6`.
- PR #116 merged as `e9a9496a`; exact merged candidate SHA-256
  `3bb31e5ceb7bf5fa5d42c2bc2f8b0c62260e8bb46b4102297fe1babd86f78179`
  passes the source-free fixture but fails the first live route-A open before
  effect. A corrected unit regression that omits command-payload
  `runtimeProfile` fails red with `existing_session_profile_identity_unproven`
  and passes at source checkpoint `82c319e7` when the guard accepts either
  exact normalized representation. The canonical-name boundary, both existing
  exact-terminal-owner tests, custom-profile compatibility, Rust format, and
  workspace clippy with warnings denied also pass.
- PR #117 merged the normalized-shape correction as `acf5d4c6`; exact merged
  candidate SHA-256
  `95d9509f2da8e002cb5f55fd8334399de8d3e93a9b523e2cd922b6de58d9d1ba`
  passes the source-free fixture but reproduces the no-effect live denial.
  The production config confirms that selecting route A injects its legacy
  `userDataDir` into `profile`. The main-preflight regression fails red on
  `acf5d4c6` with `existing_session_profile_identity_unproven` and passes at
  source checkpoint `e073f710`; it also proves that an explicit conflicting
  `--profile` remains denied. The adjacent terminal legacy-route test and
  workspace clippy with warnings denied pass.
- PR #118 merged the preflight provenance correction as `851fcebf`; exact
  candidate SHA-256
  `9becdc951f7ae2a58fedda4965a1d3abb4e023d5a4c658298dd29a54da782e24`
  passes the pinned source-free fixture. Its first forced live route-A launch
  reaches owner registration and fails with
  `runtime_lifecycle_profile_identity_mismatch`; launched-process and
  profile-lock cleanup both pass, and the exact task-owned candidate host is
  terminated afterward. The lifecycle migration regression fails red on
  `851fcebf` with that exact error and passes at source checkpoint `bbbf5abc`.
  All 19 runtime-lifecycle tests pass, including new denial coverage for
  noncanonical routes, incomplete cleanup evidence, and registered-principal
  bindings. Rust format and workspace clippy with warnings denied pass.
- PR #119 merged the owner and lifecycle migration as `0a2c8800`; exact
  candidate SHA-256
  `2b0fd4f270db39dd041c68c545493d2bf4ea9be124deb155c48189d2d9d89a5e`
  passes the pinned source-free fixture. Live route A proved that migration
  commits, then exposed the unsynchronized `BrowserProfile.userDataDir` before
  navigation. Both new regressions fail red on the merge: one reproduces the
  current stable terminal owner plus legacy profile record preflight, and one
  proves the profile record remains legacy after a successful migration. Both
  pass at source checkpoint `fefc0dca` when canonical terminal replacement
  synchronizes the profile record atomically. All 19 runtime-lifecycle tests,
  the adjacent route-host regression, Rust format, and workspace clippy with
  warnings denied pass.

## Delivery Sequence And Budget

1. Freeze transaction revision, candidate host, drain, route-display absence,
   and the no-effect launch denial.
2. Add the red admission regressions and the narrow canonical-viewer repair.
3. Run focused and changed-surface validation on one source head.
4. Commit, publish, review, and merge one issue-linked PR.
5. Build the exact merge, use it to complete the existing forward transaction,
   then run one fresh reviewed preserving install so the accepted selected
   generation contains P186.

Stop on transaction revision drift outside the reviewed resume, protected-state
removal, noncanonical profile admission, unowned browser effects, candidate
hash mismatch, or a different reconciliation failure.

## Worker Assignments

The primary owns diagnosis, Rust admission and CLI propagation, fixtures,
integration, and installed acceptance. No subagent is assigned. P181 remains a
separate writer and must reconcile only if it changes these exact files before
P186 integrates.

## Evidence And Exit

Source exit requires red-green admission proof, launch-claim propagation,
source-free installed-workflow proof, format, clippy, selected validation, and
a merged issue-linked PR. Runtime exit requires terminal acceptance of the
existing transaction, one subsequent accepted integrated P186 generation,
matching installed SHA, one supervised production runtime host, no admission
drain, three distinct canonical route displays, coherent stream publication,
and unchanged Last30Days principal, capability, owner identity, and owner
generation.
