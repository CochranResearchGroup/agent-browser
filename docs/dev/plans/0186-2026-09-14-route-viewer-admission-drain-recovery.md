# Plan 0186 | Route Viewer Admission Drain Recovery

Date: 2026-09-14

State: OPEN

Lane: P186

Product lane: PL-BUGFIX

Branch: `fix/plan-0186-route-viewer-command-scope`

Target: `main`

Integration: merge

Work item: [issue #112](https://github.com/CochranResearchGroup/agent-browser/issues/112)

Consolidation: required

## Objective

Let exact transaction-owned post-commit reconciliation recreate missing
canonical Guacamole route viewers while the admission drain remains active,
without admitting ordinary browser effects or tenant profiles.

## Current State

P185 merged through PR #111 as `ffc6e510`; its exact integrated candidate is
SHA-256 `bce36a4c7cab42f64601c94e40cf4c7c45b2d9d814779fa4baf4f1d2dce10c97`.
The existing forward-only transaction remains at revision 17 with candidate
generation `0.28.0-a28994570dd3-9d43d7f4e826` selected and the admission drain
active.

PR #113 merged the first scoped-admission repair as `3c7d29da`. Exact merged
candidate `34318d21bf7104c982a0d73a33c24875538c9b309cd459273559a08b47fc121a`
admitted route A's initial launch, but the next header command failed before
effect as `runtime_admission_draining`. Unlike navigation, `set headers` did
not carry the global managed profile when the CLI attached its claim, so its
generated prestart launch also lacked that claim. The task-owned temporary
host was terminated after its scoped cleanup left no route display.

After reboot, no canonical route viewer survived. The repaired opener used the
stable managed profile, but its first route-A launch failed before effect as
`runtime_admission_draining`. The CLI attaches a transaction claim only to
Service reconciliation and stream status. The drain accepts claimed Service
reconciliation, stream status, and browser reattach, while workstation
reconciliation itself requires launch, header configuration, navigation, and
cleanup when a canonical viewer is absent. The resulting clean-host resume is
therefore impossible despite coherent transaction and candidate identities.

## Consolidated Batch

- Recognize only canonical managed `rdp-guac-route-*-viewer` profiles as the
  transaction-owned route-viewer scope.
- Attach the exact environment-derived transaction ID and revision to the
  route viewer's launch, navigation, header, and close commands.
- Propagate that already-scoped claim to the generated prestart launch.
- Materialize the managed route scope for secondary commands only when the CLI
  session and runtime profile are exactly equal.
- Keep missing claims, stale revisions, ordinary profiles, and lookalike route
  profile names denied.
- Integrate one candidate, resume the existing transaction, then install the
  exact integrated P186 generation through one fresh preserving transaction.

## Scope

- `cli/src/runtime_adoption.rs` admission scope and focused tests.
- `cli/src/main.rs` claim attachment, launch propagation, and focused tests.
- Source-free workstation fixture proof for the embedded support workflow.
- Canonical plan, roadmap, runbook, and active-lane projections.

## Non-Goals

- No general transaction bypass, admission-drain removal, tenant browser
  launch, authentication retry, profile deletion, credential change, provider
  mutation, broad cleanup, or formal release.
- No relaxation of transaction ID, revision, candidate generation, census,
  managed-profile, owner, or immutable-generation checks.
- No second writer for the P181 lease-authority extraction surfaces.

## Test Plan

First prove the unmodified admission contract fails the canonical route-viewer
claim. Green tests must cover every required action and prove that an ordinary
profile, missing claim, stale revision, and lookalike route name remain denied.
The generated launch must inherit only the already-scoped claim. Run the
focused admission tests, relevant workstation fixture, Rust format, workspace
clippy with warnings denied, patch hygiene, and validation selection from
`ffc6e510`.

## Source Validation Evidence

- Both focused regressions failed red on the old admission contract.
- The final admission-focused Rust sweep passed 28 tests with zero failures.
- Preliminary candidate `18af9f54f67a51d9e2ba0e6d5b379b8b22f3e11cd1d3c3f30ce15f402637bf20`
  and exact PR #113 merge candidate `34318d21bf7104c982a0d73a33c24875538c9b309cd459273559a08b47fc121a`
  passed the source-free workstation fixture. The previously built P185 binary
  failed that fixture with the exact `runtime_admission_draining` signature.
- The follow-up header-shaping regression and the 28-test admission sweep pass;
  exact session-profile equality is required before secondary commands receive
  the route scope. Optimized follow-up candidate
  `4b4ca13a6b1241d689b13f07d162ee9d6da312b66cd03c8b07fa8818acab5bdb`
  passes the strengthened source-free fixture.
- Rust format, workspace clippy with warnings denied, validation selection,
  workstation host provision, fresh VM harness, Guacamole assets, PostgreSQL
  durability, route-user synchronization, and patch hygiene all pass.

## Delivery Sequence And Budget

1. Freeze transaction revision, candidate host, drain, route-display absence,
   and the no-effect launch denial.
2. Add the red admission regressions and the narrow canonical-viewer repair.
3. Run focused and changed-surface validation on one source head.
4. Commit, publish, review, and merge one issue-linked PR.
5. Build the exact merge, use it to complete the existing forward transaction,
   then run one fresh reviewed preserving install so the accepted selected
   generation contains P186.

Stop on transaction revision drift outside the reviewed resume, protected-state
removal, noncanonical profile admission, unowned browser effects, candidate
hash mismatch, or a different reconciliation failure.

## Worker Assignments

The primary owns diagnosis, Rust admission and CLI propagation, fixtures,
integration, and installed acceptance. No subagent is assigned. P181 remains a
separate writer and must reconcile only if it changes these exact files before
P186 integrates.

## Evidence And Exit

Source exit requires red-green admission proof, launch-claim propagation,
source-free installed-workflow proof, format, clippy, selected validation, and
a merged issue-linked PR. Runtime exit requires terminal acceptance of the
existing transaction, one subsequent accepted integrated P186 generation,
matching installed SHA, one supervised production runtime host, no admission
drain, three distinct canonical route displays, coherent stream publication,
and unchanged Last30Days principal, capability, owner identity, and owner
generation.
