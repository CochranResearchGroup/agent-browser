# Host integration guidance v1

Guidance contract: `agent-browser.host-guidance.v1`, revision 2026-10-22.
Read the [workflow decision guide](https://github.com/CochranResearchGroup/agent-browser/blob/main/docs/dev/host-browser-guidance.md) for tool choice, compatibility, login and cleanup.

## App-directed browser placement and handoff

An app can pass `hostBinding` to `browser_open` while retaining Agent Browser's
normal task and profile controls. Configure `AGENT_BROWSER_HOSTS_FILE` on both
its MCP transport and runtime host. It names a versioned JSON registry:

```json
{"version":1,"hosts":{"my-app":{"origin":"https://app.example.test","providerApplication":"my-app"}}}
```

The configured `AGENT_BROWSER_REMOTE_VIEW_APPLICATION` must match that host's
provider application. This selects the existing Remote View resource context;
it does not grant access to another application's assignments.

A configured hosted runtime leaves spare-desktop growth and retirement to the
app. Automatic pool maintenance also stays disabled while durable host bindings
remain, including after configuration is removed. Explicit browser tasks retain
their ordinary scoped close behavior.

```json
{"version":1,"hostId":"my-app","participantId":"worker-1","sessionId":"session-1","placement":{"mode":"required","desktop":{"desktopId":"<provider-desktop-id>","generation":1}}}
```

A required desktop must be currently observed in the configured provider pool.
Preferred placement uses `mode: "preferred"` with `desktop` and an ordered
`alternatives` array, and never falls back outside that list. A retained browser
on another desktop reports a conflict without moving or replacing it.

Hosted `browser_handoff` returns the configured app origin's opaque
`/agent-browser/handoff/<id>` URL without requiring a standalone
`AGENT_BROWSER_PUBLIC_ORIGIN`. The app must implement the resolver: validate
its exact participant, saved session, task and current native desktop binding,
then mount the existing authenticated Native viewer. Never substitute the
participant's latest session or use an acquire/launch operation while resolving.
The URL alone does not prove that an app resolver is installed or viewer-ready.
Standalone tasks keep their existing durable handoff behavior.

Use `agent-browser --json mcp host-task <task-id>` for a compact metadata-only
snapshot and `agent-browser --json mcp watch-host-task <task-id>` for an initial
snapshot followed by changed NDJSON events. The dedicated watcher checks only
SQLite/WAL metadata every 250 ms while unchanged, suppresses unrelated commits,
and never launches, focuses or reconciles browsers. Each connection has its own
`watchId` and monotonic sequence; reconnect starts with a full snapshot, not a
replay cursor. Terminate the watcher process for immediate disconnect; a broken
output pipe otherwise exits on the next emitted event. Recorded browser state,
viewer delivery, controller state and account authentication are separate;
metadata-only reads leave unobserved readiness explicitly unverified.


### Attach and explicitly share an app browser

`browser_open` can borrow a currently running app browser instead of launching.
Supply `profile` (the existing profile name), `profileMode: "existing"`, the normal
attribution/task key and configured `hostBinding`. Omit `url`; attachment never
navigates, opens an initial tab, duplicates a profile process or resets work.
Add this `attachment` object:

```json
{
  "version": 1,
  "process": {
    "pid": 1234,
    "startToken": "<current-kernel-process-start-token>",
    "executablePath": "/opt/chromium/chrome",
    "browserFamily": "chromium"
  },
  "profilePath": "/absolute/canonical/app-profile",
  "cdpEndpoint": "ws://127.0.0.1:9222/devtools/browser/<browser-id>",
  "targetId": "<existing-page-target>",
  "sharedWith": []
}
```

The supported local adapter verifies the current process/start/executable,
canonical profile directory, CDP browser PID, existing page and current Native
assignment/window PID before publishing a task. Unsupported platforms or missing
CDP return a concrete capability error without launching. Use the app's native
browser tools when attachment is unavailable. A profile already on another
desktop stays there; supported alternatives are retaining it, selecting another
profile, or an explicit relocation operation when that capability is available.

The original app owns lifecycle. Closing/detaching an attached task preserves
its borrowed initial page and browser process; task-created tabs retain scoped
cleanup. Direct close of a borrowed page is refused. A missing borrowed browser
is reported rather than automatically reopened. The owner may explicitly set
`sharedWith` to exact `{hostId, participantId, sessionId}` recipients by replaying
its original attachment task. The list controls new attachment admission; removing a recipient does not close
its existing tasks. Each recipient attaches explicitly with its own
task key and app binding; it cannot change the owner's sharing list. Sharing does
not transfer termination or relocation authority. Metadata reports
`custody: "app_borrowed"`; capability presence is not readiness or account proof.

## Explicit human control

Use `browser_control` for status/view, explicit takeover, return or scoped override.
Viewing never pauses. Takeover blocks supported automation; optional pauseReads
also stops page reads/capture. Return freshly observes the original page without
recovery; uncertain work is retained without replay. See the [control contract](https://github.com/CochranResearchGroup/agent-browser/blob/main/docs/dev/hosted-browser-control.md).

## Discover before dispatch

Read the connected MCP `tools/list` schemas and `agent-browser://contracts` plus
`agent-browser://operating-guide`. Use `browser_discover` for the exact advanced
operation schema. Tool names, schema presence and host snapshot capabilities are
compatibility evidence, not current browser/viewer/account readiness. A runtime
missing attachment or control must keep the current app-native workflow; do not
silently launch or switch profiles. Reconnect MCP after an approved runtime update.
Do not infer physical relocation support from reconnect or policy schemas.

## Account and secrets

A named profile and a ready viewer do not establish the logged-in account or
company. Verify the intended identity in the current page before consequential
work. Use the existing approved host login helper for protected input or ask the
person to sign in through the same durable handoff. Never paste credentials into
ordinary page arguments, prompts, CLI history, screenshots or exported logs.
Keep browser-site login, Native access and OAuth/API authorization separate.
Unknown login delivery is held for same-request observation, never resubmitted.

## Outcome and artifact custody

Record the task, browser, tab, profile and exact host session; keep raw process/CDP
addresses and secrets private. After uncertain mutation, inspect control status
and the original request outcome; reconcile only through the advertised operation.
Do not repeat click/fill/submit because the response was lost. Restrict screenshots
and downloads to task-owned private paths and permitted page content; export only
reviewed artifacts. Close only the task you own. Attached task close detaches;
the app owns borrowed browser/process/profile cleanup. Prove exact process start
generations absent before releasing an owned assignment, and preserve peers.
Environment-marker census alone is insufficient when provider launch environment
can overwrite markers. Preserve unresolved effects and private evidence.

Skill publication is separate from source guidance: do not overwrite shared or
installed skill copies, or distribute app-private configuration, without explicit
approval. Guidance revisions do not grant runtime, account or publication authority.

## Optional task rules and local artifacts

Discover browser_policy and browser_artifact before use. browser_policy status
reads the current revision; set_app/set_task/clear_app/clear_task/override changes
require expectedRevision, actor and reason. Policy v1 accepts deniedActions and
artifactDestinations mapping handles to canonical existing local directories.
Task rules fully replace app defaults; clear_task restores them. Optional
browser_open taskPolicy binds the same v1 task override before launch. No rules means
no extra permission ceremony. An overrideId is limited to its exact task, action,
requestId and effective policy and never bypasses human control or peer custody.

browser_artifact accepts taskId, kind screenshot/download, handle and relative
name; download needs selector. Optional requestId, fullPage and policyOverrideId
follow discovery. It refuses traversal, symlinked destinations and existing files.
Results are local metadata, publication not_requested; sharing is a separate act.
Restrictions cover supported dispatch routes, not unmanaged CDP or semantic site
policy. Physical same-process relocation remains unavailable pending Remote View
#578; metadata rebind must not be described as a move.

## Explicit reconnect and loss policy

browser_reconnect operations are status/disconnect/reconnect/rebind/configure/reopen.
All changes require requestId and current expectedRevision; status is metadata-only.
configure sets lossPolicy report_only (default) or reopen. reconnect freshly verifies
the retained process/Native without launch; rebind requires desktopId/generation.
Explicit opted-in reopen never replays history: managed browsers use blank targets;
borrowed browsers require fresh attachment plus desktop proof and no app launch.
Unknown lifecycle effects remain pending without replay; human holds persist.
Disconnect adds only its own retention hold when needed; successful reconnect,
rebind or reopen releases that hold, never another owner's leave-open intent.
Compact reconnect metadata reports disconnectHold.
Use `agent-browser --json mcp host-reconnect TASK status` for CLI metadata readback.

browser_close defaults to scope task. Explicit scope browser requires actor/reason,
refuses borrowed browsers and peer tasks, and uses fresh OS original-process
absence readback. It does not prove every child gone; independent census remains
necessary. Unknown close outcomes never authorize replay.
