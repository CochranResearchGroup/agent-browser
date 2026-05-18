# UPS Headless StealthCDP Smoke Handoff

Date: 2026-05-17

## Context

Odollo is using `agent-browser` for SoyLei Amazon APEX-1132 carrier status
polling. The operator asked for a live smoke of one shipped but undelivered UPS
package using the newly available `chromium-stealthcdp` build in headless mode.

The expectation was that headless `chromium-stealthcdp` should behave much more
like a fully headed, human-operated Chromium instance. The operator also noted
that the `agent-browser` skill should have steered the caller toward letting
agent-browser choose the profile and browser routing instead of manually
supplying a profile path.

## Smoke Target

- Source repo where smoke was run: `/home/ecochran76/workspace.local/odollo`
- Tenant flow: `soylei-prod / amazon-apex-1132`
- Amazon order: `111-1756199-8122618`
- UPS tracking number: `1Z035CX1YW53854301`
- Work-packet status before the smoke: not delivered, sheet shipping status
  `Unknown`, workflow state `amazon_confirmation_pending`

## Preflight Evidence

`agent-browser install doctor` passed and resolved the current launch
configuration to the Windows `chromium-stealthcdp` install:

```text
version: 0.26.1
path command: /home/ecochran76/.local/bin/agent-browser
launch config source: manifest
launch config ready: true
launch executable: /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
No install drift detected.
```

The browser capability preflight command was also run:

```bash
agent-browser service browser-capability preflight \
  --browser-build stealthcdp_chromium \
  --service-name Odollo \
  --agent-name codex \
  --task-name upsCarrierSmoke \
  --target-service-id ups \
  --headless
```

It returned:

```text
Browser capability preflight: apply=no reason=explicit_executable_path build=stealthcdp_chromium profile=none headless=yes cdp_free=no
```

That output is important: this smoke still forced an explicit executable path
later, so the more complete service-owned access-plan/profile routing path was
not actually exercised.

## Commands Tried

First Odollo carrier smoke, forcing headless patched Chromium and a throwaway
profile path:

```bash
scripts/odollo-with-profile.sh soylei-prod \
  sync fulfillment preview-carrier-tracking-browser-evidence \
  --flow amazon-apex-1132 \
  --carrier UPS \
  --tracking-code 1Z035CX1YW53854301 \
  --amazon-order-number 111-1756199-8122618 \
  --headless \
  --browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --wait-ms 12000 \
  --timeout-seconds 90 \
  --screenshot-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.png" \
  --json-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.json"
```

Result:

```json
{
  "status": "lookup_failed",
  "delivered": false,
  "errors": ["Navigation failed: net::ERR_HTTP2_PROTOCOL_ERROR"]
}
```

Second Odollo carrier smoke, attempting an HTTP/2 workaround:

```bash
AGENT_BROWSER_ARGS='--disable-http2' scripts/odollo-with-profile.sh soylei-prod \
  sync fulfillment preview-carrier-tracking-browser-evidence \
  --flow amazon-apex-1132 \
  --carrier UPS \
  --tracking-code 1Z035CX1YW53854301 \
  --amazon-order-number 111-1756199-8122618 \
  --headless \
  --browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile-http1 \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --wait-ms 12000 \
  --timeout-seconds 90 \
  --screenshot-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.png" \
  --json-output "$HOME/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.json"
```

Result:

```json
{
  "status": "lookup_failed",
  "delivered": false,
  "errors": ["CDP command timed out: Page.navigate"]
}
```

Control smoke proving the same patched Chromium install could launch headless:

```bash
agent-browser \
  --session stealth-smoke-direct \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --profile /tmp/odollo-stealthcdp-direct-smoke \
  batch --bail "open https://example.com" "get title" "close"
```

Result:

```text
Example Domain
https://example.com/
Example Domain
Browser closed
```

## Outcome

- `chromium-stealthcdp` launched successfully in headless mode for a simple
  page.
- UPS did not load successfully in headless mode for this tracking URL.
- No screenshot was captured in either UPS attempt because navigation failed
  before page content was available.
- The Odollo carrier artifact shape handled the failure cleanly by writing a
  read-only `odollo.fulfillment.carrier_tracking_browser_evidence.v1` artifact.

Artifact paths:

```text
~/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-smoke-111-1756199-ups.json
~/.odollo/tenants/soylei-prod/artifacts/fulfillment-carrier-status/stealthcdp-headless-http1-smoke-111-1756199-ups.json
```

## Interpretation

This is not evidence that `chromium-stealthcdp` is generally broken. It is
evidence that the current headless UPS path is still not equivalent to a
fully-headed human-operated browser for this site and this environment.

The failed first run suggests a network/protocol or site compatibility problem
at navigation time. The `--disable-http2` retry changed the symptom from an
HTTP/2 protocol error to a `Page.navigate` timeout, but did not produce usable
tracking evidence.

## Follow-up: WSL-native headed versus headless comparison

Later on 2026-05-17, the same UPS tracking URL was retested from the
`agent-browser` repo after the installed default changed to the WSL-native
`chromium-stealthcdp` manifest:

```text
/home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/chrome-linux/chrome
```

`agent-browser install doctor --json` passed and reported:

```text
defaultBrowserBuild: stealthcdp_chromium
executablePathSource: manifest
stealthCdpChromiumReady: true
executablePath: /home/ecochran76/workspace.local/chromium/artifacts/chromium-stealthcdp/150.0.7835.0+stealthcdp.3676a7503929/chrome-linux/chrome
```

The service-owned no-launch route also selected the WSL-native stealth binding:

```bash
agent-browser --json service browser-capability preflight \
  --browser-build stealthcdp_chromium \
  --target-service-id ups \
  --account-id ups-smoke \
  --url 'https://www.ups.com/track?tracknum=1Z035CX1YW53854301' \
  --headless \
  --service-name Odollo \
  --agent-name codex \
  --task-name upsCarrierSmoke
```

Result:

```text
applied: true
bindingId: default-stealthcdp-wsl-native
executableId: stealthcdp-chromium-wsl-promoted
hostId: local-wsl-linux
reason: validated_binding_applied
```

The direct WSL-native headless smoke still failed at navigation:

```bash
agent-browser --json \
  --session ups-wsl-headless-<timestamp> \
  --profile /tmp/agent-browser-ups-wsl-headless-<timestamp> \
  batch --bail \
  "open https://www.ups.com/track?tracknum=1Z035CX1YW53854301&loc=en_US&requester=ST/trackdetails" \
  "wait 10000" \
  "get title" \
  "get url" \
  "eval document.body.innerText.slice(0, 1000)" \
  "close"
```

Result:

```json
[
  {
    "command": [
      "open",
      "https://www.ups.com/track?tracknum=1Z035CX1YW53854301&loc=en_US&requester=ST/trackdetails"
    ],
    "error": "Navigation failed: net::ERR_HTTP2_PROTOCOL_ERROR",
    "success": false
  }
]
```

The direct WSL-native headed smoke succeeded against the same URL:

```bash
agent-browser --json --headed \
  --session ups-wsl-headed-<timestamp> \
  --profile /tmp/agent-browser-ups-wsl-headed-<timestamp> \
  batch \
  "open https://www.ups.com/track?tracknum=1Z035CX1YW53854301&loc=en_US&requester=ST/trackdetails" \
  "wait 10000" \
  "get title" \
  "get url" \
  "eval document.body.innerText.slice(0, 1000)" \
  "close"
```

Result:

```text
title: Tracking | UPS - United States
url: https://www.ups.com/track?tracknum=1Z035CX1YW53854301&loc=en_US&requester=ST/trackdetails
body excerpt: Delivered by Local Post Office ... Your package has been delivered by U.S. Postal Service (USPS).
```

The WSL-native headless `--disable-http2` retry did not fix the site. It hung
until the outer 90 second timeout killed the batch. A follow-up `agent-browser
--session ups-wsl-headless-noh2-20260517122531 close` closed the leftover
browser.

The Windows `chromium-stealthcdp` build was also retested headless from WSL
with a Windows-mounted isolated profile:

```bash
agent-browser --json \
  --session ups-win-headless-<timestamp> \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --profile /mnt/c/Users/ecoch/AppData/Local/Temp/agent-browser-ups-win-headless-<timestamp> \
  batch --bail \
  "open https://www.ups.com/track?tracknum=1Z035CX1YW53854301&loc=en_US&requester=ST/trackdetails" \
  "wait 10000" \
  "get title" \
  "get url" \
  "eval document.body.innerText.slice(0, 1000)" \
  "close"
```

It failed the same way:

```text
Navigation failed: net::ERR_HTTP2_PROTOCOL_ERROR
```

The Windows headless stderr also contained repeated `WebGL1 blocklisted`
messages. That means Windows-hosted headless Chromium is not currently a better
UPS posture than WSL-native headless Chromium on this workstation.

Relevant stderr evidence:

```text
chrome-4062348-1779038706274.stderr.log: headless UPS failed with repeated WebGL1 blocklisted and GPU command buffer initialization errors.
chrome-4064982-1779038732052.stderr.log: headless --disable-http2 run repeatedly failed EGL or ANGLE Vulkan initialization and timed out.
chrome-118618-1779041053860.stderr.log: Windows headless UPS failed with repeated WebGL1 blocklisted messages after the same HTTP/2 navigation failure.
```

This narrows the finding: UPS is not just failing because the older smoke used
the Windows executable from WSL. With the WSL-native stealth binary, headless
still fails and headed succeeds. With the Windows stealth binary, headless also
fails. The current `--disable-http2` workaround is not usable for UPS.

## Agent Workflow Mistake

The smoke manually supplied both:

```text
--executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe
--browser-profile /tmp/odollo-stealthcdp-ups-smoke-profile
```

That was not the ideal agent-browser workflow. For a new site or operational
service, the better path is:

1. Ask agent-browser for an access plan with service, agent, task, target
   service, URL, and preferred browser build.
2. Let agent-browser select or create the compatible profile and browser build.
3. Request the browser tab through the service control plane.
4. Inspect trace, browser capability evidence, and profile compatibility
   records if the route fails.

The skill text already says not to create a new runtime profile merely because
another automation might be active, and to prefer the service/access-plan
control plane. However, the Odollo carrier command currently exposes low-level
`--browser-profile` and `--executable-path` knobs and defaults to stock Chrome,
so a downstream caller can still bypass the intended routing model.

## Product Gaps

- `agent-browser` should make it harder for agents to skip access-plan/profile
  routing when the task is site automation rather than a low-level browser
  binary smoke.
- The service/access-plan path should have a concise copyable carrier-site
  recipe, for example a UPS target service that can recommend
  `stealthcdp_chromium`, headed versus headless, profile compatibility, and
  fallback posture before launch.
- The browser capability preflight output was terse. For this case, it surfaced
  `reason=explicit_executable_path`, but the operator-facing implication should
  be clearer: explicit executable/profile overrides mean the caller is bypassing
  normal brokered browser-build and profile routing.
- Odollo should stop treating browser build and profile choice as its own
  low-level concern. It should call a service-owned agent-browser request or an
  equivalent access-plan-guided command so profile selection, browser build,
  lease handling, and site policy stay centralized in agent-browser.

## Recommended Next Steps

1. Add or update a UPS site policy so carrier tracking prefers headed
   `stealthcdp_chromium` through the WSL-native executable and does not assume
   headless stealth is sufficient.
2. Treat UPS headless `stealthcdp_chromium` as a known-bad posture until a
   lower-level Chromium or launch-argument fix resolves the HTTP/2 navigation
   error and GPU/WebGL initialization failures.
3. Add a focused regression smoke that proves the UPS route can load headed and
   extract the delivered status text without requiring Odollo to choose the
   browser executable or profile directly.
4. Fix or inspect the `service browser-capability preflight --headed` path,
   because the headed preflight invocation echoed `headless=true` in the JSON
   request during follow-up testing.
5. Update the agent-browser skill with a stronger warning: for site smokes,
   use access-plan/service request first; explicit executable/profile flags are
   only for binary validation or operator-directed override.

## Validation

Read-only validation was run only. No repo code was changed by the smoke.

Passed:

```bash
agent-browser install doctor
```

Passed for basic headless launch:

```bash
agent-browser --session stealth-smoke-direct \
  --executable-path /mnt/c/Users/ecoch/AppData/Local/chromium-stealthcdp/current/chrome.exe \
  --profile /tmp/odollo-stealthcdp-direct-smoke \
  batch --bail "open https://example.com" "get title" "close"
```

Failed for UPS headless navigation as described above.

Follow-up validation:

Passed:

```bash
agent-browser install doctor --json
```

Passed no-launch WSL-native route selection:

```bash
agent-browser --json service browser-capability preflight \
  --browser-build stealthcdp_chromium \
  --target-service-id ups \
  --account-id ups-smoke \
  --url 'https://www.ups.com/track?tracknum=1Z035CX1YW53854301' \
  --headless \
  --service-name Odollo \
  --agent-name codex \
  --task-name upsCarrierSmoke
```

Failed for WSL-native headless UPS navigation with
`net::ERR_HTTP2_PROTOCOL_ERROR`.

Passed for WSL-native headed UPS navigation and extracted delivered status text.

Failed for WSL-native headless UPS navigation with `--disable-http2`; the batch
timed out after 90 seconds and required an explicit close.

Failed for Windows `chromium-stealthcdp` headless UPS navigation with a
Windows-mounted profile and the same `net::ERR_HTTP2_PROTOCOL_ERROR`.

## Follow-up: built-in UPS headed policy

The accepted operational direction is to treat UPS as a headed browser site
until true headless can be repaired. A hidden or uncluttered headed browser is
acceptable if it is remotely viewable by operators.

The repo now has a built-in `ups` site policy that recommends:

```text
browserBuild: stealthcdp_chromium
browserHost: remote_headed
requiresCdpFree: false
interactionMode: human_like_input
maxParallelSessions: 1
```

Access-plan tab requests for headed policies now include:

```json
{
  "params": {
    "headless": false,
    "browserHost": "remote_headed"
  }
}
```

This keeps the normal CDP-backed control plane available while preventing
software clients from accidentally launching true headless Chrome for UPS or
dropping the host selected by the site policy. A workspace CLI check confirmed
that the UPS tracking URL resolves to
`remote_headed`, `remoteViewRecommended=true`, `browserBuild=stealthcdp_chromium`,
and `serviceRequest.request.params.headless=false`.

## Follow-up: remote-headed implementation

The daemon now treats copied `remote_headed` access-plan requests as executable
launch hints. For service requests whose params include
`browserHost=remote_headed`, auto-launch forces headed mode, records the browser
host as `remote_headed`, and persists a `viewStreams` entry on the browser
record. On Linux, if `DISPLAY` is unset and the selected executable is not a
Windows-mounted browser, agent-browser starts a private Xvfb display so the
headed browser stays off the operator desktop while CDP control remains
available.

Operators can override the hidden display and view metadata with:

```text
AGENT_BROWSER_REMOTE_HEADED_DISPLAY
AGENT_BROWSER_REMOTE_VIEW_URL
AGENT_BROWSER_REMOTE_VIEW_PROVIDER
```

This is the first service-owned hidden headed host. It does not yet start a
full noVNC or WebRTC gateway; it records external view URLs when one is
configured and otherwise relies on the existing agent-browser CDP screencast
stream and dashboard surfaces.

## Follow-up: RDP gateway view streams

The service model now accepts `rdp_gateway` as a first-class `viewStream`
provider. This is intentionally an HTML5 gateway contract, not raw RDP protocol
termination inside the dashboard. Operators can point
`AGENT_BROWSER_REMOTE_VIEW_URL` at a Guacamole, FreeRDP-WebConnect, or similar
gateway URL and set `AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway`; the
dashboard browser detail view embeds that URL and also provides an external
open action.

## Follow-up: local RDP backend readiness

On 2026-05-17, the local workstation installed the apt-provided RDP backend
pieces for the gateway path:

```text
guacd
libguac-client-rdp0t64
xrdp
xorgxrdp
freerdp2-x11
```

Live checks showed `guacd`, `xrdp`, and `xrdp-sesman` active. `guacd` listened
on `127.0.0.1:4822`; `xrdp` listened on `3389`; TCP checks to both ports
passed. Ubuntu noble provides `guacd` and the RDP plugin but not the Guacamole
web application package, so this is backend readiness only. The browser-facing
HTML5 gateway URL still needs a Guacamole webapp, container, or equivalent
frontend.

The repo now includes:

```text
pnpm test:rdp-gateway-readiness-live
```

That smoke reports backend readiness and optional
`AGENT_BROWSER_REMOTE_VIEW_URL` reachability. Add `--require-html5-client`
when the HTML5 gateway URL must be available before a workstation is considered
ready.

## Follow-up: local Guacamole web gateway

The browser-facing HTML5 gateway was installed as a user-scoped Docker Compose
stack under:

```text
~/.agent-browser/guacamole
```

It runs:

```text
agent-browser-guacamole-postgres
agent-browser-guacd
agent-browser-guacamole
```

The web UI is bound to localhost only:

```text
http://127.0.0.1:8092/guacamole/
```

The default `8082` candidate was not used because it was already occupied by
`ragmail-reranker-local-user`. The Compose `guacd` container includes
`host.docker.internal:host-gateway`, and the seeded Guacamole connection named
`Local XRDP (agent-browser host)` targets `host.docker.internal:3389`.

The workstation now has:

```text
~/.agent-browser/.env
```

with:

```text
AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway
AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/#/client/MQBjAHBvc3RncmVzcWw=
```

Validation passed with:

```text
AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/#/client/MQBjAHBvc3RncmVzcWw= \
  node scripts/smoke-rdp-gateway-readiness.js --require-html5-client
```

The seeded RDP connection does not embed OS credentials; operators should
authenticate to XRDP interactively or configure a dedicated least-privilege
account later.

## Follow-up: pinned ports and user-scoped secrets

The local Guacamole stack was tightened after initial setup:

```text
~/.agent-browser/secrets/guacamole.env
~/.agent-browser/guacamole/.env
~/.agent-browser/.env
```

`~/.agent-browser/secrets/guacamole.env` is mode `0600` and stores the generated
PostgreSQL and Guacamole admin secrets. `~/.agent-browser/guacamole/.env` is
also mode `0600` and pins the stable ingress binding:

```text
AGENT_BROWSER_GUACAMOLE_BIND_ADDRESS=127.0.0.1
AGENT_BROWSER_GUACAMOLE_HTTP_PORT=8092
```

The Compose port declaration uses those variables, so local URL ingress can
depend on `127.0.0.1:8092` without reading the Compose file. The default
Guacamole `guacadmin` password was rotated through the Guacamole API. A live
check confirmed the old default password returns HTTP 403 and the generated
secret login returns HTTP 200 with an auth token.

`scripts/smoke-rdp-gateway-readiness.js` now loads `~/.agent-browser/.env`
automatically when `AGENT_BROWSER_REMOTE_VIEW_URL` is not already present in
the process environment, so the readiness check can verify the pinned
workstation URL without repeating exports.

## Follow-up: Authelia-protected agent-browser subdomain ingress

The agent-browser subdomain was published through cooper-service-ingress as
service `agent-browser` on pinned upstream port `8092`. Authelia guards the
entire `agent-browser.ecochran.dyndns.org` host router; it is not a separate
Guacamole-only path guard. Guacamole is the current upstream app under that
agent-browser-owned subdomain.

Durable cooper inventory:

```text
/home/ecochran76/workspace.local/cooper-webservices/services/agent-browser.json
```

Local route:

```text
http://agent-browser.localhost/guacamole/
```

Protected external route:

```text
https://agent-browser.ecochran.dyndns.org/guacamole/#/client/MQBjAHBvc3RncmVzcWw=
```

The external route uses bastion Traefik plus Authelia. Validation showed
unauthenticated HTTPS requests return `302` to `auth.ecochran.dyndns.org`.
`~/.agent-browser/.env` was updated so agent-browser records the protected
external URL as the default `rdp_gateway` view stream URL.

The host root also redirects to `/guacamole/` after Authelia permits the
request. This preserves host-wide Authelia protection while avoiding Guacamole's
Tomcat-context `404` at `/`.

## Follow-up: Guacamole trusted-header login under Authelia

Guacamole is treated as an agent-browser viewer component rather than a second
application boundary. The user-scoped Compose stack enables the bundled
Guacamole HTTP header authentication extension:

```text
HEADER_ENABLED=true
HTTP_AUTH_HEADER=Remote-User
POSTGRESQL_AUTO_CREATE_ACCOUNTS=true
```

The bastion Authelia middleware already forwards `Remote-User`, `Remote-Groups`,
`Remote-Name`, and `Remote-Email` after successful authentication. Guacamole now
trusts `Remote-User` and loads the header extension ahead of PostgreSQL auth, so
an Authelia-authenticated operator should not see a second Guacamole login
screen.

The Guacamole PostgreSQL database contains user entries for the Authelia
identities observed on this host:

```text
ecochran76
ecochran76@gmail.com
ECOCHRAN76@GMAIL.COM
```

Each identity has `READ` permission on connection `1`, named `Local XRDP
(agent-browser host)`.

Validation:

```text
curl -X POST \
  -H 'Remote-User: ecochran76' \
  -H 'Content-Type: application/x-www-form-urlencoded' \
  --data '' \
  http://127.0.0.1:8092/guacamole/api/tokens
```

returned HTTP 200 with `dataSource: "header"`. Using that token against
`/guacamole/api/session/data/postgresql/connections` returned the seeded local
XRDP connection. A cooper host-header probe against `http://192.168.50.108`
with `Host: agent-browser.ecochran.dyndns.org` and `Remote-User: ecochran76`
also returned a header-auth token, which verifies the local ingress hop
preserves the trusted header. Unauthenticated external access to
`https://agent-browser.ecochran.dyndns.org/guacamole/` still returns `302` to
`auth.ecochran.dyndns.org`, so the trusted header is only accepted after the
agent-browser host route has passed through Authelia.

## Follow-up: XRDP desktop login is still a separate OS boundary

After Guacamole trusted-header login, the next visible prompt is the XRDP/Xvnc
desktop login screen. That is the host operating system session, not another
Guacamole application login. The intended default is a dedicated low-privilege
agent-browser desktop account whose generated password is stored in the
user-scoped Guacamole secret file and passed only from Guacamole to XRDP.

The setup helper is:

```text
scripts/setup-rdp-autologin-user.sh
```

It creates or updates `agent-browser-rdp`, writes a minimal Openbox/Xterm
session, stores `XRDP_AGENT_BROWSER_USERNAME` and
`XRDP_AGENT_BROWSER_PASSWORD` in `~/.agent-browser/secrets/guacamole.env`,
updates Guacamole connection `1` with the XRDP username and password, disables
drive and microphone redirection, and restarts `xrdp-sesman` and `xrdp`.

This helper requires interactive `sudo` because creating a local login account
is an OS-level change. The Codex shell could not complete that root step because
non-interactive sudo was blocked by policy on this host. The operator ran the
helper from an interactive shell on 2026-05-17.

Post-setup verification:

```text
agent-browser-rdp:x:1001:1002:agent-browser RDP viewer session:/home/agent-browser-rdp:/bin/bash
xrdp active
xrdp-sesman active
```

Guacamole connection `1` now includes the XRDP `username` and `password`
parameters, with the password stored in
`~/.agent-browser/secrets/guacamole.env` rather than the repo. Drive and
microphone redirection are disabled for the connection.

Guacamole logs then showed the real authenticated path:

```text
User "ecochran76@gmail.com" successfully authenticated
User "ecochran76@gmail.com" connected to connection "1"
```

This verifies that the browser path reaches the seeded XRDP connection after
Authelia and Guacamole trusted-header login.

## Follow-up: dashboard root remains the operations UX

The proof of concept currently lets an operator navigate directly to the
Guacamole connection URL and see a basic Linux desktop. That is acceptable for
validating the remote-view plumbing, but it is not the intended product shape.

The `agent-browser.ecochran.dyndns.org` root should become the React dashboard
served through the agent-browser service. Guacamole should remain a subordinate
view-stream provider that is opened from the dashboard when an operator chooses
to inspect a hidden RDP-backed browser or tab.

Required UX direction:

- root page: React/Vite dashboard for agent-browser service operations
- primary navigation: service fleet, browser sessions, profiles, jobs,
  incidents, and trace views
- session view: tree expansion from service to browser to tab
- tab rows: URL, title, lifecycle, last service/agent/task access, and activity
  indicators
- hidden RDP tabs: interaction icon that opens the Guacamole stream in an
  iframe with fullscreen toggle and external-open fallback
- focus contract: before the iframe opens, agent-browser should make the
  selected browser/tab foreground and maximized within the remote desktop
  viewport

The next implementation slice should therefore expose the existing dashboard,
not Guacamole, as the externally routed root app. The current Guacamole URL can
remain an internal iframe target and emergency fallback.

## Follow-up: root route now serves the dashboard

The cooper ingress route was updated after the remote-view proof of concept so
the agent-browser subdomain root no longer redirects to Guacamole.

Durable cooper inventory now treats the dashboard as the primary upstream:

```text
/home/ecochran76/workspace.local/cooper-webservices/services/agent-browser.json
```

Routing:

```text
http://agent-browser.localhost/               -> dashboard on 127.0.0.1:4848
http://agent-browser.localhost/guacamole      -> Guacamole on 127.0.0.1:8092
http://agent-browser.localhost/guacamole/     -> Guacamole on 127.0.0.1:8092
https://agent-browser.ecochran.dyndns.org/    -> Authelia, then dashboard
https://agent-browser.ecochran.dyndns.org/guacamole/ -> Authelia, then Guacamole
```

`cooper-webservices` gained a reusable `path_routes` inventory field so a
service host can route subpaths to component upstreams without making that
component the root application. For agent-browser, `/guacamole` is the
path-specific viewer route and `/` remains the dashboard route.

Validation:

```text
python3 skills/cooper-service-ingress/scripts/validate_service_inventory.py --service agent-browser --strict
python3 scripts/render_traefik_config.py
python3 -m py_compile scripts/render_traefik_config.py skills/cooper-service-ingress/scripts/validate_service_inventory.py skills/cooper-service-ingress/scripts/publish_bastion_service.py
```

Local and cooper host-header smokes returned:

```text
http://agent-browser.localhost/ -> 200 dashboard HTML
http://agent-browser.localhost/guacamole -> 302 /guacamole/
http://agent-browser.localhost/guacamole/ -> 200 Guacamole HTML
Host agent-browser.ecochran.dyndns.org http://192.168.50.108/ -> 200 dashboard HTML
Host agent-browser.ecochran.dyndns.org http://192.168.50.108/guacamole -> 302 /guacamole/
Host agent-browser.ecochran.dyndns.org http://192.168.50.108/guacamole/ -> 200 Guacamole HTML
```

Unauthenticated public HTTPS smokes for both `/` and `/guacamole/` still return
`302` to `auth.ecochran.dyndns.org`. Bastion Traefik snippet
`/var/homelabos/traefik-portainer/conf.d/agent-browser-cooper.yml` was
republished without the old root redirect. The Authelia rule was not changed;
it already guards the whole host.

## Follow-up: dashboard is a durable user service

The dashboard is no longer only a foreground process started by
`agent-browser dashboard start`. The user-scoped install now has a systemd user
service that keeps the dashboard available on the stable ingress port.

Runtime unit:

```text
~/.config/systemd/user/agent-browser-dashboard.service
```

Reproducible installer:

```text
scripts/install-dashboard-user-service.sh
```

The installer resolves `agent-browser` from `PATH`, unless `AGENT_BROWSER_BIN`
is set, writes the user unit, reloads systemd, and runs:

```text
systemctl --user enable --now agent-browser-dashboard.service
```

Service contract:

```text
AGENT_BROWSER_DASHBOARD=1
AGENT_BROWSER_DASHBOARD_PORT=4848
ExecStart=/home/ecochran76/.local/bin/agent-browser
```

Validation:

```text
bash -n scripts/install-dashboard-user-service.sh
bash scripts/install-dashboard-user-service.sh
systemctl --user is-enabled agent-browser-dashboard.service -> enabled
systemctl --user is-active agent-browser-dashboard.service -> active
ss -ltnp -> 127.0.0.1:4848 owned by /home/ecochran76/.local/bin/agent-browser
curl -I http://127.0.0.1:4848/ -> 200 OK
curl -I http://agent-browser.localhost/ -> 200 OK
curl -I -H 'Host: agent-browser.ecochran.dyndns.org' http://192.168.50.108/ -> 200 OK
pnpm --dir docs build -> passed
```

This keeps `/` as the React dashboard route while `/guacamole/` remains the
subordinate remote-view provider for iframe inspection and emergency direct
access.

## Follow-up: dashboard remote-view inspection

The dashboard now treats browser view streams as first-class inspection
surfaces instead of making operators navigate directly to Guacamole.

Implemented dashboard behavior:

```text
Browser details -> Inspect on an embeddable view stream -> stream dialog
Service tab row -> View -> queued view_focus when a stable tab index exists -> stream dialog
```

The stream dialog embeds providers accepted by
`packages/dashboard/src/lib/service-view-streams.ts`, including
`rdp_gateway`, and includes a fullscreen toggle plus a direct-open fallback.
The account chip menu now owns the dashboard light/dark toggle; the old
sessions-header theme button was removed.

The tab inspection path requests `view_focus` through
`POST /api/service/request`, which is the correct service-owned queue path for
CDP-backed tabs. `view_focus` switches to the retained tab index when supplied,
calls `Page.bringToFront`, and asks Chrome to maximize the containing native
window before the dashboard opens the iframe. If the browser host cannot honor
`Browser.setWindowBounds`, the action reports `maximized: false` and preserves
`maximizeError` instead of blocking inspection.

Validation for the `view_focus` slice:

```text
pnpm generate:service-client
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm test:service-client
pnpm test:dashboard-view-streams
pnpm --dir packages/dashboard build
pnpm --dir docs build
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_request_command_accepts_contract_actions -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
pnpm test:service-api-mcp-parity
git diff --check
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
```

## Follow-up: user install refreshed and live remote-view focus smoke

The user-scoped install was rebuilt from this workspace and restarted so the
dashboard service serves the updated bundled dashboard assets and service
request action set.

Installed binary paths refreshed:

```text
/home/ecochran76/.local/bin/agent-browser
/home/ecochran76/.local/share/pnpm/global/5/node_modules/agent-browser/bin/agent-browser-linux-x64
/home/ecochran76/workspace.local/agent-browser/bin/agent-browser-linux-x64
```

Service restart and route checks:

```text
systemctl --user restart agent-browser-dashboard.service
systemctl --user is-active agent-browser-dashboard.service -> active
curl -I http://127.0.0.1:4848/ -> 200 OK
curl -I http://agent-browser.localhost/ -> 200 OK
curl -I -H 'Host: agent-browser.ecochran.dyndns.org' http://192.168.50.108/ -> 200 OK
agent-browser install doctor -> no install drift detected
```

Live smoke used an isolated `AGENT_BROWSER_HOME`, session
`view-focus-live-1779111848`, and an isolated profile under
`/tmp/agent-browser-view-focus-live.XBUTAx`. The session launched a
`remote_headed` browser through `POST /api/service/request` with:

```text
AGENT_BROWSER_REMOTE_VIEW_PROVIDER=rdp_gateway
AGENT_BROWSER_REMOTE_VIEW_URL=http://agent-browser.localhost/guacamole/
params.browserHost=remote_headed
params.headless=false
```

The retained browser record for the smoke session reported:

```json
{
  "id": "session:view-focus-live-1779111848",
  "host": "remote_headed",
  "health": "ready",
  "viewStreams": [
    {
      "id": "remote-headed-view",
      "provider": "rdp_gateway",
      "readOnly": false,
      "url": "http://agent-browser.localhost/guacamole/"
    }
  ],
  "cdpEndpoint": true
}
```

The queued `view_focus` request succeeded:

```json
{
  "broughtToFront": true,
  "maximizeRequested": true,
  "maximized": true,
  "tabSwitch": {
    "index": 0,
    "title": "View Focus Smoke"
  }
}
```

The smoke browser was closed and the isolated stream was disabled after the
check.
