# Minimal Runtime Profile Reuse Plan

Date: 2026-06-05
State: CLOSED
Lane: P13
Depends On:
- `docs/dev/plans/0010-2026-05-30-retained-orphan-profile-cleanup-plan.md`
- `docs/dev/plans/0016-2026-05-31-effective-stealth-remote-default-launch-plan.md`
- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`

## Purpose

Make agent-browser promote the minimal necessary number of runtime profiles and
live browser processes for the user account, website, browser-build, and
remote-view combinations that are actually running at the same time.

The operating invariant is:

```text
runtime profiles ~= distinct authenticated account / website isolation sets
```

Runtime profiles must not multiply per client, agent, job, command, or tab.
agent-browser owns CDP and drains browser work through its service queue, so
multiple clients should share one compatible profile and browser lane when the
same account/site context is safe to reuse.

## Current Problem

Plan 0026 added visibility and conservative cleanup after browser and display
process sprawl happens. It does not yet prevent avoidable sprawl at the broker
layer.

Current service primitives already exist:

- Access-plan selects a managed profile from service identity, target identity,
  account identity, site policy, and browser build.
- Service requests can set `profileLeasePolicy: "wait"` so a busy profile does
  not require a duplicate launch.
- Service sessions record `profileLeaseDisposition` as `new_browser`,
  `reused_browser`, or `active_lease_conflict`.
- Retained-state pruning and resource GC can clean up stale records and stale OS
  resources.

The missing product contract is a first-class reuse recommendation before a
caller launches. Access-plan should answer whether the minimal-profile path is
to reuse a live browser, wait for the profile lease, or launch a new browser
because isolation actually requires it.

## Product Contract

- Reuse first: prefer an existing live compatible browser/profile lane for the
  same profile, account/site identities, browser build, browser host, view
  stream provider, control input provider, and display-isolation posture.
- Queue instead of clone: when a compatible profile is busy, recommend
  `profileLeasePolicy: "wait"` rather than creating another equivalent runtime
  profile.
- Launch only with a reason: new browser/profile creation must be explainable by
  missing profile, no compatible live browser, explicit isolation, incompatible
  browser build, incompatible host/posture, manual seeding, or safety policy.
- Make sprawl visible: access-plan, dashboard, and doctor surfaces should expose
  duplicate compatible profile/browser pressure before operators discover it via
  `ps` or resource GC.
- Preserve identity safety: do not reuse across different authenticated account
  identities, different required site isolation sets, incompatible browser
  families/builds, or explicit operator isolation requests.

## Slice A: Read-Only Access-Plan Reuse Advisory

Goal: make the broker explain minimal-profile decisions without changing launch
behavior.

Execution status: closed for Slice A. The plan remains open for Slice B and
Slice C.

Implementation:

- Add a `profileReuse` object under access-plan `decision`.
- Include selected profile id, selected browser id when an existing compatible
  live browser can be reused, active lease conflict session ids, compatible
  waiting recommendation, duplicate profile/browser counts, recommended action,
  and stable reasons.
- Set access-plan service-request `profileLeasePolicy` to `wait` for ordinary
  selected-profile tab/launch requests when reuse is safe but another exclusive
  session currently holds the same profile.
- Keep direct launch behavior unchanged; this slice is advisory plus request
  shaping only.

Acceptance:

- Access-plan for a ready selected profile with a live compatible browser reports
  `decision.profileReuse.recommendedAction = "reuse_existing_browser"`.
- Access-plan for a selected profile held by another exclusive session reports
  `recommendedAction = "wait_for_profile_lease"` and names the conflict
  sessions.
- Access-plan for a selected profile with no compatible live browser reports
  `recommendedAction = "launch_new_browser"`.
- Access-plan warns when multiple live browsers or active exclusive sessions
  exist for the same selected profile.
- Existing profile selection, readiness, browser capability, and service request
  contracts continue to pass.

### 2026-06-05 Slice A Implementation Record

Implemented:

- Added `decision.profileReuse` to access-plan responses.
- The advisory reports selected profile id, compatible reusable browser ids,
  same-profile live browser ids, active exclusive profile lease sessions,
  duplicate pressure, posture fields, stable reasons, and recommended action.
- Added fixture tests for `reuse_existing_browser`, `wait_for_profile_lease`,
  and `launch_new_browser`.
- Updated the access-plan response schema, generated service observability
  types, README, and docs site.

Validation evidence:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml service_access_plan_recommends`
- `cargo test --manifest-path cli/Cargo.toml service_access`
- `cargo test --manifest-path cli/Cargo.toml service_profile_lease`
- `cargo test --manifest-path cli/Cargo.toml test_format_service_access_plan_text_includes_browser_build_summary`
- `cargo test --manifest-path cli/Cargo.toml test_service_access_plan_reports_browser_build_summary_without_launch -- --nocapture`
- `cargo test --manifest-path cli/Cargo.toml command_`
- `cargo build --manifest-path cli/Cargo.toml`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client`
- `pnpm --dir docs build`
- `git diff --check`

Live readback:

- `./cli/target/debug/agent-browser service access-plan --login-id canva --json`
  returned `decision.profileReuse.recommendedAction =
  "register_or_select_profile"`, `duplicatePressure = false`, and
  `profileLeasePolicy = "wait"`.
- Text output includes `profile_reuse action=register_or_select_profile`.

## Slice B: Dashboard And Doctor Visibility

Goal: show avoidable browser/profile sprawl before it becomes resource pressure.

Execution status: closed.

Implementation:

- Surface `decision.profileReuse` in dashboard launch eligibility and selected
  workspace evidence.
- Add install doctor or service doctor warnings for duplicate live browsers or
  duplicate active exclusive sessions for the same profile when those duplicates
  are not explained by distinct account/site isolation metadata.
- Add operator text that distinguishes expected simultaneous account/site
  isolation from accidental duplicate browsers.

Acceptance:

- Dashboard access-plan rows show reuse, wait, or launch-new reasoning.
- Doctor reports duplicate profile pressure with stable issue codes but does not
  fail when duplicates are explicitly isolated by account/site/build/posture.

### 2026-06-05 Slice B Dashboard Implementation Record

Implemented:

- Dashboard launcher eligibility rows use `decision.profileReuse` as the
  access-plan row reason when access-plan data has been fetched.
- The row reason now distinguishes compatible live-browser reuse, profile lease
  waiting, new-browser launch, profile seeding, and missing-profile selection.
- Dashboard launcher eligibility smoke coverage proves both
  `reuse_existing_browser` and `wait_for_profile_lease` row reasons.
- README, docs site, and agent-browser skill guidance now describe dashboard
  reuse advisory visibility.

### 2026-06-05 Slice B Doctor Implementation Record

Implemented:

- `service resources` now emits stable duplicate-pressure warning records:
  `duplicate_live_browsers_for_profile` and
  `duplicate_active_profile_leases`.
- `agent-browser install doctor` converts either resource warning into
  `service_duplicate_profile_pressure`.
- The duplicate live-browser warning groups retained browsers by selected
  profile plus retained browser posture fields, and includes retained target,
  authenticated service, and account identity evidence when known.
- README, docs site, CLI help, repo skill, and installed shared
  `agent-browser` skill document the warning and doctor issue codes.

Validation evidence:

- `cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`

## Slice C: Launch-Path Enforcement

Goal: prevent avoidable duplicate runtime profiles and browser launches.

Execution status: closed.

Implementation:

- Make service request helpers default to access-plan-selected reuse and wait
  semantics.
- Reject or warn on launch requests that would create a duplicate compatible
  browser/profile lane when an existing lane can be reused or waited on.
- Add an explicit override for reviewed isolation or throwaway browser behavior.

Acceptance:

- Compatible simultaneous clients share one profile/browser lane and serialize
  CDP work through the queue.
- Different account/site isolation sets still receive separate profiles or
  browser lanes.
- Tests prove wait/reuse behavior does not block unrelated service requests.

### 2026-06-05 Slice C Route-Hint Implementation Record

Implemented:

- Access-plan `decision.profileReuse` now reports `reusableSessionName` when a
  compatible live browser can be reused.
- When `recommendedAction` is `reuse_existing_browser`,
  `decision.serviceRequest.request` carries top-level `browserId` and
  `sessionName` route hints.
- HTTP `POST /api/service/request` and MCP `service_request` accept and preserve
  top-level `browserId` and `sessionName`.
- The HTTP relay routes ordinary non-focus service requests with top-level
  `browserId` or `sessionName` to the existing daemon session. It keeps
  action-specific `params.browserId` and `params.sessionName` scoped to
  remote-view actions such as `view_focus` and `view_takeover`.
- Generated service request and observability client types include the route
  hints, and managed-profile examples call out the fields.
- README, docs site, CLI help, repo skill, and installed shared
  `agent-browser` skill describe the route-hint contract.

Validation evidence:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml service_request_schema_and_command_accept_contract_actions`
- `cargo test --manifest-path cli/Cargo.toml service_request_`
- `cargo test --manifest-path cli/Cargo.toml service_access`
- `pnpm test:service-client-contract`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm --dir docs build`
- `pnpm test:service-client`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`

### 2026-06-05 Slice C Duplicate-Launch Guard Implementation Record

Implemented:

- The service launch profile gate now blocks direct launches that select a
  retained profile already backed by a live retained browser, unless the command
  carries access-plan route hints or sets `allowDuplicateProfileLane: true`.
- The rejection explains the existing browser ids and tells callers to reuse
  `browserId` or `sessionName`, wait for the lane, request a different profile,
  or use the explicit duplicate-lane override for reviewed isolation or
  throwaway work.
- HTTP `POST /api/service/request`, MCP `service_request`, browser MCP target
  hints, JSON schema, and generated client types accept
  `allowDuplicateProfileLane`.
- Unit coverage proves duplicate launch rejection, route-hint allowance, and
  explicit override allowance.
- README, docs site, CLI help, repo skill, and installed shared
  `agent-browser` skill document the guard and override.

Validation evidence:

- `cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate`
- `cargo test --manifest-path cli/Cargo.toml service_request_`

## Validation Plan

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_access
cargo test --manifest-path cli/Cargo.toml service_profile_lease
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm test:dashboard-launcher-eligibility
pnpm --dir docs build
git diff --check
```

Run live smokes only for enforcement slices or when changing actual launch
behavior.

## 2026-06-05 Closeout

Plan 0027 is closed. The minimal-runtime-profile invariant is now enforced at
three layers:

- Access-plan gives a no-launch `decision.profileReuse` recommendation and
  copyable route-hinted service request.
- Dashboard and doctor surfaces expose reuse/wait/duplicate-pressure evidence
  before process pressure has to be diagnosed from `ps`.
- Direct service launches that bypass route hints are blocked when a retained
  live browser already serves the selected profile, unless the caller supplies
  the explicit reviewed duplicate-lane override.

Final validation evidence:

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo test --manifest-path cli/Cargo.toml service_access`
- `cargo test --manifest-path cli/Cargo.toml service_request_`
- `cargo test --manifest-path cli/Cargo.toml service_resources -- --test-threads=1`
- `cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate`
- `cargo test --manifest-path cli/Cargo.toml install_doctor_flags`
- `pnpm test:service-client`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm --dir docs build`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `git diff --check`

## Closeout Contract

Closed after access-plan, dashboard/operator visibility, doctor
duplicate-pressure issue codes, and launch-path enforcement proved the
minimal-profile invariant.
