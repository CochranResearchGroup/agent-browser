# CDP Tab Streaming For Non-Remote Browsers Plan

Date: 2026-05-30
State: CLOSED
Lane: P08
Outcome: COMPLETE

Current state: `v0.27.0` ships robust Guacamole/RDP remote-view support for
remote-headed browsers and already contains a runtime WebSocket stream server
backed by CDP `Page.startScreencast`. The missing product surface is
service-owned, dashboard-openable CDP tab streaming for non-remote browsers:
local headless, local headed, attached CDP, and service-managed browser
records whose view does not need RDP, Guacamole, noVNC, or WebRTC.

## Purpose

Expose live browser-tab viewing for CDP-controllable non-remote browsers by
reusing the existing CDP screencast stream server and wiring it into service
state, tab focus, and dashboard view-stream controls.

The target behavior is:

- every eligible service-owned non-remote browser can advertise a
  `cdp_screencast` view stream
- the dashboard can open that stream for the active tab without launching a
  separate browser or requiring a remote desktop route
- tab-focused inspection can focus a specific service tab, attach to the
  right CDP target, and stream that tab
- streaming starts only when a viewer connects and stops when viewers leave
- service state reports readiness and failure reasons without pretending CDP
  streaming is available for CDP-free or non-CDP browsers

## Existing Source Findings

- `cli/src/native/stream/mod.rs` already owns `StreamServer`, WebSocket
  broadcast state, cached last frame, viewport dimensions, connected-client
  counts, CDP client slots, and the active CDP page session id.
- `cli/src/native/stream/cdp_loop.rs` already starts
  `Page.startScreencast`, sends an initial `Page.captureScreenshot`, acks
  `Page.screencastFrame`, and broadcasts `frame`, `status`, `url`, console,
  and page-error messages.
- `cli/src/native/stream/websocket.rs` already serves WebSocket clients,
  sends cached status/tabs/last-frame state, and accepts input messages for
  CDP-backed control.
- `cli/src/native/service_model.rs` already models
  `ViewStreamProvider::CdpScreencast` and `ControlInputProvider::CdpInput`.
- `cli/src/native/actions.rs` currently builds view streams primarily from
  `remote_headed` launch commands. Non-remote service browsers need their own
  CDP screencast stream records.
- `packages/dashboard/src/components/service-panel.tsx` already renders
  browser and tab view-stream affordances from `browser.viewStreams`; it needs
  a stable embeddable URL and tab-focus path for `cdp_screencast` streams.

CodeGraph MCP tools were not exposed in the current session, so these findings
come from direct repo inspection. They should be refined with CodeGraph if the
tools are available during implementation.

## Non-Goals

- Do not replace the Guacamole/RDP remote-headed path.
- Do not implement `chrome_tab_webrtc` or `virtual_display_webrtc` in this
  lane.
- Do not stream CDP-free browsers or browsers whose site policy forbids CDP
  attachment.
- Do not expose stream servers beyond loopback or dashboard-authenticated
  surfaces.
- Do not solve cloud-provider browser streaming unless the provider exposes a
  compatible CDP screencast target through the existing service browser record.
- Do not add recording, video encoding, audio, or multi-viewer controller
  leases beyond the existing stream server and dashboard controls.

## Product Invariants

- CDP streaming is represented as service state, not as an ad hoc CLI-only
  side channel.
- A stream record names the browser, active tab or focused tab, provider
  `cdp_screencast`, control input `cdp_input`, stream URL, readiness, and
  reason when unavailable.
- A browser with no reachable CDP endpoint, CDP-free posture, unsupported
  engine, or blocked site policy reports unavailable readiness instead of a
  broken open button.
- Streaming does not start until a viewer connects, and `Page.stopScreencast`
  runs when viewer count drops to zero, the browser changes, the target
  changes, or the stream server shuts down.
- Focusing a tab for viewing uses the service worker queue and persists a
  traceable service job/event rather than mutating shared browser state from
  the dashboard directly.
- Existing `agent-browser stream enable/status/disable` behavior remains
  backward compatible.
- The dashboard can open a non-remote CDP stream from both browser rows and
  tab details.

## Slices

### Slice A | Contract And Ownership Audit

Status: COMPLETE.

Goal: define the exact non-remote CDP stream contract before changing code.

Tasks:

- Trace current stream enable/status/disable, `StreamServer`, tab listing, and
  dashboard view-stream open paths.
- Decide whether the service owns one stream server per session, per browser,
  or per active browser with tab focus. Prefer the smallest change that keeps
  browser and tab rows honest.
- Define the `ViewStream.readiness` shape for CDP stream availability,
  unsupported engines, CDP-free posture, disconnected CDP, missing stream
  server, and no active tab.
- Decide how `frameUrl` and `externalUrl` should be represented for loopback
  WebSocket streams and dashboard-embedded viewers.

Exit criteria:

- A dated note records the selected ownership model and the exact JSON
  contract additions.
- Existing docs and tests that already mention streaming are listed as touched
  or intentionally out of scope.

Evidence:

- `docs/dev/notes/2026-05-30-p08-cdp-tab-streaming-contract-audit.md`
  records the per-daemon-session stream ownership model, the
  `cdp_screencast` `ViewStream` readiness contract, and the touched or
  deferred docs and test surfaces.

### Slice B | Service-State View Stream Records

Status: COMPLETE.

Goal: make eligible non-remote browsers advertise durable CDP screencast view
streams.

Tasks:

- Add a helper that derives `cdp_screencast` `ViewStream` records for local
  and attached CDP browsers with reachable CDP endpoints.
- Keep remote-headed RDP/Guacamole view stream generation unchanged.
- Add readiness reasons for unavailable stream states.
- Ensure reconciliation updates or removes stale CDP stream records when a
  browser becomes `process_exited`, `cdp_disconnected`, or `unreachable`.

Exit criteria:

- `agent-browser service browsers --json`, HTTP service browser resources, and
  MCP browser resources expose CDP stream records for eligible non-remote
  browsers.
- Unavailable browsers expose useful readiness rather than openable dead URLs.

Evidence:

- Launch-derived service browser persistence now derives a `cdp_screencast`
  stream for `local_headless`, `local_headed`, and `attached_existing`
  browsers when launch metadata is present, with unavailable readiness for
  missing stream server, missing CDP endpoint, or non-ready browser health.
  Remote-headed view stream generation remains unchanged.
- `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`
  passed and covers ready local CDP streams, unavailable streams without a
  stream server, and unchanged remote-headed behavior.
- `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`
  passed and covers stale browser health invalidating the CDP stream URL and
  control input instead of leaving an openable dead stream.

### Slice C | Stream Server Routing And Tab Focus

Status: COMPLETE.

Goal: bind service view-stream records to the existing runtime stream server
and the selected tab target.

Tasks:

- Add or reuse service request actions to focus a browser tab and update the
  active CDP page session id used by `StreamServer`.
- Ensure target changes restart screencast cleanly and keep cached last-frame
  state scoped to the current browser or tab.
- Keep stream start/stop viewer-driven.
- Preserve explicit `stream enable --port` and runtime `.stream` metadata
  semantics.

Exit criteria:

- A dashboard or HTTP client can request tab focus, then connect to the
  stream and receive frames for that tab.
- Switching tabs does not leak stale frames as the current stream.

Evidence:

- The post-command stream sync now includes `view_focus`, so service-queued
  tab focus updates the active CDP page session id and notifies the stream
  loop to restart against the selected target.
- `StreamServer::set_cdp_session_id` clears cached `last_frame` only when the
  CDP session target changes, preventing stale replay when the stream moves
  between tabs while preserving the cache for same-target refreshes.
- `cargo test --manifest-path cli/Cargo.toml set_cdp_session_id -- --nocapture`
  passed for target-change cache clearing and same-target cache preservation.
- `pnpm test:service-cdp-tab-streaming-live` passed by launching a service
  local headless browser, opening a CDP stream WebSocket, receiving a frame,
  focusing page B through `view_focus`, then refocusing page A and verifying
  distinct frames after each tab switch.

### Slice D | Dashboard Integration

Status: COMPLETE.

Goal: make the Service dashboard open and control CDP screencast streams for
non-remote browsers.

Tasks:

- Teach `canEmbedViewStream`, labels, readiness labels, and open titles about
  `cdp_screencast` loopback/dashboard URLs.
- Add browser-row and tab-detail flows that focus the target tab before
  opening the stream.
- Reuse existing view-stream inspect dialog where possible instead of adding a
  separate viewer.
- Show unavailable readiness in compact form when CDP streaming cannot be
  opened.

Exit criteria:

- Browser rows and tab details can open an eligible non-remote CDP stream.
- Disabled buttons explain why streaming is unavailable.

Evidence:

- Dashboard view-stream helpers now treat `cdp_screencast` as embeddable when
  the service provides a loopback URL and keep `cdp_input` streams
  controllable. Unavailable CDP streams surface `readiness.reason` in open
  titles instead of a generic missing-URL explanation.
- Existing browser-row and tab-detail control paths already queue
  `view_focus` before opening a controllable view stream. With
  `cdp_screencast` now embeddable and controllable, those paths apply to
  local CDP streams as well as remote-view streams.
- `pnpm test:dashboard-view-streams` and `pnpm build:dashboard` passed.

### Slice E | CLI, HTTP, MCP, Client, And Docs Alignment

Status: COMPLETE.

Goal: keep every user-facing and software-facing surface aligned with the new
CDP stream contract.

Tasks:

- Update CLI help, README, `skills/agent-browser/SKILL.md`, and
  `docs/src/app/streaming/page.mdx`.
- Update service contract metadata, generated client types/helpers, and MCP
  resource docs if the JSON shape changes.
- Add examples that distinguish local CDP tab streaming from
  Guacamole/RDP remote view.

Exit criteria:

- Agents and software clients can discover the supported CDP stream workflow
  without relying on internal source knowledge.

Evidence:

- Updated CLI stream help, README, `skills/agent-browser/SKILL.md`, and
  `docs/src/app/streaming/page.mdx` for the initial service-owned
  `cdp_screencast` stream contract. No schema change was needed because the
  implementation uses existing `ViewStream` URL, input, and readiness fields.
- The installed `agent-browser` skill matches the repo skill copy:
  `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
  passed.
- `pnpm --dir docs build` passed.

### Slice F | Validation Gates

Status: COMPLETE.

Goal: prove the feature without depending on remote desktop infrastructure.

Tasks:

- Add no-launch unit/contract tests for stream record derivation, readiness,
  tab focus request shape, and dashboard labels.
- Add a local live smoke that launches a service-owned non-remote browser,
  opens a stream WebSocket, receives at least one frame, switches tabs, and
  verifies frames follow the selected tab.
- Keep the existing remote-view live gates separate.

Exit criteria:

- Fast CI covers contract shape without launching Chrome.
- The local live smoke proves end-to-end CDP tab streaming for non-remote
  browsers.
- Validation selector recommends the right checks for future stream changes.

Evidence:

- Added focused Rust tests for CDP stream record derivation and stale record
  invalidation, plus dashboard view-stream helper coverage for embeddable CDP
  streams and unavailable readiness copy.
- Added `scripts/smoke-service-cdp-tab-streaming-live.js` and
  `pnpm test:service-cdp-tab-streaming-live` for local end-to-end validation.
- Updated `scripts/dev/select-validation.js` so future CDP tab streaming
  surface changes recommend the live smoke and focused Rust stream tests.
- Selector-recommended validation has passed:
  `cargo fmt --manifest-path cli/Cargo.toml -- --check`,
  `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`,
  `cargo test --manifest-path cli/Cargo.toml set_cdp_session_id -- --nocapture`,
  `cargo test --manifest-path cli/Cargo.toml cdp_screencast_view_stream -- --nocapture`,
  `cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1`,
  `pnpm test:service-cdp-tab-streaming-live`,
  `pnpm test:dashboard-view-streams`, `pnpm --dir docs build`,
  `pnpm build:dashboard`, `git diff --check`,
  `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`,
  and `node scripts/dev/select-validation.js --base HEAD --json`.

## Initial Validation Plan

- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- Focused Rust tests for stream/service model/actions touched by each slice.
- Dashboard tests covering view-stream labels, buttons, and tab-detail
  controls.
- Service client and MCP contract tests if service schema changes.
- `pnpm --dir docs build` for docs changes.
- A new local live smoke for non-remote CDP tab streaming.
