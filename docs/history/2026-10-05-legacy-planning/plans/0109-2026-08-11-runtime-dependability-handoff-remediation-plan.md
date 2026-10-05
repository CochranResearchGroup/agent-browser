# Plan 0109 | Runtime Dependability Handoff Remediation

Date: 2026-08-11

State: SOURCE ACCEPTED | INSTALLED CANARY NOT AUTHORIZED

Authority: SOURCE-ONLY

Lane: P109

Sources:

- `docs/dev/notes/0109-2026-08-11-dependability-handoff-review.md`
- `docs/dev/notes/2026-08-11-im-receipts-google-messages-rdp-handoff.md`
- `docs/dev/notes/2026-08-11-facebook-authenticated-search-blink-crash-handoff.md`
- Plans 0091, 0092, 0096, and 0108

## Current State

Source implementation and source-only validation are complete at source commit
`c00c9655`. Ambiguous global close now rejects before effects, named daemon
sessions have a validated no-browser supervisor contract, remote-view doctor
separates requested-subject readiness from global advisories, renderer crashes
project into typed command and service lifecycle evidence, and effect-capable
service requests require accountable attribution. Installed and live
acceptance remain outside this plan's authority.

## Objective

Make agent-browser fail visibly and locally when a daemon, route, renderer, or
operator command loses its intended subject. Prevent ambiguous global close,
provide first-class fixed-port supervision for a named daemon, report
requested-route readiness separately from unrelated drift, and convert
renderer crashes into typed lifecycle and incident evidence.

## Frozen Decisions

1. `close --all` remains a global operation. Combining it with an explicitly
   supplied session is rejected before discovery or mutation.
2. A named-session supervisor owns only the daemon endpoint. Starting a
   supervisor never launches or navigates a browser.
3. Requested-scope doctor readiness and global host advisories are separate
   fields. Neither is discarded.
4. Compatible privileged helpers remain ready even when bytes or optional
   probes differ. Required ownership and capability checks remain fail-closed.
5. `Inspector.targetCrashed` is crash authority. Target destruction or detach
   alone is not.
6. Opaque authenticated `/remote-view/<handoff-id>` URLs remain canonical.
7. Current profile-mismatch and read-only inventory behavior is preserved and
   tested through real ingress paths.
8. The recurring global runtime interlock remains disabled during this plan.

## Authority And Non-Goals

This plan authorizes repository source, tests, generated fixtures, operator
documentation, and plan/roadmap updates. It does not authorize:

- installing or replacing the local runtime;
- enabling the runtime interlock timer;
- starting, stopping, restarting, closing, navigating, or adopting a live
  browser or daemon;
- editing the im-receipts user unit or protected Google Messages profile;
- editing Chromium source or building a Chromium artifact;
- copying or mutating retained Facebook authentication state;
- running an ignored E2E test, live route smoke, doctor repair, or downstream
  Last30Days retry;
- committing the operator-authored untracked Google Messages note without
  separate direction.

Every compiling Rust command on WSL uses `scripts/ci/cargo-safe.sh` with the
repository lock and memory cap.

## Public Contracts

### Close Safety

The parser must distinguish the default session value from an explicitly
supplied `--session`. Before `run_close_all` scans the socket directory:

- explicit session plus `--all` returns a typed usage error;
- the error states that `close --all` is global;
- it recommends the existing single-session close spelling;
- JSON and text modes agree;
- zero PID, socket, command, signal, or cleanup operations occur.

For ordinary global close, daemon termination also requires exact recorded
process identity. An unreachable daemon with missing, ambiguous, or mismatched
identity is returned under `failed` and is not signaled. Metadata cleanup may
remove only records proven stale by the same identity decision.

### Named Session Supervisor

Add one supported command family, with final spelling chosen during Slice C:

```text
agent-browser session supervisor install <session> --stream-port <port>
agent-browser session supervisor status <session>
agent-browser session supervisor remove <session>
```

The contract requires:

- a validated session name and loopback port;
- a private versioned manifest containing the session, intended executable,
  fixed stream port, optional runtime profile and service configuration path,
  and provenance;
- an instance user service with `Restart=on-failure` and bounded restart rate;
- daemon startup with browser auto-launch disabled;
- crash recovery that may reacquire only a process-identity-proven retained
  browser belonging to the same session and profile, never a default or
  unrelated profile;
- port conflict, executable drift, invalid manifest, and restart exhaustion
  reported as typed states;
- removal to stop only the exact supervised daemon and preserve profiles,
  browser storage, service state, and unrelated units;
- doctor and dashboard health projection of supervisor state;
- Linux-first implementation if portable service-manager support cannot be
  proven in the same slice. Windows and macOS must report unsupported rather
  than silently approximating supervision.

### Requested Doctor Scope

Extend remote-view doctor with optional selectors:

```text
--session <name>
--runtime-profile <id>
--route-id <id>
```

The JSON response adds:

```json
{
  "requestedScope": {
    "selectors": {},
    "status": "ready|degraded|unavailable|not_requested",
    "issues": [],
    "nextAction": "..."
  },
  "globalAdvisories": []
}
```

Backward-compatible global fields remain present. A stale unrelated daemon is
visible under global advisories but cannot make a proven healthy requested
route unavailable. A selector that matches multiple subjects or contradicts
route ownership fails closed as ambiguous.

### Renderer Crash Evidence

Add a typed crash observation containing, when available:

- CDP method and reason;
- target and page session IDs;
- active command and request IDs;
- local principal plus service, agent, and task labels;
- daemon session, requested and detected profile, browser ID, PID, endpoint,
  browser build, and stderr path;
- observation timestamp and source.

One correlated crash must:

- make the in-flight command fail with stable code `target_crashed`;
- persist the affected service tab as `TabLifecycle::Crashed`;
- retain browser, target, profile, process, and stderr evidence;
- emit one service event and one deduplicated incident;
- invalidate ready stream/action projection for that tab;
- leave unrelated tabs and browser ownership intact when the browser process is
  still alive.

Explicit close, target replacement during navigation, and ordinary detach must
not be misclassified as crashes.

### Effect Attribution And No-Launch Reads

Effect-capable service requests require an accountable principal. Preferred
attribution is explicit `serviceName`, `agentName`, and `taskName`; authenticated
dashboard identity or a local CLI principal plus request ID is an acceptable
fallback. A request with no derivable principal is rejected before launch,
attach, restart, cleanup, or route mutation.

Profile, browser, session, tab, monitor, provider, site-policy, and challenge
collection reads remain label-optional and must never launch, attach, or
restart a browser.

## Execution Slices

### Slice A | Red Fixtures And Identity Ledger

- Freeze the exact command, daemon, route, target, profile, and principal
  identities used by later tests.
- Add a structural ingress ledger for CLI, HTTP, MCP, and dashboard paths.
- Add red fixtures for explicit-session global close, target crash, requested
  doctor scope, and supervised-daemon restart.
- Record the expected current failures without running live commands.

Commit: `test: freeze runtime dependability handoff regressions`

### Slice B | Close Scope Safety

- Track whether `--session` was explicitly supplied.
- Reject the conflicting global-close spelling before `run_close_all`.
- Replace PID-file-only unreachable-daemon force kill with bound daemon process
  identity and a conservative failure outcome.
- Add JSON/text parity and zero-effect fixture assertions.
- Update CLI help, README, skill, docs site, and inline comments.

Commit: `fix: reject ambiguous global session close`

Rollback: revert this commit only. No persisted schema changes occur.

### Slice C | Renderer Crash Lifecycle

- Subscribe to and parse `Inspector.targetCrashed`.
- Carry typed crash observations through `DrainedEvents` into one lifecycle
  owner instead of adding local string heuristics.
- Correlate the active command and retained service subject.
- Persist tab crash state, event, and incident atomically.
- Return typed failure to the command that owned the target.
- Add normal target-destroy, detach/reattach, explicit-close, browser-process
  exit, and multi-tab isolation controls.

Commit: `fix: project renderer crashes into service lifecycle`

Rollback: revert before any schema writer is deployed. New optional evidence
fields must remain backward-readable if later slices have landed.

### Slice D | Named Fixed-Port Session Supervisor

- Define the versioned manifest and pure validation model first.
- Generate and manage the exact instance user unit without shell interpolation
  of session or path values.
- Start the daemon in no-browser mode with the intended fixed port.
- Add bounded crash restart, port conflict, executable drift, and restart-rate
  diagnostics.
- Project supervisor health into install doctor and `/api/runtime/health`.
- Add a migration guide for downstream custom units without editing them.

Commit: `feat: supervise named daemon sessions`

Rollback: remove only the generated instance unit and manifest for the test
session. Preserve runtime profiles and browser state.

### Slice E | Requested-Scope Doctor And Helper Evidence

- Parse the three optional selectors.
- Build one subject-selection function with ambiguity fixtures.
- Separate requested-scope blockers from global advisories.
- Preserve the existing global doctor contract.
- Report helper command-set, contract version, capability readiness, and
  provenance separately.
- Prove a compatible helper remains ready without `verify-install` while a
  missing required capability remains blocking.
- Add opaque-handoff compatibility assertions.

Commit: `feat: scope remote view diagnostics to requested subjects`

Rollback: selectors and additive JSON fields can be reverted without changing
installed state.

### Slice F | Attribution And No-Launch Ingress Parity

- Add the typed principal derivation at the normalized request boundary.
- Preserve supplied service, agent, and task labels exactly.
- Reject unaccountable effectful requests before routing.
- Exercise profile mismatch and collection reads through CLI, HTTP, MCP, and
  generated client entrypoints.
- Add process and browser-launch sentinels proving collection reads are
  side-effect-free.

Commit: `fix: bind browser effects to accountable requests`

Rollback: keep evidence fields optional until all supported clients carry the
new attribution contract.

### Slice G | Documentation, Audit, And Source Acceptance

- Update every required user-facing documentation surface.
- Update ROADMAP and RUNBOOK with source-only status.
- Run one fresh-context drift review across the accepted findings.
- Perform one bounded remediation pass.
- Run closed-world verification only for accepted findings and regressions
  introduced by their fixes.
- Stop and split the unit if a blocking finding remains. Do not start a third
  broad review cycle.

Commit: `docs: close runtime dependability source plan`

### Slice H | Separately Authorized Installed Canary

Not authorized by this plan.

When explicitly approved, use a disposable session and profile, not Google
Messages or Facebook. Prove fixed-port restart, scoped doctor readiness,
global advisory visibility, dashboard drift warning, exact unit rollback, and
zero unrelated process or session effects. Only after that can a downstream
custom unit migration or interlock decision be proposed.

## Required Tests

1. `close --all` with explicit `--session` fails before filesystem or process
   inspection.
2. ordinary `close --all` retains its global behavior in an isolated fake
   socket directory and never signals an identity-mismatched PID.
3. single-session close remains unchanged.
4. supervised daemon restart preserves its session and fixed stream port and
   does not launch a browser.
5. port collision and restart exhaustion fail visibly without choosing a new
   port.
6. unrelated stale daemon appears only as a global advisory when requested
   route evidence is ready.
7. contradictory session, profile, and route selectors fail closed.
8. compatible installed helper without optional `verify-install` remains
   contract-ready and does not request interactive sudo.
9. `Inspector.targetCrashed` fails the owning command, marks one tab crashed,
   and emits one incident with retained evidence.
10. target destroy, detach/reattach, and explicit close do not emit a crash.
11. profile mismatch rejects before attach or launch through CLI, HTTP, MCP,
    and generated client paths.
12. all collection reads execute with launch, attach, restart, and cleanup
    sentinels at zero.
13. unaccountable effectful requests reject before the first effect; read-only
    requests remain available.
14. durable public URLs remain opaque `/remote-view/<handoff-id>` values.

## Validation

At minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml <focused-filter> -- --test-threads=1
scripts/ci/rust-tests.sh
pnpm test:service-client
pnpm test:service-api-mcp-parity
pnpm test:actions-architecture
pnpm test:wsl-cargo-safety
pnpm build:dashboard
pnpm --dir docs build
pnpm validation:select -- --base <last-known-green-ref>
git diff --check
```

The selector may add narrower workstation, remote-view, service-contract,
dashboard, or generated-client gates based on touched paths. No ignored E2E or
live smoke is source acceptance evidence.

## Hard Stops

- Stop if a close test reaches a real socket directory or process.
- Stop if a supervisor fixture resolves to the real home, a protected profile,
  or an occupied non-test port.
- Stop if crash propagation requires treating all target destruction as a
  crash.
- Stop if a requested-scope doctor hides unrelated drift rather than
  reclassifying it as advisory.
- Stop if helper compatibility weakens root ownership, fixed path, sudoers, or
  required capability checks.
- Stop if attribution requires private browser content or credentials.
- Stop before enabling the interlock, installing units, publishing a binary,
  copying authentication state, or retrying Facebook.

## Acceptance

- all fourteen behavioral requirements pass through public interfaces;
- ambiguous global close is impossible;
- a named daemon can be represented, supervised, and diagnosed without
  implicit browser work;
- requested-route readiness is truthful even when unrelated drift exists;
- renderer failure cannot leave the affected tab ready or the owning command
  successful;
- effectful requests have accountable provenance and collection reads remain
  side-effect-free;
- completed contracts from Plans 0092, 0096, and 0108 remain intact;
- source acceptance and installed acceptance are reported separately;
- the global runtime interlock remains disabled unless a later authorized
  canary changes that decision.

## External Follow-Up

The Chromium repository should identify and adopt the upstream `LineBreaker`
change that handles a larger reshaped `EndOffset()` without the old fatal
assertion. That lane requires its own plan, DCHECK-enabled disposable build,
and separately authorized authenticated comparison. It is not part of this
repository plan.

## Source Acceptance | 2026-08-11

Implementation commits, in dependency order:

- `0b0aa14b` rejects explicit-session global close before inspection or
  mutation;
- `9c538699` documents the global close contract;
- `9ab3e6b6` projects renderer crashes into typed command, tab, event, and
  incident state;
- `bcf73c7b` adds validated Linux named-session supervision with no browser
  launch;
- `beb8d18b` adds requested-subject remote-view doctor status and preserves
  global advisories;
- `7d82faf4` binds effectful service requests to an accountable principal and
  adds cross-ingress no-launch collection coverage;
- `7989ba6a` restores renderer-crash response ownership and updates the
  process-identity consumer inventory;
- `c00c9655` isolates the legacy service-status compatibility fixture from
  ambient repository state.

Fresh-context Cycle 1 found two structural regressions: the crash response
helper had entered the six-definition action dispatcher, and the process
identity consumer inventory did not include the new Plan 0109 consumers. Both
were corrected in `7989ba6a`. The first canonical validation exposed one
ambient-HOME test fixture; `c00c9655` made it repository-local and deterministic.
Closed-world Cycle 2 found no remaining source blocker.

The fresh canonical guarded Rust driver exited zero. Its parallel-safe lane
passed 1,071 tests with 57 ignored; close-scope 4/4, requested doctor scope
1/1, named-session supervisor 1/1, and every declared serial partition passed.
Strict Rust formatting and Clippy, architecture and WSL safety gates,
service-client and API/MCP parity, the no-launch collection smoke, dashboard
and docs builds, selected fixture gates, and patch checks also passed.

Slice H was not executed. No runtime was installed, no unit was enabled or
restarted, no browser was launched or adopted, no protected profile was read
or changed, and no downstream Google Messages, Facebook, or Last30Days attempt
was consumed.
