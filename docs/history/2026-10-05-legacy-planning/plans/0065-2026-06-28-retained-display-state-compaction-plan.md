# Retained Display State Compaction Plan

Date: 2026-06-28
State: COMPLETED
Lane: P65
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0029-2026-06-07-live-retained-pressure-cleanup-plan.md`
- `docs/dev/plans/0032-2026-06-08-historical-placeholder-prune-plan.md`

## Purpose

Make retained remote-view display state auditable after P46 without weakening
the route-pool and live-control guarantees that P46 just proved.

P46 closed with zero service browsers, zero service sessions, zero tabs, zero
active incidents, released remote-view routes, and both Guacamole route-pool
entries available. The residual issue is retained metadata noise: historical
orphaned display-allocation rows still appear in `service status`, including
records with no live browser/session/tab owner and no route-pool capacity held.

This plan should classify, explain, and compact that history through supported
service actions and doctor surfaces. It must not erase useful crash or
diagnostic evidence merely to make counts look smaller.

## Source Evidence

Authoritative closeout evidence:

- P46 campaign summary:
  `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- P46 execution note:
  `docs/dev/notes/2026-06-24-p46-stress-hardening-execution.md`
- Runbook closeout:
  `RUNBOOK.md`
- final service status:
  `/tmp/agent-browser-p46-final-service-status.json`
- final install doctor:
  `/tmp/agent-browser-p46-final-install-doctor.json`
- final remote-view doctor:
  `/tmp/agent-browser-p46-final-remote-view-doctor.json`
- final incident summary:
  `/tmp/agent-browser-p46-final-incidents-summary.json`
- final route-pool readiness:
  `/tmp/agent-browser-p46-final-route-pool-readiness.json`

Observed final state from P46 closeout:

- zero service browsers;
- zero service sessions;
- zero tabs;
- zero active incidents;
- `guacamole-rdp-a` and `guacamole-rdp-b` available with no current route
  allocation;
- active remote-view routes released;
- retained historical display allocations still visible, including orphaned
  rows with empty `routeIds`.

## Non-Goals

- Do not change P46 pass criteria retroactively.
- Do not manually edit service-state JSON.
- Do not delete live display, route, session, browser, tab, lease, incident, or
  profile records.
- Do not prune records that still carry unresolved incident, crash, host, or
  route-finalization evidence.
- Do not hide retained history in dashboard or doctor output without an
  explicit retained-state explanation and dry-run cleanup path.
- Do not force-kill browser or display processes as part of retained metadata
  compaction.

## Safety Rules

- Every cleanup action must have a dry-run mode and reviewed candidate counts
  before apply.
- Apply must only remove records that the dry-run classified as safe and inert.
- Route-pool readiness must remain green before and after any apply.
- Install doctor and remote-view doctor must remain green after any apply.
- Service status must still expose enough evidence to explain why a record was
  retained or removed.
- The dashboard must distinguish live control rows from retained historical
  rows.

## Candidate Classes

Classify display-related retained state into explicit groups:

- live: display allocation or route is tied to a live browser, session, tab,
  active route-pool checkout, active lease, or active incident;
- diagnostic-retained: record is not live but carries useful unresolved or
  recent diagnostic evidence;
- safe-orphan-display: display allocation has no live owner, no active route,
  no current route-pool checkout, no active incident, and no retained route IDs;
- stale-route-reference: display allocation has `routeIds` referencing only
  released remote-view routes and no current route-pool checkout;
- historical-placeholder: display allocation was produced by an older workflow
  and is neither live nor diagnostically useful;
- unknown: insufficient evidence to prune.

Only `safe-orphan-display`, reviewed `stale-route-reference`, and reviewed
`historical-placeholder` candidates may be compacted by this plan.

## Slice A | Audit And Classifier

Add a retained display-state classifier shared by service status, doctor
diagnostics, and retained cleanup.

Exit criteria:

- Classifier returns candidate type, reason, linked route IDs, linked browser
  IDs, linked session IDs, linked incident IDs, and whether the candidate is
  apply-safe.
- Tests cover live records, released routes, orphaned displays with empty
  routes, orphaned displays with released route references, active route-pool
  checkouts, and unresolved incident evidence.
- Current final P46 service-state sample can be classified without panics or
  unknown live-control ambiguity.

## Slice B | Dry-Run And Apply Surface

Extend the supported retained cleanup surface with display-state compaction.

Preferred shape:

```bash
agent-browser --json service prune-retained --display-allocations --dry-run
agent-browser --json service prune-retained --display-allocations --apply
```

If the existing prune contract cannot safely carry the shape, add a narrowly
named service request action with the same dry-run/apply behavior instead of
overloading unrelated cleanup flags.

Exit criteria:

- Dry-run reports candidate counts by class and lists candidate IDs.
- Apply removes only the reviewed safe candidate set from service state.
- Apply is idempotent; a second dry-run reports zero candidates or only
  intentionally retained records.
- Apply never changes route-pool entries from checked out to available unless
  that route-pool repair is explicitly requested and separately reported.

## Slice C | Doctor And Dashboard Explanation

Make retained display-state noise understandable without implying live failure.

Exit criteria:

- Install doctor and remote-view doctor distinguish live readiness blockers from
  retained historical display-state warnings.
- Service status or service resources reports retained display-state counts
  separately from live browser/session/tab counts.
- Dashboard retained-state warning, if shown, names display-allocation
  candidates and offers dry-run before apply.
- No native browser dialogs are introduced in the dashboard.

## Slice D | Live Cleanup And Regression Proof

Run a reviewed live cleanup on the current workstation only after Slice A
through Slice C pass.

Required evidence:

- before service status;
- dry-run candidate report;
- reviewed apply result;
- after service status;
- route-pool readiness;
- install doctor;
- remote-view doctor;
- incident summary.

Exit criteria:

- Live cleanup removes only safe retained display-state candidates or records
  why apply was intentionally skipped.
- Final route-pool entries are available or unchanged from their intended live
  state.
- Final service status has zero active incidents and no stale retained live
  control rows.
- Any remaining historical display allocation is labeled as diagnostic-retained
  or unknown with an explicit reason.

## Validation

Run the selector first and then the relevant checks it recommends:

```bash
pnpm validation:select -- --base HEAD
```

Expected focused checks for source changes:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml prune_retained_service_state -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

If service request contracts, generated clients, dashboard retained-state UI,
or docs change, also run the corresponding selected client, dashboard, docs,
and parity checks.

Live validation after installing a patched runtime:

```bash
agent-browser --json service status
agent-browser --json service prune-retained --display-allocations --dry-run
agent-browser --json service prune-retained --display-allocations --apply
agent-browser --json service reconcile
agent-browser --json service status
agent-browser --json service incidents --summary
agent-browser --json install doctor
agent-browser --json doctor remote-view
node scripts/smoke-rdp-guac-route-pool-readiness.js --report-only
```

## Closeout Criteria

P65 is complete:

- retained display-state classes are explicit and test-covered;
- cleanup dry-run and apply are supported through service-owned actions;
- service status surfaces explain retained display-state separately from
  live control readiness;
- live cleanup was explicitly skipped with a reviewed reason;
- final service status, route-pool readiness, install doctor, remote-view
  doctor, and incident summary are captured;
- remaining retained display allocations have explicit retained
  reasons;
- `RUNBOOK.md` and the relevant execution note name the outcome and next step.

## Implementation Summary

Changed source:

- Added the retained display-allocation classifier to `cli/src/native/service_model.rs`.
- Extended `service prune-retained` with `--display-allocations`.
- Added dry-run/apply JSON fields for display allocation candidates,
  candidate class counts, and candidate reasons.
- Added `retainedDisplayAllocations` to service status responses and text
  output so retained historical display rows are not confused with live
  browser, session, tab, or route-pool readiness.
- Updated service request/status contracts, generated service-client types,
  README, docs site pages, and `skills/agent-browser/SKILL.md`.

Classifier classes:

- `live`
- `diagnostic-retained`
- `safe-orphan-display`
- `stale-route-reference`
- `historical-placeholder`
- `unknown`

Apply removes only apply-safe `safe-orphan-display`,
`stale-route-reference`, and `historical-placeholder` candidates. It does not
change route-pool entries.

## Validation Results

Focused validation:

- `cargo test --manifest-path cli/Cargo.toml service_prune_retained -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_prune_retained_service_state_classifies_display_allocations -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_service_status_via_actions_does_not_launch_browser -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_status_text_includes_profile_and_session_summaries -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_status_and_collection_response_contracts_match_wire_shape -- --nocapture`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`

Runtime convergence:

- Built and installed the local debug binary to `~/.local/bin/agent-browser`.
- Ran `pnpm publish:local-dashboard -- --skip-browser --json` to sync the
  workspace and pnpm global reference binaries and restart
  `agent-browser-dashboard.service`.
- Removed two stale deleted-executable default daemon listeners that install
  doctor identified after binary replacement.

Live artifact directory:

- `/tmp/agent-browser-p65-retained-display-20260628T174225Z`

Live cleanup result:

- `display-prune-dry-run.json` reported zero apply-safe display allocation
  candidates.
- Apply was skipped because there was nothing apply-safe to remove.
- Before and final status both reported 22 retained display allocations:
  16 `diagnostic-retained`, 6 `live`, and 0 apply-safe.
- Remaining display allocations have explicit `candidateReasons` with class,
  reason, linked route IDs, linked browser IDs, linked session IDs, linked
  incident IDs, linked route-pool entry IDs, and `applySafe`.

Final live proof:

- `final2-service-status.json`: success; 1 live browser, 1 live session, 1 tab;
  retained display allocation summary present with 0 apply-safe candidates.
- `final2-incidents-summary.json`: success; incident count 0.
- `final2-install-doctor.json`: success; no issues.
- `final2-remote-view-doctor.json`: success; status `ready`; no issues.
- `final2-route-pool-readiness.json`: success; status `ready`.

Next step:

- No retained display allocation compaction is needed on the current
  workstation until a future dry-run reports apply-safe candidates.
