# Plan 0190 | Advisory Candidate Build And Promotion Orchestrator

Date: 2026-09-14

Plan version: 1

State: PLANNED

Lane: P190

Product lane: PL-PLATFORM

Planning branch: `platform/p190-advisory-candidate-orchestrator-plan`

Implementation branch: `platform/p190-advisory-candidate-orchestrator`

Target: `main`

Integration: merge

Work item: [issue #136](https://github.com/CochranResearchGroup/agent-browser/issues/136)

Source baseline: `d101eb1516a26a8ae40821c7da69bba77e43f78d`

Consolidation: required

## Objective

Build one deterministic advisory surface that explains current candidate,
build, install, and recovery state; recommends a next action; presents every
safe supported alternative; and executes the operator's selected transition
through the existing workstation transaction. Prevent concurrent corrupting
commits without turning the tool into a permission authority or forcing a
second equivalent production build after merge.

## Current State

The repository already has the durable mechanisms the orchestrator must use:
sealed workstation generations, binary and support-manifest digest validation,
resumable `UpgradeTransaction` states, runtime-replacement planning, final
doctor verification, development candidate publication, and Cargo admission.
The missing layer is a common candidate identity and advisory state machine
that lets independent sessions discover, join, queue, cancel, discard,
supersede, install, recover, or roll back without racing or rebuilding by
default.

The Plan 0186 installer collision showed that a process lock can serialize two
commands while still leaving their intent and candidate ownership ambiguous.
Policies 0051 and 0052 now treat coordination records as integrity mechanisms,
not agent-role permissions. Issue #136 tracks implementation. This planning
slice changes no executable, creates no implementation worktree, and performs
no build, install, supervisor, browser, profile, provider, or tenant effect.
Implementation admission waits for the current worktree and active-lane drift
to be reconciled.

## Frozen Decisions

- The user retains authority to start, cancel, discard, install, supersede,
  recover, or roll back. An issue, plan, branch, lease, agent role, or tool
  recommendation does not grant or revoke that authority.
- The tool is advisory at the workflow boundary and mandatory only at the
  integrity boundary. It may fence a stale writer or reject an internally
  inconsistent commit, but it must explain the named invariant and supported
  recovery choices.
- There is no permanent coordinator agent. Any session acting within current
  user authority may inspect state and request a supported transition. The
  active operation owns temporary execution custody.
- Extend the existing workstation install and runtime-replacement transaction.
  Do not create a second installer, candidate store, or runtime state machine.
- Build once and reuse the sealed artifact after integration when its complete
  executable-input closure is unchanged. A merge commit identifier or docs-only
  change is not by itself a rebuild reason.
- Keep ordinary iteration on `pnpm build:development-candidate`. Use a full
  production candidate build only at qualification or when the input digest
  proves a rebuild necessary.

## Consolidated Batch

### Candidate identity kernel

Add a focused `agent-browser-candidate` Rust crate that owns pure manifest,
input-closure digest, advisory decision, and state-transition logic. Its
manifest records schema version, candidate ID, source commit and tree,
executable-input digest, target, toolchain, Cargo profile, features, reviewed
environment-input digest, binary digest, support-manifest digest, creation
time, and validation receipt locators. It stores no credentials, browser data,
tenant payloads, or raw environment secrets.

The first implementation packet must inventory the actual build dependency
closure before freezing the digest contract. At minimum it evaluates Rust
workspace sources and manifests, `Cargo.lock`, build scripts, embedded assets,
package-version inputs, target, toolchain, profile, features, and an explicit
allowlist of build-affecting environment values. Documentation and merge
metadata stay outside the closure unless the build demonstrably embeds them.

Deduplicate an active build by that input digest plus target, toolchain,
profile, features, and reviewed environment digest. An equivalent request joins
or observes the same build and receives its sealed artifact. Distinct admitted
builds use isolated outputs and the existing Cargo resource-admission wrapper;
they do not compete for one mutable target or hold the runtime install record.

### Existing transaction adapter

Keep operating-system and runtime effects in the current CLI and native
workstation modules. The adapter reuses `StagedWorkstationGeneration`,
`validate_install_transaction_candidate()`, `UpgradeTransaction`,
`resume_install_transaction()`, `plan_runtime_replacement()`, and
`verify_final_doctors()`. The pure crate proposes transitions; the adapter
performs them with the current install, supervisor, Service State, and doctor
contracts.

Every mutating phase submits the exact operation ID, revision, and fencing
generation. At most one install or recovery transaction may commit for a
target environment. A same-candidate request joins, observes, or resumes the
existing transaction. A competing candidate reports the active operation and
offers queue, wait, cancel, discard, or transactional supersede when supported.

### Advisory command and result surface

Plan a public command family equivalent to:

- `agent-browser candidate inspect|list|status`
- `agent-browser candidate build`
- `agent-browser candidate cancel|discard`
- `agent-browser candidate install|supersede|rollback`

Effectful transitions require an explicit `--apply`; read-only inspection is
the default. Final command names must follow the repository's documentation
parity contract before implementation merges.

Machine-readable output includes `observedState`, `recommendation`,
`alternatives`, `consequences`, `integrityPreconditions`, active operation and
candidate identities, artifact reuse eligibility, rebuild reasons, and receipt
locators. Use typed results including `already_applied`, `joined_existing`,
`queued`, `rebuild_required`, `superseded`, `recovery_required`, and
`integrity_precondition_failed`. Reserve `authorization_required` for genuine
absence of user authority. Do not emit generic `permission_denied` for normal
contention, drift, or a non-recommended operator choice.

### Jam resistance and recovery

- Separate long-lived logical transactions from short physical locks. Never
  hold repository, installer, supervisor, or Service State locks during Cargo,
  CI, upload, network waits, browser convergence, or large serialization.
- Publish and test one lock order: coordination transaction, install
  transaction, supervisor or runtime mutation, then Service State commit.
- Queued requests hold no committing lease or file lock. Revalidate their
  candidate, authority context, dependencies, and runtime state when selected.
- Require durable progress evidence for renewal. Bound stalled operations and
  expose exact last phase, timestamp, process evidence, and recovery choices.
- Fence stale writers after cancellation, supersede, or recovery. A former
  process cannot commit against a later generation even if it resumes.
- Pin active and queued artifacts against garbage collection. Release the pin
  only after install, discard, supersede, rollback disposition, or terminal
  failure is durably recorded.
- Make request and transition identifiers idempotent. Duplicate requests return
  the current or terminal receipt instead of starting another operation.
- Append request, recommendation, selected action, transition, and final
  readback receipts without secrets or tenant data.

## Non-Goals

- No permanent coordinator service, agent permission registry, generalized
  policy enforcement engine, or replacement for current user and goal
  authority.
- No second workstation installer, supervisor, Service State owner, artifact
  store, or independent recovery graph.
- No automatic production install, release, browser cleanup, profile reset,
  tenant action, provider mutation, or retry of an unrelated failed workflow.
- No absorption of the P181 lease-authority kernel or its branch. Reuse shared
  concepts only through an explicit dependency review after current custody is
  reconciled.
- No broad CI replay during early packets and no rebuild merely to obtain a
  different commit identifier.

## Delivery Sequence And Budget

1. Inventory executable inputs and current candidate, transaction, supervisor,
   and doctor seams. Freeze manifest and advisory-result fixtures without a
   build.
2. Implement the pure candidate crate with digest, equivalence, idempotency,
   advice, and state-transition tests.
3. Seal development and production candidate artifacts through the existing
   build entry points. Prove tamper detection and artifact pinning.
4. Add read-only CLI inspection and status adapters, then align output, README,
   repository skill, docs site, and inline documentation.
5. Add explicit effect transitions over the existing workstation transaction,
   with compare-and-swap, fencing, join, queue, cancellation, supersede,
   recovery, and rollback receipts.
6. Run concurrency, crash, timeout, lock-duration, garbage-collection, and
   doctor-readback acceptance. Qualify one production artifact and prove
   post-merge executable-input equivalence before reusing it.
7. Perform any production install only as a separately user-directed operation
   after source integration and final state readback.

Budget the implementation as six source packets plus the separate operational
gate. Permit one development-candidate baseline build and one production
candidate build. A second production build requires a recorded changed-input,
missing-artifact, or verification-failure reason. Use focused provider-free
tests per packet, one final selected validation set, and at most one bounded
repair cycle before revising the plan. Do not start broad CI merely to observe
progress.

## Worker Assignments

This planning turn assigns no implementation worker and creates no worktree.
When admitted, one PL-PLATFORM lane owner holds the implementation branch and
integrates the complete batch. Bounded review or test specialists may work in
disposable directories or the lane checkout; they do not receive additional
durable primary worktrees. Shared CLI, workstation, policy, roadmap, and
generated-client surfaces have one active writer per transition.

## Test Plan

Provider-free fixtures must cover:

- identical and changed executable-input closures across a merge;
- docs-only and merge-metadata changes that preserve artifact reuse;
- missing, malformed, stale, or tampered candidate manifests and artifacts;
- duplicate same-candidate requests joining one operation;
- duplicate same-input build requests producing one sealed artifact;
- distinct admitted builds using isolated outputs and Cargo resource claims;
- different-candidate wait, queue, cancel, discard, and supersede choices;
- crash or timeout before effect, between commits, after runtime replacement,
  and before final doctor acceptance;
- a stale writer attempting to commit after a fencing-generation change;
- queue revalidation, bounded progress, fairness, and idempotent replay;
- active and queued artifact retention plus terminal garbage collection;
- lock-order and maximum physical-lock-hold assertions;
- exact transaction resume, rollback, supervisor, listener, generation, and
  final doctor readback through existing workstation fixtures; and
- help, README, repository skill, docs site, service schema, and generated
  client parity for every public surface actually introduced.

Use `pnpm validation:select -- --base <batch-baseline>` after each coherent
batch. Any Rust source change requires repository format and strict workspace
Clippy at batch completion. Run focused crate and workstation fixtures during
implementation. Run the comprehensive provider-free Rust lane once at the
final source gate only if selected by policy or explicitly required for merge.
Provider-backed or production acceptance remains a separate authorization and
evidence boundary.

## Evidence And Exit

Plan 0190 is implementation-complete only when:

- the pure candidate identity and advice crate has complete focused tests;
- the CLI uses the existing workstation transaction and does not introduce a
  competing installer, state store, or supervisor owner;
- two simultaneous same-candidate requests converge on one operation;
- competing candidates produce safe supported choices and cannot both commit;
- cancellation, supersede, timeout, crash, resume, recovery, and rollback each
  preserve fencing and return durable typed receipts;
- a sealed artifact is reused after merge when executable inputs are equivalent
  and rebuilt only with an exact recorded reason when they are not;
- active and queued artifacts survive generation cleanup until disposition;
- all introduced user-facing contracts are documented in every required
  surface and selected provider-free gates pass; and
- source integration is recorded separately from any installed-runtime
  acceptance. A successful plan does not itself authorize production effect.

Plan 0190 may move from `PLANNED` to active only after the current worktree
population is reconciled, issue #136 is assigned to the implementation lane,
the branch and checkout are admitted under policy 0052, and the baseline is
refreshed from canonical `origin/main`. Closeout records the integrated commit,
artifact manifest and digest, selected tests, reuse or rebuild decision, and
remaining operational gate. A production install, if requested, records its
own transaction, fencing generation, doctor evidence, and runtime receipt.
