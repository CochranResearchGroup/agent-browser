#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


sys.dont_write_bytecode = True


class CatalogSyntaxError(ValueError):
    pass


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    environment = os.environ.copy()
    environment["GIT_OPTIONAL_LOCKS"] = "0"
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        check=check,
        capture_output=True,
        text=True,
        env=environment,
    )


def parse_scalar(value: str) -> Any:
    value = value.strip()
    if value == "[]":
        return []
    if value.startswith("[") and value.endswith("]"):
        return [item.strip() for item in value[1:-1].split(",") if item.strip()]
    if value.isdigit():
        return int(value)
    return value


def parse_catalog(text: str) -> dict[str, Any]:
    catalog: dict[str, Any] = {"lanes": []}
    current: dict[str, Any] | None = None
    in_lanes = False
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not raw_line.startswith(" ") and stripped == "lanes:":
            in_lanes = True
            continue
        if not raw_line.startswith(" ") and ":" in stripped:
            key, value = stripped.split(":", 1)
            if key.strip() in catalog and key.strip() != "lanes":
                raise CatalogSyntaxError(f"invalid catalog syntax at line {line_number}: duplicate key {key.strip()}")
            catalog[key.strip()] = parse_scalar(value)
            in_lanes = False
            continue
        if in_lanes and raw_line.startswith("  - "):
            if ":" not in stripped[2:]:
                raise CatalogSyntaxError(
                    f"invalid catalog syntax at line {line_number}: expected key: value"
                )
            current = {}
            catalog["lanes"].append(current)
            key, value = stripped[2:].split(":", 1)
            current[key.strip()] = parse_scalar(value)
            continue
        if in_lanes and current is not None and raw_line.startswith("    ") and ":" in stripped:
            key, value = stripped.split(":", 1)
            if key.strip() in current:
                raise CatalogSyntaxError(
                    f"invalid catalog syntax at line {line_number}: duplicate key {key.strip()}"
                )
            current[key.strip()] = parse_scalar(value)
            continue
        raise CatalogSyntaxError(
            f"invalid catalog syntax at line {line_number}: expected key: value"
        )
    return catalog


def worktree_inventory(repo: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    current: dict[str, Any] = {}
    for line in git(repo, "worktree", "list", "--porcelain").stdout.splitlines() + [""]:
        if not line:
            if current:
                records.append(current)
                current = {}
            continue
        key, _, value = line.partition(" ")
        if key in {"bare", "detached", "locked", "prunable"} and not value:
            current[key] = True
        else:
            current[key] = value
    for record in records:
        path = record.get("worktree")
        if not isinstance(path, str):
            record["status"] = []
            continue
        status = git(Path(path), "status", "--porcelain=v1", "--untracked-files=all", check=False)
        record["status"] = status.stdout.splitlines() if status.returncode == 0 else ["<unreadable>"]
    return records


def ref_tip(repo: Path, ref: str) -> str | None:
    result = git(repo, "rev-parse", "--verify", ref, check=False)
    return result.stdout.strip() if result.returncode == 0 else None


def read_ref_file(repo: Path, ref: str, path: str) -> str | None:
    result = git(repo, "show", f"{ref}:{path}", check=False)
    return result.stdout if result.returncode == 0 else None


def plan_metadata(text: str) -> dict[str, str]:
    metadata: dict[str, str] = {}
    for key in ("state", "lane", "branch", "target", "integration"):
        match = re.search(rf"(?im)^{key}\s*:\s*(.+?)\s*$", text)
        if match:
            metadata[key] = match.group(1).strip()
    return metadata


DEFAULT_BRANCH_PREFIXES = (
    "feature/",
    "feat/",
    "fix/",
    "field/",
    "plan/",
    "plan",
    "codex/",
    "integration/",
)


def branch_ref_inventory(
    repo: Path, *, remote: str, branch_prefixes: tuple[str, ...]
) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    output = git(
        repo,
        "for-each-ref",
        "--format=%(refname)",
        "refs/heads",
        f"refs/remotes/{remote}",
    ).stdout
    for ref in output.splitlines():
        if ref.endswith("/HEAD"):
            continue
        if ref.startswith("refs/heads/"):
            branch = ref.removeprefix("refs/heads/")
            if not branch.startswith(branch_prefixes):
                continue
            result.setdefault(branch, {})["local_ref"] = ref
        elif ref.startswith(f"refs/remotes/{remote}/"):
            branch = ref.removeprefix(f"refs/remotes/{remote}/")
            if not branch.startswith(branch_prefixes):
                continue
            result.setdefault(branch, {})["remote_ref"] = ref
    return result


def exact_branch_ref_inventory(
    repo: Path, *, remote: str, branches: tuple[str, ...]
) -> dict[str, dict[str, str]]:
    result: dict[str, dict[str, str]] = {}
    for branch in branches:
        local_ref = f"refs/heads/{branch}"
        remote_ref = f"refs/remotes/{remote}/{branch}"
        if ref_tip(repo, local_ref):
            result.setdefault(branch, {})["local_ref"] = local_ref
        if ref_tip(repo, remote_ref):
            result.setdefault(branch, {})["remote_ref"] = remote_ref
    return result


def discover_open_plans(repo: Path, ref: str, plans_dir: str) -> list[dict[str, str]]:
    listing = git(repo, "ls-tree", "-r", "--name-only", ref, "--", plans_dir, check=False)
    if listing.returncode != 0:
        return []
    plans: list[dict[str, str]] = []
    for path in listing.stdout.splitlines():
        if not path.endswith(".md"):
            continue
        body = read_ref_file(repo, ref, path)
        if body is None:
            continue
        metadata = plan_metadata(body)
        if metadata.get("state") not in {"PLANNED", "OPEN", "BLOCKED"}:
            continue
        if not metadata.get("lane") or not metadata.get("branch"):
            continue
        metadata["plan"] = path
        metadata["plan_ref"] = ref
        plans.append(metadata)
    return plans


def shared_refs_containing(repo: Path, commit: str) -> list[str]:
    result = git(
        repo,
        "for-each-ref",
        f"--contains={commit}",
        "--format=%(refname)",
        "refs/heads",
        "refs/tags",
        "refs/remotes",
        check=False,
    )
    return result.stdout.splitlines() if result.returncode == 0 else []


def is_ancestor(repo: Path, ancestor: str | None, descendant: str | None) -> bool:
    if not ancestor or not descendant:
        return False
    return git(repo, "merge-base", "--is-ancestor", ancestor, descendant, check=False).returncode == 0


def local_remote_relation(repo: Path, local_tip: str | None, remote_tip: str | None) -> str:
    if not local_tip and not remote_tip:
        return "missing"
    if local_tip and not remote_tip:
        return "local_only"
    if remote_tip and not local_tip:
        return "remote_only"
    if local_tip == remote_tip:
        return "equal"
    if is_ancestor(repo, remote_tip, local_tip):
        return "local_ahead"
    if is_ancestor(repo, local_tip, remote_tip):
        return "remote_ahead"
    return "diverged"


def audit_repo(
    repo: Path,
    *,
    default_ref: str,
    catalog_path: str = "docs/dev/active-lanes.yaml",
    plans_dir: str = "docs/dev/plans",
    remote: str = "origin",
    branch_prefixes: tuple[str, ...] = DEFAULT_BRANCH_PREFIXES,
    catalog_only: bool = False,
    selected_branches: tuple[str, ...] = (),
) -> dict[str, Any]:
    problems: list[str] = []
    catalog_text = read_ref_file(repo, default_ref, catalog_path)
    if catalog_text is None:
        return {
            "schema_version": 1,
            "repo_root": str(repo),
            "default_ref": default_ref,
            "catalog_path": catalog_path,
            "lanes": [],
            "problems": [f"catalog not found at {default_ref}:{catalog_path}"],
            "ok": False,
        }

    try:
        catalog = parse_catalog(catalog_text)
    except CatalogSyntaxError as error:
        return {
            "schema_version": 1,
            "repo_root": str(repo),
            "default_ref": default_ref,
            "catalog_path": catalog_path,
            "lanes": [],
            "problems": [str(error)],
            "ok": False,
        }
    if catalog.get("schema_version") != 1:
        problems.append("catalog schema_version must be 1")
    lanes_value = catalog.get("lanes")
    if not isinstance(lanes_value, list):
        lanes_value = []
        problems.append("catalog lanes must be a list")
    lane_ids = [str(lane.get("id", "")) for lane in lanes_value if isinstance(lane, dict)]
    for lane_id in sorted(set(lane_ids)):
        if lane_id and lane_ids.count(lane_id) > 1:
            problems.append(f"duplicate lane id: {lane_id}")
    branch_names = [
        str(lane.get("branch", "")) for lane in lanes_value if isinstance(lane, dict)
    ]
    for branch in sorted(set(branch_names)):
        if branch and branch_names.count(branch) > 1:
            problems.append(f"branch claimed by multiple lanes: {branch}")

    required_fields = (
        "id",
        "objective",
        "plan",
        "plan_ref",
        "branch",
        "target",
        "plan_state",
        "custody_state",
        "checkpoint",
        "remote_ref",
        "integration",
        "dependencies",
        "overlaps",
        "updated_at",
    )
    known_lane_ids = {lane_id for lane_id in lane_ids if lane_id}
    allowed_plan_states = {"PLANNED", "OPEN", "BLOCKED", "CLOSED", "CANCELLED"}
    allowed_custody_states = {
        "ACTIVE_WORKTREE",
        "PAUSED_REF",
        "INTEGRATION_READY",
        "INTEGRATED",
        "ARCHIVED",
        "DISCARD_APPROVED",
    }
    for lane in lanes_value:
        if not isinstance(lane, dict):
            problems.append("catalog lane entries must be mappings")
            continue
        lane_id = str(lane.get("id", "<unknown>"))
        for field in required_fields:
            if field not in lane or lane[field] == "":
                problems.append(f"{lane_id}: missing required catalog field: {field}")
        dependencies = lane.get("dependencies", [])
        if not isinstance(dependencies, list):
            problems.append(f"{lane_id}: dependencies must be a list")
        else:
            for dependency in dependencies:
                if dependency not in known_lane_ids:
                    problems.append(f"{lane_id}: unknown dependency lane: {dependency}")
        if lane.get("plan_state") not in allowed_plan_states:
            problems.append(f"{lane_id}: invalid plan_state: {lane.get('plan_state')}")
        if lane.get("custody_state") not in allowed_custody_states:
            problems.append(f"{lane_id}: invalid custody_state: {lane.get('custody_state')}")
        for field in ("overlaps", "reconciled_overlaps"):
            if field in lane and not isinstance(lane[field], list):
                problems.append(f"{lane_id}: {field} must be a list")

    worktrees = worktree_inventory(repo)
    if catalog_only:
        branch_refs = {}
    elif selected_branches:
        branch_refs = exact_branch_ref_inventory(
            repo, remote=remote, branches=selected_branches
        )
    else:
        branch_refs = branch_ref_inventory(
            repo, remote=remote, branch_prefixes=branch_prefixes
        )
    lanes: list[dict[str, Any]] = []
    for catalog_lane in lanes_value:
        if not isinstance(catalog_lane, dict):
            continue
        lane_id = str(catalog_lane.get("id", ""))
        branch = str(catalog_lane.get("branch", ""))
        local_ref = f"refs/heads/{branch}" if branch else ""
        local_tip = ref_tip(repo, local_ref) if local_ref else None
        remote_ref = str(catalog_lane.get("remote_ref", ""))
        remote_tip = ref_tip(repo, remote_ref) if remote_ref else None
        archive_ref = str(catalog_lane.get("archive_ref", ""))
        archive_remote_ref = str(catalog_lane.get("archive_remote_ref", ""))
        archive_local_tip = ref_tip(repo, archive_ref) if archive_ref else None
        archive_remote_tip = ref_tip(repo, archive_remote_ref) if archive_remote_ref else None
        lane_worktrees = [
            item
            for item in worktrees
            if local_ref and item.get("branch") == local_ref
        ]
        findings: list[str] = []
        custody_state = catalog_lane.get("custody_state")
        if custody_state in {"ACTIVE_WORKTREE", "PAUSED_REF", "INTEGRATION_READY"} and not (
            local_tip or remote_tip
        ):
            findings.append("registered_but_missing")
            problems.append(f"{lane_id}: registered branch has no local or remote ref")

        plan_state = str(catalog_lane.get("plan_state", ""))
        if plan_state in {"PLANNED", "OPEN", "BLOCKED"}:
            plan_ref = str(catalog_lane.get("plan_ref", ""))
            plan_path = str(catalog_lane.get("plan", ""))
            plan_body = read_ref_file(repo, plan_ref, plan_path) if plan_ref and plan_path else None
            if plan_body is None:
                findings.append("plan_catalog_drift")
                problems.append(f"{lane_id}: registered plan is not readable at plan_ref")
            else:
                metadata = plan_metadata(plan_body)
                comparisons = (
                    ("state", "plan_state"),
                    ("lane", "id"),
                    ("branch", "branch"),
                    ("target", "target"),
                    ("integration", "integration"),
                )
                for plan_key, catalog_key in comparisons:
                    plan_value = metadata.get(plan_key)
                    catalog_value = str(catalog_lane.get(catalog_key, ""))
                    if plan_value != catalog_value:
                        if "plan_catalog_drift" not in findings:
                            findings.append("plan_catalog_drift")
                        problems.append(
                            f"{lane_id}: plan {plan_key} {plan_value or '<missing>'} "
                            f"does not match catalog {plan_key} {catalog_value or '<missing>'}"
                        )
        if (
            local_tip
            and lane_worktrees
            and catalog_lane.get("plan_state") == "OPEN"
            and custody_state == "ACTIVE_WORKTREE"
        ):
            findings.append("registered_active")
        if local_tip and remote_ref and remote_tip is None:
            findings.append("local_only")
            problems.append(f"{lane_id}: local branch has no configured remote custody")
        if any(item.get("status") for item in lane_worktrees):
            findings.append("dirty_uncheckpointed")
            problems.append(f"{lane_id}: assigned worktree has uncommitted state")
        checkpoint = str(catalog_lane.get("checkpoint", ""))
        ref_relation = local_remote_relation(repo, local_tip, remote_tip)
        if checkpoint and local_tip and checkpoint != local_tip:
            findings.append("stale_checkpoint")
            problems.append(f"{lane_id}: catalog checkpoint does not match the local branch tip")
        if custody_state == "ACTIVE_WORKTREE" and ref_relation == "local_ahead":
            findings.append("local_ahead_of_remote")
            problems.append(f"{lane_id}: active local checkpoint is ahead of remote custody")
        if custody_state == "ACTIVE_WORKTREE" and ref_relation == "remote_ahead":
            findings.append("remote_ahead_of_local")
            problems.append(f"{lane_id}: active remote custody is ahead of the local checkpoint")
        if custody_state == "ACTIVE_WORKTREE" and ref_relation == "diverged":
            findings.append("local_remote_diverged")
            problems.append(f"{lane_id}: active local and remote custody have diverged")
        if custody_state == "ACTIVE_WORKTREE" and not lane_worktrees:
            if "registered_but_missing" not in findings:
                findings.append("registered_but_missing")
            problems.append(f"{lane_id}: ACTIVE_WORKTREE lane has no assigned worktree")
        if (
            custody_state == "PAUSED_REF"
            and local_tip
            and remote_tip
            and not lane_worktrees
        ):
            findings.append("paused_ref")
        if (
            custody_state == "ARCHIVED"
            and archive_local_tip
            and archive_local_tip == archive_remote_tip == checkpoint
            and not lane_worktrees
        ):
            findings.append("archived_ref")
        if custody_state == "ARCHIVED" and "archived_ref" not in findings:
            problems.append(f"{lane_id}: ARCHIVED state lacks matching local and remote archive refs")
        if catalog_lane.get("plan_state") == "CLOSED" and custody_state == "ACTIVE_WORKTREE":
            findings.append("closed_plan_live_branch")
            problems.append(f"{lane_id}: CLOSED plan still has active worktree custody")
        overlaps = catalog_lane.get("overlaps", [])
        reconciled_overlaps = catalog_lane.get("reconciled_overlaps", [])
        if not isinstance(overlaps, list):
            overlaps = []
        if not isinstance(reconciled_overlaps, list):
            reconciled_overlaps = []
        unreconciled_overlaps = [item for item in overlaps if item not in reconciled_overlaps]
        if unreconciled_overlaps:
            findings.append("overlap_unreconciled")
            for overlap in unreconciled_overlaps:
                problems.append(f"{lane_id}: declared overlap lacks disposition: {overlap}")
        target = str(catalog_lane.get("target", ""))
        target_ref = target if target.startswith("refs/") else f"refs/heads/{target}"
        target_tip = ref_tip(repo, target_ref) if target else None
        integrated_into_target = is_ancestor(repo, local_tip, target_tip)
        integration_receipt = str(catalog_lane.get("integration_receipt", ""))
        receipt_verified = bool(
            integration_receipt
            and ref_tip(repo, integration_receipt)
            and is_ancestor(repo, integration_receipt, target_tip)
        )
        readiness_evidenced = (
            custody_state == "INTEGRATION_READY"
            and local_tip
            and local_tip == remote_tip == checkpoint
            and catalog_lane.get("validation_status") == "passed"
            and catalog_lane.get("validation_ref") == checkpoint
            and not any(item.get("status") for item in lane_worktrees)
            and not unreconciled_overlaps
            and not integrated_into_target
        )
        if readiness_evidenced:
            findings.append("integration_ready")
        if custody_state == "INTEGRATION_READY" and not readiness_evidenced:
            findings.append("integration_ambiguous")
            problems.append(f"{lane_id}: INTEGRATION_READY state lacks complete readiness evidence")
        if custody_state == "INTEGRATED" and not (integrated_into_target or receipt_verified):
            findings.append("integration_ambiguous")
            problems.append(
                f"{lane_id}: INTEGRATED state lacks ancestry or a verified integration receipt"
            )
        if (
            custody_state == "INTEGRATED"
            and (integrated_into_target or receipt_verified)
            and (local_tip or remote_tip or lane_worktrees)
        ):
            findings.append("integrated_cleanup_pending")
        lane_report = dict(catalog_lane)
        lane_report.update(
            {
                "id": lane_id,
                "local_tip": local_tip,
                "remote_tip": remote_tip,
                "local_remote_relation": ref_relation,
                "worktrees": lane_worktrees,
                "findings": findings,
                "unreconciled_overlaps": unreconciled_overlaps,
                "integrated_into_target": integrated_into_target,
                "integration_receipt_verified": receipt_verified,
                "archive_local_tip": archive_local_tip,
                "archive_remote_tip": archive_remote_tip,
            }
        )
        lanes.append(lane_report)

    registered_branches = {str(item.get("branch", "")) for item in lanes}
    default_branch = default_ref.rsplit("/", 1)[-1]
    for branch, refs in sorted(branch_refs.items()):
        if branch in registered_branches or branch == default_branch:
            continue
        source_ref = refs.get("local_ref") or refs.get("remote_ref")
        if not source_ref:
            continue
        for metadata in discover_open_plans(repo, source_ref, plans_dir):
            if metadata.get("branch") != branch:
                continue
            lane_id = metadata["lane"]
            local_ref = refs.get("local_ref")
            remote_ref = refs.get("remote_ref")
            local_tip = ref_tip(repo, local_ref) if local_ref else None
            remote_tip = ref_tip(repo, remote_ref) if remote_ref else None
            lane_worktrees = [
                item for item in worktrees if local_ref and item.get("branch") == local_ref
            ]
            lanes.append(
                {
                    "id": lane_id,
                    "objective": "",
                    "plan": metadata["plan"],
                    "plan_ref": source_ref,
                    "branch": branch,
                    "target": metadata.get("target", ""),
                    "plan_state": metadata["state"],
                    "custody_state": "ACTIVE_WORKTREE" if lane_worktrees else "PAUSED_REF",
                    "checkpoint": local_tip or remote_tip,
                    "remote_ref": remote_ref,
                    "integration": metadata.get("integration", ""),
                    "dependencies": [],
                    "overlaps": [],
                    "updated_at": "",
                    "local_tip": local_tip,
                    "remote_tip": remote_tip,
                    "worktrees": lane_worktrees,
                    "findings": ["unregistered_active"],
                }
            )
            problems.append(f"{lane_id}: active branch is absent from the default-ref catalog")

    reported_lane_ids = {str(item.get("id", "")) for item in lanes}
    for worktree in (() if catalog_only or selected_branches else worktrees):
        if not worktree.get("detached"):
            continue
        commit = str(worktree.get("HEAD", ""))
        if not commit or shared_refs_containing(repo, commit):
            continue
        for metadata in discover_open_plans(repo, commit, plans_dir):
            lane_id = metadata["lane"]
            if lane_id in reported_lane_ids:
                continue
            lanes.append(
                {
                    "id": lane_id,
                    "objective": "",
                    "plan": metadata["plan"],
                    "plan_ref": commit,
                    "branch": metadata["branch"],
                    "target": metadata.get("target", ""),
                    "plan_state": metadata["state"],
                    "custody_state": "ACTIVE_WORKTREE",
                    "checkpoint": commit,
                    "remote_ref": None,
                    "integration": metadata.get("integration", ""),
                    "dependencies": [],
                    "overlaps": [],
                    "updated_at": "",
                    "local_tip": commit,
                    "remote_tip": None,
                    "worktrees": [worktree],
                    "findings": ["orphaned_detached"],
                }
            )
            reported_lane_ids.add(lane_id)
            problems.append(f"{lane_id}: detached worktree commit is not reachable from a shared ref")

    return {
        "schema_version": 1,
        "repo_root": str(repo),
        "default_ref": default_ref,
        "catalog_path": catalog_path,
        "remote": remote,
        "branch_prefixes": (
            [] if catalog_only or selected_branches else list(branch_prefixes)
        ),
        "discovery_mode": (
            "catalog-only" if catalog_only
            else "exact-branches" if selected_branches
            else "prefixes"
        ),
        "selected_branches": list(selected_branches),
        "lanes": lanes,
        "problems": problems,
        "ok": not problems,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--default-ref", required=True)
    parser.add_argument("--catalog-path", default="docs/dev/active-lanes.yaml")
    parser.add_argument("--plans-dir", default="docs/dev/plans")
    parser.add_argument("--remote", default="origin")
    discovery_group = parser.add_mutually_exclusive_group()
    discovery_group.add_argument(
        "--catalog-only",
        action="store_true",
        help="audit only lanes registered in the default-ref catalog",
    )
    discovery_group.add_argument(
        "--branch",
        action="append",
        dest="selected_branches",
        help="exact topic branch to inspect for unregistered plans; repeat as needed",
    )
    discovery_group.add_argument(
        "--branch-prefix",
        action="append",
        dest="branch_prefixes",
        help="topic branch prefix to include; repeat to configure multiple prefixes",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = audit_repo(
        Path(args.repo_root).resolve(),
        default_ref=args.default_ref,
        catalog_path=args.catalog_path,
        plans_dir=args.plans_dir,
        remote=args.remote,
        branch_prefixes=tuple(args.branch_prefixes or DEFAULT_BRANCH_PREFIXES),
        catalog_only=args.catalog_only,
        selected_branches=tuple(args.selected_branches or ()),
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"ok: {report['ok']}")
        for problem in report["problems"]:
            print(f"- {problem}")
        for lane in report["lanes"]:
            print(f"{lane['id']}: {', '.join(lane['findings']) or 'unclassified'}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
