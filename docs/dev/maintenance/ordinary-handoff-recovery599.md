# Retained handoff after ordinary browser exit

Product lane: PL-BUGFIX
Work item: 599
Branch: fix/ordinary-handoff-recovery
Base: 4f0b61959b97f0739374831a24cb0a0342998a02
State: PRE_INTEGRATION_SOURCE_TESTS_GREEN

Recovering the original profile and last known tab through the existing human
link is established product behavior. An ordinary process exit must not require
an idle-retirement event that never happened. Preserve human control, paused
reads, invitation identity, and unknown authentication outcomes throughout.

The actual retained-link resolver fails with
browser_handoff_recovery_mapping_unrecorded; a second bounded invocation fails
with browser_handoff_recovery_publication_conflict. The replacement browser
has published launch custody but no matching idle-retirement or mode-conversion
record. Raw private runtime evidence remains in the operator tenant outside Git.

Ranked predictions: replacement publication omits the per-tab mapping when
ordinary exit has no retirement; unrelated metadata drift causes the publication
conflict; retained Native observation describes the old physical target.
The first prediction is exercised through the public owner resolver, SQLite,
real fixture-owned child process identity, and simulated replacement/provider
effects. Compiler or fixture setup errors are not product RED.

Two initial fixture runs compiled without warnings. The first omitted tab
admission; the second inherited the common child fixture's CDP-free default and
failed browser_session_recovery_identity_conflict, earlier than the reported
mapping failure. Both are excluded as reproduction evidence. The fixture now
admits its tab and explicitly selects CDP metadata while retaining a real,
fixture-owned process identity. No product code was changed for those failures.

The corrected regression reaches the exact live failure,
browser_handoff_recovery_mapping_unrecorded, in3.03seconds. Compilation has zero
errors and warnings. Product repair now records an independently versioned
ordinary-exit binding inside replacement launch publication, freezing the old
browser/tab scope and linking immutable launch custody. Readback requalifies
original kernel absence, exact current target and unchanged logical joins.
Pending retirement, profile operations, ambiguous launches, live originals and
changed targets remain fenced. Missing historical mappings remain missing;
this prevention repair alone does not repair a previously published bad link.
The expanded prevention test passes with the last recorded URL restored,
unchanged human epoch and paused reads, changed-target refusal, and deliberate
missing-receipt refusal. Five related tests pass on pinned exact unit bytes,
including the existing interactive fixture's adverse cases and both real-runtime
manual-idle fixtures. These are simulated provider effects, not installed proof.

A second regression reproduces the old-runtime missing-mapping case in14.61sec.
It constructs synthetic legacy state inside its isolated fixture, retaining the
original human invitation while dropping the new mapping. The new owner repair
qualifies immutable original/replacement launch custody, original absence and
the live replacement's kernel identity before freezing a fresh reconstruction
request through the existing exact-target reopen mechanism. The same link and
profile survive; the new tab and human invitation epoch are contemporaneous.
The old mapping remains absent. Both regressions pass. A single publication
conflict reread preserves the exact physical digest and all existing fences;
it repeats no focus, navigation or credential action. Its final source also
passes both regressions with zero compiler errors/warnings.

Ten distinct focused tests pass on final pinned unit bytes, SHA-256
1d6d1a93042a9f51aaa8bc8329a773b5c90da2e9d961de099e83148b37103f68.
Commands: scripts/ci/rust-tests.sh --focused ordinary_handoff_recovery_tests;
the same isolated unit executable with filters retained_interactive_handoff,
manual_idle_handoff, retained_handoff_recovers_after_settled_close,
closed_handoff and valid_handoff_recreates_expired_task_tab. The interactive
fixture includes its existing adverse cases. Policy wiring11/11, changed
documentation links and patch hygiene pass. Native provider/process launch
effects remain simulated; Linux fixture children supply real kernel identity.

Final Rustfmt/Clippy were not run on the pre-integration source: the admitted
job had not reached compilation and its exact own memory-capacity waiter was
cancelled after main advanced through652 and651. Its143 receipt is cancellation,
not a compiler/test failure. Strict workspace Clippy, formatting and affected
tests must qualify the integrated source before merge readiness or installation.

| Requirement | Source qualified | Integrated/installed | User verified |
| --- | --- | --- | --- |
| Ordinary-exit mapping and last URL | Yes, public owner regression | Pending integration | No |
| Previously unmapped same-link reconstruction | Yes, fresh operation regression | Pending integration | No |
| Changed target, missing receipt and prior refusal guards | Yes, focused fixtures | Pending integration | No |
| Original authenticated viewer link | No installed evidence yet | Not installed | No |
| Selected SSO completion | Incomplete, separate Auth qualification | Incomplete | No |

This admitted worktree owns browser_launch_custody_store.rs, its ordinary-exit
helper and tests, and the browser_session_host.rs reconstruction integration.
The separate active649 lane owns human-control/mode recovery;
preserve its checkout and changes. Shared catalog integration must retain both
intents. No manual runtime database edits, fabricated historical mappings, new
handoff links, credential replay, or custody return are repair mechanisms.

Source regression, affected refusal cases, Rustfmt and strict Clippy precede a
candidate. Source GREEN alone does not qualify the installed original link or
SSO. The same-link authenticated viewer must be checked after a supported,
preserving runtime adoption; no target-page inspection while human reads pause.
