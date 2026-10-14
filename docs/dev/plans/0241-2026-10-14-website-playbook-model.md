# P241: Shared website playbook model prototype

Product lane: PL-RECIPES
Work item: #199
Branch: prototype/website-playbooks-199
Baseline: cb765a7be2719383a54fb849eac9691ce0197b06
State: COMPLETED

## Bounded outcome

Record the accepted shared contract and build one self-contained, synthetic
interactive logic prototype. It asks whether SiteGuide knowledge, executable
routines and agent/deterministic playbook composition can share progress and
verification across BILL and QBO login. It is throwaway source preserved on
the prototype branch, not production automation.

## Accepted direction

The operator accepted one shared contract for software and agent skills. First
production delivery should include BILL and QBO login playbooks and a documented
way to develop, test and adapt SiteGuides. Reddit retrieval later checks whether
the contract generalizes beyond login. Production specification and ticketing
follow model review; they are not part of this prototype packet.

Remote View owns desktop input/capture. Shared grok-bots auth helpers own private
credential/MFA operations. Agent Browser owns website state, task continuity,
routine composition and agent adaptation. No new caller authority ceremony.

## Custody and overlap

Use the existing primary checkout. Its two research-note edits are owned by this
conversation and move onto the prototype branch. Preserve the dashboard and
enrollment checkouts, candidates and branches. P241 owns only domain vocabulary,
its plan/index/catalog entries, research note and prototype HTML. P237 retains
#260/#261 and all dashboard/native lifecycle code. No source changes in
books-receipts, last30days-skill, grok-bots or Remote View.

The skill-specific prototype branch naming overrides the normal recipes prefix.
Keep this branch out of main; preserve its locator on #199. No production PR,
release, deployment, credential access, provider action or reboot.

## Prototype cases

Normal login and existing login; MFA wait and continuation; changed controls and
agent adaptation; uncertain submission followed by readback; wrong-company
result; peer work preservation. Site selection and executor mode share the same
routine/result structure. No secrets, HTTP calls, external libraries or durable
browser state. All page state lives in memory.

## Validation and review

Inspect every guided scenario and relevant illegal transition in the demo,
check JavaScript syntax, local links, active planning audit and patch hygiene.
No prototype unit tests or Rust build. Publish one browser review session.
Preserve the artifact on the prototype branch and report that model acceptance
still requires operator review. Adopt the [source study](../notes/0241-2026-10-14-website-automation-source-study.md)
as this packet's design input.

## Artifact closeout

The [interactive prototype](../prototypes/0241-website-playbook-model.prototype.html)
is delivered as a single in-memory HTML file. The accepted domain terms are
recorded in CONTEXT.md. Both inline scripts parse. Scenario rehearsal covers
both site choices; installed Playwright Chromium renders the page and drives
the walkthroughs. No persisted prototype unit tests were introduced.

The first visual capture using the installed stealth Chromium crashed at
`page.screenshot`. This receipt is retained; successful rendering uses the
separate installed Playwright Chromium. Neither observation proves a real
Remote View, authentication helper, or website integration.

Existing docs-build evidence is reusable: no docs application code or dependency
changed after that pass. Local links, active planning audit and patch hygiene
are checked for the final artifact set. This packet completes prototype
delivery, not acceptance of the model. Operator review decides the next
specification; no production implementation or deployment is claimed.
