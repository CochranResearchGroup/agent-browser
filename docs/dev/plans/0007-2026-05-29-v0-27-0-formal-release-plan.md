# v0.27.0 Formal Release Plan

Date: 2026-05-29
State: OPEN
Lane: P07
Outcome: IN PROGRESS

Current state: release metadata and local validation are complete, and the
release pull request has merged. Release workflow dry runs exposed two
cross-target blockers: a Linux cfg leak in the private remote-headed display
fallback, followed by a static X11 link in the browser-focus helper. The fixes
are in progress, and P07 still needs a successful release workflow dry run,
real release workflow run, and GitHub release asset verification.

P05 kept `0.27.0` as an installed-runtime checkpoint because
the Guacamole/RDP many-to-many operation milestone still needed
productization. P06 closed that gap with validated many-to-many operation from
the installed command, a one-time privilege-install contract, idempotent
helper readiness, and install plus remote-view doctor diagnostics. P07 turns
that validated checkpoint into the formal GitHub release.

## Purpose

Prepare, validate, merge, and publish the `v0.27.0` GitHub release from the
already-synchronized `0.27.0` version metadata.

## Release Preconditions

- P06 is closed with outcome `VALIDATED`.
- `package.json`, `cli/Cargo.toml`, `cli/Cargo.lock`, and
  `packages/dashboard/package.json` agree on `0.27.0`.
- `CHANGELOG.md` release markers move from `0.26.1` to `0.27.0` before any
  release workflow run with `dry_run=false`.
- `docs/src/app/changelog/page.mdx` lists `v0.27.0` only when this lane is
  actively preparing the public release.
- `agent-browser install doctor` and `agent-browser doctor remote-view` pass
  from the installed runtime.

## Non-Goals

- Do not add new Guacamole/RDP product behavior in this lane.
- Do not change runtime contracts except for release metadata and release
  documentation.
- Do not publish to npm as the authoritative release target.
- Do not include workstation-private paths, secrets, browser auth state, or raw
  live artifacts in release notes.

## Product Invariants

- Release notes accurately summarize the already-validated P03 through P06
  campaign work.
- Only one `CHANGELOG.md` entry contains release extraction markers.
- The docs changelog mirrors the public release entry.
- The release workflow dry run passes before the real release is created.
- The final GitHub release has tag `v0.27.0`, is not a draft, and includes the
  expected platform binaries.

## Slices

### Slice A | Release Metadata

Status: COMPLETE.

Tasks:

- Move the current `## Unreleased` changelog content into `## 0.27.0`.
- Add the `### Contributors` section for contributors since `v0.26.1`.
- Move release extraction markers from `0.26.1` to `0.27.0`.
- Add the matching `v0.27.0` docs changelog entry dated May 29, 2026.

Exit criteria:

- `CHANGELOG.md` and the docs changelog describe the same release.
- Previous release entries no longer contain extraction markers.

Result: complete. `CHANGELOG.md` now wraps `0.27.0` in the release extraction
markers, `0.26.1` is a plain historical entry, and the docs changelog includes
`v0.27.0` dated May 29, 2026.

### Slice B | Local Validation

Status: COMPLETE.

Tasks:

- Run `pnpm version:sync`.
- Run the selected validation surface for release metadata changes.
- Re-run installed install and remote-view doctor checks.

Exit criteria:

- Local validation passes or every skipped gate has a written reason.

Result: complete. See
`docs/dev/notes/2026-05-29-p07-v0-27-0-release-prep-validation.md`.

### Slice C | Release Pull Request

Status: COMPLETE.

Tasks:

- Commit the release-preparation changes.
- Push `prepare-v0.27.0`.
- Open and merge the release PR into `main`.

Exit criteria:

- `main` contains the release metadata and remains synchronized with origin.

Result: complete. PR #5 merged into `main` as
`d13cddc0851395aa2e87fccc2902ad38c9978ae6`.

### Slice D | GitHub Release

Status: IN PROGRESS.

Tasks:

- Run the `Release` workflow with `dry_run=true`.
- Run the `Release` workflow with `dry_run=false` after the dry run passes.
- Verify the `v0.27.0` GitHub release and expected assets.

Exit criteria:

- `v0.27.0` exists as the public GitHub release.
- Release assets match the workflow's expected platform matrix.

Result: in progress. The first dry run failed with a cross-target Rust compile
error in `cli/src/native/cdp/chrome.rs`; the second dry run proved the cfg fix
on Windows and macOS, then failed Linux zigbuild linking because
`cli/src/native/browser.rs` linked directly against `libX11`. See
`docs/dev/notes/2026-05-29-p07-release-dry-run-cross-target-fix.md`.
