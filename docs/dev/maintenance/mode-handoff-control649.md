# Mode-change and Native control continuity

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#649
Related owner: CochranResearchGroup/remote-view#883
Branch: fix/mode-handoff-control649

## Outcome

An existing human viewer reflects a completed physical browser mode change and
continues interactive takeover and return on the same durable handoff. Lifecycle
close uses the exact current Native epoch without weakening generic input guards.

## Source delivery

PR652 delivers the frozen mode-handoff lineage proof and lifecycle epoch fence.
Remote View PR887 delivers mode refresh, completion-gated reconnect and own-lease
return before confirmed conversion. PR655 replaces unreliable keyboard close
with the Native owner's exact-window graceful close operation. Its source is
`cff99da8c08e154c24291df4a668d67def419249`; merge
`632354ddcffe058a479f9090f3599530c3427b9c` has the same executable inputs;
its additional ordinary-recovery qualification documents are preserved.

The keyboard-close regression reported dispatched delivery while an active
about:blank browser stayed alive without a modal dialog. The exact-window close
request closed it and separate OS observation confirmed exit. Remote View sends
`_NET_CLOSE_WINDOW`; the old comment describing window.close as destruction was
incorrect. The adapter now requires the matching close acknowledgement. ModeHost
still independently proves exit before replacement; inactive windows need no
focus-changing keyboard sequence.

## Validation

- The new adapter regression failed on the keyboard implementation: one pass,
  one fail, no compiler errors. Both close tests then passed after the fix.
- `scripts/ci/rust-tests.sh --focused mode_`:27 pass in one composed invocation,
  including close, handoff lineage, mode-owner and refusal cases. The earlier
  PR652 qualification had219 distinct handoff/close passes and one ignored test;
  these overlapping inventories must not be added together.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: pass.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: pass.
- Policy wiring11 and changed documentation links: pass. The earlier slice also
  qualified the docs build and provider-free workstation support fixtures.
- Guidance discovery now asserts the exact17 public tools and their published
  contract parity, replacing a stale count-only oracle. Its skill guide remains
  below the repository size limit. Shared user-scoped publication is separate.
- Hosted CI stays disabled under operator policy; no target-platform CI claim.

## Sealed candidate qualification

The production-shaped candidate built through Cargo Signal and the repository
admission wrapper with four Cargo jobs and cache disabled. Source, toolchain,
input closure, release profile and manifests are retained with its sealed artifact.
Binary SHA256:
`3ef1d9937212714b9da3cc35f432635e97e8f9dc56d9ea5dfa803e27f0eb026f`.
Binary and manifest hashes, candidate inspection, exact-binary public MCP
catalog/guidance/refusals and the source-free workstation install fixture pass.
The fixture retired its two owned hosts with zero residue.

Operator deployment, browser identities, process readbacks, control leases and
raw acceptance receipts belong in associated runtime state outside Git. Candidate
qualification is distinct from installed Native workflow acceptance.

## Remaining workflow qualification

The public CDP on/off/on/off workflow and interactive takeover/confirmed return
passed on one durable handoff after PR655 adoption. Every conversion completed
and loaded a new document on the same link; takeover used a fresh agent epoch.
Disposable test cleanup and runtime identities remain in associated private state.
This qualifies that workflow, not universal interrupted-navigation resilience.

Explicit reconciliation can continue a previously approved operation and launch
its replacement. It is not observation-only. Preserve the original operation ID
and read its durable outcome after navigation races the response; never replay
an uncertain action just to recover its response body.

A separate interrupted-navigation gap remains: a conversion paused after original
exit can return503 on fresh durable navigation, while a retained viewer can still
expose its original operation. Current owner semantics deliberately prohibit
reconnect from advancing a pending conversion. A follow-up must expose the
existing recovery controls without silently treating navigation as close/launch
consent. PR655 does not implement that protocol change. Universal handoff
resilience and full installed acceptance remain open.

## Coordination and review

This repair owns mode-close and completed-mode handoff continuity. Preserve the
ordinary recovery and interrupted-close owners integrated by PR653/654. Keep
Native desktop/viewer ownership in Remote View; do not add a second access layer.

The bounded PR655 review uses fixed point
`ed6dcf37cd17a03657e4f0b75d4b5a0c3a91031d`. Standards:0 open findings for
that source slice. Spec:0 open source findings for guarded graceful close. These
counts do not close the remaining workflow and interrupted-navigation gaps.
Generic input still requires its supplied epoch; lifecycle close neither acquires
nor returns a foreign operator lease. Private original processes and frozen
handoff targets stay in owner journals, outside source documentation.

## Read-only interrupted-conversion recovery

The next bounded slice exposes the existing operation on the same Native viewer
when its exact original has exited and a graceful close remains pending. The
owner must prove the retained handoff against its frozen mode binding, current
logical scope, exclusive browser custody and positive original absence. It
returns a not-ready recovery outcome, never owned-window readiness. No navigation
advances the operation, launches a replacement or updates Native use evidence.

Remote View accepts a narrowly validated recovery posture on its existing Native
page. That posture initializes only the existing mode controls: no desktop
stream, recording, input connection or automatic takeover. Existing mode actions
retain their original operation and confirmation requirements. A completed
operation rejoins the same durable link. Provider rendering/URL support lands
before the owner emits that posture.

Primary writer: this lane for the mode journal proof, handoff resolver and Native
mode-viewer glue. Dependencies: integrated ordinary recovery PR653/654 and the
Remote View recording work on main. Preserve recording/audio/assistance sources
and the separate shared Cargo target; use an isolated Remote View build target.
Acceptance requires a red interrupted-close fixture, read-only recovery proof,
refusal variants, exact existing-operation reconciliation and same-link Native
continuation. Keep tenant/operator evidence outside Git.


Follow-up checkpoint: public missing-original navigation reproduces HTTP503 with
its retained ClosePrompt and zero launch attempts. The existing viewer recovery
regression fails against pristine viewer source and passes against this slice.
All38 browser checks pass after updating the Native HTML fixture to substitute
production script selection; initial fixture failures are retained separately.
Policy wiring11, changed links, handoff guidance and docs build pass. Rust and
installed recovery qualification remain pending; this checkpoint is not merge-ready.


## Ordinary CDP-free exit follow-up

A fresh same-link acceptance found a separate missing-original case with no
active conversion. The owner returns `manual_login_ended` because manual runtime
reconciliation permits only mode or settled-idle recovery. A valid retained
handoff after ordinary process exit has neither. This is a newly demonstrated
blocking case for the requested existing-profile CDP-free relaunch; it reopens
only the manual runtime classification and actual-runtime ordinary-exit fixture.
The isolated source candidate already building remains bound to its earlier
commit and cannot qualify this follow-up. Do not deploy it as the final repair.

Keep this lane primary writer for manual recovery and its focused fixture;
preserve integrated599 ordinary publication/rebinding and658 mode restoration.
Require exact persisted browser/profile scope, exclusive ownership, a valid
retained handoff, published original identity and positive original absence.
Existing recovery coordination still proves whole-profile absence and Native
assignment before any launch. No idle event is fabricated; unhanded ended login
tasks, concurrent profile operations and foreign/unknown processes remain fenced.
Add the actual worker regression before the fix, then qualify same-profile,
same-task, same-link recovery without CDP. Runtime facts remain outside Git.


The actual worker regression reproduces `manual_login_ended` in2.58seconds.
After runtime classification is repaired, the existing model's Native publication
guard exposes its idle-only restriction. The admitted Native launch and complete
replacement identity are retained while extending publication to a retained
handoff. No CDP page replay is introduced. The expanded fixture then reproduces
`browser_handoff_target_changed` for an already-returned invitation after ordinary
recovery. Exact existing recovery proof now rebinds that passive invitation and
permits a Native-only no-op return without changing cooperative custody. Unknown
passive histories retain their prior no-rebind behavior. Validation remains open.
A stale compiled navigation assertion produced one fixture-only failure and is
excluded from product RED; source is frozen before subsequent compilation.
