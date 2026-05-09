# agent-browser

Browser automation CLI for AI agents. Fast native Rust CLI.

## Installation

### GitHub Release Binary (recommended)

Download the native Rust binary from this fork's GitHub releases. Pick the asset
for your platform from the latest release, then put it on your PATH.

```bash
VERSION=v0.26.1
curl -L -o ~/.local/bin/agent-browser \
  https://github.com/CochranResearchGroup/agent-browser/releases/download/$VERSION/agent-browser-linux-x64
chmod +x ~/.local/bin/agent-browser
agent-browser install  # Download Chrome from Chrome for Testing (first time only)
```

npm, Homebrew, and Cargo are not authoritative release channels for this fork.
Use GitHub release binaries or build from source.

### From Source

```bash
git clone https://github.com/CochranResearchGroup/agent-browser
cd agent-browser
pnpm install
pnpm build
pnpm build:native   # Requires Rust (https://rustup.rs)
pnpm link --global  # Makes agent-browser available globally
agent-browser install
```

### Linux Dependencies

On Linux, install system dependencies:

```bash
agent-browser install --with-deps
```

### Maintainer Release Validation

Before creating a release, maintainers can validate the release workflow without side effects by manually running the `Release` GitHub Actions workflow with `dry_run` set to `true`. The dry run builds all platform binaries, verifies the expected artifacts, and skips GitHub release creation.

### Updating

Upgrade to the latest version:

```bash
agent-browser upgrade
```

For this fork, prefer replacing the binary from the latest GitHub release. The
legacy `upgrade` command may still know about upstream package-manager installs,
but those channels are not authoritative here.

### Requirements

- **Chrome** - Run `agent-browser install` to download Chrome from [Chrome for Testing](https://developer.chrome.com/blog/chrome-for-testing/) (Google's official automation channel). Existing Chrome, Brave, Playwright, and Puppeteer installations are detected automatically. No Playwright or Node.js required for the daemon.
- **Rust** - Only needed when building from source (see From Source above).

## Quick Start

```bash
agent-browser open example.com
agent-browser snapshot                    # Get accessibility tree with refs
agent-browser click @e2                   # Click by ref from snapshot
agent-browser fill @e3 "test@example.com" # Fill by ref
agent-browser get text @e1                # Get text by ref
agent-browser screenshot page.png
agent-browser close
```

### Traditional Selectors (also supported)

```bash
agent-browser click "#submit"
agent-browser fill "#email" "test@example.com"
agent-browser find role button click --name "Submit"
```

## Commands

### Core Commands

```bash
agent-browser open <url>              # Navigate to URL (aliases: goto, navigate)
agent-browser click <sel>             # Click element (--new-tab to open in new tab)
agent-browser dblclick <sel>          # Double-click element
agent-browser focus <sel>             # Focus element
agent-browser type <sel> <text>       # Type into element
agent-browser fill <sel> <text>       # Clear and fill
agent-browser press <key>             # Press key (Enter, Tab, Control+a) (alias: key)
agent-browser keyboard type <text>    # Type with real keystrokes (no selector, current focus)
agent-browser keyboard inserttext <text>  # Insert text without key events (no selector)
agent-browser keydown <key>           # Hold key down
agent-browser keyup <key>             # Release key
agent-browser hover <sel>             # Hover element
agent-browser select <sel> <val>      # Select dropdown option
agent-browser check <sel>             # Check checkbox
agent-browser uncheck <sel>           # Uncheck checkbox
agent-browser scroll <dir> [px]       # Scroll (up/down/left/right, --selector <sel>)
agent-browser scrollintoview <sel>    # Scroll element into view (alias: scrollinto)
agent-browser drag <src> <tgt>        # Drag and drop
agent-browser upload <sel> <files>    # Upload files
agent-browser screenshot [path]       # Take screenshot (--full for full page, saves to a temporary directory if no path)
agent-browser screenshot --annotate   # Annotated screenshot with numbered element labels
agent-browser screenshot --screenshot-dir ./shots    # Save to custom directory
agent-browser screenshot --screenshot-format jpeg --screenshot-quality 80
agent-browser pdf <path>              # Save as PDF
agent-browser snapshot                # Accessibility tree with refs (best for AI)
agent-browser eval <js>               # Run JavaScript (-b for base64, --stdin for piped input)
agent-browser connect <port>          # Connect to browser via CDP
agent-browser stream enable [--port <port>]  # Start runtime WebSocket streaming
agent-browser stream status           # Show runtime streaming state and bound port
agent-browser stream disable          # Stop runtime WebSocket streaming
agent-browser service status          # Show service control-plane and configured service state
agent-browser service watch           # Poll service health until interrupted
agent-browser service reconcile       # Refresh persisted browser health records
agent-browser service profiles        # Show retained profiles and allocation state
agent-browser service sessions        # Show retained service session records
agent-browser service browsers        # Show retained browser health records
agent-browser service tabs            # Show retained service tab records
agent-browser service monitors        # Show retained service monitor records (--summary, --failed, --state)
agent-browser service monitors run-due # Run due active service monitors now
agent-browser service monitors pause <id> # Pause a noisy monitor
agent-browser service monitors resume <id> # Resume monitor checks
agent-browser service monitors reset <id> # Clear reviewed monitor failures
agent-browser service monitors triage <id> # Acknowledge monitor incident and clear reviewed failures
agent-browser service site-policies   # Show configured service site-policy records
agent-browser service providers       # Show configured service provider records
agent-browser service challenges      # Show retained service challenge records
agent-browser service cancel <job-id> # Cancel a queued, waiting, or running service control job
agent-browser service acknowledge <incident-id> # Mark a retained incident acknowledged
agent-browser service resolve <incident-id>     # Mark a retained incident resolved
agent-browser service activity <incident-id>    # Show a retained incident timeline
agent-browser service trace                     # Show related service trace records
agent-browser service jobs            # Show recent service control jobs
agent-browser service incidents       # Show grouped retained service incidents
agent-browser service remedies        # Show active browser, monitor, and OS remedy groups
agent-browser service remedies apply --escalation monitor_attention # Apply active monitor remedies
agent-browser service remedies apply --escalation browser_degraded # Enable retry for degraded browsers after review
agent-browser service remedies apply --escalation os_degraded_possible # Retry faulted browsers after host inspection
agent-browser service events          # Show recent service events
agent-browser mcp serve               # Run the MCP stdio server
agent-browser mcp resources           # List read-only service resources for MCP adapters
agent-browser mcp read agent-browser://incidents
agent-browser mcp read agent-browser://profiles
agent-browser mcp read agent-browser://sessions
agent-browser mcp read agent-browser://browsers
agent-browser mcp read agent-browser://tabs
agent-browser mcp read agent-browser://monitors
agent-browser mcp read agent-browser://site-policies
agent-browser mcp read agent-browser://providers
agent-browser mcp read agent-browser://challenges
agent-browser mcp read agent-browser://jobs
agent-browser mcp read agent-browser://events
agent-browser close                   # Close browser (aliases: quit, exit)
agent-browser close --all             # Close all active sessions
agent-browser chat "<instruction>"    # AI chat: natural language browser control (single-shot)
agent-browser chat                    # AI chat: interactive REPL mode
```

Service mode is the persistent control plane for long-lived automation. It keeps profile, session, browser, tab, monitor, job, incident, event, site-policy, provider, and challenge state aligned across CLI commands, the HTTP API, MCP resources/tools, and the dashboard. Agents should include `serviceName`, `agentName`, and `taskName` when available so multi-service work remains traceable. The normal service request is identity-first: ask for a tab or browser action, target site or login identity, and the owning service, agent, and task. agent-browser selects or reuses the managed profile and browser, serializes CDP work through the queue, and records the state needed for debugging. Service profile records and profile allocation rows include `targetReadiness`, a no-launch readiness view for target services. Google targets without authenticated evidence report `needs_manual_seeding` and recommend detached `runtime login` before attachable automation. Once a managed profile lists the target in `authenticatedServiceIds`, readiness changes to `seeded_unknown_freshness` and access-plan no longer treats first-login seeding as a required manual action. Access-plan responses also include `monitorFindings` and `decision.monitorAttentionRequired` when an active `profile_readiness` monitor is faulted for the requested target identity. Use an explicit managed runtime profile when you know where the needed login state lives; use `--profile <path>` only when bringing an external profile is part of the contract.

### Get Info

```bash
agent-browser get text <sel>          # Get text content
agent-browser get html <sel>          # Get innerHTML
agent-browser get value <sel>         # Get input value
agent-browser get attr <sel> <attr>   # Get attribute
agent-browser get title               # Get page title
agent-browser get url                 # Get current URL
agent-browser get cdp-url             # Get CDP WebSocket URL (for DevTools, debugging)
agent-browser get count <sel>         # Count matching elements
agent-browser get box <sel>           # Get bounding box
agent-browser get styles <sel>        # Get computed styles
```

### Check State

```bash
agent-browser is visible <sel>        # Check if visible
agent-browser is enabled <sel>        # Check if enabled
agent-browser is checked <sel>        # Check if checked
```

### Find Elements (Semantic Locators)

```bash
agent-browser find role <role> <action> [value]       # By ARIA role
agent-browser find text <text> <action>               # By text content
agent-browser find label <label> <action> [value]     # By label
agent-browser find placeholder <ph> <action> [value]  # By placeholder
agent-browser find alt <text> <action>                # By alt text
agent-browser find title <text> <action>              # By title attr
agent-browser find testid <id> <action> [value]       # By data-testid
agent-browser find first <sel> <action> [value]       # First match
agent-browser find last <sel> <action> [value]        # Last match
agent-browser find nth <n> <sel> <action> [value]     # Nth match
```

**Actions:** `click`, `fill`, `type`, `hover`, `focus`, `check`, `uncheck`, `text`

**Options:** `--name <name>` (filter role by accessible name), `--exact` (require exact text match)

**Examples:**

```bash
agent-browser find role button click --name "Submit"
agent-browser find text "Sign In" click
agent-browser find label "Email" fill "test@test.com"
agent-browser find first ".item" click
agent-browser find nth 2 "a" text
```

### Wait

```bash
agent-browser wait <selector>         # Wait for element to be visible
agent-browser wait <ms>               # Wait for time (milliseconds)
agent-browser wait --text "Welcome"   # Wait for text to appear (substring match)
agent-browser wait --url "**/dash"    # Wait for URL pattern
agent-browser wait --load networkidle # Wait for load state
agent-browser wait --fn "window.ready === true"  # Wait for JS condition

# Wait for text/element to disappear
agent-browser wait --fn "!document.body.innerText.includes('Loading...')"
agent-browser wait "#spinner" --state hidden
```

**Load states:** `load`, `domcontentloaded`, `networkidle`

### Batch Execution

Execute multiple commands in a single invocation. Commands can be passed as
quoted arguments or piped as JSON via stdin. This avoids per-command process
startup overhead when running multi-step workflows.

```bash
# Argument mode: each quoted argument is a full command
agent-browser batch "open https://example.com" "snapshot -i" "screenshot"

# With --bail to stop on first error
agent-browser batch --bail "open https://example.com" "click @e1" "screenshot"

# Stdin mode: pipe commands as JSON
echo '[
  ["open", "https://example.com"],
  ["snapshot", "-i"],
  ["click", "@e1"],
  ["screenshot", "result.png"]
]' | agent-browser batch --json
```

### Clipboard

```bash
agent-browser clipboard read                      # Read text from clipboard
agent-browser clipboard write "Hello, World!"     # Write text to clipboard
agent-browser clipboard copy                      # Copy current selection (Ctrl+C)
agent-browser clipboard paste                     # Paste from clipboard (Ctrl+V)
```

### Mouse Control

```bash
agent-browser mouse move <x> <y>      # Move mouse
agent-browser mouse down [button]     # Press button (left/right/middle)
agent-browser mouse up [button]       # Release button
agent-browser mouse wheel <dy> [dx]   # Scroll wheel
```

### Browser Settings

```bash
agent-browser set viewport <w> <h> [scale]  # Set viewport size (scale for retina, e.g. 2)
agent-browser set device <name>       # Emulate device ("iPhone 14")
agent-browser set geo <lat> <lng>     # Set geolocation
agent-browser set offline [on|off]    # Toggle offline mode
agent-browser set headers <json>      # Extra HTTP headers
agent-browser set credentials <u> <p> # HTTP basic auth
agent-browser set media [dark|light]  # Emulate color scheme
```

### Cookies & Storage

```bash
agent-browser cookies                 # Get all cookies
agent-browser cookies set <name> <val> # Set cookie
agent-browser cookies clear           # Clear cookies

agent-browser storage local           # Get all localStorage
agent-browser storage local <key>     # Get specific key
agent-browser storage local set <k> <v>  # Set value
agent-browser storage local clear     # Clear all

agent-browser storage session         # Same for sessionStorage
```

### Network

```bash
agent-browser network route <url>              # Intercept requests
agent-browser network route <url> --abort      # Block requests
agent-browser network route <url> --body <json>  # Mock response
agent-browser network unroute [url]            # Remove routes
agent-browser network requests                 # View tracked requests
agent-browser network requests --filter api    # Filter requests
agent-browser network requests --type xhr,fetch  # Filter by resource type
agent-browser network requests --method POST   # Filter by HTTP method
agent-browser network requests --status 2xx    # Filter by status (200, 2xx, 400-499)
agent-browser network request <requestId>      # View full request/response detail
agent-browser network har start                # Start HAR recording
agent-browser network har stop [output.har]    # Stop and save HAR (temp path if omitted)
```

### Tabs & Windows

```bash
agent-browser tab                     # List tabs
agent-browser tab new [url]           # New tab (optionally with URL)
agent-browser tab <n>                 # Switch to tab n
agent-browser tab close [n]           # Close tab
agent-browser window new              # New window
```

### Frames

```bash
agent-browser frame <sel>             # Switch to iframe
agent-browser frame main              # Back to main frame
```

### Dialogs

```bash
agent-browser dialog accept [text]    # Accept (with optional prompt text)
agent-browser dialog dismiss          # Dismiss
agent-browser dialog status           # Check if a dialog is currently open
```

By default, `alert` and `beforeunload` dialogs are automatically accepted so they never block the agent. `confirm` and `prompt` dialogs still require explicit handling. Use `--no-auto-dialog` (or `AGENT_BROWSER_NO_AUTO_DIALOG=1`) to disable automatic handling.

When a JavaScript dialog is pending, all command responses include a `warning` field with the dialog type and message.

### Diff

```bash
agent-browser diff snapshot                              # Compare current vs last snapshot
agent-browser diff snapshot --baseline before.txt        # Compare current vs saved snapshot file
agent-browser diff snapshot --selector "#main" --compact # Scoped snapshot diff
agent-browser diff screenshot --baseline before.png      # Visual pixel diff against baseline
agent-browser diff screenshot --baseline b.png -o d.png  # Save diff image to custom path
agent-browser diff screenshot --baseline b.png -t 0.2    # Adjust color threshold (0-1)
agent-browser diff url https://v1.com https://v2.com     # Compare two URLs (snapshot diff)
agent-browser diff url https://v1.com https://v2.com --screenshot  # Also visual diff
agent-browser diff url https://v1.com https://v2.com --wait-until networkidle  # Custom wait strategy
agent-browser diff url https://v1.com https://v2.com --selector "#main"  # Scope to element
```

### Debug

```bash
agent-browser trace start [path]      # Start recording trace
agent-browser trace stop [path]       # Stop and save trace
agent-browser profiler start          # Start Chrome DevTools profiling
agent-browser profiler stop [path]    # Stop and save profile (.json)
agent-browser console                 # View console messages (log, error, warn, info)
agent-browser console --json          # JSON output with raw CDP args for programmatic access
agent-browser console --clear         # Clear console
agent-browser errors                  # View page errors (uncaught JavaScript exceptions)
agent-browser errors --clear          # Clear errors
agent-browser highlight <sel>         # Highlight element
agent-browser inspect                 # Open Chrome DevTools for the active page
agent-browser state save <path>       # Save auth state
agent-browser state load <path>       # Load auth state
agent-browser state list              # List saved state files
agent-browser state show <file>       # Show state summary
agent-browser state rename <old> <new> # Rename state file
agent-browser state clear [name]      # Clear states for session
agent-browser state clear --all       # Clear all saved states
agent-browser state clean --older-than <days>  # Delete old states
```

### Navigation

```bash
agent-browser back                    # Go back
agent-browser forward                 # Go forward
agent-browser reload                  # Reload page
```

### Setup

```bash
agent-browser install                 # Download Chrome from Chrome for Testing (Google's official automation channel)
agent-browser install --with-deps     # Also install system deps (Linux)
agent-browser upgrade                 # Upgrade agent-browser to the latest version
```

## Authentication

agent-browser provides multiple ways to persist login sessions so you don't re-authenticate every run.

### Quick summary

| Approach | Best for | Flag / Env |
|----------|----------|------------|
| **Default runtime profile** | Stable browser state in `~/.agent-browser/runtime-profiles/default/user-data` across runs | Automatic |
| **Named runtime profile** | Isolated persistent browser state for a specific account or workflow | `--runtime-profile <name>` / `AGENT_BROWSER_RUNTIME_PROFILE` |
| **Persistent profile** | Full browser state in a custom directory across restarts | `--profile <path>` / `AGENT_BROWSER_PROFILE` |
| **Session persistence** | Auto-save/restore cookies + localStorage by name | `--session-name <name>` / `AGENT_BROWSER_SESSION_NAME` |
| **Import from your browser** | Grab auth from a Chrome session you already logged into | `--auto-connect` + `state save` |
| **State file** | Load a previously saved state JSON on launch | `--state <path>` / `AGENT_BROWSER_STATE` |
| **Auth vault** | Store credentials locally (encrypted), login by name | `auth save` / `auth login` |

### Import auth from your browser

If you are already logged in to a site in Chrome, you can grab that auth state and reuse it:

```bash
# 1. Launch Chrome with remote debugging enabled
#    macOS:
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --remote-debugging-port=9222
#    Or use --auto-connect to discover an already-running Chrome

# 2. Connect and save the authenticated state
agent-browser --auto-connect state save ./my-auth.json

# 3. Use the saved auth in future sessions
agent-browser --state ./my-auth.json open https://app.example.com/dashboard

# 4. Or use --session-name for automatic persistence
agent-browser --session-name myapp state load ./my-auth.json
# From now on, --session-name myapp auto-saves/restores this state
```

> **Security notes:**
> - `--remote-debugging-port` exposes full browser control on localhost. Any local process can connect. Only use on trusted machines and close Chrome when done.
> - State files contain session tokens in plaintext. Add them to `.gitignore` and delete when no longer needed. For encryption at rest, set `AGENT_BROWSER_ENCRYPTION_KEY` (see [State Encryption](#state-encryption)).

For full details on login flows, OAuth, 2FA, cookie-based auth, and the auth vault, see the [Authentication](docs/src/app/sessions/page.mdx) docs.

## Sessions

Run multiple isolated browser instances:

```bash
# Different sessions
agent-browser --session agent1 open site-a.com
agent-browser --session agent2 open site-b.com

# Or via environment variable
AGENT_BROWSER_SESSION=agent1 agent-browser click "#btn"

# List active sessions
agent-browser session list
# Output:
# Active sessions:
# -> default
#    agent1

# Show current session
agent-browser session
```

Each session has its own:

- Browser instance
- Cookies and storage
- Navigation history
- Authentication state

## Configured Runtime Profiles

The config file can now define managed runtime profiles directly. This is the
foundational model for agent-browser to become an agent-facing browser tool with
stable per-profile defaults and service-specific login hints.

```json
{
  "defaultRuntimeProfile": "work",
  "runtimeProfiles": {
    "work": {
      "userDataDir": "~/.agent-browser/runtime-profiles/work/user-data",
      "launch": {
        "headed": true,
        "leaveOpen": true,
        "proxy": "http://proxy.internal:8080"
      },
      "auth": {
        "sessionName": "work-session",
        "manualLoginPreferred": true
      },
      "services": {
        "google": {
          "manualLoginPreferred": true
        }
      },
      "preferences": {
        "defaultViewport": "960x640"
      }
    }
  },
  "service": {
    "profiles": {
      "work": {
        "name": "Work",
        "allocation": "per_service",
        "keyring": "basic_password_store",
        "targetServiceIds": ["google", "acs"],
        "authenticatedServiceIds": ["google"],
        "sharedServiceIds": ["JournalDownloader"],
        "manualLoginPreferred": true
      }
    },
    "sessions": {
      "journal-session": {
        "serviceName": "JournalDownloader",
        "agentName": "article-probe-agent",
        "taskName": "probeACSwebsite",
        "profileId": "work",
        "lease": "exclusive",
        "cleanup": "close_tabs"
      }
    }
  }
}
```

Today, agent-browser applies the selected runtime profile's `userDataDir`,
launch settings, auth session name, and service login hints. A service with
`manualLoginPreferred` emits an advisory warning when navigation targets known
login hosts for that service, so agents can switch to detached `runtime login`.
Use the attachable manual-login flow only for sites where DevTools during login
is accepted. Set `launch.leaveOpen` or pass `--leave-open` when you want
`close` to detach from a managed runtime-profile browser instead of shutting it
down. Set `preferences.defaultViewport` to a `WIDTHxHEIGHT` value, such as
`960x640`, when a runtime profile should resize the browser content area after
launch and before the requested command runs.

The `service.profiles` and `service.sessions` maps define service control-plane
metadata for profile allocation, keyring posture, caller ownership, profile
binding, lease state, and cleanup policy. These records are exposed through
service status, MCP resources, and the HTTP service APIs. Explicit
`--runtime-profile` and `--profile` values still win. When a launch command omits
both, `serviceName` plus `targetServiceId`, `targetService`, `targetServiceIds`,
`targetServices`, `siteId`, `siteIds`, `loginId`, or `loginIds` lets
agent-browser choose a persisted service profile. The
selector first prefers `authenticatedServiceIds` matches, then
`targetServiceIds` matches, then the caller `sharedServiceIds` match. Launches
that select a runtime profile or custom profile path now bind the active browser
record to a service profile. Session records expose `profileSelectionReason`
as `authenticated_target`, `target_match`, `service_allow_list`, or
`explicit_profile`, and launch events mirror that value in
`details.profileSelectionReason`. They also expose `profileLeaseDisposition`
as `new_browser`, `reused_browser`, or `active_lease_conflict` plus
`profileLeaseConflictSessionIds` when another exclusive session already holds
the same profile. Service-scoped launches reject active exclusive profile
conflicts by default before browser start; set `profileLeasePolicy: "wait"` and
`profileLeaseWaitTimeoutMs` to keep the job queued while polling for release,
leaving the worker available for other commands. Same-session retained browser
reuse remains allowed. MCP typed browser tools accept the same target profile hints,
so clients can use `browser_navigate` or other typed tools
without falling back to `browser_command`.
Run `pnpm test:service-profile-target-mcp-live` to validate the live typed MCP
target-hint profile selection path.
Run `pnpm test:service-request-live` to validate that HTTP
`/api/service/request` and MCP `service_request` queue intent-based browser
actions with profile hints and retained job metadata.
Run `pnpm test:service-profile-lease-wait-live` during manual full-CI or
release-gating checks that touch profile selection, profile lease waiting,
service request, trace summary, dashboard trace, or service observability
client behavior. It intentionally stays out of ordinary CI.
Run `pnpm test:service-status-no-launch` to validate that service status remains
read-only when launch defaults such as `AGENT_BROWSER_ARGS` are configured.
Run `pnpm test:service-remedies-cli-no-launch` to validate that the CLI remedy
ladder renders affected browser IDs and batch apply commands from persisted
incidents without launching Chrome.
Run `pnpm test:service-remedies-json-no-launch` to validate that the JSON
remedy ladder keeps the same affected browser IDs, incident IDs, and batch
apply commands without launching Chrome.
Run `pnpm test:service-remedies-apply-json-no-launch` to validate that
`service remedies apply --escalation monitor_attention`,
`--escalation browser_degraded`, and `--escalation os_degraded_possible` return
apply responses and update persisted monitor or browser state without launching
Chrome.
Run `pnpm test:service-contracts-no-launch` to validate that HTTP
`/api/service/contracts` returns compatibility metadata without launching or
recording a browser, and that `getServiceContracts()` exposes the profile
lookup/readiness client-helper metadata to software clients.
Run `pnpm test:service-profile-lookup-no-launch` to validate that HTTP
`/api/service/profiles/lookup` selects an authenticated target profile over a
target-only profile from seeded temporary service state without launching a
browser and reports the matched profile field and identity. The same smoke
calls `lookupServiceProfile()` against the live stream server so the
software-client helper is covered end to end.
Run `pnpm test:service-profile-sources-no-launch` to validate that profile
collections, profile lookup, and access-plan responses report config and
persisted-state profile provenance across HTTP, MCP, and the service client
without launching a browser.
Run `pnpm test:mcp-read-no-launch` to validate that MCP resource reads remain
read-only under the same launch defaults.
Fast CI runs the no-launch service contract metadata smoke, the no-launch
profile-source smoke, the no-launch site-policy source smoke, and the
no-launch HTTP and MCP incident-summary smokes after the Rust suite, covering
service contract metadata, effective profile and site-policy provenance, and
grouped service incident remedies without starting Chrome. Service request action changes must keep `SERVICE_REQUEST_ACTIONS`,
`docs/dev/contracts/service-request.v1.schema.json`, MCP `service_request`,
HTTP `/api/service/request`, and generated `@agent-browser/client` helpers
aligned; the fast parity, client, and Rust gates include no-launch guards for
that invariant. Run both incident-summary smokes before changing incident
summary grouping or filters; together they guard HTTP `summary=true` and MCP
`service_incidents` with `summary: true` across state, severity, escalation,
handling-state, browser, profile, session, service, agent, task, and since
filters.
Run `pnpm test:service-shutdown-faulted-live` to validate that a force-kill
failure leaves the persisted service browser record `faulted` and escalates the
incident as possible OS degradation.
When commands include `serviceName`, `agentName`, or `taskName`, the active
session record also captures that caller context for traceability. Profile
selection should prefer a profile with credentials and usable auth state for
the target site or identity provider, not merely the profile owned by the
calling service. Use `targetServiceIds` to record intended target services such
as `google`, `microsoft`, or `acs`, and `authenticatedServiceIds` for target
services currently believed to have usable login state. Request payloads can
use `siteId` or `loginId` as caller-facing aliases when that matches the
operator's vocabulary better than target service. Config mutations
enforce profile/session ownership policy: `caller_supplied` profiles must
include `userDataDir`, `per_service` profiles may list at most one
`sharedServiceIds` entry, session `profileId` must reference a persisted
profile, and omitted session `owner` values are inferred from `agentName` or
`serviceName`.

Software clients should normally request the login identity they need and let
agent-browser choose the matching profile. Register the profile with its target
identity and the service allowed to use it:

```ts
await registerServiceLoginProfile({
  baseUrl: `http://127.0.0.1:${streamPort}`,
  id: 'journal-acs',
  serviceName: 'JournalDownloader',
  loginId: 'acs',
});
```

When a client has explicit auth evidence from a bounded probe, it can attach
freshness metadata to the same registration call. `readinessState`,
`readinessEvidence`, `lastVerifiedAt`, and `freshnessExpiresAt` generate
`targetReadiness` rows, and explicit `targetReadiness` rows win for matching
targets. The service preserves explicit `fresh`, `stale`, and
`blocked_by_attached_devtools` rows through derived readiness refreshes.

Then request the tab by `loginId`:

```ts
await requestServiceTab({
  baseUrl: `http://127.0.0.1:${streamPort}`,
  serviceName: 'JournalDownloader',
  agentName: 'article-probe-agent',
  taskName: 'probeACSwebsite',
  loginId: 'acs',
  url: 'https://example.com',
});
```

The selector prefers `authenticatedServiceIds`, then `targetServiceIds`, then
`sharedServiceIds`. The retained session record reports the chosen path in
`profileSelectionReason`, so operators can tell whether agent-browser selected
the profile by authenticated target state, target scope, caller service
fallback, or explicit override. Inspect `profileLeaseDisposition` to tell
whether the selected profile opened a new browser, reused a retained session
browser, or hit another exclusive profile lease. Active exclusive conflicts are
rejected before a service-scoped launch starts another browser unless
`profileLeasePolicy: "wait"` is supplied with a bounded
`profileLeaseWaitTimeoutMs`; waiting jobs stay queued and do not occupy the
running worker. Pass `profile` or `runtimeProfile` only for override
workflows where the caller intentionally takes direct responsibility for the
browser identity.

You can register a runtime profile into user config explicitly:

```bash
agent-browser runtime create work --set-default
```

## Default runtime profile

If you do not pass `--profile` or `--runtime-profile`, agent-browser launches Chrome with a stable
user-data-dir at `~/.agent-browser/runtime-profiles/default/user-data`.

If the default runtime profile is locked by a live browser PID, do not treat a
fresh isolated profile as the generic safe fallback. agent-browser is designed
to own session and job management so operators do not have to coordinate which
browser is busy. For authenticated work, inspect `agent-browser service status`,
`agent-browser runtime status`, or the dashboard service view, then reuse the
managed runtime profile through the service/session control plane or attach to
the intended browser. Switch to a new isolated profile only for explicitly
unauthenticated throwaway QA, or when the operator asked for a separate browser
identity.
When a selected managed runtime profile already has a live agent-browser browser
with a DevTools port, normal launch commands automatically reuse that browser
through the session control plane instead of trying to start a second Chrome on
the locked profile.

For Google and similar SSO flows, the preferred bootstrap is a detached manual login first:

```bash
# First run: open a detached manual-login browser
agent-browser runtime login https://accounts.google.com

# Inspect the runtime profile before automation touches it
agent-browser runtime list
agent-browser runtime status

# Later runs reuse the same browser state automatically
agent-browser open https://gmail.com
```

If you need to bind automation to the same live browser instead of closing it first, opt into an attachable manual browser:

```bash
agent-browser runtime login https://example.com --attachable
agent-browser runtime attach
```

Do not use `--attachable` for the initial sign-in on Google, Gmail, or similar SSO flows. Google can reject sign-in when DevTools is present during the login ceremony. Sign in with detached `runtime login`, close Chrome, then relaunch the same runtime profile with `--attachable` for automation:

```bash
agent-browser --runtime-profile google-login runtime login https://accounts.google.com
# Sign in manually, then close Chrome
agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
agent-browser --runtime-profile google-login runtime attach
agent-browser --runtime-profile google-login get title
```

This default profile keeps:

- Cookies and localStorage
- IndexedDB data
- Service workers
- Browser cache
- Signed-in browser sessions

If you need a different persistent identity, pass `--runtime-profile <name>` or
`--profile <path>` explicitly. Do this for separate account lanes, not merely
because another job is already using the browser.

## Named runtime profiles

Use a named runtime profile when the operator intentionally wants a separate
persistent browser/account lane. Do not create one merely because another
agent-browser job is active; for service-mode work, request the target site or
login identity and let agent-browser select, queue, or reuse the managed
profile.

```bash
# Create and register a dedicated runtime profile
agent-browser runtime create work --set-default

# Manual login for a dedicated runtime profile
agent-browser --runtime-profile work runtime login https://app.example.com/login

# Or keep DevTools available for a later live attach
agent-browser --runtime-profile work runtime login https://app.example.com/login --attachable

agent-browser runtime attach work

# Leave the managed runtime-profile browser running when you close the session
agent-browser --runtime-profile work --leave-open open https://app.example.com

# Later automation reuses the same profile
agent-browser --runtime-profile work open https://app.example.com

# Inspect the live runtime state
agent-browser runtime list
agent-browser --runtime-profile work runtime status
```

This resolves to a persistent profile directory under `~/.agent-browser/runtime-profiles/<name>/user-data`, unless `runtimeProfiles.<name>.userDataDir` overrides it in config. Use `agent-browser runtime list` to inspect the merged view from config plus on-disk managed profiles.

Use this for ordinary authenticated sites, multi-account setups, and headed/manual bootstrap flows.

> **Important:** If you want Chrome password-manager behavior or Google sign-in, prefer `agent-browser runtime login ...` first, close Chrome after sign-in, then reuse that runtime profile for automation.

## Persistent Profiles

For a persistent custom profile directory that stores state across browser restarts, pass a path to `--profile`:

```bash
# Use a persistent profile directory
agent-browser --profile ~/.myapp-profile open myapp.com

# Login once, then reuse the authenticated session
agent-browser --profile ~/.myapp-profile open myapp.com/dashboard

# Or via environment variable
AGENT_BROWSER_PROFILE=~/.myapp-profile agent-browser open myapp.com
```

The profile directory stores:

- Cookies and localStorage
- IndexedDB data
- Service workers
- Browser cache
- Login sessions

**Tip**: Use different profile paths for different projects to keep their browser state isolated.

## Session Persistence

Alternatively, use `--session-name` to automatically save and restore cookies and localStorage across browser restarts:

```bash
# Auto-save/load state for "twitter" session
agent-browser --session-name twitter open twitter.com

# Login once, then state persists automatically
# State files stored in ~/.agent-browser/sessions/

# Or via environment variable
export AGENT_BROWSER_SESSION_NAME=twitter
agent-browser open twitter.com
```

## Inspect Runtime State

Inspect the live browser process or tab-level CDP identifiers:

```bash
agent-browser get browser-pid
agent-browser tab list
agent-browser tab list --verbose
```

`tab list --verbose` includes each tab's `targetId` and `sessionId`, which are useful when debugging daemon, CDP, or tab-tracking issues.

### State Encryption

Encrypt saved session data at rest with AES-256-GCM:

```bash
# Generate key: openssl rand -hex 32
export AGENT_BROWSER_ENCRYPTION_KEY=<64-char-hex-key>

# State files are now encrypted automatically
agent-browser --session-name secure open example.com
```

| Variable                          | Description                                        |
| --------------------------------- | -------------------------------------------------- |
| `AGENT_BROWSER_SESSION_NAME`      | Auto-save/load state persistence name              |
| `AGENT_BROWSER_ENCRYPTION_KEY`    | 64-char hex key for AES-256-GCM encryption         |
| `AGENT_BROWSER_STATE_EXPIRE_DAYS` | Auto-delete states older than N days (default: 30) |

## Security

agent-browser includes security features for safe AI agent deployments. All features are opt-in, so existing workflows are unaffected until you explicitly enable a feature:

- **Authentication Vault** stores credentials locally, always encrypted, and references them by name. The LLM never sees passwords. `auth login` navigates with `load` and then waits for login form selectors to appear. The timeout follows the default action timeout. A key is auto-generated at `~/.agent-browser/.encryption-key` if `AGENT_BROWSER_ENCRYPTION_KEY` is not set: `echo "pass" | agent-browser auth save github --url https://github.com/login --username user --password-stdin` then `agent-browser auth login github`
- **Content Boundary Markers** wrap page output in delimiters so LLMs can distinguish tool output from untrusted content: `--content-boundaries`
- **Domain Allowlist** restricts navigation to trusted domains. Wildcards like `*.example.com` also match the bare domain: `--allowed-domains "example.com,*.example.com"`. Sub-resource requests (scripts, images, fetch) and WebSocket/EventSource connections to non-allowed domains are also blocked. Include any CDN domains your target pages depend on, for example `*.cdn.example.com`.
- **Action Policy** gates destructive actions with a static policy file: `--action-policy ./policy.json`
- **Action Confirmation** requires explicit approval for sensitive action categories: `--confirm-actions eval,download`
- **Output Length Limits** prevent context flooding: `--max-output 50000`

For unattended headed or headless runs that need the real OS credential store, agent-browser can read keychain settings from a dotenv file. Environment variables take precedence, otherwise it loads `AGENT_BROWSER_ENV_FILE`, then `~/.agent-browser/.env` if present.

```bash
cat > ~/.agent-browser/.env <<'EOF'
AGENT_BROWSER_USE_REAL_KEYCHAIN=1
AGENT_BROWSER_KEYCHAIN_PASSWORD='your-login-keychain-password'
EOF

agent-browser open https://example.com
```

On macOS, `AGENT_BROWSER_KEYCHAIN_PASSWORD` unlocks the login keychain before Chrome launches. On Linux, agent-browser uses the password to call `gnome-keyring-daemon --unlock --components=secrets` and passes the exported secret-service environment into Chrome. This is aimed at Ubuntu and other GNOME-keyring setups. If you only need real keychain mode without an unlock step, set `AGENT_BROWSER_USE_REAL_KEYCHAIN=1`.

| Variable                            | Description                              |
| ----------------------------------- | ---------------------------------------- |
| `AGENT_BROWSER_CONTENT_BOUNDARIES`  | Wrap page output in boundary markers     |
| `AGENT_BROWSER_MAX_OUTPUT`          | Max characters for page output           |
| `AGENT_BROWSER_ALLOWED_DOMAINS`     | Comma-separated allowed domain patterns  |
| `AGENT_BROWSER_ACTION_POLICY`       | Path to action policy JSON file          |
| `AGENT_BROWSER_CONFIRM_ACTIONS`     | Action categories requiring confirmation |
| `AGENT_BROWSER_CONFIRM_INTERACTIVE` | Enable interactive confirmation prompts  |
| `AGENT_BROWSER_ENV_FILE`            | Optional dotenv file for agent-browser secrets |
| `AGENT_BROWSER_USE_REAL_KEYCHAIN`   | Use the real OS keychain for Chrome profile launches |
| `AGENT_BROWSER_KEYCHAIN_PASSWORD`   | Password used to unlock the macOS login keychain or Linux GNOME Keyring |

See [Security documentation](https://agent-browser.dev/security) for details.

## Snapshot Options

The `snapshot` command supports filtering to reduce output size:

```bash
agent-browser snapshot                    # Full accessibility tree
agent-browser snapshot -i                 # Interactive elements only (buttons, inputs, links)
agent-browser snapshot -i --urls          # Interactive elements with link URLs
agent-browser snapshot -c                 # Compact (remove empty structural elements)
agent-browser snapshot -d 3               # Limit depth to 3 levels
agent-browser snapshot -s "#main"         # Scope to CSS selector
agent-browser snapshot -i -c -d 5         # Combine options
```

| Option                 | Description                                                             |
| ---------------------- | ----------------------------------------------------------------------- |
| `-i, --interactive`    | Only show interactive elements (buttons, links, inputs)                 |
| `-u, --urls`           | Include href URLs for link elements                                     |
| `-c, --compact`        | Remove empty structural elements                                        |
| `-d, --depth <n>`      | Limit tree depth                                                        |
| `-s, --selector <sel>` | Scope to CSS selector                                                   |

## Annotated Screenshots

The `--annotate` flag overlays numbered labels on interactive elements in the screenshot. Each label `[N]` corresponds to ref `@eN`, so the same refs work for both visual and text-based workflows.

Annotated screenshots are supported on the CDP-backed browser path (Chrome/Lightpanda). The Safari/WebDriver backend does not yet support `--annotate`.

```bash
agent-browser screenshot --annotate
# -> Screenshot saved to /tmp/screenshot-2026-02-17T12-00-00-abc123.png
#    [1] @e1 button "Submit"
#    [2] @e2 link "Home"
#    [3] @e3 textbox "Email"
```

After an annotated screenshot, refs are cached so you can immediately interact with elements:

```bash
agent-browser screenshot --annotate ./page.png
agent-browser click @e2     # Click the "Home" link labeled [2]
```

This is useful for multimodal AI models that can reason about visual layout, unlabeled icon buttons, canvas elements, or visual state that the text accessibility tree cannot capture.

## Options

| Option | Description |
|--------|-------------|
| `--session <name>` | Use isolated session (or `AGENT_BROWSER_SESSION` env) |
| `--session-name <name>` | Auto-save/restore session state (or `AGENT_BROWSER_SESSION_NAME` env) |
| `--runtime-profile <name>` | Managed runtime profile name (or `AGENT_BROWSER_RUNTIME_PROFILE` env) |
| `--profile <path>` | Persistent custom user-data-dir path (or `AGENT_BROWSER_PROFILE` env) |
| `--state <path>` | Load storage state from JSON file (or `AGENT_BROWSER_STATE` env) |
| `--headers <json>` | Set HTTP headers scoped to the URL's origin |
| `--executable-path <path>` | Custom browser executable (or `AGENT_BROWSER_EXECUTABLE_PATH` env) |
| `--extension <path>` | Load browser extension (repeatable; or `AGENT_BROWSER_EXTENSIONS` env) |
| `--args <args>` | Browser launch args, comma or newline separated (or `AGENT_BROWSER_ARGS` env). Applies only to commands that can launch a browser. |
| `--user-agent <ua>` | Custom User-Agent string (or `AGENT_BROWSER_USER_AGENT` env) |
| `--proxy <url>` | Proxy server URL with optional auth (or `AGENT_BROWSER_PROXY` env) |
| `--proxy-bypass <hosts>` | Hosts to bypass proxy (or `AGENT_BROWSER_PROXY_BYPASS` env) |
| `--ignore-https-errors` | Ignore HTTPS certificate errors (useful for self-signed certs) |
| `--allow-file-access` | Allow file:// URLs to access local files (Chromium only) |
| `-p, --provider <name>` | Cloud browser provider (or `AGENT_BROWSER_PROVIDER` env) |
| `--device <name>` | iOS device name, e.g. "iPhone 15 Pro" (or `AGENT_BROWSER_IOS_DEVICE` env) |
| `--json` | JSON output (for agents) |
| `--annotate` | Annotated screenshot with numbered element labels (or `AGENT_BROWSER_ANNOTATE` env) |
| `--screenshot-dir <path>` | Default screenshot output directory (or `AGENT_BROWSER_SCREENSHOT_DIR` env) |
| `--screenshot-quality <n>` | JPEG quality 0-100 (or `AGENT_BROWSER_SCREENSHOT_QUALITY` env) |
| `--screenshot-format <fmt>` | Screenshot format: `png`, `jpeg` (or `AGENT_BROWSER_SCREENSHOT_FORMAT` env) |
| `--headed` | Show browser window (not headless). On Unix, agent-browser defaults `DISPLAY` to `:0.0` if `DISPLAY` is unset (or `AGENT_BROWSER_HEADED` env) |
| `--cdp <port\|url>` | Connect via Chrome DevTools Protocol (port or WebSocket URL) |
| `--auto-connect` | Auto-discover and connect to running Chrome (or `AGENT_BROWSER_AUTO_CONNECT` env) |
| `--color-scheme <scheme>` | Color scheme: `dark`, `light`, `no-preference` (or `AGENT_BROWSER_COLOR_SCHEME` env) |
| `--download-path <path>` | Default download directory (or `AGENT_BROWSER_DOWNLOAD_PATH` env) |
| `--content-boundaries` | Wrap page output in boundary markers for LLM safety (or `AGENT_BROWSER_CONTENT_BOUNDARIES` env) |
| `--max-output <chars>` | Truncate page output to N characters (or `AGENT_BROWSER_MAX_OUTPUT` env) |
| `--allowed-domains <list>` | Comma-separated allowed domain patterns (or `AGENT_BROWSER_ALLOWED_DOMAINS` env) |
| `--action-policy <path>` | Path to action policy JSON file (or `AGENT_BROWSER_ACTION_POLICY` env) |
| `--confirm-actions <list>` | Action categories requiring confirmation (or `AGENT_BROWSER_CONFIRM_ACTIONS` env) |
| `--confirm-interactive` | Interactive confirmation prompts; auto-denies if stdin is not a TTY (or `AGENT_BROWSER_CONFIRM_INTERACTIVE` env) |
| `--engine <name>` | Browser engine: `chrome` (default), `lightpanda` (or `AGENT_BROWSER_ENGINE` env) |
| `--service-reconcile-interval <ms>` | Background service browser-health reconciliation interval; `0` disables it (or `AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS` env) |
| `--service-monitor-interval <ms>` | Background active service-monitor scheduling interval; `0` disables it (or `AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS` env) |
| `--service-job-timeout <ms>` | Timeout for dispatched service control jobs; `0` disables it (or `AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS` env) |
| `--service-recovery-retry-budget <n>` | Browser recovery attempts before faulting (or `AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET` env) |
| `--service-recovery-base-backoff <ms>` | Browser recovery backoff base delay (or `AGENT_BROWSER_SERVICE_RECOVERY_BASE_BACKOFF_MS` env) |
| `--service-recovery-max-backoff <ms>` | Browser recovery backoff ceiling (or `AGENT_BROWSER_SERVICE_RECOVERY_MAX_BACKOFF_MS` env) |
| `--no-auto-dialog` | Disable automatic dismissal of `alert`/`beforeunload` dialogs (or `AGENT_BROWSER_NO_AUTO_DIALOG` env) |
| `--model <name>` | AI model for chat command (or `AI_GATEWAY_MODEL` env) |
| `-v`, `--verbose` | Show tool commands and their raw output (chat) |
| `-q`, `--quiet` | Show only AI text responses, hide tool calls (chat) |
| `--config <path>` | Use a custom config file (or `AGENT_BROWSER_CONFIG` env) |
| `--debug` | Debug output |

## Observability Dashboard

Monitor agent-browser sessions in real time with a local web dashboard showing a live viewport and command activity feed.

```bash
# Start the dashboard server (runs in background on port 4848)
agent-browser dashboard start
agent-browser dashboard start --port 8080   # Custom port

# All sessions are automatically visible in the dashboard
agent-browser open example.com

# Stop the dashboard
agent-browser dashboard stop
```

The dashboard runs as a standalone background process on port 4848, independent of browser sessions. It stays available even when no sessions are running. All sessions automatically stream to the dashboard.

The dashboard displays:
- **Live viewport** — real-time JPEG frames from the browser
- **Service view** — worker and browser health cards, a remembered operator identity for incident audit metadata, optional operator notes for incident acknowledgement and resolution, prominent incident severity, escalation, recommended action displays, and remedy summary groups sourced from the service incident contract, a service-owned incident history timeline with local fallback, a trace explorer backed by `/api/service/trace` for service, agent, task, browser, profile, session, and time-window debugging, including ownership summary cards, naming warnings, and profile lease wait cards from the shared trace payload, a browser-health transition timeline for crash/recovery visibility, a backend-owned profile allocation view from `profileAllocations` with detail inspection refreshed from `GET /api/service/profiles/<id>/allocation` and guarded by `pnpm test:dashboard-profile-allocation`, a grouped incident browser panel with handling-state filters plus acknowledge and resolve actions, incident filtering for crash/disconnect/recovery and timed-out or cancelled jobs, reconciliation status, managed entity counts, recent service jobs with naming warnings and queued/running job cancellation, browser/session/tab detail inspection, filterable service events including tab lifecycle changes, and a reconcile action
- **Activity feed** — chronological command/result stream with timing and expandable details
- **Console output** — browser console messages (log, warn, error)
- **Session creation** — create new sessions from the UI with local engines (Chrome, Lightpanda) or cloud providers (AgentCore, Browserbase, Browserless, Browser Use, Kernel)
- **AI Chat** — chat with an AI assistant directly in the dashboard (requires Vercel AI Gateway configuration)

### AI Chat

The dashboard includes an optional AI chat panel powered by the Vercel AI Gateway. The same functionality is available directly from the CLI via the `chat` command. Set these environment variables to enable AI chat:

```bash
export AI_GATEWAY_API_KEY=gw_your_key_here
export AI_GATEWAY_MODEL=anthropic/claude-sonnet-4.6           # optional, this is the default
export AI_GATEWAY_URL=https://ai-gateway.vercel.sh           # optional, this is the default
```

**CLI usage:**

```bash
agent-browser chat "open google.com and search for cats"     # Single-shot
agent-browser chat                                           # Interactive REPL
agent-browser -q chat "summarize this page"                  # Quiet mode (text only)
agent-browser -v chat "fill in the login form"               # Verbose (show command output)
agent-browser --model openai/gpt-4o chat "take a screenshot" # Override model
```

The `chat` command translates natural language instructions into agent-browser commands, executes them, and streams the AI response. In interactive mode, type `quit` to exit. Use `--json` for structured output suitable for agent consumption.

**Dashboard usage:**

The Chat tab is always visible in the dashboard. When `AI_GATEWAY_API_KEY` is set, the Rust server proxies requests to the gateway and streams responses back using the Vercel AI SDK's UI Message Stream protocol. Without the key, sending a message shows an error inline.

## Configuration

Create an `agent-browser.json` file to set persistent defaults instead of repeating flags on every command.

**Locations (lowest to highest priority):**

1. `~/.agent-browser/config.json`: user-level defaults
2. `./agent-browser.json`: project-level overrides (in working directory)
3. `AGENT_BROWSER_*` environment variables override config file values
4. CLI flags override everything

**Example `agent-browser.json`:**

```json
{
  "headed": true,
  "proxy": "http://localhost:8080",
  "profile": "./browser-data",
  "userAgent": "my-agent/1.0",
  "service": {
    "reconcileIntervalMs": 60000,
    "monitorIntervalMs": 60000,
    "jobTimeoutMs": 120000,
    "monitors": {
      "google-login-freshness": {
        "name": "Google login freshness",
        "target": { "site_policy": "google" },
        "intervalMs": 300000,
        "state": "active"
      }
    }
  },
  "ignoreHttpsErrors": true
}
```

Use `--config <path>` or `AGENT_BROWSER_CONFIG` to load a specific config file instead of the defaults:

```bash
agent-browser --config ./ci-config.json open example.com
AGENT_BROWSER_CONFIG=./ci-config.json agent-browser open example.com
```

All options from the table above can be set in the config file using camelCase keys (e.g., `--executable-path` becomes `"executablePath"`, `--proxy-bypass` becomes `"proxyBypass"`). Unknown keys are ignored for forward compatibility.

Service browser-health reconciliation runs in the daemon background every 60000 ms by default. Set `service.reconcileIntervalMs`, pass `--service-reconcile-interval <ms>`, or set `AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS` to change the interval. Use `0` to disable it.

Due active service monitors are enqueued through the same service worker every 60000 ms by default. Set `service.monitorIntervalMs`, pass `--service-monitor-interval <ms>`, or set `AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS` to change the scheduler interval. Use `0` to disable monitor scheduling. Use `agent-browser service monitors run-due`, `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to check due active monitors immediately. Use `agent-browser service monitors pause <id>` and `agent-browser service monitors resume <id>`, HTTP `POST /api/service/monitors/<id>/pause` and `POST /api/service/monitors/<id>/resume` routes, MCP `service_monitor_pause` and `service_monitor_resume`, or `pauseServiceMonitor()` and `resumeServiceMonitor()` to quiet or restore noisy monitors without clearing retained health history. Use `agent-browser service monitors triage <id>`, HTTP `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, or `triageServiceMonitor()` to acknowledge the related monitor incident and clear reviewed failures in one queued operation. Use `agent-browser service monitors reset <id>`, HTTP `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, or `resetServiceMonitorFailures()` when only the reviewed failure count should be cleared while retaining the last failure evidence. The runner updates `lastCheckedAt`, `lastSucceededAt`, `lastFailedAt`, `lastResult`, and `consecutiveFailures`; failed probes set the monitor `state` to `faulted` and append a service incident event.
Use a monitor target such as `{ "profile_readiness": "acs" }` when the service should police retained profile freshness for a target login identity without launching Chrome. If a fresh readiness row has expired, the monitor marks that row stale, removes the target from `authenticatedServiceIds`, faults the monitor, and records `staleProfileIds` in the monitor event.

Service control jobs do not time out at the worker boundary by default. Set `service.jobTimeoutMs`, pass `--service-job-timeout <ms>`, or set `AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS` to mark long-running dispatched jobs as `timed_out`. Use `0` to disable it.

Browser recovery defaults to 3 relaunch attempts, 1000 ms base backoff, and 30000 ms max backoff before marking a browser `faulted`. Set `service.recoveryRetryBudget`, `service.recoveryBaseBackoffMs`, and `service.recoveryMaxBackoffMs`, pass the matching `--service-recovery-*` flags, or use the `AGENT_BROWSER_SERVICE_RECOVERY_*` environment variables to tune this for a service host. Recovery-started trace events include `details.policySource.retryBudget`, `details.policySource.baseBackoffMs`, and `details.policySource.maxBackoffMs` so operators can see whether each active value came from defaults, config, environment, or CLI flags.

Boolean flags accept an optional `true`/`false` value to override config settings. For example, `--headed false` disables `"headed": true` from config. A bare `--headed` is equivalent to `--headed true`.

Auto-discovered config files that are missing are silently ignored. If `--config <path>` points to a missing or invalid file, agent-browser exits with an error. Extensions from user and project configs are merged (concatenated), not replaced.

> **Tip:** If your project-level `agent-browser.json` contains environment-specific values (paths, proxies), consider adding it to `.gitignore`.

## Default Timeout

The default timeout for standard operations (clicks, waits, fills, etc.) is 25 seconds. This is intentionally below the CLI's 30-second IPC read timeout so that the daemon returns a proper error instead of the CLI timing out with EAGAIN.

Override the default timeout via environment variable:

```bash
# Set a longer timeout for slow pages (in milliseconds)
export AGENT_BROWSER_DEFAULT_TIMEOUT=45000
```

> **Note:** Setting this above 30000 (30s) may cause EAGAIN errors on slow operations because the CLI's read timeout will expire before the daemon responds. The CLI retries transient errors automatically, but response times will increase.

| Variable                        | Description                              |
| ------------------------------- | ---------------------------------------- |
| `AGENT_BROWSER_DEFAULT_TIMEOUT` | Default operation timeout in ms (default: 25000) |

## Selectors

### Refs (Recommended for AI)

Refs provide deterministic element selection from snapshots:

```bash
# 1. Get snapshot with refs
agent-browser snapshot
# Output:
# - heading "Example Domain" [ref=e1] [level=1]
# - button "Submit" [ref=e2]
# - textbox "Email" [ref=e3]
# - link "Learn more" [ref=e4]

# 2. Use refs to interact
agent-browser click @e2                   # Click the button
agent-browser fill @e3 "test@example.com" # Fill the textbox
agent-browser get text @e1                # Get heading text
agent-browser hover @e4                   # Hover the link
```

**Why use refs?**

- **Deterministic**: Ref points to exact element from snapshot
- **Fast**: No DOM re-query needed
- **AI-friendly**: Snapshot + ref workflow is optimal for LLMs

### CSS Selectors

```bash
agent-browser click "#id"
agent-browser click ".class"
agent-browser click "div > button"
```

### Text & XPath

```bash
agent-browser click "text=Submit"
agent-browser click "xpath=//button"
```

### Semantic Locators

```bash
agent-browser find role button click --name "Submit"
agent-browser find label "Email" fill "test@test.com"
```

## Agent Mode

Use `--json` for machine-readable output:

```bash
agent-browser snapshot --json
# Returns: {"success":true,"data":{"snapshot":"...","refs":{"e1":{"role":"heading","name":"Title"},...}}}

agent-browser get text @e1 --json
agent-browser is visible @e2 --json
```

### Optimal AI Workflow

```bash
# 1. Navigate and get snapshot
agent-browser open example.com
agent-browser snapshot -i --json   # AI parses tree and refs

# 2. AI identifies target refs from snapshot
# 3. Execute actions using refs
agent-browser click @e2
agent-browser fill @e3 "input text"

# 4. Get new snapshot if page changed
agent-browser snapshot -i --json
```

### Command Chaining

Commands can be chained with `&&` in a single shell invocation. The browser persists via a background daemon, so chaining is safe and more efficient:

```bash
# Open, wait for load, and snapshot in one call
agent-browser open example.com && agent-browser wait --load networkidle && agent-browser snapshot -i

# Chain multiple interactions
agent-browser fill @e1 "user@example.com" && agent-browser fill @e2 "pass" && agent-browser click @e3

# Navigate and screenshot
agent-browser open example.com && agent-browser wait --load networkidle && agent-browser screenshot page.png
```

Use `&&` when you don't need intermediate output. Run commands separately when you need to parse output first (e.g., snapshot to discover refs before interacting).

## Headed Mode

Show the browser window for debugging:

```bash
agent-browser open example.com --headed
```

This opens a visible browser window instead of running headless.

On Unix, if `DISPLAY` is unset, agent-browser launches headed Chrome with `DISPLAY=:0.0` by default. This matches common WSL X server setups.

> **Note:** Browser extensions work in both headed and headless mode (Chrome's `--headless=new`).

## Authenticated Sessions

Use `--headers` to set HTTP headers for a specific origin, enabling authentication without login flows:

```bash
# Headers are scoped to api.example.com only
agent-browser open api.example.com --headers '{"Authorization": "Bearer <token>"}'

# Requests to api.example.com include the auth header
agent-browser snapshot -i --json
agent-browser click @e2

# Navigate to another domain - headers are NOT sent (safe!)
agent-browser open other-site.com
```

This is useful for:

- **Skipping login flows** - Authenticate via headers instead of UI
- **Switching users** - Start new sessions with different auth tokens
- **API testing** - Access protected endpoints directly
- **Security** - Headers are scoped to the origin, not leaked to other domains

To set headers for multiple origins, use `--headers` with each `open` command:

```bash
agent-browser open api.example.com --headers '{"Authorization": "Bearer token1"}'
agent-browser open api.acme.com --headers '{"Authorization": "Bearer token2"}'
```

For global headers (all domains), use `set headers`:

```bash
agent-browser set headers '{"X-Custom-Header": "value"}'
```

## Custom Browser Executable

Use a custom browser executable instead of the bundled Chromium. This is useful for:

- **Serverless deployment**: Use lightweight Chromium builds like `@sparticuz/chromium` (~50MB vs ~684MB)
- **System browsers**: Use an existing Chrome/Chromium installation
- **Custom builds**: Use modified browser builds

### CLI Usage

```bash
# Via flag
agent-browser --executable-path /path/to/chromium open example.com

# Via environment variable
AGENT_BROWSER_EXECUTABLE_PATH=/path/to/chromium agent-browser open example.com
```

### Serverless (Vercel)

Run agent-browser + Chrome in an ephemeral Vercel Sandbox microVM. No external server needed:

```typescript
import { Sandbox } from "@vercel/sandbox";

const sandbox = await Sandbox.create({ runtime: "node24" });
await sandbox.runCommand("agent-browser", ["open", "https://example.com"]);
const result = await sandbox.runCommand("agent-browser", ["screenshot", "--json"]);
await sandbox.stop();
```

See the [environments example](examples/environments/) for a working demo with a UI and deploy-to-Vercel button.

### Serverless (AWS Lambda)

```typescript
import chromium from '@sparticuz/chromium';
import { execSync } from 'child_process';

export async function handler() {
  const executablePath = await chromium.executablePath();
  const result = execSync(
    `AGENT_BROWSER_EXECUTABLE_PATH=${executablePath} agent-browser open https://example.com && agent-browser snapshot -i --json`,
    { encoding: 'utf-8' }
  );
  return JSON.parse(result);
}
```

## Local Files

Open and interact with local files (PDFs, HTML, etc.) using `file://` URLs:

```bash
# Enable file access (required for JavaScript to access local files)
agent-browser --allow-file-access open file:///path/to/document.pdf
agent-browser --allow-file-access open file:///path/to/page.html

# Take screenshot of a local PDF
agent-browser --allow-file-access open file:///Users/me/report.pdf
agent-browser screenshot report.png
```

The `--allow-file-access` flag adds Chromium flags (`--allow-file-access-from-files`, `--allow-file-access`) that allow `file://` URLs to:

- Load and render local files
- Access other local files via JavaScript (XHR, fetch)
- Load local resources (images, scripts, stylesheets)

**Note:** This flag only works with Chromium. For security, it's disabled by default.

## CDP Mode

Connect to an existing browser via Chrome DevTools Protocol:

```bash
# Start Chrome with: google-chrome --remote-debugging-port=9222

# Connect once, then run commands without --cdp
agent-browser connect 9222
agent-browser snapshot
agent-browser tab
agent-browser close

# Or pass --cdp on each command
agent-browser --cdp 9222 snapshot

# Connect to remote browser via WebSocket URL
agent-browser --cdp "wss://your-browser-service.com/cdp?token=..." snapshot
```

The `--cdp` flag accepts either:

- A port number (e.g., `9222`) for local connections via `http://localhost:{port}`
- A full WebSocket URL (e.g., `wss://...` or `ws://...`) for remote browser services

This enables control of:

- Electron apps
- Chrome/Chromium instances with remote debugging
- WebView2 applications
- Any browser exposing a CDP endpoint

### Auto-Connect

Use `--auto-connect` to automatically discover and connect to a running Chrome instance without specifying a port:

```bash
# Auto-discover running Chrome with remote debugging
agent-browser --auto-connect open example.com
agent-browser --auto-connect snapshot

# Or via environment variable
AGENT_BROWSER_AUTO_CONNECT=1 agent-browser snapshot
```

Auto-connect discovers Chrome by:

1. Reading Chrome's `DevToolsActivePort` file from the default user data directory
2. Falling back to probing common debugging ports (9222, 9229)
3. If HTTP-based discovery (`/json/version`, `/json/list`) fails, falling back to a direct WebSocket connection

This is useful when:

- Chrome 144+ has remote debugging enabled via `chrome://inspect/#remote-debugging` (which uses a dynamic port)
- You want a zero-configuration connection to your existing browser
- You don't want to track which port Chrome is using

## Streaming (Browser Preview)

Stream the browser viewport via WebSocket for live preview or "pair browsing" where a human can watch and interact alongside an AI agent.

### Streaming

Every session automatically starts a WebSocket stream server on an OS-assigned port. Use `stream status` to see the bound port and connection state:

```bash
agent-browser stream status
```

To bind to a specific port, set `AGENT_BROWSER_STREAM_PORT`:

```bash
AGENT_BROWSER_STREAM_PORT=9223 agent-browser open example.com
```

You can also manage streaming at runtime with `stream enable`, `stream disable`, and `stream status`:

```bash
agent-browser stream enable --port 9223   # Re-enable on a specific port
agent-browser stream disable              # Stop streaming for the session
```

The WebSocket server streams the browser viewport and accepts input events.

### Service Status

Use `service status` to inspect the service-mode control plane and configured service entities without launching a browser:

```bash
agent-browser service status
agent-browser service status --watch --interval 1000
agent-browser service watch --interval 1000 --count 5
agent-browser service reconcile
agent-browser service profiles
agent-browser service sessions
agent-browser service browsers
agent-browser service tabs
agent-browser service monitors
agent-browser service monitors --summary --failed
agent-browser service monitors --state faulted
agent-browser service monitors run-due
agent-browser service monitors pause google-login-freshness
agent-browser service monitors resume google-login-freshness
agent-browser service monitors reset google-login-freshness
agent-browser service monitors triage google-login-freshness --by operator --note reviewed
agent-browser service site-policies
agent-browser service providers
agent-browser service challenges
agent-browser service cancel <job-id> --reason stale
agent-browser service retry browser-1 --by operator --note approved
agent-browser service acknowledge browser-1 --by operator --note triaged
agent-browser service resolve browser-1 --by operator --note recovered
agent-browser service activity browser-1
agent-browser service trace --service-name JournalDownloader --task-name probeACSwebsite
agent-browser service jobs --limit 20
agent-browser service jobs --id <job-id>
agent-browser service jobs --state failed --action navigate --since 2026-04-22T00:00:00Z
agent-browser service jobs --service-name JournalDownloader --task-name probeACSwebsite
agent-browser service incidents --limit 20
agent-browser service incidents --summary --state active --handling-state unacknowledged
agent-browser service incidents --remedies
agent-browser service remedies
agent-browser service remedies apply --escalation monitor_attention --by operator --note reviewed
agent-browser service remedies apply --escalation browser_degraded --by operator --note reviewed
agent-browser service remedies apply --escalation os_degraded_possible --by operator --note host-inspected
agent-browser service incidents --id browser-1
agent-browser service incidents --handling-state unacknowledged
agent-browser service incidents --severity critical --escalation os_degraded_possible
agent-browser service incidents --state active --kind service_job_timeout
agent-browser service incidents --state recovered --handling-state resolved --browser-id browser-1
agent-browser service incidents --service-name JournalDownloader --task-name probeACSwebsite
agent-browser service events --limit 20
agent-browser service events --kind browser_health_changed --browser-id browser-1 --since 2026-04-22T00:00:00Z
agent-browser service events --service-name JournalDownloader --task-name probeACSwebsite
agent-browser service events --kind browser_recovery_started
agent-browser service events --kind browser_recovery_override
agent-browser service events --kind profile_lease_wait_started
agent-browser service events --kind tab_lifecycle_changed
```

The response includes worker state, browser health, queue depth, profile lease wait pressure, persisted service state from `~/.agent-browser/service/state.json`, and configured service-mode profiles, sessions, monitors, site policies, and providers from `agent-browser.json` and `~/.agent-browser/config.json`. In text mode, it summarizes profiles, the derived `profileAllocations` view, browsers, sessions, and profile lease wait pressure with service, agent, task, profile, profile selection reason, profile lease disposition, lease conflicts, lease, cleanup, browser linkage, health, and retained observation fields. `profileAllocations` is also returned by `service profiles`, `GET /api/service/profiles`, `GET /api/service/status`, and `agent-browser://profiles`; use `GET /api/service/profiles/<profile-id>/allocation` or `getServiceProfileAllocation()` when a software client needs one allocation row. Profile collections include `profileSources`, and profile lookup plus access-plan responses include `selectedProfileSource`, so clients can tell whether the effective profile came from config, runtime observation, or persisted state. The allocation view lists holder sessions, exclusive holders, waiting profile-lease jobs, conflict session IDs, related service, agent, and task labels, linked browsers and tabs, the current lease state, and the recommended next action for each known profile. It refreshes the persisted control-plane snapshot in `state.json` but does not launch a browser. It also probes persisted browser records: dead local PIDs are marked `process_exited`, unreachable CDP endpoints with a live PID are marked `cdp_disconnected`, unreachable CDP endpoints without a PID are marked `unreachable`, and endpoints that answer health probes but fail target-list discovery are marked `degraded`. Reachable CDP endpoints are queried for live page and webview targets, updating `tabs` and known session/tab relationships in service state. Non-ready browsers close their known tabs during reconciliation so stale tab state does not look active. When reconciliation removes stale session/tab ownership links, it appends a `reconciliation` event with `details.action: "session_tab_ownership_repaired"` and the removed relationships. Configured profiles, sessions, monitors, site policies, and providers override entries with the same IDs from the persisted state. Browser launch, close, and command-time stale-browser detection update the persisted browser health records for the active session. Browser records retain the latest non-ready health evidence in `lastHealthObservation`, so service status clients can inspect failure metadata without reconstructing events. Use `service profiles`, `service sessions`, `service browsers`, `service tabs`, `service monitors`, `service site-policies`, `service providers`, and `service challenges` for focused collection views without parsing the full status payload. Active monitor records are checked by the daemon scheduler when due; use `service monitors run-due`, `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to check due active monitors immediately. Use monitor pause/resume commands or helpers to change only the retained monitor state while preserving health evidence. Checks update `lastCheckedAt`, `lastSucceededAt`, `lastFailedAt`, `lastResult`, and `consecutiveFailures`, and failed probes set the monitor `state` to `faulted` and append a service incident event. A `profile_readiness` monitor target checks retained no-launch readiness for a target login identity; expired fresh rows are marked stale and removed from `authenticatedServiceIds` without launching Chrome. Unexpected process-exit health and recovery events include `details.processExitCause: "unexpected_process_exit"` and `details.failureClass: "browser_process_exited"`; active local Chrome exits also include `processExitDetection`, `processExitPid`, and exit code or signal when available. During close, agent-browser first tries a polite browser shutdown and then force kills owned browser processes if needed; close-generated health events include `details.shutdownReasonKind: "operator_requested_close"`, `details.processExitCause: "operator_requested_close"`, and shutdown outcome flags. Polite shutdown failure leaves the browser record `degraded`, and force-kill failure leaves it `faulted` with an OS-degraded warning. When a queued command finds the active browser process exited or the CDP connection disconnected, agent-browser records that failure before cleanup and relaunch. Runtime profile and custom profile launches also populate linked service profile and session records, including `serviceName`, `agentName`, `taskName`, `profileSelectionReason`, `profileLeaseDisposition`, and `profileLeaseConflictSessionIds` when the caller provides enough context. Service-scoped launches reject active exclusive profile conflicts by default before starting another browser; set `profileLeasePolicy: "wait"` and `profileLeaseWaitTimeoutMs` to wait for release instead. Same-session retained browser reuse remains allowed.

With `profileLeasePolicy: "wait"`, the control-plane scheduler keeps the blocked request queued while it polls for profile release, so the worker can continue dispatching unrelated service requests.

Run `pnpm test:service-site-policy-sources-no-launch` to validate that HTTP `GET /api/service/site-policies`, MCP `agent-browser://site-policies`, and `getServiceSitePolicies()` report config, persisted-state, and built-in site-policy source metadata without launching Chrome. Run `pnpm test:service-collections-live` to validate that CLI, HTTP, and MCP expose matching service-owned profile, session, browser, tab, monitor, site-policy, provider, and challenge collections for one live runtime-profile session.

The persisted service state includes a `reconciliation` snapshot with `lastReconciledAt`, `browserCount`, `changedBrowsers`, and `lastError` so operators can confirm when browser-health probes last ran.

The persisted service state also includes bounded audit records for recent control-plane jobs in `jobs`, a derived `incidents` collection that groups retained incident signals by browser or service scope, plus an `events` log with reconciliation summaries, browser health transitions, browser recovery starts, profile lease wait transitions, ownership-repair details, and tab lifecycle changes such as discovered tabs, URL or title changes, and closed tabs. Job records track request action, priority, timestamps, final success or failure, and error text without storing large command payloads.

Use `service reconcile` to run the persisted browser health and target probes intentionally without requesting a control-plane status snapshot. This command updates the same `reconciliation` snapshot, refreshes live tab records for reachable browser CDP endpoints, and appends service events.

Use `service status --watch` or `service watch` for a polling operator view of worker health, browser health, queue depth, profile lease wait pressure, and reconciliation status. In JSON mode, each poll is emitted as one JSON response line.

Use `service cancel <job-id>` to mark a queued or lease-waiting service job cancelled before it dispatches or request cooperative cancellation for a running job. Running cancellation drops the active service future, records the job as `cancelled`, and cleans up browser state before the worker accepts more work. Terminal jobs are rejected rather than rewritten. Add `--reason <text>` to record an operator-readable reason for queued cancellation.

Use `service acknowledge <incident-id>` to mark a retained incident seen by an operator. Add `--by <text>` to record who acknowledged it and `--note <text>` to persist a short operator note.

Use `service resolve <incident-id>` to mark a retained incident handled. This preserves the derived incident record while adding durable resolution metadata. Add `--by <text>` to record who resolved it and `--note <text>` to persist a resolution note.

Acknowledgement and resolution also append retained service events with `incident_acknowledged` and `incident_resolved` kinds. Incident detail includes those handling events alongside the health and job events that define the grouped incident. Use `service activity <incident-id>` to fetch a normalized chronological timeline for one retained incident without reconstructing it client-side.

The activity response is the canonical agent-facing incident timeline. It returns `{ incident, activity, count }`. Each activity item includes `id`, `source`, `timestamp`, `kind`, `title`, and `message`, plus `eventId` or `jobId` when it came from a retained event or job. Event and job items include trace context fields such as `browserId`, `profileId`, `sessionId`, `serviceName`, `agentName`, and `taskName` when known, so clients can display one timeline without rejoining raw records. Older retained incidents can include `source: "metadata"` acknowledgement or resolution items when handling metadata predates retained handling events.

Use `service trace --service-name <name> --task-name <name>` to inspect related events, jobs, incidents, and normalized activity in one response. Add `--limit <n>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--agent-name <name>`, or `--since <timestamp>` to narrow a trace view for one service, agent, task, browser, profile, session, or time window. This is the preferred service debugging surface when a client needs a complete timeline without issuing separate jobs, incidents, events, and incident activity requests. The response includes a `summary` object with compact service, agent, task, browser, profile, and session context rows plus per-context record counts, target identity hints, and naming warnings for missing service, agent, or task labels when debugging multi-agent runs. `summary.contexts[].targetServiceIds` lists the normalized target-service, site, and login identity hints observed on retained jobs in that context; text output shows them as `targets=...`. `summary.profileLeaseWaits` provides a per-job profile lease wait rollup with outcome, timing, conflict sessions, and trace labels. Text output includes a `Profile lease waits` block when that summary or the raw wait events are present. Browser crash recovery traces expose the canonical sequence in `events`: a `browser_health_changed` event with `currentHealth` such as `process_exited` or `cdp_disconnected` and `details.currentReasonKind`, then `browser_recovery_started` with `details.reasonKind`, `details.reason`, `details.attempt`, `details.retryBudget`, `details.nextRetryDelayMs`, and `details.policySource`, then a `browser_health_changed` event with `currentHealth: "ready"` after relaunch. Stale health and recovery events also include `details.failureClass`, such as `browser_process_exited`, `cdp_unresponsive`, `cdp_endpoint_unreachable`, or `target_discovery_failed`. Process-exit health and recovery events also include `details.processExitCause: "unexpected_process_exit"`. Active local Chrome process exits include `details.processExitDetection: "local_child_try_wait"`, `details.processExitPid`, and `details.processExitCode` or `details.processExitSignal` when available. `details.policySource` reports `default`, `config`, `env`, or `cli` for the retry budget, base backoff, and max backoff values. Operator-requested shutdown health events instead include `details.shutdownReasonKind: "operator_requested_close"` and `details.processExitCause: "operator_requested_close"` plus polite-close and force-kill outcome flags, so clients can separate clean or degraded closes from unexpected exits. If the next attempt would exceed the default retry budget, the browser is marked `faulted` and the command fails instead of relaunching. HTTP clients read the same payload from `/api/service/trace`, and MCP clients read it through the `service_trace` tool.

Use `service retry <browser-id> --by <operator> --note <text>` to explicitly allow one new recovery attempt for a faulted browser. It records a `browser_recovery_override` event, moves the browser back to a retryable stale health state, and resets retry counting from that override boundary. HTTP retry requests accept `service-name`, `agent-name`, and `task-name` query parameters, and MCP `service_browser_retry` accepts `serviceName`, `agentName`, and `taskName`, so override events appear in filtered service traces.

Use `service jobs --limit <n>` to inspect recent control-plane jobs without parsing the full service state. Use `service jobs --id <job-id>` to inspect one retained job directly. Add `--state <state>`, `--action <action>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter jobs before the limit is applied. Valid states are `queued`, `waiting_profile_lease`, `running`, `succeeded`, `failed`, `cancelled`, and `timed_out`. `--since` accepts RFC 3339 timestamps. Jobs include `namingWarnings` when a request is missing `serviceName`, `agentName`, or `taskName`; these warnings are advisory and do not reject anonymous compatibility requests. Jobs also retain target identity hints: singular `targetServiceId`, `siteId`, and `loginId` preserve the exact singular request fields when present, and `targetServiceIds` contains the normalized target-service, site, and login identity set used for profile selection. Current warning values are `missing_service_name`, `missing_agent_name`, and `missing_task_name`. `hasNamingWarning` is `true` when `namingWarnings` is non-empty. `pnpm test:service-health-live` includes the static API/MCP parity guard, HTTP job-naming check, and service-request live smoke; run `pnpm test:service-api-mcp-parity`, `pnpm test:service-job-naming-live`, or `pnpm test:service-request-live` directly when only one contract needs validation.

Use `service incidents --limit <n>` to inspect grouped retained incidents directly without parsing the full service state. Add `--summary` to group the current filtered incident set by escalation, severity, and state with each group's recommended next action. Add `--remedies`, or use `service remedies`, for the compact operator ladder that returns active `browser_degraded`, `monitor_attention`, and `os_degraded_possible` groups only. Use `service remedies apply --escalation monitor_attention`, HTTP `POST /api/service/remedies/apply?escalation=monitor_attention`, MCP `service_remedies_apply`, or `applyServiceRemedies()` to acknowledge all active monitor-attention incidents and reset the reviewed monitor failure counters through the service worker. Use `service remedies apply --escalation browser_degraded` after operator review to batch retry enablement for active degraded-browser incidents. Use `service remedies apply --escalation os_degraded_possible` only after host inspection to batch the existing faulted-browser retry remedy for active OS-degraded-possible incidents. HTTP clients can request the same ladder with `summary=true&remedies=true`, and MCP clients can pass `summary: true` and `remediesOnly: true` to `service_incidents`. Run `pnpm test:service-remedies-cli-no-launch` and `pnpm test:service-remedies-json-no-launch` to validate CLI text plus JSON ladders. Run `pnpm test:service-remedies-apply-json-no-launch` to validate monitor-attention, degraded-browser, and OS-degraded batch apply JSON plus persisted state changes without launching Chrome. Run `pnpm test:service-incident-summary-http` plus `pnpm test:service-incident-summary-mcp` to validate both summary paths and their shared filter matrix without launching Chrome. Use `service incidents --id <incident-id>` to fetch one retained incident together with its expanded related events and jobs. Incident detail also includes acknowledgement and resolution metadata when present. Incidents include `severity`, `escalation`, `recommendedAction`, and monitor metadata when a failed service monitor created the incident, so CLI, HTTP, MCP, and dashboard clients do not infer operator priority differently. Failed service monitors use `monitor_attention`, expose `monitorId`, `monitorTarget`, and `monitorResult` on the incident. Summary groups include `browserIds`, `monitorIds`, `monitorResetCommands`, and `remedyApplyCommand` so operators and agents can see the exact affected browsers and the batch apply command. Add `--state <state>`, `--severity <severity>`, `--escalation <escalation>`, `--handling-state <state>`, `--kind <kind>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter incidents before the limit is applied. Trace-context filters match related events and jobs. Valid incident states are `active`, `recovered`, and `service`. Valid severities are `info`, `warning`, `error`, and `critical`. Valid escalations are `none`, `browser_degraded`, `browser_recovery`, `job_attention`, `monitor_attention`, `service_triage`, and `os_degraded_possible`. Valid handling states are `unacknowledged`, `acknowledged`, and `resolved`. Valid kinds are `browser_health_changed`, `reconciliation_error`, `service_job_timeout`, and `service_job_cancelled`. `--since` compares the incident `latestTimestamp` using RFC 3339 timestamps.

Use `service events --limit <n>` to inspect recent reconciliation summaries, browser launch metadata, browser health transitions, browser recovery starts, browser recovery overrides, profile lease wait transitions, tab lifecycle changes, and incident handling events without parsing the full service state. Launch, health, recovery, and profile lease wait events include `profileId`, `sessionId`, `serviceName`, `agentName`, and `taskName` when that context is known. Add `--kind <kind>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter events before the limit is applied. Valid kinds are `reconciliation`, `browser_launch_recorded`, `browser_health_changed`, `browser_recovery_started`, `browser_recovery_override`, `tab_lifecycle_changed`, `profile_lease_wait_started`, `profile_lease_wait_ended`, `reconciliation_error`, `incident_acknowledged`, and `incident_resolved`. Profile lease wait events include `details.jobId`, `details.outcome`, `details.conflictSessionIds`, retry timing, and waited timing when known. `--since` accepts RFC 3339 timestamps.

When the session stream server is running, agents can read the same service surface over HTTP without shelling out:

```bash
curl "http://127.0.0.1:<stream-port>/api/browser/url"
curl "http://127.0.0.1:<stream-port>/api/browser/title"
curl "http://127.0.0.1:<stream-port>/api/browser/tabs?verbose=true"
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/navigate" -H "content-type: application/json" -d '{"url":"https://example.com","waitUntil":"load"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/back" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/forward" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/reload" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/new-tab" -H "content-type: application/json" -d '{"url":"https://example.com/next"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/switch-tab" -H "content-type: application/json" -d '{"index":0}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/close-tab" -H "content-type: application/json" -d '{"index":1}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/viewport" -H "content-type: application/json" -d '{"width":1280,"height":720,"deviceScaleFactor":1}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/user-agent" -H "content-type: application/json" -d '{"userAgent":"AgentBrowserClient/1.0"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/media" -H "content-type: application/json" -d '{"colorScheme":"dark","reducedMotion":"reduce"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/timezone" -H "content-type: application/json" -d '{"timezoneId":"America/Chicago"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/locale" -H "content-type: application/json" -d '{"locale":"en-US"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/geolocation" -H "content-type: application/json" -d '{"latitude":41.8781,"longitude":-87.6298,"accuracy":10}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/permissions" -H "content-type: application/json" -d '{"permissions":["geolocation"]}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/cookies/get" -H "content-type: application/json" -d '{"urls":["https://example.com"]}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/cookies/set" -H "content-type: application/json" -d '{"name":"session","value":"abc","url":"https://example.com"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/cookies/clear" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/storage/get" -H "content-type: application/json" -d '{"type":"local","key":"token"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/storage/set" -H "content-type: application/json" -d '{"type":"local","key":"token","value":"abc"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/storage/clear" -H "content-type: application/json" -d '{"type":"local"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/console" -H "content-type: application/json" -d '{"clear":false}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/errors" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/set-content" -H "content-type: application/json" -d '{"html":"<main>Ready</main>"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/headers" -H "content-type: application/json" -d '{"headers":{"X-Client":"agent-browser"}}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/offline" -H "content-type: application/json" -d '{"offline":false}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/dialog" -H "content-type: application/json" -d '{"response":"status"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/clipboard" -H "content-type: application/json" -d '{"operation":"write","text":"copied text"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/upload" -H "content-type: application/json" -d '{"selector":"#file","files":["/tmp/file.txt"]}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/download" -H "content-type: application/json" -d '{"selector":"#download","path":"/tmp/download.txt"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/wait-for-download" -H "content-type: application/json" -d '{"path":"/tmp/download.txt","timeoutMs":5000}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/pdf" -H "content-type: application/json" -d '{"path":"/tmp/page.pdf"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/response-body" -H "content-type: application/json" -d '{"url":"/api/data","timeoutMs":5000}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/har/start" -H "content-type: application/json" -d '{}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/har/stop" -H "content-type: application/json" -d '{"path":"/tmp/capture.har"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/route" -H "content-type: application/json" -d '{"url":"**/api/*","response":{"status":200,"body":"{}","contentType":"application/json"}}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/unroute" -H "content-type: application/json" -d '{"url":"**/api/*"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/requests" -H "content-type: application/json" -d '{"filter":"/api","method":"GET","status":"2xx"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/request-detail" -H "content-type: application/json" -d '{"requestId":"<request-id>"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/snapshot" -H "content-type: application/json" -d '{"selector":"main","interactive":true}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/screenshot" -H "content-type: application/json" -d '{"selector":"main","path":"page.png"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/click" -H "content-type: application/json" -d '{"selector":"#submit","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/fill" -H "content-type: application/json" -d '{"selector":"#query","value":"search text","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/wait" -H "content-type: application/json" -d '{"selector":"#result","text":"Ready","timeoutMs":5000}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/type" -H "content-type: application/json" -d '{"selector":"#query","text":" more","delayMs":10}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/press" -H "content-type: application/json" -d '{"key":"Enter"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/hover" -H "content-type: application/json" -d '{"selector":"#menu"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/select" -H "content-type: application/json" -d '{"selector":"#state","values":["CA"]}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-text" -H "content-type: application/json" -d '{"selector":"#result"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-value" -H "content-type: application/json" -d '{"selector":"#query"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/is-visible" -H "content-type: application/json" -d '{"selector":"#result"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-attribute" -H "content-type: application/json" -d '{"selector":"a","attribute":"href"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-html" -H "content-type: application/json" -d '{"selector":"main"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-styles" -H "content-type: application/json" -d '{"selector":"#result","properties":["display","width"]}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/count" -H "content-type: application/json" -d '{"selector":".row"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/get-box" -H "content-type: application/json" -d '{"selector":"#result"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/is-enabled" -H "content-type: application/json" -d '{"selector":"#submit"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/is-checked" -H "content-type: application/json" -d '{"selector":"#remember"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/check" -H "content-type: application/json" -d '{"selector":"#remember"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/uncheck" -H "content-type: application/json" -d '{"selector":"#remember"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/scroll" -H "content-type: application/json" -d '{"direction":"down","amount":500}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/scroll-into-view" -H "content-type: application/json" -d '{"selector":"#footer"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/focus" -H "content-type: application/json" -d '{"selector":"#query"}'
curl -X POST "http://127.0.0.1:<stream-port>/api/browser/clear" -H "content-type: application/json" -d '{"selector":"#query"}'
curl "http://127.0.0.1:<stream-port>/api/service/status"
curl "http://127.0.0.1:<stream-port>/api/service/profiles"
curl "http://127.0.0.1:<stream-port>/api/service/profiles/lookup?service-name=JournalDownloader&login-id=acs"
curl "http://127.0.0.1:<stream-port>/api/service/profiles/journal-downloader/allocation"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/profiles/journal-downloader" -H "content-type: application/json" -d '{"name":"Journal Downloader","allocation":"per_service","keyring":"basic_password_store","persistent":true,"targetServiceIds":["acs"],"authenticatedServiceIds":["acs"],"sharedServiceIds":["JournalDownloader"]}'
curl -X DELETE "http://127.0.0.1:<stream-port>/api/service/profiles/journal-downloader"
curl "http://127.0.0.1:<stream-port>/api/service/sessions"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/sessions/journal-run" -H "content-type: application/json" -d '{"serviceName":"JournalDownloader","agentName":"codex","taskName":"probeACSwebsite","profileId":"journal-downloader","lease":"exclusive","cleanup":"close_browser"}'
curl -X DELETE "http://127.0.0.1:<stream-port>/api/service/sessions/journal-run"
curl "http://127.0.0.1:<stream-port>/api/service/browsers"
curl "http://127.0.0.1:<stream-port>/api/service/tabs"
curl "http://127.0.0.1:<stream-port>/api/service/site-policies"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/site-policies/google" -H "content-type: application/json" -d '{"originPattern":"https://accounts.google.com","interactionProfile":"headed","challengeStrategy":"avoid"}'
curl -X DELETE "http://127.0.0.1:<stream-port>/api/service/site-policies/google"
curl "http://127.0.0.1:<stream-port>/api/service/providers"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/providers/manual" -H "content-type: application/json" -d '{"name":"Manual approval","kind":"manual_approval","enabled":true}'
curl -X DELETE "http://127.0.0.1:<stream-port>/api/service/providers/manual"
curl "http://127.0.0.1:<stream-port>/api/service/challenges"
curl "http://127.0.0.1:<stream-port>/api/service/trace?service-name=JournalDownloader&task-name=probeACSwebsite"
curl "http://127.0.0.1:<stream-port>/api/service/jobs?limit=20&state=failed"
curl "http://127.0.0.1:<stream-port>/api/service/jobs/<job-id>"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/jobs/<job-id>/cancel"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/browsers/<browser-id>/retry?by=operator&note=approved&service-name=JournalDownloader&task-name=probeACSwebsite"
curl "http://127.0.0.1:<stream-port>/api/service/incidents?summary=true&limit=20&handling-state=unacknowledged"
curl "http://127.0.0.1:<stream-port>/api/service/incidents?summary=true&remedies=true"
curl "http://127.0.0.1:<stream-port>/api/service/incidents/<incident-id>"
curl "http://127.0.0.1:<stream-port>/api/service/incidents/<incident-id>/activity"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/incidents/<incident-id>/acknowledge?by=operator&note=triaged"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/incidents/<incident-id>/resolve?by=operator&note=recovered"
curl "http://127.0.0.1:<stream-port>/api/service/events?limit=20&kind=browser_health_changed"
curl "http://127.0.0.1:<stream-port>/api/service/contracts"
curl "http://127.0.0.1:<stream-port>/api/service/access-plan?service-name=JournalDownloader&login-id=acs&site-policy-id=acs"
curl "http://127.0.0.1:<stream-port>/api/service/monitors"
curl "http://127.0.0.1:<stream-port>/api/service/monitors?summary=true&failed=true"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/monitors/google-login-freshness" -H "content-type: application/json" -d '{"name":"Google login freshness","target":{"site_policy":"google"},"intervalMs":60000,"state":"paused"}'
curl -X DELETE "http://127.0.0.1:<stream-port>/api/service/monitors/google-login-freshness"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/request" -H "content-type: application/json" -d '{"serviceName":"JournalDownloader","agentName":"article-probe-agent","taskName":"probeACSwebsite","siteId":"acs","action":"navigate","params":{"url":"https://example.com","waitUntil":"load"}}'
curl -X POST "http://127.0.0.1:<stream-port>/api/service/remedies/apply?escalation=monitor_attention&by=operator&note=reviewed"
curl -X POST "http://127.0.0.1:<stream-port>/api/service/reconcile"
```

The HTTP API loads the same persisted and configured service state as the CLI before relaying state-changing requests to the daemon. Named browser endpoints are thin wrappers over the same daemon command queue as `/api/command` and MCP tools. POST bodies accept the same command fields as the underlying action, including `serviceName`, `agentName`, `taskName`, and `jobTimeoutMs` for traceable software clients. `GET /api/service/contracts` and MCP `agent-browser://contracts` return compatibility metadata for service request schema IDs, contract versions, routes, MCP tool names, and supported actions. `GET /api/service/contracts` also advertises `serviceProfileAllocationResponse`, `serviceProfileReadinessResponse`, `serviceProfileSeedingHandoffResponse`, `serviceProfileLookupResponse`, and `serviceAccessPlanResponse`; the metadata names the `@agent-browser/client/service-observability` helpers and the lookup selection order software clients should expect. `POST /api/service/request` accepts one intent object with `serviceName`, `agentName`, `taskName`, `siteId`, `loginId`, target-service hints, explicit `profile` or `runtimeProfile` hints, `action`, `params`, and `jobTimeoutMs`, then queues that browser action through the same service-owned control path. Its request object follows `docs/dev/contracts/service-request.v1.schema.json`; the MCP `tools/call` wrapper follows `docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json`. `GET /api/service/jobs` and `GET /api/service/jobs/<id>` return service job records matching the repo schema at `docs/dev/contracts/service-job-record.v1.schema.json`; their response envelopes follow `docs/dev/contracts/service-jobs-response.v1.schema.json`. `GET /api/service/incidents` and `GET /api/service/incidents/<id>` return service incident records matching the repo schema at `docs/dev/contracts/service-incident-record.v1.schema.json`; their response envelopes follow `docs/dev/contracts/service-incidents-response.v1.schema.json`. `GET /api/service/events` returns service event records matching the repo schema at `docs/dev/contracts/service-event-record.v1.schema.json`; its response envelope follows `docs/dev/contracts/service-events-response.v1.schema.json`. HTTP and MCP profile, browser, session, tab, monitor, site policy, provider, and challenge records follow the matching repo schemas under `docs/dev/contracts/service-*-record.v1.schema.json`; profile records include derived `targetReadiness` rows. Service status and compact collection response envelopes follow the matching status and collection response schemas under `docs/dev/contracts/`. `GET /api/service/profiles/<id>/allocation` follows `docs/dev/contracts/service-profile-allocation-response.v1.schema.json` and returns one derived profile allocation row, including the same `targetReadiness` rows. `GET /api/service/profiles/<id>/readiness` follows `docs/dev/contracts/service-profile-readiness-response.v1.schema.json` and returns one profile's no-launch readiness rows without allocation details. `GET /api/service/profiles/<id>/seeding-handoff` follows `docs/dev/contracts/service-profile-seeding-handoff-response.v1.schema.json` and returns the exact detached `runtime login` command, setup URL, operator steps, and warnings derived from `targetReadiness`; pass `targetServiceId`, `siteId`, or `loginId` when a profile has multiple target identities. `GET /api/service/profiles/lookup` follows `docs/dev/contracts/service-profile-lookup-response.v1.schema.json` and applies the authoritative service profile selector for `serviceName` plus target, site, or login identity. `GET /api/service/access-plan` follows `docs/dev/contracts/service-access-plan-response.v1.schema.json` and returns the service-owned no-launch recommendation that combines the selected profile, readiness, readiness summary, site policy, enabled providers, retained challenges, and decision fields before a caller requests browser control. `GET /api/service/monitors` follows `docs/dev/contracts/service-monitors-response.v1.schema.json` and returns retained monitor records for heartbeat and freshness probes; `state`, `failed`, and `summary` query parameters filter and summarize repeated monitor failures for operator triage. Active monitors are checked by the daemon scheduler when due. `service_trace` responses follow `docs/dev/contracts/service-trace-response.v1.schema.json`, with summary and activity records covered by the matching trace schemas. Incident activity responses follow `docs/dev/contracts/service-incident-activity-response.v1.schema.json`. `POST /api/service/profiles/<id>`, `POST /api/service/profiles/<id>/freshness`, `POST /api/service/sessions/<id>`, `POST /api/service/site-policies/<id>`, `POST /api/service/monitors/<id>`, and `POST /api/service/providers/<id>` persist service config records through the service worker queue, and `DELETE` on the same entity paths removes persisted records through the same queue. Profile, session, site-policy, monitor, and provider mutation responses follow `docs/dev/contracts/service-*-upsert-response.v1.schema.json` and `docs/dev/contracts/service-*-delete-response.v1.schema.json`. Job cancel, browser retry, incident acknowledgement or resolution, and remedy apply responses follow the matching operator remedy response schemas under `docs/dev/contracts/`, including `service-remedies-apply-response.v1.schema.json` for `POST /api/service/remedies/apply`. Service reconcile responses follow `docs/dev/contracts/service-reconcile-response.v1.schema.json`. The path ID is authoritative; requests with a conflicting nested `id` field are rejected. Profile mutations reject `caller_supplied` profiles without `userDataDir` and `per_service` profiles with more than one `sharedServiceIds` entry. Session mutations infer `owner` from `agentName`, then `serviceName`, when `owner` is omitted; `profileId` must reference a persisted profile, and profile `sharedServiceIds` allow-lists are enforced. `service_profile_freshness_update` and `POST /api/service/profiles/<id>/freshness` merge bounded-probe profile freshness evidence without requiring clients to replace the whole profile. The collection endpoints return the same compact arrays as the MCP resources: `profiles`, `sessions`, `browsers`, `tabs`, `monitors`, `sitePolicies`, `providers`, and `challenges`, each with a `count` field. Site-policy collections also include `sitePolicySources` so operators and clients can see whether each effective policy came from config, persisted state, or built-in defaults. Run `pnpm test:service-config-live` to validate HTTP and MCP mutation parity for profiles, sessions, site policies, monitors, and providers.

MCP `service_profile_freshness_update` mirrors `POST /api/service/profiles/<id>/freshness` for serialized profile freshness updates. HTTP `POST /api/service/monitors/<id>`, HTTP `DELETE /api/service/monitors/<id>`, MCP `service_monitor_upsert`, MCP `service_monitor_delete`, `upsertServiceMonitor()`, and `deleteServiceMonitor()` persist monitor definitions. Active monitors are checked by the daemon scheduler when due; use HTTP `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, `runDueServiceMonitors()`, or `agent-browser service monitors run-due` to run due active monitors immediately. Use HTTP `POST /api/service/monitors/<id>/pause`, HTTP `POST /api/service/monitors/<id>/resume`, MCP `service_monitor_pause`, MCP `service_monitor_resume`, `pauseServiceMonitor()`, `resumeServiceMonitor()`, or the matching CLI commands to quiet or restore a monitor without clearing health history. Use HTTP `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, `triageServiceMonitor()`, or `agent-browser service monitors triage <id>` to acknowledge the related monitor incident and reset reviewed failures together. Use HTTP `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, `resetServiceMonitorFailures()`, or `agent-browser service monitors reset <id>` after a reviewed failure has been triaged and the retained failure evidence should remain available. Use HTTP `POST /api/service/remedies/apply?escalation=monitor_attention`, MCP `service_remedies_apply`, `applyServiceRemedies()`, or `agent-browser service remedies apply --escalation monitor_attention` to apply all active monitor remedies in one serialized operation. Use `escalation=browser_degraded` after operator review to batch retry enablement for active degraded-browser incidents. Use `escalation=os_degraded_possible` only after host inspection to batch the existing faulted-browser retry remedy for active OS-degraded-possible incidents.

Software clients can use this TypeScript-shaped payload for `POST /api/service/request`:

```ts
import {
  createServiceRequest,
  postServiceRequest,
  requestServiceTab,
} from '@agent-browser/client/service-request';

const request = {
  serviceName: 'JournalDownloader',
  agentName: 'article-probe-agent',
  taskName: 'probeACSwebsite',
  siteId: 'acs',
  action: 'navigate',
  params: {
    url: 'https://example.com',
    waitUntil: 'load',
  },
  jobTimeoutMs: 30000,
};

const result = await postServiceRequest({
  baseUrl: `http://127.0.0.1:${streamPort}`,
  request: createServiceRequest(request),
});

const tab = await requestServiceTab({
  baseUrl: `http://127.0.0.1:${streamPort}`,
  serviceName: 'JournalDownloader',
  agentName: 'article-probe-agent',
  taskName: 'probeACSwebsite',
  siteId: 'acs',
  loginId: 'acs',
  url: 'https://example.com',
  jobTimeoutMs: 30000,
});
```

The private workspace package `@agent-browser/client` is generated from the
service request schemas. Run `pnpm generate:service-client` after changing
`docs/dev/contracts/service-request*.json`, and run `pnpm test:service-client`
to verify generated files, helper types, typed service command responses,
service request helpers, and observability helpers are current, including
package export resolution through `@agent-browser/client`. `postServiceRequest`
and `requestServiceTab` return `ServiceRequestResponse`, the standard service
command envelope with `success`, optional `data`, optional `error`, optional
`warning`, and action-specific fields. Every action currently listed in
`docs/dev/contracts/service-request.v1.schema.json` gets a typed `data` shape
through
`ServiceRequestDataForAction`; `requestServiceTab` returns typed `tab_new`
data. `requestServiceTab`, `createServiceTabRequest`, and
`createServiceTabRequestFromAccessPlan` accept an access-plan response so
software clients can queue the planned tab request without manually unpacking
`decision.serviceRequest.request`. Explicit call fields such as `url`,
`params`, `jobTimeoutMs`, or caller labels override the planned defaults. When
adding or removing service request actions, update the Rust
`SERVICE_REQUEST_ACTIONS` list, the JSON schema enum, MCP `service_request`,
HTTP `/api/service/request`, and generated client files together; run
`pnpm test:service-api-mcp-parity`, `pnpm test:service-client-contract`, and
the targeted MCP and HTTP Rust action-parity tests before handoff. Service
request helpers accept `profileLeasePolicy: "reject"` or
`"wait"` plus `profileLeaseWaitTimeoutMs` for callers that want agent-browser to
keep a request queued for a profile lease rather than fail immediately. The
service request client smoke also verifies that `requestServiceTab()` preserves
`loginId` and `targetServiceId` in the payload before any browser-launching
request is sent. Use `pnpm test:service-client-contract`,
`pnpm test:service-client-types`, `pnpm test:service-client-exports`,
`pnpm test:service-request-client`, or `pnpm test:service-observability-client`
when only one client contract needs validation.

For read-side and service-configuration software clients, import
`@agent-browser/client/service-observability`. It exposes typed helpers for
`getServiceStatus`, `getServiceContracts`, collection reads for profiles,
browsers, sessions, tabs, monitors, site policies, providers, and challenges,
`getServiceProfileAllocation`, `getServiceProfileReadiness`,
`summarizeServiceProfileReadiness`, `findServiceProfileForIdentity`,
`getServiceProfileForIdentity`, `lookupServiceProfile`, `getServiceAccessPlan`,
`postServiceReconcile`, upsert and delete helpers for profiles, sessions, site policies, and providers,
`registerServiceLoginProfile` for the common login-identity profile recipe,
including optional freshness fields such as `readinessState`,
`readinessEvidence`, `lastVerifiedAt`, and `freshnessExpiresAt`,
`updateServiceProfileFreshness` for service-side bounded probe freshness updates through `POST /api/service/profiles/<id>/freshness`,
`createServiceProfileReadinessMonitor` and `upsertServiceProfileReadinessMonitor` for the standard
`profile_readiness` monitor recipe that keeps registered login profiles from silently aging out,
operator remedy helpers for job cancel, browser retry, and incident handling,
`getServiceJobs`, `getServiceJob`, `getServiceEvents`, `getServiceIncidents`,
`getServiceIncident`, `getServiceIncidentActivity`, and `getServiceTrace`.
Declarations are generated from the matching service contract schemas.

Software clients should treat agent-browser as the profile broker. A client
such as CanvaCLI should call `getServiceAccessPlan()` for
`serviceName: "CanvaCLI"`, `agentName`, `taskName`, plus `loginId: "canva"` or
`targetServiceId: "canva"` before registering a profile. That helper uses HTTP
`GET /api/service/access-plan` to return the selected profile, readiness
summary, matching site policy, relevant providers and challenges, and the
service-owned recommended action before a browser launch. Readiness summaries
and decisions are scoped to the requested target identities, so an unrelated
stale or unseeded login on the same profile does not block the requested site.
The decision includes `freshnessUpdate`, which names the selected profile,
target identities, HTTP route, MCP tool, and `updateServiceProfileFreshness`
client helper to use after a bounded auth probe reports current login state.
It also includes `serviceRequest`, a copyable service-owned tab request recipe
for `POST /api/service/request`, MCP `service_request`, and
`requestServiceTab()`. `serviceRequest.available` is true when the planned tab
request can be queued immediately. When manual seeding or challenge work must
finish first, `recommendedAfterManualAction` tells clients to reuse the same
identity request after the operator completes that step.
Access-plan responses echo `agentName` and `taskName` in `query` and report
`namingWarnings` plus `hasNamingWarning` in both `query` and `decision` when
the caller omits `serviceName`, `agentName`, or `taskName`.
The decision also separates auth providers from challenge-capable providers,
reports the challenge strategy, and lists any missing provider capabilities
such as `captcha_solve`, `sms_code`, or `human_approval`. It also reports
`interactionRisk` and a `pacing` block derived from the site policy rate limits
so clients can explain headed, human-like, jittered, or single-session behavior
before creating browser pressure. The `launchPosture` block resolves the
browser host from site policy, profile default, or the service default, and
explains whether the plan is headed, remote-view capable, or requires detached
first-login seeding.
When no local site policy exists, agent-browser applies shipped defaults for
Google, Gmail, and Microsoft login identities. Local persisted or configured
policies with the same IDs override those defaults. `sitePolicySource` reports
whether the selected policy came from config, persisted state, or a built-in default, how
it matched the request, and whether it is overrideable.
Then the client should request the tab by the same identity through `requestServiceTab()` or
`POST /api/service/request`. `lookupServiceProfile()` remains useful for the narrower profile-only decision and uses HTTP
`GET /api/service/profiles/lookup` so agent-browser applies the same server-side
selector used for service launches without returning the full profile
collection. Its `selectedProfileMatch` reports the selector reason, matched
profile field, and matched identity. `getServiceProfileForIdentity()` remains
as the older descriptive alias for the same route. Only call
`registerServiceLoginProfile()` when
agent-browser has no suitable managed profile, readiness reports
`needs_manual_seeding`, the operator wants a separate account lane, or the
client is explicitly bringing its own profile.
New profiles that need Google sign-on, Chrome sync, passkeys, or browser
plugin setup must be seeded in headed Chrome before CDP is attached. Do not use
`--attachable`, a remote debugging port, or any DevTools/CDP attachment for the
first Google sign-in. Register the profile as unauthenticated, prefer
`keyring: "basic_password_store"`, launch
`agent-browser --runtime-profile <name> runtime login https://accounts.google.com`,
let the operator complete sign-in, sync, passkey, and extension setup, then
close Chrome. Future service-owned requests can attach after that manual phase.
Structured `targetReadiness` rows expose this requirement with
`seedingMode: "detached_headed_no_cdp"`,
`cdpAttachmentAllowedDuringSeeding: false`,
`preferredKeyring: "basic_password_store"`, and setup scopes for sign-in,
Chrome sync, passkeys, and browser plugins.
When a software client has just completed a bounded auth probe, it can pass
`readinessState: "fresh"`, `readinessEvidence`, `lastVerifiedAt`, and
`freshnessExpiresAt` to the same helper so agent-browser records target
freshness without forcing the caller to hand-build a full profile record.
Explicit `targetReadiness` rows may also be supplied and override generated
rows for matching target identities.
For an already registered profile, use `updateServiceProfileFreshness()` instead:
it posts to the service-side freshness mutation endpoint, which merges the new
readiness row under the serialized service-state mutator, preserves unrelated profile fields, and updates
`authenticatedServiceIds` so stale or blocked targets stop looking fresh.

```ts
import { requestServiceTab } from '@agent-browser/client/service-request';
import {
  getServiceAccessPlan,
  registerServiceLoginProfile,
  upsertServiceProfileReadinessMonitor,
} from '@agent-browser/client/service-observability';

const baseUrl = `http://127.0.0.1:${streamPort}`;
const serviceName = 'CanvaCLI';
const loginId = 'canva';

const accessPlan = await getServiceAccessPlan({
  baseUrl,
  serviceName,
  agentName: 'canva-cli-agent',
  taskName: 'openCanvaWorkspace',
  loginId,
  targetServiceId: loginId,
});

if (!accessPlan.selectedProfile) {
  await registerServiceLoginProfile({
    baseUrl,
    id: 'canva-default',
    serviceName,
    loginId,
    authenticated: false,
  });
  await upsertServiceProfileReadinessMonitor({
    baseUrl,
    serviceName,
    loginId,
  });
}

const tab = await requestServiceTab({
  baseUrl,
  accessPlan,
  loginId,
  url: 'https://www.canva.com/',
  jobTimeoutMs: 30000,
});
```

If `accessPlan.readinessSummary.needsManualSeeding` is true, show the returned
`seedingHandoff` command, setup URL, operator steps, warnings, and recommended
actions to the operator before expecting authenticated automation to succeed
for the requested identity. Use `lookupServiceProfile()` only when the caller
needs the narrower profile-only selector response; it also includes
`seedingHandoff` when profile readiness requires manual seeding.

Direct profile selection is an override. Use it when an operator knows the
desired login state lives in a specific managed runtime profile, or when a
client explicitly brings its own profile directory and accepts responsibility
for that browser identity. Otherwise, let agent-browser coordinate profile
choice, profile lease waiting, browser reuse, and queued control requests.

See `examples/service-client/` for a copyable workflow that asks for an access
plan, requests a service tab with `requestServiceTab`, reads the matching trace,
and can demonstrate known queued-job cancellation with `cancelServiceJob`. Run
`pnpm test:service-client-example` to validate the example in dry-run mode.
The main `service-request-trace.mjs` example is the generic integration path
for non-Canva software clients: pass `--register-profile-id` and
`--register-readiness-monitor` when the service needs a recurring managed
profile, then let agent-browser register the profile, add the retained
freshness monitor, get the access plan, and submit the planned tab request.
That dry run also covers `managed-profile-flow.mjs`, a CanvaCLI-style
profile-broker recipe that uses the no-launch profile planning surfaces to
ask agent-browser for an access plan, inspect readiness and the service-owned
decision, pass the access-plan response to `requestServiceTab()`, and register
a managed login profile only when agent-browser has no suitable one. It can
also post bounded auth-probe evidence through `updateServiceProfileFreshness()`
for an existing managed profile. When the recipe registers a profile, it can
also call `upsertServiceProfileReadinessMonitor()` so the service periodically
checks retained freshness and surfaces stale profile auth through access-plan
`monitorFindings`. Access plans include `monitorFindings` so a client can see
active `profile_readiness` monitor incidents before requesting browser control.
Its output includes `readinessSummary.needsManualSeeding`, target service IDs,
recommended actions, and `seedingHandoff` when readiness says an operator must
seed the profile. Run
`pnpm test:service-client-managed-profile-flow` for the no-launch mock smoke
that proves an existing managed profile is selected without registering a new
one. Run `pnpm test:service-access-plan-no-launch` when changing the access-plan
surface; it verifies HTTP `/api/service/access-plan`, MCP
`agent-browser://access-plan`, and `getServiceAccessPlan()` agree on the same
seeded no-launch recommendation, including caller label warnings, without
creating browsers or browser-launching jobs. Run
`pnpm test:service-request-live` when changing the planned tab-request
handoff; it verifies an access-plan response can be passed into
`requestServiceTab()` against an isolated daemon and real browser session. Run
`pnpm test:service-client-example-live` to validate the main trace
example against a real isolated daemon and browser session.

For MCP clients, use `mcp serve` to run a stdio server that exposes service resources without launching a browser. The server supports `initialize`, `ping`, `resources/list`, `resources/templates/list`, `resources/read`, `tools/list`, and `tools/call`. MCP tools include `service_request`, which queues one intent-based browser action with caller context and site/login hints, `service_job_cancel`, which cancels queued service jobs or requests cancellation for running jobs, `service_browser_retry`, which enables a new recovery attempt for a faulted browser, `service_incidents`, which reads grouped retained incidents with the same state, severity, escalation, handling, kind, browser, profile, session, service, agent, task, since, and summary filters as CLI and HTTP, `service_trace`, which reads related events, jobs, incidents, and activity from persisted service state, `service_profile_upsert`, `service_profile_delete`, `service_session_upsert`, `service_session_delete`, `service_site_policy_upsert`, `service_site_policy_delete`, `service_provider_upsert`, and `service_provider_delete`, which mutate persisted service config through the service worker queue with the same ID checks as HTTP, `browser_navigate`, which queues typed navigation for the active browser session, `browser_requests`, which enables and filters request inspection, `browser_request_detail`, which reads one tracked request by ID, `browser_headers`, which sets extra HTTP headers for the active browser session, `browser_offline`, which toggles network offline emulation, `browser_cookies_get`, which reads cookies, `browser_cookies_set`, which sets cookies, `browser_cookies_clear`, which clears cookies, `browser_storage_get`, which reads localStorage or sessionStorage, `browser_storage_set`, which sets localStorage or sessionStorage, `browser_storage_clear`, which clears localStorage or sessionStorage, `browser_user_agent`, which sets the user agent, `browser_viewport`, which sets the viewport, `browser_geolocation`, which sets geolocation emulation, `browser_permissions`, which grants browser permissions, `browser_timezone`, which sets timezone emulation, `browser_locale`, which sets locale emulation, `browser_media`, which sets media emulation, `browser_dialog`, which handles dialog status or response, `browser_upload`, which uploads files, `browser_download`, which clicks and saves downloads, `browser_wait_for_download`, which waits for downloads, `browser_har_start` and `browser_har_stop`, which capture HAR files, `browser_route`, which routes matching requests, `browser_unroute`, which removes routes, `browser_console`, which reads or clears console messages, `browser_errors`, which reads page errors, `browser_pdf`, which saves PDFs, `browser_response_body`, which reads matching response bodies, `browser_clipboard`, which controls clipboard operations, `browser_command`, which queues any supported browser-control action for HTTP parity, `browser_snapshot`, which queues the existing snapshot command for the active browser session, `browser_get_url`, which reads the active browser URL, `browser_get_title`, which reads the active browser title, `browser_tabs`, which lists open tabs, `browser_screenshot`, which saves a screenshot for visual inspection, `browser_click`, which clicks a selector or cached ref through the queued control plane, `browser_fill`, which fills a field through the queued control plane, `browser_wait`, which waits for selector, text, URL, function, load-state, or fixed-duration conditions through the queued control plane, `browser_type`, which types text through the queued control plane, `browser_press`, which presses keys and key chords through the queued control plane, `browser_hover`, which hovers elements through the queued control plane, `browser_select`, which selects dropdown values through the queued control plane, `browser_get_text`, which reads element text through the queued control plane, `browser_get_value`, which reads field values through the queued control plane, `browser_get_attribute`, which reads element attributes through the queued control plane, `browser_get_html`, which reads element inner HTML through the queued control plane, `browser_get_styles`, which reads computed styles through the queued control plane, `browser_count`, which counts matching elements through the queued control plane, `browser_get_box`, which reads element geometry through the queued control plane, `browser_is_visible`, which reads element visibility through the queued control plane, `browser_is_enabled`, which reads element enabled state through the queued control plane, `browser_check`, which checks checkbox or radio controls through the queued control plane, `browser_is_checked`, which reads checkbox, radio, or ARIA checked state through the queued control plane, `browser_uncheck`, which unchecks checkbox controls through the queued control plane, `browser_scroll`, which scrolls pages or containers through the queued control plane, `browser_scroll_into_view`, which scrolls a target element into view through the queued control plane, `browser_focus`, which focuses a target element through the queued control plane, and `browser_clear`, which clears a target field through the queued control plane. MCP tool callers should include `serviceName`, `agentName`, and `taskName` when available so multi-service and multi-agent behavior remains traceable. Service jobs persist these caller context fields when commands provide them and persist advisory `namingWarnings` when any caller label is missing. Access-plan responses echo the same caller labels and report the same naming warnings in `query` and `decision`. Current warning values are `missing_service_name`, `missing_agent_name`, and `missing_task_name`; `hasNamingWarning` is `true` when `namingWarnings` is non-empty. The `agent-browser://contracts` resource mirrors HTTP `GET /api/service/contracts` with service request schema IDs, contract versions, route names, MCP tool names, and supported actions. The `agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,sitePolicyId,challengeId,readinessProfileId}` template mirrors HTTP `GET /api/service/access-plan` and returns the no-launch service-owned profile, policy, provider, challenge, readiness, and recommendation payload. The `agent-browser://jobs` resource returns the same service job record schema as HTTP: `docs/dev/contracts/service-job-record.v1.schema.json`; CLI and HTTP `service_jobs` response envelopes follow `docs/dev/contracts/service-jobs-response.v1.schema.json`. The `agent-browser://incidents` resource and `service_incidents` tool return the same service incident record schema as HTTP: `docs/dev/contracts/service-incident-record.v1.schema.json`; `service_incidents` response envelopes follow `docs/dev/contracts/service-incidents-response.v1.schema.json`. Run `pnpm test:mcp-live` to validate the live daemon, browser, MCP tool call, and retained job metadata path. Run `pnpm test:service-reconcile-live` to validate that `service reconcile` and MCP browser/tab resources agree on live service-owned state. Run `pnpm test:service-profile-live` to validate that runtime-profile launches populate MCP profile and session resources with caller metadata. Run `pnpm test:service-profile-http-live` to validate the same profile and session metadata through the HTTP service API. Run `pnpm test:service-request-live` to validate HTTP `/api/service/request` and MCP `service_request` over one isolated live browser session. Run `pnpm test:service-recovery-http-live` to validate the HTTP trace contract for crash detection, recovery start, and ready-after-relaunch events. Run `pnpm test:service-recovery-mcp-live` to validate the same recovery trace contract through MCP `service_trace`. Run `pnpm test:service-api-mcp-parity` to statically check that named browser-control HTTP endpoints, typed MCP tools, README, skill, and docs site stay aligned. For shell inspection, use `mcp resources` to list service resource contracts and `mcp read <uri>` to read one resource from persisted service state. Implemented resources are `agent-browser://contracts`, `agent-browser://access-plan`, `agent-browser://incidents`, `agent-browser://profiles`, `agent-browser://sessions`, `agent-browser://browsers`, `agent-browser://tabs`, `agent-browser://site-policies`, `agent-browser://providers`, `agent-browser://challenges`, `agent-browser://jobs`, `agent-browser://events`, `agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,sitePolicyId,challengeId,readinessProfileId}`, and `agent-browser://incidents/{incident_id}/activity`.

The equivalent MCP `tools/call` payload uses the same intent fields:

```json
{"name":"service_request","arguments":{"serviceName":"JournalDownloader","agentName":"article-probe-agent","taskName":"probeACSwebsite","siteId":"acs","action":"navigate","params":{"url":"https://example.com","waitUntil":"load"},"jobTimeoutMs":30000}}
```

Run `pnpm test:service-shutdown-health-live` to validate that a polite browser shutdown failure leaves the persisted service browser record `degraded` after the owned Chrome process is force-killed.

Typed MCP tools also include `browser_back`, `browser_forward`, `browser_reload`, `browser_tab_new`, `browser_tab_switch`, `browser_tab_close`, and `browser_set_content` for browser history, tab lifecycle, and page-content control.

Use `browser_navigate`, `browser_back`, `browser_forward`, `browser_reload`, `browser_tab_new`, `browser_tab_switch`, `browser_tab_close`, and `browser_set_content` for typed navigation, tab, and page-content control. Use `browser_requests` for request discovery, `browser_request_detail` for one tracked request, `browser_headers` for extra HTTP headers, `browser_offline` for network emulation, `browser_cookies_*` for cookies, `browser_storage_*` for localStorage or sessionStorage, `browser_user_agent` for user agent emulation, `browser_viewport` for viewport emulation, `browser_geolocation` for geolocation emulation, `browser_permissions` for permission grants, `browser_timezone` for timezone emulation, `browser_locale` for locale emulation, `browser_media` for media emulation, `browser_dialog` for dialog status and responses, `browser_upload`, `browser_download`, `browser_wait_for_download`, `browser_har_start`, `browser_har_stop`, `browser_route`, `browser_unroute`, `browser_console`, `browser_errors`, `browser_pdf`, `browser_response_body`, and `browser_clipboard` for file, HAR, routing, observability, and artifact workflows. Use `browser_command` for controls that do not yet have typed MCP tools. The `action` field is the daemon action name, and `params` contains the same fields accepted by the CLI, HTTP wrapper, or `/api/command` path:

```json
{"name":"browser_navigate","arguments":{"url":"https://example.com","waitUntil":"load","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_requests","arguments":{"filter":"/api","method":"GET","status":"2xx","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_request_detail","arguments":{"requestId":"<request-id>","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_headers","arguments":{"headers":{"Authorization":"Bearer <token>"},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_offline","arguments":{"offline":false,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_cookies_get","arguments":{"urls":["https://example.com"],"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_cookies_set","arguments":{"name":"session","value":"abc","url":"https://example.com","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_storage_set","arguments":{"type":"local","key":"token","value":"abc","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_storage_get","arguments":{"type":"local","key":"token","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_user_agent","arguments":{"userAgent":"AgentBrowserBot/1.0","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_viewport","arguments":{"width":1280,"height":720,"deviceScaleFactor":1,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_geolocation","arguments":{"latitude":41.8781,"longitude":-87.6298,"accuracy":10,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_permissions","arguments":{"permissions":["geolocation"],"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_timezone","arguments":{"timezoneId":"America/Chicago","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_locale","arguments":{"locale":"en-US","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_media","arguments":{"media":"screen","colorScheme":"light","reducedMotion":"no-preference","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_dialog","arguments":{"response":"status","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_upload","arguments":{"selector":"#file","files":["/tmp/file.txt"],"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_download","arguments":{"selector":"#download","path":"/tmp/download.txt","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_wait_for_download","arguments":{"path":"/tmp/download.txt","timeoutMs":5000,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_har_start","arguments":{"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_har_stop","arguments":{"path":"/tmp/capture.har","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_route","arguments":{"url":"**/api/*","response":{"status":200,"body":"{}","contentType":"application/json"},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_unroute","arguments":{"url":"**/api/*","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_console","arguments":{"clear":true,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_errors","arguments":{"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_pdf","arguments":{"path":"/tmp/page.pdf","printBackground":true,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_response_body","arguments":{"url":"/api/data","timeoutMs":5000,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_clipboard","arguments":{"operation":"write","text":"copied text","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_reload","arguments":{"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_tab_new","arguments":{"url":"https://example.com","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_tab_switch","arguments":{"index":0,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_tab_close","arguments":{"index":1,"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_set_content","arguments":{"html":"<main>Ready</main>","serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_command","arguments":{"action":"navigate","params":{"url":"https://example.com","waitUntil":"load"},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_command","arguments":{"action":"headers","params":{"headers":{"Authorization":"Bearer <token>"}},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_command","arguments":{"action":"offline","params":{"offline":false},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_command","arguments":{"action":"requests","params":{"filter":"/api","method":"GET","status":"2xx"},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
{"name":"browser_command","arguments":{"action":"request_detail","params":{"requestId":"<request-id>"},"serviceName":"JournalDownloader","taskName":"probeACSwebsite"}}
```

Service browser-health reconciliation runs in the daemon background every 60000 ms by default. Set `service.reconcileIntervalMs`, `--service-reconcile-interval <ms>`, or `AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS` to change the interval. Use `0` to disable it.

Due active service monitors are enqueued through the same service worker every 60000 ms by default. Set `service.monitorIntervalMs`, `--service-monitor-interval <ms>`, or `AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS` to change the interval. Use `0` to disable it. Use `service monitors run-due`, `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to check due active monitors immediately.

Service control jobs do not time out at the worker boundary by default. Set `service.jobTimeoutMs`, `--service-job-timeout <ms>`, or `AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS` to mark long-running dispatched jobs as `timed_out`. Use `0` to disable it.

Browser recovery defaults to 3 relaunch attempts, 1000 ms base backoff, and 30000 ms max backoff before marking a browser `faulted`. Set `service.recoveryRetryBudget`, `service.recoveryBaseBackoffMs`, and `service.recoveryMaxBackoffMs`, pass the matching `--service-recovery-*` flags, or use the `AGENT_BROWSER_SERVICE_RECOVERY_*` environment variables to tune this for a service host. Recovery-started trace events include `details.policySource.retryBudget`, `details.policySource.baseBackoffMs`, and `details.policySource.maxBackoffMs` so clients can audit whether each active value came from defaults, config, environment, or CLI flags.

### WebSocket Protocol

Connect to `ws://localhost:9223` to receive frames and send input:

**Receive frames:**

```json
{
  "type": "frame",
  "data": "<base64-encoded-jpeg>",
  "metadata": {
    "deviceWidth": 1280,
    "deviceHeight": 720,
    "pageScaleFactor": 1,
    "offsetTop": 0,
    "scrollOffsetX": 0,
    "scrollOffsetY": 0
  }
}
```

**Send mouse events:**

```json
{
  "type": "input_mouse",
  "eventType": "mousePressed",
  "x": 100,
  "y": 200,
  "button": "left",
  "clickCount": 1
}
```

**Send keyboard events:**

```json
{
  "type": "input_keyboard",
  "eventType": "keyDown",
  "key": "Enter",
  "code": "Enter"
}
```

**Send touch events:**

```json
{
  "type": "input_touch",
  "eventType": "touchStart",
  "touchPoints": [{ "x": 100, "y": 200 }]
}
```

## Architecture

agent-browser uses a client-daemon architecture:

1. **Rust CLI** - Parses commands, communicates with daemon
2. **Rust Daemon** - Pure Rust daemon using direct CDP, no Node.js required

The daemon starts automatically on first command and persists between commands for fast subsequent operations. To auto-shutdown the daemon after a period of inactivity, set `AGENT_BROWSER_IDLE_TIMEOUT_MS` (value in milliseconds). When set, the daemon closes the browser and exits after receiving no commands for the specified duration.

Persisted service browser-health reconciliation runs every 60000 ms while the daemon is alive. Set `AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS` to change the interval. Use `0` to disable the background loop. Due active service monitors are also enqueued every 60000 ms by default; set `AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS=0` to disable monitor scheduling. Use `agent-browser service monitors run-due` for an immediate due-monitor pass without waiting for the next scheduler tick. Use `agent-browser service monitors pause <id>` and `agent-browser service monitors resume <id>` when a noisy monitor needs an explicit operator state change. Use `agent-browser service monitors triage <id>` to acknowledge the related monitor incident and clear reviewed failures together, or `agent-browser service monitors reset <id>` when only the failure count should be cleared while keeping retained evidence.

**Browser Engine:** Uses Chrome (from Chrome for Testing) by default. The `--engine` flag selects between `chrome` and `lightpanda`. Supported browsers: Chromium/Chrome (via CDP) and Safari (via WebDriver for iOS).

## Platforms

| Platform    | Binary      |
| ----------- | ----------- |
| macOS ARM64 | Native Rust |
| macOS x64   | Native Rust |
| Linux ARM64 | Native Rust |
| Linux x64   | Native Rust |
| Windows x64 | Native Rust |

## Usage with AI Agents

### Just ask the agent

The simplest approach is to tell your agent to use it:

```
Use agent-browser to test the login flow. Run agent-browser --help to see available commands.
```

The `--help` output is comprehensive and most agents can figure it out from there.

### AI Coding Assistants (recommended)

Add the skill to your AI coding assistant for richer context:

```bash
npx skills add vercel-labs/agent-browser
```

This works with Claude Code, Codex, Cursor, Gemini CLI, GitHub Copilot, Goose, OpenCode, and Windsurf. The skill is fetched from the repository, so it stays up to date automatically. Do not copy `SKILL.md` from `node_modules` as it will become stale.

### Claude Code

Install as a Claude Code skill:

```bash
npx skills add vercel-labs/agent-browser
```

This adds the skill to `.claude/skills/agent-browser/SKILL.md` in your project. The skill teaches Claude Code the full agent-browser workflow, including the snapshot-ref interaction pattern, session management, and timeout handling.

### AGENTS.md / CLAUDE.md

For more consistent results, add to your project or global instructions file:

```markdown
## Browser Automation

Use `agent-browser` for web automation. Run `agent-browser --help` for all commands.

Core workflow:

1. `agent-browser open <url>` - Navigate to page
2. `agent-browser snapshot -i` - Get interactive elements with refs (@e1, @e2)
3. `agent-browser click @e1` / `fill @e2 "text"` - Interact using refs
4. Re-snapshot after page changes
```

## Integrations

### iOS Simulator

Control real Mobile Safari in the iOS Simulator for authentic mobile web testing. Requires macOS with Xcode.

**Setup:**

```bash
# Install Appium and XCUITest driver
npm install -g appium
appium driver install xcuitest
```

**Usage:**

```bash
# List available iOS simulators
agent-browser device list

# Launch Safari on a specific device
agent-browser -p ios --device "iPhone 16 Pro" open https://example.com

# Same commands as desktop
agent-browser -p ios snapshot -i
agent-browser -p ios tap @e1
agent-browser -p ios fill @e2 "text"
agent-browser -p ios screenshot mobile.png

# Mobile-specific commands
agent-browser -p ios swipe up
agent-browser -p ios swipe down 500

# Close session
agent-browser -p ios close
```

Or use environment variables:

```bash
export AGENT_BROWSER_PROVIDER=ios
export AGENT_BROWSER_IOS_DEVICE="iPhone 16 Pro"
agent-browser open https://example.com
```

| Variable                   | Description                                     |
| -------------------------- | ----------------------------------------------- |
| `AGENT_BROWSER_PROVIDER`   | Set to `ios` to enable iOS mode                 |
| `AGENT_BROWSER_IOS_DEVICE` | Device name (e.g., "iPhone 16 Pro", "iPad Pro") |
| `AGENT_BROWSER_IOS_UDID`   | Device UDID (alternative to device name)        |

**Supported devices:** All iOS Simulators available in Xcode (iPhones, iPads), plus real iOS devices.

**Note:** The iOS provider boots the simulator, starts Appium, and controls Safari. First launch takes ~30-60 seconds; subsequent commands are fast.

#### Real Device Support

Appium also supports real iOS devices connected via USB. This requires additional one-time setup:

**1. Get your device UDID:**

```bash
xcrun xctrace list devices
# or
system_profiler SPUSBDataType | grep -A 5 "iPhone\|iPad"
```

**2. Sign WebDriverAgent (one-time):**

```bash
# Open the WebDriverAgent Xcode project
cd ~/.appium/node_modules/appium-xcuitest-driver/node_modules/appium-webdriveragent
open WebDriverAgent.xcodeproj
```

In Xcode:

- Select the `WebDriverAgentRunner` target
- Go to Signing & Capabilities
- Select your Team (requires Apple Developer account, free tier works)
- Let Xcode manage signing automatically

**3. Use with agent-browser:**

```bash
# Connect device via USB, then:
agent-browser -p ios --device "<DEVICE_UDID>" open https://example.com

# Or use the device name if unique
agent-browser -p ios --device "John's iPhone" open https://example.com
```

**Real device notes:**

- First run installs WebDriverAgent to the device (may require Trust prompt)
- Device must be unlocked and connected via USB
- Slightly slower initial connection than simulator
- Tests against real Safari performance and behavior

### Browserless

[Browserless](https://browserless.io) provides cloud browser infrastructure with a Sessions API. Use it when running agent-browser in environments where a local browser isn't available.

To enable Browserless, use the `-p` flag:

```bash
export BROWSERLESS_API_KEY="your-api-token"
agent-browser -p browserless open https://example.com
```

Or use environment variables for CI/scripts:

```bash
export AGENT_BROWSER_PROVIDER=browserless
export BROWSERLESS_API_KEY="your-api-token"
agent-browser open https://example.com
```

Optional configuration via environment variables:

| Variable                   | Description                                      | Default                                 |
| -------------------------- | ------------------------------------------------ | --------------------------------------- |
| `BROWSERLESS_API_URL`      | Base API URL (for custom regions or self-hosted) | `https://production-sfo.browserless.io` |
| `BROWSERLESS_BROWSER_TYPE` | Type of browser to use (chromium or chrome)      | chromium                                |
| `BROWSERLESS_TTL`          | Session TTL in milliseconds                      | `300000`                                |
| `BROWSERLESS_STEALTH`      | Enable stealth mode (`true`/`false`)             | `true`                                  |

When enabled, agent-browser connects to a Browserless cloud session instead of launching a local browser. All commands work identically.

Get your API token from the [Browserless Dashboard](https://browserless.io).

### Browserbase

[Browserbase](https://browserbase.com) provides remote browser infrastructure to make deployment of agentic browsing agents easy. Use it when running the agent-browser CLI in an environment where a local browser isn't feasible.

To enable Browserbase, use the `-p` flag:

```bash
export BROWSERBASE_API_KEY="your-api-key"
agent-browser -p browserbase open https://example.com
```

Or use environment variables for CI/scripts:

```bash
export AGENT_BROWSER_PROVIDER=browserbase
export BROWSERBASE_API_KEY="your-api-key"
agent-browser open https://example.com
```

When enabled, agent-browser connects to a Browserbase session instead of launching a local browser. All commands work identically.

Get your API key from the [Browserbase Dashboard](https://browserbase.com/overview).

### Browser Use

[Browser Use](https://browser-use.com) provides cloud browser infrastructure for AI agents. Use it when running agent-browser in environments where a local browser isn't available (serverless, CI/CD, etc.).

To enable Browser Use, use the `-p` flag:

```bash
export BROWSER_USE_API_KEY="your-api-key"
agent-browser -p browseruse open https://example.com
```

Or use environment variables for CI/scripts:

```bash
export AGENT_BROWSER_PROVIDER=browseruse
export BROWSER_USE_API_KEY="your-api-key"
agent-browser open https://example.com
```

When enabled, agent-browser connects to a Browser Use cloud session instead of launching a local browser. All commands work identically.

Get your API key from the [Browser Use Cloud Dashboard](https://cloud.browser-use.com/settings?tab=api-keys). Free credits are available to get started, with pay-as-you-go pricing after.

### Kernel

[Kernel](https://www.kernel.sh) provides cloud browser infrastructure for AI agents with features like stealth mode and persistent profiles.

To enable Kernel, use the `-p` flag:

```bash
export KERNEL_API_KEY="your-api-key"
agent-browser -p kernel open https://example.com
```

Or use environment variables for CI/scripts:

```bash
export AGENT_BROWSER_PROVIDER=kernel
export KERNEL_API_KEY="your-api-key"
agent-browser open https://example.com
```

Optional configuration via environment variables:

| Variable                 | Description                                                                      | Default |
| ------------------------ | -------------------------------------------------------------------------------- | ------- |
| `KERNEL_HEADLESS`        | Run browser in headless mode (`true`/`false`)                                    | `false` |
| `KERNEL_STEALTH`         | Enable stealth mode to avoid bot detection (`true`/`false`)                      | `true`  |
| `KERNEL_TIMEOUT_SECONDS` | Session timeout in seconds                                                       | `300`   |
| `KERNEL_PROFILE_NAME`    | Browser profile name for persistent cookies/logins (created if it doesn't exist) | (none)  |

When enabled, agent-browser connects to a Kernel cloud session instead of launching a local browser. All commands work identically.

**Profile Persistence:** When `KERNEL_PROFILE_NAME` is set, the profile will be created if it doesn't already exist. Cookies, logins, and session data are automatically saved back to the profile when the browser session ends, making them available for future sessions.

Get your API key from the [Kernel Dashboard](https://dashboard.onkernel.com).

### AgentCore

[AWS Bedrock AgentCore](https://aws.amazon.com/bedrock/agentcore/) provides cloud browser sessions with SigV4 authentication.

To enable AgentCore, use the `-p` flag:

```bash
agent-browser -p agentcore open https://example.com
```

Or use environment variables for CI/scripts:

```bash
export AGENT_BROWSER_PROVIDER=agentcore
agent-browser open https://example.com
```

Credentials are automatically resolved from environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`) or the AWS CLI (`aws configure export-credentials`), which supports SSO, profiles, and IAM roles.

Optional configuration via environment variables:

| Variable                   | Description                                                          | Default          |
| -------------------------- | -------------------------------------------------------------------- | ---------------- |
| `AGENTCORE_REGION`         | AWS region for the AgentCore endpoint                                | `us-east-1`      |
| `AGENTCORE_BROWSER_ID`     | Browser identifier                                                   | `aws.browser.v1` |
| `AGENTCORE_PROFILE_ID`     | Browser profile for persistent state (cookies, localStorage)         | (none)           |
| `AGENTCORE_SESSION_TIMEOUT`| Session timeout in seconds                                           | `3600`           |
| `AWS_PROFILE`              | AWS CLI profile for credential resolution                            | `default`        |

**Browser profiles:** When `AGENTCORE_PROFILE_ID` is set, browser state (cookies, localStorage) is persisted across sessions automatically.

When enabled, agent-browser connects to an AgentCore cloud browser session instead of launching a local browser. All commands work identically.

## License

Apache-2.0
