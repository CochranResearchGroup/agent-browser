# Plan 0148 | Runtime Host Supervisor Takeover

Date: 2026-09-01

State: CLOSED

Lane: P148

Branch: `plan/runtime-host-supervisor-takeover`

Target: `main`

Source baseline: `31a1ea9cda1fd42bbc09ac21251040123663d61d`

Design checkpoint: `360fbd6daccdef6ff43ceaac20e64b8b119738d0`

Implementation checkpoint: `aad5ce20`

Integration checkpoint: `a8091c5b`

Production qualification repairs: `040f67e8`, `46414041`

Depends on: P147 Runtime Host Ingress Supervisor Restart Repair

Integration model: merge after P147 remains present on `main`; reconcile any
shared dispatcher or documentation edits explicitly.

Authority: SOURCE DESIGN, DOCUMENTATION, PROVIDER-FREE FIXTURES, AND ISOLATED
DEVELOPMENT QUALIFICATION ARE IN SCOPE. PRODUCTION INSTALLATION, LIVE PROCESS
TERMINATION, BROWSER CLOSURE, PROFILE MUTATION, ROUTE MUTATION, TENANT EFFECTS,
AND PROVIDER EFFECTS ARE OUT OF SCOPE UNTIL A LATER EXECUTION CHECKPOINT
EXPLICITLY AUTHORIZES THEM.

## Incident

A live selected runtime host survived outside the configured systemd user
supervisor. The configured supervisor could not start because the selected host
still owned one of its fixed stream ports. The supervisor correctly returned
`port_conflict`, and P147 correctly refused to replace a live selected process.
After exact live browser lanes were closed, recovery still required an operator
to inspect the runtime census, bind the selected PID to its recorded process
identity, terminate that exact browserless host, start the user unit, and prove
that P147 selected the replacement.

The safety checks were individually correct. Their composition was missing,
which turned a recoverable ownership transition into an operator-mediated
catch-22.

## Goal

Provide one fail-closed, replay-safe supervisor takeover transaction for the
case where exactly one live, browserless, same-generation selected runtime host
blocks its configured shared-host supervisor.

The transaction must preserve all browser, profile, route, tenant, and provider
state. It may retire only the exact selected host process after stable evidence
proves that no Agent Browser-owned live browser depends on it. It then starts
the already configured supervisor and relies on P147 to adopt the replacement
atomically.

## Relationship To P147

P147 owns the dead-selected-host adoption rule in
`runtime_host_ingress.rs`. P148 does not change that rule and does not write the
ingress registry directly.

P148 owns the preceding live-host transition:

```text
live selected unsupervised host
  -> stable browserless census
  -> admission drain
  -> exact selected-host retirement
  -> configured supervisor start
  -> P147 replacement self-adoption
  -> verified steady state
```

P148 depends on P147's exact postcondition: one replacement host with the same
generation and binary digest, a different PID and socket identity, and selected
ingress bound to that replacement. If the installed candidate cannot prove the
P147 capability, takeover fails before any process signal.

## Deep Module And Interface

Introduce one deep module, `runtime_host_supervisor_takeover`, at the seam
between the session-supervisor command and the existing census, process
identity, admission, systemd, and ingress modules.

Its external interface is intentionally small:

```rust
pub(crate) fn plan_supervisor_takeover() -> Result<SupervisorTakeoverPlan, String>;

pub(crate) fn apply_supervisor_takeover(
    expected_plan_digest: &str,
) -> Result<SupervisorTakeoverOutcome, String>;

pub(crate) fn resume_supervisor_takeover(
    transaction_id: &str,
    expected_revision: u64,
) -> Result<SupervisorTakeoverOutcome, String>;
```

The plan is read-only. Apply recomputes all observations and requires the exact
plan digest before creating a durable transaction. Callers do not supply a PID,
socket path, port, browser id, generation, signal, or adoption decision.

The user-facing interface is:

```text
agent-browser session supervisor recover-host --dry-run [--json]
agent-browser session supervisor recover-host --apply \
  --expected-plan-digest <sha256> [--json]
agent-browser session supervisor recover-host --resume \
  --transaction-id <id> --expected-revision <revision> [--json]
```

`recover-host` has no session argument because the configured supervisor owns
one shared host across all lane manifests. Existing per-lane install, status,
remove, and admission commands remain unchanged.

The module may use internal seams for deterministic tests:

- runtime census and browser authority observation;
- process and listener identity observation;
- verified process termination;
- systemd unit observation and start;
- monotonic time and bounded waits.

Production and fixture adapters justify these internal seams. They are not
exposed in the CLI interface.

## Read-Only Plan Contract

`--dry-run` returns `agent-browser.runtime-host-supervisor-takeover-plan.v1`
with:

- a SHA-256 digest over every effect-relevant observation;
- selected ingress revision, boot epoch, topology, generation, binary digest,
  host id, PID, process start token, socket identity, and socket directory
  identity;
- configured supervisor unit, executable digest, lane manifests, and fixed
  ports;
- current systemd ownership and main PID;
- two adjacent runtime census digests and their accepted classifications;
- exact port-listener ownership evidence;
- active installation, ingress, admission-drain, and takeover transaction
  state;
- P147 capability evidence;
- disposition, blockers, and the next allowed effect.

Safe dispositions are:

- `already_supervised`: selected PID is the ready systemd main PID;
- `ready_for_takeover`: every effect precondition is proven;
- `blocked`: one or more typed blockers prevent apply.

Dry-run never starts, stops, enables, disables, or reloads a unit; signals a
process; closes a browser; writes a transaction; or changes admission or
ingress.

## Apply Preconditions

Apply proceeds only when all of the following remain true during a fresh
observation:

1. The supplied plan digest matches the newly computed plan.
2. Ingress selects exactly one `single_host` backend from the current boot.
3. The selected process is live and its PID, start token, executable identity,
   binary SHA-256, host id, socket identity, and socket directory match the
   selected backend and retained identity record.
4. The selected executable and every supervisor manifest name the same accepted
   installed binary digest.
5. The configured shared-host unit is loaded and enabled but its main PID is
   absent or differs from the selected PID. If it already owns the selected
   PID, apply returns `already_supervised` without effects.
6. Two adjacent census rounds are stable and every record is `idle_daemon`,
   `stale_metadata`, `external_observed`, or `manual_preserve_only`.
7. No owned, adoptable, conflicting, closing, uncertain, or insufficiently
   identified live browser exists. External and manual-preservation processes
   remain untouched.
8. Each conflicting fixed port is proven to be a listener of the exact selected
   host process. An unrelated listener blocks takeover.
9. No workstation upgrade, runtime-host ingress transaction, other supervisor
   takeover, or foreign admission drain is active.
10. The installed binary proves P147 same-generation replacement adoption.

Every precondition is rechecked after acquiring the transaction lock and again
immediately before `SIGTERM`. Identity drift, census drift, or registry revision
drift fails closed without signaling.

## Transaction And State Machine

Persist a private
`agent-browser.runtime-host-supervisor-takeover.v1` record under the existing
user-scoped transaction root. Reuse the existing transaction lock and
action-specific admission-claim mechanism; do not create a second global lock
or a parallel admission model.

```text
planned
  -> census_stable
  -> admission_draining
  -> source_retiring
  -> source_absent
  -> supervisor_starting
  -> replacement_ready
  -> ingress_adopted
  -> accepted
```

Typed non-success states include:

- `blocked_live_owned_browser`;
- `blocked_ambiguous_census`;
- `blocked_identity_changed`;
- `blocked_unrelated_port_owner`;
- `blocked_active_transaction`;
- `blocked_p147_capability_missing`;
- `source_exit_timeout`;
- `supervisor_start_failed`;
- `replacement_readiness_failed`;
- `ingress_adoption_failed`;
- `operator_recovery_required`.

Every transition is compare-and-swap fenced by transaction id and revision.
Replaying an accepted transaction returns its receipt. Replaying a failed or
incomplete transaction reports its current state and exact supported recovery
edge; it never repeats a process signal or systemd start blindly.

## Effect Sequence

1. Recompute and match the dry-run plan under the transaction lock.
2. Persist `planned`, then the two-round census and exact source identity.
3. Enter the existing global admission drain with the takeover transaction id
   and revision. Only exact takeover reconciliation commands may pass.
4. Recheck process, listener, ingress revision, census, and browser authority.
5. Bind a verified process-termination handle to the recorded selected host.
6. Send `SIGTERM` and wait for a bounded graceful exit.
7. If the exact browserless process remains live, send `SIGKILL` through the
   same verified handle and wait once more. No browser process is signaled.
8. Prove the source process and its listeners are absent.
9. Start the configured shared-host unit once. Do not install, rewrite, enable,
   or reload unit files during takeover.
10. Require one ready systemd main PID, exact manifest executable identity,
    reachable supervised lanes, and no legacy daemon.
11. Wait for P147 to select that exact replacement through its normal
    self-adoption path.
12. Prove one-host steady state, clear the admission drain, and persist an
    accepted receipt.

## Failure And Recovery Semantics

Before `source_retiring`, any failure clears this transaction's admission drain
and leaves the selected host untouched.

After `source_absent`, rollback cannot truthfully recreate the old process.
Recovery is forward-only: preserve the exact transaction and source-exit
evidence, keep ordinary mutation admission drained, and expose one bounded
resume edge that starts or re-observes the already configured supervisor. A
resume requires transaction id, expected revision, and a fresh unchanged
supervisor-manifest digest.

The implementation must never:

- retry a start after an uncertain start effect without observing systemd;
- rewrite ingress to manufacture P147 acceptance;
- restart the retired host outside systemd;
- close a browser to make the census pass;
- signal an unrelated port owner or process tree;
- delete sockets, profiles, leases, tabs, routes, or retained evidence;
- infer safety from a bind failure alone.

## Expected Write Surface

Implementation should remain concentrated in:

- new `cli/src/runtime_host_supervisor_takeover.rs` for planning, transaction,
  orchestration, and typed outcomes;
- `cli/src/session_supervisor.rs` for minimal command dispatch and the narrow
  systemd adapter;
- the existing runtime-adoption admission and census modules only where a
  reusable internal function must be exposed without changing semantics;
- `cli/src/output.rs`, `README.md`, `skills/agent-browser/SKILL.md`, and the
  relevant `docs/src/app/` page for the public command;
- one provider-free no-launch smoke fixture.

P148 should not edit P147's replacement-adoption implementation unless a
verified defect in its frozen contract is discovered. Any such defect requires
explicit overlap reconciliation before implementation continues.

## Verification Matrix

The cheapest stable seam is the takeover module interface. Provider-free tests
must cover:

1. A live exact browserless selected host produces `ready_for_takeover`.
2. A ready systemd-owned selected host returns `already_supervised` without
   effects.
3. One owned live browser blocks before admission drain or process signaling.
4. Ambiguous or changing census blocks.
5. PID reuse, start-token drift, executable drift, socket drift, boot drift, or
   ingress revision drift blocks.
6. An unrelated owner of any configured port blocks.
7. An active upgrade, ingress transaction, takeover, or foreign drain blocks.
8. A stale or incorrect plan digest blocks.
9. Exact graceful source retirement starts the supervisor once and accepts only
   after P147 adopts the replacement.
10. Exact forced retirement uses one verified process handle and never signals
    a browser process.
11. Start failure after source exit persists forward-recoverable state and does
    not retry blindly.
12. Replay at every durable state is idempotent.
13. External and manual-preservation observations are reported and untouched.
14. The no-launch fixture reproduces the original live-host `port_conflict`,
    then proves takeover reaches one supervised host with selected ingress and
    zero browser launches.

Before an implementation checkpoint, run focused Rust tests, formatting,
strict Clippy, the no-launch supervisor smoke, the P147 restart smoke, and
changed-surface validation selection through the repository Cargo safety
wrapper where applicable.

## Acceptance Criteria

- One public dry-run/apply workflow replaces the manual inspect, signal,
  systemd start, and ingress verification sequence.
- Apply can signal only one exact census-proven browserless selected host.
- A live owned browser, ambiguous evidence, unrelated listener, active
  transaction, or identity drift produces a typed zero-effect blocker.
- P147 remains the only authority that adopts the same-generation replacement
  into ingress.
- Post-source-exit failures are durable and forward-recoverable without blind
  retry.
- Accepted output proves one ready systemd runtime host, exact selected ingress,
  reachable configured lanes, zero legacy daemons, no browser launches, and a
  cleared admission drain.
- User-facing help, README, repository skill, docs site, plan, and validation
  evidence agree on the command and safety contract.

## Stop Rules

- Do not implement on P147's owned branch or rewrite its checkpoint.
- Do not start implementation until P148 is registered on the default-branch
  active-lane projection and its overlap with P147 is reconciled.
- Do not terminate a production process or install a production candidate under
  this design checkpoint.
- Do not weaken census, process identity, admission, listener ownership, or
  active-transaction fences for convenience.
- Do not add an option that accepts a raw PID, signal, socket path, port, or
  browser-closure list from the caller.
- Stop if P147 cannot remain the sole ingress-adoption authority.

## Development Qualification Record

The isolated implementation checkpoint proves the source-only workflow without
installing a production candidate or touching a production runtime:

- focused parser and takeover-module Rust tests pass;
- strict Rust Clippy with warnings denied passes;
- Rust formatting and patch hygiene pass;
- the P147 runtime-host supervisor no-launch smoke passes unchanged;
- the P148 no-launch smoke reproduces `port_conflict`, obtains a blocker-free
  plan, retires only the exact fixture host, starts the fixture supervisor once,
  observes P147 select the replacement, clears the drain, and proves a
  subsequent apply is `already_supervised` and zero-effect;
- the full docs site build and remote-view documentation checks pass;
- the workstation-install slice completed 122 of 123 tests in parallel; the
  lone failure observed another test's injected phase, and that exact test
  passed immediately with `--test-threads=1`.

Production installation and live takeover remained intentionally unexecuted at
that checkpoint.

## Production Acceptance | 2026-09-01

P148 merged to `main` at `a8091c5b`. The first production installation exposed
two fail-closed defects before takeover could signal a process. A configured
but free supervisor port was incorrectly classified as an unrelated owner;
`040f67e8` now permits a free port while continuing to block any reachable port
not owned by the selected PID. A `/proc/<pid>/fd` descriptor could also vanish
between enumeration and `readlink`; `46414041` skips only that normal
`NotFound` race and retains fail-closed behavior for every other observation
error. The focused free-versus-unrelated-port regression was red before the
repair and green after it. Formatting, strict Clippy, focused tests, and the
complete provider-free no-launch takeover smoke passed on the repaired source.

The accepted production generation is
`0.28.0-895c9e201710-c6baf23f7b65`, SHA-256
`895c9e20171089e8cf1780e72cb01254a23443b3bab485b1cbb2d80489072b42`.
The audited browserless workstation transaction
`upgrade-384e3b7f-4284-4e30-8df3-ad93d1a16e26` accepted stable census digest
`64fae707a79c3343bef9a68d4163f33105f09ac241da1cd1b0c0630231d158fb`.
It launched no browser and preserved profiles, routes, tabs, and retained
evidence.

Read-only takeover plan digest
`5f2517d220c32b4bdc91b147339e7958a96708924b086962be4223aa772d9015`
had zero blockers and no unsafe census records. Transaction
`runtime-host-takeover-1612ea37-9a98-4f73-a724-b5e7847a67a9` retired exact
browserless source PID `78976`, started the configured supervisor once, and
accepted at revision `9` only after P147 selected replacement PID `82961` at
ingress revision `219`. The receipt reports `browserLaunched=false`, the
admission drain is absent, and the source PID is gone.

Final readback proves one current executable generation, one runtime host, zero
legacy daemons, converged dashboard ingress, and
`agent-browser-runtime-host.service` active with main PID `82961`. Lane
`dashboard-service-backend` is ready and reachable on fixed port `39717`.
A fresh takeover dry-run returns `already_supervised` with zero blockers.

The separately root-owned protected lease authority was initially left healthy
on its prior reviewed SHA because sudo required an interactive password. The
operator subsequently completed the exact explicit upgrade. Its unit now names
the banked generation for SHA
`895c9e20171089e8cf1780e72cb01254a23443b3bab485b1cbb2d80489072b42`,
matching source, installed user runtime, and selected runtime host exactly. The
protected socket is enabled and active. The service is inactive between
requests as expected for socket activation. Final install doctor remains
`success=true` with no blocking issues.
