# One static image round

Use this opt-in profile only for a visible static 3×3 hCaptcha grid with the
English instruction “Select all images containing buses” and one Verify button.
It supports one round, at most four selected tiles and one Verify click. Other
instructions, dynamic replacement, hidden/partial/duplicate grids and ambiguous
images require the existing human-assistance path. Recognition alone grants no
input authority.

The task owner requires the exact profile/SitePolicy membership and one enabled
`captcha_solve` provider with
`configRef: agent-browser:remote-view-native-image-static-3x3:v1`.
The SitePolicy must explicitly list that provider ID in `allowedChallengeProviders`;
an empty list grants no image authority. The checkbox provider configuration
does not authorize this image profile.
Current verified CDP origin qualification remains required; Native perception
and input use Remote View, and CDP-free tasks retain human assistance.

## Observe, select, attempt

Create one stable original observation ID through public `browser_command`:

```json
{"taskId":"TASK","action":"native_capture","requestId":"image:observation","params":{"classifyChallenges":true,"registeredActionId":"hcaptcha-image-grid-3x3-v1"}}
```

Inspect the actual returned Native image. `nativeImageObservation` contains the
frame/puzzle identities and nine candidate IDs with owner-observed bounds. An
image reader may propose candidate IDs, never coordinates or input instructions.
Do not infer the selected IDs from fixture labels, application internals or a
prearranged provider response. Unsupported or ambiguous content needs assistance.

Submit the selected IDs once with a distinct stable original attempt ID:

```json
{"taskId":"TASK","action":"native_input","requestId":"image:attempt","params":{"imageAttempt":{"observationRequestId":"image:observation","selectedCandidateIds":["OWNER-OBSERVED-ID"]}}}
```

Do not combine `imageAttempt` with `actions`, `nativeAttempt` or a caller control
epoch. The owner derives the tile centers and final Verify point, admits the
selection through the existing visual-round kernel, retains the original before
effects and revalidates exact task/process/window/frame/control evidence. The
static image profile has a separate bounded 60-second observation/deadline;
checkbox timing is unchanged. A changed frame still fails the Native owner's
original `expectedVisual` guard. No recapture substitutes a new frame into an
already admitted attempt.

Image and checkbox actions share the same taskwide attempt budget. Another
request ID, capture or family cannot renew a consumed attempt. A checkbox that
opens an image puzzle therefore remains assistance under its already consumed
budget; this profile can start directly at an eligible image puzzle.

## Verify and recover

Dispatch is not clearance. Independently observe puzzle clearance and the
intended protected site's access, preserving the original task and operation
identity. Access denial remains intervention even if the puzzle cleared. A
rejection, unexpected next puzzle, expired observation or uncertain delivery
requires bounded assistance/reconciliation. Inspect `task_operation` with the
original attempt ID; pending and uncertain input never permits replay.

Image bytes and raw OCR stay ephemeral. Durable evidence contains only bound
identities, digests, geometry, budget and owner receipts. Source tests and owned
synthetic Native qualification do not establish universal hCaptcha accuracy,
real-account authorization, installation or production readiness.

## Explicit complete-puzzle region

The default capture scope remains `complete_window`. A partial capture without
an explicit image scope never becomes a registered image observation.

For a site/provider contract declaring a complete static puzzle inside an owned
page viewport, the client may set `captureRegion: {x, y, width, height}` before
creating the original. The initial capture sends the existing `region` plus
`imageCaptureScope: "complete_puzzle_region"`. The owner requires the complete
prompt, brand, nine unique aligned cells and Verify control in that same PNG/OCR
region. The response and original retain `captureScope: "complete_puzzle_region"`.
An old serialized observation with no scope means only `complete_window` and
still requires its region to equal the whole window.

`partialWindow` and the generic classification's `completeWindow` describe the
actual window coverage. They do not change to claim a full window for this mode.
Every point must lie inside the retained region, which must lie inside the
unchanged whole window and display. Native input uses the original region and
SHA verbatim in `expectedVisual`; whole-window/process/control fences remain.
Subsequent observation uses the same region. Partial unknown classification
alone proves no clearance: both independent clearance and protected-access
checks remain mandatory. Never crop a retained original or refresh its guard.

A refused capture completion can report the static expiry cause with a bounded
`observationClock` diagnostic: `reason`, `observedAtMs`, `validationNowMs`, and
`ageMs`. Reasons distinguish `future_timestamp` from `age_exceeded`. Age is
nonnegative elapsed milliseconds, clamped to zero for a future timestamp; use
the two timestamps to inspect that direction. These values grant no authority
and do not alter the 60-second limit. Inspect the retained original without
replaying capture or input.

## Opt-in two whole-grid rounds

`createNativeImageRoundsChallengeClient` from the same client entry supports at
most two static English 3×3 bus grids. Configure an enabled `captcha` provider
with `captcha_solve`, exact
`configRef: agent-browser:remote-view-native-image-static-3x3-two-round:v1`, and
explicit membership in the owner-verified SitePolicy's allowed providers.
The checkbox and single-round configurations do not grant this authority.

Capture with `registeredActionId: hcaptcha-image-grid-3x3-two-round-v1`.
The owner adds `tileImageSha256s`, nine hashes of interiors excluding eight
pixels along every tile edge. The second grid requires all nine interiors to
change, with the original window, process, generation, control, ROI, geometry and
policy preserved. Partial replacement, loading, unchanged/rejected grids and
unsupported layouts require assistance. For this profile, any OCR line containing
“try again”, “incorrect”, “please retry”, “challenge failed” or “verification
failed”, “loading” or “please wait” vetoes an observation, regardless of recognition confidence. This
conservative veto does not alter positive recognition thresholds.

Each input uses exactly these fields:

```json
{"imageAttempt":{"originalRequestId":"image:attempt","observationRequestId":"image:observation","selectedCandidateIds":["CURRENT-OWNER-ID"]}}
```

For round one, `originalRequestId` equals the input's `requestId`. Round two uses
a new input `requestId` and fresh durable observation, retaining the original
ID. Both rounds share one task, one attempt and the original 60-second deadline;
the selection callback receives `roundIndex`, the remaining bounded deadline
and an abort signal. The maximum is four tiles plus Verify per round, ten
clicks, thirty pointer events and zero key events cumulatively.

The first operation stays immutable. The second retains accumulated counts and
the original deadline. Results include `roundOperations` for inspection. A lost
acknowledgement, cancellation, rejection or exhausted budget returns intervention
with `retryAllowed: false`. Repeating either request reads its original result
without replay. The client never creates a replacement task. Independent future
authorized tasks remain possible; this is not a browser-lifetime challenge ban.
Success still requires independent clearance and protected application access.
Provider-free protocol tests do not qualify actual Native input or real sites.
