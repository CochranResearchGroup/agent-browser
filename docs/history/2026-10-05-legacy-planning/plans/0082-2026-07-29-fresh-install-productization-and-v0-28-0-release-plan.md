# Plan 0082: Fresh Install Productization and v0.28.0 Release

Date: 2026-07-29
Status: Complete
Lane: P82
Predecessors: P06, P07, P79, P80, and P81
Release branch: `prepare-v0.28.0`
Target version: `0.28.0`
Operator authorization: explicit in the initiating request

## Goal

Turn the current source-checkout-dependent workstation into a reproducible
fresh installation from the supported GitHub release binary, validate the
complete installation on a disposable Ubuntu host across a reboot, repair every
release-blocking defect found, merge the single release pull request, publish
GitHub release `v0.28.0`, and verify the public asset by reinstalling it without
a repository checkout.

## Release Eligibility Diagnosis

- Public release `v0.27.0` points to commit `17a284f` from 2026-05-29.
- Current `main` at plan intake is `ffda60dd`, 107 commits beyond that tag, but
  every version surface still reports `0.27.0`.
- The latest route-state repair exists only on current `main`. Normal
  convergence must continue to project successful route-readiness JSON into
  `service reconcile` so stable route A resolves to `guacamole:1/:11`.
- `agent-browser install` currently installs Chrome, browser libraries, and the
  privilege helper. It does not install the complete Guacamole, PostgreSQL,
  XRDP, dashboard, backup, or convergence substrate.
- The working dashboard interlock uses the repository as its
  `WorkingDirectory` and invokes `pnpm converge:local-runtime`. A binary-only
  fresh install cannot reproduce it.
- The complete Guacamole compose file, initialization schema, and live secrets
  currently exist outside the repository. Repository scripts assume that
  substrate already exists.
- The current fresh-route path is circular: convergence invokes route sync,
  while sync requires route-specific users that only a separate guarded setup
  path can create.
- Current fast CI is not green. Rust Quality run `30389881775` reports four
  Clippy errors and skipped downstream Rust validation.
- Release workflow dry-run validation checks filenames and minimum sizes, while
  the real release accepts at least seven assets and does not prove an exact
  set, executable version, checksum, or installation.
- README still recommends a `v0.26.1` download.
- The existing untracked `--full-page` file is operator-owned material. It is
  excluded from this plan and must not be modified, removed, or staged.

The release is a feature release because it contains a large new service,
dashboard, remote-operation, durability, and installation surface. The correct
next version is `0.28.0`, not `0.27.1`.

## Product Contract

### Public entry point

Add a Linux-only workstation installation surface under
`agent-browser install workstation` with:

- `--dry-run` for a no-mutation plan;
- `--apply` for an explicit installation;
- `--json` for a durable machine-readable receipt;
- configurable local dashboard and Guacamole ports;
- a single first-install `sudo -v` authorization boundary;
- fail-closed dependency and topology checks;
- idempotent reinstall and upgrade behavior.

The existing browser-only `agent-browser install` behavior remains compatible.

### Installed ownership

- The installed release binary is the authoritative controller.
- Versioned support assets live below
  `~/.local/lib/agent-browser/<version>/` with a recorded manifest and hashes.
- User systemd units reference only the installed binary or versioned installed
  assets. They must not reference a repository path, pnpm, or mutable source
  checkout.
- Root-owned work is limited to the reviewed privilege helper: dependency
  installation, local route users, XRDP service repair, and display access.
- User-owned orchestration manages secrets, Docker Compose, schema and route
  records, viewer clients, retained service reconciliation, dashboard service,
  interlock, and backups.
- The recurring interlock runs a binary-owned reconciliation command and
  preserves P81's readiness-gated authoritative route-pool projection.

### Fresh substrate

The installer must materialize and reconcile:

- pinned Guacamole, guacd, and PostgreSQL containers;
- a complete pinned PostgreSQL initialization schema;
- an external named PostgreSQL volume with continuity identity;
- mode-safe generated PostgreSQL, Guacamole administrator, and distinct route
  user credentials;
- two route-specific Linux users with idle Openbox XRDP sessions;
- two canonical Guacamole connections with distinct route usernames;
- live distinct route displays and operator X access;
- the embedded dashboard user service;
- runtime interlock and daily PostgreSQL backup timers;
- local-only default ingress, with public ingress left as an optional operator
  concern.

## Scope

- repair the current red Rust Quality baseline;
- implement the self-contained workstation install and recurring reconcile
  surfaces;
- bundle or generate every required non-secret support artifact;
- make dependency failures nonzero and fail closed;
- add installed-asset manifest and provenance to install doctor;
- add deterministic empty-host bootstrap distinct from migration `--force`;
- preserve database identity, secrets, route IDs, active allocations, browser
  profiles, and healthy displays on rerun;
- refuse partial schema, ambiguous managed rows, conflicting active route
  allocation, or continuity drift;
- add container fixtures plus an authoritative disposable Ubuntu VM harness;
- verify one sudo prompt, reboot recovery, idempotency, backup restore, route
  reconciliation, and many-to-many operation;
- strengthen the release workflow to validate the exact asset set, sizes,
  versions, and checksums;
- bump and synchronize all version surfaces to `0.28.0`;
- update both changelogs, README, CLI help, docs site, inline docs, and agent
  skill guidance;
- create structured commits, push the release branch, open and merge the single
  release pull request, require green fast and full CI, run the release dry
  run, publish the release, and verify the public asset;
- record repo-native and Graphiti closeout evidence.

## Non-Goals

- changing source-site authentication or probing private accounts;
- consuming a Plan 0012 request ID;
- publishing to npm, Homebrew, Cargo, or the original upstream repository;
- configuring public reverse proxy, Authelia, DNS, or certificates;
- removing operator-owned files or old forensic database material;
- weakening the active-allocation, database-continuity, or route-readiness
  interlocks to make installation appear successful.

## Execution Packets

### Packet A | Authority and release audit

Status: completed

- Re-read repository policy and release authority.
- Query advisory Graphiti memory and verify it against current files and live
  readbacks.
- Compare public `v0.27.0`, current `main`, installed runtime, and GitHub
  workflow state.
- Run three independent read-only audits for installer architecture,
  clean-install testing, and version/release eligibility.

### Packet B | Baseline repair and red tests

Status: completed

- Repair the four existing Clippy failures without widening product behavior.
- Add red tests for the public command contract, embedded asset manifest,
  source-free user units, deterministic empty-host bootstrap, exact route-state
  projection, dependency failure, and idempotent materialization.
- Add a release-workflow test that rejects version-marker mismatch and an
  inexact asset set.

### Packet C | Self-contained workstation installer

Status: completed

- Add command parsing, dry-run, apply, JSON receipt, and install phase model.
- Embed versioned compose, schema, configuration, durability, and privileged
  support assets.
- Implement one-sudo dependency and host preparation.
- Generate mode-safe secrets and user-owned runtime directories.
- Bootstrap and health-check the pinned Guacamole stack and named volume.
- Create route-specific users and canonical rows through a positively proven
  empty-host path.
- Install dashboard, interlock, and backup units bound to installed artifacts.

### Packet D | Binary-owned convergence

Status: completed

- Move recurring decision logic behind an installed CLI command.
- Require successful route-readiness output before authoritative projection.
- Preserve active route conflicts and concurrent lease changes.
- Restore missing route displays and access only through typed doctor-proven
  remedies.
- Persist a structured convergence receipt and expose its provenance in
  install doctor.

### Packet E | Clean-install and recovery harness

Status: completed

- Add fast isolated-HOME payload, mutation, prompt-meter, and failure-injection
  fixtures.
- Add a disposable Ubuntu 24.04 VM harness with snapshot/reset support.
- Install from a candidate artifact with no source checkout or pnpm.
- Prove complete substrate, distinct routes, no-launch canonical selection,
  active-conflict fail-closed behavior, reboot recovery, backup restore,
  idempotent reinstall, and many-to-many operation.

### Packet F | Independent implementation audit

Status: completed

- Assign read-only reviewers to installer safety, test sufficiency, release
  workflow, and documentation parity.
- Convert every blocking finding into a focused repair and rerun the affected
  gate.
- Record reviewer handles, scopes, findings, repairs, and verdicts.

### Packet G | Version and release candidate

Status: completed

- Set root version to `0.28.0` and run `pnpm version:sync`.
- Move the sole changelog release markers to the new `0.28.0` entry.
- Add the matching `v0.28.0` docs changelog entry dated July 29, 2026.
- Update supported GitHub download examples.
- Run selector-recommended checks from `v0.27.0`, fast CI, full CI, installed
  release-candidate doctors, and live route gates.
- Create structured commits, push `prepare-v0.28.0`, open the single release
  pull request, and merge the exact reviewed head to `main`.

### Packet H | GitHub release and public-asset proof

Status: completed

- Dispatch `Release` with `dry_run=true` on the exact merged commit.
- Verify all seven platform builds and the final dry-run gate.
- Download the Linux x64 dry-run artifact and repeat the source-free install
  smoke.
- Dispatch `Release` with `dry_run=false` on the same commit.
- Verify tag SHA, release state, notes, exact seven-asset inventory, sizes, and
  checksums.
- Download the public Linux x64 asset without checkout credentials, verify
  checksum and version, reinstall it, and rerun install doctor plus no-launch
  route proof.

### Packet I | Closeout

Status: completed

- Record final VM, CI, workflow, release, asset, installed-runtime, and Git
  evidence in a dated validation note.
- Close P82 in the roadmap and add the final runbook turn.
- Store one compact source-backed Graphiti memory.
- Commit and push closeout separately, then report release-tag and post-release
  documentation commit identities independently.

## Implementation Checkpoint | 2026-07-29

- Rust Quality is green on Rust 1.97 after the bounded baseline repairs.
- The release workflow now verifies the exact seven-asset inventory, embedded
  versions, minimum sizes, Linux execution, deterministic checksums, and
  published downloads.
- The release binary embeds the pinned Guacamole Compose, normalized schema,
  versioned controller helpers, and source-free systemd user units.
- Host preparation has an Ubuntu 24.04 amd64 preflight, apt removal guard,
  exactly one `sudo -v` boundary, noninteractive privileged calls, required
  service checks, and a resumable group-refresh stop.
- Installed reconciliation now creates route users, starts the pinned stack,
  records PostgreSQL continuity, opens readiness-selected distinct displays,
  projects only canonical `guacamole:1` and `guacamole:2` routes, rejects
  active legacy conflicts, activates user units, runs final doctors, and writes
  a private receipt.
- Focused Rust tests, strict Clippy, the source-free payload fixture, embedded
  asset fixture, host-provision fixture, and privilege fixture pass locally.
- Release remains no-go. The disposable Ubuntu reboot and restore proof,
  independent final audit, full CI, release dry run, and
  public artifact reinstall remain open.

## Implementation Checkpoint | 2026-07-30

- Version surfaces are synchronized at `0.28.0`, the sole changelog markers
  belong to the new entry, and release PR 7 owns the release notes.
- The release-mode binary reports `0.28.0` and embeds 80 dashboard assets.
- Packet F audits found four installer blockers: credentials in subprocess
  arguments, incomplete payload hash verification, inherited route-pool
  override state, and a reconciliation lock acquired after apply mutation.
- Repairs move secret form data and route values to stdin, bind doctor
  readiness to binary and support-asset SHA-256 provenance, clear ambient
  route-pool JSON for installed reconciliation, and hold one install-wide lock
  before quiescence and staging.
- Independent installer-safety, evidence-sufficiency, and release-workflow
  rechecks all passed after the repairs. The source-free fixture, 10 focused
  workstation installer tests, 3 payload-status tests, and diff validation
  passed against the rebuilt current-worktree binary.
- The validation selector and fast CI now include the workstation, durability,
  route-sync, release-asset, and changelog-binding fixtures. Version sync also
  validates the `agent-browser` Cargo.lock entry.
- One iterative release-mode VM run stopped fail-closed during route opening.
  A direct redacted rerun selected canonical `guacamole:1` and
  `guacamole:2`, opened distinct `:10` and `:11` displays, and confirmed the
  remaining work is a final clean-overlay candidate run, not an authentication
  probe.
- The first authoritative clean overlay exposed a too-small 3.5 GiB cloud
  disk during package unpack. The harness now grows clean overlays to 24 GiB,
  while real-host installer preflight requires 6 GiB free and fails before
  sudo, payload staging, or package mutation.
- The resized overlay proved exactly one installer sudo prompt, exit-75
  relogin handling, a changed reboot ID, effective groups, and a zero-prompt
  continuation. That continuation then stopped fail-closed when Guacamole
  header auto-account creation raced across a first-start JVM crash. The
  repair waits for the full application, submits one request, and verifies
  the exact PostgreSQL user postcondition.
- The next clean continuation passed the repaired account gate and opened both
  route displays, then stopped on `systemctl reset-failed` because the newly
  written interlock service was not loaded. A resumed run proved that
  `systemctl show` can synthesize `LoadState=loaded` from a static unit file
  even when the user manager has no loaded unit. Activation now observes
  `is-failed` output independently of its state-bearing exit status, resets
  only an exact `failed` result, and accepts a reset race only after a second
  read proves the unit is no longer failed.
- The resumed candidate then exposed an executable-handoff cleanup race. The
  retiring daemon removed the replacement daemon's socket and session metadata
  after the replacement rebound the shared session path. Daemon shutdown now
  verifies the bound socket device and inode before removing any session
  artifact. The focused ownership regression and live executable-handoff smoke
  pass.
- The next exact-candidate clean overlay passed dry run, one-sudo host
  preparation, exit-75 relogin, reboot, effective groups, container and route
  convergence, user-unit activation, and install doctor. Final remote-view
  doctor then exposed missing `xdpyinfo` plus a legacy host-guacd assumption.
  Host preparation now includes `x11-utils`, ImageMagick, and Tesseract;
  readiness accepts the pinned running Guacd container and discovers managed
  Chrome outside `PATH`.
- The rebuilt exact candidate passed fresh install, reboot continuation,
  idempotent rerun, checksummed backup, and isolated restore. The first live
  Route A open selected `guacamole:1`, connection `1`, and `:10`, then failed
  before DevTools because Ubuntu 24.04 AppArmor denied managed Chrome's
  sandbox user namespace. Lease rollback restored the route.
- Host preparation now installs and loads a path-scoped AppArmor `userns`
  policy for the managed Chrome directory. The repair preserves both the host
  restriction and Chromium sandbox. Remote-view doctor exposes and gates on
  that policy. The same VM subsequently opened Route A with
  `operatorVisible=ready`.
- While Route A was checked out, a conflicting authoritative
  `guacamole:999/:99` definition was skipped. The retained entry remained
  byte-for-byte unchanged at `guacamole:1/:10`, with matching pre/post SHA-256
  `207ff06af5a214ee29a6cce2f2a8385f39db1e63048e2802310f627fcaef164f`.
  Cleanup returned Route A to `available` with no allocation.
- The next clean exact candidate installed the AppArmor policy from its
  embedded payload, reported the profile loaded before and after reboot, and
  completed the post-reboot continuation without a prompt. Its later
  standalone doctor exposed that versioned installed helper scripts were not
  in the discovery path outside install-time command environment. Doctor
  discovery now includes
  `~/.local/lib/agent-browser/<version>/scripts`, with a focused regression.
- Commit `ce26f0f6` rebuilt to the exact release-mode SHA-256
  `06e3b85ebc734c914ad8937afe0f169107cd6e646f5c129ebe1d7afe29aacca2`.
  Its first same-host convergence stopped fail-closed on two viewer daemons
  still running the preceding candidate. The emitted per-session close
  remedies removed only those stale runtimes; the bounded retry passed with
  exact candidate and installed hashes, an active dashboard, and an active
  interlock timer.
- A new login shell then passed standalone install and remote-view doctors.
  Doctor selected
  `/home/agent/.local/lib/agent-browser/0.28.0/scripts`, reported no issues,
  and proved remote control, many-to-many prerequisites, and the managed
  Chrome sandbox policy ready.
- The exact candidate opened `about:blank` through Route A at
  `guacamole:1`, connection `1`, display `:10`, and
  `operatorVisible.state=ready`. Cleanup returned Route A to `available` with
  no current route allocation.
- Whole-slice local validation passed after the full Rust suite exposed one
  stale installer ordering assertion. The production call order was already
  correct; the test still searched for the pre-argument helper signature.
  Updating that assertion restored the full serialized Rust harness. Strict
  Clippy, formatting, installer fixtures, service contracts, dashboard and
  docs builds, live CDP tab streaming, release verifier, version sync, and
  shared-skill parity are green.
- Exact-head fast CI run `30540857427` passed version sync, Rust Quality, the
  full Rust suite, dashboard, service-client, and workstation fixtures, then
  failed its no-launch smoke step. The smoke still expected the older profile
  lookup MCP template and ranking list, plus daemon-created service state.
  The contract assertions now share the current generated expectations, and
  the no-launch fixture accepts the offline empty control-plane snapshot and
  creates minimal seed state when no daemon has written one. The complete
  ten-command no-launch packet passes locally.
- Exact-head fast CI run `30541737279` passed every required job at commit
  `0cbd1729`. Manually dispatched full CI run `30542411936` then reached its
  cross-platform matrix and exposed a macOS Apple Silicon compile defect:
  `statvfs.f_bavail` is 32-bit there while Linux exposes the field as 64-bit.
  Filesystem byte calculation now converts platform field widths through a
  typed helper before saturating multiplication. The adjacent Linux-only
  `/proc` `Path` import is also target-gated. Formatting, strict Clippy, the
  new width-and-overflow regression, and the complete serialized Rust harness
  pass locally.
- Exact-head fast CI run `30543600554` passed at repair commit `2db64424`.
  Full CI run `30544211166` proved the Apple Silicon compile progressed past
  disk preflight, then exposed three Windows compile errors before matrix
  fail-fast cancelled both macOS jobs: a Linux-only WSL helper referenced from
  a runtime-gated test, plus an unconditional Unix `libc::kill` call in
  workstation lock recovery. The test now uses compile-time Linux gating, and
  stale-lock process probing has fail-closed Unix and non-Unix implementations.
  Adjacent Unix-only resource-monitor imports are target-gated.
- Exact-head fast CI run `30545123372` passed at repair commit `a5423d6e`.
  Full CI run `30545744595` then compiled and exercised the Windows suite for
  nearly ten minutes before an explicit-config test exited abnormally. The
  test interpolated a Windows path directly into JSON, so its backslashes
  produced invalid escape sequences. The fixture now uses JSON serialization,
  preserving the same production manifest-resolution coverage on every target.
- Exact-head fast CI run `30547407293` passed at `60c784e3`. Full CI run
  `30548163584` proved the Windows manifest fixture repair, then its ordinary
  Linux suite exposed a real private-display launch race. Xvfb allocation
  returned after a fixed delay without proving the selected display was ready
  and owned by the spawned process, allowing the next launch to select the
  same display under load. Allocation is now serialized within the daemon and
  waits for ownership-backed readiness, with early exit and timeout cleanup.
  The same run also exposed a macOS test fixture whose socket path inherited
  the runner's long temporary root and exceeded `SUN_LEN`; the fixture now
  uses a short, unique Unix temporary path.
- Exact-head fast CI run `30549724644` passed at `98316d14`. Full CI run
  `30550334355` completed enough of the Windows suite to expose native-path
  assertions, Unix-specific home isolation, and installer inventory's
  fail-closed non-Unix process-liveness stub. Path assertions now compare
  native paths, the repository isolation test is Unix-gated, and Windows
  inventory uses the existing `windows-sys` dependency for live process
  status. The native E2E lane passed 55 of 56 tests before proving a separate
  recovery lifecycle regression: terminal-browser compaction preserved the
  health event but removed the record required to emit recovery events on
  relaunch. Recovery now reconstructs a bounded tombstone from that event
  history, preserves trace context, and restores the expected
  process-exited-to-ready sequence. Focused regressions, the real-browser
  crash and relaunch E2E, formatting, strict Clippy, and the complete
  serialized Rust CI harness pass locally.
- Release remains no-go pending exact-head fast and full CI, release dry run,
  merge, publication, and public-asset reinstall.

## Release Checkpoint | 2026-07-30

- Release PR 7 candidate `412684f6` passed exact-head fast CI run
  `30552821524` and manually dispatched full CI run `30553477964`.
- Fresh evaluator `/root/p82_final_go_no_go` returned GO after independently
  checking the exact PR head, version and changelog binding, CI evidence, and
  release hard stops.
- PR 7 merged as `80f64885`. The first release dry run, `30575205599`, built
  all seven binaries and then correctly failed the exact-inventory gate
  because repository `bin/agent-browser.js` had been included beside the
  release assets.
- The single authorized remediation moved release staging to isolated
  `release-assets/` and added a regression guard. Repair commit `4132e782`
  passed exact-head full CI run `30576313066`.
- Corrected dry run `30578774481` passed. Publication run `30579564702`
  published `v0.28.0`, verified all seven binaries, generated
  `SHA256SUMS`, and re-downloaded the public assets for checksum validation.
- Public tag `v0.28.0` resolves exactly to
  `4132e78203f00db26b65456bde4bc36355714ae4`.
- The public Linux x64 binary reports `agent-browser 0.28.0` and has SHA-256
  `4af2aba4e3670b2ffcd9601ab0134ad24cd13ec9e8131212f42a5645cb9baa22`,
  matching both the published checksum manifest and GitHub asset digest.
- A source-free idempotent reinstall on the accepted disposable Ubuntu VM
  completed ready. The installed hash matched the public binary, both doctors
  passed in the normal login environment, and the route-open dry run selected
  `guacamole:1`, connection `1`, and display `:10` without requesting a
  browser launch, route checkout, or tab open.
- The dated validation note
  `docs/dev/notes/2026-07-30-v0-28-0-release-validation.md` owns the detailed
  release and public-asset receipt. No Graphiti episode was written because
  this session lacked explicit memory-write authorization.
- P82 is complete. The operator-owned untracked `--full-page` file remained
  excluded and untouched.

## Clean-Install Acceptance Matrix

| Gate | Evidence |
| --- | --- |
| Artifact integrity | Version, architecture, manifest, embedded hashes, and checksum match before mutation |
| Dry run | No filesystem, service, container, user, group, secret, or database mutation |
| Rootless payload | All user assets materialize without repository or pnpm references |
| Privilege boundary | First full install requires exactly one password prompt; rerun requires none |
| Dependency safety | Simulation, removal proposal, install, helper, and service failures exit nonzero |
| Complete substrate | Dashboard, timers, PostgreSQL, Guacamole, guacd, XRDP, users, rows, and displays are ready |
| Route isolation | Two canonical connections use distinct route users and distinct live displays |
| P81 regression | Stable Route A selects `guacamole:1` on its readiness-selected display (`:10` in the clean VM); active conflict remains unchanged |
| No-launch proof | Canonical dry run requests no browser launch, route checkout, or tab open |
| Reboot | Database identity, dashboard, timers, displays, routes, and receipts recover |
| Durability | Checksummed backup, catalog validation, retention, and isolated restore drill pass |
| Idempotency | Rerun preserves secrets, IDs, rows, allocations, profiles, units, and healthy displays |
| Many-to-many | Existing live gate proves independent visible browser windows and cleanup |
| Released asset | Public Linux asset installs without a checkout and passes doctor plus route proof |

## Validation Gates

- `git diff --check`
- `pnpm validation:select -- --base v0.27.0`
- version synchronization check
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- full Rust unit suite
- focused installer, service reconcile, route state, and asset tests
- generated service client and API/MCP parity gates
- dashboard contract suite and production build
- documentation production build
- privilege, convergence, route sync, PostgreSQL hardening, durability, route
  confusion, and readiness fixtures
- installed shared-skill parity
- clean Ubuntu candidate install, reboot, rerun, restore drill, and live route
  evidence
- green fast CI and manually dispatched full CI
- green release dry run for all seven platforms
- dry-run artifact and public-release artifact source-free installation proofs

## Release Hard Stops

- stop if fast or full CI is red, skipped, or tied to a different commit;
- stop if any installed unit references a repository, pnpm, or mutable source
  path;
- stop if installation needs more than one initial sudo authorization;
- stop if dependency installation, schema state, database continuity, route
  identity, or active allocation state is ambiguous;
- stop if a fresh VM cannot pass through reboot and idempotent reinstall;
- stop if P81's readiness-gated route projection is absent or can launch a
  browser during its proof;
- stop if dry-run and real release workflows do not target the exact merged
  commit;
- stop if changelog markers do not belong to package version `0.28.0`;
- stop if the release asset set differs from the seven supported filenames;
- stop if the public asset checksum, executable version, doctor, or no-launch
  proof fails;
- never stage, modify, or remove the operator-owned `--full-page` file.

## Commit and Push Structure

1. plan, roadmap, runbook, and baseline CI repair;
2. installer command and embedded asset contract;
3. workstation substrate and installed units;
4. binary-owned convergence and P81 regression;
5. clean-install and reboot harness;
6. documentation and release workflow hardening;
7. `0.28.0` version and changelog preparation;
8. audit remediations and release-candidate evidence;
9. post-release closeout.

Each commit must pass the gates appropriate to every touched surface since the
last green baseline. Push after each coherent validated packet.

## Subagent Receipts

- `/root/installer_architecture_audit`: read-only audit of installer ownership,
  missing assets, route bootstrap, durability, and recurring control paths.
  Verdict: current installer is not fresh-host complete; recommend a
  binary-owned orchestrator with versioned installed assets.
- `/root/fresh_install_test_audit`: read-only audit of current fixtures and
  clean-host proof gaps. Verdict: containers are appropriate for fast
  fixtures, while authoritative acceptance needs a disposable Ubuntu VM and
  reboot.
- `/root/release_version_audit`: read-only audit of version, CI, GitHub release,
  changelogs, and workflow. Verdict: release is currently no-go; target
  `v0.28.0` after CI repair and fresh-install productization.

## Completion Evidence

- Release:
  `https://github.com/CochranResearchGroup/agent-browser/releases/tag/v0.28.0`
- Published tag commit:
  `4132e78203f00db26b65456bde4bc36355714ae4`
- Exact-head candidate CI: `30552821524` and `30553477964`
- Exact-head remediation CI: `30576313066`
- Corrected release dry run: `30578774481`
- Publication and published-asset verification: `30579564702`
- Public Linux x64 SHA-256:
  `4af2aba4e3670b2ffcd9601ab0134ad24cd13ec9e8131212f42a5645cb9baa22`
- Durable validation:
  `docs/dev/notes/2026-07-30-v0-28-0-release-validation.md`
