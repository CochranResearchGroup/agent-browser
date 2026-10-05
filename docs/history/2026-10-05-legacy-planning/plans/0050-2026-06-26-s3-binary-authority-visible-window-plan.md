# P50 S3 Binary Authority And Visible Window Plan

Date: 2026-06-26
State: COMPLETE AFTER P52 CLEARANCE
Lane: P50
Depends On:
- `docs/dev/plans/0046-2026-06-24-remote-view-stress-hardening-plan.md`
- `docs/dev/plans/0049-2026-06-26-p46-s3-remediation-plan.md`
- `/tmp/agent-browser-p46-s3-2026-06-26T21-11-13-480Z`

## Purpose

Repair the P49 execution defect before any full S3 retry. The next proof must
show which `agent-browser` binary is under test, exercise the freshly built
repo binary or an explicitly installed candidate, and isolate route-bound
`remote-view open` visible-window proof before dashboard tabs or multi-operator
UX are involved.

## Non-Negotiable Rules

- Do not run full P46 S3 until the narrow S3 open proof passes.
- Live remediation retries must record the exact command path, realpath,
  version, and whether the command was explicit.
- Live remediation retries must also record the daemon listener process
  realpath for the default socket before live browser work.
- Do not rely on the ambient `agent-browser` on `PATH` for remediation retries.
- Do not accept an explicit CLI path as proof of the action-handler binary when
  a long-lived daemon is already serving the socket.
- If `remote-view open` fails, capture display content, route-pool readiness,
  service status, incident summary, and reset state before any follow-on
  browser action.
- If the narrow open proof fails twice in this plan, lock P50 and keep P46
  locked for maintainer planning.

## Goal 1: Command Authority

`/goal execute P50 goal 1: make live stress remediations require and record an explicit agent-browser command`

Work:

- Add `--agent-browser-command <path>` to the P46 stress runner.
- Add `--require-explicit-agent-browser-command` so remediation runs can fail
  before live work when the command is implicit.
- Add `--require-agent-browser-daemon-command-match` so remediation runs can
  fail before live work when the daemon listener is missing, duplicated, or
  not running the explicit command realpath.
- Write `agent-browser-command.json` into every artifact directory with command
  source, resolved command, realpath when available, daemon listener metadata,
  and `--version` output.
- Add no-live coverage so command authority cannot regress silently.

Evidence:

- `node scripts/test-p47-scenario-harness.js`
- command metadata in the live artifact directory.

## Goal 2: Narrow S3 Open Proof

`/goal execute P50 goal 2: run a route-bound S3 open proof with the explicit rebuilt binary before any dashboard or tab stress`

Work:

- Add a narrow runner mode that performs only baseline capture, route-pool
  selection, `remote-view open`, failed-open diagnostics if needed, and
  reset-after.
- Build the repo binary with `cargo build --manifest-path cli/Cargo.toml`.
- Run the narrow proof with `--agent-browser-command ./cli/target/debug/agent-browser`
  and `--require-explicit-agent-browser-command`.

Pass criteria:

- command metadata proves the repo binary was used;
- `remote-view open` reports `operatorVisible.state=ready`;
- visible-window proof reports `browser_window_visible`;
- reset-after returns to zero sessions, zero browsers, zero tabs, and zero
  active incidents.

Failure criteria:

- If the explicit repo binary still fails with `non_browser_windows`, classify
  the blocker as route display/browser-window realization, not dashboard or tab
  selection.
- If command metadata is missing or points to the wrong binary, classify the
  blocker as execution-lane authority.

## Goal 3: Lock Or Reopen

`/goal execute P50 goal 3: record whether P46 can reopen for full S3 or must remain locked`

Work:

- If narrow proof passes, update P46 and P49 to say full S3 may be retried from
  the explicit-binary lane.
- If narrow proof fails, update P46, P49, and this plan with artifact paths and
  keep P46 locked.
- Verify final runtime cleanup with a compact service-status check.

Closeout:

- P50 is complete when command authority is implemented, the narrow proof has
  been executed once with the explicit repo binary, and the repo plans record
  the resulting unlock or lock decision with artifact evidence.

## Execution Log

### 2026-06-26

Implemented:

- `scripts/lib/p46-scenario-harness.js` includes `s3-open` as a narrow
  route-bound open proof scenario.
- `scripts/run-p46-stress-scenario.js` supports
  `--agent-browser-command <path>`, `--require-explicit-agent-browser-command`,
  command metadata artifacts, and `s3-open` capture/evaluation.
- After the first live proof exposed that CLI command authority is not enough,
  the runner was extended with daemon listener metadata and
  `--require-agent-browser-daemon-command-match`.

Validation passed:

- `node scripts/test-p47-scenario-harness.js`
- explicit-command dry gate:
  `node scripts/run-p46-stress-scenario.js --scenario s3-open --require-explicit-agent-browser-command --artifact-dir <tmp>`
- `cargo build --manifest-path cli/Cargo.toml`
- daemon-command dry gate:
  `/tmp/agent-browser-p50-daemon-gate-BmulZ7`

Narrow proof attempt:

- Command:
  `node scripts/run-p46-stress-scenario.js --scenario s3-open --reset-before --reset-after --agent-browser-command ./cli/target/debug/agent-browser --require-explicit-agent-browser-command`
- Artifact: `/tmp/agent-browser-p46-s3-open-2026-06-26T23-40-14-493Z`
- Result: failed before dashboard or tab stress.
- Command metadata proved the explicit CLI realpath was
  `/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser`.
- `remote-view open` failed with `Operation timed out. The page may still be
  loading or the element may not exist.`
- Display evidence still showed route displays in `non_browser_windows`, with
  only `Openbox` visible.
- Service events showed the open path launched `session:default`, created a
  tab for `https://example.com/?p50=s3-open`, then faulted during cleanup.
- Runtime cleanup after explicit incident resolution returned to zero sessions,
  zero browsers, zero tabs, zero active incidents, and both route-pool entries
  available.

Daemon-authority discovery:

- Socket inspection found multiple listeners for
  `/run/user/1000/agent-browser/default.sock`.
- The daemon dry gate found nine listener processes under
  `/run/user/1000/agent-browser`, with only one matching
  `/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser`.
- At least one listener was `/home/ecochran76/.local/bin/agent-browser`; other
  listeners were repo debug binaries, several with deleted executable inodes.
- This means P50 proved that explicit CLI command authority is insufficient.
  The next plan must establish a single authoritative daemon listener before
  another live browser proof.

Lock decision:

- P50 remains locked for maintainer planning. Do not rerun `s3-open` or full
  S3 until daemon listener ownership is made singular and source-backed. The
  next plan should repair daemon lifecycle authority, then rerun `s3-open`
  with both explicit CLI and daemon-command-match gates enabled.

### 2026-06-27 P52 Superseding Clearance

P52 executed the follow-up daemon lifecycle and route-binding repair that P50
required.

Artifacts:

- Plan: `docs/dev/plans/0052-2026-06-27-s3-clearance-pathway-plan.md`
- Passing `s3-open`:
  `/tmp/agent-browser-p46-s3-open-2026-06-27T16-36-31-141Z`
- Passing full S3:
  `/tmp/agent-browser-p46-s3-2026-06-27T16-37-24-659Z`

Outcome:

- The default daemon listener was made singular and source-backed for the
  explicit rebuilt repo binary.
- The narrow S3 open proof passed with daemon-command-match enforcement.
- Full S3 passed after the narrow proof.
- P50 is no longer the active S3 lock. P46 may advance to S4 from the explicit
  rebuilt-binary lane.
