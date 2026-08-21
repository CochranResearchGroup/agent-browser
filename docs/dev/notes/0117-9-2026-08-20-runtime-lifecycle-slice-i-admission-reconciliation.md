# Plan 0117 Slice I Admission Reconciliation

Date: 2026-08-20

## Outcome

The Plan 0117 candidate is source accepted for controlled installed
convergence. Slice I has not been applied. The installed workstation remains on
the earlier `0.28.0-0ed74f1decdb-36f3d74f834d` generation and does not satisfy
the one-dashboard, one-runtime-host, bounded-generation terminal condition.
Explicit live authorization is still required before transactional install,
reconciliation, or cleanup.

## Concurrent maintenance reconciliation

A separate profile-owner maintenance lane installed generation
`0.28.0-0ed74f1decdb-36f3d74f834d` while this plan was at its source-only
boundary. Six compatible maintenance commits were imported into the Plan 0117
candidate:

- cross-profile retained-session attach rejection;
- pre-admission census-block recovery;
- equivalent stable-unit-link acceptance;
- rejected transferred-owner alias retirement;
- profile-lease transfer during runtime handoff;
- stale profile-holder discovery during handoff.

The maintenance commit that wrote forward lifecycle evidence into the legacy
owner registry was deliberately not imported. Slice H supersedes that design
with a versioned lifecycle sidecar while preserving the installed legacy
reader's deny-unknown registry shape.

Live evidence exposed one additional defect in the imported handoff behavior:
the transferred session's stale `profileId` could override the canonical live
browser profile and create false duplicate-profile pressure. Commit `62c38d5e`
now makes the canonical browser process profile authoritative for a
single-browser transfer, rejects repair when the session owns unrelated
browsers or tabs, and retains fail-closed behavior without browser authority.

## Candidate evidence

- Final optimized binary SHA-256:
  `2431fdc51d44403bac5b9b26024ad3a6c405366ea3e26f8fa3c553dfab7dc523`.
- `install workstation --dry-run --json` returned `success=true`,
  `state=planned`, and `mutated=false`.
- Host admission reported Ubuntu 24.04 x86_64 support, effective required
  groups, no missing commands, and about 951 GB available against the 6 GiB
  minimum.
- The official Rust cadence passed 1,358 parallel-safe tests, 57 intentional
  ignores, all integration scopes, and every serialized environment-mutating
  partition.
- The 78-test Chrome module, strict Clippy, Rust formatting, real isolated
  multi-lane hot handoff smoke, and no-launch runtime-host convergence smoke
  passed.
- Commit `77647576` replaced a race-prone GNU `yes` fake-Chrome fixture. GNU
  `yes` rejects Chrome's generated long flags and exited before process
  projection depending on scheduling. The replacement projection fixture uses
  a deterministic browser-looking child with exact process identity.

## Current installed-state readback

No live mutation was performed from this worktree while collecting this
evidence.

- The dashboard ingress and authenticated operator journey report ready on the
  installed generation.
- Install doctor reports `path_command_workspace_binary_mismatch` and
  `service_duplicate_profile_pressure`.
- Three installed-generation listener daemons remain: the default socket and
  two handoff sockets. This is not the Plan 0117 one-host architecture.
- The two retained handoff sessions both publish `profileId=default`, producing
  a false duplicate exclusive-lease warning even though their live browsers
  use distinct canonical profiles. The candidate source repair addresses this
  during transfer.
- Resource inventory reports 84 processes, about 10.57 GB RSS, 45 protected,
  39 observed, and zero process-GC candidates.
- Retained-state dry-run reports exactly one apply-safe candidate,
  `display-orphan`. It has no linked route, browser, session, profile, display
  name, PID, or timestamps. All live and diagnostic allocations remain
  protected.
- The runtime-profile root contains 219 top-level directories and consumes
  about 58 GiB.
- The installed generation root contains 26 generations and consumes about
  905 MiB. The installed pre-P117 retention logic still lets historical
  transaction metadata retain old generations.

## Slice I boundary

After explicit live authorization, use the transactional workstation path with
the candidate above. Preserve the two authenticated browsers, exact process
identities, canonical profiles, CDP targets, displays, routes, leases, and
durable handoffs. Refuse cleanup for any unrelated or ambiguous resource. Apply
only reviewed policy-eligible cleanup, then prove one dashboard, one runtime
host, current plus at most one rollback generation, no overdue reclaimable
ephemeral profile, current monitor evidence, and zero unexplained drift.

Rollback remains the immediately prior installed generation through the
transaction ledger. If any browser, profile, route, display, lease, or operator
journey proof changes unexpectedly, abort or reverse the transaction and leave
ambiguous resources preserved for operator review.
