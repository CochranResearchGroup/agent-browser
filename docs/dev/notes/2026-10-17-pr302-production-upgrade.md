# Combined dashboard production upgrade

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: [issue 301](https://github.com/CochranResearchGroup/agent-browser/issues/301), UX301 / PR302
Related lanes: P237, P240

## Authorized outcome and bounds

The user explicitly requested upgrade after the final-source development replay.
Build and qualify one final production-shaped candidate, integrate PR302 through
canonical main, then apply the exact sealed bytes through the existing governed
workstation transaction. Preserve retained browsers, profiles, handoffs and native
Remote View ownership. No account authentication, provider replacement, ingress
configuration change, formal release, CI dispatch or broad process cleanup.

This is a new explicit production-promotion milestone. Its finite allowance is
60 minutes, one production-shaped build and one committing install transaction.
At most two bounded recovery/repair cycles may follow a typed failure; resume the
same durable operation rather than starting competing installs. No force
browserless or full-shutdown policy is selected. Existing source validation and
ordinary development acceptance are reused only for unchanged executable inputs.

## Starting evidence

Source `d1ac14e74` contains the reviewed combined dashboard, handoff history, reaper
isolation and bounded native observation/sample-age repairs. The
[diagnosis receipt](2026-10-17-rail-native-observation-diagnosis.md) binds source
checks and actual ordinary development native/rail workflows. All ten focused
workspace Rust tests, strict quality and browser/client/schema/UI/docs checks
pass. Development is generation `0.28.0-1d085fb275be`. Whole summary latency remains
approximately three seconds; upgrade does not promise an additional latency fix.
The existing production-shaped `7b46` candidate predates the native worker and must
not be promoted for this outcome.

PR302 is open, mergeable and clean against freshly fetched main `8bea10f03`. Build,
sealed qualification, canonical ancestry and production transaction evidence are
pending. Raw production state and exact process/profile evidence remain private.
