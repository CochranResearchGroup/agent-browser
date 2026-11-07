# Runtime reset

Product lane: PL-PLATFORM
Work item: #565
Branch: platform/runtime-reset
Baseline: 32bd6f904b8b1a9ff9b9d916472a3b6cad2dd5e0

## Outcome

An operator can preview and explicitly reset the local production workstation,
ending active tasks and positively owned browser processes while retaining
profiles, authentication, terminal history, links and uncertain operation journals.
This replaces a manual database/process recovery with one accountable command.

## Contract

`agent-browser runtime reset` previews without starting a daemon or writing state.
`--apply` performs a production-only Linux user-systemd reset. `--resume <id>`
continues an interrupted receipt rather than creating a second reset. An active
install or reset is an exact coordination conflict, not a permission request.
The command never accepts arbitrary PIDs, paths or unit selectors.

Before browser effects, stop the previously active production dashboard producers,
interlock timer and runtime host, and create a private SQLite backup. Select
browser processes by the exact production host cgroup and verified executable
identity, retain kernel process handles, terminate politely and escalate only
those verified processes. Confirm absence before retiring active state in one
SQLite transaction. Preserve borrowed external processes and all other cgroups.

The store appends terminal task/tab history and preserves operation/replay IDs,
profile definitions, counters, configuration and profile bytes. Receipt checkpoints
record progress before effects. After commit restore only previously active units.
A failure retains the same receipt and coordination operation for explicit resume;
a completed resume never repeats browser closure or the state transition.

## Ownership and validation

This lane owns new reset modules, CLI dispatch/help, tests and documentation parity.
#562 remains the publication-reconciliation owner; consume its merged baseline
without editing its publication algorithms. #74 profile-data deletion/restore is
outside scope. No new tool deployment or provider action is authorized by source
validation. Local targeted Rust tests, strict workspace Clippy, format, docs,
policy/link and changed-surface checks provide source validation; no hosted CI.

## Acceptance

Source-qualified on Linux/WSL. Nine focused reset tests cover transactional
retirement and rollback, durable outcome replay after a lost external checkpoint,
completed coordinator reconciliation, invalid selectors, foreign-unit refusal,
restart failure/resume and kernel-handle closure with peer preservation. Existing
coordination tests provide the shared mutation-custody gate.

Passing local commands:

```bash
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused runtime_reset
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused candidate_coordination
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh build --manifest-path cli/Cargo.toml
pnpm test:policy-wiring
pnpm test:documentation-links -- --base 32bd6f904b8b1a9ff9b9d916472a3b6cad2dd5e0
pnpm test:remote-view-handoff-docs
pnpm --dir docs build
```

Public CLI preview/help and invalid apply-scope/flag checks preserve the runtime
database digest. Host-provision, VM-harness, Guacamole asset/durability and
route-user fixtures also pass. Shared-cache clients stalled after interruption;
only this task's waiting clients were cancelled, and final Cargo validation used
the supported cache opt-out. Local debug/Linux evidence replaces hosted CI;
it does not establish macOS/Windows support or a comprehensive Rust-suite pass.

Production operational reset and its disposable open/close acceptance retain
separate private receipts. Whole-command service effects are qualified through
injected orchestration, SQLite and real process-handle seams.

## Installed checkpoint

PR566 is merged as `f31882882aa6ebb91fc520fcb1bfcde89b0ced31`. The
[installed receipt](../notes/2026-11-07-runtime-reset-installed.json) binds sealed
production binary `94ec05f29f12`, exact installer/CLI qualification and accepted
preserve transaction `upgrade-62378899-40ef-4a1f-a3dc-1cc88e413bec`.
Install doctor, read-only reset preview, fresh MCP open/snapshot/close, original
process-identity preservation and synthetic cleanup pass. Installed guidance is
synchronized. Previously connected MCP clients may need reconnect.

The initial writer-busy refusal and guarded selected-candidate close/rollback
refusals remain in private evidence. Deployment did not run another reset, remove
locks or force browser shutdown. The new command is available; this is not an
additional live qualification of reset apply. Profile-data reset under #74 stays
out of scope. Retain the clean primary through 2026-11-08 for custody of the sealed
production candidate and rollback/reproduction evidence.
