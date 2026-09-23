# Plan 0217 Execution Handoff

Date: 2026-09-22

Product lane: PL-PLATFORM

Disposition: active-input

Owning plan or work item: Plan 0217

Related lanes: P207 documentation overlap; issues #189 and #190 as P217 regression subcases

This is the durable branch-local copy of the fresh-agent handoff for Plan 0217. The active plan and current repository state remain authoritative if this note becomes stale.

## Authority And Custody

Follow the user's requirements, branch-local `AGENTS.md` and policies, [Plan 0217](../plans/0217-2026-09-22-availability-first-remote-view-reliability.md), then current source, tests, Git state, runtime receipts, and installed identity. P211 evidence is historical and bounded. Chat and Graphiti are advisory.

- Worktree: `/home/ecochran76/workspace.local/agent-browser-p211`
- Branch: `platform/p211-simple-cold-upgrade`
- Handoff head: `9a03297b`
- Pull request: draft PR #191
- Product lane: `PL-PLATFORM`
- Active plan: P217
- P211 is cancelled as superseded and its revision loop must not resume.
- Production install, release, merge, branch deletion, and worktree removal are not authorized by this handoff.

Before editing, verify the clean worktree, branch, `HEAD`, upstream, divergence, and current draft PR head. Re-read policies 0028, 0042, 0044, 0045, 0047, 0050, and 0052. Re-read policy 0051 before any runtime mutation.

The operator must explicitly start the managed campaign with `/goal execute Plan 0217 availability-first remote-view reliability with a cumulative 2,000,000-token ceiling`. This note does not independently authorize goal creation or runtime effects.

## First Packet: M0

Use CodeGraph first to trace ordinary named-profile open through route selection, browser launch, handoff publication, close, and recovery. Identify every lease, fencing, generation, quarantine, cleanup-obligation, or ambiguous-ownership denial point. Separate cleanup safety from service admission: uncertain resources remain protected from destructive cleanup, but historical metadata cannot create phantom capacity reservations or deny another route when real capacity is available.

Capture one synthetic, non-private visual baseline from the existing development provider under fresh policy 0051 effect custody. Bind it to the source head, installed generation, provider manifest digest, and observed route inventory. Freeze a recovery table for browser, automation-runtime, XRDP-route, Guacamole-connection, and provider restart. For each case record required operator URL, handoff ID, profile and session identity, maximum recovery interval, and allowed interaction interruption.

End M0 in `RUNBOOK.md` or a routed evidence note with exact symbols and files, the denial path, visual symptom, evidence identity, recovery table, and M1 cut line. Do not implement a replacement architecture during M0. Stop the tactic after 30 active minutes without outcome progress or two consecutive no-progress checkpoints.

## Frozen Acceptance Boundaries

- Historical metadata cannot reserve capacity. Real capacity exhaustion is a concrete failure and does not authorize destruction of an uncertain resource.
- P217 removes Lease Authority from the trusted single-user ordinary admission path, not from the extracted library or unrelated adversarial and multi-tenant contracts.
- M3 freezes its exact candidate, binary, installed generation, provider manifest digest, route IDs, displays, and route count before testing. That inventory is the acceptance denominator.
- The same authenticated operator URL and handoff ID survive every restart case for which the M0 recovery table requires continuity.
- The Plan 0158 protected external-vantage workflow is used when its contract applies. Otherwise a reviewed manual procedure must define operator roles, deliberate route selection, the synthetic marker, timestamps, visual artifacts, and input receipts before testing.
- Blank, stale, partial, terminal-only, hidden-browser, wrong-route, unresponsive, dirty-desktop, or incorrect-z-order results fail.
- Protocol, process, URL, doctor, and unit-test evidence cannot substitute for visual and input proof.
- Close requires a fresh OS process census proving no unowned browser or daemon tree.

## Existing Evidence And Stops

P211 source `9e7c5719` and installed generation `0.28.0-1d808efe0797` proved provider-free qualification, doctor, three disposable cycles, one ready named-profile response, one authenticated opaque handoff, exact close, and obligation-free residue. Commit `131fc5a2` withdrew the integration verdict. Reuse those results only within their stated scope.

Do not add a lease, scheduler, allocator, reconciliation, quarantine, or general desktop framework. Do not build or install a candidate before M1 and M2 are consolidated and focused validation passes. Do not mutate production. Do not integrate PR #191 until every Plan 0217 evidence row is complete.

## Current Stop State

Plan 0217 consumed its one implementation attempt and one repair. Installed
replacement generation `0.28.0-cd33a6aa46d9`, binary SHA-256
`cd33a6aa46d90f93b8d7788840a38db781edffd50e8351ffec05917ffe0bf47b`,
still returns `existing_session_profile_identity_unproven` for the exact
ordinary no-effect reproducer. Generic profile-lease admission precedes the
remote-view coordinator's later launch and attribution flow; the passing unit
fixture did not model that request shape.

Development doctor independently fails because durable inventory retains four
ready warm displays on `:13` through `:16`, while fresh provider observation
finds only route 1 on `:24`. Provider preflight refuses mutation at the
deliberate `route-keeper-runtime` interlock. No provider apply or browser open
occurred. Resume only with explicit authority either to revise Plan 0217's
attempt limit for the generic lease-admission seam or to complete the broader
Plan 0211 route-keeper provider-control dependency. Integration remains
prohibited.
