# Plan 0126: Absent Closing Lifecycle Reconciliation

Date: 2026-08-23

State: OPEN

Lane: P126

Branch: `fix/reconcile-absent-closing-lifecycle`

Source baseline: `05a0c5f3e0470b23b7ba644631dea613601c574f`

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
6. The consuming X and LinkedIn feed workflow can pass launch admission and
   run one fresh tick against the existing selected profile.

## Non-Goals

- deleting, replacing, reseeding, or reauthenticating any browser profile;
- terminating a live or ambiguous process tree;
- weakening ordinary owner-generation or launch-admission checks;
- changing provider scraping or content-quality policy;
- formal release publication.

## Work Units

1. Add the exact absent-process and absent-lock reconciliation transition plus
   fail-closed tests.
2. Validate, checkpoint, and run one bounded live reconciliation through the
   candidate binary.
3. Re-read installed lifecycle and access-plan state, then run one X and
   LinkedIn feed tick only if admission is restored.
4. Record source, live, and consuming-workflow evidence and close or block the
   lane.

## Bounds

- one source implementation attempt plus one bounded remediation pass;
- one live lifecycle reconciliation attempt;
- one consuming X and LinkedIn tick after restored admission;
- no profile mutation, process termination, runtime installation, or provider
  retry outside those bounds.
