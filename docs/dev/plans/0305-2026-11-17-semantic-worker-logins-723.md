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
| #726 | Personal-account prepare/verify | none | login_domain_facts / recipes/spec723-identity | playbooks identity contract, client types, existing verification smoke | running |
| #728 | Browser autofill/LastPass pathway guidance | none | login_pathways / recipes/spec723-pathways | saved-login client, focused tests, all shared help/README/skill/docs-site guidance | running |
| #729 | Auth recovery diagnostics | none | auth_continuation / recipes/spec723-auth | owner-login composition/types and public recovery tests | running |
| #730 | Contextual guide discovery | none | root / recipes/spec723-integration | MCP observation enrichment and guide guidance module; parent playbooks edits only after726 freezes | running |
| #731 | Integrated fresh-worker acceptance | #726, #728, #729, #730 | root after source integration | existing public workflow and installed/development guidance qualification | pending |
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
