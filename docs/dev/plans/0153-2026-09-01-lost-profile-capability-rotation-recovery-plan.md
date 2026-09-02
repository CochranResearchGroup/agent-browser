# Plan 0153 | Lost Profile Capability Rotation Recovery

Date: 2026-09-01

State: ACCEPTED

Execution state: `installed_acceptance`

Lane: P153

Source baseline: `9951b64038656cd7036544bb1c66bbba64a46fa5`

Branch: `fix/profile-capability-rotation-recovery`

Target: `main`

Authority: PLAN, SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, ISOLATED
DEVELOPMENT-RUNTIME VALIDATION, EXACT PRODUCTION-CANDIDATE INSTALLATION, AND
READ-ONLY PRODUCTION PREFLIGHT ARE IN SCOPE. ROTATING A PRODUCTION CAPABILITY,
CHANGING OWNER-PRIVATE CLIENT CONFIGURATION, ACCESSING PROVIDER CREDENTIALS, OR
RUNNING ANOTHER LAST30DAYS PROVIDER TICK REQUIRE SEPARATE OPERATOR AUTHORITY.

## Incident

The Last30days principal has one active registered capability for
`last30days-facebook`, but its private capability file no longer exists. The
retained lease is idle and observation-only with no current claim or tabs.
Ordinary registration rejects a second active capability for the same
principal and profile, while reconcile and recovery require the missing raw
capability. This creates a credential-loss catch-22 despite healthy runtime and
physically available profile state.

## Goal

Provide an operator-local, fail-closed capability-status and rotation workflow
that never recovers the old secret, never accepts raw capability material on
argv, and cannot rotate beneath active profile work.

## Safety Contract

1. Status exposes only public capability identifiers, revisions, states, and
   typed blockers. It never exposes a digest or raw secret.
2. Rotation requires the exact principal, profile, current capability ID, and
   principal-registry revision.
3. Exactly one active capability must exist for that principal and profile.
4. A canonical profile claim, active session or tab work, or changed authority
   state fails closed before commit.
5. The old capability is revoked and the new capability is registered in one
   Service State mutation. An exact current owner binding may move only from the
   old capability to the new capability for the same principal and profile.
6. The new file is created with exclusive-create and mode `0600`; failure
   removes the staged file. The old raw capability is never read.
7. Rotation remains an operator-local CLI surface. HTTP and MCP do not gain a
   secret-reset operation.

## Execution Graph

| Slice | Work | Exit condition |
| --- | --- | --- |
| A | Add registry compare-and-swap rotation tests and implementation | Old grant is revoked, new grant authenticates, stale revision fails unchanged |
| B | Add CLI status, rotation, active-work fencing, and staged-file cleanup | Lost-file fixture succeeds only while idle; active-work fixture fails closed |
| C | Update help, README, Skill, docs site, roadmap, and runbook | Operator workflow and authority boundary are explicit |
| D | Run focused, format, Clippy, selected, and full Rust validation | All candidate-owned and comprehensive gates pass |
| E | Qualify an isolated development candidate and exact production candidate | Doctor and disposable launch checks pass before installation |
| F | Install the merged candidate without rotating production authority | Installed source, binary, generation, and runtime health agree |

## Acceptance Criteria

- `service leases capability-status` reports the production principal's one
  active grant, exact public CAS inputs, and rotation blockers without secrets.
- `service leases rotate-capability` refuses stale IDs, stale revisions,
  multiple or absent active grants, active claims, and subordinate work.
- A provider-free lost-file fixture rotates successfully, revokes the old
  capability, authenticates the new one, updates only the exact owner binding,
  persists no raw secret, and records a bounded lifecycle event.
- Parser and help surfaces never accept a raw capability argument.
- Comprehensive source validation and isolated development qualification pass.
- The exact merged production candidate installs with one current runtime host,
  zero legacy daemons, and successful doctor.
- Production capability rotation, Last30days private wiring, and provider work
  remain unexecuted until separately authorized.

## Initial Checkpoint

State transition: `ready -> active`.

Progress classification: `blocker_reduction`.

Evidence: live no-launch planning returns `profile_capability_required`; lease
inspection shows principal `last30days`, provenance `registered_capability`,
state `identity_reconciliation_required`, mode `idle`, zero tabs, no active
claim, and one historical capability whose private file is absent. Current
source rejects a second active grant and requires the missing raw capability
for every reconciliation path.

Next action: complete provider-free status and rotation tests, then run the
required Rust and documentation gates.

## Development Qualification Checkpoint

State transition: `source_implementation -> development_qualified`.

Progress classification: `accepted_progress`.

Implementation evidence:

- capability status exposes only public registration metadata and exact
  compare-and-swap inputs;
- rotation requires one exact active grant, the expected registry revision,
  no canonical claim, and no active subordinate session or tab work;
- the old grant is revoked and the replacement registered in one Service State
  mutation, with only an exact same-principal/profile owner binding eligible to
  advance; and
- the replacement secret is exclusive-created at mode `0600`, removed on
  mutation failure, never accepted on argv, and never persisted in Service
  State.

Validation evidence:

- focused registry, parser, lost-file, active-work, cleanup, persistence, and
  owner-binding regressions pass;
- strict workspace Clippy, Rust formatting, diff hygiene, documentation build,
  remote-view documentation, route-confusion, workstation install/provision/VM/
  Guacamole, CDP architecture, CDP screencast, and live tab-streaming gates
  pass;
- the complete Rust driver passes when the main harness is serialized: 1,873
  main tests, the transport crate, integration tests, and every env-mutating
  serial partition; two isolated parallel-only failures were reproduced as
  passing on both candidate and unchanged baseline and did not recur under the
  serialized acceptance run; and
- development generation `0.28.0-b48230cb56b0` passes doctor. Two independent
  development launch-smoke invocations each passed all three launch, URL,
  close, and residue iterations. A third redundant invocation stopped only
  because the production-state isolation guard observed concurrent production
  drift, not because a development browser operation failed.

Production state remains unchanged by this lane. No capability was rotated,
no Last30days private configuration was written, and no provider tick ran.

Next action: merge the exact qualified source, build that merge commit, and run
the workstation installation preflight without overriding unrelated live work.

## Installed Acceptance Checkpoint

State transition: `development_qualified -> installed_acceptance`.

Progress classification: `accepted_progress`.

Release evidence:

- topic commit `c09f3e19` was merged by `c664c25b` and pushed to `origin/main`;
- the exact merged optimized binary SHA-256 is
  `e2244cd2447ce0de6239d41b7fbec7e77aad9145e57ca86cd2ad2de7bf3c7d94`;
- workstation transaction
  `upgrade-c65674ff-8d5f-437c-a5e8-d46a7efed92c` installed generation
  `0.28.0-e2244cd2447c-c25a91eb0d2b`, with installed, selected, and source
  binary identity converged;
- the installation consumed an authenticated ready presentation receipt for
  opaque handoff `r580584`; the proof resolved without page navigation,
  relaunch, provider response access, or credential access; and
- production doctor succeeds with one current runtime host, one executable
  generation, zero legacy daemons, and all seven readiness axes accepted.

Operational readback:

- installed `capability-status` reports one active `last30days` /
  `last30days-facebook` grant, `rotationAllowed=true`, and no rotation
  blockers;
- the user-scoped Agent Browser skill is synchronized to the accepted source;
  and
- no production capability was rotated, no Last30days private configuration
  was changed, no credentials were accessed, and no provider tick ran.

P153 is accepted as an installed recovery capability. Executing the production
rotation, wiring the newly created private capability into Last30days, and
running a provider acceptance tick remain separate operator-authorized actions.
