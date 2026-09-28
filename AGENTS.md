# AGENTS.md

Instructions for AI coding agents working with this codebase.

## Package Manager

This project uses **pnpm**. Always use `pnpm` instead of `npm` or `yarn` for installing dependencies, running scripts, etc. (e.g., `pnpm install`, `pnpm run build`).

## Code Style

- Do not use emojis in code, output, or documentation. Unicode symbols (✓, ✗, →, ⚠) are acceptable.
- In documentation and markdown, never use double hyphens (`--`) as a dash. Use an emdash (—) sparingly when needed. Prefer rewriting the sentence to avoid dashes entirely.
- CLI colored output uses `cli/src/color.rs`. This module respects the `NO_COLOR` environment variable. Never use hardcoded ANSI color codes.
- CLI flags must always use kebab-case (e.g., `--auto-connect`, `--allow-file-access`). Never use camelCase for flags (e.g., `--autoConnect` is wrong).

## Documentation

When adding or changing user-facing features (new flags, commands, behaviors, environment variables, etc.), update **all** of the following:

1. `cli/src/output.rs` — `--help` output (flags list, examples, environment variables)
2. `README.md` — Options table, relevant feature sections, examples
3. `skills/agent-browser/SKILL.md` — so AI agents know about the feature
4. `docs/src/app/` — the Next.js docs site (MDX pages)
5. Inline doc comments in the relevant source files

This applies to changes that either human users or AI agents would need to know about. Do not skip any of these locations.

In the `docs/src/app/` MDX files, always use HTML `<table>` syntax for tables (not markdown pipe tables). This matches the existing convention across the docs site.

## Dashboard (packages/dashboard)

- Never use native browser dialogs (`alert`, `confirm`, `prompt`). Use shadcn/ui components (`Dialog`, `AlertDialog`, etc.) instead.
- Use param-case (kebab-case) for all file and folder names (e.g., `session-tree.tsx`, not `SessionTree.tsx`). The `ui/` directory follows shadcn conventions which already uses param-case.

## Releasing

Releases are manual, single-PR affairs. There is no changesets automation. The maintainer controls the changelog voice and format.

Normal project work lands in this project's `origin` repository. Do not open release or feature PRs against the original upstream repository unless a maintainer explicitly asks for an upstream contribution.

Normal roadmap checkpoint work may update the local installed runtime and may
advance workspace version metadata for validation, but it is not a formal
release. Formal releases are reserved for explicit maintainer direction after a
fully hardened and operational many-to-many Guacamole/RDP remote operation
milestone is met. That milestone must include a robust installer that requires
sudo exactly once on first install and a fully diagnostic doctor surface.

To prepare a release:

1. Create a branch (e.g. `prepare-v0.24.0`)
2. Bump `version` in `package.json`
3. Run `pnpm version:sync` to update `cli/Cargo.toml`, `Cargo.lock`, and `packages/dashboard/package.json`
4. Write the changelog entry in `CHANGELOG.md` at the top, under a new `## <version>` heading, wrapped in `<!-- release:start -->` and `<!-- release:end -->` markers. Remove the `<!-- release:start -->` and `<!-- release:end -->` markers from the previous release entry so only the new release has markers.
5. Add a matching entry to `docs/src/app/changelog/page.mdx` at the top (below the `# Changelog` heading)
6. Validate the user-scoped install/update surface with `agent-browser install doctor` after installing or replacing the release candidate binary
7. Open a PR and merge to `main`

The `Release` workflow is manually dispatched. With `dry_run=true`, it builds
and verifies all 7 platform binaries without creating a GitHub release. With
`dry_run=false`, it creates or updates the GitHub release for the current
`package.json` version using the content between the `<!-- release:start -->`
and `<!-- release:end -->` markers in `CHANGELOG.md`. npm is not an
authoritative release target for this fork.

To validate the release workflow without creating a release, run the `Release` workflow manually from GitHub Actions with `dry_run` set to `true`. This builds the release binaries, verifies all expected artifacts, and skips GitHub release creation.

After installing a built release candidate locally, run `agent-browser install doctor`. Treat any nonzero result as an install drift or browser-build readiness issue before calling the release candidate validated.

### Writing the changelog

Review the git log since the last release and write the entry in `CHANGELOG.md`. Follow the existing format and voice. Group changes under `### New Features`, `### Bug Fixes`, `### Improvements`, etc. Bold the feature/fix name, then describe it concisely. Reference PR numbers in parentheses.

Wrap the release notes (everything between the `## <version>` heading and the previous version) in markers so CI can extract them for the GitHub release. Only the current release should have markers; remove the `<!-- release:start -->` and `<!-- release:end -->` markers from any previous release entry:

```markdown
## 0.24.1

<!-- release:start -->
### Bug Fixes

- Fixed **baz** not working when qux is enabled (#1235)

### Contributors

- @ctate
<!-- release:end -->

## 0.24.0

### New Features

- **Foo command** - Added `foo` command for bar (#1234)
```

Include a `### Contributors` section listing the GitHub usernames (with `@` prefix) of everyone who contributed to the release. Check the git log between the previous tag and HEAD to find them.

Do not prefix entries with commit hashes. Do not use the changesets `### Patch Changes` / `### Minor Changes` headings. Use descriptive section names instead.

### Docs changelog

The docs changelog at `docs/src/app/changelog/page.mdx` mirrors `CHANGELOG.md` but uses a slightly different format. Each entry uses:

- A `v` prefix on the version (e.g. `## v0.24.0`)
- A date line with the full date: `<p className="text-[#888] text-sm">March 30, 2026</p>`
- A `---` separator between entries

Match the existing style in that file.

## Architecture

This is a Rust workspace. The `agent-browser` binary package lives in `cli/`,
and the focused `agent-browser-cdp` library package owns the CDP websocket
transport, command lifecycle, and protocol types under
`crates/agent-browser-cdp/`. The `agent-browser-lease-authority` library package
owns lease claims, fencing, signing and verification, the protected protocol
and durable store, custody, and pure principal and profile-identity mechanics
under `crates/agent-browser-lease-authority/`. Service State joins and browser
or runtime-owner orchestration remain CLI adapters. The
`agent-browser-desktop-services` library package owns provider-neutral desktop
transaction contracts, deterministic event sequencing, and process-local
route coordination. Service State, durable filesystem, capture, locator, and
platform-input implementations remain CLI adapters. Browser process launch,
Chrome and Lightpanda selection, `BrowserManager`, the automation daemon,
snapshots, and state remain in `cli/src/native/`. The `--engine` flag selects
Chrome vs Lightpanda. The `install` command downloads Chrome from Chrome for
Testing directly.

## Isolated Development Runtime

- Use `agent-browser-dev` and the `development-runtime:*` package scripts for
  experimental installed validation. Do not publish experimental binaries into
  production.
- Development runtime JSON status reports configured listener numbers under
  `ports`; service process identities remain under `units.*.mainPid`. Doctor
  prints the configured port while separately checking listener ownership.
- Build ordinary installed candidates with
  `pnpm build:development-candidate`. It uses the optimized Cargo `ci` profile
  at `cli/target/ci/agent-browser`. Reserve `pnpm build:native` and its full
  release profile for the final production or release gate.
- The development publisher pins a Linux-compatible browser executable. Set
  `AGENT_BROWSER_DEV_BROWSER_EXECUTABLE` only to an absolute reviewed
  executable before installation.
- Run `pnpm smoke:development-browser-launch` after development publication.
  It performs three disposable open, URL-read, close, and residue checks while
  preserving production identity.
- Development service GC requires positive development-environment ownership.
  A process that is merely absent from development Service State is foreign or
  unknown, not a cleanup candidate.
- Use `pnpm development-runtime:skill-sync` to publish repository guidance into
  the development pseudo-home. Do not overwrite the shared user-scoped skill
  when validating experimental behavior.
- The development presentation provider is optional for dashboard-only work.
  Provider-backed acceptance must set
  `AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED=1` and pass the exact
  development provider doctor without borrowing production resources.
- Before staging the development provider, set
  `AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL` to its reviewed public HTTPS origin
  and `AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION` to the immutable reviewed
  Cooper deployment revision or receipt ID. They are required together and
  are bound by a deterministic digest. Read-only status and doctor commands
  reuse a validated binding from an installed v2 provider manifest when both
  variables are absent. Initial staging and provider mutation still require
  the explicit pair; partial or changed explicit values fail closed. Loopback
  is a local diagnostic only.
- Before provider mutation, run `pnpm development-runtime:provider-plan`,
  `pnpm development-runtime:provider-stage`, and
  `pnpm development-runtime:provider-preflight`. Apply only with
  `pnpm development-runtime:provider-apply -- --apply --defer-ingress`, then
  publish Cooper ingress after the provider-ready checkpoint.
- Provider doctor success does not prove Service capacity projection. Require
  non-null `presentationCapacity` from development Service Status before
  running presentation-capacity acceptance.
- Plan 0158 external-vantage evidence runs only through the manually dispatched
  `p158-external-vantage.yml` workflow. Its protected GitHub environment must
  supply the durable handoff, dashboard credentials, expected retained
  identity, prepared synthetic pixel-marker region, and synthetic-only visual
  capture attestation as secrets. Never add
  an automatic trigger or retry to this lane.
  Dispatch calibration with one shared RFC3339 start at least 30 minutes in
  the future; both clients bind to its hashed 20-minute schedule and end.
  The two client jobs must finish successfully before accepting the aggregate
  receipt; partial artifacts are diagnostic evidence only.
- Use `pnpm development-runtime:provider-scale-out -- --apply` and
  `pnpm development-runtime:provider-scale-in -- --apply` for one-route elastic
  lifecycle effects. Scale-out is pressure-admitted. Scale-in requires elapsed
  cooldown and an exact reference-free route. Scale-out must defer unless the
  installed helper already proves exact, idempotent route-session reclamation.
  A quarantined result is a failed
  operation with a retained cleanup obligation, not permission for broad
  process cleanup.

## RDP and Remote-View Handoffs

- Give operators only the authenticated, opaque
  `/remote-view/<handoff-id>` URL returned as `handoffUrl` or the durable
  `externalUrl` on `remote_view_open`.
- Never bookmark, persist, or send a raw Guacamole URL, `providerExternalUrl`,
  `routeBinding` URL, local embed URL, dashboard embed URL, or health URL as an
  operator handoff. Those values describe the current provider route and may
  change during recovery.
- Use `requestServiceRemoteViewHandoff()` when software needs a user-facing
  link. It returns only `handoffId` and `handoffUrl`.
- Require `operatorVisible.state=ready` before saying the browser is visible.
  URL presence alone is not readiness evidence.
- Reconnect by opening the same durable handoff URL. Do not run another
  `remote-view open` merely because the Guacamole route, display, or viewer
  lease changed.
- Do not confuse a remote-view handoff with `agent-browser handoff
  prepare|resume`, which transfers a browser between daemon executables, or
  with a profile-seeding handoff, which guides manual authentication.
- Follow the [RDP remote-view handoff guide](docs/src/app/remote-view/page.mdx)
  for the operator and software-client workflows.

## Worktree Closeout

- Before closing a registered worktree, run:

  ```bash
  pnpm run worktree:closeout inspect \
    --worktree <path> \
    --repository-root <path>
  ```

  The command is advisory and returns the exact worktree identity, candidate
  obligations, and supported dispositions without applying an effect.
- Record one explicit `retain`, `archive`, or `discard` disposition for every
  pinned candidate in the inspection request, then use `begin` to create or
  join the repository-scoped durable operation. A second process must join the
  same request instead of copying or removing the checkout independently.
- Run `apply` without `--apply` to inspect consequences. Use `apply --apply`
  only for the reviewed operation. Interrupted archive or removal work resumes
  the same operation with `--recover`; changed dispositions start no effect and
  return a typed conflict.
- Use `lookup-archive --candidate-id <id>` to resolve a completed archive from
  a fresh process. The locator is separate from the preserved candidate bytes
  and is verified before reuse.
- Do not use the helper to take custody from another active lane, bypass an
  active candidate build, or infer runtime, install, or production authority.
  Raw `git worktree remove` remains an explicit bypass and does not create a
  governed closeout receipt.

## Testing

### Unit Tests

```bash
scripts/ci/rust-tests.sh
```

Runs the comprehensive provider-free Rust lane. The runner keeps every test
serial inside its process, gives every CLI process a disposable home and XDG
runtime tree, and partitions CLI tests into disjoint action, browser, service,
stream, other native, workstation, and remaining core compartments. Two
balanced lanes overlap through the existing Cargo admission wrapper. The
support lane then runs the Candidate, Challenge Control, Desktop Services,
Lease Authority, and CDP crates plus the CLI integration-test binaries.
First-failure logs remain separate and are printed before each compartment
result.

Use `scripts/ci/rust-tests.sh --focused <filter>` during implementation or to
re-run one failed invariant. Use `--compartment <name>` for `lease-authority`,
`candidate`, `challenge-control`, `desktop-services`, `transport`, `cli-native`, one of the narrower `cli-native-*` groups, `cli-workstation`,
`cli-core`, or `cli-integration`, and `--list-compartments` for machine-readable
discovery. The runner defaults
`RUST_MIN_STACK` to 16 MiB so state-heavy async fixtures do not require
per-test source wrapping. Override it only through
`RUST_TEST_STACK_SIZE_BYTES` when measuring a reviewed test-stack change.

On WSL, every Cargo command that can compile code must run through
`scripts/ci/cargo-safe.sh`. The wrapper admits up to two concurrent repository
Cargo invocations when current memory, swap, CPU, disk, and active claims can
preserve the configured host reserve. Low free swap is treated as current
pressure only when available memory cannot also cover the missing swap reserve;
stale swapped pages alone do not block admission. Each invocation defaults to
eight Cargo jobs and runs in a user-systemd scope with `MemoryHigh=20G`, `MemoryMax=24G`,
and `MemorySwapMax=4G`; all admitted scopes share an aggregate
`agent-browser-cargo.slice` capped at `MemoryHigh=28G`, `MemoryMax=32G`, and
`MemorySwapMax=4G`. It fails closed when the WSL user-systemd manager is
unavailable. Do not invoke `cargo check`,
`cargo build`, `cargo clippy`, or `cargo test` directly from WSL agent sessions.
Set `AGENT_BROWSER_CARGO_BUILD_JOBS` only when a particular build needs a
different bounded parallelism level. Do not set it to `2` merely because the
wrapper admits at most two concurrent Cargo invocations: invocation concurrency
and the number of build jobs inside each invocation are independent controls.
Capacity admission holds an exclusive
lock only while reconciling claims; Cargo does not hold that lock. A third
invocation waits with a typed pressure reason, and admission automatically
drops below two when current resources cannot preserve the reserve.
On native Linux CI runners, the wrapper skips WSL host admission and cgroups,
then executes Cargo with the configured build jobs and acceleration settings.
The wrapper automatically uses `sccache` and `mold` for native Linux builds
when those exact executables are available. Set `AGENT_BROWSER_CARGO_CACHE=off`
or `AGENT_BROWSER_CARGO_FAST_LINKER=off` for a deterministic opt-out. Run
`pnpm benchmark:cargo-build-jobs` for an isolated 4/6/8-job comparison. The
benchmark uses disposable target directories and never cleans the shared
target directory.

### End-to-End Tests

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml e2e -- --ignored --test-threads=1
```

Runs the ignored e2e suite that launches real headless Chrome instances and exercises the full native daemon command pipeline. Requirements:

- Chrome must be installed
- Must run serially (`--test-threads=1`) to avoid Chrome instance contention
- Tests are `#[ignore]`'d so they don't run during normal `cargo test`
- E2E tests must not depend on or mutate the default runtime profile at `~/.agent-browser/runtime-profiles/default/user-data`. The CI job sets `AGENT_BROWSER_PROFILE` to a runner-local temp profile for the whole suite. Tests that need a dedicated profile should use `e2e_temp_profile()` and clean it up.

The e2e tests live in `cli/src/native/e2e_tests.rs` and cover: launch/close, navigation, snapshots, screenshots, form interaction, cookies, storage, tabs, element queries, viewport/emulation, domain filtering, diff, state management, error handling, and Phase 8 commands.

### CI Cadence

GitHub CI is temporarily disabled by explicit operator direction. The dormant
workflow is retained as `.github/workflows/ci.yml.disabled`; there is no active
`.github/workflows/ci.yml`, automatic trigger, schedule, or manual CI dispatch.
Do not re-enable or dispatch CI without new maintainer direction.

When re-enabled, pull requests use the versioned changed-surface classifier and
selected Documentation, Version Sync, Dashboard, Service Client, Repository
Tooling, Rust Quality, affected Rust compartments, and Workstation Fixtures
jobs. The stable
`Presubmit` aggregate fails when a selected job is skipped, cancelled, or
failed, and records the tier, exclusions, elapsed time, and observed runner
minutes without describing the selected lane as comprehensive. Unknown paths
and changes to dependencies, toolchains, the classifier, or the workflow fail
safe to the broad ordinary presubmit. The dormant workflow has no `main` push
trigger, schedule, manual dispatch, commit-message escape hatch, comprehensive
Rust job, or slow platform matrix. Pull-request concurrency cancels an older
run when a newer head for the same pull request starts. Issue #164 owns live
enforcement of `Presubmit` if CI resumes.

Documentation and governance-only changes run patch hygiene, policy and
planning audits, changed-link validation, and the docs build without
application suites. Service Client runs
`pnpm test:browser-capability-registry-draft` and `pnpm test:service-client`,
which check the draft browser capability registry sample, generated service
client files, JavaScript type coverage, service request helper contracts,
service observability helper contracts, managed-profile flow contracts, and
the no-launch service-client example broker-first contract without launching
Chrome. Repository Tooling runs `pnpm test:repository-tooling` for the
provider-free candidate-build and worktree-closeout contracts without building
Rust or launching Chrome. Dashboard action-surface changes should run
`pnpm test:dashboard-inspector-actions` so the Service right-pane inspector
keeps selected-record state separate from mutable incident and job actions.
Rust Quality runs Linux format and clippy before selected Rust compartments, so
style or lint failures fail fast. `scripts/ci/rust-tests.sh --compartment`
keeps each compartment serial inside its process and gives every CLI
compartment a disposable home and XDG runtime tree; at most two disjoint
lanes overlap through Cargo admission. CLI compartments run serially in one
lane so Cargo cannot replace a running shared `agent-browser` test executable;
independent crate compartments run serially in the other lane. When a service-owned Rust surface
is selected, the Rust job then builds one exact-head debug CLI and pins every
command-based no-launch smoke to that binary. Unrelated focused Rust changes
skip that service smoke bundle. `scripts/ci/rust-tests.sh` without a
compartment remains a local comprehensive provider-free command, not a GitHub
CI route. Service request action changes
must keep `cli/src/native/service_contracts.rs` `SERVICE_REQUEST_ACTIONS`,
`docs/dev/contracts/service-request.v1.schema.json`, MCP `service_request`,
HTTP `/api/service/request`, and generated `@agent-browser/client` helpers
aligned; the selected parity, client, and service-Rust gates include no-launch guards for
that invariant. The selected service-Rust path also runs the no-launch service contract metadata
smoke, no-launch MCP resource-read smoke, no-launch profile-source smoke,
no-launch site-policy source smoke, and no-launch HTTP and MCP incident-summary
smokes after the Rust suite, so the service contracts, MCP read resources,
effective profile and site-policy provenance, and grouped incident summary
contracts stay covered without starting Chrome. Set `CARGO_TEST_PROFILE=ci`
when intentionally validating the optimized CI profile locally.

The path-filtered Lease Authority matrix is also disabled and retained as
`.github/workflows/lease-authority.yml.disabled`. When explicitly re-enabled,
its non-fail-fast matrix runs the `agent-browser-lease-authority` package
directly on Linux, macOS ARM, macOS x86, and Windows. Until then, do not claim
target-platform CI evidence for the extracted crate.

At a completed repair batch, before merge readiness or governed runtime
effects, match validation to every touched surface since the batch baseline,
not only the final commit. Follow policy 0042 for intermediate custody commits
and reuse of passed gates. If any Rust source under
`cli/src/` or `crates/` changed in the current slice, run
`scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check` and
`scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`. If a service
schema, service model, output formatter, service contracts metadata, or
generated client changed, also run the focused Rust or client contract tests
for that surface. Recent CI failures were caused by skipping these gates after
Rust and contract-shape changes, then pushing follow-up docs or client commits
that could not fix the already-broken fast CI baseline.

Use `pnpm validation:select -- --base <ref>` to print recommended local checks
from changed file paths before push. The default base is `HEAD`, which is useful
for staged or uncommitted work. For a whole slice, pass the last known green
commit or another explicit base ref. Run `pnpm run test:validation-selection`
after changing the selector, CI workflow, aggregate verifier, documentation
link checker, economics receipt, or Rust compartment inventory.

Do not babysit GitHub Actions as part of normal implementation closeout. CI
evaluation is a separate task and should only include active waiting, log
analysis, reruns, or further CI tuning when the maintainer explicitly asks for
CI evaluation or release gating. For ordinary commits, it is acceptable to do
one lazy status check for a run that should already have completed, then report
the current state without waiting. If the run is still queued or in progress,
leave the run URL and move on unless the user asked for active monitoring.

### Linting and Formatting

```bash
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml
```

## Graphiti Memory Discovery

- Use the `graphiti-discovery` skill at the start of non-trivial planning,
  service-roadmap, architecture, debugging, or handoff work.
- Query repo memory group `agent_browser_main` before assuming prior context
  only exists in chat history or git history.
- Treat Graphiti as advisory memory; verify cited facts against repo files,
  notes, commits, or tests before changing code or live systems.
- Seed or refresh memory only from curated source-backed artifacts such as
  `docs/dev/notes/`, roadmap notes, bounded plans, validation reports, and
  release checkpoints.
- Keep memory episodes compact, atomic, and source-oriented. Do not seed raw
  command logs, secrets, auth state, browser artifacts, private site data, or
  every small commit.

## Product Lane Routing

- Before creating a substantive plan, branch, or worktree, select one primary
  product lane from `docs/dev/product-lanes.md`.
- Record `Product lane: <ID>` in the plan and `product_lane: <ID>` in the active
  lane catalog. Product lanes own long-lived boundaries; numbered plans own one
  bounded delivery outcome.
- Use the lane's branch prefix and default worktree limit. Do not let a bugfix
  become a shadow architecture or feature lane.
- When two lanes need a shared source or documentation surface, record one
  primary writer and an explicit dependency or overlap before either agent
  edits it. Merge the provider contract before dependent consumers.
- Route field notes through `docs/dev/notes/README.md`. Notes preserve evidence;
  they are not branch custody, execution authority, or a second backlog.

## Windows Debugging

A remote Windows Server 2022 EC2 instance is available for debugging Windows-specific issues. It uses AWS Systems Manager (SSM) with no SSH or open ports. Commands run via `aws ssm send-command` and return stdout/stderr.

### Prerequisites

The instance must be provisioned first (one-time, by a human):

```bash
./scripts/windows-debug/provision.sh
```

Requires: AWS CLI v2 configured with `ec2:*`, `iam:CreateRole`, `iam:AttachRolePolicy`, `ssm:SendCommand`, `ssm:GetCommandInvocation` permissions and a default VPC.

### Usage

Start the instance (if stopped):

```bash
./scripts/windows-debug/start.sh
```

Run a command on Windows:

```bash
./scripts/windows-debug/run.sh "<powershell-command>"
```

Sync the current git branch and rebuild:

```bash
./scripts/windows-debug/sync.sh
```

Stop the instance when done (avoids cost):

```bash
./scripts/windows-debug/stop.sh
```

### Common Workflows

Run unit tests on Windows:

```bash
./scripts/windows-debug/run.sh "cd C:\agent-browser && cargo test --manifest-path cli\Cargo.toml"
```

Run e2e tests on Windows:

```bash
./scripts/windows-debug/run.sh "cd C:\agent-browser && cargo test e2e --manifest-path cli\Cargo.toml -- --ignored --test-threads=1"
```

Check bootstrap progress (first boot only):

```bash
./scripts/windows-debug/run.sh "Get-Content C:\bootstrap.log"
```

The repo lives at `C:\agent-browser` on the instance. Rust, Git, and Chrome are pre-installed. The `run.sh` wrapper automatically adds cargo and git to PATH.

<!-- opensrc:start -->

## Source Code Reference

Source code for dependencies is available in `opensrc/` for deeper understanding of implementation details.

See `opensrc/sources.json` for the list of available packages and their versions.

Use this source code when you need to understand how a package works internally, not just its types/interface.

### Fetching Additional Source Code

To fetch source code for a package or repository you need to understand, run:

```bash
npx opensrc <package>           # npm package (e.g., npx opensrc zod)
npx opensrc pypi:<package>      # Python package (e.g., npx opensrc pypi:requests)
npx opensrc crates:<package>    # Rust crate (e.g., npx opensrc crates:serde)
npx opensrc <owner>/<repo>      # GitHub repo (e.g., npx opensrc vercel/ai)
```

<!-- opensrc:end -->

## Policy Loading Contract

- `AGENTS.md` is a routing surface, not a one-time pointer.
- When a task has a checked-in profile under `docs/dev/policy-capsules/`, run
  its deterministic `--check` command and use the generated capsule as the
  operative admission read. Reuse it while its policy hashes and listed
  re-read triggers remain unchanged. Read the canonical policy files when the
  check fails, a trigger fires, or the capsule does not resolve an ambiguity.
  A capsule is derived guidance and never overrides canonical policy.
- When no current capsule applies, re-read the relevant policy files under
  `docs/dev/policies/` at the start of any non-trivial turn.
- Re-read the relevant policy files when task scope changes mid-session.
- When behavior is ambiguous, prefer re-reading policy over improvising from stale assumptions.

## Policy Re-read Triggers

- Before substantive bug hunting or feature work, read policies
  `0028-goal-execution-governance.md`, `0042-code-testing-discipline.md`, and
  `0045-model-selection-and-calibration.md` for delivery bounds, batch validation,
  and economical task routing; read `0021-subagent-workflow-optimization.md`
  when considering delegation. These apply to ongoing continuations too.
- Read policy `0044-planning-discipline.md` before substantive planning, a
  second related defect, or another expensive build, deployment, or acceptance
  cycle. It routes consolidation, evidence, delivery budgeting, and worker
  assignments.
- Read policies `0047-multi-session-development-operating-model.md` and
  `0050-collaborative-development-workflow.md` before
  starting or resuming a substantive development lane, assigning top-level session ownership,
  delegating work, creating a lane runtime, or integrating parallel branches.
- Read policy `0051-shared-runtime-effect-custody.md` before any production or
  staging install, recovery, reconcile, supervisor, ingress, browser, profile,
  or Service State mutation. A prior chat checkpoint or file lock is not live
  effect custody.
- Read policy `0052-session-and-worktree-admission.md` at the start and closeout
  of every top-level development session and before any primary, reviewer,
  benchmark, fixture, red-team, or replacement worktree is created or
  assigned. Serialize conflicting Git transitions; do not invent a permanent
  coordinator or permission role.
- Read policies `0048-forge-issue-reporting.md` and
  `0049-github-issue-operations.md` before any issue provider mutation,
  including settings, labels, issue creation, editing, assignment, planning,
  closure, or transfer.
- re-read planning-related policy before opening, revising, or closing a substantive plan
- re-read documentation-related policy before changing docs, contracts, or canonical authorities
- re-read validation and closeout policy before claiming work complete
- re-read runtime or environment-boundary policy before touching live state, tenant state, deploy state, or off-repo operator data
- re-read branch, commit, and integration policy before starting a multi-file or multi-step implementation slice

## Policy Entry

This repo keeps its durable repo-local policy under `docs/dev/policies/`.

Read and follow:
- `docs/dev/policies/0001-policy-management.md`
- `docs/dev/policies/0002-policy-upgrade-management.md`
- `docs/dev/policies/0003-policy-adoption-feedback-loop.md`
- `docs/dev/policies/0004-git-worktree-hygiene.md`
- `docs/dev/policies/0005-commit-history-discipline.md`
- `docs/dev/policies/0006-branch-and-integration-strategy.md`
- `docs/dev/policies/0007-commit-and-push-cadence.md`
- `docs/dev/policies/0008-versioning-and-release.md`
- `docs/dev/policies/0009-turn-closeout.md`
- `docs/dev/policies/0010-validation-and-handoff.md`
- `docs/dev/policies/0011-graph-backed-memory-usage.md`
- `docs/dev/policies/0012-codegraph-usage.md`
- `docs/dev/policies/0014-website-surface-targeting.md`
- `docs/dev/policies/0015-db-backed-state-governance.md`
- `docs/dev/policies/0016-live-drift-reconciliation.md`
- `docs/dev/policies/0017-backup-and-recovery-operations.md`
- `docs/dev/policies/0018-visual-release-qa.md`
- `docs/dev/policies/0019-web-interface-quality.md`
- `docs/dev/policies/0020-documentation-change-control.md`
- `docs/dev/policies/0021-subagent-workflow-optimization.md`
- `docs/dev/policies/0022-preview-artifact-review.md`
- `docs/dev/policies/0023-upstream-fork-maintenance.md`
- `docs/dev/policies/0024-notes-and-memories.md`
- `docs/dev/policies/0026-subagent-runtime-governance.md`
- `docs/dev/policies/0028-goal-execution-governance.md`
- `docs/dev/policies/0029-parallel-plan-design.md`
- `docs/dev/policies/0031-runtime-vs-product-boundary.md`
- `docs/dev/policies/0032-runtime-state-governance.md`
- `docs/dev/policies/0033-tenant-isolation-and-operator-state.md`
- `docs/dev/policies/0034-fieldwork-productization.md`
- `docs/dev/policies/0035-monolith-extraction-discipline.md`
- `docs/dev/policies/0036-architecture-guardrails.md`
- `docs/dev/policies/0037-active-lane-coordination.md`
- `docs/dev/policies/0038-multi-agent-reconciliation.md`
- `docs/dev/policies/0039-policy-harvest-loop.md`
- `docs/dev/policies/0042-code-testing-discipline.md`
- `docs/dev/policies/0043-roadmap-runbook-governance.md`
- `docs/dev/policies/0044-planning-discipline.md`
- `docs/dev/policies/0045-model-selection-and-calibration.md`
- `docs/dev/policies/0046-work-item-traceability.md`
- `docs/dev/policies/0047-multi-session-development-operating-model.md`
- `docs/dev/policies/0048-forge-issue-reporting.md`
- `docs/dev/policies/0049-github-issue-operations.md`
- `docs/dev/policies/0050-collaborative-development-workflow.md`
- `docs/dev/policies/0051-shared-runtime-effect-custody.md`
- `docs/dev/policies/0052-session-and-worktree-admission.md`

## Scope

- `AGENTS.md` includes repo-local guidance plus the policy entry section.
- The durable policy body lives under `docs/dev/policies/`.
- Keep repo-specific commands, environment details, and operational caveats in this file or adjacent local docs.

## Tenant Boundary Reminder

- Keep tenant-scoped or user-scoped runtime state out of the product repo unless the repo's runtime-state policy explicitly says it belongs in a separately governed tracked state surface.
- Re-check boundary policy before copying runtime facts, artifacts, or fieldwork output into tracked repo files.
