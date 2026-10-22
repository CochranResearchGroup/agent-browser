---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Token-protected native control uses `AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE`, an absolute private owner-only regular file. Read [Remote View setup](references/cli-and-service.md) before configuring native control.

## Handoff creator context

Supply `serviceName`, `agentName` and concrete `taskName` on `browser_open`. CLI flags: `--service-name`, `--agent-name`, `--task-name`; `remote-view open` also requires `--origin-kind agent`. Blank labels fail before creation. MCP sets origin `agent`. Service requests set originKind in params. Principal identity does not replace creator labels. Do not infer `humanName` from the requester; labels are not authenticated identities.

Polling or Share preserves creation context. Cards fall back to profile/session context and omit missing identity fields, rather than inventing an owner. Job IDs do not replace creator labels.

## MCP task workflow

Use `browser_open` with `{ "profile": "editorial", "profileMode": "persistent", "taskKey": "editing", "url": "https://example.com" }`. It creates or reuses the profile and returns `data.taskId`, `profileId`, `browserId`, `tabId`, and `url` in the MCP text payload. Pass `taskId` to subsequent page actions. Replay the same `taskKey` to return its current page and preserve edits without navigating again; use distinct keys for independent tasks. End only your task with `browser_close({ "taskId": "<returned-task-id>" })`.

Use `profileMode: "existing"` to require an existing profile. Use `profileMode: "disposable"` without `profile` for temporary work. Profile creation, session selection, replay, and scoped closure happen inside the workflow operation.

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })`. Require `operatorVisible.state=ready`, then send its complete `data.handoffUrl`. Continue with the same task ID. On `remote_view_public_origin_missing`, distinguish missing configuration from a stale MCP connection. Set `AGENT_BROWSER_PUBLIC_ORIGIN` in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; explicit process values take precedence. Reconnect after startup configuration changes and reuse the same task. Callers do not assemble provider URLs.

The handoff URL resolves the current owned desktop and redirects to native Remote View authentication without a Dashboard login. The public locator remains read-only. A valid link whose target expired continues through native authentication to browser-owner recovery when the private gateway callback is configured. Keep the same opaque URL; missing callback configuration reports unavailability. Human use requires native display confirmation.

When the person finishes, call `browser_continue({ "taskId": "<same-task-id>" })` to read the current task identity and URL, then continue page actions. This preserves the current page and edits; it is separate from CLI executable-transfer handoff.

[Host placement, routing and metadata](references/host-integration.md).

The default MCP catalog contains 12 workflow tools: `browser_open`, `browser_navigate`,
`browser_snapshot`, `browser_click`, `browser_fill`, `browser_screenshot`, `browser_tab`,
`browser_handoff`, `browser_continue`, `browser_close`, `browser_command` and `browser_discover`.
Page actions require `taskId`. Use `browser_tab` to list, create, select or close this
task's tabs; repeat a stable `tabKey` when retrying creation. Creating a tab after
an owner restart reconnects to the verified live browser and preserves existing
tabs. Peer tabs stay outside this scope. Use `browser_discover` for one advanced operation's argument schema,
then `browser_command` with `taskId`, `action` and page arguments in `params`.
Close rejects a changed selection.

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

Selection and Share wait their turn on the same native desktop.

Diagnostic and administrative discovery points to supported CLI/API operations.
Retired lease, recovery, repair and reset MCP tools are unavailable, including
through advanced dispatch. There is no legacy catalog mode.

Return, store, and reopen only `handoffUrl` at `/remote-view/<handoff-id>`. Never send providerExternalUrl.

## Operator workspace

Read [Operator workspace](references/cli-and-service.md#operator-workspace) before dashboard tab selection, sharing, handoff-policy editing or investigating rail freshness. Remote View owns the viewer and its existing authentication.

For non-CDP human sign-in, use [manual login](references/cli-and-service.md#manual-login) with `browser_open` mode `manual_login`.

## Sharing a profile between tasks

Use a separate task/session and tab for each workflow. Shared profiles share login and persistent site state. Read [Sharing a profile between tasks](references/cli-and-service.md#sharing-a-profile-between-tasks) before coordinating shared-profile work, foreground input or closing a peer task.

## CLI fallback

If `browser_open` is absent from the client's callable tools, report missing
Agent Browser MCP integration. Register the installed `agent-browser mcp serve`
stdio command in that client and reconnect to refresh its cached catalog. Use
the installed CLI to continue the task while integration is corrected; server
`tools/list` alone does not prove client exposure.

Command help, including `service --help` and `service -h`, runs locally without browser launch or Service State writes. Unknown command help returns a typed error. `runtime create --help` describes profiles; creation returns `created` and `userDataDir`. MCP payloads use `success`, `data`, `error`; CLI failures add `type`.

Inspect `agent-browser.runtime-health.v1` for proactive runtime drift and
bounded recourse. Read the advanced reference's runtime custody diagnostics
before maintenance, retirement reconciliation or interpreting unknown effects.
Compact logs; scrub exported URLs.


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

Retained-task recovery rechecks interrupted launch custody in the running host.
It requires proven process and profile absence, preserves launch history, and
leaves other profiles' claims unchanged. Continue through the original taskId.

New-process memory pressure uses eligible owner cleanup and a bounded wait;
continue existing tasks through their taskId. Completed session/tab history is
bounded in active memory and retained durably. Use `agent-browser runtime history
sessions 100 --json` or `runtime history tabs 100 --json` for bounded archive pages;
pass `data.nextCursor` as the next positional cursor. Handoff target evidence
remains available. Read the advanced reference for host resource settings.

- [Commands](references/commands.md): detailed CLI syntax.
- [Snapshot references](references/snapshot-refs.md): find and use element refs.
- [Authentication](references/authentication.md): requested login and state reuse.
- [Session management](references/session-management.md): named CLI sessions.
- [Advanced CLI and service reference](references/cli-and-service.md): flags,
  installation, API contracts, diagnostics and explicit maintenance.

Use `browser_discover` for advanced MCP metadata.


### Dashboard summary diagnostics

For subsystem health, workspace inventory or embedded-viewer failures, read [Dashboard summary diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics). A prepared handoff or reachable service alone does not prove a connected Native display. For a failed embedded page, read [failure guidance](references/cli-and-service.md#embedded-native-desktop-failure-guidance) before retrying.
