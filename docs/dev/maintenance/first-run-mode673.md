# Fresh-profile CDP-free mode continuity

Product lane: PL-BUGFIX
State: OPEN
Work item: CochranResearchGroup/agent-browser#673
Primary writer: MODE-HANDOFF-CONTROL-649

## Bounded outcome

Keep Chrome first-run onboarding and default-browser prompts from replacing the
intended page when a fresh automation profile becomes CDP-free. Preserve native
session restoration, initial URL behavior, exact mode scope, process custody and
no-CDP posture. Reuse the existing checkout and retained candidate custody.
This lane owns the manual Chrome launch argument list and matched regressions;
peer placement and MCP default-mode work retain their existing ownership.

## Diagnosis

A disposable Chrome/X11 loop first visits a loopback marker with first-run UI
suppressed, closes gracefully, then requests native session restoration. Omitting
first-run suppression in the replacement reliably opens the generic Chrome
welcome window and loses the visible marker. Adding only the two suppression
flags restores the marker with the same profile, executable and saved session.
Both runs exit every owned process. This narrows the defect to manual startup
posture, without account, credential or production-browser effects.

Add a failing argument-builder regression before changing production code. Run
matched Rust checks, strict workspace Clippy/formatting, required guidance and
exact-artifact qualification. Freeze source during production build. Actual
acceptance must keep the same page and handoff through mode conversion and prove
interactive return, then clean only the disposable browser. Source, installed
and actual workflow proof remain separate.

## Source qualification

The new argument-builder regression failed on the missing first-run flag before
production changed. After the narrow manual/no-debugging branch fix, 21 launch
argument tests, 3 ordinary recovery tests, 28 mode tests and the manual-login
guard passed. Strict workspace Clippy and formatting passed. Policy wiring (11),
changed links, handoff documentation, docs build, host provisioning, VM harness,
viewer assets, database durability and route-user contracts passed locally.
The root guide is 9,991 bytes; independently edited shared guidance is preserved.
No hosted CI ran. Production and installed workflow qualification remain open.

Serial Standards review found no blocking breach: no new API, viewer, CDP,
private-input reuse or process-identity bypass. Serial Spec review found the
change matches the isolated first-run failure while retaining the distinct
CDP-backed manual-login posture. Actual page continuity through the public mode
toggle remains a required acceptance gate, not inferred from argument tests.
