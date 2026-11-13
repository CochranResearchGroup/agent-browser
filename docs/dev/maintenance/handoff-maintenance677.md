# Handoff maintenance latency

Product lane: PL-BUGFIX. Work item: [677](https://github.com/CochranResearchGroup/agent-browser/issues/677).
Owner: ecochran76. Branch: `fix/handoff-maintenance-latency`.

## Outcome

Make an existing authenticated handoff responsive while background maintenance
retains every lifecycle, identity, publication and peer-work fence. Source repair
and local validation are separate from production installation and original-link
acceptance. Remote View remains the viewer and authentication owner.

## Evidence and feedback loop

Bounded private runtime evidence showed continuation requests above 60 seconds,
a 98.061-second direct owner callback and a 31.319-second read-only inspection.
A correlated inspection returned 0.755 seconds after an 89.212-second maintenance
pass completed. The shared owner mutex encloses that maintenance pass.
Private invitations, account state, runtime payloads and traces stay outside Git.

The synthetic loop uses the actual retirement adapter and SQLite store with
known and legacy peer groups. Before the repair it failed twice at the intended
assertion: five aggregate reads instead of at most two, in 7.33 and 8.53 seconds.
Compilation and helper setup failures are separate from those RED results.
The helper retains legacy-identity deferral, no native preparation and unchanged
session-state assertions. The repair must then pass this loop and existing
custody tests without a provider or browser.

## Bounds

Investigate repeated state/custody reads and native or filesystem work under
maintenance. Do not weaken validation, cache mutable authority across operations,
drop history, change retention, or move effects to a second owner. Record the
responsible substep before selecting a repair. 676 owns managed launch defaults
in the peer checkout; 673 owns Chrome onboarding. Avoid their implementation
surfaces and reconcile shared catalog prose during integration.

## Validation and completion

Run focused regression and affected custody/model checks, Rust formatting and
strict workspace Clippy. Use local validation only. Preserve the original public
latency evidence and do not claim first-pixel or deployed acceptance from source
tests. Re-run the original public link only against a separately qualified
candidate; unchanged production is expected to retain the defect.

## Source repair and local evidence

The SQLite adapter now reads selected assignments in one transaction and joins
peer assignments with immutable process identities in a second transaction.
Each transaction loads and validates the current owner aggregate and launch
ledger once. Transactions end before provider reads or effects. Legacy identity
and release fences remain required; final admission and publication are unchanged.

Baseline source: `296c0f14c6ff72ff5a1045d2b346ad73e8b8d303`.
Focused `scripts/ci/rust-tests.sh --focused retirement_preflight677` passed both
regressions in 10.92 seconds after compilation. The affected pool selection
completed with 140 passed, two failed and one ignored in 952.32 seconds. The two
failures are `retained_interactive_handoff_recovers_with_original_invitation` and
`retained_interactive_recovery_rejects_unqualified_scope_and_control`.
Restoring both changed implementation files to the baseline reproduced those
same failures in 10.03 seconds with `--focused retained_interactive`. The new
batch APIs remain unused on that baseline call path. This comparison separates
the pre-existing recovery defect, tracked under [599](https://github.com/CochranResearchGroup/agent-browser/issues/599),
from the passing latency regressions; it does not make the pool selection green.

Policy wiring (11 tests), changed documentation links and patch hygiene pass.
All 235 service-model tests, strict workspace Clippy and final formatting pass. Validation runs
locally on WSL with disposable fixture homes, Rust 1.94.1, four build jobs,
private target storage, debug symbols disabled and eight codegen units. No
hosted CI, provider/browser acceptance, production install or first-pixel
qualification is included. Private raw logs and comparison artifacts remain
outside the repository.


Commands used for the final source checks:

```bash
scripts/ci/rust-tests.sh --focused retirement_preflight677
scripts/ci/rust-tests.sh --focused pool_tests
scripts/ci/cargo-safe.sh test -p agent-browser-service-model --lib -- --test-threads=1
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
pnpm test:policy-wiring
pnpm test:documentation-links -- --base origin/main
git diff --check
```

The model/lint lane used a 4 GiB memory cap; CLI test compilation used a 7 GiB
cap. A waiting larger reservation was cancelled before compilation. Prior
resource, helper-compilation and intentionally RED receipts are retained
separately. The final source passed focused tests and lint without a retry for a
product failure. The broader pool result remains failed, with baseline comparison
as described above. Keep this repair in draft until that integration risk is
resolved; source qualification does not satisfy original-link acceptance.

A final fetch observed `origin/main` at
`ea9bcb186b41209b2887a871aaf38d7ef6f3115f`, including peer work 673 and 676.
This draft retains the tested baseline above; integration with the new tip and
its overlapping domain/catalog prose remains a separate gate before merge.


## Authorized review, merge and deployment

The operator requested review, merge and deploy on 2026-11-13. Integrated main
at df24da179. Serial Standards review found no blocking convention or ownership
issue. Spec review retained operation-local snapshot authority and unchanged
admission/publication fences; live timing remains an acceptance gate.

The two baseline recovery failures reproduced after integration. The recovery
proof compared the retired and replacement desktops for equality even though
current recovery claims a fresh desktop. Align only that field for the logical
browser comparison, then require both immutable launch records to confirm their
own exact assignments, the completed retirement and original absence. Existing
positive and ten adverse recovery cases changed from RED to GREEN; add a foreign
assignment case to make that publication guard explicit. This bounded correction
resolves the validation dependency with issue599 without changing recovery owners.
The initial production build was stopped at its exact task-owned Cargo scope
before source correction; its failed receipt and partial cache remain retained.


## Merged and installed | 2026-11-13

PR682 merged as `9d9a6aca5846f821ec60ee18e779b7c343a51e64`.
The sealed source `24bb970c84e5d1d98fc9e7c0f50d88e1c1d74538` entered main's
ancestry with an identical complete source tree. Integration includes receiving
binding681. Its combined CLI Clippy and formatting pass; the selected pool suite
was rerun from an immutable test binary with the runner's disposable HOME/XDG
layout and passes 144 tests, with one existing ignored test. The earlier integrated
pool run also passed. All 424 service-model tests across 26 suites remain valid:
receiving binding681 changes no model code or dependencies. Policy wiring and
changed-link checks pass. No hosted CI ran.

### Standards

Serial primary review found no outstanding accepted standards finding. Model
contracts, SQLite custody and CLI orchestration remain separate; fresh snapshots
end before effects and do not replace final admission/publication.

### Spec

The repeated-read regression passes at the real adapter/store seam after RED.
The accepted recovery blocker is repaired: two exact immutable assignments,
completed retirement and original absence remain mandatory while a qualified
replacement may use its published desktop. Positive recovery and eleven adverse
cases pass, including a foreign replacement assignment. No outstanding source
finding remains; live latency limitations below remain explicit.

The production release build passed with zero Rust warnings/errors. Binary
SHA256 is `934f03443c98ccc92db21ea9f43c6a42148c81c3a7fe826eb472b3ee1bc335c5`;
artifact seal SHA256 is
`53ec036c810b420c240b9d8148ee74d58e4ea5f5c9e5e92bca7e23ff7d5300f3`.
Exact-binary source-free installer fixtures and sealed install dry-run pass.
Fresh readback found no active transaction before the preserving install.
Installation returns success, complete and ready; candidate status is accepted
with no active custody. Installed CLI and the current maintenance producer both
hash to the sealed binary. Final doctor succeeds.

A fresh installed MCP client opened one disposable synthetic page. Native handoff
reported an owned browser window ready; the page snapshot contained its expected
heading. Five independently authenticated public redirect chains reached the
viewer document in 6.284, 3.720, 3.194, 5.373 and 3.389 seconds. The original link
currently reports a closed task with its link still valid; it was not reopened.
Checks used public HTTPS from Cooper, not an independent off-host vantage. Viewer
first-pixel and private input latency were not measured. The retired old MCP
connection closed as expected; fresh installed MCP passed.

Current-load maintenance samples before installation were 3.939, 3.123 and
3.228 seconds. Post-install samples ranged from 2.720 to 8.957 seconds, including
startup and browser-check activity. These do not establish a controlled wall-time
improvement. The historical 89-second incident is separate evidence. The source
read amplification is removed and this deployment's public probes passed, but
several-second latency remains; keep677 open for further performance qualification.

Task-scoped close reports browser_closed. Fresh OS readback proves its original
PID absent and zero processes referencing the exact disposable profile. Profile
files retain their configured policy; no peer browser, profile or rollback
payload was removed. Raw protocols, identities, invitations and receipts stay in
private user state. Retain this worktree and its sealed candidate as deployment
provenance; no worktree or candidate cleanup is implied.


## Residual latency TDD continuation

Product lane: PL-BUGFIX. Branch: fix/handoff-latency677-tdd. Reuse the existing
primary checkout and preserve its sealed installed candidate. The operator
invoked TDD for 677 after reconnecting MCP. This slice owns addressed handoff and
operation setup; preserve peer launch, mode and receiving-binding surfaces.

Seams under test:

- Browser Runtime `service_remote_view_handoff_resolve`: prove an exact live
  target remains ready while another profile's Native retirement provider is
  blocked. This catches foreground dependence on unrelated provider work; it
  misses external ingress and first-pixel delivery.
- Existing profile retirement admission: retain own-profile unresolved-operation
  refusal after the first GREEN result. This catches accidentally weakening the
  target's lifecycle fence; add only after the first slice completes.

Use the existing real SQLite/host/provider harness. Block only the external
Native provider boundary with explicit synchronization. Assert the addressed
handoff response and target within a deadline while the foreign provider remains
held. Do not assert internal read counts, query the database as a result oracle,
cache mutable ownership across requests or remove uncertain journals.
Source work preserves production; exact installed acceptance remains separate.


The first command-seam test is RED on baseline 2e9106fcb: the exact failure is
"live handoff waited on another profile's blocked maintenance provider: Timeout".
The focused suite ran in 8.40 seconds after compilation. The minimal implementation
scopes operation setup to the addressed profile while retaining all records and
full snapshot authority. The same test is GREEN in 7.09 seconds after compilation;
its handoff result arrived while the foreign provider was still blocked.
Recheck with per-fixture control HOME isolation, and exercise the existing
pending-effect guard through addressed initialization. Global maintenance is
unchanged. This command-seam proof does not bound waiting behind an already
running global maintenance pass or prove installed end-to-end timing.

Controller: primary 677. Bound this continuation to one addressed-operation slice,
one review pass with at most one correction, one qualified candidate build and
one preserving installed acceptance cycle within two hours of this continuation.
Reassess at two no-progress checkpoints or 30 minutes without outcome progress.
The existing review/merge/deploy authorization covers continuation of 677 inside
its original target and preserving effect class; source readiness alone grants
no additional effect. Retain the prior installed candidate and every peer task.


Source qualification for the continuation: pool 145 pass with one existing ignored
test; all 424 service-model tests, the addressed own-profile pending-effect guard,
strict workspace Clippy, formatting, policy wiring, changed links and docs build
pass. Test homes, runtime directories and provider replies are synthetic. WSL
uses rustc 1.94.1, four Cargo jobs, debug symbols disabled, eight test codegen units
and a private target directory. No hosted or cross-platform qualification ran.

Exact RED and GREEN test command (same command, before and after the repair):

```bash
scripts/ci/rust-tests.sh --focused handoff677_live_target
```

Affected checks:

```bash
scripts/ci/rust-tests.sh --focused automatic_retirement_pending_effect_denies_conflicting_browser_commands
scripts/ci/rust-tests.sh --focused pool_tests
scripts/ci/cargo-safe.sh test -p agent-browser-service-model -- --test-threads=1
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
```

Serial Standards review against 2e9106fcb found no blocking ownership or convention
finding. Serial Spec review found the blocked foreign-provider case repaired
without relaxing own-profile or shared-group custody. No refactoring is required
for this slice. Preserve the warm installed handoff across the one authorized
preserving upgrade for a same-task, same-URL before/after comparison. This remains
separate from the command-seam test and does not claim independent off-host pixels.


## Addressed-operation deployment and retained-browser comparison

PR691 merged as 3a7ab465f87bd7bda3a580e50ced27fb90b04b81. The single qualified
production-shaped build sealed binary
88972f41067b0a9b9fbce88c2698414bbc0b92a79e91741bb080376b70eec9f1 from that clean
canonical source. Release optimization is level 3, fat LTO and one codegen unit;
rustc 1.94.1 targets x86_64-unknown-linux-gnu. The maintained builder completed
successfully with no compiler warnings or errors. The exact sealed binary passed
`AGENT_BROWSER_FIXTURE_BIN=<sealed-binary> pnpm test:workstation-install-fixture`.
No hosted CI ran. Local Linux validation does not establish other platform results.

A fresh fetch matched the candidate source exactly, installation dry-run reported
an eligible sealed production artifact with no integrity preconditions, and fresh
status showed no active transaction. The existing authorization selected one
preserving installation, without force or full shutdown. Transaction
upgrade-9005d7bf-102d-46fa-96b6-5012ef5a8130 reached accepted in 95.1 seconds.
Installed command and a freshly observed maintenance producer both hash to the
sealed binary. Final candidate status is accepted with no active operation;
`agent-browser install doctor` exits zero. Doctor retains advisory observations
about the different workspace candidate and missing authenticated dashboard
operator-journey receipt; this is not dashboard journey qualification.

Before installation, a disposable synthetic page obtained an owned-window ready
handoff. After installation, fresh MCP automation read the same page marker.
The exact original browser PID and process start token, profile path, browser ID,
and opaque handoff URL were unchanged. Neither the original reported user task
nor another browser was reopened or closed for this check.

Five authenticated public HTTPS probes from Cooper followed the same durable
handoff URL to the native viewer document before and after installation:

| Sample | Before seconds | After seconds |
| --- | ---: | ---: |
| 1 | 6.766 | 13.607 |
| 2 | 3.519 | 4.327 |
| 3 | 7.915 | 4.769 |
| 4 | 3.757 | 3.483 |
| 5 | 7.231 | 3.958 |
| Median | 6.766 | 4.327 |

Every sample passed the 60-second document budget. The first post-install sample
was slower: its initial handoff redirect completed at 0.146 seconds, the native
continuation redirect at 13.394 seconds, and viewer document at 13.607 seconds.
Retain that sample. These observations are descriptive, not a controlled latency
improvement claim. They exclude authentication setup, first pixels and interactive
input, and do not provide an independent off-host network vantage. The source
regression proves foreground operation setup avoids another profile's blocked
provider; waiting behind an already-running global maintenance pass remains
unbounded by this repair. Issue677 remains open for that residual path.

Task-scoped close returned browser_closed. Fresh OS readback proves the original
PID absent and zero processes referencing the exact synthetic profile. Profile
files and peer state retain existing policy. Private protocols and identities
remain in user-scoped evidence. Retain this primary checkout and both sealed
candidates as deployment provenance. The bounded TDD delivery is complete; no
further production effect is selected by this packet.


## Remaining shared-owner wait diagnosis

Product lane: PL-BUGFIX. Primary branch: fix/handoff677-maintenance-wait.
The operator selected diagnosis of the remaining global-maintenance wait after
the bounded addressed-operation deployment. Reuse the retained primary checkout
and both sealed artifacts. Reconciled inventory has 14 registered worktrees;
stock-popup launch changes on current main remain outside this slice's writer
scope. Use the actual queued handoff consumer and actual maintenance adapter,
blocking only an external provider response. First require an observed deadline
failure while maintenance is held; then minimise and test ranked hypotheses.
Retain same-profile lifecycle and publication fences. Bound this continuation
to two hours, one correction, one qualified candidate and one preserving install;
reassess after two no-progress checkpoints or 30 minutes without acceptance
progress. Existing review/merge/deploy authority applies to the same target and
preserving effect class; no foreign closure, broad cleanup or ingress change.


The real queued-consumer/maintenance regression is RED twice on baseline
3b93da921: "live handoff waited behind active background maintenance". The
one-command loop is `scripts/ci/rust-tests.sh --focused handoff677_queued_target`;
the two test runs take 5.98 and 6.35 seconds excluding compilation. The existing
queued handoff without held maintenance passes. A missing fixture directory was
a harness failure, corrected before these RED results. Minimal case: one retained
browser, one handoff, one maintenance provider response held after entry.

Ranked predictions: (1) a shared owner mutex causes the wait, so a separate
retained maintenance adapter should remove it; (2) a SQLite writer remains held,
so independent adapters would still wait; (3) provider/runtime serialization
remains, so independent host and database access would still wait. Test the first
prediction with persistent request and maintenance adapters over the same SQLite
owner and existing custody/publication protocol. This does not add another viewer
or allocation authority, discard adapters between passes, or relax target fences.


The independent-adapter regression is GREEN in 4.80 seconds. Restoring only the
maintenance entrypoint's old request-mutex choice makes the final concurrent
fixture RED again (5.99 seconds), confirming the first prediction. The final
fixture additionally renews activity with current time and checks that the
retained target, browser and activity survive maintenance publication. Restore
the independent maintenance mutex for final qualification. No production effect
has occurred in this diagnosis checkpoint.


The full affected pool run passes 146 cases with one existing ignored case;
both677 regressions, own-profile pending-retirement guard, ordinary queued
handoff, both startup reaper cases, strict workspace Clippy and formatting pass.
Policy wiring (11 cases) and docs build pass locally. The selector recommends
focused Rust plus policy/link gates for these seven changed paths, excluding
unrelated Dashboard, Service Client, Repository Tooling and Workstation suites.
The service-model sources are unchanged; no new model-package execution is claimed.

Serial Standards review against the fetched baseline finds no hard ownership or
style violation. Its one accepted test correction preserves the original ordinary
queued fixture's explicit rejection of unexpected provider launch/acquisition.
The maintenance variant alone tolerates a deferred provider result. Serial Spec
review finds the concurrency requirement implemented and target renewal retained;
no additional workflow or authority is introduced. Verify the corrected ordinary
case, both677 regressions, daemon routing and tab-close contracts, then reuse the
full pool evidence for unchanged cases under policy0042. Runtime acceptance is
still pending; no public latency improvement claim is made by source tests.


### Standards

Final correction verification passes the ordinary queued case, both677 cases,
13 daemon tests and three regular tab-close contracts (physical opt-in remains
ignored), plus strict workspace Clippy and formatting. One accepted test
finding is resolved; zero remaining hard standards findings.

### Spec

Zero blocking findings. Actual maintenance waits at the external boundary while
the public queued handoff returns ready within one second, retaining the same
target and renewed activity after maintenance returns. Own-profile unresolved
retirement remains fenced. Source qualification does not prove public pixels.

Final focused commands use `scripts/ci/rust-tests.sh --focused` with filters
`handoff677`, `automatic_retirement_pending_effect_denies_conflicting_browser_commands`,
`queued_action_executor_resolves_managed_handoff_through_the_registered_host`,
`startup_reaper`, `native::daemon::tests`, `operator_tab_close_` and `pool_tests`.
All Rust commands use cargo-safe admission, private test HOME/runtime and target,
rustc 1.94.1, four build jobs, eight test codegen units and no debug symbols.
Linux local results do not establish hosted CI or other platform execution.
