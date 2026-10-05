# Live Dashboard Runtime Publish Plan

Date: 2026-05-30
State: CLOSED
Lane: P11
Outcome: PASSED

## Purpose

Make dashboard and runtime-facing changes available immediately on the same
local surface the operator checks, without relying on a temporary Next dev
server or a repo-local debug binary.

The current failure mode is that `pnpm build:dashboard` updates
`packages/dashboard/out`, but the user-scoped dashboard service serves assets
embedded into the already-running `~/.local/bin/agent-browser` process. A code
slice can pass source tests and rendered QA while `http://127.0.0.1:4848`
continues to serve the old binary until the CLI is rebuilt, installed, and the
user service is restarted.

## Non-Goals

- Do not turn ordinary development validation into a formal release.
- Do not publish to GitHub Releases, npm, Homebrew, or upstream channels.
- Do not restart unrelated browser sessions or service-owned browser workers.
- Do not delete operator state, dashboard auth, runtime profiles, or retained
  service history.
- Do not make live install mandatory for every test-only source edit.

## Desired Operator Contract

Add one repo command for local operator-visible publication, tentatively:

```bash
pnpm publish:local-dashboard
```

The command should:

1. Build the dashboard export with `pnpm build:dashboard`.
2. Rebuild the local CLI that embeds `packages/dashboard/out`.
3. Back up the current `~/.local/bin/agent-browser` to a timestamped file.
4. Replace `~/.local/bin/agent-browser` with the rebuilt binary.
5. Restart `agent-browser-dashboard.service` when that user service exists.
6. Verify `http://127.0.0.1:4848/` serves the new bundle.
7. Run a small browser or HTTP smoke against port `4848`, not a dev server.
8. Print a compact before/after summary with binary path, backup path, service
   PID, dashboard URL, and smoke result.

## Slices

### Slice A | Publish Script

Create a checked-in script, for example
`scripts/publish-local-dashboard-runtime.sh`, and wire it through
`package.json`.

Behavior:

- Use `pnpm`, not `npm` or `yarn`.
- Fail fast on build, compile, copy, restart, or smoke failures.
- Detect the installed command path with `command -v agent-browser`, but default
  to `~/.local/bin/agent-browser` for this repo's user-scoped install.
- Require the target path to be user-writable and under the current user's home
  unless an explicit reviewed override is passed.
- Write a timestamped backup before replacement.
- Preserve executable mode on the installed binary.
- Restart only `agent-browser-dashboard.service` through `systemctl --user`.
- If the service is not installed, start `agent-browser dashboard start` only
  when the caller passes an explicit option such as `--start-if-missing`.
- Emit machine-readable `--json` output for future automation.

Exit criteria:

- Running the script updates the installed binary timestamp and preserves a
  backup path.
- The user service restarts with a fresh PID.
- A failed build or smoke leaves the prior binary in place.
- A failed post-copy service restart reports the backup and restore command.

### Slice B | Live Smoke And Cache Proof

Add a focused smoke script that proves the served runtime is current.

Checks:

- `curl http://127.0.0.1:4848/` returns dashboard HTML.
- Served JS chunks include a caller-supplied marker string when provided, for
  example `--expect-marker WorkspaceSelectionPanel` or `--expect-marker "Stream
  port"`.
- Dashboard auth is handled through the existing user-scoped auth file without
  printing secrets.
- A browser smoke can open the external dashboard URL, select or restore a
  workspace route, and report viewport/readiness/detail facts as JSON.
- The smoke distinguishes "source built", "binary installed", "service
  restarted", and "browser sees the change" as separate evidence.

Exit criteria:

- The smoke can be run independently after the publish script.
- The publish script runs the smoke by default.
- Failure copy tells the operator whether to hard refresh, restart the service,
  rebuild the binary, or restore the backup.

### Slice C | Documentation And Closeout Discipline

Document when agents must use the local publish command before telling the user
to check the dashboard.

Update:

- `README.md` dashboard development section.
- `docs/src/app/dashboard/page.mdx`.
- `skills/agent-browser/SKILL.md`.
- Any developer validation notes or selector guidance if a dashboard source
  change should recommend the publish command.

Required closeout wording for future dashboard-runtime work:

- State whether validation used a temporary dev server or the live dashboard
  service.
- If the user is expected to inspect the change externally, include the publish
  command result and live dashboard PID.
- If live publication was intentionally skipped, say that the external dashboard
  is not updated yet.

Exit criteria:

- Docs explain the repo source, embedded binary, and running systemd service
  boundary.
- The validation selector or closeout checklist points dashboard UI changes at
  `pnpm publish:local-dashboard` when operator-visible live QA is required.

## Validation Matrix

Source checks:

- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- `pnpm build:dashboard`
- `cargo build --manifest-path cli/Cargo.toml`

Publish checks:

- `pnpm publish:local-dashboard -- --expect-marker <marker>`
- `systemctl --user status agent-browser-dashboard.service --no-pager`
- HTTP bundle marker check against `http://127.0.0.1:4848/`
- Browser smoke against `http://127.0.0.1:4848/`, not `localhost:3128` or any
  temporary dev port

Closeout evidence:

- Installed binary path and timestamp.
- Backup binary path.
- Dashboard service PID and start time.
- Exact live URL tested.
- Marker or DOM evidence proving the new code is served.

## Open Questions

- Resolved: the command defaults to the debug build for fast local operator QA
  and supports `--release` when a release-like local binary is required.
- Resolved: the script restores the backup automatically when install,
  restart, or smoke fails after the backup is created.
- Resolved: publishing remains a separate development command instead of
  expanding `scripts/install-dashboard-user-service.sh`.

## Result

Implemented `pnpm publish:local-dashboard` through
`scripts/publish-local-dashboard-runtime.js`.

The publish command now:

- Runs `pnpm build:dashboard`.
- Runs `cargo build --manifest-path cli/Cargo.toml` by default, or release mode
  with `--release`.
- Backs up the installed user-scoped binary to
  `~/.local/bin/agent-browser.pre-local-dashboard-<timestamp>`.
- Installs the rebuilt binary through a staged copy plus atomic rename so Linux
  does not fail with `ETXTBSY` while the previous binary is executing.
- Restarts only `agent-browser-dashboard.service` when that user service is
  installed.
- Runs `scripts/smoke-local-dashboard-runtime.js` against
  `http://127.0.0.1:4848/`.
- Emits structured JSON with the installed binary path, backup path, service
  before/after PID, served bundle marker evidence, and browser smoke evidence.
- Restores the backup and restarts the service if a post-backup step fails.

Implemented `pnpm smoke:local-dashboard-runtime` through
`scripts/smoke-local-dashboard-runtime.js`.

The smoke command now:

- Fetches the live dashboard HTML.
- Reads served `_next/static` JavaScript chunks.
- Verifies one or more `--expect-marker` strings in served HTML or chunks.
- Authenticates the dashboard browser smoke with the user-scoped
  `~/.agent-browser/dashboard-auth.env` without printing secrets.
- Opens `http://127.0.0.1:4848/` through `agent-browser`, verifies dashboard
  chrome, and optionally verifies a daemon workspace route when
  `--workspace-session` is supplied.

Documentation and validation routing were updated:

- `README.md` documents the source export, embedded binary, and running systemd
  service boundary.
- `docs/src/app/dashboard/page.mdx` includes a Local Runtime Publish section.
- `skills/agent-browser/SKILL.md` and the installed skill copy mention
  `pnpm publish:local-dashboard` before operator-visible dashboard checks.
- `scripts/dev/select-validation.js` recommends
  `pnpm publish:local-dashboard -- --expect-marker <changed-ui-marker>` for
  operator-visible dashboard QA.

Validation evidence from implementation:

- `node --check scripts/smoke-local-dashboard-runtime.js` passed.
- `node --check scripts/publish-local-dashboard-runtime.js` passed.
- `pnpm smoke:local-dashboard-runtime -- --expect-marker "Stream port"
  --skip-browser --json` passed against `http://127.0.0.1:4848/`.
- `pnpm test:dashboard-workspace-navigator` passed.
- `pnpm test:dashboard-view-streams` passed.
- `pnpm test:dashboard-workspace-nodes` passed.
- `pnpm validation:select -- --base HEAD --json` includes the new publish
  recommendation.
- `diff -q skills/agent-browser/SKILL.md
  /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md` passed.
- `git diff --check` passed.
- `pnpm publish:local-dashboard -- --expect-marker "Stream port" --json`
  passed. It backed up the installed binary to
  `/home/ecochran76/.local/bin/agent-browser.pre-local-dashboard-20260530230345`,
  installed `/home/ecochran76/workspace.local/agent-browser/cli/target/debug/agent-browser`,
  restarted `agent-browser-dashboard.service` from PID `48915` to PID `98244`,
  verified the served bundle marker `Stream port`, and completed a browser
  smoke against `http://127.0.0.1:4848/`.
- `systemctl --user status agent-browser-dashboard.service --no-pager` showed
  PID `98244` active since `Sat 2026-05-30 18:03:46 CDT`.
- `stat` showed `/home/ecochran76/.local/bin/agent-browser` updated at
  `2026-05-30 18:03:46.526774204 -0500` with a timestamped backup present.
- `pnpm --dir docs build` passed.
- A deliberate negative publish with marker
  `__definitely_missing_plan_0011_marker__` failed as expected, reported
  `restoredBackup: true`, and restarted `agent-browser-dashboard.service` after
  restoring the backup. The service moved from PID `98244` to PID `31039`.
- A final positive `pnpm smoke:local-dashboard-runtime -- --expect-marker
  "Stream port" --json` passed after the rollback test. It verified
  `http://127.0.0.1:4848/`, found `Stream port` in a served JavaScript chunk,
  authenticated the browser smoke, and saw dashboard chrome plus the Workspace
  tab.
- Final `systemctl --user status agent-browser-dashboard.service --no-pager`
  showed PID `31039` active since `Sat 2026-05-30 18:06:13 CDT`.
- Final `stat` showed `/home/ecochran76/.local/bin/agent-browser` updated at
  `2026-05-30 18:06:13.058083012 -0500` with rollback backup
  `/home/ecochran76/.local/bin/agent-browser.pre-local-dashboard-20260530230611`
  present.
