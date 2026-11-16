# Ordinary browser activity after unrelated metadata updates

Product lane: PL-BUGFIX
Status: SOURCE_QUALIFIED
Branch: fix/activity-metadata-admission
Baseline: 2817e3bae17512a8b90e80d1c7391ce744dae224

User requirement: stale metadata MUST NOT BLOCK valid activity.
Reuse the clean canonical checkout on a short-lived branch; no new worktree.
Primary writer: current repair; admission store, focused regressions and cause mapping.
Earlier IDLE-ACTIVITY-562 publication repair is retained; this repair owns admission before dispatch, not acknowledgement/publication. No other checkout edits.

Acceptance: two SQLite connections publish unrelated metadata then admit activity on an unchanged browser using the original snapshot. Preserve current metadata and reject actual changed browser/close scope. Error cause classification is a separate observed limitation, not the acceptance oracle for this repair.
No authenticated profile resets, replay of uncertain effects, browser shutdown, or runtime replacement needed for diagnosis.

Graphiti discovery timed out; source and executable local evidence are authoritative. Governance packet unavailable (manifest missing at server); relevant canonical repo policies read directly.

## Diagnosis

RED command: `cli/target/debug/deps/agent_browser-3104eecfbe9c33ba --exact native::browser_idle_retirement_store::tests::ordinary_activity_survives_unrelated_metadata_publication --nocapture`.
Two runs failed with `unrelated metadata must not block activity on the unchanged browser: Conflict`; direct repeat took0.25s.
Minimal difference is a single next_session_sequence increment published by a second SQLite connection. No browser, desktop, account or provider mutation.
Ranked predictions shown before the fix: whole-state equality rejects independent metadata; accidental browser identity change; active retirement record. Removing the broad equality while checking the selected current resource distinguishes them. Existing replacement/retirement controls remain mandatory.

Initial compiler-cache version query hung; stopped only the task-owned Cargo/cache processes and reran with AGENT_BROWSER_CARGO_CACHE=off. The first real compilation took2m37s; the compiled regression is subsecond on repeat. No product cause inferred from the build tooling failure.

## Validation checkpoint

Candidate fixes the first ranked cause: whole-document equality treated independent metadata as a conflicting browser effect. The minimized counter-only regression changes RED to GREEN. Current-root checks and durable close/activity exclusion remain inside one SQLite writer transaction; no stale snapshot is written back.

Focused store module:14passed. Adjacent activity-publication selection:6passed. Unknown-command original-process recovery:1passed. The first incorrectly qualified adjacent filter selected zero cases; corrected by enumerating actual compiled test names before the successful six-plus-one selection. Zero selected cases are not counted as validation.
Strict workspace Clippy with warnings denied passed in2m16s. Policy wiring11cases, changed documentation links, Remote View guidance checks and44-page documentation build pass. Next.js reported its existing multiple-lockfile workspace-root warning; build succeeded. Final formatted-source compilation and Rustfmt check pending.

Validation selector's installer/Guacamole suggestions are not applicable: no installer, payload, provisioning, provider or asset files changed; the output.rs change is one help sentence. Existing globally installed skill is not published by this source-only repair. No hosted CI, runtime installation or live provider action.

## Final source qualification

Final formatted-source store module14passed in2.92s after1m37s rebuild. Rustfmt check passed. Strict Clippy, policy and documentation gates above remain current: only formatting and the documented current-state shared-root lookup changed after initial GREEN, and the final binary was rebuilt. No debug instrumentation or throwaway harness was introduced; both regressions are retained at the real SQLite seam.

Branch remains local and uncommitted at baseline2817e3bae17512a8b90e80d1c7391ce744dae224; no merge or installation claimed. Source fix is complete. Review/integration and exact installed acceptance are the remaining delivery boundary. The live LinkedIn failure's specific underlying store cause was not proven; generic admission error conversion is unchanged and remains diagnostically ambiguous.

Memory disposition:queued; receipt ~/.local/state/agent-browser/activity-metadata-admission-memory-20261116.json; job37dea224-a3e9-44c5-bbb0-9743f905927d. Queue acceptance is not persistence/retrieval proof. No personal/provider content included.

## Requested review and deployment

Operator explicitly requests review, merge and deployment. Fixed point2817e3bae17512a8b90e80d1c7391ce744dae224; source diff and committed diff captured against that baseline. Origin main adds prior peer-attachment recovery PR718 and its evidence PR719; integrate before production build.

### Standards review

No accepted blocking findings. Source uses existing SQLite transaction and fixture boundaries, matching neighboring Rust conventions. The small physical identity predicate is local to admission and adds no framework or public contract. Required help/README/skill/site guidance is updated. Existing formatter, strict Clippy and policy checks pass. No documented-standard violations or actionable baseline smells found.

### Spec review

No accepted blocking findings against the stated stale-metadata requirement and bounded plan. Two live SQLite clients exercise the exact failure path. Current browser fields, current closed marker and nonterminal same-profile retirement are checked atomically; peer metadata is neither replaced nor silently discarded. Physical replacement controls and adjacent uncertain-command recovery pass. Generic error-cause conflation remains an explicit separate limitation rather than an alleged live root cause.

Summary: Standards0accepted blocking; Spec0accepted blocking. User authorizes normal reviewed integration and preserving deployment; no formal release/version bump, profile reset or broad browser shutdown.
