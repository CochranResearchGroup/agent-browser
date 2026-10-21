# P258 bounded interlock logging acceptance

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P258 / #294
Related lanes: P240, P257

## Behavior and evidence

Both scheduled unit producers now omit explicit JSON. Routine reconciliation
projects only sanitized state and summary fields, each capped at 256 UTF-8
bytes. Status and error output stay on one line below 1 KiB and identify
`agent-browser install doctor --json` as read-only recourse. Failure exit code
remains 1. Explicit `install workstation reconcile --json` retains full detail
and still performs maintenance. The private runtime-monitor receipt remains a
single latest-pass file, overwritten rather than archived. Maintenance, fencing,
cleanup authority, cadence and backoff are unchanged.

A fresh read-only pre-repair one-hour journal count observed 21,349 entries and
1,026,592 message bytes. This is a baseline, not installed after evidence.
The old generated-unit regression failed before implementation. Final focused
interlock tests passed 4/4 in 0.12s; monitor tests passed 3/3 and source-free
unit test passed 1/1. A 10,000-process synthetic report yielded a 128-byte,
one-line routine projection versus 819,024 bytes / 40,008 lines of explicit
JSON; the JSON round-tripped without loss. Hostile multiline/control fields
remain bounded. The isolated public command failure test preserves exit 1,
actionable routine stderr and full explicit JSON errors even with ambient
`AGENT_BROWSER_JSON=1`.

The first harness compile used an incorrect reference comparison, fixed before
the real red. An initial JSON assertion assumed error wording; it was corrected
to verify the complete fixture path rather than a brittle phrase. Final results
above follow those corrections.

## Validation and limits

Cargo-safe formatting and strict workspace Clippy passed. Docs build, policy
wiring (48 identities / 11 tests), documentation links, service/API/MCP parity,
Remote View handoff docs, shell syntax, patch hygiene and P258 planning audit
passed. Standards review found bounded allocation, UTF-8 safety and unchanged
effect ownership. Spec review checked both unit producers, full opt-in detail,
error status and no automatic diagnostic maintenance. No unresolved findings.

The broad selector also recommends installer/provisioning/database fixtures
from the shared installer file. Those suites were not run: this slice changes
output projection and unit command strings, with direct focused coverage; it
does not change provisioning, lifecycle or database behavior. No installed
service, production binary, shared skill, browser, provider or CI was changed.
Actual timer journal reduction requires separate installed qualification.
