# Profile Allocation API Checkpoint

Date: 2026-05-03

## Decision

The profile allocation API and dashboard detail lane is complete enough to
pause. It should be treated as a closed backend-first observability slice, not
as an invitation to expand the dashboard further before the next service
authority slice is chosen.

## Roadmap Alignment

This slice follows the service roadmap direction that agent-browser should be
the durable authority for browser state, runtime profiles, session leases,
queued jobs, and operator-visible recommendations.

The immediate predecessor note, `2026-05-03-profile-lease-observability-checkpoint.md`,
identified a profile/session allocation view model as the next useful slice.
That model now exists across the service status, profile collection, one-row
HTTP lookup, generated service observability client, dashboard detail dialog,
contract metadata, docs, and smoke tests.

## What Changed

- `service_status`, `service profiles`, HTTP `GET /api/service/status`, HTTP
  `GET /api/service/profiles`, and MCP `agent-browser://profiles` expose the
  backend-owned `profileAllocations` view.
- HTTP `GET /api/service/profiles/<id>/allocation` returns one derived profile
  allocation row for software clients and dashboard detail views that do not
  need the full collection.
- The generated service observability client exposes
  `getServiceProfileAllocation()`.
- The dashboard Service view renders profile allocation rows from service
  state and refreshes the detail dialog from the one-row HTTP endpoint.
- `GET /api/service/contracts` advertises the HTTP-only
  `serviceProfileAllocationResponse` contract.
- The static service API and MCP parity checker has a separate HTTP-only route
  section so this endpoint is audited without inventing an MCP tool or
  resource.
- README, docs site, contracts docs, CLI help, and the installed
  `agent-browser` skill document the allocation view, one-row lookup, and
  dashboard behavior.

## Evidence

Relevant commits on `main`:

- `1bcb8e7` added the service profile allocation view.
- `94bbbfa` rendered profile allocations in the dashboard.
- `3742010` guarded the dashboard profile allocation contract.
- `80b97ec` added the dashboard profile allocation detail dialog.
- `59922f2` added the profile allocation HTTP lookup and generated client
  helper.
- `e7a08ce` refreshed dashboard profile allocation details from the one-row
  endpoint.
- `e0d4313` added a dashboard smoke for the allocation lookup helper.
- `1906514` added HTTP-only service allocation route coverage to the parity
  checker.
- `373d4e9` advertised the profile allocation response contract through
  service contracts metadata.
- `3e47835` added live HTTP smoke coverage for the one-row allocation detail.
- `8cd6b3f` added no-launch negative-path coverage for unknown profile IDs.

## Validation Now Available

Use targeted local checks for ordinary implementation closeout when touching
this surface:

```bash
pnpm test:service-observability-client
pnpm test:dashboard-profile-allocation
pnpm test:service-contracts-no-launch
pnpm test:service-api-mcp-parity
git diff --check
```

Use the live HTTP smoke for manual full-CI or release-gating checks when a
change touches profile allocation derivation, HTTP collection payloads, or the
one-row endpoint:

```bash
pnpm test:service-profile-http-live
```

The live smoke proves that the one-row allocation endpoint returns the same row
as the full status and profile collection during a real service-owned
runtime-profile browser session. The no-launch smoke proves an unknown profile
ID returns a `404` JSON failure envelope without creating browser state.

## What This Proves

The service can answer the core operator questions from one backend-owned
allocation model:

- which profile is known to the service
- which sessions hold the profile
- whether the lease is shared, exclusive, waiting, conflicted, or available
- which jobs are waiting for the profile
- which sessions conflict with new work
- which service, agent, and task labels are associated with the profile
- which browsers and tabs are linked to the profile
- which operator action is recommended next

HTTP, MCP resources, generated software clients, and the dashboard now consume
that model instead of reconstructing profile state independently.

## Residual Gaps

- The allocation view is derived from retained service state. It is only as
  current as reconciliation and event recording make it.
- The one-row lookup is intentionally HTTP/client-only for now. MCP clients can
  read `agent-browser://profiles` and filter the `profileAllocations` array.
- The dashboard detail dialog is smoke and build validated, but not
  browser-visually inspected in this checkpoint.
- The model identifies profile contention and recommended actions, but it does
  not yet implement richer profile-management operations such as explicit
  lease transfer, profile retirement, profile health probes, or identity
  freshness checks.

## Google Sign-In Constraint

Profile readiness cannot treat every missing target login as something the
service can repair through CDP. Google sign-in is a special case that must be
modeled explicitly.

Live testing in `google-runtime-profile-login.md` and
`2026-04-16-google-runtime-profile-live-test-report.md` showed that Google can
reject sign-in when Chrome is launched with an attached DevTools port. A new
Google profile therefore needs a manual seeding phase:

1. agent-browser launches headed Chrome for the target runtime profile without
   DevTools.
2. the user signs in manually.
3. the user closes Chrome.
4. agent-browser relaunches the same profile with an attachable DevTools port
   for automation.

For the service model, this means a Google target identity can be:

- `needs_manual_seeding`: no source-backed evidence says the profile has been
  manually signed in without DevTools for this target identity
- `seeded_unknown_freshness`: the profile was manually seeded, but the service
  has not recently verified target auth state
- `fresh`: the service recently verified authenticated state for the requested
  target service or login ID
- `stale`: the service has previous auth evidence, but it is outside the
  configured freshness window or a probe failed
- `blocked_by_attached_devtools`: a first-login attempt is being requested
  through an attached CDP browser for a policy that requires detached seeding

The important policy distinction is that `needs_manual_seeding` is not a
browser crash, profile lock, lease conflict, or ordinary stale-cookie problem.
The recommended action should be an operator-facing manual bootstrap: launch
detached `runtime login`, complete Google sign-in, close Chrome, then let the
service resume attachable automation on the same managed profile.

## Best Next Slice

Return to the backend-first roadmap and work on service-owned profile/session
freshness before adding more dashboard panels.

The next useful slice is an identity freshness and profile readiness model:

- record when a profile was last known to satisfy a target service or login ID
- record when a target service or identity requires detached manual seeding
  before first automation, starting with Google sign-in
- expose whether authenticated target state is fresh, stale, unknown, or needs
  operator verification
- expose the recommended manual seeding action when the right profile is not
  yet usable for Google sign-in automation
- add a no-browser-launch status surface for profile readiness
- add a bounded live smoke that marks one target service authenticated, opens a
  service tab by login identity, and verifies the selected profile reason and
  readiness state through HTTP and MCP resources

This keeps future auth, 2FA, challenge handling, and site policy work grounded
in authoritative service state instead of dashboard or client-side inference.
