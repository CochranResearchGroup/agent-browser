---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Agent Browser

Change operation, artifact, reconnect and lane budgets live: [capacity guide](references/cli-and-service.md#task-operation-capacity).

For recovery, capacity and owner continuation, see [continuation](references/cli-and-service.md#continue-after-a-refusal).

Inspect an unfinished initial launch with `browser_command` using `action: "launch_status"` and `params.launchId` from the Open failure. This read works before a task exists. If Chrome occupies the profile, keep it open or close that exact window. Launch status reports the original worker boot evidence. A distinct Open can settle a legacy claim with no worker record once it verifies that the exact profile is vacant. A verified owner-host reboot clears prior-boot workers. Recovery checks current vacancy in both cases.

For MCP `browser_open`, supply an optional `requestId` when you need durable launch readback. Reuse the same arguments and request ID to check the original launch without dispatching another Open. Use a fresh request ID for a distinct Open.

For Native application-pool `remote_view_open`, use the top-level `openRequestId` for an initial Open. An identical retry returns launch readback. Existing-target selection uses its existing continuation.

For public refusal occurrence IDs and no-launch task error-log readback, see [the CLI and service reference](references/cli-and-service.md#recover-a-public-refusal).

Authelia supplies access; dashboard login, enrollment and roles are disabled.

[Control and credentials](references/cli-and-service.md).

Builds: [defaults/readiness](references/cli-and-service.md#build-readiness).

`candidate status` exposes `coordinationLedger`; `candidate inspect` uses `--input-closure`; `candidate install` needs `--sealed-artifact`. [Guide](references/cli-and-service.md#sealed-candidate-maintenance).

## Handoff creator context

Immutable `serviceName`, `agentName` and concrete `taskName` attribute creation; they are not authenticated identity. [Creator context](references/cli-and-service.md#handoff-creator-context).

## MCP task workflow

New Chromium launches retain normal popup blocking. Trigger expected popup
workflows through an ordinary user-activation click. A workflow that explicitly
needs unsolicited popups may opt out with the existing browser launch args;
changing launch args does not reconfigure a retained browser.

Unrelated session metadata updates do not block ordinary activity on an unchanged browser; admission checks the current browser identity and conflicting close operations atomically.

Writer custody survives executable unlink.

For a named account and service, resolve the profile in the registry before opening a task. Use `agent-browser service profiles lookup --account-id '<account>' --target-service-id '<service-id>' --json`. Confirm the selected record contains both the requested account and service, then pass its registry ID to `browser_open` with `profileMode: "existing"`. Use `--search` for free-text hints. If a display-label search misses, retry with the structured fields before reporting the profile missing.

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Clicks deliver pointer input; verify page state. [Profile rules](references/cli-and-service.md#task-clicks-and-profiles).

Installed viewers: [qualification](references/cli-and-service.md#native-journey-qualification).

Use `browser_handoff({ "taskId": "<task-id>" })` for human assistance. Interactive capability alone does not pause; actual viewer activity does. Require `operatorVisible.state=ready`, return complete `data.handoffUrl`, retain the task ID and never assemble provider URLs. For `remote_view_public_origin_missing`, review `AGENT_BROWSER_PUBLIC_ORIGIN`; reconnect MCP after configuration changes. See [handoff recovery and configuration](references/cli-and-service.md#retained-handoff-recovery).

Reuse the same handoff after a CDP-free exit; reconcile interrupted conversions explicitly.
A confirmed mode change restarts the same profile, retains its tabs and session names, and updates every attached task to the replacement process. After exit, the old PID and desktop assignment are historical records, not reasons to refuse the replacement. Use Stock Google Chrome or configured Chromium StealthCDP. If a retained executable points to Chrome for Testing, select an approved build before relaunch and check profile compatibility.
Retained handoff recovery: [guide](references/cli-and-service.md#retained-handoff-recovery).

Actual input pauses until qualified quiet; explicit Take control holds until that viewer selects view-only. Other viewers may still hold control; inspect `browser_control` status. Use `browser_continue` for fresh observation with the same `taskId`. Preserve edits and the [pending-mode boundary](references/cli-and-service.md#retained-handoff-recovery).

Nested login outlines require same-frame boundary or bounded connected-field proof; conflicting mask/value evidence stays unverified. Distinct controls retain ambiguity.

Saved login: use [native login controls](references/cli-and-service.md#native-login-controls) with the retained episode; no password export or input replay.

For an operator-requested stuck-browser restart, inspect `mode`, then use `mode_convert` with that same mode and one stable `operationId`: `prepare`, then `relaunch` with `ownerConfirmed: true`. Keep the profile and logical tasks; reconcile lost replies on that operation and verify the retained page with `browser_continue`. Browser-scope `browser_close` is not the shared-browser restart path.

## Standalone authentication

Follow explicit service authentication instructions and authorized credential paths. Keep credential pairs together and secrets inside private input channels. Read [standalone authentication](references/cli-and-service.md#standalone-authentication) before Auth discovery or saved-password use.

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

## Capture browser lessons

After a semantic browser task reveals a reusable success or failure, use
`writing-for-agents` and follow [Capture and consolidate browser lessons](references/playbooks.md#capture-and-consolidate-browser-lessons) before closeout.

## Sharing a profile between tasks

Use a separate task/session and tab per workflow. Shared profiles share login and
persistent site state. Successful activity refreshes the addressed task and tab.
The owner tracks inactivity across restarts and automatically reclaims eligible
idle browsers and unreferenced disposable profiles. Read [sharing and continuity](references/cli-and-service.md#shared-profile-workflow-continuity)
before coordinating peers, foreground input or lifecycle effects.

## CLI fallback

If browser_open is absent, report missing MCP integration and use installed CLI help while configuring the client. Preserve task IDs, profiles, peers and uncertain originals. Read [CLI fallback](references/cli-and-service.md#cli-fallback) before maintenance, closed-task recovery or fallback commands.

## SiteGuide routines

Discover `category:"playbooks"` and use `browser_command` in the existing task. Before login, hosted-helper use, guide development or bounded collection, read [SiteGuide routines](references/cli-and-service.md#siteguide-routines) and [the routine guide](references/playbooks.md). Verify the intended account/company; browser login and OAuth/API readiness are separate.

## References

Preserve peers and original task IDs during recovery. Discover advanced
operations with `browser_discover`. Read [commands](references/commands.md),
[auth and delivery recourse](references/authentication.md), [sessions](references/session-management.md)
and [diagnostics](references/cli-and-service.md#dashboard-summary-diagnostics).

## Additional operations

For an operator-requested fresh start, [runtime reset](references/cli-and-service.md#explicit-runtime-reset)
ends production tasks and unsaved pages. Read its preview and recovery procedure before applying it.
Read [builds, modes, teaching and owner waits](references/cli-and-service.md#browser-mode-and-builds)
before those operations. Copies do not prove login; never replay pending or uncertain input.

Public task tools: `browser_snapshot`, `browser_screenshot`, `browser_click`,
`browser_fill`, `browser_navigate`, `browser_tab`, `browser_retention`,
`browser_reconnect`, `browser_policy`, `browser_artifact`.

Native saved entry/submit keeps exact-account, mask and ambiguity checks. `ownerTimeoutMs` (100..300000) and optional `ownerDeadlineAtMs` bound one invocation without renewing freshness or replaying input; see the Native workflow reference.

SDK:300s default. ACK-prefix resume keeps original clocks; never replay unknown input.

### Exact lost-task retirement

Read [exact retirement](references/cli-and-service.md#exact-lost-task-retirement) before browser `task_retirement`. Preserve task/operation IDs; never replay unknown input. Identity inspection accepts reconciled history; retirement remains strict.

[Window recovery](references/cli-and-service.md#replacement-placement).

Completed ordinary click/evaluate failures support original-request reconciliation
while the shared dispatcher supervisor remains alive. Exact producer completion,
current task/process/epoch and fresh target observation are required. Unknown
outcomes remain retained; this never permits replay or private/native input.


If the task has no tab (`mode_operation_tab_missing`), create one through `browser_tab` with `operation: "new"` and a stable `tabKey`, using `about:blank` or an explicitly requested read-only URL. This rejoins the verified browser without removing profile data. Then prepare the restart with the same operation ID. A recovery tab is a new action; it does not resolve or replay the original uncertain action.


## Evidence-backed login progress

Read [evidence-backed login progress](references/cli-and-service.md#evidence-backed-login-progress) for Auth progress, recovery, independent verification and continuation. Native ownership metadata never proves human activity.

Latency: [phases](references/cli-and-service.md#optional-command-latency-diagnostics).

[Login and grant continuation](references/cli-and-service.md#continued-login-and-scoped-override-capacity).

Time: [freshness and deadlines](references/cli-and-service.md#native-freshness-time).


For a configured CDP-free Auth Receipts route, `browser_command` with
`action: "native_paste_target"` returns the current native window binding to
the Auth owner. This is a read-only metadata action. Auth and Remote View
deliver the approved password privately. Use native input for focus and
submit only after acknowledged paste delivery.
