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
