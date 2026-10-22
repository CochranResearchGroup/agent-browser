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
