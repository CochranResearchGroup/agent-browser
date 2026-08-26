# Plan 0131 Slice E Current-Main Production Candidate

Date: 2026-08-26

Status: CANDIDATE QUALIFIED | PRODUCTION TRANSACTION NOT APPLIED

Authority: SLICE E SOURCE RECONCILIATION AND PRODUCTION AUTHORITY GRANTED WITH
ONE BOUNDED CARGO GUARD OVERRIDE

## Outcome

The reconciled production-capable source and release candidate are qualified.
The installed production selector, provider configuration, browsers, routes,
displays, handoffs, and processes were not changed.

This receipt supersedes the candidate identity in
`docs/dev/notes/0131-4-2026-08-25-controlled-x11-provider-production-candidate-preflight.md`.
That earlier candidate remains historical evidence and is not eligible for a
production transaction.

The production transaction and controlled fixture remain deferred. Plan 0133
is development accepted but retains its production-read-only boundary, and the
latest live projection provided no apply-safe fixture route without expansion,
takeover, browser closure, workload parking, or displacement.

## Reconciled History

The branch `feature/plan0131-production-candidate-reconciled` was created from
current main without rewriting the published Plan 0131 branches. The source
work was split into truthful units:

- `287a5c53` fixes unattended service GC to use one clock sample for token
  issuance and validation;
- `8c3a0da5` admits the production provider and revalidates its exact generation
  identity inside the route fence before every effect;
- `fa9a03c3` supersedes the pre-focus candidate receipt;
- `2d940811` merges the independent policy rollout at main commit `7cf1f575`.

The reflog-only commit `c462c7d1` was not restored or merged. It was a mistaken
local cherry-pick with no unique source change required beyond the reconciled
provider implementation.

## Candidate Identity

- Branch: `feature/plan0131-production-candidate-reconciled`
- Main base: `f46b9e82dec8a1c985b991bfd9ffb1280eed241d`
- Reconciled current-main commit: `7cf1f575125263425b76db2b2fa0887e673b23a1`
- Binary-affecting provider commit: `8c3a0da5`
- Binary-affecting GC commit: `287a5c53`
- Source-tree head at build: `2d940811955475d688a8df14ba0d775c3608a58e`
- Source tree: `3c47ecc32491673223da787244e41c37b5997589`
- Candidate path: `cli/target/release/agent-browser`
- Candidate version: `0.28.0`
- Candidate SHA-256:
  `69c6829fe432d77fc4da140b91509a5d861e5efd981765447643c54f98e8d766`

The receipt itself is a documentation-only successor to the source-tree head.
It does not change candidate binary inputs.

## Provider Effect-Boundary Regression

The named risk was a valid provider generation changing between construction
and an individual effect. The regression was placed at the provider admission
seam.

The red run failed with Rust compiler error E0432 because
`revalidate_provider_admission` did not exist. The implementation then added a
fresh environment, executable, manifest, and binary identity read inside the
guarded effect callback. A different valid generation now returns
`desktop_input_provider_generation_changed` before journal preparation or
input emission. The focused green run passed, followed by all 12 provider
admission tests, both controlled-provider tests, and all 28 desktop interaction
tests.

## Validation Receipt

All validation here was run by the primary agent in the dedicated Plan 0131
worktree. Cargo used `scripts/ci/cargo-safe.sh`, four jobs, the repository WSL
cgroups, per-invocation memory limits, and aggregate slice limits. The only
override was `AGENT_BROWSER_CARGO_MINIMUM_SWAP_FREE_KIB=0`.

Focused and presubmit validation passed:

- Rust formatting and strict Clippy with warnings denied;
- the unattended GC single-clock regression;
- provider admission, controlled provider, desktop interaction, workstation
  installation, and remote-view open test selections;
- service-client generation, type, export, request, observability,
  fixed-input, managed-profile, example, parity, and no-launch contracts;
- dashboard view-stream, browser-row, browser-table, inspector, and production
  build gates;
- remote-view documentation and documentation production builds;
- workstation host, fresh-VM, Guacamole asset, PostgreSQL durability, and
  route-specific user synchronization contracts;
- the source-free workstation installation fixture;
- WSL Cargo safety.

The comprehensive repository Rust runner completed with exit zero. Its
parallel-safe partition reported 1,548 passed, 57 intentionally ignored, and
zero failed, followed by green integration binaries and every serial
environment-mutating partition. No retry was used.

The actions structural regression checker passed. The aggregate actions
architecture command stopped at the existing P0101 and P0108 remediation
findings. Running the same remediation checker at current main produced the
same finding list, including the same interface total of 294 versus the frozen
263 expectation. This is classified as inherited baseline debt, not a Slice E
regression. The downstream WSL check in that aggregate command was not selected
after the baseline failure, but its standalone gate passed.

Two earlier aggregate attempts were interrupted while nested Cargo admission
waited on the default swap threshold. They were not code failures. The
remaining selections were rerun with the authorized swap-only override and
passed. The comprehensive Rust lane and release build each used one clean
attempt.

The release build completed from source-tree head `2d940811` in 13 minutes 49
seconds and produced the candidate identity above.

## Remaining Transaction Gate

Before any production execution, re-anchor the remote main, exact candidate,
installed generation, rollback generation, service state, provider state,
process census, browser ownership, route and display allocation, controller
lease, and resource projection.

Both conditions remain mandatory:

1. Plan 0133's production-read-only boundary is explicitly cleared or
   superseded by current source-backed authority.
2. One exact controlled fixture route is apply-safe without route expansion,
   controller takeover, browser closure, workload parking, or displacement.

Only then may the existing workstation hot-upgrade transaction install the
exact candidate. After installation, provider identity must be re-read before
the controlled fixture. Any identity drift, uncertain effect, missing rollback
readiness, or route ambiguity stops and follows the transactional rollback
contract.

## Effects Not Taken

- no production installation, selector change, or rollback;
- no production provider admission or input event;
- no controlled fixture browser or route allocation;
- no controller takeover, browser closure, workload parking, or route release;
- no process cleanup or retry;
- no real authentication, credential, private profile, or tenant-content use.
