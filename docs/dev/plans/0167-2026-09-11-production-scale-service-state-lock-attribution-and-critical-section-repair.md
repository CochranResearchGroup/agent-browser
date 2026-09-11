# Plan 0167: Production-Scale Service State Lock Attribution and Critical-Section Repair

Date: 2026-09-11

State: CLOSED

Consolidation: required

Lane: P167 future maintenance

Branch: `maintenance/plan-0167-service-state-lock`

Target: `main`

Integration: short-lived branch through the protected `main` workflow

Corrective successor to: [Plan 0142](0142-2026-08-29-service-state-concurrency-and-client-recourse-reliability-plan.md)

Related acceptance: [Plan 0142 final acceptance](../notes/0147-2026-08-29-plan-0142-final-acceptance.md)

Related program: [Plan 0160](0160-2026-09-06-production-profile-identity-and-operational-readiness.md), A1 and AX

Current execution status: [RUNBOOK.md](../../../RUNBOOK.md)

Integration receipt: PR 29, merge
`14fb3db060d8fb644665b0181ac8f2860b0af3f2`

## Objective and authority

Make production-scale Service State contention attributable across processes,
reproduce it at or above the observed state size, and reduce the exclusive
file-lock interval enough that the ordinary one-second deadline remains usable
without weakening crash safety, revision fencing, effect attribution, or live
owner protection.

This is a future-execution maintenance plan. It authorizes repository analysis,
provider-free synthetic fixtures, source and contract changes, documentation,
and isolated development-runtime validation. It does not authorize a retry of
the failed X evaluation, a new browser or profile, production installation,
production Service State mutation, service restart, process termination, broad
cleanup, provider or tenant effects, or release publication. The completed
RuFresh no-result remains valid and outside this plan.

P167 owns the canonical worktree for this execution. Its active-lane projection
records the paused P157 public-contract overlap; P167 must reconcile that branch
before editing overlapping public surfaces. No other lane may edit P167's
Service State implementation while this custody remains active.

## Current State

Execution opened from `origin/main` at
`7cfc1fd2c31a394eca5c378f6fa3b26d23b96b81` on branch
`maintenance/plan-0167-service-state-lock`. The active Plan 0165 worktree and
the unique P0240 worktree are clean and remain outside this plan's custody.

The installed incident source revision
`8070505d6462da4d802359e4adf42f4b4a8f118e` and current `main` have no diff in
`cli/src/native/service_store.rs`. The relevant current behavior is therefore
still present on the target branch:

- `DEFAULT_SERVICE_STATE_LOCK_TIMEOUT` is one second;
- a mutation takes the exclusive cross-process file lock before loading state
  and retains it through the mutator, transaction preparation, serialization,
  and durable save;
- process-local lock diagnostics retain 32 recent activities and do not identify
  a holder in another process;
- the realistic mixed-burst fixture uses threads in one process and requires
  only 2.9 MB, while the last accepted receipt measured about 3.15 MB; and
- the independent-process test proves only that a raw held lock causes a typed
  timeout before mutation. It does not run competing production-scale
  transactions or prove holder attribution.

The 2026-09-11 Last30days incident adds materially different evidence:

- the failed X evaluation waited 1,002 ms for the file lock;
- the immutable failure journal contains 15 comparable timeouts across 10
  actions and 10 installed builds, so an X-specific defect is not the leading
  explanation;
- the observed production Service State was 6,536,547 bytes with 200 retained
  jobs, 261 sessions, 91 profiles, and revision 71,959;
- the existing 3,258,955-byte threaded fixture passed in 542 ms with a 250 ms
  maximum observed hold, so it does not exercise the production-scale,
  multi-process boundary;
- the historical holder is unavailable because current diagnostics are
  process-local and bounded, and the original job was evicted; and
- inverted job timestamps make the apparent overlap with another X evaluation
  unreliable as causal evidence.

The source evidence supports a shared Service State persistence and coordination
bottleneck. It does not establish which process or action held the incident lock,
which phase dominated its hold, or which architecture change is the minimum
sufficient repair. Increasing the timeout alone is a mitigation and cannot close
this plan.

The detailed incident intake currently exists as an uncommitted change in the
sibling Last30days repository at
`docs/dev/notes/0113-2026-09-11-plan0066-post-reinstall-tick.md`. P167 must treat
that locator as pending foreign custody until Last30days preserves it on its own
branch. P167 must not edit, commit, merge, or discard that change.

## Corrective relationship to Plan 0142

Plan 0142 remains historically closed. Its acceptance was valid for the frozen
approximately 3.15 MB, single-process mixed burst and the separate raw
cross-process lock-holder test. The new incident invalidates any broader claim
that those fixtures prove production-scale cross-process operability or durable
holder attribution.

P167 narrows the unresolved part of Plan 0142 rather than repeating its client
recourse and lifecycle work. Existing structured timeout, effect-state, retry,
revision-CAS, transaction recovery, and no-duplicate-effect contracts are
preserved unless a red regression proves a specific defect in one of them.

## Consolidated batch

The batch contains four joined outcomes:

1. durable, privacy-safe cross-process holder and phase attribution that does
   not depend on the contested Service State lock;
2. a synthetic, provider-free, production-scale multi-process regression that
   fails on the frozen baseline for the intended contention reason;
3. a measured critical-section repair selected from phase evidence; and
4. aligned status, doctor, client, documentation, and validation surfaces for
   the implemented diagnostic and repair contract.

Do not split telemetry into a documentation-only completion or close the plan
after the regression becomes green through a longer timeout. The usable outcome
is attributable contention plus a demonstrated reduction of the bottleneck at
the real cross-process persistence boundary.

## Non-goals

- Do not identify a holder from job timestamp overlap, profile name, action
  proximity, or a stale process ID alone.
- Do not copy production Service State, provider data, page content, profile
  paths, capabilities, URLs, credentials, or tenant payloads into fixtures or
  tracked diagnostics.
- Do not force-unlock a file, delete a lock file, terminate a suspected holder,
  or rewrite retained state as a repair.
- Do not make blind retry, duplicate launch, replacement profile creation, or
  automatic X replay safe by policy.
- Do not weaken revision comparison, transaction recovery, unknown-field
  preservation, mixed-version compatibility, or crash-safe commit ordering.
- Do not convert the entire Service State repository to a new database without
  a separately justified architecture decision and plan revision.
- Do not reopen completed RuFresh research or reinterpret its no-result.

## Frozen invariants

1. A contender that fails before mutation remains provably `no_effect`.
2. A timeout after an external effect remains `effect_uncertain` until exact
   current-state inspection resolves it.
3. One request identity cannot create duplicate browser, tab, route, recovery,
   or provider effects.
4. Cross-process file locking remains authoritative for standalone CLI, daemon,
   and service-host writers.
5. Holder telemetry never becomes lock authority and never justifies cleanup or
   takeover by itself.
6. Process identity requires a PID plus a start-identity or equivalent anti-reuse
   token. A PID alone is insufficient.
7. Missing, stale, corrupt, or racy holder telemetry is reported as unknown. It
   is never converted into a guessed holder.
8. Diagnostic persistence does not acquire the contested Service State lock and
   cannot recursively create the same bottleneck.
9. Diagnostic records exclude private payloads and operator handoff secrets.
10. A stale prepared transaction fails before commit and is never replayed by an
    adapter-owned retry loop.
11. Successful competing mutations preserve monotonic revisions, unknown fields,
    and exactly-once logical effects.
12. The ordinary one-second deadline is not increased to make acceptance pass.

## Delivery sequence and budget

Execution follows one evidence-first critical path. Attribution and the red
production-scale fixture precede repair selection; public-surface alignment and
final qualification operate only on the frozen candidate.

### P167-A: Preserve and freeze the incident contract

Budget: 30 minutes.

- Verify the exact target commit, active lanes, worktrees, and source-to-installed
  identity before opening implementation custody.
- Preserve a redacted incident aggregate or durable cross-repo locator without
  copying private runtime state into this repository.
- Freeze baseline commands, fixture topology, size, record shape, expected red
  result, and quantitative gates.
- Confirm that the existing Plan 0142 client recourse remains valid and identify
  only the acceptance claims superseded by this incident.

Exit: one reproducible contract identifies the missing holder evidence and the
production-scale cross-process gap without rerunning X.

### P167-B: Add durable cross-process attribution

Budget: 60 minutes.

Add a bounded diagnostic surface outside the main Service State transaction.
The exact representation is an implementation decision, but it must support:

- a unique acquisition token;
- process ID plus process start identity, executable generation, operation,
  lock mode, and current phase;
- acquisition wall time plus process-local monotonic wait and hold durations;
- coarse state bytes, revision, and record counts when already available;
- load, mutation, serialization, commit, and finalization timing boundaries;
- released, timed-out, crashed-or-stale, and unknown dispositions; and
- a timeout snapshot of the observable holder record.

A current-holder sidecar may be written atomically only after the file lock is
acquired and must be cleared or terminally marked before release. A contender
must validate process start identity before calling a retained record current.
The acquisition-to-sidecar race and crash residue must yield typed unknown or
stale results. Any append-only incident journal must be independently bounded
and rotated without using the Service State lock.

Exit: two independent processes can attribute an intentional hold to the exact
safe holder identity and phase, while stale, crashed, and race-window cases fail
closed to unknown.

### P167-C: Build the production-scale multi-process regression

Budget: 45 minutes.

- Generate synthetic state at or above 8 MiB, providing at least 20 percent
  headroom over the observed 6,536,547-byte state.
- Preserve the observed high-level record mix with at least 200 jobs, 261
  sessions, and 91 profiles, using synthetic identifiers and payloads only.
- Use independent operating-system processes and the real
  `LockedServiceStateRepository` transaction path. Threads alone are
  insufficient.
- Coordinate readiness with explicit barriers or files, not arbitrary startup
  sleeps.
- Include at least one writer, one competing writer, and concurrent snapshot
  readers. A test-only phase barrier may lengthen one real transaction phase to
  make the baseline failure deterministic, but a raw lock-only helper is not the
  regression oracle.
- Prove baseline failure is the expected file-lock deadline, captures the holder
  phase, performs no contender mutation, and leaves no fixture residue.
- After repair, prove zero timeouts, zero duplicate logical effects, monotonic
  revisions, valid JSON, unknown-field preservation, and clean process exit.

Exit: the baseline fails for the intended production-scale cross-process
critical-section reason and the candidate passes without extending the timeout.

### P167-D: Reduce or coordinate the measured critical section

Budget: 75 minutes.

Choose the smallest repair justified by P167-B and P167-C measurements:

- if serialization or pure derivation dominates, prepare the candidate payload
  outside the exclusive file lock and perform a revision-fenced commit;
- if load dominates, introduce a revision-safe snapshot or bounded state
  partition that cannot expose partial transactions;
- if retained job or event churn dominates, split only that ledger behind an
  explicit schema, projection, recovery, and rollback contract; or
- if admission rather than persistence dominates, route service-host mutations
  through one explicit writer coordinator while retaining the cross-process
  file lock as the standalone and crash-recovery authority.

The file-lock critical section is already serialized. Therefore “serialize the
critical section” is not a sufficient design. The repair must reduce time under
exclusive ownership or move waiting into one observable coordinator with a
bounded queue and truthful cancellation semantics.

Do not move a mutator outside the lock until source inspection proves it is pure
and revision-replay safe. When that cannot be proven, preserve the mutator inside
the transaction and optimize a narrower measured phase.

Exit: the production-scale regression and transaction safety fixtures pass on
the same frozen source candidate.

### P167-E: Align public surfaces and qualify the batch

Budget: 30 minutes.

If holder or phase diagnostics are user-visible, update every required feature
surface in the same slice: `cli/src/output.rs`, `README.md`,
`skills/agent-browser/SKILL.md`, relevant `docs/src/app/` pages, generated client
or service contracts, and inline source documentation. MDX tables must use HTML.

Run focused checks during repair, then select and run the complete changed-surface
gates once on the frozen candidate. Build at most one development candidate when
installed behavior is necessary to prove the diagnostic boundary. Production
installation and any X replay remain separate gates.

Exit: source, contracts, docs, and provider-free acceptance agree, with exact
unexecuted production gates recorded.

The complete batch has a 240-minute active-work ceiling. It permits two attempts
per work unit, one review and rework cycle, one production-scale fixture design,
and at most one development candidate build. Reassess after two consecutive
checkpoints or 30 active minutes without outcome progress. Exceeding a bound
requires a split, changed tactic, or explicit incomplete closeout. It does not
authorize a longer timeout, another live retry, or a renamed continuation.

## Worker assignments

One primary executor owns the critical path: incident contract, fixture design,
telemetry semantics, repair selection, integration, and final acceptance. No
parallel writer is assigned by default because the telemetry and persistence
changes overlap in `cli/src/native/service_store.rs`.

After P167-A freezes the contract, a separate worker may inspect public contract
and documentation surfaces without editing the Service State implementation.
After the candidate freezes, one fresh-context reviewer may evaluate only:

- cross-process attribution truth and privacy;
- effect-state and retry preservation;
- revision, crash, and exactly-once safety; and
- whether the regression exercises the real production-scale boundary.

The primary adjudicates findings and performs any accepted repair. No worker or
reviewer has production, provider, cleanup, or retry authority.

## Quantitative gates

- Synthetic fixture size: at least 8 MiB.
- Record shape: at least 200 jobs, 261 sessions, and 91 profiles.
- Processes: at least two competing writers plus concurrent readers.
- Ordinary file-lock deadline: unchanged at one second.
- Candidate contention result: zero lock timeouts at the frozen target.
- Logical effects: zero duplicates and zero lost committed mutations.
- State integrity: monotonic revision, valid transaction recovery, valid JSON,
  and unknown-field preservation.
- Attribution: every intentionally held timeout identifies the safe holder and
  phase, or returns an explicit race, stale, or unknown reason.
- Diagnostic privacy: zero capabilities, URLs, profile paths, page content,
  provider payloads, credentials, or tenant identifiers.
- Residue: zero fixture processes, lock sidecars, temporary state, or diagnostic
  files after the test cleanup check.
- Performance: freeze p50, p95, and maximum wait and hold thresholds from the
  baseline before optimization. The accepted maximum must remain below the
  one-second deadline with stated safety headroom. If a stable threshold cannot
  be justified, the plan remains open.

## Required validation

The executor must begin with existing-coverage discovery and a red baseline.
All Cargo commands on WSL run through `scripts/ci/cargo-safe.sh`.

Minimum focused families:

```bash
scripts/ci/rust-tests.sh --focused service_store
scripts/ci/rust-tests.sh --compartment cli-integration
pnpm validation:select -- --base <batch-baseline>
```

If Rust source changes, the final frozen batch also requires:

```bash
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
```

Add focused service-contract, generated-client, status, doctor, dashboard, docs,
and skill checks whenever those surfaces change. Record the exact tier,
selection, processes, fixture bytes, record counts, waits, holds, phase timings,
retries, exclusions, and first failure. A passing small threaded fixture cannot
substitute for the required multi-process production-scale gate.

## Evidence and exit

| Requirement | Required evidence | Exit condition |
| --- | --- | --- |
| Incident custody | Durable redacted Last30days locator or source-backed aggregate | The incident is preserved without importing private state or editing foreign custody |
| Baseline attribution | Independent-process red test on the frozen baseline | The contender reports the exact observable holder and phase, or a typed reason attribution is unavailable |
| Production scale | Synthetic fixture receipt at or above 8 MiB with the frozen record mix | The fixture uses real repository transactions across processes and reproduces the target contention boundary |
| Critical-section repair | Before and after phase timings on identical fixture and source identities | The candidate meets the ordinary deadline without a timeout increase |
| Transaction safety | Revision, stale candidate, crash boundary, rollback, recovery, mixed-version, and unknown-field fixtures | No corruption, partial commit, lost update, duplicate logical effect, or silent replay occurs |
| Diagnostic safety | Schema and projection tests plus content scan | Holder telemetry is bounded, cross-process useful, fail-closed, and free of private or capability-bearing data |
| Public parity | Required CLI, HTTP, MCP, generated client, dashboard, help, README, docs, skill, and inline checks for touched surfaces | Every maintained surface describes the same implemented behavior and safe recourse |
| Frozen source | Validation selector, focused suites, format, strict Clippy, and changed contract checks | Every touched surface since the batch baseline passes once on the final candidate |
| Isolated installed behavior | Development manifest, doctor, exact contention smoke, browser-launch smoke when applicable, and fresh process census | The development identity is healthy and no fixture residue remains |
| Scope preservation | Git and runtime postflight | No X or RuFresh replay, production install, provider effect, profile replacement, force unlock, process kill, or broad cleanup occurred |

Plan 0167 closes only when durable cross-process attribution and the
production-scale regression both pass on the same integrated repair, all
required changed-surface gates are green, and the exact remaining production
installation and consumer replay gates are recorded as incomplete or separately
authorized. Source completion does not prove that X succeeds, and a later X
retry does not belong to this plan.

Plan 0167 completed on 2026-09-11 through PR 29. The integrated source adds
validated cross-process holder attribution, revision-fenced prepared commits,
one repository-owned stale replay, unknown-field preservation, and a
9,642,672-byte two-writer and two-reader regression. The qualified candidate
completed in 709 to 739 ms across six samples with zero commit wait and 358 to
380 ms exclusive holds while retaining the ordinary one-second deadline. The
complete low-pressure Rust suite, selected focused families, public contract
checks, documentation build, and workstation fixtures passed. Production
installation, shared skill publication, installed doctor, and X evaluation
remain separate unauthorized gates; RuFresh was not repeated.
