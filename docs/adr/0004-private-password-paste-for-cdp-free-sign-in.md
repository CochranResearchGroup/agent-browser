# Private password paste for CDP-free sign-in

Status: Accepted.

The agent receives the approved username, observes the sign-in page, focuses the
password field using native input, and invokes one approved paste. Auth Receipts
retains credential custody. The agent owns page and field interpretation.
Remote View checks the current control and window binding. Its paste contract
requires no second browser-origin or password-field classifier.

Remote View stages the password on the desktop-scoped clipboard, pastes it, and
clears it within the same operation. Clipboard sharing stays suspended until
cleanup is verified. A cleanup failure is recovered independently of the paste.
The agent receives the outcome without access to clipboard contents.

The agent submits the form separately after acknowledged paste delivery and
verifies the provider response. When Remote View proves delivery never started,
it returns the exact refocus or control-return action. The agent can retry using
the original approval while it is valid.

An uncertain paste retains its original attempt for readback before another
paste or submit. If readback cannot resolve delivery, Remote View finishes
clipboard cleanup and offers the same handoff link for operator inspection.
The operator can continue on the page or explicitly start over. The original
attempt remains in the record.

## Approval

Auth Receipts applies its existing rules for the account, sign-in request,
and browser task. If an owner decision is required, Auth uses the normal
operator notification flow. CDP-free delivery adds no approval step.

## Acceptance

First demonstrate the flow with synthetic credentials in stock Chrome with
CDP disabled. Verify that a focused field receives one private paste, clipboard
cleanup is confirmed, a proven no-paste outcome can be retried, and an uncertain
result preserves the original attempt for readback and operator handoff.

Then run the Synchrony sign-in journey in stock Chrome with CDP disabled.
Record paste delivery, form submission, and the observed provider outcome
separately. Call the journey complete when the intended account is verified as
signed in.
