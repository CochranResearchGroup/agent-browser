# Mode-change and Native viewer continuity

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#649
Related owner: CochranResearchGroup/remote-view#883
Branch: fix/mode-handoff-control649

## Outcome

An existing human viewer reflects a completed physical browser mode change and can continue interactive takeover and return on the same durable handoff. Lifecycle close supplies the exact current Native epoch without weakening generic input guards.

## Current State

PR652 and Remote View PR887 are merged. Preserving installation and the public
CDP on to CDP-free workflow passed, including same durable-link rejoin and Native
interactive control. A reverse CDP-free to CDP acceptance failed because the
keyboard close did not close the blank original window. The bounded exact-window
close follow-up below remains in validation; installed round-trip acceptance is
not complete. FigureLabs remains CDP-free and its durable link loads successfully.

## Validation

- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: pass.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: pass.
- `scripts/ci/rust-tests.sh --focused completed_mode_handoff`: three pass, with seven refusal variants in the negative case.
- The exact compiled test binary then runs `native_mode_close --test-threads=1` and `handoff --test-threads=1` in disposable HOME/XDG roots:219 distinct passes, one ignored. It is pinned outside shared Cargo output with source-path and test-inventory qualification; a redundant waiting Cargo stage was canceled rather than racing another lane's build.
- `NEXT_TELEMETRY_DISABLED=1 pnpm --dir docs build`, `pnpm test:policy-wiring`, `pnpm test:documentation-links -- --base origin/main` and `pnpm test:remote-view-handoff-docs`: pass.
- Source-independent workstation provisioning, VM harness, Guacamole assets/durability and route-user fixtures pass. Native install and discovery smokes require the sealed fork candidate; an unpinned package fallback is not qualifying evidence.
- Hosted CI is disabled; all checks are local. No target-platform or installed acceptance is inferred.

## Evidence and delivery

The production viewer script has a deterministic RED: an owner changes from CDP to CDP-free while the existing toolbar retains CDP on after focus and visibility return. Retained replacement can also leave a prior-process Native observation and invitation digest. Private workflow identities and probes stay outside Git.

Build failing tests at the actual viewer and host/callback seams, then repair one cause at a time. Verify current-process lineage from the retained mode journal before rebinding an existing invitation. Preserve unknown original request IDs and all unrelated browser/profile state. No raw runtime edits, generic epoch autofill, force close, or new viewer grants.

Run local changed-surface checks, review the full slice, merge through a PR and adopt qualified exact artifacts while preserving peer processes. Final acceptance must exercise CDP mode and interactive controls in the public same-handoff viewer; process-only checks are insufficient.

## Coordination

This repair is the primary writer for the mode-close and handoff continuity surfaces and the Remote View mode toolbar. The linked owner is the dependent half of this single serial repair. Earlier retained-target timestamp repair remains integrated; its interrupted build is separate.

## Standards

One final serial review compares `git diff 4f0b61959b97f0739374831a24cb0a0342998a02...HEAD`, including source
checkpoint `ff2a33610`. Governing standards are AGENTS.md, the repository policy
entry, its testing/ownership/runtime boundaries and existing domain vocabulary.
No current hard-standard breach or blocking maintainability finding is accepted.
The frozen target is private journal evidence; Native still owns physical input.
Local source checks are distinct from candidate and installed acceptance.

## Spec

The user's coupled CDP/interactive failure, issues649/883 and the browser product
contract govern the review. The implementation retains the same handoff, scopes
Native return to this viewer, fences lifecycle close, derives replacement
continuity from its completed owner record and refreshes the visible posture.
Development RED cases for failed/in-flight refresh and premature reconnect were
resolved before this final pass. No current blocking Spec finding is accepted.
Standards:0 open; Spec:0 open. Final broad review count:1. The installed same-viewer
journey remains a separate acceptance gate.

## Installed acceptance follow-up

PR652 and Remote View PR887 were merged and their exact candidates installed
through preserving adoption. The public disposable workflow passed CDP on to
CDP-free, same durable-handoff rejoin, and interactive takeover. FigureLabs
remained CDP-free and its exact pending return was confirmed before takeover.

The reverse disposable CDP-free to CDP transition exposed a further live defect:
`native_close` dispatched Alt+F4, but its active blank browser stayed alive,
without a modal dialog. Remote View's exact-process, current-epoch `window.close`
then gracefully closed that same window and the kernel confirmed process exit.
Its implementation sends `_NET_CLOSE_WINDOW`; the previous comment describing
it as window destruction was incorrect. The retained reverse mode operation
remains open with zero replacement launches. No forced termination was used.

The bounded follow-up replaces the keyboard chord with Remote View's guarded
window-close operation. Regression coverage must require this exact window and
process identity for active and inactive cases, preserve operator-lease refusal
and generic input epoch requirements, and normalize only a qualified close ACK
as dispatched. Process exit remains a separate ModeHost observation. Source
checks and a fresh installed public round trip are still required.

## Bounded close follow-up review

Fixed point: `ed6dcf37cd17a03657e4f0b75d4b5a0c3a91031d`, preserving
PR653/654. Review scope is the lifecycle-close adapter, its existing owner tests,
the exact catalog assertion correction, and explanatory context.
Standards: zero open findings. The Native owner retains window/process and epoch
fencing; no generic input relaxation or foreign lease effect is introduced.
Spec: zero open source findings. The dedicated graceful close addresses the
measured keyboard delivery failure, while ModeHost still independently proves
exit. Installed full round-trip acceptance remains open and is not inferred from
this bounded review or the passing source tests.
