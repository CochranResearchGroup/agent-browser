# Policy | Code Testing Discipline

## Policy

- Treat tests as maintained product assets with both protective value and lifecycle cost. Test count, assertion count, and raw coverage percentage are not success metrics by themselves.
- Name the invariant or failure risk before adding a test. Place it at the cheapest layer that can prove it reliably: prefer a focused unit or contract test, use a narrow integration test for boundary behavior, and reserve end-to-end, live, soak, and exhaustive tests for risks that cheaper layers cannot establish.
- Before adding a regression test, inspect existing coverage for the invariant. Demonstrate that the new or changed test detects the defect before the fix when practical, then passes after the fix. Consolidate overlapping cases instead of accumulating historical duplicates.
- Put a regression test at a stable seam that exercises the real failure
  pattern. If no such seam exists, do not add a shallow or implementation-
  coupled proxy merely to claim coverage; record the unprotected risk and the
  architecture or testability gap, then route remediation as a separate bounded
  decision.
- Keep each test independent, deterministic, order-agnostic, and hermetic by default. Declare inputs, isolate writable state, use explicit readiness signals instead of arbitrary sleeps, and keep network, provider, browser, large-data, and live-system tests out of the default local lane unless their exact risk requires them.
- Define repo-local execution tiers and concrete wall-clock plus compute/resource budgets. At minimum distinguish focused development checks, blocking presubmit checks, periodic comprehensive regression, and opt-in live/soak/provider checks. A long comprehensive lane may remain valuable without blocking every change.
- Use affected-test selection or explicit changed-surface manifests for fast feedback only when the dependency mapping is trustworthy. Unknown impact must widen to a documented safe fallback, and a periodic comprehensive run must detect selection drift. Never describe a selected subset as the full suite.
- Measure suite economics over time: selection size, collection/startup cost, p50 and p95 wall time, total compute, peak constrained resources where material, slowest tests, flake rate, retry rate, and failure yield. Optimize repeated setup and collection costs before merely adding workers.
- Parallelize or shard only after tests are isolated and reproducible. Balance shards by observed duration when practical, retain exact shard identity in resumable receipts, and lower concurrency when contention increases failures or total resource cost.
- Treat retries as diagnostic or infrastructure-recovery evidence, not as erasure. Preserve the first failure, classify a pass-on-retry as flaky, and do not report the lane clean until policy-defined flake disposition is satisfied. Reconcile uncertain external effects before retrying any test that can mutate shared or live state.
- Quarantine a flaky test only with an owner, reason, issue or locator, quarantine date, expiry or service-level target, and replacement blocking coverage when the risk requires it. Repair, redesign, or remove quarantined tests promptly; quarantine is not permanent storage.
- Review expensive, redundant, obsolete, and low-yield tests on a recurring cadence. Every retained expensive test should protect a distinct named risk. Consolidation or deletion requires a retained-risk mapping and validation that the surviving suite still proves the intended contract.
- Use coverage to locate consequential gaps, not to chase a universal percentage. Prefer behavior, branch-risk, contract, and selectively applied mutation evidence over copy-pasted tests that only increase coverage.
- When a suite exceeds its local budget, profile before changing the gate. Prefer cheaper seams, shared-fixture optimization without weakened isolation, case consolidation, tier correction, trustworthy selection, caching on declared inputs, or duration-aware sharding. Raising a budget requires an explicit risk/economics decision and a follow-up date.
- Record exactly which tier, selection, environment, retries, shards, and exclusions ran. Validation claims must distinguish `focused`, `presubmit`, `comprehensive`, and `live_or_soak`, and must report any budget breach, flake, quarantine, or unexecuted risk.

## Repair Batch Validation

- This section applies equally to bug repairs and feature implementation.
- Run required integration gates at the completed repair batch or governed
  execution boundary, not after each intermediate custody commit. Use focused
  checks while repairing a coherent batch; record full gates as pending and do
  not describe intermediate checkpoints as merge-ready.
- Run the required gates once against the final batch before merge readiness
  or governed runtime effects. Preserve all changed-surface coverage, including
  changes in earlier commits of that batch.
- Do not repeat a passed gate without a relevant change, failure, or unresolved
  risk. A docs follow-up does not invalidate current code validation evidence.
- Prose-only changes need relevant documentation, link, and contract checks,
  not application suites. Executable configuration, schemas, and test-selection
  logic still require checks for their code impact.

## Fixture And Build Economics

- Before an expensive replay, rehearse the fixture's setup, identity checks,
  assertion and cleanup at the cheapest useful layer. Verify that the oracle
  recognizes known-good behavior and rejects the intended failure. Syntax and
  path checks should precede browser launches and production replacement.
- Compare relevant fixture conditions with the failing workflow: origin,
  permissions, process command-line format, namespace, profile, transport and
  lifecycle. A data-origin download refusal or an argv parsing mistake is not
  automatically a product bug. Change one causal variable when possible and
  state attribution limits when several variables change.
- A harness failure consumes the same milestone allowance as a product failure.
  Repair the harness locally before another full replay. Do not repeatedly
  deploy the product to discover test setup errors. Follow policy 0028's
  cumulative stop and delivery-assessment rules.
- Keep one acceptance matrix for the implementation batch: requirement,
  evidence, source/binary identity, scope and remaining gap. Reuse valid evidence
  when its covered behavior and dependencies have not changed, with an explicit
  impact rationale. Never present older-binary results as final-installed proof
  when the acceptance contract requires that exact artifact.
- Use focused checks during implementation and the required gates once at the
  completed batch boundary. Rebuild only when executable inputs changed or a
  required artifact is missing; documentation checkpoints alone do not justify
  recompilation. Reserve the full release build for production qualification.

## Adoption Notes

Each adopting repo should define a local test-suite contract with concrete values for:

- `fast_feedback_target`
- `presubmit_blocking_budget`
- `presubmit_compute_budget`
- `comprehensive_lane_cadence`
- `unknown_impact_fallback`
- `flaky_test_disposition_sla`
- `retry_result_mode`

Keep exact commands, marker names, CI job names, hardware assumptions, provider gates, and risk-specific test inventories repo-local.
