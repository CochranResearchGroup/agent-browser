# Plan 0208 | Worktree Closeout And Candidate Custody

Date: 2026-09-16

Plan version: 3

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P208

Work item: `CochranResearchGroup/agent-browser#171`

Branch: `platform/p208-worktree-closeout`

Target: `main`

Integration: merge through the protected `main` workflow after provider-free
concurrency, crash-recovery, candidate-custody, and changed-surface validation

Source baseline: `2632e31ce34e62873598088d0c92c498357aafe2`

Implementation checkpoint: `907ae9f2fe669478b73f152f84d9e25a59cfb7db`

Validation-wiring checkpoint: `9d34365d302baa7725b3d4689a7b079dea4258a5`

Pull request: `CochranResearchGroup/agent-browser#182` (draft pending exact-head validation)

## Objective

Provide one advisory, durable, cross-process worktree-closeout transaction that
lets an operator inspect consequences, select retain, archive, or discard for
each pinned candidate, serialize the exact Git removal transition, and recover
an interrupted operation without losing candidate bytes or Git custody. A
second process must join or observe the same operation rather than race it.

## Current State

Issue #171 records the P190 race: one session removed the worktree after
preserving the sealed candidate while another session was still performing its
closeout census and preparing a second preservation copy. Current policies
require inventory and custody checks but provide no tracked closeout helper or
durable operation. Candidate build outputs and completion receipts live under
the removable checkout's `cli/target`, and sealed-candidate lookup requires the
original absolute paths. An archive that merely copies bytes would therefore
survive removal but remain undiscoverable for supported reuse.

P208 admission reproduced the coordination class without changing another
checkout: after a clean inventory showed no P207, another session admitted P207
for issue #175 before P208's `git worktree add`. The attempted add failed
because the path already existed and left only an unused local branch, which
was reconciled into this P208 branch. This is evidence for serializing the
inventory-to-transition boundary, not a substitute for the required
provider-free regression.

P204 merged through PR #179 as `f5e3f31b` after its exact-head CI passed. P208
is rebased onto that integration point and now owns its bounded shared-surface
transition while preserving P204's still-open issue #164 records. The merged
selector and aggregate now include a dedicated provider-free `Repository
Tooling` lane for candidate-build and worktree-closeout contracts.

## Consolidated Batch

1. Freeze a versioned advisory plan and durable operation/receipt contract
   keyed by repository common-Git identity plus worktree incarnation, exact
   HEAD/ref custody, request digest, fencing generation, candidate identities,
   selected dispositions, and archive identity.
2. Add a deterministic two-process fixture with explicit barriers. It must go
   red on the current census-to-removal race and prove that only one operation
   can commit while a peer joins or observes the same terminal receipt.
3. Implement read-only inspection and supported operator choices without
   turning advisory state into a general permission service. A pinned candidate
   without a selected disposition returns typed evidence and exact next
   choices; explicit safe retain, archive, discard, or remove choices remain
   available.
4. Implement durable begin, resume, revalidation, archive/discard, exact Git
   transition, and terminal readback. Long copies and hashing occur outside the
   short physical lock while the durable fenced operation excludes another
   writer.
5. Preserve sealed candidate bytes byte-for-byte and publish a separate durable
   archive locator that supported candidate lookup can verify without rewriting
   the original manifest, closure, support manifest, seal, binary, or receipts.
6. Align policies 0004, 0050, 0051, and 0052 plus the documented command surface
   with the implemented ownership and disposition contract, then run the
   selected provider-free gates once at the final batch boundary.

## Scope And Non-Goals

Expected writes are new provider-free closeout planner, durable filesystem
adapter, command entrypoint, deterministic fixtures, the narrow candidate
archive-locator lookup seam, and required policy/help/package projections after
shared-writer reconciliation.

This plan does not authorize a production build, installation, runtime
replacement, browser or profile effect, process cleanup, provider effect,
deletion of an existing development generation, direct mutation of another
lane's worktree, or interception of arbitrary raw `git worktree remove`
commands. The repository helper is the governed path; raw-Git bypass remains an
explicit limitation unless a separate enforcement design is approved.

## Delivery Sequence And Budget

- Critical path: contract freeze to red two-process fixture to pure transition
  repair to filesystem/archive adapter to crash/replay matrix to shared-surface
  reconciliation to final selected validation and PR integration.
- First evidence deadline: one deterministic red-capable two-process command
  within 30 active minutes of implementation start.
- Maximum implementation attempts: 3.
- Maximum review/rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 180 active minutes through source qualification and
  integration. No release build or runtime acceptance is required.
- Validation strategy: focused Node fixtures during implementation; one final
  changed-surface selection and only the presubmit gates selected by that
  final diff. No browser, installer, or comprehensive Rust replay unless the
  final executable surface actually requires it.

## Worker Assignments

The P208 primary owns plan and issue state, all writes, Git custody, causal
decisions, worker reconciliation, validation, integration, and final
acceptance.

- `/root/p171_seam_reproducer`, `gpt-5.6-sol` medium: read-only implementation
  seam and red-fixture design. Return exact symbols/files, hypotheses, minimal
  repair, and overlap risks; stop before edits or effects.
- `/root/p171_concurrency_review`, `gpt-6-astra` high: read-only adversarial
  transaction and crash-recovery review. Return evidence-shaped invariants and
  an acceptance matrix; stop before edits or effects.
- `/root/p171_policy_overlap`, `gpt-5.6-luna` medium: deterministic portfolio,
  plan, policy, and validation inventory. Return a compact control record and
  admission hazards; stop before edits or effects.

Workers have no branch, worktree, issue, runtime, or acceptance authority and
may not spawn nested workers. Their output is advisory until primary
reconciliation. This topology optimizes elapsed time while assigning mechanical
inventory to the economical tier and reserving consequential concurrency
reasoning for the specialist tier.

## Evidence And Exit

| Requirement | Evidence required | Current state |
| --- | --- | --- |
| One closeout writer | Deterministic two-process barrier fixture; one committed removal and one joined/observed receipt | proven on rebased implementation `907ae9f2` |
| Replay and conflict safety | Same request is idempotent; changed disposition is a typed request conflict | proven on rebased implementation `907ae9f2` |
| Exact worktree identity | Common-Git identity, worktree incarnation, HEAD/ref, path, and dirty-state drift are revalidated before commit | proven on rebased implementation `907ae9f2` |
| Candidate choice | Pinned candidate without selection reports retain/archive/discard consequences and performs no removal | proven on rebased implementation `907ae9f2` |
| Retain | Checkout and candidate remain; receipt truthfully reports no removal | proven, including explicit retained-operation succession, on `907ae9f2` |
| Archive and reuse | Exact bytes and digests survive outside the checkout; a fresh process resolves the archive through a verified locator | proven on rebased implementation `907ae9f2` |
| Discard isolation | Only the selected candidate is removed; unrelated candidates, archives, branches, and Git custody remain | proven on rebased implementation `907ae9f2` |
| Interrupted preservation | Fault points before, during, and after publication resume the same fenced operation; partial archives never become complete | proven by interrupted archive publication and fresh terminal verification on `907ae9f2` |
| Interrupted removal | Recovery reconciles filesystem and Git registration after intent or effect without a second removal | proven by post-removal recovery and CLI replay on `907ae9f2` |
| Advisory authority | Status and plan remain read-only; explicit operator choices are supported and typed rather than reduced to generic denial | proven on rebased implementation `907ae9f2` |
| Policy and documentation | Policies and command guidance describe the implemented boundary and raw-Git limitation | complete through policies, AGENTS guidance, package invocation, and repository-tooling validation at `9d34365d` |
| Integration | Final published head passes selected gates and enters `main` through the linked PR | draft PR #182; final exact-head gates and protected integration pending |

## Version 3 Checkpoint

The source-qualified packet uses immutable per-generation effect claims rather
than reclaimable lock files. It binds coordination and archives to the verified
common Git directory, discovers completed and active candidate-build
obligations, and makes candidate claim publication perform a before-and-after
closeout handshake. Two process fixtures prove that a peer joins while the
selected writer removes the checkout. Retain, later archive succession,
archive corruption detection, discard with branch custody, malformed operation
and effect rejection, detached worktree inspection, and fresh-process archive
lookup are covered without using a real repository candidate or runtime.

The final adversarial review of pre-rebase checkpoint `785a20d8` found no
remaining P1 blocker. Its patch-equivalent rebased implementation is
`907ae9f2`. P204 / PR #179 is integrated, and checkpoint `9d34365d` registers
the helper as `pnpm run worktree:closeout`, adds the provider-free repository
tooling test command, and extends the versioned selector plus stable Presubmit
aggregate with a dedicated `Repository Tooling` job. Local focused checks are
green. PR #182 remains draft only until the reconciled final head is published
and exact-head forge evaluation completes.

Exit requires every row complete or an explicit separately tracked deferral that
does not weaken the issue's promised outcome. A clean worktree, copied archive,
single passing process, or policy-only update is insufficient.

## Stop Conditions

Stop before touching another active worktree, deleting a real candidate,
terminating a process, installing a binary, or mutating shared runtime state.
Stop and reconcile with P204 before editing its current package, validation,
AGENTS, roadmap, runbook, or active-lane transition. Stop and split rather than
silently widening the helper into general Git authorization, runtime candidate
promotion, or production cleanup.
