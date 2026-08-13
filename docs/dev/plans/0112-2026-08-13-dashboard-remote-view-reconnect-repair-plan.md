# Plan 0112: Dashboard Remote-View Reconnect Repair

Date: 2026-08-13

State: CLOSED

Lane: P112

Source baseline: `23a4aa8b76bd`

## Goal

Make dashboard viewer reconnect use the authoritative service-owned remote-view
route and allow the state-backed lease request enough bounded time to return its
typed result. The dashboard must report the backend failure that actually
occurred and render readiness guidance without duplicated wording.

## Incident Evidence

- The dashboard reported `Viewer reconnect failed` after its Service API proxy
  timed out reading a retired per-session stream port.
- The persisted reconnect job used pseudo-route `daemon:qbo-soylei` and failed
  with `remote view route 'daemon:qbo-soylei' not found`.
- The selected browser's authoritative RDP view stream was bound to
  `guacamole:2`.
- The 6.46 MB service state required about 3.5 to 4.0 seconds for current status
  responses, while ordinary dashboard requests were limited to two seconds.
- Current remote-view doctor evidence reports ready route displays, Guacamole
  local and public ingress, route pool, and runtime convergence.

## Frozen Repairs

### 1. Authoritative viewer route selection

Viewer lease request, release, and controller takeover use an eligible
service-owned RDP stream route. A selected daemon CDP stream is presentation
state, not viewer-route authority. When the selected stream is not eligible,
the dashboard resolves the first eligible RDP route from the same immutable
workspace projection. Synthetic `daemon:`, `foreign-cdp:`, and
`service-cdp-snapshot:` routes cannot enter viewer lease requests.

### 2. Bounded remote-view request timeout

Dashboard Service API proxy classification keeps the two-second default for
ordinary requests, keeps the existing sixty-second durable handoff resolution
allowance, and assigns a fifteen-second bound to registered remote-view route,
viewer lease, and controller lease actions. This bound exceeds the current
state-backed request time while remaining finite and shorter than durable
handoff resolution.

### 3. Typed backend failure rendering

Dashboard recovery errors preserve the backend's error string and stable code.
When both are present, the rendered failure includes the code exactly once.
Transport failures continue to report their proxy code and stage. Invalid or
non-JSON responses remain bounded and fail closed.

### 4. Readiness wording

Readiness component labels are normalized before composing titles, evidence,
and recovery guidance. A component already named `readiness` renders
`Readiness failed` and `Inspect readiness before opening the workspace stream`,
never `readiness readiness failed`.

## Test Strategy

Use vertical red-to-green slices through existing interfaces:

1. dashboard view-stream test proves a selected CDP stream resolves the
   authoritative RDP viewer route and rejects synthetic-only candidates;
2. focused Rust tests prove exact timeout classification for remote-view,
   handoff, and ordinary actions;
3. dashboard recovery test proves typed error formatting and non-duplication;
4. workspace readiness test proves normalized title and recovery copy.

After focused tests pass, run the validation selector and every selected Rust,
dashboard, service-client, formatting, lint, build, and documentation gate.

## Installation And Live Acceptance

This is a roadmap checkpoint install, not a formal release. After validation:

1. commit coherent source slices and push the current architecture branch;
2. build the native binary and dashboard assets from the pushed commit;
3. install through the repository's user-scoped checkpoint workflow;
4. restart the dashboard and daemon sessions without launching replacement
   browsers or changing profile, route, authentication, or page state;
5. prove installed binary, packaged binary, manifest, dashboard assets, and
   every active agent-browser process match the built artifacts;
6. rerun install doctor, remote-view doctor, listener checks, and the repaired
   viewer reconnect request against the existing QBO route;
7. require a persisted active observer lease on `guacamole:2` and a successful
   dashboard response before closing the plan.

## Worktree Reconciliation

Pre-existing `daemon.rs` and `service_store.rs` changes are the source of the
currently installed SIGABRT and large-state stack repairs. They remain a
separate coherent commit, but they must be validated, committed, pushed, and
included in the same checkpoint installation so source provenance and active
runtime state do not diverge.

## Acceptance Criteria

- All four frozen repairs have focused regression coverage that fails on the
  incident behavior and passes on the repaired behavior.
- No viewer lease request can carry a synthetic daemon route when an
  authoritative RDP stream exists.
- Current large-state viewer recovery completes within the bounded proxy
  allowance and surfaces its typed backend result.
- The existing browser, profile, display, and Guacamole route remain in place.
- The branch is committed and pushed.
- Installed and active components are byte-for-byte synchronized with the
  validated build and current pushed commit.
- Live readback proves remote-view readiness and a successful observer lease on
  the exact current route.

## Closeout Evidence

- Commits `885db6db` and `cadccee3` are pushed on
  `architecture-deepening-20260809`.
- Focused route-selection, timeout, typed-error, and readiness-copy tests pass,
  along with the selected dashboard, Rust, documentation, and installation
  gates.
- The supported checkpoint publisher installed matching native and dashboard
  artifacts. Runtime convergence reports no stale executable processes, and
  the workstation payload and session supervisors report current provenance.
- The existing QBO browser remained PID `51579` on profile `qbo-soylei` and
  display allocation `display:shared_display:qbo-soylei-11`.
- A dashboard-proxied browser reattach returned success, restored
  `attached_ready` on `guacamole:2`, and a dashboard-proxied observer lease
  request persisted an active observing lease on that route.
- The installed dashboard workspace smoke passed with the repaired reconnect
  marker, the RDP gateway viewport, ready runtime health, and the existing QBO
  browser process.
- Install doctor has no binary, manifest, dashboard, workstation-payload,
  supervisor, or runtime-convergence drift. Its sole remaining warning is the
  pre-existing P111 duplicate-profile-pressure evidence for two empty retained
  `default` profile rows; reviewed retained-state cleanup exposes no safe
  candidate, so P112 does not delete that forensic state.
