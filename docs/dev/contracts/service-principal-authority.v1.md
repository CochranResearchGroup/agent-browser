# Service Principal Authority v1

This contract defines the internal authority model that lets a registered
service continue using its own managed browser profile across request, task,
session, and process churn. It is the authority foundation for the later
profile lease control plane.

## Authority boundary

`serviceName`, `agentName`, `taskName`, and session labels are attribution
only. They may explain who submitted work, but they cannot establish profile
ownership or authorize a lifecycle effect.

A trusted registration creates a stable `principalId` and grants that
principal a capability for one exact `profileId`. The raw capability is
returned only through the trusted registration path. Service State retains a
domain-separated SHA-256 digest, capability id, revision, state, principal,
and profile. It never retains or projects the raw capability.

Successful capability authentication produces an internal authority record
containing:

- the stable principal id;
- the exact profile id;
- the capability id and current revision; and
- registered-capability provenance.

Caller-supplied request fields cannot construct that record. Public transport
ingestion accepts the capability only through the reviewed ephemeral channel
for that transport: an absolute private capability file for CLI mutations, an
`Authorization: Bearer` header for HTTP, or the `profileCapability` argument
for MCP tools. A caller-supplied `principalId` therefore continues to fail
request validation.

## Owner binding

Principal authority is attached to the existing runtime owner registry. The
binding is keyed by the canonical profile identity digest and includes the
principal id, profile id, capability id, provenance, and owner generation.
Binding uses compare-and-swap semantics against the current ready owner and
its exact generation. A missing binding is legacy observation, not implied
authority.

This is not a second owner registry. Browser, process, route, and generation
authority remain on the existing runtime owner record.

## Subordinate work leases

Sessions and tabs are expiring work leases beneath the stable profile-owning
principal. Each stores principal provenance, a deterministic work lease id,
a monotonic work lease revision, and an expiry. A tab lease must name a
session already bound to the same authenticated principal and profile.

Creating or replacing subordinate work does not transfer profile ownership or
change the runtime owner generation. A stale capability revision cannot bind
new work.

## Continuity recourse

Every continuity decision returns exactly one of these states:

- `rejoin_owned_browser`: current capability and owner binding agree, and the
  retained lane is available to the same principal;
- `replace_stale_same_principal_session`: the same principal has only released
  or expired subordinate sessions and may create replacement work;
- `wait_for_foreign_principal`: a proven different principal owns or holds the
  profile, so the requester cannot mutate it; or
- `reconcile_principal_identity`: capability, owner binding, or legacy holder
  evidence is missing or contradictory, so effects remain unavailable.

Only the first two states are effect-capable for the authenticated principal.
Neither state permits owner transfer, takeover, broad cleanup, browser close,
or profile deletion.

## Legacy migration

Legacy Service State remains readable with empty principal registries and
owner bindings. Existing labels are never promoted into a principal.

Migration planning is conservative:

- an already principal-bound session remains bound;
- exact agreement among active registration, capability, profile, and current
  owner binding yields a staged candidate that still requires an explicit
  commit; and
- missing, multiple, stale, or contradictory evidence remains
  `unproven_principal`, observation-only, with
  `reconcile_principal_identity` recourse.

First-class lease commands, public HTTP and MCP operations, generated-client
helpers, dashboard controls, and staged state migration are implemented.
Clients must feature-detect the advertised contract. Installed-runtime
acceptance remains bound to the exact candidate transaction and final doctor
receipt recorded by Plan 0134.
