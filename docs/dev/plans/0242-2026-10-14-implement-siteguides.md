# P242: Implement shared website automation spec271

Product lane: PL-RECIPES
Work items: #199, #271, #272, #273, #274, #275, #276
Branch: recipes/siteguides-271
Baseline: cb765a7be2719383a54fb849eac9691ce0197b06
State: IN_PROGRESS

## Objective and scope

Implement the full approved spec271, all five vertical slices, on one validated
integration branch. Public supported client workflows, owned synthetic websites,
shared SiteGuide/routine/playbook results and deterministic/agent continuation
are the acceptance seam. No production deployment, real credentials/account
login, foreign source edits, external messages or reboot. Retain the accepted
prototype branch d236a8802 outside main.

Execution started 2026-10-14T13:25:26+00:00; stop before 2026-10-14T19:25:26+00:00 or
2,000,000 goal tokens, whichever comes first. Check goal accounting at each
slice boundary and preserve the full objective on interruption. No narrowing
completion to source tests, advertisements or fixture-only simulations.

## Custody and frozen review point

Reuse the clean primary checkout from fetched origin/main. Preserve foreign
P237 dashboard and enrollment worktrees and all candidates. P242 owns new
playbook adapters, bounded MCP/catalog extensions, matching public client,
CLI/help/README/skills/MDX/docs and synthetic fixtures. P237 retains dashboard
and browser/tab/handoff lifecycle. Consume its existing task interfaces.

The review fixed point is cb765a7be2719383a54fb849eac9691ce0197b06, recorded under operator-authorized implement-spec
execution. One broad Standards/Spec review and one bounded remediation pass.
Spec271 and ticket272–276 bodies are authoritative; source research and the
accepted prototype are locators and design references, not runtime proof.

## Resumable ledger

| Ticket | Blocked by | State | Primary ownership | Evidence |
| --- | --- | --- | --- | --- |
| #272 BILL existing-session verification | none | validated | primary, public playbook/MCP/client adapters | installed generation 0.28.0-bf895a5c9b61; receipts below |
| #273 BILL helper login/MFA | #272 | pending | primary, auth-helper adapter | helper discovery below |
| #274 QBO same contracts | #273 | pending | primary, QBO guide and fixtures | none |
| #275 guide development/adaptation | #274 | pending | primary, guide tooling/skills/client acceptance | none |
| #276 Reddit retrieval | #275 | pending | primary, retrieval guide/routine/fixtures | none |

Readiness requires the predecessor validated on this branch, not issue closure.
For every slice retain command, result, exact source/binary and installed identity.
Whole-spec completion requires all tickets validated and accepted review blockers
resolved. Commit/tracker integration and live publication are separate states.

## Dependency discovery

Read-only worker /root/auth_helper_contract returned source-pinned findings.
grok-bots PR34 is draft OPEN at cbe071a0073bbac177fefa125747d5aa06c57c03;
its active source work is foreign and dirty. The current public helper supports
request_login, inspect_request, authenticate plus operator request management.
It has no MFA continuation, task-target registration or installed readiness.
Current receiving interface is trusted service-side injection; capture privacy
only applies to readers wired to that helper. Adapter design must expose missing
capabilities and preserve unknown effect custody. No invented integrated MFA or
private-capture qualification may be claimed from a synthetic double.

Source and synthetic work may continue within the approved acceptance contract.
Missing provider capabilities become named dependencies, not a new credential
store or fake production success. Qualification distinguishes public protocol
fixtures, actual source-helper dummy receiving, installed helper and real account.

## Ticket272 validation receipt

Focused source gates: cargo-safe fmt passed; strict workspace Clippy passed with
isolated CARGO_TARGET_DIR=cli/target/p242-quality; focused mcp::tests passed
132 tests. Client checks, strict new-module JavaScript typing, docs build,
planning audit, changed documentation links and patch hygiene passed.
Receipts: /tmp/p242-fmt.log, /tmp/p242-clippy-isolated.log,
/tmp/p242-mcp-tests.log, /tmp/p242-client-tests.log,
/tmp/p242-playbook-types-fixed.log, /tmp/p242-docs-build.log,
/tmp/p242-planning.log, /tmp/p242-links.log.

Retained failures: initial sccache build stalled without a compiler and its
exact owned scope was stopped. Cache-off builds passed. Shared debug Clippy
read an inconsistent retained model artifact; isolated target strict Clippy
passed without lifecycle source changes. Receipts remain in /tmp/p242-clippy.log
and /tmp/p242-discovery-build.log. No shared cache or foreign process cleanup.

Actual public browser TDD: discovery/action/inspect-guide red receipts preceded
implementation. Final installed workflow verifies correct company, rejects
wrong company, projects password required and stale observation, preserves peer
work, reuses one task/tab, and reports zero owned process residue. The supported
software wrapper and agent calls consume the same routine.

Installed p242 generation: 0.28.0-bf895a5c9b61; binary SHA256
bf895a5c9b617c433a65d0d98648b80ed335228a8d0528511ebd54eaee0bd8bf.
Published without activating service units, with reviewed /usr/bin/google-chrome
and namespace ports5380–5386. /tmp/p242-install.json records installation;
/tmp/p242-installed-launch.json passes three opens/readbacks/closes and preserves
production identity. /tmp/p242-installed-verification-corrected.log passes the
public fixture. The first installed fixture invocation used a wrong executable
path and failed before launch; /tmp/p242-installed-verification.log retains that
harness failure. No production publication or provider readiness claim.
The unsealed candidate records binary identity, not a fabricated clean-source
manifest. Integration custody commit records this slice; whole-batch merge
readiness and review remain pending.

## Additional provider facts

Trusted grok-bots receiver injection supports guarded private apply and human
MFA waiting today. It has a 30-second IPC timeout; request inspection after
uncertainty must prevent resubmission. Source b1fa8e8ef2f9f0f4f65b0373a2c0fdb6227091b0
is a foreign active draft and must be qualified separately from installation.
There is no implemented second-source MFA authority or provider binding.

Read-only Remote View source inspection at
c67d1f60ae33d8aebf8fe0948ddf777d5e17e121 finds no desktop-wide privacy hold.
Input hold is held buttons/keys, and route revocation destroys viewer sessions;
neither gates captures, OCR, watchers or viewers. General private-input provider
acceptance remains a named dependency. Owned synthetic acceptance must omit
viewers/watchers/recorders and guard every fixture read through authPrivacy.

## Next action

Advance ticket273 using supported client composition and exact helper request
custody. Qualify a constrained installed dummy workflow with actual helper
source, separately from unavailable provider-wide privacy and real MFA methods.
Keep all five tickets in scope; do not claim full-spec readiness from ticket272.
