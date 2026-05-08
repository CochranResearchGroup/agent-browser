---
name: agent-browser
description: Browser automation CLI for AI agents. Use when the user needs to interact with websites, including navigating pages, filling forms, clicking buttons, taking screenshots, extracting data, testing web apps, or automating any browser task. Triggers include requests to "open a website", "fill out a form", "click a button", "take a screenshot", "scrape data from a page", "test this web app", "login to a site", "automate browser actions", or any task requiring programmatic web interaction.
allowed-tools: Bash(npx agent-browser:*), Bash(agent-browser:*)
---

# Browser Automation with agent-browser

The CLI uses Chrome/Chromium via CDP directly. For this fork, install the native binary from the GitHub releases at `CochranResearchGroup/agent-browser`; npm, Homebrew, and Cargo are not authoritative release channels. Run `agent-browser install` to download Chrome. Existing Chrome, Brave, Playwright, and Puppeteer installations are detected automatically. To update, replace the binary from the latest GitHub release.

## Core Workflow

Every browser automation follows this pattern:

1. **Navigate**: `agent-browser open <url>`
2. **Snapshot**: `agent-browser snapshot -i` (get element refs like `@e1`, `@e2`)
3. **Interact**: Use refs to click, fill, select
4. **Re-snapshot**: After navigation or DOM changes, get fresh refs

```bash
agent-browser open https://example.com/form
agent-browser snapshot -i
# Output: @e1 [input type="email"], @e2 [input type="password"], @e3 [button] "Submit"

agent-browser fill @e1 "user@example.com"
agent-browser fill @e2 "password123"
agent-browser click @e3
agent-browser wait 2000
agent-browser snapshot -i  # Check result
```

## Command Chaining

For agent work, prefer `agent-browser batch` for 2 or more sequential commands.
Batch is the canonical way to run dependent browser steps in order.

```bash
# Run open + snapshot in one call
agent-browser batch "open https://example.com" "snapshot -i"

# Run multiple interactions
agent-browser batch "fill @e1 \"user@example.com\"" "fill @e2 \"password123\"" "click @e3"

# Navigate and capture
agent-browser batch "open https://example.com" "screenshot"
```

Run commands separately only when you must read intermediate output before deciding the next step, for example `snapshot -i` to discover refs. Plain shell chaining with `&&` is acceptable for human use, but it is not the preferred skill pattern for agents.

## Real Chrome and Anti-Bot Sites

For sites that aggressively challenge automation, start with:

1. `--headed`
2. `--profile Default` or another real Chrome profile
3. `--executable-path /usr/bin/google-chrome-stable` when you specifically want Google Chrome instead of the bundled browser

Example:

```bash
agent-browser --headed --profile Default --executable-path /usr/bin/google-chrome-stable open https://www.google.com
```

This is materially more reliable than a fresh headless session for sites like Google, Gmail, and other consumer properties that react to automation signals.

## Handling Authentication

By default, agent-browser uses a stable runtime profile at `~/.agent-browser/runtime-profiles/default/user-data`. If a user signs in manually once, later runs reuse that state automatically. Use `--runtime-profile <name>` for a named managed profile, or `--profile <path>` for a custom user-data-dir path.

When the default runtime profile is locked by a live browser PID, do not treat a fresh isolated profile as the generic safe fallback. agent-browser is meant to own session and job management so operators do not have to coordinate which browser is busy. If the task needs existing login state, inspect `agent-browser service status`, `agent-browser runtime status`, or the dashboard service view, then reuse the managed runtime profile through the service/session control plane or attach to the intended browser. Switch to a new isolated profile only for explicitly unauthenticated throwaway QA, or when the operator asked for a separate browser identity. When a selected managed runtime profile already has a live agent-browser browser with a DevTools port, normal launch commands automatically reuse that browser through the session control plane instead of trying to start a second Chrome on the locked profile.

Runtime profiles can also be declared in `agent-browser.json` via
`defaultRuntimeProfile` and `runtimeProfiles.<name>`. Today that config can
drive `userDataDir`, launch defaults, auth session naming, and service login
hints. If navigation returns a warning for a service with
`manualLoginPreferred`, switch to `runtime login` for detached manual sign-in,
then relaunch the same runtime profile for automation. Use
`runtime login <url> --attachable` followed by `runtime attach` only for sites
where DevTools during manual login is known to be accepted. Treat
`runtimeProfiles.<name>.preferences.defaultViewport` as a `WIDTHxHEIGHT` default
content size, for example `960x640`. Set `runtimeProfiles.<name>.launch.leaveOpen` or pass
`--leave-open` when you want `close` to detach from a managed runtime-profile
browser instead of shutting it down.

To create and track a managed profile explicitly, use:

```bash
agent-browser runtime create work --set-default
```

When automating a site that requires login, prefer intent-based service use
over manual profile coordination. Ask for the target site or login identity,
include `serviceName`, `agentName`, and `taskName` when available, and let
agent-browser select or reuse the managed profile and browser through its
queue. Request a specific managed runtime profile only when you know that
profile has the needed login state. Use `--profile <path>` only when bringing
an external profile is part of the contract. The default assumption is that
agent-browser owns browser and profile coordination so operators and agents do
not waste effort avoiding another job's browser.

Do not create a new runtime profile merely because another automation is using
agent-browser or because a site has a login. First inspect the service profile
set and ask for the desired target identity. For software clients, call
`getServiceAccessPlan()` with `serviceName`, `agentName`, `taskName`, and
`loginId`, `siteId`, or `targetServiceId` before registering a profile. Then
request the tab by the same identity through `requestServiceTab()` or
`POST /api/service/request`. Use `getServiceProfiles()` or
`getServiceProfileReadiness({ id })` only for narrower inspection when you
already know which profile you are evaluating. Register a new managed login
profile only when agent-browser has no suitable profile, readiness reports
`needs_manual_seeding`, the operator wants a separate account lane, or the
client is explicitly bringing its own profile.

When automating a site that requires login, choose the approach that fits:

**Option 1: Import auth from the user's browser (fastest for one-off tasks)**

```bash
# Connect to the user's running Chrome (they're already logged in)
agent-browser --auto-connect state save ./auth.json
# Use that auth state
agent-browser --state ./auth.json open https://app.example.com/dashboard
```

State files contain session tokens in plaintext. Add them to `.gitignore` and delete them when no longer needed. Set `AGENT_BROWSER_ENCRYPTION_KEY` for encryption at rest.

**Option 2: Runtime profile manual login (best default for recurring manual sign-in)**

```bash
# First run: launch a detached manual-login browser
agent-browser runtime login https://accounts.google.com

# Inspect the runtime profile set before automation uses it
agent-browser runtime list
agent-browser runtime status

# Later runs reuse the same runtime profile automatically
agent-browser open https://gmail.com
```

If the workflow needs automation to bind to the still-open manual browser, use
an attachable manual launch:

```bash
agent-browser runtime login https://example.com --attachable
agent-browser runtime attach
```

For Google, Gmail, and similar SSO flows, do not use `--attachable` for the
initial sign-in. Live testing showed Google can reject sign-in when DevTools is
enabled during the login ceremony, even with otherwise minimal Chrome flags.
Use this two-phase flow instead:

```bash
# Phase 1: user signs in without DevTools
agent-browser --runtime-profile google-login runtime login https://accounts.google.com
# User closes Chrome after sign-in

# Phase 2: automation reuses the signed-in profile
agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
agent-browser --runtime-profile google-login runtime status
agent-browser --runtime-profile google-login get url
agent-browser --runtime-profile google-login get title
```

Run those post-relaunch reads sequentially. Do not issue the first `get url`
and `get title` in parallel immediately after the attachable relaunch, because
live testing found a daemon-startup race in that exact pattern even when the
profile itself was healthy.

If phase 2 `runtime status` shows `Browser alive: true`, `get url` returns
`https://myaccount.google.com/`, and `get title` returns `Google Account`, the
profile is authenticated and safe for agent browsing.

If the first read after attachable relaunch fails but `runtime status` already
shows a live browser, retry the read sequentially before treating the profile
as broken.

For anti-bot-sensitive sites, prefer a real profile plus a visible window:

```bash
agent-browser --headed --profile Default --executable-path /usr/bin/google-chrome-stable open https://www.google.com
```

Use `--runtime-profile <name>` when you need a separate persistent managed profile:

```bash
agent-browser runtime create work --set-default
agent-browser --runtime-profile work runtime login https://app.example.com/login
agent-browser --runtime-profile work runtime login https://app.example.com/login --attachable
agent-browser --runtime-profile work --leave-open open https://app.example.com
agent-browser runtime attach work
agent-browser runtime list
agent-browser --runtime-profile work open https://app.example.com
```

`runtime list` merges config-declared runtime profiles with on-disk managed
profiles. If `runtimeProfiles.<name>.userDataDir` is set in config, both
`runtime list` and `runtime status` report that configured path even before the
browser has written runtime state.

For unattended runs that still need a real OS credential-store-backed Chrome profile, prefer a dotenv-backed setup over putting secrets on the command line:

```bash
cat > ~/.agent-browser/.env <<'EOF'
AGENT_BROWSER_USE_REAL_KEYCHAIN=1
AGENT_BROWSER_KEYCHAIN_PASSWORD='your-login-keychain-password'
EOF

agent-browser open https://example.com
```

agent-browser loads `AGENT_BROWSER_ENV_FILE` first when set, otherwise `~/.agent-browser/.env` if it exists. Environment variables still override file values. On macOS, `AGENT_BROWSER_KEYCHAIN_PASSWORD` unlocks the login keychain before Chrome launches. On Linux, it is used with `gnome-keyring-daemon --unlock --components=secrets`, which is useful on Ubuntu and other GNOME-keyring setups.

**Option 3: Persistent profile path (for a separate recurring task profile)**

```bash
# First run: login manually or via automation
agent-browser --profile ~/.myapp open https://app.example.com/login
# ... fill credentials, submit ...

# All future runs: already authenticated
agent-browser --profile ~/.myapp open https://app.example.com/dashboard
```

**Option 4: Session name (auto-save/restore cookies + localStorage)**

```bash
agent-browser --session-name myapp open https://app.example.com/login
# ... login flow ...
agent-browser close  # State auto-saved

# Next time: state auto-restored
agent-browser --session-name myapp open https://app.example.com/dashboard
```

**Option 5: Auth vault (credentials stored encrypted, login by name)**

```bash
echo "$PASSWORD" | agent-browser auth save myapp --url https://app.example.com/login --username user --password-stdin
agent-browser auth login myapp
```

`auth login` navigates with `load` and then waits for login form selectors to appear before filling/clicking, which is more reliable on delayed SPA login screens.

**Option 6: State file (manual save/load)**

```bash
# After logging in:
agent-browser state save ./auth.json
# In a future session:
agent-browser state load ./auth.json
agent-browser open https://app.example.com/dashboard
```

See [references/authentication.md](references/authentication.md) for OAuth, 2FA, cookie-based auth, and token refresh patterns.

## Essential Commands

```bash
# Batch: ALWAYS use batch for 2+ sequential commands. Commands run in order.
agent-browser batch "open https://example.com" "snapshot -i"
agent-browser batch "open https://example.com" "screenshot"
agent-browser batch "click @e1" "wait 1000" "screenshot"

# Navigation
agent-browser open <url>              # Navigate (aliases: goto, navigate)
agent-browser close                   # Close browser
agent-browser close --all             # Close all active sessions

# Snapshot
agent-browser snapshot -i             # Interactive elements with refs (recommended)
agent-browser snapshot -i --urls      # Include href URLs for links
agent-browser snapshot -s "#selector" # Scope to CSS selector

# Interaction (use @refs from snapshot)
agent-browser click @e1               # Click element
agent-browser click @e1 --new-tab     # Click and open in new tab
agent-browser fill @e2 "text"         # Clear and type text
agent-browser type @e2 "text"         # Type without clearing
agent-browser select @e1 "option"     # Select dropdown option
agent-browser check @e1               # Check checkbox
agent-browser press Enter             # Press key
agent-browser keyboard type "text"    # Type at current focus (no selector)
agent-browser keyboard inserttext "text"  # Insert without key events
agent-browser scroll down 500         # Scroll page
agent-browser scroll down 500 --selector "div.content"  # Scroll within a specific container

# Get information
agent-browser get text @e1            # Get element text
agent-browser get url                 # Get current URL
agent-browser get title               # Get page title
agent-browser get cdp-url             # Get CDP WebSocket URL

# Wait
agent-browser wait @e1                # Wait for element
agent-browser wait 2000               # Wait milliseconds
agent-browser wait --url "**/page"    # Wait for URL pattern
agent-browser wait --text "Welcome"   # Wait for text to appear (substring match)
agent-browser wait --load networkidle # Wait for network idle (caution: see Pitfalls)
agent-browser wait --fn "!document.body.innerText.includes('Loading...')"  # Wait for text to disappear
agent-browser wait "#spinner" --state hidden  # Wait for element to disappear

# Downloads
agent-browser download @e1 ./file.pdf          # Click element to trigger download
agent-browser wait --download ./output.zip     # Wait for any download to complete
agent-browser --download-path ./downloads open <url>  # Set default download directory

# Tab management
agent-browser tab list                         # List all open tabs
agent-browser tab new                          # Open a blank new tab
agent-browser tab new https://example.com      # Open URL in a new tab
agent-browser tab 2                            # Switch to tab by index (0-based)
agent-browser tab close                        # Close the current tab
agent-browser tab close 2                      # Close tab by index

# Network
agent-browser network requests                 # Inspect tracked requests
agent-browser network requests --type xhr,fetch  # Filter by resource type
agent-browser network requests --method POST   # Filter by HTTP method
agent-browser network requests --status 2xx    # Filter by status (200, 2xx, 400-499)
agent-browser network request <requestId>      # View full request/response detail
agent-browser network route "**/api/*" --abort  # Block matching requests
agent-browser network har start                # Start HAR recording
agent-browser network har stop ./capture.har   # Stop and save HAR file

# Viewport & Device Emulation
agent-browser set viewport 1920 1080          # Set viewport size (default: 1280x720)
agent-browser set viewport 1920 1080 2        # 2x retina (same CSS size, higher res screenshots)
agent-browser set device "iPhone 14"          # Emulate device (viewport + user agent)

# Capture
agent-browser screenshot              # Screenshot to temp dir
agent-browser screenshot --full       # Full page screenshot
agent-browser screenshot --annotate   # Annotated screenshot with numbered element labels
agent-browser screenshot --screenshot-dir ./shots  # Save to custom directory
agent-browser screenshot --screenshot-format jpeg --screenshot-quality 80
agent-browser pdf output.pdf          # Save as PDF

# Live preview / streaming
agent-browser stream enable           # Start runtime WebSocket streaming on an auto-selected port
agent-browser stream enable --port 9223  # Bind a specific localhost port
agent-browser stream status           # Inspect enabled state, port, connection, and screencasting
agent-browser stream disable          # Stop runtime streaming and remove the .stream metadata file
agent-browser service status          # Inspect service control-plane and configured service entities
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

# Clipboard
agent-browser clipboard read                      # Read text from clipboard
agent-browser clipboard write "Hello, World!"     # Write text to clipboard
agent-browser clipboard copy                      # Copy current selection
agent-browser clipboard paste                     # Paste from clipboard

# Dialogs (alert, confirm, prompt, beforeunload)
# By default, alert and beforeunload dialogs are auto-accepted so they never block the agent.
# confirm and prompt dialogs still require explicit handling.
# Use --no-auto-dialog (or AGENT_BROWSER_NO_AUTO_DIALOG=1) to disable automatic handling.
agent-browser dialog accept              # Accept dialog
agent-browser dialog accept "my input"   # Accept prompt dialog with text
agent-browser dialog dismiss             # Dismiss/cancel dialog
agent-browser dialog status              # Check if a dialog is currently open

# Diff (compare page states)
agent-browser diff snapshot                          # Compare current vs last snapshot
agent-browser diff snapshot --baseline before.txt    # Compare current vs saved file
agent-browser diff screenshot --baseline before.png  # Visual pixel diff
agent-browser diff url <url1> <url2>                 # Compare two pages
agent-browser diff url <url1> <url2> --wait-until networkidle  # Custom wait strategy
agent-browser diff url <url1> <url2> --selector "#main"  # Scope to element

# Chat (AI natural language control)
agent-browser chat "open google.com and search for cats"  # Single-shot instruction
agent-browser chat                                        # Interactive REPL mode
agent-browser -q chat "summarize this page"               # Quiet (text only, no tool calls)
agent-browser -v chat "fill in the login form"            # Verbose (show command output)
agent-browser --model openai/gpt-4o chat "take a screenshot"  # Override model
```

## Typing Strategy

Rule of thumb:

- Prefer `fill` for ordinary forms.
- Prefer `focus` + `keyboard type` + `press Enter` for search boxes, chat inputs, and sites where synthetic value injection behaves differently from human typing.

Google example:

```bash
agent-browser --headed --profile Default --executable-path /usr/bin/google-chrome-stable open https://www.google.com
agent-browser snapshot -i
agent-browser focus @e15
agent-browser keyboard type "Soylei"
agent-browser press Enter
```

In live testing, this was more reliable than:

```bash
agent-browser fill @e15 "Soylei"
agent-browser press Enter
```

## Streaming

Streaming is opt-in. Use `agent-browser stream enable` to start a runtime WebSocket stream server, then `agent-browser stream status` to inspect the bound port and connection state. Use `stream disable` to tear it down, and `stream enable --port <port>` to bind a specific port.

## Service Status

Use `agent-browser service status` for a service-mode snapshot. It reports worker state, browser health, queue depth, profile lease wait pressure, persisted service state from `~/.agent-browser/service/state.json`, and configured service-mode profiles, sessions, monitors, site policies, and providers without launching a browser. In text mode, it summarizes profiles, the derived `profileAllocations` view, browsers, sessions, and profile lease wait pressure with service, agent, task, profile, profile selection reason, profile lease disposition, lease conflicts, lease, cleanup, browser linkage, health, and retained observation fields. `profileAllocations` is also returned by `agent-browser service profiles`, `GET /api/service/profiles`, `GET /api/service/status`, and `agent-browser://profiles`; use `GET /api/service/profiles/<profile-id>/allocation` or `getServiceProfileAllocation()` when a software client needs one allocation row. Use `GET /api/service/profiles/<profile-id>/readiness` or `getServiceProfileReadiness()` when a client only needs one profile's no-launch target readiness. Use `GET /api/service/access-plan`, `getServiceAccessPlan()`, or MCP `agent-browser://access-plan?serviceName=<name>&agentName=<agent>&taskName=<task>&loginId=<id>` when a client needs the broader no-launch recommendation that combines selected profile, readiness, site policy, enabled providers, retained challenges, caller-label warnings, and decision fields before requesting browser control. The allocation view lists holder sessions, exclusive holders, waiting profile-lease jobs, conflict session IDs, related service, agent, and task labels, linked browsers and tabs, the current lease state, and the recommended next action for each known profile. Configured profiles and sessions can describe profile allocation, keyring posture, caller ownership, profile binding, target-service login scope, believed authenticated target services, profile selection reason, profile lease disposition, lease state, and cleanup policy. Explicit `--runtime-profile` and `--profile` values still win. When a launch command omits both, `serviceName` plus `targetServiceId`, `targetService`, `targetServiceIds`, `targetServices`, `siteId`, `siteIds`, `loginId`, or `loginIds` lets agent-browser choose a persisted service profile. The selector first prefers `authenticatedServiceIds` matches, then `targetServiceIds` matches, then the caller `sharedServiceIds` match. Runtime profile and custom profile launches also populate linked service profile and session records, including `serviceName`, `agentName`, `taskName`, `profileSelectionReason`, `profileLeaseDisposition`, and `profileLeaseConflictSessionIds` when the caller provides enough context. Service-scoped launches reject active exclusive profile conflicts by default before browser start; set `profileLeasePolicy: "wait"` and `profileLeaseWaitTimeoutMs` to wait for release instead. Same-session retained browser reuse remains allowed. It refreshes the persisted control-plane snapshot in `state.json` and probes persisted browser records for dead PIDs and unreachable CDP endpoints. Dead local PIDs are marked `process_exited`, unreachable CDP endpoints with a live PID are marked `cdp_disconnected`, unreachable CDP endpoints without a PID are marked `unreachable`, and endpoints that answer health probes but fail target-list discovery are marked `degraded`. Reachable CDP endpoints are queried for live page and webview targets, updating `tabs` and known session/tab relationships in service state. Non-ready browsers close their known tabs during reconciliation so stale tab state does not look active. When reconciliation removes stale session/tab ownership links, it appends a `reconciliation` event with `details.action: "session_tab_ownership_repaired"` and the removed relationships. Browser launch, close, and command-time stale-browser detection update the active session's persisted browser health record. Browser records retain the latest non-ready health evidence in `lastHealthObservation`, so service status clients can inspect failure metadata without reconstructing events. Use `agent-browser service profiles`, `agent-browser service sessions`, `agent-browser service browsers`, `agent-browser service tabs`, `agent-browser service monitors`, `agent-browser service site-policies`, `agent-browser service providers`, and `agent-browser service challenges` for focused collection views without parsing the full status payload. Active monitor records are checked by the daemon scheduler when due; use `agent-browser service monitors run-due`, HTTP `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to check due active monitors immediately. Use `agent-browser service monitors triage <id>`, HTTP `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, or `triageServiceMonitor()` to acknowledge the related monitor incident and clear reviewed failures in one queued operation. Use `agent-browser service monitors reset <id>`, HTTP `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, or `resetServiceMonitorFailures()` when only the reviewed failure count should be cleared without deleting retained failure evidence. Checks update `lastCheckedAt`, `lastSucceededAt`, `lastFailedAt`, `lastResult`, and `consecutiveFailures`, and failed probes set the monitor `state` to `faulted` and append a service incident event. Unexpected process-exit health and recovery events include `details.processExitCause: "unexpected_process_exit"` and `details.failureClass: "browser_process_exited"`; active local Chrome exits also include `processExitDetection`, `processExitPid`, and exit code or signal when available. During close, agent-browser first tries a polite browser shutdown and then force kills owned browser processes if needed; close-generated health events include `details.shutdownReasonKind: "operator_requested_close"`, `details.processExitCause: "operator_requested_close"`, and shutdown outcome flags. Polite shutdown failure leaves the browser record `degraded`, and force-kill failure leaves it `faulted` with an OS-degraded warning. When a queued command finds the active browser process exited or the CDP connection disconnected, agent-browser records that failure before cleanup and relaunch. Configured profiles, sessions, monitors, site policies, and providers override entries with the same IDs from the persisted state.

For release or pre-release checks that touch service mode, validate the public contract with `pnpm test:service-api-mcp-parity`, `pnpm test:service-contracts-no-launch`, `pnpm test:service-profile-lookup-no-launch`, `pnpm test:service-incident-summary-http`, `pnpm test:service-incident-summary-mcp`, `cd docs && pnpm build`, and `git diff --check`. If any Rust source under `cli/src/` changed anywhere in the current slice, run `cargo fmt --manifest-path cli/Cargo.toml -- --check` and `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings` before pushing, even when the final commit only changes docs, examples, or client code. Add `pnpm test:service-request-live` when HTTP `/api/service/request` or MCP `service_request` changed. Add `pnpm test:service-profile-lookup-no-launch` when HTTP `/api/service/profiles/lookup`, profile target readiness, or target profile selection changed. Add `pnpm test:service-profile-lease-wait-live` during manual full-CI or release-gating checks that touch profile selection, profile lease waiting, service request, trace summary, dashboard trace, or service observability client behavior. Add `pnpm test:service-health-live` when Chrome-backed live validation is required, and add `cargo test --manifest-path cli/Cargo.toml service_ -- --test-threads=1` when service implementation changed. Fast CI runs the no-launch service contract metadata smoke, the no-launch profile-source smoke, the no-launch site-policy source smoke, and both no-launch incident-summary smokes after the Rust suite. Service request action changes must keep `SERVICE_REQUEST_ACTIONS`, `docs/dev/contracts/service-request.v1.schema.json`, MCP `service_request`, HTTP `/api/service/request`, and generated `@agent-browser/client` helpers aligned; use `pnpm test:service-api-mcp-parity`, `pnpm test:service-client-contract`, `cargo test --manifest-path cli/Cargo.toml service_request_schema_and_command_accept_contract_actions`, and `cargo test --manifest-path cli/Cargo.toml service_request_command_accepts_contract_actions` for the targeted no-launch action-parity guard set. Run both no-launch incident-summary smokes before changing incident summary grouping or filters; together they guard HTTP `summary=true` and MCP `service_incidents` with `summary: true` across state, severity, escalation, handling-state, browser, profile, session, service, agent, task, and since filters.

With `profileLeasePolicy: "wait"`, the control-plane scheduler keeps the blocked request queued while it polls for profile release, so the worker can continue dispatching unrelated service requests.

Run `pnpm test:service-site-policy-sources-no-launch` to validate that HTTP `GET /api/service/site-policies`, MCP `agent-browser://site-policies`, and `getServiceSitePolicies()` report config, persisted-state, and built-in site-policy source metadata without launching Chrome. Run `pnpm test:service-collections-live` to validate that CLI, HTTP, and MCP expose matching service-owned profile, session, browser, tab, monitor, site-policy, provider, and challenge collections for one live runtime-profile session.

Use `agent-browser service status --watch` or `agent-browser service watch` for a polling operator view of worker health, browser health, queue depth, profile lease wait pressure, and reconciliation status. Add `--interval <ms>` to set the poll interval and `--count <n>` for bounded scripts. In JSON mode, each poll is emitted as one JSON response line.

Launch-shaping options such as `--args` or `AGENT_BROWSER_ARGS` apply only to commands that can launch a browser. Service inspection commands such as `service status`, `service sessions`, and `mcp read` must remain read-only and must not start Chrome just because launch defaults are configured.

Run `pnpm test:service-status-no-launch` to validate that service status remains read-only when launch defaults such as `AGENT_BROWSER_ARGS` are configured.
Run `pnpm test:service-contracts-no-launch` to validate that HTTP `GET /api/service/contracts` returns compatibility metadata without launching or recording a browser, and that `getServiceContracts()` exposes profile lookup/readiness client-helper metadata to software clients.
Run `pnpm test:service-profile-lookup-no-launch` to validate that HTTP `GET /api/service/profiles/lookup` selects an authenticated target profile over a target-only profile from seeded temporary service state without launching a browser. The same smoke calls `lookupServiceProfile()` against the live stream server so the software-client helper is covered end to end. Profile collections include `profileSources`, and profile lookup plus access-plan responses include `selectedProfileSource`, so callers can distinguish config, runtime-observed, and persisted profile provenance. Run `pnpm test:service-profile-sources-no-launch` when changing effective profile source metadata. For the broader no-launch recommendation, use `GET /api/service/access-plan`, `getServiceAccessPlan()`, or MCP `agent-browser://access-plan?serviceName=<name>&agentName=<agent>&taskName=<task>&loginId=<id>` so agent-browser can combine profile selection, readiness, site policy, providers, retained challenges, caller-label warnings, and the recommended action before a caller requests a tab. Access-plan readiness decisions are scoped to the requested target identities, so an unrelated stale or unseeded login on the same profile does not block the requested site. The decision includes auth provider IDs, challenge provider IDs, challenge strategy, missing challenge-provider capabilities, interaction risk, pacing details derived from site-policy rate limits, launch posture for headed, headless, remote-view, or detached-seeding behavior, and `freshnessUpdate` instructions with the selected profile, target identities, HTTP route, MCP tool, and `updateServiceProfileFreshness` helper to use after a bounded auth probe. Both `query` and `decision` include `namingWarnings` plus `hasNamingWarning` when `serviceName`, `agentName`, or `taskName` is missing. Google, Gmail, and Microsoft have shipped default site policies when no local policy overrides them. `sitePolicySource` explains whether the selected policy came from config, persisted state, or a built-in default. Run `pnpm test:service-access-plan-no-launch` when changing that surface; it checks HTTP, MCP, and the service client helper against the same seeded temporary service state without creating browsers or browser-launching jobs. Run `pnpm test:service-site-policy-sources-no-launch` when changing effective site-policy collection source metadata.

Run `pnpm test:mcp-read-no-launch` to validate that MCP resource reads remain read-only under the same launch defaults.

Use `agent-browser service reconcile` to run persisted browser health and target probes intentionally without requesting a control-plane status snapshot. It updates `~/.agent-browser/service/state.json`, refreshes live tab records for reachable browser CDP endpoints, and returns the reconciled service state plus total and changed browser counts. The persisted service state includes a `reconciliation` snapshot with `lastReconciledAt`, `browserCount`, `changedBrowsers`, and `lastError`, bounded recent control-plane job records in `jobs`, a derived `incidents` collection grouped by browser or service scope, and a bounded `events` log for reconciliation summaries, browser health transitions, browser recovery starts, profile lease wait transitions, ownership repairs, and tab lifecycle changes.

Use `agent-browser service cancel <job-id>` to mark a queued or lease-waiting service job cancelled before it dispatches or request cooperative cancellation for a running job. Running cancellation drops the active service future, records the job as `cancelled`, and cleans up browser state before the worker accepts more work. Terminal jobs are rejected rather than rewritten. Add `--reason <text>` to record why a queued job was cancelled.

Use `agent-browser service retry <browser-id> --by <operator> --note <text>` to explicitly allow one new recovery attempt for a faulted browser. It records a `browser_recovery_override` event, moves the browser back to a retryable stale health state, and resets retry counting from that override boundary. HTTP retry requests accept `service-name`, `agent-name`, and `task-name` query parameters, and MCP `service_browser_retry` accepts `serviceName`, `agentName`, and `taskName`, so override events appear in filtered service traces.

Use `agent-browser service acknowledge <incident-id>` to mark a retained incident seen by an operator. Add `--by <text>` to record who acknowledged it and `--note <text>` to persist a short operator note.

Use `agent-browser service resolve <incident-id>` to mark a retained incident handled while preserving the derived incident record. Add `--by <text>` to record who resolved it and `--note <text>` to persist a resolution note.

Acknowledgement and resolution also append retained service events with `incident_acknowledged` and `incident_resolved` kinds. Incident detail includes those handling events alongside the health and job events that define the grouped incident. Use `agent-browser service activity <incident-id>` to fetch a normalized chronological timeline for one retained incident without reconstructing it client-side.

The activity response is the canonical agent-facing incident timeline. It returns `{ incident, activity, count }`. Each activity item includes `id`, `source`, `timestamp`, `kind`, `title`, and `message`, plus `eventId` or `jobId` when it came from a retained event or job. Event and job items include trace context fields such as `browserId`, `profileId`, `sessionId`, `serviceName`, `agentName`, and `taskName` when known, so clients can display one timeline without rejoining raw records. Older retained incidents can include `source: "metadata"` acknowledgement or resolution items when handling metadata predates retained handling events.

Use `agent-browser service trace --service-name <name> --task-name <name>` to inspect related events, jobs, incidents, and normalized activity in one response. Add `--limit <n>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--agent-name <name>`, or `--since <timestamp>` to narrow a trace view for one service, agent, task, browser, profile, session, or time window. Prefer this command when a client needs a complete service/task timeline without issuing separate jobs, incidents, events, and incident activity requests. The response includes a `summary` object with compact service, agent, task, browser, profile, and session context rows plus per-context record counts, target identity hints, and naming warnings for missing service, agent, or task labels when debugging multi-agent runs. `summary.contexts[].targetServiceIds` lists the normalized target-service, site, and login identity hints observed on retained jobs in that context; text output shows them as `targets=...`. `summary.profileLeaseWaits` provides a per-job profile lease wait rollup with outcome, timing, conflict sessions, and trace labels. Text output includes a `Profile lease waits` block when that summary or the raw wait events are present. For crash recovery, read the `events` array in order: `browser_health_changed` with `currentHealth` such as `process_exited` or `cdp_disconnected` and `details.currentReasonKind`, `browser_recovery_started` with `details.reasonKind`, `details.reason`, `details.attempt`, `details.retryBudget`, `details.nextRetryDelayMs`, and `details.policySource`, then `browser_health_changed` with `currentHealth: "ready"` after relaunch. Stale health and recovery events also include `details.failureClass`, such as `browser_process_exited`, `cdp_unresponsive`, `cdp_endpoint_unreachable`, or `target_discovery_failed`. Process-exit health and recovery events also include `details.processExitCause: "unexpected_process_exit"`. Active local Chrome process exits include `details.processExitDetection: "local_child_try_wait"`, `details.processExitPid`, and `details.processExitCode` or `details.processExitSignal` when available. Operator-requested shutdown health events instead include `details.shutdownReasonKind: "operator_requested_close"` and `details.processExitCause: "operator_requested_close"` plus polite-close and force-kill outcome flags, so agents can separate clean or degraded closes from unexpected exits. If the next attempt would exceed the configured retry budget, the browser is marked `faulted` and the command fails instead of relaunching. Operators can use `service retry <browser-id>` to record a `browser_recovery_override` event and allow a new recovery attempt. HTTP clients get the same sequence from `/api/service/trace`; MCP clients get it from the `service_trace` tool.

Use MCP `service_request` when a service wants to queue one intent-based browser action with caller context, site or login hints, explicit `profile` or `runtimeProfile` hints, `action`, `params`, and `jobTimeoutMs` in a single request object. This is the service-oriented equivalent of `browser_command` for clients that want agent-browser to own profile selection, queueing, and trace metadata deterministically.

Retained service jobs preserve target identity hints for profile debugging. Singular `targetServiceId`, `siteId`, and `loginId` fields keep exact singular request values when present, and `targetServiceIds` stores the normalized target-service, site, and login identity set used for profile selection. Prefer these fields in `service jobs`, HTTP jobs, MCP `agent-browser://jobs`, and service trace output instead of reconstructing identity from the original command payload.

HTTP service request objects follow `docs/dev/contracts/service-request.v1.schema.json`. MCP `service_request` tool-call wrappers follow `docs/dev/contracts/service-request-mcp-tool-call.v1.schema.json`. HTTP `GET /api/service/contracts` and MCP `agent-browser://contracts` expose matching compatibility metadata for service request schema IDs, contract versions, routes, MCP tool names, and supported actions. The HTTP contracts metadata also advertises the HTTP-only `serviceProfileAllocationResponse` contract for `GET /api/service/profiles/<id>/allocation` and `serviceProfileReadinessResponse` for `GET /api/service/profiles/<id>/readiness`, and `serviceProfileLookupResponse` for `GET /api/service/profiles/lookup`. Readiness and lookup metadata names the `@agent-browser/client/service-observability` helpers and the lookup selection order.

Software clients in this repo can use `@agent-browser/client/service-request` for generated `createServiceRequest`, `createServiceRequestMcpToolCall`, `postServiceRequest`, `createServiceTabRequest`, `requestServiceTab`, and TypeScript declarations. Use `requestServiceTab` when software asks agent-browser for a queued tab by site or login identity and does not need to hand-build the underlying `tab_new` service request. `postServiceRequest` and `requestServiceTab` return `ServiceRequestResponse`, the standard service command envelope with `success`, optional `data`, optional `error`, optional `warning`, and action-specific fields. Every action currently listed in `docs/dev/contracts/service-request.v1.schema.json` gets a typed `data` shape through `ServiceRequestDataForAction`; `requestServiceTab` returns typed `tab_new` data and preserves `loginId` and `targetServiceId` in the request payload before any browser-launching request is sent. When adding or removing service request actions, update the Rust `SERVICE_REQUEST_ACTIONS` list, the JSON schema enum, MCP `service_request`, HTTP `/api/service/request`, and generated client files together. Service request helpers accept `profileLeasePolicy: "reject"` or `"wait"` plus `profileLeaseWaitTimeoutMs` for callers that want agent-browser to wait for a profile lease rather than fail immediately. Run `pnpm generate:service-client` after changing the service request schemas, then run `pnpm test:service-client` to verify generated files, helper types, package export resolution, typed service command responses, service request helpers, and observability helpers. Use `pnpm test:service-client-contract`, `pnpm test:service-client-types`, `pnpm test:service-client-exports`, `pnpm test:service-request-client`, or `pnpm test:service-observability-client` when only one client contract needs validation.

For `profileLeasePolicy: "wait"`, service request helpers submit a queued request that waits at the scheduler until the lease clears rather than occupying the running worker.

Use `@agent-browser/client/service-observability` for generated HTTP helpers around service status, contract metadata, collections, config mutations, operator remedies, reconcile, jobs, events, incidents, incident activity, and traces. It exposes `getServiceStatus`, `getServiceContracts`, `getServiceProfiles`, `getServiceProfileAllocation`, `getServiceProfileReadiness`, `summarizeServiceProfileReadiness`, `findServiceProfileForIdentity`, `getServiceProfileForIdentity`, `lookupServiceProfile`, `getServiceBrowsers`, `getServiceSessions`, `getServiceTabs`, `getServiceSitePolicies`, `getServiceProviders`, `getServiceChallenges`, `postServiceReconcile`, `upsertServiceProfile`, `registerServiceLoginProfile`, `updateServiceProfileFreshness`, `deleteServiceProfile`, `upsertServiceSession`, `deleteServiceSession`, `upsertServiceSitePolicy`, `deleteServiceSitePolicy`, `upsertServiceProvider`, `deleteServiceProvider`, `cancelServiceJob`, `retryServiceBrowser`, `applyServiceRemedies`, `acknowledgeServiceIncident`, `resolveServiceIncident`, `getServiceJobs`, `getServiceJob`, `getServiceEvents`, `getServiceIncidents`, `getServiceIncident`, `getServiceIncidentActivity`, and `getServiceTrace`. `registerServiceLoginProfile` can also record bounded-probe freshness with `readinessState`, `readinessEvidence`, `lastVerifiedAt`, and `freshnessExpiresAt`; explicit `targetReadiness` rows override generated rows for matching targets. `updateServiceProfileFreshness` posts to the service-side freshness mutation endpoint so agent-browser performs the serialized merge, preserves unrelated fields, and updates `authenticatedServiceIds` for fresh, stale, or blocked target states.

Use `examples/service-client/` as the copyable software-client workflow for `requestServiceTab`, trace inspection, and optional known queued-job cancellation with `cancelServiceJob`. Run `pnpm test:service-client-example` to validate it in dry-run mode, or `pnpm test:service-client-example-live` to validate it against an isolated live daemon and browser session. The same package includes `managed-profile-flow.mjs`, a CanvaCLI-style profile-broker recipe that uses `getServiceAccessPlan()` to inspect the selected profile, readiness, site policy, providers, retained challenges, and service-owned decision before requesting tabs by login identity, registering a managed login profile only when agent-browser has no suitable one, and posting bounded auth-probe evidence with `updateServiceProfileFreshness()` for an existing profile. `getServiceProfileForIdentity()` remains as the older descriptive alias for the same route. Its output includes `readinessSummary.needsManualSeeding` plus target service IDs and recommended actions when readiness says an operator must seed the profile. Run `pnpm test:service-client-managed-profile-flow` for the no-launch mock smoke that proves the recipe selects an existing managed profile without registering a new one.

For software-client profile selection, prefer requesting the target identity instead of naming a browser profile. A service profile with `authenticatedServiceIds: ["acs"]`, `targetServiceIds: ["acs"]`, and `sharedServiceIds: ["JournalDownloader"]` lets `requestServiceTab({ serviceName: "JournalDownloader", loginId: "acs", ... })` select the profile with usable ACS login state. A client such as CanvaCLI should use the same pattern: request `serviceName: "CanvaCLI"` plus `loginId: "canva"` or `targetServiceId: "canva"` and let agent-browser pick or reuse the managed profile. If no managed profile is suitable, register one with `registerServiceLoginProfile({ id: "canva-default", serviceName: "CanvaCLI", loginId: "canva", authenticated: false })`, have the operator seed it when readiness reports `needs_manual_seeding`, then request tabs by login identity rather than by profile name. The selector prefers `authenticatedServiceIds`, then `targetServiceIds`, then `sharedServiceIds`. Retained session records expose `profileSelectionReason` as `authenticated_target`, `target_match`, `service_allow_list`, or `explicit_profile`, `profileLeaseDisposition` as `new_browser`, `reused_browser`, or `active_lease_conflict`, and browser launch events mirror both in `details`. Active exclusive profile conflicts are rejected before a service-scoped launch starts another browser unless `profileLeasePolicy: "wait"` is supplied with a bounded `profileLeaseWaitTimeoutMs`. Use explicit `profile` or `runtimeProfile` only for override workflows where the caller intentionally owns the browser identity choice.

Copyable software-client broker pattern:

```js
import { requestServiceTab } from '@agent-browser/client/service-request';
import {
  lookupServiceProfile,
  registerServiceLoginProfile,
} from '@agent-browser/client/service-observability';

const profileLookup = await lookupServiceProfile({
  baseUrl,
  serviceName: 'CanvaCLI',
  agentName: 'canva-cli-agent',
  taskName: 'openCanvaWorkspace',
  loginId: 'canva',
  targetServiceId: 'canva',
});

if (!profileLookup.selectedProfile) {
  await registerServiceLoginProfile({
    baseUrl,
    id: 'canva-default',
    serviceName: 'CanvaCLI',
    loginId: 'canva',
    authenticated: false,
  });
}

await requestServiceTab({
  baseUrl,
  serviceName: 'CanvaCLI',
  agentName: 'canva-cli-agent',
  taskName: 'openCanvaWorkspace',
  loginId: 'canva',
  targetServiceId: 'canva',
  url: 'https://www.canva.com/',
});
```

If `profileLookup.readinessSummary.needsManualSeeding` is true, show the returned recommended actions to the operator and seed the managed profile before expecting authenticated automation to succeed for the requested identity. After a bounded no-launch auth probe, update an existing managed profile through `updateServiceProfileFreshness({ id, loginId, readinessState: "fresh", readinessEvidence, lastVerifiedAt, freshnessExpiresAt })`, HTTP `POST /api/service/profiles/<id>/freshness`, or MCP `service_profile_freshness_update`. Use `registerServiceLoginProfile({ ..., readinessState: "fresh", readinessEvidence, lastVerifiedAt, freshnessExpiresAt })` only when registering or refreshing the whole login profile recipe.

For MCP clients, use `agent-browser mcp serve` to run a stdio server that exposes service resources without launching a browser. The server supports `initialize`, `ping`, `resources/list`, `resources/templates/list`, `resources/read`, `tools/list`, and `tools/call`. MCP tools include `service_job_cancel`, which cancels queued service jobs or requests cancellation for running jobs, `service_browser_retry`, which enables a new recovery attempt for a faulted browser, `service_incidents`, which reads grouped retained incidents with the same state, severity, escalation, handling, kind, browser, profile, session, service, agent, task, since, and summary filters as CLI and HTTP, `service_trace`, which reads related events, jobs, incidents, and activity from persisted service state, `service_profile_upsert`, `service_profile_delete`, `service_session_upsert`, `service_session_delete`, `service_site_policy_upsert`, `service_site_policy_delete`, `service_monitor_upsert`, `service_monitor_delete`, `service_monitor_pause`, `service_monitor_resume`, `service_monitor_reset_failures`, `service_monitor_triage`, `service_provider_upsert`, and `service_provider_delete`, which mutate persisted service config through the service worker queue with the same ID checks as HTTP, plus `service_monitors_run_due`, which runs due active monitors through the service worker, `browser_navigate`, which queues typed navigation for the active browser session, `browser_requests`, which enables and filters request inspection, `browser_request_detail`, which reads one tracked request by ID, `browser_headers`, which sets extra HTTP headers for the active browser session, `browser_offline`, which toggles network offline emulation, `browser_cookies_get`, which reads cookies, `browser_cookies_set`, which sets cookies, `browser_cookies_clear`, which clears cookies, `browser_storage_get`, which reads localStorage or sessionStorage, `browser_storage_set`, which sets localStorage or sessionStorage, `browser_storage_clear`, which clears localStorage or sessionStorage, `browser_user_agent`, which sets the user agent, `browser_viewport`, which sets the viewport, `browser_geolocation`, which sets geolocation emulation, `browser_permissions`, which grants browser permissions, `browser_timezone`, which sets timezone emulation, `browser_locale`, which sets locale emulation, `browser_media`, which sets media emulation, `browser_dialog`, which handles dialog status or response, `browser_upload`, which uploads files, `browser_download`, which clicks and saves downloads, `browser_wait_for_download`, which waits for downloads, `browser_har_start` and `browser_har_stop`, which capture HAR files, `browser_route`, which routes matching requests, `browser_unroute`, which removes routes, `browser_console`, which reads or clears console messages, `browser_errors`, which reads page errors, `browser_pdf`, which saves PDFs, `browser_response_body`, which reads matching response bodies, `browser_clipboard`, which controls clipboard operations, `browser_command`, which queues any supported browser-control action for HTTP parity, `browser_snapshot`, which queues the existing snapshot command for the active browser session, `browser_get_url`, which reads the active browser URL, `browser_get_title`, which reads the active browser title, `browser_tabs`, which lists open tabs, `browser_screenshot`, which saves a screenshot for visual inspection, `browser_click`, which clicks a selector or cached ref through the queued control plane, `browser_fill`, which fills a field through the queued control plane, `browser_wait`, which waits for selector, text, URL, function, load-state, or fixed-duration conditions through the queued control plane, `browser_type`, which types text through the queued control plane, `browser_press`, which presses keys and key chords through the queued control plane, `browser_hover`, which hovers elements through the queued control plane, `browser_select`, which selects dropdown values through the queued control plane, `browser_get_text`, which reads element text through the queued control plane, `browser_get_value`, which reads field values through the queued control plane, `browser_get_attribute`, which reads element attributes through the queued control plane, `browser_get_html`, which reads element inner HTML through the queued control plane, `browser_get_styles`, which reads computed styles through the queued control plane, `browser_count`, which counts matching elements through the queued control plane, `browser_get_box`, which reads element geometry through the queued control plane, `browser_is_visible`, which reads element visibility through the queued control plane, `browser_is_enabled`, which reads element enabled state through the queued control plane, `browser_check`, which checks checkbox or radio controls through the queued control plane, `browser_is_checked`, which reads checkbox, radio, or ARIA checked state through the queued control plane, `browser_uncheck`, which unchecks checkbox controls through the queued control plane, `browser_scroll`, which scrolls pages or containers through the queued control plane, `browser_scroll_into_view`, which scrolls a target element into view through the queued control plane, `browser_focus`, which focuses a target element through the queued control plane, and `browser_clear`, which clears a target field through the queued control plane. MCP tool callers should include `serviceName`, `agentName`, and `taskName` when available so multi-service and multi-agent behavior remains traceable. Service jobs persist these caller context fields when commands provide them and persist advisory `namingWarnings` when any caller label is missing. Access-plan responses echo the same caller labels and report the same naming warnings in `query` and `decision`. Run `pnpm test:mcp-live` to validate the live daemon, browser, MCP tool call, and retained job metadata path. Run `pnpm test:service-reconcile-live` to validate that `service reconcile` and MCP browser/tab resources agree on live service-owned state. Run `pnpm test:service-profile-live` to validate that runtime-profile launches populate MCP profile and session resources with caller metadata. Run `pnpm test:service-profile-http-live` to validate the same profile and session metadata through the HTTP service API. Run `pnpm test:service-recovery-http-live` to validate the HTTP trace contract for crash detection, recovery start, and ready-after-relaunch events. Run `pnpm test:service-recovery-mcp-live` to validate the same recovery trace contract through MCP `service_trace`. Run `pnpm test:service-config-live` to validate HTTP and MCP mutation parity for persisted profiles, sessions, site policies, monitors, and providers. Run `pnpm test:service-api-mcp-parity` to statically check that named browser-control HTTP endpoints, typed MCP tools, README, skill, and docs site stay aligned. For shell inspection, use `agent-browser mcp resources` to list service resource contracts and `agent-browser mcp read <uri>` to read one resource from persisted service state. Implemented resources include `agent-browser://contracts`, `agent-browser://access-plan`, `agent-browser://access-plan{?serviceName,agentName,taskName,targetServiceId,targetServiceIds,siteId,siteIds,loginId,loginIds,sitePolicyId,challengeId,readinessProfileId}`, `agent-browser://incidents`, `agent-browser://profiles`, `agent-browser://sessions`, `agent-browser://browsers`, `agent-browser://tabs`, `agent-browser://monitors`, `agent-browser://site-policies`, `agent-browser://providers`, `agent-browser://challenges`, `agent-browser://jobs`, `agent-browser://events`, and `agent-browser://incidents/{incident_id}/activity`.

The MCP `agent-browser://monitors` resource returns retained service monitor records for heartbeat, tab, site-policy, and login freshness probes without launching Chrome. For operator triage, `agent-browser service monitors --summary --failed`, `GET /api/service/monitors?summary=true&failed=true`, and `getServiceMonitors({ summary: true, failedOnly: true })` return compact failure counts, repeated-failure counts, never-checked counts, and matching monitor IDs. Add `--state faulted` or `state: "faulted"` to focus on faulted monitors. Use `agent-browser service monitors run-due`, `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to run due active monitor checks immediately. Use monitor pause/resume commands or helpers to change only the retained monitor state while preserving health evidence. Use `agent-browser service monitors triage <id>`, `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, or `triageServiceMonitor()` to acknowledge the related monitor incident and clear reviewed failures in one queued operation. Use `agent-browser service monitors reset <id>`, `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, or `resetServiceMonitorFailures()` when only the failure counter should be cleared without deleting the last failed probe evidence. MCP keeps the unfiltered resource shape for stable agent reads.

Run `pnpm test:service-profile-target-mcp-live` to validate that typed MCP `browser_navigate` target hints select the authenticated target service profile.
Run `pnpm test:service-profile-lease-wait-live` to validate queued profile lease contention and the `service_trace.summary.profileLeaseWaits` rollup during manual full-CI or release-gating checks.

Profile selection prefers a profile with credentials and usable auth state for the target site or identity provider, not merely the profile owned by the calling service. Profile records use `sharedServiceIds` for caller services allowed to use the profile, `targetServiceIds` for target sites or identity providers whose credentials or login state should live in the profile, and `authenticatedServiceIds` for targets currently believed to have usable authenticated state. Include `targetServiceId`, `targetService`, `targetServiceIds`, `targetServices`, `siteId`, `siteIds`, `loginId`, or `loginIds` in launch, HTTP, or MCP browser-tool payloads when no explicit profile is supplied and the target auth scope matters. Inspect `profileSelectionReason` on sessions to verify the selector used authenticated target state before falling back to target scope or caller service sharing. Inspect `profileLeaseDisposition` and `profileLeaseConflictSessionIds` to see whether the selected profile started a new browser, reused a retained session browser, or hit another exclusive profile lease. Service-scoped launches reject or wait on active exclusive profile conflicts according to `profileLeasePolicy`; use `profileLeaseWaitTimeoutMs` to bound waits. Profile mutations reject `caller_supplied` profiles without `userDataDir` and `per_service` profiles with more than one `sharedServiceIds` entry. Session mutations infer `owner` from `agentName`, then `serviceName`, when `owner` is omitted; `profileId` must reference a persisted profile, and profile `sharedServiceIds` allow-lists are enforced.

Waiting lease requests stay queued until the selected profile becomes available or `profileLeaseWaitTimeoutMs` expires.

Service job and access-plan warning values are `missing_service_name`, `missing_agent_name`, and `missing_task_name`; `hasNamingWarning` is true when `namingWarnings` is non-empty. HTTP `/api/service/jobs`, HTTP `/api/service/jobs/<id>`, and MCP `agent-browser://jobs` job records follow `docs/dev/contracts/service-job-record.v1.schema.json`. CLI and HTTP `service_jobs` response envelopes follow `docs/dev/contracts/service-jobs-response.v1.schema.json`.

HTTP `/api/service/incidents`, HTTP `/api/service/incidents/<id>`, MCP `agent-browser://incidents`, and MCP `service_incidents` incident records follow `docs/dev/contracts/service-incident-record.v1.schema.json`. CLI, HTTP, and MCP `service_incidents` response envelopes follow `docs/dev/contracts/service-incidents-response.v1.schema.json`.

HTTP `/api/service/events`, MCP `agent-browser://events`, and service trace event records follow `docs/dev/contracts/service-event-record.v1.schema.json`. CLI and HTTP `service_events` response envelopes follow `docs/dev/contracts/service-events-response.v1.schema.json`.

HTTP and MCP profile, browser, session, tab, monitor, site policy, provider, and challenge records follow the matching `docs/dev/contracts/service-*-record.v1.schema.json` files. Profile records include derived `targetReadiness` rows for no-launch target-service readiness; Google targets without authenticated evidence report `needs_manual_seeding` and recommend detached `runtime login` before attachable automation. Once a managed profile lists the target in `authenticatedServiceIds`, readiness changes to `seeded_unknown_freshness` and access-plan no longer treats first-login seeding as a required manual action. Explicit `fresh`, `stale`, and `blocked_by_attached_devtools` readiness rows, plus rows with `lastVerifiedAt` or `freshnessExpiresAt`, are preserved through derived refreshes. Service status and compact collection response envelopes follow the matching status and collection response schemas under `docs/dev/contracts/`. `GET /api/service/monitors` returns retained monitor records for heartbeat and freshness probes; active monitors are checked by the daemon scheduler when due. Profile collection responses include `profileSources`, and profile allocation rows include the same `targetReadiness` rows. `GET /api/service/profiles/<id>/readiness` returns one profile's no-launch readiness rows without allocation details. `GET /api/service/profiles/lookup` applies the authoritative service profile selector for `serviceName` plus target, site, or login identity and returns the selected profile, selected profile source, selector reason, matched profile field, matched identity, readiness, and readiness summary. Profile, session, site-policy, monitor, and provider mutation responses follow the matching upsert and delete response schemas under `docs/dev/contracts/`. Job cancel, browser retry, and incident acknowledgement or resolution responses follow the matching operator remedy response schemas under `docs/dev/contracts/`. Service reconcile responses follow `docs/dev/contracts/service-reconcile-response.v1.schema.json`.

`service_trace` responses follow `docs/dev/contracts/service-trace-response.v1.schema.json`, with summary and activity records covered by `docs/dev/contracts/service-trace-summary-record.v1.schema.json` and `docs/dev/contracts/service-trace-activity-record.v1.schema.json`.

Incident activity responses follow `docs/dev/contracts/service-incident-activity-response.v1.schema.json`.

Run `pnpm test:service-shutdown-health-live` to validate that a polite browser shutdown failure leaves the persisted service browser record `degraded` after the owned Chrome process is force-killed.

Run `pnpm test:service-shutdown-faulted-live` to validate that a force-kill failure leaves the persisted service browser record `faulted` and escalates the incident as possible OS degradation.

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

Use `agent-browser service jobs --limit <n>` to inspect recent control-plane jobs directly without parsing the full service status payload. Use `agent-browser service jobs --id <job-id>` to inspect one retained job directly. Add `--state <state>`, `--action <action>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter jobs before the limit is applied. Valid states are `queued`, `waiting_profile_lease`, `running`, `succeeded`, `failed`, `cancelled`, and `timed_out`. `--since` accepts RFC 3339 timestamps. Jobs include advisory `namingWarnings` when a request is missing `serviceName`, `agentName`, or `taskName`. Current warning values are `missing_service_name`, `missing_agent_name`, and `missing_task_name`. `hasNamingWarning` is true when `namingWarnings` is non-empty. `pnpm test:service-health-live` includes the static API/MCP parity guard and HTTP job-naming check; run `pnpm test:service-api-mcp-parity` or `pnpm test:service-job-naming-live` directly when only one contract needs validation.

Use `agent-browser service incidents --limit <n>` to inspect grouped retained incidents directly without parsing the full service status payload. Add `--summary` to group the current filtered incident set by escalation, severity, and state with each group's recommended next action. Add `--remedies`, or use `agent-browser service remedies`, for the compact operator ladder that returns active `browser_degraded`, `monitor_attention`, and `os_degraded_possible` groups only. Use `agent-browser service remedies apply --escalation monitor_attention`, HTTP `POST /api/service/remedies/apply?escalation=monitor_attention`, MCP `service_remedies_apply`, or `applyServiceRemedies()` to acknowledge all active monitor-attention incidents and reset reviewed monitor failure counters through the service worker. Use `agent-browser service remedies apply --escalation os_degraded_possible` only after host inspection to batch the existing faulted-browser retry remedy for active OS-degraded-possible incidents. HTTP clients can request the same ladder with `summary=true&remedies=true`, and MCP clients can pass `summary: true` and `remediesOnly: true` to `service_incidents`. Run `pnpm test:service-incident-summary-http` and `pnpm test:service-incident-summary-mcp` to validate both summary paths and their shared filter matrix without launching Chrome. Use `agent-browser service incidents --id <incident-id>` to fetch one retained incident together with its expanded related events and jobs. Incident detail also includes acknowledgement and resolution metadata when present. Incidents include `severity`, `escalation`, `recommendedAction`, and monitor metadata when a failed service monitor created the incident, so CLI, HTTP, MCP, and dashboard clients do not infer operator priority differently. Failed service monitors use `monitor_attention`, expose `monitorId`, `monitorTarget`, and `monitorResult` on the incident, and summary groups include `monitorIds` plus `monitorResetCommands` for reviewed failure cleanup. Add `--state <state>`, `--severity <severity>`, `--escalation <escalation>`, `--handling-state <state>`, `--kind <kind>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter incidents before the limit is applied. Trace-context filters match related events and jobs. Valid incident states are `active`, `recovered`, and `service`. Valid severities are `info`, `warning`, `error`, and `critical`. Valid escalations are `none`, `browser_degraded`, `browser_recovery`, `job_attention`, `monitor_attention`, `service_triage`, and `os_degraded_possible`. Valid handling states are `unacknowledged`, `acknowledged`, and `resolved`. Valid kinds are `browser_health_changed`, `reconciliation_error`, `service_job_timeout`, and `service_job_cancelled`. `--since` accepts RFC 3339 timestamps and compares the incident `latestTimestamp`.

Use `agent-browser service events --limit <n>` to inspect recent service events directly without parsing the full service status payload. Launch, health, recovery, and profile lease wait events include `profileId`, `sessionId`, `serviceName`, `agentName`, and `taskName` when that context is known. Add `--kind <kind>`, `--browser-id <id>`, `--profile-id <id>`, `--session-id <id>`, `--service-name <name>`, `--agent-name <name>`, `--task-name <name>`, or `--since <timestamp>` to filter events before the limit is applied. Valid kinds are `reconciliation`, `browser_launch_recorded`, `browser_health_changed`, `browser_recovery_started`, `browser_recovery_override`, `tab_lifecycle_changed`, `profile_lease_wait_started`, `profile_lease_wait_ended`, `reconciliation_error`, `incident_acknowledged`, and `incident_resolved`. `profile_lease_wait_started` and `profile_lease_wait_ended` include `details.jobId`, `details.outcome`, `details.conflictSessionIds`, retry timing, and waited timing when known. `--since` accepts RFC 3339 timestamps.

When a session stream server is running, use `GET /api/browser/url`, `GET /api/browser/title`, `GET /api/browser/tabs?verbose=true`, `POST /api/browser/viewport`, `POST /api/browser/user-agent`, `POST /api/browser/media`, `POST /api/browser/timezone`, `POST /api/browser/locale`, `POST /api/browser/geolocation`, `POST /api/browser/permissions`, `POST /api/browser/cookies/get`, `POST /api/browser/cookies/set`, `POST /api/browser/cookies/clear`, `POST /api/browser/storage/get`, `POST /api/browser/storage/set`, `POST /api/browser/storage/clear`, `POST /api/browser/console`, `POST /api/browser/errors`, `POST /api/browser/set-content`, `POST /api/browser/headers`, `POST /api/browser/offline`, `POST /api/browser/dialog`, `POST /api/browser/clipboard`, `POST /api/browser/upload`, `POST /api/browser/download`, `POST /api/browser/wait-for-download`, `POST /api/browser/pdf`, `POST /api/browser/response-body`, `POST /api/browser/har/start`, `POST /api/browser/har/stop`, `POST /api/browser/route`, `POST /api/browser/unroute`, `POST /api/browser/requests`, `POST /api/browser/request-detail`, `POST /api/browser/navigate`, `POST /api/browser/back`, `POST /api/browser/forward`, `POST /api/browser/reload`, `POST /api/browser/new-tab`, `POST /api/browser/switch-tab`, `POST /api/browser/close-tab`, `POST /api/browser/snapshot`, `POST /api/browser/screenshot`, `POST /api/browser/click`, `POST /api/browser/fill`, `POST /api/browser/wait`, `POST /api/browser/type`, `POST /api/browser/press`, `POST /api/browser/hover`, `POST /api/browser/select`, `POST /api/browser/get-text`, `POST /api/browser/get-value`, `POST /api/browser/is-visible`, `POST /api/browser/get-attribute`, `POST /api/browser/get-html`, `POST /api/browser/get-styles`, `POST /api/browser/count`, `POST /api/browser/get-box`, `POST /api/browser/is-enabled`, `POST /api/browser/is-checked`, `POST /api/browser/check`, `POST /api/browser/uncheck`, `POST /api/browser/scroll`, `POST /api/browser/scroll-into-view`, `POST /api/browser/focus`, and `POST /api/browser/clear` for named browser-control wrappers over the same daemon command queue as `/api/command` and MCP tools. POST bodies accept the same command fields as the underlying action, including `serviceName`, `agentName`, `taskName`, and `jobTimeoutMs` for traceable software clients. Use `GET /api/service/status`, `GET /api/service/contracts`, `GET /api/service/access-plan`, `GET /api/service/profiles`, `GET /api/service/profiles/lookup`, `GET /api/service/profiles/<profile-id>/allocation`, `POST /api/service/profiles/<id>`, `DELETE /api/service/profiles/<id>`, `GET /api/service/sessions`, `POST /api/service/sessions/<id>`, `DELETE /api/service/sessions/<id>`, `GET /api/service/browsers`, `POST /api/service/browsers/<browser-id>/retry`, `GET /api/service/tabs`, `GET /api/service/site-policies`, `POST /api/service/site-policies/<id>`, `DELETE /api/service/site-policies/<id>`, `GET /api/service/providers`, `POST /api/service/providers/<id>`, `DELETE /api/service/providers/<id>`, `GET /api/service/challenges`, `GET /api/service/trace?limit=<n>&browser-id=<id>&profile-id=<id>&session-id=<id>&service-name=<name>&agent-name=<name>&task-name=<name>&since=<timestamp>`, `GET /api/service/jobs?limit=<n>&state=<state>&action=<action>&service-name=<name>&task-name=<name>&since=<timestamp>`, `GET /api/service/jobs/<job-id>`, `POST /api/service/jobs/<job-id>/cancel`, `GET /api/service/incidents?summary=true&limit=<n>&severity=<severity>&escalation=<escalation>&handling-state=<state>&kind=<kind>&browser-id=<id>&service-name=<name>&task-name=<name>&since=<timestamp>`, `GET /api/service/incidents?summary=true&remedies=true`, `GET /api/service/incidents/<incident-id>`, `GET /api/service/incidents/<incident-id>/activity`, `POST /api/service/incidents/<incident-id>/acknowledge?by=<actor>&note=<text>`, `POST /api/service/incidents/<incident-id>/resolve?by=<actor>&note=<text>`, `GET /api/service/events?limit=<n>&kind=<kind>&browser-id=<id>&service-name=<name>&task-name=<name>&since=<timestamp>`, or `POST /api/service/reconcile` on the stream port for a programmatic service surface that does not require shelling out. Profile, session, site policy, and provider mutation paths persist service config records through the service worker queue, use the path ID as authoritative, reject a request body whose nested `id` conflicts with the path, and enforce the profile/session ownership policy. The collection endpoints return compact arrays matching MCP resources, each with a `count` field. Site-policy collections also include `sitePolicySources` so operators and clients can see whether each effective policy came from config, persisted state, or built-in defaults.

Use HTTP `GET /api/service/monitors` on the stream port to inspect retained monitor records. Add `state=active|paused|faulted`, `failed=true`, or `summary=true` to filter and summarize monitor health without scanning events. Active monitors are checked by the daemon scheduler when due.

Use HTTP `POST /api/service/monitors/<id>`, HTTP `DELETE /api/service/monitors/<id>`, MCP `service_monitor_upsert`, MCP `service_monitor_delete`, `upsertServiceMonitor()`, or `deleteServiceMonitor()` to persist monitor definitions for the scheduler. Use HTTP `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, `runDueServiceMonitors()`, or `agent-browser service monitors run-due` to run due active monitors immediately. Use HTTP `POST /api/service/monitors/<id>/pause`, HTTP `POST /api/service/monitors/<id>/resume`, MCP `service_monitor_pause`, MCP `service_monitor_resume`, `pauseServiceMonitor()`, `resumeServiceMonitor()`, or the matching CLI commands to quiet or restore a monitor without clearing health history. Use HTTP `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, `triageServiceMonitor()`, or `agent-browser service monitors triage <id>` to acknowledge the related monitor incident and reset reviewed failures together. Use HTTP `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, `resetServiceMonitorFailures()`, or `agent-browser service monitors reset <id>` after triage to clear only the failure counter while retaining last failure evidence. Use HTTP `POST /api/service/remedies/apply?escalation=monitor_attention`, MCP `service_remedies_apply`, `applyServiceRemedies()`, or `agent-browser service remedies apply --escalation monitor_attention` to apply all active monitor remedies in one serialized operation. Use `escalation=os_degraded_possible` only after host inspection to batch the existing faulted-browser retry remedy for active OS-degraded-possible incidents.

Use `POST /api/service/request` on the stream port when a software client wants to submit one explicit request object with caller context, site or login hints, target-service hints, `action`, `params`, and `jobTimeoutMs`. The route queues the requested browser action through the same service-owned control path as MCP `service_request`.

For software integrations, use one intent object and let agent-browser handle profile selection and queueing:

```json
{"serviceName":"JournalDownloader","agentName":"article-probe-agent","taskName":"probeACSwebsite","siteId":"acs","action":"navigate","params":{"url":"https://example.com","waitUntil":"load"},"jobTimeoutMs":30000}
```

For MCP integrations, send the same fields as the `arguments` for `service_request`:

```json
{"name":"service_request","arguments":{"serviceName":"JournalDownloader","agentName":"article-probe-agent","taskName":"probeACSwebsite","siteId":"acs","action":"navigate","params":{"url":"https://example.com","waitUntil":"load"},"jobTimeoutMs":30000}}
```

Service browser-health reconciliation runs in the daemon background every 60000 ms by default. Set `service.reconcileIntervalMs` in config, pass `--service-reconcile-interval <ms>`, or set `AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS` to change the interval. Use `0` to disable the background loop.

Due active service monitors are enqueued through the same service worker every 60000 ms by default. Set `service.monitorIntervalMs`, pass `--service-monitor-interval <ms>`, or set `AGENT_BROWSER_SERVICE_MONITOR_INTERVAL_MS` to change the scheduler interval. Use `0` to disable monitor scheduling. Use `agent-browser service monitors run-due`, HTTP `POST /api/service/monitors/run-due`, MCP `service_monitors_run_due`, or `runDueServiceMonitors()` to check due active monitors immediately. Use `agent-browser service monitors pause <id>` and `agent-browser service monitors resume <id>`, HTTP `POST /api/service/monitors/<id>/pause` and `POST /api/service/monitors/<id>/resume` routes, MCP `service_monitor_pause` and `service_monitor_resume`, or `pauseServiceMonitor()` and `resumeServiceMonitor()` to quiet or restore noisy monitors without clearing retained health history. Use `agent-browser service monitors triage <id>`, HTTP `POST /api/service/monitors/<id>/triage`, MCP `service_monitor_triage`, or `triageServiceMonitor()` to acknowledge the related monitor incident and clear reviewed failures in one queued operation. Use `agent-browser service monitors reset <id>`, HTTP `POST /api/service/monitors/<id>/reset-failures`, MCP `service_monitor_reset_failures`, or `resetServiceMonitorFailures()` to clear only a reviewed failure count while retaining the last failure evidence. The runner updates `lastCheckedAt`, `lastSucceededAt`, `lastFailedAt`, `lastResult`, and `consecutiveFailures`; failed probes set the monitor `state` to `faulted` and append a service incident event.

Service control jobs do not time out at the worker boundary by default. Set `service.jobTimeoutMs`, pass `--service-job-timeout <ms>`, or set `AGENT_BROWSER_SERVICE_JOB_TIMEOUT_MS` to mark long-running dispatched jobs as `timed_out`. Use `0` to disable it.

Browser recovery defaults to 3 relaunch attempts, 1000 ms base backoff, and 30000 ms max backoff before marking a browser `faulted`. Set `service.recoveryRetryBudget`, `service.recoveryBaseBackoffMs`, and `service.recoveryMaxBackoffMs`, pass `--service-recovery-retry-budget <n>`, `--service-recovery-base-backoff <ms>`, or `--service-recovery-max-backoff <ms>`, or set the matching `AGENT_BROWSER_SERVICE_RECOVERY_*` environment variables to tune this for a service host. Recovery-started trace events include `details.policySource.retryBudget`, `details.policySource.baseBackoffMs`, and `details.policySource.maxBackoffMs` so agents can see whether each active value came from defaults, config, environment, or CLI flags.

## Batch Execution

Use `batch` when running 2 or more commands in sequence. Batch executes commands in order, so dependent commands like navigate then screenshot work correctly. Each quoted argument is a separate command.

```bash
# Navigate and take a snapshot
agent-browser batch "open https://example.com" "snapshot -i"

# Navigate, snapshot, and screenshot in one call
agent-browser batch "open https://example.com" "snapshot -i" "screenshot"

# Click, wait, then screenshot
agent-browser batch "click @e1" "wait 1000" "screenshot"

# With --bail to stop on first error
agent-browser batch --bail "open https://example.com" "click @e1" "screenshot"
```

Only use a single command (not batch) when you need to read the output before deciding the next command. For example, you must run `snapshot -i` as a single command when you need to read the refs to decide what to click. After reading the snapshot, batch the remaining steps.

Stdin mode is also supported for programmatic use:

```bash
echo '[["open","https://example.com"],["screenshot"]]' | agent-browser batch --json
agent-browser batch --bail < commands.json
```

## Efficiency Strategies

These patterns minimize tool calls and token usage.

**Use `--urls` to avoid re-navigation.** When you need to visit links from a page, use `snapshot -i --urls` to get all href URLs upfront. Then `open` each URL directly instead of clicking refs and navigating back.

**Snapshot once, act many times.** Never re-snapshot the same page. Extract all needed info (refs, URLs, text) from a single snapshot, then batch the remaining actions.

**Multi-page workflow (e.g. "visit N sites and screenshot each"):**

```bash
# 1. Get all URLs in one call
agent-browser batch "open https://news.ycombinator.com" "snapshot -i --urls"
# Read output to extract URLs, then visit each directly:
# 2. One batch per target site
agent-browser batch "open https://github.com/example/repo" "screenshot"
agent-browser batch "open https://example.com/article" "screenshot"
agent-browser batch "open https://other.com/page" "screenshot"
```

This approach uses 4 tool calls instead of 14+. Never go back to the listing page between visits.

## Common Patterns

### Form Submission

```bash
# Navigate and get the form structure
agent-browser batch "open https://example.com/signup" "snapshot -i"
# Read the snapshot output to identify form refs, then fill and submit
agent-browser batch "fill @e1 \"Jane Doe\"" "fill @e2 \"jane@example.com\"" "select @e3 \"California\"" "check @e4" "click @e5" "wait 2000"
```

### Authentication with Auth Vault (Recommended)

```bash
# Save credentials once (encrypted with AGENT_BROWSER_ENCRYPTION_KEY)
# Recommended: pipe password via stdin to avoid shell history exposure
echo "pass" | agent-browser auth save github --url https://github.com/login --username user --password-stdin

# Login using saved profile (LLM never sees password)
agent-browser auth login github

# List/show/delete profiles
agent-browser auth list
agent-browser auth show github
agent-browser auth delete github
```

`auth login` waits for username/password/submit selectors before interacting, with a timeout tied to the default action timeout.

### Authentication with State Persistence

Use this when you need a portable auth snapshot. For recurring work on the same
machine, prefer a managed runtime profile instead.

```bash
# Login once and save state
agent-browser batch "open https://app.example.com/login" "snapshot -i"
# Read snapshot to find form refs, then fill and submit
agent-browser batch "fill @e1 \"$USERNAME\"" "fill @e2 \"$PASSWORD\"" "click @e3" "wait --url **/dashboard" "state save auth.json"

# Reuse in future sessions
agent-browser batch "state load auth.json" "open https://app.example.com/dashboard"
```

### Session Persistence

Use `--session-name` for lightweight cookie and storage reuse. For browser-level
identity and long-lived authenticated automation, prefer `--runtime-profile`.

```bash
# Auto-save/restore cookies and localStorage across browser restarts
agent-browser --session-name myapp open https://app.example.com/login
# ... login flow ...
agent-browser close  # State auto-saved to ~/.agent-browser/sessions/

# Next time, state is auto-loaded
agent-browser --session-name myapp open https://app.example.com/dashboard

# Encrypt state at rest
export AGENT_BROWSER_ENCRYPTION_KEY=$(openssl rand -hex 32)
agent-browser --session-name secure open https://app.example.com

# Manage saved states
agent-browser state list
agent-browser state show myapp-default.json
agent-browser state clear myapp
agent-browser state clean --older-than 7
```

### Working with Iframes

Iframe content is automatically inlined in snapshots. Refs inside iframes carry frame context, so you can interact with them directly.

```bash
agent-browser batch "open https://example.com/checkout" "snapshot -i"
# @e1 [heading] "Checkout"
# @e2 [Iframe] "payment-frame"
#   @e3 [input] "Card number"
#   @e4 [input] "Expiry"
#   @e5 [button] "Pay"

# Interact directly — no frame switch needed
agent-browser batch "fill @e3 \"4111111111111111\"" "fill @e4 \"12/28\"" "click @e5"

# To scope a snapshot to one iframe:
agent-browser batch "frame @e2" "snapshot -i"
agent-browser frame main          # Return to main frame
```

### Data Extraction

```bash
agent-browser batch "open https://example.com/products" "snapshot -i"
# Read snapshot to find element refs, then extract
agent-browser get text @e5           # Get specific element text

# JSON output for parsing
agent-browser snapshot -i --json
agent-browser get text @e1 --json
```

### Parallel Sessions

Use `--session` for lightweight command namespace isolation. For authenticated
service work, prefer service requests with caller labels and login or target
hints so agent-browser can share or queue the right managed profile. Add
`--runtime-profile` only when the operator intentionally chooses a separate
browser identity or when service profile readiness shows that a new managed
identity lane is needed.

```bash
agent-browser --session site1 open https://site-a.com
agent-browser --session site2 open https://site-b.com

agent-browser --session site1 snapshot -i
agent-browser --session site2 snapshot -i

# Or explicitly choose a separate managed identity lane
agent-browser --session reviewer-a --runtime-profile work open https://app.example.com

agent-browser session list
```

### Connect to Existing Chrome

```bash
# Auto-discover running Chrome with remote debugging enabled
agent-browser --auto-connect open https://example.com
agent-browser --auto-connect snapshot

# Or with explicit CDP port
agent-browser --cdp 9222 snapshot
```

Auto-connect discovers Chrome via `DevToolsActivePort`, common debugging ports (9222, 9229), and falls back to a direct WebSocket connection if HTTP-based CDP discovery fails.

### Color Scheme (Dark Mode)

```bash
# Persistent dark mode via flag (applies to all pages and new tabs)
agent-browser --color-scheme dark open https://example.com

# Or via environment variable
AGENT_BROWSER_COLOR_SCHEME=dark agent-browser open https://example.com

# Or set during session (persists for subsequent commands)
agent-browser set media dark
```

### Viewport & Responsive Testing

```bash
# Set a custom viewport size (default is 1280x720)
agent-browser set viewport 1920 1080
agent-browser screenshot desktop.png

# Test mobile-width layout
agent-browser set viewport 375 812
agent-browser screenshot mobile.png

# Retina/HiDPI: same CSS layout at 2x pixel density
# Screenshots stay at logical viewport size, but content renders at higher DPI
agent-browser set viewport 1920 1080 2
agent-browser screenshot retina.png

# Device emulation (sets viewport + user agent in one step)
agent-browser set device "iPhone 14"
agent-browser screenshot device.png
```

The `scale` parameter (3rd argument) sets `window.devicePixelRatio` without changing CSS layout. Use it when testing retina rendering or capturing higher-resolution screenshots.

### Visual Browser (Debugging)

```bash
agent-browser --headed open https://example.com
agent-browser highlight @e1          # Highlight element
agent-browser inspect                # Open Chrome DevTools for the active page
agent-browser record start demo.webm # Record session
agent-browser profiler start         # Start Chrome DevTools profiling
agent-browser profiler stop trace.json # Stop and save profile (path optional)
```

Use `AGENT_BROWSER_HEADED=1` to enable headed mode via environment variable. Browser extensions work in both headed and headless mode.

On Unix, if `DISPLAY` is unset, agent-browser defaults headed Chrome launches to `DISPLAY=:0.0`. Assume that fallback unless the user explicitly wants a different display.

For runtime debugging, use:

```bash
agent-browser get browser-pid
agent-browser tab list
agent-browser tab list --verbose
```

`tab list --verbose` exposes CDP `targetId` and `sessionId` for each tab.

### Local Files (PDFs, HTML)

```bash
# Open local files with file:// URLs
agent-browser --allow-file-access open file:///path/to/document.pdf
agent-browser --allow-file-access open file:///path/to/page.html
agent-browser screenshot output.png
```

### iOS Simulator (Mobile Safari)

```bash
# List available iOS simulators
agent-browser device list

# Launch Safari on a specific device
agent-browser -p ios --device "iPhone 16 Pro" open https://example.com

# Same workflow as desktop - snapshot, interact, re-snapshot
agent-browser -p ios snapshot -i
agent-browser -p ios tap @e1          # Tap (alias for click)
agent-browser -p ios fill @e2 "text"
agent-browser -p ios swipe up         # Mobile-specific gesture

# Take screenshot
agent-browser -p ios screenshot mobile.png

# Close session (shuts down simulator)
agent-browser -p ios close
```

**Requirements:** macOS with Xcode, Appium (`npm install -g appium && appium driver install xcuitest`)

**Real devices:** Works with physical iOS devices if pre-configured. Use `--device "<UDID>"` where UDID is from `xcrun xctrace list devices`.

## Security

All security features are opt-in. By default, agent-browser imposes no restrictions on navigation, actions, or output.

### Content Boundaries (Recommended for AI Agents)

Enable `--content-boundaries` to wrap page-sourced output in markers that help LLMs distinguish tool output from untrusted page content:

```bash
export AGENT_BROWSER_CONTENT_BOUNDARIES=1
agent-browser snapshot
# Output:
# --- AGENT_BROWSER_PAGE_CONTENT nonce=<hex> origin=https://example.com ---
# [accessibility tree]
# --- END_AGENT_BROWSER_PAGE_CONTENT nonce=<hex> ---
```

### Domain Allowlist

Restrict navigation to trusted domains. Wildcards like `*.example.com` also match the bare domain `example.com`. Sub-resource requests, WebSocket, and EventSource connections to non-allowed domains are also blocked. Include CDN domains your target pages depend on:

```bash
export AGENT_BROWSER_ALLOWED_DOMAINS="example.com,*.example.com"
agent-browser open https://example.com        # OK
agent-browser open https://malicious.com       # Blocked
```

### Action Policy

Use a policy file to gate destructive actions:

```bash
export AGENT_BROWSER_ACTION_POLICY=./policy.json
```

Example `policy.json`:

```json
{ "default": "deny", "allow": ["navigate", "snapshot", "click", "scroll", "wait", "get"] }
```

Auth vault operations (`auth login`, etc.) bypass action policy but domain allowlist still applies.

### Output Limits

Prevent context flooding from large pages:

```bash
export AGENT_BROWSER_MAX_OUTPUT=50000
```

## Diffing (Verifying Changes)

Use `diff snapshot` after performing an action to verify it had the intended effect. This compares the current accessibility tree against the last snapshot taken in the session.

```bash
# Typical workflow: snapshot -> action -> diff
agent-browser snapshot -i          # Take baseline snapshot
agent-browser click @e2            # Perform action
agent-browser diff snapshot        # See what changed (auto-compares to last snapshot)
```

For visual regression testing or monitoring:

```bash
# Save a baseline screenshot, then compare later
agent-browser screenshot baseline.png
# ... time passes or changes are made ...
agent-browser diff screenshot --baseline baseline.png

# Compare staging vs production
agent-browser diff url https://staging.example.com https://prod.example.com --screenshot
```

`diff snapshot` output uses `+` for additions and `-` for removals, similar to git diff. `diff screenshot` produces a diff image with changed pixels highlighted in red, plus a mismatch percentage.

## Timeouts and Slow Pages

The default timeout is 25 seconds. This can be overridden with the `AGENT_BROWSER_DEFAULT_TIMEOUT` environment variable (value in milliseconds).

**Important:** `open` already waits for the page `load` event before returning. In most cases, no additional wait is needed before taking a snapshot or screenshot. Only add an explicit wait when content loads asynchronously after the initial page load.

```bash
# Wait for a specific element to appear (preferred for dynamic content)
agent-browser wait "#content"
agent-browser wait @e1

# Wait a fixed duration (good default for slow SPAs)
agent-browser wait 2000

# Wait for a specific URL pattern (useful after redirects)
agent-browser wait --url "**/dashboard"

# Wait for text to appear on the page
agent-browser wait --text "Results loaded"

# Wait for a JavaScript condition
agent-browser wait --fn "document.querySelectorAll('.item').length > 0"
```

**Avoid `wait --load networkidle`** unless you are certain the site has no persistent network activity. Ad-heavy sites, sites with analytics/tracking, and sites with websockets will cause `networkidle` to hang indefinitely. Prefer `wait 2000` or `wait <selector>` instead.

## JavaScript Dialogs (alert / confirm / prompt)

When a page opens a JavaScript dialog (`alert()`, `confirm()`, or `prompt()`), it blocks all other browser commands (snapshot, screenshot, click, etc.) until the dialog is dismissed. If commands start timing out unexpectedly, check for a pending dialog:

```bash
# Check if a dialog is blocking
agent-browser dialog status

# Accept the dialog (dismiss the alert / click OK)
agent-browser dialog accept

# Accept a prompt dialog with input text
agent-browser dialog accept "my input"

# Dismiss the dialog (click Cancel)
agent-browser dialog dismiss
```

When a dialog is pending, all command responses include a `warning` field indicating the dialog type and message. In `--json` mode this appears as a `"warning"` key in the response object.

## Session Management and Cleanup

When running multiple agents or automations concurrently, always use named sessions to avoid command namespace conflicts. Do not add a new runtime profile merely to avoid another active job. For service-mode work, include `serviceName`, `agentName`, `taskName`, and a target identity so agent-browser can queue work against the right managed browser:

```bash
# Each agent gets its own isolated session
agent-browser --session agent1 open site-a.com
agent-browser --session agent2 open site-b.com

# A worker targets its own runtime profile only for a deliberate identity lane
agent-browser --session agent1 --runtime-profile billing open https://app.example.com
agent-browser --session agent2 --runtime-profile support open https://app.example.com

# Check active sessions
agent-browser session list
```

Always close your browser session when done to avoid leaked processes:

```bash
agent-browser close                    # Close default session
agent-browser --session agent1 close   # Close specific session
agent-browser close --all              # Close all active sessions
```

If a previous session was not closed properly, the daemon may still be running. Use `agent-browser close` to clean it up, or `agent-browser close --all` to shut down every session at once.

To auto-shutdown the daemon after a period of inactivity (useful for ephemeral/CI environments):

```bash
AGENT_BROWSER_IDLE_TIMEOUT_MS=60000 agent-browser open example.com
```

## Ref Lifecycle (Important)

Refs (`@e1`, `@e2`, etc.) are invalidated when the page changes. Always re-snapshot after:

- Clicking links or buttons that navigate
- Form submissions
- Dynamic content loading (dropdowns, modals)

```bash
agent-browser click @e5              # Navigates to new page
agent-browser snapshot -i            # MUST re-snapshot
agent-browser click @e1              # Use new refs
```

## Annotated Screenshots (Vision Mode)

Use `--annotate` to take a screenshot with numbered labels overlaid on interactive elements. Each label `[N]` maps to ref `@eN`. This also caches refs, so you can interact with elements immediately without a separate snapshot.

```bash
agent-browser screenshot --annotate
# Output includes the image path and a legend:
#   [1] @e1 button "Submit"
#   [2] @e2 link "Home"
#   [3] @e3 textbox "Email"
agent-browser click @e2              # Click using ref from annotated screenshot
```

Use annotated screenshots when:

- The page has unlabeled icon buttons or visual-only elements
- You need to verify visual layout or styling
- Canvas or chart elements are present (invisible to text snapshots)
- You need spatial reasoning about element positions

## Semantic Locators (Alternative to Refs)

When refs are unavailable or unreliable, use semantic locators:

```bash
agent-browser find text "Sign In" click
agent-browser find label "Email" fill "user@test.com"
agent-browser find role button click --name "Submit"
agent-browser find placeholder "Search" type "query"
agent-browser find testid "submit-btn" click
```

## JavaScript Evaluation (eval)

Use `eval` to run JavaScript in the browser context. **Shell quoting can corrupt complex expressions**. Use `--stdin` or `-b` to avoid issues.

```bash
# Simple expressions work with regular quoting
agent-browser eval 'document.title'
agent-browser eval 'document.querySelectorAll("img").length'

# Complex JS: use --stdin with heredoc (RECOMMENDED)
agent-browser eval --stdin <<'EVALEOF'
JSON.stringify(
  Array.from(document.querySelectorAll("img"))
    .filter(i => !i.alt)
    .map(i => ({ src: i.src.split("/").pop(), width: i.width }))
)
EVALEOF

# Alternative: base64 encoding (avoids all shell escaping issues)
agent-browser eval -b "$(echo -n 'Array.from(document.querySelectorAll("a")).map(a => a.href)' | base64)"
```

**Why this matters:** When the shell processes your command, inner double quotes, `!` characters (history expansion), backticks, and `$()` can all corrupt the JavaScript before it reaches agent-browser. The `--stdin` and `-b` flags bypass shell interpretation entirely.

**Rules of thumb:**

- Single-line, no nested quotes -> regular `eval 'expression'` with single quotes is fine
- Nested quotes, arrow functions, template literals, or multiline -> use `eval --stdin <<'EVALEOF'`
- Programmatic/generated scripts -> use `eval -b` with base64

## Configuration File

Create `agent-browser.json` in the project root for persistent settings:

```json
{
  "headed": true,
  "proxy": "http://localhost:8080",
  "profile": "./browser-data"
}
```

Priority (lowest to highest): `~/.agent-browser/config.json` < `./agent-browser.json` < env vars < CLI flags. Use `--config <path>` or `AGENT_BROWSER_CONFIG` env var for a custom config file (exits with error if missing/invalid). All CLI options map to camelCase keys (e.g., `--executable-path` -> `"executablePath"`). Boolean flags accept `true`/`false` values (e.g., `--headed false` overrides config). Extensions from user and project configs are merged, not replaced.

## Deep-Dive Documentation

| Reference                                                            | When to Use                                               |
| -------------------------------------------------------------------- | --------------------------------------------------------- |
| [references/commands.md](references/commands.md)                     | Full command reference with all options                   |
| [references/snapshot-refs.md](references/snapshot-refs.md)           | Ref lifecycle, invalidation rules, troubleshooting        |
| [references/session-management.md](references/session-management.md) | Parallel sessions, state persistence, concurrent scraping |
| [references/authentication.md](references/authentication.md)         | Login flows, OAuth, 2FA handling, state reuse             |
| [references/video-recording.md](references/video-recording.md)       | Recording workflows for debugging and documentation       |
| [references/profiling.md](references/profiling.md)                   | Chrome DevTools profiling for performance analysis        |
| [references/proxy-support.md](references/proxy-support.md)           | Proxy configuration, geo-testing, rotating proxies        |

## Cloud Providers

Use `-p <provider>` (or `AGENT_BROWSER_PROVIDER`) to run against a cloud browser instead of launching a local Chrome instance. Supported providers: `agentcore`, `browserbase`, `browserless`, `browseruse`, `kernel`.

### AgentCore (AWS Bedrock)

```bash
# Credentials auto-resolved from env vars or AWS CLI (SSO, IAM roles, etc.)
agent-browser -p agentcore open https://example.com

# With persistent browser profile
AGENTCORE_PROFILE_ID=my-profile agent-browser -p agentcore open https://example.com

# With explicit region
AGENTCORE_REGION=eu-west-1 agent-browser -p agentcore open https://example.com
```

Set `AWS_PROFILE` to select a named AWS profile.

## Browser Engine Selection

Use `--engine` to choose a local browser engine. The default is `chrome`.

```bash
# Use Lightpanda (fast headless browser, requires separate install)
agent-browser --engine lightpanda open example.com

# Via environment variable
export AGENT_BROWSER_ENGINE=lightpanda
agent-browser open example.com

# With custom binary path
agent-browser --engine lightpanda --executable-path /path/to/lightpanda open example.com
```

Supported engines:
- `chrome` (default): Chrome/Chromium via CDP
- `lightpanda`: Lightpanda headless browser via CDP (10x faster, 10x less memory than Chrome)

Lightpanda does not support `--extension`, `--profile`, `--state`, or `--allow-file-access`. Install Lightpanda from https://lightpanda.io/docs/open-source/installation.

## Observability Dashboard

The dashboard is a standalone background server that shows live browser viewports, command activity, and console output for all sessions.

```bash
# Start the dashboard server (background, port 4848)
agent-browser dashboard start

# All sessions are automatically visible in the dashboard
agent-browser open example.com

# Stop the dashboard
agent-browser dashboard stop
```

The dashboard runs independently of browser sessions on port 4848 (configurable with `--port`). All sessions automatically stream to the dashboard. Sessions can also be created from the dashboard UI with local engines or cloud providers. Use the Service view to inspect worker/browser health, set a remembered operator identity for incident audit metadata, add optional operator notes when acknowledging or resolving incidents, inspect prominent service-sourced incident severity, escalation, recommended action displays, and remedy summary groups, inspect the service-owned incident history timeline with local fallback, load combined service, agent, task, browser, profile, session, and time-window traces from `/api/service/trace`, including ownership summary cards, naming warnings, and profile lease wait cards from the shared trace payload, view a browser-health transition timeline for crash/recovery visibility, inspect the backend-owned `profileAllocations` view and detail dialog for holder sessions, waiting jobs, conflicts, lease state, and recommended actions refreshed from `GET /api/service/profiles/<id>/allocation`; run `pnpm test:dashboard-profile-allocation` when changing that detail lookup. Use a grouped incident browser panel with handling-state filters plus acknowledge and resolve actions, inspect incident filtering for crash/disconnect/recovery and timed-out or cancelled jobs, check reconciliation status, review managed entity counts, inspect recent service jobs with naming warnings, inspect browser records and session/tab relationships, inspect event details, and run reconciliation from the UI.

### Dashboard AI Chat

The dashboard has an optional AI chat tab powered by the Vercel AI Gateway. Enable it by setting:

```bash
export AI_GATEWAY_API_KEY=gw_your_key_here
export AI_GATEWAY_MODEL=anthropic/claude-sonnet-4.6           # optional default
export AI_GATEWAY_URL=https://ai-gateway.vercel.sh           # optional default
```

The Chat tab is always visible in the dashboard. Set `AI_GATEWAY_API_KEY` to enable AI responses.

## Ready-to-Use Templates

| Template                                                                 | Description                         |
| ------------------------------------------------------------------------ | ----------------------------------- |
| [templates/form-automation.sh](templates/form-automation.sh)             | Form filling with validation        |
| [templates/authenticated-session.sh](templates/authenticated-session.sh) | Login once, reuse state             |
| [templates/capture-workflow.sh](templates/capture-workflow.sh)           | Content extraction with screenshots |

```bash
./templates/form-automation.sh https://example.com/form
./templates/authenticated-session.sh https://app.example.com/login
./templates/capture-workflow.sh https://example.com ./output
```
