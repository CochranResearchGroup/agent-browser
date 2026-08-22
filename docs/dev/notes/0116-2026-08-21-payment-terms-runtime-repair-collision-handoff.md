# Plan 0116 Payment Terms Runtime Repair Collision Handoff

Date: 2026-08-21

State: source repair candidate only; shared-worktree reconciliation required

## Purpose

This note hands an interrupted Agent Browser repair to the active Agent Browser
owner. The repair was discovered while qualifying Odollo and SoyLei payment
terms matrix attempt 18. It does not authorize an install, global runtime
restart, merge, commit, or payment-terms matrix attempt.

## Authority order

Before changing anything, re-read these sources in order:

1. `AGENTS.md` and the relevant policy modules under `docs/dev/policies/`.
2. `docs/dev/active-lanes.yaml`, the current Agent Browser roadmap and runbook,
   and the active Plan 0118 owner state.
3. `docs/dev/plans/0116-2026-08-15-runtime-adoption-and-transactional-upgrade-plan.md`.
4. Current Git status, current process state, and the active agent's exact
   branch or worktree ownership.
5. This note as an incident handoff, not as integration authority.

Graphiti group `agent_browser_main` has older source-backed P66 facts confirming
that `service_remote_view_browser_reattach` is intended to synthesize a
no-launch route checkout for a retained browser. That recall is advisory. The
current source, tests, and runtime receipts remain authoritative.

## Collision and custody

The shared worktree was read back at:

- path: `/home/ecochran76/workspace.local/agent-browser`
- branch: `architecture-deepening-20260809`
- HEAD: `1e55235c0706b6f3ca0c4a557a8e9d9d43c386f0`
- remote state: one commit ahead of `origin/architecture-deepening-20260809`
- worktree state: dirty

Another Agent Browser lane was actively running this policy-compliant isolated
test when the collision was recognized:

```bash
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  shared_runtime_host_pid_never_joins_independent_supervisor_lanes -- --exact
```

Its target was under `/tmp/agent-browser-plan0118-build.7gqo7s/`. Do not stop,
rewrite, or claim that lane from this note. Freshly reconcile its current state
because process observations are ephemeral.

The interrupted payment-terms slice had started a direct shared-target
`cargo build --release`. That build violated the WSL build-wrapper policy and
raced with the active lane. Its exact shell, Cargo, and rustc processes were
terminated. Treat everything under `cli/target/release` as unqualified until a
clean isolated rebuild proves its source identity.

## Dirty-file attribution

The interrupted payment-terms slice changed these five files:

- `cli/src/native/runtime_lifecycle.rs`
- `cli/src/native/actions.rs`
- `cli/src/native/action_runtime/runtime/navigation.rs`
- `cli/src/native/remote_view/open/route_lifecycle.rs`
- `cli/src/native/actions/remote_view_route_tests_one.rs`

These four files were already dirty and belong to another lane:

- `cli/src/process_identity.rs`
- `cli/src/runtime_adoption.rs`
- `cli/src/runtime_owner_transfer.rs`
- `cli/src/runtime_profile.rs`

Do not commit, revert, stash, or rewrite either set until the active Agent
Browser owner has reconciled intent and selected a disjoint worktree. No commit
was created for the payment-terms repair.

## Diagnosed defects

### Terminal replacement admission

An exact terminal browser generation remained bound in memory after successful
close. The generic owner-effect admission gate then rejected
`remote_view_open`, making the registry's supported terminal replacement
transition unreachable.

The candidate repair:

- introduces `RuntimeEffectAdmission::TerminalReplacement`;
- admits only exact `remote_view_open` replacement for the same logical browser
  and session;
- requires the current owner claim, no pending transfer, exact generation and
  profile identity, terminal lifecycle state, and satisfied cleanup;
- clears only the matching in-memory binding before replacement registration;
- completes close and clears the exact binding atomically through
  `complete_close_and_release_binding`.

The candidate regression in `runtime_lifecycle.rs` covers current-owner,
terminal-observation, wrong-action, wrong-session, and terminal-current-owner
cases. A prior focused run reported ten lifecycle tests passing, and an exact
live replacement reached a new ready owner generation. Those results are
diagnostic evidence only because the commands and built artifact were not
preserved through the required wrapper and isolated build flow.

### Remote display identity corruption

After replacement, `service_remote_view_browser_reattach` correctly projected
an RDP stream, but `handle_service_remote_view_route_checkout` copied the
adopted browser host `attached_existing` onto the display allocation. Desktop
capture requires the display allocation itself to remain `remote_headed` and
failed closed with:

```text
desktop_identity_mismatch: display allocation ownership does not match the route
```

The candidate source change forces the display allocation host to
`ServiceBrowserHost::RemoteHeaded` during route checkout. This matches the
display-workspace contract even when the replacement browser was adopted as
`attached_existing`.

The current test edit is not acceptable yet. It accidentally changes the
browser host in `test_remote_view_route_and_lease_actions_mutate_service_state`
near line 186, while the intended reattach fixture near line 591 still uses
`RemoteHeaded`. The new assertion therefore passes without reproducing the
observed replacement-browser failure. Correct the fixture or add a dedicated
test before accepting the source fix.

## Diagnostic validation

The following observations were made before handoff:

- `git diff --check`: pass after the interrupted build was terminated.
- focused reattach test: one test passed, but it did not cover the intended
  `AttachedExisting` fixture because of the misplaced edit.
- direct Cargo formatting and test commands were used before the WSL wrapper
  policy was re-read. Do not treat them as acceptance evidence.
- release build: interrupted and explicitly unqualified.
- no global Agent Browser install or global runtime-host restart was performed.

## Ephemeral runtime snapshot

At handoff, the isolated payment-terms runtime answered on port `37404` with:

- session and browser id: `p0204-a06` and `session:p0204-a06`
- browser PID: `60208`
- browser health and profile: `ready`, `default`
- display: `:11`
- RDP route: `guacamole:2`, state `ready`
- display allocation: `remote-view-display:11`, state `ready`
- RDP stream attachability: `attached_ready`
- incorrect allocation host: `attached_existing`

The runtime uses an ephemeral socket directory under
`/tmp/agent-browser-p0204-a18-6kTLmP`. No token is recorded here. Re-read all
process, route, allocation, and browser identities before relying on this
snapshot. Do not restart or converge the global runtime host because unrelated
supervisor lanes are active.

## Required next packet

The Agent Browser owner should take one bounded reconciliation packet:

1. Reconcile Plan 0118 and shared-worktree ownership before editing.
2. Move or reconstruct the five candidate changes in a dedicated worktree.
3. Use a dedicated `CARGO_TARGET_DIR` and the repository Cargo wrapper.
4. Correct the reattach regression so a browser with
   `ServiceBrowserHost::AttachedExisting` retains a
   `RemoteHeaded` display allocation and passes desktop capture identity
   validation.
5. Review the terminal-replacement admission against Plan 0116 invariants and
   the active runtime-adoption changes. Reject or revise it if it conflicts.
6. Run at minimum:

```bash
scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  terminal_replacement_admission_releases_only_the_matching_observation_binding \
  -- --nocapture
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml \
  test_remote_view_browser_reattach_reuses_retained_browser_without_duplicate_row \
  -- --nocapture
scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings
```

7. Build only in the isolated target, then prove the installed or isolated
   executable identity before runtime use.
8. Reproduce terminal replacement, reattach Route B, and run
   `desktop capture` against the exact replacement browser. Acceptance requires
   a fresh PNG receipt bound to the exact browser, session, route, display, and
   accepted executable generation.
9. Hand the accepted commit and runtime receipt back to the Odollo Plan 0204
   owner. Do not run the payment-terms matrix from this repo.

## Hard stops

- Do not use the shared `cli/target/release` artifact.
- Do not alter the payment-terms retry ledger.
- Do not install or restart the global Agent Browser runtime from this note.
- Do not touch unrelated supervisor lanes or runtime profiles.
- Do not merge, commit, stash, or revert the pre-existing four-file lane.
- Do not claim desktop capture fixed until the exact runtime receipt is green.

## Suggested skills

- `graphiti-discovery` for Plan 0116 and P66 source-backed recall.
- `codegraph-workspace` for lifecycle and route-checkout impact analysis.
- `diagnosing-bugs` for regression-first reproduction and hypothesis ranking.
- `agent-browser` and `agent-browser-service` for isolated runtime validation.
- `handoff` for the accepted-commit return packet.

## Reconciliation Outcome

The active Agent Browser owner reconciled the five source files on 2026-08-22.
The terminal-replacement admission design was retained. The misplaced
remote-view fixture edit was corrected so the reattach regression now starts
with an `AttachedExisting` browser and proves that its display allocation
remains `RemoteHeaded`.

Source validation passed:

- Rust formatting check;
- terminal-replacement admission regression;
- retained-browser reattach regression;
- strict Clippy with warnings denied;
- route-confusion no-launch gates; and
- three `cdp_screencast_view_stream` regressions.

The selected CDP tab-streaming live smoke did not reach the changed behavior.
Its disposable harness attempted retired legacy per-session daemon creation and
was rejected with `runtime_host_admission_required`. No admission bypass was
used. Exact replacement-browser desktop capture remains a separate live
acceptance boundary; this reconciliation commits the source repair without
claiming that runtime proof.
