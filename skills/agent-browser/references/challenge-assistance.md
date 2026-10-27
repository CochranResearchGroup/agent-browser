# Native challenge assistance

Use `browser_command` with `action: native_capture` and
`params: {classifyChallenges: true}` for current bounded Native evidence. A
classification is not attempt authority or protected-site admission.

The optional receiving-backend helper
`@agent-browser/client/challenge-assistance` exposes `resolve({taskId,
requestId})`. Its trusted `resolvePolicy` port reads actual existing SitePolicy
and current task-policy records. Passive checks get at most three observations
and ten seconds. A registered visible checkbox gets at most one durable attempt
per task across both registered families. The runtime owner independently
checks current origin, profile/SitePolicy membership, task policy and Native
window/frame/control evidence. Source labels from a caller grant nothing.

Provider identity is separate from family and action identity. Use the existing
ServiceProvider catalog: an enabled `captcha` row with `captcha_solve` and the new
versioned adapter reference `configRef: agent-browser:remote-view-native:v1`.
That reference selects only the existing Native backend and adds no endpoint or
access authority. `SitePolicy.allowedChallengeProviders` contains this row's
provider ID, not a registered action ID. Exactly one eligible row is required;
missing or ambiguous capability/binding means assistance.

Automatic origin qualification requires the existing verified CDP task. It
reads that target's URL and performs one exact-task focus inside the retained
attempt, followed by URL/policy and Native-frame revalidation. CDP-free tasks
use human assistance here. Never attach a debugger or substitute launch URL for
their current origin.

For a low-level registered capture, supply a stable outer `requestId`,
`classifyChallenges: true` and `registeredActionId` equal to
`turnstile-checkbox-p169-v1` or `hcaptcha-checkbox-p181-v2`. An opted-in input
supplies only `nativeAttempt: {observationRequestId, registeredActionId}` and a
stable outer `requestId`; the existing owner derives its one click. Read an
interrupted original with `action: task_operation` and
`params: {requestId: ORIGINAL}`. Never repeat a pending or uncertain effect or
renew the budget by taking another capture. The original ID is an Agent Browser
operation ID, not a provider receipt.

Treat `dispatched` only as an owner dispatch acknowledgement. Require separate
source-backed challenge-clearance and protected-application admission verifiers.
A callback, disappeared widget, changed URL or vendor test token does not prove
protected-site access. Keep task completion separate.

On refusal, stale/ambiguous evidence, changed control or unverified access, use
`prepare({taskId})`, the same existing authenticated Native handoff and explicit
operator takeover/return. Use `recordTakeover` and `continueAfterReturn` to
observe that journey. Viewing alone is not takeover; do not add a viewer grant,
parallel viewer or guessed click. Native and cooperative control epochs are
different authorities.

The versioned OCR/locator fixtures qualify synthetic behavior, not vendor
accuracy or production clearance. Runtime evidence must name the actual
candidate, owner pins, input path, independent HTTPS oracle and exact cleanup.
