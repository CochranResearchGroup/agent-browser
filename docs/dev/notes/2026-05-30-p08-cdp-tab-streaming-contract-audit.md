# P08 CDP Tab Streaming Contract Audit

Date: 2026-05-30
Plan: `docs/dev/plans/0008-2026-05-30-cdp-tab-streaming-for-non-remote-browsers-plan.md`
Slice: A, Contract And Ownership Audit

## Selected Ownership Model

Use the existing per-daemon-session `StreamServer` as the owner of CDP tab streaming. The daemon already starts a loopback stream server, writes the `.stream` port file, holds a CDP client slot, tracks connected WebSocket clients, starts `Page.startScreencast` only after a viewer connects, and stops screencast when viewers leave or the client, viewport, or CDP page session changes.

Service state should therefore advertise a `cdp_screencast` stream for each eligible non-remote service browser that is backed by a live daemon session stream server. It should not create a new route-pool object or remote-view display allocation for CDP streaming. Browser rows and tab details can focus the active tab with the existing queued `view_focus` request before opening the stream.

This keeps the Guacamole and RDP route model unchanged. Remote-headed browsers continue to use their retained `remote-headed-view` records and route checkout or takeover flow. Non-remote CDP streaming is a loopback, daemon-owned stream record rather than a remote-display route.

## JSON Contract

No new top-level service browser fields are required for the first implementation slice. The existing `BrowserProcess.viewStreams[]` and `ViewStream.readiness` fields are sufficient.

Eligible non-remote CDP browser record:

```json
{
  "id": "cdp-screencast",
  "provider": "cdp_screencast",
  "controlInput": "cdp_input",
  "url": "http://127.0.0.1:<stream-port>/",
  "frameUrl": "http://127.0.0.1:<stream-port>/",
  "externalUrl": "http://127.0.0.1:<stream-port>/",
  "readOnly": false,
  "readiness": {
    "state": "ready",
    "reason": "stream_server_ready",
    "sessionName": "<daemon-session>",
    "browserId": "session:<daemon-session>",
    "streamPort": <stream-port>,
    "cdpEndpoint": "<browser-cdp-endpoint>"
  }
}
```

Unavailable non-remote CDP browser record:

```json
{
  "id": "cdp-screencast",
  "provider": "cdp_screencast",
  "controlInput": null,
  "url": null,
  "readOnly": true,
  "readiness": {
    "state": "unavailable",
    "reason": "<reason>",
    "sessionName": "<daemon-session>",
    "browserId": "session:<daemon-session>"
  }
}
```

Initial readiness reasons:

- `unsupported_remote_view_host`: browser host uses remote desktop or a provider path owned by another stream contract.
- `browser_not_ready`: browser health is not `ready`.
- `missing_cdp_endpoint`: no CDP endpoint is recorded.
- `missing_stream_server`: the daemon has no stream server for the session.
- `stream_server_ready`: the loopback stream server and CDP endpoint are both present.

The `state` field remains a compact string under existing `readiness`; dashboard helpers can already display it without schema changes. If a later slice adds tab-specific stream ids or stream target fields, update `docs/dev/contracts/service-browser-record.v1.schema.json`, generated client types, HTTP and MCP contract metadata, README, docs site, CLI help where relevant, and `skills/agent-browser/SKILL.md`.

## Existing Docs And Tests

Expected touched surfaces for the implementation slices:

- Rust service-state derivation in `cli/src/native/actions.rs` or a nearby service helper.
- Existing Rust contract tests around `ViewStream`, `BrowserProcess`, stream status, and launch-derived service metadata.
- HTTP and MCP service browser resources through the shared serialized service state, if the JSON shape stays within existing fields.
- Dashboard stream helpers in `packages/dashboard/src/lib/service-view-streams.ts`.
- Dashboard browser row and tab-detail flows in `packages/dashboard/src/components/service-panel.tsx`.
- User docs in `README.md`, `skills/agent-browser/SKILL.md`, and `docs/src/app/streaming/page.mdx` once the workflow is user-visible.

Intentionally out of scope for this slice:

- `chrome_tab_webrtc` and `virtual_display_webrtc`.
- Route-pool checkout, Guacamole connection ids, and RDP gateway repair logic.
- Recording, audio, video encoding, and multi-controller lease policy.

## CodeGraph Note

The repo has a `.codegraph/` directory, but CodeGraph MCP tools were not exposed in this session. This audit used direct source inspection and should be refined with CodeGraph if the tools become available during a later structural refactor.
