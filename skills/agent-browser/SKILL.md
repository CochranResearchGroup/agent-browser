---
name: agent-browser
description: Browser task workflows through the Agent Browser MCP, with CLI fallback for AI agents. Use when the user needs to interact with websites, including navigating pages, filling forms, clicking buttons, taking screenshots, extracting data, testing web apps, or automating any browser task. Triggers include requests to "open a website", "fill out a form", "click a button", "take a screenshot", "scrape data from a page", "test this web app", "login to a site", "automate browser actions", or any task requiring programmatic web interaction.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

## MCP task workflow

Use `browser_open` with `{ "profile": "editorial", "profileMode": "persistent", "taskKey": "editing", "url": "https://example.com" }`. It creates or reuses the profile and returns `data.taskId`, `profileId`, `browserId`, `tabId`, and `url` in the MCP text payload. Pass `taskId` to subsequent page actions. Replay the same `taskKey` to return its current page and preserve edits without navigating again; use distinct keys for independent tasks. End only your task with `browser_close({ "taskId": "<returned-task-id>" })`.

Use `profileMode: "existing"` to require an existing profile. Use `profileMode: "disposable"` without `profile` for temporary work. Profile creation, session selection, replay, and scoped closure happen inside the workflow operation.

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })` and give the person its complete `data.handoffUrl`. Require `data.operatorVisible.state: "ready"`. Continue page actions with the same task ID afterward. If `remote_view_public_origin_missing` is returned, report the server configuration guidance; the operator configures `AGENT_BROWSER_PUBLIC_ORIGIN` for the Agent Browser service. Task callers do not assemble provider URLs.

The handoff URL resolves the current owned desktop and redirects to native Remote View authentication without a Dashboard login. The public locator remains read-only. A valid link whose target expired continues through native authentication to browser-owner recovery when the private gateway callback is configured. Keep the same opaque URL; missing callback configuration reports unavailability. Readiness alone does not record human use; the owner requires native display confirmation. Keep using the same opaque URL while the owner maintains pending handoff context.

When the person finishes, call `browser_continue({ "taskId": "<same-task-id>" })` to read the current task identity and URL, then continue page actions. This preserves the current page and edits; it is separate from CLI executable-transfer handoff.

The default MCP catalog contains 12 workflow tools: `browser_open`, `browser_navigate`,
`browser_snapshot`, `browser_click`, `browser_fill`, `browser_screenshot`, `browser_tab`,
`browser_handoff`, `browser_continue`, `browser_close`, `browser_command` and `browser_discover`.
Page actions require `taskId`. Use `browser_tab` to list, create, select or close this
task's tabs; repeat a stable `tabKey` when retrying creation. Peer tabs stay outside
this scope. Use `browser_discover` for one advanced operation's argument schema,
then `browser_command` with `taskId`, `action` and page arguments in `params`.
Diagnostic and administrative discovery points to supported CLI/API operations.
Retired lease, recovery, repair and reset MCP tools are unavailable, including
through advanced dispatch. There is no legacy catalog mode.


Returned durable links use `/remote-view/<handoff-id>`. Require `operatorVisible.state=ready`.
Return, store, and reopen only `handoffUrl`, including after viewer reconnection.
Never hand off a raw Guacamole URL or `providerExternalUrl` to the operator.

## Sharing a profile between tasks

Use one named session and tab per task. When tasks deliberately need the same login and stored browser data, select the same durable profile so they reuse one browser. Give each task its own session name; address commands through that session rather than a global active tab.

Tab-targeted automation can run without either tab being physically visible. Serialize foreground desktop clicks, keyboard input and human control. Separate tabs share cookies, login/logout state and persistent site storage: coordinate account switches, logout and conflicting application changes. Choose separate profiles when independent state is required.

Close only the task's session. Closing one session must preserve its peers; the final session releases the browser. Do not delete the shared profile to clean up a task. If a site requires foreground execution, serialize that part of the workflow.

## Product purpose

Use Agent Browser when reusable browser operation, task-appropriate profiles,
continuity, native Remote View human handoff and scoped cleanup make the task
easier to complete. Existing client profiles, managed persistent profiles and
disposable profiles are all valid choices. Preserve existing authentication
when requested. Human assistance uses a direct native desktop link through
Remote View authentication; the person and agent continue in the same browser.
Keep returned context concise and diagnostics actionable. A simpler scripting
workflow or specialized CDP code may already meet the task.

In the compact workspace preview, a selected tab missing from current inventory
removes its stale viewer and offers Refresh browsers. The pending refresh is
shown explicitly. The requested selection and peer tabs remain unchanged;
refresh does not automatically choose another tab.
Dashboard/profile permission rejections offer access-check guidance before retrying.
In the compact workspace preview, open a tab’s SVG overflow menu or use its
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


## CLI fallback

If `browser_open` is absent from the client's callable tools, report missing
Agent Browser MCP integration. Register the installed `agent-browser mcp serve`
stdio command in that client and reconnect to refresh its cached catalog. Use
the installed CLI to continue the task while integration is corrected; server
`tools/list` alone does not prove client exposure.

Unknown command help fails with a concise typed error. Use `agent-browser runtime create --help` for focused profile creation guidance. Runtime creation returns a top-level record (`created`, `userDataDir`); workflow MCP text payloads use `success`, `data`, and `error`; JSON CLI failures use `success: false`, `error`, and `type`.

Browser Runtime reads SQLite schemas 1 through 4. The first native idle-retirement
custody claim promotes schema 1 to schema 2 in the same transaction as its record.
Command-activity reservations require at least schema 3. Original-process
activity evidence and ended-resource settlement require schema 4. Each write
atomically advances the database to its required reader version without downgrading it. Keep a schema-4-capable binary with that database and its
backups; older readers reject it. Activity reservations prevent automatic retirement
through command execution and session publication. Unknown effects retain their
veto and original readback obligation; acknowledgement alone does not release it.
Positive original-process exit permits resource recovery while preserving the
unknown command record. Unavailable observation retains the veto; recovery does
not replay the original command or adopt a replacement process as its evidence.

The automatic-close candidate persists its exact operation before preparing
native viewer exclusion, records each close attempt before the physical effect,
and publishes session state with retirement custody before completing the fence.
A lost completion reply is reconciled against the original fence after restart.
Unresolved physical close outcomes retain custody; native workflow and installed
acceptance remain pending.

Pending retirement custody also fences changes to its browser, tab and session
projection inside SQLite publication and launch/recovery admission transactions.
Unrelated profiles can continue. `browser_session_publication_idle_retirement_fenced`
means that publication would overwrite an unresolved original operation; it does
not authorize another close or replacement launch.

A physical close requires a fresh atomic attempt admission. A stale coordinator
reads the original pending attempt instead of treating an idempotent record
update as permission to close again. When one command expires old work and
reopens its profile, the host publishes and completes the old retirement before
reusing its logical browser identity.

Preparation attempts also require fresh atomic admission. After interruption,
the owner reads the exact original fence. A known prepared operation with no
admitted close can publish unchanged task state before completing that fence;
a delayed actor then cannot admit its old effect. An absent fence after an
unknown prepare remains unresolved.

Before delegating a browser command, the candidate checks durable profile custody.
An unresolved retirement returns `browser_session_idle_readback_required` before
tab acquisition, creation, navigation, focus or command execution. Process-liveness
observation remains available. Atomic admission against a retirement starting
concurrently with a new command remains pending source qualification.

Normal close confirmation requires positive OS process absence. An unavailable
process observation returns `browser_session_process_observation_unavailable`
and leaves the outcome unresolved, including after a lost close reply. This
absence check alone does not prove original process identity across PID reuse.

The launch-custody candidate retains kernel start identity captured with the
original launched child or verified profile attachment. Restart reads the stored
identity; a reused PID or missing-identity replay cannot replace it. Older records
remain explicitly without that evidence. Recording the first kernel identity
promotes the private database reader version atomically. A new retirement scope
freezes the published launch identity, and SQLite validates that exact join in
its claim transaction. Restart preserves the frozen evidence. Admitted automatic
effects and readback use that scope. A changed kernel start proves the original
ended without attaching or closing its replacement. Live-target readback queries
only the verified original browser and frozen target. New automatic groups with
missing legacy evidence defer before any new claim, native preparation or attempt.
Existing unresolved operations retain original readback obligations; a scoped
runtime request without identity returns `browser_session_idle_original_identity_missing`
and cannot adopt the current PID. Native workflow and installed qualification
remain pending.

For controller takeover, `desktop_input_route_fence_contended` reports
`file_lock_wait` and `no_effect` before authority mutation. Inspect Service
status and trace, then resolve the exact contention before any new attempt.
`inspect_before_retry` does not authorize automatic replay. Other actions may
have earlier effects and retain conservative failure recourse.

Authorized native Service lane title and URL reads observe the exact target without activating its
native tab or raising the browser. Use explicit workspace focus or tab selection
when the person should see that target. Child ownership and permissions still
apply to background reads.
Managed tasks sharing one browser also preserve the visible peer tab during
title and URL reads. Use explicit focus to select the task's tab for a person.
Retained tab-creation replay after managed task closure returns
`browser_session_tab_request_target_closed`. Treat this as a closed task; the
replay does not authorize a replacement browser.
On retained session browser routes, authenticated operator focus shares a native
selection queue for each declared desktop within one host, including requests
from separate clients or runtime lanes. The host rechecks the operator proof after waiting. Different desktop
allocations remain independent. Waits use the request timeout, with a ten-second
default; inspect pending focus work before a new explicit selection if waiting
expires.
Service Status `browserRuntime.workspace` reports current managed browsers,
sessions and tabs with supplied task metadata. `reported` is owner inventory,
not native viewer readiness or current process liveness. Debugger endpoints and
private launch environment are omitted. The dashboard keeps its own login,
session checks and logout.
Native workspace embedding requires the same public origin as Remote View.
Iframe handoffs use its existing workspace mode and same-origin framing policy;
ordinary durable links retain the standalone native viewer and existing auth.
Preparing a managed handoff with an exact existing target and no URL preserves
the live document instead of navigating or relaunching it.

Managed view_focus marked as an operator request also requires current authenticated
operator authority. An unbound request is rejected before selecting its target;
ordinary explicit managed focus keeps its existing behavior.

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
The CLI remains available for independent diagnostics and supported maintenance.

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

The advanced reference is not a second MCP catalog or a caller authority
workflow. `browser_discover` is the current source for advanced MCP metadata.
