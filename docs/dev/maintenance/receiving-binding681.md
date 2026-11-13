# Current receiving identity after reconciled control

Issue681; PL-BUGFIX; owner: primary SSO agent. Existing primary continues in
`fix/reconciled-binding-inspect`; no auxiliary checkout.

A valid operator-directed login reaches the password page, but read-only
`task_retirement` inspection cannot freeze its current identity after a successful
control reconciliation. The retirement lock rejects reconciled history because
inspection and actual retirement share one admission path.

Separate binding inspection from retirement admission. Binding inspection may
reserve current identity under completed or explicitly reconciled history. It
must still reject human control, awaiting observation and unresolved dispatch.
Actual retirement keeps its strict guard and its existing refusal of pending
task operations, occupied profiles and unverifiable process loss. No original
outcome is cleared, replayed or converted into credential permission.

Qualify the actual guard regression RED/GREEN, the SQLite/model seams, strict
Clippy and applicable local checks. Bind package proof to the exact candidate;
preserve selected runtime lineage, tenant custody and peer browsers during any
normal supported adoption. Then inspect only the original binding and continue
Auth's existing password request. Private input, SMS MFA and independently
verified Superhuman account acceptance remain distinct live gates.

Local evidence: actual guard RED failed as expected; all 11 focused native
tests passed after the repair. Standard workspace Clippy with `-D warnings`,
formatting, policy wiring, documentation links and remote-view handoff checks
passed. Supplemental all-targets Clippy exposed 14 existing style errors in
unchanged test fixtures; that failed receipt is retained. Production candidate
and installed binding acceptance remain pending. No retirement, private input, profile replacement or new browser
is authorized by a passing inspection.
