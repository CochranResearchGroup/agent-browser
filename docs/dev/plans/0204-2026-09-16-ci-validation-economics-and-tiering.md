# Plan 0204 | CI Validation Economics And Tiering

Date: 2026-09-16

Plan version: 4

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P204

Work item: `CochranResearchGroup/agent-browser#174`

Branch: `platform/p204-ci-validation-tiering`

Target: `main`

Integration: merge through the protected pull-request workflow after selector fixtures, workflow validation, and changed-surface validation

Source baseline: `2632e31cfdc1858336af5acbf6e07143ee7787e7`

## Objective

Make ordinary pull-request validation deterministic and proportional to changed
surfaces while preserving one stable aggregate presubmit check and a fail-safe
fallback for unknown impact. Reserve comprehensive Rust and platform validation
for explicit qualification, material dependency or toolchain changes, and a
low-frequency selection-drift audit. Preserve the current `main` fallback until
issue #164 proves the aggregate check is enforced by live branch rules.

## Current State

Issue #174 is claimed and P204 is admitted from `origin/main@2632e31c`.
Implementation checkpoint `7f6c7e2e` adds the pure versioned classifier,
exact-head workflow routing, pull-request cancellation, fixed job outputs,
hermetic fixtures, fail-closed `Presubmit` aggregate, focused Rust compartment
selection, changed-document link validation, and bounded economics receipts.
Classifier and workflow self-changes fail safe to every ordinary presubmit job
without duplicating the comprehensive Rust lane.

The first organic broad run exposed one remaining proportionality leak: every
focused Rust change inherited the command-based no-launch service smoke bundle.
Version 3 adds a deterministic `serviceSmokes` selection bit. Service-owned
Rust surfaces and fail-safe broad selection retain the bundle, while unrelated
focused Rust compartments no longer pay for the extra CLI build and smokes.

Organic run `35171646162` then proved cancellation and every non-Rust selected
job, but failed closed after multiple CLI compartments ran concurrently against
the same Cargo target. One invocation replaced the running `agent_browser` test
executable, yielding nine deterministic `No such file or directory` failures.
Version 4 serializes CLI compartments in one lane while retaining overlap with
one independent-crate lane.

The pre-implementation docs-only probe against merge `aa7b67b1` remains the red
baseline. Local validation is green for the selector and aggregate suites,
workflow syntax and semantics, policy wiring, planning audit, changed links,
the docs production build, and the newly exposed Challenge Control compartment.
Independent review found and the candidate corrected a fail-open shell pipeline,
CLI adapter test omissions, omitted member manifests and installer fixtures,
and missing in-CI execution of the classifier contract suite. Issue #164 remains
open, and live readback reports no `main` branch protection or repository
ruleset, so removing the post-merge fallback is not currently allowed.

Graphiti was healthy but returned no issue-specific prior decision. Current
repository, issue, workflow, and run evidence therefore govern this plan.

## Consolidated Batch

1. Extract a pure, versioned changed-surface classifier and keep the existing
   CLI as its Git adapter for local recommendations and exact CI ranges.
2. Add hermetic fixtures for docs/governance, dashboard, service client,
   workstation/release, focused Rust, classifier self-change, dependency or
   toolchain qualification, and unknown-impact fallback.
3. Add pull-request concurrency cancellation, a classifier job with fixed
   outputs, conditionally selected jobs, and one stable aggregate `Presubmit`
   check that fails closed on malformed or missing selected-job results.
4. Replace ordinary comprehensive Rust execution with mapped compartments and
   run the no-launch service smoke bundle only for service-owned Rust surfaces;
   retain comprehensive execution only for the explicit qualification routes.
5. Record selection tier, included and excluded lanes, elapsed time, and
   runner-time inputs without describing focused validation as comprehensive.
6. Update the repository CI contract and preserve the `main` fallback until
   issue #164 supplies live enforcement evidence.

## Scope And Effect Boundary

Expected writes are limited to `.github/workflows/ci.yml`, the validation
selector and its pure library, focused CI verification scripts and fixtures,
`scripts/ci/rust-tests.sh` only if a missing compartment is required,
`package.json`, `AGENTS.md`, this plan, and P204's active-lane entry. Existing
tests that merely regex-match selector source may be adjusted only to move the
same invariant into behavioral selector fixtures.

This plan does not authorize branch-protection or ruleset mutation, workflow
dispatch or retry, browser or provider access, credential use, installed-runtime
or Service State mutation, production or staging mutation, release, or deletion
of tests to improve timing. Organic GitHub runs and the deliberately authorized
comprehensive dispatch remain external acceptance evidence, not effects granted
by this plan.

## Delivery Sequence And Budget

- Critical path: red selector fixture to pure classifier to conditional workflow
  and aggregate verifier to focused validation to protected integration and
  organic-run evidence.
- Attempt 1: versioned classifier, six required surface fixtures, and safe
  unknown fallback.
- Attempt 2: workflow wiring, aggregate evaluator, Rust compartment selection,
  and measurement summary.
- Attempt 3: one bounded correction after static or organic workflow evidence.
- Maximum work-unit attempts: 3.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 240 active minutes through implementation, protected
  integration, and the first available organic evidence. External waiting for
  issue #164 does not authorize repeated synthetic runs.
- First outcome artifact: the already captured deterministic docs-only red
  probe, replaced by `pnpm test:validation-selection` before implementation.

## Worker Assignments

The P204 lane owner retains the critical path, causality decisions, writes,
integration, and final acceptance. Read-only worker `/root/ci_architecture`
used requested `gpt-5.6-sol` at medium effort to inspect workflow and selector
architecture and returned the pure-classifier plus stable-aggregate design.
Read-only worker `/root/admission_plan` used requested `gpt-5.6-luna` at medium
effort to reconcile plan numbering and worktree admission. Read-only worker
`/root/issue_dependency` used requested `gpt-5.6-luna` at medium effort to
inspect issue #164, branch rules, and recent CI runs. The runtime did not expose
effective model or effort for independent verification.

Read-only reviewer `/root/candidate_review` was requested on `gpt-6-astra` at
high effort for the frozen candidate. It identified the aggregate pipeline
masking defect and four selection-coverage gaps. One bounded rework corrected
all blocking findings and added regression assertions. The runtime again did
not expose effective model or effort for independent verification.

P197 remains primary writer for challenge consumers and generated
service-request contracts. P204 owns CI workflow, selector, aggregate verifier,
and its fixtures. The shared planning overlap is explicit: P204 writes only its
own bounded `ROADMAP.md`, `RUNBOOK.md`, and active-lane sections and preserves
P197's records. P202 closeout is integrated; its retained clean checkout is
attributable to merged PR #177 and has no P204 write overlap.

## Evidence And Exit

Exit requires current evidence that:

- classifier fixtures cover docs-only, dashboard-only, service-client-only,
  workstation-only, focused Rust, and unknown-impact fallback cases;
- classifier and workflow self-changes fail safe to broad presubmit validation;
- superseded pull-request runs cancel and the stable aggregate check reports the
  selected tier without accepting a skipped selected job;
- documentation and governance-only changes run hygiene, policy, link, and
  documentation checks without application builds unless executable
  configuration changed;
- workstation, dashboard, service-client, Rust quality, Rust compartments, and
  comprehensive qualification run only for their mapped surfaces;
- selection, exclusions, tier, elapsed time, and runner-time inputs are recorded;
- one organic docs-only pull request and one organic narrow Rust pull request
  demonstrate the selected behavior;
- one explicitly authorized comprehensive dispatch proves the retained complete
  suite; and
- removal of equivalent `main` validation occurs only after issue #164 provides
  live branch-rule readback enforcing `Presubmit`.

| Requirement | Current evidence | State |
| --- | --- | --- |
| Deterministic classifier contract | `pnpm run test:validation-selection` passes the versioned selector and exact-range CLI contracts | green locally |
| Surface fixture matrix | Docs, dashboard, client, workstation, Rust adapters and crates, dependency, self-change, rename, and unknown fixtures pass | green locally |
| PR cancellation and stable aggregate | Run `35170641311` cancelled when replacement `35171646162` started; its Rust failure propagated through `Presubmit` | green organically |
| Proportional job routing | Exact-range readback selects broad ordinary validation for this classifier/workflow change and excludes comprehensive Rust | green locally |
| Service smoke routing | Selector fixtures distinguish unrelated Rust from service-owned Rust; workflow contract gates the smoke bundle on `serviceSmokes` | green locally |
| Rust lane isolation | Organic run `35171646162` exposed same-target CLI executable replacement; version 4 serializes CLI compartments while overlapping only the independent crate lane | pending validation |
| Comprehensive qualification retained | Manual, monthly, material dependency, toolchain, and temporary `main` routes select comprehensive without duplicate focused Rust | green statically; authorized dispatch pending |
| Economics receipt | Fixture covers selected lanes, exclusions, bounded wall time, observed runner time, and explicit measurement limits | green locally; organic receipt pending |
| Organic docs and narrow-Rust evidence | Existing runs prove the broad baseline only | pending candidate workflow |
| Protected aggregate before removing `main` fallback | Live branch protection returns 404 and rulesets are empty | blocked on issue #164 |

## Stop Condition

Stop before branch-protection or ruleset mutation, workflow dispatch or retry,
test deletion for timing, automatic retry, installed-runtime or browser effect,
provider or credential access, production or staging mutation, or release.
Retain the `main` fallback and keep the plan open if implementation is ready but
issue #164, organic-run evidence, or explicit comprehensive-dispatch authority
is still missing.
