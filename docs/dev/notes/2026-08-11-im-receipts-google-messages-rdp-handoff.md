# Im Receipts Google Messages RDP Handoff

Date: 2026-08-11

## Current boundary

The installed agent-browser `0.28.0` workstation can operate the Google
Messages stock-Chrome profile through Guacamole/RDP without interactive sudo.
The root-owned privileged helper reports ready and is callable through its
narrow sudoers rule. Interactive sudo is a bootstrap concern, not a normal
runtime requirement.

The im-receipts integration uses:

- daemon session `im-receipts-google-messages-stock-v4`
- runtime profile `im-receipts-google-messages-stock-v4`
- a user-level boot-persistent stream initializer on loopback port `39716`
- stock Chrome executable `/opt/google/chrome/chrome`
- remote-headed browser host, RDP gateway view stream, manual attached desktop
  control, and shared display isolation

The user-level controller unit is owned by the im-receipts installation for
now. Authentication data remains outside both repositories.

## Validated evidence

- `agent-browser install doctor --json` reported the privileged helper ready
  with `requiresInteractiveSudo=false`.
- The RDP gateway readiness probe reported guacd, xrdp, xrdp-sesman, the HTML5
  client, and private display allocation ready.
- Browser capability preflight applied binding
  `default-stock-chrome-wsl-native`, executable ID
  `stock-chrome-wsl-stable`, and `/opt/google/chrome/chrome` for the v4 profile.
- The account-scoped access plan selected the v4 profile and allowed managed
  CDP after the operator's earlier CDP-free profile seeding.
- The installed controller started converged daemon PID `44937` on fixed port
  `39716`.
- Remote-view job
  `http-service-request-remote_view_open-18d90d62-d761-4459-b582-60495a118567`
  succeeded on route `guacamole:1`, pool entry `guacamole-rdp-a`, and display
  `:10`; the emitted integration event reported
  `operatorVisibleState=ready`.
- Chrome PID `58467` reached a `/web/conversations/...` target titled
  `Google Messages for web: Conversations`.

## Remaining agent-browser TODOs

1. Add an installed, first-class way to supervise a named daemon session on a
   fixed loopback stream port. Consumers should not need to author a user unit
   to obtain a durable service endpoint. The current oneshot unit initializes
   the daemon at login but cannot independently detect and restart a later
   daemon crash.
2. Make `close --all` reject or explicitly confirm its combination with
   `--session`. On 2026-08-11 that combination closed five daemon sessions even
   though the operator intended to close only the named Google Messages
   session. Profile data was retained, but unrelated retained sessions were
   disrupted.
3. Make scoped remote-view diagnostics distinguish a healthy requested route
   from unrelated stale daemon sessions. The current global doctor can fail the
   requested route because a different session is stale.
4. Reconcile the privileged-helper verification probe with the installed
   helper command set. The helper is ready, but the doctor's internal
   `verify-install` probe is not supported by the installed helper.
5. Decide whether the recurring runtime interlock timer should be enabled by
   workstation reconciliation. It was installed but inactive during this
   acceptance pass.
6. Preserve opaque `/remote-view/<handoff-id>` links as the durable operator
   handoff contract. Do not promote raw Guacamole provider URLs.

## Operator incident note

The accidental global close removed the agent-browser daemon records for
`dashboard-service-backend`, `default`, an unrelated Facebook session, the
Google Messages session, and an Odollo carrier session. The system dashboard
process remained active. A detached stealth Chromium process also remained
alive and was not killed or relaunched because its ownership was outside this
slice. Future recovery must inspect ownership before acting on that process.

## Suggested skills for continuation

- `agent-browser`
- `graphiti-discovery`
- `handoff`

## Hard stops

- Do not reuse the v4 profile with Chromium or stealth Chromium.
- Do not claim successful operator handoff unless
  `operatorVisible.state=ready` and an opaque handoff URL is returned.
- Do not infer authentication from route readiness, PID liveness, or CDP
  reachability alone.
- Do not close or recreate unrelated retained sessions during scoped recovery.
