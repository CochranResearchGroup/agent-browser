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

Updated candidate build and exact development/native workflow results are pending.
The earlier sealed candidate remains retained and is not the repaired executable.
