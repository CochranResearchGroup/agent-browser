#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
from pathlib import Path
from typing import Any

from check_policy_upgrades import upgrade_report
from select_policy import (
    adopted_policy_id,
    build_install_plan,
    enumerate_policy_library,
    extract_existing_policy_surfaces,
    policy_adoption_coverage,
    render_agents_wirein,
)


def canonical_policy_map(existing_policy_surfaces: list[dict[str, Any]]) -> dict[str, list[str]]:
    mapping: dict[str, list[str]] = {}
    for item in existing_policy_surfaces:
        if item["source_type"] != "canonical-policy":
            continue
        path = item["path"]
        key = adopted_policy_id(Path(path))
        mapping.setdefault(key, []).append(path)
    return mapping


def current_review_paths(existing_policy_surfaces: list[dict[str, Any]], module_ids: list[str]) -> dict[str, list[str]]:
    coverage = canonical_policy_map(existing_policy_surfaces)
    return {module_id: coverage.get(module_id, []) for module_id in module_ids}


def build_retirement_plan(
    repo_root: Path,
    existing_policy_surfaces: list[dict[str, Any]],
    retirement_review_modules: list[str],
) -> dict[str, Any]:
    review_paths = current_review_paths(existing_policy_surfaces, retirement_review_modules)
    retire_paths = sorted({path for paths in review_paths.values() for path in paths})
    keep_surfaces = [
        item
        for item in existing_policy_surfaces
        if item["path"] not in retire_paths
    ]
    agents_patch = render_agents_wirein(repo_root, [], keep_surfaces)
    return {
        "retire_review_modules": retirement_review_modules,
        "retire_paths": retire_paths,
        "agents_wirein_patch": agents_patch,
    }


def build_upgrade_action_plan(
    repo_root: Path,
    installed_library: dict[str, Any],
    profile_id: str,
    baseline_ref: str | None,
    current_ref: str,
    policy_repo_root: Path | None,
) -> dict[str, Any]:
    existing_policy_surfaces = extract_existing_policy_surfaces(repo_root)
    current_modules = installed_library.get("parsed_profiles", {}).get(profile_id, {}).get("modules", [])
    if not isinstance(current_modules, list):
        current_modules = []
    coverage = policy_adoption_coverage(existing_policy_surfaces, current_modules, installed_library)
    report = upgrade_report(
        repo_root=repo_root,
        installed_library=installed_library,
        profile_id=profile_id,
        baseline_ref=baseline_ref,
        current_ref=current_ref,
        policy_repo_root=policy_repo_root,
    )
    install_plan = build_install_plan(repo_root, report["newly_available_modules"], coverage, installed_library)
    changed_paths = current_review_paths(existing_policy_surfaces, report["changed_adopted_modules"])
    retirement_paths = current_review_paths(existing_policy_surfaces, report["retirement_review_modules"])
    retirement_plan = build_retirement_plan(repo_root, existing_policy_surfaces, report["retirement_review_modules"])

    actions: list[dict[str, Any]] = []
    for item in install_plan:
        actions.append(
            {
                "action": "install-new",
                "module_id": item["module_id"],
                "target_policy_path": item["target_policy_path"],
                "source_module_path": item["source_module_path"],
            }
        )
    for module_id, paths in changed_paths.items():
        actions.append(
            {
                "action": "upgrade-review",
                "module_id": module_id,
                "local_policy_paths": paths,
            }
        )
    for module_id, paths in retirement_paths.items():
        actions.append(
            {
                "action": "retire-review",
                "module_id": module_id,
                "local_policy_paths": paths,
            }
        )
    return {
        "selected_profile": profile_id,
        "baseline_ref": baseline_ref,
        "current_ref": current_ref,
        "upgrade_report": report,
        "retirement_plan": retirement_plan,
        "action_plan": actions,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--policy-root", required=False)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--baseline-ref")
    parser.add_argument("--current-ref", default="HEAD")
    parser.add_argument("--policy-git-root")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    policy_root = Path(args.policy_root).resolve() if args.policy_root else None
    policy_git_root = Path(args.policy_git_root).resolve() if args.policy_git_root else None
    installed_library = enumerate_policy_library(policy_root)
    plan = build_upgrade_action_plan(
        repo_root=repo_root,
        installed_library=installed_library,
        profile_id=args.profile,
        baseline_ref=args.baseline_ref,
        current_ref=args.current_ref,
        policy_repo_root=policy_git_root,
    )
    if args.json:
        print(json.dumps(plan, indent=2, sort_keys=True))
    else:
        print(f"selected_profile: {plan['selected_profile']}")
        print(f"baseline_ref: {plan['baseline_ref'] or '-'}")
        print(f"current_ref: {plan['current_ref']}")
        print("action_plan:")
        for item in plan["action_plan"]:
            if item["action"] == "install-new":
                print(f"- install-new {item['module_id']} -> {item['target_policy_path']}")
            else:
                paths = ", ".join(item.get("local_policy_paths", [])) or "-"
                print(f"- {item['action']} {item['module_id']} ({paths})")
        retirement_plan = plan["retirement_plan"]
        if retirement_plan["retire_paths"]:
            print("retirement_plan:")
            for path in retirement_plan["retire_paths"]:
                print(f"- remove {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
