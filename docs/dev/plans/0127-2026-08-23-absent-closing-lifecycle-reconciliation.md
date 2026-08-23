# Plan 0127: Absent Closing Lifecycle Reconciliation

Date: 2026-08-23

State: OPEN

Lane: P127

Branch: `fix/reconcile-absent-closing-lifecycle`

Source baseline: `e3945810b3e15c507c00dd0218656735f266fcc0`

## Goal

Restore automatic runtime lifecycle convergence when a managed browser process
has already exited after close began, while preserving the selected named
profile and every live or ambiguous runtime lane.

## Incident

An accepted installed upgrade left one exact lifecycle record at
`closing/owned` after its recorded process group had exited. Service
reconciliation and garbage collection both preserved the record, so launch
admission correctly rejected a replacement browser with
`runtime_lifecycle_existing_owner_requires_explicit_transition` even though no
live browser, service browser record, or service session remained.

The first consuming tick then exposed a second lifecycle defect. Reconciliation
had correctly produced `terminal/satisfied`, but terminal replacement required
the incoming service lane to reuse the prior logical browser ID. The existing
owner was `session:plan0117-final-runtime` while the bounded feed service uses
`session:last30days-home-feed`, so both providers failed before navigation with
`runtime_lifecycle_terminal_replacement_rejected`.

## Acceptance Criteria

1. Reconciliation completes only an exact `closing/owned` lifecycle whose
   owner browser and generation match, recorded process group is absent,
   registered profile path hashes to the lifecycle profile identity, and
   `SingletonLock` is absent.
2. Live process groups, present profile locks, missing profiles, identity
   mismatches, and generation mismatches remain unchanged.
3. Completion uses the existing `CompleteClose` compare-and-swap transition
   and records deterministic terminal evidence.
4. Deterministic tests, formatting, strict Clippy, and the focused Service
   Health test surface pass.
5. A live bounded reconciliation completes the observed stale lane without
   launching, killing, replacing, or modifying the authenticated profile.
6. An exact `terminal/satisfied` owner can be replaced under a new logical
   browser ID only at the next owner generation, with no pending transfer,
   duplicate profile lifecycle, or logical-ID collision. The old lifecycle key
   is removed, the record moves to the new ID, and one cleanup obligation is
   retained.
7. A graceful exact-process exit with successful auxiliary-process cleanup
   records the force-kill attempt as successful and does not create a false
   `force_kill_failed` classification.
8. Provider-free isolated-development fixtures prove normal-close convergence
   and one same-profile, next-generation replacement without changing the
   accepted workstation installation or opening a provider.

## Non-Goals

- deleting, replacing, reseeding, or reauthenticating any browser profile;
- terminating a live or ambiguous process tree;
- weakening ordinary owner-generation or launch-admission checks;
- changing provider scraping or content-quality policy;
- navigating to X, LinkedIn, or another authenticated provider during this
  provider-free packet;
- running another workstation upgrade;
- formal release publication.

## Work Units

1. Add the exact absent-process and absent-lock reconciliation transition plus
   fail-closed tests.
2. Validate, checkpoint, and run one bounded live reconciliation through the
   candidate binary.
3. Re-read installed lifecycle and access-plan state, then run the one already
   authorized X and LinkedIn feed tick only if admission is restored.
4. Reproduce the graceful-close and force-kill contradiction in a
   provider-free fixture, repair the classification, and validate the wider
   shutdown surface.
5. Prove provider-free normal-close convergence and same-profile replacement
   in the isolated development runtime without changing production.
6. Record source and provider-free acceptance evidence and close or block the
   lane.

## Execution Checkpoint

- Commits `71e57e1b` and `ffd5aae6` implement exact absent-process/absent-lock
  convergence and persist the reconciled registry transition.
- The isolated candidate reconciled the selected profile digest
  `0921530e77a78f65acb295bfcaadc9200fb5b4b22e958fc6152a00f0fce2ca59`
  from `closing/owned` to `terminal/satisfied` with evidence
  `service_reconcile_process_group_absent:27742` and
  `service_reconcile_profile_lock_absent`. The owner and authenticated profile
  were preserved.
- Access planning then returned `launch_new_browser` with no manual action or
  owner conflict.
- The one authorized consuming tick
  `tick-7224876f30d729e41ff5435b387be4df` completed degraded. X job `r923698`
  and LinkedIn job `r841495` each launched and politely closed one browser, but
  both failed `remote_view_open` with
  `runtime_lifecycle_terminal_replacement_rejected` before provider navigation.
  Both process identities exited, profile locks were released, and route and
  display leases rolled back.
- The bounded remediation now permits an exact next-generation terminal
  replacement to move to a collision-free new logical browser ID and
  recomputes the package launch identity for that generation. The 12 lifecycle
  tests, 50 service-health tests, strict Clippy, formatting, and diff hygiene
  pass.
- Commit `e517beea` repairs the graceful-exit aggregation bug: successful
  auxiliary-process cleanup now records a successful force-kill outcome instead
  of becoming `browser_shutdown_force_kill_failed`. Its focused regression
  passes.
- Provider-free acceptance remains open. No second provider tick was run, and
  the failed prior tick provides no evidence about X or LinkedIn authentication,
  retrieval, or filtering.

## Bounds

- one source implementation attempt plus one bounded remediation pass;
- one live lifecycle reconciliation attempt;
- one consuming X and LinkedIn tick after restored admission;
- one provider-free normal-close and same-profile replacement acceptance pass;
- no profile mutation, process termination, runtime installation, or provider
  retry outside those bounds.
