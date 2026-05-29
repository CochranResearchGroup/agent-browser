# Left Pane Workspace Navigator Slice 8

Date: 2026-05-23

## Scope

Slice 8 closed the campaign documentation, validation, and rendered UX pass for
the left-pane workspace navigator campaign.

This slice also fixed two readiness gaps found during final visual inspection:

- the guided launcher derived every browser/profile row but only rendered the
  first four rows
- a successful service launch stayed behind the open dialog and routed only to
  Jobs, even when the service response contained browser identity and the
  retained browser had an embeddable stream

The launcher now renders the full derived browser/profile set inside the
scrollable launcher list. On launch submit, a response with browser or tab
identity closes the dialog, queues `view_focus` through the workspace viewport,
and routes the operator to `view=workspace:control` or `view=workspace:view`
when retained stream metadata is embeddable. Job-only responses still fall back
to `/service?view=service:jobs`.

## Documentation

Updated the user-facing surfaces required by `AGENTS.md`:

- `README.md`
- `docs/src/app/dashboard/page.mdx`
- `docs/src/app/service-mode/page.mdx`
- `docs/src/app/commands/page.mdx`
- `cli/src/output.rs`
- `skills/agent-browser/SKILL.md`

The installed skill copy was synced to:

```text
/home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

## Rendered Inspection

Rendered inspection used `agent-browser` against a local dashboard at
`http://127.0.0.1:3408/`, backed by a local fixture service on
`http://127.0.0.1:3498`.

Screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/desktop-service-browsers-wide-pane-clean.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/desktop-launcher-unplanned.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/desktop-launcher-planned.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/desktop-launch-to-viewport-clean.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/desktop-viewport-after-refresh.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/mobile-workspaces-clean.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/mobile-viewport-clean.png`

Findings:

- Desktop `/service?view=service:browsers` shows useful browser rows in the
  first viewport. The old service-panel problem where summary chrome pushed
  rows below the fold was not present in the rendered fixture.
- The default desktop left pane is now wide enough for service and task labels
  to be useful without manual resizing.
- The launcher shows eligible, needs-action, and blocked combinations in a
  dense scrollable list. The summary counts reflect the full combination set,
  not just visible rows.
- Planning a supported row makes Launch available with access-plan, profile
  readiness, browser capability, and service-request evidence visible.
- Launch submit posted a `cdp_free_launch` request through
  `/api/service/request`, then the dashboard queued `view_focus` and opened the
  embedded `rdp_gateway` viewport.
- Refreshing the viewport route preserved `view=workspace:control`,
  `workspace`, `browser`, `session`, `tab`, `profile`, and `job`.
- Mobile Workspaces and Viewport tabs preserved the same route context and did
  not show horizontal overflow in the fixture.

The fixture captured this launch request shape:

```json
{
  "serviceName": "ResearchAgent",
  "agentName": "article-probe",
  "taskName": "download-pdf",
  "targetServiceIds": ["acs"],
  "accountIds": ["research@example.test"],
  "browserBuild": "remote_headed",
  "profileLeasePolicy": "wait",
  "action": "cdp_free_launch",
  "jobTimeoutMs": 60000,
  "url": "about:blank",
  "requiresCdpFree": true,
  "cdpAttachmentAllowed": false,
  "params": {
    "browserHost": "remote_headed",
    "viewStreamProvider": "rdp_gateway",
    "controlInputProvider": "manual_attached_desktop",
    "displayIsolation": "private_virtual_display",
    "url": "about:blank"
  }
}
```

The subsequent focus request was:

```json
{
  "action": "view_focus",
  "serviceName": "agent-browser-dashboard",
  "agentName": "operator",
  "taskName": "workspace-viewport-control",
  "params": {
    "index": 0,
    "maximize": true
  },
  "jobTimeoutMs": 5000
}
```

## Validation

Passed:

- `pnpm validation:select -- --base HEAD --json`
- `pnpm test:dashboard-workspace-nodes`
- `pnpm test:dashboard-workspace-navigator`
- `pnpm test:dashboard-launcher-eligibility`
- `pnpm test:dashboard-view-streams`
- `pnpm test:dashboard-inspector-actions`
- `pnpm test:dashboard-browser-table`
- `pnpm test:dashboard-browser-row-actions-render`
- `pnpm test:dashboard-profile-allocation`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:service-client-contract`
- `pnpm test:service-client-types`
- `pnpm --dir docs build`
- `pnpm build:dashboard`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo test --manifest-path cli/Cargo.toml service_request_command -- --test-threads=1`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

`pnpm --dir docs build` emitted the existing multiple-lockfile workspace-root
warning. `pnpm build:dashboard` emitted the existing Next export warning about
rewrites. Both builds completed successfully.

`agent-browser install doctor` was run after the final installed-binary
replacement; see the live deployment addendum.

## Live Deployment Addendum

After the fixture pass, the installed local dashboard binary was replaced and
the user-scoped dashboard service was restarted. `agent-browser install doctor`
passed with matching hashes for:

- `/home/ecochran76/.local/bin/agent-browser`
- the pnpm package binary
- `bin/agent-browser-linux-x64`

The standalone dashboard service is active on port 4848 and uses a dedicated
`dashboard-service-backend` session for proxied `/api/service/*` calls. That
backend was restarted with the operator RDP gateway environment from
`~/.agent-browser/.env`.

Live request proof through `http://127.0.0.1:4848/api/service/request`:

```json
{
  "success": true,
  "data": {
    "browserId": "session:dashboard-service-backend",
    "sessionId": "dashboard-service-backend",
    "profileId": "dashboard-live-rdp-proof",
    "runtimeProfile": "dashboard-live-rdp-proof",
    "url": "data:text/html,<title>Dashboard RDP Proof</title><h1>Dashboard RDP Proof</h1>"
  }
}
```

The retained live browser record reports:

```json
{
  "health": "ready",
  "host": "remote_headed",
  "profileId": "dashboard-live-rdp-proof",
  "displayIsolation": "private_virtual_display",
  "viewStreams": [
    {
      "provider": "rdp_gateway",
      "controlInput": "manual_attached_desktop",
      "readOnly": false
    }
  ]
}
```

Rendered live inspection used `agent-browser` against
`http://127.0.0.1:4848/`.

Additional screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/live-final-service-browsers.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/live-final-launcher-defaults.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-slice-8/live-final-workspace-guac-viewport.png`

Findings:

- The live Service Browsers page shows browser rows in the first viewport.
- The left pane is populated, dense, and URL-stable.
- The launcher defaults to `private_virtual_display`, `rdp_gateway`, and
  `manual_attached_desktop`.
- The workspace viewport route opens directly with
  `view=workspace:control`, shows the `rdp_gateway / manual_attached_desktop`
  capability badge, and queues `view_focus`.
- Local 127.0.0.1 visual inspection cannot authenticate the public Guacamole
  iframe, so the iframe content itself shows Chrome's blocked-frame page in the
  screenshot. The service stream metadata and public dashboard origin are in
  place for an authenticated browser session.

Operational note:

- During live backend replacement, an earlier `close --all` command closed
  existing agent-browser sessions. The final backend restart was targeted only
  at `dashboard-service-backend` and did not repeat that broad close.

## Live Guacamole Campaign Addendum

The final live campaign pass supersedes the earlier fixture-only viewport
evidence. The public dashboard code path now launches a selected
browser/profile combination from the left pane as a fresh daemon session rather
than routing a `tab_new` request through the already-running
`dashboard-service-backend` browser.

Root causes found during live inspection:

- A service request against `dashboard-service-backend` could only add tabs to
  that backend browser. It could not switch the browser host, runtime profile,
  or display for the selected launcher row.
- Fresh CLI launches accepted the new remote-view flags, but explicit
  `remote_headed` launches still attached to an existing managed runtime
  profile browser when one was alive. That silently reused a headless browser.
- The XRDP display `:10` rejected the launcher user's X cookie until the
  existing Guacamole desktop granted local access with
  `xhost +SI:localuser:ecochran76`.

Fixes added in this addendum:

- The left-pane launcher now calls `/api/exec` with a fresh session launch:

```json
{
  "args": [
    "--session",
    "session-2",
    "--executable-path",
    "/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/chrome-linux/chrome",
    "--runtime-profile",
    "stealthcdp-default",
    "--browser-host",
    "remote_headed",
    "--view-stream-provider",
    "rdp_gateway",
    "--control-input-provider",
    "manual_attached_desktop",
    "--display-isolation",
    "shared_display",
    "--headed",
    "open",
    "about:blank"
  ]
}
```

- The Rust daemon skips managed-runtime attach for explicit headed or
  `remote_headed` launches, so a live headless runtime-profile browser no
  longer overrides the requested display posture.
- The dashboard closes the launcher modal, selects the newly launched browser,
  and opens the embedded `rdp_gateway` workspace viewport.

Live service-state proof for the left-pane-launched workspace:

```json
{
  "id": "session:session-2",
  "health": "ready",
  "pid": 407502,
  "host": "remote_headed",
  "profileId": "stealthcdp-default",
  "displayIsolation": "shared_display",
  "displayName": ":10",
  "viewStreams": [
    {
      "controlInput": "manual_attached_desktop",
      "id": "remote-headed-view",
      "provider": "rdp_gateway",
      "readOnly": false,
      "url": "/guacamole/#/client/MQBjAHBvc3RncmVzcWw="
    }
  ],
  "lastError": null,
  "activeSessionIds": ["session-2"]
}
```

The launched Chrome process command line does not include `--headless=new`.
The embedded Guacamole viewport visibly shows the remote Chrome window and its
address bar inside the dashboard workspace control pane.

Additional live screenshots:

- `/tmp/agent-browser-dashboard-workspace-navigator-campaign-live/guac-xhost-applied-final.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-campaign-live/dashboard-viewport-after-session-2-launch.png`
- `/tmp/agent-browser-dashboard-workspace-navigator-campaign-live/dashboard-guac-live-final.png`

Additional validation after the live Guacamole fix:

- `cargo test --manifest-path cli/Cargo.toml test_managed_runtime_attach_is_only_for_compatible_headless_launches`
- `cargo test --manifest-path cli/Cargo.toml test_remote_view_launch_flags`
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `cargo build --release --manifest-path cli/Cargo.toml`
- `node scripts/copy-native.js`
- `agent-browser install doctor`
- `diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md`
- `git diff --check`

Operational caveat:

- The `xhost` grant was applied inside the currently running
  `agent-browser-rdp` XRDP session. If that XRDP session is restarted, the
  same display access needs to be granted again or the RDP autologin setup
  script should install the grant into the session startup path.

## Campaign Deliverable Status

Complete in this campaign:

- workspace navigator replacement with Active, Attention, and Retained groups
- derived `WorkspaceNode` model from service-owned state plus daemon sessions
- URL-persisted route and selected workspace identity
- guided browser/profile launcher through the service request queue
- launcher eligibility from access-plan, readiness, capability, allocation, and
  service-request contract evidence
- visible disabled reasons for blocked browser/profile combinations
- launch-to-viewport routing when service response identity and retained stream
  metadata allow it
- embedded dashboard viewport for `rdp_gateway` and other embeddable providers
  with fullscreen and external-open fallback
- live left-pane launch of the Stealth CDP Chromium/default profile into an
  embedded Guacamole viewport, using a fresh `remote_headed` session on display
  `:10`
- human takeover visibility with queue, lease, owner, conflict, and disabled
  Resume affordance, while preserving Control or View as the primary operator
  action when the blocked browser still has a controllable stream
- README, docs site, CLI help, installed skill, validation selector, and
  focused tests updated

Residual risks:

- Human takeover release and true Resume remain blocked on a backend
  service-owned action. The dashboard intentionally keeps Resume disabled.
  Blocked or takeover rows with controllable stream metadata remain remotely
  controllable; blocked means automation or lease progress is blocked, not that
  operator viewing or control is unavailable.
- Live Guacamole validation passed for the current workstation session. A
  durable XRDP startup grant should be added before relying on the same display
  after an XRDP session restart.
- CDP-free launched browsers remain lifecycle-only until non-CDP control
  primitives exist. The dashboard surfaces the viewport when stream metadata is
  available, but automation follow-up still depends on backend capability.
