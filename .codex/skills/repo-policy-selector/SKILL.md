---
name: repo-policy-selector
description: choose and adapt the right reusable agent-policy modules for a target repository by inspecting repo signals such as AGENTS.md, docs/dev/policies, roadmap/runbook files, repo shape, and workflow complexity, then recommend a policy profile from this policy library and draft the local policy patch.
---

Select the right reusable policy bundle for a repository, then adapt it into repo-local guidance.

## Workflow

1. Confirm the selector has an installed policy library available locally.
2. Deterministically enumerate the installed policy library from `catalog.yaml` before making recommendations.
3. Inspect the target repo lightly:
   - `AGENTS.md` if present
   - `docs/dev/policies/` if present
   - roadmap/runbook/progress files if present
   - cluttered or legacy planning/note surfaces that may need migration into canonical locations
   - repo shape and workflow complexity
4. Run `scripts/select_policy.py` for a deterministic first-pass profile/module recommendation.
5. Extract the repo's current policy surfaces deterministically before drafting adoption changes.
6. If the repo shows cluttered or legacy plans, notes, or memories, treat migration as part of adoption rather than patching only the final steady-state policy.
7. Classify existing policy surfaces against the installed templates:
   - `keep`
   - `merge`
   - `retire`
8. If the repo adopts planning discipline, run
   `scripts/audit_planning_contract.py` to check the applicable contract.
   Supply `--plans-dir`, `--roadmap-path`, or `--runbook-path` for documented
   alternate authorities. `--active-only` is a steady-state gate and reports
   unclassified legacy exclusions; it does not complete migration. Use
   `--force` only for pre-adoption assessment.
9. If the repo adopts `goal-execution-governance`, run `scripts/audit_planning_contract.py --goal-only` and require concrete local bounds and checkpoint fields.
10. Validate that the recommended profile and modules exist in the installed library bundle before drafting changes.
11. Read the referenced policy modules from this policy library before drafting changes.
12. Decide whether the repo needs:
   - a starter profile with minimal edits
   - a profile plus module overrides
   - a missing-modules patch when the repo already partially or mostly matches the selected profile
   - a migration-first adoption because plans, notes, or memories are cluttered
   - a custom composition because no single profile fits cleanly
13. Draft the repo-local policy patch or recommendation, keeping the adopted policy in `docs/dev/policies/` and using `AGENTS.md` as the wire-in entrypoint.

## Required references

- Read [references/selection-workflow.md](references/selection-workflow.md) before doing non-trivial selection work.
- Read [references/policy-shapes.md](references/policy-shapes.md) before drafting repo-local policy files or the `AGENTS.md` wire-in.

## Command recipes

```bash
python scripts/manage_policy.py --repo-root /path/to/target-repo adopt --json
python scripts/manage_policy.py --repo-root /path/to/target-repo adopt --write-drafts
python scripts/manage_policy.py --repo-root /path/to/target-repo --policy-root /path/to/target-repo/.codex/skills/repo-policy-selector/policy-library check-for-updates --json
python scripts/manage_policy.py --repo-root /path/to/target-repo upgrade-check --profile skill-repo-maintainer --policy-git-root /path/to/agent-policies --baseline-ref <old-tag-or-commit> --current-ref HEAD --json
python scripts/manage_policy.py --repo-root /path/to/target-repo upgrade-plan --profile skill-repo-maintainer --policy-git-root /path/to/agent-policies --baseline-ref <old-tag-or-commit> --current-ref HEAD --json
python scripts/manage_policy.py --repo-root /path/to/agent-policies release-bundle --source-root /path/to/agent-policies --bundle-version <version> --release-ref <tag-or-channel> --source-ref <commit-or-tag> --json
python scripts/manage_policy.py --repo-root /path/to/agent-policies release-notes --current-ref <tag-or-commit> --previous-ref <older-tag-or-commit> --write --json
python scripts/manage_policy.py --repo-root /path/to/agent-policies release-publish --tag <tag> --notes-file repo-policy-selector/releases/<tag>.md --latest --json
python scripts/manage_policy.py --repo-root /path/to/agent-policies release-cut --bundle-version <version> --release-ref <tag> --previous-ref <tag> --publish --latest --json

python scripts/manage_policy.py --repo-root /path/to/target-repo install-downstream --bundle-git-url https://github.com/CochranResearchGroup/agent-policies.git --bundle-ref <tag-or-branch> --target-repo-root /path/to/target-repo --write-drafts --json

python scripts/select_policy.py --repo-root /path/to/repo --policy-root /path/to/agent-policies
python scripts/select_policy.py --repo-root /path/to/target-repo --policy-root .. --json
python scripts/select_policy.py --repo-root /path/to/target-repo --policy-root .. --write-drafts
python scripts/check_policy_upgrades.py --repo-root /path/to/target-repo --profile skill-repo-maintainer --policy-git-root /path/to/agent-policies --baseline-ref <old-tag-or-commit> --current-ref HEAD --json
python scripts/plan_policy_upgrade_actions.py --repo-root /path/to/target-repo --profile skill-repo-maintainer --policy-git-root /path/to/agent-policies --baseline-ref <old-tag-or-commit> --current-ref HEAD --json
python scripts/audit_planning_contract.py --repo-root /path/to/repo --json
python scripts/audit_planning_contract.py --repo-root /path/to/repo --active-only --json
python scripts/audit_planning_contract.py --repo-root /path/to/repo --plans-dir doc/dev/plans --json
```

## Guardrails

- Do not overwrite repo-local nuance just to match a shared profile exactly.
- Prefer composing modules over inventing a monolithic new policy block.
- If the target repo has mature local rules, recommend a partial adoption instead of a full replacement.
- `--write-drafts` should only create missing planned policy files and rewrite `AGENTS.md`; it should refuse to overwrite an existing planned target file.
- This skill should remain self-contained when installed with its policy library.
- Installation, policy enumeration, and repo wiring should be handled deterministically.
- Downstream install should support a one-shot path that copies a pinned selector bundle into the target repo from either a reviewed git ref or a local bundle path and can draft the initial local policy set immediately.
- Released selector bundles should carry a deterministic `release-manifest.json` next to the bundled `policy-library/`.
- Treat the policy library as a source library, not the runtime source of truth for the target repo.
