# P301 BILL login incident repairs

Product lane: PL-BUGFIX

Spec: Agent Browser issues #518 through #522 and their approved triage briefs.
Accountable owner: ecochran76. Integration branch: fix/desktop-evidence-518.
Base/fixed-point candidate: bddb5b3fa9437639215a8431664dd6ef2da88575.
Starting tip: 78132d85d190d8da77d70fd62e6ff483214ce40d. PR #531.

## Authority and boundaries

User invokes implement-spec to close tickets, root owns one ticket and workers
own one unblocked ticket each; one final Standards/Spec review and one bounded
TDD remediation pass. No hosted CI, production installation, shared-service
commissioning, credential submission, blind recovery or unrelated cleanup.
Source validation and tracker closure remain distinct from live acceptance.
Operator authorized dedicated worktree custody and requires closeout afterward.
Parallel invocation admits two necessary temporary worker checkouts under this
parent; nine total checkouts temporarily exceed the warning threshold. Preserve
six foreign P300/Auth/inventory checkouts; no foreign checkout is reassigned.

## Frozen acceptance and seams

- #518: actual normalized/authenticated request reaches evidence parser; public
  authority/provider-plumbing injection remains rejected. Existing 57-test evidence.
- #519: owner authentication and consumer input readiness are distinct from viewer
  readiness; exact owned target and private token-file custody; guarded input
  prerequisites through isolated owner HTTP fixtures. Actual commissioning is gated.
- #520: allowlisted owner status/cause/correlation survive original request and
  readback; observations never imply uncertain input; no-replay for uncertain
  effects. Actual HTTP/wrapper and adapter/recovery seams; bounded redaction.
- #521: supported current discovery/request/private-use SDK/helper examples;
  distinguish authority, unavailable/partial source, complete no-match and account
  ambiguity. Public SDK fixture; no live authentication or source unlock.
- #522: recover original safe cause before causal fixture/repair; preserve original
  custody. Original evidence or precise unresolved gate, never infer actual cause.

No dependency cycles. #522 depends on validated #520 diagnostic contract and
original private owner evidence. #519 and #521 source work are independent;
#521 consumes documented readiness semantics without changing Native code.

## Resumable ledger

| Ticket | Blockers | Owner | State | Owned scope | Branch and result |
| --- | --- | --- | --- | --- | --- |
| #518 | none | root prior slice | validated | evidence parser/CLI tests | a7f55e83a; custody78132d85d; tracker open |
| #519 | none for source | /root/native519 | validated | new Native readiness module/tests; root integrates shared host hook | fix/bill-native-readiness-519; base78132d85d; result5a801208d; integratedb0aeb82c3 |
| #520 | none | root | validated | owner HTTP error projection, Native wrapper, service-model recovery errors/tests | integration branch |
| #521 | none for source | /root/guidance521 | validated | auth guidance and public SDK/helper fixture | fix/bill-auth-guidance-521; base78132d85d; result47dee514b; integrated7be3c8f22 |
| #522 | #520 and original evidence | pending | blocked | original owner evidence/read-only diagnosis | no effect authorized |

Root is sole writer for shared native module registration, browser_native_host,
README/help/docs parity and catalog. Worker519 supplies an isolated readiness
module and proposed host integration contract; no shared host writes. Worker521
owns auth playbook/standalone-auth guidance and its fixture only. P300 retains
workspace UI and ingress; do not edit its foreign source.

## Validation and review ledger

#518: exact red clientSubjectId rejection; focused desktop_evidence 57/57 green,
workspace Clippy, fmt, policy wiring and changed links passed. Broad unrelated
route-confusion lane excluded, initial compiler-cache stall retained in PR531.
Workers record exact red/green commands and source/effect limits. Root runs final
fmt and strict workspace Clippy plus causally affected focused contracts once.
Default builds disable cache after the observed sccache stall; Cargo admission
remains mandatory. Coordinate at most two repository Cargo invocations, use
isolated homes and fixture ports, preserve first failures and unknown outcomes.

Final broad review: complete, exactly one invocation. Review fixed point is starting origin/main bddb5b3fa9437639215a8431664dd6ef2da88575;
optional clarification requested and no alternate supplied. Accepted findings: Standards S1 and Spec F1/F2, all P2 and resolved. One TDD remediation: complete.

## Closeout obligations

Integrate serially; verify source SHA and scope before tracker closure. Close
worker checkouts after durable custody and the clean-state/candidate inspection.
Close integration checkout after durable PR/integration custody. Use governed
worktree closeout for all three. Leave unresolved runtime gates truthful.

## Integrated source evidence

#519 actual missing-auth red failed unconditional readiness. Integrated green:
`AGENT_BROWSER_CARGO_CACHE=off CARGO_TARGET_DIR=<shared-target> scripts/ci/rust-tests.sh --focused native_readiness`, 5 passed. Exact lifecycle policy read is
independent of viewing identity/control/viewer readiness. Root integrated Native
ready and pre-input hooks. No live commissioning acceptance is claimed.

#520 red Native HTTP rejection produced Rejected instead of the authentication
cause; red public recovery fixture lost Transport(Rejected). Green Native owner
HTTP tests 3 passed (auth, observation versus uncertain input, malformed and
oversized body redaction). `cargo-safe.sh test --manifest-path Cargo.toml -p
agent-browser-service-model --test browser_handoff_recovery -- --test-threads=1`
passed all10. Generic consumer mutation uncertainty remains unchanged; safe
consumer diagnostics apply only to launch-environment reads, plus auth refusal.

#521 actual installed verified SDK source047372bbf7d148fe3bac75513ebf9c83b1777ecd
and hosted helper/public factory/doc example:17 integrated checks pass with only
a synthetic external broker and receiving browser. Exact command:
`AUTH_RECEIPTS_SDK_INSTALLATION=<reviewed-installation> node scripts/test-auth-discovery-guidance.mjs`.
This includes the causal owner-login-playbook source fix for choice_required,
partial/truncated/unavailable and completed not_found reason codes. Existing
legacy/hosted playbook tests and JavaScript type coverage pass. No real vault,
source unlock, account enrollment, private input or installed skill publication.

Docs build, policy wiring and changed links pass. Required strict workspace
Clippy and cargo-safe format final checks pass. Root module registration order
was corrected after the first format diagnostic; only ordering changed.
Private #522 original readback was located: no lower cause or phase retained.
/root/recovery522 investigates only existing evidence and supported read-only
exact-assignment recourse; no reproduction of launch/recovery is authorized.

## Final source gate and custody readback

`--focused remote_view_application_http`:12 passed. `--focused task_operations_tests`:14 passed, including original request readback and uncertain-input no-replay.
`--focused desktop_evidence`:57 passed on integrated source. Readiness5 and
recovery10 passed. Current SDK/helper/doc17 pass. Strict workspace Clippy,
cargo-safe fmt, documentation build, policy wiring and changed links pass.

The initial playbook command exited1 without a terminal diagnostic; its log is
retained. Explicit docs-scoped TypeScript reproduced unrelated implicit MDX JSX
namespace failures. The harness now uses its declared docs-workspace TypeScript
and `--types node`, matching these non-UI modules. Complete playbook/client type
lane passes without ambient UI types or a global TypeScript fallback. This is a
harness repair, not a claim that an unexplained failed run passed.

Worker519 clean exact tip5a801208d removed via governed operation
worktree-closeout-70c837aa-33e8-41fa-8cf2-b6c47573828b; worker521 clean exact
tip47dee514b removed via worktree-closeout-eeccf688-2c64-4e7c-9f8b-496517321f0e.
No candidate pins/builds existed. Integrated remote source retains both outcomes.
Root integration checkout remains until final review/remediation and publication.

#522 evidence-only investigation found no original cause or exact provider
generation custody in the retained evidence. No safe original-assignment probe
can be targeted. Original-cause and cause-specific fixture criteria remain
unproven; keep the ticket open. No live probe/input/configuration/recovery occurred.

Source tickets518-521 are eligible for closure under the approved source briefs;
no merge, installation, provider commissioning or actual login is claimed.

## Single broad review and adjudication

Frozen review tip77921bc1646c3f78140d01a363e3bd9fcd636d54 against fixed
basebddb5b3fa9437639215a8431664dd6ef2da88575; nonempty three-dot diff and
commit list captured. Independent /root/review_standards and /root/review_spec
reviewed the same packet once. No second broad review is authorized in this slice.

### Standards

S1 blocking P2: accepted. Direct readiness-helper tests do not detect removal of
actual Host input and ready projection hooks. Add an actual Host/SQLite/HTTP seam
regression and qualify its oracle against removal of the production hooks.
S2 nonblocking_backlog: duplicated static owner-code allowlists are a possible
maintenance smell; operation-specific uncertainty remains intentional. No broad
refactor is necessary for the source repair. Selector registration is an optional
automation gap, not an unmet gate: manual fixture coverage is explicit and the
selector's broad fallback remains intact.

### Spec

F1 blocking P2: accepted. Non-JSON, malformed or oversized launch-environment
HTTP failure can lose known status before the safe cause projection.
F2 blocking P2: accepted. Checkbox/image control-status preflights discard typed
auth causes before the newly repaired Native request wrapper.
#519/#520 reopened for bounded remediation; #518/#521 source closure remains.
#522 remains an evidence gate, not a source-fix acceptance claim.

## One bounded TDD remediation pass

Invocation count1. Seams: real owner control-status HTTP projection (catches auth
cause erasure; no actual provider proof), real consumer launch-environment HTTP
read (catches status loss independently of invalid bodies), and actual Host
Native process command with real SQLite/fake external owner (catches missing
readiness/input wiring; no live guarded-input commissioning).
Work vertical slices per accepted finding; retain red/green commands and known
bad-source diff for the Host oracle. No new broad review or unrelated feature work.

## Remediation result

Standards S1 and Spec F1/F2 are resolved in the single bounded pass. No second
broad review ran. The nonblocking duplication/selector advice remains deferred;
no unrelated architecture or validation-selector work was added.

Exact test environment for all CLI Rust commands:
`AGENT_BROWSER_CARGO_CACHE=off CARGO_TARGET_DIR=/home/ecochran76/workspace.local/agent-browser/cli/target`.
All commands use `scripts/ci/rust-tests.sh --focused FILTER`:

| Finding/seam | Filter | Red evidence | Green evidence |
| --- | --- | --- | --- |
| F2 control preflight | native_control_preflight | generic control-unverified lost401 | 1 passed; both source callers use composed safe projection |
| F1 consumer failure | launch_environment_failure_retains | HTML503 became OutcomeUnknown | 1 passed across HTML/malformed/oversized cases |
| S1 actual Host | native_host_readiness | known-bad removal returned ready=true and accepted input | 2 passed after exact source restoration; disabled owner receives zero input |

The first Host fixture attempt failed setup with unsupported empty SQLite schema0;
it is retained as a harness failure, not product red evidence. The fixture now
uses supported SQLite migration. The qualified red run exercised both actual
Host failures after deleting the production hooks. A retained outside-Git diff
and good/bad SHA256 receipt prove mutation of the real source; restoration's
checksum matches the pristine copy. No known-bad hook deletion remains.

Final focused transport suite14 and original-request/readback suite14 passed.
Final strict workspace Clippy and cargo-safe fmt passed. Policy wiring (11 harness
tests), links and patch hygiene pass. Prior evidence57, readiness5, recovery10,
Auth SDK/helper17, JavaScript/types and docs-build results are reused because
those source seams/pages did not change during remediation; the affected HTTP,
Host and readback risks were explicitly rerun. This is focused local Linux source
qualification, not a comprehensive/multiplatform or installed-provider claim.

Source closure restored for519/520;518/521 remain closed.522 stays open on the
exact original-evidence gate. Integration branch and PR531 retain all source;
root checkout will close after the final clean-state/durable-custody readback.
