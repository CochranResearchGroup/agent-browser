# Authentication Patterns

Login flows, session persistence, OAuth, 2FA, and authenticated browsing.

**Related**: [session-management.md](session-management.md) for state persistence details, [SKILL.md](../SKILL.md) for quick start.

## Contents

- [Choose an authentication path](#choose-an-authentication-path)
- [Standalone Auth Receipts](#standalone-auth-receipts)

- [Managed Runtime Profiles](#managed-runtime-profiles)
- [Import Auth from Your Browser](#import-auth-from-your-browser)
- [Persistent Profiles](#persistent-profiles)
- [Session Persistence](#session-persistence)
- [Basic Login Flow](#basic-login-flow)
- [Saving Authentication State](#saving-authentication-state)
- [Restoring Authentication](#restoring-authentication)
- [OAuth / SSO Flows](#oauth--sso-flows)
- [Two-Factor Authentication](#two-factor-authentication)
- [HTTP Basic Auth](#http-basic-auth)
- [Cookie-Based Auth](#cookie-based-auth)
- [Token Refresh Handling](#token-refresh-handling)
- [Security Best Practices](#security-best-practices)

## Choose an authentication path

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

Examples: use a service skill's dedicated agent username with its matching private password file; use a password manager's saved username with that saved password; retain an existing authenticated session; or follow an explicit user-directed SSO/manual flow. Use Auth for generic discovery and owner-reviewed login when those paths do not already resolve the task. A failure in Auth does not cancel a separately authorized path, and a source-specific permission or actual rejected login still needs its own resolution.

## Standalone Auth Receipts

The public Auth helper exposes `inspectLoginProgress({loginId})` or `inspectLoginProgress({requestId})` for read-only, evidence-backed progress. `summarizeLoginProgress` projects an existing public observation without effects. Selection, request submission, confirmed actionable delivery, expiry, denial and unknown input remain distinct; a review state or URL alone never means the operator received a request. Native operator ownership is metadata, not proof of human activity. Follow the returned read-only next operation; the summary grants no credential or Native input authority.

After durable owner waiting, the playbook receives the Auth decision before private use. An Auth-issued approved unused permission may start a new use window for the same request and scope. The helper saves that permission reference and deadline, then freshly validates the original receiving plan before input. The original caller intent stays unchanged; attempted, uncertain, changed or unpersisted authority remains held.

When using Auth for a browser login, read Auth's `auth-receipts://guides/login-orchestration` resource before asking for a phone number or mailbox address already held by connected services. Both connected Auth helpers expose `discoverLoginChannels({tenant, smsSuffix, emailHint, im, mail})`, forwarding only to the verified Auth SDK. It composes read-only IM/Mail MCP account metadata and bounded source evidence into destination proposals with provenance; it cannot authorize, arm, send, consume or submit input. Keep the original task/request, qualify fresh receiving state and use Auth's normal owner decision. Missing SDK support reports `auth_channel_discovery_unavailable` and requires installation repair, without an alternate client or private-store fallback.

Auth provides the generic discovery, owner-review and private-delivery workflow;
Bots is another consumer. Credentials selected from Auth remain subject to its
SDK and approval policy. The Node `connectHostedAuthHelper()` adapter discovers
Auth's verified SDK at its data-directory `auth-receipts/client/current` selector.
Missing Auth SDK capabilities require diagnostics for that path; they do not
forbid a separately authorized stored credential or service-specific path.
Do not read an Auth-managed vault directly to bypass its policy. Private
discovery and bearer material stay in the SDK. Owner decisions, required
delivery and exact browser binding still gate Auth input. Never repeat an
unknown credential attempt.

For an explicitly enrolled unattended application, use the separate
`connectEnrolledAuthHelper({discoveryFile, operationContext})` export from
`@agent-browser/client/hosted-auth-helper`. Context includes canonical UUID
operationId/recoveryId and exactly taskId/browserId/profileId/tabId/origin/
controlGeneration, with a canonical HTTPS origin. The verified Auth SDK's async
`createEnrolledAuthClient` owns discovery, consumer identity and live binding
validation. No Codex caller or environment discovery fallback is used. A missing
factory or rejected enrollment is a producer/installation prerequisite; do not
copy thread IDs, inspect connection files or export browser credentials.
Both entry points share owner-capability checks and the original private attempt
handling. This source API does not install or enroll a service consumer.
Use current `auth_capabilities` before the owner login example in
[playbooks.md](playbooks.md#private-login-composition). Current credential discovery
is `discover_credentials({query, accountHint})`; legacy `list_accounts` only lists
configured caller scopes. An empty legacy list is not an empty vault. Missing
discovery authority (`auth_not_authorized`), `partial` source searches, completed
`not_found` searches and `choice_required` account selection are separate outcomes.
Partial searches with unavailable sources do not identify whether a source is
locked, expired or absent. Auth's owner inspects source health and recovers its
configured service-owned source session; caller enrollment is a different concern.
The public owner playbook preserves these distinctions without creating a request
or selecting the first available candidate. Personal/company intent must be exact.

For a supported saved-password fallback, retain the exact owned browser and
account/origin. Qualify Native owner authentication, consumer input capability and
private capture/input guards before selection or submission. A visible viewer
alone is insufficient; source recovery is not authority to borrow another desktop,
change enrollment or replay uncertain input. SDK/example qualification does not
establish installed provider login acceptance.

Before private use is attempted, owner login may renew a typed `plan_stale` hold caused by owner-review delay. Renewal requires complete, unchanged guide, control, expected-state and task/profile/browser/tab bindings with the same receiving generation. The fresh plan is validated and durably saved against the original request before dispatch intent. Missing or changed proof stays held; attempted or unknown input never renews a plan or creates replacement approval.

Use `createOwnerLoginPlaybook` with trusted browser routines, connected `auth` and
metadata checkpoints for new requests. `createOwnerTotpContinuation` composes
Auth's observed factor action after password/SSO. Retain the original password request ID for account-less factor screens; Auth resolves its binding and creates the factor request through the observed continuation. When `factor_choice_required` is returned, use the supplied issuer/account labels to select a returned `recordId` and resume the same checkpoint. Do not rediscover or guess among opaque IDs. Number matching follows the
original attempt with `createOwnerNumberMatchingContinuation`. Exact generation,
privacy and identity evidence are required. Unsupported paths hold. Owner
`reconcile: true` cannot create or repeat credential use. Legacy checkpoints keep
their original issuer. Input/submission acknowledgement never proves login.

## Managed task receiving

Agent Browser and the Auth Receipts service own the receiving integration.
Agents pass the existing opaque browser task target through the supported login
flow; operators do not maintain per-task receiving files. The Auth host resolves
retained task metadata, checks live physical ownership and cooperative control,
and obtains dashboard or Slack-via-Receipts approval before private input.
Task-briefing process IDs are retained evidence and never authorize input alone.
A missing service adapter or unsupported task control is an integration failure
with the original request preserved. Do not request a redundant operator login,
read vault values, substitute targets or replay an unknown input attempt.
Installed workflow qualification remains separate from source support.

## Managed Runtime Profiles

Managed runtime profiles are the default long-lived authentication path for
agent-browser. They persist a real Chrome user-data-dir under
`~/.agent-browser/runtime-profiles/<name>/user-data` and work well for
recurring agent tasks.

```bash
# Default runtime profile
agent-browser runtime login https://app.example.com/login

# Named runtime profile
agent-browser --runtime-profile work runtime create work --set-default
agent-browser --runtime-profile work runtime login https://app.example.com/login

# Later automation reuses the same profile automatically
agent-browser --runtime-profile work open https://app.example.com/dashboard
```

For Google, Gmail, and similar SSO flows, use a two-phase workflow:

```bash
# Phase 1: sign in without DevTools
agent-browser --runtime-profile google-login runtime login https://accounts.google.com
# User closes Chrome after sign-in

# Phase 2: relaunch the same profile with DevTools for automation
agent-browser --runtime-profile google-login runtime login https://myaccount.google.com --attachable
agent-browser --runtime-profile google-login runtime status
agent-browser --runtime-profile google-login get url
agent-browser --runtime-profile google-login get title
```

Run those first post-relaunch reads sequentially. If `runtime status` shows
`Browser alive: true` and the first read fails, retry once before treating the
profile as broken.

Use `--leave-open` or `runtimeProfiles.<name>.launch.leaveOpen` when you want
`close` to detach from a managed runtime-profile browser instead of shutting it
down.

## Import Auth from Your Browser

The fastest way to authenticate is to reuse cookies from a Chrome session you are already logged into.

**Step 1: Start Chrome with remote debugging**

```bash
# macOS
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --remote-debugging-port=9222

# Linux
google-chrome --remote-debugging-port=9222

# Windows
"C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
```

Log in to your target site(s) in this Chrome window as you normally would.

> **Security note:** `--remote-debugging-port` exposes full browser control on localhost. Any local process can connect and read cookies, execute JS, etc. Only use on trusted machines and close Chrome when done.

**Step 2: Grab the auth state**

```bash
# Auto-discover the running Chrome and save its cookies + localStorage
agent-browser --auto-connect state save ./my-auth.json
```

**Step 3: Reuse in automation**

```bash
# Load auth at launch
agent-browser --state ./my-auth.json open https://app.example.com/dashboard

# Or load into an existing session
agent-browser state load ./my-auth.json
agent-browser open https://app.example.com/dashboard
```

This works for any site, including those with complex OAuth flows, SSO, or 2FA -- as long as Chrome already has valid session cookies.

> **Security note:** State files contain session tokens in plaintext. Add them to `.gitignore`, delete when no longer needed, and set `AGENT_BROWSER_ENCRYPTION_KEY` for encryption at rest. See [Security Best Practices](#security-best-practices).

**Tip:** Combine with `--session-name` so the imported auth auto-persists across restarts:

```bash
agent-browser --session-name myapp state load ./my-auth.json
# From now on, state is auto-saved/restored for "myapp"
```

## Persistent Profiles

Use `--profile` when you need a custom unmanaged Chrome user-data-dir path.
Prefer managed runtime profiles unless you explicitly need a path outside
`~/.agent-browser/runtime-profiles`.

Use `--profile` to point agent-browser at a Chrome user data directory. This persists everything (cookies, IndexedDB, service workers, cache) across browser restarts without explicit save/load:

```bash
# First run: login once
agent-browser --profile ~/.myapp-profile open https://app.example.com/login
# ... complete login flow ...

# All subsequent runs: already authenticated
agent-browser --profile ~/.myapp-profile open https://app.example.com/dashboard
```

Use different paths for different projects or test users:

```bash
agent-browser --profile ~/.profiles/admin open https://app.example.com
agent-browser --profile ~/.profiles/viewer open https://app.example.com
```

Or set via environment variable:

```bash
export AGENT_BROWSER_PROFILE=~/.myapp-profile
agent-browser open https://app.example.com/dashboard
```

## Session Persistence

Use `--session-name` for lightweight cookie and storage reuse. Prefer runtime
profiles when the browser identity itself matters, especially for Google sign-in,
password-manager behavior, service workers, or other profile-level state.

Use `--session-name` to auto-save and restore cookies + localStorage by name, without managing files:

```bash
# Auto-saves state on close, auto-restores on next launch
agent-browser --session-name twitter open https://twitter.com
# ... login flow ...
agent-browser close  # state saved to ~/.agent-browser/sessions/

# Next time: state is automatically restored
agent-browser --session-name twitter open https://twitter.com
```

Encrypt state at rest:

```bash
export AGENT_BROWSER_ENCRYPTION_KEY=$(openssl rand -hex 32)
agent-browser --session-name secure open https://app.example.com
```

## Basic Login Flow

```bash
# Navigate to login page
agent-browser open https://app.example.com/login

# Get form elements
agent-browser snapshot -i
# Output: @e1 [input type="email"], @e2 [input type="password"], @e3 [button] "Sign In"

# Fill credentials
agent-browser fill @e1 "user@example.com"
agent-browser fill @e2 "password123"

# Submit
agent-browser click @e3
agent-browser wait --url "**/dashboard"

# Verify login succeeded
agent-browser get url  # Should be dashboard, not login
```

## Saving Authentication State

After logging in, save state for reuse:

```bash
# Login first (see above)
agent-browser open https://app.example.com/login
agent-browser snapshot -i
agent-browser fill @e1 "user@example.com"
agent-browser fill @e2 "password123"
agent-browser click @e3
agent-browser wait --url "**/dashboard"

# Save authenticated state
agent-browser state save ./auth-state.json
```

## Restoring Authentication

Skip login by loading saved state:

```bash
# Load saved auth state
agent-browser state load ./auth-state.json

# Navigate directly to protected page
agent-browser open https://app.example.com/dashboard

# Verify authenticated
agent-browser snapshot -i
```

## OAuth / SSO Flows

For OAuth redirects:

```bash
# Start OAuth flow
agent-browser open https://app.example.com/auth/google

# Handle redirects automatically
agent-browser wait --url "**/accounts.google.com**"
agent-browser snapshot -i

# Fill Google credentials
agent-browser fill @e1 "user@gmail.com"
agent-browser click @e2  # Next button
agent-browser wait 2000
agent-browser snapshot -i
agent-browser fill @e3 "password"
agent-browser click @e4  # Sign in

# Wait for redirect back
agent-browser wait --url "**/app.example.com**"
agent-browser state save ./oauth-state.json
```

## Two-Factor Authentication

Handle 2FA with manual intervention:

```bash
# Login with credentials
agent-browser open https://app.example.com/login --headed  # Show browser
agent-browser snapshot -i
agent-browser fill @e1 "user@example.com"
agent-browser fill @e2 "password123"
agent-browser click @e3

# Wait for user to complete 2FA manually
echo "Complete 2FA in the browser window..."
agent-browser wait --url "**/dashboard" --timeout 120000

# Save state after 2FA
agent-browser state save ./2fa-state.json
```

## HTTP Basic Auth

For sites using HTTP Basic Authentication:

```bash
# Set credentials before navigation
agent-browser set credentials username password

# Navigate to protected resource
agent-browser open https://protected.example.com/api
```

## Cookie-Based Auth

Manually set authentication cookies:

```bash
# Set auth cookie
agent-browser cookies set session_token "abc123xyz"

# Navigate to protected page
agent-browser open https://app.example.com/dashboard
```

## Token Refresh Handling

For sessions with expiring tokens:

```bash
#!/bin/bash
# Wrapper that handles token refresh

STATE_FILE="./auth-state.json"

# Try loading existing state
if [[ -f "$STATE_FILE" ]]; then
    agent-browser state load "$STATE_FILE"
    agent-browser open https://app.example.com/dashboard

    # Check if session is still valid
    URL=$(agent-browser get url)
    if [[ "$URL" == *"/login"* ]]; then
        echo "Session expired, re-authenticating..."
        # Perform fresh login
        agent-browser snapshot -i
        agent-browser fill @e1 "$USERNAME"
        agent-browser fill @e2 "$PASSWORD"
        agent-browser click @e3
        agent-browser wait --url "**/dashboard"
        agent-browser state save "$STATE_FILE"
    fi
else
    # First-time login
    agent-browser open https://app.example.com/login
    # ... login flow ...
fi
```

## Security Best Practices

1. **Never commit state files** - They contain session tokens
   ```bash
   echo "*.auth-state.json" >> .gitignore
   ```

2. **Use environment variables for credentials**
   ```bash
   agent-browser fill @e1 "$APP_USERNAME"
   agent-browser fill @e2 "$APP_PASSWORD"
   ```

3. **Clean up after automation**
   ```bash
   agent-browser cookies clear
   rm -f ./auth-state.json
   ```

4. **Use short-lived sessions for CI/CD**
   ```bash
   # Don't persist state in CI
   agent-browser open https://app.example.com/login
   # ... login and perform actions ...
   agent-browser close  # Session ends, nothing persisted
   ```


## Native input and control ownership

Before saved-password Native input, use `native_ready`: owner authentication and
consumer input policy must be ready for the exact target; viewer readiness alone
is insufficient. Public `ready` and legacy `local_ready` desktop states require
matching lifecycle identity and enabled input policy; Native control remains separate. Owner decisions, delivery and exact browser binding gate input. Managed-task receiving is product-owned: do not ask operators to create per-task enrollment or read vault secrets as a fallback. Approval comes through the Auth dashboard or Slack via Receipts. `browser_control` status/view inspect ordinary managed tasks; explicit return also supports standalone handoffs after exact-process and fresh same-target verification. Takeover/override keep hosted-app requirements. Passive view does not return a human hold. Signed return uses an exact read scope; pending mode changes and peer bindings stay intact. Activity drift is tolerated; mode relaunch retains handoff control. Task-briefing PIDs need live verification. Keep unknown
attempts on their original requests; never replay private use.


`helper.loginRecovery.reconcile({requestId})` observes and reconciles the same original owner attempt without dispatching credentials. `recordFailure` forwards qualified caller observations to Auth completion and independently reads the settlement; it preserves unknown input history and never grants retry permission. `prepareFresh` may ask Auth to prepare a distinct challenge only after a settled business failure, with the same explicit account and receiving scope; Auth still owns current target readiness, permission and any subsequent private use. A running or changed original remains held. No clock is synthesized by this helper.


`verifyLoginAccess` composes the existing independent application readback with original Auth request and completion reads. The intended account and any required organization must match the exact addressed browser binding. Receiving-provider origin and application origin remain distinct. Auth completion is caller-observed bookkeeping, not independent identity proof; receiver settlement is reported separately. Closed source tickets, filled fields and provider sign-in cannot complete the workflow. No verification call writes credentials or completion.


`discoverLoginReadiness` checks supported Auth capabilities and source metadata before manual unlock. A signed-out extension does not imply unavailable private-source recovery. Credential readiness means uniquely selected metadata, never private-use authority. A hidden owner permission view is visibility unavailable, not an absent grant. Trusted host compositions may use `createLoginDiscovery` with an already authorized scoped owner reader; this neither broadens access nor renews an expired request.


`resumeLoginContinuation` extends only an existing private record’s `loginContinuation` namespace, through the existing store lock/load/save interface. Preserve its Native episode, exact caller and browser bindings, requested account/organization and unresolved attempts. Fresh Auth reads and independent verification decide the outcome; a stored success flag never does. Equivalent progress is coalesced across resumes. Notification intent is persisted before the optional callback, and callback acknowledgement is not human delivery; an unknown notification is not blindly resent. An injected trusted Wake reader normalizes its supported receipt: registered/held is not ready, readiness requires the exact armed intent and ready monitor, and cancellation remains a separate cleanup obligation.


The local pre-push gate uses the existing validation selector to run the public login workflow checks for relevant committed changes. It requires clean current source and rejects unknown coverage or failed checks. Enable it for one checkout with `git config --local extensions.worktreeConfig true` and `git config --worktree core.hooksPath .husky`; peer checkout settings and hosted CI remain unchanged. This focused source check does not replace full required validation or grant merge/install authority.


For a Native control observation, pass the addressed browser task ID rather than the Auth caller ID: the progress summary returns a read-only `browser_command` / `task_briefing` action. Missing binding is explicit as `addressed_browser_task_required`. Continuation preserves sanitized incomplete verifier categories, evidence, qualification and supported next reads. Broad executable, dependency and selector changes select the workflow and gate checks as well; the gate is not duplicated inside the tooling aggregate.


Approval transport failure (`auth_parent_unavailable`) is not an authorization denial. Inspect the original Auth request and its `approvalDelivery` cause through the existing authenticated helper. Repair its operator notification connection, then call `receive_credential_approval` with the same `requestId`. Auth retains idempotent notification delivery and the operator decision; valid current permission does not require reapproval because transport failed. Preserve pending and uncertain input without creating a replacement request.
