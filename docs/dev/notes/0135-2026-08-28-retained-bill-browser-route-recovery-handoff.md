# Retained BILL Browser Route Recovery Handoff

Date: 2026-08-28

Status: OPERATOR HANDOFF READY | LIFECYCLE AND MAINTENANCE DEFECTS OPEN

Scope: Agent Browser profile identity, retained-browser reuse, and remote-headed presentation

Authority: FIELD OBSERVATION AND PROVIDER-FREE PRODUCT FOLLOW-UP ONLY

## Purpose

Record the recovery of one retained BooksReceipts BILL browser after a machine
crash and subsequent service regeneration. The browser is presently reusable
and its durable operator handoff is ready. Reaching that outcome required
manual reconciliation across profile principal authority, lifecycle ownership,
display allocation, route checkout, and remote-view presentation.

This note is an Agent Browser implementation handoff. It does not authorize a
workstation upgrade, replay against BILL, browser closure, profile deletion,
route takeover, provider mutation, or tenant-data inspection.

The tenant account identifier, managed profile identifier, capability secret,
credentials, cookies, page content, raw provider URLs, and durable handoff URL
are intentionally omitted. They remain in user-scoped runtime state.

## Executive Summary

The browser was eventually recovered without taking another workload's route,
deleting the retained profile, or copying private browser data. The successful
path used the existing consumer browser identity and Route B, then established
an operator-visible window on the route's current shared X display.

The difficulty was not one failed subsystem. Several individually reasonable
control-plane projections disagreed:

1. The access plan initially advertised a replacement-capable service request,
   while request admission rejected the same profile as having unproven
   retained-session identity.
2. A legacy profile lease did not prove the current principal until a
   profile-scoped capability was registered outside the repository.
3. Owner-generation changes during close, reconcile, rejoin, and relaunch
   repeatedly invalidated the lease binding that had just been proven.
4. Route and display records retained stale post-crash identity. Route B was
   live on one display while a retained display-allocation record still named
   an earlier display and owner.
5. Browser reattachment reported that presentation recovery capacity was not
   configured even though the route pool itself was available.
6. Remote-view requests inherited an implicit runtime profile unless every
   exact profile and route field was supplied with the principal capability.
7. A normal service reconcile reported broad orphan repair but did not repair
   the exact route-to-display mapping blocking the requested browser.

The interlocks correctly prevented an unproven profile or wrong desktop from
being presented. The product gap is that Agent Browser could not converge the
same evidence through one bounded service-owned transaction.

## Relationship To Existing Notes

This incident extends, rather than replaces, these handoffs:

- `docs/dev/notes/2026-08-26-books-receipts-post-crash-browser-regeneration.md`
  records the original post-crash dependency chain and manual field recovery.
- `docs/dev/notes/0133-2026-08-25-operator-visible-window-focus-gap-handoff.md`
  records the stronger mapped, active, topmost, visible, and non-minimized
  window postcondition.
- `docs/dev/notes/0134-2026-08-26-exclusive-profile-lease-holder-reuse-divergence.md`
  records the general case in which session, browser, lease, and profile
  identities disagree.

The new evidence is that principal authentication, owner-generation repair,
route checkout, and remote-view open can each succeed in isolation while the
next lifecycle transition makes a previously accepted lease binding stale.

## Observed Failure Sequence

### Access plan and request admission diverged

The initial request-scoped access plan reported terminal cleanup satisfied,
replacement eligibility, no active lease, and an available copyable service
request. Executing that request failed with:

```text
existing_session_profile_identity_unproven
```

The plan therefore described a route that request admission would not accept.
The plan and executor must evaluate the same principal, profile, session,
browser, owner-generation, and cleanup evidence.

### Legacy principal authority required out-of-band repair

The retained profile lease was observation-only with:

```text
legacy_principal_unproven
```

A principal-bound profile capability was created in the private Agent Browser
runtime home and registered through the service control plane. No capability
contents or private path belong in this repository. Registration was necessary
before retained-browser reuse could become effect-capable.

### Every lifecycle transition reopened identity reconciliation

Sealed reconciliation advanced one owner generation and allowed the next
action. Rejoin then advanced the binding again. The final route-bound browser
open advanced the owner once more, leaving the prior lease record with both:

```text
owner_generation_or_binding_mismatch
unproven_session_authority
```

At the final readback, the access plan nevertheless found one compatible live
browser, no active lease conflict, a ready lifecycle owner at generation 15,
and exact retained-browser route hints. Install doctor still retained the two
lease warnings above for the same consumer profile. Operational reuse and
maintenance diagnosis therefore do not yet tell one coherent story.

### Presentation recovery selected stale or foreign context

Browser reattachment failed with:

```text
operator_presentation_authority_unavailable:
presentation recovery capacity is not configured
```

A route preflight also evaluated another session's display and returned:

```text
display_allocation_owner_mismatch
```

Route B targeted the current shared display, while one retained display record
still named the prior display and prior owner. Remote-view open correctly
failed closed with:

```text
route_pool_target_mismatch
```

The route pool had usable capacity. The defect was selection and reconciliation
of the requested browser's current route and display authority.

### Capability and profile context were not preserved uniformly

The profile capability authenticated MCP browser acquisition. Remote-view and
takeover requests could still inherit the caller runtime's default profile and
then fail with:

```text
explicit_profile_conflicts_with_current_owner
```

The direct CLI remote-view path could not carry the ephemeral profile
capability and returned the earlier unproven-identity failure. The working MCP
request had to supply the capability together with the exact runtime profile,
user-data directory, logical browser, session, route, display, allocation, and
target.

This is too much reconstruction for a consumer. The access plan should produce
one opaque, principal-bound, copyable presentation request whose executor
preserves every selected identity.

## Successful Recovery Path

The bounded field recovery performed these actions:

1. Registered one private capability for the BooksReceipts principal and the
   existing retained profile.
2. Applied one sealed lifecycle reconciliation against the exact current owner
   generation.
3. Reused or relaunched only the same retained profile; no profile directory
   was copied or replaced.
4. Closed the first hidden-display launch after it failed the operator-visible
   outcome, then launched the same profile on Route B's verified idle shared
   display.
5. Checked out Route B against the exact logical browser, session, route-pool
   entry, display, and display allocation.
6. Rejoined the principal-bound profile lease.
7. Opened remote view through MCP with the ephemeral capability and all exact
   selected profile and presentation fields.

The final remote-view result reported:

- browser health `ready`;
- one retained browser on the intended managed profile;
- Route B and its stable route-pool entry;
- shared display `:10` with the matching shared-display allocation;
- attachability `attached_ready`;
- operator-visible state `ready`;
- browser window mapped, process-bound, active, topmost, non-minimized,
  unoccluded, and on the visible workspace; and
- one durable opaque handoff in `ready` state.

The browser retained multiple restored tabs. They were preserved because no
reviewed duplicate-tab cleanup policy authorized closing them.

## Current Readback

Fresh readback on 2026-08-28 established these separate axes:

### Browser acquisition

- Installed CLI version: `0.28.0`.
- Selected profile build: `stealthcdp_chromium`.
- Profile reuse action: `reuse_existing_browser`.
- Compatible live browser count: `1`.
- Same-profile live browser count: `1`.
- Active profile lease count in the access plan: `0`.
- Lifecycle owner state: `ready`, generation `15`.
- Copyable service request: available with exact browser and session hints.
- Authentication freshness: unknown; the access plan recommends a bounded
  auth probe before relying on authenticated automation.

The login page previously observed is not proof of a current authenticated
session, and no private page content was captured for this note.

### Operator presentation

The last field acceptance produced a ready durable handoff on Route B with the
strong operator-visible window postcondition. That evidence proves the end of
the recovery operation. It is not a crash-replay guarantee.

### Runtime maintenance

The workstation status selects generation
`0.28.0-92d2015dd76c-d017d3f4db8a`, reports admission draining false, and
reports every readiness axis ready. Its latest transaction is retained as
`failed_preserved_old_generation`.

Install doctor separately reports runtime multiplicity steady with no
multiplicity issues, but returns nonzero because it classifies a workstation
transaction as `blocked_ambiguous_runtime` and retains several profile-lease
warnings. For the BooksReceipts profile, the retained warnings are
`owner_generation_or_binding_mismatch` and `unproven_session_authority`.

This maintenance inconsistency is not evidence that the ready retained browser
must be closed. It is a product defect in transaction and lease-state
convergence.

## Product Repair Contract

One bounded repair should make these guarantees:

1. `service_access_plan`, request admission, route checkout, and remote-view
   open consume the same principal-bound identity envelope.
2. The envelope binds profile ID and digest, logical browser, daemon session,
   process identity, owner generation, route, display allocation, and target.
3. An accepted lifecycle transition advances or atomically rebinds the lease;
   it cannot immediately strand the previous lease generation.
4. A retained lease superseded by a proven ready owner becomes terminal or
   historical. Doctor must not continue to present it as current ambiguity.
5. Route preflight is scoped to the requested browser and session. It cannot
   substitute another workload's display.
6. Route and display reconciliation refreshes boot-bound display evidence
   before selection while preserving live foreign ownership.
7. Presentation recovery capacity is derived from current provider and route
   evidence or returns one exact external blocker. Available route capacity
   cannot coexist with an unexplained `not configured` result.
8. The copyable request emitted by access planning preserves profile capability
   authentication without serializing the capability into state, URLs, logs,
   or handoff records.
9. CLI, HTTP, MCP, dashboard, doctor, and workstation status converge on the
   same terminal transaction and lease classification.
10. Recovery returns either one ready retained browser and ready durable
    handoff, or one typed blocker. It never requires the consumer to assemble
    internal route and owner fields manually.

## Required Acceptance

Start with provider-free fixtures on the active crash-profile lifecycle lane:

1. Reproduce access-plan availability followed by executor rejection for the
   same retained identity.
2. Reproduce an accepted reconcile or rejoin whose next owner generation
   immediately strands its principal-bound lease.
3. Reproduce route preflight selecting another session's display.
4. Reproduce an available route pool with presentation reattach reporting
   recovery capacity not configured.
5. Reproduce workstation status ready while install doctor classifies the same
   transaction as nonterminal or ambiguous.
6. Verify one principal-bound access-plan request survives acquisition,
   lifecycle transition, route checkout, and remote-view open without caller
   reconstruction.
7. Verify a foreign active route is preserved and never selected for the
   requested browser.
8. Verify stale route, display, lease, and transaction records become terminal
   or historical after reconciliation.

After source acceptance, use an isolated development runtime and harmless page
to prove same-profile close, relaunch, rejoin, route-bound presentation, and
reconnect. Production BILL replay requires separate operator authority and is
not part of this handoff.

## Hard Stops

- Do not close or replace the currently reusable BooksReceipts browser as a
  diagnostic shortcut.
- Do not take, park, or release another workload's route.
- Do not delete, reset, clone, or check in an authenticated profile.
- Do not weaken route-to-display agreement or principal identity checks.
- Do not put profile capabilities in command arguments, URLs, persisted state,
  logs, fixtures, or repo notes.
- Do not treat authentication freshness as proven from a login URL or retained
  browser existence.
- Do not start another workstation upgrade to repair this lifecycle defect.

## Suggested Skills

- `agent-browser-service` for profile, lease, route, and handoff ownership.
- `diagnosing-bugs` for the provider-free identity and convergence reproducer.
- `codegraph-workspace` for the access-plan to request to lifecycle flow.
- `tdd` for the public CLI, HTTP, MCP, and dashboard contract fixtures.
- `handoff` when refreshing this note after source or installed acceptance.

## Best Next Action

Continue the existing `plan/crash-profile-lifecycle-coherence` lane from its
current checkpoint. Add one provider-free principal-bound transaction fixture
that spans access planning through route-bound remote-view open, then make
lease and transaction terminality converge across access plan, doctor, and
workstation status. Preserve the working production browser and use only the
isolated development runtime for effectful acceptance until source proof is
complete.
