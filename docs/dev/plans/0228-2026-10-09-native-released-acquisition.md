# P228 Native released acquisition regrowth

Date: 2026-10-09
Plan version: 2
State: CLOSED
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

Consumer PR 237 integrated the exact native Released-record repair. Installed acquisition acceptance passed on 2026-10-10: twelve disposable browsers ready, twelve independent profiles and URL/marker checks, ten native desktop assignments at the configured cap, and wraparound onto two occupied desktops. All twelve task closes passed; fresh process readback found no test browser residue and one warm assignment remained. Foreign development desktop process identities were preserved. Remote View selected resources without Agent Browser choosing a display.

The first stable repeat exposed a retained failed native reservation on an occupied development display. Exact native lifecycle reconciliation, drain and removal retired only that failed production reservation; the next repeat passed. Current Remote View already includes provider PR 348 collision admission and retired-reservation recovery. No additional source change, broad cleanup or reboot was needed. Earlier failed evidence remains retained outside the product repository.

The installed Agent Browser generation contains the PR 237 runtime change, and install doctor passed. The provider running and installed binary identities matched throughout the accepted repeat. Earlier focused regressions, full Service Model tests, formatting, strict workspace Clippy and documentation gates remain the source qualification; no Rust changed during acceptance closeout.

## Acceptance boundary

This closes acquisition regrowth, ten-desktop capacity and wraparound for synthetic browser workflows. Actual human handoff, durable storage across close/reopen, and production retention timing are separate lifecycle checks. The accepted pressure run does not establish external viewer pixels, private-site authentication or all browser workflows. The source branch remains retained; its clean checkout was removed after verified remote integration and sealed binary preservation.
