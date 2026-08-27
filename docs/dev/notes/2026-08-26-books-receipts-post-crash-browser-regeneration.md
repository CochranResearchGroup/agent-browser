# Books Receipts Post-Crash Browser Regeneration

Date: 2026-08-26

Status: FIELD RECOVERED | PRODUCT RECOVERY GAP OPEN

Scope: Agent Browser runtime lifecycle and operator presentation

Authority: PRODUCTION FIELD RECOVERY COMPLETE | NO REPLAY OR UPGRADE AUTHORITY

Source baseline inspected: `main` at
`2d0e9106cb1a9b6aa12b6a920d69dcce9f8acb12`

## Purpose

Record a production field incident in which a machine crash and full runtime
restart left an authenticated, remote-headed browser profile unable to produce
a usable operator handoff. The field recovery restored the browser without
deleting its profile or changing private site data. Agent Browser still needs
a crash-consistent regeneration path that can produce the same result without
manual systemd, Docker, XRDP, or route repair.

The live consumer was BooksReceipts. Tenant identifiers, credentials, private
page content, the authenticated profile name, and provider URLs are
intentionally omitted.

## Executive Summary

The crash was the initiating event. Agent Browser's failure to recover was a
lifecycle reconciliation defect across several independently retained and
recreated resources.

The browser regeneration request crossed all of these boundaries:

1. selected immutable Agent Browser generation;
2. user systemd runtime-host unit;
3. selected runtime-host ingress socket;
4. named supervisor lane;
5. Guacamole web service;
6. XRDP session and X display assignment;
7. retained route-pool and display-allocation state;
8. Chrome process and operator-visible window state; and
9. durable remote-view handoff.

Durable records survived the crash, but the associated process IDs, sockets,
container mounts, XRDP sessions, X display numbers, and browser windows did
not retain the same runtime identities. No single recovery transaction
invalidated the stale identities and rebuilt the stack in dependency order.

## Initial Symptoms

The operator-facing dashboard reported that the browser used a remote-headed
RDP posture, but it could not embed the stream. The typed failure was:

```text
route_pool_target_mismatch: route pool entry 'guacamole-rdp-b' does not target display allocation 'display:private_virtual_display:<redacted-session>'
```

The dashboard marked the stream as a manual attached desktop and made the
viewport view-only. Its external popout path also resolved to an internal
loopback address instead of the authenticated opaque handoff contract.

During the broader recovery attempt, the browser window was also minimized or
displaced from the operator-visible desktop. The separate operator-visible
postcondition gap is documented in
`docs/dev/notes/0133-2026-08-25-operator-visible-window-focus-gap-handoff.md`.

## Observed Failure Chain

### 1. Installed generation and user unit diverged

The user runtime-host unit still referenced an immutable generation that was
no longer installed. The accepted selected generation existed, but the booted
unit did not follow it. Restarting the unit could not restore the runtime host
until its executable path was reconciled.

### 2. Runtime-host ingress and socket publication diverged

After the executable was corrected, the runtime host initially published into
the default user socket location while the controller's selected ingress
registry pointed at a generation-specific socket directory. A live process
therefore did not prove that normal clients could reach the selected host.

The current source exposes the selected socket through
`cli/src/runtime_host_ingress.rs::selected_socket_dir`. The shared supervisor
unit writer in `cli/src/session_supervisor.rs::write_manifest_and_unit` renders
the unit from a lane manifest's executable path. During field recovery, adding
a temporary lane rewrote that shared unit and removed the explicit selected
socket environment override. The override had to be restored before final
acceptance.

### 3. Guacamole did not return as a complete provider

The Guacamole web container was stopped with a stale Docker Desktop or WSL
bind-mount handle after the crash. PostgreSQL and guacd remained available.
Agent Browser correctly classified presentation as unavailable, but its
consumer-facing regeneration path did not own or invoke a bounded repair for
the stateless web tier.

### 4. XRDP assigned a new display identity

The recovered Route B XRDP session appeared on display `:10`, while retained
route and allocation evidence referred to pre-crash or private display
identity. The route mismatch interlock correctly refused to attach a browser
to a desktop it could not prove was the requested route.

The safety refusal was correct. The missing behavior was boot-aware
invalidation and rediscovery before route selection.

### 5. Browser acquisition and operator presentation were not one outcome

A browser could launch on a private display without becoming usable through
the selected shared RDP route. The successful recovery required a route-bound
launch on the XRDP display followed by process-bound checks that the exact
Chrome window was mapped, active, topmost, non-minimized, on the visible
workspace, and inside the authorized geometry.

## Field Repair

The field repair performed these bounded actions:

1. Repointed the user runtime-host unit to selected generation
   `0.28.0-32e8c9318beb-b2bd0fba532f`.
2. Set the runtime host's socket directory to the socket selected by the
   runtime-host ingress registry, then reloaded and restarted the user unit.
3. Registered one supervisor lane for the retained consumer profile on its
   fixed loopback stream port.
4. Recreated only the stateless Guacamole web container. PostgreSQL, guacd,
   Guacamole data, and unrelated containers were preserved.
5. Opened one temporary Route B viewer to establish the XRDP session and
   discover its assigned X display.
6. Used the installed privileged helper to grant the route user access to
   display `:10`.
7. Opened the retained browser profile directly on Route B and display `:10`.
8. Required process-bound operator-presentation evidence before publishing the
   handoff.
9. Removed the temporary viewer lane after the durable handoff became ready.

The recovery did not upgrade Agent Browser, delete or replace the authenticated
profile, recreate the Guacamole database, capture private page content, or
modify tenant data.

## Final Acceptance Evidence

The final live readback reported:

- browser acquisition health `ready`;
- shared-display isolation on display `:10`;
- route `guacamole:2` through stable pool entry `guacamole-rdp-b`;
- attachability `attached_ready`;
- route proof and route state `ready`;
- exact browser window mapped, active, topmost, non-minimized, and visible on
  the operator workspace;
- named supervisor state `ready` with no issues;
- durable opaque remote-view handoff state `ready`; and
- install doctor runtime multiplicity `steady_current`, with one dashboard,
  one executable generation, one runtime host, zero legacy daemons, and no
  reported issues.

These values prove the recovered outcome at the end of the field operation.
They do not prove that a subsequent crash or boot will reproduce the recovery
automatically.

## Relationship to Prior Repairs

Plan 0081 and
`docs/dev/notes/2026-07-28-guacamole-route-pool-state-reconciliation.md`
already repaired a post-reboot defect in which readiness discovered current
Guacamole routes but retained stable entries continued to reference legacy
routes.

That repair refreshes stale inactive route definitions and preserves active
conflicts. This incident extends the failure boundary:

- route reconciliation could not run while the runtime host and Guacamole web
  tier were unavailable;
- a retained allocation could appear authoritative after its boot-bound
  display ceased to exist;
- the active-conflict safety rule had no boot-epoch evidence with which to
  distinguish a live protected allocation from stale post-crash ownership;
- supervisor-unit regeneration could remove the selected socket binding; and
- successful browser launch did not by itself establish route-bound operator
  presentation.

The incident is in the same post-reboot reconciliation family as Plan 0081,
but it is not evidence that weakening Plan 0081's active-conflict protection
would be safe.

## Causal Diagnosis

The root product gap is the absence of a boot-aware, dependency-ordered,
idempotent regeneration transaction.

Agent Browser currently retains durable service, route, allocation, and
handoff records while several referenced resources are ephemeral. The records
do not carry enough machine boot identity to invalidate process, socket,
display, viewer, and container observations after a crash. Each subsystem can
therefore be locally reasonable while the complete browser outcome remains
impossible.

The fail-closed `route_pool_target_mismatch` result prevented presentation of
the wrong desktop. That behavior should remain. Recovery should re-observe and
reconcile the inputs before selection, then return a ready handoff or one typed
blocker that identifies the remaining external dependency.

## Required Product Follow-Up

1. Bind ephemeral ownership evidence to the machine boot ID or an equivalent
   boot epoch. Invalidate stale PIDs, process start tokens, socket identities,
   XRDP sessions, X displays, and viewer leases when the epoch changes.
2. Generate the runtime-host unit from the accepted selected generation and
   selected ingress registry as one atomic configuration. Installing or
   removing a supervisor lane must not remove the selected socket binding.
3. Add one idempotent post-crash recovery transaction that restores the
   runtime host, admits retained lanes, checks provider readiness, discovers
   current route displays, reconciles retained allocations, and reattaches or
   relaunches the retained browser only when its process is actually absent.
4. Bind RDP route identity primarily to stable route, connection, and route
   user evidence. Treat the XRDP-assigned display number as rediscovered
   runtime evidence rather than a durable identity.
5. Add a bounded provider-recovery contract. If Agent Browser does not own
   Docker effects, return a single exact remedy for the stateless Guacamole web
   tier without suggesting database recreation or broad container cleanup.
6. Make remote-headed regeneration establish both browser acquisition and the
   stronger operator-visible window postcondition before it reports success.
7. Ensure every external operator link uses the authenticated opaque
   `/remote-view/<handoff-id>` contract. Never expose a loopback or raw
   Guacamole URL as the external popout target.
8. Add crash-recovery acceptance that simulates stale generation paths,
   selected-socket drift, dead provider web tier, new XRDP display assignment,
   stale active allocation, and a displaced browser window. One regeneration
   request must either restore the same profile and handoff or return one typed
   external blocker without duplicating or deleting browser state.

## Evidence Limits

- The machine crash removed or truncated some pre-repair journal history.
- This note records direct command results observed during the bounded field
  recovery and the final live readbacks.
- The exact initiating crash cause is outside this note's scope.
- The precise sequence that displaced the earlier browser window remains
  unproven and belongs to Plan 0133's evidence boundary.
- No provider-free fixture or development-runtime crash replay has yet proved
  the proposed product repair.

## Hard Stops

- Do not weaken route and display agreement to make regeneration appear
  successful.
- Do not delete or replace an authenticated profile as a crash-recovery
  shortcut.
- Do not recreate Guacamole PostgreSQL or run broad Docker cleanup when only
  the stateless web tier is unhealthy.
- Do not initiate another production workstation upgrade as a recovery step.
- Do not publish tenant identifiers, credentials, private page content, raw
  provider URLs, or machine-local handoff URLs in fixtures or repo evidence.
- Do not modify the installed production runtime while developing the
  provider-free reproducer.

## Best Next Action

Reconcile this incident with Plans 0131 and 0133 before opening a successor
lane. Start with provider-free fixtures for boot-epoch invalidation and shared
runtime-host unit rendering. Then add an isolated development-provider crash
replay that proves one regeneration request restores a route-bound,
operator-visible browser without profile replacement or unrelated cleanup.
