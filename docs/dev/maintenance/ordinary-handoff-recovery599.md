# Retained handoff after ordinary browser exit

Product lane: PL-BUGFIX
Work item: 599
Branch: fix/ordinary-handoff-recovery
Base: 4f0b61959b97f0739374831a24cb0a0342998a02
State: INSTALLED_NATIVE_USE_OBSERVED_USER_VERIFICATION_PENDING

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
tests subsequently pass on integrated source
ae3fe011ebbbcb14d4731c146f5dc973ff9b2a64, including main through
df359a635308ceb58dca457fdae6bc0d224efdde. The exact commands are
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check,
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings,
and scripts/ci/rust-tests.sh --focused ordinary_handoff_recovery_tests.
All have zero compiler errors/warnings. Fifteen distinct affected tests pass on
the integrated pinned unit binary, SHA-256
20b7e419f686640c5a631a21dbfb20607311267cd84cc01bb8da05a9e84f36f2,
including the three mode-lineage and two Native close cases. Their negative
variants retain existing authority fences. Full logs and candidate/runtime state
remain intentionally private in the operator tenant; these fixture commands
reproduce the source checks without live credentials. No hosted CI was used.

An initial pin-inspection assertion assumed an absolute manifest directory was
embedded in the executable; that assumption is excluded as setup evidence.
The final pin is bound instead to the successful Cargo job, exact SHA-256, and
the composed two ordinary, three mode-lineage and two close test identities.
At that source checkpoint, draft PR653 published the source while sealed
candidate and installed acceptance remained pending. The independent preceding
upgrade reached accepted on its own generation; it was not this lane's effect.

## Installed qualification, November 12

PR653 merged as ed6dcf37cd17a03657e4f0b75d4b5a0c3a91031d, composing the
ordinary-exit repair with the current mode-restoration repair through PR654.
Formatting, strict workspace Clippy and both focused repair selections pass on
that exact source with zero compiler errors or warnings. Sixteen distinct
affected tests also pass on pinned unit bytes, SHA-256
cee4cfcb6a5565f0bcbdb4f2f85a3d64e80703eb8e2fa797cb97b6bbb042e9c2.

The clean production-shaped sealed release, SHA-256
dd600ab8ad22ba54138a55368a3dc97aa07c5a022695ac9c6be3fae5d66e38ad,
passes the workstation installation fixture and all five selected read-only
service smokes. Supported preserving adoption reaches accepted on generation
0.28.0-dd600ab8ad22-3c6fd6bacd94. No profile reset, manual database edit,
fabricated historical mapping, replacement invitation or credential replay was
used. The original saved profile's kernel-verified browser remains live.

The first authenticated original-link probe timed out during navigation. Its
receipt is retained; it is not a first-open viewer pass. A subsequent explicit
owner diagnostic resolves the original link, and later settled opens of that
same URL reach a connected Native viewer canvas. The owner additionally records
private Native provider evidence matching the exact target digest, viewer
connection, lifecycle/viewing generations and frame-sync acknowledgement.
Qualified first use is recorded independently of page navigation. This proves
installed same-link use after recovery, without claiming that the intervening
owner diagnostic was unnecessary or that the original short probe qualified
automatic first-open behavior.

Fresh OS readback confirms the saved browser is live and all seven isolated QA
scopes are inactive. Human custody remains held at the fresh reconstruction's
epoch, with paused reads and zero pending control outcomes. No SSO target DOM,
pixels or input were read during that hold. User verification/return and selected
SSO acceptance remain separate gates. Private evidence stays outside Git.

| Requirement | Source qualified | Integrated/installed | User verified |
| --- | --- | --- | --- |
| Ordinary-exit mapping and last URL | Yes, public owner regression | Merged and installed; ordinary-exit prevention remains fixture proof | No |
| Previously unmapped same-link reconstruction | Yes, fresh operation regression | Installed original link recovers saved profile and target | Pending |
| Changed target, missing receipt and prior refusal guards | Yes, focused fixtures | Qualified code installed; no adverse live effects attempted | Not applicable |
| Original authenticated viewer link | Connected canvas and exact-context Native frame acknowledgement | Same URL used after owner diagnostic; first short probe inconclusive | Pending |
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
