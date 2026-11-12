# Pointer-down button activation659

Product lane: PL-BUGFIX. Work item: CochranResearchGroup/agent-browser#659.
Owner: current SSO qualification lane. Target: main. State: OPEN.

## Outcome and observed failure

A normal account-menu button must receive the pointer sequence its handler uses.
The installed public click acknowledges the request, but a disposable synthetic
pointer-down menu records only an untrusted JavaScript click and remains closed.
The real selected SSO account menu exhibits the same failure. Browser and Auth
independently agree on document identity; the button is visible, focused,
enabled and unobstructed. This blocks independent website-account proof.

`interaction::click` currently invokes HTMLElement.click for one left click on
buttons, button-like inputs and role=button controls. Its successful shortcut
never reaches the existing CDP mouse dispatch path. The bounded repair routes
ordinary activation through that existing pointer path. Preserve the separate
link and explicit new-tab branches; do not redesign navigation or add site code.

## Custody and scope

Continue in the same owned SSO repair primary checkout on
`fix/sso-pointerdown-click`, from276d58c85. Prior ordinary-handoff source and its
named branch remain integrated and retained; candidate/evidence artifacts are
preserved. Its remaining operator/viewer verification is runtime work, not a
second source writer. The lane inventory was refreshed; five existing checkouts
and seven missing prunable administrative records were observed. No new checkout,
prune, foreign edit, candidate deletion or process cleanup was performed for
admission. Every foreign checkout and concurrent mode-restoration intent remains.

Write surface: interaction.rs and its existing tests, one synthetic pointer-event
acceptance, required help/README/skill/docs, and this plan/catalog. Authority is
the resumed full SSO qualification. No vault mutation, private credential replay,
manual runtime DB edits, profile reset, Native ACL workaround, human-owned target
inspection or memory write is included.

## Acceptance and validation

1. Preserve the installed public synthetic red event receipt. Its one click
   records only click/isTrusted=false; expected pointer-down/up and mouse-down/up
   are absent and aria-expanded stays false.
2. Add the protocol regression before the repair. Cover CSS selector, cached
   button ref and selected child-frame session. Keep real Chrome event truth
   separate from stubbed protocol routing.
3. Route through existing target geometry and input dispatch, once. Do not
   synthesize DOM pointer events, combine DOM and pointer activation, or retry
   an uncertain dispatch. Check ordinary onclick once and relevant refusal paths.
4. Run affected checks plus Rustfmt/strict workspace Clippy. Select remaining
   local checks from the complete slice. Bind artifacts to exact source/inputs;
   use Cargo Signal and the repository admission wrapper. Reuse passing evidence
   only with explicit unchanged-input proof.
5. Qualify the exact candidate on the synthetic page before supported preserving
   adoption. Record source, merged checkpoint, binary, installed task click and
   selected SSO website-account proof separately. An expired SSO journey stays
   expired; a fresh requested journey needs a new intent/key.
6. Close fixture task/browser/server through their owning paths and verify fresh
   OS process absence. Keep historical first failures and private evidence out
   of Git. Full SSO and original viewer-access gates remain independent.

## Current evidence

Installed public red is captured in the private SSO qualification packet.
Disposable fixture Chrome and server were confirmed absent after closure.
The initial MCP close transport failed; fresh request readback found no operation,
and the same bounded close intent completed through a fresh client. Its terminal
receipt and OS readback are retained. Existing Browser process identities and
human custody were preserved during Auth's separate R3 adoption.

The compiled protocol regression goes red for the intended missing pointer
sequence (empty actual sequence against mouseMoved/mousePressed/mouseReleased).
The first build exhausted compiler helper threads; the second caught a missing
test import. Those are retained setup/resource failures, not behavioral RED.
Warm shared-target test compilation then produced the actual RED. The repair
removes the low-level DOM shortcut and retains upper link/new-tab branches;
focused GREEN and strict validation are pending. No659 candidate or installed
Browser effect has occurred.

Two bare-CLI fixture setups were stopped before input: one lacked explicit
reviewed executable selection; the next lacked an owned tab handle. That tactic
is ended. The supported MCP fixture admits its task; a parsing error and then
an unverified process guard are retained as separate setup/identity failures.
No additional pointer replay is allowed while the fixture's mode is unverified.
A final read-first persistent-profile probe may diagnose mode before any click,
matching the selected SSO workflow. If it stays unverified, stop that fixture
approach and retain the exact reason; do not change authority or production
configuration. The production public synthetic RED remains valid.

Current659 execution window ends2026-11-12T14:50Z, including earlier effort.
Controller: this SSO lane. Before any build/adoption beyond that window, assess
remaining complete proof and report the exact unmet gate. The unchanged broader
SSO objective and original first failures remain. No cheap fixture repair resets
the allowance; source integration, exact artifact qualification, preserving
adoption and actual selected account proof remain required. Review date:2026-11-13, owner: current SSO
qualification lane. Prior handoff candidates remain retained without alteration.
