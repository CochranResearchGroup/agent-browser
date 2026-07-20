# Development Plan

This is the developer handoff home for the repo-policy skill family.

## Current state

The repo-policy tooling now has three generally reusable pieces plus one repo-local maintainer tool:

1. purpose-aware policy library in `agent-policies`
2. deterministic selector in `repo-policy-selector/scripts/select_policy.py`
3. deterministic planning audit helper in `repo-policy-selector/scripts/audit_planning_contract.py`
4. repo-scoped maintainer harvester in `.codex/skills/repo-policy-harvester/scripts/harvest_policy.py`

## Current supported policy families

- `product-engineering`
- `writing-project`
- `workspace-agent`
- `library-cli`

The strongest implemented families today are:
- `product-engineering`
- `writing-project`

## Current shared modules worth preserving

### Product-engineering

- `planning-discipline`
- `roadmap-runbook-governance`
- `architecture-guardrails`
- `documentation-change-control`
- `git-worktree-hygiene`
- `turn-closeout`
- `validation-and-handoff`

### Writing-project

- `planning-discipline`
- `turn-closeout`
- `writing-environment-aware-workflow`
- `writing-authoritative-deliverables`
- `writing-document-edit-safety`
- `writing-review-evidence-discipline`

## Important recent work

Recent commits from the original `agent-skills` repo for this family:

- `c4eb94c` Add repo policy selector and harvester skills
- `a3c3dac` Make repo policy selector purpose-aware
- `de7e5a6` Calibrate selector for review and analysis workspaces
- `927d09e` Make policy harvester purpose-aware for writing projects
- `064198f` Reuse shared writing-project policy modules in harvester
- `e8c5267` Recommend full writing-project policy modules
- `50b4ee1` Add product-engineering policy harvesting
- `6a769a4` Refine engineering policy detection for shared seams
- `3d01cbb` Add deterministic planning contract audit

## Concrete adoption examples

### Product-engineering

- `/home/ecochran76/workspace.local/litscout/AGENTS.md`
- `/home/ecochran76/workspace.local/google-messages-cli/AGENTS.md`

### Writing-project

- `/mnt/h/My Drive/Project Management/Proposals/2026-02-26 USDA AFRI SAS Strengthening Agricultural Systems/AGENTS.md`
- `/mnt/h/My Drive/Project Management/Proposals/2026-05-19 NSF TTP/AGENT.MD`

## Planning-contract position

The current planning standard is now explicit:

- `ROADMAP.md` is the master plan
- `RUNBOOK.md` is the dated turn log
- actionable plans live under `docs/dev/plans/`
- roadmap headings should use `P## | <Lane Title>`
- runbook headings should use `Turn N | YYYY-MM-DD`
- plan filenames should use `0001-YYYY-MM-DD-plan-slug.md`
- plan states should be deterministic:
  - `PLANNED`
  - `OPEN`
  - `CLOSED`
  - `CANCELLED`

Use:

```bash
python scripts/audit_planning_contract.py --repo-root /path/to/repo --json
```

to audit that contract.

The auditor now derives applicability from adopted planning policies. Use
`--force` for a pre-adoption assessment, path flags for documented alternate
authorities, and `--active-only` for a steady-state gate that explicitly reports
unclassified legacy exclusions.

## Next best work

1. Use the selector and audit helper on more real repos before adding more abstraction.
2. Add new policy families only when at least two real repos show the same reusable pattern.
3. Prefer improving adoption docs and heuristics over adding more modules prematurely.

## Avoid

- mixing workspace-agent memory rules into normal engineering repos
- forcing the strict planning contract onto a mature repo without a bounded migration
- adding broad new policy families without concrete adoption examples
