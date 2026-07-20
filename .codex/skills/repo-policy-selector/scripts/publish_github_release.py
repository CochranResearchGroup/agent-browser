#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


def run_gh(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(["gh", *args], check=True, capture_output=True, text=True)


def release_exists(tag: str) -> bool:
    result = subprocess.run(["gh", "release", "view", tag], capture_output=True, text=True)
    return result.returncode == 0


def publish_release(
    *,
    repo_root: Path,
    tag: str,
    title: str | None,
    notes_file: Path,
    latest: bool,
    prerelease: bool,
) -> dict[str, Any]:
    release_title = title or f"repo-policy-selector {tag}"
    rel_path = notes_file.resolve().relative_to(repo_root.resolve())
    if release_exists(tag):
        cmd = [
            "release",
            "edit",
            tag,
            "--title",
            release_title,
            "--notes-file",
            str(notes_file),
        ]
        if latest:
            cmd.append("--latest")
        if prerelease:
            cmd.append("--prerelease")
        run_gh(*cmd)
        action = "edited"
    else:
        cmd = [
            "release",
            "create",
            tag,
            "--title",
            release_title,
            "--notes-file",
            str(notes_file),
        ]
        if latest:
            cmd.append("--latest")
        if prerelease:
            cmd.append("--prerelease")
        run_gh(*cmd)
        action = "created"
    view = run_gh("release", "view", tag, "--json", "url,name,tagName,isDraft,isPrerelease,isImmutable,publishedAt")
    data = json.loads(view.stdout)
    return {
        "action": action,
        "tag": tag,
        "title": release_title,
        "notes_file": str(notes_file),
        "notes_repo_path": str(rel_path),
        "url": data.get("url", ""),
        "is_draft": data.get("isDraft", False),
        "is_prerelease": data.get("isPrerelease", False),
        "is_immutable": data.get("isImmutable", False),
        "published_at": data.get("publishedAt", ""),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--title")
    parser.add_argument("--notes-file", required=True)
    parser.add_argument("--latest", action="store_true")
    parser.add_argument("--prerelease", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = publish_release(
        repo_root=Path(args.repo_root).resolve(),
        tag=args.tag,
        title=args.title,
        notes_file=Path(args.notes_file).resolve(),
        latest=args.latest,
        prerelease=args.prerelease,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        for key, value in report.items():
            print(f"{key}: {value}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
