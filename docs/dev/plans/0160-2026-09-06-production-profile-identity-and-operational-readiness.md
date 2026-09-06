# Plan 0160: Production Profile Identity and Operational Readiness

Date: 2026-09-06

State: PLANNED

Execution state: `ready_for_execution_not_started`

Lane: P157

Branch: plan/profile-permissions-and-request-provenance

Target: main

Integration: merge

Source baseline: `51da03ee`

Dependencies: [P159]

Overlaps: [P157, P158]

## Objective and authority

Make the production candidate operational across all four outstanding readiness
items: reliable browser control, the authenticated operator journey, maintenance
and ownership reconciliation, and unattended operation. Profile ownership and
identity proof blockers are the highest priority. Causal error tracing is part
of repairing those blockers, starting with the first reproduction.

The operator requested this plan. This checkpoint writes and reviews the plan;
it does not start implementation or mutate production. Existing authorization
for candidate installation and the reviewed 77 policy writes remains recorded
in the [delivery proposal](../notes/0154-2026-09-06-plan-0159-production-delivery-proposal.md).
During execution, apply that authority to its actual scope without requesting
it again. Do not infer authority for unrelated client eviction, broader grants,
private content capture, new external systems, or formal release.

## Current state and reconciliation

[Plan 0159](0159-2026-09-05-client-recovery-logging-and-remote-view-remediation.md)
closed a bounded repair/delivery contract; it did not establish whole production
readiness. Its [acceptance audit](../notes/0155-2026-09-06-plan-0159-acceptance-audit.md)
and delivery proposal preserve that distinction. This plan cannot close merely
because source tests, an older candidate, or a subset of user journeys pass.

Last recorded production generation is
`0.28.0-e4243235af0c-072303ae3e67`, containing profile repair `8bd7f58a`.
The exact release passed isolated original-two-client retained-handle recovery
and installed custom-profile headless control, interaction, close and reopen.
The ambient remote-headed reopen failed when Xvfb could not start display `:91`.
Headless success does not clear that failure. Refresh live identity at execution
start; these are source-backed historical checkpoints, not a fresh live census.

The last doctor report includes 11 owner/generation/binding mismatches, five
missing principal bindings, one unproven session authority, unknown resource
ownership, monitor readiness, operator journey and selected-generation acceptance
findings. A stopped synthetic supervisor finding may already have been removed;
verify instead of carrying it forward blindly. The reconciliation timer was
enabled but inactive. Neither historical lease rows nor failed upgrade receipts
may be erased to manufacture readiness.

## Frozen acceptance contract

| Criterion | Required outcome on the final installed candidate |
| --- | --- |
| A1: reliable profile control, highest priority | Every current profile ownership/identity blocker has an evidence-backed disposition. Authorized named-profile, custom-directory and session-only control passes launch, navigation, interaction, retained reuse, close and reopen in both supported headless and remote-headed production paths. Two independently authenticated clients retain their original authorized handles through an exact disposable host interruption; legitimate foreign, ambiguous or changed identities are denied before effects with useful recourse. No unintended browser, tab, profile or grant replacement masks a pass. |
| A2: authenticated operator journey | The ordinary authenticated dashboard selects the intended synthetic browser. Its durable remote-view URL reaches `operatorVisible.state=ready`, shows verified synthetic pixels, accepts mouse and keyboard input, and reconnects using the same URL with two concurrent authorized viewers. Browser, profile, target and route ownership stay attributable across transitions; unauthorized viewers are denied. |
| A3: maintenance and readiness | Every refreshed doctor finding and ownership-related pressure finding is repaired or explicitly classified with current evidence, consequence and disposition. No unresolved actionable production blocker remains. Supported doctor returns zero, and selected generation, payload, ingress, convergence, operator journey, monitor and rollback readiness reflect real observed state. Historical failures remain inspectable without falsely blocking current readiness. |
| A4: unattended operation | After A1–A3 pass, restore the intended timer and observe three consecutive normal scheduled cycles plus one controlled runtime restart and its next scheduled cycle. All complete without new ownership/identity errors, route loss, duplicated owners, unsafe cleanup or accumulating owned residue. Original authorized handles and durable URL remain usable after restart. |
| AX: diagnosis across all four | Each selected failure is traceable from its returned correlation identifier to the first causal decision, relevant ownership evidence, source/build identity, affected operation and safe repair action. Expected projections have zero unexplained missing, duplicate or conflicting records. An operator can reconstruct the cause using the documented diagnosis surface without searching unrelated logs or accessing secrets. |

A genuine foreign resource may remain present when it is positively classified,
excluded from service cleanup and does not block an authorized user workflow.
Unexplained identity gaps, blanket historical exemptions and false-positive
errors in ordinary authorized use do not satisfy A1 or A3.

## Work units and dependencies

The primary agent owns all units and integration. Active-agent concurrency is
one; no delegated discovery or parallel runtime mutation is planned.

| Unit | Work and expected write surface | Exit evidence | Dependency |
| --- | --- | --- | --- |
| W0 | Refresh exact installed identity, doctor, leases, principal bindings, processes, namespaces, ingress, provider and timer state. Create a private blocker ledger and deployment/evidence manifest. | Every finding has a stable key, affected workflow, provenance and A1–A4 mapping; final-candidate mismatch is explicit. | Planning complete |
| W1 | Repair profile ownership and identity proof at the actual selection, attachment, relaunch or reconciliation seam. Add causal logging at the failing seam immediately. Likely surfaces: native action runtime, Service ownership/lease model, request diagnostics and focused regressions. | A1 and its AX cases pass; custom profile repair stays covered; evidence binds principal, process and target before effects. | W0 |
| W2 | Fix remote-headed launch/Xvfb ownership and ordinary durable remote-view failures. Update provider/launch and handoff surfaces only where the reproducer establishes the cause. | A2 passes on the installed candidate, including remote-headed close/reopen; synthetic external evidence and exact cleanup accounted for. | W1; Xvfb ownership diagnosis begins in W0/W1 if it blocks A1 |
| W3 | Reconcile remaining maintenance records, monitor readiness and selected-generation/upgrade acceptance through supported product paths. Fix false-positive classification and broken transition software instead of hand-editing green receipts. | A3 passes; every ledger item is resolved or justified; retained failure history remains readable. | W1; A2 for operator acceptance |
| W4 | Restore timer, observe scheduled cycles, perform controlled restart and verify client/operator continuity. | A4 and associated AX cases pass with before/after census and bounded resource comparison. | A1–A3 |
| W5 | Join all evidence against final binary and support identities, perform closed-world acceptance audit, update plan and delivery status. | A1–A4 and AX all pass on one accepted final candidate; no unowned cleanup obligations hidden. | W1–W4 |

W1 takes precedence over unrelated maintenance or presentation polish. Ownership
findings from W3 that affect profile usability are pulled into W1 immediately.
Read-only evidence collection for later units may proceed without delaying W1.

## W1 ownership proof and failure matrix

For each blocker, distinguish requested identity, inherited/default selection,
stored authority and live observation. Record canonical profile identity,
logical browser/session and target, authenticated principal and grant reference,
lease/fence, owner generation, executable identity, PID plus process start/boot
identity, CDP endpoint and relevant namespace. A PID or directory match alone
is insufficient. Verify real authorization; do not weaken checks to reduce
error counts or create replacement profiles as recovery.

Cover positive named and custom profiles, session-only reuse, two authorized
clients, clean terminal close/reopen and retained host recovery. Negative cases
must cover conflicting explicit profile, foreign principal, missing binding,
ambiguous owner, changed generation, PID reuse and stale endpoint/target.
Use focused deterministic tests for identity counterexamples and disposable
installed fixtures for process/transport claims. Each negative case must prove
no browser effects and identify the safe next action.

For the Xvfb failure, inspect the runtime host's actual mount/process namespace,
display allocation record, child exit status and bounded stderr before deciding
whether a lock is stale. Caller-visible `/tmp` is insufficient under PrivateTmp.
Reclaim only positively owned released resources; never unlink an unknown lock
or terminate an unrelated browser/display to make a probe pass.

## Causal logging and backtrace contract

A failure envelope and its durable diagnostic record must provide:

- Stable error code, first cause, causal parent/span identifiers, operation,
  phase/axis, UTC time and ordered attempt sequence; wrappers and retries retain
  the original cause and distinguish their own failure from it.
- Request, trace, job and event correlation; session/profile/logical browser,
  owner generation, lease/fence and principal-binding references when applicable.
  Include requested versus inherited profile-selection provenance and the exact
  proof predicate that failed, with expected and observed safe summaries.
- Exact binary/source and support generation; process start/boot identity and
  endpoint/target references where relevant. Mark unavailable evidence explicitly
  rather than inventing values or omitting the reason it is unavailable.
- Effect certainty, retry safety, cleanup obligations and concrete recourse.
  Link the decision to its source component and relevant regression/reproducer.
- For unexpected internal failures, bounded symbolizable stack backtraces or
  source locations plus the error cause chain. Expected authorization denials
  need the decision trace; a noisy stack alone is not an explanation.

Expose a documented lookup from the returned correlation ID through existing
CLI/doctor/diagnostic surfaces where practical. Add a product lookup only if
existing surfaces cannot reconstruct the chain. Keep logs structured, bounded,
redacted and durably correlated across response, job, event, trace and journal;
record which projections are applicable and why. Record rotation/retention and
lookup behavior so a restart does not sever the diagnostic trail. Logging
failure must not turn an error into success or lose the first cause silently.

Selected AX fixtures: profile-selection conflict, missing principal binding,
owner-generation mismatch, stale transport/target, Xvfb startup failure,
journal append interruption, and failed maintenance/restart transition. Preserve
red evidence, then show the repaired case or safe typed denial. For each, produce
one end-to-end causal reconstruction from the returned ID, including restart
lookup for the journal case, with explicit expected/observed/missing/duplicate/
conflicting counts. Historical P159 logging proofs may be reused as scoped
regression evidence but do not replace new-seam or final-installed proof.

Never log credentials, raw capabilities, cookies, authenticated page content or
unredacted private payloads. Keep access-controlled raw diagnostics and identity
manifests outside the product repository; tracked summaries contain only safe
conclusions, hashes and source references.

## Installation, operational checks and evidence

Validate changes first in the isolated development runtime. Use the optimized
candidate build and focused smoke there; use the full release build for the
production candidate. Preserve unrelated production browsers and the retained
P158 development fixture. The known old upgrade prerequisite failure must be
reproduced and corrected through a supported path before that transition is
called operational. A successful controlled publication does not itself prove
upgrade recovery or clear selected-generation acceptance.

Before publication, record exact binary/support hashes, compatibility, owned
resources, rollback target and preserved clients. After publication, read back
all three production unit executables, current selector, ingress, provider
assets and doctor. Do not fabricate acceptance receipts. If the candidate
changes during validation, invalidate affected checks and repeat the dependent
acceptance slices against the new identity before the final join.

Production user-journey fixtures use synthetic content and dedicated identities.
External visual evidence uses the protected manually dispatched
`p158-external-vantage.yml` lane with its required synthetic-only bindings,
credentials and exact retained identity. No automatic trigger or unchanged
retry. Historical run 34022233372 is baseline evidence, not proof of this
production candidate. Keep preparation and credentials out of tracked artifacts.

W4 records the timer's intended schedule before enabling it, then observes three
actual scheduled cycles at that cadence, not three manually forced invocations.
Bound observation to five scheduled opportunities and an explicit deadline
calculated from the refreshed schedule. Restart only the exact owned runtime
unit after recording rollback and protected client identities; then observe the
next scheduled cycle and recheck original handles, remote-view and doctor.
A new ownership error ends unattended acceptance and returns its causal evidence
to W1 or W3. Preserve timer state and the reason for any safety stop.

## Validation, bounds and completion

For changed Rust surfaces, run focused regressions plus repository-safe format
and workspace clippy checks. Run Service/client contract parity checks when
models, schemas or diagnostics change; update CLI help, README, repository skill,
docs site and inline contracts for changed user-facing behavior. Use
`pnpm validation:select` against the slice baseline. Documentation-only planning
requires link, whitespace and acceptance/dependency review; it does not require
browser launches or Rust builds.

max_work_unit_attempts: 3
max_review_rework_cycles: 1
max_hardening_checkpoints: 2
checkpoint_interval: 30 minutes
max_review_discovery_passes: 1
review_verification_mode: closed_world_if_reviewed
authorization_gate: material_departure_or_explicit_action_gate_only
continuation_default: execute_obvious_in_scope_low_risk
bound_exhaustion_mode: local_replan_before_escalation
checkpoint_mode: material_boundary_with_cadence_backstop

The primary controls each reproduce/repair/retest loop. Three materially distinct
attempts per unit is the hard local bound; do not repeat unchanged failed live
operations. The external lane retains its stricter no-retry rule. At a bound,
record the causal result, split or reframe locally, and continue unaffected safe
work. A bound does not authorize broader mutation or erase a blocked criterion.
One final acceptance review checks the frozen criteria; one rework cycle may
address accepted blocking findings. Other concerns go to an explicit backlog.

Checkpoints record `state_transition`, `acceptance_state`,
`progress_classification`, `evidence`, `material_blockers`, and
`next_action_or_stop_reason`. Unit states are ready, active, awaiting-review,
awaiting-gate, blocked, complete, failed or cancelled. Classify progress as
outcome_progress, blocker_reduction, hardening, no_progress or regression.

Close only when all five acceptance rows pass against the final installed
candidate and private evidence manifests account for owned-process cleanup and
retained obligations. Publish a concise criterion-by-criterion verdict with
current doctor and scheduled-cycle results. If anything remains blocked, retain
OPEN execution status and name it; do not call production fully operational.

## Planning checkpoint

All execution units are unstarted. This plan covers the four requested readiness
items and makes profile ownership proof and causal diagnosis the critical path.
The next execution action is W0's current census followed immediately by W1's
highest-impact reproducible ownership blocker.
