# Plan0297: Independent task tab expiry

State: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#490
Related intake: CochranResearchGroup/agent-browser#488
Branch: fix/issue488-tab-ttl
Target: main
Base: 7d9b4b7ad41114f313b4329743904b9181b10b29

## Current State

Installed source `53fd459` durably reconciled the two prior eligible borrowed
tasks. Later whole-process namespace observation blocked ordinary owner status;
the bounded batch follow-up now passes its final source gates. The primary owns
the next sealed preserve installation and installed owner responsiveness plus
fresh-task expiry acceptance. The ticket remains open.

## Initial evidence

The operator-authorized ChatGPT task accepted a thirty-second per-tab lifetime,
had no pending-human hold and retained an unchanged activity timestamp, but
remained live throughout a ninety-four-second passive observation. AuraCall
later attempted closure under its independent five-minute deadline. That later
disappearance does not qualify Agent Browser automatic expiry.

Two bounded periodic fd2 observations captured `browser_session_idle_custody_unavailable`. The configured native pool adapter required a published Agent Browser launch assignment for a borrowed browser that has no Native desktop and is positively marked shared. Source provenance ties the installed binary to PR455's merge. The periodic
reaper, host and manager source match this plan's base. No runtime replacement
is authorized by this worker packet without coordination with the primary.

## Bounded outcome

An eligible expired task tab retires through the normal owner reaper, preserves
protected peers and shared browser process custody, and publishes truthful
terminal lifecycle state. An uncertain independent browser must remain deferred
without starving the eligible tab. CI is excluded.

Test the public session-manager reaper with controlled time and boundary effects;
then the existing native-host fixture for publication. Record each red and green
command. Do not add speculative cleanup authority or infer live ownership from
the absence of a Service State row. Installed acceptance must exclude competing
AuraCall cleanup and remains the primary's coordinated integration gate.

## Execution ledger

- Source qualification: validated at the recorded base.
- Regression: two red-green loops at the public manager seam. A failed older browser observation cannot starve another task; the composed native-pool adapter reproduces the exact borrowed-browser custody error and preserves a blocked service guard.
- Implementation: exact shared/no-Native-desktop tasks retain local service activity exclusion without claiming native launch authority. A mismatched browser descriptor or genuinely owned native scope still requires custody. Independent browser observation errors remain visible after successful peers publish.
- Model validation: all 396 service-model checks passed; format, strict workspace Clippy and documentation parity passed. Native pool gate: 97 passed, 0 failed, 1 explicitly ignored live-viewer fixture. Installed acceptance remains pending.
- Integration: pending.
- Installed acceptance: pending; preserve the production host and all peers.
- Review: the primary owns one broad Standards/Spec review after ticket validation
  and one bounded TDD remediation pass; this worker does not start another review.

The clean, fully integrated former P240 checkout is reused. Its old branch and
all candidate custody remain preserved; no additional worktree is created.

## Source validation receipts

The public manager regression for an uncertain older browser first failed with
`synthetic_owner_observation_unavailable` and an unretired peer. Per-browser
qualification now preserves that scope, records its typed error and permits
independent expiry. The real composed pool-effects regression then failed with
`browser_session_idle_custody_unavailable` for the borrowed task itself. The exact
shared/no-desktop branch fixes that case while retaining the local denial case.
Both red and green invocations use the existing service-model integration runner,
with the named `uncertain_browser_observation_does_not_starve_another_tabs_lifetime`
and `borrowed_tab_lifetime_does_not_require_native_browser_launch_custody` filters.
All 396 service-model checks and documentation parity checks passed. Workspace
Rust format and strict Clippy passed. Native pool-host gate subsequently passed 97 checks with one pre-existing live Native viewer fixture ignored; installed
acceptance remains pending. A stalled shared-cache
transport was bypassed through the supported `AGENT_BROWSER_CARGO_CACHE=off`
setting; no peer cache server or production process was restarted.


## Sealed candidate checkpoint

Source commit `83888301f55e0cf500979bc91da5599ad0b0f9b0` produced
candidate `candidate-6d2fd4e6acae47f7-bf0bcd3d87632f09`. The supported
build completion receipt records request
`eca7df843b33e0419305dc8c515bbb98def02ec06cac6bdde1a4bf7a959ffe83`
as complete at 2026-10-28T19:16:38.351Z. Binary SHA256:
`bf0bcd3d87632f09d6397685f93b067e903c8bf67f5e0d5a863e8848d023ca5e`.
The production-shaped artifacts remain pinned beneath the worker's
`cli/target/candidate-builds/eca7df843b33e0419305dc8c515bbb98/sealed/`.

The initial native harness aborted at the documented default test-thread stack
limit. One retry used the identical prebuilt test binary with a 16 MiB thread
stack and a 4 GiB enforced scope: 97 passed, zero failed, one existing Native
viewer fixture ignored, in 223.90 seconds. This slice does not claim live viewer
acceptance. The release build used four jobs, a 12 GiB admission claim and
enforced maximum, 10 GiB high watermark, and the unchanged 16 GiB host reserve.
The interrupted pre-reboot request was reconciled through supported recovery;
the later queued build was cancelled with a terminal failed receipt before the
bounded replacement request. No peer processes or caches were cleaned.

The primary installed this exact candidate through the supported preserve
transaction `upgrade-56d0e64b-20b1-49ab-b422-c2d3ccebc63a`. Live acceptance is
still open: task creation reported `browser_session_idle_publication_conflict`
despite durable task creation. Read-only investigation is ongoing; no successful
automatic TTL retirement or ticket closure is claimed at this checkpoint.


## Installed publication blocker

The first installed task-open returned `browser_session_idle_publication_conflict`
after physical task creation. Read-only owner/store comparison found three older
settled Native close operations with unpublished retirement projections. A failed
reap had retained proposed browser/session retirement in host memory. The next
operation reset its local retirement IDs and attempted to publish that old
projection with unrelated task work. SQLite correctly rejected the mutation
under the still-pending retirement fence.

The actual host with the actual SQLite store reproduced this boundary: inject
one retirement publication failure after the close settles, then verify the
next task sees durable state and can publish independently. Red failed on the
before-image assertion. The repair adopts current durable aggregate state after
a failed maintenance pass, retaining the exact physical-effect journal and the
original observable error. It neither clears a fence nor repeats a close.
A continuation verifies old journal completion and independent short-lifetime
expiry with one physical close per browser. The Native-window fixture explicitly
observes the second synthetic process, and a separate effect counter avoids its
older single-global-fence marker when two operations complete in one pass.
The intermediate window and counter fixture failures are retained separately
from the product regression receipt. Final gate results follow at qualification.


Publication regression commands (CI excluded):

```sh
AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=4 AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=4194304 AGENT_BROWSER_CARGO_MEMORY_RESERVE_KIB=16777216 AGENT_BROWSER_CARGO_MEMORY_HIGH=3G AGENT_BROWSER_CARGO_MEMORY_MAX=4G RUST_MIN_STACK=16777216 scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml failed_retirement_publication_does_not_poison_an_unrelated_task -- --test-threads=1
AGENT_BROWSER_CARGO_CACHE=off AGENT_BROWSER_CARGO_BUILD_JOBS=4 AGENT_BROWSER_CARGO_MEMORY_CLAIM_KIB=4194304 AGENT_BROWSER_CARGO_MEMORY_RESERVE_KIB=16777216 AGENT_BROWSER_CARGO_MEMORY_HIGH=3G AGENT_BROWSER_CARGO_MEMORY_MAX=4G RUST_MIN_STACK=16777216 scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --bin agent-browser failed_retirement_publication_does_not_poison_an_unrelated_task -- --test-threads=1
```

The red command failed the durable before-image assertion. After repairing the
maintenance transaction boundary and qualifying both fixture observations, the
green command passed the complete continuation in 3.86 seconds. The exact
prebuilt native test binary SHA256 is
`aba9f2895c93e9617425a8b9493587830451391975e66334c1f38df7292a7da1`.
Workspace format and strict Clippy passed again after this executable change.
The unchanged model-only repair retains its earlier 396-check qualification;
this follow-up changes only the native host failure boundary and native fixture.


Follow-up qualification: the actual native host gate passed 11 checks in 14.29
seconds. The native consumer gate, including all pool-host cases and the new
transaction regression, passed 123 checks with zero failures and one existing
live-viewer fixture ignored in 296.68 seconds. Both ran the exact compiled binary
above with 16 MiB test-thread stack and an enforced 4 GiB maximum. Consumer scope
peak was 169,959,424 bytes. No live Native viewer behavior is claimed.

```sh
env RUST_MIN_STACK=16777216 cli/target/debug/deps/agent_browser-3104eecfbe9c33ba native::browser_session_host::tests --test-threads=1
env RUST_MIN_STACK=16777216 cli/target/debug/deps/agent_browser-3104eecfbe9c33ba native::browser_session_consumer_tests --test-threads=1
```

The commands ran in separate user systemd scopes with MemoryHigh=3G,
MemoryMax=4G and MemorySwapMax=1G. The primary owns the replacement candidate
installation and final live acceptance. Draft implementation PR: #491.


## Installed multi-session acceptance repair

The supported follow-up candidate from `c3a83438b` was installed successfully.
The passive live check still failed truthful lifecycle publication: the eligible
target disappeared, all four protected peers and its browser generation survived
150 seconds, and AuraCall recorded no competing closure, but Agent Browser
retained an active task with no terminal history. Physical disappearance was
first observed 52.64 seconds after unchanged activity. This does not qualify
Agent Browser TTL acceptance.

A bounded fd2-only observation then captured the exact missing-target error. The
first-pass error was not captured. Equivalent public manager/real pool-effects
state reproduces a deterministic first-pass failure: after one same-browser
task retires, its changed logical `active_session_ids` made the borrowed-browser
helper reject the next expired task as if physical browser identity changed.
The regression covers an expired heartbeat peer and a second expired TTL peer,
including local-authority denial. Red returned
`browser_session_idle_custody_unavailable`; green passes with both task outcomes
and without whole-browser close authority. Frozen PID, profile, endpoint, browser
ID and Native desktop still must match; only logical task membership can change.

The already-absent owned target also reproduced through the public
`BrowserManagerRuntime::close_tab` seam with real CDP transport and current
process binding. Red returned the installed missing-attached-target error. The
scoped runtime path now reconciles only a fresh complete target census plus
matching CDP browser PID and unchanged kernel identity. Malformed inventory and
a changed PID fail closed; ordinary generic release stays strict. A second red
showed the absent cached target reused as a new task's bootstrap. The existing
cache-removal boundary now runs only after positive absence, and the same public
runtime continuation acquires a new owned target without touching its physical
peer. Inventory and PID reads each have a two-second bound.

Receipts: `multi-session-red.log` / `multi-session-green.log`,
`absent-target-red.log` / `absent-target-green.log`, and
`absent-cache-red.log` / `absent-cache-green.log` in the worker's private
issue490 evidence directory. They contain synthetic inputs, not copied private
browser payloads. Final combined runtime regression passed in 0.09 seconds.
The private raw live failure and fd2 trace remain outside Git.

This is required live-acceptance remediation. No reserved post-closure broad
review has run; the original full-descriptor equality review hypothesis is now
reproduced and repaired rather than an untested future finding.


Final source gate for the multi-session/absence repair: all 396 service-model
checks passed. Native runtime: 29 passed, eight existing local-Chrome cases
ignored, 49.99 seconds. Native consumer/pool: 123 passed, one existing live
viewer fixture ignored, 295.12 seconds. Native host: 11 passed. Strict generic
close preservation: one table-driven check passed in 2.14 seconds. Workspace
format, strict workspace Clippy and diff checks passed. Native gates used the
compiled binary with SHA256
`9649c7e72c6b4c351e5f00ee849340be085e971e6631934433254e461335ee39`,
a 16 MiB test-thread stack and 4 GiB enforced scopes. No live Chrome/viewer
behavior is inferred from ignored cases. CI remains excluded.

Focused new regression commands use the same reviewed four-job, 4 GiB maximum
Cargo profile shown above and the unchanged 16 GiB host reserve:

```sh
scripts/ci/cargo-safe.sh test -p agent-browser-service-model --test browser_handoff_recovery borrowed_tab_lifetime_does_not_require_native_browser_launch_custody -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path cli/Cargo.toml --bin agent-browser owned_tab_retirement_reconciles_only_verified_fresh_absence -- --test-threads=1
```

The model invocation first failed on unavailable Native custody, then passed
both expired-peer variants and authority denials. The runtime invocation first
failed missing-target reconciliation, then passed it; the cached continuation
then failed returning the absent original target, and finally passed new-target
acquisition plus malformed-inventory and post-attachment PID-mismatch denials.
The primary owns the next supported preserve install and installed acceptance.


## Native readback and late-maintenance isolation

The `399ded40b` candidate built as binary `b969060d610f` and was installed through
accepted preserve transaction `upgrade-ddb72273-4a11-4f7a-a490-390463b2e398`.
Its passive 95-second prior-task reconciliation failed: both owned targets were
physically absent but still active without terminal history. Owner memory equaled
SQLite, proving phase rollback rather than another unpublished memory leak.
Bounded fd2-only observation captured `browser_session_idle_readback_required`.

A real public host plus SQLite regression reproduced that exact error using an
older Native prepared/settled-unpublished scope and a separate eligible borrowed
task. Guard failures now defer their browser and remain observable; failed or
denied qualification restores only transient publication eligibility, preserving
the original durable journal/fence. The red failed in 3.21 seconds; green passed
in 3.48 seconds with independent terminal publication, protected peer, exact old
Native projection and unchanged unpublished journal.

A second real host/SQLite test table reproduced the later-error rollback class.
It covers Native close failure after a settled prefix, a later profile deletion
failure, and a later borrowed close failure. The first red lost independent
terminal history. All three green rows pass with exact Native state restoration,
unpublished effect journals, failed-task/peer preservation and no second-phase
retry of the unknown borrowed close.

The host now publishes borrowed/shared/no-Native task expiry before Native viewer
observation, preparation recovery and disposable maintenance. The second phase
keeps the existing atomic Native rollback, whose durable baseline already holds
the qualified borrowed outcome. This changes no quota selection or cleanup
authority and uses no per-tab/global-state undo snapshots. Borrowed close methods
write logical changes only after their effects qualify; later failed borrowed
tasks defer while earlier qualified prefixes remain publishable. Deferred scopes
are protected in the second phase. Cleanup-expiry arithmetic is validated before
physical effects.

The new phase is explicitly forwarded by both `RuntimeSessionEffects` arms,
`BrowserSessionEffectAdapter`, and `BrowserRuntimeDriver`; concrete Remote View
effects propagate the borrowed mode and defer Native preparation recovery. The
existing dispatcher fixture cannot distinguish phase ordering without a new
Native preparation fixture, so the primary selected exact source routing, full
native gates and installed acceptance rather than a forwarding-only proxy test.
`native-readback-red.log`, `native-readback-green.log`,
`late-maintenance-red.log`, and `late-maintenance-green.log` retain the synthetic
real-store receipts in the private worker evidence directory. The raw failed live
verdicts and traces remain outside Git. Final gates and the next installed verdict
follow at the new source checkpoint. No reserved post-closure broad review ran.


### Final deferred-task and production-wired qualification

The cheap public model seam also qualifies the precomputed expiry list: after a
borrowed task's first close fails, both of that task's tabs and its session must
remain exact. Red showed a later tab still removed; dynamic protection is now
rechecked when consuming tab/session lists. A distinct later expired task on the
same borrowed browser must still retire after a fresh guard. Red showed
browser-wide target-error protection starving it; target errors now protect only
the failed task, while guard errors retain whole-browser protection. Green keeps
the failed two-tab task exact, retires both independent tasks, and sends no
failed-task suffix attempt. The second phase protects all remaining sessions
under the browser-scoped error key, avoiding same-pass retries. Receipts:
`deferred-task-{red,green}.log` and
`deferred-independent-task-{red,green}.log`.

Final frozen-batch gates: all 397 model checks pass; native consumer/pool 125 pass,
one existing live viewer fixture ignored, in 313.27 seconds; host 11 pass; runtime
29 pass and eight existing disposable-Chrome cases ignored; production dispatcher
three pass; strict generic-close preservation passes. Workspace format, strict
Clippy and diff checks pass. Exact native test binary SHA256:
`8e48406dceb80e2b06730dbc47e7011b9dd733db6cca01f04c80cac74f639356`.
Native fixtures use 16 MiB test-thread stack with 4 GiB enforced scopes; the
reviewed four-job resource profile keeps the 16 GiB host reserve. No ignored
Chrome/viewer case or source-only success is promoted to installed acceptance.
The primary owns the next sealed preserve installation and real shared-browser
proof. CI remains excluded; the reserved post-closure review has not run.


### Installed borrowed publication and cleanup census latency

The sealed source `53fd459` installed through the primary's supported preserve
transaction and durably retired both prior eligible borrowed tasks. Ordinary
owner status then exceeded both 10-second and 30-second bounds while later
maintenance held the shared host lock. Read-only diagnostics showed repeated
namespace-observer launches for unrelated processes. Disposable cleanup's full
process census called `capture_process_identity` for every PID, including its
up-to-ten retry loop, before profile relevance was evaluated. The inherited
per-PID observer had a bounded individual request but no aggregate census bound.
The source and first installed candidate remain preserved; this partial installed
success does not close the ticket or satisfy fresh-task acceptance.

Cleanup now gathers the same complete local process/argument census and requests
fresh executable/start evidence in one bounded identity-only batch when direct
namespace observation is unavailable. The helper accepts at most 8192 unique
positive PIDs, emits at most 4 MiB, cannot recursively delegate, and retains the
existing local-user namespace and runtime limits. The caller drains stdout while
the helper runs, limits the exchange to four seconds, and reaps its owned runner
on errors. Response sets must match exactly; duplicate, missing, extra, changed
PID, boot or kernel-start evidence cannot grant cleanup eligibility. Command lines
stay local and are reread after the batch: changed or unreadable arguments refuse
cleanup because kernel start identity alone survives execve. Unknown processes
retain their blocker metadata and cannot establish a selected browser tree.
Every effect guard still gathers fresh evidence; no census is cached across
allocation or effect boundaries.

The original installed status timeouts and helper stream are the latency red
receipt. External observer protocol tests qualify exact response identity and
bounded owned-child transport, including output beyond pipe capacity, overflow
and timeout. Existing public cleanup eligibility and foreign-process protections,
native consumer gates, strict workspace lint/format, and a new installed owner
responsiveness/fresh TTL proof are required before closure. Source gate results
and the next sealed identity will follow; no runtime change or reserved broad
review is performed by the worker.


The final census source gates pass after repairing one non-hermetic pre-existing
public guard fixture. That fixture had implicitly read host namespaces despite
the unit lane prohibiting namespace services. It now supplies complete synthetic
process-table evidence, retains job/claim and foreign-process cases, and explicitly
proves that a census timeout refuses admission, preserves the profile and releases
exclusion. The failed intermediate runtime receipt is retained. Production
transport failure remains a deferred error, never a complete empty census.

Final selected native gates: consumer/pool 125 passed, one existing viewer case
ignored, in 283.06 seconds; runtime 29 passed and eight existing Chrome cases
ignored; host 11, observer/protocol five, public cleanup guard four, resources 32,
process identity 22, dispatcher three and strict generic close one passed. The
exact native test binary SHA256 is
`71e66d9a4b480892622b423f1de8c8ef831c86011d2116ce71f141cbc81a20e7`.
The 4 GiB aggregate test scope peaked at 1,058,197,504 bytes with 16 MiB thread
stack; strict workspace Clippy, format and diff checks pass. Prior 397 model
checks remain applicable because this follow-up changes no model source. These
are source and fixture results, not installed responsiveness or fresh TTL proof.
CI remains excluded; no reserved broad review has run.
