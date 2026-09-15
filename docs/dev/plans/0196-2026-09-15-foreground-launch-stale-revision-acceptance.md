# Plan 0196 | Foreground Launch Stale Revision Acceptance

Date: 2026-09-15

State: OPEN

Consolidation: required

Product lane: PL-BUGFIX

Lane: P196

Work item: `CochranResearchGroup/agent-browser#87`

Branch: `fix/issue-87-foreground-launch-cas`

Target: `main`

Integration: pending protected pull request

## Objective

Prove that an ordinary foreground browser launch can persist its pure browser
projection through adjacent Service State revisions without replaying the
browser effect, then install one consolidated production candidate containing
the integrated #76, #131, and #87 outcomes.

## Current State

Issue #87 was blocked while issue #76 owned the shared Service State persistence
algorithm. PR #146 has now integrated that dependency. Its bounded repository
replay proves a generic pure mutation converges after adjacent revision changes,
but the issue still lacks a provider-free multi-process regression at the
foreground launch projection seam, two sequential launch outcomes under churn,
and exact task-owned residue evidence.

Issue #131 is separately closed through PR #147. Current `origin/main` at lane
admission is `6e052f147c7bf549476be7af581df4a13039c54c`; it contains both repairs and
has no open pull request or competing #87 branch. Production installation has
not occurred for this combined source.

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
- Do not build a second production candidate unless executable inputs change or
  the first candidate fails a named acceptance criterion.

## Delivery Sequence And Budget

- Plan and feedback loop: one fixture design and at most two correction passes,
  target 45 active minutes.
- Diagnosis or repair: current-source verdict plus at most one implementation
  correction, target 60 active minutes.
- Qualification and protected integration: focused checks, selected gates,
  format, strict Clippy, and one pull request, target 120 active minutes.
- Production build, install, reconciliation, and bounded acceptance: one
  production candidate and one replacement transaction, target 120 active
  minutes.
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

## Stop Condition

Stop source work after the exact foreground regression proves current bounded
replay satisfies every source acceptance criterion, or after one evidence-backed
adapter repair and its qualification. Stop before installation if the candidate
is not integrated into current canonical `origin/main`, another runtime mutation
is active, or runtime preflight cannot establish coherent transactional custody.
Preserve the exact failed evidence and do not blind-retry a live mutation.
