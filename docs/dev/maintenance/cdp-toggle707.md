# CDP toggle continuation repair

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/707
Branch: fix/operator-mode-restart707
Base: a354cb4411144b85d1677c02485354a899d64013
State: RUST_SOURCE_QUALIFIED; pinned installer fixture and installed external acceptance pending

The actual external Native workflow completes CDP-off, then leaves the CDP-on
replacement awaiting Native qualification. One replacement process exists;
reconciliation returns `browser_session_launch_publication_conflict` instead
of inspecting that existing launch. This blocks the original navigation-loss
acceptance and must not trigger another process launch.

Reuse the clean canonical primary. Own host refresh, observation-only window
readiness, profile startup arguments and focused regression tests; preserve
the completed mode649 source, all foreign clean checkouts,
retained candidates, original uncertain requests and the exact pending test
replacement. Runtime identifiers and synthetic browser artifacts stay outside Git.

First reproduce the owner refresh failure with the real SQLite launch ledger
and RemoteViewSessionEffects. A peer observation must not force an unqualified
replacement to publish against the original process state. Retain durable launch
custody, local changes and no-relaunch fencing. Then qualify the original
external operation and assert its URL before navigation. The saved-page defect,
Chromium row and full toggle acceptance remain open until observed.

Use one bounded source repair and review cycle. Run focused tests, Rustfmt,
strict workspace Clippy and selected local checks before merge readiness.
No hosted CI. Production effects require the existing sealed candidate and
preserving installation gates; source checks do not qualify the live workflow.

## Reproduction

The external Native workflow completed CDP-off with one launch and correct
debugging posture. CDP-on launched one exact replacement, then retained
`ReplacementPending` with `mode_operation_native_unavailable`. Explicit recovery
returned `browser_session_launch_publication_conflict`. Native observation
finds the replacement's window titled `about:blank - Google Chrome`.

A four-second isolated Chrome restart probe reuses the observed CDP replacement
arguments with a fresh synthetic profile. It loses the original page in4.532s.
Changing only the saved-session restore flag retains the URL and localStorage
marker in4.300s. This isolates the missing startup request; it is not full
external UI acceptance.

Three Rust regressions cover startup intent, pending-launch refresh after a
peer update, and observation-only window readiness after a peer update. Their
first run is queued through Cargo Signal and the repository resource wrapper,
using four jobs, an8GiB claim and hard limit, and the unchanged16GiB host reserve.
The prior default claim was cancelled while waiting, before compilation.
Policy wiring11 cases and changed documentation links pass.

The owner-refresh and startup regressions failed with the expected publication
conflict and missing restore argument. The readiness fixture initially lacked
its catalog; after fixture correction it failed with
`browser_native_target_changed:refresh task`. These are distinct receipts;
the initial fixture failure is not counted as readiness evidence.

The focused `pending_mode_replacement` group now passes5 cases. Owner refresh
begins a fresh observation scope only when no local logical delta exists,
retaining the durable launch record. Both startup paths share the restore
decision. Observation-only readiness keeps process/window identity checks and
leaves publication and effectful input state fences intact. Broader host/runtime,
mode/control, formatting and strict workspace Clippy gates were selected.

## Shared refresh dependency

Concurrent work711 independently reproduces the same cached pending-launch
failure across two profiles. Its worktree is preserved.711 owns integration
of the shared refresh predicate and its two-profile regression;707 owns saved
session startup, observation-only readiness and the mode-specific regression.
The compatible predicate currently in this checkout is frozen validation input.
Integrate711 first, then consume its qualified predicate before707 merges;
do not publish two competing implementations. Its separate runtime-host restart
preserved the pending replacement and changes the installed baseline, so live
recovery requires a fresh process readback.

## Source checkpoint

Focused5, host40, runtime36, Rustfmt and strict workspace Clippy pass. Runtime
qualification retains8 intentionally ignored cases. Pool compilation was
interrupted at the execution ceiling; pool and mode/control gates remain
pending. The qualification job exited130 with76 passing tests and no test
failures. This custody checkpoint is not merge-ready.
The original installed operation reconciled after711's host restart with its
single replacement intact and still returned aboutblank. Manual navigation
confirmed the original storage marker. Its ordinary close reported residue;
one freshly observed, exact-process Native window close then ended that process.
Peer browser processes remain protected; disposable cleanup follows normal GC.

Review against6556af69d separates Standards and Spec. No source standards blocker
was found. Spec requires installed external acceptance and the qualified711
dependency before this work is complete. Effectful Native guards, atomic launch
publication and exact process/window joins are retained.

Draft PR712 preserves commit3c865df09 and this source checkpoint. The recorded
two-hour overall diagnostic ceiling expired; a one-hour extension was requested
and remains unanswered. No production candidate, installation or fixed-build
external acceptance is claimed. Resume the pending local gates and qualified711
dependency before candidate qualification and fresh supported-browser rows.

## Resumed qualification

The operator authorized completing validation, PR integration, a qualified
production candidate, preserving installation and fresh workflow acceptance
in the veto-diagnosis session. This resumes the previously stopped packet;
it does not replay the original uncertain launch requests.
Retain the prior two-hour effort in cumulative accounting. This explicitly
resumed delivery window has a two-hour ceiling from2026-11-15T04:00Z,
one production build, one preserving installation and one fresh round trip
per supported browser. Reassess at the30-minute no-outcome boundary; source
qualification, integration and installed workflow acceptance remain separate.

Policy wiring11 cases, changed documentation links and patch hygiene pass
again. The pool retry first stalled in the compiler cache's version probe
before tests began and was interrupted with exit130. Its retained retry uses
the supported cache opt-out, four jobs, an8GiB claim and hard limit, and the
unchanged16GiB host reserve. Pool and mode/control jobs retain private receipts.

The existing Native-control integration target failed compilation because its
test crate omitted the login perception and OCR modules referenced by its
imported production code. Add those module declarations so this planned gate
can execute; this is fixture wiring, not a new browser failure. Update the
readiness comment to describe exact process/window observation accurately.
The provider711 merge remains a prerequisite to707 integration.

PR713 is now merged as85c98e7ab and integrated into this branch. The refresh
function is byte-identical to both qualified parents. Glossary and lane-catalog
conflicts preserve both intents; policy wiring and links pass after resolution.

Mode-operation integration passes17 cases. Native-control compilation now
succeeds, exposing one stale close fixture:61 cases pass and one fails with
`browser_native_close_control_unavailable`. The exact single-case binary loop
reproduces in under a second. The fixture predates current control-status
admission and confirmed, epoch-bound Native window close; it still expects
Alt+F4. Update only that fixture's owner reply and assertions, retaining exact
window/process/control identity and the dispatch-versus-exit distinction.
Its red receipt remains retained; the full affected integration target is rerun.

The corrected Native-control target passes62 cases; mode-operation integration
passes17. Final strict workspace Clippy and formatting pass. The pool job
exhausted its1800-second command budget after67 passing cases and no reported
test failure, including10m42s of compilation. Its interrupted case has no
verdict. Resume exactly the81 unfinished cases with a hash-pinned copy of that
same test executable, an isolated home and serial execution. List verification
proves that the continuation excludes every completed green case. This is a
disjoint continuation, not a clean full-suite result until it completes.

The external harness passes syntax and independent Cooper sign-in. The first
catalog helper used an invalid relative API URL; correcting its base resolves
that harness error. Authenticated catalog readback then returns503
`selected_backend_unavailable` on the prior installed generation. No application
browser is launched. All ten temporary controller process identities are absent
after scoped closure. This is baseline availability evidence, not fixed-build
acceptance or a global zero-residue claim.

Concurrent711 work is preparing a production package. Runtime ownership for a
combined707/711 candidate is being reconciled before another build or upgrade;
source validation continues independently. No707 production build or installation
has started in this resumed window.

## Qualified source checkpoint

The continuation completed79 passing cases, one failure and one ignored case.
With the initial67 passes, that is146 passes, one ignored case and one failing
argv-recorder fixture. Its isolated retry passed, so the lane was not called
clean. The fixture rejected an unavailable identity observation even when the
recorder had become the exact unreaped zombie.

Extract its existing predicate unchanged. A small regression with a real zombie
and unavailable identity observation is deterministic RED in1.04s. A fixture-only
raw-stat check bound to the recorded boot identity and start token makes it
GREEN in1.038s. Live or changed processes remain refused. The original manual
ordinary-exit caller passes after rebuilding. Production process checks and
cleanup authority are unchanged. Retain146 unaffected passes and one ignored
case; that repaired caller supplies the147th passing case. Only that caller
enables the argv recorder.

Final formatting, strict workspace Clippy and the normal CLI-package `argv_probe`
target pass. Host40, runtime36 with8 ignored, focused5, mode17 and Native-control62
remain qualified by unchanged inputs. Relevant executed commands include:

```bash
scripts/ci/rust-tests.sh --focused pending_mode_replacement
scripts/ci/rust-tests.sh --focused browser_session_host::
scripts/ci/rust-tests.sh --focused browser_session_runtime::
scripts/ci/rust-tests.sh --focused pool_tests::
scripts/ci/rust-tests.sh --focused retained_manual_handoff_recovers_after_ordinary_browser_exit_through_real_runtime
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --test remote_view_native_control --test browser_mode_operation -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --test remote_view_native_control -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --test argv_probe -- --test-threads=1
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
```

The combined integration invocation retained its first failure; the corrected
target was rerun separately. Hash-pinned pool continuation excluded green cases.
Private receipts bind commands, results, source readback and binary identity.
These are selected Linux/WSL checks, not comprehensive or platform-matrix CI.
No hosted CI, formal release or707 production installation is claimed here.

## Operator-commanded restart repair

Product lane: PL-BUGFIX. The operator has renewed implementation direction:
mode conversion must complete the requested browser restart, preserve all tabs
and session names, record the replacement process, and update every consumer of
the old browser. An exited PID is expected transition state, not refusal evidence.

PR712 merged at a354cb441. Its exact production candidate was installed and
installation doctor passed. Fresh Chrome conversion stopped before replacement
launch with `browser_session_recovery_custody_conflict`. Reconciliation established
`LaunchAbsent`, not successful conversion. Chromium was not run. The unresolved
operation and synthetic profile remain outside Git; the independent controller
was closed with a fresh process census.

The renewed source repair has one two-hour window from2026-11-15T15:15Z to17:15Z,
with the previous delivery effort retained. Build a failing restart regression,
repair the actual cause, validate three-tab and shared-session continuity, and
run the selected local Rust quality checks. Reassess before another build or
installed acceptance cycle. No hosted CI or speculative replacement launch.


## Restart refusal audit

The continuation audits the complete owned restart path: graceful process exit,
exact-profile absence, assignment admission, one replacement launch, restored
page acquisition, Native readiness, atomic publication, logical session and tab
references, hosted desktop bindings, and durable Remote View handoffs.

The same-desktop custody regression failed before its repair. The three-restored
page regression failed when recovery created duplicate pages. Recovery now claims
unattributed restored pages, prefers the recorded URL, and preserves each restored
document without replaying navigation. Shared-session fixtures initialize Bob's
lazy first tab before asserting three tabs; setup failures are not product REDs.
The corrected model recovery and session suite passes69 cases.

A real SQLite publication-readback regression changes only the session sequence
and fails with `mode_operation_publication_conflict` despite an exact published
process. The same test also changes the browser PID and requires rejection.
Publication readback now consults current durable state and exact launch custody;
publication itself uses the existing atomic reconciliation transaction instead
of a second whole-service equality veto. A separate admission regression checks
unrelated peer bookkeeping before an exited browser restarts on the same desktop.

Retained integrity checks cover changed task/profile/build selection, actual
profile occupancy, unresolved replacement launches, changed kernel process
identity, conflicting assignment ownership, incomplete unique tab mappings and
changed handoff lineage. Their presence is not evidence of a false refusal.
Source regressions and live supported-browser acceptance remain separate gates.


The expanded custody loop reproduced four independent false refusals: admission,
owner-release recording, absent-claim reconciliation, and exact-profile absence
settlement. Each rejected unrelated profile operations or task counters. The
repair compares this profile's operation and launch inputs while retaining exact
browser, assignment, unresolved-launch and occupancy checks. A hosted-consumer
case independently reproduced rejection of a valid owner restart; its existing
profile operation now permits that transition. Changed target-operation cases
remain conflicts in each step.

Review also reproduced a genuine integrity gap: publication accepted a changed
operation on the same profile. The selected-profile comparison now runs inside
the existing SQLite publication transaction, after the exact idempotent case.
This is distinct from removing whole-service bookkeeping vetoes.

The supported chooser defect is confirmed independently: both testing-brand
version strings were classified as ordinary Chrome. They now have the distinct
`chrome_for_testing` identity, excluded by the existing Chrome/Chromium chooser
filter. The regression invokes the real executable-inspection path. It does not
qualify a testing browser as a supported workflow.


## Final source qualification

The PID-reuse loop also failed before repair: a positively different kernel epoch
at the recorded PID was treated as the original browser. Recovery now recognizes
the exited recorded identity while refusing a matching live original and unknown
observations. All21 custody cases, final formatting and strict workspace Clippy
pass after that last production change. The prior selected CLI batch passes155
cases with8 intentionally ignored live-browser cases; its136 unaffected cases
are reused, with custody rerun on the final source. The full Service Model lane
passes424 cases. Both mode integration17 and Native control62 are in the selected
CLI batch. These are Linux/WSL local checks, not hosted or platform-matrix CI.

Executed source commands include:

```bash
scripts/ci/cargo-safe.sh test -p agent-browser-service-model -- --test-threads=1
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/rust-tests.sh --focused browser_launch_custody_store::tests::
scripts/ci/rust-tests.sh --focused testing_brand_is_distinct_from_supported_chrome
scripts/ci/rust-tests.sh --focused browser_session_host::tests::
scripts/ci/rust-tests.sh --focused browser_session_runtime::tests::
scripts/ci/rust-tests.sh --focused handoff_lineage_tests::
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --test browser_mode_operation --test remote_view_native_control -- --test-threads=1
pnpm test:policy-wiring
pnpm test:documentation-links -- --base a354cb4411144b85d1677c02485354a899d64013
pnpm test:remote-view-handoff-docs
pnpm --dir docs build
```

The initial workstation installer fixture used a mutable debug executable that
Cargo replaced during the run. Its later payload failure is retained as an
invalidated harness run, not a product RED or a passing gate. The rerun pins an
immutable debug copy. Installer/support inputs and browserless fixture paths are
unchanged by the later PID-epoch branch; production-profile candidate checks and
installed doctor remain separate. Other selected host-provision, VM-harness,
Guacamole-asset, PostgreSQL-durability and route-user-sync fixtures pass.

Standards review accepts no remaining source finding. Spec review resolved the
atomic selected-profile publication gap and confirms that all active sessions,
logical tab IDs, hosted desktop bindings and durable handoffs use the replacement
browser record. Source qualification does not assert saved-page or native-viewer
acceptance. The pinned installer fixture is still pending at this custody commit;
merge readiness, sealed candidate reuse, preserving installation and both actual
supported-browser rows remain open gates. No hosted CI or formal release.
