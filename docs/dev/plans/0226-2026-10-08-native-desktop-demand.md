# P226 Native desktop demand

Product lane: PL-BUGFIX
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/229
Branch: fix/native-desktop-demand
Status: COMPLETED

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

## Installed follow-up

PR 230 entered canonical main and the sealed candidate passed three isolated cycles and installed doctor. The live second-browser request failed because new native desktops lacked viewer mappings. The original source allowed additional bounded acquisitions alongside unknown readbacks. Preserve that failed installed acceptance; it is not a pass.

Provider dependency: Remote View issue 334 / plan 0067 provisions a missing viewer mapping before managed readiness. The consumer follow-up stops growth when a new assignment has unknown window readiness. Red regression returned capacity_exhausted instead of assignment_unavailable and made another acquisition; the repaired behavior must stop at two. Branch: fix/native-desktop-readiness.

## Bounded installed acceptance

PRs 230 and 231 are merged. All 33 consumer tests, Service Model tests, format, strict workspace Clippy and documentation checks passed. The final sealed production-shaped candidate passed three isolated open/read/close/residue cycles, supported production installation and install doctor.

Four synthetic independent browsers opened on four distinct native desktops. Two authenticated opaque handoffs displayed distinct synthetic markers, reconnected through the same URLs after software adoption, and continued automation succeeded. All four synthetic browsers were closed. The isolated development services were stopped and disabled. Existing user sessions and profiles were preserved; reboot remained deferred.

Qualification is bounded: ten configured slots is not ten-browser simultaneous stress acceptance. Existing shared-profile browsers retain their placement. Remote View PR 337 is installed with passing runtime adoption and doctor; its plan 0067 records the accepted fresh missing-mapping loop and preserves the failed first shrink/restore fixture. Private raw runtime receipts remain outside product sources. Memory disposition: forbidden because the provider requires explicit memory-write authorization.

Final provider-dependent proof: after exact owner retirement and viewer projection reconciliation, a normal independent-browser request allocated a new lifecycle desktop whose native slot had no mapping. The installed provider automatically created the mapping and returned a ready handoff without a manual provision during that request. Five independent synthetic browsers were simultaneously ready on separate desktops. This is bounded workflow acceptance, not ten-browser stress qualification.
