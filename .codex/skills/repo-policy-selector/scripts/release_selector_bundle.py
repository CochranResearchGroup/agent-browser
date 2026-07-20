#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from select_policy import enumerate_policy_library
from sync_policy_library import sync_policy_library


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def git_value(repo_root: Path, *args: str) -> str:
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


def git_is_clean(repo_root: Path) -> bool:
    status = git_value(repo_root, "status", "--porcelain")
    return not status




def materialize_source_ref(source_root: Path, source_ref: str) -> tuple[tempfile.TemporaryDirectory[str], Path, str]:
    temp_dir = tempfile.TemporaryDirectory(prefix="selector-release-")
    snapshot_root = Path(temp_dir.name) / "snapshot"
    snapshot_root.mkdir(parents=True, exist_ok=True)
    archive_cmd = f"git -C {shlex_quote(str(source_root))} archive {shlex_quote(source_ref)} | tar -x -C {shlex_quote(str(snapshot_root))}"
    subprocess.run(["bash", "-lc", archive_cmd], check=True, capture_output=True, text=True)
    source_commit = git_value(source_root, "rev-parse", source_ref) or "UNRELEASED"
    return temp_dir, snapshot_root, source_commit


def shlex_quote(value: str) -> str:
    return "'" + value.replace("'", "'\''") + "'"

def compute_tree_digest(path: Path) -> str:
    digest = hashlib.sha256()
    for file_path in sorted(p for p in path.rglob("*") if p.is_file()):
        digest.update(str(file_path.relative_to(path)).encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def write_release_manifest(
    *,
    selector_root: Path,
    policy_library_root: Path,
    bundle_version: str,
    release_ref: str,
    source_root: Path,
    source_commit: str,
    source_tree_state: str,
    source_ref: str | None,
) -> dict[str, Any]:
    manifest = {
        "schema_version": 1,
        "bundle_name": "repo-policy-selector",
        "bundle_version": bundle_version,
        "release_ref": release_ref,
        "source_repo_root": str(source_root),
        "source_commit": source_commit or "UNRELEASED",
        "source_tree_state": source_tree_state,
        "source_ref": source_ref or "",
        "policy_library": {
            "relative_root": "policy-library",
            "catalog_path": "policy-library/catalog.yaml",
            "schema_path": "policy-library/SCHEMA.md",
            "content_sha256": compute_tree_digest(policy_library_root),
        },
    }
    manifest_path = selector_root / "release-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return manifest


def release_bundle(
    *,
    source_root: Path,
    selector_root: Path,
    bundle_version: str,
    release_ref: str | None,
    source_ref: str | None,
) -> dict[str, Any]:
    output_root = selector_root / "policy-library"
    temp_dir: tempfile.TemporaryDirectory[str] | None = None
    effective_source_root = source_root
    effective_source_ref = source_ref
    if source_ref:
        temp_dir, effective_source_root, source_commit = materialize_source_ref(source_root, source_ref)
        source_tree_state = "clean-ref"
    else:
        if not git_is_clean(source_root):
            raise RuntimeError("release-bundle without --source-ref requires a clean source tree")
        source_commit = git_value(source_root, "rev-parse", "HEAD") or "UNRELEASED"
        source_tree_state = "clean"
    sync_policy_library(effective_source_root, output_root)
    try:
        manifest = write_release_manifest(
            selector_root=selector_root,
            policy_library_root=output_root,
            bundle_version=bundle_version,
            release_ref=release_ref or bundle_version,
            source_root=source_root,
            source_commit=source_commit,
            source_tree_state=source_tree_state,
            source_ref=effective_source_ref,
        )
        installed_library = enumerate_policy_library(output_root)
    finally:
        if temp_dir is not None:
            temp_dir.cleanup()
    validation_problems: list[str] = []
    if not installed_library.get("catalog_found"):
        validation_problems.append("missing catalog.yaml in bundled policy library")
    if not installed_library.get("module_ids"):
        validation_problems.append("bundled policy library has no enumerated modules")
    if not installed_library.get("profile_ids"):
        validation_problems.append("bundled policy library has no enumerated profiles")
    return {
        "selector_root": str(selector_root),
        "policy_library_root": str(output_root),
        "manifest_path": str(selector_root / "release-manifest.json"),
        "bundle_version": manifest["bundle_version"],
        "release_ref": manifest["release_ref"],
        "source_commit": manifest["source_commit"],
        "source_tree_state": manifest["source_tree_state"],
        "source_ref": manifest.get("source_ref", ""),
        "module_count": len(installed_library.get("module_ids", [])),
        "profile_count": len(installed_library.get("profile_ids", [])),
        "validation_problems": validation_problems,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--selector-root")
    parser.add_argument("--bundle-version", required=True)
    parser.add_argument("--release-ref")
    parser.add_argument("--source-ref")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    source_root = Path(args.source_root).resolve()
    selector_root = (
        Path(args.selector_root).resolve()
        if args.selector_root
        else Path(__file__).resolve().parents[1]
    )
    report = release_bundle(
        source_root=source_root,
        selector_root=selector_root,
        bundle_version=args.bundle_version,
        release_ref=args.release_ref,
        source_ref=args.source_ref,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"selector_root: {report['selector_root']}")
        print(f"policy_library_root: {report['policy_library_root']}")
        print(f"manifest_path: {report['manifest_path']}")
        print(f"bundle_version: {report['bundle_version']}")
        print(f"release_ref: {report['release_ref']}")
        print(f"source_commit: {report['source_commit']}")
        print(f"source_tree_state: {report['source_tree_state']}")
        print(f"source_ref: {report['source_ref'] or '-'}")
        print(f"module_count: {report['module_count']}")
        print(f"profile_count: {report['profile_count']}")
        print(f"validation_problems: {', '.join(report['validation_problems']) or '-'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
