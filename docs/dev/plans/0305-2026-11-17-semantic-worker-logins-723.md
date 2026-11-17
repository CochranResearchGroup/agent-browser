# P305: Dependable semantic-worker logins

Product lane: PL-RECIPES
Status: OPEN
Consolidation: required
Spec: https://github.com/CochranResearchGroup/agent-browser/issues/723
Accountable human: ecochran76
Integration branch: recipes/spec723-integration
Integration base: bbbcc97e35edf48e5864c75b3dc49725e7c58485
Review fixed point: proposed integration base; operator question pending.

## Current State

The operator confirmed the spec design and explicitly directed publishing child tickets, root owning one ticket, one unblocked ticket per worker, ticket closure, then one final Standards/Spec review and one bounded TDD remediation pass. Existing saved-login, Native capture/input and Auth journey owners are reused. Source implementation is authorized; production replacement, live credential access and provider mutation are separate effects. Existing private/profile state remains outside Git.

## Consolidated batch

Deliver personal-account login verification, browser-first/LastPass guidance, honest Auth diagnostics and contextual guide discovery through existing public worker interfaces. Qualification distinguishes source, installed guidance, desktop readiness and actual credential/factor workflows. No second controller, credential store, guide registry or viewer. Preserve legacy organization requirements and original uncertainty. Existing public MCP/client seams and harnesses provide behavioral tests; avoid speculative fixture infrastructure.

## Delivery sequence and budget

Four independent source slices run within four agent slots; root integrates serially. #731 depends on validated #726, #728, #729 and #730. #732 depends on #731 and actual selected-runtime/account/unlock authority. Source tests cannot close installed/provider acceptance. Each substantive slice uses meaningful public red/green checks. Cumulative touched-surface gates run locally, never hosted CI. After tickets close, run exactly one broad review on separate Standards and Spec axes, adjudicate findings, then exactly one bounded TDD remediation pass and focused verification. Broad review count: 0. Post-review remediation count: 0.

## Worker assignments

| Ticket | Outcome | Blockers | Owner / branch | Owned scope | State |
| --- | --- | --- | --- | --- | --- |
| #726 | Personal-account prepare/verify | none | login_domain_facts / recipes/spec723-identity | playbooks identity contract, client types, existing verification smoke | validated; CLOSED |
| #728 | Browser autofill/LastPass pathway guidance | none | login_pathways / recipes/spec723-pathways | saved-login client, focused tests, all shared help/README/skill/docs-site guidance | validated; CLOSED |
| #729 | Auth recovery diagnostics | none | auth_continuation / recipes/spec723-auth | owner-login composition/types and public recovery tests | validated; CLOSED |
| #730 | Contextual guide discovery | none | root / recipes/spec723-integration | MCP observation enrichment and guide guidance module; parent playbooks edits only after726 freezes | validated; CLOSED |
| #731 | Integrated fresh-worker acceptance | #726, #728, #729, #730 | root after source integration | existing public workflow and installed/development guidance qualification | validated; CLOSED |
| #732 | Actual installed paths and factors | #731 | root; exact acceptance inputs required | private runtime receipts outside Git and public bounded disposition | pending |

Shared writer: #726 initially owns playbooks.rs; root supplies an isolated child module and applies the narrow parent integration after freeze. #728 owns shared user-facing docs; other workers supply bullets. Root owns package scripts, CONTEXT, plan/catalog and integration. All worker bases are the integration base above; results and validation are appended on integration.

## Custody

Primary/root checkout is /var/tmp/agent-browser-spec723-integration. Three task auxiliaries are /var/tmp/agent-browser-spec723-identity, /var/tmp/agent-browser-spec723-pathways and /var/tmp/agent-browser-spec723-auth. These implement the expressly requested separate-worker execution; source refs preserve all prior work. Initial inventory had11 existing directories and7 missing registrations, so the retained population exceeds the advisory threshold. No prior checkout was taken over, pruned, removed, reset or cleaned. Preserve dirty canonical Contacts work and retained candidate directories. Admission was serialized under the repository spec723 admission lock. Owned auxiliaries require governed closeout after integration and durable publication.

## Evidence and exit

The highest seam is public addressed-task MCP observation, playbook discovery/verification and existing Native/Auth client compositions. Observe actual output, intended-account predicates, task identity and effects, rather than private layouts or prose. Match Rust changes with cargo-safe focused tests, formatting and strict workspace Clippy; client changes with existing runtime and type checks; documentation with policy/link/build gates as applicable. Worker identity owns the initial Cargo target; root reuses it serially after freeze to avoid duplicate compilation.

Exit: tickets have truthful validated/blocked and tracker dispositions, one source-bound integration branch is published, actual unqualified external criteria remain visible, and no synthetic result is promoted into installed/live acceptance. PR724's two confirmed domain definitions are incorporated into integration; its standalone PR remains separately open until supersession/integration. No merge or production effect is inferred.

## Integrated checkpoint

Workers returned identity179d89eae, pathways5597ad6af and Authefc15fbb4, integrated as7f07f207e, c6d3e8a2c and5a7d98a3a respectively. Root contextual discovery is4b6a0df14 with formatting/compact-output followup pending. Canonical main advanced toed62bb3a3 through PR727; it is merged into integration without copying foreign working-tree bytes. The original implementation/review baseline remainsbbbcc97e35edf48e5864c75b3dc49725e7c58485 pending operator fixed-point confirmation.

#729 is tracker CLOSED after integrated pnpm test:playbook-client passes, including diagnostics and retained service-window checks plus type coverage. Integrated pnpm test:native-saved-login also passes. Pathway worker docs build and link checks pass; its MDX source is unchanged by integration. Changed documentation links and policy wiring pass. No installed/provider claim follows.

Identity and contextual Rust source are implemented, not validated. Default and lean Cargo attempts were never admitted; root exclusively owns the transferred target. Current owned build waits for memory admission behind a foreign release build; disk16GiB is also below the unchanged20GiB reserve. No resource reserve was lowered, foreign claim stopped or foreign artifact removed. Root formatted the new isolated Rust module directly; required full cargo-safe format and strict Clippy remain pending.

Public personal baseline rejected the new guide as siteguide_invalid with exact owned cleanup. Contextual baseline attempts stop before the intended assertion atbrowser_task_operation_process_unverified, including one exact-ELF correction; cleanup proves zero owned residue. Do not call this the feature RED or repeat it without a concrete prerequisite change. The current external Auth/source composition fixture returnsauth_not_authorized before its expected approval; unchanged base reproduces it, so retain producer/fixture compatibility as an unqualified boundary rather than suppressing a denial. Actual installed acceptance #732 still needs selected task/profile/site/account and applicable unlock authority; the operator question is pending. Broad final review0; post-review remediation0.

## Current bounded result

Source checkpoint efe5388a1 integrates all four source slices. #728 and #729 are source-validated and tracker CLOSED. #726 and #730 remain OPEN because candidate-backed GREEN and cumulative Rust gates are missing. #731 remains dependency-blocked; #732 is needs-info/dependency-blocked. Parent723 remains OPEN. Draft PR733 owns integration; no merge, executable install or actual-provider acceptance has occurred.

A focused #728 regression found that unverified Native readiness skipped browser inspection. Worker43d5fc913 corrected ordering; integration then caught that native_ready is owner-internal, not a public MCP action. A separate public-operation RED/GREEN now recommends supported native_capture. Current saved-login11-test/type lane, playbook diagnostics/types, policy wiring, changed links, patch hygiene and current docs build pass. All touched Rust files pass a direct rustfmt parse/format check. Required cargo-safe format and strict workspace Clippy each return75 before execution, reasonmemory_pressure; they are missing checks, not passes. The owned build never admitted and its waiting wrapper was stopped; no background compiler or foreign claim was stopped. Disk remains below the unchanged20GiB reserve, so lowering memory alone would not establish admission.

The existing development skill publisher and separate readback qualify only the task-owned pseudo-home copy. Source and target SHA256 are both4660eccc7b8403c0af47091eddd8088abe49f5c1945e64a4bebeba659efce0c0; production/shared skill and browser runtime were not changed. This proves guidance propagation, not execution of the installed credential paths.

Bounded source diagnosis explains why the existing headless public fixture cannot yet prove GREEN: it selects the Local runtime adapter after stripping Native provider configuration, whereas command admission requires positive original-process custody. Native launch custody or supported app adoption supplies that proof; the Local fixture does not establish it. The installed personal RED stopped earlier in guide metadata parsing and never exercised this guard. Do not insert synthetic database identities, borrow production provider configuration, weaken process verification or repeat failed fixtures. Use an already-qualified owned Native/app-attachment seam when available; retain this current prerequisite rather than building a parallel fixture framework.

Both operator questions remain pending: final review fixed point, and exact actual acceptance task/profile/site/account with applicable unlock authority. Final broad review count0 and post-review remediation count0 remain reserved until ticket closure. Resume source validation from this integration checkout and current remote tip; reuse the passed JS/docs evidence within unchanged scope.

### Checkout handoff

All three owned worker auxiliaries are cleanly removed after exact remote SHA and integrated-source custody checks. Terminal governed operations: Auth78a65fa5-887d-401e-8636-a15c7b1e7296, pathways65eb1b1b-2564-4622-8545-b28ef665a880, identity8c4ecc58-1c79-4e78-8482-f03af41ab738. None had active or pinned candidates. Branches/results and outside-Git validation receipts remain retained; unresolved identity/public acceptance continues from the root integration checkout. The root-owned duplicate guidance fixture was verified equal to its committed canonical copy before removal from the identity auxiliary. No foreign worktree, candidate, registration or process was changed. Only the root checkout is retained for this task; no owned build/test waiter remains.

## Resumed source qualification, 2026-11-17

This checkpoint supersedes the earlier resource and fixture blockers above. Main48174c314 is merged as32c8f1a41. Worker adoption and identity guards are integrated as9298c65d6 and8db1dbbfe. #726 and #730 are CLOSED. #728 and #729 remain CLOSED. Root completed #731 (CLOSED); #732 remains awaiting an actual selected signed-out task/profile/site/account and applicable unlock authority. No other ticket is independently executable.

The candidate compiled from32c8f1a41 passes cargo-safe build, full format and strict workspace Clippy. Candidate SHA256:73305ab87bb2a384873814b1266dc70a129ddf36fa4c6703672ad70ebcaa7742. Later changes are JavaScript and evidence only. The public fixture now adopts its own real Chrome child using the public existing-profile path; it does not manufacture process identity or weaken admission. Personal, organization, changed-account, changed-control, evidence and real-expiry guards pass. Contextual guidance passes observed-provider, authored-qualification and ordinary-fallback checks. Every browser fixture reports zero owned process residue.

The integrated HTTPS synthetic-site journey passes through public MCP discovery and actual Auth/source continuation, preserving one task/profile/browser/tab. It reports14 browser calls,7 Auth calls, one journey, one provider selection, one synthetic return, zero credential uses and zero approval prompts. Rendering unreadiness cannot pass website verification; wrong website account holds; corrected external return completes the original journey and independent SiteGuide verification. Provider transport is explicitly synthetic. Existing diagnostics cover unavailable sources, original-request recovery and unknown outcomes. These results do not qualify actual Native autofill, LastPass or live Auth factors.

Source composition uses clean Auth/SDK57d6e5e301a0312f3c7cbe0da5e58efe3540c59a and qualified Receipts companion5783fea7495983c1b3947bdf0bd118da4aa71519. An older companion rejected nullable expiry before approval; the supported companion passes the actual source composition tests. Pending approval-channel refusal now retains its original request as an integration diagnostic; unknown effects take precedence over denial-shaped codes and prohibit fallback/replay. Public diagnostic RED/GREEN receipts are outside Git.

Local evidence: /var/tmp/spec723-resume-build.log, /var/tmp/spec723-resume-fmt.log, /var/tmp/spec723-resume-clippy.log, /var/tmp/spec723-guidance-adoption-green.log, /var/tmp/spec723-identity-personal-first-green.log, /var/tmp/spec723-identity-personal-plan-guards-green.log, /var/tmp/spec723-identity-expiry-time-receipt.json, /var/tmp/spec723-owner-qualified-parent-green.log, /var/tmp/spec723-owner-recovery-qualified-parent.log and /var/tmp/spec723-composed-journey-first.log. The task-owned installed skill copy qualification above remains applicable because its source is unchanged. No hosted CI, production installation or live-provider claim.

Final broad review count:0. Post-review remediation count:0. Both remain reserved after ticket closure. The code-review skill requires a user-selected fixed point; that question and #732 target selection remain pending. Stable Time qualification at this checkpoint:2026-11-17T18:25:43.273668082Z, NTS-verified finite bound. Memory disposition:forbidden, no explicit memory-write request.

## Installed qualification repair packet

The operator selected an actual existing-account business workflow for #732 and accepted the original integration base as the final review fixed point. Private selected-site/account/task evidence is outside Git. Installed acquisition and same-browser page observation pass; authentication/company verification has not completed. No private login input, MFA trigger or business write occurred.

Actual Native observation exposes a distinct owner-control gate: Native desktop owner is operator while cooperative task owner is agent. No automatic Native takeover or control-epoch substitution is permitted. An operator-return request uses the existing ready opaque handoff. Missing capture qualification must not be reported as missing credentials or provider denial. #737 owns actionable Native diagnostics, root assigned. Actual Auth discovers an eligible credential but preserves one original pending request with receiving_target_unavailable; #738 owns the receiving bridge, identity worker assigned one ticket. No replacement approval or request replay. Parent #732 remains OPEN until actual criteria are established.

Current root source is2a3c1f914. The sole retained worker auxiliary is reused on recipes/spec723-auth-receiver after preserving its earlier branch remotely. No foreign checkout or runtime reset. A fresh task-scoped local MCP connection restores tools after the client transport closes; it reuses the same physical browser. New repair scope is diagnosis/source only until exact effects are separately qualified. Current free disk is below unchanged Cargo reserve, so no unadmitted Rust build can count as validation. Broad final review0; post-review TDD remediation0. Memory disposition remains forbidden.

#737 source repair distinguishes exact Native operator control from changed identity without releasing control or dispatching capture/input. All five guidance surfaces are updated. The existing public Native integration harness gains an operator-held regression. Direct rustfmt succeeds; policy wiring, documentation links and patch hygiene pass. Focused cargo-safe integration test and required cargo-safe format each return75 before execution, reason memory_pressure; they are missing checks, not passes. No candidate rebuild or installed change has occurred. Source remains unvalidated until required Rust gates run. #738 additionally isolates an original failed read-only Native observation retained as uncertain, which prevents Auth receiving observation before private use. Do not erase that record or conflate it with uncertain password input.

Bounded admission followup uses the previously qualified1GiB format and8GiB focused-test claims, retaining all host reserves; both refuse75 disk_pressure before execution. No test failure or pass is inferred. Updated guidance is separately synced/read back in the task-owned development pseudo-home, source and target SHA2560b7138a862d11a2931203ad3c424d12cc2bcbedf1e9305575e816b3c0c50a44a. This supersedes the older guidance hash only; production/shared skill and executable are unchanged. #738 proposed future-only acknowledged no-effect finalization reuses existing operation states and must preserve failed response/retry prohibition; legacy proofless records and uncertain input remain held.

#738 workerbe1f4b726471fdd9656343bfb3859f72fa5ed0a1 is integrated as91c0c1818. Future known Native operator-control refusal preserves a terminal failed/no-effect receipt and remains non-replayable. Generic errors, lost responses, misattribution, old proofless records and native_input stay uncertain. Existing public tests cover those boundaries; direct rustfmt/patch hygiene pass, but Rust tests/Clippy and required wrapper format are unrun due disk admission. All source repairs remain IMPLEMENTED_UNVALIDATED, tickets737/738 OPEN. No live state is backfilled, Auth request renewed/replaced or credential input dispatched. Worker branch is remotely preserved and clean; auxiliary retained for the pending validation packet. Review0/remediation0 unchanged.
