# Terminal cooperative request reconciliation

Product lane: PL-BUGFIX
Work item: 611
Branch: fix/terminal-control-reconcile611
Base: 66bf62d326c56cd69d9723494caf7c8086da768d
State: OPEN
Owner: ecochran76
Consolidation: required

## Current state

The existing public reconcile route requires a hosted task and awaiting-observation
control state. A terminal uncertain request on an owned standalone task after
human return cannot reconcile without manufacturing a control transition.
The original dispatcher is positively absent; no original action is replayed.
Source qualification passes; one sealed successor build and parent-owned installed
acceptance remain pending.

## Scope

Repair browser_control reconcile only: exact retained task/browser/target and
current original process identity, immutable epoch, fresh URL observation and
positively terminal original dispatcher. Preserve the original flight and owner,
record observed_uncertain_no_replay and attribution. Refuse human ownership,
active or unknown dispatcher, stale process/epoch and changed target. Other
hosted-only control mutations retain their current restrictions.

## Seams and ownership

Public MCP browser_control through an isolated socket transport catches dispatch,
existing-owner binding and original history; it does not prove real website login.
Primary owns host_control.rs, browser_host_control.rs, new public reconciliation
tests and shared docs. CLI610 auxiliary owns mcp.rs and native_cli_tests.rs in an
isolated auxiliary and returns a commit; no independent runtime custody.

## Admission

Ten existing checkouts were inventoried, nine clean; canonical607 integration
has attributed staged596 and607 edits and remains untouched. Parent explicitly
directed this isolated successor after seeing the population. Original596 remains
retained for immutable candidate custody. New checkout reviews at next source
handoff. Governance packet unavailable; canonical0052/0051/0044/0042 apply.

## Validation and exit

Public seam RED then GREEN, focused adverse invariants, Rustfmt and strict
workspace Clippy plus relevant docs/parity and611 planning audit. Combine610
before one production-shaped sealed candidate build. Parent owns adoption and
actual Overleaf104 acceptance. No runtime/control/browser effects in source work.
No extra broad review; final accepted106 review/TDD counters remain unchanged.

## First evidence

Exact qualified8edb25b binary public host-control reconcile in a disposable SQLite
home returns browser_control_hosted_task_required, exit1, instead of reconciling
the terminal original. The first failing response and fixture locator remain in
the private parent packet. Minimal repair preserves original task-only URL reads,
checks current process custody before and after, and leaves original uncertainty
and owner history rather than consuming another human-control epoch.

Source-local docs production build, API/MCP parity, handoff guidance, policy
wiring, changed links and own-lane planning checks pass. Focused actual Rust
fixture and required strict gates remain pending;610 holds the shared warm target
for its RED/GREEN. No original browser, control or input effect has occurred.

## Qualified source checkpoint

The [validation receipt](terminal-control-reconcile611-validation.json) binds the
actual Rust blobs to passing local gates: 33 affected control cases, CLI610,
ordinary managed-task status, strict workspace Clippy and Rustfmt, docs build,
parity, policy, links and five provider-free installation helper fixtures.
The first public RED, legacy fixture mismatch and lint failure remain retained.
The legacy fixture now refuses unknown ownership before positively identifying a
reused dispatcher; no production guard was weakened. No live control or input
occurred. Source is ready for one production-shaped candidate, with exact artifact
qualification and actual installed Overleaf acceptance still separate.
