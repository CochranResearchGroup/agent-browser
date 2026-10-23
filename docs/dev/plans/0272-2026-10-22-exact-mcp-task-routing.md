# P272: preserve exact MCP task routing

Status: CLOSED
Product lane: PL-BUGFIX
Work items: CochranResearchGroup/agent-browser#365
Branch: fix/mcp-task-routing-365
Owner: ecochran76

## Current State

PR370 is integrated. The sealed release from source `16450473a` is installed as
`0.28.0-ff0b22c3bb93-e5a1b9f945e7`; the preserve transaction is accepted. Installed
doctor passes, both pre-existing browser-root identities survive, and the original
open/snapshot/continue/scoped-close cycle passes over fresh installed MCP stdio.
See the [installed qualification](../notes/0272-2026-10-22-exact-mcp-task-routing-installed.md).

On2026-10-23 the configured Codex MCP connection works again. The operator-invoked
Cooper ingress skill's existing agent account clears Authelia; authenticated
AuraCall client and inventory200, streamed views and scoped task cleanup are
verified. The remaining Agent Browser gate is satisfied. AuraCall #240 stays
separate: Writing is empty, and intended physical root/input proof is incomplete.
No new marker preparation, ownership rebind or provider input was used.

## Outcome and acceptance

Preserve the returned task ID through public page adapters and workflow reads.
The host must select that exact durable session and reject missing or conflicting
identity without launching a fallback browser. Name-only callers retain their
existing ambiguity refusal. Persistent profiles and peer tasks remain intact.

Public stdio regression uses isolated SQLite and a synthetic Unix transport
with two same-name tasks. Provider-free host composition verifies exact target
selection and rejection. Run focused Rust/MCP tests, format, strict workspace
Clippy, and relevant policy/documentation checks. Preserve red/green evidence.

## Custody and boundaries

Reuse the clean canonical checkout on a new branch from origin/main 8dcfb37bb;
preserve unmerged fix/mcp-manual-login-357 and three other registered checkouts.
Expected writes: MCP adapters/workflow, managed host dispatch, regression fixtures
and these source-contract notes. No provider probes, production replacement,
deployment, broad cleanup or memory writes. Canonical issue remains the intake
authority; source validation is separate from installed desktop acceptance.

## Validation evidence

The public fixture against the installed CLI fails with
`browser_session_name_ambiguous:workflow-duplicate`; the patched debug CLI passes
both IDs across snapshot, navigate, click, fill, screenshot, advanced URL and
advanced evaluation dispatch. A missing ID is rejected before any transport send.
Run it with `python3 scripts/test-mcp-task-routing.py cli/target/debug/agent-browser`.

The real host/SQLite composition regression failed before the host fix with
`browser_session_name_ambiguous:Alice`; after repair it selects each exact target,
retains name-only ambiguity refusal and rejects missing IDs or conflicting names
and browser profiles. The tab-creation branch is covered in the same fixture.

Focused evidence: 138 MCP tests pass. Across the broader consumer selection,
116 pass and one opt-in Native viewer fixture is ignored. The initial raw Cargo
run omitted the required 16 MiB stack and aborted at the queued-action test;
98 prior passes and one ignore are retained, and the failed/unexecuted 18 cases
pass with the repository-required stack. This was a harness correction, not a
product retry. The final tab assertion changes only the regression test, so
unmodified implementation coverage is retained and that test is rerun through
`rust-tests.sh --focused consumer_exact_task_route`.

Strict workspace Clippy, format, policy wiring, documentation links, Remote View
handoff guidance and the service collection no-launch smoke pass. The first build
attempt stalled in sccache; its owned Cargo process was stopped and compilation
completed with cache disabled under unchanged resource limits. A concurrent
Clippy attempt was deferred by memory admission, then completed after release.
These source checks alone do not establish installed acceptance; the installed
qualification above records the later bounded runtime result.

## Authorized installed continuation

The operator directed continuation after PR370 source delivery. Integrate current
main by preserving both source intents, then build and seal one production-shaped
candidate, qualify its exact bytes in isolated fixtures, and install through the
preserve-mode workstation transaction. Reuse native configuration and retain peer
browser/profile custody. Do not select force-browserless, full-shutdown, provider
replacement, account authentication or unrelated process cleanup. Verify selected
binary, live host, component convergence, doctor and one original exact-task
open/snapshot/scoped-close cycle. Keep AuraCall authenticated desktop acceptance
separate from a successful browser inspection transport.

Main already reserved P271 for installer component convergence (#367/PR368), so
this plan and lane become P272 without changing issue/branch identity. Main also
contains the shared-host connection repair from PR369. Revalidate the MCP,
connection and exact task seams plus strict Rust quality before completing the
merge. Historical source-only results remain bounded to their original code.
