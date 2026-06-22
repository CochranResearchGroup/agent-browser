# Roadmap

Date: 2026-05-26

This file is the top-level planning index for durable agent-browser lanes.
Detailed research notes and validation reports remain under `docs/dev/notes/`;
bounded implementation and validation plans remain under `docs/dev/plans/`.

## P01 | Remote View Backend Reliability

State: CLOSED

### Current State

- Plan `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
  is closed with a validated RDP and Guacamole reliability gate.
- The validated handoff is recorded in
  `docs/dev/notes/2026-05-26-rdp-guac-slice-e-reliability-gate.md`.
- This lane validates the current RDP and Guacamole deployment as a
  supportable full-control path. It does not by itself switch default backend
  settings.
- CDP streaming and VNC/noVNC remain separate future backend campaign items.

### Evidence

- `docs/dev/notes/2026-05-26-remote-view-backends-campaign.md`
- `docs/dev/notes/2026-05-26-rdp-guac-slice-a-ownership-audit.md`
- `docs/dev/notes/2026-05-26-rdp-guac-slice-b-live-validation.md`
- `docs/dev/notes/2026-05-26-rdp-guac-slice-c-live-validation.md`
- `docs/dev/notes/2026-05-26-rdp-guac-slice-d-live-validation.md`
- `docs/dev/notes/2026-05-26-rdp-guac-slice-e-reliability-gate.md`

### Next Recommendation

Keep P01 closed unless a release gate regresses. Open a new lane for CDP
streaming or VNC/noVNC rather than reopening this RDP and Guacamole lane for
unrelated backend families.

## P02 | Guacamole Remote View Routing Hardening

State: CLOSED
Current state: P02 route authority, takeover-event, and shared-route
RDP/Guacamole validation are complete. Distinct-route Guacamole coverage is a
future provider-configuration expansion.

### Current State

- Plan `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
  is closed.
- This lane addresses the post-P01 review findings: hardcoded Guacamole route
  repair, metadata-only `view_takeover`, and external-open behavior that can
  race ahead of the service-owned takeover result.
- P02 keeps RDP and Guacamole as the current full-control path, but requires
  route identity and viewer ownership to become service-owned before calling
  the path hardened for multiple external browser workspaces.
- Production code no longer synthesizes the current workstation Guacamole
  client hash. Service stream records carry route metadata, dashboard external
  open waits for `view_takeover` acceptance, and `view_takeover` persists a
  `viewer_takeover_requested` service event.
- Same-day viewer-transfer and browser-switch live gates passed with the
  configured shared Guacamole route and service-visible route identity.

### Evidence

- `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`
- `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
- `docs/dev/notes/2026-05-26-remote-view-backends-campaign.md`
- `docs/dev/notes/2026-05-27-guac-route-authority-audit.md`
- `docs/dev/notes/2026-05-27-guac-route-hardening-validation.md`

### Next Recommendation

Open a new lane only when a second live Guacamole connection or distinct-route
provider setup is available. Keep P02 closed for the current shared-route
hardening path.

## P03 | Guacamole RDP Many-To-Many Viewing

State: CLOSED
Current state: P03 is complete. The route-pool, private display allocation,
viewer lease, dashboard tiling, reconcile, doctor, and Linux privilege-helper
installer surfaces are implemented, and the OCR-backed many-to-many live gate
passed with two simultaneous Guacamole/RDP browser routes. P03 covers the
distinct-route and private-display provider expansion that P02 intentionally
deferred.

### Current State

- Plan `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
  is closed.
- The target behavior is many browser workspaces and many external viewers:
  each preferred remote-headed browser gets a private virtual display, each
  private display gets a distinct Guacamole/RDP route, and observers and
  controllers are tracked by service-owned viewer leases.
- The current workstation can still use the validated shared Guacamole route
  as an explicit fallback, but shared route behavior is focus switching, not
  simultaneous multi-browser viewing.
- The first supported implementation path is a static Guacamole route pool
  backed by distinct RDP targets. Dynamic Guacamole connection generation can
  come later. `agent-browser doctor remote-view` is now the unified
  doctor/setup discovery surface for install state, existing RDP users,
  Guacamole records, network health, service state, and route-display state.
- P03 Slice B is complete for no-launch service allocation contracts. The
  service model, HTTP read collections, MCP read resources, contract metadata,
  service job audit fields, and client read helpers expose remote-view
  allocation records. Service request actions and generated client helpers
  mutate route checkout, route release, viewer lease request, viewer lease
  release, and controller lease takeover state without launching a browser.
  Dashboard workspace rows, browser details, view-stream cards, and workspace
  viewport headers render route id, display allocation, provider mode, viewer
  count, controller lease, and readiness from typed stream metadata.
- P03 Slice C is complete. Remote-headed launches now default to private
  virtual display allocation, records display allocation ids on browser records
  and view streams, creates per-session private display allocation records,
  keeps explicit shared-display and ambient-display requests modeled as
  non-private scope, releases only the closed browser's owned display
  allocation, and marks owned allocations orphaned when a browser process
  exits. The live private-display smoke passed with two distinct display names.
- P03 Slice D is in progress. `service_remote_view_route_checkout` can select
  compatible static route-pool entries for private display allocations, rejects
  target mismatches and private-route contention, and returns
  `route_pool_unavailable` when no compatible pool entry is available.
  Checkout also rejects explicit failed or stale route-pool readiness with
  `route_pool_not_ready` before marking a route externally viewable. The
  remaining Slice D gates are live provider probes and a live two-entry
  Guacamole route pool smoke with distinct RDP targets.
- P03 Slice E is in progress. Viewer lease heartbeat is a service request
  action, single-viewer routes return typed denial metadata for extra active
  viewers, controller requests return typed denial metadata when another
  controller is active, explicit controller takeover remains auditable, and
  retained service events cover viewer connect/disconnect, controller
  requested/granted/denied, and route release.
- P03 Slice F is in progress. Workspace rows and the remote viewport now score
  retained streams so private pool, generated, or discovered routes outrank
  shared fallback streams, duplicate Guacamole route diagnostics continue to
  explain shared-route contention on affected rows, and `view=workspace:tile`
  renders the top two embeddable service-owned remote routes with independent
  tile refresh and shared-route warnings. Single-workspace view now has
  service-owned recovery controls for route refresh, observer reconnect,
  controller takeover, and retained viewer release. The remaining Slice F gate
  is live rendered inspection with two RDP-capable workspace rows.
- P03 Slice G is complete. `service_reconcile` now repairs remote-view
  allocation drift by orphaning display allocations and routes whose owner
  browser is missing or unhealthy, disconnecting unavailable-route viewer
  leases, expiring stale viewer leases, clearing stale controller references,
  preserving healthy routes, and persisting those reconciled remote-view
  records through the repository merge path. Service incidents now distinguish
  route-pool exhaustion, route unreachable, missing display allocation,
  provider-auth failure, and iframe-blocked readiness from retained
  remote-view state. `service_route_pool_repair` now gives operators a
  dry-run-first service-request action for stale checked-out route-pool
  entries, reporting stale reasons and resetting only stale entries to
  `available` when `apply` is true. The live route-cleanup gate
  `pnpm test:rdp-guac-route-cleanup-live` passed on 2026-05-28 with artifacts
  at `/tmp/agent-browser-rdp-guac-route-cleanup-2026-05-28T04-52-11-882Z`,
  proving stream restart preserves a healthy checkout, browser crash reconcile
  orphans the route, dry-run repair reports one stale checkout, and apply
  returns the pool entry to `available`.
- P03 Slice H now has a guarded live gate script,
  `pnpm test:rdp-guac-many-to-many-live`. The harness is wired into docs and
  requires two distinct route-pool entries before it can launch the full matrix.
  The first invocation failed early with a configuration artifact at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-28T04-59-03-972Z/failure.json`
  because the workstation does not currently expose two distinct Guacamole/RDP
  routes. A follow-up live topology check confirmed the user-scoped environment
  only exposes the shared `AGENT_BROWSER_REMOTE_VIEW_URL`, service state has no
  persisted route pool, and the Guacamole database has one RDP connection to
  host XRDP. P03 now has `pnpm test:rdp-guac-route-pool-readiness` as a
  non-secret preflight for this blocker; it checks the Guacamole Compose
  containers, Guacamole web ingress, guacd-to-RDP TCP reachability, redacted
  connection metadata, and distinct target identity before emitting a route
  pool. Its current `--report-only` output shows Guacamole Postgres,
  Guacamole web ingress, guacd, and the existing host-XRDP backend ready, but
  only one RDP connection and one distinct target identity are available. P03
  also has `pnpm setup:rdp-guac-route-pool` as the interactive provisioning command for
  the first static two-route shape. It creates two local XRDP users and two
  Guacamole RDP connections, but it needs interactive `sudo` and therefore was
  syntax-checked rather than run in the current non-interactive session. P03
  now also has `pnpm sync:rdp-guac-existing-user-route-pool` for the existing
  `agent-browser-rdp` user path. That no-sudo sync created Guacamole
  connections 2 and 3 with color depths 24 and 32, and route-pool readiness
  now selects those managed connections as ready distinct targets. P03
  also has `pnpm inspect:rdp-route-displays` as a non-sudo post-bootstrap
  helper that maps the route users to active XRDP display names and prints the
  display-target variables needed by the many-to-many live gate. The display
  inspector and route-pool readiness smoke can print copyable shell exports
  when run with `--shell`. A follow-up topology check confirmed that
  host-XRDP route creation is only a
  bootstrap: current private browser displays are service-owned Xvfb
  allocations, while host XRDP creates separate login Xorg sessions. The final
  P03 gate must prove each route displays its claimed browser, not merely a
  separate XRDP desktop. The many-to-many live gate now enforces that with
  screenshot crop plus OCR target-binding proof against each tile iframe. It
  also supports the first testable host-XRDP topology: route entries can carry
  distinct display names, and the gate will launch each browser directly onto
  its route's XRDP display before checking out the route.
- P03 is now refocused around a doctor-first setup contract.
  `agent-browser doctor remote-view` composes install doctor, runtime status,
  Guacamole/RDP readiness, route-pool inventory, route-display inspection,
  user-scoped secret key presence, Docker/network checks, and RDP user
  inventory. Current live evidence shows managed Guacamole connections 2 and 3
  are selected route-pool candidates and the route pool is ready, but opening
  both route clients still produced one existing-user XRDP display (`:10`).
  XRDP logs show both clients logged in on display 10 and connected to the
  same Xorg PID. The doctor now recommends an explicit route-specific user or
  XRDP policy isolation fallback instead of further ad hoc Guacamole records.
  `pnpm install:privileges` now installs the narrow root-owned helper and
  `agent-browser` group path for one-time authorization, with sudoers limited
  to the installed helper outside the writable checkout.
  `pnpm setup:rdp-guac-route-pool` is guarded by that route-display evidence
  and refuses to create route-specific users unless the current inspector
  output proves the existing-user route collapsed, or an operator passes a
  reviewed `--force` override. After route-specific sessions exist, `pnpm
  grant:rdp-route-display-access` reports or applies the narrow local X access
  grants needed for the agent user to launch Chrome onto those XRDP-owned
  displays. The CLI installer now includes
  `agent-browser install --with-deps --with-remote-view-privileges` so release
  binaries can install the `agent-browser` group, root-owned helper, and
  sudoers rule with one intentional authorization. The live doctor reports the
  helper ready, the operator user in the group, and no interactive sudo
  required for recurring desktop setup. The OCR-backed many-to-many live gate
  passed on 2026-05-29 with
  route A on display `:12`, route B on display `:11`, local Guacamole frame
  URLs, two dashboard clients, refresh coverage, Browser A close, Browser B
  survival, and route-pool release proof. Artifacts:
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T01-34-49-701Z`.

### Evidence

- `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
- `docs/dev/notes/2026-05-28-guac-rdp-p03-provider-topology-audit.md`
- `docs/dev/plans/0002-2026-05-27-guac-remote-view-routing-hardening-plan.md`
- `docs/dev/notes/2026-05-27-guac-route-hardening-validation.md`
- `docs/dev/notes/2026-05-26-remote-view-backends-campaign.md`

### Next Recommendation

Keep P03 closed unless the live gate regresses. The next release checkpoint
should build a candidate binary, run `agent-browser install doctor`,
`agent-browser doctor remote-view`, and the many-to-many live gate from the
installed candidate.

## P04 | Release Candidate Install Validation

State: CLOSED
Current state: P04 validated the release-candidate checkpoint after P03. The
installed 0.26.1 candidate now proves that the installer-owned remote-view
privilege setup, install doctor, remote-view doctor, default runtime attach
path, and many-to-many Guacamole/RDP live gate work from the operator command
path rather than from the mutable repo checkout.

### Current State

- Plan
  `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`
  is closed.
- P03 proved the feature path on the live host. P04 proved the operator install
  and release surfaces around that path.
- The installed candidate exposes
  `agent-browser install --with-deps --with-remote-view-privileges`, keeps the
  helper root-owned under `/usr/local/libexec/agent-browser`, reports
  `requiresInteractiveSudo=false` from `agent-browser doctor remote-view
  --json`, and passes the many-to-many live gate with the installed command on
  `PATH`.
- The default-profile lock regression is fixed: an implicit
  `agent-browser --json get title` attaches to the live default runtime profile
  instead of launching another Chrome against the locked profile directory.

### Evidence

- `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`
- `docs/dev/notes/2026-05-29-p04-release-candidate-install-validation.md`
- `docs/dev/plans/0003-2026-05-28-guac-rdp-many-to-many-viewing-plan.md`
- `docs/dev/notes/2026-05-28-p03-doctor-first-refactor.md`

### Next Recommendation

Keep P04 closed unless the installed command, doctor surfaces, privilege helper,
or many-to-many live gate regresses. The next slice should keep hardening the
Guacamole/RDP productization milestone rather than turning the checkpoint into
a formal release.

## P05 | Runtime Checkpoint And No-Release Handoff

State: CLOSED
Current state: P05 validated and installed a `0.27.0` roadmap checkpoint
runtime without publishing a formal release.

### Current State

- Plan
  `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`
  is closed.
- The authoritative validation base is `v0.26.1`; `v0.25.4` is not on the
  current `HEAD` ancestry and was not used as the release base.
- The checkpoint runtime version is `0.27.0`.
- Version metadata is synchronized across `package.json`, `cli/Cargo.toml`,
  `cli/Cargo.lock`, and `packages/dashboard/package.json`.
- `CHANGELOG.md` keeps current work under `## Unreleased`, release extraction
  markers remain around the latest published `0.26.1` entry, and
  `docs/src/app/changelog/page.mdx` does not list a public `v0.27.0` release.
- The GitHub Actions `Release` workflow is manual-only so ordinary pushes to
  `main` cannot publish a GitHub release accidentally.
- Selected validation passed, including Rust format, clippy, focused Rust
  service tests, service API/MCP parity, browser capability registry draft,
  service client, docs build, dashboard tests, dashboard build, installed skill
  sync, install doctor, remote-view doctor, default-profile attach, and the
  OCR-backed many-to-many live gate.
- The installed 0.27.0 checkpoint checksum is
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`.

### Evidence

- `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`
- `docs/dev/notes/2026-05-29-p05-release-preparation-validation.md`
- `docs/dev/notes/2026-05-29-p05-validation-selector.txt`
- `docs/dev/plans/0004-2026-05-29-release-candidate-install-validation-plan.md`
- `docs/dev/notes/2026-05-29-p04-release-candidate-install-validation.md`

### Next Recommendation

Proceed to P06. The next lane should harden the installer, doctor, route-pool,
Guacamole/RDP preflight, and many-to-many operational evidence needed before a
formal release milestone.

## P06 | Guacamole RDP Productization Hardening

State: CLOSED
Current state: P06 validated the Guacamole/RDP productization hardening
milestone without publishing a formal release.

### Current State

- Plan
  `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`
  is closed.
- The rebuilt installed checkpoint runtime passes install doctor, remote-view
  doctor, and the many-to-many live gate from the installed command.
- `agent-browser install doctor --json` now reports remote-view privilege
  readiness with helper, sudoers, group, membership, helper check, nested issue
  fields, service readiness from a no-launch service-status probe, and
  `requiresInteractiveSudo=false` on the provisioned machine.
- The privilege installer now exits before privileged changes on an
  already-provisioned machine when the helper, sudoers file, group, membership,
  and non-interactive helper check are ready.
- `agent-browser doctor remote-view --json` now reports stable top-level issue
  codes, viewer browser and OCR prerequisites, privilege readiness, route-pool
  readiness, route displays, display access, and many-to-many readiness.
- The many-to-many harness now hydrates route-pool and route-display
  environment from doctor output, auto-discovers common viewer browsers,
  prefers installed `agent-browser`, and classifies public Guacamole route URLs
  with `non_embeddable_guacamole_url`.
- `pnpm test:install-privileges-clean-fixture` proves the clean reset-fixture
  first-apply privilege installer path uses exactly one `sudo -v` boundary and
  the second apply performs only a non-interactive helper readiness check.
- `agent-browser install --with-deps --with-remote-view-privileges` now runs
  remote-view privilege setup before Linux dependency installation, so the
  explicit helper authorization boundary comes first.
- Route-pool readiness passed after restarting `agent-browser-guacamole` and
  `agent-browser-guacd`.
- The final installed 0.27.0 checkpoint checksum for P06 is
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.
- P06 did not publish a formal release, move release markers, or add a public
  `0.27.0` docs changelog entry.

### Evidence

- `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`
- `docs/dev/notes/2026-05-29-p06-installer-doctor-productization.md`

### Next Recommendation

Keep P06 closed unless install doctor, remote-view doctor, route-pool
readiness, or the many-to-many live gate regresses. Open a separate formal
release lane when the maintainer wants to prepare and publish a release.

## P07 | v0.27.0 Formal Release

State: CLOSED
Current state: `v0.27.0` is released. The public GitHub release exists with
all seven expected platform assets.

### Current State

- Plan
  `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`
  is closed.
- P06 closed the operational milestone that kept P05 from publishing a public
  release.
- This lane moves the validated `0.27.0` checkpoint into release metadata,
  validation, PR merge, and GitHub release publication.
- Release-preparation validation passed and is recorded in
  `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.
- Early release workflow dry runs failed on cross-target Rust compile errors
  and Linux X11 linking; the fix note is
  `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.
- The successful dry run and real release workflow both ran against
  `17a284f8624e6108473970e2ec2b380debf9f7ac`.
- GitHub release:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.27.0`

### Evidence

- `docs/dev/plans/0005-2026-05-29-runtime-checkpoint-and-no-release-handoff-plan.md`
- `docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`
- `docs/dev/plans/0007-2026-05-29-v0-27-0-formal-release-plan.md`
- `docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`
- `docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`

### Next Recommendation

Keep P07 closed unless the published assets or release tag need correction.
Start a new lane for any post-release patch or next-version work.

## P08 | CDP Tab Streaming For Non-Remote Browsers

State: OPEN
Current state: P08 is the next feature-planning lane after the `v0.27.0`
release. Existing runtime streaming already uses CDP screencast, but
service-owned non-remote browsers do not yet advertise dashboard-openable,
tab-focused `cdp_screencast` view streams.

### Current State

- Plan
  `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`
  is open.
- P03 through P07 hardened remote-headed Guacamole/RDP viewing and release
  delivery. P08 intentionally targets local or attached CDP-controllable
  browsers that do not need a remote desktop route.
- Existing source has `StreamServer`, CDP `Page.startScreencast`,
  `ViewStreamProvider::CdpScreencast`, and dashboard view-stream rendering.
  The missing work is service-state ownership, readiness, tab focus, and
  dashboard-openable URLs for non-remote browsers.

### Evidence

- `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`
- `cli/src/native/stream/mod.rs`
- `cli/src/native/stream/cdp_loop.rs`
- `cli/src/native/stream/websocket.rs`
- `cli/src/native/service_model.rs`
- `packages/dashboard/src/components/service-panel.tsx`

### Next Recommendation

Start P08 Slice A with a contract and ownership audit before editing runtime
streaming code.

## P13 | Resource Monitor And Garbage Collector

State: OPEN
Current state: P13 has cleanup visibility in place and is moving to
profile/browser sprawl prevention.

### Current State

- Plan
  `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
  is closed with read-only resource inventory, guarded GC apply, dashboard
  visibility, timer summary output, and install doctor resource warnings.
- Plan
  `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`
  is open to make access-plan and launch behavior promote the minimal necessary
  number of runtime profiles for simultaneous account, website, browser-build,
  and remote-view isolation sets.
- The 2026-06-04 cleanup found stale multi-day `chromium-stealthcdp` process
  groups, orphaned Xvfb displays, stale no-argument `agent-browser` daemon
  siblings, and stale default runtime-state pointers.
- The live dashboard service remained healthy, but stale resources outside the
  service MainPID consumed high CPU and several GB of memory.
- Existing retained-state cleanup covers stale service records and custom
  profile metadata. P13 covers live OS resource inventory, stale process
  classification, dry-run GC, guarded apply, dashboard resource-pressure
  visibility, and prevention of avoidable duplicate runtime profile/browser
  lanes.

### Evidence

- `docs/dev/plans/0026-2026-06-04-resource-monitor-and-garbage-collector-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`
- `docs/dev/plans/0010-2026-05-30-retained-orphan-profile-cleanup-plan.md`
- `docs/dev/plans/0025-2026-06-01-remote-view-target-attribution-and-idle-display-plan.md`

### Next Recommendation

Start Plan 0027 Slice A with a read-only access-plan `profileReuse` advisory.
The broker should explain whether the minimal-profile path is to reuse an
existing browser, wait for the selected profile lease, or launch a new browser
because isolation actually requires it.

## P14 | AuraCall Service CDP Upgrade

State: OPEN
Current state: P14 is a high-level migration-support lane for service-owned
profile origin, tab handles, controlled CDP attach, bounded evaluate,
diagnostics, readiness evidence, and client ergonomics.

### Current State

- Plan
  `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`
  is open.
- The motivating downstream user is AuraCall, but the lane is intentionally
  framed as generic agent-browser service primitives rather than
  provider-specific AuraCall scraping logic.
- The handoff note
  `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md` records the
  requested feature set and links the relevant sibling AuraCall source paths.
- Existing access-plan and service-request contracts provide the foundation.
  Slices A through D now provide explicit profile-origin and BYOP registration
  semantics, lease-backed service tab handles, policy-gated CDP attach/detach
  helpers, and bounded evaluate service requests. Slice E has started with a
  compact diagnostics service request and generated client helper for valid
  service tab handles. Software clients still need readiness evidence and
  migration ergonomics before migrating raw CDP provider code safely.

### Evidence

- `docs/dev/notes/2026-05-09-access-plan-service-request-handoff.md`
- `docs/dev/notes/2026-06-13-auracall-cdp-feature-requests.md`
- `docs/dev/plans/0033-2026-06-13-auracall-service-cdp-upgrade-plan.md`
- `docs/dev/plans/0027-2026-06-05-minimal-runtime-profile-reuse-plan.md`

### Next Recommendation

Continue P14 Slice E with readiness/freshness lifecycle gating. Keep focused
live smokes for attach-read-detach, bounded evaluate, and diagnostics evidence
capture as validation follow-up before treating the AuraCall migration bridge
as live-proven.

## P16 | Remote Control Ready Command

State: CLOSED
Current state: P16 is closed. The route-specific `remote_view_open` path is
live-proven, documented, and handed off for downstream clients.

### Current State

- Plan
  `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`
  is closed.
- The motivating live failure loaded LinkedIn successfully in
  `stealthcdp-default`, but the operator first saw a Guacamole error document
  because the Guacamole PostgreSQL schema was missing, then saw only an
  `xterm` because the browser was on hidden Xvfb display `:90` while the
  external Guacamole route showed XRDP display `:10`.
- The route-specific live path now proves the desired outcome through
  `remote-view open`: the selected route-pool entry resolves to Guacamole
  connection `3`, route `guacamole:3`, display `:11`, and display allocation
  `remote-view-display:11`.
- `agent-browser doctor remote-view --json` reports `status=ready`,
  `remoteControl.status=ready`, and `manyToMany.status=ready` for the current
  route-pool topology.
- `remote_view_open` now grants route-display access through the installed
  privileged helper when needed before launching on the selected route display.
- `remote-view open` accepts the documented `--browser-build
  stealthcdp_chromium` and `--provider rdp_gateway` flags, and post-launch
  route verification failures clean up before returning the typed error.
- Downstream handoff is recorded in
  `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`.

### Evidence

- `docs/dev/plans/0036-2026-06-18-rdp-ready-to-go-plan.md`
- `docs/dev/plans/0038-2026-06-19-remote-headed-cutover-proof-plan.md`
- `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`
- `docs/dev/notes/2026-06-21-remote-view-open-route-specific-handoff.md`

### Next Recommendation

Keep P16 closed. Downstream clients should adopt the generic `remote_view_open`
path and run the required remote-view doctor, fixture, and many-to-many gates
in their own environment before changing browser-owner defaults.

## P42 | Runtime Convergence

State: CLOSED
Current state: P42 closed after making dashboard, daemon sessions, route
helpers, service state, and live workspace rows converge on one explicit
runtime identity.

### Current State

- Plan `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md` is closed.
- Install doctor now reports active runtime inventory, live dashboard runtime
  readiness, explicit runtime convergence summary states, stale daemon
  executable drift, and stale stream-backend drift.
- Daemon reuse compares executable SHA-256, not only package version.
- The dashboard live rail excludes retained/no-action diagnostic records and
  groups detected non-owned CDP browsers separately.
- `pnpm converge:local-runtime -- --apply --json` is the bounded local repair
  command for publish/restart, stale daemon remedies, Guacamole schema guard,
  route-pool readiness, and route display-access grants.
- Final installed readbacks reported install doctor ready, remote-view ready,
  `runtimeConvergence.status=converged`, and route-pool readiness
  `success=true`.

### Evidence

- `docs/dev/plans/0040-2026-06-21-dashboard-binary-harmonization-plan.md`
- `docs/dev/plans/0041-2026-06-22-foreign-cdp-browser-discovery-and-control-plan.md`
- `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`

### Next Recommendation

Keep P42 closed. Downstream work should use the convergence command and doctor
readbacks before live browser work, then proceed to the many-to-many
Guacamole/RDP live gate and P41 foreign-CDP browser management without turning
non-owned browser addressability into agent-browser lifecycle ownership.

## P43 | Route Handoff Confusion Audit

State: IN PROGRESS
Current state: P43 is the active audit lane for the Facebook remote-view
incident where the route infrastructure was ready and CDP targets existed, but
the dashboard still presented terminal-only Guacamole views for active browser
rows. Slice G is complete; the remaining work is the repeatable no-launch and
live gate layer.

### Current State

- Plan
  `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`
  is open.
- Slice A is complete. `pnpm audit:route-handoff -- --json` now emits the
  read-only route-handoff audit artifact for active browsers, tabs, displays,
  routes, route-pool entries, viewer leases, runtime convergence, stream URLs,
  and retained visual proof.
- Slice B is complete. `agent-browser remote-view open --help` now shows the
  route-bound one-liner, flag-placement guidance, and session versus
  session-name distinction, and parser coverage preserves post-subcommand
  runtime/profile/session-state flags.
- Slice C is complete. `route_pool_unavailable`,
  `route_pool_entry_missing`, and `route_pool_entry_unavailable` now keep
  stable error codes and append compact diagnostic JSON with requested
  route/display/provider identity, matching and available pool entries, ready
  display allocations, existing remote-view routes, and recommended commands.
- Slice D is complete. Chrome profile-lock failures now append diagnostic JSON
  with lock PID, runtime-profile and service-browser ownership matches, primary
  owner, and safe reuse, close, inspect, or separate-profile remedies.
- Slice E is complete. `remote-view open` now returns top-level
  `operatorVisible` proof. Dry-runs report `not_checked`; successful opens
  report `ready` with route, browser, session, display, provider, and visible
  proof identity.
- Slice F is complete. Dashboard workspace rows now carry operator-visible
  route-proof state, require browser-window proof before RDP gateway View,
  Control, or external open actions, and keep terminal-only or missing-proof
  route rows as disabled live diagnostics rather than no-action attention
  entries.
- Slice G is complete. Service-client route-bound remote-view helpers now
  require `operatorVisible.state=ready` before non-dry-run handoff success,
  expose a compact route, tab, profile, and visual-proof summary helper, and
  keep infrastructure-only readiness as an explicit caller opt-in.
- `last30days` now calls the route-bound `agent-browser remote-view open`
  one-liner for Facebook, uses the `last30days-facebook` runtime profile, and
  rejects missing-proof, CDP-only, or terminal-only Guacamole/RDP handoff
  success before scraping.
- The incident note is
  `docs/dev/notes/2026-06-22-facebook-remote-view-open-friction.md`.
- Live readback on 2026-06-22 showed `session:default` on profile
  `last30days-facebook`, display `:11`, and generic `rdp_gateway` stream
  metadata, plus a separate LitScout browser on display `:93` with multiple
  `127.0.0.1` tabs.
- The current gap is not binary convergence. P42 remains green. The gap is
  route, browser, tab, stream, and operator-visible proof convergence.

### Evidence

- `agent-browser doctor remote-view --json` reported remote-view ready and
  still recommended the OCR-backed many-to-many gate as the next proof.
- `agent-browser service browsers --json` reported Facebook and LitScout as
  separate active remote-headed browser rows with generic Guacamole stream
  URLs.
- CodeGraph inspection identified the route-binding path in
  `cli/src/native/actions.rs` and the dashboard stream helper in
  `packages/dashboard/src/lib/service-view-streams.ts` as the key audit joins.

### Next Recommendation

Execute P43 Slice H next. Add the no-launch route-confusion fixtures and the
OCR-backed live route gate so terminal-only route displays fail before an agent
or downstream tool can report Facebook or any other Guacamole/RDP handoff as
successful.
