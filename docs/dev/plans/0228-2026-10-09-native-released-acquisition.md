# P228 Native released acquisition regrowth

Date: 2026-10-09
Plan version: 1
State: OPEN
Product lane: PL-BUGFIX
Owner: primary agent
Branch: fix/native-released-acquisition
Target: main
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/236
Authority: user authorized repair, installation and bounded pressure testing. Reboot deferred.

## Outcome

New browser demand can regrow after Remote View explicitly releases the exact prior assignment, without requiring Agent Browser to have initiated the release. Remote View continues to select native desktop resources. Immutable acquisition history stays intact; missing or mismatched native records and unresolved acquisitions still require readback.

## Scope and acceptance

Use a regression through the real SQLite acquisition store and provider fixture. Accept a released record only when assignment, registration, pool, desktop and generation match. Keep existing unknown-outcome and inventory-absence tests. Update command help, README, skill and Remote View documentation. Validate Rust formatting, strict workspace Clippy, pool/session and model tests before issue-linked PR integration.

Install the reviewed candidate through the supported guarded upgrade. Repeat the two-browser repro and twelve-browser synthetic pressure test under the configured ten-desktop cap and wraparound. Clean only test-owned browser and assignment resources; preserve foreign desktops and retain failed evidence.

## Current State

Remote View admission and exact failed-reservation recovery are integrated in provider PR 348 and installed. The two-browser repro passed. The first twelve-browser repeat failed after native test cleanup because Agent Browser required local release custody despite an explicit exact native released record. The consumer regression failed before repair and now passes. Pool-demand and broader consumer tests, the complete Service Model suite, strict workspace Clippy, format, handoff documentation checks, docs build and active planning audit pass. Source integration and installed pressure acceptance remain pending.
