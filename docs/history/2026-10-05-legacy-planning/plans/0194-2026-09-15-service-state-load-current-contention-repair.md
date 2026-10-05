# Plan 0194 | Service State Load-Current Contention Repair

Date: 2026-09-15

State: CLOSED

Consolidation: required

Product lane: PL-BUGFIX

Lane: P194

Work items: `CochranResearchGroup/agent-browser#76`; issue #87 supplies the
one-revision stale-candidate acceptance case

Branch: `fix/issue-76-service-state-lock`

Target: `main`

Integration: merged through PR #146 as `f1435195bcdde4f63fb2371cfdb4008c74b34875`

Corrective successor to: [Plan 0167](0167-2026-09-11-production-scale-service-state-lock-attribution-and-critical-section-repair.md)

## Objective

Eliminate the residual production-scale `prepared_commit/load_current` Service
State timeout without extending the ordinary one-second lock deadline or
weakening revision fencing, transaction recovery, exactly-once logical effects,
holder attribution, or crash safety.

## Current State

Plan 0167 is integrated and already provides durable cross-process holder
telemetry, a 9,642,672-byte two-writer and two-reader fixture, prepared payloads
outside the exclusive lock, and one repository-owned stale replay. Its accepted
fixture slows preparation and completes below the deadline, but a later installed
runtime still timed out twice while the holder reported operation
`prepared_commit`, phase `load_current`, against 6,918,163 bytes of state.

The current prepared-commit path acquires the exclusive file lock and process
mutex, then performs a full `load_without_recovery()` to compare one revision.
That reload parses the complete state, joins split registries, overlays provider
inventory, marks sources, and refreshes derived views. The existing production-
scale fixture does not deliberately slow or isolate that phase. The source
therefore has a residual test gap even though the broader Plan 0167 regression
is green.

PR 142 is integrated into `main`; its clean challenge worktree was removed
during P194 admission. Before PR #146 merged, P194 reconciled with the later
P169 closeout so both product-lane records remained intact. No production
runtime, browser, profile, provider, or Service State mutation is authorized by
this plan.

Candidate checkpoint `9fa6c586` replaces the full reload with a persisted
revision-only probe when transaction recovery is not pending. Stores without a
cheaper probe retain the full-load default; pending recovery still uses the
complete recovery path. The lightweight JSON projection retains the same
explicit 8 MiB parser stack used by full-state persistence so deeply nested
retained values cannot consume a constrained caller stack. The red fixture
failed on unchanged production logic at 1,002 ms with an active
`prepared_commit/load_current` holder and passed on the candidate with a
9,642,672-byte fixture. All 41 `service_store` tests, format, and strict
workspace Clippy pass. PR #146 merged the reconciled source as `f1435195`;
canonical merge-commit CI run 35007032152 passed every ordinary fast gate.

## Consolidated Batch

1. Retain the existing Plan 0167 production-scale fixture as the green control.
2. Add a deterministic multi-process fixture at or above 8 MiB that makes the
   real full-state `load_current` path slow and proves the current contender
   crosses the one-second deadline for that reason.
3. Measure the holder phase and select the smallest repair supported by the red
   fixture, preferring a revision-only freshness probe if it preserves the
   transaction and recovery contract.
4. Cover issue #87 by proving one intervening revision recomputes only a pure
   repository mutation and never replays a browser, tab, route, provider, or
   tenant effect.
5. Qualify the frozen source once across every touched surface and publish it
   through a linked pull request.

## Scope

- `cli/src/native/service_store.rs` repository, store, telemetry, and focused
  provider-free fixtures;
- narrow callers only when a replay-safety compile or test failure proves they
  require correction;
- Plan 0194 and the active-lane projection;
- user-facing help, README, agent skill, and docs-site parity only if the
  observable contract changes.

P194 is the sole writer for the Service State persistence and locking algorithm.
Issue #87 does not receive a competing worktree or concurrent store edit.

## Non-Goals

- Do not increase the default lock timeout as root-fix evidence.
- Do not retry the production monitor, force-unlock state, restart or install a
  runtime, terminate a process, launch a browser, or touch a protected profile.
- Do not replace Service State with a database or introduce a new coordinator
  unless the focused evidence disproves a smaller revision-safe repair.
- Do not duplicate Plan 0167 telemetry or its already accepted slow-preparation
  fixture.
- Do not make adapter-owned retries safe or replay external effects.

## Delivery Sequence And Budget

- Diagnosis and red loop: one focused control run plus one slow-load fixture
  design, target 45 active minutes.
- Repair: one evidence-selected implementation and at most one correction pass,
  target 75 active minutes.
- Qualification and publication: selected checks, formatting, strict Clippy,
  one comprehensive provider-free Rust run when required, and one PR, target
  120 active minutes.
- Overall active-work ceiling: 240 minutes. Reassess after two checkpoints or
  30 active minutes without outcome progress. One fresh-context review pass is
  available after candidate freeze.

No subagent, browser worker, runtime operator, provider operator, benchmark
worktree, or auxiliary checkout is assigned.

## Worker Assignments

The primary session owns diagnosis, the regression, repair selection,
implementation, validation, issue updates, and integration. Deterministic tools
run tests and Git checks. Any later reviewer is read-only against the frozen
published diff and has no runtime or integration authority.

## Evidence And Exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Existing control | Exact Plan 0167 production-scale test on current `main` | The current slow-preparation control remains green and records fixture, wait, and hold values |
| Residual reproduction | At least 8 MiB, independent processes, real JSON store, explicit barriers | Baseline fails at the one-second boundary with holder operation `prepared_commit` and phase `load_current` |
| Root repair | Same red fixture on the candidate with phase timings | Zero contender timeout without changing the ordinary deadline |
| Revision safety | Stale candidate, no-op, transaction recovery, crash, and unknown-field fixtures | Monotonic revisions, no lost update, no duplicate logical effect, and valid JSON |
| Issue #87 acceptance | One intervening revision during a pure foreground-style mutation | Safe recomputation succeeds or returns typed recourse without replaying an external effect |
| Holder truth | Timeout and stale-holder tests | Exact active holder evidence survives cross-process observation and stale evidence fails closed |
| Changed surfaces | `pnpm validation:select -- --base 81de07cfb5790685ff506bdabff3609e92fd5eeb`, focused tests, format, strict Clippy, and required broader lane | Every selected source gate passes on one frozen checkpoint |
| Integration | Remote branch, linked PR, exact-head checks, and merged-main readback | Source repair enters `origin/main` through the protected workflow |
| Live boundary | Explicitly separate later authorization and installed receipt | Production install and monitor soak remain open until separately authorized |

## Execution Evidence

- Green control before repair:
  `production_scale_independent_mutations_do_not_timeout_behind_slow_preparation`.
- Red baseline on unchanged production logic: 9,644,366 bytes, 1,002 ms,
  `service_state_lock_timeout`, validated active holder operation
  `prepared_commit`, phase `load_current`.
- Candidate replay: 9,642,672 bytes, zero timeout, contender completed in
  1,027 ms end to end while each individual lock acquisition retained the
  ordinary one-second deadline.
- Focused family: 41 `service_store` tests passed, including transaction
  atomicity, crash residue, durable holder attribution, slow preparation, slow
  current load, no-op freshness, and two-adjacent-revision convergence.
- Quality gates: Rust format check and strict workspace Clippy passed.
- The large-state constrained-worker test also exercises the revision probe's
  explicit bounded parser stack.
- PR #146 merged reconciled source head `510bf266` as canonical merge commit
  `f1435195`. Both commits have Git tree
  `bae40f4c21375f066334621f59249f09e6376dac`.
- Canonical CI run 35007032152 passed Version Sync Check, Dashboard, Service
  Client, Rust Quality, Workstation Fixtures, the comprehensive provider-free
  Rust suite, and the no-launch Service smokes. Slow and live gates remained
  intentionally skipped.
- Issue #76 closed as completed through the merged pull request.
- No production runtime, browser, profile, provider, install, retry, or Service
  State effect occurred.

## Stop Condition

Stop this source batch after protected integration and exact source validation,
or earlier if the red fixture disproves the `load_current` hypothesis and no
other ranked hypothesis can be discriminated within the remaining ceiling.
Do not cross into production installation or monitor retry under this plan.

The source stop condition is satisfied. Production installation and monitor
soak remain a separate live-effect gate and are not evidence required to close
this provider-free repair plan.
