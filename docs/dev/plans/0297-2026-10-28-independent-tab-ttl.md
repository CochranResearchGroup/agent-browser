# Plan0297: Independent task tab expiry

State: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#490
Related intake: CochranResearchGroup/agent-browser#488
Branch: fix/issue488-tab-ttl
Target: main
Base: 7d9b4b7ad41114f313b4329743904b9181b10b29

## Current State

The operator-authorized ChatGPT task accepted a thirty-second per-tab lifetime,
had no pending-human hold and retained an unchanged activity timestamp, but
remained live throughout a ninety-four-second passive observation. AuraCall
later attempted closure under its independent five-minute deadline. That later
disappearance does not qualify Agent Browser automatic expiry.

Two bounded periodic fd2 observations captured `browser_session_idle_custody_unavailable`. The configured native pool adapter required a published Agent Browser launch assignment for a borrowed browser that has no Native desktop and is positively marked shared. Source provenance ties the installed binary to PR455's merge. The periodic
reaper, host and manager source match this plan's base. No runtime replacement
is authorized by this worker packet without coordination with the primary.

## Bounded outcome

An eligible expired task tab retires through the normal owner reaper, preserves
protected peers and shared browser process custody, and publishes truthful
terminal lifecycle state. An uncertain independent browser must remain deferred
without starving the eligible tab. CI is excluded.

Test the public session-manager reaper with controlled time and boundary effects;
then the existing native-host fixture for publication. Record each red and green
command. Do not add speculative cleanup authority or infer live ownership from
the absence of a Service State row. Installed acceptance must exclude competing
AuraCall cleanup and remains the primary's coordinated integration gate.

## Execution ledger

- Source qualification: validated at the recorded base.
- Regression: two red-green loops at the public manager seam. A failed older browser observation cannot starve another task; the composed native-pool adapter reproduces the exact borrowed-browser custody error and preserves a blocked service guard.
- Implementation: exact shared/no-Native-desktop tasks retain local service activity exclusion without claiming native launch authority. A mismatched browser descriptor or genuinely owned native scope still requires custody. Independent browser observation errors remain visible after successful peers publish.
- Model validation: all 396 service-model checks passed; native host, format, Clippy and installed gates pending.
- Integration: pending.
- Installed acceptance: pending; preserve the production host and all peers.
- Review: the primary owns one broad Standards/Spec review after ticket validation
  and one bounded TDD remediation pass; this worker does not start another review.

The clean, fully integrated former P240 checkout is reused. Its old branch and
all candidate custody remain preserved; no additional worktree is created.

## Source validation receipts

The public manager regression for an uncertain older browser first failed with
`synthetic_owner_observation_unavailable` and an unretired peer. Per-browser
qualification now preserves that scope, records its typed error and permits
independent expiry. The real composed pool-effects regression then failed with
`browser_session_idle_custody_unavailable` for the borrowed task itself. The exact
shared/no-desktop branch fixes that case while retaining the local denial case.
Both red and green invocations use the existing service-model integration runner,
with the named `uncertain_browser_observation_does_not_starve_another_tabs_lifetime`
and `borrowed_tab_lifetime_does_not_require_native_browser_launch_custody` filters.
All 396 service-model checks and documentation parity checks passed. Workspace
Rust format and strict Clippy passed. Native pool-host tests and installed
acceptance are pending at this custody checkpoint. A stalled shared-cache
transport was bypassed through the supported `AGENT_BROWSER_CARGO_CACHE=off`
setting; no peer cache server or production process was restarted.
