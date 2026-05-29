# P07 v0.27.0 Release Prep Validation

Date: 2026-05-29
Lane: P07
Status: PASSED

## Scope

Validate the release-preparation metadata before opening the release pull
request.

## Release Metadata

- `CHANGELOG.md` contains one release extraction marker pair.
- The marker pair wraps `## 0.27.0`.
- `## 0.26.1` no longer contains release extraction markers.
- `docs/src/app/changelog/page.mdx` contains the matching `## v0.27.0` entry
  dated May 29, 2026.
- Contributors since `v0.26.1` resolve to `@ecochran76`.

## Commands

- `git log v0.26.1..HEAD --format='%an <%ae>' | sort -u`
- `pnpm version:sync`
- `git diff --check`
- `pnpm validation:select -- --base HEAD`
- `pnpm --dir docs build`
- `agent-browser install doctor --json`
- `agent-browser doctor remote-view --json`

## Results

- Version sync reported `0.27.0` and no metadata drift.
- Validation selector recommended `git diff --check` and
  `pnpm --dir docs build` for the changed release metadata surfaces.
- `git diff --check` passed.
- `pnpm --dir docs build` passed.
- Installed doctor passed with no issues.
- Remote-view doctor passed with status `ready` and no issues.
- Installed runtime checksum:
  `cb9f81a245464c516d313aee875fa076049cdc5559e9342250c9680463faa9e4`.

## Release Workflow Status

The GitHub release workflow has not run yet in this note. P07 still requires
release PR merge, `Release` workflow dry run, real release workflow run, and
verification of the `v0.27.0` GitHub release assets.
