# Native handoff current-assignment repair

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301)
Related lanes: P237, P240

## Bounded outcome

The user authorized repair of the remaining native handoff issuance failure and
qualification of an updated combined candidate. Existing UX301 primary and PR302
retain custody. This follow-up owns only current assignment selection in
`remote_view_session_effects.rs`, its actual host/SQLite regression, guidance and
synthetic development acceptance. Production installation, provider replacement,
ingress changes and private account authentication are excluded. Ceiling:
60 minutes including one qualification build and two product-repair cycles.

## Reproduction and repair

The exact issuance seam, `resolve_tab_handoff_view`, fails with
`remote_view_tab_view_assignment_unavailable` when inventory contains one active
assignment and released history for the same desktop generation. The minimized
regression fails with one released record; it also retains the original three
released-record shape. Existing published browser custody is valid throughout.
The selector incorrectly required uniqueness across all assignment states.
It now considers only active assignments before the existing unique-current and
published-custody checks. No history is deleted, viewer contract changed or
retention deadline renewed.

The initial test used direct native-desktop resolution, which already succeeds
with released history and follows a different path. That wrong-seam failure is
retained separately from the actual issuance RED. Green exercises the repaired
issuance path across host restart, exact target readiness, rejection of multiple
active assignments, rejection of released-only history, recovery after restoring
the original assignment, stable grant identity and no extra launch/navigation.
Existing issuance tests retain the published-custody mismatch rejection.

## Source validation

The nine-check batch passes: 18 selected Rust tests across native handoff,
issuance, published-view resolution and three service-model suites; Cargo format,
strict workspace Clippy, handoff guidance, policy wiring and docs build. The
[prior complete PR review](2026-10-17-pr302-review.md) and rail/browser/client gates
remain applicable to their unchanged surfaces. Primary bounded Standards review
finds no new violation; Spec review finds no remaining defect in the active-state
filter or retained identity checks. Installed qualification remains separate.

## Development evidence boundary

Fresh default development readback has zero managed browsers/sessions/tabs and
zero native windows. Its existing Remote View application has one active and
twelve released assignments, allowing actual-provider checks without private
account browsers or provider configuration changes. All generated state and raw
receipts remain private.

Initial CLI probes used ordinary open followed by incomplete handoff parameters;
they stopped at missing sessionName/url validation and do not reproduce issuance.
The documented single remote-view open URL plus session-name command succeeds on
the earlier installed build, consistent with the direct native path above. Each
exact synthetic task was explicitly closed. An HTTP preparation probe stopped at
400 before qualification and is not accepted as native-operation proof.

## Updated candidate and installed qualification

Clean source `7f9cff6e06facb75c3662bb92f15f33485797214` produced sealed
candidate `candidate-abe6e80e41677b6f-7b46a43fffe577e2`, binary SHA256
`7b46a43fffe577e209c99fa079ae801abbca052f8f746381a922f936b42a53fd`.
An independent fresh process returned `reused_sealed`, and the exact executable
reports version 0.28.0. The earlier candidate remains retained.

Only default development was updated to generation `0.28.0-7b46a43fffe5`.
The publisher asserted production unchanged. Development doctor passes, and the
actual required native provider doctor passes. Native provider executable SHA256
`8f73d0ded3042a43546f1043caaac5a5e38c54c051efc4ce2d1db07e4151f613`
is unchanged from the previously accepted native viewer implementation.
Provider configuration, ingress and account authentication were not changed.
The mandatory development browser-launch smoke passes all three disposable
open/read/close/residue cycles after the native fixture has closed.

The actual development inventory's twelve released assignments are from older
generations. It does not reproduce the same-generation issuance collision. The
host/SQLite regression supplies that exact RED/GREEN; direct native preparation
already worked on the earlier installed build with this actual inventory.

Two task-owned synthetic pages share one browser. Actual native preparation
passes. Authenticated local dashboard focus switches Alpha, Beta, then Alpha;
independent CDP reads prove selected visibility and peer hiding. Native X11
keyboard input reaches each selected page, and both values survive switching.
Closing Alpha preserves Beta's value. Both exact tasks close successfully;
captured PID/start identities and profile-specific process census show no residue.
These are actual owner/provider/X11 observations, not fresh public authenticated
viewer or WebSocket-frame acceptance. Synthetic screenshots and raw receipts are
private until curated publication.

## Blocking rail qualification finding

Combined acceptance fails. The initial immediate presence check and three fresh
owner observations all report `unknown` for the demonstrably live browser and
both tabs. A separate selection/input run preserves that failure explicitly as
`railPresenceLive: false`; its native success does not override the rail failure.

A diagnostic run measures the actual provider inventory at 1024 ms and native
windows at 1430 ms, with exact browser PID in both CDP process information and
the provider window. Assignment observation takes 1.3 ms. The collector gives
each provider HTTP request only 100 ms and stops admitting desktops after
450 ms. The actual provider cannot meet those bounds, explaining omitted native
results and unknown presence. This is a source-to-provider timing mismatch,
not missing browser lifetime accounting and not an authentication failure.

No production installation or overall acceptance is qualified. The one permitted
qualification build is consumed. The next source repair should decouple bounded
native observation from dashboard request latency, retain single-flight work and
fair desktop scheduling, and attach the actual observation age to cached results.
Expired, failed, ambiguous or not-yet-completed evidence must remain unknown;
positive absence must still require a successful exact read. Do not merely raise
the synchronous timeout or relabel unknown as live. Regression acceptance must
include a provider taking longer than 100 ms, a fast dashboard response, eventual
live convergence, stale-result expiry, failure invalidation and no observation
effects. Rebuild and repeat combined development acceptance after that repair.

