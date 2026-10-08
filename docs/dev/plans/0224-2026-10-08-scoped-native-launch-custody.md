# P224: Scope unfinished native launch custody

Product lane: PL-BUGFIX
Status: IN_PROGRESS
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/224
Branch: fix/scoped-launch-custody
Checkout: agent-browser-p204, reused from a clean integrated branch after inventory.

## Outcome

Recover an idle retained browser through an ordinary command and its existing
native handoff even when another profile has an unfinished launch. Preserve that
claim, logical browser/session/tab identity, same-profile launch protection and
assignment-generation fencing. Remote View continues to own the desktop/viewer.

## Diagnosis and loop

The installed ordinary retained-session URL read returns
`remote_view_session_launch_readback_required` before recovery. Runtime effects
construction globally examines every pending claim and requires a retained
browser even for an unrelated cold-launch claim. The service-model constructor
and operation admission repeat a global pending-claim barrier.

The provider-free regression adds one pending unrelated profile launch to the
existing ordinary idle-recovery workflow through the real host and SQLite store.
It checks continued automation, stable logical identity, changed physical target,
no new provider assignment, and exact preservation of the unrelated claim.
Same-profile restart refusal and SQLite assignment conflict tests must remain green.

## Boundaries and validation

This lane is primary writer for the native session effects and consumer fixtures.
Issue 218 is related acceptance work; PRs 191 and 184 and every unrelated dirty
checkout remain preserved. No legacy RDP or root-helper changes, reboot, proposal
edits, authentication reset, new PRF handoff, or manual runtime-state repair.

Run the red regression before implementation; then focused consumer and custody
checks, service-model tests, format and strict workspace Clippy. Integrate through
a linked PR from fresh canonical main. Qualify an installed candidate before
production recovery; source tests alone do not prove the original handoff ready.

## Source validation

The original cross-profile regression failed with the installed error in 0.09s
before the fix. The repaired consumer suite passed all 30 tests, preserving
same-profile restart refusal. All Service Model tests passed; its existing
unused-import warning in a separate manager fixture remains unchanged. The six
SQLite launch-custody tests passed, including conflicting assignment admission,
recovery history, release fencing and unknown-claim persistence. Format and
strict workspace Clippy passed. The final augmented regression additionally
checks that the retained native handoff resolves as operator-visible ready;
its final execution passed in 0.14s. Original authenticated browser presentation
still reproduces the user's exact unavailable message on the old installation.
Production publication and original-link recovery remain pending.
