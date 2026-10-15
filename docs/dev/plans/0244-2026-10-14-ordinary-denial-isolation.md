# P244: ordinary denial isolation

Product lane: PL-BUGFIX
Work item: #288
State: IN_PROGRESS
Branch: fix/ordinary-denial-isolation

## Outcome

Restore Alice/Bob G05/G23/G42 for ordinary profile selection and existing-browser publication. Reproduce ambiguous historical owner binding before current policy checks, malformed custody history, and unrelated desktop release fences. Preserve current access policy, exact resource retirement and new-process deduplication.

## Custody and overlap

Reuse the clean canonical root checkout at a8b3f1a82 under this branch; no new worktree. P237 dashboard and P240 enrollment checkouts are clean and preserved. P237 owns independent lifecycle and UI work; this slice owns daemon profile selection, ordinary state-publication checks and focused regressions. Reconcile shared adapters by behavior before integration.

## Validation

Require deterministic red/green at actual profile selection and SQLite publication seams, focused Rust checks, format and strict workspace Clippy. Source and private synthetic validation only; no production or provider mutation. New-process admission and destructive recovery retain their exact resource checks; this slice proves ordinary work on existing browsers and host startup, not fresh-launch recovery from arbitrary database corruption.

## Feedback checkpoints

The installed public malformed-custody fixture fails with browser_session_publication_release_fenced and zero owned residue. Both actual SQLite ordinary-publication regressions fail with that same code before the repair and pass afterward. The shared-local owner-ambiguity test fails with existing_session_profile_identity_ambiguous before the ordering repair; its first draft had a Rust fixture move error, corrected before establishing the assertion failure. Native startup reads the same historical document and is included in this bounded regression set.

Clippy first encountered a cache-wrapper compiler-spawn failure before lint evaluation. Retained output is redacted; retry uses a sanitized environment, cache disabled and four jobs. This is not a source lint failure or a passed gate.
