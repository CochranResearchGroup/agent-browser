# Ordinary browser activity after unrelated metadata updates

Product lane: PL-BUGFIX
Status: CLOSED
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

## Merged and installed acceptance

PR721 merged as8ebf0c33a848dae081dc9dcda22859b78b37573c. Exact tracked-tree comparison against sealed source2feac50c2880317b6a8715a8eebaf88fe7f3cea8 is empty. Rebased14store and7adjacent cases pass; refreshed strict workspace Clippy passes in49.35s. Local checks satisfy validation; no hosted CI run was requested.

The supported production-shaped release/fat-LTO build completed in33m21s and sealed candidate-af11d43917c5d88b-abdbd60b73db6da5. Binary SHA256 abdbd60b73db6da5c1fb5a3b10d6f4b96e0201569007c2401b96c67ec50391d7. Supported install preview reports eligible; preserving installation upgrade-c7a38a1f-0558-4632-af2f-330dc9956409 reports ready/accepted. Selected generation0.28.0-abdbd60b73db-d083484bbb96 and on-disk installed binary match the seal. Doctor success is true. All8live managed browsers recorded immediately before installation preserve exact OS start time and executable after installation and after workflow acceptance. No profile reset or whole-browser shutdown.

Exact-binary installer fixture: first attempt failed at reconcile with SIGPIPE; both owned fixture hosts were cleaned. One bounded unmodified repeat passed, including source-free install and zero owned-host residue. Cause of the transient signal is unproven and no signal repair is claimed. Retain both receipts; do not describe this as an uninterrupted first-run pass.

Installed public workflow: original tool connection reported Transport closed after upgrade. A fresh installed stdio MCP client opened one disposable task. The initial heading assertion was unsuitable: the snapshot returned the correct example.com origin and Learn more link, but no heading even with non-interactive text requested. Acceptance was corrected to the observed origin plus expected link semantics, then passed on the same addressed task; task close succeeded. No second browser was opened for these snapshot checks. The test browser is absent from current published browser inventory; absence alone is not independent physical exit proof. Eight original managed processes remain intact. This is not LinkedIn acquisition, Enrich-button acceptance, controlled latency evidence, or attribution of the historical LinkedIn failure.

Private deployment, failure and workflow receipts live in the user-scoped activity-metadata-review runtime directory. Candidate artifact retained in the canonical checkout for installed rollback/support; review by2026-11-23. Startup build storage was62GiB with197GiBfree; the active sealed build is an explicit retention exception to the20GiB review threshold. No unrelated cache, checkout or browser cleanup.

Deployment closeout memory disposition:queued, job25b4fe57-3c91-4111-89eb-e0a39da772f9; user-scoped receipt activity-metadata-review/deployment-memory.json. Queue accepted; persistence and retrieval are not claimed.
