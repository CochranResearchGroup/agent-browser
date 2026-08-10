# Plan 0108 | Runtime Process Identity And PID Reuse Repair

Date: 2026-08-10

State: DRAFT | RED FIRST REQUIRED

Authority:

- `docs/dev/notes/2026-08-10-last30days-stale-runtime-pid-lock-handoff.md`
- `docs/dev/plans/0077-2026-07-25-profile-discovery-and-manual-browser-launch-ux-plan.md`
- the current `architecture-deepening-20260809` branch after Candidate 4 final
  test remediation is committed

## Objective

Prevent operating-system PID reuse from making a stale runtime record or
Chromium profile lock appear to belong to a live managed browser. Runtime
status, manual-browser projection, launch-time lock protection, stale-lock
cleanup, diagnostics, recovery, and service ownership must use one process
identity decision instead of PID existence alone.

## Hard Safety Boundary

This plan does not authorize any mutation of the real
`last30days-facebook` profile, PID 63205, installed runtime, browser storage,
X session, service route, or downstream Last30Days acceptance attempt.

All fixtures use temporary profiles and homes. No test may resolve to
`~/.agent-browser`. No browser, install, doctor, live route, release, push, or
ignored end-to-end effect is authorized. Every compiling Rust command on WSL
must use `scripts/ci/cargo-safe.sh`.

The unrelated untracked architecture inventory cache remains excluded.

## Confirmed Defect

Current source has three independent PID-only decisions:

1. `runtime_status_with_user_data_dir()` derives `browserAlive` from signal-zero
   liveness.
2. `ensure_profile_not_in_use()` rejects a singleton lock whenever its PID
   exists.
3. `cleanup_stale_profile_lock()` removes artifacts only when that PID no
   longer exists.

`RuntimeState` stores the PID but no process-start identity. A later unrelated
process can therefore reuse the number and inherit false ownership. The
current unreachable-DevTools test encodes that incorrect result.

## Frozen Ownership Model

Add one deep, side-effect-free decision interface backed by a narrow
platform-observation adapter.

```rust
pub struct RecordedProcessIdentity {
    pub pid: u32,
    pub start_token: String,
    pub executable_path: Option<String>,
    pub browser_family: Option<String>,
}

pub struct ObservedProcessIdentity {
    pub pid: u32,
    pub start_token: Option<String>,
    pub executable_path: Option<String>,
    pub browser_family: Option<String>,
}

pub enum RuntimeProcessOwnership {
    MatchingBrowser,
    Missing,
    ReusedUnrelated,
    AmbiguousLegacyBrowser,
}
```

Names may adjust to match local conventions, but these four meanings and their
decision table are fixed.

| Recorded evidence | Observed process | Endpoint evidence | Ownership | Status alive | Lock cleanup |
|---|---|---|---|---|---|
| exact start token and browser family | same identity | optional | matching browser | yes | preserve |
| exact start token | process missing | any | missing | no | remove |
| start token differs | any live process | any | reused unrelated | no | remove |
| executable proves non-browser | live unrelated process | any | reused unrelated | no | remove |
| legacy record without identity | live non-browser | any | reused unrelated | no | remove |
| legacy record without identity | live browser | unreachable or unproven | ambiguous legacy browser | no | preserve |
| legacy automated record without identity | live browser | reachable and profile-consistent | matching browser compatibility | yes | preserve |

Authorization and service metadata cannot upgrade an identity mismatch into a
match. Endpoint reachability supports only the narrow legacy automated case.
New records always capture process identity, including manual no-CDP launches.

## Platform Observation Contract

The adapter observes without signaling or mutating the process.

- Linux reads `/proc/<pid>/stat` process start ticks and `/proc/<pid>/exe`.
- Windows reads process creation time and executable path through the existing
  `windows-sys` integration.
- macOS uses its supported process metadata API when available.
- A platform or permission failure returns unavailable evidence and therefore
  the conservative ambiguous outcome for a live browser-looking process. It
  never guesses a match.

The serialized start token is opaque and platform-qualified. No wall-clock
conversion is required. Matching is exact within one host boot and process
namespace.

## Persistence Compatibility

Add an optional recorded process identity to `RuntimeState`. Existing JSON
continues to deserialize. `write_runtime_state()` writes the identity for every
new managed or manual browser after spawn and before ownership is advertised.

Legacy records remain readable. Their compatibility rule is explicit:

- a live unrelated executable is stale PID reuse;
- a live browser with profile-consistent reachable DevTools may remain live for
  legacy automated mode;
- a live browser without sufficient endpoint evidence is ambiguous, not live,
  and its lock is preserved;
- a missing process is stale.

Do not silently backfill a legacy identity from PID alone.

## Shared Consumers

Route the following through the same ownership decision:

- `runtime_status_with_user_data_dir()` and runtime-profile listing;
- manual-runtime browser projection;
- `ensure_profile_not_in_use()`;
- `cleanup_stale_profile_lock()`;
- lock-owner diagnostics and service-browser attribution;
- managed-runtime recovery, attach, close, and service-health ownership checks
  that currently call a PID-only helper.

Raw `pid_is_running()` may remain only as a platform primitive inside the
observation adapter or for explicitly non-ownership uses. Add a structural
guard or exact call-site ledger so ownership consumers cannot regress to it.

## Red-First Feedback Loop

Before production changes, add and run a deterministic public-seam regression:

`test_runtime_status_rejects_reused_unrelated_pid`

The test uses a temporary home/profile, writes runtime state naming the current
test process or a controlled child, records mismatching browser identity, uses
an unreachable disposable DevTools port, and expects `browserAlive=false`.
The current implementation must fail this assertion. Preserve the red output
in the execution receipt.

Then add one vertical green slice through the shared ownership decision and
runtime status before extending it to profile locks and other consumers.

## Required Behavioral Tests

All tests are isolated and deterministic.

1. A live unrelated process reusing the recorded PID is not browser-live.
2. Cleanup removes only disposable singleton and DevTools artifacts for the
   reused unrelated process, while the process remains alive.
3. A matching live browser identity remains protected and produces the current
   actionable lock error.
4. A new manual no-CDP runtime with matching identity remains live.
5. A legacy live non-browser PID is stale and safe to clean.
6. A legacy live browser without sufficient endpoint proof is ambiguous,
   reported not live, and not cleaned.
7. A legacy automated browser with reachable profile-consistent DevTools
   retains compatibility.
8. A mismatched PID is not attributed to a runtime profile or service browser
   in lock diagnostics.
9. New launch persistence records identity before status or ownership becomes
   observable.
10. Platform observation failure fails conservatively and never deletes an
    ambiguous live browser lock.

Update the old unreachable-DevTools expectation to the new identity contract
and document why it changed.

## Implementation Slices

### R01 | Red Fixture And Decision Ledger

- Add the public runtime-status PID-reuse fixture and observe it fail.
- Freeze exact ownership outcomes and legacy compatibility fixtures.
- Add a no-real-home guard to every new fixture.

### R02 | Shared Process Identity

- Add the platform observation adapter and pure ownership decision.
- Add optional identity persistence to `RuntimeState`.
- Capture identity for automated and manual launch paths.
- Make the runtime-status red fixture green.

### R03 | Lock Safety And Diagnostics

- Replace PID-only lock rejection and cleanup with the shared decision.
- Preserve matching and ambiguous live browser locks.
- Clean proven missing or reused-unrelated artifacts without signaling the
  unrelated process.
- Filter runtime and service ownership diagnostics through the same decision.

### R04 | Remaining Ownership Consumers

- Migrate manual projection, recovery, attach, close, and service-health
  ownership decisions identified by CodeGraph impact analysis.
- Delete duplicate local identity heuristics.
- Add the exact call-site guard.

### R05 | Validation And Independent Evaluation

- Run focused status, lock, manual, diagnostic, and service-owner tests.
- Run the guarded canonical Rust suite, formatting, strict Clippy, and the
  validation selector.
- Use an independent evaluator for deletion safety, legacy compatibility, and
  cross-platform behavior before any installation decision.
- Commit source and a source-bound test receipt only after green evaluation.

## Validation Commands

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  test_runtime_status_rejects_reused_unrelated_pid -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  runtime_profile::tests -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  native::cdp::chrome::tests -- --test-threads=1
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/rust-tests.sh
pnpm validation:select -- --base <last-known-green-ref>
git diff --check
```

The canonical driver must first include Candidate 4's bounded no-launch and
parallel-state fixes. A suite that launches against the default runtime profile
is not acceptable validation evidence.

## Acceptance

- The deterministic PID-reuse fixture is observed red before production code
  and green afterward.
- All ownership consumers use the shared decision.
- A reused unrelated PID is neither reported live nor protected as the browser
  owner.
- Cleanup never kills or signals the reused process and removes only disposable
  test artifacts.
- Matching and ambiguous live browser locks remain protected.
- Manual no-CDP lifecycle and legacy automated compatibility are preserved.
- Current focused and canonical no-launch gates pass under the WSL guard.
- An independent evaluator accepts deletion safety and compatibility.
- No real profile, installed runtime, live browser, X route, or downstream tick
  is mutated.

## Return Handoff

Return the repair commit, branch, focused and broad test receipts, exact
installed-runtime state if separately authorized, and a no-real-profile-change
statement to the Last30Days agent. The Last30Days agent alone owns its Plan
0040 preflight and one remaining X-only acceptance attempt.
