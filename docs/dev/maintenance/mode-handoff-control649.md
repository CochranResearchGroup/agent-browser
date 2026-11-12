# Mode-change and Native viewer continuity

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#649
Related owner: CochranResearchGroup/remote-view#883
Branch: fix/mode-handoff-control649

## Outcome

An existing human viewer reflects a completed physical browser mode change and can continue interactive takeover and return on the same durable handoff. Lifecycle close supplies the exact current Native epoch without weakening generic input guards.

## Current State

Source qualification passes. The exact compiled owner test artifact passes217
handoff tests (one ignored) and two lifecycle-close tests, including active and
returned mode invitations, current kernel/publication identity and refusal
variants. Strict workspace Clippy, Rustfmt, docs build, policy wiring and changed
links pass. Production candidate packaging, exact-binary install/discovery smokes,
integration, preserving adoption and actual same-viewer acceptance remain open.
The Remote View peer has37 passing browser checks; its source is separately owned.

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
