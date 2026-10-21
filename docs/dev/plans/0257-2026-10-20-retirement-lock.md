# P257: retirement lock and recovery diagnostics

Status: OPEN
Product lane: PL-BUGFIX
Work item: CochranResearchGroup/agent-browser#307
Branch: fix/retirement-lock-diagnostics
Owner: ecochran76

## Outcome and bounds

Keep diagnostic reads available during retirement effects, preserve retirement
fencing and browser/profile/tab custody, and report current lock duration with
verified owner attribution. Recovery must return usable state or an actionable
bounded refusal. No production installation, restart, termination, account
access or broad runtime cleanup.

## Custody and validation

Reuse the clean dashboard primary from fetched main `bca063738`; preserve the
foreign root and resource-bounds worktrees and candidates. P249 and P256 are
integrated dependencies; this writer owns retirement repository admission and
lock diagnostics only. No auxiliary checkout or delegated work.

First reproduce blocked reads with an acquired retirement guard and assert
independent controller writes remain fenced. Qualify transaction recovery,
duration reporting and existing handoff preservation through focused
provider-free tests, formatting and strict workspace Clippy. Review Standards
and issue acceptance separately. Source integration does not prove deployment.
