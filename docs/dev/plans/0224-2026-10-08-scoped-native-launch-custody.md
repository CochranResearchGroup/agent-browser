# P224: Scope unfinished native launch custody

Product lane: PL-BUGFIX
Status: COMPLETED
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

## Installed acceptance

Source integrated through PR 225, merge
`ea42b7181f8dac4fae9ad4926c50c617c2fe681a`. The sealed production binary
SHA256 is `2b7487260770c53422bcbdb4cca338cf8a4aed1b25e4c09a048987cc26c0d9d3`.
It passed three isolated fresh-profile launch/read/close/residue cycles with
production unchanged. The supported production install transaction accepted
those exact bytes, and the installed native doctor passed.

The original ordinary browser command succeeded after installation. The same
retained handoff opened the native desktop through existing authentication,
showed the intended application login page, and reconnected successfully.
Native mouse/keyboard input produced a synthetic address-bar marker; Escape
restored the page and ordinary automation read the unchanged URL afterward.
The logical browser, profile, active sessions and exact handoff binding were
preserved. The unrelated unfinished claim remained byte-equivalent by curated
record hash. No manual runtime-state repair or reboot occurred. Application
login continuity is not accepted: the application is at its login page.
Private browser artifacts and runtime receipts remain outside the repository.

Preserved harness limits: initial build-result parsing stopped before any
installation; one canvas locator click timed out before input and the corrected
coordinate-based check passed. The new isolated namespace had no published local
ingress and no native provider binding, so its general doctor reported missing
ingress and its local-mode install doctor reported legacy helper prerequisites.
No legacy helper repair or ingress publication was attempted. The standalone
candidate doctor against the old production selection reported expected executable
mismatch; installed native doctor passed after the guarded transaction selected
the candidate. These failed probes are not relabeled as clean doctor results.
The three temporary isolated services were stopped and disabled after smoke;
the authenticated observer page was closed and shared observer resources preserved.
