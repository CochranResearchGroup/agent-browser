# Donor recovery after completed retirement

Product lane: PL-BUGFIX
Parent: P300, ticket 530
Branch: fix/p300-donor-retirement-readback
Base: e2785ee97a273d192bc9a8985c3f37795be3e6fe
State: CLOSED; donor recovery and input/handoff acceptance passed

## Bounded outcome

Ordinary browser commands must acknowledge exact completed durable retirement
receipts before rejecting stale process-local custody. Unknown or unpublished
effects and unsaved navigation remain fenced. Acknowledgement reads owner
receipts and does not close, launch, focus or navigate a browser.

The selected donor later exited through an acknowledged idle retirement.
Its retained task metadata is intentional. Fresh ordinary owner commands reject
with browser_session_publication_conflict. Resident and durable public aggregates
agree; hidden operation fields prevent claiming a conclusive live root cause.
This later recovery blocker is distinct from the original no-input report.
Private identity, account and raw runtime evidence remain outside Git.

## Ownership and acceptance

Use the sole existing clean checkout. The source worker owns host/effect code and
regressions. Root owns this note, catalog, serial integration and runtime effects.
No additional checkout, state-file repair or raw shared-host restart is admitted.

Prove a completed peer receipt permits an ordinary command with zero repeated
browser-close effects. Keep existing unpublished-navigation and unknown-custody
refusals. Run focused regression tests, affected host/effect tests, workspace
format and strict Clippy, plus relevant documentation checks. Canonical source,
preserving installation and actual donor/input recovery are separate gates.

Ticket 530 still requires the fourth actual account/storage/cleanup row. Final
P300 broad review and post-review remediation counts remain zero. Source tests
here are bounded blocker repair, not the final whole-spec review/remediation.

## Reproduction checkpoint

The public owner regression
`completed_peer_retirement_does_not_poison_an_ordinary_host_command` failed on
the unmodified implementation. Its original close settled, an independent peer
completed the same SQLite custody record, and an ordinary open for another
profile returned browser_session_publication_conflict. Setup and peer completion
assertions passed. Test execution took 3.24 seconds; compilation and execution
together took 352.143 seconds with no compiler errors or warnings.

Command: `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused completed_peer_retirement_does_not_poison_an_ordinary_host_command`.
Cargo Signal retained the exact test patch, inputs and failed attempt privately.
The preceding cache-enabled attempt stalled before compilation and was cancelled
by exact owned process identity; it is not a failing test result.

This establishes the source hazard. It does not prove the original mouse failure
or conclusively classify the resident owner's private unpublished state.

## Source qualification

The repair acknowledges only the exact completed successor of a cached retirement.
Intent, preparation, target scopes, effect settlement and previously recorded
publication/completion identity must remain unchanged. This receipt read makes
no provider call, process observation or state publication. Unpublished local
state and pending launch custody still veto an ordinary refresh.

Local checks on the six-file formatted code patch
`22b7f8c94861706c90852a008af1634307ae78740d85c637f6a4e09c33a7775b`:

- Focused `peer_retirement`: 2 passed, including unknown-completion refusal.
- Focused `ordinary_refresh`: 1 passed, preserving unpublished navigation.
- Focused `automatic_retirement_`: 14 passed.
- Focused `failed_retirement_publication_does_not_poison_an_unrelated_task`: 1 passed.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: passed.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: passed.
- `scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser-service-model -- --test-threads=1`: 416 passed, no ignores.
- Policy wiring: 11 passed. Changed documentation links and patch hygiene passed.

Cargo Signal CLI receipts retain every attempt outside Git. Compilation used the
existing shared Cargo target and safety wrapper with compiler caching disabled
after the initial cache stall. Rust formatting corrected three wrapping-only
differences; source semantics and earlier behavior evidence are unchanged.
Validation used Linux/WSL, Rust 1.94.1, synthetic providers and disposable SQLite
fixtures. Hosted CI was skipped by operator policy. Canonical integration,
sealed-artifact qualification, preserving installation and actual viewer input
remain separate gates. No account acceptance is inferred from these checks.

## Installed recovery checkpoint

PR 555 integrated the first repair at canonical commit
`2fab4b793ab835fed169911f1b591d4c32489f5b`. The qualified sealed production
artifact passed its installer fixture and normal preserving installation.
Installed doctor passed. The publication conflict cleared, and the ended donor
task was normally retired while preserving its profile data.

A fresh open then exposed a separate allocator blocker. Read-only inventory,
window counts and durable custody show that selection chooses the first empty
desktop despite its pending idle-retirement reservation. Admission correctly
refuses it before creating a launch claim. Another empty desktop has no such
reservation. Pool eligibility must exclude retirement reservations before
selection, retaining the final admission fence. No reservation was settled or
removed to work around this failure, and no blind launch retry was issued.

The original viewer-input report remains unqualified until a live donor can be
tested through dev-browser and independently observed desktop input. Ticket 530
remains open; three copy rows are accepted and disposed, with the fourth unrun.

## Allocator repair qualification

The real SQLite pool regression failed before the patch because both desktops
were offered instead of only the unreserved desktop. The repair composes the
existing launch eligibility check with a validated read of nonterminal retirement
desktop reservations. It preserves reservation history and leaves transactional
launch admission unchanged. A concurrent reservation still fails at admission.

The source patch digest is
`81852f57aaf5aa093c6eb86945047df0ce13b45731c89d31dd03f1296cc860ee`.
Local checks passed: nine `pool_demand_` tests, fourteen
`automatic_retirement_` tests, workspace format and strict workspace Clippy.
The initial format check found two wrapping differences in the new regression;
formatting was corrected and the final check passed. Policy wiring passed all
eleven tests. No hosted CI ran. Sealed build, preserving installation and actual
viewer-input evidence were qualified in the following checkpoint.

## Preserved donor and Native input checkpoint

PR 556 integrated the allocator repair at canonical
`db8da1e3f1571b9750eb205dfeb0eba68b23f903`. The source-bound production candidate
passed its normal preserving installation and installed doctor. All nine prior
browser roots and all 125 recorded Native process identities were preserved;
one browser child exited without attributed cause. The restored donor uses the
selected durable profile and remains explicitly preserved. No pending foreign
retirement was settled to obtain it.

Autonomous dev-browser diagnosis then proved a separate Native input defect:
per-edge owner callbacks stretched a two millisecond tap into a 661 millisecond
physical hold and generated an unwanted repeat. Remote View PR 812 fixed bounded
read-ahead while preserving fresh proof per batch and live route/epoch checks.
Its canonical source `98287ca9fca749903cb3b0dc96963674615951e4` passed 624 Rust
tests, 83 viewer contracts, strict lint and formatting, then normal source-bound
release assembly and 28-role software adoption. All 100 physical native processes,
ten prior browser roots, generations and protected configuration were preserved.

The actual Native viewer delivered exact repeated letters, digits and punctuation
and a trusted click to an owned synthetic page. An initial immediate click/type
attempt missed its first letter; the retained exact pass allowed focus to settle.
Independent XI2 timing measured the repaired sample tap within one server
millisecond. Both owned probe tabs and servers are closed, and normal control
return passed. Remote View issue 811 is closed through its qualified plan 0228;
its auxiliary checkout was archived and removed without touching foreign lanes.

A later ordinary handoff refresh again reports publication conflict while the
donor process remains alive and explicitly retained. Public resident state agrees
with durable state, which excludes neither hidden operation drift nor a stale
snapshot. The same normal preserve operation also refuses the conflict. The
current bounded repair must establish a real hidden-metadata regression and
refresh only a clean owner snapshot, preserving unsaved local work and unknown
effects. No state-file editing, donor relaunch or raw shared-host restart is
admitted. Original account navigation must be restored before human handoff;
no protected account content has been captured or authentication entered.
That navigation was restored through asserted active Native input, and an
address-bar-only observation confirmed the account verification redirect.
Fresh Native control status proves normal return to the agent afterward.

Ordinary status does not expose the private baseline or effect cache needed to
distinguish those guards. A bounded stderr-only diagnostic draft was prepared.
Its regression failed at the missing-log assertion; its follow-up was cancelled
without an accepted test result when fresh checkout inventory revealed the
operator-diagnostic lane's explicit publication-reconciliation continuation.
The draft is archived privately and removed from this checkout. No diagnostic
candidate was built or deployed. A speculative hidden-metadata refresh is
insufficient: external metadata drift already refreshes a clean local aggregate.

The operator-diagnostic lane is now primary writer for host and SQLite
publication reconciliation under work item 558. Its source contract preserves
uncertain original effects and both writers' records, retries publication only,
and confines conflicts to actual resources. P300 depends on that qualified
canonical repair, then consumes one coordinated preserving deployment and exact donor
readback. Consume no unmerged checkout bytes, duplicate source edits or competing
runtime installation. The foreign checkout, candidate, tests and processes stay
with that lane. This coordination does not claim the live conflict is solved.

PR 557 integrated the qualified reconciliation at canonical
`a5639bc0fce82ff7cbe8cd3f61ca1e41bf8b4258`, including source `2ce59d782`.
The canonical validation receipt records 25 host/reconciliation and 71 affected
tests, strict workspace Clippy, formatting and documentation gates. Source hashes
match the receiving checkout. The earlier broader browser run is retained with
its failures and scoped resolutions; this is not a new full-suite pass. The
primary writer is building one sealed production candidate and will preserve
both programs' owned browsers. Installation and fresh donor handoff are pending.

Before adoption, a focused consumer check found three remaining handoff entry
guards comparing stale state before reconciliation. The real peer-advance test
failed with the exact publication conflict, then passed viewer issuance,
handoff inspection and Native opening with no effect replay. All 97 surrounding
regressions pass against the pinned test binary. The primary writer performed
this qualification in the receiving checkout while its first release checkout
remained frozen, then preserved the patches and restored the exact clean head.

PR 560 integrated the corrected source `ffd3432d75f9c0179fcf826c874d77edffd8932f`
at canonical `c2a8b879aed6bab3b57b087120ff8d392e145780`. The earlier sealed
candidate is superseded and uninstalled. One corrected production build is now
running under the same primary writer; installation and live handoff acceptance
remain pending. P300 does not start a second build or competing installation.
The exact required workspace Clippy receipt was read independently: exit zero,
zero warnings or errors, 174.806 seconds. This is the required workspace command,
not a claim that the expanded all-targets baseline passed.

Ticket 530 remains open with three accepted and disposed copy rows. Its fourth
row and P300 final broad review/remediation remain unrun.

## Installed donor handoff acceptance

The coordinated preserving installation accepted transaction
`upgrade-91498402-31ed-4edf-bcb5-cc9140c20923`, generation
`0.28.0-acd74b124577-2f0ec3b99bee`. Installed SHA256 is
`acd74b124577d9e9129fa51e3378b3de9937606470b6368ad5c882369b0d239e`.
The exact-binary workstation fixture and fresh installed doctor passed. Canonical
acceptance PR 561 records the shared installation and original-workflow results.
The older sealed candidate remains superseded and uninstalled.

P300 independently verified the same donor process, start token, executable,
selected profile and absence of CDP flags. Its fresh handoff reports ready with
owned-browser-window proof. Dev-browser followed that authenticated opaque link
into the Native WebSocket viewer. Normal takeover enabled mouse control; actual
Chrome toolbar focus and exact public text entry were observed. Escape restored
the original account-verification route without Enter, authentication-form input
or page-body capture. Normal return completed, all physical keys were released,
and only the owned viewer tab closed. The borrowed blank tab remains retained.

All 125 prior Native process identities and both protected browser processes
survived. One unrelated stock-Chrome root was absent across the longer observation
interval; its cause is unattributed, so full-host browser preservation is not
claimed. Screenshots and raw process/account evidence remain private.

This closes the bounded input and handoff recovery, not ticket 530's remaining
account/profile-copy experiment. The original profile and browser stay preserved
for human Google verification. No fourth copy or original fourth operation was
prepared, and no account usability is inferred from the verification screen.
Ticket 529 still needs actual recording/narration inputs. P300 final broad review
and findings-remediation counters remain zero. Keep the canonical checkout and
existing candidate custody for that parent continuation; do not remove a source
owner's retained artifact cache as part of this closeout.
