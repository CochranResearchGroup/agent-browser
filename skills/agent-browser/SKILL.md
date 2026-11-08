---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Browser Automation with agent-browser

Workspace access requires the shared Authelia guard; native dashboard password login and session cookies are disabled pending compatible Remote View authentication.

Native credentials: [Remote View setup](references/cli-and-service.md).

A `dashboard_build_incomplete` install/doctor failure requires rebuilding the frontend and recompiling the candidate. The maintained `pnpm build:development-candidate` command includes both steps.

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

Borrowed-browser expiry uses service activity guards and closes only owned tabs. Unknown ownership defers that browser; independent eligible tabs continue.

Use a separate task/session and tab for each workflow. Shared profiles share login and persistent site state. Read [Sharing a profile between tasks](references/cli-and-service.md#sharing-a-profile-between-tasks) before coordinating shared-profile work, foreground input or closing a peer task.

Idle capability registration reports `registered_idle` and `acquire_profile`, without claiming ownership. Read [Idle registration](references/cli-and-service.md#idle-registration) before diagnosing a missing binding.

Task continuation, handoff inspection and viewer/native opening reconcile session
state differences internally before selecting the current target. Continue
valid independent work; do not reset the profile, remove locks or repeat an
uncertain action to resolve a metadata difference. Current resource identity and
private reconciliation receipts preserve the competing observations.

Acknowledged command activity is removed only with an atomic publication receipt.
The latest 4096 receipts allow bookkeeping retries after a lost reply without
replaying browser actions. Unknown or expired IDs and changed physical browser
identities remain rejected. Idle-publication errors distinguish `unavailable`,
`invalid_record`, `activity_active` and `conflict` using the
`browser_session_idle_publication_` prefix. Preserve the original action record;
these errors do not authorize replay, profile reset or clearing activity state.

Known occupied or reserved capacity can justify separate bounded capacity for
an independent task. The original reservation stays intact. Failed window
observations never qualify a desktop or justify another acquisition.

Legacy Chrome singleton-lock ownership diagnostics are advisory. An otherwise ready
profile recommends `continue_authorized_workflow`; continue the authorized task
through exact browser/process checks without removing locks. Inspect actual
execution failures and uncertain requests before replaying any effect.

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

## Explicit runtime reset

For an operator-requested fresh production start, use local CLI
`agent-browser runtime reset --json` to preview, then `--apply` to execute.
This is a whole-production Linux user-systemd maintenance command, outside MCP
page actions. It ends tasks and unsaved pages, backs up SQLite, closes verified
production-owned browser processes and retires active task/tab records. It
preserves profiles, login data, links, history and uncertain-effect journals;
foreign/development browsers and Remote View infrastructure remain independent.
An interrupted reset may leave services stopped: use its exact
`runtime reset --apply --resume <reset-id>` command. Completed resume cannot close
later work. Keep the private receipt/backup under the runtime home. Reset does not
prove uncertain actions safe to replay or resolve an unknown profile operation.

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

Structurally valid historical protected-owner observations remain readable even
when their browser row has ended or been removed. They do not create a browser
record or grant current authority. Invalid structure, forged authority and
inconsistent live PID bindings still return
`protected_browser_owner_observation_invalid`.

Opening a profile can finish its original retirement transaction when the close
is already settled and its logical scope has ended. Agent Browser requires exact
Native fence readback and atomic publication before completing that fence; it
never repeats the physical close. This reconciliation is limited to the requested
profile. Unresolved effects, changed identities and unavailable Native proof
retain their original reconciliation requirement.
