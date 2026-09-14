# Plan 0183 | Registered Owner Browser-Missing Migration Repair

Date: 2026-09-13

Revised: 2026-09-14

State: OPEN

Lane: P183

Product lane: PL-BUGFIX

Branch: `fix/plan-0183-inert-tab-owner-reconciliation`

Target: `main`

Integration: merge

Work item: [issue #104](https://github.com/CochranResearchGroup/agent-browser/issues/104)

Consolidation: required

## Objective

Unblock the integrated P180 plus P182 preserving install by allowing Service
State migration to restore referential browser and session placeholders around
one invalid `browser_missing` tab while preserving the exact registered
principal, capability, owner, and owner generation.

## Current State

Candidate source `caff5e08` and binary digest
`6b808d533257491e640502c6f2e390f1f6275daf3d80edf82d92017afead0f68`
passed the no-effect workstation dry-run. Its single preserving apply stopped
before payload mutation in transaction
`upgrade-1f5e27b1-28ee-4277-a1ca-c15b4dddde32` with:

```text
service_state_tab_browser_missing:target:F825ADAB6A652C3691E3ED00E05DBC4B:session:terminal-profile-4efa5eaf85940d2924b62480
```

The tab has `valid=false`, `staleReason=browser_missing`, no principal, no work
lease, no profile, and no retained browser or session. Its generation 90 owner
is bound to active registered principal `last30days`, profile
`last30days-facebook`, and the active profile capability. The authoritative
state has no runtime lifecycle or process-identity row for that browser.

The 2026-09-14 reboot proved the old process absent, but the retained owner and
stale profile lock remained. Installed `service reconcile` then failed before
effect with `runtime_host_admission_required` because it attempted retired
legacy daemon creation. A new dry-run remained blocked by the same missing
browser reference. The repair therefore belongs only in migration referential
integrity. It must not terminalize the registered owner or require invented
lifecycle evidence.

No payload, selector, browser, profile, authentication, or tenant data changed.
The installed generation remains authoritative. A second install apply is not
allowed until this source repair passes and a fresh dry-run is ready.

The repaired candidate SHA-256 is
`e30af9fb431d67e0ff8bcbbe7d86f187c7d342489c66d9e9d2c6e8adbeef3a67`.
Its no-effect migration preview accepts the formerly blocking row, preserves
the exact tab, principal, active capability, and generation 90 runtime owner,
and reports zero protected-record removals. The same preview exposes 117
browser and 115 session placeholder additions from older retained references.
That broader existing class diff must be reviewed again from the integrated
commit before the single remaining apply gate.

## Consolidated Batch

This batch contains one migration invariant at an existing seam:

- An invalid `browser_missing` tab may receive an inert `not_started` browser
  and released system session placeholder when it carries no principal or work
  lease and no retained browser, session, or live process identity exists.
- A retained owner is admissible only when it is unique, has no pending
  transfer, and its exact profile digest and generation are backed by an active
  registered principal and active profile capability.
- Migration preserves the owner and principal binding byte-for-byte at their
  existing generation. The placeholders restore references only and create no
  claim, lease, capability, or browser effect.
- An unbound owner, revoked capability, inactive principal, binding ahead of
  the owner generation, valid tab, principal, work lease, retained browser or
  session, live process identity, or pending transfer remains blocking.

P180 activation ordering and P182 Authentication Run mutation logic are not
reopened. Their merged tests and CI evidence remain valid because this batch
touches only reconciliation and migration handling for absent runtime history.

## Scope

- `cli/src/native/service_state_migration.rs` registered-owner placeholder
  qualification and focused regression coverage.
- This plan plus the compact roadmap, runbook, and active-lane projections.
- One candidate-side workstation migration preview of the exact installed
  record, followed by one fresh workstation dry-run and at most one new
  preserving install apply after integration.

## Non-Goals

- No deletion of tabs, profiles, owner history, credentials, or authentication
  state.
- No browser launch, navigation, retry of the preserved Authentication Run, or
  BILL accounting effect.
- No Service health reconciliation change, owner terminalization, general
  owner-registry redesign, or Lease Authority crate extraction.
- No widening of GC, process cleanup, profile access, or install timeout rules.
- No full CI replay during implementation and no formal release.

## Test Plan

### Red-Green Regression

Add a migration fixture with the installed shape: one invalid
`browser_missing` tab, no browser or session, one ready owner, and one exact
active registered-principal capability binding. It must fail before the repair,
then materialize the inert browser and released session while retaining the
stale tab, owner, binding, and generation.

### Negative Matrix

Keep migration blocked for:

- a valid tab;
- a tab with a principal or work lease;
- a retained browser or session;
- pending transfer authority;
- an unbound owner or inactive principal or capability;
- a live or indeterminate retained process identity; and
- owner, profile, browser, or generation mismatch.

### Validation

During implementation run the exact regression and the focused migration
module through `scripts/ci/rust-tests.sh --focused`. At the frozen batch
boundary run Rust
format and workspace clippy once because Rust source changed, then use
`pnpm validation:select -- --base caff5e08` to confirm no additional local
surface is required. Reuse merged P180 and P182 comprehensive evidence for
unaffected paths. CI receives one lazy readback after push.

## Delivery Sequence And Budget

1. Add the registered-owner migration fixture and confirm the intended failure,
   20 minutes.
2. Implement the migration-only registered-owner guard, 30 minutes.
3. Run focused tests, format, clippy, and validation selection once, 45 to 75
   minutes.
4. Commit, push, open and merge one issue-linked PR after required checks, 30
   minutes excluding CI queue time.
5. Build one integrated candidate, preview the exact stale row with that
   candidate, run a fresh dry-run, and perform at most one preserving apply.
   Stop on any changed identity, uncertain effect, or new migration blocker.

The active source slice has a two-hour ceiling. Two consecutive checkpoints or
30 active minutes without outcome progress require a different tactic or an
incomplete handoff. The failed apply is retained as first-failure evidence and
does not authorize repeated installation attempts.

## Worker Assignments

- The primary owns diagnosis, the migration source file, validation, integration, and
  runtime acceptance.
- No subagent is assigned. The two edits share one authority invariant and the
  current instructions prohibit unrequested delegation.
- P181 remains disjoint and must reconcile if its Lease Authority extraction
  later touches the owner-binding helper used by migration.

## Evidence And Exit

The source packet exits when the red-green fixture and negative matrix pass on
one frozen candidate, formatting and clippy pass, and the commit is integrated
into `main`. Installed acceptance exits only when the failed transaction
remains terminal and auditable, the candidate migration preview accepts the
exact row without changing owner authority, a fresh dry-run is ready, one apply
is accepted, installed SHA matches the integrated candidate, the supervisor
and selected runtime host are coherent, admission drain is absent, and retained
BILL identity is unchanged.
