---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Token-protected native control uses `AGENT_BROWSER_REMOTE_VIEW_CONTROL_TOKEN_FILE`, an absolute private owner-only regular file. See Remote View setup.

## MCP task workflow

Use `browser_open` with `{ "profile": "editorial", "profileMode": "persistent", "taskKey": "editing", "url": "https://example.com" }`. It creates or reuses the profile and returns `data.taskId`, `profileId`, `browserId`, `tabId`, and `url` in the MCP text payload. Pass `taskId` to subsequent page actions. Replay the same `taskKey` to return its current page and preserve edits without navigating again; use distinct keys for independent tasks. End only your task with `browser_close({ "taskId": "<returned-task-id>" })`.

Use `profileMode: "existing"` to require an existing profile. Use `profileMode: "disposable"` without `profile` for temporary work. Profile creation, session selection, replay, and scoped closure happen inside the workflow operation.

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })` and give the person its complete `data.handoffUrl`. Require `operatorVisible.state=ready`. Continue page actions with the same task ID afterward. If `remote_view_public_origin_missing` is returned, report the server configuration guidance; the operator configures `AGENT_BROWSER_PUBLIC_ORIGIN` for the Agent Browser service. Task callers do not assemble provider URLs.

The handoff URL resolves the current owned desktop and redirects to native Remote View authentication without a Dashboard login. The public locator remains read-only. A valid link whose target expired continues through native authentication to browser-owner recovery when the private gateway callback is configured. Keep the same opaque URL; missing callback configuration reports unavailability. Human use requires native display confirmation. Reuse the opaque URL.

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

For downloads, use `action: "download"` with the link selector and destination
path in `params`. Existing-profile tasks use the current browser; clients do not
establish ownership or repair stale internal records before downloading.
Ordinary continuation reconciles interrupted publication using the current live
profile browser, preserving the task and authenticated state.

Selection and Share wait their turn on the same native desktop.

Diagnostic and administrative discovery points to supported CLI/API operations.
Retired lease, recovery, repair and reset MCP tools are unavailable, including
through advanced dispatch. There is no legacy catalog mode.

Return, store, and reopen only `handoffUrl` at `/remote-view/<handoff-id>`. Never send providerExternalUrl.

## Operator workspace

Open the dashboard root `/` or `/browsers` for the native browser workspace.
Select the owned tab, inspect Properties or Share its durable link, and use
Handoffs for retained records and independent policies. Sort the drawer by task,
creation or link expiry. Inactive links are amber and hidden by default behind
Show inactive; target expiry does not imply link expiry. Icon controls have hover
help. Managed ownership alone does not prove a native desktop is available.
The rail requires fresh matching native-window, browser-process and tab evidence
for managed desktop tasks. Empty desktops and unverified/ended tabs stay out of
live navigation; retained handoffs remain available. The rail polls every three
seconds after completion; diagnostics do not block it and stale evidence expires
independently after 30 seconds. Native window reads refresh in one background
worker; initial reads can remain unverified until completion. Cached results retain
their elapsed age, and a failed refresh cannot establish presence or renew tab
lifetime. Search uses displayed labels and navigable URLs,
not embedded document payloads. Refresh retains the requested identity. Diagnostics at `/service` keeps Service inspection,
Selection, Activity, Chat, Console, Network, Storage and Extensions optional.
`?view=service:<name>` and the selected workspace/browser/session/tab/profile/job
query fields remain supported for diagnostic links. Native Remote View uses its
existing authentication; dashboard logout remains separate.
Same-link native auth selects the addressed tab and waits for owner preparation without replay.
Current handoff presentation ignores released desktop-assignment history. It still requires one active assignment for the exact desktop generation and an exact match to published browser custody; multiple active assignments or a mismatch remain unavailable.

## Sharing a profile between tasks

Use one named session and tab per task. When tasks deliberately need the same login and stored browser data, select the same durable profile so they reuse one browser. Give each task its own session name; address commands through that session rather than a global active tab.

Tab-targeted automation can run without either tab being physically visible. Serialize foreground desktop clicks, keyboard input and human control. Separate tabs share cookies, login/logout state and persistent site storage: coordinate account switches, logout and conflicting application changes. Choose separate profiles when independent state is required.

Close only the task's session. Closing one session must preserve its peers; the final session releases the browser. Do not delete the shared profile to clean up a task. If a site requires foreground execution, serialize that part of the workflow.

An exact private producer `native_use_missing` response refreshes only the short native observation after current assignment readback. It preserves the durable link, addressed tab, first-use history and pending-human hold; timeouts, other errors and changed generations remain failures. A failed native handoff observation does not acknowledge human use or renew retention. Eligible TTL cleanup still runs under its existing activity and publication checks before the owner reports the observation error. A failed reaper response can therefore follow committed cleanup; read current owner state before deciding what remains.

Disposable quota retirement remains under development. Its admission includes verified child processes of the selected browser; unrelated or unverified processes using the profile still defer retirement. Retain pending-human, viewer, controller, operation and peer protections.

Stale selections remove the viewer and offer Refresh browsers without choosing a peer.
Use the tab SVG menu or Shift+F10 for Properties, Copy URL and permitted Close Tab.
Properties includes supplied origin, purpose, task/job/request and creation context;
missing values are Unknown, and names do not prove identity. Copy URL preserves selection.
Close confirms the exact managed target and preserves peers; the final tab closes its browser.
If completion is uncertain, refresh and inspect permission before retrying.
Share prepares the existing task without navigation. The Handoffs rail control opens
retained records, context, independent link/tab policies and explicit link deletion.
Copy and Open reuse the opaque link; failed refresh retains the prior records.

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

Discover `category:"playbooks"` and run reviewed routines through `browser_command`
in the existing task. Read [the routine guide](references/playbooks.md) before
login or guide development. `verify_login` requires intended account/company evidence;
QBO browser login and OAuth/API readiness are separate. The login client
`@agent-browser/client/login-playbook` requires preparation and validation, masked
password input and checkpoint persistence. Use `reviewedPlan` for reviewed changes;
use `connectHostedAuthHelper()` for installed private SDK discovery and pass its
attested `callerTaskId` separately from the browser `taskId`. Select configured
account/source metadata through the existing owner, never from a profile name.
After independent human/browser completion, `advance({...originalOptions,
reconcile:true})` reads back the same checkpoint without credential retrieval or
input replay. Current principal/native safety must release capture before company
verification; missing evidence stays held. `helper_attempt_released` is terminal
failed login after trusted untouched-input proof or independently empty protected
fields, plus clean-login/native safety. Preserve attempted/unknown effects;
readback cannot clear controls or retrieve another factor. Keep its checkpoint;
the playbook cannot replay it. A trusted native receiving port preserves an
exact current identifier match without clearing or retyping. Unknown prior input
cannot become equality success; password and factor checks remain unchanged.
Read the routine guide for exact setup.
Task tab listing discovers attributable popups after the capture barrier is ready.
For `collect_posts`, supply reviewed guide and item/page/scroll budgets. Empty output
is unproven unless `noResultsVerified`; preserve partial results and their reason.

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


### Handoff creator context

For tasks you create, supply `serviceName`, `agentName` and a concrete `taskName` on `browser_open`. For example, a Codex task can use serviceName `codex`, agentName `Codex`, and taskName `Launch QBO browser`. The CLI equivalents are `--service-name`, `--agent-name` and `--task-name`; for `remote-view open` creation, also supply `--origin-kind agent`. Missing or blank labels are rejected before task creation. MCP records origin `agent` without a caller field. For service requests, pass originKind in params; an authenticated or local principal is not a substitute for the creator labels. Do not fill `humanName` from the person requesting automation. Descriptive labels are not authenticated identities.

Creation context is retained once. Later polling or Share does not replace it. Cards fall back to profile/session context and omit missing identity fields, rather than inventing an owner. A job ID alone does not provide useful creator attribution.
