# RDP Guac Slice D Readiness Source Checkpoint

Date: 2026-05-26
State: IMPLEMENTED_PENDING_LIVE_VALIDATION
Plan: `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`

## Scope

This note records the Slice D source checkpoint for RDP and Guacamole
readiness productization. It does not validate intentionally failed live
provider states. That remains the Slice D live checkpoint.

## Baseline

- Branch: `main`.
- Validation base: `HEAD`.
- Dirty state: broad active remote-view lane with existing Rust, dashboard,
  docs, script, client, and untracked plan or note files. The Slice D source
  changes are part of that lane.
- Touched surfaces in this checkpoint: dashboard readiness state model,
  workspace viewport recovery copy, launcher eligibility readiness parsing,
  RDP gateway readiness smoke output, dashboard source tests, docs, and the
  installed agent-browser skill.

## Source Changes

- Added compact workspace viewport readiness classification in
  `packages/dashboard/src/lib/workspace-viewport-state.ts`.
- The classifier distinguishes selected browser failure, dashboard auth
  failure, missing stream URL, iframe embedding limits, provider or ingress
  failure, viewer ownership or takeover, focus and takeover pending states,
  stale selected target recovery, and compact stream readiness components such
  as Guacamole connection or focus job state.
- `WorkspaceRemoteViewport` now renders readiness status and action data
  attributes, readiness badges, and recovery copy for the same classified
  states. Sign-in, Take over, and Open externally actions are tied to the
  derived readiness action.
- Launcher eligibility now parses optional compact remote-view readiness from
  access-plan payloads and can block or warn a launcher row before opening a
  broken route.
- `scripts/smoke-rdp-gateway-readiness.js` now emits a compact readiness
  payload with component, status, evidence, next action, and recovery fields
  for `guacd`, `xrdp`, `xrdp-sesman`, backend TCP, Guacamole web app,
  dashboard auth follow-up, iframe embedding follow-up, and public ingress.
- The readiness smoke discovers common `/usr/sbin` and `/usr/local/sbin`
  command paths so service binaries do not look missing only because of shell
  `PATH` drift.
- README, docs site pages, and the repo plus installed agent-browser skill were
  updated with the readiness surface and validation guidance.

## Validation

Passed:

- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-launcher-eligibility`
- `node --check scripts/smoke-rdp-gateway-readiness.js`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `pnpm validation:select -- --base HEAD`
- `node scripts/dev/select-validation.js --base HEAD --json`
- `git diff --check`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-inspector-actions`

Healthy live readiness baseline:

- `pnpm test:rdp-gateway-readiness-live -- --require-html5-client`
- Result: passed.
- `readiness.status`: `ready`.
- `guacd`: ready, `/usr/sbin/guacd`, TCP `127.0.0.1:4822` reachable.
- `xrdp`: ready, `/usr/sbin/xrdp`, TCP `127.0.0.1:3389` reachable.
- `xrdp_sesman`: ready, `/usr/sbin/xrdp-sesman`.
- Guacamole web app: ready, HTTP 302 from the configured route.
- Public ingress: ready. Public URL recorded in command output and intentionally
  omitted here.
- Dashboard auth and iframe embedding remain follow-up checks for the browser
  live harness.

Selector recommendations not run for this source checkpoint:

- Rust format, clippy, and focused Rust suites are still recommended by
  `validation:select` because the whole active lane contains Rust and contract
  changes. This Slice D source checkpoint did not add or edit Rust source.
- Service API, generated-client, and full service-client tests are still
  recommended by `validation:select` because the broader lane contains service
  contract and generated client changes. This checkpoint did not change those
  contract shapes.

## Residual Risk

- Slice D is not validated until the live failure-state checkpoint proves auth
  failure, missing or invalid Guacamole connection, refused or unreachable
  provider route, viewer takeover or remote disconnect, browser failure, and
  retained focus or takeover job ambiguity in rendered dashboard states.
- The healthy readiness baseline proves the compact output shape on the current
  deployment, but not intentionally failed public ingress or iframe-policy
  states.
