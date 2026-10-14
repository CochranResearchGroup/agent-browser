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

Freeze the branch for one whole-branch Standards and Spec review against
cb765a7be2719383a54fb849eac9691ce0197b06 and spec271. Adjudicate findings, perform
one bounded remediation, verify accepted blockers/critical regressions, and
retain precise final source/installed identity and pending broader gate results.
