# Plan 0195 | Custom Profile Identity Repair

Date: 2026-09-15

State: CLOSED

Consolidation: required

Product lane: PL-BUGFIX

Lane: P195

Work item: `CochranResearchGroup/agent-browser#131`

Branch: `fix/issue-131-custom-profile-identity`

Target: `main`

Integration: merged through PR #147 as `172ccdd2f74db1ff9586fa9bf1c52a38f058b662`

## Objective

Make a documented `--profile <absolute-path>` launch retain one stable custom
profile identity through launch planning and validation without interpreting
that internal identity as a managed `--runtime-profile` name.

## Current State

Installed Agent Browser 0.28.0 derived `custom:14805924951506933012` from a
valid absolute profile path, then rejected that generated identity as
`invalid_runtime_profile` because managed profile names prohibit colons. The
current source still derives `custom:<stable-hash>` in `service_profile_id()`
and contains launch paths that can place the derived service identity into the
managed runtime-profile field before Chrome validation.

Candidate `fd5bb67f` keeps the established opaque Service State identifier but
maps only named service profile IDs into `LaunchOptions.runtime_profile`.
Explicit custom directories continue through `LaunchOptions.profile`, while an
explicit caller-supplied managed name still reaches the unchanged validator.
The launch-path regression failed on the original source at the exact identity
boundary and passes on the candidate. Seven custom-profile tests, the invalid
managed-name oracle, all 585 native service tests, format, strict workspace
Clippy, docs build, remote-view docs, and every source-free workstation check
selected from the full diff pass. The exact current binary also passes the
workstation install fixture; its two exact disposable daemon residues were
identified by deleted executable path and terminated after the pass.

PR #147 merged exact source head `ada1c62e` into `main` as `172ccdd2` after
exact-head CI run 35013644563 passed every ordinary required gate. Earlier run
35009619406 retains one unrelated failure in the unchanged Lease Authority
process-observation test. That exact test passed immediately in isolation, all
108 Lease Authority crate tests then passed together, and the failure did not
recur in the clean exact-head run.

Issue #143 owns the separate retained managed-profile owner and inventory
projection defect. P194 and issues #76 and #87 own Service State persistence
and locking. P195 must not edit `service_store.rs`, retained-owner projection,
or the managed profile-name grammar.

## Consolidated Batch

1. Add one provider-free launch-path regression that begins with an absolute
   custom profile path and fails on the current `invalid_runtime_profile`
   symptom.
2. Keep custom directory identity and managed runtime-profile identity distinct
   through launch shaping and validation.
3. Add focused behavior coverage for stable identity, distinct paths, invalid
   managed names, and collision resistance without exposing raw paths in
   service identity.
4. Qualify the frozen source across every selected Rust, documentation,
   formatting, and lint surface, then publish it through a linked pull request.

## Scope

- custom profile identity derivation in `cli/src/native/service_lifecycle.rs`;
- launch option shaping under `cli/src/native/action_runtime/runtime/`;
- managed profile validation only as an unchanged compatibility oracle;
- focused provider-free tests at the launch interface;
- Plan 0195, roadmap, runbook, and active-lane custody records;
- user-facing CLI, README, skill, and docs-site clarification of the existing
  distinction between custom directories and named runtime profiles.

## Non-Goals

- Do not change the accepted grammar for managed `--runtime-profile` names.
- Do not repair retained Default-profile ownership or inventory issue #143.
- Do not edit Service State persistence, locking, or revision replay owned by
  P194.
- Do not launch Chrome, install a candidate, mutate a protected profile, or
  perform any provider or production effect.
- Do not replace stable opaque service profile IDs with raw filesystem paths.

## Delivery Sequence And Budget

- Diagnosis and red loop: one focused launch-path fixture plus at most one seam
  correction, target 30 active minutes.
- Repair: one identity-shaping implementation and at most one correction pass,
  target 45 active minutes.
- Qualification and publication: selected checks, format, strict Clippy,
  required Rust lanes, and one pull request, target 90 active minutes.
- Overall active-work ceiling: 180 minutes. Reassess after two checkpoints or
  30 active minutes without outcome progress. One fresh-context review pass is
  available after candidate freeze.

No subagent, browser worker, runtime operator, provider operator, benchmark
worktree, or auxiliary checkout is assigned.

## Worker Assignments

The primary session owns the plan, red regression, diagnosis, implementation,
validation, issue updates, publication, and integration. Deterministic tools
run source discovery, tests, Git checks, and planning audits. Any later reviewer
is read-only against the frozen published diff and has no runtime authority.

## Evidence And Exit

| Requirement | Evidence | Exit condition |
| --- | --- | --- |
| Exact reproduction | Focused provider-free launch-path test using an absolute custom profile | Baseline deterministically returns the current invalid managed-profile error |
| Custom path behavior | The same test on the candidate | Launch shaping preserves the explicit path and does not set a custom service ID as a managed runtime profile |
| Stable identity | Repeated derivation fixture | The same exact path yields one opaque identity without exposing the path |
| Path separation and collision resistance | A deterministic 4,096-path corpus | Every distinct fixture path produces a distinct opaque identity |
| Managed-name compatibility | Existing and focused invalid-name tests | Valid managed names remain accepted and invalid names remain rejected |
| Changed surfaces | `pnpm validation:select -- --base f1435195bcdde4f63fb2371cfdb4008c74b34875`, focused tests, format, strict Clippy, docs build, native service compartment, and workstation fixtures | Every selected source gate passes on the frozen candidate |
| Integration | Remote branch, linked PR, exact-head checks, and merged-main readback | The repair enters `origin/main` through the protected workflow |
| Live boundary | Explicitly not applicable | No browser, install, profile, provider, or production mutation occurs |

## Execution Evidence

- Candidate `fd5bb67f` introduced the custom-profile identity repair; merge
  checkpoint `ada1c62e` reconciled current `main` without changing that source.
- The focused `custom_profile` family passed all 7 tests after reconciliation.
- The native-service compartment passed all 585 tests; format, strict workspace
  Clippy, documentation builds, and every selected workstation fixture passed.
- CI run 35013644563 passed Version Sync Check, Dashboard, Service Client, Rust
  Quality, Workstation Fixtures, the comprehensive Rust suite, and no-launch
  service smokes on exact source head `ada1c62e`.
- CI run 35009619406 preserves the first failure: the unchanged Lease Authority
  direct-child identity fixture observed stale process evidence once. The exact
  test and the complete 108-test Lease Authority compartment passed locally,
  and the exact-head CI rerun passed without recurrence.
- PR #147 merged as `172ccdd2`; issue #131 closed from that merge.
- No browser, installation, profile, provider, production, or release effect
  occurred.

## Stop Condition

Stop after protected integration and exact source validation, or earlier if a
red launch-path fixture cannot exercise the reported symptom and the missing
installed input cannot be reconstructed provider-free. If the repair requires
`service_store.rs` or retained-owner projection changes, stop and route that
work through P194 or issue #143 instead of widening this lane.

The source stop condition is satisfied. The documented custom profile path flow
is integrated and issue #131 is closed; live runtime effects were not required.
