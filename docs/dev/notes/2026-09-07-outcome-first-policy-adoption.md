# Outcome-first policy adoption, 2026-09-07

## Decision and provenance

The operator authorized adapting LitScout's policy updates, including its latest
policy 0006 runbook compaction. Reviewed source commits: `640f14426b505861ae399fb2b0635c9b48a366e7`
and `03f6f4026bef63bdbc4a5ba472ce140fb5c5d442` in `../litscout`.
The installed selector remains v0.1.22, source
`12a7f9fef466522e99be44d980c44a4ff056f540`; its bundle was not upgraded.

This is a targeted custom composition for product engineering, biased toward
minimum total effort to acceptance. Existing runtime, ownership, integration,
release, and safety policies are retained. Merge updates into local 0028 goal
execution, 0042 testing, 0010 validation, and 0020 documentation. Add roadmap /
runbook governance as 0043 because local 0006 already owns branch strategy.
No duplicate module identity or new profile-wide adoption is intended.

These dated local overrides supersede conflicting generic continuation and
per-step ceremony in the installed selector templates. Authorization is not
itself evidence that another repair/verify attempt is worth its cost. Sustained
work needs a finite overall ceiling; successor plans inherit cumulative effort.
Two checkpoints or 30 active minutes without acceptance progress end the failed
approach unless an approved plan specifies another finite allowance. This does
not create routine approval requests or relax existing safety requirements.

## Wiring and application

[AGENTS.md](../../../AGENTS.md) routes the updated policies. Plan 0160 now inherits
the cumulative bounds and batch validation rules and points to the sole current
execution entry in [RUNBOOK.md](../../../RUNBOOK.md). Acceptance scope is intact.
The previous 9,016-line runbook is archived intact in the repository root to
preserve relative links. The active file retains current status and scoped stop
instructions. No known tracked incoming `RUNBOOK.md#...` links were found during
adoption. Historical plan transcripts remain in place; no general history
migration or roadmap reprioritization is included.

Memory discovery assessment: repo default remains use; this task uses current
source diffs and authoritative files, so additional historical discovery is
unnecessary. No shared skill publication or runtime action is part of adoption.

## Feedback and validation

The prior continuation rule permitted useful local repairs without requiring
aggregate acceptance progress. Repeated validation and duplicate status records
made the cost worse. The adopted rules address those failure patterns directly.
Reduced runtime cost under the new rules is not yet evidenced; a structural
policy audit cannot establish behavioral enforcement.

Validation passed: goal-only planning audit; unique module identities and
AGENTS wire-in paths; local Markdown link targets; whitespace checks; exact
archive byte comparison against the pre-adoption committed runbook. The active
runbook is 62 lines. Application suites and browser experiments were not run
for this prose-only adoption. The audit validates structural policy fields,
not runtime enforcement or production acceptance.
