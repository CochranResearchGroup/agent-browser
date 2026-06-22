# Runtime Convergence Plan

Date: 2026-06-22
State: CLOSED
Lane: P42
Depends On:
- `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`
- `docs/dev/plans/0040-2026-06-21-dashboard-binary-harmonization-plan.md`
- `docs/dev/plans/0041-2026-06-22-foreign-cdp-browser-discovery-and-control-plan.md`

## Purpose

Make every live agent-browser control surface converge on one explicit runtime
identity instead of relying on operator memory, manual binary replacement, or a
single dashboard manifest.

The immediate failure class is that the operator can see a dashboard, daemon
session, Guacamole route, or left-rail row that looks current while some other
runtime piece is stale, manually repaired, or owned by a different process. P40
made the dashboard service identify the embedded UI and executable it serves.
That is necessary, but it is not sufficient: daemon sessions, service status,
route helpers, retained workspace records, and external browser discovery all
need a common convergence contract.

## Current Evidence

- P40 added `/api/runtime/manifest`, dashboard bundle SHA-256, executable
  SHA-256, service contract version, and dashboard drift warnings.
- P40 publish validation restarts `agent-browser-dashboard.service` and proves
  that port `4848` serves the just-installed dashboard-embedded binary.
- `ensure_daemon()` restarts a session when daemon version metadata or auth
  metadata is missing or mismatched, but it does not compare executable
  SHA-256.
- `agent-browser install doctor --json` compares the current executable,
  command on `PATH`, pnpm package binary, workspace binary, service probe, and
  dashboard runtime manifest. It does not report a complete inventory of every
  active daemon session and stream process.
- Remote-view recovery required manual Guacamole schema import, route-pool
  repair, display grants, local binary replacement, and daemon restart before
  the live path became ready. The Postgres/schema repair is now being moved
  into bootstrap, but the same convergence gap remains for binary/runtime
  identity.
- The dashboard left rail previously showed stale or retained records as live
  attention rows even when no operator action existed. P41 separately scopes
  live non-owned CDP browser discovery so foreign addressable browsers are not
  mixed with agent-browser-owned rows.

## Vocabulary

- **Expected runtime identity**: the executable SHA-256, package version,
  service UI contract version, dashboard bundle SHA-256, and supported feature
  set that the installed user-scoped runtime should expose.
- **Active runtime inventory**: every currently running agent-browser process
  or daemon session that can serve commands, dashboard APIs, streams, or
  service state.
- **Converged runtime**: an active runtime whose executable identity and
  contract match the expected user-scoped runtime identity.
- **Stale runtime**: an active runtime that is reachable but was started by a
  different executable, missing metadata, stale contract, or an unsupported
  feature set.
- **Diagnostic retained record**: historical service state, incidents, jobs,
  old stream metadata, or failed browser evidence that is useful for logs and
  trace views but must not appear as a live control target.

## Operating Invariants

- A dashboard manifest proves only the dashboard service identity. It does not
  prove that existing daemon sessions, stream backends, or retained browser
  rows are current.
- Every live control target must carry enough runtime identity to explain which
  executable and contract produced it.
- A version match is not a sufficient convergence proof. Executable SHA-256 is
  the primary local identity check.
- Stale diagnostic records belong in Service, trace, event, job, incident, and
  log viewers, not the live workspace left rail.
- Repairable drift should produce a runnable remedy command.
- Non-owned but addressable browsers must remain distinct from agent-browser
  owned runtimes.
- Bootstrap commands must be idempotent and explicit about which changes are
  detect-only, repairable, or manual-review-required.

## Target Runtime Contract

Each active daemon or dashboard process should expose:

```json
{
  "schemaVersion": "agent-browser.runtime-identity.v1",
  "process": {
    "pid": 1234,
    "startedAt": "2026-06-22T00:00:00Z"
  },
  "executable": {
    "path": "/home/ecochran76/.local/bin/agent-browser",
    "sha256": "<sha256>"
  },
  "packageVersion": "0.27.0",
  "serviceContractVersion": "service-ui-runtime.v1",
  "dashboard": {
    "sha256": "<sha256-or-null>"
  },
  "features": [
    "workspace.detectedBrowsers",
    "workspace.noRetainedLiveRail"
  ],
  "owner": {
    "kind": "dashboard_service | daemon_session | service_worker | stream_backend",
    "session": "default"
  }
}
```

The exact JSON can evolve during implementation, but it must preserve these
semantics: executable identity, contract identity, process identity, owner kind,
and feature support.

## Slices

### Slice A: Active Runtime Inventory

Slice progress: done on 2026-06-22.

Add a no-launch inventory that reports every active agent-browser runtime that
can affect the operator control surface.

Deliverables:

- Extend daemon metadata to include executable path and SHA-256 when a daemon
  starts.
- Add a read-only inventory builder that reads daemon socket metadata, dashboard
  runtime manifest, service process evidence, stream ports, and relevant PID
  files without launching Chrome.
- Include inventory rows in `agent-browser install doctor --json`.
- Include inventory rows in `agent-browser doctor remote-view --json`.

Acceptance:

- A live stale daemon with the same package version but different SHA-256 is
  reported as stale.
- A dashboard service with the current manifest but a stale daemon session is
  reported as partially converged, not ready by omission.
- Inventory rows do not expose secrets, auth state, cookies, page contents, or
  browser profile data.

Completed on 2026-06-22:

- `agent-browser install doctor --json` now includes
  `runtimeInventory.schemaVersion=agent-browser.runtime-inventory.v1`.
- The inventory scans the user-scoped daemon metadata directory without
  launching Chrome and reports daemon session PID, PID liveness, package
  version match, executable SHA-256 match, stream port, and metadata presence.
- Active daemon sessions with stale or incomplete executable metadata add
  `active_runtime_stale_executable` install doctor issues.
- `agent-browser doctor remote-view --json` lifts the install doctor's
  `runtimeInventory` to a top-level `runtimeInventory` field.
- Live debug-binary readback reported `runtimeInventory.status=stale`,
  `runtimeCount=4`, and `staleCount=4`, proving the inventory catches active
  runtimes that do not match the invoking executable identity.

### Slice B: Daemon SHA Convergence

Slice progress: done on 2026-06-22.

Make daemon reuse compare executable identity, not only package version.

Deliverables:

- Persist daemon executable SHA-256 next to existing daemon version metadata.
- Update `ensure_daemon()` to restart a reachable daemon when its executable
  SHA-256 differs from the invoking executable SHA-256.
- Add a compatibility path for old metadata: missing SHA is stale unless an
  explicit environment override allows legacy reuse.
- Add focused unit tests for same-version SHA drift and missing-SHA drift.

Acceptance:

- Replacing `~/.local/bin/agent-browser` and then invoking the same session
  restarts the old daemon before command dispatch.
- Version-only matches no longer mask stale executable drift.
- The restart path preserves the existing auth-token safety model.

Completed on 2026-06-22:

- Daemons now write `<session>.sha256` next to existing PID, version, token,
  and stream metadata.
- Client daemon reuse compares the invoking executable SHA-256 against the
  daemon metadata when the invoking executable can be hashed.
- Missing daemon SHA metadata is treated as stale by default. Set
  `AGENT_BROWSER_ALLOW_LEGACY_DAEMON_SHA_REUSE=1` only for a reviewed legacy
  compatibility escape hatch.
- Stale cleanup removes the SHA metadata file with the rest of the daemon
  session metadata.
- Focused tests cover matching SHA, mismatched SHA, missing SHA, explicit
  legacy reuse, and cleanup.

### Slice C: Convergence Doctor And Remedy

Slice progress: done on 2026-06-22.

Turn runtime drift into an actionable doctor surface.

Deliverables:

- Add stable issue codes for stale daemon SHA, missing daemon SHA, stale
  dashboard runtime, stale stream backend, and diagnostic retained rows in the
  live rail.
- Add a runnable next command for repairable drift.
- Add a convergence summary to install doctor and remote-view doctor:
  `converged`, `partial`, `stale`, or `manual_review_required`.
- Add text output that names the stale runtime owner and session.

Acceptance:

- Doctor output explains exactly which session or service is stale.
- The next action is runnable and bounded.
- Manual-review-required states are not auto-repaired.

Completed on 2026-06-22:

- Active stale daemon issues now include the affected `session`,
  `nextAction=restart_stale_daemon_session`, and an argv-safe operator remedy:
  `["agent-browser", "close", "--session", "<session>"]`.
- `agent-browser doctor remote-view --json` now promotes install drift caused
  by `active_runtime_stale_executable` to
  `nextAction=restart_stale_daemon_sessions_then_rerun_doctor` instead of
  collapsing it into generic install drift.
- The remote-view next command intentionally points back to install doctor and
  tells the operator to use each issue's `remedy.argv`, so the remedy remains
  session-scoped rather than blindly closing all active sessions.
- `close --session <name>` now targets an existing daemon before daemon
  prestart, force-cleans explicitly requested stale metadata when the daemon is
  unauthorized or not ready, and returns a successful bounded close result.
- Install doctor service status now runs as a local pre-daemon action, uses a
  unique owned probe session, terminates the probe daemon after reading status,
  and treats the isolated no-state probe as no-launch ready.
- Runtime inventory now classifies running PID metadata without an addressable
  socket, stream, or port as `diagnostic` rather than stale live runtime.
- Install doctor now probes the local dashboard service's
  `/api/runtime/manifest` endpoint without requiring the dashboard to be
  running. If a running local dashboard fails to serve a readable manifest or
  serves an executable SHA-256 that does not match the current executable, it
  emits `dashboard_runtime_stale_or_unreadable` with the bounded remedy
  `pnpm converge:local-runtime -- --apply --json`.
- Remote-view doctor promotes
  `dashboard_runtime_stale_or_unreadable` to
  `nextAction=converge_local_runtime_then_rerun_doctor` before generic install
  drift.
- `pnpm converge:local-runtime -- --apply --json` now treats nonzero initial
  doctor JSON as repairable input instead of aborting before publish. Dry-run
  remains strict.
- Live validation started from `dashboard_runtime_stale_or_unreadable`, ran the
  convergence apply command, and ended with install doctor `success=true`, no
  issue codes, `liveDashboardRuntime.ready=true`, and
  `runtimeInventory.status=none`.
- Install doctor now emits `runtimeConvergence` with schema
  `agent-browser.runtime-convergence.v1` and status values `converged`,
  `partial`, `stale`, or `manual_review_required`, derived from runtime
  inventory plus live dashboard manifest state.
- Remote-view doctor lifts install doctor's `runtimeConvergence` summary and
  text output prints the summary status separately from raw runtime inventory.
- Live validation after publishing the summary-state build reported install
  doctor `success=true`, no issue codes,
  `runtimeConvergence.status=converged`, `liveDashboardRuntime.state=ready`,
  and `runtimeInventory.status=none`.
- Runtime inventory now probes advertised stream ports. A live daemon with
  stale or unreachable stream metadata is classified stale with
  `driftReasons=["stream_unreachable"]` or
  `driftReasons=["stream_metadata_invalid"]`.
- Install doctor emits `active_runtime_stale_stream_backend` with the same
  bounded `agent-browser close --session <session>` remedy used for stale
  daemon executable drift.
- Remote-view doctor treats `active_runtime_stale_stream_backend` as a
  session-scoped daemon restart prerequisite before generic install drift.
- Live validation after publishing the stream-backend build reported install
  doctor `success=true`, no issue codes,
  `runtimeConvergence.status=converged`, `staleRuntimeCount=0`, and
  `runtimeInventory.status=none`.

### Slice D: Idempotent Remote-View Bootstrap

Slice progress: done on 2026-06-22.

Move the manual Guacamole and Postgres repair into durable bootstrap.

Deliverables:

- Add `pnpm ensure:rdp-guac-postgres -- --apply`.
- Route Guacamole route-pool setup, existing-user sync, and legacy autologin
  setup through the shared schema guard before writing Guacamole records.
- Harden the live Guacamole Postgres compose for WSL hard stops by keeping
  durable Postgres settings explicit.
- Refuse automatic schema import over partial `guacamole_*` state.

Acceptance:

- An empty initialized Guacamole database is repaired by the bootstrap guard.
- A partial schema fails loudly with manual recovery guidance.
- Route-pool readiness reports the repair command for schema drift.
- `agent-browser doctor remote-view --json` remains ready after applying the
  compose change.

Completed on 2026-06-22:

- `pnpm ensure:rdp-guac-postgres -- --apply` exists and is invoked by the
  local convergence command.
- `scripts/setup-rdp-guac-route-pool.sh`,
  `scripts/sync-rdp-guac-existing-user-route-pool.sh`, and
  `scripts/setup-rdp-autologin-user.sh` route Guacamole writes through the
  shared schema guard before mutating route or login records.
- The schema guard starts Postgres when needed, waits for `pg_isready`, imports
  the Guacamole schema only when the required `guacamole_*` relations are
  absent, refuses partial `guacamole_*` state with manual recovery guidance,
  and checkpoints after ready/imported states.
- The live Guacamole compose file keeps Postgres durability settings explicit:
  `fsync=on`, `synchronous_commit=on`, `full_page_writes=on`,
  `checkpoint_timeout=5min`, and `max_wal_size=1GB`.
- `bash scripts/ensure-rdp-guac-postgres.sh --dry-run` reported
  `Guacamole Postgres schema is ready.`
- `pnpm --silent test:rdp-guac-route-pool-readiness -- --report-only`
  reported `success=true`; `guacamole_postgres`, `guacamole_schema`,
  `guacamole_web`, `guacamole_login`, `guacd`,
  `guacamole_rdp_connections`, `guacamole_connection_permissions`,
  `distinct_rdp_targets`, and both RDP backend TCP checks were ready.
- Direct installed `agent-browser doctor remote-view --json` reported
  `success=true`, `status=ready`, `remoteControl.ready=true`,
  `runtimeConvergence.status=converged`, `runtimeInventory.status=none`, and
  `nextAction=run_many_to_many_live_gate`.

### Slice E: Live Rail Convergence Boundary

Slice progress: done on 2026-06-22.

Make the left rail show only live actionable control targets.

Deliverables:

- Remove stale retained and no-action attention records from the live left rail.
- Keep diagnostic retained evidence in Service, trace, job, incident, event, and
  log views.
- Add a distinct `Detected non-owned browsers` group for P41 foreign CDP rows.
- Gate live controls on runtime convergence and ownership capability.

Acceptance:

- A stale retained browser record does not appear as a live left-rail target.
- A reachable non-owned CDP browser appears only in the non-owned group.
- A stale daemon session row shows a repair action or disabled controls with a
  convergence reason.

Completed on 2026-06-22:

- The dashboard workspace navigator filters the live left rail to active
  agent-browser-owned rows and detected non-owned browser rows.
- The live rail no longer renders `Needs attention`, `Retained browsers`,
  `Retained profiles`, `Sessions`, or `Jobs` as live control groups.
- Detected non-owned CDP browsers render under the distinct
  `Detected non-owned browsers` group.
- Contract tests cover stale retained browser omission, non-owned CDP grouping,
  disabled non-owned control action reasons, and removal of no-action
  attention records from the live rail.

### Slice F: One-Command Local Convergence

Slice progress: done on 2026-06-22.

Provide one bounded command or script for local operator repair.

Deliverables:

- Add a dry-run by default convergence command or package script.
- In apply mode, repair only safe local drift: dashboard service restart,
  daemon-session restart for stale agent-browser daemons, Guacamole schema
  ensure, route-pool readiness refresh, and route-display access grants.
- Refuse foreign process lifecycle changes and partial database repair.
- Emit retained evidence for every action taken.

Acceptance:

- Dry-run reports the same stale runtime inventory as doctor.
- Apply mode converges repairable local runtime drift without requiring the
  operator to remember the sequence.
- Apply mode does not kill foreign browsers or delete diagnostic evidence.

Completed on 2026-06-22:

- `pnpm publish:local-dashboard -- --skip-browser --json` now synchronizes the
  user-scoped install binary, ignored workspace package binary, and user pnpm
  package binary to the same freshly built executable SHA-256 by default.
- The publish report includes a `referenceBinaries` array with before and
  after SHA-256 evidence for every synchronized reference binary.
- The script keeps `--skip-reference-sync` for cases where the operator wants
  to restart the dashboard without changing reference binaries.
- Live execution synchronized
  `/home/ecochran76/.local/bin/agent-browser`,
  `bin/agent-browser-linux-x64`, and the user pnpm global package binary to
  `94d1d022b4f1315b2f3eb9ff08fdc3faa816d77960500c6b6854cab98161cfa8`.
- After applying the publish convergence step, installed `agent-browser install
  doctor --json` reported `success=true`, no issue codes, and
  `runtimeInventory.status=converged`.
- Installed `agent-browser doctor remote-view --json` reported
  `remoteControl.ready=true`, `runtimeInventory.status=converged`, and
  `nextAction=run_many_to_many_live_gate`.
- `pnpm --silent converge:local-runtime -- --json` now provides the dry-run
  local convergence report. It reads install doctor and remote-view doctor,
  reports runtime inventory, and enumerates safe stale-daemon remedies. Use
  `--silent` when consuming JSON through pnpm so pnpm does not prefix the
  output with a run banner.
- `pnpm --silent converge:local-runtime -- --apply --json` now provides the
  bounded apply path. It runs local dashboard publication, applies only
  `agent-browser close --session <name>` stale-daemon remedies reported by
  doctor, runs the Guacamole Postgres schema ensure, runs route-pool readiness,
  applies route display-access grants only when remote-view doctor reports
  `nextAction=grant_route_display_access`, and reruns doctors.
- The apply path refuses foreign process lifecycle changes by accepting only
  the exact session-scoped stale-daemon remedy argv shape.
- Apply mode writes retained evidence to
  `~/.agent-browser/convergence/local-runtime-latest.json` by default, or to a
  caller-provided `--evidence-path <path>`.
- Validated syntax with `node --check scripts/converge-local-runtime.js` and
  `node --check scripts/test-local-runtime-convergence.js`.
- Validated the contract smoke with `pnpm test:local-runtime-convergence`.
- Live dry-run validation reported `success=true`, final install doctor ready,
  final remote-view ready, zero safe stale remedies, and zero skipped remedies.
- Live apply validation with explicit evidence path
  `/tmp/agent-browser-converge-local-runtime-evidence.json` reported
  `success=true`, evidence exists with `success=true`, final install doctor
  ready, final remote-view ready, and zero skipped remedies.

## Validation Plan

Use the smallest gate that proves each touched surface, then widen for
cross-surface runtime work:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture
cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --nocapture
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
pnpm test:rdp-guac-route-pool-readiness -- --report-only
agent-browser install doctor --json
agent-browser doctor remote-view --json
```

For live publication or user-visible dashboard changes, also run:

```bash
pnpm publish:local-dashboard -- --skip-browser --json
```

For P41 foreign browser interaction, run the dedicated foreign-CDP live smoke
once implemented. Do not use AuraCall or im-receipts private page contents as
durable plan evidence.

## Closeout Requirements

- ROADMAP and RUNBOOK identify the active P42 lane and latest validation.
- Commits are structured by coherent slice.
- The branch is pushed after a coherent validation point.
- Completion requires runtime inventory, daemon SHA convergence, doctor remedy,
  live rail boundary, and one-command convergence to be implemented and
  validated. Completing only Slice D is not enough to close P42.
