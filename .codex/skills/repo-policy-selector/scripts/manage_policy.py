#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
from pathlib import Path
from typing import Any

from check_for_updates import check_for_updates
from check_policy_upgrades import upgrade_report
from generate_release_notes import release_notes_report, write_release_notes
from install_selector_bundle import install_selector_bundle
from publish_github_release import publish_release
from release_cut import release_cut
from plan_policy_upgrade_actions import build_upgrade_action_plan
from release_selector_bundle import release_bundle
from select_policy import (
    build_install_plan,
    choose_adoption_mode,
    choose_profile,
    detect_signals,
    enumerate_policy_library,
    extract_existing_migration_surfaces,
    extract_existing_policy_surfaces,
    infer_repo_local_policy_findings,
    policy_adoption_coverage,
    profile_expectation_gaps,
    recommendation_mode,
    render_agents_wirein,
    summarize_migration_surface_actions,
    summarize_policy_surface_actions,
    validate_recommendations,
    write_drafts,
)


def run_adopt(repo_root: Path, installed_library: dict[str, Any], write_drafts_flag: bool) -> dict[str, Any]:
    signals = detect_signals(repo_root)
    existing_migration_surfaces = extract_existing_migration_surfaces(repo_root)
    existing_policy_surfaces = extract_existing_policy_surfaces(repo_root)
    purpose, subtype, execution_bias, profile, modules, reasons = choose_profile(signals, installed_library)
    repo_local_policy_findings = infer_repo_local_policy_findings(repo_root, modules, profile, signals)
    coverage = policy_adoption_coverage(existing_policy_surfaces, modules, installed_library)
    expectation_gaps = profile_expectation_gaps(profile, signals, installed_library)
    adoption_mode, migration_reasons, migration_targets = choose_adoption_mode(signals, expectation_gaps, coverage)
    validation_problems = validate_recommendations(profile, modules, installed_library)
    rec_mode = recommendation_mode(coverage)
    next_modules = coverage["missing_recommended_modules"] if rec_mode == "patch-missing" else modules
    install_plan = build_install_plan(repo_root, next_modules, coverage, installed_library)
    agents_patch = render_agents_wirein(repo_root, install_plan, existing_policy_surfaces, purpose)
    written_paths: list[str] = []
    if write_drafts_flag:
        written_paths = write_drafts(repo_root, install_plan, agents_patch)
    return {
        "mode": "adopt",
        "repo_root": str(repo_root),
        "policy_root": installed_library["policy_root"],
        "installed_bundle_release": installed_library.get("release_manifest", {}),
        "repo_purpose": purpose,
        "workflow_subtype": subtype,
        "execution_bias": execution_bias,
        "recommended_profile": profile,
        "recommended_modules": modules,
        "recommendation_mode": rec_mode,
        "next_modules": next_modules,
        "install_plan": install_plan,
        "agents_wirein_patch": agents_patch,
        "written_paths": written_paths,
        "adoption_mode": adoption_mode,
        "migration_reasons": migration_reasons,
        "migration_targets": migration_targets,
        "profile_expectation_gaps": expectation_gaps,
        "policy_adoption_coverage": coverage,
        "existing_policy_surfaces": existing_policy_surfaces,
        "repo_local_policy_findings": repo_local_policy_findings,
        "policy_surface_actions": summarize_policy_surface_actions(existing_policy_surfaces),
        "existing_migration_surfaces": existing_migration_surfaces,
        "migration_surface_actions": summarize_migration_surface_actions(existing_migration_surfaces),
        "validation_problems": validation_problems,
        "reasons": reasons,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--policy-root")

    subparsers = parser.add_subparsers(dest="mode", required=True)

    adopt_parser = subparsers.add_parser("adopt")
    adopt_parser.add_argument("--write-drafts", action="store_true")
    adopt_parser.add_argument("--json", action="store_true")

    upgrade_check_parser = subparsers.add_parser("upgrade-check")
    upgrade_check_parser.add_argument("--profile", required=True)
    upgrade_check_parser.add_argument("--baseline-ref")
    upgrade_check_parser.add_argument("--current-ref", default="HEAD")
    upgrade_check_parser.add_argument("--policy-git-root")
    upgrade_check_parser.add_argument("--json", action="store_true")

    upgrade_plan_parser = subparsers.add_parser("upgrade-plan")
    upgrade_plan_parser.add_argument("--profile", required=True)
    upgrade_plan_parser.add_argument("--baseline-ref")
    upgrade_plan_parser.add_argument("--current-ref", default="HEAD")
    upgrade_plan_parser.add_argument("--policy-git-root")
    upgrade_plan_parser.add_argument("--json", action="store_true")

    release_parser = subparsers.add_parser("release-bundle")
    release_parser.add_argument("--source-root", required=True)
    release_parser.add_argument("--selector-root")
    release_parser.add_argument("--bundle-version", required=True)
    release_parser.add_argument("--release-ref")
    release_parser.add_argument("--source-ref")
    release_parser.add_argument("--json", action="store_true")

    install_parser = subparsers.add_parser("install-downstream")
    install_parser.add_argument("--selector-root")
    install_parser.add_argument("--bundle-git-url")
    install_parser.add_argument("--bundle-ref")
    install_parser.add_argument("--selector-subdir", default="repo-policy-selector")
    install_parser.add_argument("--target-repo-root", required=True)
    install_parser.add_argument("--install-relpath", default=".codex/skills/repo-policy-selector")
    install_parser.add_argument("--force", action="store_true")
    install_parser.add_argument("--write-drafts", action="store_true")
    install_parser.add_argument("--json", action="store_true")

    updates_parser = subparsers.add_parser("check-for-updates")
    updates_parser.add_argument("--github-repo", default="CochranResearchGroup/agent-policies")
    updates_parser.add_argument("--limit", type=int, default=10)
    updates_parser.add_argument("--include-prereleases", action="store_true")
    updates_parser.add_argument("--json", action="store_true")

    notes_parser = subparsers.add_parser("release-notes")
    notes_parser.add_argument("--current-ref", default="HEAD")
    notes_parser.add_argument("--previous-ref")
    notes_parser.add_argument("--write", action="store_true")
    notes_parser.add_argument("--json", action="store_true")

    publish_parser = subparsers.add_parser("release-publish")
    publish_parser.add_argument("--tag", required=True)
    publish_parser.add_argument("--title")
    publish_parser.add_argument("--notes-file", required=True)
    publish_parser.add_argument("--latest", action="store_true")
    publish_parser.add_argument("--prerelease", action="store_true")
    publish_parser.add_argument("--json", action="store_true")

    cut_parser = subparsers.add_parser("release-cut")
    cut_parser.add_argument("--selector-root")
    cut_parser.add_argument("--bundle-version", required=True)
    cut_parser.add_argument("--release-ref", required=True)
    cut_parser.add_argument("--previous-ref")
    cut_parser.add_argument("--publish", action="store_true")
    cut_parser.add_argument("--latest", action="store_true")
    cut_parser.add_argument("--prerelease", action="store_true")
    cut_parser.add_argument("--json", action="store_true")

    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    policy_root = Path(args.policy_root).resolve() if args.policy_root else None
    installed_library = enumerate_policy_library(policy_root)

    if args.mode == "adopt":
        out = run_adopt(repo_root, installed_library, args.write_drafts)
    elif args.mode == "upgrade-check":
        policy_git_root = Path(args.policy_git_root).resolve() if args.policy_git_root else None
        out = upgrade_report(
            repo_root=repo_root,
            installed_library=installed_library,
            profile_id=args.profile,
            baseline_ref=args.baseline_ref,
            current_ref=args.current_ref,
            policy_repo_root=policy_git_root,
        )
        out["mode"] = "upgrade-check"
    elif args.mode == "release-bundle":
        selector_root = Path(args.selector_root).resolve() if args.selector_root else Path(__file__).resolve().parents[1]
        out = release_bundle(
            source_root=Path(args.source_root).resolve(),
            selector_root=selector_root,
            bundle_version=args.bundle_version,
            release_ref=args.release_ref,
            source_ref=args.source_ref,
        )
        out["mode"] = "release-bundle"
    elif args.mode == "release-notes":
        out = release_notes_report(repo_root, args.current_ref, args.previous_ref)
        if args.write:
            out["written_path"] = str(write_release_notes(repo_root, out))
        out["mode"] = "release-notes"
    elif args.mode == "release-publish":
        out = publish_release(
            repo_root=repo_root,
            tag=args.tag,
            title=args.title,
            notes_file=Path(args.notes_file).resolve(),
            latest=args.latest,
            prerelease=args.prerelease,
        )
        out["mode"] = "release-publish"
    elif args.mode == "release-cut":
        selector_root = Path(args.selector_root).resolve() if args.selector_root else repo_root / "repo-policy-selector"
        out = release_cut(
            repo_root=repo_root,
            selector_root=selector_root,
            bundle_version=args.bundle_version,
            release_ref=args.release_ref,
            previous_ref=args.previous_ref,
            publish=args.publish,
            latest=args.latest,
            prerelease=args.prerelease,
        )
        out["mode"] = "release-cut"
    elif args.mode == "install-downstream":
        selector_root = Path(args.selector_root).resolve() if args.selector_root else None
        out = install_selector_bundle(
            selector_root=selector_root,
            bundle_git_url=args.bundle_git_url,
            bundle_ref=args.bundle_ref,
            selector_subdir=args.selector_subdir,
            target_repo_root=Path(args.target_repo_root).resolve(),
            install_relpath=args.install_relpath,
            force=args.force,
            write_drafts=args.write_drafts,
        )
        out["mode"] = "install-downstream"
    elif args.mode == "check-for-updates":
        out = check_for_updates(
            repo_root=repo_root,
            policy_root=policy_root,
            github_repo=args.github_repo,
            limit=args.limit,
            include_prereleases=args.include_prereleases,
        )
        out["mode"] = "check-for-updates"
    else:
        policy_git_root = Path(args.policy_git_root).resolve() if args.policy_git_root else None
        out = build_upgrade_action_plan(
            repo_root=repo_root,
            installed_library=installed_library,
            profile_id=args.profile,
            baseline_ref=args.baseline_ref,
            current_ref=args.current_ref,
            policy_repo_root=policy_git_root,
        )
        out["mode"] = "upgrade-plan"

    if getattr(args, "json", False):
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        print(f"mode: {out['mode']}")
        for key in sorted(k for k in out.keys() if k != "mode"):
            value = out[key]
            if isinstance(value, (dict, list)):
                print(f"{key}: {json.dumps(value, indent=2, sort_keys=True)}")
            else:
                print(f"{key}: {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
