# P252 workflow entry-point budget repair

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: [P252](../plans/0252-2026-10-20-workflow-skill-budget.md), CochranResearchGroup/agent-browser#325
Related lanes: P251, UX301

## Outcome

The workflow entry point shrinks from 17,670 to 9,163 UTF-8 bytes under the unchanged 10,000-byte limit. Ordinary task creation, creator attribution, task identity, public tool catalog, human handoff and continuation remain inline. Dashboard, profile-sharing, detailed SiteGuide and summary-diagnostic guidance moves to the existing CLI/service reference behind explicit branch-trigger links. Creator attribution appears before the task example. No workflow semantics or public API changed.

## Qualification

The original `pnpm test:service-api-mcp-parity` fails only the workflow-size assertion. After the split, it passes 66 browser controls, 26 service CLI/API adapters, 19 service resources, 103 native service actions and 120 service-request actions. The assertion and implementation are unchanged. The full service-client bundle and existing development presentation-provider fixture pass.

A paragraph-preservation comparison finds every original guidance paragraph in the entry point or reference, accounting for the relocated routine guide's relative link, a promoted creator-context heading and an explicit setup pointer. All skill and reference local links resolve. Selected P252 planning and patch hygiene pass. No additional implementation-mirroring test was introduced for this documentation move.

The existing development publisher atomically copies the complete skill directory. Development skill sync and status pass with equal complete-directory digest `02b70f00545012776a9fdfbfe92a0f41bdd04fa357082cb5fa741e63a1c243b1`, covering references alongside SKILL.md. The final validation selector also requires installed guidance equality. The shared entry point differed only by the previously installed P251 Properties description; its reference matched the baseline exactly. The two reviewed files were published through a staged copy preserving unrelated installed files, with the complete prior skill directory retained in user-scoped backup. Exact source/installed comparisons pass. No binary publication, browser launch, runtime restart, provider mutation, formal release or CI dispatch occurred. Product executable source is unchanged; no Rust or dashboard build is claimed for this documentation-only repair.

## Custody

Reuse the dashboard primary on `platform/workflow-skill-budget`; preserve the foreign stale-session and resource worktrees. Retain the primary and pinned P250/P251 candidates for continuing dashboard work. Private comparison and publication receipts remain in user-scoped state outside Git.
