# Plan 0113: Workspace Viewport Just-Works UX

Date: 2026-08-14

State: COMPLETE

Lane: P113

Source baseline: `67d8dc66ccc26e207dd4be220133ab4f25ff67bd`

## Goal

Selecting a healthy managed browser should display its best usable view without
requiring the operator to understand stream providers, route reattachment,
viewer leases, or iframe reloads. The normal viewport should present one clear
connection state and reserve provider mechanics for progressive disclosure.

## Incident Evidence

- The QBO browser was healthy, but its RDP stream reported a safely
  reattachable stale route and appeared under `Needs attention`.
- The browser became visible only after the operator guessed which large
  `Use RDP gateway` button would activate the useful presentation path.
- The focused viewport renders one large button for every stream record using
  only its provider label, so distinct records can have indistinguishable
  visible names.
- Two adjacent circular-arrow controls perform different operations: one
  reloads the viewport and service projection, while the other requests a new
  viewer lease.
- Current automatic retry begins only after an embeddable iframe exists. It
  does not first select a better stream or execute a safely recommended route
  recovery.

## Frozen Product Contract

### 1. One automatic connection coordinator

The dashboard derives one bounded next step for the selected workspace:

1. keep a currently usable explicit source;
2. otherwise select the best usable source, preferring the service-owned RDP
   desktop in control mode;
3. execute a service-recommended route reattach or route switch exactly once
   per browser, stream, route, and readiness generation when the browser is
   live and the remedy is marked safe;
4. request an observer lease when an attached-ready viewer route has no active
   viewer lease;
5. refresh the projection and load the frame;
6. retain the existing bounded frame retry after embed.

The coordinator never requests controller takeover, releases another viewer,
launches a browser, changes a profile, or retries an effect indefinitely.

### 2. Compact source selection

The normal viewport shows a compact `View` menu only when more than one
semantically distinct source exists. Choices use user-facing names such as
`Desktop`, `Live page`, and `Snapshot`, with provider and route detail in
secondary copy. Equivalent provider, route, display, and mode records are
deduplicated. No primary action is labelled `Use RDP gateway`.

### 3. One connection recovery action

The normal header does not expose separate reload, reattach, viewer reconnect,
takeover, and release icons. When automatic recovery cannot complete, one
visible `Retry connection` action reruns the bounded safe coordinator.
Controller takeover appears only when an ownership conflict is proven and is
labelled `Take control`.

Low-level operations remain available under an `Advanced connection controls`
menu with visible text labels. Fullscreen and opening the durable external view
remain ordinary presentation actions.

### 4. Honest state axes

Browser lifecycle, presentation connection, and control authority remain
separate facts. A live browser with a safely recoverable stream stays in the
active inventory and may show `Connecting` or `Reconnecting`. It enters
`Needs attention` only after recovery fails or an operator decision such as
authentication or takeover is required.

### 5. Progressive disclosure and accessibility

The header shows the browser name, one connection status, optional source
menu, fullscreen, and an overflow menu. Icon-only controls retain accessible
names, but materially different recovery operations must not rely on identical
icons or hover-only explanations. Connection progress and failures use polite
live-region feedback.

## Test Strategy

Use vertical TDD slices through public pure interfaces and the rendered source
contract:

1. selected unusable CDP plus ready RDP resolves to the RDP source without an
   operator click;
2. duplicate RDP records collapse into one `Desktop` choice;
3. a live browser with a reattachable route produces one automatic reattach
   step, then no duplicate step for the same attempt identity;
4. an attached-ready route without a viewer lease produces one observer-lease
   step;
5. takeover, release, browser launch, profile mutation, and repeated recovery
   are never automatic;
6. a safely recoverable live browser remains active inventory while a failed
   or authority-blocked browser remains in attention;
7. focused viewport source assertions prove the large provider buttons and
   ambiguous adjacent refresh icons are gone;
8. dashboard build and installed QBO workspace smoke prove the current browser
   opens through RDP without manual stream or lease actions.

## Documentation And Installation

Update CLI help, README, the repository Agent Browser skill, and the remote-view
docs to describe automatic connection and the Advanced fallback controls. On
source acceptance, commit and push the architecture branch, publish the local
dashboard checkpoint, synchronize the installed skill, and prove installed
binary, dashboard, workstation payload, supervisors, and active runtime
provenance remain converged.

## Implementation And Validation Evidence

- Added a pure workspace connection resolver that deduplicates equivalent
  sources, preserves a usable explicit choice, otherwise prefers a ready RDP
  desktop in control mode, and derives one bounded automatic recovery step.
- Added a compact semantic `View` menu and one connection-state badge. Moved
  iframe reload, route reattachment, viewer reconnect, controller operations,
  and input settings into `Advanced connection controls` with text labels.
- Added one visible `Retry connection` fallback. Automatic recovery can select
  a ready source, execute a service-approved route reattach or route switch,
  or request an observer lease once per readiness generation. It cannot launch
  a browser, change a profile, take control, release a viewer, or loop.
- Changed workspace classification so a live browser with an automatic safe
  recovery remains active, and a recovered informational incident no longer
  leaves the browser under `Needs attention`.
- Passed focused source, projection, navigator, inspector, viewport-controller,
  route-confusion, handoff-documentation, dashboard build, documentation build,
  Rust formatting, strict Clippy, and CLI-help gates.
- Published the release-mode checkpoint and synchronized the user binary,
  repository release binary, packaged Linux binary, dashboard assets,
  workstation payload, shared skill, and session-supervisor manifest.
- Installed runtime smoke passed against the QBO workspace with authenticated
  runtime health ready, RDP gateway frame attached, readiness `ready`, and
  workspace state `controllable`. Existing Chrome PID `51579`, profile
  `qbo-soylei`, CDP port `38415`, display `:11`, and route `guacamole:2` were
  preserved.
- Installed executable SHA-256 is
  `2ec94b993431da5c91db921250c2a7fadb363f2a9cb4bfa9a52e1b2712141452`;
  installed dashboard SHA-256 is
  `017b73508b33138b6b61bd552e26630db871daefb391dc760d41a3843d0e6d3e`.
- Install doctor reports no binary, dashboard, workstation, or supervisor
  drift. Its remaining duplicate-profile-pressure finding belongs to P111 and
  was deliberately preserved for forensic resolution.

## Acceptance Criteria

- [x] Selecting the healthy QBO workspace renders its RDP desktop with no stream,
  reattach, or viewer-lease click.
- [x] The normal viewport has no duplicate `Use RDP gateway` buttons and no two
  indistinguishable circular-arrow recovery controls.
- [x] QBO remains in active inventory during safe automatic stream recovery.
- [x] Automatic effects are bounded, idempotent per attempt identity, and never
  take control.
- [x] Focused tests, dashboard tests and build, documentation build, selected Rust
  gates, installed runtime smoke, and provenance readback pass.
- [x] The existing browser process, profile, authentication, display, and route
  identities are preserved.
