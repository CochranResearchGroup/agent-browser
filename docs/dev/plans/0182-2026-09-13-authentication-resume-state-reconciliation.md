# Plan 0182 | Authentication Resume State Reconciliation

Date: 2026-09-13

State: BLOCKED

Lane: P182

Product lane: PL-BUGFIX

Branch: `fix/issue-96-auth-resume-state-reconciliation`

Target: `main`

Integration: merge

Work item: [issue #96](https://github.com/CochranResearchGroup/agent-browser/issues/96)

Dependency: P180 source integration and explicit shared-runtime handback

Related lane: P181 Lease Authority extraction retains the Service State
repository implementation but may later edit adjacent product adapters

Consolidation: required

## Objective

Make a durable Authentication Run resume converge under an active Service State
writer without duplicating a browser, tab, handle, run, or external effect.
Preserve exact no-effect and effect-uncertain classifications, and provide one
safe same-run continuation after the source repair and serialized runtime gate.

## Current State

PR #98 merged exact source head
`dbe70d8d48908e0b5452595176a7dd26548d24dd` as
`caff5e085eda4ba31b32af183a9d5909e19d5465`. The source branch is an ancestor
of current `main`, its local and remote refs agree, and P191 retired the clean
integrated source and red-team worktrees. There is no active P182 source lane.

Plan 0186 subsequently installed and accepted a cumulative generation that
contains the repository contention repair, but it deliberately performed no
authentication workflow. The preserved run
`authrun-8b8d1c46947be0910b540a4e` therefore remains the outstanding acceptance
boundary. Its last recorded evidence was `ready` at transition 0 with zero
observations, zero action receipts, no pending effect, and two terminal
`no_effect` failures on the same browser, session, and tab. Issue #96 and this
plan remain OPEN until a separately authorized fresh readback supports at most
one same-run recourse. No retry, cancellation, replacement run, or duplicate
profile lane is authorized by repository cleanup.

## Consolidated Batch

The batch contains one persistence invariant and its Authentication Run proof:

- reproduce two consecutive stale prepared candidates against the real JSON
  repository with pure, provider-free mutations;
- add a bounded contended fallback that commits the same pure mutation against
  the current revision while holding exclusive cross-process authority;
- retain the low-contention optimistic path and its reduced lock interval;
- prove the fallback does not duplicate mutator results, logical effects, or
  revision increments;
- prove Authentication Run observation remains before-effect and same-run; and
- after P180 handback, inspect the exact preserved run and issue only the safe
  recourse supported by the accepted source and installed evidence.

## Scope

- `cli/src/native/service_store.rs` prepared mutation contention behavior and
  provider-free tests.
- `cli/src/native/service_authentication_run.rs` only if a red fixture proves
  an authentication-specific orchestration defect remains after repository
  convergence.
- Plan, roadmap, runbook, active-lane, issue, and validation evidence needed
  for traceability.
- One later integrated-candidate installation and same-run live recourse only
  after the P180 runtime handback and the applicable effect gate.

## Non-Goals

- No blind retry, automatic provider retry, or wider retry loop.
- No lock-timeout increase or weakening of revision comparison.
- No new Authentication Run, browser, tab, profile lane, or credential source.
- No direct edit of the preserved run or Service State artifact.
- No browser closure, process cleanup, force unlock, or supervisor restart.
- No BILL transaction, QBO, accounting, or unrelated tenant effect.
- No general Service State database replacement or architecture migration.
- No formal release.

## Delivery Sequence And Budget

1. **Red fixture and classification, 35 minutes.** Build a deterministic store
   fixture that forces two adjacent stale candidates and proves the existing
   fixed replay count returns `service_state_stale_revision` with no commit.
2. **Repository repair and focused proof, 60 minutes.** Add the smallest
   serialized contended fallback, preserve pure-mutator replay semantics, and
   prove exact revision and effect counts.
3. **Authentication boundary proof, 35 minutes.** Confirm that a Ready-run page
   observation uses the repaired repository without reserving or performing an
   external effect. Add only the narrow regression needed for this boundary.
4. **Batch qualification, 60 to 90 minutes.** Freeze the candidate and run the
   affected focused tests, formatting, workspace Clippy with warnings denied,
   validation selection, and the comprehensive provider-free Rust lane once.
5. **Integration and gated acceptance.** Publish and merge the issue-linked
   pull request. After source qualification, build and transactionally apply
   one candidate from integrated `main` containing both P180 and P182. Permit
   no Authentication Run resume until that exact candidate, runtime continuity,
   and fresh same-run state pass the installed gate.

The source batch has a three-hour active-work ceiling. End the current tactic
after two consecutive checkpoints or 30 active minutes without outcome
progress. The later installed window permits one build, one transactional
apply, and one same-run recourse chosen from fresh evidence. Any uncertain
effect stops the lane without an automatic retry.

## Worker Assignments

- **Primary and coordination owner:** this P182 session owns diagnosis, source,
  tests, plan detail, and proposed shared-authority projections.
- **P180 owner:** retains exclusive installed-runtime custody and is the only
  lane that may release the runtime dependency.
- **P181 owner:** retains Lease Authority extraction and must reconcile its
  later Service State adapter packet against P182's published repository fix.
- **Review owner:** one later bounded reviewer may inspect the frozen source
  diff and red-green proof without editing this branch or operating the runtime.
- **Books Receipts:** remains a downstream observer and must not continue the
  preserved run until P182 supplies exact same-run recourse.

No parallel implementation worker is assigned. The Service State repository
and Authentication Run call site form one tightly coupled correctness boundary.

## Test Plan

The red-green repository fixture must prove:

- two consecutive stale candidates reproduce the issue without external I/O;
- the repaired call commits exactly one logical mutation at one next revision;
- the mutator may be replayed but its returned value and durable effect appear
  only from the committed attempt;
- a false `mutate_if` predicate remains a no-op at the authoritative baseline;
- errors from the final serialized mutator do not commit;
- transaction recovery and atomic prepared-save ordering remain unchanged;
- the uncontended optimistic path still prepares outside the exclusive lock;
  and
- lock telemetry identifies the contended fallback without exposing state.

Run focused implementation checks through:

```text
scripts/ci/rust-tests.sh --focused <exact-test-filter>
```

At the frozen batch boundary run:

```text
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
pnpm validation:select -- --base 44e5dc16971cb7a09ad36842d0b517a7369eb853
scripts/ci/rust-tests.sh
```

## Evidence And Exit

Source integration readiness requires one deterministic red result on the
baseline, one green result on the candidate, exact revision and logical-effect
assertions, all changed-surface gates, a clean published checkpoint, and an
issue-linked pull request. Passing source tests does not release the shared
runtime or prove the preserved run can continue.

Live acceptance begins only after P182's integrated candidate records installed
identity, transaction terminality, multiplicity, Service State health, and
retained-browser continuity. P182 must then re-read the exact run, tab, handle,
pending-effect state, failed jobs, installed identity, and current writer
evidence before choosing recourse. Success preserves the same run and advances
it once without duplicate effect. Any changed handle, pending effect, unknown
outcome, new writer ambiguity, or failed install stops the live packet.
