# P282 recovery dashboard identity

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P282 / CochranResearchGroup/agent-browser#405
Related lanes: P280
Work item: CochranResearchGroup/agent-browser#405

A newer recovery controller rejected the selected older dashboard with `current_selection_dashboard_identity_unproven`. Live readback showed that the registered manifest matched the serving dashboard and selected sealed executable. The expected-manifest helper instead combined the controller's own assets with the older executable identity.

Current-selection proof now reuses the existing observed-backend adapter. It binds the served manifest to the sealed selected executable path and checksum, compares the registered digest, and retains the second backend probe plus generation, presentation, runtime-host process/start, socket and checkpoint requirements. The incorrect production synthesis helper is removed; fixture-only synthesis stays under test configuration.

The differing-assets regression failed first with the exact production error. All five current-selection tests then passed, including stale registered digest, wrong executable path/checksum, checkpoint lineage, absent versus invalid presentation, host identity and unrelated terminal history. Strict workspace Clippy, final format verification and patch hygiene pass. The previously qualified combined-source suites remain evidence only for their unchanged surfaces. No comprehensive or target-platform result is claimed.

The sealed original combined candidate is preserved. A repaired production-shaped candidate, full installer fixture, exact retained-operation recovery, coordinated preserve-mode installation and public workflow acceptance remain separate gates. No ingress/provider reconfiguration, private account activity, foreign cleanup or formal release is part of this repair.
