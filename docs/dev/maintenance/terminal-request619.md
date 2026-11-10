# Terminal request reconciliation (#619)

Product lane: PL-BUGFIX
Work item: 619
Branch: fix/terminal-request619
Base: 4410f56484fbe6fba85157b59d85e6c0c7d9b7c7
State: OPEN
Owner: ecochran76
Consolidation: required

Scope: recognize producer terminal DOM operation provenance while the shared
supervisor remains alive. Preserve unknown-input no replay, fresh target and epoch
checks. No runtime reset, installation or private input waiver.

Terminal task-operation timestamps alone are insufficient: lost transport can
also populate them. Qualification additionally requires the owner's terminal
uncertain_no_replay control outcome, exact request/context and fresh observation.

Actual public RED: primary's raw browser_control reconcile response returned
browser_control_dispatch_still_in_flight for the original completed click. Private
receipt remains in the consumer's user state directory; no private identity is
copied into source. Added public MCP fixture for live-supervisor completed DOM
click reconciliation and refusal variants: pending, native input, missing
completion, wrong epoch/target and active flight. Execution remains pending.

Repair accepts only the conjunction of owner-terminal uncertain_no_replay flight
and existing uncertain completed click with exact request/task/browser/profile/
tab/target/process/epoch provenance. No timestamp is synthesized. Existing fresh
process/target observation and atomic commit checks still apply; legacy missing
provenance remains guarded. No native/private input exception is introduced.

Validation: rustfmt and diff hygiene pass. Rust compile/public fixture execution
and strict integration checks are PENDING: workstation memory pressure with full
swap prevents the governed Cargo resource admission. No GREEN, installable or
live-qualified claim is made. Parent owns runtime adoption and final validation.

## Bounded validation outcome

The resource-constrained attempt is retained in
[validation receipt](terminal-request619-validation.json). First metadata stage
failed on an unrelated baseline integration-harness module omission. Corrected
CLI-only test compilation reached the original900-second deadline with zero
source diagnostics, zero executed tests and no qualified executable. This is
incomplete validation, not a pass. Both stages used one job, existing shared warm
target and hard2GiB/no-swap scope. First stage soft limit was raised to its unchanged
hard cap after measured soft-reclaim; second used equal soft/hard2GiB throughout.
The own timed-out scope is inactive and a fresh process census found no members.
No runtime adoption, native input or sharedhost change occurred. Do not close619
or104 from this checkpoint. Resume only the required missing qualification.
