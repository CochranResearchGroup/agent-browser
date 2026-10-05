# Plan 0068: Operator Handoff And One-Time Profile Hardening

State: Implemented; live Guacamole/RDP smoke passed

Created: 2026-07-06

Source defects:

- `docs/dev/notes/2026-07-05-route-staleness-and-operator-handoff-defect.md`
- `docs/dev/notes/2026-07-05-one-time-profile-sprawl-defect.md`

## Goal

Make operator-assisted browser handoffs durable enough that a returned public
Guacamole URL is proof of the selected browser, display, route, and tab, and
make ordinary one-time operator tasks stop creating arbitrary profile/session
lanes.

## Current Failure Modes

1. A non-remote browser can still be opened first for a task that clearly needs
   manual login, payment, or challenge handling.
2. A route URL can look usable to the agent because CDP sees the right tab while
   the operator-facing Guacamole route is stale, wrong, or no longer coupled to
   the selected display.
3. `remote-view open` proves visibility before route checkout, but checkout and
   finalization can mutate retained route state after that proof.
4. One-time task retries can use hand-invented runtime profile names instead of
   one canonical managed task profile.
5. Browser build selection and executable evidence exist in access-plan and
   preflight surfaces, but the route-bound handoff response does not make the
   requested build and actual executable proof compact enough for operators.

## Product Requirements

- `remote-view open` emits a successful operator URL only after final route,
  route-pool, display, browser-window, selected-tab, and Guacamole URL evidence
  all agree.
- A stale retained route, wrong route display, wrong selected tab, missing CDP
  target, or unavailable Guacamole route must fail closed with a typed
  `operatorVisible.state` before the command reports `status=opened`.
- Manual-login, payment, and challenge-like workflows use route-bound remote
  view by default, or emit an explicit warning before opening a non-visible
  browser path.
- One-time operator tasks get a service-managed profile class and stable task
  profile identity. Retries reuse that profile unless a lock conflict,
  browser-family incompatibility, explicit identity request, or isolation policy
  requires a new lane.
- Service state exposes profile class values such as `default`,
  `managed_one_time`, `durable_named`, and `operator_supplied`.
- `remote-view open` returns compact browser-build proof containing the requested
  build, selected build, actual executable path when known, and mismatch state.

## Slice A: Final Handoff Proof Gate

Status: implemented for the final proof gate

Implementation:

- Recompute `operatorVisible` after route checkout using the post-checkout
  route-pool entry and remote-view route state.
- Return `finalOperatorVisible` alongside `operatorVisible` so clients can see
  the post-checkout proof explicitly.
- Make `operatorVisible` in the success response equal to the final proof.
- Roll back the acquisition lease and clean up the launched tab/browser if the
  final proof is no longer ready.
- Add no-launch tests for a stale retained route appearing at the final
  post-checkout boundary.
- 2026-07-06: Implemented post-checkout `operatorVisible` recomputation,
  `preCheckoutOperatorVisible`, `finalOperatorVisible`, and fail-closed rollback
  when the final proof is not ready.

Acceptance:

- A stale route-pool entry after checkout cannot produce `status=opened`.
- The final proof carries the same browser, session, display allocation, route,
  route-pool entry, and selected tab evidence returned to the operator.
- Existing dry-run behavior remains `operatorVisible.state=not_checked`.

## Slice B: Browser Build And Executable Proof

Status: implemented

Implementation:

- Add a `browserBuildProof` object to `remote-view open` responses.
- Populate requested build from the handoff intent.
- Populate selected build and actual executable path from the browser capability
  launch resolution when available.
- Mark proof as `not_checked`, `matched`, or `mismatch` so a requested
  `stock_chrome` launch cannot silently look like a successful stealth launch.
- 2026-07-06: Implemented `browserBuildProof` and no-launch mismatch coverage
  for a requested `stock_chrome` handoff resolving to a stealth executable path.

Acceptance:

- `remote-view open` responses include requested browser build and executable
  proof.
- Tests cover explicit `stock_chrome` request with a mismatched selected build
  or missing executable proof.

## Slice C: Managed One-Time Task Profile Contract

Status: implemented for `remote-view open`, `service access-plan`, and retained cleanup

Implementation:

- Add `oneTimeProfileWarning` to `remote-view open` responses when a
  RDP/manual/remote-headed login, payment, or challenge-like handoff passes a
  runtime profile that is not already a known service profile.
- Return the requested profile as `operator_supplied`, recommend
  `managed_one_time`, and include a deterministic recommended task profile id.
- Add a profile-class model to service state and generated client contracts.
- Generate a deterministic one-time task profile id from service, agent, task,
  and target URL.
- Teach `remote-view open` to select the managed one-time profile when a request
  is operator-assisted and does not specify a durable identity.
- Warn when callers pass a new arbitrary `runtimeProfile` for a request that
  looks like a one-time operator handoff.
- Extend cleanup to remove abandoned managed one-time profiles without touching
  default or durable profiles.
- 2026-07-06: Implemented `ProfileClass` in service profile records and
  profile-allocation records, generated observability client types, dashboard
  display/preservation, deterministic `managed_one_time` profile planning and
  reuse in `remote-view open`, warning for arbitrary one-time runtime profile
  names, and retained cleanup for unreferenced nonpersistent managed one-time
  profiles.
- 2026-07-06: Extended `service access-plan`, HTTP, MCP resource templates, and
  generated clients to carry explicit `runtimeProfile`. A known persisted
  runtime profile wins access-plan selection, and RDP/manual/remote-headed
  one-time handoffs now return `decision.oneTimeProfileRecommendation` before
  callers post the service request. The recommendation plans a deterministic
  `managed_one_time` profile when no durable profile is selected and warns when
  the caller supplied an unknown `operator_supplied` runtime profile.

Acceptance:

- `remote-view open` warns on arbitrary runtime profile names for one-time
  operator handoffs.
- A one-time operator handoff can be opened without hand-naming a runtime
  profile.
- Retries for the same one-time task reuse one managed one-time profile.
- Service state reports the profile class and cleanup eligibility.
- `service access-plan` accepts explicit runtime profile hints, chooses a known
  matching service profile, and returns a read-only one-time profile
  recommendation or warning before launch.

## Slice D: Regression And Stress Coverage

Status: no-launch implemented; live smoke passed

Required tests:

- No-launch final proof test: stale Route A retained record versus healthy Route
  B cannot return Route A as ready.
- No-launch final proof test: wrong display coupling returns
  `wrong_route_display` or another typed non-ready state.
- No-launch selected-tab test: visible browser plus wrong selected URL returns
  `wrong_tab`.
- One-time profile test: SOSDirect-style temporary login retry reuses one
  managed one-time profile.
- Browser-build proof test: requested `stock_chrome` cannot report success with
  a stealth executable and no warning.
- Live smoke: one route retry with public Guacamole URL render evidence for the
  selected browser.

Implemented coverage:

- Final post-checkout stale-route proof regression.
- Wrong selected-tab, missing CDP target, unavailable Guacamole route, and stale
  route state operator-visible regressions.
- Managed one-time profile planning and deterministic reuse regressions.
- Arbitrary one-time runtime-profile warning regression.
- Access-plan managed one-time recommendation, explicit known runtime-profile
  selection, and arbitrary one-time runtime-profile warning regressions.
- Managed one-time retained cleanup regression.
- Browser-build proof mismatch regression.
- Live fixture-backed Guacamole/RDP open, repeat open, and HTTP helper smoke
  with OCR proof for the selected route-bound browser.

## Validation Plan

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- Focused Rust tests for remote-view proof and service profile cleanup.
- Client contract tests if generated service request or response contracts
  change.
- Live Guacamole/RDP smoke only after no-launch proof gates pass.

## Validation Evidence

2026-07-06 no-launch validation:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml remote_view_open_ -- --test-threads=1 --nocapture`
- `cargo test --manifest-path cli/Cargo.toml parse_service_access_plan -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1 --nocapture`
- `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml prune_retained -- --test-threads=1`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm generate:service-client`
- `pnpm test:service-client`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:route-confusion-gates`
- `pnpm --dir docs build`
- `git diff --check`

Known remaining live gate:

- None for this plan. The live Guacamole/RDP fixture smoke passed on
  2026-07-06 with artifact directory
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T02-46-46-379Z`.

2026-07-06 live Guacamole/RDP validation:

- Before the live smoke, route-pool readiness was green with explicit route
  displays `:10` and `:11`, and `pnpm grant:rdp-route-display-access -- --apply`
  granted the local operator user display access for both route users.
- `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD="$PWD/cli/target/debug/agent-browser" AGENT_BROWSER_REMOTE_VIEW_OPEN_TIMEOUT_MS=300000 AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:10 AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:11 AGENT_BROWSER_RDP_ROUTE_POOL_JSON="$(...)"
  pnpm test:remote-view-open-fixture-live`
- Summary:
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T02-46-46-379Z/summary.json`
  reports `success=true`, route `guacamole:4`, display allocation
  `remote-view-display:10`, public operator URL
  `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/NABjAHBvc3RncmVzcWw=`,
  local frame URL
  `http://127.0.0.1:8092/guacamole/#/client/NABjAHBvc3RncmVzcWw=`,
  route-handoff classification `route_bound_ready`, visual state
  `browser_window_visible`, selected target
  `E4560E168838D04EEC862F34C692E172`, and
  `duplicateIntentTabCount=1`.
- `cli-first.json` opened target `E4560E168838D04EEC862F34C692E172` on
  `guacamole:4` / `remote-view-display:10` with
  `operatorVisible.state=ready`.
- `cli-repeat.json` reused the same target with
  `tabAcquisitionDecision=reused_compatible_target`.
- `http-helper.json` also reused the same target with
  `tabAcquisitionDecision=reused_compatible_target`.
- OCR proof:
  `/tmp/agent-browser-remote-view-open-live-2026-07-06T02-46-46-379Z/route-display-root-ocr.txt`
  contains the fixture marker `REMOTE VIEW OPEN FIXTURE 11022`.

## Closeout Criteria

- All Slice A and Slice B tests pass.
- Slice C has landed for `remote-view open`, `service access-plan`, generated
  clients, docs/help surfaces, and retained cleanup.
- The two source defect notes can point to this plan and the implemented tests
  as their remediation authority.
- The live Guacamole/RDP fixture smoke passes with route-bound first, repeat,
  and HTTP helper evidence.
