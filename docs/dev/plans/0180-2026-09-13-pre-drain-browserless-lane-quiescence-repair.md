# Plan 0180 | Pre-Drain Browserless Lane Quiescence Repair

Date: 2026-09-13

State: OPEN

Lane: P180

Product lane: PL-BUGFIX

Branch: `fix/plan-0240-predrain-quiesce`

Target: `main`

Integration: merge

Work item: [issue #95](https://github.com/CochranResearchGroup/agent-browser/issues/95)

Consumer: Books Receipts Plan 0240, issue #1, PR #3

Consolidation: required

## Objective

Make a preserving workstation upgrade able to close only proven browserless
competing lanes when the selected runtime is an older generation, without
requiring that old runtime to understand a newly introduced admission-drain
claim. Preserve every browser-bearing lane and enter admission drain only after
pre-drain quiescence has completed and the Service State revision is stable.

## Current State

Issue #95 is reproducible from the committed activation ordering. The
workstation transaction first advances to `AdmissionDraining`, writes the
admission-drain record, and later reaches browserless-lane quiescence through
`complete_runtime_transfer_phase()` and `transfer_discovered_runtimes()`.
Quiescence invokes `service status` and `close` through the selected old
binary.

Current `main` attaches the exact transaction claim to those commands and
allows claimed `close`, but this allowance was introduced by commit
`e33d34df`. The installed old generation is sourced from `0e18b351`, where
`runtime_admission_claim_matches()` does not allow `close`. The candidate
therefore supplies a valid claim to a pre-upgrade executor that cannot
recognize it. The executor returns `runtime_admission_draining` before closing
the already-proven browserless lane. This is an upgrade-compatibility
inversion, not evidence of a wrong transaction revision, wrong socket, or
browser-bearing competing lane.

The inherited candidate has been reconciled into one narrow activation seam.
Preserving upgrades now run exact browserless-lane quiescence through the
selected old binary before transition to `AdmissionDraining`. The full-shutdown
path bypasses that quiescence, and the post-drain claimed `close` exception has
been removed.

The fast source-order oracle was deterministic on two runs. Against committed
`HEAD`, it reported `quiesce_line=missing`,
`admission_draining_line=7240`, and `FAIL`. Against the inherited candidate it
reported quiescence at line 7240, admission draining at line 7245, and `PASS`.
The executable regression was first run with the old ordering and failed with
the exact `runtime_admission_draining: legacy runtime rejected close` symptom.
After changing only the seam ordering, the same test passed. A failed pre-drain
quiescence leaves both the in-memory and persisted transaction at
`StateMigrationValidated`, does not create a drain, and permits a subsequent
successful invocation. This establishes retry safety without introducing a
new durable receipt because `close` is idempotent for a proven `NotStarted`
lane and no transaction transition is committed until the complete quiescence
callback succeeds.

The first admitted compile attempt failed before compiling project code when
the optional sccache wrapper could not spawn a compiler under current host
process pressure. The successful runs kept the mandatory Cargo admission and
cgroup controls, disabled only the optional cache, used one build job, and
used a reviewed 6 GiB incremental memory claim after the initial isolated
worktree build established a roughly 4.8 GiB compiler peak.

## Diagnosis And Ranked Hypotheses

1. **Confirmed: quiescence occurs after the compatibility boundary.** The
   drain exists before the old binary receives `close`; the old binary cannot
   accept the new claimed-close exception. Moving the operation before the
   drain removes that dependency.
2. **Rejected as primary: transaction-claim revision mismatch.** The candidate
   passes the transaction id and revision used to persist the drain, while the
   installed source rejects `close` as an action before comparing those fields.
3. **Rejected by current evidence: wrong runtime-host socket.** The observed
   request reached the selected old host and returned its admission error. The
   repair must still retain exact selected-backend and socket equality checks.
4. **Rejected by current evidence: the lane owns a browser.** Exact status
   reported `dashboard-service-backend` as `NotStarted`. Missing or any other
   health state must continue to fail closed.

## Consolidated Batch

This batch contains one invariant and its complete proof:

- discover the exact cooperative-transfer primary session set from the sealed
  transaction and current Service State;
- while no admission drain exists, inspect every other supervised lane through
  the selected old runtime and close only lanes whose exact browser health is
  `NotStarted`;
- prove the Service State revision is quiet after those closes;
- revalidate the source runtime binding needed by the prepared transaction;
- only then transition to `AdmissionDraining`, persist the drain, and begin
  cooperative handoff; and
- remove claimed `close` from the admission exception surface because the
  repaired sequence no longer needs it.

The batch must also define restart behavior for interruption after a lane is
closed but before admission drain is committed. A preserving retry must either
recognize the already-quiesced terminal lane or safely re-establish its
browserless status. It must not manufacture a second lane, broaden process
cleanup, or treat absence alone as ownership proof.

## Scope

- `cli/src/workstation_install.rs` transaction activation, quiescence, and
  focused tests.
- `cli/src/runtime_adoption.rs` least-privilege admission rules and focused
  tests.
- Transactional restart or resume state only where required to make the
  pre-drain close boundary idempotent and reviewable.
- Plan, runbook, active-lane, and issue evidence needed for traceability.

## Non-Goals

- No general runtime-admission redesign.
- No extension of the admission timeout or Service State lock timeout.
- No browser-bearing lane closure, unknown-process cleanup, force unlock, or
  supervisor-wide shutdown.
- No change to full-shutdown replacement semantics.
- No BILL navigation, authentication, credential, transaction, QBO, or
  accounting effect.
- No Turnstile, presentation-provider, profile-reset, or unrelated runtime
  repair.
- No formal release and no repeated full build during implementation.

## Repair Design

1. Add a stable orchestration seam that exposes the ordering of pre-drain
   quiescence, drain persistence, and runtime transfer without launching a
   browser or replacing the installed runtime.
2. Determine whether the prepared transaction requires cooperative transfer
   before changing its state. Skip pre-drain quiescence for the reviewed
   full-shutdown path and isolated roots.
3. Resolve all cooperative-transfer primary sessions and exclude the complete
   set from competing-lane closure. Fail on changed socket identity, ambiguous
   source binding, or missing current source evidence.
4. Inspect and close competing lanes with the selected old binary while the
   drain file is absent. Do not attach or require an admission claim for these
   pre-drain commands.
5. Require `browser_health=NotStarted` before each close and an explicit
   successful close receipt. Preserve the current fail-closed treatment of
   missing and browser-bearing health.
6. Require the existing stable Service State revision window after quiescence.
   Revalidate any prepared source or migration evidence invalidated by those
   state changes before writing the drain.
7. Define and test interruption recovery at the boundary. Prefer a durable,
   transaction-owned pre-drain receipt when repeated close is not inherently
   idempotent; do not infer completion from a missing socket or process.
8. Transition to `AdmissionDraining` and persist its record only after the
   preceding conditions pass. Existing runtime-handoff admission exceptions
   continue to protect prepare, resume, finalize, abort, and rollback.
9. Remove the claimed `close` exception and its command wrapper if no remaining
   caller requires it. A `close` presented after drain must remain denied even
   when it carries the transaction claim.

## Test Plan

### Red-Green Regression

Add one deterministic, provider-free test at the activation seam using a
scripted old-runtime adapter whose behavior matches source `0e18b351`:

- `service status` returns exact `NotStarted` health;
- `close` succeeds only while the drain is absent;
- ordinary effects are denied after the drain appears; and
- handoff lifecycle commands remain available after drain.

The test must fail on committed `HEAD` with the exact
`runtime_admission_draining` close symptom and pass only when close completes
before the transition and drain write. A source-order assertion may remain as
a cheap secondary guard, but it is not sufficient as the regression oracle.

### Focused Invariants

- one browserless competing lane closes before drain;
- claimed `close` is denied after drain;
- missing health evidence fails before drain with no close;
- browser-bearing health fails before drain with no close;
- every cooperative-transfer primary session is excluded;
- a changed selected runtime-host socket fails closed;
- no cooperative migration performs no quiescence;
- full shutdown and isolated-root paths preserve their existing behavior;
- interruption immediately before and after close has a deterministic,
  preserving resume result;
- Service State revision movement after close blocks drain entry; and
- successful quiescence proceeds into the existing handoff path without
  weakening abort, rollback, or preservation behavior.

Run focused tests through:

```text
scripts/ci/rust-tests.sh --focused <exact-test-filter>
```

Use separate filters while iterating. Do not run the comprehensive suite after
each edit.

### Completed-Batch Gates

Because the batch changes Rust source under `cli/src/`, run once against the
frozen final source candidate:

```text
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
pnpm validation:select -- --base 17b8925e240204c215770c5c372f4c0459661c27
scripts/ci/rust-tests.sh
```

Preserve the first failure. Do not use reruns to erase a flaky or
resource-pressure result. CI gets one lazy status readback after push unless
the maintainer separately requests active CI evaluation.

## Delivery Sequence And Budget

1. **Regression seam and baseline, 30 minutes.** Replace the source-order-only
   oracle with a scripted old-runtime activation test and observe the exact
   failure on committed behavior.
2. **Repair and focused proof, 60 minutes.** Reconcile the inherited patch,
   implement pre-drain sequencing and restart safety, and run only affected
   filters.
3. **Batch validation and review, 45 to 90 minutes.** Freeze the source
   candidate, run formatting, clippy, changed-surface selection, and the
   required Rust lane once. Host admission waiting does not justify bypassing
   the wrapper or starting another build.
4. **Integration, 30 minutes excluding CI queue time.** Publish one coherent
   checkpoint, open the issue-linked PR, perform one lazy CI readback, merge
   only after required review and checks, and verify exact ancestry.
5. **Installed acceptance, separate governed window.** Build one integrated
   candidate, run preserving dry-run and preflight, perform at most one
   transactional apply, then verify supervisor identity, selected generation,
   runtime multiplicity, Service State stability, drain removal, transaction
   terminality, and cleanup obligations. Do not build before the source batch
   is merged and frozen.

The source batch has a three-hour active-work ceiling before replan. Stop the
current tactic after two consecutive checkpoints or 30 active minutes without
outcome progress. The installed window permits one build and one apply; a new
source defect returns to source repair rather than consuming another apply.

## Worker Assignments

- **Primary owner:** one Agent Browser maintainer owns diagnosis, source,
  validation selection, integration, and the final acceptance claim.
- **Review owner:** one bounded reviewer inspects the frozen diff and the
  red-green test after the source checkpoint is published. The reviewer does
  not edit the branch or operate the runtime.
- **Runtime owner:** the primary retains exclusive production runtime custody
  during the separately governed install window and explicitly releases it to
  Books Receipts afterward.
- **Books Receipts:** remains a downstream observer until release. It does not
  install, retry R1, or alter Agent Browser runtime state during this batch.

No parallel implementation worker is assigned because both changed Rust files
form one ordering invariant and already contain uncommitted work.

## Implementation Evidence

The frozen source candidate has the following local evidence:

- red: `preserving_upgrade_quiesces_legacy_runtime_before_admission_drain`
  failed on the deliberately retained old ordering with
  `runtime_admission_draining: legacy runtime rejected close`;
- green: the same regression passed after quiescence moved ahead of drain;
- `failed_pre_drain_quiescence_leaves_preserving_upgrade_retryable` passed;
- `full_shutdown_enters_admission_drain_without_preserving_quiescence` passed;
- the admission-drain test passed with an exact claimed `close` explicitly
  denied;
- all five `shared_runtime_host_quiesce` tests passed;
- all 159 `workstation_install` tests passed serially;
- Rust formatting and workspace clippy with warnings denied passed; and
- `git diff --check` and validation selection passed; and
- the comprehensive provider-free Rust runner passed both native and support
  lanes in 1,215 seconds, including CLI core, CDP transport, CLI integration,
  and the production-scale Service State performance compartment.

The validation selector also named workstation shell, VM, Guacamole asset,
PostgreSQL, and route-user fixtures solely because the shared installer Rust
module changed. This batch does not change those scripts, assets,
provisioning, database, or route-user surfaces, so those path-based suggestions
are not part of the focused source gate.

## Evidence And Exit

The source repair is integration-ready only when:

- the regression test fails on committed ordering with the issue #95 symptom
  and passes on the frozen candidate;
- the candidate does not depend on an old runtime understanding claimed
  `close`;
- interruption and resume behavior at the new pre-drain boundary is proved;
- missing, browser-bearing, and changed-source evidence all fail before drain;
- every required changed-surface gate passes with no hidden retry; and
- the branch has a clean published checkpoint linked to issue #95.

Installed acceptance additionally requires one integrated candidate whose
transaction reaches an accepted terminal state, selects the new generation,
leaves no admission drain or active convergence window, reports one coherent
runtime host and supervisor identity, preserves retained browsers, and leaves
no unresolved cleanup obligation. That receipt may unblock Books Receipts Plan
0240 R1, but it does not itself prove BILL authentication or authorize any
accounting mutation.

Stop and preserve evidence if the old runtime cannot safely close a proven
browserless lane before drain, if pre-drain interruption cannot be made
idempotent without a broader state-machine change, if a browser-bearing or
unproven lane would be affected, or if one installed apply does not reach a
coherent terminal state.
