#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import shutil
from pathlib import Path


def reset_dir(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


def copy_tree(src: Path, dst: Path) -> None:
    if src.exists():
        shutil.copytree(src, dst, dirs_exist_ok=True)


def sync_policy_library(source_root: Path, output_root: Path) -> Path:
    reset_dir(output_root)
    copy_tree(source_root / "modules", output_root / "modules")
    copy_tree(source_root / "profiles", output_root / "profiles")
    shutil.copy2(source_root / "catalog.yaml", output_root / "catalog.yaml")
    shutil.copy2(source_root / "SCHEMA.md", output_root / "SCHEMA.md")
    return output_root


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", required=True)
    parser.add_argument("--output-root")
    args = parser.parse_args()

    source_root = Path(args.source_root).resolve()
    output_root = (
        Path(args.output_root).resolve()
        if args.output_root
        else Path(__file__).resolve().parents[1] / "policy-library"
    )

    sync_policy_library(source_root, output_root)

    print(f"synced policy library to {output_root}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
