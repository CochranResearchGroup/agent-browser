# Last30Days Stale Runtime PID Lock Handoff

Date: 2026-08-10

## Handoff Purpose

Repair agent-browser managed-runtime and Chromium profile-lock liveness so a
reused operating-system PID cannot make a stale authenticated profile appear
owned by a live browser.

This note is a cross-repository implementation handoff. It supplies current
evidence and acceptance boundaries. It does not authorize deleting the real
`last30days-facebook` profile, killing PID 63205, clearing browser storage,
starting a second profile lane, launching X, or consuming the downstream
Last30Days acceptance attempt.

## Startup And Authority

Before changing source:

1. Read `AGENTS.md` and the relevant policies under `docs/dev/policies/`, in
   particular policies 0004 through 0012.
2. Run `git status --short --branch`, inspect ancestry and remote tips, and
   preserve pre-existing work. At handoff creation, the repository is on local
   branch `architecture-deepening-20260809` at
   `c7e7302266ad8c07c88018b8ab2e4fcc75aba3d1`; untracked
   `scripts/architecture/actions-inventory/target/` belongs to existing work
   and must not be modified or committed by this repair. During handoff
   creation, concurrent uncommitted edits also appeared under
   `cli/src/native/action_runtime/`, `cli/src/native/actions/`,
   `cli/src/native/remote_view/`, and the `service_*` modules. They are not
   owned by this handoff. Re-read status and coordinate before touching an
   overlapping path.
3. Use CodeGraph before source exploration or edits. The index was healthy at
   handoff creation with 527 files, 18,031 nodes, and 61,705 edges.
4. Invoke the `graphiti-discovery` skill and query `agent_browser_main` if the
   runtime is healthy. At handoff creation, `graphiti-runtime doctor` reported
   `mcp_http: down` while FalkorDB and the local inspector were healthy. No
   repair was attempted and no memory result was used.
5. Open a bounded plan or attach this defect to an explicitly compatible active
   lane before implementation. Do not silently mix it into the current
   architecture-deepening slice.

## Downstream Failure

Last30Days Plan 0040 version 2/checkpoint P0040-C02 is the downstream authority:

- plan:
  `/home/ecochran76/workspace.local/last30days-skill/docs/dev/plans/0040-2026-08-10-x-agent-browser-boundary-recovery.md`;
- diagnosis commit: `2acc426e5ea87ea565ab169c27519fa29b2b8cca` on
  `CochranResearchGroup/last30days-skill` `main`;
- failed tick: `tick-e15b1ed57efbb0c618253ecd90429295`;
- failed provider attempt:
  `provider-attempt-69155789e4000dbd795a7fb1f586e006`;
- public result: transient `agent_browser_error`, zero browser operations, zero
  page signals, zero observed candidates, one request, and no retry.

The downstream plan remains open at `confirmed_external_repository_defect`.
Its one new X-only acceptance attempt is unconsumed and remains owned by the
Last30Days agent after this repair is validated and installed.

## Exact Current Reproducer

Installed runtime identity:

```text
agent-browser 0.28.0
binary=/home/ecochran76/.local/bin/agent-browser
sha256=17f393c716f63de5008a25045f1ead0a4377efb7936300c8e1bcce2247d5995b
profile=last30days-facebook
userDataDir=/home/ecochran76/.agent-browser/runtime-profiles/last30days-facebook/user-data
```

The no-launch runtime status currently reports:

```json
{
  "runtimeProfile": "last30days-facebook",
  "browserPid": 63205,
  "browserAlive": true,
  "devtoolsPort": 37539,
  "devtoolsReachable": false,
  "launchMode": "automation"
}
```

The retained state contradicts current process identity:

- `runtime-state.json` records historical browser PID 63205 and DevTools port
  37539;
- Chromium `SingletonLock` resolves to `cooper-63205`;
- `/proc/63205/exe` resolves to the Codex executable, not Chrome or Chromium;
- no TCP listener exists on port 37539.

Run this read-only predicate before implementation. The current defective build
prints `false` and exits 1:

```bash
set -o pipefail
agent-browser --json --runtime-profile last30days-facebook runtime status \
  | jq -e '(.browserAlive == false) or (.devtoolsReachable == true)'
```

Do not use a bare session command as a diagnostic. Existing note
`docs/dev/notes/0098-2026-08-08-last30days-sequential-social-liveness-investigation.md`
records that apparently read-only session commands can auto-launch a browser
when retained ownership has drifted.

## Disproved Causes

The Last30Days investigation already disproved these alternatives without a
browser launch:

- The installed Last30Days service PATH resolves the same
  `/home/ecochran76/.local/bin/agent-browser` binary.
- The exact X `service access-plan` arguments succeed under the installed
  service environment and select profile `last30days-facebook` with no manual
  action.
- Browser-capability preflight succeeds with the validated
  `stealthcdp_chromium` binding.
- The exact remote-view open command succeeds with `--dry-run`, selects an
  available RDP route, and requests no browser launch.

These checks do not reach the live profile-lock guard. They establish that the
remaining blocker is not PATH, access-plan parsing, build selection, or route
planning.

## Confirmed Owning Seams

Current CodeGraph source confirms the defect spans three agent-browser seams:

1. `cli/src/runtime_profile.rs:282`,
   `runtime_status_with_user_data_dir()`, sets `browser_alive` from
   `pid_is_running()` alone. The Unix helper at line 563 uses `kill(pid, 0)`.
2. `cli/src/native/cdp/chrome.rs:990`, `ensure_profile_not_in_use()`, rejects a
   `SingletonLock` whenever the embedded PID accepts signal 0, without proving
   that the process is the browser that created the lock.
3. `cli/src/native/cdp/chrome.rs:1199`, `cleanup_stale_profile_lock()`, removes
   singleton and DevTools files only when that same PID-only test says the PID
   is absent. PID reuse therefore prevents stale-lock recovery.

The current unit test
`test_runtime_status_marks_unreachable_devtools_port` deliberately writes the
test process PID with a dead DevTools port and expects `browser_alive=true`.
That expectation preserves the exact misclassification exposed here.

Plan 0077 already established the stronger product contract in
`docs/dev/plans/0077-2026-07-25-profile-discovery-and-manual-browser-launch-ux-plan.md`:
manual lifecycle must bind PID plus process start identity, and PID reuse must
not create a false owner. The generic `RuntimeState` at
`cli/src/runtime_profile.rs:24` still stores PID without process start identity,
so the profile-lock and automated-runtime path does not satisfy that completed
contract.

## Required Repair Properties

Implement the smallest coherent agent-browser-owned repair. The design may
reuse an existing process-observation abstraction or add a shared one, but it
must satisfy all of these properties:

1. A runtime record is live only when the observed process identity belongs to
   the browser instance recorded for that runtime profile. PID existence alone
   is insufficient.
2. A reachable, profile-consistent DevTools endpoint may support automated
   runtime identity, but unreachable DevTools must not convert an unrelated
   reused PID into a live browser.
3. Non-CDP manual browsers retain their existing supported lifecycle. Do not
   require DevTools as the only proof of identity.
4. Profile-lock cleanup may remove stale `SingletonLock`, `SingletonSocket`,
   `SingletonCookie`, and `DevToolsActivePort` only after proving the lock does
   not belong to the observed live browser. It must never kill or signal the
   unrelated process that reused the PID.
5. A genuine live Chrome or Chromium owner remains protected by the
   one-process-per-profile invariant and continues to produce the existing
   actionable lock diagnostic.
6. Legacy runtime-state records without the new identity evidence receive an
   explicit conservative compatibility rule. Do not silently treat every
   legacy live PID as the recorded browser, and do not blindly delete an
   ambiguous live lock.
7. Runtime status, launch-time lock rejection, stale-lock cleanup, manual
   lifecycle projection, and service ownership use one consistent process
   identity rule rather than diverging local heuristics.
8. No public command, schema, or documentation surface should change unless
   the selected repair genuinely adds user-visible behavior. If it does, obey
   the complete documentation matrix in `AGENTS.md`.

## Required Red And Green Tests

Add deterministic, isolated fixtures using temporary homes and profiles. Do not
touch `~/.agent-browser` from tests.

At minimum prove:

- a runtime-state record and singleton lock naming a live unrelated test
  process are classified stale or identity-mismatched, not browser-live;
- cleanup removes only the disposable stale profile artifacts and the unrelated
  process remains alive;
- runtime status does not report the reused PID as a live browser;
- a genuine matching live browser lock is still rejected and its artifacts are
  preserved;
- a genuine manual no-CDP runtime with matching process identity remains live;
- legacy state follows the documented conservative rule;
- profile-lock diagnostics do not mislabel a reused unrelated process as a
  matching runtime or service browser owner.

Existing focused tests near the affected seams include:

- `test_runtime_status_marks_unreachable_devtools_port`;
- `test_cleanup_stale_profile_lock_removes_stale_files`;
- `test_ensure_profile_not_in_use_rejects_live_lock`;
- `test_locked_profile_message_reports_runtime_and_service_owner`;
- `test_locked_profile_message_reports_unknown_owner_remedies`.

Observe the new PID-reuse fixture red before implementation. Record which old
expectations changed and why.

## Validation

On WSL, route every compiling Cargo command through the repository wrapper:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  test_runtime_status_marks_unreachable_devtools_port
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  test_cleanup_stale_profile_lock_removes_stale_files
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  test_ensure_profile_not_in_use_rejects_live_lock
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm validation:select -- --base <last-known-green-ref>
```

Also add a disposable no-launch or local temporary-profile regression that
proves PID reuse without mutating the real authenticated profile. If a live
browser smoke is necessary, use an isolated temporary profile and record its
cleanup. Do not use `last30days-facebook` for implementation validation.

Before installation, use an independent evaluator for the consequential
process-identity and deletion-safety judgment. Adjudicate findings under policy
0010 and keep review/rework bounded.

## Hard Stops

Stop and ask the operator before any action that would:

- delete, rename, repair, or clear the real `last30days-facebook` runtime
  profile or its browser storage;
- kill PID 63205 or any process identified as Codex;
- close a retained browser, tab, viewer, session, or route not created by an
  isolated repair smoke;
- run non-dry-run X navigation or acquisition;
- consume the downstream Last30Days X-only tick;
- mix the repair with the pre-existing architecture inventory build output;
- release, tag, open a pull request, or push to upstream.

## Completion And Return Handoff

The agent-browser slice is complete only when source and tests prove the shared
identity contract, the applicable full local gates pass, an exact candidate is
committed through the repository's integration model, and installed-runtime
validation is explicitly authorized and recorded.

Return these facts to the Last30Days agent:

- repair commit and branch;
- installed agent-browser version and binary SHA-256, if installation was
  authorized;
- focused and broad validation receipts;
- no-real-profile-mutation statement;
- current no-launch runtime-status result for `last30days-facebook`;
- whether stale lock artifacts remain and why;
- any remaining human gate.

The Last30Days agent then owns its separate Plan 0040 preflight and sole X-only
acceptance attempt. The agent-browser agent must not run that tick as part of
this repair.

## Suggested Skills

- `graphiti-discovery` for advisory `agent_browser_main` recall when the runtime
  is healthy;
- `diagnosing-bugs` for the deterministic red loop and one-prediction-at-a-time
  investigation;
- `tdd` for the PID-reuse, lock-safety, and legacy-state fixtures;
- `agent-browser` for service/profile lifecycle and no-launch operational
  boundaries;
- `repo-policy-selector` only to re-anchor the existing repo-local policy, not
  to replace mature policy files.
