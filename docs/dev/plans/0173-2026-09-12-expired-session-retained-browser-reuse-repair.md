# Plan 0173: Expired Session Retained Browser Reuse Repair

Date: 2026-09-12

Plan version: 2

State: OPEN

Execution state: `test_first_implementation`

Consolidation: required

Lane: P173

Roadmap: P173

Branch: fix/plan-0173-expired-session-reuse

Target: main

Integration: merge

Authority: the operator authorized planning, implementation, testing, merge,
production installation, and preserving installed acceptance for this repair.

## Objective

Make retained-browser reuse truthful and usable when the browser is healthy but
its historical daemon-session lease has expired. An authenticated service
request must either reacquire exact same-principal work authority before any
browser effect and return a canonically valid handle, or fail pre-effect with a
typed recourse. Handle refresh must never present an inactive lease as valid.

## Current state

Production browser `session:handoff-34c90cf1d3a26934` remains ready on PID
49619, while linked session `handoff-0ad9a97d1c2766af` is expired. The access
plan nevertheless recommends `reuse_existing_browser`. A `tab_new` request
created a tab and fabricated a shared valid handle, but the next canonical
projection marked it `lease_expired`; authentication request
`mcp-service-request-service_authentication_run_start-0393b8fe-4027-403d-8741-1cb57a16e820`
therefore stopped without a credential or tenant effect. The read-only
planner/state assertion reproduced the invalid recommendation twice.

Plan 0165 repaired profile identity binding and retained browser/session links.
This successor owns the separately exposed session-authority and handle-truth
defect. Plan 0144 remains the authority model: retained history is not mutation
authority, and acquisition plus effect admission must use one current claim.

## Scope and non-goals

Included:

1. Lease-aware retained-session selection at the typed acquisition seam.
2. Exact authenticated same-principal session reacquisition before tab effects,
   using current profile claim, capability, browser, and runtime-owner evidence.
3. Pre-effect typed denial for expired, foreign, ambiguous, or changed custody.
4. Canonical post-persistence tab-handle projection.
5. Refresh behavior that preserves inactive-lease invalidity.
6. Provider-free regressions, source qualification, merge, one production
   replacement, doctor, and one preserving installed BILL reuse acceptance.

Not included: BILL transaction mutation, credential changes, profile reset,
browser replacement, broad runtime cleanup, unrelated P144 completion, or
general redesign of every lease consumer.

## Consolidated batch

The batch closes one end-to-end invariant across planning, admission, and
projection: a reusable browser route is executable only when the exact request
can establish current subordinate work authority before effect, and every
returned handle is the canonical projection of that authority. The repair must
not duplicate the retained browser or silently renew foreign or ambiguous work.

## Delivery sequence and budget

1. Add one public access-plan regression for the exact healthy-browser plus
   expired-session state and prove RED on the baseline.
2. Implement the smallest typed acquisition classification and pre-effect
   reacquisition needed to make that case GREEN.
3. Add one tab-new contract regression proving the returned handle equals the
   immediate canonical projection, then implement only the required projection
   change.
4. Add one refresh regression proving an inactive handle remains invalid or is
   rejected absent a real authority transition, then implement the repair.
5. Run focused checks, formatting, strict Clippy, and the changed-surface test
   selection once on the frozen candidate.
6. Commit, push, merge through the normal origin workflow, build one optimized
   candidate, install it through the transactional installer, and verify exact
   installed identity, doctor, process census, retained profile/PID continuity,
   access plan, one disposable tab, canonical handle validity, URL/title read,
   and cleanup.

Overall effort ceiling: 120 active minutes. Work-unit attempts: three. Review
and rework cycles: one. Expensive builds and production replacements: one each
unless a demonstrated source defect invalidates the frozen candidate. The
critical path is serialized because all source changes share the acquisition
and lease-authority seam.

## Worker assignments

The primary agent owns planning, implementation, validation, review,
integration, installation, and acceptance. No subagent is assigned because the
operator did not request delegation and the source surfaces overlap tightly.

## Evidence and exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Planner truth | Focused public access-plan regression | Expired history alone is never returned as immediately executable authority |
| Preserving liveness | Focused acquisition/admission regression | Exact authenticated same-principal custody is reacquired before tab effect without launching a browser |
| Fail closed | Foreign, ambiguous, and changed-revision cases | Each stops before effect with typed recourse |
| Handle truth | Tab-new contract regression | Returned handle equals immediate canonical Service State projection and is usable |
| Refresh truth | Refresh contract regression | Expired or released authority cannot produce `valid: true` |
| Source qualification | Formatting, strict Clippy, and selected Rust gates | Final source commit passes every changed-surface gate |
| Integration | Remote PR/merge and ancestry readback | Source commit is reachable from current `origin/main` |
| Installed identity | Transaction receipt, SHA-256, generation, doctor | Exact merged candidate is active with stable runtime census |
| BILL acceptance | Access plan, tab creation, canonical read, cleanup receipts | Retained PID/profile survive; disposable handle is valid and released; no BILL data changes |

The plan closes only when every row is proven by current evidence.

## Checkpoint P0173-C01 | 2026-09-12

State transition: `test_first_implementation -> repair_active`.

Progress classification: `blocker_reduction`; PR 52 passed formatting, strict
Clippy, dashboard, service-client, version, and workstation fixture gates. The
comprehensive Rust run exposed two lease regressions in the first repair: the
candidate selector counted an older same-principal session that did not match
the current runtime owner, and the stale-session fixture incorrectly expected
the coherent canonical profile claim to become observation-only. Restrict the
rejoin candidate to the current owner's exact daemon route or browser and let
rejoin reactivate that stale subordinate session. Preserve the unrelated
production-scale timing assertion at 963 ms as a load-sensitive failure for
fresh-run classification rather than changing its contract without evidence.

## Checkpoint P0173-C02 | 2026-09-12

State transition: `repair_active -> repair_active`.

Progress classification: `blocker_reduction`; PR 52 run `34715407978` proved
the stale-session reactivation and owner-generation regressions green. Two
existing ambiguity tests showed the narrowed selector must still count every
active profile session as contention. The second correction includes all
active profile sessions while adding only the inactive same-principal session
that exactly matches the current runtime owner's route or browser. The
unrelated production-scale timing assertion repeated at 961 ms and remains
unchanged for the source-required rerun.
