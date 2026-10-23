# P242: Implement shared website automation spec271

Product lane: PL-RECIPES
Work items: #199, #271, #272, #273, #274, #275, #276
Branch: recipes/siteguides-271
Baseline: cb765a7be2719383a54fb849eac9691ce0197b06
State: COMPLETED

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
CLI/help/README/skills/MDX/docs and synthetic fixtures. P237 retains dashboard and its existing lifecycle/handoff work. P242 is the
primary writer for the narrow observed-popup task-ownership contract required
by ticket273: a pure method on the existing manager, one runtime observation
adapter, host publication and existing MCP tab-list consumption. These overlap
browser_session_host.rs and browser_session_runtime.rs with P237. Preserve its
dirty checkout and establish this provider contract on P242 before dependent
recipe use; any later integration with P237 must reconcile the narrow provider
change explicitly. No second task owner or viewer is introduced.

The review fixed point is cb765a7be2719383a54fb849eac9691ce0197b06, recorded under operator-authorized implement-spec
execution. One broad Standards/Spec review and one bounded remediation pass.
Spec271 and ticket272–276 bodies are authoritative; source research and the
accepted prototype are locators and design references, not runtime proof.

## Resumable ledger

| Ticket | Blocked by | State | Primary ownership | Evidence |
| --- | --- | --- | --- | --- |
| #272 BILL existing-session verification | none | validated | primary, public playbook/MCP/client adapters | installed generation 0.28.0-bf895a5c9b61; receipts below |
| #273 BILL helper login/MFA | #272 | validated | primary, client composition and narrow popup-owner provider contract | installed synthetic + actual helper-source dummy; limits below |
| #274 QBO same contracts | #273 | validated | primary, QBO guide and shared fixtures | installed identifier/password variants; receipt below |
| #275 guide development/adaptation | #274 | validated | primary, frozen plans and versioned candidate workflow | installed authoring/private-flow receipts below |
| #276 Reddit retrieval | #275 | validated | primary, non-login guide/routine/fixture | installed bounded retrieval receipt below |

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

## Ticket273 validation receipt

The supported login-playbook client composes the existing routine and actual
request_login/inspect_request/authenticate protocol, with durable metadata
custody before submission. A restart inspects the same request. Source-unavailable
receiving readback with capture ready and no input marker proves no private
input. Exact locked-vs-unavailable disclosure is absent from the producer;
password_source_unavailable is actionable without inventing that fact.

Task tab list now admits provider-observed popup opener chains through the
existing BrowserSessionManager and CAS publication. Peer, unknown-opener and
pre-existing tab claims are excluded. Known owned popups attach through the
existing browser transport. One fresh account/company-matching new destination
continues the original task; ambiguous matches require adaptation.

Focused source gates: strict Clippy /tmp/p242-273-clippy.log; format
/tmp/p242-273-fmt.log; host tests /tmp/p242-273-host-tests.log (6 pass);
model integration /tmp/p242-273-model-tests.log (34 pass); public package exports
/tmp/p242-273-exports.log; client typing/contracts /tmp/p242-273-client-final.log;
docs /tmp/p242-273-docs.log; links /tmp/p242-273-links.log. Existing generated
client checks are retained because no generated helper or generator changed.

Red receipts preserve actual popup non-attribution, then attached-page lookup
failure: /tmp/p242-popup-attribution-probe.log and
/tmp/p242-popup-helper-green-attempt.log. The focused custody oracle also failed
before the overflow atomicity fix: /tmp/p242-popup-atomic-red.log, then green.
Locked-source first assertion incorrectly expected immediate knowledge from an
error response; retained /tmp/p242-273-locked-source.log. Corrected fixture
inspects the same request before claiming no effect;
/tmp/p242-273-locked-source-reconciled.log passes with zero residue.

Final installed candidate 0.28.0-6e2aa0ccd463, SHA256
6e2aa0ccd4631d65a826527cb9a30bf288c5d92d76ff49ba5c961b1ced0f9ed1;
/tmp/p242-273-install.json and /tmp/p242-273-installed-green.log prove isolated
publication and the actual-source dummy private fill, human MFA wait, popup
continuation, capture hold, no resubmission, peer preservation and zero residue.
/tmp/p242-helper-source-identity.json pins the four actual draft helper modules.
The producer is source-qualified in this constrained fixture, not a claimed
installed LastPass or stock-desktop acceptance. Provider-wide observer privacy,
exact password-manager locked/presence discrimination, passkeys and un-attributable
extension targets are named producer capabilities. They do not manufacture live
acceptance or prevent this approved provider-free/source delivery. Tracker
closure, broad final review and production publication remain separate.

## Ticket274 validation receipt

QBO consumes the same software/agent routine, private helper and owned-popup
contracts with different selectors, account element and realm identity. Reviewed
SiteGuide controls now configure the trusted receiving adapter. No QBO OAuth,
API operation or tenant state is included. The embedded BILL/QBO guides are
starter templates, not claims that real controls have been qualified.

Public old-binary red: /tmp/p242-qbo-guide-red.log rejected the new guide shape
before source support. Candidate green: /tmp/p242-qbo-identifier-green-attempt.log
and /tmp/p242-qbo-password-green-attempt.log. Final gates: format
/tmp/p242-qbo-fmt.log, strict Clippy /tmp/p242-qbo-clippy.log, MCP
/tmp/p242-qbo-mcp-tests.log (132 pass), shared client typing/contracts
/tmp/p242-qbo-client.log, docs /tmp/p242-qbo-docs.log, changed links
/tmp/p242-qbo-links.log and patch hygiene.

Installed p242 candidate: 0.28.0-beadcb7eb149, SHA256
beadcb7eb149958fab519df32e8be8dd9bb12a551cafa01b2ac8513ebf715c5a.
/tmp/p242-qbo-install.json records isolated publication. Installed public client
receipts /tmp/p242-qbo-installed-identifier.log,
/tmp/p242-qbo-installed-password.log and
/tmp/p242-qbo-installed-bill-regression.log pass existing-session reuse, private
actual-source dummy login, human MFA wait, asynchronous owned destination,
wrong realm, freshness, controller restart, peer preservation and zero residue.
Earlier host/model custody gates remain causally valid because this slice did
not change them. Provider/helper production gaps from ticket273 remain explicit.

## Ticket275 validation receipt

prepare_login freezes guide/task/control/state/freshness evidence; validate_plan
rejects changed evidence before private input. The supported login client retains
its helper request/action identity and checks the plan before credential retrieval.
A stale observation of unchanged controls can refresh without another grant.
Password preparation requires a masked input; fresh preparation cannot bless
missing controls or an unmasked replacement.

The authoring fixture validates a v2 proposal through a candidate MCP client
in the original task and proves canonical bytes/registry remain unchanged.
Controller switch during MFA preserves checkpoint/action identity; lost helper
acknowledgement reconciles receiving state without another retrieval/submission.

Retained reds: /tmp/p242-authoring-red.log, /tmp/p242-authoring-native-red.log,
/tmp/p242-authoring-preparation-red.log and
/tmp/p242-275-unmasked-preparation-red.log. MDX prose braces failed parsing in
/tmp/p242-275-docs.log; inline-code correction passed. Final gates:
/tmp/p242-275-final-fmt.log, /tmp/p242-275-final-clippy.log,
/tmp/p242-275-final-client.log, /tmp/p242-275-final-docs.log and
/tmp/p242-275-links.log. MCP metadata tests passed 132 in
/tmp/p242-275-mcp-tests.log; mask-only predicate change is separately exercised
by public preparation/authoring fixtures.

Qualification correction: earlier private-flow fixtures let the receiving
adapter insert authenticated DOM. Those receipts establish protocol traversal,
not independent credential delivery. The strengthened website independently
checks an opaque runtime-generated dummy password and username, presents its
MFA challenge and opens the receiving page only on human confirmation. The
adapter no longer writes authenticated DOM. Producer/final results are checked
for absence of that opaque secret. /tmp/p242-275-wrong-private-credential.log
proves a wrong private password produces no false completion and leaves zero
owned process residue. These stronger receipts supersede that limitation for
all three preceding tickets; earlier receipts remain retained.

Final installed candidate 0.28.0-157f9ec751df, SHA256
157f9ec751df7df182338f7dc7d062eac1a218b915fcd4e3bf9ed7681be6e665.
/tmp/p242-275-final-install.json and installed public-client receipts
/tmp/p242-275-installed-bill-authoring-ack.log,
/tmp/p242-275-installed-qbo-identifier.log,
/tmp/p242-275-installed-qbo-password.log pass independent website login,
authoring/adaptation, source-helper private filling, human MFA, controller
continuation, injected lost acknowledgement, owned popup, exact company/realm,
peer preservation and zero residue. Default MCP stays at 12 tools.
Real credentials, installed helper production readiness and stock desktop/viewer
privacy remain unclaimed provider-qualified work.

## Ticket276 validation receipt

Non-login SiteGuides declare retrieval assertions without fake account/company
fields. collect_posts uses the existing task and shared result interface,
bounded normalized items, page/link revalidation and DOM-scroll budget. Positive
empty-state evidence is distinct from empty/malformed extraction and login/blocked
pages. Actual coverage/freshness and partial results remain explicit. No ranking,
posting, live social request or downstream migration is included.

Red /tmp/p242-retrieval-red.log preceded the routine. A public malformed-link
oracle failed in /tmp/p242-276-missing-link-red.log before the missing-href fix;
/tmp/p242-276-fixed-public-green.log passes all cases. Source gates:
/tmp/p242-276-final-fmt.log, /tmp/p242-276-final-clippy.log,
/tmp/p242-276-final-client.log, /tmp/p242-276-docs.log and patch hygiene.
The source change is also covered by final broader gates listed below.

Installed candidate 0.28.0-42211b09d2c2, SHA256
42211b09d2c2a71ad8120684ed980bbaa363f2e39f0cf18af1aa9e665e18f15f.
/tmp/p242-276-install.json and /tmp/p242-276-installed-retrieval.log prove actual
public-client bounded pages/scroll, deduplication, no-results, unproven emptiness,
malformed/missing links, login/blocked/partial output, original task/peer
preservation and zero residue. /tmp/p242-276-qbo-regression.log retains the
shared login contract regression after optional non-login guide sections.

## Whole-branch gates and dependencies

All five slice states are validated on this integration branch; tracker closure
is separate. The final comprehensive provider-free Rust run remains active under
session82194, log /tmp/p242-final-rust-comprehensive.log. Its partition logs are
/tmp/agent-browser-rust-tests.vhSKC3 while running and the aggregate preserves
results on completion. Workstation fixtures/release-asset fixture and validation
selection/client/handoff documentation contracts ran under
/tmp/p242-final-workstation.log, /tmp/p242-final-client.log,
/tmp/p242-final-validation-contract.log and /tmp/p242-final-handoff-docs.log.

An extra policy-wiring check failed in /tmp/p242-final-policy-wiring.log because
it expects retired bulk AGENTS links to policies0047–0049. Both AGENTS.md and
scripts/test-policy-wiring.js are byte-unchanged from cb765a7b; that baseline
already lacks the links and uses the current hash-bound policy-context pilot.
No canonical routing or manifest hash is changed to appease this unrelated stale
oracle. Current implementation/runtime governance packets were checked with
explicit authorized synthetic effects. Record this baseline limitation separately
from actual changed-surface failures.

[Provider qualification dependencies](../notes/0242-2026-10-14-website-automation-provider-dependencies.md)
are adopted by P242. The note preserves exact source capabilities and ready
provider proposals without foreign publication or a duplicate desktop layer.

## Next action

The full bounded implement-spec delivery is complete locally. Source and
owned synthetic qualification are retained on recipes/siteguides-271; no PR,
merge, tracker closure, production deployment or real-account acceptance is
claimed. Future canonical integration must reconcile the named P237 overlap
and retain the two independently proven baseline validation limitations.

## Frozen whole-branch review and bounded remediation

Standards and Spec reviewers examined cb765a7b through 7f7a8cb2 read-only.
Accepted four findings: mandatory prepare/validate dependencies; checked explicit
replacement of an unattempted plan on the retained request/action; reconciliation
of typed pre-input refusals using fresh exact-request evidence; bundled guide
IDs in copyable default examples. Remediation is confined to these findings,
associated documentation and meaningful continuation tests. No second broad
review or foreign edits. reviewedPlan retains guideId and exact scope/task
binding; reviewed content can change in a candidate registry under that ID.

The comprehensive provider-free run finished nativeLane=0 supportLane=1 in
1279 seconds. All native and workstation compartments passed. One existing
integration assertion expects service --help to succeed, but the current CLI
rejects it as Unknown subcommand. The unchanged baseline parser/test provenance
is being checked; this is retained as failed comprehensive evidence rather than
reported as a comprehensive pass.

Bounded remediation source gates passed: /tmp/p242-review-client.log (including
required plan dependencies, invalid replacement rejection, preserved request
and proven pre-input refusal retry), /tmp/p242-review-fmt.log,
/tmp/p242-review-clippy.log and /tmp/p242-review-docs.log. The expanded client
test fails against frozen 7f7a8cb2 in /tmp/p242-review-red.log at the transient
pre-input continuation assertion. Public installed source-helper adaptation
passed /tmp/p242-review-installed-adaptation-green.log, including changed
controls, reviewed replacement, same action, dummy private fill, human MFA,
popup attribution, injected lost acknowledgement and zero residue. Its first
harness attempt failed because it forwarded login-only options to strict native
preparation; /tmp/p242-review-installed-adaptation.log is retained, and the
fixture now passes only declared preparation fields.

/tmp/p242-final-service-collections.log passed the selected no-launch public
parity check against generation42211b09d2c2. Documentation links passed in
/tmp/p242-review-links.log. /tmp/p242-baseline-gate-provenance.json proves the
service parser, main dispatch and service-state integration test are byte
unchanged from cb765a7b, as are both files responsible for the extra policy-wiring
failure. No unrelated source repair or automatic CI activation is included.

## Final installed qualification and closeout

All five tickets are implemented, integrated locally and validated within the
approved source/owned-synthetic scope. The one broad review and one bounded
remediation are complete; all four accepted findings have focused protection
and the actual existing-task adaptation fixture continues the retained helper
request/action through a reviewed replacement plan. Default MCP remains 12.

Qualified executable source commit: a427f65eefe0e95ee2652edacde3abcd96bb2514.
Final isolated generation: 0.28.0-4af40efff22c. Binary SHA256:
4af40efff22c41a56463385272f96ba9b3a9a6bba926b47be8e440cc04b25574.
The executable is under the p242 generation bin directory. Publication receipt
/tmp/p242-review-install.json proves production and default-development identity
preservation and p242-only skill parity. Generation source metadata is null
(unsealed build), not a fabricated clean-source provenance assertion. Units were
not activated and the installed runtime status remains ready:false as expected
for this no-activate qualification lane.

The active foreign helper changed from the initial b1fa8e8e snapshot to
23877a5c0c200b11517a0369cf35ced65e31ca99 during final checks. Final tests use
an immutable read-only copy of its actual four modules, pinned by
/tmp/p242-helper-source-final-identity.json at /tmp/p242-auth-helper-final.
No foreign edits or installation occurred. This remains source/dummy producer
qualification, separate from an installed producer and real credentials,
LastPass extension/passkeys, automated factor access or desktop-wide privacy.

Final-candidate public receipts:
- /tmp/p242-final-installed-bill-pinned.log: verification, bounded retrieval,
  versioned guide authoring, changed-control replacement, private dummy fill,
  human MFA, popup continuation, restart and injected lost acknowledgement.
- /tmp/p242-final-installed-qbo-identifier.log and
  /tmp/p242-final-installed-qbo-password.log: distinct QBO controls/identity,
  both login variants, MFA, receiving popup, exact company and peer preservation.
- /tmp/p242-final-installed-wrong-credential.log: independent website rejection
  of a wrong opaque credential, no false completion and zero residue.
Each fixture reports a fresh zero-owned-process census.

The required three-iteration development launch smoke passed in
/tmp/p242-review-development-launch-green.json after an initial stale-daemon
failure retained in /tmp/p242-review-development-launch.json. Exact idle p242
host37416 still used the first bf895a5c9b61 generation. Read-only Service Status
proved zero browser/session/tab records and runtime owners before SIGTERM of
that positively identified owned host; no profiles, borrowed processes or
foreign resources were removed. Final smoke-created idle host88643 was likewise
retired after zero-record/owner readback. /tmp/p242-final-runtime-closeout.json
records a fresh OS census with zero remaining p242 fixture processes.

Source checks, docs and links passed. Comprehensive provider-free Rust remains
accurately nativeLane=0/supportLane=1 because of the unchanged service-help
integration assertion. The extra policy-wiring test remains a separately
identified stale baseline oracle. Neither is erased or described as a pass;
these are broader baseline repair inputs, not a defect in a validated slice.
No GitHub CI was activated or dispatched. All owned fixtures were cleaned up;
primary and foreign worktrees, retained candidate generations and prototype
branch remain intact.

Memory disposition: queued. The qualified curated P242 closeout was accepted
by graphiti-runtime remember into agent_browser_main, job
ea38b434-88e1-40a9-a090-c31f8d52c56f, receipt /tmp/p242-closeout-memory.json.
Queue acceptance is recorded; persistence and grouped retrieval are not claimed.
The source-backed singleton was written once; no Codex memory files were edited.

## Pull-request preparation

Operator approved the ask-matt /pr step after local delivery. On 2026-10-14,
fresh origin/main remains cb765a7be2719383a54fb849eac9691ce0197b06. The completed
branch is based on that exact tip, so none of its executable inputs changed
and the recorded validation remains applicable. No further broad review,
Rust rebuild, provider replay, production effect or CI dispatch is necessary
for this documentation/publication step.

P237 remains an independent active lane. Published draft PR270 is at
5a68c9bb12896dce36b7216754750817d3018e1b; its clean local checkout has advanced
to 9cdb48c32e16b766abe6d13d25b93054ebcd6b28. Prospective merge-tree checks against
both snapshots find the same two conflicts: browser_session_runtime.rs and
active-lanes.yaml. No merge was applied, and neither foreign branch nor its
checkout was modified. The other ten shared paths merge textually. A textual
merge alone does not qualify the combined behavior.

The runtime conflict joins adjacent independent requirements. Preserve P242's
attach_owned_target_if_missing for attributed popup targets and its sanitized
Target.getTargets observation. Preserve P237's bind_observation_target for
managed title/URL reads instead of replacing the person's physical selection.
When joining the branches, handle internal popup inventory without activating
a target, attach a missing addressed owned target, then retain P237's split
between non-activating title/URL reads and explicit target activation for other
actions. Validate peer-selection/restart and private-login popup continuation
on the combined source. Do not resolve by choosing one side's whole block.

The catalog conflict is two independent insertions: retain both P242 and P237
records with their own updated_at fields. P242 can be reviewed independently
against current main; merging it first establishes its narrow provider contract,
and P237 must reconcile the documented shared block before its own integration.
This sequencing note is not acceptance of unfinished P237 work.

The PR will remain draft with the explicit baseline-failure disposition visible:
comprehensive Rust failed the unchanged service-help assertion; policy wiring
expects retired links. Their verified unchanged-source provenance is recorded
above. They are not silently waived for a clean comprehensive or production
claim. Tracker issues remain open, and no merge/deployment is included in this
publication step.

## Shared source custody

Draft [PR285](https://github.com/CochranResearchGroup/agent-browser/pull/285) is
open against main and assigned to ecochran76. Initial forge readback matches
published commit41f2054ff51a85f33ccb7f49e5a62d4722894cd6 and base
cb765a7be2719383a54fb849eac9691ce0197b06; GitHub reports MERGEABLE against
current main. This does not resolve the separately documented prospective
P237 merge. The PR references spec271 and tickets272–276 without closing them.
No CI activation, issue closure, merge or deployment occurred. A final
documentation-only custody commit retains the PR locator on the same branch.

PR closeout memory disposition: unavailable. graphiti-runtime doctor reports
mcp_http down and provider readiness skipped, despite a working advisory tool
read. No write is attempted on an unqualified runtime; the machine-readable
non-write receipt is /tmp/p242-pr-memory-disposition.json. The prior implementation
closeout's queued receipt remains preserved and is not retried.

## Baseline repair batch for PR285

Operator approved /diagnosing-bugs after draft publication. Reuse the clean
recipes/siteguides-271 checkout, baseline860358db8, with bounded ownership of
main.rs help dispatch, the existing service-state integration oracle and
policy-wiring script plus matching documentation. Preserve P237; no merge,
production effect or CI activation. Both minimized REDs were run before fixes:
/tmp/p242-baseline-service-help-red.log (installed generation4af40efff22c,
service --help exits1 while state bytes stay unchanged) and
/tmp/p242-baseline-policy-red.log (Node assertion requires retired AGENTS links).
The feedback loops finish in under one second.

Ranked hypotheses distinguished parser flag leakage, absent renderer and
installed drift for help; old AGENTS contract, missing manifest route and
selector drift for policy wiring. Source proves clean_args retains help switches
and main validates those raw command arguments before an existing service help
renderer. Canonical pilot0053 and current AGENTS replace individual links with
profile/task loading plus fallback; the inventory retains the four policies,
and selector version remains0.1.26. Repair those exact causes without loosening
unknown-subcommand checks or refreshing canonical hashes.

Baseline repair results supersede the previous unresolved disposition without
erasing either RED. main now strips --help/-h only from its local help-path
arguments before validating commands. Ordinary command cleaning and dispatch
are unchanged. The existing service-state integration test also exercises
short/pre-command help, runtime create and eval stdin help, plus a real typed
unknown-subcommand negative. All four CLI integration binaries passed on the
final source debug executable: 10 tests total. Receipts are
/tmp/p242-baseline-service-final.log and /tmp/p242-baseline-other-integration.log.
The original isolated CLI loop passes in /tmp/p242-baseline-service-help-green.log,
without a daemon or state-byte changes. Source debug SHA256:
7625c2eed8f44d6a98cc6d3fb026dbd17903fcbc0288996055d5810ba15ed452.

Policy wiring now checks the current AGENTS pilot trigger, all inventoried
source hashes, required profiles/task kinds and canonical fallback routing,
rather than restoring old inline links. /tmp/p242-baseline-policy-final.log
passes with 48 unique policy identities and unchanged selector0.1.26. The
actual script rejects source drift, missing profile, missing fallback and
uninventoried policy in a disposable copied-source sandbox, then passes again
after restoration: /tmp/p242-baseline-policy-negative.log. No canonical policy,
AGENTS or reviewed manifest hash changed.

Final strict workspace Clippy, fmt, docs build, validation-selector contracts
and changed-document links passed in /tmp/p242-baseline-clippy.log,
/tmp/p242-baseline-final-fmt.log, /tmp/p242-baseline-docs.log,
/tmp/p242-baseline-validation-selector.log and /tmp/p242-baseline-links.log.
No temporary instrumentation or fixture source remains in the repository.
Previously passed native/crate compartments and installed website-flow receipts
remain applicable to unchanged executable behavior outside the has_help branch.
The earlier full comprehensive receipt remains a failed historical run; this
batch runs the entire affected integration compartment, not another full suite.
The p242 installed generation remains4af40efff22c and was not replaced by this
source repair. Real-provider acceptance and production deployment stay separate.

PR285 can now be marked ready for independent review against unchanged current
main. Its two baseline check blockers are repaired. P237 remains foreign,
active and unmerged; preserve both runtime intents when its branch later joins.
No live workflow needs unfinished P237 source to qualify this PR.

Baseline repair source commit b0a4c75075be410ee03c1b717a9202aa20f29778 is
published and read back on PR285. The PR description now distinguishes repaired
source checks from earlier installed website qualification and retained
comprehensive failure. GitHub reports OPEN, isDraft=false and MERGEABLE against
basecb765a7be. This is ready-for-review evidence, not a merged-source receipt.

Baseline repair memory disposition: queued. Doctor now reports healthy MCP
HTTP and storage. The qualified source-backed singleton was accepted once in
agent_browser_main, job a51e5c66-0a30-4b66-a9fa-2c3a09376e7b, receipt
/tmp/p242-baseline-memory-disposition.json. Queue acceptance is sufficient for
this closeout; persistence and retrieval are not claimed. Prior queued and
unavailable dispositions remain historical and are not retried.


## Tracker reconciliation and current fixture repair, 2026-10-23

Issues272–276 remained open after merged PR285 (0c1fde54dfb6e845503eb569b0d04cdf264f7231). Root audits272; independent read-only agents audit273,274 and275/276 against each criterion and canonical receipts. All recommend closure within the original source/owned-synthetic scope. PR285 and its qualified source are ancestors of current main. The historical temporary installed logs are no longer present; the canonical receipts above and source remain. This audit is not a fresh replay of the historical helper-source/MFA/private-input run or current production qualification.

A fresh public272 smoke discovers a current harness defect: browser_open calls omit required serviceName/agentName/taskName. It fails before browser launch and reports zero fixture residue. Add meaningful synthetic attribution to both retained task and peer calls; do not weaken public attribution requirements or change product behavior. This branch is recipes/siteguide-fixture-attribution-272 in the reused retained primary, Product lane PL-RECIPES. No new checkout, production publication or account effect.

Current test:playbook-client passes software/hosted contract and type checks. Sealed fast_iteration f16e5df1a313d2ecd4db43c452f9c9f6fb5c17bc binary33fdd29ed2b643ddabdcf7215f26c934b1fdb5146bc2d651895691dd73d0e8d7 with reviewed Chrome70bb98cb qualifies three fresh owned public workloads: BILL existing-session verification; BILL guide authoring plus bounded Reddit retrieval; QBO existing-session plus popup attribution. Correct account/company, wrong company, stale evidence, login-required, task/tab replay and peer preservation pass. Authoring rejects changed controls and validates a versioned candidate without canonical rewrite; retrieval distinguishes bounded/empty/malformed/login/blocked/partial states. All report zero owned process residue. Independent current-user readable proc marker census finds zero fixture references; denied foreign reads remain an explicit coverage limit in the private receipt.

Preserve the extra diagnostic combining QBO with the BILL-only authoring helper: it fails before a frozen action plan. That undocumented flag combination is not claimed supported, and the historical qualification is not expanded. Canonical BILL authoring and separate QBO popup workloads pass. Fresh logs are /tmp/p242-272-current-verification-green.log, /tmp/p242-current-bill-authoring-retrieval.log and /tmp/p242-current-qbo-popup.log; failures remain separate. Real credentials, real provider/helper readiness, LastPass/passkeys, installed production and desktop-wide privacy remain outside this closure. No code under cli or crates changes.
