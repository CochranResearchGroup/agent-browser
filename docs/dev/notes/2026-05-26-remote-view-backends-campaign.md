# Remote View Backends Campaign

Date: 2026-05-26

## Trigger

The live dashboard can now launch and focus remote-headed browser workspaces,
but the current RDP and Guacamole deployment still behaves like a prototype in
important operator flows:

- one dashboard client can take over another client that is viewing the same
  Guacamole connection
- iframe and popout clients can compete for the same remote desktop
- stale retained tab identity can make a healthy browser look unavailable
- multiple retained daemon sessions can point at the same browser and confuse
  workspace ownership
- non remote-desktop browsers do not yet have a default remote review stream

The product goal is not to replace one provider with another prematurely. The
goal is to make Agent Browser's `viewStreams` contract support several
backends, pick the right default by browser posture, and keep fallback paths
available if one backend proves weak in operator use.

## Source Context

Relevant repo authorities:

- `docs/dev/notes/2026-04-22-agent-browser-service-roadmap.md` names
  remote-headed browser management as a roadmap pillar and separates
  `BrowserHost`, `ViewStreamProvider`, and `ControlInputProvider`.
- `docs/dev/notes/2026-05-20-remote-view-control-posture-checkpoint.md`
  records `rdp_gateway` and `manual_attached_desktop` as access-plan posture
  choices, with queued `view_focus` before opening the dashboard stream.
- `docs/dev/notes/2026-05-23-left-pane-workspace-navigator-campaign.md`
  makes the left pane and viewport a service-owned workspace control surface.
- `docs/dev/notes/2026-05-23-left-pane-workspace-navigator-slice-8.md`
  records that the launcher and workspace viewport are live, URL-stable, and
  backed by rendered inspection.
- Live validation on 2026-05-25 and 2026-05-26 proved that the current
  Guacamole-backed RDP connection can render the UPS browser, but behaves as a
  single-active-viewer path when two clients attach to the same connection.

Graphiti discovery for `agent_browser_main` on 2026-05-26 was healthy and
returned the existing service-control-plane direction: Agent Browser should own
browser lifecycle, CDP connections, and authoritative service state. The repo
files above remain the source of truth for this campaign.

## Product Decision

Support three remote viewing families behind one service-owned stream model:

1. **RDP and Guacamole**
   Keep this path and harden it first. It already works for full-desktop human
   control, but it must be productized for one human moving between several
   devices and for many concurrently tracked RDP-capable browser workspaces.
2. **CDP streaming**
   Add CDP screencast streams for browsers that have CDP enabled but are not
   remote-control desktop browsers. Read-only streaming is required first and
   should become the default review path for non RDP workspace rows. Read-write
   CDP input can follow as an explicit UX-enabled mode. The normal managed
   browser build for this path is `stealthcdp_chromium` when validated and
   allowed by policy.
3. **VNC and noVNC**
   Keep VNC/noVNC as the likely better multi-client desktop path, but sequence
   it after RDP hardening and CDP review streams. It is more development work
   because Agent Browser must own display allocation, stream process lifecycle,
   input routing, and multi-client semantics instead of relying on the current
   Guacamole RDP deployment.

The near-term priority is reliability, not backend churn:

1. Harden RDP and Guacamole.
2. Add CDP read-only streaming and make it the default for non RDP browsers.
3. Add CDP read-write mode behind an explicit UX and policy gate.
4. Add VNC/noVNC as a switchable multi-client remote desktop backend.

The detailed hardening and testing plan for item 1 is
`docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`.

## Backend Choice Matrix

### RDP And Guacamole

Use for full remote-control browser workspaces where a human needs the desktop,
native browser window, keyboard and mouse, browser chrome, extensions, native
dialogs, or OS-level prompts.

Expected product behavior:

- one active controller per Guacamole connection
- clear takeover UX when another dashboard, popout, or device owns the
  connection
- no silent white, black, unhappy-frame, or stale-tab states when the browser is
  still healthy
- stable tracking for many RDP-capable browser workspaces
- no duplicate retained sessions claiming the same live tab or CDP endpoint
- iframe and popout behavior is explicit: shared observation only when proven,
  otherwise deliberate transfer with a Take over affordance

Known tradeoffs:

- quick to keep improving because the current stack already exists
- good desktop semantics and dynamic browser-window resize
- likely single-active-viewer for one connection
- per-browser or per-display allocation is needed before many remote-control
  browsers become reliable

### CDP Streaming

Use for review streams when a browser has an active CDP endpoint but does not
need a full remote desktop. This includes ordinary managed local headless,
local headed, attached, or service-owned browsers when policy allows CDP.

Expected product behavior:

- `cdp_screencast` read-only stream available by default for non RDP browsers
  with reachable CDP
- dashboard rows can show a remote review viewport even when no RDP or VNC
  gateway exists
- read-only mode cannot send input
- read-write mode is enabled explicitly from the UX and recorded as a
  service-owned operator control state
- site policy can suppress CDP streaming where CDP presence or CDP input is not
  acceptable

Known tradeoffs:

- lightweight and broadly useful for review
- no OS desktop, native file picker, extension popup, or non-tab prompt
  coverage
- CDP must be enabled and reachable
- input through CDP should be gated because it competes with agent automation
  and may be site-policy-sensitive

### VNC And noVNC

Use for future full-desktop remote-control workspaces where multiple observers
or simpler shared-framebuffer behavior matter more than RDP session semantics.

Expected product behavior:

- same service `viewStreams` abstraction as RDP and CDP
- per-workspace display allocation
- one controller lock with optional multiple read-only observers
- noVNC stream endpoint owned by Agent Browser service state
- switchable backend so RDP can remain the fallback if VNC/noVNC is not stable
  enough

Known tradeoffs:

- better fit for multi-client observation
- more implementation work than hardening the current RDP stack
- clipboard, keyboard layout, resize, auth, and stream lifecycle require more
  product polish

## Cross-Cutting Contracts

- The service remains the source of truth for stream provider, stream URL,
  input provider, display isolation, connection ownership, takeover state, and
  browser/tab focus.
- The dashboard can derive presentation models, but it must not invent mutable
  stream or ownership truth.
- `viewStreams` must represent both desktop streams and tab-review streams.
- Every stream should report whether it is embeddable, externally openable,
  read-only, controllable, currently owned, or temporarily unavailable.
- Opening a controllable stream must queue a service-owned focus or takeover
  request before embedding when that backend supports it.
- A stale selected tab must not prevent the dashboard from focusing a live
  browser if the service can reconcile a current CDP target.
- Multi-device behavior must be explicit. If the provider is single-viewer,
  the UI should say so and offer Take over. If multiple viewers are supported,
  the UI should distinguish observer and controller roles.

## Policy Contract For Every Slice

Each slice in this campaign must follow the adopted repo policy:

- Start with `git status --short` and treat pre-existing dirty state as a
  constraint.
- Re-read the relevant planning policy before changing roadmap or campaign
  authority.
- Use Graphiti discovery at the start of non-trivial planning, debugging, or
  handoff work, then verify useful claims against repo files or tests.
- Keep service-owned authority for provider choice, stream readiness, browser
  focus, takeover, and input.
- Do not let dashboard-only behavior bypass service request contracts.
- Use `pnpm validation:select -- --base <ref>` for implementation slices and
  run or justify the recommended checks.
- For UI-affecting slices, visually inspect with `agent-browser` during the
  slice across desktop and mobile-width views, including iframe and popout
  routes when stream UX changes.
- For remote-view correctness, use two independent browser clients before
  calling a path healthy. Prove either simultaneous viewing or clean
  single-viewer transfer.
- For user-facing behavior, update the docs surfaces required by `AGENTS.md`.
- Close every slice with validation evidence, screenshot paths when relevant,
  and residual risk.

## Planning Slices

### Slice 1: RDP/Guac Ownership And State Audit

Goal: make the existing RDP and Guacamole path explainable before adding more
backend surface area.

Scope:

- Audit the current retained state for browser, session, tab, display,
  `viewStreams`, and Guacamole connection identity.
- Define the canonical ownership relation for one RDP-backed browser
  workspace: service session, daemon session, CDP endpoint, current tab,
  remote display, Guacamole connection, and active viewer.
- Add or update no-launch diagnostics that detect duplicate sessions claiming
  the same live CDP endpoint or tab.
- Add a dashboard-facing state vocabulary for Guacamole readiness, connected,
  disconnected, owned by another client, stale selected tab, and unavailable.
- Document the expected single-active-viewer behavior for the current
  Guacamole connection.
- Ground the audit in the detailed RDP and Guacamole plan at
  `docs/dev/plans/0001-2026-05-26-rdp-guac-hardening-test-plan.md`.

Validation:

- `pnpm test:dashboard-view-streams`
- focused service-state or dashboard helper tests for duplicate ownership
  detection
- `pnpm validation:select -- --base <ref>`
- `git diff --check`

Handoff:

- List any runtime state repaired manually.
- Include the exact ownership model and fields still missing from service
  state.
- State whether the slice changed behavior or only surfaced diagnostics.

### Slice 2: RDP/Guac Multi-Device Takeover Productization

Goal: make one human moving between devices reliable and understandable.

Scope:

- Productize Take over for iframe, popout, mobile, and desktop clients.
- Detect Guacamole single-active-viewer disconnects without leaving a white or
  black viewport.
- Make iframe and popout transfer behavior explicit so they do not silently
  fight each other.
- Show the active viewer or last viewer when the backend can supply it, or show
  a clear provider-limited message when it cannot.
- Keep the selected browser session alive during client disconnect, refresh,
  route change, and device transfer.
- Add visual regression coverage for disconnected, takeover-ready, connecting,
  and connected viewport states.
- Pass the two-active-viewer transfer requirement from the detailed plan:
  client A and client B must either view simultaneously or transfer control
  cleanly in both directions without blank, black, or unhappy-frame states.

Validation:

- `pnpm test:dashboard-view-streams`
- `pnpm test:service-dashboard-remote-control-ui-live`
- `pnpm test:rdp-gateway-readiness-live` on an operator workstation
- rendered `agent-browser` two-client smoke with different browser
  executables proving clean transfer from client A to client B
- desktop and mobile screenshots of connected and takeover states
- `git diff --check`

Handoff:

- State whether same-connection simultaneous viewing is supported or explicitly
  single-viewer.
- Include screenshot paths and the two-client proof.
- Record remaining Guacamole or ingress failure modes.

### Slice 3: RDP/Guac Many-Browser Tracking

Goal: make RDP-backed remote-control workspaces reliable when many browser and
profile combinations exist.

Scope:

- Model RDP display and Guacamole connection allocation per workspace, with
  shared display allowed only as an explicit low-contention override.
- Track display name, stream URL, Guacamole connection id or route, browser id,
  daemon session id, and profile id together.
- Prevent duplicate retained browser/session rows from claiming the same live
  tab unless they are explicitly modeled aliases.
- Add repair or reconcile behavior for stale selected tabs and duplicate
  ownership after daemon restart or manual browser reuse.
- Keep launcher defaults on the hardened RDP path until a different backend is
  selected.
- Pass the managed remote browser switching requirement from the detailed
  plan: one viewer must move from remote browser A to remote browser B, refresh
  on B, and return to A without stale stream state or duplicate browser
  ownership.

Validation:

- service reconcile tests for stale tab and duplicate ownership repair
- dashboard workspace-node tests for many RDP-backed workspaces
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-view-streams`
- rendered `agent-browser` inspection with at least two RDP-capable workspace
  rows
- `git diff --check`

Handoff:

- List the allocation fields that are authoritative.
- State whether per-workspace private displays are implemented or still
  planned.
- Include retained-state before and after examples for duplicate repair.

### Slice 4: RDP/Guac Reliability Gate

Goal: make the existing RDP backend supportable as the first production remote
control path.

Scope:

- Add a compact readiness surface for `xrdp`, `xrdp-sesman`, `guacd`,
  Guacamole web app, connection permissions, backend TCP reachability,
  dashboard auth, and public ingress.
- Surface readiness failures in the launcher and workspace viewport before an
  operator sees a blank stream.
- Preserve the existing CLI and script diagnostics, but make the dashboard
  reason strings actionable.
- Record operational recovery steps for common failures: connection missing,
  refused iframe, black popout, white desktop, auth expired, and active viewer
  takeover.
- Treat the detailed plan's combined viewer-transfer and browser-switching
  smokes as the reliability gate before calling RDP/Guac productized.

Validation:

- `pnpm test:rdp-gateway-readiness-live`
- dashboard source tests for readiness and disabled-state copy
- public and local route smokes where available
- rendered `agent-browser` inspection of healthy and failed readiness states
- `git diff --check`

Handoff:

- Include the readiness result payload shape.
- State which checks are local-only and which prove public ingress.
- Include any manual operator steps that remain outside Agent Browser.

### Slice 5: CDP Read-Only Screencast Streams

Goal: make non RDP browsers remotely reviewable by default when CDP is enabled.

Scope:

- Add a service-owned `cdp_screencast` read-only stream record for browsers
  with reachable CDP and no preferred desktop stream.
- Keep `stealthcdp_chromium` as the preferred managed build when policy allows
  CDP and the validated build is available.
- Add dashboard viewport support for read-only CDP frames without exposing
  input controls.
- Make non RDP workspace rows prefer CDP read-only View instead of appearing
  uninspectable.
- Respect site policy that disables CDP streaming or marks CDP attachment as
  unsafe.

Validation:

- service contract tests for `cdp_screencast` stream records
- dashboard stream tests for read-only capability and disabled Control
- `pnpm test:dashboard-view-streams`
- `pnpm test:service-client-contract`
- rendered `agent-browser` inspection of a non RDP browser row opening a
  read-only review viewport
- `git diff --check`

Handoff:

- Include the frame transport and lifecycle contract.
- State which browser hosts can emit CDP read-only streams.
- Record policy cases where the stream is intentionally suppressed.

### Slice 6: CDP Read-Write Control Mode

Goal: let operators enable CDP input deliberately when policy and ownership
allow it.

Scope:

- Add a service-owned CDP input provider state, separate from read-only CDP
  streaming.
- Gate read-write mode through UX, site policy, stream capability, and operator
  takeover or control lease.
- Show clear visual state for read-only review, control requested, control
  active, and control released.
- Route mouse and keyboard through service-owned CDP input commands rather than
  dashboard-local shortcuts.
- Record audit events for enabling, using, and releasing CDP read-write
  control.

Validation:

- backend contract tests for CDP control mode and audit events
- dashboard tests for enable and release controls
- `pnpm test:dashboard-view-streams`
- rendered `agent-browser` inspection of read-only to read-write transition
- live smoke against a harmless page proving input only after enablement
- `git diff --check`

Handoff:

- Include the policy gate and audit event names.
- State how agent queue coordination works while CDP read-write control is
  active.
- Record any input types deferred from the first implementation.

### Slice 7: VNC/noVNC Backend Spike

Goal: decide the minimum viable noVNC backend without disturbing the hardened
RDP path.

Scope:

- Prototype one service-owned VNC/noVNC display path in an isolated runtime.
- Compare multi-client behavior against the RDP single-viewer model.
- Validate browser painting, resize, keyboard, mouse, clipboard expectations,
  and stream lifecycle.
- Keep provider selection explicit through `viewStreamProvider=novnc` and
  `controlInputProvider=vnc_input`.
- Do not make VNC the default in this slice.

Validation:

- no-launch contract test for `novnc` stream metadata
- one isolated live spike smoke with two independent browser clients
- screenshots for simultaneous observer or clean control-lock behavior
- `git diff --check`

Handoff:

- Recommend whether to proceed, pause, or discard noVNC for the next campaign
  slice.
- Record operational dependencies and rough performance observations.
- List gaps versus RDP hardening.

### Slice 8: VNC/noVNC Productization And Backend Switching

Goal: make VNC/noVNC a supported, switchable backend after the spike proves it
is worth carrying.

Scope:

- Add service-owned lifecycle for noVNC or websockify processes as needed.
- Integrate noVNC display allocation with the same workspace and stream model
  as RDP.
- Support multiple read-only observers with a single controller lock when the
  backend permits it.
- Keep RDP as a selectable fallback in access-plan, launcher, and site policy.
- Update docs and installed skill guidance so agents and operators know when
  to choose RDP, CDP streaming, or VNC/noVNC.

Validation:

- service lifecycle tests for noVNC process and stream records
- dashboard tests for backend switching and observer/controller states
- rendered `agent-browser` two-client desktop and mobile inspection
- docs build and selector-recommended tests
- `git diff --check`

Handoff:

- State the default backend decision after productization.
- Include fallback instructions for switching back to RDP.
- Include final screenshot paths and remaining provider limitations.
