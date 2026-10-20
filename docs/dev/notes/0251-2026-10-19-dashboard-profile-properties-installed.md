# P251 installed Properties and profile retention

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: [P251](../plans/0251-2026-10-19-dashboard-profile-properties.md), CochranResearchGroup/agent-browser#323
Related lanes: P250, P248, UX301

## Delivered workflow

The operator can inspect recorded task context and creation time, distinguish Throwaway and Durable profiles, edit the profile display name, and promote a throwaway profile from Tab properties. Names and retention change without relocating profile data or changing stable profile IDs. Missing historical attribution is explained rather than fabricated.

Source PR324 merged as `fc339cb0559a6bbfda29ac8aeac676241f11af5d`. The sealed source commit `d8b88df427d8990a568ff12c6ad1dd9b2046c698` is in canonical ancestry. All 543 executable inputs remained equivalent through installation.

## Installed identity

Candidate: `candidate-8d75852291b53b25-4e5a65f7463ef3de`.

Binary SHA256: `4e5a65f7463ef3de981ec07e766a7cd5f5ec7198adc3fc6c0d47a9f6a9278c85`.

Embedded dashboard SHA256: `1bb7c989b3760d6940e1f243f64996f1d485b6fbe8dce59668a32ef4243c0a6f`.

Production transaction: `upgrade-3f7347ec-6f0b-46f1-8855-e4e6e0f27a8c`, accepted with ready install result. The exact sealed bytes were qualified in the isolated development runtime before production installation. This is a local roadmap checkpoint, not a formal release.

## Acceptance and limits

Both install doctors pass. Three disposable development browser open, URL-read, close and residue cycles pass. Actual public production Properties opens with editable profile name, known creation time and retention status. The display-only profile map contains 14 referenced records, with only ID, name and kind; malformed authenticated PATCH requests return 400. Existing business profiles were neither renamed nor promoted during QA. Rename, promotion, stale-writer, cleanup and pending-custody semantics are covered by the previously qualified Rust and mounted UI regressions documented in the [plan](../plans/0251-2026-10-19-dashboard-profile-properties.md).

The first Properties probe checked before rail loading completed. Waiting for an actual Tab actions control fixes the harness timing; the installed workflow then passes. An expired diagnostic Authelia session initially redirected the Native frame. Refreshing that existing diagnostic session yields an embedded Native WebSocket connection with a 1472 by 960 canvas. Human-use acknowledgement was withheld by the QA harness.

Before/after readback preserves all 68 observed Native process births, desktop generations, handoff IDs, active profile IDs and protected configuration hashes. The numeric-PID browser-record check decreases from six to five: one recorded browser PID is absent afterward. The upgrade census also contains a QBO runtime entry classified as metadata without a live runtime; that observation does not establish the identity or exit timing of the numeric PID. The retained browser record and profile ID remain. The precheck did not capture executable identity, so it cannot prove that every numeric PID was a browser or establish when or why that process exited. Full browser-process continuity is not claimed. No QA browser closure or business-profile recovery was requested or performed.

Private screenshots, raw workspace snapshots, install reports, transaction and validation logs remain in user-scoped state outside Git. The prior shared ingress incident was recovered before this installation; no Docker/WSL restart or Native desktop restart was performed by this lane. The existing workflow-skill size gate remains tracked separately in #325; CI stays disabled.
