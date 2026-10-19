# P251: useful Properties and profile durability

Status: OPEN
Lane: P251
Product lane: PL-PLATFORM
Work item: CochranResearchGroup/agent-browser#323
Pull request: https://github.com/CochranResearchGroup/agent-browser/pull/324
Branch: platform/dashboard-profile-properties
Owner: ecochran76

## Current state

Draft PR324 has qualified source for Properties context, profile naming and one-way promotion. Four focused profile tests, the corrected cleanup guard and the pending-retirement fence pass. Strict workspace Clippy and mounted UI checks pass. The final strict-schema projection and service contract metadata checks pass. A new sealed build and development/production installed qualification remain. Two uninstalled builds were deliberately stopped before sealing for the imported-profile and strict-schema findings; both receipts are retained.

## Outcome

Properties preserves known creation time and original task context, explains absent historical attribution, shows authoritative throwaway or durable status, and allows display-name edits and one-way promotion without moving profile data or restarting browsers. Profile IDs remain stable.

## Custody and overlap

Reuse the clean dashboard primary from integrated P250, retaining its sealed candidates. Three registered checkouts were inspected; stale-session and resource checkouts are clean and foreign. No open PR overlaps this surface. Own dashboard Properties, managed workspace display projection and atomic runtime profile metadata writes. Preserve lifecycle cleanup, handoff ownership and all browser processes.

## Qualification

The red adapter probe loses authoritative `createdAtMs` (undefined versus 123). The last private installed snapshot has 29 tabs, four metadata records, one supplied service label, and 29 creation timestamps. Historical missing labels must not be fabricated. Check recoverable exact-target capsules before declaring original attribution unrecorded.

Require projection, profile mutation, stale-host and reaper race regressions; mounted UI save/failure/focus checks; generated client parity; dashboard build; Rust formatting and strict workspace Clippy plus affected compartments. Installation remains separately qualified.

## Implementation checkpoint

Owner projection now reads profile catalog and managed state in one SQLite snapshot, preserves tab creation time, and recovers original context only from the same logical tab/session/profile capsule. Properties displays recorded context without repeated Unknown rows. The authenticated dashboard-only PATCH accepts exact profile ID, expected name and expected kind. It changes only display name and one-way retention; directory and runtime identities remain fixed. Promotion atomically removes disposable cleanup eligibility and clears disposable recreation intent. Cleanup and quota retirement recheck the exact allocation under their existing effect guard. Pending launch/retirement custody and concurrent catalog writes fail closed. Hosts refresh catalog alongside state before ordinary work.

Mounted Properties checks pass for absent context, creation time, renaming, promotion, conflict, focus restoration, last confirmed receipt preservation and mobile layout using actual production CSS. A pre-existing Dialog animation/box-sizing conflict caused narrow-screen overflow; explicit panel dimensions and disabled layout animation repair it. One stylesheet check overlapped a dashboard rebuild and was rerun after the build completed; the settled health fixture passes. Dashboard build/type checks, docs build, generated client/consumer checks, navigator and inspector checks pass. Strict workspace Clippy passes. The expanded browser-session suite is still running at this custody checkpoint; no merge or installed qualification is claimed yet.

The API/MCP parity command fails the existing workflow-skill size cap: untouched main is 17,423 bytes against a 10,000-byte limit. No service action was added; the new endpoint is dashboard-only. Preserve this pre-existing finding rather than claim the whole parity command passed. An initial new Rust assertion needed an explicit test-module import, corrected before the suite rerun. CI stays disabled.


Review qualification additionally bounds profile projection to browser-referenced IDs and persists catalog-only one-time profile renames. The initial candidate build was deliberately stopped before sealing after discovering that imported-profile edge. The expanded owner suite initially reported 167 passed, one fixture failure and nine intentional real-viewer/browser exclusions: the cleanup fixture lacked the authoritative allocation now required by the new guard. The fixture is updated to publish its synthetic allocation, preserve its original owner snapshot, and verify that removing allocation eligibility prevents stale cleanup. This is not a weakened guard. The pre-existing skill-size gate is tracked in #325.

## Final source qualification

The catalog-only rename and bounded-projection regressions first failed (stored Old label versus New label; unused historical profile included), then passed after the correction. All four focused profile tests pass. The corrected actual cleanup-guard fixture passes and now explicitly rejects a stale allocation after eligibility removal. The original pending-retirement claim test passes with same-profile refusal and unrelated-profile admission assertions. The 167 passed owner tests from the earlier broad run are retained within their unchanged host/lifecycle scope; nine real browser/viewer tests remain explicit exclusions. The original broad run is not relabeled as clean. Strict workspace Clippy, Rust formatting, the original adapter repro, mounted Properties and health controls, client checks, docs build and changed links pass. The existing workflow-skill parity-size failure remains tracked in #325. No CI was enabled or dispatched.

A final schema check caught the additive profile map missing from the strict workspace schema (`additionalProperties: false`). The second uninstalled build was deliberately stopped, the optional display-only map was declared, and actual Rust projection keys are now checked recursively against that schema. No deployment used either stopped build. The selected-lane planning audit also requires exact header fields and a ROADMAP link; those documentation bindings are reconciled before final sealing.

The corrected strict-schema projection test, four profile tests, service contract metadata, full client bundle, current documentation build, selected P251 planning audit and final strict Clippy pass. API/MCP parity reports only the known #325 skill-size failure. Source is qualified for integration; actual installed behavior is still a separate gate.
