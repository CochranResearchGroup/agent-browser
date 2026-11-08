# Cleanup102 publication diagnosis

Product lane: PL-BUGFIX
Work item: 586 (cleanup evidence only; Native capture cause remains separate)
Owner: ecochran76
Branch: fix/cleanup102-publication-trace
Base: 8db2baf3f08022f007696c940a7daf1e2e21feb3
State: ACTIVE

Operator approved the bounded tracing proposal in Overleaf cleanup101. One isolated
checkout preserves all existing worktrees. Installed ffd614f is independently
sealed to this exact clean source; it differs from prior304 only in build/package
identity. Pending585/PR588 source is not included and is not claimed as this fix.

Instrument the existing error boundaries without changing predicates, locks,
record formats, input authority, rejection or recovery. Default tracing is off;
AGENT_BROWSER_DEBUG_CLEANUP102=1 emits at most32 events per
process: source-only guard labels, snapshot SHA256 hashes and names of changed
top-level struct fields. Rejected publications emit context once; the same budget
covers all event types. No browser content, identifiers, account labels, credentials or raw
records are logged. Temporary instrumentation must be removed after diagnosis.

First run a fresh public close through the real host/SQLite and scripted effects.
A private snapshot replay is opt-in and hash-pinned; audit all paths, ownership and
provider inventory before effects. A setup refusal is not an exact-close RED.
No raw snapshot is tracked. Only if local replay cannot expose the guard may a
qualified sealed candidate use the approved preserving live diagnostic packet.
Do not replay the retained login100 close or credentials, reset profiles or remove
fences. Native and source receipts remain separate from real cleanup acceptance.

Root owns trace code, builds and all runtime effects. Delegated helper owns only
cleanup102_replay_tests.rs; it has no Git or runtime custody. Existing source lanes
and dirty CONTEXT.md are untouched. Qualification uses focused controls, publisher
refusal coverage, required Rustfmt/Clippy and exact binary/install checks if needed.
Final broad Overleaf review/remediation counters remain unchanged.

## Local qualification before candidate freeze

2026-11-08: two model tests pass (budget and field-summary privacy). Focused CLI
control passes1, private replay is ignored by default. With tracing enabled, fresh
public close, activity-publication refusal fences and interrupted-retirement
recovery controls each pass. Refusal coverage emits exactly32 bounded events,
verified to contain only static labels, SHA256 hashes and field-name lists.
The private captured-state replay still refuses isolation preflight; this is not
the user's exact-close RED. Rustfmt, workspace Clippy with -D warnings, policy
wiring11/11 and handoff-document validation pass. CI test profile artifacts are
not installed. Production packaging and exact artifact fixture remain pending.

Private Cargo Signal receipts and source fingerprints are retained in the
operator's cleanup102 diagnostic evidence directory. Temporary trace qualification
is not a causal diagnosis, cleanup acceptance, release or final broad review.
