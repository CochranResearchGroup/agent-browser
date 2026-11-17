---
name: agent-browser
description: Browser task workflows through Agent Browser MCP, with CLI fallback. Use for website navigation, forms, login, extraction, screenshots, testing and browser automation.
allowed-tools: Bash(agent-browser:*)
---

# Agent Browser

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

Use `browser_open` with `{"serviceName":"my-app","agentName":"editor","taskName":"Edit draft","profile":"editorial","profileMode":"persistent","taskKey":"editing","url":"https://example.com"}`. Read `data.taskId`, `profileId`, `browserId`, `tabId`, `url`. Address actions with `taskId`. Reuse `taskKey` to preserve the page/edits without navigation; distinct keys separate tasks. `browser_close` ends only the addressed task.

Clicks deliver pointer input; verify page state. [Profile rules](references/cli-and-service.md#task-clicks-and-profiles).

Installed viewers: [qualification](references/cli-and-service.md#native-journey-qualification).

Use `browser_handoff({ "taskId": "<task-id>" })` for human assistance. It defaults to interactive input and pauses cooperative automation. Use `interaction:"view"` for passive viewing. The authorized recipient needs no administrator override; visibility alone does not prove input readiness. Require `operatorVisible.state=ready`; return the complete `data.handoffUrl` and keep the task ID. For `remote_view_public_origin_missing`, check reviewed `AGENT_BROWSER_PUBLIC_ORIGIN` in `~/.agent-browser/.env` or `AGENT_BROWSER_ENV_FILE`; process settings win. Reconnect MCP after configuration changes because its environment is pinned at startup. Preserve the task. Never assemble provider URLs.

Reuse the same handoff after a CDP-free exit; reconcile interrupted conversions explicitly.
A confirmed mode change restarts the same profile, retains its tabs and session names, and updates every attached task to the replacement process. After exit, the old PID and desktop assignment are historical records, not reasons to refuse the replacement. The supported workspace chooser includes Chrome and Chromium; Chrome for Testing is identified separately and excluded.
Retained handoff recovery: [guide](references/cli-and-service.md#retained-handoff-recovery).

After human input, use Give control back or `browser_control`, then `browser_continue` with the same `taskId`. Preserve edits and the [pending-mode boundary](references/cli-and-service.md#retained-handoff-recovery).

Nested login outlines require same-frame boundary or bounded connected-field proof; conflicting mask/value evidence stays unverified. Distinct controls retain ambiguity.

Saved login: use [native login controls](references/cli-and-service.md#native-login-controls) with the retained episode; no password export or input replay.

For an operator-requested stuck-browser restart, inspect `mode`, then use `mode_convert` with that same mode and one stable `operationId`: `prepare`, then `relaunch` with `ownerConfirmed: true`. Keep the profile and logical tasks; reconcile lost replies on that operation and verify the retained page with `browser_continue`. Browser-scope `browser_close` is not the shared-browser restart path.

## Semantic-worker logons

On a signed-out observation, inspect applicable login guidance early. First check this browser/profile's saved credentials. Focus the login field, capture the browser-owned autofill selector and select the intended account through Native vision/input; CDP cannot operate that selector. Reuse the [saved-login routine](references/cli-and-service.md#native-login-controls) when it fits; supported semantic Native interaction remains available for other layouts. Use `browser_command` `native_capture`/`native_input`, not challenge-only `desktop_interact`; observe the same task with `native_capture` and respect the owner's current readiness/control checks before input.

For installed LastPass, inspect the toolbar and extension UI: red with three white dots is the expected active cue, requiring actual UI/account confirmation. Use Native interaction for credentials/passkeys. Non-red indicates another authentication state; try a suitable authorized Auth path before operator interruption, then use the same ready Native handoff if unblock is needed. Auth passwords, passkeys, TOTP and mail/im MFA remain separately qualified capabilities in testing. Diagnose possible integration bugs without overriding real denials. Preserve task/profile/account, original Auth journey and uncertain operations; reconcile before fallback. Verify website-account access and original-work continuation after return. Follow the [complete login sequence](references/authentication.md#semantic-worker-login-sequence); source guidance does not prove installed readiness.

## Standalone authentication

Follow explicit user instructions and service-specific authentication guidance.
Use an available authorized stored credential through its supported private
channel. Auth Receipts is the generic path when an agent needs credential
discovery, owner approval or private login orchestration and no more specific
authorized path is available. It is not required for every browser login.

Keep the username and password from the same credential source together. A
service account name selects its matching credential pair; it does not replace
the username belonging to another saved password. Do not mix pairs based on an
account hint or discovery ranking. An explicitly requested alternative mapping
must remain explicit rather than becoming an inferred default.

Preserve account, realm, browser ownership and private-input checks for every
path. Read secret files only inside the authorized login process; never put
secrets in model context, command arguments, logs, screenshots or Git. An
already-authorized credential file or saved login does not need migration into
Auth before use.

For route selection and special cases, read [authentication paths](references/authentication.md#choose-an-authentication-path). The following Auth helper rules apply when Auth is the selected path. Discover connected channels before asking the operator; proposals grant no authority.

Use `connectHostedAuthHelper` only for a genuine current Codex caller.
An explicitly enrolled unattended application uses `connectEnrolledAuthHelper`
with its own discovery file and exact operation/recovery/browser context; never
copy CODEX_THREAD_ID or fall back to Codex discovery. Auth owns enrollment and
current authority; the adapter does not expose bearer or credential contents.
Owner-login `advance()` returns bounded `recovery` hints and existing next operations. Preserve original request/binding/effect state; `fallbackEligible` never authorizes credentials or retries. Unknown input blocks fallback until reconciled. Source/SDK failures are integration diagnostics, not denial or an empty vault. Use the returned Auth repair owner for a redacted owning issue.
Use current `discover_credentials`, not empty legacy scopes, to distinguish
missing authority, unavailable/partial sources, no match and account choice.
Before saved-password Native input, read [input and control ownership](references/authentication.md#native-input-and-control-ownership). Keep original unknown attempts; never replay private use.

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

Native login capture can report `browser_native_operator_control`: use the existing ready handoff for explicit desktop return. Cooperative task ownership does not release Native operator control.

A positively acknowledged Native capture refusal remains a terminal failure with no effect, never a successful capture or permission to retry. Older records without that proof and uncertain input remain held.
