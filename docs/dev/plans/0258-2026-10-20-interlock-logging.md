# P258: bounded runtime interlock logging

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#294
Branch: fix/interlock-log-summary
Owner: ecochran76

## Outcome and custody

Stop routine interlock runs from publishing the full process/authority census
to journald. Preserve maintenance, fencing, cleanup checks, exit status and
actionable failure reporting. Keep detailed diagnostics explicitly available
in the current private monitor receipt and requested JSON output.

Reuse the clean dashboard primary from freshly fetched canonical `3ba078c9c`.
Preserve its sealed candidates and the foreign root/resource worktrees. This
writer owns the scheduled unit templates and reconcile output only; P257 is
integrated and the resource-bounds lane retains lifecycle ownership. No
production mutation, browser cleanup, provider effect or CI dispatch.

## Feedback and acceptance

First reproduce the scheduled unit selecting verbose JSON. Test byte and line
bounds with a large synthetic census, explicit JSON detail preservation,
actionable errors and unchanged failure status. Qualify Rust formatting,
strict workspace Clippy, unit templates, shell syntax and all user-facing
guidance. Source acceptance remains separate from installed timer acceptance.

## Source acceptance

The regression failed on the old JSON-emitting unit and passes after both unit
producers select bounded routine output. Eight focused tests pass, including
large-census detail preservation, hostile fields, failure exit status, ambient
JSON configuration, monitor backoff and source-free units. See the
[source receipt](../notes/0258-2026-10-20-interlock-logging.md). Installation and
actual journal-volume acceptance remain separate.
