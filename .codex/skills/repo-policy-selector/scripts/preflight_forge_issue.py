#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import urllib.parse
from pathlib import Path
from typing import Any


ACTIONS = {
    "read",
    "create",
    "comment",
    "edit",
    "apply_labels",
    "create_labels",
    "assign",
    "milestone",
    "planning",
    "close",
    "reopen",
    "transfer",
}
RELATIONSHIPS = {"owned", "permissioned"}
SECURITY_ROUTES = {
    "private_vulnerability_reporting",
    "security_policy",
    "private_contact",
    "confidential_issue",
    "none",
}
GITHUB_ROLES = {"NONE": 0, "READ": 10, "TRIAGE": 20, "WRITE": 30, "MAINTAIN": 40, "ADMIN": 50}
GITLAB_ROLES = {
    "NO_ACCESS": 0,
    "MINIMAL_ACCESS": 5,
    "GUEST": 10,
    "PLANNER": 15,
    "REPORTER": 20,
    "SECURITY_MANAGER": 25,
    "DEVELOPER": 30,
    "MAINTAINER": 40,
    "OWNER": 50,
}
GITLAB_ACCESS_NAMES = {value: key for key, value in GITLAB_ROLES.items()}
PROVIDER_TIMEOUT_SECONDS = 20

MINIMUM_ROLE = {
    "github": {
        "read": "READ",
        "create": "READ",
        "comment": "READ",
        "edit": "TRIAGE",
        "apply_labels": "TRIAGE",
        "create_labels": "WRITE",
        "assign": "TRIAGE",
        "milestone": "TRIAGE",
        "planning": "WRITE",
        "close": "TRIAGE",
        "reopen": "TRIAGE",
        "transfer": "WRITE",
    },
    "gitlab": {
        "read": "GUEST",
        "create": "GUEST",
        "comment": "GUEST",
        "edit": "PLANNER",
        "apply_labels": "PLANNER",
        "create_labels": "MAINTAINER",
        "assign": "PLANNER",
        "milestone": "PLANNER",
        "planning": "PLANNER",
        "close": "PLANNER",
        "reopen": "PLANNER",
        "transfer": "MAINTAINER",
    },
}


class PreflightError(RuntimeError):
    pass


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise PreflightError(f"file not found: {path}") from exc
    except json.JSONDecodeError as exc:
        raise PreflightError(f"invalid JSON in {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PreflightError(f"expected a JSON object in {path}")
    return value


def validate_registry(registry: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    if registry.get("schema_version") != 1:
        problems.append("registry schema_version must be 1")
    targets = registry.get("targets")
    if not isinstance(targets, list):
        return problems + ["registry targets must be a list"]
    seen: set[str] = set()
    for index, target in enumerate(targets):
        prefix = f"targets[{index}]"
        if not isinstance(target, dict):
            problems.append(f"{prefix} must be an object")
            continue
        target_id = target.get("id")
        if not isinstance(target_id, str) or not target_id.strip():
            problems.append(f"{prefix}.id must be a non-empty string")
        elif target_id in seen:
            problems.append(f"duplicate target id: {target_id}")
        else:
            seen.add(target_id)
        forge = target.get("forge")
        if forge not in {"github", "gitlab"}:
            problems.append(f"{prefix}.forge must be github or gitlab")
        for field in ("host", "repository"):
            if not isinstance(target.get(field), str) or not target[field].strip():
                problems.append(f"{prefix}.{field} must be a non-empty string")
        repository = target.get("repository", "")
        minimum_parts = 2 if forge == "github" else 2
        if isinstance(repository, str) and len([part for part in repository.split("/") if part]) < minimum_parts:
            problems.append(f"{prefix}.repository must include a namespace and repository")
        if forge == "github" and isinstance(repository, str) and len(repository.split("/")) != 2:
            problems.append(f"{prefix}.repository must use OWNER/REPO for GitHub")
        if target.get("relationship") not in RELATIONSHIPS:
            problems.append(f"{prefix}.relationship must be owned or permissioned")
        allowed = target.get("allowed_actions")
        if not isinstance(allowed, list) or any(action not in ACTIONS for action in allowed):
            problems.append(f"{prefix}.allowed_actions contains an unsupported action")
        elif len(allowed) != len(set(allowed)):
            problems.append(f"{prefix}.allowed_actions contains duplicates")
        if target.get("security_route") not in SECURITY_ROUTES:
            problems.append(f"{prefix}.security_route is unsupported")
        elif forge == "github" and target.get("security_route") == "confidential_issue":
            problems.append(f"{prefix}.security_route confidential_issue is GitLab-specific")
        elif forge == "gitlab" and target.get("security_route") == "private_vulnerability_reporting":
            problems.append(f"{prefix}.security_route private_vulnerability_reporting is GitHub-specific")
        label_map = target.get("label_map")
        if not isinstance(label_map, dict):
            problems.append(f"{prefix}.label_map must be an object")
        else:
            provider_labels: set[str] = set()
            for intent, mapping in label_map.items():
                if not isinstance(intent, str) or not intent.strip():
                    problems.append(f"{prefix}.label_map has an empty intent key")
                    continue
                if not isinstance(mapping, dict) or not isinstance(mapping.get("provider_label"), str) or not mapping["provider_label"].strip():
                    problems.append(f"{prefix}.label_map[{intent}] needs provider_label")
                    continue
                provider_label = mapping["provider_label"]
                if provider_label in provider_labels:
                    problems.append(f"{prefix}.label_map maps more than one intent to {provider_label}")
                provider_labels.add(provider_label)
    return problems


def select_target(registry: dict[str, Any], target_id: str) -> dict[str, Any]:
    matches = [target for target in registry.get("targets", []) if target.get("id") == target_id]
    if len(matches) != 1:
        raise PreflightError(f"target id must resolve exactly once: {target_id}")
    return matches[0]


def run_json(command: list[str], *, env: dict[str, str] | None = None) -> Any:
    try:
        result = subprocess.run(
            command,
            text=True,
            capture_output=True,
            env=env,
            check=False,
            timeout=PROVIDER_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as exc:
        raise PreflightError(
            f"read-only provider command timed out after {PROVIDER_TIMEOUT_SECONDS}s: {command[0]}"
        ) from exc
    if result.returncode != 0:
        detail = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "command failed"
        raise PreflightError(f"read-only provider command failed: {command[0]}: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise PreflightError(f"provider returned invalid JSON from {command[0]}") from exc


def label_names(value: Any) -> list[str]:
    if isinstance(value, dict):
        value = value.get("nodes", [])
    if not isinstance(value, list):
        return []
    names: list[str] = []
    for item in value:
        if isinstance(item, str):
            names.append(item)
        elif isinstance(item, dict) and isinstance(item.get("name"), str):
            names.append(item["name"])
    return sorted(set(names))


def inspect_github(target: dict[str, Any], idempotency_key: str | None) -> dict[str, Any]:
    if shutil.which("gh") is None:
        raise PreflightError("gh is not installed")
    host = target["host"]
    repository = target["repository"]
    env = os.environ.copy()
    env["GH_HOST"] = host
    actor = run_json(["gh", "api", "--hostname", host, "user"], env=env).get("login")
    project = run_json(
        [
            "gh",
            "repo",
            "view",
            repository,
            "--json",
            "nameWithOwner,viewerPermission,hasIssuesEnabled,isArchived,isFork,url,labels",
        ],
        env=env,
    )
    duplicates: list[dict[str, Any]] = []
    if idempotency_key:
        found = run_json(
            [
                "gh",
                "issue",
                "list",
                "--repo",
                repository,
                "--state",
                "all",
                "--search",
                idempotency_key,
                "--limit",
                "20",
                "--json",
                "number,title,url,state",
            ],
            env=env,
        )
        duplicates = found if isinstance(found, list) else []
    return {
        "forge": "github",
        "host": host,
        "repository": project.get("nameWithOwner"),
        "authenticated_actor": actor,
        "effective_role": project.get("viewerPermission", "NONE"),
        "issues_enabled": bool(project.get("hasIssuesEnabled")),
        "archived": bool(project.get("isArchived")),
        "available_labels": label_names(project.get("labels")),
        "duplicate_candidates": duplicates,
        "url": project.get("url"),
        "is_fork": bool(project.get("isFork")),
    }


def gitlab_effective_role(project: dict[str, Any]) -> str:
    permissions = project.get("permissions") if isinstance(project.get("permissions"), dict) else {}
    levels = []
    for key in ("project_access", "group_access"):
        value = permissions.get(key)
        if isinstance(value, dict) and isinstance(value.get("access_level"), int):
            levels.append(value["access_level"])
    level = max(levels, default=0)
    return GITLAB_ACCESS_NAMES.get(level, f"CUSTOM_{level}")


def inspect_gitlab(target: dict[str, Any], idempotency_key: str | None) -> dict[str, Any]:
    if shutil.which("glab") is None:
        raise PreflightError("glab is not installed")
    host = target["host"]
    repository = target["repository"]
    encoded = urllib.parse.quote(repository, safe="")
    actor = run_json(["glab", "api", "--hostname", host, "user"]).get("username")
    project = run_json(["glab", "api", "--hostname", host, f"projects/{encoded}"])
    labels = run_json(
        ["glab", "api", "--hostname", host, f"projects/{encoded}/labels?per_page=100"]
    )
    duplicates: list[dict[str, Any]] = []
    if idempotency_key:
        query = urllib.parse.urlencode(
            {"scope": "all", "state": "all", "search": idempotency_key, "in": "description", "per_page": 20}
        )
        found = run_json(
            ["glab", "api", "--hostname", host, f"projects/{encoded}/issues?{query}"]
        )
        duplicates = found if isinstance(found, list) else []
    return {
        "forge": "gitlab",
        "host": host,
        "repository": project.get("path_with_namespace"),
        "project_id": project.get("id"),
        "authenticated_actor": actor,
        "effective_role": gitlab_effective_role(project),
        "issues_enabled": (
            project.get("issues_access_level") != "disabled"
            if project.get("issues_access_level") is not None
            else bool(project.get("issues_enabled"))
        ),
        "archived": bool(project.get("archived")),
        "available_labels": label_names(labels),
        "duplicate_candidates": duplicates,
        "url": project.get("web_url"),
    }


def inspect_live(target: dict[str, Any], idempotency_key: str | None) -> dict[str, Any]:
    if target["forge"] == "github":
        return inspect_github(target, idempotency_key)
    return inspect_gitlab(target, idempotency_key)


def role_level(forge: str, role: str) -> int:
    normalized = role.upper().replace(" ", "_").replace("-", "_")
    if normalized.startswith("CUSTOM_"):
        try:
            return int(normalized.removeprefix("CUSTOM_"))
        except ValueError:
            return 0
    roles = GITHUB_ROLES if forge == "github" else GITLAB_ROLES
    return roles.get(normalized, 0)


def evaluate(
    target: dict[str, Any],
    snapshot: dict[str, Any],
    *,
    action: str,
    label_intents: list[str],
    report_kind: str,
    idempotency_key: str | None,
) -> dict[str, Any]:
    problems: list[str] = []
    warnings: list[str] = []
    required_actions = [action]
    if label_intents and "apply_labels" not in required_actions:
        required_actions.append("apply_labels")

    for field in ("forge", "host"):
        if snapshot.get(field) != target.get(field):
            problems.append(f"target drift: snapshot {field} does not match registry")
    expected_repo = str(target.get("repository", ""))
    observed_repo = str(snapshot.get("repository", ""))
    matches = expected_repo.lower() == observed_repo.lower() if target["forge"] == "github" else expected_repo == observed_repo
    if not matches:
        problems.append("target drift: snapshot repository does not match registry")
    if snapshot.get("archived") is not False:
        problems.append("target repository is archived or archive state is unknown")
    if snapshot.get("issues_enabled") is not True:
        problems.append("target issue surface is disabled or unknown")
    if not snapshot.get("authenticated_actor"):
        problems.append("authenticated actor is missing")

    allowed = set(target.get("allowed_actions", []))
    for required in required_actions:
        if required not in allowed:
            problems.append(f"action is not allowlisted: {required}")

    forge = target["forge"]
    current_role = str(snapshot.get("effective_role", ""))
    current_level = role_level(forge, current_role)
    for required in required_actions:
        minimum_name = MINIMUM_ROLE[forge][required]
        if forge == "gitlab" and action == "create" and required == "apply_labels":
            # GitLab Guests may set metadata while creating their own issue,
            # even though later metadata mutation requires Planner.
            minimum_name = "GUEST"
        if current_level < role_level(forge, minimum_name):
            problems.append(
                f"insufficient provider role for {required}: {current_role or 'UNKNOWN'} < {minimum_name}"
            )

    if action == "create" and not idempotency_key:
        problems.append("create requires a stable idempotency key")
    duplicates = snapshot.get("duplicate_candidates", [])
    if action == "create" and isinstance(duplicates, list) and duplicates:
        problems.append("duplicate candidates found for the idempotency key")

    if report_kind == "security" and target.get("security_route") == "none":
        problems.append("security report has no approved private route")
    if report_kind != "security" and target.get("security_route") == "confidential_issue":
        warnings.append("confidential_issue is configured but confidentiality still requires explicit use")

    available_labels = set(snapshot.get("available_labels", []))
    mappings = target.get("label_map", {})
    resolved_labels: list[str] = []
    label_resolution: list[dict[str, str]] = []
    for intent in label_intents:
        mapping = mappings.get(intent)
        if not isinstance(mapping, dict):
            problems.append(f"unknown normalized label intent: {intent}")
            continue
        provider_label = mapping.get("provider_label")
        if provider_label not in available_labels:
            problems.append(f"mapped provider label does not exist: {provider_label}")
            continue
        resolved_labels.append(provider_label)
        label_resolution.append({"intent": intent, "provider_label": provider_label})

    if target.get("relationship") == "permissioned" and action in {
        "create_labels",
        "assign",
        "milestone",
        "planning",
        "transfer",
    }:
        warnings.append("permissioned-target metadata action requires explicit least-invasive review")

    return {
        "ok": not problems,
        "target_id": target["id"],
        "forge": forge,
        "host": target["host"],
        "repository": target["repository"],
        "relationship": target["relationship"],
        "security_route": target["security_route"],
        "action": action,
        "required_actions": required_actions,
        "authenticated_actor": snapshot.get("authenticated_actor"),
        "effective_role": current_role or None,
        "resolved_labels": resolved_labels,
        "label_resolution": label_resolution,
        "idempotency_key": idempotency_key,
        "duplicate_candidates": duplicates if isinstance(duplicates, list) else [],
        "problems": problems,
        "warnings": warnings,
        "live_provider_read": snapshot.get("source") == "live",
        "operator_authority_verified": False,
        "mutation_authorized": False,
        "assessment_scope": "target registry, provider capability, duplicate, security-route, and label preflight only",
    }


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Read-only GitHub/GitLab issue-action preflight")
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--target", required=True, help="target id from the registry")
    parser.add_argument("--action", choices=sorted(ACTIONS), required=True)
    parser.add_argument("--label", action="append", default=[], dest="labels", help="normalized label intent")
    parser.add_argument(
        "--report-kind",
        choices=["defect", "feature", "operations", "governance", "security"],
        default="defect",
    )
    parser.add_argument("--idempotency-key")
    parser.add_argument("--snapshot", type=Path, help="provider-free JSON snapshot instead of live discovery")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        registry = read_json(args.registry)
        registry_problems = validate_registry(registry)
        if registry_problems:
            result = {"ok": False, "problems": registry_problems, "warnings": []}
        else:
            target = select_target(registry, args.target)
            snapshot = read_json(args.snapshot) if args.snapshot else inspect_live(target, args.idempotency_key)
            snapshot["source"] = "fixture" if args.snapshot else "live"
            result = evaluate(
                target,
                snapshot,
                action=args.action,
                label_intents=args.labels,
                report_kind=args.report_kind,
                idempotency_key=args.idempotency_key,
            )
    except PreflightError as exc:
        result = {"ok": False, "problems": [str(exc)], "warnings": []}
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result.get("ok") else 2


if __name__ == "__main__":
    sys.exit(main())
