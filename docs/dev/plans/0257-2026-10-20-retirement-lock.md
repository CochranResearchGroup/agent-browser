# P257: retirement lock and recovery diagnostics

Status: CLOSED
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

## Result

The [source acceptance](../notes/0257-2026-10-20-retirement-lock.md) records the
red/green regression, 49 passing repository tests, 58 passing retirement tests
with one real-Chrome case ignored, three retained browserless recovery checks,
strict Clippy and documentation gates. Separate retirement serialization keeps
diagnostic reads available while controller and handoff writes remain fenced.
Bounded refusal supplies executable read-only recourse; current Unix hold
duration advances without sidecar updates. Standards and Spec have no blocking
findings. Following explicit operator deployment direction, the
[installed checkpoint](../notes/0257-2026-10-20-retirement-lock-installed.md)
records accepted production generation `9c8edd62a16f`, development and public
qualification, and exact preservation limits.
