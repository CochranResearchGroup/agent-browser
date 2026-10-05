# Live Retained Pressure Cleanup Plan

Date: 2026-06-07
State: CLOSED
Lane: P13
Depends On:
- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`
- `docs/dev/plans/0028-2026-06-06-review-validation-and-commit-plan.md`

## Purpose

Reduce current workstation browser pressure using the service-owned cleanup
surfaces added in Plans 0026 and 0027, without force-killing protected or
ambiguous browser processes.

## Initial Evidence

`agent-browser --json service resources` reported:

- `candidateCount: 0`
- `observedCount: 81`
- `protectedCount: 1`
- `totalProcesses: 82`
- `totalRssBytes: 12082565120`
- six duplicate-profile-pressure warnings

`agent-browser --json install doctor` remained nonzero because:

- PATH debug runtime differs from the pnpm global binary.
- PATH debug runtime differs from the workspace release binary.
- duplicate profile pressure is present in live retained state.

`agent-browser --json service gc --dry-run` reported no process GC
candidates. The safe next action is retained-state cleanup, not process
termination.

`agent-browser --json service prune-retained --dry-run --released-sessions
--abandoned-sessions --orphaned-profiles --process-exited-browsers` reported
1,311 retained cleanup candidates:

- 120 browser records
- 702 closed tabs
- 369 orphaned custom profiles
- 120 sessions

`agent-browser --json service repair-retained --dry-run` reported 126 legacy
session placeholders missing `lastLeaseObservedAt` evidence. Repair is useful
for future age-gated cleanup, but it is not the first cleanup action because
stamping current observation time intentionally makes those sessions too fresh
for abandoned-session pruning.

## Scope

- Apply only service-owned retained-state cleanup that already passed dry-run.
- Do not force-kill live browser processes.
- Do not edit private runtime state by hand.
- Reconcile service state after cleanup.
- Re-run resource and doctor readbacks.
- If duplicate pressure remains, record the remaining profile/session groups
  and decide whether code needs a narrower stale-duplicate lease cleanup command.

## Validation

Required live checks:

- `agent-browser --json service prune-retained --apply --released-sessions --abandoned-sessions --orphaned-profiles --process-exited-browsers`
- `agent-browser --json service reconcile`
- `agent-browser --json service resources`
- `agent-browser --json install doctor`

Required repo checks if this plan is the only file changed:

- `git diff --check`

If source, docs, generated clients, or scripts change while closing this plan,
run `pnpm validation:select -- --base HEAD` and execute the relevant selected
checks before committing.

## Closeout Contract

Close this plan only after cleanup is applied or explicitly proven unsafe,
live readbacks are recorded, and any remaining duplicate-pressure state is
identified with its next concrete remediation.

## Closeout

Completed on 2026-06-07.

Applied retained cleanup without force-killing browser processes:

- First reviewed apply removed 120 browser records, 702 closed tabs, 369
  orphaned profiles, and 120 sessions.
- Reconcile rehydrated closed tab and failed browser evidence from the running
  service cache, so the retained prune was rerun against the reconciled state.
- Second reviewed apply removed 120 browser records, 702 closed tabs, and 120
  sessions.
- Third reviewed apply removed 56 newly orphaned profiles.

The remaining duplicate warnings were old failed or unreachable retained
session lanes with stale view-stream metadata. The existing cleanup policy only
accepted inert `not_started` browser placeholders, so this plan added a narrow
source fix: when the operator explicitly passes both `--abandoned-sessions` and
`--process-exited-browsers`, old abandoned sessions can prune linked
`process_exited` or `unreachable` browser records that have no retained tabs.
The flag remains explicit because those records can carry crash or host-failure
evidence.

After rebuilding with that fix:

- Patched dry-run found 61 old failed retained sessions and 61 linked browser
  records.
- Patched apply removed those 61 sessions and 61 browser records.
- A final orphan pass removed 13 more profiles.
- Final prune dry-run reported zero candidates:
  `browsers: 0`, `closedTabs: 0`, `orphanedProfiles: 0`, `sessions: 0`,
  `total: 0`.

Final live readbacks:

- `agent-browser --json service resources` reported `candidateCount: 0`,
  `observedCount: 68`, `protectedCount: 3`, `totalProcesses: 71`,
  `totalRssBytes: 9213554688`, and one duplicate-pressure warning after the
  final live CDP smoke.
- `agent-browser --json install doctor` reported service resources available,
  zero cleanup candidates, and one duplicate-pressure warning. It remained
  nonzero because the local debug runtime intentionally differs from the pnpm
  global and workspace release binaries, and because one duplicate active
  profile-lease group remains.
- The remaining duplicate group is `custom:1239175796708298334` with sessions
  `odollo-usps-debug`, `soylei-live-deploy`, `soylei-nav-fix`,
  `soylei-unsubscribe-review`, and `ups-headed`. Those records lack parseable
  lease-age evidence, so they are retained by policy instead of deleted.

Validation passed:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml prune_retained_service_state -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml test_service_prune_retained -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm test:service-cdp-tab-streaming-live`
- `pnpm --dir docs build`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm validation:select -- --base HEAD`
- `pnpm publish:local-dashboard -- --skip-browser --json`

The patched local runtime was published to `/home/ecochran76/.local/bin/agent-browser`
and `agent-browser-dashboard.service` was restarted successfully.
