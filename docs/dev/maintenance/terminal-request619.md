# Terminal request reconciliation (#619)

Product lane: PL-BUGFIX
Work item: 619
Branch: fix/terminal-request619
Base: 4410f56484fbe6fba85157b59d85e6c0c7d9b7c7
State: OPEN

Current qualification: focused source/public guard PASS; exact regular client
candidate and actual original-consumer reconciliation remain pending. Historical
failed/incomplete stages below remain truthful and are not erased.
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

One explicitly authorized warm resume used the same source/cache with2.5GiB
hard/soft cap and2GiB host reserve. Initial no-wait admission refused without
compilation when capacity was28MiB below threshold. The later single guarded
admission-wait stage reused completed dependencies and reached actual CLI test
code generation, but timed out at608.218seconds including termination after its
600-second deadline. It emitted no diagnostics and ran zero tests. No candidate
was qualified. Prior failures, refusal and cumulative cost remain in the receipt.
No further compile attempts or runtime changes were made.

The final cheaper strategy disabled only CLI package test debug symbols and
sccache, preserving Cargo objects and dependency profiles. Actual compiler argv
confirmed direct CLI rustc without debuginfo2 and all dependencies reused.
It timed out at600.158seconds, with zero diagnostics/tests and no executable.
The hard2.5GiB cap was reached with436812 observed memory-max reclaim events;
there was no observed OOM. This establishes a concrete constrained working-set
gate, not a source pass or unexplained cache wait. All further compile stages
stopped; an exact owned-scope census found no remaining members. No runtime
metadata reconciliation was attempted from unqualified source.

A separately approved capacity reframe retained2GiB host reserve and raised only
its compiler claim/cap to4GiB after owned browser cleanup. Guarded admission
waited for actual capacity, then reused warm dependencies with unchanged source,
CLI debug0 and directrustc. The stage timed out at600.452seconds with zero
source diagnostics/tests and no executable. A partial sample observed3.756GB
peak with no memory-max/OOM events; final peak was not retained after scope exit.
Fresh exact-scope census found no members. Further compilation is on hold; no
candidate-backed metadata reconciliation or live credential attempt followed.

## Proper setup budget and measured capacity stop

The coordinator corrected the arbitrary600-second setup cutoff: the runner has
no such compile limit. One3600-second setup stage used5GiB hard/soft cap,
2GiB host reserve and2 build jobs, with the same CLI debug0/directrustc warm
inputs and no source changes. Guarded admission met the7GiB threshold before
compilation. It made real user-CPU progress, then reached the5GiB cap with443208
memory-max events. In the decisive interval user CPU advanced3.5s while system
CPU advanced58.34s and major faults increased15396, establishing sustained
hard reclaim and insufficient working-set capacity rather than a source error.
Per the explicit resource stop predicate, only that owned compile scope was
stopped at1929.31seconds (exit143); no setup deadline was reset. The scope is
inactive and a fresh process census is empty. No diagnostics/tests/executable
were produced. All5486.532seconds of attempted qualification remain recorded.
Source619 and actual104 remain unqualified; further stages require a material
capacity or test-harness reframe. No live browser recovery budget was changed.

## Actual source/public qualification after capacity changed

With freshly observed16.47GiB available, normal admission accepted one10GiB
hard/soft, no-swap compiler claim plus2GiB host reserve. The same unchanged
control/test behavior compiled successfully in195.119s with zero diagnostics.
Actual compiler argv confirmed CLI-only test debug0/directrustc and warm dependency
reuse. Partial observed peak was6.788GB, with all memory events zero at that sample.
The final peak was not retained after scope exit; no exact maximum claim is made.

The exact built harness SHA256
`5d4aad4d491d39e1168ecaec8779423f83cf58ad44546937c02af7818d73e1ca`
listed2 focused619 and37 host_control tests. Direct execution with the repository's
RUST_MIN_STACK16777216 and test-threads1 passed2/2 in1.069s and37/37 in9.301s,
zero failed/ignored. The37 group includes those2 new tests. Positive same-target
live-supervisor reconciliation and all6 refusal variants are exercised. Production
RED remains retained; no browser input was replayed to obtain a test outcome.

Mandatory public documentation now explains the narrow producer-proof exception;
post-test Rust differences are an inline comment and help string only. Control
and test behavior remain the qualified63eaaebd9 implementation. Candidate-backed
actual reconciliation and broader integration gates remain separate. No default
binary/shared supervisor or original recovery/input history was changed.
