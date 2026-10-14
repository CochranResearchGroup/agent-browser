# Website automation source study

Product lane: PL-RECIPES
Disposition: active-input
Owning plan or work item: GitHub issue #199
Related lanes: PL-AUTH, PL-PLATFORM

Adopted by [P241](../plans/0241-2026-10-14-website-playbook-model.md) for a
synthetic model prototype. The operator accepted the shared vocabulary and
BILL/QBO first-delivery direction; detailed production contracts remain open.

## Question and bounds

How can existing social retrieval routines and QBO/BILL login guidance become
reusable website automation that deterministic software and agents can compose?
The operator selected login as the first major workflow and assigned desktop
input, capture, audio/video and desktop events to Remote View.

This is a source study and proposed model, not an implementation specification.
Sources are local source code, maintained skills and synthetic test definitions.
No browser, authentication, message retrieval or provider effects were executed.
No live account qualification or test-suite pass is claimed. Tenant-specific
accounting facts, credentials and private artifacts are excluded from this note.

Read source at Agent Browser `cb765a7be2719383a54fb849eac9691ce0197b06`,
Books Receipts `a65aa5b19af9df0d8de1033045bf918be89bd05f`, and Last30Days
`a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1`. Both source repositories were
clean at inspection. Installed skills are separate, mutable source material.

## What the sources establish

| Source | Reusable behavior | Keep specific to the consumer/site |
| --- | --- | --- |
| QBO logon skill | Resolve intended identity; use existing task/profile; distinguish visible browser from authenticated company; refresh tabs after password-manager launch; select fresh matching MFA evidence; preserve peers | Intuit page variants, company identity, mailbox binding and account-recovery presentation |
| Books Receipts authentication runner | Observe, classify, choose bounded actions, verify receipts and observe again; deterministic action IDs; private credential/challenge results | Provider adapters and existing capability/custody names |
| BILL authentication wrapper | Bind current task and exact organization; accept fresh receiving proof; reject stale/wrong-company proof; reconcile uncertain submission before replay | BILL organization paths, selectors and accounting continuation |
| Agent Browser BILL recipe | Versioned selectors, form labels, origins, login entry fallback and page-state classification | Current loader and validator are explicitly hard-coded to BILL |
| Last30Days Reddit/X/Facebook adapters | Site-specific auth/page probes; bounded search/scroll/extraction; normalized results; distinct empty, malformed, blocked and partial outcomes | Social post ranking, date/relevance filtering, API-specific data models and site extraction selectors |
| SoyLei monthly-closeout skill | Compose provider routines and verify downstream readbacks; preserve exceptions and distinguish different operation outcomes | Tenant accounts, periods, destinations and accounting decisions |

Primary sources:

- [QBO logon skill](/home/ecochran76/.codex/shared/skills/books-receipts-qbo-logon/SKILL.md) and [browser/email reference](/home/ecochran76/.codex/shared/skills/books-receipts-qbo-logon/references/browser-and-email.md).
- [QBO operations skill](/home/ecochran76/.codex/shared/skills/books-receipts-qbo-operations/SKILL.md): browser authentication, OAuth token readiness and read-only API work are separate outcomes.
- [Generic authentication runner](https://github.com/CochranResearchGroup/books-receipts/blob/a65aa5b19af9df0d8de1033045bf918be89bd05f/src/books_receipts/browser_authentication_recipe.py), especially policy, candidate actions and safe action receipts.
- [BILL wrapper](https://github.com/CochranResearchGroup/books-receipts/blob/a65aa5b19af9df0d8de1033045bf918be89bd05f/src/books_receipts/bill_spend_closeout/authentication.py) and [authentication tests](https://github.com/CochranResearchGroup/books-receipts/blob/a65aa5b19af9df0d8de1033045bf918be89bd05f/tests/test_bill_spend_closeout_authentication.py).
- [Existing BILL site recipe](../../../cli/src/native/site_login_recipe.rs).
- [Reddit adapter](https://github.com/CochranResearchGroup/last30days-skill/blob/a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1/skills/last30days/scripts/lib/reddit_browser.py), [X adapter](https://github.com/CochranResearchGroup/last30days-skill/blob/a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1/skills/last30days/scripts/lib/x_browser.py), and [Facebook adapter](https://github.com/CochranResearchGroup/last30days-skill/blob/a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1/skills/last30days/scripts/lib/facebook.py).
- [Reddit fixtures/tests](https://github.com/CochranResearchGroup/last30days-skill/blob/a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1/tests/test_reddit_browser.py) and [X fixtures/tests](https://github.com/CochranResearchGroup/last30days-skill/blob/a0cdcfb75a9e90a4a31bfe9f48f0ef12f4b461a1/tests/test_x_browser.py).

The SoyLei skill was inspected only for its composition pattern. Its body and
private references are not reproduced or admitted into this product note.

## Proposed shared vocabulary

The operator accepted these terms after the source study. P241 records their
definitions in CONTEXT.md and supplies a synthetic model prototype.

**SiteGuide:** versioned knowledge for recognizing a website's states, locating
controls, interpreting results and verifying an intended outcome. It includes
agent-readable explanations and machine-usable assertions. It contains no
tenant credentials or runtime account selections.

**Routine:** an executable reusable subtask with declared inputs, expected state,
result and effect semantics. It may navigate, fill nonsecret fields, invoke a
private credential capability, collect results or verify a destination.

**Playbook:** composition of routines to reach a goal. A deterministic program,
an agent using a skill, or a hybrid controller can choose the next routine.

**Automation skill:** guidance for selecting routines and interpreting unexpected
states. It references the same SiteGuide and routine contracts as software;
it must not become a separately maintained competing recipe.

Keep these distinct: site knowledge, executable actions, composition, and agent
guidance. A SiteGuide alone does not execute a workflow; a credential-fill
receipt alone does not prove successful login.

## Common execution model

Observe current task, classify site state, choose a suitable routine, execute,
verify the receiving state, and retain progress for continuation. Expected
states run deterministically. An unfamiliar state yields current evidence and
available capabilities to the agent, which can adapt or obtain human help.

Routine results need to distinguish completion, partial completion, no effect,
uncertain effect, waiting for a helper/person, and changed site behavior. Keep
task identity and checkpoints shared across deterministic and agent execution.
After an uncertain submission, read back before deciding whether another action
is appropriate. A stale control is grounds to observe again, not to repeat an
old click. The executing routine consumes a frozen action plan; refreshed
observations create a new plan at the playbook/controller layer.

Do not start with a new scripting language or arbitrary workflow interpreter.
First prove these contracts with ordinary executable routines and matching
agent guidance. Choose a higher-level coding surface from demonstrated needs.

## Ownership and migration

Remote View supplies desktop interaction and capture. Agent Browser supplies
browser task/profile continuity, website observation and reusable automation
composition. Missing desktop capabilities become Remote View tickets.

The [shared auth-helper spec](https://github.com/ecochran76/grok-bots/issues/21)
owns protected storage, credential access, private delivery and MFA adapters.
[Issue23](https://github.com/ecochran76/grok-bots/issues/23) and
[draft PR34](https://github.com/ecochran76/grok-bots/pull/34) establish dummy
LastPass progress; real LastPass acceptance remains open. Protected entry lookup
does not prove browser-extension detection, autofill or passkey provisioning.
Agent Browser supplies site recognition and receiving assertions; agree which
helper executes those assertions instead of implementing duplicate verifiers.

Books Receipts retains accounting operations, QBO OAuth/API clients and business
postconditions. Last30Days retains retrieval ranking and research semantics.
Their browser workflows should consume shared routines rather than import
their entire business domain into Agent Browser.

The [BILL Agent Browser client](https://github.com/CochranResearchGroup/books-receipts/blob/a65aa5b19af9df0d8de1033045bf918be89bd05f/src/books_receipts/agent_browser_bill_authentication.py)
still calls the retired `service_request` MCP tool. This is source-visible
transport drift, not a demonstrated live failure in this study. Preserve the
identity, replay and verification behavior while adapting to supported CLI/API
or workflow interfaces. Social adapters also retain `rdp_gateway` and raw
provider-link expectations. Do not adopt that plumbing as the new contract.

## First proving slice

Build one shared login routine contract and exercise it with both BILL and QBO
SiteGuides. The second site is necessary to expose BILL-specific assumptions.
Use synthetic page states and private helper doubles before selected installed
qualification. Keep real credentials and account actions in separately
authorized acceptance.

Cover existing authenticated session, identifier/password forms, password-manager
availability and locked state, helper filling, fresh MFA correlation, new-tab
launch, changed controls, wrong company, uncertain submission, resume and peer
preservation. Import fixtures by behavior with synthetic identities; do not copy
private recordings or entire source implementations.

Then use one Reddit search/extraction routine to test whether the shared model
works beyond login. Preserve explicit no-results versus empty extraction,
bounded pagination/scroll, malformed page and partial-result semantics.

The general "how to automate websites" guidance should teach: inspect the
current supported interface; bind a task and expected identity; define success;
recognize states; implement the stable subtask; use semantic locators; verify
effects; preserve progress; adapt from fresh evidence; and turn a repeatedly
successful sequence into a tested routine plus SiteGuide.

## Open design decisions

Define the minimum SiteGuide/routine/playbook contract; align the auth
helper's receiving-target interface; determine how agents propose and test guide
changes; distinguish using existing passkeys from provisioning new ones; and
choose the first synthetic fixture set. This research does not make #199
ready for implementation or resume historical authentication acceptance gates.
