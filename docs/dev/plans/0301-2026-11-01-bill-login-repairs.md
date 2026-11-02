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
| #519 | none for source | /root/native519 | integrated | new Native readiness module/tests; root integrates shared host hook | fix/bill-native-readiness-519; base78132d85d; result5a801208d; integratedb0aeb82c3 |
| #520 | none | root | implemented | owner HTTP error projection, Native wrapper, service-model recovery errors/tests | integration branch |
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

Final broad review: pending, exactly one invocation. Review fixed point awaiting
operator confirmation. Accepted findings: none yet. One TDD remediation: pending.

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
Clippy and cargo-safe format final results pending. Intermediate custody only.
Private #522 original readback was located: no lower cause or phase retained.
/root/recovery522 investigates only existing evidence and supported read-only
exact-assignment recourse; no reproduction of launch/recovery is authorized.
