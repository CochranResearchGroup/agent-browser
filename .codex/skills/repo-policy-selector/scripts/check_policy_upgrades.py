#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from select_policy import (
    adopted_policy_id,
    enumerate_policy_library,
    extract_existing_policy_surfaces,
    parse_catalog,
    parse_profile,
    policy_adoption_coverage,
    read_text,
)


def git_show(repo_root: Path, ref: str, path: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), "show", f"{ref}:{path}"],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return ""
    return result.stdout


def parse_json_text(text: str) -> dict[str, Any]:
    if not text:
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return data if isinstance(data, dict) else {}


def parse_release_manifest_at_ref(policy_repo_root: Path, ref: str) -> dict[str, Any]:
    return parse_json_text(git_show(policy_repo_root, ref, "repo-policy-selector/release-manifest.json"))


def parse_catalog_at_ref(policy_repo_root: Path, ref: str) -> dict[str, list[dict[str, Any]]]:
    text = git_show(policy_repo_root, ref, "catalog.yaml")
    if not text:
        return {"modules": [], "profiles": []}
    temp = policy_repo_root / ".codex-tmp-catalog.yaml"
    temp.write_text(text, encoding="utf-8")
    try:
        return parse_catalog(temp)
    finally:
        temp.unlink(missing_ok=True)


def parse_profile_at_ref(policy_repo_root: Path, ref: str, rel_path: str) -> dict[str, Any]:
    text = git_show(policy_repo_root, ref, rel_path)
    if not text:
        return {}
    temp = policy_repo_root / ".codex-tmp-profile.yaml"
    temp.write_text(text, encoding="utf-8")
    try:
        return parse_profile(temp)
    finally:
        temp.unlink(missing_ok=True)


def catalog_map(items: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {item["id"]: item for item in items if "id" in item}


def changed_paths_between_refs(policy_repo_root: Path, baseline_ref: str, current_ref: str) -> list[str]:
    try:
        result = subprocess.run(
            ["git", "-C", str(policy_repo_root), "diff", "--name-only", f"{baseline_ref}..{current_ref}"],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return []
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def current_profile_modules(profile_id: str, installed_library: dict[str, Any]) -> list[str]:
    profile = installed_library.get("parsed_profiles", {}).get(profile_id, {})
    modules = profile.get("modules", [])
    return list(modules) if isinstance(modules, list) else []


def current_adopted_module_ids(repo_root: Path, installed_library: dict[str, Any], recommended_modules: list[str]) -> list[str]:
    surfaces = extract_existing_policy_surfaces(repo_root)
    coverage = policy_adoption_coverage(surfaces, recommended_modules, installed_library)
    return coverage["already_adopted_modules"]


def upgrade_report(
    repo_root: Path,
    installed_library: dict[str, Any],
    profile_id: str,
    baseline_ref: str | None,
    current_ref: str,
    policy_repo_root: Path | None,
) -> dict[str, Any]:
    recommended_modules = current_profile_modules(profile_id, installed_library)
    adopted_modules = current_adopted_module_ids(repo_root, installed_library, recommended_modules)
    installed_release_manifest = installed_library.get("release_manifest", {})
    report: dict[str, Any] = {
        "selected_profile": profile_id,
        "current_profile_modules": recommended_modules,
        "already_adopted_modules": adopted_modules,
        "newly_available_modules": [module_id for module_id in recommended_modules if module_id not in adopted_modules],
        "changed_adopted_modules": [],
        "retirement_review_modules": [],
        "baseline_ref": baseline_ref,
        "current_ref": current_ref,
        "installed_bundle_release": installed_release_manifest,
        "baseline_bundle_release": {},
        "current_bundle_release": installed_release_manifest,
    }
    if not baseline_ref or policy_repo_root is None:
        return report

    baseline_catalog = parse_catalog_at_ref(policy_repo_root, baseline_ref)
    baseline_profiles = catalog_map(baseline_catalog["profiles"])
    baseline_profile_entry = baseline_profiles.get(profile_id)
    baseline_modules: list[str] = []
    if baseline_profile_entry and "path" in baseline_profile_entry:
        baseline_profile = parse_profile_at_ref(policy_repo_root, baseline_ref, baseline_profile_entry["path"])
        modules = baseline_profile.get("modules", [])
        if isinstance(modules, list):
            baseline_modules = list(modules)

    changed_paths = changed_paths_between_refs(policy_repo_root, baseline_ref, current_ref)
    changed_module_ids = {
        Path(path).stem
        for path in changed_paths
        if path.startswith("modules/") and path.endswith(".md")
    }
    current_set = set(recommended_modules)
    baseline_set = set(baseline_modules)
    report["newly_available_modules"] = sorted(current_set - baseline_set)
    report["retirement_review_modules"] = sorted(baseline_set - current_set)
    report["changed_adopted_modules"] = sorted(changed_module_ids & set(adopted_modules))
    report["changed_paths"] = changed_paths
    report["baseline_bundle_release"] = parse_release_manifest_at_ref(policy_repo_root, baseline_ref)
    current_bundle_release = parse_release_manifest_at_ref(policy_repo_root, current_ref)
    report["current_bundle_release"] = current_bundle_release or installed_release_manifest
    return report


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
    report = upgrade_report(
        repo_root=repo_root,
        installed_library=installed_library,
        profile_id=args.profile,
        baseline_ref=args.baseline_ref,
        current_ref=args.current_ref,
        policy_repo_root=policy_git_root,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"selected_profile: {report['selected_profile']}")
        print(f"baseline_ref: {report['baseline_ref'] or '-'}")
        print(f"current_ref: {report['current_ref']}")
        installed_version = report.get("installed_bundle_release", {}).get("bundle_version", "-")
        baseline_version = report.get("baseline_bundle_release", {}).get("bundle_version", "-")
        current_version = report.get("current_bundle_release", {}).get("bundle_version", installed_version or "-")
        print(f"installed_bundle_version: {installed_version}")
        print(f"baseline_bundle_version: {baseline_version}")
        print(f"current_bundle_version: {current_version}")
        print(f"already_adopted_modules: {', '.join(report['already_adopted_modules']) or '-'}")
        print(f"newly_available_modules: {', '.join(report['newly_available_modules']) or '-'}")
        print(f"changed_adopted_modules: {', '.join(report['changed_adopted_modules']) or '-'}")
        print(f"retirement_review_modules: {', '.join(report['retirement_review_modules']) or '-'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
