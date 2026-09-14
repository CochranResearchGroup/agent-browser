# Plan 0185 | Route Viewer Runtime Profile Identity

Date: 2026-09-14

State: OPEN

Lane: P185

Product lane: PL-BUGFIX

Branch: `fix/plan-0185-route-viewer-runtime-profile`

Target: `main`

Integration: merge

Work item: [issue #110](https://github.com/CochranResearchGroup/agent-browser/issues/110)

Consolidation: required

## Objective

Make workstation reconciliation launch and close canonical Guacamole route
viewers through their stable managed runtime-profile identities so a forward
install resume can converge across immutable support generations.

## Current State

P184 merged through PR #109 as `3b7e8411`. Its exact integrated binary SHA-256
is `a28994570dd3cb23d1e67306272c47cd3d3ebf198aceb798a22973b8283127ec`.
The preserving apply committed Service State with no changes and selected
candidate generation `0.28.0-a28994570dd3-9d43d7f4e826`, then stopped in
forward-only transaction `upgrade-8c858bc3-4507-48a0-8eea-c85cd3326fbf`.

Exact revision 13 resume reproduced `open canonical Guacamole route displays`
and restored active units. The route opener passes managed names through
`--profile`, which is a custom path flag. Its generation-specific working
directory therefore changes the derived `custom:<digest>` identity. The same
stable viewer session is rejected as `existing_session_profile_identity_unproven`.
All three intended route viewer names are configured managed runtime profiles
with stable user-data directories.

## Consolidated Batch

- Replace named route-viewer `--profile` arguments with `--runtime-profile` for
  open, header configuration, navigation, and cleanup.
- Align development provider quarantine and reclaim cleanup with that identity
  contract.
- Prove the source and embedded installed support bundle reject the old flag.
- Integrate one repair, refresh the exact forward-only candidate support
  payload through the governed installer, then invoke one exact guarded resume.

## Scope

- `scripts/open-rdp-guac-route-displays.js`.
- `scripts/lib/development-presentation-provider-system-effects.js`.
- Focused static and installed-workstation fixture assertions.
- Canonical plan, roadmap, runbook, and active-lane projections.

## Non-Goals

- No profile deletion, credential reset, authentication retry, tenant action,
  accounting mutation, broad browser cleanup, or formal release.
- No weakening of profile identity, owner generation, admission drain,
  immutable generation, or transaction revision checks.
- No unrelated development provider permission-mode repair.

## Test Plan

The red source contracts must fail while route viewer names use `--profile`.
Green validation must cover the development provider contract assertions, the
source-free workstation install fixture built from the candidate, syntax, patch
hygiene, and validation selection from `3b7e8411`. Rust format and clippy are
not required unless Rust source changes.

## Source Validation Evidence

- Both route-profile assertions failed red while the opener used `--profile`.
- The rebuilt candidate's source-free workstation fixture passed and proved the
  embedded support payload uses `--runtime-profile`.
- The development presentation provider fixture passed under its expected
  `022` artifact umask; the session default is `077`.
- Both changed scripts pass Node syntax and focused static identity checks.
- Workstation host provision, fresh VM harness, Guacamole assets, PostgreSQL
  durability, and route-specific user synchronization fixtures all pass.

## Delivery Sequence And Budget

1. Freeze transaction revision, candidate identity, route inventory, managed
   profile resolution, and the no-effect failure signature, 15 minutes.
2. Repair the shared route-viewer flag contract and its focused assertions, 20
   minutes.
3. Build and validate one source head, then publish and merge one issue-linked
   PR, 45 minutes excluding CI wait.
4. Build or restage the exact integrated support payload without creating a
   second workstation transaction, then resume the current forward-only
   transaction once using its current revision, generation, and census digest.

Stop on changed candidate identity, transaction drift, protected-record
removal, unowned browser effect, profile mutation, or a different failure.

## Worker Assignments

The primary owns diagnosis, both script surfaces, fixtures, integration, and
the exact forward resume. No subagent is assigned. P181 must reconcile only if
it changes the same support or installer surfaces before P185 integrates.

## Evidence And Exit

Source exit requires red-green flag assertions, installed-bundle proof, selected
validation, and a merged issue-linked PR. Runtime exit requires transaction
acceptance, the exact installed binary and selected generation, one supervised
runtime host, no admission drain, coherent stream publication, preserved route
profiles, preserved external browser custody, and unchanged Last30Days
principal, capability, owner identity, and owner generation.
