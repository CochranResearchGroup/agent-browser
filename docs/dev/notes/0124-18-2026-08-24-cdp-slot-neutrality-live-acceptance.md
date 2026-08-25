# Plan 0124 CDP Slot Neutrality Live Acceptance

Date: 2026-08-24

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Authority: DEVELOPMENT RUNTIME EFFECTS | PRODUCTION READ-ONLY

Status: ACCEPTED

## Accepted Boundary

One ordinary CDP-only browser ran on the isolated development runtime without
allocating or changing presentation capacity. The browser used session and
runtime profile `p124-cdp-neutral-20260824-c` on development generation
`0.28.0-57fe9956d43f`.

While the browser was live:

- navigation and a second `get title` command both returned
  `Plan 0124 CDP Neutrality Direct`;
- the exact browser process was PID `98891` and health was `ready`;
- `displayAllocationId`, `displayIsolation`, and `displayName` were null;
- the only view stream was `cdp_screencast` with `cdp_input`;
- all four presentation slots remained `warm_idle`;
- the configured hard and pressure-admitted maxima remained six;
- one human-protected slot and one recovery-reserved slot remained configured;
- no viewer lease or presentation queue entry appeared.

The exact session then closed successfully. Runtime status reported no live PID,
no DevTools endpoint, and no targets. Service status retained no browser or
session for the acceptance identity, kept all four slots `warm_idle`, and kept
the viewer-lease set empty. Runtime resource inventory recorded
`exact_process_exited` and `profile_lock_released` for the browser generation,
with zero cleanup candidates and zero candidate RSS.

Development runtime doctor remained green after cleanup. Production was not
mutated by this acceptance.

## Rejected Harness Evidence

Two preliminary standalone MCP harnesses completed no-presentation browser
requests but did not keep their browsers attached to the selected development
runtime host. Their process-exit incidents were preserved and resolved as
recovered. Neither harness was used as acceptance evidence, and neither
completed request was retried.

## Retained-State Cleanup Observation

A reviewed development-only retained-state prune removed three dead degraded
browser placeholders and one closed tab. It removed no profile, session,
display allocation, or live process. The final Service State had zero modeled
browsers and four live warm display allocations.

The cleanup census initially exposed four older presentation lifecycle rows as
`closing` with owned cleanup obligations even though their process groups were
absent and zero reclaimable process candidates existed. Reconciliation could
not join current managed-profile shorthand records to their canonical profile
identities and treated stale Chrome `SingletonLock` symlinks as live locks.

Commits `c9487b51` and `217f2abf` repaired those two boundaries without
weakening lifecycle authority. Managed-profile shorthand is considered only
when `userDataDir` exactly equals the profile ID. A stale lock is accepted only
when its PID exactly matches the recorded process group and both the PID and
process group are absent. The terminal receipt distinguishes stale-lock proof
from an absent lock.

Development generation `0.28.0-df2844eaf9d2`, binary SHA-256
`df2844eaf9d25da02277264c05a27cee0f74791e7576599f0c431151bda96e9a`,
completed exactly three current `v5` lifecycle rows. Cleanup obligations moved
from 4 owned and 30 satisfied to 1 owned and 33 satisfied. The remaining
`development-presentation-provider-v1-2` row has no current profile mapping and
remains protected rather than guessed away. Candidate process count and RSS
remain zero. This final legacy row is retained lifecycle migration residue,
not live system pressure, and remains part of Plan 0124 convergence work.

## Remaining Boundary

Plan 0124 remains in progress. This acceptance does not yet prove active human
controller precedence, passive-viewer non-disruption, two concurrent desktop
observations plus recovery reservation, or retained authenticated-browser
survival through route movement and unrelated scale-in. The full
browser-external episode also has not been repeated on the final cleanup
overlay generation. The one unprovable legacy `v1` lifecycle row also remains
open.
