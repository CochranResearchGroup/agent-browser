---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Authelia admission grants operator access without local dashboard enrollment or roles. Dashboard-local password/cookie login is disabled; attribution uses the verified owner principal.

[Control and credentials](references/cli-and-service.md).

Build failures: [readiness guidance](references/cli-and-service.md#build-readiness).

`candidate status` exposes `coordinationLedger`; `candidate inspect` uses `--input-closure`; `candidate install` needs `--sealed-artifact`. [Guide](references/cli-and-service.md#sealed-candidate-maintenance).

## Handoff creator context

Creation requires immutable `serviceName`, `agentName` and concrete `taskName`.
They are attribution, not authenticated identity. Read [creator context](references/cli-and-service.md#handoff-creator-context).

## MCP task workflow

Writer custody survives executable unlink.

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Existing profiles: `profileMode:"existing"`. Temporary: `profileMode:"disposable"` without `profile`. The workflow owns selection, replay and scoped closure.

Installed viewers: [qualification](references/cli-and-service.md#native-journey-qualification).

Use `browser_handoff({ "taskId": "<task-id>" })` for human assistance. It defaults to interactive input and pauses cooperative automation. Use `interaction:"view"` for passive viewing. The authorized recipient needs no administrator override; visibility alone does not prove input readiness. Require `operatorVisible.state=ready`; return the complete `data.handoffUrl` and keep the task ID. For `remote_view_public_origin_missing`, check reviewed `AGENT_BROWSER_PUBLIC_ORIGIN` in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; process settings win. Reconnect MCP after configuration changes because its environment is pinned at startup. Preserve the task. Never assemble provider URLs.

Same-browser and desktop rail tab switches retain the viewer connection.
Retained handoff recovery: [guide](references/cli-and-service.md#retained-handoff-recovery).

After human input, use Give control back in the viewer, then `browser_continue` with the same `taskId` reads identity/URL and preserves edits. It is separate from CLI executable-transfer handoff. A pending browser mode change does not block the same durable handoff while the exact original browser remains live. Reconnecting preserves the mode operation and does not close or replace that browser.

Nested login outlines require same-frame boundary or bounded connected-field proof; conflicting mask/value evidence stays unverified. Distinct controls retain ambiguity.

Saved login: use [native login controls](references/cli-and-service.md#native-login-controls) with the retained episode; no password export or input replay. The SDK tiles oversized windows under the owner pixel limit and freshly recaptures its chosen region; it never stitches input evidence or extends the episode.

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
matching lifecycle identity and enabled input policy; Native control remains separate. Owner decisions, delivery and exact browser binding gate input. Managed-task receiving is product-owned: do not ask operators to create per-task enrollment or read vault secrets as a fallback. Approval comes through the Auth dashboard or Slack via Receipts. `browser_control` status/view inspect ordinary managed tasks; explicit return also supports standalone handoffs after exact-process and fresh same-target verification. Takeover/override keep hosted-app requirements. Passive view does not return a human hold. Retained task-briefing process IDs require independent physical verification. Keep unknown
attempts on their original requests; never replay private use.

## Hosted workflow guidance v1

Read [host integration](references/host-integration.md) before placement,
attachment, sharing or takeover. Preserve existing task ownership. Discover
runtime schemas; visible pixels do not prove account or input ownership.
For advanced operations and recovery, read [workflow transport](references/cli-and-service.md#retained-workflow-transport).
Return, store, and reopen only `handoffUrl`, the opaque `/remote-view/<handoff-id>` URL.
Never send providerExternalUrl or raw Guacamole URLs.
Page actions require `taskId`; retries keep the same task and request identity.

## Operator workspace

Read [Workspace](references/cli-and-service.md#operator-workspace) for tabs, sharing and handoffs. Focus needs an owned tab; reviews are opt-in. Rail catches up once. Remote View owns view/auth. Terminal pool replay retains history.

[Manual login](references/cli-and-service.md#manual-login) is CDP-free. For CDP, end the process with browser scope and actor/reason, then reopen that profile.

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
Uncorrelated displays stay protected: missing Service State proves no ownership or cleanup authority.

CDP-free idle close uses verified original OS identity and exit confirmation.
Explicit handoff can supersede idle cleanup after its worker ends and the exact
original is verified alive. Attempt history stays intact; this retention decision
requires a schema-5-capable runtime.
Deleted handoff links cannot be restored by concurrent lifetime edits.
Closed-task links offer **Reopen task** on the same URL. GET is read-only;
missing disposable storage needs a separate disclosed fresh-profile choice.
Never substitute a named profile. Read [closed-task reopening](references/cli-and-service.md#closed-task-reopening)
for software operations, retry and human-control semantics.

Idle-closed CDP-free handoffs reopen through the owner, preserving task/tab/link
and profile, but not unsaved page state. Unknown proof or changed scope defers;
never reset the profile as a workaround.

CLI fallback: use installed help, exact profile and named session; preserve peers.

Read [CLI commands](references/commands.md) for examples.

## SiteGuide routines

Discover `category:"playbooks"` and use `browser_command` in the existing task. Before login, hosted-helper use, guide development or bounded collection, read [SiteGuide routines](references/cli-and-service.md#siteguide-routines) and [the routine guide](references/playbooks.md). Verify the intended account/company; browser login and OAuth/API readiness are separate.

## References

Preserve peers and original task IDs during recovery. Discover advanced
operations with `browser_discover`. Read [commands](references/commands.md),
[authentication](references/authentication.md), [sessions](references/session-management.md)
and [diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics).

## Additional operations

For an operator-requested fresh start, [runtime reset](references/cli-and-service.md#explicit-runtime-reset)
ends production tasks and unsaved pages. Read its preview and recovery procedure before applying it.
Read [builds, modes, teaching and owner waits](references/cli-and-service.md#browser-mode-and-builds)
before those operations. Copies do not prove login; never replay pending or uncertain input.

Public task tools: `browser_snapshot`, `browser_screenshot`, `browser_click`,
`browser_fill`, `browser_navigate`, `browser_tab`, `browser_retention`,
`browser_reconnect`, `browser_policy`, `browser_artifact`.

Saved-entry/submit refinement preserves exact-account, mask and ambiguity checks. `ownerTimeoutMs` (100..300000) plus optional `ownerDeadlineAtMs` bound one admitted Native command to the remaining invocation deadline, without renewing frame freshness or permitting uncertain-input replay. See the Native workflow reference.

SDK allowance:300s default. ACK-prefix resume preserves original clocks; unknown input stops, never replays.
