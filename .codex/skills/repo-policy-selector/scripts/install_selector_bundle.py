#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys

sys.dont_write_bytecode = True

import tempfile
from pathlib import Path
from typing import Any

INSTALL_RECORD_RELPATH = ".codex/policy-selector-install.json"


def copy_selector_bundle(
    selector_root: Path,
    target_repo_root: Path,
    install_relpath: str,
    force: bool,
) -> Path:
    install_root = target_repo_root / install_relpath
    if install_root.exists():
        if not force:
            raise FileExistsError(f"install target already exists: {install_root}")
        shutil.rmtree(install_root)
    shutil.copytree(
        selector_root,
        install_root,
        ignore=shutil.ignore_patterns("__pycache__", "*.pyc", ".DS_Store"),
    )
    return install_root


def clone_selector_source(bundle_git_url: str, bundle_ref: str, selector_subdir: str) -> tuple[tempfile.TemporaryDirectory[str], Path]:
    temp_dir = tempfile.TemporaryDirectory(prefix="repo-policy-selector-")
    clone_root = Path(temp_dir.name) / "source"
    subprocess.run(
        ["git", "clone", "--depth", "1", "--branch", bundle_ref, bundle_git_url, str(clone_root)],
        check=True,
        capture_output=True,
        text=True,
    )
    selector_root = clone_root / selector_subdir
    if not selector_root.exists():
        raise FileNotFoundError(f"selector subdir not found in cloned repo: {selector_root}")
    return temp_dir, selector_root


def run_installed_adopt(
    install_root: Path,
    target_repo_root: Path,
    write_drafts: bool,
) -> dict[str, Any]:
    cmd = [
        sys.executable,
        str(install_root / "scripts" / "manage_policy.py"),
        "--repo-root",
        str(target_repo_root),
        "--policy-root",
        str(install_root / "policy-library"),
        "adopt",
        "--json",
    ]
    if write_drafts:
        cmd.insert(-1, "--write-drafts")
    result = subprocess.run(cmd, check=True, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return payload if isinstance(payload, dict) else {}


def write_install_record(target_repo_root: Path, record: dict[str, Any]) -> Path:
    record_path = target_repo_root / INSTALL_RECORD_RELPATH
    record_path.parent.mkdir(parents=True, exist_ok=True)
    record_path.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return record_path


def install_selector_bundle(
    *,
    selector_root: Path | None,
    bundle_git_url: str | None,
    bundle_ref: str | None,
    selector_subdir: str,
    target_repo_root: Path,
    install_relpath: str,
    force: bool,
    write_drafts: bool,
) -> dict[str, Any]:
    temp_dir: tempfile.TemporaryDirectory[str] | None = None
    source_descriptor: dict[str, Any]
    resolved_selector_root: Path
    if selector_root is not None:
        resolved_selector_root = selector_root
        source_descriptor = {
            "source_type": "local-path",
            "selector_root": str(selector_root),
        }
    else:
        if not bundle_git_url or not bundle_ref:
            raise ValueError("either selector_root or bundle_git_url + bundle_ref is required")
        temp_dir, resolved_selector_root = clone_selector_source(bundle_git_url, bundle_ref, selector_subdir)
        source_descriptor = {
            "source_type": "git-ref",
            "bundle_git_url": bundle_git_url,
            "bundle_ref": bundle_ref,
            "selector_subdir": selector_subdir,
        }

    try:
        install_root = copy_selector_bundle(resolved_selector_root, target_repo_root, install_relpath, force)
        manifest_path = install_root / "release-manifest.json"
        manifest: dict[str, Any] = {}
        if manifest_path.exists():
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        adopt_report = run_installed_adopt(install_root, target_repo_root, write_drafts)
        install_record = {
            **source_descriptor,
            "install_relpath": install_relpath,
            "installed_selector_root": str(install_root),
            "installed_policy_root": str(install_root / "policy-library"),
            "installed_bundle_release": manifest,
        }
        install_record_path = write_install_record(target_repo_root, install_record)
        return {
            "target_repo_root": str(target_repo_root),
            "installed_selector_root": str(install_root),
            "installed_policy_root": str(install_root / "policy-library"),
            "install_relpath": install_relpath,
            "installed_bundle_release": manifest,
            "install_record_path": str(install_record_path),
            "install_source": source_descriptor,
            "write_drafts": write_drafts,
            "adopt_report": adopt_report,
        }
    finally:
        if temp_dir is not None:
            temp_dir.cleanup()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--selector-root")
    parser.add_argument("--bundle-git-url")
    parser.add_argument("--bundle-ref")
    parser.add_argument("--selector-subdir", default="repo-policy-selector")
    parser.add_argument("--target-repo-root", required=True)
    parser.add_argument("--install-relpath", default=".codex/skills/repo-policy-selector")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--write-drafts", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    selector_root = Path(args.selector_root).resolve() if args.selector_root else None
    report = install_selector_bundle(
        selector_root=selector_root,
        bundle_git_url=args.bundle_git_url,
        bundle_ref=args.bundle_ref,
        selector_subdir=args.selector_subdir,
        target_repo_root=Path(args.target_repo_root).resolve(),
        install_relpath=args.install_relpath,
        force=args.force,
        write_drafts=args.write_drafts,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"target_repo_root: {report['target_repo_root']}")
        print(f"installed_selector_root: {report['installed_selector_root']}")
        print(f"installed_policy_root: {report['installed_policy_root']}")
        print(f"install_relpath: {report['install_relpath']}")
        print(f"install_record_path: {report['install_record_path']}")
        bundle_version = report["installed_bundle_release"].get("bundle_version", "-")
        print(f"installed_bundle_version: {bundle_version}")
        print(f"write_drafts: {report['write_drafts']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
