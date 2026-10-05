# Remote View Foundational Architecture Plan

Date: 2026-06-24
State: DONE
Lane: P47
Depends On:
- `docs/dev/plans/0045-2026-06-23-remote-view-architecture-consolidation-plan.md`
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- `/tmp/architecture-review-agent-browser-2026-06-24T19-29-03-056Z.html`

## Purpose

Resolve the foundational architecture issues exposed by P46 before adding
another remote-view remediation layer. The plan tackles one deepening target at
a time. Each item is intentionally shaped as a `/goal`-compatible objective
that can be executed, validated, closed, or blocked independently.

P46 remains locked until S2 can prove two independent UX operators viewing one
route-bound browser with functional controls and zero active incidents. P47 is
the sequencing plan for the structural work that should make that retry
boring.

## Execution Rules

- Execute the goals in order unless a later goal is explicitly split out by the
  maintainer.
- Do not treat more routes as a fix for S2. S2 should consume one route-bound
  target browser and zero route leases for viewer clients.
- Keep every goal evidence-backed. Each closeout must cite source changes,
  focused tests, and, when applicable, no-launch contract checks.
- Do not unlock or rerun P46 S2 from inside these architecture goals unless
  the active goal explicitly says to do so.
- If a goal uncovers that the planned module shape is wrong, stop and update
  this plan or a dated note before continuing to the next goal.
- Preserve the installed/runtime contract: CLI, HTTP service, MCP surface,
  generated client, and dashboard must agree or fail with a typed capability
  error.

## Goal 1: Separate Viewer Client From Target Browser

`/goal execute P47 goal 1: separate viewer-client from target-browser so dashboard operator browsers can observe and control viewport UX without becoming service-owned route-bound browsers or consuming route leases`

### Problem

P46 S2 failed because the harness used `agent-browser` sessions as dashboard
operator browsers. Those viewer clients became service-owned target browsers,
competed for route-pool resources, and created faulted incidents.

### Desired Shape

Create a clear viewer-client module or harness adapter with a small interface:

- launch or attach a dashboard viewer;
- authenticate against the dashboard;
- load a workspace URL;
- click viewport controls;
- capture dashboard state and screenshots;
- close without touching service-owned route, display, browser, tab, or lease
  state.

The viewer-client interface must not be able to reserve routes, allocate
displays, publish service browsers, or mutate target-browser ownership.

### Evidence Required

- Focused code or harness changes showing viewer-client operations do not call
  `agent-browser remote-view open`, service session launch, or route checkout.
- A no-live test or harness fixture proving viewer-client launch metadata is
  captured and classified separately from target-browser state.
- P46 S2 runner remains parseable and records external viewer launch
  executable, arguments, port, stdout, stderr, and readiness URL.
- Service status after a viewer-client-only smoke has no additional service
  browsers, route-pool checkouts, or active incidents.

### Stop Conditions

- Stop if the only available viewer path still requires service-owned browser
  registration.
- Stop if dashboard auth or screenshots require leaking credentials into
  artifacts.

## Goal 2: Deepen Route-Bound Lease Ownership

`/goal execute P47 goal 2: deepen route-bound lease ownership so route, display, browser, tab, proof, finalize, and rollback are owned by one typed module instead of reinterpreted across remote-view code paths`

### Problem

Route, display, browser, tab, stream, and proof records are individually
reasonable but shallow as a combined ownership model. Multiple modules can
still answer who owns a route-bound browser or whether a route is safe to
reuse.

### Desired Shape

Introduce or consolidate a route-bound lease module with one external
interface:

- plan without mutation;
- reserve;
- attach browser and tab;
- finalize only after proof is ready;
- rollback after failure;
- inspect typed state for diagnostics.

The module should concentrate the state machine from `requested` through
`finalized`, `rolled_back`, or `failed_diagnostic`.

### Evidence Required

- Typed Rust structs or enums for the lease state and allowed transitions.
- Unit tests for legal transitions and illegal skips.
- Forced-failure tests proving post-reservation failures pass through rollback
  or produce a typed repair record that the live rail cannot render as a
  controllable target.
- Existing route-pool, display-allocation, and remote-view open behavior stays
  contract-compatible through CLI, HTTP, MCP, and generated client surfaces.

### Stop Conditions

- Stop if `actions.rs` or dashboard code still needs to infer final ownership
  from partial route/display/browser records.
- Stop if a failure path can publish a live control row without finalized
  proof.

## Goal 3: Publish Canonical Workspace Inventory

`/goal execute P47 goal 3: publish canonical workspace inventory so the dashboard renders ownership, role, actionability, and live-control state from one record instead of reconstructing it from row shape, URL params, or stream URLs`

### Problem

Dashboard modules currently derive workspace rows, selected context, viewport
state, and actionability from service status shape, URL params, stream scoring,
and local heuristics. That makes the UI shallow and allows ownership answers
to drift.

### Desired Shape

Move the canonical answer into a workspace inventory module. Rust should
publish a typed record that includes:

- inventory class;
- role: target browser or viewer client;
- ownership summary;
- selected tab and target evidence;
- view-stream capabilities;
- allowed actions with disabled reasons;
- diagnostics and retained-history classification.

Dashboard TypeScript should become an adapter that renders this record instead
of recomputing ownership and actionability.

### Evidence Required

- Contract/schema for the canonical inventory record.
- Rust no-launch tests proving service-owned target browser, viewer client,
  detected non-owned browser, retained history, and diagnostic rows classify
  correctly.
- Dashboard tests proving live rail, selected context, viewport, and inspector
  actions consume canonical class and actions.
- Generated client updates if the HTTP/MCP contract changes.

### Stop Conditions

- Stop if URL params can still upgrade a retained, diagnostic, or viewer row
  into a controllable target.
- Stop if the dashboard can synthesize mutating controls for a row whose
  canonical actions forbid them.

## Goal 4: Deepen Operator-Visible Proof

`/goal execute P47 goal 4: deepen operator-visible proof so remote-view success returns one ready proof or one typed blocker covering CDP target, route display, Guacamole route, dashboard viewport, and selected tab freshness`

### Problem

Operator-visible proof is scattered across CDP URL/title readback, X11 display
inspection, Guacamole route checks, dashboard iframe state, screenshots, and
viewport readiness copy. The caller has to assemble the proof by convention.

### Desired Shape

Create one proof module with one interface:

- input: intent plus finalized or pending route-bound lease facts;
- output: `ready` proof or typed blocker;
- evidence rows: target freshness, browser window visible, route routable,
  dashboard viewport reachable, tab selection agreement, and display content
  classification.

The proof module should be usable by the runtime and by P46 artifacts without
copying the logic into scripts.

### Evidence Required

- Fixture tests for terminal-only display, terminal-topmost display,
  wrong-tab, stale target, blank target, non-routable Guacamole route,
  dashboard login required, and success.
- Remote-view open only reports `operatorVisible.state=ready` after this proof
  passes.
- P46 runner can record proof output directly instead of recomputing the same
  conclusions from independent artifacts.

### Stop Conditions

- Stop if success can be reported while the selected target is stale, blank,
  or not visible on the selected route display.
- Stop if dashboard readiness copy and runtime proof can disagree without a
  typed diagnostic.

## Goal 5: Turn P46 Into A Scenario Harness Module

`/goal execute P47 goal 5: turn the P46 stress runner into a scenario harness module with declarative scenarios, explicit roles, reset protocol, evidence recorder, and failure audit classification`

### Problem

`scripts/run-p46-stress-scenario.js` is useful but shallow. Scenario setup,
runtime mutation, viewer-client automation, evidence capture, evaluation,
reset, and audit classification all share one script-level interface.

### Desired Shape

Split the runner into a harness module with:

- scenario specs for S0, S1, S2, and future S3+;
- role declarations: target browser, viewer client, route, profile, operator;
- runtime adapter;
- viewer-client adapter;
- evidence recorder;
- reset protocol;
- evaluator;
- failure audit writer.

The scenario spec should state invariants such as "S2 uses one target-browser
route lease and two viewer clients that consume zero route leases."

### Evidence Required

- No-live tests for scenario spec parsing, role validation, audit
  classification, and reset invariants.
- S0 and S1 still run through the harness.
- S2 remains locked until maintainer approval, but its spec exists and rejects
  viewer clients that consume routes.
- Failure artifacts remain under `/tmp/agent-browser-p46-<scenario>-<timestamp>/`.

### Stop Conditions

- Stop if the harness cannot distinguish product failure from viewer-client
  adapter failure.
- Stop if scenario retry logic can bypass the two-failure lock rule.

## Goal 6: Re-Audit P46 S2 And Unlock The Stress Matrix

`/goal execute P47 goal 6: after goals 1 through 5 are validated, re-audit P46 S2 from a clean runtime and either unlock one S2 retry or keep P46 locked with a new evidence-backed blocker`

### Problem

P46 is currently locked by two S2 failures. The next live retry should happen
only after viewer-client role separation, route-bound lease ownership,
canonical inventory, operator-visible proof, and harness structure are strong
enough to make the retry meaningful.

### Desired Shape

Run a no-mutation audit first:

- install doctor;
- remote-view doctor;
- service status;
- route-pool readiness;
- display-content inspection;
- dashboard contract readback;
- harness spec validation.

If the audit is clean, unlock exactly one S2 retry. If it fails, keep P46
locked and write a dated note with the blocker.

### Evidence Required

- Clean preflight artifacts.
- S2 retry artifact showing:
  - one route-bound target browser;
  - two viewer clients;
  - one route lease for the target browser;
  - zero route leases for viewer clients;
  - dashboard screenshots for both operators;
  - refresh/control proof;
  - route display screenshot;
  - controlled-browser URL/title after navigation;
  - zero active incidents after reset.
- If S2 passes, update P46 state from `LOCKED` to the next explicit state and
  continue the matrix only under the P46 rules.

### Stop Conditions

- Stop if preflight finds stale helper, stale installed binary, dirty route
  leases, display permission drift, or active incidents.
- Stop if S2 creates extra route leases, extra target browsers, or viewer
  client incidents.

## Completion Criteria

P47 is complete only when every goal above is either:

- completed with evidence in source, tests, and notes; or
- explicitly split into a newer plan with this plan updated to point at that
  successor.

P46 should not be considered recovered by P47 until Goal 6 proves the S2
shared-view case or records a new blocker that is not the original harness role
confusion.

## Closeout

Completed goals 1 through 5 with focused source changes and no-live or unit
tests. Goal 6 ran a clean preflight and exactly one S2 retry. P46 remains
locked with a new blocker recorded in
`docs/dev/notes/2026-06-24-p47-6-s2-reaudit-blocker.md`: route/display
allocation finalization or reconciliation drift after an otherwise functional
S2 run.
