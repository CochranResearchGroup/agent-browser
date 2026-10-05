# Runtime Checkpoint And No-Release Handoff Plan

Date: 2026-05-29
State: CLOSED
Lane: P05
Outcome: PASSED

Current state: P05 closed as a roadmap runtime checkpoint, not a formal
release. The workspace version metadata is synchronized at `0.27.0`, the local
installed runtime was rebuilt and installed, selected validation passed, and
the installed command passes install doctor, remote-view doctor,
default-profile attach, and the OCR-backed many-to-many Guacamole/RDP live
gate. The formal release remains deferred until the many-to-many Guacamole/RDP
remote operation milestone is fully hardened and operational.

## Purpose

Tidy the active roadmap campaign state, validate the installed runtime, and
leave a source-backed handoff for the next Guacamole/RDP hardening lane without
publishing a GitHub release.

This lane exists because the project is still in an active productization
campaign. The right output is a checked-in checkpoint with validation evidence,
not a tagged release. A formal release is only appropriate once the remote
operation milestone includes:

- reliable many-to-many Guacamole/RDP browser operation
- an installer that needs sudo exactly once on first install
- recurring route and helper maintenance that does not need interactive sudo
- a fully diagnostic doctor surface that explains missing runtime, privilege,
  display, route-pool, Guacamole, RDP, and viewer prerequisites

## Completion Evidence

- Checkpoint validation note:
  `docs/dev/notes/2026-05-29-p05-release-preparation-validation.md`
- Validation selector artifact:
  `docs/dev/notes/2026-05-29-p05-validation-selector.txt`
- Selected validation base: `v0.26.1`
- Checkpoint runtime version: `0.27.0`
- Installed runtime checksum:
  `e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f`
- Many-to-many artifact:
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T12-38-46-972Z`

## Release Boundary

P05 intentionally does not publish a formal release.

- `.github/workflows/release.yml` is manual-only, so pushing roadmap checkpoint
  work to `main` cannot create a GitHub release by accident.
- `CHANGELOG.md` keeps the current work under `## Unreleased`, while the
  existing `0.26.1` release markers remain around the latest published release
  body.
- `docs/src/app/changelog/page.mdx` does not list a `v0.27.0` public release.
- The `0.27.0` version is a local checkpoint version for installed-runtime
  validation during the active campaign.

## Non-Goals

- Do not publish a GitHub release.
- Do not run the real release workflow with `dry_run=false`.
- Do not present `0.27.0` as shipped public release content.
- Do not reopen P01 through P04 unless a checkpoint gate proves one of their
  invariants regressed.
- Do not include workstation-private paths, secrets, browser auth state, or raw
  live artifacts in portable docs.

## Product Invariants

This lane is complete only if these invariants hold:

- Roadmap, runbook, plan, policy, validation-note, script, docs, dashboard,
  generated-client, contract, and source surfaces are intentionally part of the
  checkpoint.
- `package.json`, `cli/Cargo.toml`, `cli/Cargo.lock`, and
  `packages/dashboard/package.json` agree on the checkpoint version.
- Release markers in `CHANGELOG.md` do not point at unpublished work.
- The docs changelog does not imply a public `0.27.0` release exists.
- The GitHub release workflow cannot auto-publish on ordinary `main` pushes.
- `agent-browser install doctor` and `agent-browser doctor remote-view` pass
  from the installed checkpoint runtime.
- The installed checkpoint runtime passes the many-to-many Guacamole/RDP live
  gate with local route-pool and viewer preconditions supplied.

## Slices

### Slice A | Durable Surface And Base Preflight

Resolve the checkpoint authorities and validation base before editing version
or changelog files.

Exit criteria:

- The checkpoint authorities are not accidentally untracked.
- The selected validation base is explicit and justified.
- Non-ancestor tags are recorded as non-authoritative for this lane.

Result: passed. `v0.26.1` was selected as the reachable authoritative base.

### Slice B | Checkpoint Version Decision

Decide whether the installed runtime checkpoint should advance version metadata
without treating the result as a formal release.

Exit criteria:

- The checkpoint version target is explicit.
- Any mismatch between the selected base, current package version, and
  changelog state is explained before edits.

Result: passed. The checkpoint version is `0.27.0`.

### Slice C | Version Sync

Apply the checkpoint version through the repo's supported workflow.

Exit criteria:

- Version files agree.
- `git diff` shows no unrelated metadata churn from the version sync.

Result: passed with `pnpm version:sync`.

### Slice D | No-Release Changelog And Workflow Boundary

Ensure release metadata cannot accidentally publish the active roadmap
checkpoint.

Tasks:

- Keep current work under `## Unreleased` in `CHANGELOG.md`.
- Keep release extraction markers on the latest published release entry until a
  formal release lane intentionally moves them.
- Remove the docs changelog `v0.27.0` entry until a real release ships.
- Make `.github/workflows/release.yml` manual-only.
- Update `AGENTS.md` to record the formal-release milestone boundary.

Exit criteria:

- Ordinary pushes to `main` cannot create a GitHub release.
- Public changelog surfaces do not claim `0.27.0` has shipped.
- A future formal release lane still has a clear manual workflow.

Result: passed by this refactor.

### Slice E | Documentation And Skill Audit

Confirm operator-facing documentation matches the checkpoint runtime behavior.

Exit criteria:

- User-facing docs are consistent with the installed checkpoint and doctor
  output.
- Docs distinguish one-time privilege authorization from recurring
  helper-mediated route maintenance.

Result: passed during P05 validation.

### Slice F | Selected Validation Gate

Run the repo-selected validation set for the checkpoint diff.

Exit criteria:

- Validation evidence is concrete and source-backed.
- Any skipped selected gate has a reason that a maintainer can evaluate.

Result: passed. See
`docs/dev/notes/2026-05-29-p05-release-preparation-validation.md` and
`docs/dev/notes/2026-05-29-p05-validation-selector.txt`.

### Slice G | Installed Runtime Regression Gate

Reconfirm the operator install path after checkpoint metadata and docs are
ready.

Exit criteria:

- Installed runtime, workspace binary, and pnpm package binary agree.
- Install doctor and remote-view doctor pass.
- Default-profile attach smoke passes.
- Many-to-many Guacamole/RDP live gate passes from the installed runtime.

Result: passed.

### Slice H | Handoff

Close P05 as a checkpoint and open the next Guacamole/RDP productization lane.

Exit criteria:

- P05 states no release was published.
- The next plan targets the installer, doctor, and many-to-many operational
  hardening needed before a formal release.

Result: P06 is
`docs/dev/plans/0006-2026-05-29-guac-rdp-productization-hardening-plan.md`.
