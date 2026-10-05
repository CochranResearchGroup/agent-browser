# Remote Headed Cutover Proof Plan

Date: 2026-06-19
State: DONE
Lane: P14/P16
Depends On:
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`
- `docs/dev/plans/0034-2026-06-14-generic-browser-service-routines-plan.md`
- `docs/dev/plans/0036-2026-06-18-rdp-ready-to-go-plan.md`
- `docs/dev/plans/0037-2026-06-19-runtime-profile-sharing-plan.md`

## Purpose

Agent-browser should be able to own the preferred hidden browser lane for
downstream services before any downstream cutover asks those services to mutate
live state. The target lane is generic:

- `browserBuild=stealthcdp_chromium`
- `browserHost=remote_headed`
- `viewStreamProvider=rdp_gateway`
- `controlInputProvider=manual_attached_desktop` or another explicit
  remote-view-safe provider
- `displayIsolation=private_virtual_display`

AuraCall is the first pressure test for this lane, but this plan must not make
AuraCall-specific selectors, accounts, migrations, or repo mutations part of
agent-browser. The output should be a generic, live-testable service contract.
After agent-browser proves the contract, a separate AuraCall handoff note can
describe the new features and let AuraCall's own agent decide how to adopt or
migrate.

## Current Evidence

The 2026-06-19 agent-browser readback shows:

- `agent-browser doctor remote-view --json` reports overall `status=ready`.
- The same doctor reports install drift:
  `path_command_pnpm_binary_mismatch` and
  `path_command_workspace_binary_mismatch`, so closeout still recommends
  install drift repair before cutover.
- Remote-view prerequisites are otherwise healthy: Guacamole local embed and
  public operator routes are reachable, route pool readiness is true, route
  displays `:12` and `:11` are accessible, privileged helper readiness is true,
  and many-to-many readiness is true.
- Public operator ingress remains available through
  `https://agent-browser.ecochran.dyndns.org/guacamole/`; local
  `http://127.0.0.1:8092/guacamole/` URLs are route-descriptor fields for
  local dashboard and live harness embedding.
- An AuraCall-shaped no-launch access-plan request with
  `--browser-build stealthcdp_chromium --browser-host remote_headed
  --view-stream-provider rdp_gateway --control-input-provider
  manual_attached_desktop --display-isolation private_virtual_display` selects
  `auracall-chatgpt-wsl-chrome-2-consult` and accepts the requested browser
  build.
- That same access-plan still resolves the launch posture and service request
  back to `browserHost=local_headed`, `viewStreamProvider=cdp_screencast`,
  `controlInputProvider=cdp_input`, `displayIsolation=null`, and
  `profileReuse.recommendedAction=launch_new_browser`.

Conclusion: the RDP/Guacamole stack is close enough to test, but the cutover
contract is not ready. Access-plan must preserve the requested remote-headed
lane and recommend safe reuse or service-owned acquisition before downstream
clients should default to this path.

## Goal

Prove a generic agent-browser-owned remote-headed cutover lane that downstream
services can select without duplicate profile pressure, route ambiguity, or
live mutation risk.

Done means:

- install and remote-view doctor surfaces return ready or explain a bounded
  non-blocking drift state;
- no-launch access-plan preserves requested remote-headed posture across CLI,
  HTTP, MCP, and generated client helpers;
- access-plan returns structured route descriptors with local embed and public
  operator URLs instead of one ambiguous URL;
- a selected authenticated profile with a compatible retained browser yields
  `reuse_existing_browser` plus shared tab acquisition, or a clearly safe
  service-owned acquisition path;
- access-plan does not fall back to `local_headed`, `cdp_screencast`,
  `cdp_input`, or `launch_new_browser` when the caller requested a ready
  remote-headed lane and a compatible live holder exists;
- live smokes prove two clients can share one profile through a retained
  remote-headed browser using separate tabs, explicit leases, and safe release;
- duplicate Chrome process launches against the same authenticated profile
  remain rejected by default;
- the final handoff is a note for downstream repos, not a downstream repo
  mutation.

## Non-Goals

- Do not edit the AuraCall repo in this plan.
- Do not run AuraCall live-follow mutation as part of this plan.
- Do not make Guacamole/RDP the default lane for downstream services until the
  no-mutation and live sharing gates pass.
- Do not hardcode AuraCall account IDs, profile IDs, URLs, or migration logic
  into generic agent-browser code.
- Do not weaken profile lock safety by allowing independent Chrome process
  groups to share one authenticated profile directory.
- Do not remove public dyndns.org operator ingress or force public URLs into
  local iframe tests.

## Operating Invariants

```text
The service-selected profile directory remains exclusive to one browser
process group. Runtime sharing means retained-browser tab or window sharing,
not duplicate Chrome launches.
```

```text
Route descriptors must carry separate audience roles: local embedding,
dashboard embedding, public operator ingress, health checks, and backward
compatible external URL.
```

```text
Downstream services should be able to request remote-headed hidden browser
work through generic posture fields. Service-specific recipes may select or
extract identity, but the browser lane itself remains reusable across clients.
```

## Subagent Work Allocation

Use subagents by slice. Each subagent should return:

```text
Slice:
Goal:
Files changed:
Contract delta:
No-launch validation:
Live validation:
Downstream impact:
Residual risks:
Next slice readiness:
```

Recommended subagents:

1. Readiness Agent: install drift, remote-view doctor, route pool, display
   access, and privileged helper readiness.
2. Access-Plan Contract Agent: posture preservation, route descriptor output,
   HTTP/MCP/client parity, and generic profile-selection behavior.
3. Runtime Sharing Agent: retained-browser reuse, tab acquisition, duplicate
   launch rejection, tab release, and lease evidence.
4. Live Gate Agent: no-mutation and live synthetic-profile smokes across
   Guacamole/RDP and `chromium-stealthcdp`.
5. Handoff Agent: concise downstream adoption note that describes new
   features, required gates, and client instructions without mutating
   downstream repos.

Slices A and B can run in parallel after they agree on readiness and
access-plan JSON shape. Slice C depends on B's reuse recommendation. Slice D
depends on A-C. Slice E is written only after the generic gates pass.

## Slice A: Remote-View Readiness Baseline

State: DONE

Goal: make the workstation and installed command unambiguously ready for the
remote-headed lane.

Deliverables:

- Resolve or explicitly classify install drift from `agent-browser doctor
  remote-view --json`.
- Ensure `agent-browser install doctor --json` and remote-view doctor agree on
  the active binary, workspace binary, pnpm package binary, version, and
  launch configuration.
- Preserve the existing ready evidence for Guacamole local embed, public
  operator ingress, route display access, privileged helper readiness, and
  many-to-many route pool readiness.
- Make doctor output distinguish readiness blockers from advisory drift that
  does not affect remote-headed route execution.
- Keep route-specific RDP users as an explicit reviewed state if they remain
  necessary for display isolation, rather than stochastic drift.

Acceptance:

- `doctor remote-view` returns `status=ready` without install drift blockers,
  or returns a documented advisory classification with a clear reason.
- Route pool readiness selects at least two distinct Guacamole/RDP route
  candidates with both `localEmbedUrl` and `publicOperatorUrl`.
- The doctor next action is a runnable gate, not manual guesswork.

Suggested validation:

```bash
agent-browser install doctor --json
agent-browser doctor remote-view --json
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-many-to-many-live
git diff --check
```

Completed on 2026-06-19:

- Rebuilt the native release binary with `pnpm build:native`.
- Synchronized the rebuilt Linux binary to:
  - `/home/ecochran76/workspace.local/agent-browser/bin/agent-browser-linux-x64`
  - `/home/ecochran76/.local/bin/agent-browser`
  - `/home/ecochran76/.local/share/pnpm/global/5/node_modules/agent-browser/bin/agent-browser-linux-x64`
- Backed up the prior user-scoped PATH and pnpm global binaries with
  `.pre-plan0038-<timestamp>` suffixes before replacement.
- `agent-browser install doctor --json` now passes with no issues. The current
  executable, PATH command, pnpm global binary, and workspace binary all share
  checksum
  `80ed59f4901fcaed8256198b5c291002d8d848d44d200d89e741e0c96d652f8a`.
- `agent-browser doctor remote-view --json` now passes with `status=ready`,
  no issues, `nextAction=run_many_to_many_live_gate`, route-pool readiness,
  route display access readiness, private display allocator readiness, and
  privileged helper readiness.
- Installed `agent-browser --json service access-plan ...` now uses the fixed
  access-plan CLI path and preserves the requested remote-headed posture.

Validation passed:

```bash
pnpm build:native
agent-browser install doctor --json
agent-browser doctor remote-view --json
pnpm test:rdp-guac-route-pool-readiness
node --check scripts/test-rdp-guac-many-to-many-live.js
pnpm test:rdp-guac-many-to-many-live
agent-browser --json service access-plan --service-name AuraCall --agent-name codex --task-name remote-headed-cutover-proof --target-service-id chatgpt --login-id chatgpt --account-id consult --browser-build stealthcdp_chromium --browser-host remote_headed --view-stream-provider rdp_gateway --control-input-provider manual_attached_desktop --display-isolation private_virtual_display
```

The many-to-many live smoke passed with artifacts at
`/tmp/agent-browser-rdp-guac-many-to-many-2026-06-20T02-19-23-125Z`. During
this validation, the first many-to-many rerun exposed a dashboard authentication
reload race for the second viewer. The harness now waits for authenticated
dashboard state after login before checking route tiles.

## Slice B: Access-Plan Posture Preservation

State: DONE

Goal: make no-launch access-plan output preserve generic remote-headed
requests instead of falling back to local CDP posture.

Deliverables:

- Add or harden access-plan tests where explicit request fields select
  `stealthcdp_chromium`, `remote_headed`, `rdp_gateway`,
  `manual_attached_desktop`, and `private_virtual_display`.
- Ensure the resolved `decision.launchPosture`,
  `decision.profileReuse`, and `decision.serviceRequest.request` carry the
  requested posture when the route is ready.
- Ensure CLI, HTTP, MCP, and generated client helpers serialize the same
  posture fields.
- Return structured route descriptors in the access-plan or linked route
  hints so clients know the local embed URL, public operator URL, health URL,
  and dashboard embed URL.
- Keep site-policy or profile policy overrides visible in the response when
  they intentionally override a caller request.

Acceptance:

- A generic request with the target posture does not resolve to
  `local_headed`, `cdp_screencast`, `cdp_input`, or `displayIsolation=null`
  unless a structured policy reason names the override.
- A service request copied from access-plan carries top-level route hints and
  action parameters that are contract-clean.
- No AuraCall-specific fields are required to pass the test.

Suggested validation:

```bash
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:service-client-contract
pnpm test:service-client-types
agent-browser --json service access-plan \
  --service-name CutoverProof \
  --agent-name codex \
  --task-name no-mutation-remote-headed \
  --target-service-id synthetic-auth \
  --login-id synthetic-auth \
  --account-id synthetic \
  --browser-build stealthcdp_chromium \
  --browser-host remote_headed \
  --view-stream-provider rdp_gateway \
  --control-input-provider manual_attached_desktop \
  --display-isolation private_virtual_display
git diff --check
```

Completed on 2026-06-19:

- Fixed the CLI command path for `service access-plan` so global launch posture
  flags consumed by `clean_args()` are still copied into the service
  access-plan command. This covers `--browser-host`,
  `--view-stream-provider`, `--control-input-provider`, and
  `--display-isolation`.
- Added a command-layer regression test that simulates the real CLI pipeline:
  parse global flags, clean args, then parse `service access-plan`.
- Verified the rebuilt workspace CLI with an AuraCall-shaped no-launch command.
  The response now preserves `browserHost=remote_headed`,
  `viewStreamProvider=rdp_gateway`,
  `controlInputProvider=manual_attached_desktop`, and
  `displayIsolation=private_virtual_display` in `query`,
  `decision.launchPosture`, `decision.profileReuse`, and
  `decision.serviceRequest.request.params`.

Validation passed:

```bash
cargo test --manifest-path cli/Cargo.toml test_service_access_plan_preserves_global_remote_view_flags_after_cleaning -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan_uses_requested_remote_view_posture -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml test_command_executes_service_access_plan_locally_before_daemon -- --test-threads=1
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm test:service-api-mcp-parity
pnpm test:service-client-contract
pnpm test:service-client-types
cargo build --manifest-path cli/Cargo.toml
./cli/target/debug/agent-browser --json service access-plan --service-name AuraCall --agent-name codex --task-name remote-headed-cutover-proof --target-service-id chatgpt --login-id chatgpt --account-id consult --browser-build stealthcdp_chromium --browser-host remote_headed --view-stream-provider rdp_gateway --control-input-provider manual_attached_desktop --display-isolation private_virtual_display
```

Boundary:

- Keep the retained-browser reuse and route descriptor work in Slice C and
  Slice D; this step only proves that requested posture survives the CLI
  no-launch access-plan surface.

## Slice C: Retained Remote-Headed Reuse

State: DONE

Goal: when a compatible retained browser exists, access-plan and
service-request should reuse it through shared tabs instead of recommending a
new profile process.

Deliverables:

- Start or adopt one retained remote-headed browser for a synthetic
  authenticated profile.
- Refresh access-plan and assert
  `profileReuse.recommendedAction=reuse_existing_browser`.
- Require `sharedAcquisition.mode=tab_new`, `browserId`, and `sessionName`
  when a compatible retained holder exists.
- Ensure `requestServiceTabFromAccessPlan` and the no-launch example copy the
  retained route hints.
- Keep second independent launches against the same authenticated profile
  rejected unless explicitly marked as reviewed throwaway or isolated work.

Acceptance:

- Two clients can obtain distinct service-owned tab handles in one retained
  browser process group.
- Release of tab A reports that tab A was closed or marked released without
  closing tab B or the browser.
- Tab B remains evaluable after tab A release.
- Duplicate process pressure is visible in service incidents or response
  evidence when a caller ignores route hints.

Suggested validation:

```bash
cargo test --manifest-path cli/Cargo.toml tab_new_shared_acquisition -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml tab_handle_release -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_profile_lease_gate_allows_duplicate_lane_route_hints -- --test-threads=1
pnpm test:service-request-live
git diff --check
```

Completed on 2026-06-19:

- Hardened isolated remote-headed smokes so they preserve the real configured
  `chromium-stealthcdp` manifest through
  `AGENT_BROWSER_STEALTHCDP_CHROMIUM_MANIFEST_PATH` when using a temporary
  `AGENT_BROWSER_HOME`.
- Updated the duplicate-profile live check to use a supported `navigate`
  service request without retained route hints, rather than the obsolete
  unsupported `launch` service-request action.
- `pnpm test:service-request-live` now passes. The smoke proves:
  - first remote-headed service request launches the selected authenticated
    profile;
  - access-plan recommends `reuse_existing_browser` with
    `sharedAcquisition.mode=tab_new`;
  - client helpers copy retained `browserId` and `sessionName` route hints;
  - a second shared tab opens through the retained browser;
  - an independent duplicate profile lane is rejected;
  - releasing tab A preserves the browser/session route;
  - tab B remains usable after tab A release.

Validation passed:

```bash
node --check scripts/smoke-remote-headed-utils.js
node --check scripts/smoke-service-request.js
pnpm test:service-request-live
```

## Slice D: Live No-Mutation Cutover Gate

State: DONE

Goal: prove the full remote-headed lane with synthetic or explicitly test
profiles before any downstream live-follow mutation.

Deliverables:

- Add a named live smoke that runs the no-mutation cutover proof end to end:
  readiness, access-plan, retained launch or reuse, two tab acquisitions,
  route descriptor inspection, duplicate launch rejection, tab release, and
  surviving-tab evaluation.
- Prefer `chromium-stealthcdp` and Guacamole/RDP for this smoke.
- Avoid private downstream site mutation; use synthetic pages or read-only
  identity/account detection probes.
- Capture artifact paths for screenshots, route descriptors, service traces,
  and relevant job or incident IDs without storing credentials or cookies.
- Classify WSL pre-DevTools failures as lane health blockers with stderr log
  paths and remediation hints.

Acceptance:

- The smoke passes from a clean service state without manual stale-lock
  cleanup.
- The smoke proves both local embed route and public operator route readiness.
- The smoke does not depend on AuraCall profile names or private account data.
- The smoke can be run by CI/manual operators as a cutover gate.

Suggested validation:

```bash
pnpm test:remote-headed-cutover-proof-live
pnpm test:rdp-guac-route-pool-readiness
pnpm test:rdp-guac-many-to-many-live
pnpm test:service-request-live
git diff --check
```

Completed on 2026-06-19:

- Added the named manual cutover gate
  `pnpm test:remote-headed-cutover-proof-live`.
- The gate chains route-pool readiness, the Guacamole/RDP many-to-many live
  proof, and the service-request HTTP/MCP live proof.
- The route-pool proof verified both local embed and public operator Guacamole
  routes. The public operator URL was
  `https://agent-browser.ecochran.dyndns.org/guacamole/`.
- The many-to-many proof passed with artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-06-20T02-29-41-100Z`.
- The service-request proof passed after preserving the real
  `chromium-stealthcdp` manifest for isolated smoke homes and updating the
  duplicate-profile lane check to use a supported `navigate` service request.

Validation passed:

```bash
pnpm test:remote-headed-cutover-proof-live
```

## Slice E: Downstream Handoff Note

State: DONE

Goal: give AuraCall and other clients clear adoption guidance after
agent-browser proves the generic lane.

Deliverables:

- Write an agent-browser repo handoff note under `docs/dev/notes/` describing:
  - the new generic posture fields;
  - the no-launch access-plan proof command;
  - the live cutover gate command;
  - how clients should prefer Guacamole/RDP plus `chromium-stealthcdp`;
  - how clients should request retained tab acquisition rather than launch a
    duplicate browser;
  - how identity/account detection should use generic routines plus
    service-specific recipes.
- State explicitly that downstream repos decide their own migrations.
- Include a short AuraCall-specific section only as an example consumer, not as
  code or policy in agent-browser.

Acceptance:

- Handoff cites passing generic validation evidence.
- Handoff has no secrets, cookies, profile artifacts, or private site state.
- Handoff does not require agent-browser to edit AuraCall.

Suggested validation:

```bash
pnpm validation:select -- --base HEAD
git diff --check
```

Completed on 2026-06-19:

- Wrote `docs/dev/notes/2026-06-19-remote-headed-cutover-proof-handoff.md`.
- The note describes the generic posture fields, no-launch access-plan proof,
  live cutover gate, retained tab acquisition expectations, profile-sharing
  boundary, identity/account detection shape, and downstream migration
  boundary.
- The note uses AuraCall only as an example consumer and does not mutate the
  AuraCall repo.

## Done Definition

- Remote-view readiness is deterministic and doctor output is actionable.
- Access-plan preserves requested remote-headed `chromium-stealthcdp` posture
  across all public surfaces.
- Runtime profile sharing prefers retained-browser tabs and rejects duplicate
  profile process pressure.
- A live cutover proof passes without private downstream mutation.
- The next AuraCall instruction can safely say to prefer Guac/RDP and
  `chromium-stealthcdp` only after it checks the agent-browser proof gate.

## Closeout Validation

Passed on 2026-06-19:

```bash
pnpm test:remote-headed-cutover-proof-live
git diff --check
pnpm validation:select -- --base HEAD
```

`pnpm validation:select -- --base HEAD` reported a broad recommendation set
because the worktree includes prior Plan 0033-0037 service, dashboard, docs,
and client changes. The Plan 0038-specific gates completed in this plan are
the remote-view doctor, install doctor, access-plan Rust tests, service
contract/client parity tests, Guacamole/RDP many-to-many live proof, and
service-request HTTP/MCP live proof.

Follow-up repair on 2026-06-19:

- Fixed `agent-browser doctor remote-view --json` so installed invocations from
  downstream repo directories resolve the agent-browser remote-view helper
  scripts from the agent-browser checkout or package root instead of the
  caller's current working directory.
- Added unit coverage for repo-root and direct script-root resolution and for
  absolute helper script arguments.
- Rebuilt and reinstalled the user-scoped binary. The workspace binary,
  `/home/ecochran76/.local/bin/agent-browser`, and the pnpm global package
  binary all share checksum
  `757efd42ff837d69e397003d8af5df7b46cb7e4b1eeea85f11d9375853c81fea`.
- Verified from `/home/ecochran76/workspace.local/auracall` that installed
  `agent-browser doctor remote-view --json` returns `status=ready`, resolves
  `scriptRoot=/home/ecochran76/workspace.local/agent-browser/scripts`, reports
  route-pool, RDP gateway, and route-display helper success, reports
  privileged helper readiness, and returns no issues.

Follow-up repair on 2026-06-20:

- Fixed the service-request execution safety boundary for copied access-plan
  requests. If a service request names `runtimeProfile` or `profile` and the
  active browser in that daemon session is using a different runtime profile
  or user-data directory, agent-browser now fails before executing browser
  actions instead of silently opening a tab in the wrong authenticated profile.
- This directly addresses the AuraCall Plan 0142 blocker where
  `decision.serviceRequest.request` selected
  `auracall-chatgpt-wsl-chrome-2-consult` but execution landed in
  `session:default` / `runtimeProfile:default`.
- Rebuilt and reinstalled the user-scoped binary. The workspace binary,
  `/home/ecochran76/.local/bin/agent-browser`, and the pnpm global package
  binary all share checksum
  `4ae6838cd3c8b4de3341b4dc8b884e2dc14db62269212427e55b8239e4e4b6df`.
- Verified from `/home/ecochran76/workspace.local/auracall` that installed
  `agent-browser doctor remote-view --json` still returns `status=ready`,
  privileged helper readiness, no issues, and
  `nextAction=run_many_to_many_live_gate`.
