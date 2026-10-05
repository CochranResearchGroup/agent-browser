# Effective Stealth Remote Default Launch Plan

Date: 2026-05-31
State: COMPLETE
Lane: P09/P12 runtime posture
Depends On:
- `docs/dev/plans/0009-2026-05-30-p08-packaging-and-integration-plan.md`
- `docs/dev/plans/0011-2026-05-30-live-dashboard-runtime-publish-plan.md`
- `docs/dev/plans/0012-2026-05-31-workspace-inspection-pane-app-intelligence-roadmap.md`

## Purpose

Make the workstation default real, not advisory.

Today `service.defaultBrowserBuild=stealthcdp_chromium` is visible in service
status and honored by `service access-plan`, but ordinary fresh launches can
still fall through to `runtimeProfile=default`, `browserHost=local_headless`,
and `cdp_screencast`. That is why a Company Assets Google Sheets browser QA
opened in `session:default` instead of the hidden remote-headed
`stealthcdp-default` posture.

This plan makes a bare or ordinary launch on this workstation resolve to the
configured stealth remote posture unless the caller explicitly overrides it.

## Current Baseline

Observed on 2026-05-31:

- `~/.agent-browser/config.json` sets
  `service.defaultBrowserBuild=stealthcdp_chromium`.
- `agent-browser service status` reports
  `launchConfig.defaultBrowserBuild=stealthcdp_chromium` and
  `launchConfig.stealthCdpChromiumReady=true`.
- `agent-browser service access-plan --service-name odollo --task-name ups`
  selects `stealthcdp-default`, `stealthcdp_chromium`, `remote_headed`,
  `rdp_gateway`, and `manual_attached_desktop`.
- The Company Assets Google Sheets QA tab was retained under
  `browserId=session:default`, `profileId=default`, `host=local_headless`, and
  `stream=cdp_screencast`.
- The `session:default` service-session record reported
  `browserCapabilityLaunch.applied=false` with reason `missing_browser_build`
  and `profileSelectionReason=explicit_profile`.

The defect is not that stealth Chromium is unavailable. The defect is that the
ordinary launch path does not consume the configured default launch posture.

## Implementation Progress

Updated on 2026-05-31:

- Added an effective launch-default resolver in `cli/src/native/actions.rs`
  that builds the same service access-plan request used by service clients and
  merges the planned `browserBuild`, managed profile, browser host, view stream,
  control input, display isolation, and lease policy into ordinary launches
  when those fields are not explicitly supplied by the caller.
- Preserved explicit caller overrides for profile, runtime profile, headless
  mode, browser host, browser build, and operator-supplied executable paths.
- Tagged CLI-inserted executable paths with `executablePathSource` so
  manifest-derived default executables no longer block guarded browser
  capability selection.
- Added a built-in `google_sheets` site policy for
  `https://docs.google.com/spreadsheets` that selects
  `stealthcdp_chromium`, `remote_headed`, `rdp_gateway`, and
  `manual_attached_desktop`.
- Tightened built-in site-policy URL matching so path-specific policies such as
  Google Sheets do not match unrelated `docs.google.com` document URLs.
- Added focused unit coverage for effective service defaults, manifest
  executable handling, explicit local-headless overrides, and Google Sheets
  policy matching.

## Validation Evidence

Collected on 2026-05-31:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check` passed.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` passed.
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
  passed with 27 tests.
- `cargo test --manifest-path cli/Cargo.toml native::service_health --
  --nocapture` passed with 32 tests.
- `cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture
  --test-threads=1` passed with 177 tests. The same filter is not safe in
  parallel because existing tests mutate the shared service-state repository.
- `pnpm test:dashboard-view-streams` passed.
- `pnpm test:dashboard-workspace-navigator` passed.
- `pnpm --dir docs build` passed.
- `pnpm build:dashboard` passed.
- `git diff --check` passed.
- `pnpm publish:local-dashboard -- --expect-marker Workspaces --skip-browser
  --json` rebuilt the dashboard and CLI, replaced `~/.local/bin/agent-browser`,
  restarted `agent-browser-dashboard.service`, and proved the local dashboard
  bundle contains the `Workspaces` marker.
- `agent-browser --json service access-plan --url
  'https://docs.google.com/spreadsheets/d/example/edit'` selected
  `profileId=stealthcdp-default`, `browserHost=remote_headed`,
  `browserBuild=stealthcdp_chromium`, `viewStreamProvider=rdp_gateway`, and
  `controlInputProvider=manual_attached_desktop`.
- `agent-browser --json --session default-posture-smoke --leave-open open
  https://example.com` succeeded without explicit profile, host, or browser
  build flags.
- The retained `default-posture-smoke` service browser record is `ready`,
  `host=remote_headed`, `profileId=stealthcdp-default`, has PID `87825`, and
  exposes an `rdp_gateway` view stream with `manual_attached_desktop` input.
- The retained `default-posture-smoke` service session records
  `browserCapabilityLaunch.applied=true`, binding
  `default-stealthcdp-wsl-native`, executable
  `stealthcdp-chromium-wsl-promoted`, and passed validation evidence.
- `node scripts/smoke-local-dashboard-runtime.js --dashboard-url
  https://agent-browser.ecochran.dyndns.org/ --workspace-session
  default-posture-smoke --session dashboard-smoke-plan0016 --browser-profile
  /tmp/agent-browser-dashboard-smoke-plan0016-profile --expect-marker
  Workspaces --json` passed after authenticating with the user-scoped
  dashboard auth env file. It proved the hosted workspace-control route for
  `default-posture-smoke` renders the workspace pane and viewport, reports
  `readinessStatus=ready`, exposes a `cdp_screencast` canvas, and shows Codex
  app server chat context for the selected browser.

## Product Contract

On this workstation, these should be equivalent for ordinary browser work when
the caller does not explicitly request a different profile, executable, or host:

```bash
agent-browser open <url>
```

and:

```bash
agent-browser open <url> \
  --runtime-profile stealthcdp-default \
  --browser-host remote_headed \
  --view-stream-provider rdp_gateway \
  --control-input-provider manual_attached_desktop \
  --display-isolation private_virtual_display
```

The exact profile and display isolation may still come from config, service
profile selection, or site policy, but the effective behavior must be a hidden
remote-headed stealth Chromium browser with an operator-visible viewport.

## Non-Goals

- Do not remove explicit local headless, local headed, stock Chrome, or
  caller-supplied executable support.
- Do not force Google sign-in flows into CDP attachment when a site policy
  requires detached seeding or CDP-free operation.
- Do not make remote-headed mandatory on machines without a ready remote-view
  route.
- Do not silently attach patched Chromium to a Chrome-owned profile.
- Do not change Company Assets canonical catalog or Google Sheet contents.

## Precedence Rules

Explicit caller intent still wins:

1. `--executable-path`, `AGENT_BROWSER_EXECUTABLE_PATH`, or command
   `executablePath`.
2. `--profile`, command `profile`, or caller-supplied profile path.
3. `--runtime-profile`, command `runtimeProfile`.
4. Explicit `--browser-host`, command `browserHost`, or nested
   `params.browserHost`.
5. Site policy and service profile selection for known target URLs.
6. Configured service default browser build and matching default profile.
7. Built-in fallback.

The new behavior changes item 6: configured defaults must affect ordinary
launches, not only access-plan recommendations.

## Desired Resolution

For a fresh direct launch with no explicit profile, runtime profile,
browser host, or executable:

- Read the same effective config used by service status.
- If `service.defaultBrowserBuild=stealthcdp_chromium` and the ready manifest
  exists, set `browserBuild=stealthcdp_chromium`.
- Select a compatible managed profile, preferring the service profile marked
  for that build, such as `stealthcdp-default`.
- If remote-view config is ready, set:
  - `browserHost=remote_headed`
  - `viewStreamProvider=rdp_gateway`
  - `controlInputProvider=manual_attached_desktop`
  - `displayIsolation=private_virtual_display` unless config says otherwise
- Record a launch diagnostic that says the defaults were applied, for example
  `browserCapabilityLaunch.applied=true` and
  `reason=configured_default_browser_build`.

For direct launches with target URL metadata:

- Apply matching site policy before the global default.
- A `docs.google.com/spreadsheets` policy should prefer the same hidden remote
  posture for browser QA unless a future Google Workspace policy requires a
  different login seeding flow.

## Implementation Slices

### Slice 1 | Locate The Direct Launch Gap

Goal: identify the launch path that produces `missing_browser_build` for
ordinary `agent-browser open`.

Tasks:

- Trace command construction in `cli/src/main.rs`.
- Trace launch option application in `cli/src/native/actions.rs`.
- Confirm where service profile/default build lookup is skipped for direct
  launches.
- Add a failing unit test or no-launch smoke fixture showing a bare open would
  choose `default/local_headless`.

Exit criteria:

- A focused test captures the current bug without launching Chrome.

### Slice 2 | Effective Launch Defaults Helper

Goal: centralize default launch posture resolution.

Tasks:

- Add a helper that receives the command JSON, effective config, and optional
  target URL.
- Return a normalized launch defaults object with `browserBuild`,
  `runtimeProfile`, `browserHost`, `viewStreamProvider`,
  `controlInputProvider`, `displayIsolation`, and a diagnostic reason.
- Reuse existing service profile and browser capability registry helpers where
  possible instead of inventing a parallel selector.
- Keep explicit flags and explicit profile paths untouched.

Exit criteria:

- Unit tests cover default application, explicit override preservation,
  incompatible profile avoidance, and missing manifest fallback.

### Slice 3 | Apply Defaults To Direct Launches

Goal: make ordinary launch commands consume the helper.

Tasks:

- Apply the helper before `apply_remote_headed_launch_env_hints`.
- Ensure prelaunch daemon command JSON carries the selected defaults.
- Ensure retained browser/session records preserve the applied diagnostic.
- Make `runtime status` and dashboard workspace rows show the selected
  `stealthcdp-default` profile and `remote_headed` host.

Exit criteria:

- A no-launch or isolated temp-state test proves a bare launch command resolves
  to the configured stealth remote posture.

### Slice 4 | Site Policy For Google Workspace Review

Goal: keep Company Assets and similar review workflows from falling into
`session:default`.

Tasks:

- Add or persist a site policy for `https://docs.google.com/spreadsheets`.
- Select `stealthcdp_chromium`, `remote_headed`, `rdp_gateway`, and
  `manual_attached_desktop`.
- Decide whether it should require profile freshness or manual seeding
  distinct from Google login.
- Add access-plan coverage proving the Sheets URL selects the intended posture.

Exit criteria:

- `agent-browser service access-plan --url <sheets-url>` reports the hidden
  remote stealth posture without hand-entered browser flags.

### Slice 5 | Dashboard And Docs Alignment

Goal: remove the “song and dance” from operator-facing surfaces.

Tasks:

- Update dashboard guided launcher defaults to match the effective launch
  default helper.
- Update `README.md`, `skills/agent-browser/SKILL.md`, and docs site language
  so “default” means ordinary launches, not only access-plan output.
- Add a short troubleshooting note for explicit overrides that intentionally
  choose local headless.

Exit criteria:

- Docs and skill guidance no longer tell agents to spell out the full remote
  posture for ordinary work on a configured workstation.

### Slice 6 | Runtime Publish And External Proof

Goal: prove the installed runtime behaves correctly.

Tasks:

- Publish the local dashboard/runtime after source validation.
- Open a benign page with a bare command or the closest safe equivalent:

```bash
agent-browser --session default-posture-smoke open https://example.com
```

- Verify service records show:
  - `profileId=stealthcdp-default`
  - `host=remote_headed`
  - `viewStreams[0].provider=rdp_gateway` or configured remote view provider
  - `browserCapabilityLaunch.applied=true`
- Open the dashboard workspace route and verify an operator-visible viewport.

Exit criteria:

- The installed runtime proves a short ordinary launch uses hidden remote
  stealth by default.

## Validation Matrix

Required source checks:

```bash
cargo test --manifest-path cli/Cargo.toml native::actions -- --nocapture
cargo test --manifest-path cli/Cargo.toml native::service_health -- --nocapture
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-view-streams
git diff --check
```

Required runtime checks:

```bash
agent-browser service status
agent-browser service access-plan --url 'https://docs.google.com/spreadsheets/d/example/edit'
pnpm publish:local-dashboard -- --expect-marker Workspaces --json
agent-browser --json --session default-posture-smoke open https://example.com
agent-browser --json service browsers
agent-browser --json service sessions
```

Required hosted smoke:

```bash
node scripts/smoke-local-dashboard-runtime.js \
  --dashboard-url https://agent-browser.ecochran.dyndns.org/ \
  --workspace-session default-posture-smoke \
  --expect-marker data-codex-app-server-contextual-chat \
  --json
```

## Risks And Mitigations

- Risk: breaking CI or tests that assume bare launches are headless.
  Mitigation: make config-driven defaults explicit in tests and allow env or
  temp config to force local headless.
- Risk: remote-headed launch on a host without route readiness.
  Mitigation: only apply remote-headed when remote view config and display
  readiness are present; otherwise apply stealth build/profile but report a
  fallback diagnostic.
- Risk: profile family mismatch.
  Mitigation: keep existing browser-family compatibility checks and refuse
  patched Chromium on Chrome-owned profiles unless explicitly overridden.
- Risk: Google login flows need detached seeding.
  Mitigation: distinguish `accounts.google.com` login policy from
  `docs.google.com/spreadsheets` review policy.

## Completion Criteria

This plan is complete when:

- Bare ordinary launches consume `service.defaultBrowserBuild`.
- A configured workstation defaults to hidden remote-headed
  `stealthcdp_chromium` with a compatible managed profile.
- Explicit local/headless/profile/executable overrides still work.
- Google Sheets browser QA no longer lands in `session:default` unless the
  caller explicitly asks for it.
- Source validation passes.
- Installed local runtime is republished.
- External dashboard smoke proves the short-launch default is inspectable in
  the UX.

## Recommended Next Step

Start with Slice 1 and Slice 2. Do not patch callers or runbooks first; the
launch resolver itself needs to make the configured default effective.
