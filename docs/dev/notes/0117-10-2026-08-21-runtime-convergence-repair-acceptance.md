# Plan 0117 runtime convergence repair acceptance

Date: 2026-08-21

## Outcome

The installed workstation runtime is accepted on generation
`0.28.0-fb5a8ef317c2-9cf9b4f6919d`, binary SHA-256
`fb5a8ef317c29d6e751097f0e3da2577753961f7add7efeb4d4a90e1610f79cb`.
The workspace artifact, PATH command, and selected executable have the same
digest.

Steady-state runtime topology is:

- one logical dashboard, implemented by one selected backend plus the stable
  ingress process
- one selected single-host runtime process
- one executable generation across managed runtime processes
- zero legacy per-session daemons

The accepted transaction is
`upgrade-3d5cf3e2-72a7-4b85-8de9-d49b67f9c048`. Every transaction phase
completed through candidate dashboard management. Admission is open, all seven
workstation readiness axes are true, and install doctor reports no issues.

## Repair scope

The repair addressed all four Plan 0117 concerns.

1. Logical-lane cleanup now preserves shared runtime-host control metadata.
   A live but unreachable host cannot cause a duplicate host bootstrap.
2. Candidate runtime-host identity is captured and staged even when no live
   lane needs transfer. Hot upgrade selection no longer depends on a nonempty
   transfer list.
3. A pre-admission installer-lock collision now has a safe terminal recovery
   transition. The audit receipt is retained while the old selected generation
   remains authoritative.
4. Unknown process pressure is gated by observed RSS rather than aggregate RSS.
   Protected authenticated-browser memory remains visible but does not create a
   false ownership failure.

The repair commits are `39b78bde`, `3de55b1c`, `8dad9eb6`, and `610310e9`.
Supporting bootstrap and identity-ordering repairs are `19150575`, `4d8b137e`,
and `76d715a8`.

## Browser and presentation preservation

Durable handoff `r539344` resolved ready on presentation generation 31. Its
candidate-bound presentation receipt is
`durable-handoff-3acf955c1307f5e76850f1c3538b3abdb18a4f67da91c889a53fef6fe3dd9d26`.
The long-lived social browser retained PID 16807 and its original start
identity. The default-profile browser data was preserved, although its browser
process was relaunched during the authorized hard reset.

## Garbage collection and pressure

The final service GC dry run reports zero candidates and zero projected
reclaimed RSS. Generation GC reports zero candidates after one earlier
unreferenced sealed generation was removed. The selected generation and the
previous healthy rollback generation remain retained.

The final resource census reports 59 protected processes using 4,363,243,520
bytes RSS and 36 observed processes using 889,008,128 bytes RSS. There are zero
reclaimable candidates and zero unknown cleanup obligations. External AuraCall
and Playwright browser processes remain observed and untouched because they are
outside Agent Browser ownership.

## Unattended acceptance

The runtime interlock timer is enabled and active. Its first post-upgrade tick
completed at `2026-08-21T21:59:39.012665659Z` with state `healthy`, zero
consecutive failures, and no incident. A doctor run after that tick remained
green with the same one-dashboard, one-host, one-generation, zero-legacy-daemon
topology.

## Recovery evidence

Pre-repair and transaction recovery evidence is retained at:

- `/home/ecochran76/.agent-browser/recovery/plan117-hard-reset-20260821T145609`
- `/home/ecochran76/.agent-browser/recovery/plan117-host-duplication-20260821T1548`

The second recovery directory includes the runtime-host ingress snapshot taken
before the final exact-identity registry repair. No unrelated browser process
was terminated during that repair.

## Validation

- focused runtime-host cleanup and duplicate-launch tests passed
- focused zero-lane candidate-host staging tests passed
- focused observed-RSS pressure test passed
- focused pre-admission lock-collision recovery test passed
- Rust formatting, clippy with warnings denied, and diff hygiene passed
- final install doctor passed with zero issues
- final workstation status passed with all readiness axes true
- post-upgrade unattended reconciliation passed
