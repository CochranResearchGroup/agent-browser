# Runbook History | P194 Through P196 Source Checkpoints

This archive preserves superseded Turns 340 through 343. Turn 344 in
`RUNBOOK.md` owns the final integrated and installed P196 state.

## Turn 343 | 2026-09-15

P196 source checkpoint `73001841` proves #76 already repairs #87's foreground
projection race. The legacy oracle takes two independent writer processes and
returns the historical stale-revision error; current replay completes two
sequential foreground projections after one writer each, preserves all updates,
records no duplicate, and clears exact residue. All 43 store tests, owned-launch
cleanup, format, and strict Clippy pass. Protected integration is next; no
browser, profile, provider, production runtime, or installation effect occurred.

## Turn 342 | 2026-09-15

P196 and issue #87 are active on `fix/issue-87-foreground-launch-cas` from
current merged-main checkpoint `6e052f14`. The first packet will drive the real
foreground browser-projection persistence seam through independent adjacent
writers, two sequential launch projections, and exact residue checks. Current
source will be changed only if that missing acceptance remains red. Protected
integration precedes one production build and transactional install; no browser,
profile, provider, credential, tenant, Service State, or runtime effect has yet
occurred.

## Turn 341 | 2026-09-15

[Plan 0195](docs/dev/plans/0195-2026-09-15-custom-profile-identity-repair.md)
and issue #131 are closed after PR #147 merged exact source head `ada1c62e` as
`172ccdd2`; CI run 35013644563 passes every ordinary required gate. Earlier run
35009619406 retains one unrelated Lease Authority process-observation flake;
the exact test and all 108 crate tests passed locally before the clean run. No
browser, install, profile, provider, production, or release effect occurred.

## Turn 340 | 2026-09-15

P194 and issue #76 are closed after PR #146 merged source head `510bf266` as
`f1435195`. The revision-only freshness probe and bounded parser stack pass the
9.64 MB regression, all 41 `service_store` tests, issue #87's pure adjacent-
revision case, format, strict Clippy, and canonical CI run 35007032152. No
production install, monitor retry, browser, profile, provider, or Service State
effect occurred; those live gates remain separate.
