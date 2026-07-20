#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

from generate_release_notes import release_notes_report, write_release_notes
from publish_github_release import publish_release
from release_selector_bundle import git_is_clean, git_value, release_bundle


def run_git(repo_root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo_root), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def tag_exists(repo_root: Path, tag: str) -> bool:
    result = subprocess.run(["git", "-C", str(repo_root), "rev-parse", "-q", "--verify", f"refs/tags/{tag}"], capture_output=True, text=True)
    return result.returncode == 0


def release_cut(
    *,
    repo_root: Path,
    selector_root: Path,
    bundle_version: str,
    release_ref: str,
    previous_ref: str | None,
    publish: bool,
    latest: bool,
    prerelease: bool,
) -> dict[str, Any]:
    if not git_is_clean(repo_root):
        raise RuntimeError("release-cut requires a clean starting worktree")
    if tag_exists(repo_root, release_ref):
        raise RuntimeError(f"tag already exists: {release_ref}")

    source_commit = git_value(repo_root, "rev-parse", "HEAD") or "UNRELEASED"
    bundle_report = release_bundle(
        source_root=repo_root,
        selector_root=selector_root,
        bundle_version=bundle_version,
        release_ref=release_ref,
        source_ref=source_commit,
    )
    notes_report = release_notes_report(repo_root, "HEAD", previous_ref)
    notes_path = write_release_notes(repo_root, notes_report)

    run_git(repo_root, "add", str(selector_root / "release-manifest.json"), str(notes_path))
    commit_message = f"Release selector bundle {release_ref}"
    run_git(repo_root, "commit", "-m", commit_message)
    release_commit = git_value(repo_root, "rev-parse", "HEAD")
    run_git(repo_root, "tag", "-a", release_ref, "-m", f"repo-policy-selector {release_ref}")
    run_git(repo_root, "push", "origin", "main")
    run_git(repo_root, "push", "origin", release_ref)

    publish_report: dict[str, Any] | None = None
    if publish:
        publish_report = publish_release(
            repo_root=repo_root,
            tag=release_ref,
            title=f"repo-policy-selector {release_ref}",
            notes_file=notes_path,
            latest=latest,
            prerelease=prerelease,
        )

    return {
        "bundle_report": bundle_report,
        "notes_path": str(notes_path),
        "release_commit": release_commit,
        "release_ref": release_ref,
        "bundle_version": bundle_version,
        "source_commit": source_commit,
        "publish_report": publish_report,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--selector-root")
    parser.add_argument("--bundle-version", required=True)
    parser.add_argument("--release-ref", required=True)
    parser.add_argument("--previous-ref")
    parser.add_argument("--publish", action="store_true")
    parser.add_argument("--latest", action="store_true")
    parser.add_argument("--prerelease", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    selector_root = Path(args.selector_root).resolve() if args.selector_root else repo_root / "repo-policy-selector"
    report = release_cut(
        repo_root=repo_root,
        selector_root=selector_root,
        bundle_version=args.bundle_version,
        release_ref=args.release_ref,
        previous_ref=args.previous_ref,
        publish=args.publish,
        latest=args.latest,
        prerelease=args.prerelease,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
