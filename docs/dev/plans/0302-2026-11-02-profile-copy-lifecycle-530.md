# P302: Restore normal closure for owned manual profile copies

Product lane: PL-BUGFIX.
Work item: P300 dependent #530.
Branch: fix/profile-copy-native-close-530. Canonical base:33de9da49ff15cb4cf844ed203ac55e205f3e8db.

## Bounded outcome

An owned CDP-free manual browser can be gracefully closed for a consistent profile copy through its existing lifecycle owner even when generic desktop input is unavailable. Reuse P301 authenticated, PID-scoped WM_DELETE and fresh Native desktop/process context. Do not add a viewer, change input policy, force-close, or weaken original-process, task, generation, clean-exit, profile-lock or source-custody checks.

A strict lifecycle-close hook must positively acknowledge the normal close request rather than return idempotent success for an already absent process. Retain a durable attempt before dispatch. Unknown or lost acknowledgement blocks automatic redispatch and clean-copy promotion; a known missing or rebooted original is not clean-close proof. Preserve borrowed/nonmanual behavior. Public operation cancellation remains terminal and releases only its own fence.

## Custody

The P300 worker is sole source writer in the integration checkout for the copy host and minimal effects/runtime/model hook and tests. Root owns docs, focused integration inspection and runtime. Root consumes the frozen patch serially in the main-based bugfix checkout; no simultaneous source edits. The prior temporary repair checkout was lost across reboot and restored at its retained Git ref before switching to this canonical-main branch. No candidate bytes or missing test logs were reconstructed. Preserve foreign checkouts and the integration candidate. The other missing worker registration was restored at its exact remote-backed c4e8079b6 source, inspected clean with no pinned or active candidate, then removed through governed operation worktree-closeout-735d2da1-f185-4c22-836a-7d9c0a468022. Its branch and source remain retained; no missing candidate or test bytes were reconstructed.

## Qualification

Require a public hosted RED/GREEN with generic input disabled; exact lifecycle dispatch and original identity; changed task/process/desktop generation rejection; acknowledged pending close without repeat; unknown acknowledgement without replay; reboot/missing source without clean-copy promotion; terminal cancellation without clean evidence. Reuse the existing authenticated Xvfb close transport fixture. Run relevant native/model regressions, full format and strict workspace Clippy through cargo-safe. Hosted CI stays disabled.

Build one source-bound production-shaped candidate, merge the bounded repair through this fork, verify canonical source equivalence, inspect preserve-mode installation and current transaction before applying, then run installed doctor. Actual account portability, synthetic storage divergence and exact owned cleanup remain separate qualification under530. No private profile/account/runtime artifacts belong in Git.

P300529/530 remain open. Its final broad code-review and bounded post-review TDD counters remain0; ordinary implementation RED/GREEN and focused self-check do not consume them.
