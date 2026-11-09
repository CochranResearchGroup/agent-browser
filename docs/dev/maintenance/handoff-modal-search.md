# Compact handoff discovery

Product lane: PL-BUGFIX
Work item:603
Branch: fix/handoff-modal-search
State: INSTALLED_UI_ACCEPTED

Reuse the clean canonical checkout from origin/main a8a8f1051. The prior
manual recovery source and its sealed candidate remain retained; no checkout
removal, candidate replacement or production installation is part of this work.
No new worktree. Primary writer: handoff modal, its styling and existing browser
fixture. Existing lanes own runtime recovery and their isolated checkouts.

Outcome: compact handoff rows with full context behind Inspect, plus instant
case-insensitive multiword filtering over task/profile/session/creator/link
context. Preserve inactive toggle, sorting, refresh, copy/open, deletion and
policy editing. Validate search, no-match, clear, hidden metadata and inactive
composition in the existing real-browser fixture, plus responsive rendering
and dashboard build. Update public guidance. One bounded implementation/review
cycle;30-minute target. Run local checks only; no hosted CI or live effects.

## Qualification and review

The modal is capped at920px with compact task/profile, date, link-state and
accessible action rows. Full creator, session, lifetime and identity fields
remain under Inspect. Search filters each input change locally, case-insensitively,
with multiword matching across visible and collapsed metadata. Clearing restores
focus and results. Search composes with inactive visibility and sorting. Counts
appear only when inventory is available. Phone/tablet layouts keep actions beside
status and avoid vertical letter wrapping.

Passing local checks: dashboard production build, docs production build,
TypeScript, workspace strict Clippy and Rustfmt, policy wiring, changed links,
handoff docs contract, browser-table and workspace-navigator contracts. The
existing real-Chromium handoff-context, complete compact-workspace and explicit
refresh fixtures pass. Cases cover incremental typing, case/whitespace/multiword
search, hidden creator/session metadata, link IDs, no results, clear/focus,
inactive composition, inspection, sorting, copy/open, policy edits and deletion.
Screenshots at1280,768 and390px were reviewed. The tablet review found cramped
headers; the final layout and header-height assertion qualify the correction.

The inspector-actions check initially failed because its parent-source regex
still expected a Diagnostics link that already moved into WorkspaceLibraryDialog.
No application routing changed here. The corrected check proves parent route
forwarding and the child's ordinary diagnostics anchor; the check passes. Both
initial fixture failures remain in private diagnostics. This is not a product
regression attributed to the modal.

Standards review: no blocking finding. Existing Radix dialog, accessible controls,
durable-link validation, refresh and action owners are preserved. Spec review:
both requested improvements are implemented without provider or browser effects.
The only Rust change is explanatory help prose; strict Rust gates cover it.
Installer behavior and service models are unchanged, so broad installer/model
suites are not repeated for this UI source slice. No hosted CI, installed skill
publication, new installed binary, or production acceptance is claimed. The
previous sealed recovery candidate remains intact and separate from this UI.

## Production deployment,2026-11-09

The operator explicitly requested deployment after source integration, authorizing
this preserving production operation beyond the earlier source-only packet.
Production-shaped source7ac7f95ca was sealed as
candidate-1e1b2414fdd0cb7a-39a84cdf7d4858da, SHA-256
39a84cdf7d4858daaf6f85b413728446c563761faf99d6a02dc6d93ac178e5d1.
Full release build completed in26m32s. Its exact install fixture and six no-launch
service checks pass, with two fixture-owned hosts retired and zero residue.
The canonical preserving installation is accepted and selects generation
0.28.0-39a84cdf7d48-4b6e06404c9a. Installed doctor passes. Public authenticated
runtime-manifest hash matches those exact bytes. The live modal has search;
filtering an existing link, clearing, no-result feedback and the separate
anonymous sign-in guard all pass. Temporary QA browser roots exit. The new
modal usage paragraph was published to both installed skill roots while
preserving their other existing text; private backups are retained.

The independent process-continuity axis is qualified separately: six of seven
original managed Chrome roots retain exact PID/start/executable identity. One
disposable root is absent afterward; its profile directory and task record
remain. The journal proves it survived unit stop and was still observed during
startup. Its later closure cause is unconfirmed.605 tracks this observation;
no installer causality, full preservation, or data-loss claim is inferred.
No full-shutdown option, profile reset or manual process cleanup was applied.

The older research handoff initially reproduced its exact unavailable page after
installation. An explicit authenticated owner resolve then returned opened and
operatorVisible.state=ready; the same durable URL subsequently rendered a Remote
View canvas.599 remains open for automatic first-open recovery, which that
owner-assisted result does not qualify. No website action was replayed.
Private user/runtime state, browser images, auth data and process evidence stay
outside Git. This receipt qualifies the deployed modal, not every browser workflow.
