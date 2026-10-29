# Plan0297: Independent task tab expiry

State: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#490
Related intake: CochranResearchGroup/agent-browser#488
Branch: fix/issue488-tab-ttl
Target: main
Base: 7d9b4b7ad41114f313b4329743904b9181b10b29

## Current State

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
