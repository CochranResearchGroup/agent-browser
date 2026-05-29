# RDP Guac Slice B Source Checkpoint

Date: 2026-05-26
State: VALIDATED
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

Slice B source work now has a service-owned viewer takeover path for RDP and
Guacamole workspace viewports. The source checkpoint adds `view_takeover` as a
service request action, keeps HTTP, MCP parity metadata, schema, generated
client helpers, dashboard routing, and user-facing docs aligned, and preserves
browser process and session ownership during viewer handoff.

The live two-client RDP and Guacamole proof has now run. Slice B live evidence
is recorded in
`docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`.

## Baseline

- Branch: `main`.
- Validation selector base: `HEAD`.
- Dirty state: broad and pre-existing. The worktree already included many
  service, dashboard, docs, and plan edits before this checkpoint. Slice B
  touched the service request contract, generated service request client,
  Rust request handling and relay routing, dashboard viewport takeover behavior,
  dashboard source smokes, docs, and the agent-browser skill.
- Installed skill sync: `skills/agent-browser/SKILL.md` was copied to
  `/home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md` after the
  selector-recommended diff showed only the new `view_takeover` guidance was
  missing from the installed copy.

## Source Changes

- Added `view_takeover` to `SERVICE_REQUEST_ACTIONS`,
  `service-request.v1.schema.json`, generated `@agent-browser/client` request
  helpers, and service request helper tests.
- Added a Rust `view_takeover` handler that returns typed takeover metadata and
  skips browser launch, close, relaunch, or process repair.
- Routed `view_takeover` through the HTTP service request relay by browser,
  session, and stream hints.
- Updated the dashboard workspace viewport so embedded Take over and external
  open both queue `view_takeover`, refresh or open the stream after the service
  request, and expose `takeover_ready`, `reconnecting`, and `taken_over` states.
- Kept the Guacamole interaction settings control available in the viewport
  toolbar for desktop and mobile-size clients.
- Updated `README.md`, `cli/src/output.rs`, `docs/src/app/`, contract docs, and
  the repo and installed agent-browser skill for the new behavior.

## Validation

Passed:

- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_job_control_plane_mode_marks_cdp_free_lifecycle_requests -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm test:service-client`
- `pnpm test:service-request-client`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-inspector-actions`
- `pnpm build:dashboard`
- `pnpm --dir docs build`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

Known warnings:

- `pnpm build:dashboard` reported existing Next.js export rewrite warnings.
- `pnpm --dir docs build` reported the existing multiple-lockfile workspace
  root warning.

## Live Gate

The Slice B live checkpoint passed on 2026-05-26:

- Open the same RDP or Guacamole workspace in two independent clients.
- Prove either simultaneous viewing or deterministic takeover in both
  directions.
- Capture screenshots from both clients, service-state samples before and after
  each takeover, iframe and external-open behavior, refresh recovery, and
  mobile-size Take over and interaction-settings controls.
- Record whether the current Guacamole deployment is simultaneous-viewer
  capable or single-active-viewer takeover only.

Observed provider behavior: `simultaneous_view`.

Evidence:

- `docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`
- `/tmp/agent-browser-rdp-guac-hardening-2026-05-26T14-32-50-612Z`
