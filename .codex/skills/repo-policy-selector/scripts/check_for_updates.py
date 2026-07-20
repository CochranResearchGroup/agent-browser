#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

from select_policy import enumerate_policy_library, read_json

DEFAULT_REPO = "CochranResearchGroup/agent-policies"
INSTALL_RECORD_RELPATH = ".codex/policy-selector-install.json"


def run_gh(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True)


def read_install_record(repo_root: Path) -> dict[str, Any]:
    return read_json(repo_root / INSTALL_RECORD_RELPATH)


def parse_version_tuple(value: str) -> tuple[int, ...]:
    match = re.search(r"(\d+)\.(\d+)\.(\d+)", value)
    if not match:
        return ()
    return tuple(int(part) for part in match.groups())


def normalized_installed_ref(installed_release: dict[str, Any], install_record: dict[str, Any]) -> str:
    for key in ("release_ref", "bundle_ref", "bundle_version"):
        value = installed_release.get(key) or install_record.get(key)
        if isinstance(value, str) and value:
            return value
    source = install_record.get("install_source", {}) if isinstance(install_record.get("install_source"), dict) else {}
    for key in ("bundle_ref",):
        value = source.get(key)
        if isinstance(value, str) and value:
            return value
    return ""


def github_releases(repo: str, limit: int) -> tuple[list[dict[str, Any]], str]:
    try:
        result = run_gh(
        "release",
        "list",
        "--repo",
        repo,
        "--limit",
        str(limit),
        "--json",
        "tagName,name,isLatest,isDraft,isPrerelease,publishedAt,createdAt",
    )
    except subprocess.CalledProcessError as exc:
        stderr = (exc.stderr or exc.stdout or str(exc)).strip()
        return [], stderr
    data = json.loads(result.stdout)
    return (data if isinstance(data, list) else []), ""


def choose_latest_release(releases: list[dict[str, Any]], include_prereleases: bool) -> dict[str, Any]:
    filtered = [
        rel for rel in releases
        if not rel.get("isDraft") and (include_prereleases or not rel.get("isPrerelease"))
    ]
    return filtered[0] if filtered else {}


def releases_since_installed(releases: list[dict[str, Any]], installed_ref: str, include_prereleases: bool) -> list[dict[str, Any]]:
    filtered = [
        rel for rel in releases
        if not rel.get("isDraft") and (include_prereleases or not rel.get("isPrerelease"))
    ]
    if not installed_ref:
        return filtered
    out: list[dict[str, Any]] = []
    for rel in filtered:
        if rel.get("tagName") == installed_ref:
            break
        out.append(rel)
    return out


def check_for_updates(
    *,
    repo_root: Path,
    policy_root: Path | None,
    github_repo: str,
    limit: int,
    include_prereleases: bool,
) -> dict[str, Any]:
    installed_library = enumerate_policy_library(policy_root)
    install_record = read_install_record(repo_root)
    installed_release = installed_library.get("release_manifest", {})
    installed_ref = normalized_installed_ref(installed_release, install_record)
    installed_version = installed_release.get("bundle_version", "") if isinstance(installed_release, dict) else ""
    releases, query_error = github_releases(github_repo, limit)
    latest = choose_latest_release(releases, include_prereleases)
    latest_tag = latest.get("tagName", "") if isinstance(latest, dict) else ""
    latest_version = ""
    if isinstance(latest, dict):
        latest_version = latest_tag.removeprefix("v") if latest_tag.startswith("v") else latest_tag

    installed_tuple = parse_version_tuple(installed_version or installed_ref)
    latest_tuple = parse_version_tuple(latest_version or latest_tag)
    update_available = False
    if latest_tag and installed_ref:
        update_available = latest_tag != installed_ref
        if installed_tuple and latest_tuple:
            update_available = latest_tuple > installed_tuple
    elif latest_tag:
        update_available = True

    newer_releases = releases_since_installed(releases, installed_ref, include_prereleases)
    if query_error:
        recommended_action = "unable to query GitHub releases"
    elif update_available:
        recommended_action = f"review upgrade to {latest_tag}"
    elif latest_tag:
        recommended_action = "stay pinned"
    else:
        recommended_action = "unable to determine latest release"

    return {
        "repo_root": str(repo_root),
        "policy_root": installed_library.get("policy_root", ""),
        "github_repo": github_repo,
        "installed_bundle_release": installed_release,
        "install_record": install_record,
        "installed_ref": installed_ref,
        "installed_version": installed_version,
        "latest_release": latest,
        "release_query_error": query_error,
        "latest_tag": latest_tag,
        "latest_version": latest_version,
        "update_available": update_available,
        "newer_releases": newer_releases,
        "recommended_action": recommended_action,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--policy-root")
    parser.add_argument("--github-repo", default=DEFAULT_REPO)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--include-prereleases", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = check_for_updates(
        repo_root=Path(args.repo_root).resolve(),
        policy_root=Path(args.policy_root).resolve() if args.policy_root else None,
        github_repo=args.github_repo,
        limit=args.limit,
        include_prereleases=args.include_prereleases,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"repo_root: {report['repo_root']}")
        print(f"github_repo: {report['github_repo']}")
        print(f"installed_ref: {report['installed_ref'] or '-'}")
        print(f"installed_version: {report['installed_version'] or '-'}")
        print(f"latest_tag: {report['latest_tag'] or '-'}")
        print(f"latest_version: {report['latest_version'] or '-'}")
        print(f"update_available: {report['update_available']}")
        print(f"recommended_action: {report['recommended_action']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
