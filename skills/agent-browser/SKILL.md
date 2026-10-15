---
name: agent-browser
description: Browser task workflows through the Agent Browser MCP, with CLI fallback for AI agents. Use when the user needs to interact with websites, including navigating pages, filling forms, clicking buttons, taking screenshots, extracting data, testing web apps, or automating any browser task. Triggers include requests to "open a website", "fill out a form", "click a button", "take a screenshot", "scrape data from a page", "test this web app", "login to a site", "automate browser actions", or any task requiring programmatic web interaction.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

## MCP task workflow

Use `browser_open` with `{ "profile": "editorial", "profileMode": "persistent", "taskKey": "editing", "url": "https://example.com" }`. It creates or reuses the profile and returns `data.taskId`, `profileId`, `browserId`, `tabId`, and `url` in the MCP text payload. Pass `taskId` to subsequent page actions. Replay the same `taskKey` to return its current page and preserve edits without navigating again; use distinct keys for independent tasks. End only your task with `browser_close({ "taskId": "<returned-task-id>" })`.

Use `profileMode: "existing"` to require an existing profile. Use `profileMode: "disposable"` without `profile` for temporary work. Profile creation, session selection, replay, and scoped closure happen inside the workflow operation.

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })` and give the person its complete `data.handoffUrl`. Require `operatorVisible.state=ready`. Continue page actions with the same task ID afterward. If `remote_view_public_origin_missing` is returned, report the server configuration guidance; the operator configures `AGENT_BROWSER_PUBLIC_ORIGIN` for the Agent Browser service. Task callers do not assemble provider URLs.

The handoff URL resolves the current owned desktop and redirects to native Remote View authentication without a Dashboard login. The public locator remains read-only. A valid link whose target expired continues through native authentication to browser-owner recovery when the private gateway callback is configured. Keep the same opaque URL; missing callback configuration reports unavailability. Readiness alone does not record human use; the owner requires native display confirmation. Keep using the same opaque URL while the owner maintains pending handoff context.

When the person finishes, call `browser_continue({ "taskId": "<same-task-id>" })` to read the current task identity and URL, then continue page actions. This preserves the current page and edits; it is separate from CLI executable-transfer handoff.

The default MCP catalog contains 12 workflow tools: `browser_open`, `browser_navigate`,
`browser_snapshot`, `browser_click`, `browser_fill`, `browser_screenshot`, `browser_tab`,
`browser_handoff`, `browser_continue`, `browser_close`, `browser_command` and `browser_discover`.
Page actions require `taskId`. Use `browser_tab` to list, create, select or close this
task's tabs; repeat a stable `tabKey` when retrying creation. Creating a tab after
an owner restart reconnects to the verified live browser and preserves existing
tabs. Peer tabs stay outside this scope. Use `browser_discover` for one advanced operation's argument schema,
then `browser_command` with `taskId`, `action` and page arguments in `params`.
Close rejects a changed selection.
Diagnostic and administrative discovery points to supported CLI/API operations.
Retired lease, recovery, repair and reset MCP tools are unavailable, including
through advanced dispatch. There is no legacy catalog mode.

Return, store, and reopen only `handoffUrl` at `/remote-view/<handoff-id>`. Never send providerExternalUrl.

## Operator workspace

Open the dashboard root `/` or `/browsers` for the native browser workspace.
Select the owned tab, inspect Properties or Share its durable link, and use
Handoffs for retained records and independent policies. Refresh retains the
requested identity. Diagnostics at `/service` keeps Service inspection,
Selection, Activity, Chat, Console, Network, Storage and Extensions optional.
`?view=service:<name>` and the selected workspace/browser/session/tab/profile/job
query fields remain supported for diagnostic links. Native Remote View uses its
existing authentication; dashboard logout remains separate.

## Sharing a profile between tasks

Use one named session and tab per task. When tasks deliberately need the same login and stored browser data, select the same durable profile so they reuse one browser. Give each task its own session name; address commands through that session rather than a global active tab.

Tab-targeted automation can run without either tab being physically visible. Serialize foreground desktop clicks, keyboard input and human control. Separate tabs share cookies, login/logout state and persistent site storage: coordinate account switches, logout and conflicting application changes. Choose separate profiles when independent state is required.

Close only the task's session. Closing one session must preserve its peers; the final session releases the browser. Do not delete the shared profile to clean up a task. If a site requires foreground execution, serialize that part of the workflow.

Disposable quota retirement remains under development. Its admission includes verified child processes of the selected browser; unrelated or unverified processes using the profile still defer retirement. Retain pending-human, viewer, controller, operation and peer protections.

In the default browser workspace, a selected tab missing from current inventory
removes its stale viewer and offers Refresh browsers. The pending refresh is
shown explicitly. The requested selection and peer tabs remain unchanged;
refresh does not automatically choose another tab.
Dashboard/profile permission rejections offer access-check guidance before retrying.
In the default browser workspace, open a tab’s SVG overflow menu or use its
context menu (Shift+F10) for Properties and Copy URL. Properties shows supplied
origin and purpose alongside current browser, profile, desktop, URL, title and
status. Missing values remain Unknown; descriptive names are not identity proof.
Copy URL copies the task URL without creating a handoff or changing selection.
Close Tab confirms one managed task before requesting its exact target through the
existing browser owner. It requires authenticated dashboard access and current
profile permission to close any tab. Cancelling sends no request. A confirmed
close preserves peer tabs; closing the final owned tab also closes its browser.
If close cannot be confirmed, refresh inventory and inspect permission before
trying again.

Share prepares the exact existing task through the public handoff helper and
shows its opaque native link in Handoffs. It preserves the selected peer and
does not navigate the task. Use the small Handoffs rail control to reopen the
panel, then copy or open the same link through existing Remote View authentication.

Handoffs lists retained owner records even after target expiry, with supplied
origin, creation/use status and separate link/tab policies. Inspect shows task
context; Copy and Open reuse the opaque link. A failed refresh retains the last
reported records. Unavailable inventory is distinct from an empty list.
Edit retention or delete links in Handoffs.

## CLI fallback

If `browser_open` is absent from the client's callable tools, report missing
Agent Browser MCP integration. Register the installed `agent-browser mcp serve`
stdio command in that client and reconnect to refresh its cached catalog. Use
the installed CLI to continue the task while integration is corrected; server
`tools/list` alone does not prove client exposure.

Use `agent-browser runtime create --help` for profile guidance. Runtime creation returns `created` and `userDataDir`; MCP payloads use `success`, `data`, `error`; CLI failures use `success: false`, `error`, `type`.

Inspect `agent-browser.runtime-health.v1` for proactive runtime drift and
bounded recourse. Read the advanced reference's runtime custody diagnostics
before maintenance, retirement reconciliation or interpreting unknown effects.

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

Use the workflow MCP tools for durable profile/task continuity and human handoff.

## Read more only when needed

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
