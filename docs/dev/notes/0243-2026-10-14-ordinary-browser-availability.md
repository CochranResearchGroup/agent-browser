# Ordinary browser availability repair

Date: 2026-10-14
Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P243, #284 and #286
Related lanes: PL-PLATFORM

## Contract and change

The [Alice/Bob reconciliation](2026-10-08-alice-bob-contract-reconciliation.md)
retains G05/G23 availability and the preserved G42 liveness decision: current
heartbeat and direct browser/process observation govern ordinary use. Historical
ownership and custody records are diagnostic, not permission authority.

Both native download handlers no longer require or compare a historical runtime
owner binding. They retain current process identity, exact target/frame and
GUID correlation, process revalidation and process-bound artifact delivery.

Ordinary Remote View effects no longer refuse work because a process-local or
profile-wide launch claim remains pending. Current profile/process/CDP and native
window observations can join an interrupted launch to the existing atomic
publication. The ordinary whole-state writer no longer treats pending history
as a blanket veto. Identical publication replay is idempotent. Recovery retains
the managed lifecycle for a matching observed own launch while preserving an
externally discovered borrowed browser.

Absent observed launches now reconcile using current full-profile/process
absence plus an exact SQLite compare-and-save. They do not require historical
owner/publication proof. Original observations remain intact; only their
occupancy is retired. Current exact desktop-retirement fences and process-effect
deduplication remain separate from ordinary-use permission.

## Red and green evidence

Installed generation `0.28.0-53f9c81eee8b-93bb61bc8815`, executable SHA256
`53f9c81eee8bcc256fb5875abd44edf06f23d141c0f480aef56d52db57abfe09`,
fails the asserting synthetic existing-profile download with
`download_source_identity_unproven: current owner binding unavailable`.
The new public fixture also reproduces
`browser_session_publication_release_fenced` on concurrent continuation after
adding stale custody only to its disposable SQLite database. Both reds clean up
to a fresh zero-owned-process census.

The deterministic ordinary-host/SQLite observed-launch regression is red on
baseline `cb765a7be`. The repair publishes the already-running observed browser
with the same logical session/tab and no new process launch. Further cases prove
that mismatched history does not block a live browser, a dead observed launch
reconciles after current absence, and later recovery after mismatched-history
continuation does not depend on an old published owner row. Concurrent identical
publication replay and interrupted state retain atomicity.

Integrated source checkpoint: `2278205344cb8045b77184aabf50b44995a613e3`,
including canonical `0c1fde54dfb6e845503eb569b0d04cdf264f7231` (PR285).
Pinned debug executable SHA256:
`cea1675d4cb96056abce475fe73b0d3266fa03f776ab87d3bfd3706aafeca6aa`.
It is a source-validation executable, not an installed or production-promotable
release candidate.

Final public qualification passes on that pinned executable: exact synthetic
download bytes and task/browser/tab continuity; two concurrent continuation calls
with stale published/unpublished custody retain the exact live OS process/start
token and still download successfully; Alice/Bob persistent-profile sharing,
page-preserving replay, scoped closure, borrowed-browser preservation, missing
profile and disposable-task workflows pass. Each fixture reports a fresh zero
owned-process census. These are isolated synthetic results, not real-account or
native-viewer rendering evidence.

## Selected validation and preserved failures

- 60 ordinary consumer tests pass after canonical integration.
- The final extended mismatch-history/later-absence regression passes separately
  after the last bounded reconciliation repair; six launch-store tests pass.
- Eleven native runtime tests pass, with six real-browser tests ignored. Public
  synthetic browser checks separately qualify the selected workflow.
- Four download-artifact tests and 39 output tests pass.
- All 330 integrated Service Model tests pass on the resource-bounded retry.
- Format, strict workspace Clippy, docs build, changed-link validation, policy
  wiring and Remote View guidance checks pass.

The first integrated Service Model build failed before assertions because the
OS refused to spawn two linker processes (`EAGAIN`). A single retry with four
Cargo jobs passed. Preserve that infrastructure failure; it is not erased by the
successful retry. The older baseline policy-wiring test failed against a retired
policy alias; canonical PR285 already supplies its independent correction.
These are selected source and isolated workflow gates, not a comprehensive CLI,
platform matrix, production, provider or soak claim. CI remains disabled.

## Standards

Primary self-review against the canonical base found no remaining blocking
standards finding. The implementation stays in existing download, session,
Remote View adapter and SQLite boundaries; no new authority, viewer or recovery
API was added. The documentation surfaces agree. Independent fixture setup is
retained to keep each failure scenario isolated.

## Spec

Primary spec review used #284/#286, P243 and preserved Alice/Bob G05/G23/G42.
The old published-owner prerequisite found during review was removed and locked
down by the extended regression. No blocking finding remains for this bounded
ordinary-use repair. The original manual service-recovery executable/presentation
symptoms, real-account authentication and native provider rendering have not been
independently requalified. They are not claimed as successful outcomes.

## Boundaries and custody

PR287 is the canonical integration boundary. Preserve P237/PR270's separate dirty
lifecycle/UI work and reconcile its overlapping adapters without restoring the
removed historical vetoes. No foreign checkout or private runtime records were
modified. No production installation, service restart, external provider,
real account or broad cleanup effect occurred. The installed selector still
points to the reported failing generation at closeout readback.

Local diagnostic logs remain under the task-scoped `/tmp/p243-` prefix; this
curated receipt preserves the substantive evidence and limits without tracking
private runtime state or full logs. [The plan](../plans/0243-2026-10-14-ordinary-browser-availability.md)
and [PR287](https://github.com/CochranResearchGroup/agent-browser/pull/287) own
source custody and the remaining live-acceptance boundary.
