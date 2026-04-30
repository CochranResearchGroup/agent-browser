# Agent Browser Service Roadmap

Date: 2026-04-22

## Scope

This note records the roadmap direction for turning agent-browser from a CLI
and native daemon into an always-available browser service.

The service should manage long-lived browser processes, durable profiles,
agent sessions, tabs, control queues, site access policy, authentication flows,
challenge handling, live viewing, and operator intervention.

The target is not only browser automation. The target is a browser control
plane that agents and software projects can rely on.

## Product Direction

Agent Browser as a service should be the durable authority for browser state.

The service owns:

- browser process lifecycle
- CDP connections
- runtime profiles
- tabs and targets
- session leases
- queued control requests
- site policies
- authentication and challenge workflows
- live state subscriptions
- crash and health monitoring
- audit and event history

Agents should usually interact through MCP. Independent software projects
should usually interact through an HTTP and WebSocket API. The CLI should
become a compatibility client over the same service authority.

## Architecture Split

The service architecture should keep these surfaces separate:

- core service process: owns browser state, queues, profiles, providers, and
  lifecycle
- MCP server: exposes agent-native resources and atomic tools
- HTTP and WebSocket API: exposes project integration and dashboard access
- CLI: calls the service API for user workflows and scripting
- dashboard: provides fleet visibility, browser viewing, and manual control

No client should own browser truth independently. The service should be the
only authority for current state and command ordering.

## Durable Entities

The first service design should define these entities explicitly:

- `BrowserProfile`: durable identity, user data dir, login hints, site policy,
  and profile storage settings
- `BrowserProcess`: supervised Chrome, Lightpanda, cloud browser, or attached
  browser with health and endpoint metadata
- `Session`: logical agent or user lease over one or more browsers or tabs
- `Tab`: CDP target with URL, title, lifecycle, owner, snapshots, screenshots,
  console events, and network observations
- `Job`: queued browser control request with owner, target, timeout, result,
  and completion signal
- `Monitor`: recurring heartbeat, page check, login freshness probe, or site
  workflow check
- `AuthFlow`: reusable login flow with credential references, provider hooks,
  approval gates, and validation probes
- `Challenge`: detected captcha, passkey prompt, 2FA request, suspicious login
  gate, or blocked flow
- `SitePolicy`: per-site defaults for browser mode, pacing, interaction mode,
  authentication, challenge handling, and provider choices
- `Provider`: integration for credentials, 2FA, captcha or visual reasoning,
  SMS, email, dashboard approval, or external services

## Worker And Queue Control Plane

The worker and queue system remains the first enabling slice.

Every browser-mutating CDP operation should pass through a serialized control
queue scoped by browser and target. This prevents command races, gives clear
backpressure, and makes service state explainable.

The worker should own:

- command dispatch order
- CDP event draining
- browser process checks
- crash detection
- queue backpressure
- idle and shutdown behavior
- per-target leases
- command cancellation and timeout handling
- status fields for observability

Socket, MCP, CLI, dashboard, and API handlers should be adapters over this
worker model. They should not independently mutate browser state.

## Browser Health And Crash Detection

Crash detection should become a first-class service capability.

The service should maintain browser health from multiple signals:

- child process exit
- CDP reader termination
- pending command channel closure
- failed liveness probes
- disconnected DevTools endpoint
- missing or stale targets
- stream and screencast failures

Suggested health states:

- `not_started`
- `launching`
- `ready`
- `degraded`
- `unreachable`
- `process_exited`
- `cdp_disconnected`
- `reconnecting`
- `closing`
- `faulted`

When Chrome exits or CDP disconnects, the service should update state,
publish an event, clear or reconnect browser handles, stop dependent streams,
fail or retry queued work according to policy, and keep non-browser commands
available when possible.

## Access Policy Engine

Site access reliability should be a named product pillar.

The service should include an `AccessPolicy` layer that chooses the safest and
least intrusive strategy that is likely to work for a given site. This should
be framed as reliable authorized access, not as a generic bypass mechanism.

The policy engine should decide:

- headed, headless, Docker headed, remote headed, local, or cloud browser
- persistent profile, temporary profile, or attached existing browser
- direct CDP, DOM action, browser input, human-like input, or manual control
- rate limits, jitter, cooldowns, retry budgets, and parallelism limits
- whether manual login is preferred
- which 2FA providers are allowed
- which challenge providers are allowed
- when to stop and request human approval

Site policies should be layered:

- built-in defaults shipped with agent-browser
- organization config
- project config
- runtime or session overrides
- locally learned observations

Built-in site knowledge should accumulate for major providers such as Google,
Microsoft, GitHub, Canvas, and other commonly automated services.

## Interaction Modes

The service should support multiple interaction backends and choose among them
through policy.

Recommended modes:

- `cdp_direct`: fastest mode for trusted apps and test environments
- `dom_action`: element-aware browser behavior without full pointer synthesis
- `browser_input`: real mouse and keyboard events through browser input APIs
- `human_like_input`: pointer paths, key cadence, pauses, scrolling, focus
  changes, and jitter
- `manual`: operator takeover through the dashboard or attached browser

Direct CDP should remain available. It should not be the only mode.

## Authentication And 2FA Providers

Authentication should be provider-based.

Provider categories:

- browser-native stored credentials
- Chrome profile cookies and passkeys
- browser password manager extensions
- LastPass or similar hosted managers
- KeePassXC or other local vaults
- built-in encrypted credential references
- TOTP providers
- SMS providers, including projects such as `../imcli`
- email providers, including `msgcli`, `gws`, and `gog`
- manual dashboard approval
- intelligence providers for visual/code extraction

Agents should receive capabilities, not raw secrets, unless policy explicitly
allows disclosure. For example, a tool may complete a 2FA prompt internally or
return a redacted status instead of exposing a password or recovery code.

Passkey support may be possible through browser-native flows or password
manager browser plugins. The service should model passkeys as an auth
capability with manual fallback, not as a guaranteed automation primitive.

## Challenge Handling

Captcha solving is a practical requirement for some authorized workflows, but
it should live inside a broader challenge framework.

The default challenge strategy should be:

1. avoid the challenge through persistent profiles, sane pacing, and headed
   browser modes
2. reuse valid login state when available
3. retry with lower intensity and more realistic interaction when policy allows
4. request manual intervention
5. use an approved intelligence or captcha provider only when policy permits it

Potential challenge providers:

- OpenAI-compatible API
- Codex JSON-RPC provider
- site-specific human approval workflow
- external captcha provider where explicitly configured

Every challenge resolution should be auditable. The audit record should include
site, session, policy decision, provider selected, result, and whether a human
approved the action.

## Remote Headed Browser Management

Remote headed browser management should be a named roadmap pillar.

The service should support browsers that are headed from the site's
perspective, but remote from the operator's perspective. This keeps automation
windows off the user's desktop while preserving the compatibility advantages of
headed Chrome.

Separate these concepts:

- `BrowserHost`: where and how the browser process runs
- `ViewStreamProvider`: how the dashboard sees the browser
- `ControlInputProvider`: how remote user input reaches the browser

Suggested browser hosts:

- `local_headless`
- `local_headed`
- `docker_headed`
- `remote_headed`
- `cloud_provider`
- `attached_existing`

Suggested view stream providers:

- `cdp_screencast`
- `chrome_tab_webrtc`
- `virtual_display_webrtc`
- `novnc`
- `external_url`

Suggested control input providers:

- `cdp_input`
- `webrtc_input`
- `vnc_input`
- `manual_attached_desktop`

The first implementation does not need to prove every provider. It should
define the abstraction, then prototype one practical path.

## Docker Headed Browser Option

Docker headed Chrome should become a first-class browser host.

The service should be able to launch a container with:

- headed Chrome
- persistent profile volume
- display server
- streaming sidecar
- expected fonts and media dependencies
- optional browser extensions
- stable DevTools endpoint
- service-managed lifecycle

This gives headless servers the website acceptance profile of a headed browser
while still allowing operators to view and control the browser from the web
dashboard.

## Dashboard Requirements

The dashboard should become the operations console for browser sessions.

It should show:

- browser fleet status
- live browser or tab view
- current URL, title, and tab lifecycle
- active agent, human, or system lease
- queue depth and active job
- site policy selected for the current page
- login and challenge state
- recent events, console logs, screenshots, and crash reports
- controls for watch, takeover, release, pause queue, resume queue, and close

Human takeover should be modeled as a lease. When a human is controlling a tab,
the agent queue should pause, enter cooperative mode, or require explicit
resume.

## MCP Surface

MCP should expose agent-native primitives and resources.

Suggested resources:

- `agent-browser://profiles`
- `agent-browser://browsers`
- `agent-browser://sessions`
- `agent-browser://tabs`
- `agent-browser://jobs`
- `agent-browser://incidents`
- `agent-browser://incidents/{incident_id}/activity`
- `agent-browser://monitors`
- `agent-browser://events`
- `agent-browser://site-policies`
- `agent-browser://providers`
- `agent-browser://challenges`

Suggested tools:

- `profile_create`
- `profile_list`
- `profile_update`
- `browser_launch`
- `browser_attach`
- `browser_restart`
- `session_create`
- `session_release`
- `tab_list`
- `tab_open`
- `tab_focus`
- `tab_close`
- `navigate`
- `click`
- `fill`
- `press`
- `snapshot`
- `screenshot`
- `monitor_create`
- `monitor_pause`
- `monitor_resume`
- `incident_list`
- `incident_get`
- `incident_activity`
- `incident_acknowledge`
- `incident_resolve`
- `auth_flow_start`
- `auth_flow_status`
- `challenge_status`
- `challenge_approve`
- `site_policy_get`
- `site_policy_update`

Avoid workflow-shaped MCP tools as the default surface. Domain tools may exist
when they provide vocabulary, guardrails, or efficiency, but primitives should
remain available.

## API Surface

The service API should support:

- CRUD for profiles, sessions, monitors, policies, and providers
- browser lifecycle commands
- tab lifecycle commands
- action commands
- job submission and cancellation
- incident listing, detail, handling, and canonical activity timelines
- event streaming through WebSocket or server-sent events
- view stream discovery
- challenge and approval workflows
- audit log access

Every long-running command should return a job ID or explicit completion
signal. Clients should not infer completion from idle state.

Incident activity should be a first-class service-owned read model rather than
client-side reconstruction. The canonical shape for MCP and HTTP clients is:

```json
{
  "incident": {
    "id": "browser-1",
    "label": "browser-1",
    "latestKind": "service_job_timeout",
    "latestTimestamp": "2026-04-22T00:03:00Z"
  },
  "activity": [
    {
      "id": "event-1",
      "source": "event",
      "eventId": "event-1",
      "timestamp": "2026-04-22T00:00:00Z",
      "kind": "browser_health_changed",
      "title": "Browser health changed",
      "message": "Browser browser-1 health changed from Ready to ProcessExited",
      "browserId": "browser-1",
      "details": null
    },
    {
      "id": "job-1",
      "source": "job",
      "jobId": "job-1",
      "timestamp": "2026-04-22T00:01:00Z",
      "kind": "service_job_timeout",
      "title": "Service job timed out",
      "message": "Timed out after 30000 ms"
    }
  ],
  "count": 2
}
```

Clients should preserve `source`, `eventId`, and `jobId` when displaying or
linking activity items. Older incident records may produce `metadata` source
items for acknowledgement or resolution metadata that predate retained
handling events.

## Implementation Phases

### Phase 0: Service Contract

- define durable entities
- define event model
- define queue semantics
- define lifecycle states
- define service API shape
- define MCP resource and tool shape
- define site policy schema

### Phase 1: Single-Host Service Manager

- implement process supervisor
- add durable state store
- expose service health
- route commands through worker queue
- publish browser health events
- improve crash detection and reconnect behavior

### Phase 2: Sessions And Tabs

- add leases
- support shared and exclusive tabs
- track tab lifecycle and ownership
- cache latest snapshots
- reconcile service state with CDP target discovery
- expose state subscriptions

### Phase 3: Access Policy Engine

- load built-in and user site policies
- select browser host and interaction mode per site
- apply rate limits and jitter
- add site observations
- add challenge detection events
- add policy explain output for dashboard and logs

### Phase 4: Authentication Providers

- add provider registry
- add browser credential and profile-based auth hooks
- add external 2FA provider interface
- add email and SMS code retrieval adapters
- add manual approval checkpoints
- add login validation probes

### Phase 5: Remote Headed Browser Viewing

- define `BrowserHost`, `ViewStreamProvider`, and `ControlInputProvider`
- expose view stream discovery in the API
- add dashboard live view mode
- prototype Docker headed Chrome with noVNC or WebRTC
- add human takeover leases

### Phase 6: Challenge Handling

- add `Challenge` entity
- add challenge detector interface
- add provider interface for visual reasoning or captcha solving
- add policy gates and audit records
- add dashboard approval and manual handoff

### Phase 7: System Service

- add systemd, launchd, and Windows service support
- add remote API authentication
- add service configuration management
- add multi-host registry
- add quotas and cleanup policies
- add profile backup and migration

## First Recommended Slice

Start with Phase 0 and a small part of Phase 1.

The first implementation slice should deliver:

- `SitePolicy` schema and config loading
- `BrowserHost` and `ViewStreamProvider` enum definitions
- service state model for browsers, sessions, tabs, jobs, and health
- worker queue ownership of browser-mutating commands
- health endpoint or status command that reports worker and browser state
- durable note in the dashboard or API plan showing how live browser viewing
  will attach later

This slice does not need to solve Docker headed Chrome, captcha solving, or
password manager integration yet. It should create the contracts that make
those features clean additions instead of special cases.

## Open Questions

- Which durable store should back the service state in the first version:
  JSON files, SQLite, or another embedded database?
- Should Docker headed Chrome be managed directly by agent-browser, or should
  it run through a provider adapter that can also target remote hosts?
- Which view stream provider should be the first prototype: noVNC, WebRTC over
  a virtual display, or CDP screencast?
- How should policy distinguish between site access reliability, authorized
  automation, and actions that require explicit human approval?
- Which external provider interface should be implemented first: SMS, email,
  TOTP, captcha, or generic intelligence provider?
