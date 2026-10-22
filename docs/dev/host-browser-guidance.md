# Versioned host browser workflow guidance

Guidance contract: `agent-browser.host-guidance.v1`.
Revision: 2026-10-22. Parent: #347; guidance: #354; execution: P270.

This guide applies to the source contracts for hosted placement, attachment and
cooperative control. Discover the connected runtime before choosing an operation.
The source skill is not proof that an installed runtime or host app has adopted
it. Publishing shared or installed skills remains separately approval-controlled.

## Choose the smallest useful workflow

| Situation | Workflow |
| --- | --- |
| Bots already has a usable browser and its native tools complete the task | Keep Bots-native tools and the app's original browser/session. |
| Reusable Agent Browser task actions and continuation reduce work | Use Agent Browser with the intended named profile and stable task key. |
| The app owns an existing browser with unfinished work | Use supported attachment through a trusted app observer; omit URL. |
| The app requests a new browser on its assigned desktop | Use explicit launch with current required/preferred host placement. |
| A person only wants to look | Open the ready durable handoff; viewing does not pause automation. |
| A person needs input | Explicit coordinated takeover, then return to the same task. |

Bots owns its app participant/session, browser lifecycle and native control journal.
Agent Browser supplies task/page/profile continuity and its supported automation
control gate. Remote View owns the existing authenticated desktop and native
viewer. None should create a second viewer, extra Open desktop ceremony or hidden
replacement browser. Direct CDP or Bots-native tools remain appropriate when they
already solve the task; Agent Browser is optional rather than a migration mandate.

## Discover compatibility and current capability

1. Read connected MCP `tools/list`, `agent-browser://contracts` and
   `agent-browser://operating-guide`. Confirm actual argument schemas, required
   attribution fields and supported profile modes before dispatch.
2. Use `browser_discover` for the selected advanced operation, for example
   `{"category":"browser","operation":"press"}`. Do not guess dispatch names.
3. Compact `mcp host-task TASK` and `mcp watch-host-task TASK` report versioned
   host identity/capabilities and controller metadata. Version 1 means the
   supported wire shape, not provider/browser/viewer/account readiness. Watcher
   reconnect begins with a full snapshot; it is not a replay cursor.
4. Unknown versions, absent tool fields or a concrete unsupported capability
   mean retain the app-native workflow or use the supported alternate profile.
   Preserve the current browser. Report the missing capability instead of
   changing a profile, launching a replacement or removing placement constraints.

Capability discovery, exact process/browser observation, Native pixels/control
and intended account/company verification are separate evidence. A current schema
cannot prove a live desktop. Do not infer physical relocation support from reconnect or policy
schemas. Consult the connected operation schema and current acceptance boundary. Old clients must not silently drop unknown required host intent;
refresh their MCP catalog after an approved runtime update.

## Launch, attach and preserve the original task

For launch, supply serviceName, agentName and a concrete taskName as descriptive
creator context; these labels do not authenticate authority. Select a named
`persistent` profile for reusable work or named `existing` profile to refuse
creation. `disposable` mode omits profile and is for genuinely temporary work.
Do not silently use the default or a disposable profile for established login.

The configured hostBinding v1 binds hostId, participantId, the app's original
sessionId and current desktopId/generation. Required placement never falls back.
Preferred placement permits only the explicit ordered alternatives. An existing
browser elsewhere is a conflict, not permission to move it or reset the profile.
The app's registry origin and provider application are reviewed configuration,
not arbitrary browser-request URLs.

Attachment uses profileMode existing plus the trusted current process/start token,
executable, canonical profile path, full CDP browser endpoint and existing page
identity. It omits URL and preserves initial tabs, draft/storage and process.
In Bots the server's observeAttachment factory derives this proof; the web
request must never accept arbitrary PID/path/CDP descriptors. An unavailable
attachment capability means continue app-native work. Each explicitly shared
recipient uses its own exact participant/session/task key and the owner's same
sharing list. Logical tab IDs differ even when the physical target is shared.
Sharing grants no termination or relocation authority.

Retain returned taskId/browserId/tabId/profileId and original app session. Reuse
stable taskKey only to continue the same intent. Do not replace the app session
with its newest thread, select a peer tab, or replay open with a new URL to recover
unfinished work. Read current page identity before consequential action.

## Route viewing and explicit control

Use only the returned ready handoffUrl. Hosted links are opaque app-origin
`/agent-browser/handoff/<id>` routes; standalone links remain `/remote-view/<id>`.
The app validates the exact original participant/session/task/desktop before
mounting the existing Native viewer. Resolution is read-only: no acquire, generic
viewer ensure, browser launch or latest-session substitution. Raw provider/embed
URLs and health endpoints are not operator links.

Viewing alone never pauses. Query browser_control status/view for current epoch.
Explicit takeover supplies expectedEpoch, actor, reason and optional pauseReads;
pauseReads also blocks page reads/capture. Supported ordinary mutations stop
while human-owned or awaiting observation. Bots coordinates this with its existing
Native selective-control journal; do not bypass it with direct SDK control.
Return releases only that handoff's exact Native intent, returns Agent Browser
ownership and freshly observes the same retained target before automation resumes.
A peer's grant cannot qualify or be revoked by this return. Direct unmanaged CDP
and OS controllers must cooperate; Agent Browser cannot intercept them.

Uncertain native or website effects stay held with the original intent/request ID.
Read status and explicitly reconcile the advertised original request; do not
retry click, fill, submit or native takeover after a lost acknowledgment. A lost
browser is reported, not silently replaced. Read-only status/resolve do not renew
an expired Native grant; expired-grant reopen remains a separate acceptance scope.
Use the host's retained-intent recovery/readback rather than inventing a retry.


## Explicit reconnect and loss handling

browser_reconnect takes taskId and operation status/disconnect/reconnect/rebind/
configure/reopen. Only status omits requestId and expectedRevision. configure
sets lossPolicy report_only (default) or reopen; rebind supplies exact
{desktopId,generation}. Status is metadata-only; reconnect freshly verifies the
retained process/Native assignment without launch. Unreachable, positively absent
lost, rebind-required and pending states are distinct. Returned reconnect metadata
includes revision, state, lossPolicy, lastRequestId, observedDesktop, pending and
automaticReplay:false and nullable recoveryDesktop.


When the original managed process is positively absent, explicit rebind may retain
an exact current ready desktop as approved recoveryDesktop. State stays
browser_lost: this records a future recovery destination, not a physical move,
connection or rewrite of the old browser binding. A subsequent explicit reopen
under lossPolicy reopen requires agent ownership and no pending effects. Only a
verified replacement on that approved target publishes its new physical binding
and clears recoveryDesktop. Live managed generation-changing rebind remains
capability-refused. PID-reuse absence is conservatively blocked. Borrowed reopen
still requires fresh app attachment/desktop proof and preserves any human hold.
This source contract's new recovery path still needs native custody qualification.

Disconnect never returns human control. Explicit opted-in reopen refuses uncertain
pending effects. Managed reopen uses empty history/blank targets under agent
control; borrowed reopen requires fresh app attachment and desktop proof and does
not launch the app browser. Human holds remain. Unknown lifecycle effects retain
the original request without replay. This is continuity, not physical relocation.

## Optional hosted rules and artifact handles

Discover `browser_policy` and `browser_artifact` on the connected runtime before
use. No configured restrictions means ordinary tasks need no extra grant.
`browser_policy` takes taskId and operation: status, set_app, set_task, clear_app,
clear_task or override. Changes require expectedRevision, actor and reason.
Policy v1 contains deniedActions (supported dispatch action names) and
artifactDestinations (handle to an absolute, canonical, existing local directory).
A per-task policy replaces the complete app default; clear_task restores defaults.
Optional browser_open taskPolicy accepts the same v1 policy before launch so an
explicit task override is not blocked by app defaults during creation.
These are cooperative supported-route rules, not a website-semantic sandbox or
interception of unmanaged CDP clients.

```json
{"taskId":"<task>","operation":"set_task","expectedRevision":0,"actor":"operator","reason":"Refuse direct click and fill","policy":{"version":1,"deniedActions":["click","fill"],"artifactDestinations":{"evidence":"/private/task-evidence"}}}
```

An explicit override names overrideAction and requestId and returns overrideId.
Pass requestId and policyOverrideId at the top level of the supported page tool
or browser_command (for example action fill with selector/value in params).
Use the override only on the exact task/action/request under the same
effective policy. It never bypasses human takeover, profile or peer custody.
Read status to obtain the current revision rather than guessing it from examples.

`browser_artifact` takes taskId, kind (screenshot or download), handle and relative
name; download also requires selector. Optional fullPage, requestId and
policyOverrideId follow the connected schema. The app-authorized destination must
already exist; traversal, symlinked destinations and existing output files are
refused. Returned artifactId/path/handle/name/result are local metadata with
publication not_requested. Capture/download and publishing remain separate;
review content and obtain existing sharing authority before export.

```json
{"taskId":"<task>","kind":"screenshot","handle":"evidence","name":"page.png","requestId":"capture-once"}
```

## Physical relocation capability gap

A retained app browser stays on its current desktop. No implemented physical
same-process relocation is promised here. Remote View #578 must first provide an
exact supported transfer with generation and process/window continuity. Until
then, retain the original browser or explicitly choose another task/profile.
Metadata rebinding is not physical relocation. #351 and the joined #355 outcome
remain dependent on that provider proof; do not reopen or reset a profile to
simulate a move. See the source [provider gap note](notes/0270-2026-10-22-hosted-relocation-provider-gap.md).

## Verify accounts and keep login secret-safe

Check the intended service, logged-in identity and selected company/tenant in the
current page before consequential work. Profile names, cookies being present,
viewer access and OAuth/API authorization are not account-verification evidence.
Use an existing approved host login helper or the same human handoff for password
and private MFA entry. Never put protected input into ordinary fill arguments,
prompts, CLI history, screenshots, downloadable receipts or exported logs.
Unknown login delivery remains held for same-request observation. Do not sign in
to a different account or reset authentication to make a trial pass.

## Artifacts and cleanup

Choose task-owned private artifact paths and capture only permitted content.
Review screenshots/downloads before sharing; avoid credentials, tokens, raw CDP
endpoints and private profile contents in exported evidence. Keep uncertainty
journals and cleanup receipts private while publishing only curated conclusions.

browser_close scope task is the default. Explicit scope browser requires actor
and reason, refuses borrowed browsers and any peer task, and reports fresh OS
absence of the original managed process only. Independently census children before
claiming no residue. Uncertain close is not replayed.

Close only your task. Attached close detaches from the borrowed initial page and
browser; the app owns their lifecycle. Task-created tabs have scoped cleanup.
Before claiming an owned browser is gone, check exact PID/start generations and
profile references, not only an environment marker that provider launch settings
can overwrite. Release only the exact owned assignment after absence is proven.
Preserve baseline peers and held outcomes. Cleanup failure remains an obligation,
not an excuse to delete profiles or kill unfamiliar processes.

## Synthetic trial and evidence boundaries

Run `node scripts/test-host-browser-guidance.mjs` with an absolute reviewed
AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD and optional
HOST_GUIDANCE_EXPECTED_BINARY_SHA256. It initializes the actual public MCP server
in a disposable home, reads connected discovery/contracts, and submits synthetic
example.test intents that must refuse invalid profile modes, unnamed existing
profiles, URL-bearing attachment, unsupported host versions and peer retargeting.
It checks metadata-only refusal without allocating a profile or daemon socket.
This is a no-launch compatibility/refusal trial, not successful page interaction.

Prior P268/P267 Native04/05 acceptance in
[the accepted source note](notes/0268-2026-10-22-hosted-attachment-control.md)
separately proves app-owned/shared browser continuity, real SDK takeover/return,
public supported-route gating, synthetic cookie/draft/storage preservation and
exact owned cleanup with real Native. It is bounded to #348–#350 source fixtures,
not installed adoption or private-account access. P270/#355 owns the joined
successful trial after #351–#353; this guide does not preclaim that result.
