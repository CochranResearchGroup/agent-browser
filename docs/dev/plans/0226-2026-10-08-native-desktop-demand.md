# P226 Native desktop demand

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/229
Branch: fix/native-desktop-demand
Status: IN_PROGRESS

## Outcome

Independent browser launches acquire a free native desktop on demand up to provider policy. Existing sessions sharing one profile retain their browser and desktop. Preserve retained browser work and opaque handoffs. Ten total configured desktops includes the existing direct-use desktop, leaving nine managed desktops.

## Scope and proof

Use the existing pool preparation and durable acquisition seam. DESKTOP_COUNT remains a warm minimum, not an eager upper limit. Current provider policy supplies the hard maximum. Unknown occupied/unavailable assignments cannot justify desktop reuse. At capacity report actionable exhaustion. Reconcile provider unit drift separately through exact owned assets. Reboot is deferred.

Regression seam: real SQLite request custody with public provider transport, followed by host/profile reuse tests. Run red/green focused tests, format, strict workspace Clippy, docs parity and isolated browser placement before canonical integration and authorized production publication. Live proof requires two distinct browser desktop IDs and ready durable handoffs; clean up only task-owned probes.

Reuse the clean p204 checkout. Preserve pinned prior candidate artifacts. Older PRs 191/184 and other dirty checkouts remain outside this repair.

## Validation receipts

Baseline: 67c02b7a7fea771d416ce4a11d572cb9669f64f6.

Red: `AGENT_BROWSER_CARGO_CACHE=off scripts/ci/rust-tests.sh --focused pool_demand_acquires_a_free_desktop_for_an_independent_browser` failed because acquisition count remained 1 instead of 2. Green: the same regression and four existing pool-demand tests passed. The extended host/profile-reuse and capacity-exhaustion assertions are included in the final consumer run.

The first compile attempt stalled in sccache compiler-version discovery and was terminated through its exact task-owned systemd scope. The documented cache opt-out produced the red verdict.

Review uses the recorded batch baseline and issue 229 as the spec. Standards and Spec axes run separately in the primary context.

Private provider unit reconciliation verified every recorded file hash, retained the exact files and integration record, and recreated the installation-bound current units through the supported installer. Repeat install returned no_change. No desktop or application restart occurred during that reconciliation.
