# P296 image rounds472 source checkpoint

Status: SOURCE QUALIFIED, RUNTIME BLOCKED
Product lane: PL-CHALLENGE, integrated under PL-RECIPES P296
Tracker: [472](https://github.com/CochranResearchGroup/agent-browser/issues/472), blocker [484](https://github.com/CochranResearchGroup/agent-browser/issues/484)
Integration: recipes/spec466-integration / draft PR480

## Changed behavior

The separately granted hcaptcha-image-grid-3x3-two-round-v1 profile accepts at most two complete static English bus grids. Fresh candidate IDs and all nine changed interior hashes are required for round two. Both rounds share the first operation's original60-second deadline, one attempt and10-click/30-pointer-event budget. The first result stays immutable; later results retain cumulative state. Context, policy, stale evidence, partial refresh, rejection, cancellation and uncertain delivery stop automation without replay.

The receiving client exports createNativeImageRoundsChallengeClient from the existing native-image-challenge entry. The selection port receives each exact Native PNG and current roundIndex. Selection supplies untrusted IDs; the owner derives input authority and coordinates. Independent application checks distinguish CAPTCHA clearance from protected access.

A public revision-drift regression reproduced an overly strict binding comparison. The repair excludes only advisory observedServiceRevision from cloned prior/current objects, retaining exact comparison of every other field, including unknown fields. Real policy changes still alter the digest and refuse. No clock guard was weakened.

## Exact source and private candidate

- Feature worker55bdeb525279f93ff6db24c0c4b54465d8b44478, integrationd59bda7a0.
- Shared metadata7cc6b9965, feature documentation207ab5b5e and owned qualifier97e41a3afb62fab7e9ab7e0d1f8e552cbf44e27d.
- Revision repair worker7a359a33d8730116517f60fa0083669d455825f5, integrated0b1c9638d89ce4694e457361b46eddb4c2a6c377.
- Repaired ci candidate SHA256ba93db054d40d5f85d9751e10e120ac42f5815cbdb07578b050219995126436c. Sealed private locator /tmp/agent-browser-spec466-implementation/472-repaired-pin.json; bytes retained beside its immutable pin. No production installation.
- Native sourcea530a970d7b5c8c6e35284cd3840c63881d786c8, SHA25698ec819840ba49a6f54766267e501954f58dc11a28579fb327b71d4d66025651, already integrated via Native PR690. The debug candidate remains private and uninstalled.

## Validation boundaries

| Evidence | Identity | Outcome and limit |
| --- | --- | --- |
| Public durable RED/GREEN, legacy and rounds protocols | Feature55bdeb / root97e41a3af |165 MCP tests,23 Native-control fixture tests, client matrix and strong types pass; includes interior hashing, cumulative deadline and no-replay boundaries |
| Revision-only drift RED/GREEN and actual policy-change negative | Repair7a359a33d / root0b1c9638d |11 focused image tests pass; revision changes without digest changes continue, actual policy change refuses |
| Format and strict workspace Clippy | Root0b1c9638d |Pass; optimized ci candidate build passes |
| Documentation, API/MCP parity, handoff docs, links | Root97e41a3af and documentation checkpoints |Pass; six initial changed documentation files plus later bounded ledger links. The prior broad tests are reused only for unchanged surfaces, with fresh focused coverage for the repaired owner guard |
| Actual051 | Agent Browser97e41a3af/5ccb7bcc, same Native98ec |Two-round protected access passes with8Native clicks; partial refresh and rejection stop after4each. Lost-second fault not reached because of policy-binding refusal. Whole run INCOMPLETE; not repaired-candidate acceptance |
| Actual052 | Agent Browser0b1c9638d/ba93db05, same Native98ec |Capture completion future_timestamp1172ms; no selection or CAPTCHA input. INCOMPLETE, cleanupFailure null, adapterErrors empty |
| Actual053 | Same executable and fixture; documentation-only source checkpoint3815d4f7d |Capture completion future_timestamp1220ms; no selection or CAPTCHA input. INCOMPLETE, cleanupFailure null, adapterErrors empty |

052 observedAtMs1793092494901 exceeded validationNowMs1793092493729.053 observedAtMs1793092803710 exceeded validationNowMs1793092802490. The cause remains unresolved. NTP reports synchronized; relevant journals contain no entries. A read-only336-sample/28-CPU affinity clock check showed no current backward readings, which neither explains the failures nor proves stability. The older042 failure remains separately retained, not retrospectively repaired or attributed.

## Cleanup, custody and exact remaining gate

Both failed repaired-candidate trials explicitly released the acquired assignment in final inventory and authoritative Native pools while preserving the peer. The first053 census saw the coordinator's still-running read-only selection waiter; it did not qualify. After that waiter exited, the fresh001–053 census qualified1420 recorded process generations absent, zero unknown generations, no owned positive processes, Docker resources or units. Retained build images and older offline assignment obligations are separate from process absence.

The final admitted trial was053. The runtime retry tactic has ended; no fourth trial or fresh expensive build is admitted by this checkpoint. [484](https://github.com/CochranResearchGroup/agent-browser/issues/484) owns bounded causal timestamp diagnosis before another qualified consolidated candidate cycle.472 stays OPEN. Ten of the original twelve children are closed;478 still requires actual narrated media and exact portal/account/task/effect scope. The single final Standards/Spec review and post-review TDD remediation remain unused0/0; fixed review base remains pending.

Worker source is clean and integrated into the draft branch. Its auxiliary and old branches remain retained pending canonical PR480 integration; Native candidate custody is also retained. No production, provider mutation, CI dispatch or universal hCaptcha qualification is claimed.
