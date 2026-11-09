---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Workspace access requires the shared Authelia guard; native dashboard password login and session cookies are disabled pending compatible Remote View authentication. Authelia admission grants operator access without local dashboard account enrollment or roles; attribution uses the exact verified owner principal.

Control and credentials: [setup and reconcile](references/cli-and-service.md).

Build failures: [readiness guidance](references/cli-and-service.md#build-readiness).

## Handoff creator context

Creation requires `serviceName`, `agentName`, concrete `taskName`. CLI: `--service-name`, `--agent-name`, `--task-name`; `remote-view open` requires `--origin-kind agent`. MCP sets agent origin; service requests set params.originKind. Labels are immutable attribution, not authenticated identity. Never infer `humanName` or replace labels with polling/job IDs.

## MCP task workflow

Warnings advise; writer custody survives executable unlink.

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Existing profiles: `profileMode:"existing"`. Temporary: `profileMode:"disposable"` without `profile`. The workflow owns selection, replay and scoped closure.

Installed viewers: [Native journey qualification](references/cli-and-service.md#native-journey-qualification).

Use `browser_handoff({ "taskId": "<task-id>" })` for human assistance. It defaults to interactive input and pauses cooperative automation. Use `interaction:"view"` for passive viewing. The authorized recipient needs no administrator override; visibility alone does not prove input readiness. Require `operatorVisible.state=ready`; return the complete `data.handoffUrl` and keep the task ID. For `remote_view_public_origin_missing`, check reviewed `AGENT_BROWSER_PUBLIC_ORIGIN` in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; process settings win. Reconnect MCP after configuration changes because its environment is pinned at startup. Preserve the task. Never assemble provider URLs.

Same-browser and desktop rail tab switches retain the viewer connection.
A retained handoff may finish its original settled browser-close publication before recovery; keep the same task and handoff, and preserve unresolved effect or identity failures for readback. An interactive invitation survives physical replacement only through its recorded exact recovery mapping; missing legacy mapping or changed targets stay fenced, with the human hold preserved.

After human input, use Give control back in the viewer, then `browser_continue` with the same `taskId` reads identity/URL and preserves edits. It is separate from CLI executable-transfer handoff.

Saved login: use [native login controls](references/cli-and-service.md#native-login-controls) with the retained episode; no password export or input replay.

## Standalone authentication

Website credentials use standalone Auth Receipts through its verified SDK.
Use `connectHostedAuthHelper` only for a genuine current Codex caller.
An explicitly enrolled unattended application uses `connectEnrolledAuthHelper`
with its own discovery file and exact operation/recovery/browser context; never
copy CODEX_THREAD_ID or fall back to Codex discovery. Auth owns enrollment and
current authority; the adapter does not expose bearer or credential contents.
Before login, MFA or recovery, read [Standalone Auth](references/authentication.md#standalone-auth-receipts).
Use current `discover_credentials`, not empty legacy scopes, to distinguish
missing authority, unavailable/partial sources, no match and account choice.
Before saved-password Native input, use `native_ready`: owner authentication and
consumer input policy must be ready for the exact target; viewer readiness alone
is insufficient. Public `ready` and legacy `local_ready` desktop states require
matching lifecycle identity and enabled input policy; Native control remains separate. Owner decisions, delivery and exact browser binding gate input. Managed-task receiving is product-owned: do not ask operators to create per-task enrollment or read vault secrets as a fallback. Approval comes through the Auth dashboard or Slack via Receipts. `browser_control` status/view can inspect ordinary managed tasks; control changes keep hosted-app requirements. Retained task-briefing process IDs require independent physical verification. Keep unknown
attempts on their original requests; never replay private use.

## Hosted workflow guidance v1

Reuse Bots' browser/task. Before attachment, sharing or takeover, read [host integration v1](references/host-integration.md). Discover schemas; visibility proves neither account nor control. Shared skill publication needs explicit approval.

Tools: `browser_open`, `browser_navigate`, `browser_snapshot`, `browser_click`, `browser_fill`, `browser_screenshot`, `browser_tab`, `browser_handoff`, `browser_continue`, `browser_close`, `browser_control`, `browser_reconnect`, `browser_retention`, `browser_policy`, `browser_artifact`, `browser_command`, `browser_discover`.
Page actions require `taskId`. `browser_tab` manages this task's tabs; retry creation
with the same `tabKey`. After restart, creation rejoins the verified browser
and preserves tabs; peer tabs remain outside scope. Discover actions with `browser_discover`; use `browser_command` with `taskId`, `action`, `params`.
Close rejects changed selection. A pre-commit return failure keeps human control
and epoch; inspect status and use a fresh explicit intent, never replay uncertainty.

Read [retained workflow transport](references/cli-and-service.md#retained-workflow-transport)
before downloads, stale transport recovery, retirement or advanced administration.

Return, store, and reopen only `handoffUrl` (standalone: `/remote-view/<handoff-id>`).
Never send providerExternalUrl or raw Guacamole URLs.

## Operator workspace

Read [Workspace](references/cli-and-service.md#operator-workspace) for tabs, sharing and handoffs. Focus needs an owned tab; reviews are opt-in. Rail catches up once. Remote View owns view/auth. Terminal pool replay retains history.

[Manual login](references/cli-and-service.md#manual-login) is CDP-free. Native control and human takeover are separate.

## Sharing a profile between tasks

Use a separate task/session and tab per workflow. Shared profiles share login and
persistent site state. Read [sharing and continuity](references/cli-and-service.md#shared-profile-workflow-continuity)
before coordinating peers, foreground input or lifecycle effects.

## CLI fallback

If browser_open is absent, report missing Agent Browser MCP integration.
Register installed agent-browser mcp serve in that client and reconnect.
Use the CLI while correcting integration; server tools/list does not prove client exposure.

Help is launch-free. MCP returns `success/data/error`; CLI errors add `type`. Read private journal `details.phase` for unknown outcomes.

Read [Runtime health and cleanup diagnostics](references/cli-and-service.md#runtime-health-and-cleanup-diagnostics) before maintenance, expiry diagnosis or unknown-effect interpretation. Preserve peers and scrub exported URLs.

CDP-free idle close uses verified original OS identity and exit confirmation.
Explicit handoff can supersede idle cleanup after its worker ends and the exact
original is verified alive. Attempt history stays intact; this retention decision
requires a schema-5-capable runtime.
Deleted handoff links cannot be restored by concurrent lifetime edits.


A valid retained handoff can wake its conclusively idle-closed CDP-free browser
through the existing owner. Logical task/tab/link identity and the profile persist;
the saved launch URL reopens, not unsaved in-memory page state. Unknown close proof
or changed scope still defers. Do not close the task or reset its profile as a workaround.

CLI fallback: use installed `agent-browser <command> --help`, the exact profile
and named session. Preserve peers/authentication; opening never requires cleanup
or profile resets.

Read [CLI commands](references/commands.md) for examples.

## SiteGuide routines

Discover `category:"playbooks"` and use `browser_command` in the existing task. Before login, hosted-helper use, guide development or bounded collection, read [SiteGuide routines](references/cli-and-service.md#siteguide-routines) and [the routine guide](references/playbooks.md). Verify the intended account/company; browser login and OAuth/API readiness are separate.

## References

Preserve peers and original task IDs during recovery.
[Commands](references/commands.md), [snapshots](references/snapshot-refs.md),
[authentication](references/authentication.md), [sessions](references/session-management.md).
Discover advanced operations with `browser_discover`.

Sign-in and teaching: [workspace tools](references/workspace-sign-in.md); viewer failures: [diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics).

## Additional operations

For an operator-requested fresh start, [runtime reset](references/cli-and-service.md#explicit-runtime-reset)
ends production tasks and unsaved pages. Read its preview and recovery procedure before applying it.
Read [builds, modes, teaching and owner waits](references/cli-and-service.md#browser-mode-and-builds)
before those operations. Copies do not prove login; never replay pending or uncertain input.
