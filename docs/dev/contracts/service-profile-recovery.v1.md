# Service Profile Recovery Contract v1

Plan 0137 makes profile acquisition the stable client contract. Lifecycle
owner, daemon route, browser, session, process, and presentation records are
typed evidence. They are not interchangeable identities.

The machine-readable contract is
`docs/dev/contracts/service-profile-recovery.v1.schema.json`. Rust types and
zero-effect planning live in `cli/src/native/service_profile_recovery.rs`.

## Public outcome

Every acquisition returns one stable state:

- `acquired` with the exact browser and daemon route;
- `recovery_available` with one sealed Recovery Plan; or
- `blocked` with one Dominant Blocker and retained secondary evidence.

Clients branch on the top-level state. Internal evidence codes may expand
without creating a new client workflow.

## Identity joins

The recovery plan binds the authenticated principal, managed profile and
profile digest, lifecycle owner and generation, durable browser, daemon route,
service session when present, process instance, and presentation route when
present. Shared prefixes or historical string derivation never prove a join.

The durable browser id may differ from `session:<daemonSessionRoute>`. A valid
cooperative transfer changes the daemon route without rewriting durable browser
history.

## Source authority map

<table>
  <thead>
    <tr><th>Responsibility</th><th>Authority</th></tr>
  </thead>
  <tbody>
    <tr><td>Authenticate principal and profile capability</td><td><code>service_principal.rs</code></td></tr>
    <tr><td>Propose a zero-effect recovery plan</td><td><code>service_profile_recovery.rs</code></td></tr>
    <tr><td>Own lifecycle generations and terminal evidence</td><td><code>runtime_owner_transfer.rs</code> and <code>runtime_lifecycle.rs</code></td></tr>
    <tr><td>Persist state and receipts</td><td><code>service_store.rs</code> through <code>ServiceStateRepository</code></td></tr>
    <tr><td>Project acquisition guidance</td><td><code>service_access.rs</code></td></tr>
    <tr><td>Apply an exact recovery</td><td>Plan 0137 Slice B repository-backed recovery authority</td></tr>
    <tr><td>Render operator actions</td><td>CLI, HTTP, MCP, generated client, and dashboard adapters</td></tr>
  </tbody>
</table>

## First mitigation classes

`supersede_terminal_owner` applies only when the exact owner generation is
terminal, cleanup is satisfied, process exit and profile-lock release are
proven, and no pending transfer or foreign authority exists.

`reconcile_exact_principal_profile_identity` is distinct. The Odollo
contractor-portal fixture has retained session evidence without a current
owner binding and currently returns
`existing_session_profile_identity_unproven`. That case must preserve the
observation until authenticated principal and profile evidence prove an exact
repair. It cannot borrow the terminal-owner action.

Planning is always zero-effect. Apply must compare the sealed plan with current
state before any effect, persist a terminal receipt, and retry the original
acquisition intent only after recovery succeeds.
