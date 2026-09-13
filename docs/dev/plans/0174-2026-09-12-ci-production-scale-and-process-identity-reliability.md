# Plan 0174 | CI Production-Scale And Process-Identity Reliability

Date: 2026-09-12

State: OPEN

Consolidation: required

Lane: P174

Roadmap: P174

Branch: `fix/plan-0174-ci-reliability`

Target: `main`

Integration: short-lived branch through the protected `main` workflow

## Objective and authority

Make the comprehensive Rust presubmit deterministic without weakening the
one-second Service State deadline or production process-identity fencing. The
operator authorized planning and execution of this CI repair. Scope includes
provider-free tests, the minimum supporting source required for deterministic
fixtures or measurements, CI validation, fork PR integration, and unblocking
the P171 active-lane cleanup. It authorizes no install, browser launch, profile,
provider, production Service State, process cleanup, or release effect.

## Current State

PR 51 changes only `docs/dev/active-lanes.yaml`. Its first Rust run failed the
production-scale independent-mutation test at 1,061 ms. One bounded rerun failed
the same test at 962 ms and also failed finalized shared-runtime-host retirement
because the captured `sleep` fixture identity no longer matched at signal time.
All other required checks passed. Current `origin/main` is `976e2031` after
Plan0173; this isolated branch starts from that exact commit.

## Consolidated batch

The batch repairs both CI failures because they share the same expensive Rust
gate and both confuse host scheduling or fixture lifecycle with a production
invariant. Preserve the original failed receipts and distinguish test-harness
repair from product-behavior change.

## Non-goals and invariants

- Do not increase the one-second Service State lock deadline.
- Do not weaken PID plus start-token and executable-family validation.
- Do not accept arbitrary sleeps, unconditional retries, quarantine, or CI
  bypass as the repair.
- Do not modify production runtime state or install a candidate.
- Retain exactly-once mutation, revision, unknown-field, and crash-safety checks.

## Delivery sequence and budget

1. Build focused red-capable loops for both named tests and capture timing or
   identity evidence. Bound diagnosis to one source-reading and instrumentation
   pass plus repeated focused provider-free runs.
2. Repair the timing assertion at the actual lock/commit seam, preserving the
   deadline and quantitative wait/hold evidence. Validate repeatedly.
3. Repair the process fixture with explicit readiness and a stable exact
   identity. Validate repeatedly.
4. Run formatting, clippy, both focused tests, affected compartments, and the
   comprehensive Rust runner once on the frozen candidate.
5. Push one reviewable PR, require green protected checks, merge, then reconcile
   PR 51 onto current `main` and require its green checks before merge.

No subagents are assigned; current orchestration policy prohibits delegation.

## Worker assignments

- Critical-path owner: primary Codex agent.
- Parallel workers: none.
- Write surfaces: this plan, roadmap/runbook/catalog, the two existing Rust test
  seams, and CI runner contracts only if evidence proves they are causal.

## Evidence and exit

| Requirement | Evidence | State |
|---|---|---|
| Red-capable focused loops | Repeated exact focused commands reproduce each symptom | pending |
| Deadline preserved | Source diff plus lock wait/hold assertions | pending |
| Identity fence preserved | Stable exact-identity fixture plus mismatch coverage | pending |
| Candidate qualified | fmt, clippy, focused, affected compartments, comprehensive Rust | pending |
| Integrated and cleanup unblocked | Green repair PR, then green and merged PR 51 | pending |

Terminal success requires both defects repaired, the comprehensive Rust gate
green without retry, the repair merged, PR 51 rebased or reconciled and merged,
and clean branch/default-ref readback. A repeated failure after the bounded
repair pass stops with exact evidence rather than weakening an invariant.

## Checkpoint P0174-C01 | 2026-09-12

State transition: `planned to diagnosis_active`.

Acceptance state: evidence loops pending.

Progress classification: `blocker_reduction`; the two terminal CI failures are
frozen and isolated from the documentation-only PR that exposed them.

Evidence: run `34711447417`, attempts 1 and 2; source baseline `976e2031`.

Material blockers: none.

Next action: reproduce and minimize both failures through focused commands.

## Checkpoint P0174-C02 | 2026-09-12

State transition: `diagnosis_active to candidate_validation`.

Acceptance state: both focused repairs pass; broader validation pending.

Progress classification: `implementation`; the Service State assertion now
uses measured exclusive commit wait rather than whole-operation wall time, and
the retirement fixture records a unique executable only after exact identity
readiness. The one-second deadline and production identity verifier are
unchanged.

Evidence: the baseline timing test passed 12 times at 660 to 711 ms wall time,
0 ms commit wait, and 335 to 372 ms exclusive hold, while CI failed only the
wall proxy at 962 and 1,061 ms. The patched timing test passed 12 times with
0 ms commit wait and at most 447 ms hold. The patched identity test passed one
guarded compile run and ten repeated executions.

Material blockers: the first guarded compile attempt admitted but sccache hit
host thread-creation pressure and emitted inherited environment values in its
fatal diagnostic. No values entered repository files. The supported cache-off,
four-job mode compiled and tested successfully; remaining Cargo validation is
queued behind the repository resource-admission threshold.

Next action: complete fmt, Clippy, affected compartments, and one comprehensive
Rust run on the frozen candidate.

## Checkpoint P0174-C03 | 2026-09-12

State transition: `candidate_validation to protected_ci_validation`.

Acceptance state: local affected surfaces accepted; local comprehensive run is
non-qualifying because host task capacity failed before the native test lane.

Progress classification: `blocker_reduction`; fmt, strict Clippy, 674 Service
State tests, and 163 workstation tests pass. The comprehensive support lane
completed its displayed test surfaces, but the native fresh-target compile
failed with `EAGAIN` while creating compiler and linker threads.

Evidence: five pre-existing Cargo scopes from September 10 and 11 retain 905
of the shared slice's 1,024 task slots. They use little memory but leave
insufficient task capacity for another fresh Rust compiler. The failed run is
preserved as a host-infrastructure receipt, not a source failure.

Material blockers: local comprehensive qualification cannot proceed without
unauthorized stale-process cleanup or changing the shared task limit. Protected
Linux CI does not use the WSL admission layer and is the next authoritative
qualification surface.

Next action: push the frozen candidate, open the fork PR, and require all
protected checks including comprehensive Rust to pass without rerun.

Remote review: PR 54 at
`https://github.com/CochranResearchGroup/agent-browser/pull/54`.
