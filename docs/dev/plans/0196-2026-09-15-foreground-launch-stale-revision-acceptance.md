# Plan 0196 | Foreground Launch Stale Revision Acceptance

Date: 2026-09-15

State: OPEN

Consolidation: required

Product lane: PL-BUGFIX

Lane: P196

Work items: `CochranResearchGroup/agent-browser#87`, reopened
`CochranResearchGroup/agent-browser#131`, and deployment-discovered acceptance
repairs `CochranResearchGroup/agent-browser#143` and
`CochranResearchGroup/agent-browser#151`

Branch: `fix/post-install-acceptance-repairs`

Target: `main`

Integration: pending protected pull request

## Objective

Prove that an ordinary foreground browser launch can persist its pure browser
projection through adjacent Service State revisions without replaying the
browser effect, then deliver installed acceptance for the integrated #76,
#131, and #87 outcomes plus exact defects exposed by that acceptance.

## Current State

Issue #87 was blocked while issue #76 owned the shared Service State persistence
algorithm. PR #146 has now integrated that dependency. Its bounded repository
replay proves a generic pure mutation converges after adjacent revision changes,
but the issue still lacks a provider-free multi-process regression at the
foreground launch projection seam, two sequential launch outcomes under churn,
and exact task-owned residue evidence.

Issue #131 entered `main` through PR #147. Issue #87 entered `main` through PR
#150 at merge commit `552f692502ec63032b0a936029993510f3299fb7` after every fast
CI gate passed. The one production candidate from that exact clean source was
built and transactionally installed as generation
`0.28.0-dcfa3b547433-83b9a9076b19`, binary SHA-256
`dcfa3b5474339594d524e0011a141f023b124e0a8c20e3905eaea9834319359b`.

Candidate `73001841` added the missing foreground call-seam proof. The legacy
oracle admits a second optimistic attempt, accepts two independent writer
processes, and deterministically returns `service_state_stale_revision`. Current
bounded replay accepts only the first writer per launch, serializes the pure
projection replay, and succeeds for two sequential foreground projections. Both
writer updates survive, each browser projection appears exactly once, and every
helper, holder, transaction, barrier, and disposable-state path is gone. No
launch adapter repair was required for #87. All 43 `service_store` tests, the
exact owned-launch cleanup test, format, and strict workspace Clippy passed.

The production transaction reached accepted revision 25 and finalized after
bounded repair of two newly exposed installer defects. The sealed Guacamole
extension tree inherited owner-only modes and the native prestart path ran
retained-session selection before deriving its stable self-declared subject.
The installed runtime is coherent and doctor passes from the installed binary,
but disposable acceptance reproduced #131's original `custom:<id>` validation
failure before effect. Issue #131 is reopened. This plan now owns one
consolidated follow-up source batch for those three acceptance defects before a
replacement candidate is built.

## Consolidated Batch

1. Add one deterministic provider-free multi-process regression that drives
   the foreground browser-record persistence seam while independent writers
   advance Service State between baseline read and prepared commit.
2. Run two sequential foreground projections under the same reproduced churn,
   prove the pure mutation converges, and prove no transaction, holder, helper
   process, or disposable-state residue remains.
3. If current `main` passes, close the launch-specific source gap without an
   adapter change. If it fails, make the smallest repair that preserves the
   repository replay boundary and never repeats browser, tab, route, provider,
   credential, or tenant effects.
4. Qualify and integrate the frozen source once, then build and install one
   production candidate from verified canonical `origin/main` under the shared
   runtime transaction contract.
5. Run doctor, identity, supervisor, listener, Service State, and bounded
   disposable foreground-launch acceptance against the exact installed bytes.
6. Correct the three exact acceptance defects without widening their product
   boundaries: keep custom service profile IDs out of managed runtime-profile
   decoding, derive native self-declared identity before retained-session
   selection, and make only the shipped Guacamole extension bundle publicly
   readable inside an otherwise sealed generation.
7. Requalify the changed Rust surfaces, integrate once, then perform one
   replacement production build and transaction because executable inputs
   changed after the first candidate's failed acceptance.

## Scope

- foreground browser projection through `persist_service_browser_record_in_repository`;
- the prepared Service State replay test seam and independent writer fixture;
- exact helper, transaction, holder, and disposable-state cleanup evidence;
- Plan 0196, roadmap, runbook, active-lane, issue, pull-request, and deployment
  receipts;
- one production-shaped build, transactional installation, and post-install
  acceptance after protected integration.

## Non-Goals

- Do not change Service State timeouts or weaken revision fencing.
- Do not replay browser, tab, route, provider, credential, or tenant effects.
- Do not widen into issue #143 retained-profile inventory projection.
- Do not perform authenticated site work, tenant mutation, challenge solving,
  profile reset, credential use, or formal release publication.
- Do not build more than the one replacement production candidate now required
  by changed executable inputs and the failed #131 acceptance criterion.

## Delivery Sequence And Budget

- Plan and feedback loop: one fixture design and at most two correction passes,
  target 45 active minutes.
- Diagnosis or repair: current-source verdict plus at most one implementation
  correction, target 60 active minutes.
- Qualification and protected integration: focused checks, selected gates,
  format, strict Clippy, and one pull request, target 120 active minutes.
- Production build, install, reconciliation, and bounded acceptance: the
  completed initial candidate plus one replacement candidate and transaction,
  target 180 active minutes total.
- Overall effort ceiling: 360 active minutes. Reassess after two checkpoints or
  30 active minutes without outcome progress. Maximum work-unit attempts: 3.
  Maximum review and rework cycles: 1.

No subagent, reviewer worktree, benchmark checkout, or parallel runtime operator
is assigned. Deterministic commands own test execution and state readback.

## Worker Assignments

The primary session owns the plan, regression, diagnosis, any repair, validation,
issue and pull-request state, integration, production transaction, installed
acceptance, and closeout. The P169 challenge lane is integrated and does not
overlap this branch's expected source writes. Any later reviewer is read-only
against the frozen published diff and has no runtime authority.

## Evidence And Exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Exact foreground race | Provider-free test calling the browser-record persistence seam with independent writer processes and deterministic barriers | The fixture forces an adjacent revision between baseline and prepared commit and is red-capable against the pre-#76 replay algorithm |
| Safe convergence | Two sequential foreground projections under reproduced churn | Both persist without `service_state_stale_revision`, both independent writer updates survive, and each browser projection appears exactly once |
| No effect replay | Foreground seam and state assertions | Browser launch remains outside the replayed closure; no duplicate browser, event, tab, route, provider, or tenant effect is recorded |
| Exact cleanup | Child exit status plus transaction, holder, path, and process readback | Every helper is terminal and no task-owned transaction, holder, process, or disposable state remains |
| Source qualification | `pnpm validation:select -- --base 6e052f147c7bf549476be7af581df4a13039c54c`, focused Rust, format, strict workspace Clippy, and all selected gates | Every required changed-surface gate passes on one frozen source checkpoint |
| Integration | Published branch, linked pull request, exact-head checks, and merged-main readback | The source and evidence enter protected `origin/main` |
| Candidate identity | Production artifact manifest, source ancestry, executable-input closure, and binary digest | One production-shaped artifact is bound to verified canonical source |
| Installed coherence | Shared runtime transaction, installed manifest and binary digest, supervisor and listener census, doctor, Service State readback, and disposable launch smoke | The installed runtime matches the candidate, is coherent and healthy, and the bounded foreground acceptance passes without residue |
| Custom profile acceptance | Disposable explicit filesystem profile through the real native prestart path | Launch, URL read, close, and exact residue cleanup pass without converting `custom:<id>` into a managed runtime profile |
| Retained route continuity | Native prestart attribution fixture and canonical route recovery | Stable self-declared subject exists before profile selection and all three route displays recover without manual request shaping |
| Sealed Guacamole readability | Materialization mode fixture plus replacement container startup | Extension directory and files are container-readable after sealing while secrets remain private |

## Execution Evidence

- Red-capability oracle:
  `foreground_launch_projection_fixture_rejects_legacy_optimistic_retry`
  observes two independent writer processes and the historical typed stale-
  revision failure before any foreground browser projection commits.
- Current-source acceptance:
  `foreground_launch_projection_converges_twice_under_cross_process_revision_churn`
  completes two sequential projections, each after one independent writer,
  with final revision 4, both writer jobs, two unique browser records, and no
  duplicate projection.
- Exact cleanup: every writer child exits successfully; writer outcome,
  barrier, durable holder, transaction, and disposable state paths are absent
  after readback. The existing
  `failed_owned_launch_persistence_runs_cleanup_exactly_once` contract also
  passes.
- Focused validation: all 43 `native::service_store::tests` pass.
- Changed-surface validation: `git diff --check`, Rust format, and strict
  workspace Clippy pass at source checkpoint `73001841`.
- PR #150, canonical fast CI, production artifact identity, transaction
  acceptance, finalization, installed doctor, one runtime host, one dashboard
  generation, healthy monitor, and three route displays are complete.
- The disposable custom-profile launch failed before effect with
  `invalid_runtime_profile`, reopened #131, and was fully cleaned up. The
  replacement source tests for custom-ID decoding, native prestart identity,
  and Guacamole extension modes pass; broader qualification, integration, and
  replacement installed acceptance remain pending.

## Stop Condition

Stop source work after the exact foreground regression proves current bounded
replay satisfies every source acceptance criterion, or after one evidence-backed
adapter repair and its qualification. Stop before installation if the candidate
is not integrated into current canonical `origin/main`, another runtime mutation
is active, or runtime preflight cannot establish coherent transactional custody.
Preserve the exact failed evidence and do not blind-retry a live mutation.
