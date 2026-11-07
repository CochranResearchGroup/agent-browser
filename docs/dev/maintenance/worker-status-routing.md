# Worker status routing

Work item: `LOCAL-WORKER-STATUS-ROUTING`. Owner: ecochran76. Lane: PL-BUGFIX.
State: DONE (source qualification). Branch: `fix/worker-status-routing`. Base: `3d7a8141f`.

Restore supervisor admission when a managed task exists under the same lane.
The isolated native-protocol reproduction succeeds before task creation and
returns `Not yet implemented: worker_status` afterward, using the same binary
and explicit runtime lane. Its disposable browser closes successfully.

Acceptance: worker status returns the lane control-plane status without browser
execution or managed-state changes, both before and after managed task creation.
Keep the regression at the authenticated daemon boundary. No installed-runtime,
bank, messaging, or provider effects are in scope.

The two existing clean worktrees retain their catalogued P300 custody. This
primary checkout owns only status routing, its regression, and this lane's docs.
Remove the checkout after durable integration or handoff; retain the source branch.

## Diagnosis and validation

The daemon tries ordinary managed-browser routing before its control-plane
dispatch. `worker_status` was absent from `action_skips_browser_launch`, so a
same-name managed task intercepted the request and sent it to browser execution,
whose unknown-action response is `Not yet implemented: worker_status`. Marking
that existing control-plane action browser-free restores the intended route.

Ranked alternatives were managed interception, wrong lane selection, and binary
version mismatch. The reproduction uses an explicit runtime lane and the same
binary for both client and daemon, eliminating the latter two. Task creation is
the load-bearing input: the status request succeeds before it and fails after it.
The classification change is the only production-code change.

Run `AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD=/absolute/path/to/candidate pnpm
test:worker-status-routing`. The unchanged binary
`f640d387fc0b7b49d76439fd1ea4bd23ca10d0752198d9f82c1c4c7d39819e01`
fails with the exact error. The repaired candidate
`97599e72d0431a71aa10cd1d54fd76fccb7fe3d14ea070006c71fcba0434c185`
passes, reporting Ready/NotStarted with no browser-task response and identical
managed state. Owned browser closure passed; a fresh OS process census found
no remaining worker-status fixture processes.

Local checks on Linux/WSL:

- `scripts/ci/cargo-safe.sh build --manifest-path Cargo.toml -p agent-browser --profile ci`: PASS.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: PASS.
- `pnpm test:worker-status-routing` with the candidate override above: PASS.
- `pnpm test:policy-wiring`: PASS, 11 cases.
- `pnpm test:documentation-links -- --base origin/main`: PASS.
- `node --check scripts/smoke-worker-status-routing.js` and `git diff --check`: PASS.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: PASS, no warnings.

The existing `test:service-status-no-launch` CLI assertions pass, but its MCP
portion fails with `Unknown MCP tool: service_status`. Isolated `tools/list`
probes confirm that tool is absent in both the unchanged debug binary and this
candidate. This is an existing adjacent test/contract discrepancy, not a passing
check or part of this status-routing repair. The generic validation selector
also recommends streaming checks because the shared action file changed; no
stream implementation changed, and the focused daemon regression covers the
changed routing boundary. Its new script is not yet mapped by that selector.

Hosted CI was skipped under the operator's cost policy. Local synthetic browser
proof does not establish installed supervisor, messaging, or bank acceptance.
The build used the existing target cache through the resource wrapper. The first
job was stopped after a compiler-cache query delay (exit 143); the successful
retry disabled sccache, preserved caches, and waited for resource admission.

Source is ready for review. Integration and installed acceptance remain separate;
retain this primary checkout for review through 2026-11-08, then close it through
the governed worktree lifecycle once its source has durable handoff custody.
