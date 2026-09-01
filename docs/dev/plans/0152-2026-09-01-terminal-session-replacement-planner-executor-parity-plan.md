# Plan 0152 | Terminal Session Replacement Planner Executor Parity

Date: 2026-09-01

State: OPEN

Execution state: `slice_a_red_fixture`

Lane: P152

Source baseline: `09e1d6f69eecd0a5e2590a44f2ecba903e36214d`

Branch: `fix/terminal-session-replacement-parity`

Target: `main`

Integration model: one cohesive validated checkpoint on a short-lived topic
branch, followed by a merge to `main` and exact candidate installation.

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED
DEVELOPMENT-RUNTIME VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION, AND
ONE BOUNDED LAST30DAYS ACCEPTANCE TICK ARE IN SCOPE. Credential entry,
authentication mutation, profile replacement, broad process cleanup, provider
reconfiguration, and formal release are out of scope.

Depends on:

- Plan 0137 terminal owner replacement planning;
- Plan 0144 authenticated cold acquisition and protected lease authority; and
- Plan 0148 installed runtime-host supervisor takeover.

## Incident

The `last30days-facebook` profile has no live holder, browser, process, or
profile lock. Access planning therefore recommends `launch_new_browser`, but
it copies the terminal owner's old daemon session route into the proposed
request. The executor then classifies that route as an existing session and
requires stronger identity reconciliation. A retained principal binding at an
older owner generation prevents the terminal relaunch exception, so execution
fails before Chrome startup with
`existing_session_profile_identity_unproven`.

The same Last30days tick exposes a second deterministic defect: saved provider
configuration permits three attempts while database schema 16 admits only
retry ordinals zero and one. Attempt ordinal two raises an integrity error and
prevents the next provider from starting.

## Goal

Make one authenticated, terminal, physically absent profile owner produce an
executor-admissible replacement request without weakening live-owner,
profile-mismatch, principal-mismatch, process, or profile-lock fences. Align
Last30days persistence with its public three-attempt contract so a bounded
retry cannot fail at database insertion.

## Execution Graph

| Slice | Depends on | Work | Exit condition |
| --- | --- | --- | --- |
| A | none | Add a public access-plan-to-executor regression for the observed terminal owner and older principal binding | The fixture reproduces the identity failure before implementation |
| B | A | Repair terminal replacement session and principal reconciliation with negative fences | Focused planner, executor, lifecycle, and profile tests pass |
| C | B | Add the Last30days schema migration regression and widen retry ordinal persistence to the configured contract | Migration and three-attempt tick tests pass |
| D | C | Align plan, roadmap, runbook, and affected operator contracts | Source and documentation describe one replacement contract |
| E | D | Run selected and comprehensive source validation, then qualify an isolated Agent Browser development candidate | Rust, client, Python, migration, doctor, and disposable launch checks pass |
| F | E | Merge, install exact Agent Browser and Last30days candidates, and reconcile installed identity | Source, installed artifacts, protected authority, services, and runtime generations agree |
| G | F | Execute one bounded X and LinkedIn acceptance tick | No identity or retry-ordinal failure occurs; terminal receipts and zero active residue are recorded |

## Frozen Safety Contract

1. Historical terminal owner metadata cannot force a launch request onto a
   route that the executor will reject as an unreconciled existing session.
2. A terminal replacement is admissible only after exact process absence,
   profile-lock release, terminal lifecycle, satisfied cleanup, exact profile
   digest, and authenticated principal capability are proven.
3. Live, transferring, closing, ambiguous, profile-mismatched, path-mismatched,
   or capability-mismatched owners remain fail closed.
4. The repair must not authorize a duplicate live profile process.
5. Last30days retry ordinals must be bounded by the configured attempt ceiling,
   with schema migration preserving existing receipts and idempotence.
6. No source test launches a provider browser or mutates credentials.

## Acceptance Criteria

- The observed planner output can be executed through the public service
  request seam without `existing_session_profile_identity_unproven`.
- Negative fixtures preserve every live-owner and identity mismatch fence.
- Access planning and execution agree on the exact replacement session route.
- Last30days records retry ordinal two when `attempts=3` and rejects ordinals
  outside its configured or schema-supported bound.
- Focused tests are red before each repair and green afterward.
- Selected presubmit and comprehensive suites for both repositories pass.
- The Agent Browser development candidate passes doctor plus three disposable
  launch cycles before production installation.
- Installed Agent Browser source, binary, protected authority, selected runtime
  host, and generation agree, with one current host and zero legacy daemons.
- Installed Last30days reports schema and service compatibility, and its saved
  80-item configuration remains unchanged.
- One bounded acceptance tick reaches provider execution without either
  repaired failure and returns all attempts and leases to zero.

## Bounds And Stop Rules

- Agent Browser implementation attempts: `0/2`.
- Last30days implementation attempts: `0/2`.
- Review and rework cycles: `0/1`.
- Production candidate installations: `0/2`, one per repository.
- Acceptance ticks: `0/1`.
- Checkpoint interval: three slices or ninety minutes.
- Stop before installation if development or comprehensive validation fails.
- Stop before the acceptance tick if any live profile holder, active tick,
  provider attempt, or unreconciled runtime multiplicity appears.
- Do not directly edit Agent Browser service state or the Last30days database.

## Lane Reconciliation

P150 owns automatic convergence of inactive remote-view acquisition
quarantines in `service_health.rs` and `service_retained_state.rs`. P152 owns
terminal browser-owner replacement planning and executor admission in
`service_access.rs`, `action_runtime/runtime/daemon.rs`, their focused tests,
and the Last30days schema/migration surfaces. The conceptual overlap is
terminal retained state, but the failure predicates, implementation files, and
acceptance effects are distinct. Reconcile both tips before integration and
rerun selected validation if either lane changes shared planning or model
contracts.

## Initial Checkpoint

State transition: `ready -> active`.

Acceptance state: live diagnosis complete; source repair not started.

Progress classification: `blocker_reduction`.

Evidence: current access planning recommends `launch_new_browser` but returns
`last30days-force-20260901-c35`; the exact terminal lifecycle proves process
exit and profile-lock release; the retained principal binding is owner
generation 66 while the terminal owner is generation 71; execution failed
before page observation; schema 16 rejected retry ordinal two.

Material blocker: planner and executor have no end-to-end terminal replacement
parity regression, and Last30days schema does not implement its configured
three-attempt contract.

Next action: add the single Agent Browser red regression at the public
access-plan-to-executor seam.
