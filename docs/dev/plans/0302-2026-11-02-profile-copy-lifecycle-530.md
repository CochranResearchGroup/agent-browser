# P302: Restore normal closure for owned manual profile copies

Product lane: PL-BUGFIX.
Work item: P300 dependent #530.
Branch: fix/profile-copy-native-close-530. Canonical base: 33de9da49ff15cb4cf844ed203ac55e205f3e8db.

## Bounded outcome

An owned CDP-free manual browser can be gracefully closed for a consistent profile copy through its existing lifecycle owner even when generic desktop input is unavailable. Reuse P301 authenticated, PID-scoped WM_DELETE and fresh Native desktop/process context. Do not add a viewer, change input policy, force-close, or weaken original-process, task, generation, clean-exit, profile-lock or source-custody checks.

A strict lifecycle-close hook must positively acknowledge the normal close request rather than return idempotent success for an already absent process. Retain a durable attempt before dispatch. Unknown or lost acknowledgement blocks automatic redispatch and clean-copy promotion; a known missing or rebooted original is not clean-close proof. Preserve borrowed/nonmanual behavior. Public operation cancellation remains terminal and releases only its own fence.

## Custody

The P300 worker is sole source writer in the integration checkout for the copy host and minimal effects/runtime/model hook and tests. Root owns docs, focused integration inspection and runtime. Root consumes the frozen patch serially in the main-based bugfix checkout; no simultaneous source edits. The prior temporary repair checkout was lost across reboot and restored at its retained Git ref before switching to this canonical-main branch. No candidate bytes or missing test logs were reconstructed. Preserve foreign checkouts and the integration candidate. The other missing worker registration was restored at its exact remote-backed c4e8079b6 source, inspected clean with no pinned or active candidate, then removed through governed operation worktree-closeout-735d2da1-f185-4c22-836a-7d9c0a468022. Its branch and source remain retained; no missing candidate or test bytes were reconstructed.

## Destination window qualification

Canonical main combines agent-input policy with Native control readiness. Copy launch qualification instead needs an exact owned browser window, independently of permission to inject desktop input. Add a private read-only window verifier using the existing Native control window matcher and an authenticated observe-only owner adapter, preserving persisted task/browser/tab/original/assignment joins. The adapter cannot request input, window mutation or control. Copy launch uses this window proof; existing full Native readiness and guarded-input authorization remain unchanged. No new public flag, command or access layer is added.

## Qualification

Require a public hosted RED/GREEN with generic input disabled; exact lifecycle dispatch and original identity; changed task/process/desktop generation rejection; acknowledged pending close without repeat; unknown acknowledgement without replay; reboot/missing source without clean-copy promotion; terminal cancellation without clean evidence. Also require an authenticated observe-only owner regression with changed identity/generation refusal, plus unchanged main Native-input readiness tests. Reuse the existing authenticated Xvfb close transport fixture. Run relevant native/model regressions, full format and strict workspace Clippy through cargo-safe. Hosted CI stays disabled.

Build one source-bound production-shaped candidate, merge the bounded repair through this fork, verify canonical source equivalence, inspect preserve-mode installation and current transaction before applying, then run installed doctor. Actual account portability, synthetic storage divergence and exact owned cleanup remain separate qualification under #530. No private profile/account/runtime artifacts belong in Git.

P300 tickets #529 and #530 remain open. Its final broad code-review and bounded post-review TDD counters remain at zero; ordinary implementation RED/GREEN and focused self-check do not consume them.

## Local source qualification

Source checkpoint cae98bc3f was integrated with canonical main at 356a8664b through 31b721c71ea0fc96e411c543fe33c0d69f8741b8. Preserve MODE534 reconciliation and the P302 lifecycle hooks together. Only the active-lane catalog conflicted; both entries were retained with explicit overlap. The source worker checkpoint 27138b762 preserves the original nine-file patch in P300 separately from unrelated edits.

On combined source 31b721c71, the four focused CLI filters `profile_duplicate`, `lifecycle_window_readiness`, `native_readiness_` and `manual_native_context_` passed 14 tests. Full service-model tests passed 408 checks across 24 binaries, including doctests. The `browser_mode_operation` integration binary passed 16 tests. All ran locally through the Cargo admission wrapper, with a disjoint persistent target, two jobs and a 5 GiB memory bound. Full format and strict workspace Clippy passed. The docs build, policy wiring, changed documentation links and Remote View handoff documentation checks passed. Hosted CI was not used.

The worker independently executed the authenticated Xvfb close fixture, the same focused repair/context tests, and the original copy-operation integration suite: 13 passed and one existing private-mount quota test was ignored. Its format and strict workspace Clippy passed. An initial HTTP fixture omitted the manual task's first-tab acquisition; correcting that setup to match production copy launch produced a passing rerun without a production change. Earlier shared-target contamination was discarded as invalid validation and replaced by private cold targets.

This qualification paragraph changes documentation only; the passed Rust and MDX sources remain unchanged. At source qualification, the production candidate, canonical merge and installed acceptance were still pending; the bounded disposition below supersedes that checkpoint. This source result does not qualify real-account portability or close P300 tickets #529 and #530. Final P300 broad review and post-review remediation counters remain zero.

## Bounded checkpoint disposition

P302 is complete as a source and installed repair checkpoint through merged PR #536. The sealed production candidate passed its exact-binary install fixture, canonical source/input-equivalence checks, preserving installation and installed doctor. User-scoped artifact, transaction and process receipts stay outside the product repository. This does not qualify actual account portability or operation-owned copy disposal under #530. The registered auxiliary and immutable candidate remain retained for P300 serial integration, with explicit private custody rather than raw checkout removal.
