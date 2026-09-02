# Plan 0152 | Terminal Session Replacement Planner Executor Parity

Date: 2026-09-01

State: ACTIVE

Execution state: `slice_f_production_installation`

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

## Agent Browser Source Checkpoint

State transition: `active -> source_repair_validated` for slices A and B.

Acceptance state: the planner now derives a deterministic fresh
`terminal-profile-*` launch lane from the selected profile, logical browser,
and terminal owner generation. The retired daemon lane remains lifecycle
evidence. Request normalization admits only the exact fresh lane projected by
the current plan and attaches authenticated-cold route authority; arbitrary
missing sessions still fail closed. The daemon revalidates the current owner
ID, generation, registry revision, profile digest, principal capability, and
fresh session before launch.

Red/green evidence:

- the Plan 0137 reproduction initially showed that planning reused the retired
  daemon session;
- the copied unauthenticated-plan request initially failed authenticated
  normalization with `explicit_session_route_invalid`;
- the executor fixture with terminal owner generation 57 and retained
  principal binding generation 56 initially returned `false` at authenticated
  profile selection; and
- all three pass after the repair, together with the missing-explicit-session
  negative fence.

Validation evidence:

- `cargo fmt --check`: pass;
- production binary `cargo clippy --bin agent-browser -- -D warnings`: pass;
- complete Rust suite: 2,827 pass, 15 fail, 57 ignored under parallel
  execution; every failure used the same shared temporary HOME and the failed
  control-plane cluster (43 tests) plus both independent failed tests pass when
  rerun serially;
- all-target clippy remains blocked by three pre-existing
  `clippy::octal_escapes` warnings in `desktop_capture.rs`, outside P152.

Progress classification: `blocker_reduction`.

Next action: implement the Last30days retry-ordinal schema migration with a
red migration regression, then qualify both repositories before installation.

## Integrated Development Qualification

State transition: `source_repair_validated -> development_qualified` for
Slices C through E.

The Last30days companion branch adds schema 17, preserves referenced provider
results while rebuilding only `service_tick_provider_attempts`, admits retry
ordinal two, and continues to reject ordinal three. Its complete Python suite,
Go suite, lifecycle rollback coverage, compatibility-contract generation, and
planning audits pass.

Agent Browser was reconciled with the integrated P150/P151 mainline at
`29962d9f`. Both plan histories and current lane ownership were retained. On
that exact merged tree:

- focused planner-to-executor regressions pass;
- production-binary Clippy with warnings denied passes;
- the complete serial Rust workspace passes with 2,854 tests, zero failures,
  and 57 intentional ignores;
- the all-target Clippy gate still reports 31 mainline test-only warnings in
  unrelated P150/P151 fixtures, while the production target is clean;
- optimized candidate SHA prefix `4a9882a9f4d7` installed as isolated
  development generation `0.28.0-4a9882a9f4d7`;
- development doctor reports ready across selected generation, executable,
  runtime host, dashboard, protected authority, provider isolation, routes,
  and warm displays; and
- three disposable browser open, URL-read, close, and residue cycles pass.

Production remained unchanged during development qualification.

Progress classification: `outcome_progress`.

Next action: checkpoint this evidence, merge P152 to `main`, build the exact
integrated production candidate, and run the workstation dry-run before apply.

## Production Acceptance And Capability Gate

State transition: `development_qualified -> installed_acceptance_blocked`.

Agent Browser generation `0.28.0-390ee922ae7b-e2f4b5e5d874` and Last30days
service 0.3.93/schema 17 were installed transactionally. Agent Browser source,
installed binary, and protected-authority digests agree; doctor succeeds with
one current runtime host and zero legacy daemons. Last30days is ready and
compatible with MCP 4.0.4, SQLite quick-check succeeds, and the schema admits
retry ordinals zero through two.

The single authorized X and LinkedIn tick `tick-5dc9aa4fdac60025dddb7ea283713b00`
terminalized `complete_degraded`. Both providers executed and persisted all
three attempts, proving the schema repair and removing the old cross-provider
insertion blocker. All tick, provider-attempt, and lease residue returned to
zero.

The six browser attempts retained `existing_session_profile_identity_unproven`.
Readback proved Last30days copied the fresh planned `terminal-profile-*` lane;
it did not supply a profile capability to either `service_access_plan` or
`service_request`. The executor correctly requires that capability before it
can attach authenticated route authority. The remaining parity defect was the
unauthenticated planner advertising `serviceRequest.available=true` despite
that executor requirement.

The follow-up repair makes an exact terminal owner with satisfied cleanup and
no authenticated principal return `available=false`, null request, and
`acquisitionBlocker=profile_capability_required`. Request normalization now
returns `service_access_plan_request_unavailable:profile_capability_required`
before daemon relay for both fresh and historical terminal session hints. The
authenticated terminal replacement path remains executable. All 61 focused
service-access tests, the Plan 0137 regression, formatting, and strict
production-target Clippy pass.

Acceptance state: retry persistence is production-proven; truthful planning is
source-qualified; successful Last30days acquisition remains blocked on a new
private capability registration and client wiring. That work is an
authentication and identity-authority mutation excluded by the frozen P152
scope and requires explicit operator authorization. The one permitted
acceptance tick is consumed.

Progress classification: `blocker_reduction`.

Next action: merge and install the truthful-planning candidate without another
provider tick, then request explicit authority for Last30days capability
registration and private configuration wiring.

## Truthful-Planning Installation Checkpoint

State transition: `installed_acceptance_blocked -> truthful_planning_installed`.

Merge `8719b57b` is pushed to `origin/main`. Its exact CI-profile binary SHA-256
is `3a090663b346fab7e8d2e1d7d3aa2ce707002a2c99e8eefd9d373b8c59c52656`,
which matches the installed command and selected generation
`0.28.0-3a090663b346-7118cc148917`. Transaction
`upgrade-1fb19429-683b-4203-b202-82eed2e37cb5` is accepted. Doctor succeeds in
steady state with one current runtime host, zero legacy daemons, and one
executable generation.

A live, no-launch Last30days/X access plan selects `last30days-facebook`, sees
the exact generation-71 terminal owner with process absence and satisfied
cleanup, and now returns `serviceRequest.available=false`, null request, and
`acquisitionBlocker=profile_capability_required`. This proves the public
planner no longer promises an executor-inadmissible request. No provider tick
or browser launch was used for this check.

Acceptance state: truthful planner/executor parity is production-accepted for
the unauthenticated case. End-to-end Last30days browser acquisition remains
authorization-blocked on capability registration and private client wiring.

Progress classification: `outcome_progress`.

Next action: obtain explicit operator authority for the identity mutation;
after implementation, obtain separate authority for any additional acceptance
tick because the P152 tick budget is exhausted.
