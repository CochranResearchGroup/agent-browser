# Native viewer continuity during tab selection

Product lane: PL-BUGFIX
Work item: [552](https://github.com/CochranResearchGroup/agent-browser/issues/552)
Source baseline: b2863d39cfd02e57e67769a32702d20b719bab59.

## Diagnosis

The mounted production viewport reproduced viewer teardown twice with
`pnpm exec node scripts/test-native-tab-continuity.cjs`. Switching between two
current tabs on one desktop replaced the iframe node and document. Returning
the same opaque handoff URL for both tabs still failed, ruling out a changed
URL as a necessary cause. Census and desktop identity remained fixed.

The cause was task identity controlling viewer lifetime: selection immediately
hid the old view, then opening state removed its iframe. Even an unchanged URL
created a new viewer document and lost the existing connection.

## Repair and boundaries

Keep task identity for exact authorized focus and use browser plus desktop
identity for viewer continuity. Preserve the already opened URL and iframe
through same-desktop focus and its successful acknowledgement. A different
browser/desktop, authoritative failure, confirmed absence or changed unverified
selection still clears the viewer. Exact focus remains serialized through the
existing owner-authorized handoff request; no provider or authentication API
changes are introduced.

The mounted rail regression covers separate task sessions in the same browser,
held focus requests, late success/refusal, independent desktops, stale census,
absence, framing/access errors and explicit retry. Assertions now await focus
responses rather than treating a viewer reload as evidence of completed focus.
Framing refusal is injected on an actual page load.

This is a bounded local patch in the existing checkout and branch. Preserve the
pre-existing browser mode context-menu edits. No new checkout, commit, canonical
integration, installed runtime replacement or actual Native connection proof
is implied by these synthetic checks. The focused fixture checks exact iframe
and document continuity; it does not simulate the Native display transport.

## Validation

Passed locally: focused red/green continuity test, `pnpm test:native-task-selection`,
`pnpm test:compact-native-workspace`, dashboard TypeScript, docs build,
`pnpm test:remote-view-handoff-docs`, `pnpm test:policy-wiring`, documentation
links and patch hygiene. The first broader fixture run waited for an obsolete
viewer reload; the repaired fixture waits for focus acknowledgement. Its failure
is retained in the session output rather than classified as a flaky retry.

Dashboard build and validation-selector behavioral/CLI/aggregate/economics
checks also pass. Rust gates passed:
`scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check` and
`AGENT_BROWSER_CARGO_CACHE=off scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`.
Cargo admission initially reported memory pressure. The first lint
attempt then stalled in sccache's compiler-version query; its exact validation
scope was stopped and lint restarted with `AGENT_BROWSER_CARGO_CACHE=off`.
The host reserve and Cargo job bounds remain unchanged.
The cache-disabled lint finished successfully in 2 minutes 42 seconds. The
ordinary formatting invocation completed after its admission wait. No Rust
behavioral suite is claimed: the only Rust edit is dashboard-help prose.

`pnpm test:service-api-mcp-parity` fails for the workflow skill's 10 KB limit
and two existing public `browser_retention` documentation assertions. A
read-only `fs.readFileSync` overlay supplied every checked text file from
`git show HEAD:<path>` and reproduced exactly those three failures against the
baseline, whose workflow skill is already 10,607 bytes. These are retained
baseline failures; this patch does not claim the broader parity gate passes.

Executable diff identity for the tracked viewport, selection/rail fixtures and
package script is SHA256
`8a13c02b57554e4162d79a6286de4c88769520fe27bc0d8c2f6b5747f4c40f88`.
The new focused test has SHA256
`07cbbe3a391678617a499ca460e9f9b5718809f7ee0de23a8981d6ed2f0c212d`.
Hosted GitHub CI remains disabled. Local tests use Linux Chrome and synthetic
owner/viewer replies rather than installed provider state. The existing Next
static-export rewrite warning and docs workspace-lockfile warning remain.

## Canonical integration

The operator explicitly requested PR, merge, deployment and cleanup. Preserve
the complete prior checkout on remote branch
`archive/tab552-pre-integration-20261106`, checkpoint `981bbaaa4`. The fresh
`fix/native-tab-continuity-552` branch starts at canonical main `89e844266`.
Current main already contains the functional browser mode context menus, so
their implementation and newer fullscreen documentation are retained without
duplicating the old patch. Its local historical note remains in the archive.

The two obsolete tool-documentation failures are resolved on current main.
Concise wording in the touched workflow skill keeps the full guide at 9,995
bytes and preserves its tool inventory and safety semantics. The API/MCP
parity gate now passes. Current-main mounted task-selection, focused continuity
and full compact workspace tests pass after conflict resolution. Earlier
baseline failures above remain historical evidence rather than current gates.
