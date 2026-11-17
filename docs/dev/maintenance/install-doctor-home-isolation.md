# Install doctor service home isolation

Status: OPEN
Product lane: PL-BUGFIX
Branch: fix/install-doctor-home-isolation
Owner: current Contacts release operator
Issue: CochranResearchGroup/agent-browser#734
Review base: ed62bb3a30803b5ebf0c50427d99c308f04cdb0e

## Current state

PR727 is merged and its sealed production binary is selected. The preserving transaction remains forward recovery required because the no-launch status probe exceeds its15second deadline. All14 original live managed browser processes survived transfer. Source scope fix retains selected-profile checks and removes peer observations; its new installed acquisition upper98.042seconds did not improve the previous76.833second baseline. Initial profile upper40.311seconds passes60seconds. Four profile categories and10connections observed; network remains partial. Numerical comparisons are qualified by Stable Time and are single observations, not a benchmark distribution.

## Bounded repair

The status probe sets AGENT_BROWSER_HOME, but native Service State uses the operating-system home. Give only status and its matching close child the private HOME/USERPROFILE and matching .agent-browser directory; inspect that exact state path. Parent production identity, existing timeout, readiness rules and active browsers remain unchanged. Direct unchanged child returned successfully after upper25.181seconds; private operating-system home returned in upper9.626seconds. Raw receipts stay in private tenant storage.

No additional checkout or delegated lane. Sole writer owns install.rs; retain PR727 sealed artifact and existing transaction. Scope excludes browser resets, source-feature reduction, arbitrary doctor exemptions and unrelated cleanup. Local checks satisfy hosted CI per operator policy.

## Acceptance

Formatting and strict workspace lint; serial Standards/Spec review; exact sealed production installer fixture; preserving forward transaction recovery and original live UI Enrich acceptance. No new fixture framework or mirror test. Existing fixture executes the real doctor subprocess; direct before/after probes exercise the installed seam. Keep the original failure visible in final evidence.

## Standards

Serial review against AGENTS, CONTEXT, product contract, shared runtime custody and testing policy: no accepted findings. Two identical child environment additions preserve the existing command shape rather than add a new configuration abstraction. Existing production timeout and requirements remain binding. Parent environment is unchanged.

## Spec

Serial review against issue734: both status and matching close use the actual private home, and state-path observation matches native Service State. No doctor readiness bypass. Exact production fixture and installed recovery still required; no installed completion claim.

Local strict workspace Clippy passes at the reviewed source diff. Formatting check initially identified one wrapping difference, corrected before candidate seal. Hosted CI omitted under operator cost policy. Cargo uses2jobs,4GiB claim,16GiB reserve and8GiB disk floor for this incremental correction; original default20GiB floor reported disk pressure with roughly14GiB available. No cleanup or reserve reduction performed.
