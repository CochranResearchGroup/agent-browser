# Plan 0126 | Pre-Development Runtime Safety And Browser Launch Stabilization

Date: 2026-08-23

State: ACCEPTED

Execution state: `accepted_published_source_and_installed_development`

Lane: P126

Authority: SOURCE AND DEVELOPMENT RUNTIME EFFECTS | REVIEWED TERMINAL PROCESS CLEANUP | PRODUCTION READ-ONLY

Depends on:

- `docs/dev/plans/0125-2026-08-23-development-runtime-isolation-and-build-capacity-plan.md`
- `docs/dev/plans/0117-2026-08-19-runtime-lifecycle-authority-and-convergence-plan.md`
- `docs/dev/plans/0029-2026-06-07-live-retained-pressure-cleanup-plan.md`
- `docs/dev/policies/0031-runtime-vs-product-boundary.md`
- `docs/dev/policies/0032-runtime-state-governance.md`
- `docs/dev/policies/0036-architecture-guardrails.md`

## Goal

Close every known runtime-safety blocker before new feature development. The
development environment must launch a fresh Linux-profile browser through its
stable command, and its garbage collector must never classify a process as a
candidate merely because that process is absent from the development ledger.
The workstation must end with reviewed terminal test residue reclaimed, exact
production identities preserved, and a fresh resource census that separates
active mixed-owner pressure from package-owned cleanup obligations.

P126 is complete when the original launch command is green without a caller
override, the original development GC dry-run reports no production-owned
candidate, deterministic regressions prove both boundaries, and every applied
cleanup is bound to an exact process identity and terminal ownership proof.

## Current Evidence

### Development GC Cross-Environment Defect

The development and production resource views classify the same live PIDs
differently:

- development GC proposes Xvfb PIDs 8319, 30094, and 68300 as
  `orphaned_remote_display_process`, totaling 83.5 MiB;
- production resource authority protects those same processes through retained
  display allocations;
- development has no remote-display provider or display allocation of its own;
- no GC apply occurred.

The cause is structural. `service_resources` observes the whole OS process set
but correlates it only against the current environment's Service State. An
uncorrelated Xvfb process becomes a candidate without positive proof that the
current Runtime Environment owns it. A review token authenticates the candidate
set but does not repair incorrect ownership classification.

### Development Browser Launch Defect

This dev-only command is the red-capable reproduction:

```bash
agent-browser-dev --session p126-fresh-launch \
  --runtime-profile p126-fresh-launch --json open about:blank
```

It fails deterministically in under two seconds after three attempts. Each
attempt reports Chrome exit code zero before DevTools and an empty stderr log.

The development no-launch status selects a Windows-mounted Chromium executable
while the managed profile is under the Linux-only development pseudo-home.
Running the same isolated flow with
`AGENT_BROWSER_EXECUTABLE_PATH=/opt/google/chrome/chrome` succeeds, returns
`about:blank`, and closes without residue. This confirms an environment-owned
browser executable compatibility defect rather than a profile lock, sandbox,
or host-pressure failure.

### Mixed-Owner Workstation Pressure

The current census contains Agent Browser, AuraCall, Playwright/dev-browser,
Chrome crash handlers, remote displays, and a two-day-old browser rooted in a
temporary Agent Browser test home. Raw Chrome-family process count is not an
ownership verdict. Production GC currently reports zero candidates, while the
temporary test browser is not modeled strongly enough for automatic cleanup.

## Frozen Decisions

### 1. Candidate Requires Positive Environment Ownership

Absence from the current environment ledger is never ownership proof.

For the development environment, a process can become a GC candidate only when
current evidence positively binds it to that environment, such as:

- an exact retained lifecycle owner in development Service State;
- a managed profile path under the development state root;
- the selected development generation or stable socket namespace;
- a future display allocation carrying explicit development environment
  identity.

An OS-visible browser, daemon, display, or helper without that proof remains
observed and protected from development GC. Production behavior remains
unchanged in this slice. Future development presentation cleanup must add
explicit environment identity rather than weaken this guard.

The same guard applies to reviewed and unattended GC. Review tokens and force
flags cannot convert foreign or unknown ownership into current-environment
authority.

### 2. Browser Executable Is Part Of Runtime Environment Identity

The development publisher selects one host-compatible browser executable and
exports it consistently through the stable launcher and all development units.
The initial Linux workstation selection is `/opt/google/chrome/chrome`, unless
an explicit `AGENT_BROWSER_DEV_BROWSER_EXECUTABLE` override supplies another
reviewed executable.

The publisher fails before activation when the selected browser path is not an
absolute executable file or is a Windows executable paired with a Linux-only
development profile root. It does not inherit an ambient production or Windows
browser manifest accidentally.

Doctor reports the configured browser executable and verifies that the stable
launcher and all units agree. A caller may still provide an explicit
`AGENT_BROWSER_EXECUTABLE_PATH` for one diagnostic command, but ordinary
development commands require no override.

### 3. Cleanup Is Exact And Recovery-Aware

P126 may reclaim only:

- disposable P126 profiles after their exact browser session is closed;
- the exact two-day-old test browser process tree after PID, start token,
  executable, test-root path, lack of service ownership, and terminal age are
  rechecked immediately before termination;
- its matching temporary test root after no process references it.

Cleanup is recoverable where practical. Directories move to desktop trash.
Process termination is not recoverable, so it requires the full exact identity
check and must stop if the identity changes.

Production browsers, AuraCall profiles, Playwright/dev-browser workloads,
retained displays, routes, viewers, handoffs, and provider processes are out of
scope.

### 4. Resource Health Has Separate Axes

Acceptance reports:

- current environment candidates and protected resources;
- foreign or unknown observed resources;
- package-owned terminal residue;
- total OS browser-family pressure;
- production retained-browser and display authority.

A high raw process count is not by itself a cleanup instruction. A zero
candidate count is not by itself proof of low system pressure.

## Implementation Slices

### Slice A | Red Fixtures And Environment Contract

- add a resource-classification fixture where development sees a
  production-owned Xvfb process without its production display ledger;
- require the fixture to remain non-candidate with a typed foreign-environment
  reason;
- add publisher fixtures for Linux browser selection, launcher/unit propagation,
  invalid path rejection, and Windows/Linux profile incompatibility;
- add a dev fresh-browser smoke that exercises stable-command open, URL read,
  close, and residue checks.

### Slice B | GC Ownership Guard

- bind service resource classification to the current runtime environment;
- require positive development ownership before any candidate disposition;
- apply the same classifier to dry-run, reviewed apply, and unattended apply;
- surface the guard in policy output and documentation;
- preserve production classification behavior.

### Slice C | Development Browser Selection

- add browser executable to the development descriptor;
- validate the selected path before writing or activating a generation;
- render the exact selection into the launcher and all three units;
- extend status and doctor with path, compatibility, and propagation checks;
- reinstall only the development environment and verify the no-launch status.

### Slice D | Reviewed Runtime Cleanup And Acceptance

1. Capture exact production and development runtime identities.
2. Reinstall and restart only development units.
3. Run the original fresh-browser reproduction at least three times.
4. Close each exact disposable session and verify no matching process remains.
5. Prove development GC no longer proposes production-owned displays.
6. Prove production still protects its retained displays and reports no GC
   candidates.
7. Recheck and terminate only the exact terminal test browser tree.
8. Move its unreferenced temporary test root and P126 disposable profiles to
   trash.
9. Record final memory, swap, browser-root, process, RSS, and runtime identity
   evidence.

## Required Validation

```bash
pnpm test:development-runtime
pnpm test:service-resource-environment-safety
pnpm smoke:development-browser-launch
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
pnpm validation:select -- --base HEAD
git diff --check
```

Run focused Rust service-resource tests and the installed development doctor.
Bind live acceptance to fresh production before/after identity snapshots.

## Acceptance Criteria

- the original development launch command succeeds without an executable
  override on three consecutive disposable profiles;
- every launch returns the expected URL and exact close succeeds;
- no P126 browser or profile process remains afterward;
- development launcher, units, doctor, and no-launch status agree on one
  Linux-compatible browser executable;
- development GC returns zero candidates for every production-protected Xvfb
  process;
- a foreign or unknown process cannot become a candidate through review-token
  or unattended paths;
- production GC behavior and retained-display protection remain unchanged;
- the exact stale test browser and only its process tree are removed after a
  repeated identity check;
- production selected generation, daemon, dashboard, handoff, retained browser,
  session, display, and route identities survive all development effects;
- the final census distinguishes active mixed-owner pressure from reclaimable
  package residue;
- P124 remains source-not-started until P126 is accepted and published.

## Hard Stops

- Do not apply the current development GC token.
- Do not kill an Xvfb process proposed only by development state absence.
- Do not run production reconciliation, installation, or unit restart.
- Do not close a retained, AuraCall, Playwright, dev-browser, human, or unknown
  browser.
- Do not make Windows Chrome consume a Linux-only managed profile.
- Do not make development cleanup read production private state as its normal
  ownership database.
- Do not turn raw process count or elapsed age into deletion authority.
- Stop if PID, start token, executable, profile root, or owner evidence changes
  between review and effect.

## First Bounded Packet

Execute Slices A through D as one pre-development stabilization lane. Do not
start P124 feature implementation until P126 is accepted, pushed, and the
final installed readback is green.

## Acceptance

Accepted on 2026-08-23. Source and installed-development evidence is recorded
in
`docs/dev/notes/0126-2026-08-23-pre-development-runtime-safety-and-browser-launch-acceptance.md`.
Production remained read-only. P124 may proceed only through its separately
governed first bounded packet.
