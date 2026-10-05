# RDP Browser Deterministic Refactor Plan

Date: 2026-06-22
State: COMPLETE
Lane: P44
Depends On:
- `docs/dev/plans/0039-2026-06-20-remote-control-ready-command-plan.md`
- `docs/dev/plans/0040-2026-06-21-dashboard-binary-harmonization-plan.md`
- `docs/dev/plans/0041-2026-06-22-foreign-cdp-browser-discovery-and-control-plan.md`
- `docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md`
- `docs/dev/plans/0043-2026-06-22-route-handoff-confusion-audit-plan.md`
- `docs/dev/notes/2026-06-22-rdp-browser-determinism-audit.md`

## Purpose

Refactor remote-view browser acquisition so "open this URL in an
operator-visible RDP browser" is one deterministic, explainable service
transaction instead of a chain of partially overlapping launch, route, display,
tab, proof, and dashboard behaviors.

The desired outcome is code that can be explained in one sentence:

> Agent Browser normalizes an operator-visible browser intent, selects or
> reuses exactly one compatible service-owned route-bound browser, acquires a
> route and display atomically, opens or refreshes one target tab, proves the
> selected browser is visible through the operator route, then publishes one
> canonical dashboard state.

## Problem Statement

The current implementation has useful parts, but they are organized around
several independent recovery surfaces:

- CLI parsing and provider selection;
- profile and browser reuse;
- route-pool selection;
- display allocation;
- browser launch or attach;
- tab creation and focus;
- visible-window proof;
- Guacamole route checkout;
- incident state;
- dashboard left-rail grouping and stream selection.

That organization lets stale or partial facts outrank the user's intent. A
Chrome process can be ready while the route is terminal-only. A Guacamole URL
can be ready while the selected tab is stale. A direct remote-headed launch can
lock the profile before the route-bound path runs. The dashboard can render a
daemon CDP stream or stale service row as if it were the route-bound operator
handoff.

P44 should replace this with a small set of explicit concepts and a strict
state machine.

## Target Concepts

### `RemoteViewOpenIntent`

The normalized request. It should contain only user or client intent:

- URL;
- profile identity and login identity hints;
- service, agent, and task labels;
- browser build;
- session hint;
- view stream provider;
- control input provider;
- tab acquisition policy;
- reuse policy;
- route preference, if explicitly supplied.

It must reject ambiguous provider fields. `rdp_gateway` is a view-stream
provider, not a cloud CDP provider.

### `RemoteViewAcquisitionPlan`

The no-mutation plan. It should answer:

- which profile is requested;
- whether an existing browser can be reused;
- which route-pool entry will be reserved;
- which display allocation will be used;
- whether the route desktop is clean enough to launch;
- what tab action will happen;
- what proof is required before success;
- what cleanup will happen on failure.

This should power both dry-run output and the fast preflight used by the
one-liner.

### `RouteBoundBrowserLease`

The pending and final service-owned ownership record for one operator-visible
browser. It binds:

- browser ID;
- session ID;
- profile ID;
- route ID;
- route-pool entry ID;
- display allocation ID;
- display name;
- PID and CDP endpoint;
- stream provider and control provider;
- selected tab target ID;
- operator-visible proof.

### `OperatorVisibleProof`

The success oracle. It must be stronger than "some browser window exists":

- CDP target proof for the selected target ID and expected URL;
- X11 proof that the browser window for that process is mapped, visible,
  focused or topmost, and not terminal-obscured;
- route proof that route, pool entry, display allocation, browser, profile, and
  session agree;
- Guacamole route proof that the local and public operator URLs are routable;
- optional frame or pixel proof when available.

### `WorkspaceInventoryClass`

The dashboard grouping model:

- service-owned controllable browsers;
- detected non-owned addressable CDP browsers;
- diagnostics and history.

Stale records, resolved incidents, inactive browsers, viewer clients, and
"needs attention" rows without a user action do not belong in the live control
rail.

## Architectural Direction

Create an explicit remote-view orchestration layer instead of continuing to
grow `handle_remote_view_open` as a large procedure.

Recommended Rust module layout:

- `cli/src/native/remote_view/intent.rs`
  - parse and validate `RemoteViewOpenIntent`;
  - convert CLI, service request, and access-plan payloads into one shape.
- `cli/src/native/remote_view/planner.rs`
  - build `RemoteViewAcquisitionPlan` from service state and intent;
  - strict route, profile, browser, and tab reuse rules;
  - no mutation.
- `cli/src/native/remote_view/lease.rs`
  - reserve and finalize route-pool entry, display allocation, and browser
    lease records;
  - rollback pending state on failure.
- `cli/src/native/remote_view/proof.rs`
  - CDP, X11, route, and Guacamole proof helpers;
  - terminal-only, terminal-topmost, stale-target, proof-missing, and
    route-mismatch diagnostics.
- `cli/src/native/remote_view/open.rs`
  - small coordinator that executes the state machine.

Keep `cli/src/native/remote_view.rs` as a compatibility facade during the
migration, then split it once call sites are stable.

Recommended dashboard layout:

- keep URL and stream helpers in `packages/dashboard/src/lib/`;
- add a single inventory classifier used by left rail and workspace viewport;
- make `workspace-remote-viewport.tsx` consume already-classified state rather
  than re-deciding ownership from URL/session shape.

## Operating Invariants

- A route-bound handoff succeeds only when `operatorVisible.state=ready`.
- Direct remote-headed launch is never the fallback for a route-bound request.
- Existing browser reuse requires profile, browser build, host, route,
  display, stream provider, control provider, and ownership agreement.
- Existing display allocation reuse requires route-pool, route, browser,
  session, profile, and fresh proof agreement.
- `doctor remote-view` is not the launch critical path. The one-liner uses a
  fast preflight with freshness metadata and typed blockers.
- Route desktop terminal windows are never considered a healthy control
  surface.
- Dashboard workspace URLs must recover or reject stale target IDs before
  rendering a control viewport.
- Foreign CDP browsers are addressable inventory, not service-owned lifecycle
  objects.
- Incident resolution must remove or demote rows from live control surfaces.

## Implementation Slices

### Slice A: Intent Normalization And Parser Harmonization

Goal: make every route-bound open request enter the same semantic path.

Deliverables:

- Add `RemoteViewOpenIntent`.
- Convert CLI `remote-view open`, service action `remote_view_open`, and
  generated client requests into this shape.
- Resolve the `--provider rdp_gateway` ambiguity:
  - either accept it as a subcommand-local alias for
    `--view-stream-provider rdp_gateway`; or
  - remove that form from all docs and make the error explicitly say to use
    `--view-stream-provider`.
- Reject cloud-provider and view-stream-provider ambiguity before launch.
- Add command-position and global-position parser tests.

Acceptance:

- The documented one-liner has exactly one parse result regardless of whether
  flags appear before or after `remote-view open`, where supported.
- `rdp_gateway` never reaches the cloud provider connector path.
- Help, README, docs site, skill guidance, and generated client examples agree.

Validation:

- Focused Rust parser tests for `remote-view open`.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`.
- `pnpm validation:select -- --base HEAD` to confirm any added docs or client
  gates.

Progress:

- 2026-06-22: Added `RemoteViewOpenIntent` normalization for
  `remote_view_open`, routed execution through it before acquisition, made
  `viewStreamProvider` the canonical RDP gateway field, kept
  `provider=rdp_gateway` as a compatibility alias, and added parser plus
  runtime intent tests for the alias boundary. Updated the generated
  service-client helper surface and examples so `remote_view_open` preserves
  `viewStreamProvider`. Slice A is complete for the no-launch parser,
  runtime-intent, docs, skill, and service-client surfaces.

### Slice B: No-Mutation Acquisition Planner

Goal: make route selection and browser reuse explainable before mutation.

Deliverables:

- Add `RemoteViewAcquisitionPlan`.
- Move route binding fallback logic into planner decisions with named reasons.
- Add strict reuse eligibility:
  - profile identity;
  - browser build;
  - host;
  - display allocation;
  - route-pool entry;
  - route ID;
  - stream provider;
  - control provider;
  - tab policy.
- Separate `strict_operator_open` from explicit repair/reuse modes.
- Return typed blockers and suggested commands.

Acceptance:

- A stale browser display allocation cannot outrank an available clean route
  unless all owner and proof fields match.
- Named-session display-allocation mismatches produce a direct, actionable
  error with available route IDs.
- Dry-run output explains the selected route and why alternatives were skipped.

Validation:

- Rust unit tests for planner decisions.
- Fixture tests for stale browser, route-pool unavailable, checked-out same
  owner, checked-out other owner, and inline route entry cases.
- `pnpm test:route-handoff-audit`.

Progress:

- 2026-06-22: Added `RemoteViewAcquisitionPlan` and routed
  `remote_view_open`, route preflight, and route checkout through the
  no-mutation planner before state-changing acquisition. The planner records
  display-allocation and route-pool decisions with named reasons, includes
  typed blockers and suggested commands, prefers clean available route-pool
  entries over stale browser display fallbacks, allows same-owner checked-out
  route reuse, and reports named-session display mismatches with route-pool
  diagnostics. Added Rust fixture coverage for stale browser fallback ordering,
  checked-out same-owner reuse, checked-out other-owner rejection,
  display-allocation owner mismatch diagnostics, inline route entries, and
  dry-run acquisition-plan output. Slice B is complete for the no-launch
  planner surface.

### Slice C: Atomic Lease And Rollback

Goal: stop partial launches from leaving route, display, tab, and incident
state that the dashboard treats as live.

Deliverables:

- Add pending route/display/browser lease state.
- Execute route reservation, display allocation, browser launch or attach, tab
  acquisition, proof, and final checkout as one service transaction.
- Roll back pending reservations and close partial tabs/browser processes when
  proof fails.
- Preserve a compact diagnostic event when cleanup runs.
- Replace manual route-pool state cleanup with service actions.

Acceptance:

- Failure during tab open, focus, proof, or checkout leaves no live left-rail
  row unless diagnostics explicitly preserve it.
- Route-pool entries recover from failed pending acquisition without manual JSON
  editing.
- A cold start followed by a forced proof failure returns typed cleanup
  evidence.

Validation:

- Rust unit tests for lease transitions.
- No-launch service contract tests for route checkout and release metadata.
- Focused live smoke with an intentionally failing display proof in a disposable
  route/profile when available.

Progress:

- 2026-06-22: Added persisted `RemoteViewAcquisitionLease` state and wrapped
  `remote_view_open` with a pending acquisition transaction that marks the
  selected route-pool entry, display allocation, and remote-view route as
  pending before display access, browser launch, tab acquisition, proof, and
  checkout complete. Display access, launch, tab, focus, proof, and checkout
  failures now roll back the saved route-pool/display/route/browser state and
  return typed cleanup JSON that includes lease rollback evidence. Added a
  no-launch Rust fixture proving a failed acquisition restores an available
  route-pool entry and removes pending display/route rows. Slice C still needs
  the focused forced-proof live smoke and any remaining service-contract
  metadata checks before it should be marked complete.
- 2026-06-22: Added service-state wire-contract coverage for
  `remoteViewAcquisitionLeases`, including nested previous route-pool entry,
  display allocation, and remote-view route snapshots. Strengthened route
  checkout and release fixtures so they assert acquisition-plan, checked-out
  route-pool entry, route provider event, release status, released viewer-lease
  list, and released route metadata. The remaining Slice C validation gap is
  the focused forced-proof failing live smoke.
- 2026-06-23: Closed the forced-proof live-smoke gap. The live fixture now has
  an explicit `--force-proof-failure` mode backed by
  `AGENT_BROWSER_REMOTE_VIEW_FORCE_PROOF_FAILURE`, and the smoke asserts typed
  cleanup plus lease rollback state. The forced-proof live smoke passed with
  route `guacamole:4`, display allocation `remote-view-display:16`, cleanup
  state `closed_new_browser`, rollback state `rolled_back`, and artifacts in
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-16-04-025Z`.
  The normal fixture smoke then passed with repeat open, HTTP helper, CDP
  readback, X11 PID proof, route-handoff classification `route_bound_ready`,
  visual state `browser_window_visible`, OCR proof, and artifacts in
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T03-17-24-564Z`.
  During live validation, Slice C also fixed two state-ordering bugs exposed by
  the smoke: released or orphaned display allocations from previous sessions are
  reclaimable, and same-owner pending route reservations are reusable during
  checkout/repeat. Slice C is complete; continue with Slice D.

### Slice D: Browser-Only Route Desktop

Goal: eliminate terminal-first route desktops from browser-control routes.

Deliverables:

- Change route-user bootstrap so agent-browser RDP route sessions do not start
  XTerm as the foreground UI.
- If the route desktop still starts helper UI, ensure the launch path can
  identify and background or close it safely by client window, not window-manager
  frame.
- Add route desktop readiness checks to fast preflight.
- Add terminal-topmost and terminal-only fixture coverage.

Acceptance:

- A new RDP route session opens to a browser-control-ready desktop, not a shell.
- Terminal-only and terminal-topmost states fail proof with typed reasons.
- The operator-visible proof cannot pass while XTerm is topmost over the
  browser.

Validation:

- `node scripts/inspect-rdp-route-displays.js` fixture or no-launch tests.
- `pnpm test:route-handoff-audit`.
- One live route display inspection after cold route start.

Progress:

- 2026-06-23: Removed the foreground terminal from new route-pool XRDP user
  sessions. The privileged helper and route-pool setup fallback now write an
  idle Openbox `.xsession` that keeps XRDP alive without starting terminal UI.
  Added `scripts/test-rdp-route-xsession.js` and wired it into
  `pnpm test:route-confusion-gates` so route bootstrap scripts cannot
  reintroduce terminal startup. Added `terminal_topmost` display-content
  classification plus `terminal_topmost_route` proof failure coverage, so
  browser proof cannot pass when a terminal is the top application window above
  the browser. Installed-helper readback showed
  `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` still has
  the old `xterm` writer, and `sudo -n true` failed with password required, so
  Slice D still needs a helper refresh from an interactive sudo boundary plus a
  cold route display inspection.
- 2026-06-23: Added installed-helper route desktop readiness to
  `agent-browser install doctor` and `agent-browser doctor remote-view`.
  Both JSON reports now include `helperDesktopSession` with the parsed
  `.xsession` template state, and both emit
  `remote_view_route_desktop_helper_stale` when the installed root-owned helper
  still writes a terminal-first route desktop. Rebuilt debug-binary readbacks on
  this host reported `state=terminal_first_template`,
  `terminalStartupDetected=true`, and the stale-helper issue code from both
  doctor surfaces. The live boundary remains unchanged: refresh the installed
  helper from an interactive sudo shell, then start a cold route and inspect the
  desktop.
- 2026-06-23: Rechecked the installed-helper boundary. `pnpm
  test:rdp-route-xsession` still passes for repo sources, and `sudo -n
  /usr/local/libexec/agent-browser/agent-browser-privileged-helper check`
  succeeds. A direct diff still shows the installed helper writes `xterm`
  while the repo helper writes idle Openbox/no-terminal startup. `pnpm
  install:privileges -- --apply` could not refresh the root-owned helper from
  this noninteractive session and failed with `sudo: a terminal is required to
  read the password`. Slice D remains open on interactive helper refresh and
  cold route display inspection.
- 2026-06-23: Repaired the missing Guacamole route-specific connection layer
  without mutating existing Linux users. `scripts/setup-rdp-guac-route-pool.sh`
  now reuses already-created `agent-browser-rdp-a` and `agent-browser-rdp-b`
  users plus stored route passwords when present, skips privileged helper user
  rewrites and XRDP restart in that reuse path, and only updates Guacamole
  records. Live setup created route-specific connections 3 and 4 for
  `Agent Browser RDP Route A/B`. Opening both route clients then produced
  distinct XRDP sessions: route A on `:11` for `agent-browser-rdp-a`, route B
  on `:12` for `agent-browser-rdp-b`. The inspection still showed an XTerm on
  route A, proving the installed helper/user session template remains stale
  even though the browser route itself can now materialize.
- 2026-06-23: Hardened the source privileged helper and installer around the
  remaining stale-helper boundary. The helper `grant-display-access` predicate
  now accepts both filesystem X11 sockets and XRDP abstract sockets from
  `/proc/net/unix`, matching the daemon and route-pool readiness predicates,
  while still bounding `xhost` with `timeout`. The route helper contract guard
  now asserts no terminal startup, idle Openbox session lifetime, abstract X11
  socket support, and bounded display grants. The privilege installer now
  prints a readiness breakdown before dry-run exit or sudo escalation, so a
  noninteractive apply reports `helper: installed helper differs from bundled
  helper and must be refreshed` before failing at the interactive sudo
  boundary. Validation passed:
  - `bash -n scripts/install-agent-browser-privileges.sh scripts/libexec/agent-browser-privileged-helper`
  - `pnpm test:rdp-route-xsession`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_helper_desktop_session_status -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml install_doctor_issues -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_issues -- --nocapture`

  Live install readback still shows the installed root-owned helper is stale
  and requires an interactive privileged refresh before the cold route desktop
  inspection can pass.
- 2026-06-23: Added a versioned helper self-report contract for the next
  installed helper refresh. Source `agent-browser-privileged-helper
  status-json` now returns schema version, helper version
  `2026-06-23.p44-route-desktop-v2`, route desktop template readiness, and
  display-access capabilities including filesystem and abstract X11 socket
  support plus the bounded `xhost` timeout. `agent-browser install doctor` and
  `agent-browser doctor remote-view` now include `helperStatus` alongside
  `helperCheck` and `helperDesktopSession`, so a stale installed helper no
  longer looks healthy merely because `check` succeeds. Live readback with the
  rebuilt debug binary reports `helperStatus.success=false`, `exitCode=2`, and
  `Unknown command: status-json` from the installed root-owned helper, while
  `helperDesktopSession.state=terminal_first_template` continues to emit the
  stale-helper issue. Validation passed:
  - source helper `status-json` parses with the expected capability payload
  - `pnpm test:rdp-route-xsession`
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_helper_desktop_session_status -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml install_doctor_issues -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_issues -- --nocapture`
  - `cargo build --manifest-path cli/Cargo.toml`
  - `git diff --check`
- 2026-06-23: Tightened remote-view privilege readiness so `check` alone
  cannot make an older helper look ready. `agent-browser install doctor` and
  `agent-browser doctor remote-view` now require the parsed `helperStatus`
  contract to report the P44 route desktop and display-access capabilities.
  Both doctor surfaces emit `remote_view_privileged_helper_status_stale` when
  the installed helper lacks the contract or does not report abstract X11 socket
  support. Live readback with the rebuilt debug binary now reports
  `remoteViewPrivileges.ready=false`, `helperStatus.success=false`, and
  `Unknown command: status-json` for the installed root-owned helper. Focused
  validation passed:
  - `cargo test --manifest-path cli/Cargo.toml remote_view_helper_status_contract -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml install_doctor_issues -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_issues -- --nocapture`
  - `pnpm test:rdp-route-xsession`
  - `cargo build --manifest-path cli/Cargo.toml`
  - `git diff --check`

### Slice E: Strong Operator-Visible Proof

Goal: make success match what the operator sees.

Deliverables:

- Extend `OperatorVisibleProof`.
- Correlate selected target ID to browser process and window.
- Add X11 mapped, focus/topmost, minimized, and terminal-obscured checks.
- Record selected target URL/title readiness separately from display readiness.
- Preserve Guacamole local/public route readiness with freshness timestamps.
- Return proof components in `remote-view open`, service responses, and route
  records.

Acceptance:

- `operatorVisible.state=ready` means route, browser, tab, display, and stream
  agree.
- Proof distinguishes:
  - terminal-only route;
  - terminal obscuring browser;
  - browser visible but wrong tab;
  - Guacamole route unavailable;
  - stale route record;
  - CDP target unavailable.
- Dashboard and service clients can render the same proof vocabulary.

Validation:

- Rust proof unit tests.
- Dashboard readiness fixture tests.
- `pnpm test:dashboard-view-streams`.
- `pnpm test:remote-view-open-fixture-live` when route infrastructure is ready.

Progress:

- 2026-06-23: Added the first structured `operatorVisible` proof envelope for
  successful `remote_view_open` responses. The response still preserves the
  existing `proof` field, and now also includes selected target evidence plus
  route, display, browser, tab, stream, and Guacamole component states. Updated
  `summarizeServiceRemoteViewOpenProof()` to prefer
  `operatorVisible.target` and `operatorVisible.components` when present before
  falling back to the top-level tab result. Added focused Rust and service-client
  coverage for the ready proof shape. Slice E remains open for the remaining
  failure vocabulary and dashboard readiness fixtures.
- 2026-06-23: Added selected-target URL readiness to `operatorVisible`. The
  proof now distinguishes a visible browser window from the wrong selected tab:
  when the opened target URL does not match the requested URL, the top-level
  state and tab component state become `wrong_tab` while the display component
  can still report browser-window visibility. The service-client proof summary
  now surfaces `reason=wrong_tab` from the structured component state.
- 2026-06-23: Added Guacamole route availability to the same proof vocabulary.
  When display proof and target URL proof are ready but the route has no ready
  Guacamole operator URL or its readiness state is not ready, `operatorVisible`
  reports `state=guacamole_route_unavailable` and
  `components.guacamole.state=guacamole_route_unavailable`. The service-client
  proof summary surfaces `reason=guacamole_route_unavailable`.
- 2026-06-23: Added CDP target availability to the selected-tab proof. A visible
  browser window with a URL-bearing tab result but no CDP `targetId` now reports
  `state=cdp_target_unavailable` and
  `components.tab.state=cdp_target_unavailable`; the service-client proof
  summary surfaces `reason=cdp_target_unavailable`.
- 2026-06-23: Bounded route display-access grants so a stuck XRDP/X11 helper no
  longer wedges the daemon. `remote_view_open` now runs the privileged
  `grant-display-access` helper through a 2 second `timeout` wrapper with null
  stdio, returns `display_access_grant_timeout` as a typed blocker, preserves
  that machine-readable code through the AI-friendly error mapper, and rolls
  back the acquisition lease before browser launch. The live fixture still
  fails on this host because route display `:11` does not accept the helper
  `xhost` grant, but it now fails deterministically with typed cleanup instead
  of a generic page timeout or stuck daemon. Evidence:
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T14-02-17-060Z`.
- 2026-06-23: Tightened route-display readiness so Guacamole and RDP TCP
  readiness cannot masquerade as a live route display. The route-pool readiness
  smoke now emits a `route_display_socket` component for each selected route and
  blocks export when the claimed `target.displayName` has no X11 socket. The
  runtime route binding path also rejects stale persisted route-pool entries
  with `route_display_unavailable` before checkout or browser launch; direct
  dry-run evidence for stale `guacamole-rdp-a` returned
  `route_display_unavailable: route pool entry 'guacamole-rdp-a' target display
  ':11' has no local X11 socket`. The privileged helper source now fails fast
  when the display socket is absent and bounds its internal `xhost` call so a
  future installed helper cannot leave root-owned `runuser` children behind.
  Live fixture evidence:
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T14-10-28-952Z`.
- 2026-06-23: Corrected the display socket predicate for XRDP's actual socket
  mode on this host. Route-specific XRDP sessions published abstract Unix
  sockets such as `@/tmp/.X11-unix/X11` and `@/tmp/.X11-unix/X12`, not
  filesystem nodes under `/tmp/.X11-unix`. The route-pool readiness smoke and
  Rust route-binding guard now accept either filesystem or abstract X11
  sockets. With `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:11` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:12`, route-pool readiness returned
  `success=true` and exported a route-pool JSON for route-specific connections
  3 and 4.
- 2026-06-23: Proved the deterministic route-bound Facebook open path using an
  inline route-pool entry. A dry run with `--route-pool-entry-json` selected
  connection 3, route `guacamole:3`, route user `agent-browser-rdp-a`, and
  display `:11`. The real open completed in about 12 seconds with
  `status=opened`, `operatorVisible.state=ready`, display-access grant
  `state=granted`, selected route `guacamole:3`, and visible-window proof for
  Facebook on `:11`. The full response was saved at
  `/tmp/agent-browser-facebook-open-inline-route.json`.
- 2026-06-23: Identified a remaining daemon harmonization bug. Supplying
  `AGENT_BROWSER_RDP_ROUTE_POOL_JSON` to the CLI process is not sufficient when
  the long-lived daemon already has stale persisted route-pool entries; a
  dry-run still selected existing-user connection 1 for `guacamole-rdp-a`.
  Supplying the selected route as an inline request payload fixed the binding.
  The durable refactor should make route-pool discovery a fresh service
  preflight or explicit request payload, not an operator shell environment that
  can drift from daemon state.
- 2026-06-23: Closed the immediate env-to-daemon split for CLI opens. The
  `remote-view open` parser now copies `AGENT_BROWSER_RDP_ROUTE_POOL_JSON`
  into the `remote_view_open` command as request-scoped `routePool` data when
  no inline route entry is supplied, so a fresh readiness export outranks stale
  daemon-retained route-pool state. A dry run with only the env route pool and
  `--route-pool-entry-id guacamole-rdp-a` selected connection 3,
  route `guacamole:3`, route user `agent-browser-rdp-a`, and display `:11`
  instead of the stale existing-user connection.
- 2026-06-23: Fixed two repeat-open convergence failures exposed by the live
  Facebook handoff. First, route-pool rows left `available` after rollback now
  still qualify for same-owner reuse when the retained `remoteViewRoutes`
  record proves the same browser, session, route, and display owner. Second,
  when a daemon restart leaves the matching runtime-profile Chrome alive but
  absent from daemon memory, route-bound open attaches to the live runtime
  profile via its runtime-state CDP port instead of trying to launch a second
  Chrome on the locked profile. Live evidence:
  `/tmp/agent-browser-facebook-open-env-route-pool-runtime-profile-after-runtime-attach-fix.json`
  returned `success=true`, `status=opened`, route `guacamole:3`, connection 3,
  display `:11`, `operatorVisible=ready`, display access `already_ready`, and
  visible-window proof `ready`.
- 2026-06-23: Added retained route metadata to the route component. When the
  selected route-pool entry has stale retained state or points at a mismatched
  current route allocation, `operatorVisible` reports
  `state=stale_route_record` and `components.route.state=stale_route_record`;
  the service-client proof summary surfaces `reason=stale_route_record`.
- 2026-06-23: Added dashboard readiness fixture coverage for the expanded
  proof vocabulary. Workspace rows now preserve structured
  `wrong_tab`, `guacamole_route_unavailable`, `cdp_target_unavailable`, and
  `stale_route_record` states from retained stream readiness and keep View and
  Control disabled with state-specific reasons.
- 2026-06-23: Revalidated the no-launch Slice E proof vocabulary end to end.
  Rust operator-visible unit coverage now proves `ready`, `wrong_tab`,
  `guacamole_route_unavailable`, `cdp_target_unavailable`, and
  `stale_route_record` states. The service-client helper tests prove summary
  and failure-reason rendering for the same vocabulary plus terminal-only and
  missing-proof rejection, and the dashboard view-stream smoke proves the
  readiness mapping still renders those blockers as disabled view/control
  states. The remaining Slice E live validation is the final
  `pnpm test:remote-view-open-fixture-live`/Facebook handoff, which is gated by
  Slice D's installed-helper refresh and cold route desktop proof rather than
  missing proof vocabulary. Validation passed:
  - `cargo test --manifest-path cli/Cargo.toml remote_view_open_operator_visible -- --nocapture`
  - `pnpm test:service-request-client`
  - `pnpm test:dashboard-view-streams`

### Slice F: Tab Acquisition And Stale Target Recovery

Goal: stop repeated route-bound opens from producing duplicate tabs and stale
dashboard links.

Deliverables:

- Add explicit tab acquisition policy:
  - `reuse_compatible`;
  - `open_new`;
  - `refresh_existing`;
  - `replace_duplicates`.
- Store current selected target in the route-bound lease.
- Recover or reject stale `tab=target:*` URL params before rendering the
  workspace viewport.
- Close or mark duplicate tabs according to policy.
- Return structured stale-target recovery evidence.

Acceptance:

- Repeating the Facebook one-liner does not accumulate unbounded duplicate
  Facebook tabs by default.
- A dashboard URL with a stale target either redirects to the current target or
  renders an explicit stale-target recovery state.
- The workspace viewport never binds control mode to a target that is not live
  in the selected browser.

Validation:

- Service tab refresh and release tests.
- `pnpm test:dashboard-workspace-inspector-tab`.
- `pnpm test:dashboard-view-streams`.
- Focused live smoke that runs the one-liner twice and verifies one intended
  active target.

Progress:

- 2026-06-23: Added `replace_duplicates` to the generic
  `tab_handle_refresh` repair policy. The daemon, HTTP ingress, MCP ingress,
  service schema, generated client template, README, docs site, and agent skill
  now agree on the policy. The policy reuses or opens one compatible target and
  returns `duplicateTargetCleanup` evidence for best-effort closure of other
  compatible live targets.
- 2026-06-23: Added dashboard stale-target URL recovery. The workspace viewport
  now treats missing, closed, blank, or target-shaped stale `tab=target:*`
  selections as recoverable stale target identity, replaces the URL selection
  with the current live service tab, and only then queues control-mode
  `view_focus`.
- 2026-06-23: Routed `remote_view_open` tab acquisition through same-origin live
  target reuse before opening a new tab. Successful repeat opens now return
  `tabAcquisitionDecision` and `duplicateTargetCleanup` evidence, and the live
  smoke asserts CLI first, CLI repeat, and HTTP helper opens converge to one
  active intended target.
- 2026-06-23: Revalidated the no-launch Slice F tab and stale-target recovery
  gates. Rust tab-handle tests cover refresh policies, compatible duplicate
  target selection, stale target classification, and tab-handle release that
  closes only the selected tab record. Dashboard viewport tests still prove a
  stale target-shaped URL selection recovers to a live non-blank service-owned
  target and queues `view_focus` before embedding control. The selected
  workspace inspector tab test now also asserts the P44 live-rail boundary:
  post-termination browser history is absent from selected workspace control
  context, while non-terminal retained rows remain inspectable as retained.
  The remaining Slice F live validation is the final repeat-open route smoke,
  gated by Slice D's installed-helper refresh and cold route desktop proof.
  Validation passed:
  - `cargo test --manifest-path cli/Cargo.toml tab_handle -- --nocapture`
  - `pnpm test:dashboard-workspace-inspector-tab`
  - `pnpm test:dashboard-view-streams`

### Slice G: Fast Route Preflight

Goal: remove full doctor latency from the happy path.

Deliverables:

- Add a fast no-launch preflight endpoint for route-bound opens across HTTP,
  MCP, and generated client surfaces.
- Include cached freshness timestamps for:
  - Guacamole web route;
  - Guacamole login;
  - connection permissions;
  - RDP backend TCP;
  - route display state;
  - display access grant;
  - route desktop readiness.
- Keep full `doctor remote-view` as deep diagnosis and repair.
- Make slow or stale preflight components typed blockers instead of silent
  launch delays.

Acceptance:

- The one-liner can decide launch eligibility without running full doctor.
- Full doctor can be slow without making the happy path hang.
- The dashboard can show "preflight stale" separately from "browser failed."

Validation:

- No-launch preflight tests.
- Service contract schema parity tests.
- A timing smoke that asserts fast preflight stays within a bounded threshold
  on a healthy route.

Progress:

- 2026-06-23: Added the first `fastPreflight` payload to the existing
  `service_remote_view_route_preflight` no-launch action. The response now
  reports `ready`, `partial`, `stale`, or `blocked` from component evidence for
  acquisition planning, Guacamole route URL shape, retained Guacamole
  web/login/permission and RDP TCP readiness, display access, and route desktop
  state. The display-access probe is now bounded with `timeout` so fake or
  unreachable route displays cannot hang the preflight path.
- 2026-06-23: Added first-class convenience surfaces for the same fast
  preflight response: HTTP `GET /api/service/remote-view/route-preflight`, MCP
  `service_remote_view_route_preflight`, generated
  `getServiceRemoteViewRoutePreflight()`, and
  `service-remote-view-route-preflight-response.v1.schema.json`.
- 2026-06-23: Added `pnpm test:remote-view-route-preflight-timing`, an isolated
  no-launch smoke that starts a temp daemon, calls the HTTP route through
  `getServiceRemoteViewRoutePreflight()`, and asserts the response stays within
  the bounded fast-preflight threshold while returning `fastPreflight`.
- 2026-06-23: Added `privileged_helper_status` to `fastPreflight`. The no-launch
  preflight now runs a bounded installed-helper `status-json` probe and blocks
  route-bound launch when the helper lacks the P44 route desktop and
  display-access capability contract, including abstract X11 socket support.
  This keeps the one-liner and dashboard/client preflight aligned with
  `install doctor` and `doctor remote-view`: a helper that still passes
  `check` but lacks `status-json` is not remote-view ready. Updated
  `pnpm test:remote-view-route-preflight-timing` to publish an abstract X11
  socket fixture instead of relying on a nonexistent display path, so the smoke
  satisfies the stricter route-display guard. Validation passed:
  - `cargo test --manifest-path cli/Cargo.toml remote_view_helper_status_contract -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml test_remote_view_route_and_lease_actions_mutate_service_state -- --nocapture`
  - `pnpm test:remote-view-route-preflight-timing`
  - `cargo build --manifest-path cli/Cargo.toml`
  - `git diff --check`
- 2026-06-23: Propagated the helper-status preflight blocker through dashboard
  readiness mapping. `privileged_helper_status` now maps to the explicit
  dashboard action `refresh_remote_view_helper` instead of generic readiness
  inspection, with copy that tells the operator to refresh the installed
  remote-view privileged helper from an interactive terminal and rerun route
  preflight. Validation passed:
  - `pnpm test:dashboard-view-streams`
  - `pnpm test:service-observability-client`
  - `git diff --check`

### Slice H: Dashboard Inventory Refactor

Goal: make the live control rail match ownership and actionability.

Deliverables:

- Add `WorkspaceInventoryClass`.
- Render primary live rail groups:
  - service-owned controllable browsers;
  - detected non-owned addressable CDP browsers.
- Move these to diagnostics/log surfaces:
  - stale retained browsers;
  - inactive browsers;
  - resolved incidents;
  - viewer clients;
  - "needs attention" rows with no user action.
- Ensure daemon CDP streams do not override service-owned route streams for the
  same browser/session identity.
- Make non-owned rows visually and textually explicit.

Acceptance:

- A Unix terminal-only route does not appear as a successful browser control
  row.
- Foreign CDP browsers appear only in the non-owned group when reachable and
  addressable.
- Resolved incidents and stale rows do not crowd the live control surface.
- The workspace viewport consumes canonical classified state rather than
  inferring ownership from URL shape.

Validation:

- `pnpm test:dashboard-workspace-nodes`.
- `pnpm test:dashboard-view-streams`.
- `pnpm test:dashboard-inspector-actions`.
- Manual dashboard smoke after local publish when UI changes are present.

Progress:

- 2026-06-23: Started Slice H by making route-bound workspace inventory
  actionability explicit. RDP gateway rows whose operator-visible proof is not
  ready now move to `needs-attention` with the route-proof reason and Repair
  available instead of remaining in the active control group with disabled
  View and Control actions. The fixture covers terminal-only, unbound,
  missing-proof, wrong-tab, unavailable-route, missing-CDP-target, and
  stale-route cases while preserving ready service-owned browsers in the active
  controllable group and detected external CDP browsers in the detected
  non-owned group.
- 2026-06-23: Added `WorkspaceInventoryClass` to the shared workspace node
  model and selected-workspace context. Rows now expose canonical classes for
  service-owned controllable browsers, service-owned view-only browsers,
  service-owned diagnostic browsers, detected non-owned browsers, viewer
  clients, retained history, service-owned sessions, and profile action rows so
  inspector, chat, console, and automation surfaces can use the same ownership
  vocabulary instead of inferring ownership from row group or URL shape.
- 2026-06-23: Completed Slice H dashboard inventory refactor. The selected
  Workspace inspector now renders the row's canonical inventory Class, and the
  navigator, workspace-node, selected-context, inspector-action, dashboard build,
  local publish, and full local dashboard runtime smokes passed. Local publish
  installed executable SHA
  `6c7c9b879c1b564130fb74e4d2abec7502252033be14e66586c20477e7762649`
  with dashboard bundle SHA
  `10177dc55ce0a76f29fbcce7ede2acf8e7b5cbb896d83987ddff2e2aaa193967`;
  runtime smoke loaded `http://127.0.0.1:4848/`, found
  `service-owned-controllable-browser`, and confirmed the workspace pane.
- 2026-06-23: Tightened the live-rail boundary after the helper/preflight work.
  Added `deriveLiveWorkspaceNodes()` and `isLiveWorkspaceNode()` as the shared
  dashboard projection for live control rows, then changed the workspace
  navigator to consume that projection instead of locally filtering full
  diagnostic nodes. The reusable workspace-node model can still derive
  `needs-attention` rows for diagnostics and future log viewers, but the live
  left rail has one canonical active/detected projection and cannot
  accidentally reintroduce attention rows through a component-local filter.
  Validation passed:
  - `pnpm test:dashboard-workspace-nodes`
  - `pnpm test:dashboard-workspace-navigator`
  - `pnpm build:dashboard`
  - `git diff --check`
- 2026-06-23: Harmonized the CLI dashboard help with the P44 live-rail
  contract. `agent-browser dashboard --help` now says the left workspace
  navigator renders only live Agent-browser owned browsers and detected
  non-owned addressable browsers, while attention, stale, retained, and
  resolved incident records stay in Service, trace, event, job, incident, and
  log viewers. The workspace navigator structural smoke now asserts the help
  text cannot regress to the old "Active, Attention, and Retained" live-group
  wording. Validation passed:
  - `pnpm test:dashboard-workspace-navigator`
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo build --manifest-path cli/Cargo.toml`

### Slice I: Foreign CDP Integration Boundary

Goal: align P44 with P41 without merging ownership semantics.

Deliverables:

- Use P41 discovery output as the source for non-owned addressable browser
  rows.
- Keep foreign CDP read-only inspection and stream actions separate from
  service-owned route-bound control.
- Require explicit borrow/adoption action before mutating a foreign browser.
- Disable close, kill, profile release, and route repair actions for foreign
  rows.

Acceptance:

- AuraCall and im-receipts reachable CDP browsers can appear as non-owned
  addressable rows.
- They cannot be confused with service-owned route-bound browsers.
- Agent-browser does not mutate their lifecycle without an explicit action.

Validation:

- P41 fixture tests.
- Dashboard non-owned row tests.
- Optional live read-only smoke against AuraCall and im-receipts when those
  browsers are running.

Progress:

- 2026-06-23: Added explicit read-only action vocabulary for detected
  non-owned CDP browser rows. Live detected rows now expose `Inspect`,
  `Stream`, and `Screenshot` actions while `Control`, `Add tab`,
  `Borrow control`, service route repair, close, and kill remain disabled with
  ownership-specific reasons. The workspace navigator can render these actions
  without promoting the row to an agent-browser-owned control route. Added
  dashboard workspace-node fixture coverage for the read-only versus
  mutation/lifecycle boundary.
- 2026-06-23: Hardened the foreign CDP discovery source that feeds detected
  non-owned rows. `/api/sessions` process discovery now normalizes both normal
  NUL-separated argv and single-string `/proc/<pid>/cmdline` forms, resolves
  `--remote-debugging-port=0` through `DevToolsActivePort`, requires a bounded
  `/json/version` or `/json/list` DevTools HTTP probe, emits explicit
  `ownership=foreign_cdp`, `addressability=cdp_reachable`, read-only
  capability, borrow, and source metadata, and skips profiles under
  `~/.agent-browser` so service-owned Chrome processes are not reclassified as
  foreign. Focused Rust fixtures cover AuraCall-style fixed ports,
  im-receipts-style dynamic ports, renderer exclusion, and agent-browser-owned
  profile exclusion. Live checkout-dashboard readback on port 4850 showed
  im-receipts `detected-profile-mirror-37537` plus AuraCall
  `detected-chatgpt-45011`, `detected-chatgpt-45013`, and
  `detected-grok-37427` as `foreign_cdp` and `cdp_reachable`, with explicit
  port source metadata and no `~/.agent-browser` profiles in the detected set.

### Slice J: Incident And State Durability

Goal: make runtime state recoverable and stop stale incident rows from acting
like live controls.

Deliverables:

- Fix incident resolution so recovered incidents are not active.
- Add service actions for route-pool cleanup, stale route release, and incident
  repair.
- Harden Guacamole Postgres setup against WSL crash or hard stop:
  - idempotent schema check;
  - startup recovery check;
  - forced checkpoint after schema and route writes;
  - no automatic overwrite of partial schema;
  - explicit repair output.
- Add cold restart readback that checks route pool, display allocations,
  Guacamole connections, and service state agree.

Acceptance:

- No manual edits to `~/.agent-browser/service/state.json` are required for the
  audited failure classes.
- A resolved incident disappears from live control and remains available in the
  incident log.
- After a WSL hard stop or service restart, stale checked-out route entries are
  repaired or reported with one explicit command.

Validation:

- Incident resolver unit and service tests.
- Route-pool cleanup service tests.
- `pnpm test:rdp-guac-route-cleanup-live` when route infrastructure is ready.
- A documented cold restart smoke before closeout.

Progress:

- 2026-06-23: Fixed incident resolution durability across derived-state
  refresh. `refresh_derived_views()` now preserves an operator-resolved
  incident as recovered when the derived evidence is not newer than
  `resolvedAt`, clears stale `currentHealth`, and recalculates severity,
  escalation, and recommended action as no-action-required. Strengthened the
  resolver and service-model tests so stale health events cannot reactivate a
  resolved incident in the live control surface.
- 2026-06-23: Extended `service_route_pool_repair` from pool-entry reset to
  graph cleanup. Dry-runs now report stale route and display-allocation
  records tied to stale checked-out pool entries, and apply returns the pool
  entries to `available` while marking the stale remote-view route and display
  allocation `released`. Released routes no longer derive live route incidents.
  Updated the live route-cleanup smoke and user-facing docs so operators can
  use the service action instead of hand-editing retained state after a browser
  crash.
- 2026-06-23: Hardened Guacamole Postgres route-write durability. The existing
  `ensure-rdp-guac-postgres.sh` already performs idempotent schema checks,
  startup readiness, partial-schema refusal, and schema checkpoints. Added
  `ON_ERROR_STOP=1` plus explicit `CHECKPOINT` after Guacamole route writes in
  the route-pool setup, existing-user sync, and autologin setup scripts, and
  added a no-launch `test:rdp-guac-postgres-hardening` guard to keep those
  properties covered through `test:route-confusion-gates`.
- 2026-06-23: Added `pnpm test:rdp-guac-cold-restart-readback-live` as the
  cold restart/readback gate. It uses current Guacamole route-pool readiness,
  launches one isolated remote-headed browser, seeds and checks out one
  Guacamole-backed route-pool entry, restarts the stream/control daemon, and
  asserts route-pool entry, remote-view route, display allocation, browser, and
  Guacamole connection IDs still agree before and after reconcile.
- 2026-06-23: Ran the live Slice J gates successfully after repairing the
  empty Guacamole schema through `pnpm ensure:rdp-guac-postgres -- --apply`
  and provisioning existing-user Guacamole RDP route records through
  `pnpm sync:rdp-guac-existing-user-route-pool`. `pnpm
  test:rdp-guac-cold-restart-readback-live` passed with artifacts under
  `/tmp/agent-browser-rdp-guac-cold-restart-2026-06-23T13-23-33-699Z`, and
  `pnpm test:rdp-guac-route-cleanup-live` passed with artifacts under
  `/tmp/agent-browser-rdp-guac-route-cleanup-2026-06-23T13-24-31-334Z`.
- 2026-06-23: Extended the live remote-view fixture to repair stale pending
  acquisition leases before route selection through the generated
  `requestServiceRoutePoolRepair()` client helper. The fixture proved
  `service_route_pool_repair` can clean stale pending route/display/pool state
  left by wedged helper attempts without manual JSON edits; the pre-open repair
  in `/tmp/agent-browser-remote-view-open-live-2026-06-23T13-57-00-972Z`
  repaired two stale pending acquisitions, two stale display allocations, and
  two stale routes. The normal live fixture now stops even earlier when route
  readiness is not green, reporting `repair_rdp_route_display_session` because
  `:11` and `:14` have no `/tmp/.X11-unix/X11` or `/tmp/.X11-unix/X14`
  sockets. Evidence:
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T14-10-28-952Z`.
- 2026-06-23: Bounded `scripts/open-rdp-guac-route-displays.js` agent-browser
  calls after the display opener hung inside `agent-browser doctor remote-view
  --json` before opening Guacamole route clients. The opener now accepts
  `--agent-browser-timeout-ms` and reports a structured timeout instead of
  blocking indefinitely. Direct evidence shows `timeout --kill-after=1 8
  cli/target/debug/agent-browser doctor remote-view --json` exits 124 with no
  JSON, so the next route-display repair slice must make doctor remote-view
  bounded and split slow probes into typed readiness components.
- 2026-06-23: Made `doctor remote-view --json` bounded enough to return
  operator-usable JSON instead of hanging behind a slow subprobe. Remote-view
  doctor now wraps JSON/text/`xdpyinfo` child probes in timeouts, reports
  `timedOut` on the relevant component, and distinguishes a claimed display
  name from a ready route display. Install doctor now bounds its service-status
  and service-resource subprobes, caps dashboard manifest probing, and uses
  bounded `sha256sum` for large doctor fingerprints so debug builds do not
  spend unbounded time hashing 275 MB workspace binaries in-process. Evidence:
  `timeout --kill-after=1 35 cli/target/debug/agent-browser doctor remote-view
  --json` returned in about 21 seconds with `installTimedOut=false`,
  `routePool.data.success=false`, `routeDisplayClaimed=true`,
  `routeDisplayReady=false`, and `routeDisplayAccessReady=false`. The live
  fixture now fails in about two seconds with artifact
  `/tmp/agent-browser-remote-view-open-live-2026-06-23T14-32-00-815Z`,
  explicitly blocked on missing display sockets for `:11` and `:14` rather than
  a stale CDP stream, generic timeout, or silent helper hang.

## Refactor Guardrails

- Keep compatibility facades while migrating call sites.
- Do not mix foreign CDP discovery with service-owned route acquisition.
- Prefer typed structs and enums over ad hoc JSON path checks in Rust.
- Keep dashboard classification pure and fixture-testable.
- Do not let repair fallback behavior run silently in the happy path.
- Record proof and blocker vocabulary once, then reuse it across CLI, HTTP,
  MCP, generated client, dashboard, and tests.
- Avoid broad rewrites of unrelated service request actions.

## Validation Matrix

Minimum no-launch gates:

```bash
pnpm test:route-handoff-audit
pnpm test:dashboard-view-streams
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-inspector-actions
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
```

Remote-view focused gates when Rust route acquisition changes:

```bash
cargo test --manifest-path cli/Cargo.toml remote_view -- --nocapture
pnpm test:remote-view-open-fixture-live
```

Service/client contract gates when request, schema, or generated client shapes
change:

```bash
pnpm test:service-client
pnpm test:browser-capability-registry-draft
```

Live closeout gates for this plan:

```bash
pnpm audit:route-handoff -- --json
agent-browser --json remote-view open https://www.facebook.com/ \
  --runtime-profile last30days-facebook \
  --browser-build stealthcdp_chromium \
  --view-stream-provider rdp_gateway
```

The final live proof must show:

- `operatorVisible.state=ready`;
- route, display, browser, profile, and target IDs agree;
- no terminal-only or terminal-topmost route state;
- dashboard workspace URL without a stale `tab` param renders the RDP browser;
- direct Guacamole URL renders the same browser;
- no active incident or stale retained row appears in the live control rail.

Validation progress:

- 2026-06-23: Revalidated the broad no-launch and selected live gates after
  the P44 route, dashboard, service-contract, and foreign-CDP changes. The
  previously open `pnpm test:service-cdp-tab-streaming-live` concern did not
  reproduce in this run; the smoke passed with service-owned session
  `session:cdp-tab-stream-3327965` and stream `37647`. The selected validation
  matrix now has current pass evidence for:
  - `pnpm validation:select -- --base HEAD`
  - `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
  - `pnpm test:browser-capability-registry-draft`
  - `pnpm test:route-confusion-gates`
  - `pnpm test:service-cdp-tab-streaming-live`
  - `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1`
  - `cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1`
  - `pnpm test:service-api-mcp-parity`
  - `pnpm test:service-client`
  - `pnpm test:dashboard-selected-workspace-context`
  - `pnpm test:dashboard-selected-workspace-chat-packet`
  - `pnpm test:dashboard-selected-workspace-console`
  - `pnpm test:dashboard-launcher-eligibility`
  - `pnpm --dir docs build`
  - `pnpm build:dashboard`
  - `git diff --check`
  - `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`

  P44 remains open on the final live route-bound closeout: the installed
  root-owned privileged helper still requires an interactive refresh before a
  cold route desktop can prove no terminal-first session and before the final
  Facebook `remote-view open` dashboard/direct-Guacamole proof can close.
- 2026-06-23: Cleared the non-privileged runtime convergence noise that was
  masking the final P44 blocker. The live dashboard manifest probe was flaky
  because `/api/runtime/manifest` recomputed the embedded dashboard hash and
  reread the 275 MB native executable on every request, which could exceed
  install doctor's short local read timeout and surface as
  `dashboard_runtime_stale_or_unreadable`. `runtime_manifest_json()` now caches
  the manifest once per dashboard process. After publishing the local dashboard
  runtime with `pnpm publish:local-dashboard -- --skip-browser --json`, install
  doctor reported `liveDashboardRuntime.state=ready`,
  `runtimeConvergence.status=converged`, and `staleCount=0`. Current remaining
  blockers are narrowed to:
  - installed helper drift:
    `remote_view_route_desktop_helper_stale` and
    `remote_view_privileged_helper_status_stale`, with
    `helperDesktopSession.state=terminal_first_template` and
    `helperStatus.success=false`;
  - route B display repair: route A connection 3 has abstract X11 socket
    `@/tmp/.X11-unix/X11`, while route B connection 4 still reports no X11
    socket for `:14`;
  - service-status no-launch readiness remains blocked by the same remote-view
    install drift.

  Validation passed:
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml install_doctor_dashboard_runtime_manifest_shape_stays_public_safe -- --nocapture`
  - `pnpm publish:local-dashboard -- --skip-browser --json`
  - `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
  - compact readbacks from `agent-browser install doctor --json` and
    `agent-browser doctor remote-view --json`
- 2026-06-23: Rechecked the route-bound Facebook one-liner with fresh
  route-pool evidence. Route B was a stale local config problem rather than a
  missing Guacamole route: `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME` pointed at
  `:14`, while the live route-specific XRDP session is on `:12`. Updating the
  local runtime env made the route-pool readiness smoke pass for route A
  `:11` and route B `:12`, both through abstract X11 sockets. A narrow
  privileged display grant then made `doctor remote-view` report display access
  for both displays, but `remoteControl` and `install doctor` still correctly
  remain blocked because the installed root-owned helper lacks the P44
  `status-json` contract and still reports the terminal-first route desktop
  template. A dry run with request-scoped route-pool JSON selected connection
  `3`, route `guacamole:3`, route user `agent-browser-rdp-a`, and display
  `:11`, proving fresh request evidence outranks stale retained route-pool
  state for the CLI path. The live open now reaches route A and fails closed
  with lease rollback instead of leaving a CDP-only or terminal-only live row.
  The structured failure is `wrong_tab`: route, display, stream, and
  Guacamole are ready, but the selected target
  `C56AA9196002956C9838EA9A4FFE5914` still reports `about:blank` for the
  requested `https://www.facebook.com/`. This moves the remaining software
  repair boundary to selected-target navigation readiness and P45's dedicated
  tab module.
- 2026-06-23: Repaired the selected-target readiness failure and the retained
  route-pool convergence gap. The root cause of the repeated `wrong_tab`
  failure was the reuse path: the planner selected a same-origin target from
  cached metadata, but live `tabSwitch` proved that target was still
  `about:blank`, and only the open-new path ran the selected-target readiness
  wait. `remote_view_open` now runs the same target-readiness helper on reused
  targets, drains CDP target events while waiting, updates and persists the
  service tab handle after readback, and includes target switch/navigation
  diagnostics in `operatorVisible.components.tab`. The open path also persists
  all request-scoped route-pool entries from fresh `routePool` evidence on
  non-dry-run opens, so non-selected retained rows converge too. Final local
  publish installed executable SHA
  `fc5da74cb9813b4b3eb5453d7ee17465c9ced8ebb781b4cc1b8a28531e7d6b1d`.

  Final live proof:
  - fresh route-pool readiness exported route A `:11` and route B `:12`;
  - `agent-browser --json remote-view open https://www.facebook.com/
    --runtime-profile last30days-facebook --browser-build stealthcdp_chromium
    --view-stream-provider rdp_gateway` returned `success=true`,
    `status=opened`, `operatorVisible.state=ready`, route `guacamole:3`,
    route-pool entry `guacamole-rdp-a`, and public operator URL
    `https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MwBjAHBvc3RncmVzcWw=`;
  - selected target `67704DA80474C9FE718CB3FBE7D378AF` reported title
    `Facebook`, URL `https://www.facebook.com/`, and `urlReadiness=ready`;
  - `agent-browser --json service status` persisted one ready
    `session:default` browser on profile `last30days-facebook`, PID `906954`,
    one valid Facebook tab, and an RDP gateway view stream for connection `3`;
  - retained route-pool state now has `guacamole-rdp-a` checked out on `:11`
    and `guacamole-rdp-b` available on `:12`, replacing the stale retained
    `:14` row.

  Validation passed:
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo test --manifest-path cli/Cargo.toml remote_view_open -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml route_pool -- --nocapture`
  - `cargo test --manifest-path cli/Cargo.toml tab_handle -- --nocapture`
  - `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
  - `pnpm publish:local-dashboard -- --skip-browser --json`
  - `git diff --check`

  P44 remains open for the privileged install boundary and cold desktop proof:
  `agent-browser doctor remote-view --json` still reports the installed
  root-owned helper as stale (`remote_view_route_desktop_helper_stale` and
  `remote_view_privileged_helper_status_stale`), and the route desktop still
  contains the old XTerm session until the helper is refreshed from an
  interactive sudo boundary and the route desktop is cold-started.
- 2026-06-24: Resumed from the fresh-session handoff and reaudited the
  remaining closeout gates against current runtime state. Graphiti runtime was
  healthy, but the focused `agent_browser_main` query did not return a
  P44-specific sourced memory episode, so repo files and live command evidence
  remained authoritative. CodeGraph index files exist under `.codegraph/`, but
  CodeGraph MCP tools were not exposed in this session; direct source reads and
  command evidence were used for the audit.

  Non-privileged runtime drift was repaired. `pnpm publish:local-dashboard --
  --skip-browser --json` rebuilt and restarted the local dashboard runtime with
  executable SHA
  `fc5da74cb9813b4b3eb5453d7ee17465c9ced8ebb781b4cc1b8a28531e7d6b1d` and
  dashboard SHA
  `2caf3aca7718add187c9835e2b5020a07ff69b8abe3cf3fb0c8c591785757677`.
  `agent-browser install doctor --json` now reports
  `liveDashboardRuntime.state=ready` and
  `runtimeConvergence.status=converged`.

  Route readiness is green. `scripts/smoke-rdp-guac-route-pool-readiness.js
  --report-only` selected `guacamole-rdp-a` as route `guacamole:3`,
  connection `3`, display `:11`, user `agent-browser-rdp-a`, and
  `guacamole-rdp-b` as route `guacamole:4`, connection `4`, display `:12`,
  user `agent-browser-rdp-b`; both entries reported readiness `ready`.
  `scripts/inspect-rdp-route-displays.js` reported both route-specific Xorg
  displays present. The cold restart/readback state smoke also passed with
  artifacts under
  `/tmp/agent-browser-rdp-guac-cold-restart-2026-06-24T03-22-06-552Z`.

  The blocker remains the installed root-owned privileged helper and live
  terminal desktop proof. Source helper SHA is
  `8331be13d7f02bae15c026bfddef24a3cc2e9ba1245720b7851b1c1a3a9385f7`,
  but `/usr/local/libexec/agent-browser/agent-browser-privileged-helper` is
  still
  `e5bab71e89028c718581c8afb044219658a766dffadcc33a1c8bd28b96b6a336`.
  `pnpm install:privileges -- --apply` still reports that the installed helper
  differs from the bundled helper, then fails because sudo requires an
  interactive terminal. `agent-browser install doctor --json` still reports
  `remote_view_route_desktop_helper_stale`,
  `remote_view_privileged_helper_status_stale`,
  `helperDesktopSession.state=terminal_first_template`, and
  `helperStatus.success=false` with `Unknown command: status-json`.
  `scripts/inspect-rdp-route-displays.js --display-content` shows route A with
  Facebook Chromium plus an XTerm window and route B as `terminal_only` with
  XTerm. P44 therefore remains open until an interactive helper refresh,
  cold route restart, and terminal-free dashboard/direct-Guacamole proof pass.
- 2026-06-24 final closeout: The privileged helper was refreshed outside the
  prior noninteractive sudo boundary, then the route users were rewritten and
  XRDP was restarted through the refreshed helper. Source and installed helper
  SHA now match:
  `8331be13d7f02bae15c026bfddef24a3cc2e9ba1245720b7851b1c1a3a9385f7`.
  `sudo -n /usr/local/libexec/agent-browser/agent-browser-privileged-helper
  status-json` reports helper version
  `2026-06-23.p44-route-desktop-v2`,
  `routeDesktopSession.state=browser_control_ready_template`,
  `terminalStartupDetected=false`, and abstract X11 socket support.

  Route displays were cold-started onto route A `:13` and route B `:14`, and
  `/home/ecochran76/.agent-browser/.env` now pins
  `AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME=:13` and
  `AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME=:14`. The route display inspector now
  prefers those configured displays, falling back to the newest route-user Xorg
  PID when no configured display is present. Final display proof is stored at
  `/tmp/agent-browser-p44-display-content-final.json`: route A is
  `browser_window_visible` with `Facebook - Chromium`, route B is
  `non_browser_windows`, and neither route display contains XTerm.

  The install doctor service-status probe was also hardened. The isolated
  no-launch `service status` response is large enough to fill a pipe when the
  timeout wrapper waits without draining stdout, so the doctor command runner
  now spools stdout and stderr to temporary files while polling child status.
  The service-status probe has a bounded 15 second timeout and no longer
  deadlocks on the large JSON payload. Final local publish installed executable
  SHA
  `9628c8111540494abc632370259fe5039748a9742ca8af4441473b65885ae3b8` and
  dashboard SHA
  `88e513b00a196f9cb8d53dc658ef23b21d2fb7e53ee6f7c3462146ecb263e4ea`.

  Final live proof:
  - `/tmp/agent-browser-p44-route-pool-after-publish-close.json` reports route
    A `guacamole:3` on `:13` and route B `guacamole:4` on `:14`, both ready
    with Guacamole, RDP, login, permissions, and abstract X11 socket evidence;
  - `/tmp/agent-browser-p44-facebook-open-after-spool-publish.json` reports
    `success=true`, `status=opened`, route-pool entry `guacamole-rdp-a`, route
    `guacamole:3`, display allocation `remote-view-display:13`,
    `operatorVisible.state=ready`, selected URL `https://www.facebook.com/`,
    and visible-window proof containing `Facebook - Chromium`;
  - `/tmp/agent-browser-p44-install-doctor-final.json` reports
    `success=true`, no issues, `remoteViewPrivileges.ready=true`,
    `service.ready=true`, `service.timedOut=false`, and
    `runtimeConvergence.status=converged`;
  - `/tmp/agent-browser-p44-remote-view-doctor-final.json` reports
    `success=true`, `status=ready`, no issues, `remoteControl.ready=true`,
    `installReady=true`, `routePoolReady=true`, `routeDisplayReady=true`, and
    route display `:13`;
  - `/tmp/agent-browser-p44-service-status-final-after-resolve.json` reports
    route A checked out on `:13`, route B available on `:14`, browser
    `session:default` healthy on profile `last30days-facebook`, and zero active
    incidents;
  - `/tmp/agent-browser-p44-service-incidents-summary-after-resolve.json`
    reports no active incidents after resolving the stale route-viewer
    incidents created by the intentional installed-binary refresh close;
  - `/tmp/agent-browser-p44-guacamole-route-a-final.html` was fetched from the
    public Guacamole route and returned HTTP 200 through the authenticated
    dashboard entry path.

  Final validation passed:
  - `cargo fmt --manifest-path cli/Cargo.toml -- --check`
  - `cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture`
  - `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
  - `pnpm test:rdp-route-xsession`
  - `pnpm test:dashboard-inspector-actions`
  - `pnpm publish:local-dashboard -- --skip-browser --json`
  - `git diff --check`

## Closeout Criteria

P44 is complete only when:

- route-bound open has one normalized intent path;
- strict acquisition is the default for operator-visible opens;
- repair fallback behavior is explicit;
- visible proof is strong enough to match the operator view;
- route desktops no longer foreground terminal UI;
- duplicate tab and stale target behavior is deterministic;
- left rail groups ownership and actionability correctly;
- foreign CDP rows are addressable but clearly non-owned;
- incident resolution and route cleanup are service actions, not manual state
  edits;
- one cold-start Facebook live smoke passes through the dashboard and direct
  Guacamole link.
