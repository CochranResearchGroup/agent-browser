# P251: useful Properties and profile durability

Product lane: PL-PLATFORM

Owner: ecochran76. Work item: #323. Branch: `platform/dashboard-profile-properties`.
Status: IMPLEMENTING.

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

The API/MCP parity command fails the existing workflow-skill size cap: untouched main is 17,423 bytes against a 10 KiB limit. No service action was added; the new endpoint is dashboard-only. Preserve this pre-existing finding rather than claim the whole parity command passed. An initial new Rust assertion needed an explicit test-module import, corrected before the suite rerun. CI stays disabled.
