# Retained mode Native target continuity

Product lane: PL-BUGFIX. Work item:647. Branch: fix/retained-mode-target647.
Source base: bcf97e15. Owner: ecochran76. State: OPEN.

Preserve an existing mode operation through harmless handoff activity metadata
changes. A frozen mode binding currently supplies complete historical objects
to the Native guard, which correctly requires exact current objects. Diagnose
with a real SQLite host at the mode target selector and concrete Native command
boundary; original process custody is deliberately absent so the fixture cannot
dispatch. RED must reject changed activity/expiry metadata at the target guard.

Repair only mode target selection, retaining Native's complete current object
and persisted-state comparison. Permit only explicitly enumerated observation
timestamps; preserve physical browser/PID/profile/endpoint/desktop, task/current
tab/target, created identities, affected task membership, original process and
concurrent persistence fences. No provider, runtime, control, authentication,
close, force, replacement or source installation actions are authorized. No new
worktree. Root owns private evidence, integration and installed verification.
No commit/push before root review. Bound45minutes/three meaningful attempts.

This upstream dependency is separate from Contacts final review/findings TDD.
Memory disposition: forbidden under delegated source-only scope.

## Qualified diagnosis and repair

Concrete RED: job-20261111T231200Z-345a74a384a6,143.702seconds including
resource-admission wait, one failure/zero passes. The actual Native command
returned browser_native_target_changed for only the three timestamp changes;
its next missing-original-custody boundary was expected. No setup/compiler
failure was mistaken for RED. Private field-diff evidence independently confirms
the same three field names; no private values or identifiers enter this plan.

Binding.target now requires current state. It returns current browser/session/
tab clones only when complete identity comparison succeeds after substituting
retained session last-activity/expiry and tab last-activity timestamps. Every
call site uses it: graceful close, Escape cancellation, cancellation readiness,
focus and replacement assignment qualification. Existing Native and Remote View
full-object baseline guards remain unchanged. This does not rewrite the retained
operation or grant a launch/close/force action.

Regressions cover the actual Native command guard,18 changed identity/membership
fields and concurrent persisted-state conflict. A concrete configured Remote
assignment test also verifies that frozen metadata is refused and refreshed
metadata reaches the absent-desktop boundary without any provider connection.
Candidate architecture, policy wiring, handoff documentation and five changed
documentation link checks pass; root skill9992bytes preserves candidate tokens.
Final source test/Clippy/format receipts follow after terminal qualification.

## Final source qualification

GREEN job-20261111T231952Z-f53f5622b13e passed both regressions in61.964seconds
including build. An intermediate run correctly passed target selection but
exposed the fixture's exact downstream original-custody refusal as unavailable,
not missing; its assertion was corrected, with no additional production change.

Forty-one distinct focused tests passed with zero failures or ignored cases:
retained-mode Native/assignment regressions; Remote wrapper; local and Remote
signed return during pending conversion; native handoff; physical mode
inspection; Native control targeting; host-control integration; public standalone
return process/target/epoch cases. Overlapping filters counted once.
Exact executable SHA256:
`24fdd62d7e5ae91442f64ada776fe3b6ce7db5722f981bf5d2f3a07d21ad0fed`.

Strict workspace `clippy --workspace -- -D warnings` passed, job231957,
88.963seconds including lock wait, zero diagnostics. Formatting check passed.
Hosted CI is skipped under operator cost policy. These are source checks only;
no deployed mode completion, launch, close, authentication or force is claimed.
Parent owns actual installed workflow acceptance. No commit/push before review.
