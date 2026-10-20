# P256: absent-browser stale workflow recovery

Status: CLOSED
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#327
Branch: fix/absent-browser-workflow
Owner: ecochran76

## Outcome and bounds

Remove the stale-daemon/handoff-no-candidate recovery dead end for an existing profile whose browser is positively absent. Preserve profile files, history, peers, and unknown or unpublished browser-effect holds. Reuse existing runtime ownership and fencing boundaries. No account access, production installation, process termination, runtime reset or broad cleanup.

## Custody

Reuse the clean dashboard primary at fetched canonical `3ca65ae38`; preserve root stale-session and resource-bounds worktrees and pinned candidates. P249 host snapshot work is integrated and owns a different seam; this writer owns connection refresh and absent-browser handoff recovery only. #307 retirement lock work stays separate. No auxiliary checkout or delegated work.

## Feedback and validation

Build a provider-free regression exercising stale authenticated transport, no-browser handoff and the resulting recovery recommendation. Keep live/unknown browser and pending-effect refusal tests. Match validation to changed Rust and documentation surfaces, including cargo-safe formatting and strict workspace Clippy. Record actual source qualification separately from installed behavior.

## Result and acceptance

The [source receipt](../notes/0256-2026-10-20-absent-browser-workflow.md) records the red/green native-handler regression, stale transport and legacy output checks, retained handoff bytes and platform-aware read-only recourse. This delivers an executable supported recovery path, not automatic browserless host replacement. Production remains unchanged. Final connection (47), preparation (10), targeted regressions (3), strict workspace Clippy, formatting, docs build, policy, parity and links pass. The broad handoff run passed 190 with one disposable-Chrome test ignored; the separate interrupted installer fixture is explicitly incomplete.
