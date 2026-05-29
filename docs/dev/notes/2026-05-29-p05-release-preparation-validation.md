# P05 Runtime Checkpoint Validation

Date: 2026-05-29
Status: Passed

Update after maintainer clarification: P05 is a roadmap checkpoint, not a
formal release lane. The installed `0.27.0` runtime remains useful validation
evidence for the active Guacamole/RDP productization campaign, but it should
not create a GitHub release, a docs changelog release entry, or a published
`0.27.0` claim.

## Slice A Preflight

- Graphiti discovery was healthy for advisory repo memory.
- `origin` is the authoritative release repository for this fork.
- `git fetch --tags origin` added the missing local `v0.26.0` and `v0.26.1`
  tags.
- GitHub release `v0.26.1` exists, is not a draft, is not a prerelease, and was
  published on April 30, 2026.
- Selected validation base: `v0.26.1`.
- `v0.26.1` resolves to `4446827b` and is an ancestor of `HEAD`.
- Local tag `v0.25.4` resolves to `2114bdf8` but is not an ancestor of `HEAD`;
  it is not the validation base for this lane.
- Current package version before release prep: `0.26.1`.
- Current `CHANGELOG.md` release markers still wrap the already-published
  `0.26.1` entry and must move to the new release entry.

## Durable Surface Decision

The P05 durable authorities are checkpoint content, not local-only scratch
material:

- `ROADMAP.md`
- `RUNBOOK.md`
- `docs/dev/plans/`
- `docs/dev/policies/0011-graph-backed-memory-usage.md`
- `docs/dev/policies/0012-codegraph-usage.md`
- P03, P04, and P05 dated validation notes under `docs/dev/notes/`

The current worktree still contains many untracked release-lane files. They are
not ignored for checkpoint handoff; validation and handoff must treat them as
part of the active roadmap checkpoint unless a later cleanup explicitly
excludes them.

## Slice B Version Decision

Checkpoint runtime version: `0.27.0`.

Rationale:

- `v0.26.1` already shipped as the latest GitHub release artifact.
- The diff from `v0.26.1` to `HEAD` includes new remote-view, dashboard,
  service-control-plane, browser capability, access-plan, generated client,
  contract, and validation behavior.
- The checkpoint is therefore allowed to advance minor version metadata for
  installed-runtime validation, but this does not make it a formal release.

Contributors from `v0.26.1..HEAD`:

- @ecochran76

## Slice C Version Sync

Command:

```bash
pnpm version:sync
```

Result: passed.

Synchronized version files:

- `package.json`: `0.27.0`
- `cli/Cargo.toml`: `0.27.0`
- `cli/Cargo.lock`: `0.27.0`
- `packages/dashboard/package.json`: `0.27.0`

## Slice D Checkpoint Notes

Checkpoint notes were prepared for the current work.

- `CHANGELOG.md` keeps the current checkpoint work under `## Unreleased`.
- `CHANGELOG.md` keeps `<!-- release:start -->` and `<!-- release:end -->`
  around the latest published `0.26.1` release entry.
- `docs/src/app/changelog/page.mdx` does not list `v0.27.0`, because no formal
  `0.27.0` release has shipped.
- `.github/workflows/release.yml` is manual-only, so pushing to `main` cannot
  publish the checkpoint as a GitHub release.

## Slice E Documentation And Skill Audit

Audited command and privilege wording across:

- `README.md`
- `cli/src/output.rs`
- `skills/agent-browser/SKILL.md`
- `docs/src/app/`

Result: no checkpoint-blocking drift found in the documented command names.

The surfaces refer to the implemented release commands and scripts:

- `agent-browser install doctor`
- `agent-browser doctor remote-view`
- `agent-browser install --with-deps --with-remote-view-privileges`
- `pnpm test:rdp-guac-many-to-many-live`

The privilege wording describes the one-time `agent-browser` group, root-owned
helper, and narrow sudoers rule rather than broad passwordless sudo.

## Slice F Selected Validation

Selector artifact:

```text
docs/dev/notes/2026-05-29-p05-validation-selector.txt
```

Validation commands:

```bash
pnpm validation:select -- --base v0.26.1
git diff --check
pnpm version:sync
node scripts/dev/select-validation.js --base HEAD --json
diff -q skills/agent-browser/SKILL.md /home/ecochran76/.codex/shared/skills/agent-browser/SKILL.md
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo clippy --manifest-path cli/Cargo.toml -- -D warnings
cargo test --manifest-path cli/Cargo.toml service_model -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_health -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_contracts -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_config -- --test-threads=1
cargo test --manifest-path cli/Cargo.toml service_monitors -- --test-threads=1
pnpm test:service-api-mcp-parity
pnpm test:browser-capability-registry-draft
pnpm test:service-client
pnpm --dir docs build
pnpm test:dashboard-view-streams
pnpm test:dashboard-browser-row-actions-render
pnpm test:dashboard-browser-table
pnpm test:dashboard-workspace-nodes
pnpm test:dashboard-launcher-eligibility
pnpm test:dashboard-workspace-navigator
pnpm test:dashboard-inspector-actions
pnpm build:dashboard
```

Result: all passed.

Notes:

- The installed shared skill was stale and was refreshed from
  `skills/agent-browser/SKILL.md`; the sync check passed after refresh.
- The first docs build failed on an MDX parse of
  `AGENT_BROWSER_RDP_ROUTE_A_*` and `AGENT_BROWSER_RDP_ROUTE_B_*` in
  `docs/src/app/service-mode/page.mdx`. Escaping the wildcard text fixed the
  docs build.

## Slice G Installed Runtime Regression Gate

Checkpoint runtime build:

```bash
pnpm build:native
```

Result: passed.

Checkpoint version: `agent-browser 0.27.0`

Checkpoint checksum:

```text
e99093bb46891983afe71c2bf992a5f5c1ded16ecbbd29504a3e9e55a16be33f
```

The installed command, workspace binary, and pnpm global package binary were
synced to the same checksum.

Rollback paths:

```text
/home/ecochran76/.local/bin/agent-browser.bak-20260529T123327Z
/home/ecochran76/.local/share/pnpm/global/5/.pnpm/agent-browser@file+..+..+..+..+..+.agent-browser+releases+agent-browser-0.26.1-preference-guide-20260517121218.tgz/node_modules/agent-browser/bin/agent-browser-linux-x64.bak-20260529T123358Z
```

Installed regression commands:

```bash
agent-browser install doctor --json
agent-browser doctor remote-view --json
agent-browser --json get title
```

Result: all passed.

Doctor summary:

- `agent-browser install doctor --json` reported `success: true`, no issues,
  version `0.27.0`, and matching installed, workspace, and pnpm package binary
  checksums.
- `agent-browser doctor remote-view --json` reported `success: true`, route
  pool ready, route displays ready on `:12` and `:11`, route display access
  ready, simultaneous viewing ready, privileged helper ready, and
  `requiresInteractiveSudo=false`.
- The default-profile attach smoke returned `success: true` and title
  `BufferCLI`.

Many-to-many gate:

```bash
AGENT_BROWSER_REMOTE_VIEW_URL=http://127.0.0.1:8092/guacamole/ \
AGENT_BROWSER_RDP_TEST_CLIENT_A_EXECUTABLE=/usr/bin/google-chrome \
AGENT_BROWSER_RDP_TEST_CLIENT_B_EXECUTABLE=/usr/bin/brave-browser \
pnpm test:rdp-guac-many-to-many-live
```

Result: passed.

Artifact directory:

```text
/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T12-38-46-972Z
```

Earlier failed attempts are retained as useful precondition evidence:

- `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T12-34-20-117Z`
  failed because route-pool variables were not exported.
- `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T12-35-30-643Z`
  failed because viewer client executable variables were not exported.
- `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T12-36-28-362Z`
  reached Guacamole but used the authenticated public URL; rerunning with the
  local Guacamole URL passed.

## Slice H Handoff

P05 prepared a validated installed-runtime checkpoint for `0.27.0`; it did not
publish a GitHub release and did not open a PR.

Next maintainer action: proceed to P06 and harden the installer, doctor,
route-pool, Guacamole/RDP preflight, and many-to-many operational evidence
needed before a formal release milestone.

Residual risk:

- The worktree contains a large checkpoint diff with many previously
  untracked source, docs, plan, policy, script, dashboard, and validation-note
  files that must be included intentionally in the checkpoint commit.
- The GitHub Actions release dry-run was not run because P05 is not a release
  lane.
