# P117 Slice F Source Acceptance

Date: 2026-08-20

Plan: `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`

## Accepted Boundary

Slice F is accepted at the source, isolated workstation-fixture, and
disposable-browser boundary. The candidate was not installed into the live
workstation, live user units were not replaced, and authenticated profiles
were not acquired. Controlled installed convergence remains Slice I.

## Transaction-Bounded Host Convergence

The P116 transaction now carries the complete runtime-lane census, old and
candidate runtime-host identities, lane observations, owner-generation fences,
commit receipts, rollback receipts, and exact old-host exit evidence. Stable
runtime-host ingress stages the candidate, commits only the matching
transaction and revision, and restores the prior host or legacy topology on
rollback. The dashboard ingress and current-generation selector remain behind
their existing presentation and executable-evidence gates.

Candidate crash before commit preserves the old host. Failure after commit
reverses lane ownership and restores ingress. Final promotion commits ingress,
retires the old host exactly, and clears the bounded convergence record. The
transaction fixture enforces the two-host ceiling and rejects unbound host or
generation changes.

## Browser-Bearing Proof

The disposable hot-handoff smoke launches two independent managed Chrome
profiles as two logical lanes in one old runtime host. It transfers both
durable runtime-handoff descriptors to one candidate host, commits both owner
generations, proves both old lanes are effect-fenced, and verifies both pages
and browser PIDs remain unchanged.

The smoke then reverses both transfers, proves the empty candidate host exits,
changes both pages through the restored old owner, and retries the upgrade. On
the second commit, finalizing the first old lane leaves the second lane alive;
finalizing the second proves exact old-host exit. The candidate remains one
host serving both lanes until both are explicitly closed.

The fixture uses native Linux Chrome. The configured Windows Chrome build is
observed as the WSL `/init` interop process and therefore cannot satisfy the
managed-profile process-identity contract. The launcher now retains all retry
errors and a redacted identity observation so that this fail-closed condition
is diagnosable instead of being hidden by later profile-lock failures.

## Defects Closed During the Gate

- The bootstrap lane now records whether configuration is committed. The first
  authenticated lane configuration replaces an unconfigured worker exactly
  once; later CLI defaults cannot reconfigure a live lane.
- Custom profile paths are carried through the authenticated lane envelope and
  injected into the lane launch command instead of depending on host-global
  environment state.
- A committed candidate refreshes lifecycle evidence through its existing
  owner claim. It does not attempt to register a second owner.
- Managed CDP reattachment treats the selected profile path as identity
  metadata rather than an incompatible local-launch argument.
- A final-lane rollback, finalize, or close retires an empty runtime host while
  lane-scoped close continues to preserve unrelated lanes.
- Navigation cancellation issues both a stop and replacement-navigation fence,
  preventing a released stale response from overwriting the next command.

## Validation

- `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`
- `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`
- focused runtime-lifecycle tests: 9 passed
- focused runtime-host and host-ingress tests: 14 passed
- runtime-host convergence no-launch smoke: passed
- runtime-host multi-lane real-Chrome stress smoke: passed
- runtime-host two-lane hot-handoff real-Chrome smoke: passed
- route-confusion no-launch gates: passed
- patch whitespace check: passed

Four exact disposable diagnostic directories were verified to have no live
matching processes and moved to trash. No default runtime socket, authenticated
browser, or live profile was targeted.

## Accepted Commits

- `5fbdfabb` carries the lane set through the P116 census.
- `d5b03a0a` records bounded runtime-host convergence.
- `8e2b0424` fences lane transfer receipts by transaction and owner generation.
- `9c16d698` converges candidate handoffs through one host.
- `e8d1ea0f` stages and commits runtime-host ingress transactionally.
- `b51a7022` fences cancelled page navigation.
- `ffb6cf0a` exercises no-launch host convergence and rollback.
- `bb8510ac` validates browser-bearing two-host upgrades and closes the defects
  exposed by that gate.

## Next Boundary

Execute Slice G by routing the enabled pressure and convergence timer through
the same reconciler and retention authority, surfacing typed repeated-failure
incidents, and deleting the remaining per-session compatibility paths.
