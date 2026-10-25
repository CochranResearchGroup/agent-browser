---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Read [Remote View setup](references/cli-and-service.md) for native credentials.

## Handoff creator context

Supply `serviceName`, `agentName` and concrete `taskName` on `browser_open`. CLI flags: `--service-name`, `--agent-name`, `--task-name`; `remote-view open` also requires `--origin-kind agent`. Blank labels fail before creation. MCP sets origin `agent`. Service requests set originKind in params. Principal identity does not replace creator labels. Do not infer `humanName` from the requester; labels are not authenticated identities.

Creation context is immutable. Cards omit missing identity; polling, Share and job IDs never replace creator labels.

## MCP task workflow

Warnings advise; writer custody survives executable unlink.

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Use `profileMode: "existing"` for an existing profile, or `"disposable"` without `profile` for temporary work. The workflow owns selection, replay and scoped closure.

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })`. Require `operatorVisible.state=ready` before giving the person its complete `data.handoffUrl`; continue with the same task ID. For `remote_view_public_origin_missing`, check reviewed `AGENT_BROWSER_PUBLIC_ORIGIN` configuration and stale MCP startup. Set it in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; explicit process settings win. MCP connections pin startup configuration: reconnect after changes, preserving the task. Never assemble provider URLs.

A standalone handoff resolves its current owned desktop and redirects to Native authentication without a Dashboard login. The public locator remains read-only. Native authentication precedes configured browser-owner recovery. Missing recovery configuration reports unavailability. Reuse the opaque URL; require native display confirmation.

After human input, `browser_continue` with the same `taskId` reads identity/URL and preserves edits. It is separate from CLI executable-transfer handoff.

## Hosted workflow guidance v1

Use Bots-native browsing when it already owns the right browser and task. Choose
Agent Browser for its reusable task, page and continuity controls; do not create a
second browser just to switch tools. Read [host integration v1](references/host-integration.md)
before app placement, attachment, sharing or human takeover. Discover the connected
runtime's schemas first; documentation and a visible browser do not prove support,
account identity or control ownership. Shared skill publication needs explicit approval.

The default MCP catalog contains 16 workflow tools: `browser_open`, `browser_navigate`,
`browser_snapshot`, `browser_click`, `browser_fill`, `browser_screenshot`, `browser_tab`,
`browser_handoff`, `browser_continue`, `browser_close`, `browser_control`, `browser_reconnect`, `browser_policy`,
`browser_artifact`, `browser_command` and `browser_discover`.
Page actions require `taskId`. `browser_tab` manages this task's tabs; retry creation
with the same `tabKey`. After restart, creation rejoins the verified browser
and preserves tabs; peer tabs remain outside scope. Discover actions with `browser_discover`; use `browser_command` with `taskId`, `action`, `params`.
Close rejects changed selection. A pre-commit return failure keeps human control
and epoch; inspect status and use a fresh explicit intent, never replay uncertainty.

For downloads, use `action: "download"` with the link selector and destination
path in `params`. Existing-profile tasks use the current browser; clients do not
establish ownership or repair stale internal records before downloading.
Ordinary continuation reconciles interrupted publication using the current live
profile browser, preserving the task and authenticated state.
A new ordinary task command refreshes stale clean session state before acting;
unpublished effects remain held for reconciliation without command replay.

If stale transport recommends `handoff prepare` but returns `prepared=false`
and `browserPresent=false`, there is no candidate to attach. On Linux, inspect
`agent-browser install workstation --dry-run --runtime-replacement-policy preserve --json`
and follow its plan before retrying. Keep profiles, peers and unresolved effect
custody intact; this read-only plan does not authorize a reset or host restart.

Retirement permits diagnostic reads but fences handoff writes. On lock timeout,
run `agent-browser service jobs --json`, then refresh the access plan. Reconcile
uncertain effects before retrying; never terminate the holder or reset a profile.

Diagnostic and administrative discovery points to supported CLI/API operations.
Retired lease, recovery, repair and reset MCP tools are unavailable, including
through advanced dispatch. There is no legacy catalog mode.

Return, store, and reopen only `handoffUrl`: standalone `/remote-view/<handoff-id>` or hosted app `/agent-browser/handoff/<id>`. Never send providerExternalUrl.

## Operator workspace

Read [Workspace](references/cli-and-service.md#operator-workspace) for tabs, sharing and handoffs. Focus needs an owned tab; reviews are opt-in. Rail catches up once. Remote View owns view/auth. Terminal pool replay retains history.

[Manual login](references/cli-and-service.md#manual-login) is CDP-free. Native control and human takeover are separate.

## Sharing a profile between tasks

Use a separate task/session and tab for each workflow. Shared profiles share login and persistent site state. Read [Sharing a profile between tasks](references/cli-and-service.md#sharing-a-profile-between-tasks) before coordinating shared-profile work, foreground input or closing a peer task.

## CLI fallback

If `browser_open` is absent from the client's callable tools, report missing
Agent Browser MCP integration. Register the installed `agent-browser mcp serve`
stdio command in that client and reconnect to refresh its cached catalog. Use
the installed CLI to continue the task while integration is corrected; server
`tools/list` alone does not prove client exposure.

Help never launches browsers. MCP uses `success`, `data`, `error`; CLI failures add `type`. Private launch journals retain secret-free `details.phase`; unknown outcomes need readback.

Read `agent-browser.runtime-health.v1` for drift and bounded recourse. Consult
advanced runtime custody diagnostics before maintenance, retirement or unknown
effect interpretation. Compact logs; scrub exported URLs.


Use the installed `agent-browser` executable when MCP is unavailable. Read
`agent-browser <command> --help` for the requested operation. Choose the task's
profile and named session explicitly; preserve peers and stored authentication.
Do not run setup, process cleanup or profile resets just to open a browser task.

A normal CLI sequence is:

```bash
agent-browser --runtime-profile editorial --session editing open https://example.com
agent-browser --session editing snapshot -i
agent-browser --session editing fill @e1 "Draft text"
agent-browser --session editing click @e2
```

## SiteGuide routines

Discover `category:"playbooks"` and use `browser_command` in the existing task. Before login, hosted-helper use, guide development or bounded collection, read [SiteGuide routines](references/cli-and-service.md#siteguide-routines) and [the routine guide](references/playbooks.md). Verify the intended account/company; browser login and OAuth/API readiness are separate.

## Read more only when needed

Retained-task recovery requires proven process/profile absence and preserves peer claims. Continue through the original taskId.

Read the advanced reference for bounded host resource waits and retained session/tab history; continue existing tasks through taskId.

- [Commands](references/commands.md): detailed CLI syntax.
- [Snapshot references](references/snapshot-refs.md): find and use element refs.
- [Authentication](references/authentication.md): requested login and state reuse.
- [Session management](references/session-management.md): named CLI sessions.
- [Advanced CLI and service reference](references/cli-and-service.md): flags,
  installation, API contracts, diagnostics and explicit maintenance.

Use `browser_discover` for advanced MCP metadata.


### Dashboard summary diagnostics

For health, inventory or viewer failures, read [Dashboard diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics). A prepared handoff or reachable service does not prove a connected Native display. Read [embedded-page guidance](references/cli-and-service.md#embedded-native-desktop-failure-guidance) before retrying. Static assets use gzip and hashed caching.

After upgrades, reconnect MCP clients; doctor checks `installedMcpClients`.

Startup: [diagnostics](references/cli-and-service.md#development-host-startup).

## Browser mode and builds

Use `browser_command`/`taskId` action `mode`. Discover `mode_convert`, `profile_duplicate`, `browser_build`. Build family/version and CDP mode are independent. Inspect before choosing; copy success never proves login portability. Read [controls and recovery](references/cli-and-service.md#browser-build-selection) before changing a build. Reconcile original operations after lost replies.

Diagnostics retain selection query fields. Rail presence revalidates at 250/500/1000 ms, then polls every three seconds.
