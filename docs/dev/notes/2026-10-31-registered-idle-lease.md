# Registered idle lease diagnostic repair

Product lane: PL-BUGFIX. Work item: #515. Baseline: `54b2ed116b93b02f8ccdf0e0db7dd2d4d0154a15`.

A current capability may be registered before the first browser launch. The unbound-capability projection unconditionally diagnosed a missing runtime-owner binding even when no owner or subordinate work existed. Requiring reconciliation in that state offered no executable repair.

The projection now reports observation-only `registered_idle` with `acquire_profile` recourse when authority and profile identity are valid and no retained owner, browser, session, tab or cleanup obligation exists. Reads create no claim, binding or browser. Existing authenticated acquisition remains responsible for current ownership checks. Retained work, unresolved identity and invalid authority keep their diagnostics. Model, schema, generated client and user guidance share the additive recourse. Detailed workflow guidance moved into linked references to satisfy the existing 10 KB skill budget; the first parity failure is retained in local validation evidence.

The real projection/doctor regression failed before the repair with `identity_reconciliation_required` instead of `registered_idle`. It passes after the repair, including exact Service State equality. Counterexamples retain owner, session or browser evidence, remove profile identity, or invalidate authority. All 53 focused lease tests and the complete service-model suite pass. Strict Rust formatting/clippy, generated client contracts/types and aggregate client checks, dashboard lease actions, API/MCP parity, policy wiring, documentation links, handoff guidance and docs build pass. Hosted CI is disabled by operator policy; no target-platform CI claim is made.

Source qualification and installed proof are separate. Private runtime observations and the original live repro stay outside Git. Production acceptance must rerun that original repro without identity rebinding, capability rotation, profile reset or private-account access, then qualify the selected generation with a disposable Native handoff/continue workflow.
