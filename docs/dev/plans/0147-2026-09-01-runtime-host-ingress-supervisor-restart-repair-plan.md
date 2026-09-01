# Plan 0147 | Runtime Host Ingress Supervisor Restart Repair

Date: 2026-09-01

State: OPEN

Lane: P147

Branch: `fix/runtime-ingress-same-generation-adoption`

Target: `main`

Source baseline: `c661e64c11245360d52ab8bb07ae8007ff4c0d94`

Implementation checkpoint: `0d00a8505ea05aa156732ad33db76caf57c63aeb`

Plan 0144 reconciliation checkpoint: `45ca9556c08813ac9181ebf695668262698681dd`

Authority: SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, DEVELOPMENT
QUALIFICATION, EXACT PRODUCTION INSTALLATION, AND BOUNDED PRODUCTION RUNTIME
INGRESS RECONCILIATION ARE IN SCOPE. BROWSER, PROFILE, ROUTE, TENANT, AND
PROVIDER MUTATIONS ARE OUT OF SCOPE.

## Incident

The production session supervisor started the selected `0.28.0` binary on a
new user-runtime socket after the prior selected host exited. The new host was
healthy and directly reachable, but `runtime-host-ingress.json` continued to
select the dead host's socket. Ordinary unscoped commands therefore reached
partial retained control metadata and failed with `Runtime host endpoint
metadata is incomplete` before any browser or CDP operation.

The privileged Plan 0144 lease-authority installation is a distinct root-owned
surface at `/run/agent-browser/lease-authority.sock`. It neither caused nor
repaired the stale user-runtime selection.

## Goal

Make a supervised same-generation runtime-host restart atomically adopt its new
socket when the selected PID is positively absent and the binary identity is
unchanged. Preserve fail-closed behavior for a live selected process, an active
upgrade transaction, a binary mismatch, or failed process observation.

## Work Units

1. Preserve the exact production failure and healthy explicit-socket control.
2. Add a focused regression for dead selected-host adoption across socket
   directories.
3. Implement self-reconciliation after the replacement host binds its socket
   and publishes exact process, executable, and socket identity.
4. Validate negative fences for binary mismatch and active transactions.
5. Qualify the exact candidate in the isolated development runtime and
   reconcile the separately prepared Plan 0144 development evidence.
6. Integrate and push the repair, install the exact accepted candidate in
   production, restart only the governed production runtime host, and verify
   ordinary unscoped commands use the selected live socket.

## Acceptance

- The focused regression is red before the repair and green after it.
- A same-generation replacement may advance ingress only when the selected PID
  is missing, the current boot is proven, no candidate transaction is active,
  and the binary SHA-256 matches.
- The replacement records its current PID, host id, socket directory, and
  socket identity; stale fallback and transaction rollback authority are
  cleared.
- Rust formatting, strict Clippy, focused tests, and validation selection pass.
- The development candidate passes its applicable doctor and no-launch runtime
  checks before production installation.
- Production ingress selects the live supervised host, install doctor reports
  the current installed identity, and an unscoped read command no longer fails
  on incomplete endpoint metadata.

## Stop Rules

- Do not overwrite the live ingress registry by hand to satisfy acceptance.
- Do not kill browsers, delete profiles, or clean unrelated runtime processes.
- Do not cross an active runtime-host upgrade transaction or replace a live
  selected host.
- Preserve the other agent's uncommitted Plan 0144 custody and plan changes.
- Stop production installation if source validation or development
  qualification fails.
