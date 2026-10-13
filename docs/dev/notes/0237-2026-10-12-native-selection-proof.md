# Native physical selection tracer

Product lane: PL-PLATFORM
Disposition: active-input
Owning plan or work item: [P237](../plans/0237-2026-10-11-native-dashboard-handoffs.md), [261](https://github.com/CochranResearchGroup/agent-browser/issues/261)
Related lanes: none
State: lower physical selection qualified; provider and workspace acceptance pending

## Requirement and oracle

Selecting an exact browser target must activate its native owning window, raise and maximize it, and preserve peers on the correct desktop. The opt-in `scripts/test-p237-native-selection.py` runs `physical_selection_activates_exact_target_window_and_maximizes` from an immutable test executable. It creates a private user/mount/network namespace, mounts its own X11 socket directory only there, starts fresh Xvfb/Openbox and launches the reviewed headed Chrome with a new private profile. Cleanup admits only descendants in exact OS sessions created by the driver. No shared socket directory, installed provider, default profile or private authentication is changed.

The test uses actual BrowserManager target switching and focus/maximize. Independent native title/PID, X11 input focus, active window, topmost window-manager stacking and maximized-state checks are the oracle. Browser command acknowledgement is insufficient. This lower physical seam does not qualify the replacement workspace UI, service inventory selection or native provider routing.

## Retained setup failures

The first driver setup cannot allocate an X11 listener in the shared socket directory. A readback shows root ownership and no write access for this user. Hypotheses were directory permissions, occupied display addresses and the readiness pipe. The isolated namespace supplies a writable private socket directory and private loopback, preserving the shared environment. The known-invalid old test pin then reaches a running desktop but selects zero tests; the driver correctly rejects this false green. Logs are `/tmp/p237-physical-selection-oracle.log` and `/tmp/p237-physical-selection-private-oracle.log`, with sibling artifact directories. Both cleanup receipts are empty.

The first actual physical candidate fails in 28.88 seconds while waiting for `Page.loadEventFired` during synthetic title/navigation preparation. It has not reached the native selection oracle. Preserve `/tmp/p237-physical-selection-first.log` and its artifact directory; cleanup reports no remaining task-session processes. Candidate `/tmp/p237-physical-selection-agent-browser-tests` has SHA256 `dfe9c99002b1a2799018742fab5ab300fc9bc44cb90261f4e5d0feb36ce9b2e9`, built against source baseline `4a25c8bc` plus the new fixture, without product behavior changes.

Before the next probe, ranked causes are a missed load event, unavailable renderer/document, and desktop setup failure. The next fixture sets titles through Runtime.evaluate on existing blank documents, removing an unnecessary navigation wait while retaining exact target/window selection. It also checks real input focus and stacking, rather than trusting a product-authored active-window property. Its build and physical result are pending. No focus defect or repair is established by the setup failures.

## Remaining acceptance

Run the revised physical oracle, preserve the original failures, and classify its actual result before product changes. Exact desktop selection across provider contexts, service generation fencing, rapid/stale selection, reviewed UI, foreign/provider and post-effect proof, old default UI retirement and canonical integration remain open. Do not call P237 complete from a passing lower-level physical test.


## Closed CDP channel probe

The blank-document variant fails in 4.17 seconds with `CDP response channel closed`, before any physical-selection success record. Its task-session cleanup list is empty. Preserve `/tmp/p237-physical-selection-document.log` and its artifact directory; pin SHA256 is `219d1ba56d69ac8832e06834ddedb61c90755527eac243489ae520779832c839`. Removing the load wait exposes this earlier transport/setup failure; it does not prove the missed-event hypothesis or a focus defect.

Before further changes, ranked causes are browser exit, connection closure with a live browser, and second-window setup. The next diagnostic candidate labels command failures, records browser process state before explicit fixture close and copies only the task profile's Chrome stderr into temporary evidence. It does not change product behavior. Its first result is pending; diagnostic evidence must be classified before a repair.


## Executable validity boundary

The contextual candidate cannot run even `--list`: both original and immutable copy segfault before the test harness. They match SHA256 `2ad0d101a8fac2063749675d234e5392ba69758ca3b4c69441bd46896cae7ea7`. GDB reports corrupt ELF string-table metadata and a dynamic-loader fault before Rust; readelf cannot find the dynamic section described by the executable's dynamic segment. This is invalid executable evidence, not a native selection or Chrome result. Its latest driver cleans all task sessions and rejects the absent test output. Preserve the first failures rather than treating rebuilding as erasure.

Earlier temporary logs, pins and the checkpoint directory are no longer present after the environment changed. Source commits and dirty fixture files persist. Historical source notes retain their recorded results; they do not prove those old temporary artifacts are currently available. The resource baseline is retained without another reset. A fresh owned test rebuild is pending; require a healthy exact `--list` before launching another desktop. No broad target-cache cleanup, installed runtime change or product repair is inferred.

## Current UID probe and physical result

The fresh eight GiB bounded rebuild completed successfully. Immutable executable `/tmp/p237-physical-selection-fresh-agent-browser-tests`, SHA256 `c40676cecd34f482d5790ec79696330a40e941d7cd8158fce065441f5f075cb7`, passes its exact test-list guard. Its mapped-root namespace run fails at second-window creation with `CDP response channel closed`. Chrome stderr contains a fatal permission-denied check in `base/process/process_linux.cc:201`; current local source identifies the checked operation as `setpriority` in `Process::SetPriority`. This is a fixture crash before selection, not a proven focus defect.

Before the next probe, ranked hypotheses were mapped-root priority assumptions, priority restrictions independent of UID, and renderer transition behavior independent of mapping. The driver changes only namespace mapping to preserve the current UID, retaining setup capabilities and distinct user/mount/network namespaces. The same immutable test executable then passes Alice, Bob, Alice exact native selection in 2.69 seconds. Independent X11 title/PID, active window, input focus, topmost stacking and maximized-state checks pass; peer targets survive. This supports the mapping hypothesis without establishing every kernel permission detail. No production focus implementation changed.

Current evidence is `/tmp/p237-physical-selection-current-uid.log` and `/tmp/p237-physical-selection-current-uid-artifacts/`. The driver return code is zero; cleanup admits only its three created OS sessions and reports an empty remaining list. The failed mapped-root result remains in `/tmp/p237-physical-selection-fresh-owned-artifacts/`. These are temporary synthetic fixture artifacts, not installed or provider acceptance. Cross-desktop routing, generation fences, reviewed UI and full P237 acceptance remain open.

## Fixture source gates

The changed fixture passes Cargo workspace format check, strict workspace Clippy with warnings denied, Python syntax compilation, patch hygiene and local links for all three changed Markdown files. Clippy runs through Cargo admission with one job and a three GiB cap and finishes successfully in 2m31s. These are focused source gates; no CI, production publication or broader P237 merge readiness is claimed.
