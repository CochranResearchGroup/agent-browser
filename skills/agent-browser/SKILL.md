---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Read [Remote View setup](references/cli-and-service.md) for native credentials.

## Handoff creator context

Creation requires `serviceName`, `agentName`, concrete `taskName`. CLI: `--service-name`, `--agent-name`, `--task-name`; `remote-view open` requires `--origin-kind agent`. MCP sets agent origin; service requests set params.originKind. Labels are immutable attribution, not authenticated identity. Never infer `humanName` or replace labels with polling/job IDs.

## MCP task workflow

Warnings advise; writer custody survives executable unlink.

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Use `profileMode: "existing"` for an existing profile, or `"disposable"` without `profile` for temporary work. The workflow owns selection, replay and scoped closure.

For installed viewers, read [Native journey qualification](references/cli-and-service.md#native-journey-qualification).

For human assistance, call `browser_handoff({ "taskId": "<returned-task-id>" })`. Require `operatorVisible.state=ready` before giving the person its complete `data.handoffUrl`; continue with the same task ID. For `remote_view_public_origin_missing`, check reviewed `AGENT_BROWSER_PUBLIC_ORIGIN` configuration and stale MCP startup. Set it in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; explicit process settings win. MCP connections pin startup configuration: reconnect after changes, preserving the task. Never assemble provider URLs.

After human input, `browser_continue` with the same `taskId` reads identity/URL and preserves edits. It is separate from CLI executable-transfer handoff.

## Standalone authentication

Website credentials use standalone Auth Receipts through its verified SDK.
Before login, MFA or recovery, read [Standalone Auth](references/authentication.md#standalone-auth-receipts).
Use current `discover_credentials`, not empty legacy scopes, to distinguish
missing authority, unavailable/partial sources, no match and account choice.
Before saved-password Native input, use `native_ready`: owner authentication and
consumer input policy must be ready for the exact target; viewer readiness alone
is insufficient. Owner decisions, delivery and exact browser binding gate input. Keep unknown
attempts on their original requests; never replay private use.

## Hosted workflow guidance v1

Reuse the browser/task already owned by Bots. Read [host integration v1](references/host-integration.md) before attachment, sharing or takeover. Discover connected schemas; visibility does not prove account identity or control ownership. Shared skill publication needs explicit approval.

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

Borrowed-browser expiry uses service activity guards and closes only owned tabs. Unknown ownership defers that browser; independent eligible tabs continue.

Use a separate task/session and tab for each workflow. Shared profiles share login and persistent site state. Read [Sharing a profile between tasks](references/cli-and-service.md#sharing-a-profile-between-tasks) before coordinating shared-profile work, foreground input or closing a peer task.

Idle capability registration reports `registered_idle` and `acquire_profile`, without claiming ownership. Read [Idle registration](references/cli-and-service.md#idle-registration) before diagnosing a missing binding.

## CLI fallback

If browser_open is absent, report missing Agent Browser MCP integration.
Register installed agent-browser mcp serve in that client and reconnect.
Use the CLI while correcting integration; server tools/list does not prove client exposure.

Help is launch-free. MCP returns `success/data/error`; CLI errors add `type`. Read private journal `details.phase` for unknown outcomes.

Read [Runtime health and cleanup diagnostics](references/cli-and-service.md#runtime-health-and-cleanup-diagnostics) before maintenance, expiry diagnosis or unknown-effect interpretation. Preserve peers and scrub exported URLs.


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

For sign-in and workspace teaching, read [workspace tools](references/workspace-sign-in.md); for viewer failures, read [diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics).

## Browser mode and builds

Inspect `browser_command`/`taskId` action `mode`. Discover `mode_convert`, `profile_duplicate`, `browser_build`. Build and mode are independent; copies do not prove login. Read [copy experiments](references/cli-and-service.md#profile-lab-independent-copies) and [controls/recovery](references/cli-and-service.md#browser-build-selection). Reconcile the original operation after lost replies.

Owned manual copies use normal close without desktop agent input. Inspect uncertain closes; cancel the original operation before reviewing a new attempt. A reboot is not clean-close proof. Window readiness is separate from agent input authority.

After verification and normal task close, `profile_duplicate` choice `dispose_copy` uses only the original completed `operationId` and `ownerConfirmed: true`, without `taskId`. The owner protects source/peers, refuses live or uncertain custody, and retains exact disposal history. Continue the original interrupted disposal; cancellation does not remove a completed copy.

### Tasks and teaching

Read task_briefing / task_operation via browser_command (params.requestId).
Never replay pending/uncertain input.
See [recovery](references/session-management.md).
Teaching: [A/V/Speech](../../docs/src/app/teaching-evidence/page.mdx),
inert [drafts](references/teaching-drafts.md), [review](references/teaching-review.md),
[grilling](references/teaching-grilling.md), [repeat](references/report-repeat.md).
Recheck Saved provenance; keep gaps. Proposals are not consent. Promotion needs fresh-repeat proof.
Dates: YYYY-MM-DD; downloads need live process proof.
[OCR](references/challenge-assistance.md), [elapsed image timing](references/image-challenge.md).

### Owner service waits

Use the owning service’s bounded lifecycle retention for authentication waits. Read [Owner service waits](references/cli-and-service.md#owner-service-waits); never transfer a claim to a replacement browser or treat retention as credential/control authority.

Fresh Native assignment requires proven profile-process absence and exact owner
`released` evidence. Preserve launch history and separate owner termination from
local cleanup. Missing inventory, denial, mismatched identity or a fixed recovery
target cannot authorize relocation.

Review CDP/CDP-free relaunch from dashboard right-click menus or fullscreen Native handoff CDP toggle.
Mode conversion verifies visibility without input; selective input is governed separately.
