# Plan 0156: Full Runtime Shutdown Replacement

Date: 2026-09-02

State: ACTIVE

Lane: P156

Branch: `feature/full-runtime-shutdown`

Target: `main`

Source baseline: `5d35900a555f90fb6731ad8760ccae74e7b729c1`

Authority: SOURCE, DOCUMENTATION, PROVIDER-FREE FIXTURES, AND ISOLATED
DEVELOPMENT QUALIFICATION ARE IN SCOPE. APPLYING FULL SHUTDOWN TO THE
PRODUCTION RUNTIME, CLOSING THE RETAINED RESEARCH.GOV BROWSER, DELETING A
PROFILE, ENTERING CREDENTIALS, OR MUTATING PROVIDER STATE IS OUT OF SCOPE.

Dependencies: [P148, P150, P155]

Overlaps: [P144]

## Incident

The workstation installer can preserve live owned browsers through cooperative
handoff, and the supervisor takeover transaction can replace an exact
browserless selected host. When a live owned browser cannot complete the
cooperative path, those safety boundaries compose into a permanent installer
block. The operator has no first-class way to choose session loss, close the
old runtime completely, preserve on-disk profile state, and converge onto one
new supervised runtime.

## Objective

Add one explicit runtime-replacement policy to workstation installation. The
default policy preserves live browser continuity. The `full-shutdown` policy
authorizes the installer to close every exact old-runtime-owned browser and
retire the exact selected old host so installation can converge cleanly.

Full shutdown preserves managed profile directories, cookies, password-manager
state, passkey registrations, and other on-disk browser data. It intentionally
ends tabs, live CDP attachments, streams, viewer leases, and remote-view
continuity. It never kills an unknown or foreign process and never deletes a
profile.

## Deep Module And Interface

Introduce one `runtime_replacement` transaction module. Workstation installation
selects a policy and delegates replacement planning and effects to this module;
callers do not enumerate PIDs, sessions, browsers, routes, or signals.

```rust
pub(crate) enum RuntimeReplacementPolicy {
    Preserve,
    FullShutdown,
}

pub(crate) fn plan_runtime_replacement(
    policy: RuntimeReplacementPolicy,
) -> Result<RuntimeReplacementPlan, String>;

pub(crate) fn apply_runtime_replacement(
    expected_plan_digest: &str,
    policy: RuntimeReplacementPolicy,
) -> Result<RuntimeReplacementOutcome, String>;
```

The user-facing installer option is:

```text
agent-browser install workstation --dry-run \
  --runtime-replacement-policy full-shutdown --json

agent-browser install workstation --apply \
  --runtime-replacement-policy full-shutdown \
  --expected-runtime-replacement-plan-digest <sha256> --json
```

`preserve` remains the default. `full-shutdown` apply requires the exact digest
from a current dry run. The policy is persisted in the workstation transaction
and cannot change during resume.

## Frozen Safety Contract

1. Dry-run is read-only and lists every exact browser, host, lane, route,
   display, handoff, viewer lease, and profile preservation consequence.
2. Apply recomputes all observations and requires the exact plan digest before
   any effect.
3. Only package-owned processes with verified PID, start token, executable,
   profile identity, and runtime ownership may be signaled.
4. Each browser receives one bounded graceful close through its owning runtime.
   If the exact process remains, termination escalates through the same verified
   identity handle. PID-only or name-based cleanup is forbidden.
5. Browser exit and profile-lock release are required before retiring the old
   runtime host.
6. Profile directories and credential-bearing browser state are never removed,
   replaced, or copied by full shutdown.
7. Routes, displays, viewers, handoffs, sessions, and owner records transition
   through their public lifecycle rules. Direct state-file editing is forbidden.
8. Unknown and foreign processes remain untouched. Conflicts are isolated into
   a fresh replacement namespace or returned as a typed external conflict.
9. After the first confirmed browser close, rollback cannot restore live tabs.
   Recovery is forward-only and the admission drain remains until one coherent
   replacement or an exact recovery edge is recorded.
10. Acceptance requires one selected generation, one supervised runtime host,
    no old owned browser or host process, no old listeners, and no active old
    runtime ownership records.

## Work Units

| Unit | Scope | Depends on | Exit condition |
| --- | --- | --- | --- |
| W1 | Register plan and add parser and authority regressions | none | Default preserve, explicit full shutdown, and digest requirements are red then green |
| W2 | Add replacement plan and durable receipt model | W1 | Exact effect inventory and digest are deterministic and replay-safe |
| W3 | Add exact owned-browser shutdown and host retirement executor | W2 | Provider-free fixtures prove graceful close, verified escalation, profile preservation, and foreign-process fences |
| W4 | Integrate replacement with workstation prepare, resume, and recovery | W3 | Installation cannot strand a policy-less or policy-changing transaction |
| W5 | Update every user and agent documentation surface | W4 | Help, README, skill, docs site, inline docs, roadmap, and runbook agree |
| W6 | Validate and qualify an isolated development candidate | W5 | Focused tests, format, strict Clippy, installer fixtures, doctor, and disposable launch smoke pass |

## Bounds And Stop Rules

- Maximum implementation attempts: 2
- Maximum review and remediation cycles: 1
- Maximum no-progress checkpoints: 2
- Checkpoint interval: each completed work unit or 90 minutes
- Do not apply full shutdown to production in this plan.
- Do not use broad process-name matching, recursive process cleanup, or direct
  service-state editing.
- Stop before browser effects if the plan digest, runtime census, process start
  token, profile identity, selected ingress revision, or transaction authority
  changes.

## W1 Checkpoint

State transition: `ready -> active`.

Acceptance state: W1 complete; W2 through W6 remain.

Progress classification: `blocker_reduction`.

Evidence: the parser regression failed before implementation because the
installer had no replacement-policy or reviewed-plan-digest fields. It now
defaults to `preserve`, accepts only `preserve` or `full-shutdown`, requires an
exact 64-character SHA-256 for full-shutdown apply, rejects a digest on the
preserve path, and rejects combining full shutdown with the older browserless
override. Focused parser tests, Rust formatting, and patch hygiene pass.

Material blocker: no deterministic dry-run replacement plan, durable
replacement receipt, or shutdown executor exists yet. The source flag must not
be installed or exercised until those boundaries are implemented and
qualified.

Next action: add the deterministic effect inventory and durable replacement
transaction model, with red fixtures for digest drift and policy immutability.
