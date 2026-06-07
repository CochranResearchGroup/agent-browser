# Resource Monitor And Garbage Collector Plan

Date: 2026-06-04
State: CLOSED
Lane: P13
Depends On:
- `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`
- `docs/dev/plans/0010-2026-05-30-retained-orphan-profile-cleanup-plan.md`
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`
- `docs/dev/plans/0025-2026-06-01-remote-view-target-attribution-and-idle-display-plan.md`

## Purpose

Add a service-owned resource monitor and conservative garbage collector for
agent-browser runtime processes, browser process groups, temporary profiles,
and remote-view display helpers.

The 2026-06-04 live cleanup found several multi-day stale runtime resources:

- `chromium-stealthcdp` process groups launched with `/tmp/agent-browser-*`
  profiles from old plan and smoke runs.
- Xvfb displays left behind after the owning `agent-browser` process was no
  longer operational.
- no-argument `agent-browser` daemon siblings outside
  `agent-browser-dashboard.service`, including one process around 3.9 GB RSS.
- a default runtime profile status whose recorded browser PID and DevTools URL
  were stale.

The service queue was empty, and the live dashboard service was healthy, but OS
resource use remained high because stale resource ownership was not visible or
reclaimed.

## Current State

- Live cleanup on 2026-06-04 reduced agent-browser-specific resource use to the
  running `agent-browser-dashboard.service` process.
- The repo has retained-state cleanup for orphaned custom profile records, but
  no first-class inventory for live OS processes, stale process groups,
  orphaned display servers, or stale runtime-state pointers.
- The dashboard can show service state and selected workspace evidence, but it
  does not yet summarize system resource pressure or stale process candidates.
- No unattended garbage collection is approved.

## Execution Record

### 2026-06-05 Slice A Through C Read-Only Implementation

Implemented:

- Added `cli/src/native/service_resources.rs` with Linux process-table
  collection, service-state correlation, sanitized command previews, conservative
  disposition reasons, and fixture-backed classifier tests.
- Added `agent-browser service resources` for read-only resource inventory.
- Added `agent-browser service gc --dry-run` for review-only candidate grouping.
  Apply mode is intentionally rejected until Slice D review tokens and PID
  identity rechecks are implemented.
- Wired no-launch command parsing, native action dispatch, text output, CLI help,
  README, docs site, and `skills/agent-browser/SKILL.md`.

Validation evidence:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_resources
cargo test --manifest-path cli/Cargo.toml test_service_gc
cargo test --manifest-path cli/Cargo.toml test_command_skips_browser_launch_for_service_resource_maintenance
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo build --manifest-path cli/Cargo.toml
./cli/target/debug/agent-browser service resources --json
./cli/target/debug/agent-browser service gc --dry-run --json
```

Live proof on the current workstation:

- `service resources --json` returned `success=true`, `totalProcesses=90`,
  `correlatedProcesses=2`, `protectedCount=3`, `candidateCount=0`, and no
  warnings.
- Retained Xvfb display `:107` was protected with
  `retained_display_allocation`.
- The retained active browser on the same display was protected with
  `retained_active_browser`, `retained_named_or_persistent_profile`, and
  `retained_display_allocation`.
- `service gc --dry-run --json` returned `success=true`, `candidateCount=0`,
  projected reclaimed RSS `0`, and no warnings.

Remaining:

- Slice E dashboard and doctor resource-pressure surfaces.
- Slice F optional read-only monitor timer.
- Slice G disposable live GC smoke that proves apply mode touches only generated
  stale resources.

### 2026-06-05 Slice D Guarded Apply Implementation

Implemented:

- Added short-lived dry-run review tokens based on the current candidate identity
  set.
- Added `agent-browser service gc --apply --review-token <token>` and explicit
  `--force-without-review`.
- Apply now re-reads process identity before termination and skips candidates
  whose PID identity changed after dry-run.
- Apply sends SIGTERM first, then SIGKILL only if the process still appears to
  be running.
- Apply always uses the persisted service-state repository and appends a compact
  service event with candidate count, terminated/skipped/failed counts, token
  mode, and projected reclaimed RSS.
- Missing or invalid review tokens fail the command instead of returning a
  successful data-level error.

Validation evidence:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_resources
cargo test --manifest-path cli/Cargo.toml test_service_gc
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo build --manifest-path cli/Cargo.toml
./cli/target/debug/agent-browser service gc --apply --json
./cli/target/debug/agent-browser service gc --dry-run --json
./cli/target/debug/agent-browser service gc --apply --review-token <token> --json
./cli/target/debug/agent-browser service events --limit 20 --json
```

Live proof on the current workstation:

- Missing-token apply returned `success=false`, `error=review_token_required`,
  and process exit code `1`.
- Dry-run returned a review token with `candidateCount=0`.
- Tokened apply returned `success=true`, `applied=true`, `candidateCount=0`,
  and counts `terminated=0`, `skipped=0`, `failed=0`.
- Event readback returned persisted `details.resourceGc` records with compact
  counts and `tokenMode=review_token`.

Remaining:

- Slice E dashboard and doctor resource-pressure surfaces.
- Slice F optional read-only monitor timer.
- Slice G disposable live GC smoke that creates a generated stale resource and
  proves apply terminates only that generated candidate.

### 2026-06-05 Slice E Through G Visibility, Timer, And Live Smoke

Implemented:

- Added HTTP `GET /api/service/resources` for the same read-only resource
  monitor summary used by the CLI.
- Added a Service dashboard resource status light and non-destructive alert for
  candidate count and estimated candidate RSS.
- Added selected-workspace resource joining so workspace diagnostics show
  related resource candidates, protected resources, and resource reasons for
  the selected browser, service session, profile, or PID.
- Extended `agent-browser install doctor` with a no-launch resource GC dry-run
  and a readiness issue for stale remote display or temporary-profile
  candidates.
- Added `agent-browser service resources --write-monitor-summary` and
  `agent-browser service resources --monitor-summary` for aggregate-only timer
  output.
- Added `scripts/install-resource-monitor-user-timer.sh` and
  `scripts/remove-resource-monitor-user-timer.sh` for an optional user-scoped
  systemd timer. The timer runs read-only inventory only and is not enabled by
  default.
- Added `pnpm test:service-resource-gc-live`, which launches a generated Xvfb
  display, refuses to run if unrelated candidates already exist, proves dry-run
  finds only the generated PID, applies with the review token, and verifies the
  generated candidate is gone.
- Updated README, docs site, and the repo plus installed agent skill guidance.

Validation evidence:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_resources
cargo test --manifest-path cli/Cargo.toml test_service_gc
cargo test --manifest-path cli/Cargo.toml test_command_skips_browser_launch_for_service_resource_maintenance
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo build --manifest-path cli/Cargo.toml
pnpm build:dashboard
pnpm --dir docs build
pnpm test:service-resource-gc-live
pnpm test:service-api-mcp-parity
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm test:dashboard-inspector-actions
pnpm test:dashboard-selected-workspace-context
pnpm test:dashboard-selected-workspace-chat-packet
pnpm test:dashboard-selected-workspace-console
pnpm test:dashboard-view-streams
pnpm test:dashboard-browser-row-actions-render
pnpm test:dashboard-browser-table
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-workspace-nodes
cargo test --manifest-path cli/Cargo.toml install_doctor_flags_readiness_impacting_resource_candidates
bash -n scripts/install-resource-monitor-user-timer.sh scripts/remove-resource-monitor-user-timer.sh
git diff --check
```

Live proof on the current workstation:

- Disposable GC smoke passed with generated Xvfb PIDs `19686` and `49670` in
  earlier runs and PID `17887` in the final run, each ending with
  `after_candidates=0`.
- Monitor summary write/read returned
  `/home/ecochran76/.agent-browser/service/resource-monitor-summary.json`,
  `candidateCount=0`, `protectedCount=3`, and a current `observedAt`
  timestamp.
- Selected-workspace context tests prove related candidate and protected
  resource records appear in workspace evidence and diagnostic bundles.
- Install doctor tests prove readiness-impacting resource candidates produce a
  stable `service_resource_candidates_ready` issue code.
- Live `agent-browser install doctor --json` readback reported
  `serviceResources.available=true`, `candidateCount=0`,
  `readinessImpactingCandidates=0`, and retained only pre-existing binary path
  drift issue codes.

## Product Contract

Agent Browser owns browser process lifecycle, profile leases, CDP attachment,
and remote-view display helpers. The service must therefore expose resource
pressure as first-class service state before operators need to inspect `ps`.

The garbage collector must be dry-run first and fail closed. Apply mode may
terminate only resources that are proven stale by multiple independent signals:
missing or inactive service job, no active profile lease, unreachable CDP or
dead browser root, temporary profile or smoke identity, and age over a
configured threshold.

## Non-Goals

- Do not delete named managed runtime profiles such as `default`,
  `stealthcdp-default`, or site-login profiles.
- Do not terminate operator-owned local Chrome, Brave, or Edge sessions.
- Do not make unattended cleanup the default behavior in this plan.
- Do not conflate retained service-state pruning with live OS process
  reclamation.
- Do not hide resource pressure by silently dropping historical browser,
  session, tab, or incident records.

## Resource Model

The monitor should normalize these resource kinds:

- `agent_browser_daemon`: installed, workspace, or debug `agent-browser`
  process.
- `browser_root`: Chrome or Chromium browser root process.
- `browser_child`: renderer, GPU, utility, zygote, broker, or crashpad child.
- `display_server`: Xvfb or route display process owned by a browser session.
- `runtime_profile`: managed profile directory plus runtime-state pointer.
- `temporary_profile`: `/tmp/agent-browser-*`, smoke, plan, or generated custom
  profile directory.
- `service_record`: service browser, session, tab, profile allocation, job,
  incident, display allocation, view stream, route, or viewer lease record.

Each resource record should carry:

- PID, process group, parent PID, command summary, age, CPU, RSS, and executable
  source.
- Profile path, runtime profile id, service session id, browser id, tab ids,
  display name, CDP port, stream port, and route id when available.
- Liveness evidence: process alive, CDP reachable, service job active, profile
  lease active, target tab active, display process alive, and dashboard service
  membership.
- Classification: `active`, `retained`, `stale-candidate`, `gc-eligible`, or
  `protected`.
- Human-readable reason list and machine-readable issue codes.

## Implementation Slices

### Slice A | Resource Inventory And Correlation

Goal: make live OS resource pressure visible without terminating anything.

Tasks:

- Add a resource inventory collector under the native service runtime.
- Correlate process table entries with service state using process group, PID,
  profile path, runtime-state path, CDP port, display name, session id, and
  browser id.
- Add `agent-browser service resources --json`.
- Add no-launch unit tests with synthetic process tables and service-state
  fixtures.
- Keep command output sanitized: no cookies, auth state, page bodies, or raw
  target-site payloads.

Exit criteria:

- `service resources --json` reports the live dashboard service, active
  browsers, temporary profile browsers, Xvfb displays, and stale daemon
  siblings with stable issue codes.
- The command is read-only and safe to run repeatedly.

### Slice B | Stale Classification Policy

Goal: separate active, retained, protected, and reclaimable resources.

Tasks:

- Define conservative stale predicates for each resource kind.
- Treat systemd `agent-browser-dashboard.service` MainPID as protected unless
  the operator explicitly targets service restart.
- Treat named managed runtime profiles as protected even when their runtime
  state is stale.
- Classify temporary smoke and plan profile resources as candidates only after
  age and ownership checks pass.
- Add explicit stale runtime-state diagnosis when recorded browser PID or
  DevTools URL is no longer alive.

Exit criteria:

- A stale temp browser process group is classified as `gc-eligible` only when
  it is old, not tied to an active job, and not reachable through current
  service state.
- A live named profile browser is classified as `protected`.
- A stale runtime-state pointer is reported as metadata repair, not as a
  reason to delete the profile.

### Slice C | Dry-Run Garbage Collector

Goal: provide an operator-reviewable cleanup plan.

Tasks:

- Add `agent-browser service gc --dry-run --json`.
- Group candidates by action:
  - terminate stale browser process group
  - terminate orphaned Xvfb display
  - terminate stale `agent-browser` daemon sibling
  - clear stale runtime-state pointer
  - retain protected record with reason
- Include aggregate projected CPU and RSS relief when available.
- Return nonzero only for command failure, not for candidate presence.

Exit criteria:

- Dry-run shows exact PIDs, process groups, resource ids, actions, and reasons.
- Dry-run never mutates process state or service records.
- Tests prove active dashboard service PID is never included by default.

### Slice D | Apply Mode With Guardrails

Goal: reclaim clearly stale resources after dry-run review.

Tasks:

- Add `agent-browser service gc --apply --json`.
- Require a recent matching dry-run token or explicit `--force-without-review`
  for apply.
- Send SIGTERM first, wait briefly, then SIGKILL only for remaining processes
  that still match the same candidate identity.
- Re-read process identity before kill to avoid PID reuse mistakes.
- Persist a compact service event or incident summary with counts and issue
  codes, not raw command output.

Exit criteria:

- Apply terminates only the reviewed stale process groups.
- Apply refuses to kill a resource whose identity changed after dry-run.
- Apply reports killed, skipped, protected, and failed counts.

### Slice E | Dashboard And Doctor Surfaces

Goal: show resource pressure before it becomes an operator surprise.

Tasks:

- Add a dashboard Service warning when stale candidates or high resource usage
  are present.
- Add resource summary rows to selected workspace diagnostics where a selected
  browser, profile, stream, or daemon has stale resource evidence.
- Extend `agent-browser install doctor` or `agent-browser doctor remote-view`
  only for readiness-impacting stale resources, such as orphaned route displays
  or stale remote-view browser process groups.
- Keep cleanup controls explicit and review-gated.

Exit criteria:

- Dashboard shows stale resource candidate count, estimated RSS, and top issue
  codes.
- The dashboard does not offer one-click destructive cleanup without a visible
  dry-run result.

### Slice F | Optional Monitor Timer

Goal: allow continuous visibility without unattended destructive cleanup.

Tasks:

- Add an optional user-scoped monitor timer that runs read-only inventory and
  writes compact aggregate status.
- Do not enable the timer by default.
- Do not add unattended apply mode in this plan.
- Document safe thresholds and operator override points.

Exit criteria:

- The timer can be installed and removed explicitly.
- Timer output is aggregate-only and does not store private page data.
- Dashboard or CLI can read the latest monitor summary.

### Slice G | Validation And Live Proof

Goal: prove this prevents a repeat of the 2026-06-04 stale-resource incident.

Tasks:

- Add fixture tests for resource classification and GC candidate generation.
- Add a no-launch integration smoke with fake process-table data.
- Add a live guarded smoke that launches a temporary browser, proves it is
  discoverable, stops its owner, then proves dry-run identifies and apply
  removes only that disposable resource.
- Re-run dashboard and service-client contract checks for changed surfaces.

Exit criteria:

- Focused Rust and dashboard tests pass.
- Live disposable GC smoke passes without touching the dashboard service or
  named profiles.
- `agent-browser service resources --json` after apply reports no disposable
  stale leftovers from the smoke.

## Validation Plan

Minimum local validation for implementation slices:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml resource -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml gc -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:service-client-contract
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-selected-workspace-context
pnpm build:dashboard
git diff --check
```

Live validation before enabling any monitor timer:

```bash
agent-browser service resources --json
agent-browser service gc --dry-run --json
pnpm test:agent-browser-resource-gc-live
agent-browser service resources --json
```

## Next Step

Start with Slice A and Slice B together. The first code change should be a
read-only `service resources --json` surface plus fixture-backed stale
classification. Do not implement apply mode until dry-run output is accurate
against both synthetic fixtures and this workstation's current live state.
