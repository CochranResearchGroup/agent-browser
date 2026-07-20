#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


MANIFEST_PATH = "repo-policy-selector/release-manifest.json"
RELEASES_DIR = "repo-policy-selector/releases"


def git_text(repo_root: Path, *args: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo_root), *args],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError:
        return ""
    return result.stdout.strip()


def git_lines(repo_root: Path, *args: str) -> list[str]:
    text = git_text(repo_root, *args)
    return [line for line in text.splitlines() if line.strip()]


def parse_json_text(text: str) -> dict[str, Any]:
    if not text:
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return data if isinstance(data, dict) else {}


def manifest_for_ref(repo_root: Path, ref: str) -> dict[str, Any]:
    if ref == "HEAD":
        path = repo_root / MANIFEST_PATH
        if path.exists():
            return parse_json_text(path.read_text(encoding="utf-8"))
    return parse_json_text(git_text(repo_root, "show", f"{ref}:{MANIFEST_PATH}"))


def previous_tag(repo_root: Path, current_ref: str) -> str:
    return git_text(repo_root, "describe", "--tags", "--abbrev=0", f"{current_ref}^")


def commit_summaries(repo_root: Path, previous_ref: str, current_ref: str) -> list[dict[str, str]]:
    lines = git_lines(repo_root, "log", "--format=%H%x09%s", f"{previous_ref}..{current_ref}")
    out: list[dict[str, str]] = []
    for line in lines:
        sha, _, subject = line.partition("\t")
        out.append({"sha": sha, "subject": subject})
    return out


def changed_paths(repo_root: Path, previous_ref: str, current_ref: str) -> list[str]:
    return git_lines(repo_root, "diff", "--name-only", f"{previous_ref}..{current_ref}")


def categorize_paths(paths: list[str]) -> dict[str, list[str]]:
    categories: dict[str, list[str]] = {
        "modules": [],
        "profiles": [],
        "scripts": [],
        "docs": [],
        "release": [],
        "other": [],
    }
    for path in paths:
        if path.startswith("modules/") or path.startswith("repo-policy-selector/policy-library/modules/"):
            categories["modules"].append(path)
        elif path.startswith("profiles/") or path.startswith("repo-policy-selector/policy-library/profiles/"):
            categories["profiles"].append(path)
        elif path.startswith("repo-policy-selector/scripts/"):
            categories["scripts"].append(path)
        elif path == MANIFEST_PATH or path.startswith("repo-policy-selector/releases/"):
            categories["release"].append(path)
        elif path.endswith(".md") or path.endswith(".yaml"):
            categories["docs"].append(path)
        else:
            categories["other"].append(path)
    return categories


def markdown_notes(report: dict[str, Any]) -> str:
    manifest = report.get("current_manifest", {})
    lines = [
        f"# Selector Release {report['release_ref']}",
        "",
        f"- Bundle version: `{manifest.get('bundle_version', '-')}`",
        f"- Source commit: `{manifest.get('source_commit', '-')}`",
        f"- Source ref: `{manifest.get('source_ref', '-') or '-'}`",
        f"- Previous release: `{report['previous_ref']}`",
        "",
        "## Summary",
        "",
    ]
    categories = report.get("changed_categories", {})
    for key in ["scripts", "release", "modules", "profiles", "docs", "other"]:
        items = categories.get(key, [])
        if items:
            lines.append(f"- `{key}`: {len(items)} changed")
    lines.extend(["", "## Commits", ""])
    for item in report.get("commits", []):
        lines.append(f"- `{item['sha'][:7]}` {item['subject']}")
    lines.append("")
    lines.append("## Changed Paths")
    lines.append("")
    for key in ["scripts", "release", "modules", "profiles", "docs", "other"]:
        items = categories.get(key, [])
        if not items:
            continue
        lines.append(f"### {key.title()}")
        lines.append("")
        for path in items:
            lines.append(f"- `{path}`")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def release_notes_report(repo_root: Path, current_ref: str, previous_ref: str | None) -> dict[str, Any]:
    current_manifest = manifest_for_ref(repo_root, current_ref)
    release_ref = current_manifest.get("release_ref") or current_ref
    resolved_previous = previous_ref or previous_tag(repo_root, current_ref)
    paths = changed_paths(repo_root, resolved_previous, current_ref)
    report = {
        "release_ref": release_ref,
        "current_ref": current_ref,
        "previous_ref": resolved_previous,
        "current_manifest": current_manifest,
        "changed_paths": paths,
        "changed_categories": categorize_paths(paths),
        "commits": commit_summaries(repo_root, resolved_previous, current_ref),
    }
    report["markdown"] = markdown_notes(report)
    return report


def write_release_notes(repo_root: Path, report: dict[str, Any]) -> Path:
    releases_dir = repo_root / RELEASES_DIR
    releases_dir.mkdir(parents=True, exist_ok=True)
    output_path = releases_dir / f"{report['release_ref']}.md"
    output_path.write_text(report["markdown"], encoding="utf-8")
    return output_path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--current-ref", default="HEAD")
    parser.add_argument("--previous-ref")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    report = release_notes_report(repo_root, args.current_ref, args.previous_ref)
    if args.write:
        report["written_path"] = str(write_release_notes(repo_root, report))
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(report["markdown"], end="")
        if args.write:
            print(f"written_path: {report['written_path']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
