# Plan 0145 | Emergency Profile Lease Fail-Open

Date: 2026-08-31

State: COMPLETE

Execution state: `unsafe_claim_any_validated_development_runtime`

Lane: P145

Source baseline: `a127c872914cc06530d9017b566d156840f1919d`

Branch: `fix/emergency-profile-lease-fail-open-20260831`

Target: `main`

Integration model: one isolated maintenance hotfix with explicit reconciliation
against the concurrent lease-authority rewrite.

Authority: SOURCE, TESTS, DOCUMENTATION, AND ISOLATED DEVELOPMENT-RUNTIME
VALIDATION ARE IN SCOPE. Production installation is out of scope until the exact
validated commit and binary are reviewed against the active lease rewrite.

## Goal

Provide explicit emergency modes that keep ordinary service clients able to
acquire a browser while the profile lease system is under repair. The existing
safe fallback routes conflicts to an isolated profile. A second, loudly unsafe
mode lets an attributable service request select and take an exact session and
profile despite profile lease, principal, capability, or owner-generation claim
conflicts.

## Acceptance Criteria

1. `AGENT_BROWSER_PROFILE_LEASE_MODE=fail_open_ephemeral` leaves conflict-free
   acquisition unchanged.
2. A duplicate live profile lane or exclusive profile lease conflict rewrites
   the request to a deterministic isolated runtime profile and admits it.
3. The rewritten request does not retain the original raw profile path or
   profile identity, and its lifecycle record is nonpersistent
   `managed_one_time`.
4. Without the environment value, existing reject and wait behavior is
   unchanged.
5. Upgrade admission, owner-generation fencing, profile capability authority,
   and viewer/controller leases are unchanged.
6. CLI help, README, Agent Browser skill guidance, docs site, and inline source
   documentation describe the emergency-only semantics and authentication loss.
7. Focused Rust tests, formatting, clippy, contract checks, and isolated
   development-runtime browser smoke pass.
8. `AGENT_BROWSER_PROFILE_LEASE_MODE=unsafe_claim_any` accepts an explicitly
   named session/profile route without principal continuity approval and admits
   that route despite duplicate or exclusive profile lease conflicts.
9. The unsafe override is recorded on the normalized command and scheduler
   response metadata. Dashboard authentication, action policy, confirmation, controller
   and viewer authority, and workstation upgrade admission remain unchanged.

## Execution Sequence

1. Add one focused red test at the profile-lease admission seam.
2. Add the minimal request-rewrite implementation and make that test green.
3. Add coverage for exclusive conflicts, unchanged conflict-free requests, and
   disabled-mode behavior one vertical slice at a time.
4. Update every user-facing environment-variable surface.
5. Run focused and changed-surface validation, then publish and smoke only the
   isolated development runtime.
6. Commit a reviewable hotfix checkpoint and reconcile overlap before any merge
   or production installation.

## Hard Stops

- Do not disable workstation upgrade admission drain or transactional fencing.
- Outside the exact `unsafe_claim_any` mode, do not bypass profile capability,
  principal, or owner-generation authority. Never bypass controller or viewer
  authority.
- Do not launch two Chrome processes against the same user-data directory.
- Do not claim the fallback preserves authenticated state; it intentionally uses
  an isolated profile.
- Do not modify, rebase, clean, or absorb the active lease rewrite worktree.
- Do not install this branch into production without an explicit production
  installation decision after validation and overlap reconciliation.

## Validation Record

- Focused emergency-mode tests: 4 passed, including duplicate-lane rewrite,
  exclusive-conflict rewrite, conflict-free pass-through, and response metadata.
- Disabled-mode regression: 1 passed; ordinary rejection remains unchanged.
- `cargo fmt --check`: passed through `scripts/ci/cargo-safe.sh`.
- `cargo clippy -- -D warnings`: passed through `scripts/ci/cargo-safe.sh`.
- Service fixed-input producer/generated-client harness: passed.
- Service-client managed-profile and example flows: passed.
- Documentation build and remote-view handoff documentation checks: passed.
- Workstation install fixture passed against the branch binary; host provision,
  fresh-VM, Guacamole assets, PostgreSQL durability, and route-user sync checks
  also passed.
- Development runtime generation `0.28.0-c98119b8c2ec` installed and selected;
  `development-runtime doctor` passed with all three development units active.
- One-iteration development browser launch/get/close smoke passed and reported
  `productionUnchanged: true`.
- The development runtime host and dashboard backend have reversible systemd
  drop-ins setting `AGENT_BROWSER_PROFILE_LEASE_MODE=fail_open_ephemeral`.
- An authenticated development service request reached the backend but stopped
  before lease admission with HTTP 503 because no service session was
  registered after the development-unit restart. This does not invalidate the
  focused scheduler coverage, but it is not claimed as live conflict proof.
- The broader legacy `service_profile_lease_gate` filter is not fully green: 4
  tests passed and 2 unchanged tests failed with
  `existing_session_profile_identity_unproven` and a service-control admission
  assertion, including under a clean temporary HOME. P145 does not modify the
  failing control-action list or existing-session identity proof path.
- Unsafe claim-any focused coverage passed: explicit foreign-session route
  adoption, exclusive profile-conflict admission, dashboard relay-session
  recovery for a non-capability client, and scheduler response annotation.
- Existing safe fail-open coverage (3 tests) and disabled-mode regression (1
  test) remained green after the unsafe mode was added.
- `cargo fmt --check`, `cargo clippy -- -D warnings`, `git diff --check`, the
  documentation production build, and remote-view documentation checks passed.
- The tenant-neutral checker named by repo policy is absent from this checkout;
  the changed active surfaces contain no tenant-specific defaults.
- Development generation `0.28.0-5be1285b1898` is installed. The runtime-host
  and dashboard-backend unit environments both read
  `AGENT_BROWSER_PROFILE_LEASE_MODE=unsafe_claim_any`; production is unchanged.
- Unauthenticated dashboard service access returned HTTP 401. Authenticated
  dashboard login returned HTTP 200, and an explicit named session/profile
  request returned HTTP 200 with the requested browser session. Both live smoke
  sessions were closed; the final session census contains only `runtime-host`.
- The development doctor still reports only the pre-existing absent
  `development-default` lane port. The unsafe dashboard path no longer depends
  on that lane: it recovers and proxies directly to the client-named session.
