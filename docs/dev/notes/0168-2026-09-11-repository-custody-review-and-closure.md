# Plan 0168 Repository Custody Review and Closure

Date: 2026-09-11

Branch: `maintenance/plan-0168-repository-closeout`

Baseline: `2618ddb2c97cedbcc453a9f9584063a7c721eca3`

## W0 custody

The canonical worktree's uncommitted `commands.rs` and `navigation.rs` changes
belong to a still-running Codex process rooted in the Odollo workspace. Its
exact workstation-install child exited after installing its candidate. Plan
0168 did not alter, stage, stash, reset, or copy either file. P168 operates in
its own worktree from the baseline above.

## Closed-world review

| Item | Review evidence | Disposition |
| --- | --- | --- |
| P165 `fix/bill-login-email-continue` | Clean local and remote tip `5ee95d67`; zero unique commits; tip is an ancestor of `origin/main` | Git custody integrated; operational Plan 0165 remains open |
| P166 | PR 27 receipt `6030c71b`; plan closed; refs already absent | Remove from active catalog |
| P167 | PR 29 receipt `14fb3db0`; plan closed; refs already absent | Remove from active catalog |
| P144 | Checkpoint `ae5ad34c` first reached `main` through merge `74883c6c`; plan remains open | Close historical Git ref; preserve plan |
| P155 | Checkpoint `804519f0` is in PR 14 receipt `ba3916ca`; plan closed | Remove stale integration-ready catalog entry |
| P156 | Checkpoint `3bfb1c49` is in PR 14 receipt `ba3916ca`; source scope complete; production shutdown was outside the plan | Close source plan and remove stale catalog entry |
| P157 `plan/profile-permissions-and-request-provenance` | `a54fee12`, `83e28c2f`, and `665bd0d0` are patch-equivalent to `main`; `cccf3636` is behaviorally represented by `ad673377` with only formatting and test argument-order differences | Close historical Git refs; keep Plan 0158 and Plan 0162 open |
| Last30days X documentation branch | `git cherry` marks `6f198a8b` patch-equivalent to `main` | Close local and remote refs |
| Reddit documentation branch | `git cherry` marks `47a1040f` and `cc0cd29c` patch-equivalent to `main` | Close local and remote refs |
| P0240 runtime-compatible branch | First three commits are patch-equivalent; five are not | Preserve worktree and branch for W3 |

## P0240 residual set

| Commit | Classification | Current decision |
| --- | --- | --- |
| `45360140` | Formatting-only change against an older `navigation.rs` | Superseded; do not replay |
| `1104e16e` | Disambiguates a geometry-epoch test literal | Candidate residual |
| `67a2b98f` | Records the runtime-compatible BILL candidate | Historical evidence; retain only if updated for the final source disposition |
| `022d820d` | Adds the provider-neutral exact role/name `semantic_click` action and contract validation | Substantive candidate residual |
| `0bb7fc56` | Qualifies the semantic-click candidate | Historical evidence; refresh after current-main validation |

No P0240 residual is accepted for integration until it is reconstructed on
current `main`, its current contract counts and generated surfaces are
reconciled, and all selected validation passes.

## Executed closure

The review checkpoint was published at `9a61432a`. A fresh pre-deletion census
found the Plan 0165 worktree clean and found no process cwd beneath either the
Plan 0165 or P0240 worktree. Local and remote tips matched for every branch
closed below.

Removed worktree:

- `/home/ecochran76/workspace.local/agent-browser-bill-login-continue`

Removed local and remote branches:

- `fix/bill-login-email-continue` at `5ee95d67`;
- `plan/profile-permissions-and-request-provenance` at `cccf3636`;
- `docs/last30days-x-identity-rejection-20260903` at `6f198a8b`; and
- `docs/reddit-handoff-errors-20260902` at `cc0cd29c`.

Removed remote-only branches whose exact tips were ancestors of `origin/main`:

- `feature/p0240-sealed-auth` at `a76e3223`;
- `integration/plan-0161` at `68a59645`;
- `maintenance/consolidate-field-notes-20260908` at `5d82be62`; and
- `plan/lease-authority-coordination` at `ae5ad34c`.

These removals are recoverable from the named commits and integrated history.
The P0240 runtime-compatible worktree and branch remain intact for W3.

## P0240 qualified residual

The published P0240 branch was not rebased or rewritten. A fresh worktree at
`/home/ecochran76/workspace.local/agent-browser-p0168-p0240` reconstructed its
two accepted residuals on current `main`:

- `2a5545db` carries the geometry-epoch fixture clarification from `1104e16e`;
- `7f2b7083` carries and hardens exact semantic click from `022d820d`.

The semantic action now requires `exact: true`, refuses unsupported locator
fields, requires exactly one live accessibility-tree match, validates before
job creation, and returns locator and activation evidence. README, CLI help,
the repository agent skill, the service schema, MCP description, inline docs,
and the docs site describe the behavior. Pull request 30 merged as
`de614fbe303fac9dccb996b1c5aea4decfb53d9d`, the integration receipt.

Provider-free validation passed for Rust formatting, clippy, focused semantic
click, desktop capture and service request tests, service contract and client
parity, no-launch service collections, the docs build, documentation contracts,
and the selector-nominated workstation fixtures. The first unconstrained Cargo
attempts failed before project compilation because the shared Cargo slice was
at 949 of 1,024 tasks. The same gates passed with one Cargo build job and cache
disabled. The repository and installed user-scoped skills differ only because
the accepted source change has not yet been published into that installed
surface; Plan 0168 did not overwrite it.

After the merge receipt was durable, the exact cleanup preflight found one
task-owned debug daemon left by the passing no-launch collection smoke. PID
94587 used the integration worktree binary, isolated home
`/tmp/ab-service-collections-no-launch-95VDIZ`, and session
`service-collections-no-launch-94578`. It exited after `SIGTERM`; a fresh cwd
scan then found neither P0240 worktree in use.

The campaign removed these worktrees:

- `/home/ecochran76/workspace.local/agent-browser-p0240-runtime-compatible`;
- `/home/ecochran76/workspace.local/agent-browser-p0168-p0240`.

It removed the matching local and remote branches at their reviewed tips:

- `feature/p0240-sealed-auth-runtime-compatible` at `0bb7fc56`;
- `integration/p0168-p0240-residual` at `7f2b7083`.

After prune, only canonical `main` and the P168 closeout worktree remained.
The original P0240 historical note commits are recoverable by their commit IDs;
the accepted source is recoverable through PR 30 and `de614fbe`.

## Final closeout

PR 31 merged the campaign review and P0240 closure record as
`9f711ebcccf6393fcf1ff22778fba4b3450c3224`. The final catalog retains only
P165, P157, and P144. Each has integrated Git custody, no historical remote ref,
and an open operational or future-source plan read from `origin/main`.

PR 32 is the final plan-state and catalog receipt. After it merges, only the
canonical `main` worktree is intended to survive. Its uncommitted
`cli/src/commands.rs` and
`cli/src/native/action_runtime/runtime/navigation.rs` remain owned by the
foreign Odollo agent and are neither campaign residue nor safe cleanup targets.
The exact P168 worktree and local and remote maintenance refs become eligible
for deletion only after that receipt is durable.

## Negative effects

This campaign performed no production installation, browser or profile
mutation, credential or Auth Vault mutation, provider or tenant action, X or
RuFresh retry, formal release, or foreign-process termination. It terminated
only its exact isolated no-launch fixture daemon after the cleanup gate found
that passing test had left PID 94587 running. The observed Odollo-owned
installation was foreign to P168 and is not claimed as campaign evidence.
