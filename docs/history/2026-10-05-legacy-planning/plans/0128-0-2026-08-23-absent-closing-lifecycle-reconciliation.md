# Plan 0128 Historical Packet: Absent Closing Lifecycle Reconciliation

Date: 2026-08-23

State: CLOSED

Lane: P128 historical predecessor, originally numbered P127 on its isolated branch

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
8. Provider-free isolated-development fixtures prove normal-close convergence,
   while deterministic lifecycle coverage proves one same-profile,
   next-generation replacement without changing the accepted workstation
   installation or opening a provider.
9. A harmless route-bound local fixture proves the same-profile replacement
   across the presentation handoff before this lane closes.

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
- Merge checkpoint `f11f3bd4` incorporates current `origin/main` at
  `7b235254` and preserves accepted P126 while assigning this lane P127.
- Commit `750b17e8` prevents an operator's installed ingress registry from
  silently opting unit tests into runtime-host admission, while preserving
  installed product behavior. It also aligns the unknown-session dashboard
  test with the intentional removal of arbitrary first-session fallback.
- Canonical Rust CI passes, including 1,412 parallel-safe tests, every isolated
  environment-mutating test module, and the integration partitions. Strict
  Clippy, Rust formatting, diff hygiene, route-confusion gates, the selected
  workstation and Guacamole fixtures, docs build, remote-view handoff docs,
  and installed skill parity also pass.
- The candidate was published only to development generation
  `0.28.0-b1a74a64a0dc` at SHA-256
  `b1a74a64a0dc0a80bb145a7334b741b7376c04b06829f77c72aa2ca955d9f22f`.
  Development doctor passes, and three disposable provider-free `about:blank`
  open/read/close cycles pass with zero remaining sessions, zero active
  incidents, and zero force-kill failure classifications.
- Production remained selected at generation
  `0.28.0-4b975a51aa89-d0782705d5ff` with installed SHA-256
  `4b975a51aa892241ea73cc6e8acef42bb67d781c8b9be43edbc1086f4d7956f8`.
  No production install, provider navigation, profile mutation, or second
  provider tick occurred.
- Source and provider-free development acceptance were complete before the
  final route-bound packet.
- The operator subsequently authorized a production hotfix while unrelated
  feature work remained active. The transactional installer accepted
  generation `0.28.0-a89625b870c3-1e2c09b12ebc` from binary SHA-256
  `a89625b870c3cda3cde9b41f27271ebe36d60683b1235c4196c4be337bb39ea6`
  in transaction `upgrade-7d9a2776-2c7e-458c-8e4c-eb2bbe989c46`.
- The final repair prevents a terminal owner binding for an exited process
  from being refreshed against a newly launched PID. Exact same-process
  refresh remains allowed; a new process must pass the terminal replacement
  compare-and-swap.
- A provider-free route-bound fixture replaced the exact same logical browser
  and profile at owner generation 3, reached operator-visible `ready`, and
  closed to `terminal/satisfied` with `exact_process_exited` and
  `profile_lock_released` evidence. Five readbacks found no browser or session
  record, Route B available, and zero cleanup candidates. This predecessor
  packet is closed and incorporated into Plan 0128.

## Authority Change

The original packet excluded another production installation. The operator's
later explicit hotfix instruction superseded that bound only for the
transactional workstation apply and its provider-free acceptance. It did not
authorize provider navigation, profile reseeding, unrelated browser closure,
or feature-lane integration.

## Bounds

- one source implementation attempt plus one bounded remediation pass;
- one live lifecycle reconciliation attempt;
- one consuming X and LinkedIn tick after restored admission;
- one provider-free normal-close and same-profile replacement acceptance pass;
- no profile mutation, process termination, production runtime installation,
  or provider retry outside those bounds.
