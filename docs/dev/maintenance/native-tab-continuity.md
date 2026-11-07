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

Final source gates pass on `088f23377`: task-selection/continuity and full rail
tests, API/MCP parity, dashboard/docs builds, policy wiring, changed documentation
links, handoff docs and validation-selector contracts. Workspace format and
strict Clippy pass with cache disabled, four Cargo jobs, one invocation,
`AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=8388608`, `MemoryHigh=6G` and
`MemoryMax=8G`. The 16 GiB host reserve is unchanged. Clippy finishes in
1 minute 35 seconds. The original default-limit invocations were still waiting
for admission and were stopped before compilation. No hosted CI ran.

The local-source patches were self-checked against the published PR diff and
current main. Browser mode and Native fullscreen controls are preserved. Runtime
installation and actual Native transport acceptance remain subsequent gates.

## Installed checkpoint and cleanup

PR553 merged as `eb8cef70c610cc4a02da00d3b3793e3d68072202`. The clean merged
source produced sealed production-shaped release binary
`31dac9aa8a92b41908286b8412da5d5717c55bcb2a975a1f991b58f5ccb844fa` in
28 minutes 12 seconds, using the bounded four-job configuration above. The
exact-binary workstation installer fixture passed with zero owned host residue.
Candidate review passed before the explicit preserving installation.

Transaction `upgrade-033ed0aa-5ea3-4fd7-b15f-0f5b2129038e` accepted revision 13,
with zero outstanding owner obligations. Installed generation
`0.28.0-31dac9aa8a92-a41dd8a76003` reports ready. Doctor passes; its workspace
candidate warning is advisory. The new continuity JavaScript chunk matches the
built bytes at both loopback and public HTTPS, SHA256
`2fc8ee51cb17882d7db2ca5a6d86ef354b87c7d2c387f3f5e80159c97c021778`.

Keep these acceptance boundaries explicit:

- The mounted production component and full rail regression pass. An installed
  synthetic task also opened both expected pages and returned a ready Native
  handoff. Its exact browser was normally closed and subsequently absent from
  the OS; its loopback server and all harness browser contexts were closed.
- The authenticated installed rail test remains unqualified: Cooper sign-in
  succeeded, but workspace status returned HTTP200 with `authenticated:false`.
  No access mapping, credential or ingress policy was changed. Early fixture
  failures incorrectly expected an automatic sign-in redirect, used an outdated
  form selector, and misread the successful sign-in response. The first actual
  authentication effect was reconciled against the account's success-only audit
  metadata before another isolated session was used. Raw authentication and
  runtime artifacts stay outside Git.
- A standalone Native probe did not establish frame delivery through its DOM
  observer. Final installed status nevertheless has `operatorJourneyReady=true`
  and a receiver-qualified ready frame receipt bound to the new backend. That
  receipt belongs to a different handoff from concurrent activity; it is not
  evidence for this run's repeated tab-selection test.
- The install-window census retained 111 of 113 original browser process identities;
  two exited with unproven cause while two new roots appeared. Of 134 Remote View
  units, 132 retained exact identities. The other two MCP units were already in a
  rapid restart loop before installation. This does not prove full-fleet
  preservation and does not authorize broad recovery or cleanup of peer work.

Cleanup removed 889,733,120 allocated bytes from this build's unsealed host and
target outputs, then 5,715,894,272 bytes of disposable shared incremental cache.
Both operations held real Cargo target locks, checked executable/mapping/FD
references, rejected symlinks and nested mounts, and verified the sealed binary
digest afterward. The target tree fell to 16,673,837,056 bytes. All 12 pinned
candidates and their manifests/receipts have explicit retain decisions; no
candidate or source checkout was discarded. The canonical checkout remains for
integration. The earlier P300 source remains on its branch and verified remote
archive; its checkout custody is a paused ref. The merged repair branch is removed.
