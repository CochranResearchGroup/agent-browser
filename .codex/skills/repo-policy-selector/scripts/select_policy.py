#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import re
from pathlib import Path
from typing import Any


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def read_json(path: Path) -> dict[str, Any]:
    text = read_text(path)
    if not text:
        return {}
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return data if isinstance(data, dict) else {}


def parse_catalog(catalog_path: Path) -> dict[str, list[dict[str, Any]]]:
    text = read_text(catalog_path)
    result: dict[str, list[dict[str, Any]]] = {"modules": [], "profiles": []}
    section: str | None = None
    current: dict[str, Any] | None = None
    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.endswith(":") and stripped[:-1] in {"modules", "profiles"}:
            section = stripped[:-1]
            current = None
            continue
        if section is None:
            continue
        if stripped.startswith("- "):
            current = {}
            result[section].append(current)
            payload = stripped[2:]
            if ":" in payload:
                key, value = payload.split(":", 1)
                current[key.strip()] = value.strip()
            continue
        if current is None or ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        key = key.strip()
        value = value.strip()
        if value.startswith("[") and value.endswith("]"):
            items = [item.strip() for item in value[1:-1].split(",") if item.strip()]
            current[key] = items
        else:
            current[key] = value
    return result


def parse_profile(profile_path: Path) -> dict[str, Any]:
    text = read_text(profile_path)
    result: dict[str, Any] = {}
    section: str | None = None
    current_list_key: str | None = None
    current_dict_key: str | None = None
    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if not line.startswith(" ") and stripped.endswith(":"):
            section = stripped[:-1]
            current_list_key = None
            current_dict_key = section if section == "overrides" else None
            if section == "overrides":
                result[section] = {}
            continue
        if section in {"modules", "workflow_subtypes"} and stripped.startswith("- "):
            result.setdefault(section, []).append(stripped[2:].strip())
            continue
        if section == "overrides" and ":" in stripped:
            key, value = stripped.split(":", 1)
            result["overrides"][key.strip()] = value.strip()
            continue
        if section is None and ":" in stripped:
            key, value = stripped.split(":", 1)
            result[key.strip()] = value.strip()
    return result


def parse_module(module_path: Path) -> dict[str, Any]:
    text = read_text(module_path)
    result: dict[str, Any] = {"body": text}
    if text.startswith("---\n"):
        _, _, remainder = text.partition("---\n")
        frontmatter, _, body = remainder.partition("\n---\n")
        if body:
            result["body"] = body
        for raw_line in frontmatter.splitlines():
            line = raw_line.rstrip()
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if stripped.startswith("- "):
                continue
            if ":" not in stripped:
                continue
            key, value = stripped.split(":", 1)
            key = key.strip()
            value = value.strip()
            if value.startswith("[") and value.endswith("]"):
                result[key] = [item.strip() for item in value[1:-1].split(",") if item.strip()]
            else:
                result[key] = value
    return result


def enumerate_policy_library(policy_root: Path | None) -> dict[str, Any]:
    base_dir = policy_root or Path(__file__).resolve().parents[1] / "policy-library"
    catalog_path = base_dir / "catalog.yaml"
    manifest_path = base_dir.parent / "release-manifest.json"
    catalog = parse_catalog(catalog_path)
    module_ids = [item["id"] for item in catalog["modules"] if "id" in item]
    profile_ids = [item["id"] for item in catalog["profiles"] if "id" in item]
    parsed_modules: dict[str, dict[str, Any]] = {}
    for item in catalog["modules"]:
        module_id = item.get("id")
        module_path = item.get("path")
        if not module_id or not module_path:
            continue
        parsed_modules[module_id] = parse_module(base_dir / module_path)
    parsed_profiles: dict[str, dict[str, Any]] = {}
    for item in catalog["profiles"]:
        profile_id = item.get("id")
        profile_path = item.get("path")
        if not profile_id or not profile_path:
            continue
        parsed_profiles[profile_id] = parse_profile(base_dir / profile_path)
    return {
        "policy_root": str(base_dir),
        "catalog_path": str(catalog_path),
        "release_manifest_path": str(manifest_path),
        "release_manifest": read_json(manifest_path),
        "catalog_found": catalog_path.exists(),
        "modules": catalog["modules"],
        "profiles": catalog["profiles"],
        "parsed_modules": parsed_modules,
        "parsed_profiles": parsed_profiles,
        "module_ids": module_ids,
        "profile_ids": profile_ids,
    }


def read_policy_text(repo_root: Path) -> str:
    parts: list[str] = []
    agents_text = read_text(repo_root / "AGENTS.md") or read_text(repo_root / "AGENT.MD")
    if agents_text:
        parts.append(agents_text)

    policy_dir = repo_root / "docs" / "dev" / "policies"
    if policy_dir.exists():
        for path in sorted(policy_dir.rglob("*.md")):
            if path.is_file():
                parts.append(read_text(path))
    return "\n".join(part for part in parts if part)


def adopted_policy_id(path: Path) -> str:
    stem = path.stem
    match = re.match(r"^\d{4}(?:-\d{4}-\d{2}-\d{2})?-(.+)$", stem)
    if match:
        return match.group(1)
    return stem


def next_policy_serial(repo_root: Path) -> int:
    policy_dir = repo_root / "docs" / "dev" / "policies"
    max_serial = 0
    if policy_dir.exists():
        for path in policy_dir.glob("*.md"):
            match = re.match(r"^(\d{4})-", path.name)
            if match:
                max_serial = max(max_serial, int(match.group(1)))
    return max_serial + 1


def tokenize(text: str) -> set[str]:
    def normalize(token: str) -> str:
        for prefix, replacement in [
            ("install", "install"),
            ("wire", "wire"),
            ("enumerat", "enumerat"),
            ("adopt", "adopt"),
            ("reconcil", "reconcil"),
            ("document", "document"),
            ("version", "version"),
            ("release", "release"),
            ("commit", "commit"),
            ("branch", "branch"),
            ("plan", "plan"),
            ("goal", "goal"),
            ("parallel", "parallel"),
            ("memory", "memory"),
            ("note", "note"),
            ("harvest", "harvest"),
            ("validat", "validat"),
            ("architect", "architect"),
            ("delegat", "delegat"),
            ("agent", "agent"),
            ("preview", "preview"),
            ("artifact", "artifact"),
            ("approv", "approv"),
            ("codegraph", "codegraph"),
            ("refactor", "refactor"),
        ]:
            if token.startswith(prefix):
                return replacement
        return token

    return {
        normalize(token)
        for token in re.findall(r"[a-z0-9]+", text.lower())
        if len(token) >= 4 and token not in {
            "this",
            "that",
            "with",
            "when",
            "from",
            "into",
            "under",
            "use",
            "used",
            "using",
            "policy",
            "policies",
            "repo",
            "repos",
            "docs",
            "dev",
            "shared",
            "local",
            "module",
            "modules",
            "profile",
            "profiles",
            "should",
            "keep",
            "work",
            "workflow",
        }
    }


def module_signature(module_id: str, module: dict[str, Any]) -> set[str]:
    parts = [module_id.replace("-", " ")]
    for key in ("title", "summary"):
        value = module.get(key)
        if isinstance(value, str):
            parts.append(value)
    tags = module.get("tags", [])
    if isinstance(tags, list):
        parts.extend(tag.replace("-", " ") for tag in tags if isinstance(tag, str))
    return tokenize("\n".join(parts))


def module_anchor_tokens(module_id: str, module: dict[str, Any]) -> set[str]:
    parts = [module_id.replace("-", " ")]
    title = module.get("title", "")
    if isinstance(title, str):
        parts.append(title)
    tags = module.get("tags", [])
    if isinstance(tags, list):
        parts.extend(tag.replace("-", " ") for tag in tags if isinstance(tag, str))
    return tokenize("\n".join(parts))


def semantic_policy_text(text: str) -> str:
    body = text
    if body.startswith("---\n"):
        _, _, remainder = body.partition("---\n")
        _, _, after_frontmatter = remainder.partition("\n---\n")
        if after_frontmatter:
            body = after_frontmatter
    policy_only = re.split(r"(?im)^##\s+adoption notes\s*$", body, maxsplit=1)[0]
    return policy_only.strip() or body


def semantic_module_matches(
    existing_policy_surfaces: list[dict],
    installed_library: dict[str, Any],
) -> dict[str, list[str]]:
    canonical_policy_paths = [
        Path(item["path"])
        for item in existing_policy_surfaces
        if item["source_type"] == "canonical-policy"
    ]
    local_texts = {str(path): read_text(path) for path in canonical_policy_paths}
    local_policy_texts = {
        path: semantic_policy_text(text) for path, text in local_texts.items()
    }
    local_tokens = {path: tokenize(text) for path, text in local_policy_texts.items()}
    explicit_match_paths: dict[str, set[str]] = {path: set() for path in local_texts}
    for module_id, module in installed_library.get("parsed_modules", {}).items():
        title = module.get("title", "")
        for path_str, text in local_policy_texts.items():
            lower_text = text.lower()
            if module_id == "notes-and-memories":
                if (
                    (module_id in lower_text or (isinstance(title, str) and title and title.lower() in lower_text))
                    and "docs/dev/notes" in lower_text
                    and "docs/dev/memories" in lower_text
                    and any(
                        phrase in lower_text
                        for phrase in [
                            "prefer notes",
                            "prefer memories",
                            "dated notes",
                            "durable memories",
                            "dated observations",
                            "persist across many slices",
                        ]
                    )
                ):
                    explicit_match_paths[path_str].add(module_id)
                continue
            if module_id in lower_text or (isinstance(title, str) and title and title.lower() in lower_text):
                explicit_match_paths[path_str].add(module_id)
    matches: dict[str, list[str]] = {}
    for module_id, module in installed_library.get("parsed_modules", {}).items():
        signature = module_signature(module_id, module)
        anchors = module_anchor_tokens(module_id, module)
        if not signature:
            continue
        scored_matches: list[tuple[int, str]] = []
        for path_str, text in local_policy_texts.items():
            lower_text = text.lower()
            title = module.get("title", "")
            if (
                module_id == "notes-and-memories"
                and (
                    "notes and memories" in lower_text
                    or "notes-and-memories" in lower_text
                    or ("docs/dev/notes" in lower_text and "docs/dev/memories" in lower_text)
                )
                and any(
                    phrase in lower_text
                    for phrase in [
                        "prefer notes",
                        "prefer memories",
                        "dated notes",
                        "durable memories",
                        "dated observations",
                        "persist across many slices",
                    ]
                )
            ):
                scored_matches.append((100, path_str))
                continue
            if module_id in lower_text or (isinstance(title, str) and title and title.lower() in lower_text):
                scored_matches.append((100, path_str))
                continue
            if explicit_match_paths[path_str]:
                continue
            if (
                module_id == "policy-management"
                and "agents.md" in lower_text
                and "docs/dev/policies" in lower_text
                and "install" in lower_text
                and ("wiring" in lower_text or "wire-in" in lower_text or "enumeration" in lower_text)
            ):
                scored_matches.append((80, path_str))
                continue
            if (
                module_id == "notes-and-memories"
                and (
                    "notes and memories" in lower_text
                    or "notes-and-memories" in lower_text
                    or ("docs/dev/notes" in lower_text and "docs/dev/memories" in lower_text)
                )
                and any(
                    phrase in lower_text
                    for phrase in [
                        "prefer notes",
                        "prefer memories",
                        "dated observations",
                        "dated notes",
                        "durable memories",
                        "stable context",
                        "continuity",
                    ]
                )
            ):
                scored_matches.append((85, path_str))
                continue
            if module_id == "notes-and-memories":
                continue
            if (
                module_id == "lms-cli-governance"
                and (
                    "canvas cli" in lower_text
                    or "canvas-cli" in lower_text
                    or "lms cli" in lower_text
                    or "lms-backed" in lower_text
                )
                and any(token in lower_text for token in ["course", "assignment", "assignments", "quiz", "quizzes", "modules", "grades"])
                and any(token in lower_text for token in ["live", "read-only", "dry-run", "--apply", "validate", "export"])
            ):
                scored_matches.append((85, path_str))
                continue
            if (
                module_id == "graph-backed-memory-usage"
                and (
                    "graphiti" in lower_text
                    or "graph-backed memory" in lower_text
                    or "graph backed memory" in lower_text
                    or "durable memory system" in lower_text
                )
                and any(
                    token in lower_text
                    for token in [
                        "durable retrievable context",
                        "graphiti-discovery",
                        "memory-discovery",
                        "memory discovery",
                        "memory atlas",
                        "atlas/routing",
                        "repo group",
                        "scratchpad",
                        "search_memory_facts",
                        "search_nodes",
                        "add_memory",
                        "memory spam",
                        "duplicate",
                        "destructive",
                        "group_id",
                    ]
                )
            ):
                scored_matches.append((85, path_str))
                continue
            if (
                module_id == "memory-service-runtime-governance"
                and (
                    "graphiti" in lower_text
                    or "memory service" in lower_text
                    or "memory-service" in lower_text
                    or "mcp memory server" in lower_text
                    or "durable memory service" in lower_text
                )
                and any(
                    token in lower_text
                    for token in [
                        "health",
                        "readiness",
                        "queue",
                        "dead-letter",
                        "dead letter",
                        "provider",
                        "embedding",
                        "installed release",
                        "manifest",
                        "runtime home",
                        "service manager",
                        "smoke",
                        "read-after-write",
                    ]
                )
            ):
                scored_matches.append((85, path_str))
                continue
            if (
                module_id == "preview-artifact-review"
                and (
                    "previews" in lower_text
                    or "preview service" in lower_text
                    or "preview session" in lower_text
                    or "browser review" in lower_text
                    or "review packet" in lower_text
                    or "approval packet" in lower_text
                    or "artifact review" in lower_text
                )
                and any(
                    token in lower_text
                    for token in [
                        "human review",
                        "approval",
                        "approve",
                        "feedback",
                        "session url",
                        "browser",
                        "local artifacts",
                        "generated artifacts",
                        "pdf",
                        "office documents",
                        "html",
                        "screenshots",
                        "galleries",
                    ]
                )
            ):
                scored_matches.append((85, path_str))
                continue
            if (
                module_id == "codegraph-usage"
                and (
                    "codegraph" in lower_text
                    or "../codegraph" in lower_text
                    or "code graph" in lower_text
                    or "indexed code" in lower_text
                    or "symbol graph" in lower_text
                )
                and any(
                    token in lower_text
                    for token in [
                        "before code edits",
                        "before editing code",
                        "architecture",
                        "trace",
                        "callers",
                        "callees",
                        "impact",
                        "refactor",
                        "symbol",
                        "source reads",
                    ]
                )
            ):
                scored_matches.append((85, path_str))
                continue
            overlap = signature & local_tokens[path_str]
            anchor_overlap = anchors & local_tokens[path_str]
            overlap_ratio = len(overlap) / max(len(signature), 1)
            if len(anchor_overlap) >= 2 and len(overlap) >= 3 and overlap_ratio >= 0.20:
                score = len(anchor_overlap) * 10 + len(overlap)
                scored_matches.append((score, path_str))
        if scored_matches:
            max_score = max(score for score, _ in scored_matches)
            matched_paths = sorted({path for score, path in scored_matches if score == max_score})
            matches[module_id] = matched_paths
    return matches


def is_thin_agents_wirein(text: str) -> bool:
    lower = text.lower()
    return (
        "docs/dev/policies/" in lower
        and "## policy entry" in lower
        and len(text.splitlines()) <= 80
        and len(text) <= 4000
    )


def markdown_sections(text: str) -> list[tuple[str, str]]:
    matches = list(re.finditer(r"(?m)^##\s+(.+?)\s*$", text))
    if not matches:
        return []
    sections: list[tuple[str, str]] = []
    for index, match in enumerate(matches):
        heading = match.group(1).strip()
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        body = text[start:end].strip()
        sections.append((heading, body))
    return sections


def infer_repo_local_policy_findings(
    repo_root: Path,
    recommended_modules: list[str],
    profile_id: str,
    signals: dict[str, Any],
) -> list[dict[str, Any]]:
    agents_path = repo_root / "AGENTS.md"
    text = read_text(agents_path)
    if not text or is_thin_agents_wirein(text):
        return []

    findings: list[dict[str, Any]] = []
    shared_section_names = {"policy entry", "scope"}
    keep_heading_names = {
        "repo context",
        "repo-specific guidance",
        "project overview",
        "current working set",
        "cli shape",
        "project context",
        "quickstart",
        "safety and data integrity rules",
    }
    merge_keyword_map = {
        "planning-discipline": ["plan", "planning", "bounded plan", "definition of done"],
        "goal-execution-governance": ["/goal", "goal execution", "long-running goal", "goal checkpoint", "goal-compatible"],
        "roadmap-runbook-governance": ["roadmap", "runbook", "progress", "current working set"],
        "git-worktree-hygiene": ["worktree", "worktrees"],
        "commit-history-discipline": ["commit", "atomic", "history"],
        "branch-and-integration-strategy": ["branch", "rebase", "merge", "integration"],
        "commit-and-push-cadence": ["push cadence", "when should i push", "checkpoint"],
        "versioning-and-release": ["release", "version", "tag", "deploy cut"],
        "validation-and-handoff": ["validation", "handoff", "remaining risk", "verification"],
        "architecture-guardrails": ["architecture", "boundary", "execution engine", "provider api logic"],
        "runtime-vs-product-boundary": ["runtime home", "outside the repo", "user-scoped runtime", "product repo"],
        "runtime-state-governance": ["version controlled", "runtime state", "redaction", "pruning"],
        "tenant-isolation-and-operator-state": ["tenant", "tenant-scoped", "one profile per tenant", "state isolation"],
        "fieldwork-productization": ["fieldwork", "keep as product", "refactor before keep", "archive as note only"],
        "monolith-extraction-discipline": ["monolith", "monolithic", "strong trunk", "oversized cli"],
        "policy-management": ["docs/dev/policies", "policy library", "agents.md"],
    }

    for heading, body in markdown_sections(text):
        lower_heading = heading.lower()
        lower_body = body.lower()
        if lower_heading in shared_section_names:
            continue

        matched_modules = [
            module_id
            for module_id, keywords in merge_keyword_map.items()
            if module_id in recommended_modules and any(keyword in lower_heading or keyword in lower_body for keyword in keywords)
        ]

        action = "keep"
        rationale = "section appears repo-specific and should remain in AGENTS.md"

        if lower_heading in keep_heading_names:
            action = "keep"
            rationale = "section describes repo-specific context or operating guidance that should remain local"
        elif "docs/dev/policies" in lower_body and ("full policy body" in lower_body or "entire policy body" in lower_body):
            action = "review-conflict"
            rationale = "section conflicts with the adopted model where AGENTS.md is the entrypoint and the durable policy body lives under docs/dev/policies"
        elif signals.get("has_roadmap") and "do not use roadmap" in lower_body:
            action = "review-conflict"
            rationale = "section appears to conflict with the repo's canonical roadmap usage"
        elif signals.get("has_runbook") and "do not use runbook" in lower_body:
            action = "review-conflict"
            rationale = "section appears to conflict with the repo's canonical runbook usage"
        elif matched_modules:
            action = "merge"
            rationale = "section overlaps shared policy concepts and should be reviewed for merge into adopted local policy files"

        findings.append(
            {
                "path": str(agents_path),
                "section_heading": heading,
                "action": action,
                "rationale": rationale,
                "matched_modules": matched_modules,
            }
        )
    return findings


def md_file_count(path: Path) -> int:
    if not path.exists():
        return 0
    return sum(1 for child in path.rglob("*.md") if child.is_file())


def existing_paths(paths: list[Path]) -> list[str]:
    return [str(path) for path in paths if path.exists()]


def canonical_planning_authorities(repo_root: Path) -> dict[str, list[str]]:
    docs_dev = repo_root / "docs" / "dev"
    candidates = {
        "roadmap": [
            repo_root / "ROADMAP.md",
            repo_root / "roadmap.md",
            docs_dev / "ROADMAP.md",
            docs_dev / "roadmap.md",
        ],
        "runbook": [
            repo_root / "RUNBOOK.md",
            repo_root / "runbook.md",
            docs_dev / "RUNBOOK.md",
            docs_dev / "runbook.md",
        ],
    }
    return {
        surface_type: existing_paths(paths)
        for surface_type, paths in candidates.items()
    }


def inspect_clutter(repo_root: Path) -> dict:
    docs_dev = repo_root / "docs" / "dev"
    plans_dir = docs_dev / "plans"
    notes_dir = docs_dev / "notes"
    memories_dir = docs_dev / "memories"
    planning_authorities = canonical_planning_authorities(repo_root)

    legacy_planning_candidates = [
        repo_root / "actionable-plan.md",
        repo_root / "plan.md",
        repo_root / "plans.md",
        repo_root / "dev-journal.md",
        repo_root / "execution-plan.md",
        docs_dev / "roadmap.md",
        docs_dev / "runbook.md",
        docs_dev / "progress.md",
        docs_dev / "dev-journal.md",
        docs_dev / "execution-plan.md",
    ]
    legacy_note_candidates = [
        repo_root / "notes.md",
        repo_root / "note.md",
        repo_root / "memory.md",
        repo_root / "memories.md",
        repo_root / "journal.md",
        repo_root / "dev-notes.md",
        repo_root / "dev-journal.md",
        docs_dev / "notes.md",
        docs_dev / "memory.md",
        docs_dev / "memories.md",
        docs_dev / "journal.md",
        docs_dev / "debrief.md",
    ]

    docs_dev_loose_md: list[str] = []
    if docs_dev.exists():
        for child in sorted(docs_dev.glob("*.md")):
            if child.is_file():
                docs_dev_loose_md.append(str(child))

    legacy_planning_paths = existing_paths(legacy_planning_candidates)
    legacy_note_paths = existing_paths(legacy_note_candidates)
    duplicate_planning_authorities = {
        surface_type: paths
        for surface_type, paths in planning_authorities.items()
        if len(paths) > 1
    }

    planning_migration_needed = bool(legacy_planning_paths or duplicate_planning_authorities)
    notes_migration_needed = bool(legacy_note_paths)

    return {
        "canonical_roadmap_paths": planning_authorities["roadmap"],
        "canonical_runbook_paths": planning_authorities["runbook"],
        "duplicate_planning_authorities": duplicate_planning_authorities,
        "has_plans_dir": plans_dir.exists(),
        "has_notes_dir": notes_dir.exists(),
        "has_memories_dir": memories_dir.exists(),
        "plans_file_count": md_file_count(plans_dir),
        "notes_file_count": md_file_count(notes_dir),
        "memories_file_count": md_file_count(memories_dir),
        "legacy_planning_paths": legacy_planning_paths,
        "legacy_note_paths": legacy_note_paths,
        "docs_dev_loose_md": docs_dev_loose_md,
        "planning_migration_needed": planning_migration_needed,
        "notes_migration_needed": notes_migration_needed,
    }


def extract_existing_policy_surfaces(repo_root: Path) -> list[dict]:
    items: list[dict] = []
    seen: set[str] = set()

    def add(path: Path, source_type: str, canonical: bool) -> None:
        resolved = str(path)
        if resolved in seen or not path.exists() or not path.is_file():
            return
        seen.add(resolved)
        text = read_text(path)
        action = "merge"
        rationale = "existing policy should be reconciled with installed templates during adoption"
        if source_type == "agents-entrypoint":
            if is_thin_agents_wirein(text):
                action = "keep"
                rationale = "thin AGENTS.md wire-in already matches the desired entrypoint role"
            else:
                action = "merge"
                rationale = "inline AGENTS policy should be thinned and merged into canonical repo-local policy files"
        elif source_type == "canonical-policy":
            action = "keep"
            rationale = "repo-local policy already lives in the canonical adopted policy directory"
        elif source_type == "legacy-policy":
            action = "merge"
            rationale = "legacy policy should be merged into canonical policy files or thinned into the AGENTS wire-in"
        elif source_type == "duplicate-policy":
            action = "retire"
            rationale = "duplicate policy surface should be retired after canonical policy and wire-in are established"
        items.append(
            {
                "path": resolved,
                "source_type": source_type,
                "canonical": canonical,
                "action": action,
                "rationale": rationale,
            }
        )

    agents = repo_root / "AGENTS.md"
    agent_upper = repo_root / "AGENT.MD"
    if agents.exists():
        add(agents, "agents-entrypoint", True)
    if agent_upper.exists():
        add(agent_upper, "duplicate-policy" if agents.exists() else "agents-entrypoint", not agents.exists())

    policy_dir = repo_root / "docs" / "dev" / "policies"
    if policy_dir.exists():
        for path in sorted(policy_dir.rglob("*.md")):
            add(path, "canonical-policy", True)

    legacy_candidates = [
        repo_root / "POLICY.md",
        repo_root / "policies.md",
        repo_root / "policy.md",
        repo_root / "agent-policy.md",
        repo_root / "agents-policy.md",
        repo_root / "dev-policy.md",
        repo_root / "docs" / "dev" / "policy.md",
        repo_root / "docs" / "dev" / "policies.md",
        repo_root / "docs" / "dev" / "agent-policy.md",
    ]
    for path in legacy_candidates:
        add(path, "legacy-policy", False)

    return items


def extract_existing_migration_surfaces(repo_root: Path) -> list[dict]:
    items: list[dict] = []
    seen: set[str] = set()
    docs_dev = repo_root / "docs" / "dev"
    canonical_roots = {
        "plan": docs_dev / "plans",
        "note": docs_dev / "notes",
        "memory": docs_dev / "memories",
    }

    def add(path: Path, surface_type: str, canonical: bool) -> None:
        resolved = str(path)
        if resolved in seen or not path.exists() or not path.is_file():
            return
        seen.add(resolved)
        action = "keep" if canonical else "merge"
        rationale = (
            f"{surface_type} already lives in the canonical location"
            if canonical
            else f"{surface_type} should be migrated into the canonical docs/dev location"
        )
        items.append(
            {
                "path": resolved,
                "surface_type": surface_type,
                "canonical": canonical,
                "action": action,
                "rationale": rationale,
            }
        )

    for surface_type, root in canonical_roots.items():
        if root.exists():
            for path in sorted(root.rglob("*.md")):
                add(path, surface_type, True)

    legacy_surface_candidates = {
        "plan": [
            repo_root / "actionable-plan.md",
            repo_root / "plan.md",
            repo_root / "plans.md",
            repo_root / "dev-journal.md",
            repo_root / "execution-plan.md",
            docs_dev / "roadmap.md",
            docs_dev / "runbook.md",
            docs_dev / "progress.md",
            docs_dev / "dev-journal.md",
            docs_dev / "execution-plan.md",
        ],
        "note": [
            repo_root / "notes.md",
            repo_root / "note.md",
            repo_root / "journal.md",
            repo_root / "dev-notes.md",
            docs_dev / "notes.md",
            docs_dev / "journal.md",
            docs_dev / "debrief.md",
        ],
        "memory": [
            repo_root / "memory.md",
            repo_root / "memories.md",
            docs_dev / "memory.md",
            docs_dev / "memories.md",
        ],
    }
    for surface_type, paths in legacy_surface_candidates.items():
        for path in paths:
            add(path, surface_type, False)

    return items


def detect_signals(repo_root: Path) -> dict:
    agents_entry_text = read_text(repo_root / "AGENTS.md") or read_text(repo_root / "AGENT.MD")
    policy_text = read_policy_text(repo_root)
    readme = read_text(repo_root / "README.md")
    planning_authorities = canonical_planning_authorities(repo_root)
    runbook_text = ""
    for candidate in planning_authorities["runbook"]:
        runbook_text = read_text(Path(candidate))
        if runbook_text:
            break
    actionable_plan = read_text(repo_root / "actionable-plan.md")
    roadmap = bool(planning_authorities["roadmap"])
    runbook = bool(planning_authorities["runbook"])
    progress = (repo_root / "PROGRESS.md").exists()
    docs_dev = (repo_root / "docs" / "dev").exists()
    policy_dir = (repo_root / "docs" / "dev" / "policies").exists()
    memory = (repo_root / "memory").exists() or (repo_root / "MEMORY.md").exists()
    proposalish = any(
        token in str(repo_root).lower()
        for token in ["proposal", "grant", "journal article", "manuscript", "patent application"]
    )
    clutter = inspect_clutter(repo_root)
    has_modules_dir = (repo_root / "modules").exists()
    has_profiles_dir = (repo_root / "profiles").exists()
    has_catalog = (repo_root / "catalog.yaml").exists()
    has_selector_bundle = (repo_root / "repo-policy-selector").exists()
    has_local_harvester = (repo_root / ".codex" / "skills" / "repo-policy-harvester").exists()
    git_dir = repo_root / ".git"
    git_config = read_text(git_dir / "config").lower() if git_dir.is_dir() else read_text(git_dir).lower()
    top_level_files = [child for child in repo_root.iterdir() if child.is_file()]
    document_suffixes = {".pdf", ".doc", ".docx", ".xls", ".xlsx", ".csv", ".tsv"}
    code_manifest_names = {
        "package.json",
        "pyproject.toml",
        "cargo.toml",
        "go.mod",
        "requirements.txt",
        "makefile",
        "dockerfile",
    }
    top_level_document_count = sum(1 for child in top_level_files if child.suffix.lower() in document_suffixes)
    top_level_code_manifest_count = sum(
        1 for child in top_level_files if child.name.lower() in code_manifest_names
    )
    course_config_paths = [
        repo_root / "canvas-cli.yml",
        repo_root / "canvas-cli.yaml",
        repo_root / "canvas.yml",
        repo_root / "canvas.yaml",
        repo_root / "course.yml",
        repo_root / "course.yaml",
    ]
    course_config_text = "\n".join(read_text(path) for path in course_config_paths if path.exists()).lower()
    top_level_names = " ".join(child.name.lower() for child in repo_root.iterdir())
    google_native_placeholder_count = 0
    try:
        for path in repo_root.rglob("*"):
            if path.is_file() and path.suffix.lower() in {".gsheet", ".gform", ".gdoc", ".gslides"}:
                google_native_placeholder_count += 1
                if google_native_placeholder_count >= 3:
                    break
    except OSError:
        google_native_placeholder_count = 0

    text = policy_text.lower()
    entrypoint_text = agents_entry_text.lower()
    semantic_text = "\n".join([entrypoint_text, readme.lower(), course_config_text, top_level_names])
    combined = "\n".join([text, readme.lower(), runbook_text.lower(), actionable_plan.lower(), course_config_text, top_level_names])
    repo_path = str(repo_root).lower()
    has_explicit_goal_command = "/goal" in combined
    has_agentic_goal_context = any(
        phrase in combined
        for phrase in [
            "agent loop",
            "agent execution",
            "agent orchestration",
            "autonomous agent",
            "autonomous",
            "unattended",
            "multi-session",
            "multiple sessions",
            "context window",
            "subagent",
            "sub-agent",
            "checkpoint",
        ]
    )
    has_long_goal_marker = any(
        phrase in combined
        for phrase in [
            "goal mode",
            "goal execution",
            "goal-compatible",
            "goal compatible",
            "long-running goal",
            "long running goal",
            "goal checkpoint",
        ]
    )
    signals = {
        "has_agents": bool(agents_entry_text),
        "has_roadmap": roadmap,
        "has_runbook": runbook,
        "canonical_roadmap_paths": clutter["canonical_roadmap_paths"],
        "canonical_runbook_paths": clutter["canonical_runbook_paths"],
        "duplicate_planning_authorities": clutter["duplicate_planning_authorities"],
        "has_progress": progress,
        "has_docs_dev": docs_dev,
        "has_policy_dir": policy_dir,
        "has_modules_dir": has_modules_dir,
        "has_profiles_dir": has_profiles_dir,
        "has_catalog": has_catalog,
        "has_selector_bundle": has_selector_bundle,
        "has_local_harvester": has_local_harvester,
        "has_plans_dir": clutter["has_plans_dir"],
        "has_notes_dir": clutter["has_notes_dir"],
        "has_memories_dir": clutter["has_memories_dir"],
        "plans_file_count": clutter["plans_file_count"],
        "notes_file_count": clutter["notes_file_count"],
        "memories_file_count": clutter["memories_file_count"],
        "legacy_planning_paths": clutter["legacy_planning_paths"],
        "legacy_note_paths": clutter["legacy_note_paths"],
        "docs_dev_loose_md": clutter["docs_dev_loose_md"],
        "planning_migration_needed": clutter["planning_migration_needed"],
        "notes_migration_needed": clutter["notes_migration_needed"],
        "has_memory_files": memory,
        "mentions_parallel": "parallel" in combined or "subagent" in combined,
        "mentions_subagents": any(
            phrase in combined
            for phrase in [
                "subagent",
                "sub-agent",
                "multi-agent",
                "delegation",
                "delegate",
                "parallel lane",
                "parallel track",
            ]
        ),
        "mentions_goal_execution": has_explicit_goal_command or (has_long_goal_marker and has_agentic_goal_context),
        "mentions_subagent_runtime": any(
            phrase in combined or phrase in semantic_text
            for phrase in [
                "subagent runtime",
                "sub-agent runtime",
                "subagent run id",
                "subagent session id",
                "subagent session key",
                "subagent transcript",
                "transcript path",
                "announce payload",
                "announce message",
                "non-blocking spawn",
                "spawn depth",
                "maxspawndepth",
                "max spawn depth",
                "nested subagent",
                "nested sub-agent",
                "maxchildrenperagent",
                "max children per agent",
                "maxconcurrent",
                "concurrency cap",
                "cascade stop",
                "tool policy",
                "subagent allowlist",
                "subagent denylist",
                "run timeout",
                "runtime stats",
                "subagent token usage",
                "cost metadata",
                "archive after",
                "subagent cleanup",
            ]
        ),
        "mentions_dev_speed_bias": any(
            phrase in combined
            for phrase in [
                "max-dev-speed",
                "developer speed",
                "dev speed",
                "wall-clock speed",
                "move fast",
                "favor speed",
                "parallel first",
            ]
        ),
        "mentions_token_efficiency_bias": any(
            phrase in combined
            for phrase in [
                "max-token-efficiency",
                "token efficiency",
                "minimize tokens",
                "reduce token cost",
                "coordination cost",
                "avoid duplicated context",
                "context efficiency",
            ]
        ),
        "mentions_upstream_fork": any(
            phrase in combined or phrase in git_config
            for phrase in [
                "[remote \"upstream\"]",
                "active upstream",
                "non-owned upstream",
                "private fork",
                "upstream sync",
                "upstream rebase",
                "force-push",
                "force push",
                "fork maintenance",
                "downstream fork",
            ]
        ),
        "mentions_git_policy": "git policy" in text or "worktree" in text or "branch" in text,
        "mentions_closeout": "closeout" in text or "best recommendation" in text,
        "mentions_policy_harvest": "policy" in semantic_text and "harvest" in semantic_text,
        "mentions_policy_library": any(
            phrase in semantic_text
            for phrase in [
                "policy library",
                "shared policy templates",
                "shared policy artifacts",
                "source library",
                "starter bundles",
                "reusable policy modules",
                "reusable policy templates",
                "policy selection",
                "policy adoption",
                "install the selector",
                "installed policy library",
                "repo-policy-selector",
                "repo-policy-harvester",
                "catalog.yaml",
            ]
        ),
        "mentions_memory": "memory.md" in combined or "heartbeats" in combined or "group chats" in combined,
        "mentions_graph_backed_memory": any(
            phrase in combined or phrase in semantic_text
            for phrase in [
                "graph-backed memory",
                "graph backed memory",
                "graphiti",
                "graphiti-discovery",
                "memory-discovery",
                "memory discovery",
                "memory atlas",
                "atlas/routing",
                "repo group",
                "durable memory system",
                "search_memory_facts",
                "search_nodes",
                "add_memory",
                "get_episodes",
                "group_id",
                "memory spam",
                "duplicate writes",
            ]
        ),
        "mentions_codegraph_usage": any(
            phrase in combined or phrase in semantic_text
            for phrase in [
                "../codegraph",
                "codegraph",
                "code graph",
                "indexed code",
                "code intelligence",
                "symbol graph",
                "call graph",
                "caller",
                "callers",
                "callee",
                "callees",
                "impact analysis",
                "trace analysis",
                "refactor planning",
            ]
        ),
        "mentions_memory_service_runtime": any(
            phrase in combined or phrase in semantic_text
            for phrase in [
                "memory service runtime",
                "memory-service runtime",
                "mcp memory server",
                "graphiti mcp server",
                "memory queue",
                "memory job",
                "dead-letter memory",
                "dead letter memory",
                "get_memory_queue_status",
                "installed memory service",
                "memory service health",
                "read-after-write smoke",
                "graphiti-openclaw smoke",
                "provider boundary",
                "embedding provider",
                "installed release",
            ]
        ),
        "mentions_architecture_guardrails": any(
            phrase in semantic_text
            for phrase in [
                "current architecture",
                "service-first",
                "local service contract",
                "shared seams",
                "ownership boundaries",
                "contract surfaces",
                "canonical data stores",
                "system-architecture",
                "structural drift",
                "top-level endpoint",
                "operator workflow",
                "deployment boundary",
                "tightening semantics",
            ]
        ),
        "mentions_doc_hygiene": any(
            phrase in semantic_text
            for phrase in [
                "doc hygiene",
                "update the roadmap",
                "update the runbook",
                "user-facing docs",
                "same slice",
                "dev-journal",
                "execution-plan",
                "local-api.md",
            ]
        ),
        "mentions_validation_handoff": any(
            phrase in combined
            for phrase in [
                "verification",
                "validation",
                "tests",
                "handoff",
                "manual-tests",
                "green validation",
                "relevant checks",
                "remaining risk",
            ]
        ),
        "mentions_writing_deliverable": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "proposal",
                "review outputs",
                "review runbook",
                "review context",
                "journal article",
                "patent application",
                "manuscript",
                "technical writing",
                "project summary",
                "project description",
            ]
        )
        or proposalish,
        "mentions_analysis_workspace": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "prior-art",
                "prior art",
                "patentability",
                "ip analysis",
                "ip scan",
                "fto",
                "claim strategy",
                "closest-art",
                "closest art",
                "analysis phase",
                "review_in_progress",
                "competitor_analysis",
            ]
        ),
        "mentions_review_workspace": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "review runbook",
                "review outputs",
                "review context",
                "rubric",
                "debrief",
                "review_in_progress",
                "reviews/",
                "/reviews/",
            ]
        ),
        "mentions_website_surface": any(
            phrase in semantic_text
            for phrase in [
                "website",
                "wordpress",
                "cms",
                "public site",
                "canonical site",
                "local mirror",
                "staging",
                "preview host",
                "deprecated surface",
                "legacy surface",
            ]
        ),
        "mentions_db_backed_state": any(
            phrase in semantic_text
            for phrase in [
                "db-backed",
                "database-backed",
                "mysql",
                "cms state",
                "page builder",
                "menus",
                "widget settings",
                "customizer",
                "plugin settings",
                "admin state",
            ]
        ),
        "mentions_live_drift": any(
            phrase in semantic_text
            for phrase in [
                "live drift",
                "drift",
                "outside the repo workflow",
                "admin-side changes",
                "repo-owned code drift",
                "before deploy",
            ]
        ),
        "mentions_backup_recovery": any(
            phrase in semantic_text
            for phrase in [
                "backup",
                "borg",
                "recovery",
                "restore",
                "archive",
                "snapshot",
                "recoverable state",
            ]
        ),
        "mentions_visual_release_qa": any(
            phrase in semantic_text
            for phrase in [
                "visual",
                "browser review",
                "screenshot",
                "lighthouse",
                "design-facing",
                "ux-facing",
                "local release qa",
                "post-release live verification",
            ]
        ),
        "mentions_preview_artifact_review": any(
            phrase in semantic_text or phrase in combined
            for phrase in [
                "previews skill",
                "$previews",
                "preview service",
                "preview session",
                "preview url",
                "browser review",
                "browser-based review",
                "review packet",
                "approval packet",
                "artifact review",
                "local artifacts",
                "generated artifacts",
                "publish_session",
                "publish preview",
                "approval feedback",
                "human approval",
                "human review",
                "rendered document",
                "rendered documents",
                "office documents",
                "image gallery",
                "galleries",
            ]
        ),
        "mentions_tenant_runtime": any(
            phrase in semantic_text
            for phrase in [
                "tenant-scoped",
                "tenant scoped",
                "one profile as one tenant",
                "one runtime profile per tenant",
                "per-tenant runtime",
                "per tenant runtime",
                "tenant-specific",
                "tenant specific",
                "tenant isolation",
                "multi-tenant",
                "multi tenant",
                "operator environment",
            ]
        ),
        "mentions_runtime_home": any(
            phrase in semantic_text
            for phrase in [
                "~/.odollo",
                "runtime home",
                "user-scoped runtime",
                "user scoped runtime",
                "tenant home",
                "actions.ndjson",
                "tenant-scoped resources",
                "tenant-scoped memories",
                "tenant-scoped artifacts",
            ]
        ),
        "mentions_fieldwork_productization": any(
            phrase in semantic_text
            for phrase in [
                "fieldwork",
                "field/",
                "live tenant work",
                "reactive live tenant",
                "keep as product",
                "refactor before keep",
                "archive as note only",
                "discard",
                "productization",
            ]
        ),
        "mentions_monolith_debt": any(
            phrase in semantic_text
            for phrase in [
                "monolithic",
                "monolith",
                "strong trunk",
                "oversized cli",
                "huge cli",
                "cli trunk",
            ]
        ),
        "top_level_document_count": top_level_document_count,
        "top_level_code_manifest_count": top_level_code_manifest_count,
        "has_lms_config": any(path.exists() for path in course_config_paths),
        "google_native_placeholder_count": google_native_placeholder_count,
        "mentions_lms_course": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "canvas course",
                "course id",
                "course_id",
                "default course",
                "canvas-cli",
                "canvas cli",
                "moodle",
                "blackboard",
                "google classroom",
                "lms-backed",
                "lms backed",
                "assignment overrides",
                "assignment group",
                "submissions",
                "gradebook",
                "roster",
                "announcements",
                "quizzes",
            ]
        ),
        "mentions_course_workspace": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "course workspace",
                "course folder",
                "course operations",
                "live course",
                "spring 2026",
                "fall 2026",
                "syllabus",
                "lecture notes",
                "seminars",
                "handouts",
                "archive 2025",
                "/isu/",
            ]
        ),
        "mentions_student_assessment_data": any(
            phrase in semantic_text
            for phrase in [
                "ferpa",
                "student-identifiable",
                "student identifiable",
                "student data",
                "student work",
                "graded",
                "grading",
                "submissions",
                "reflections",
                "evaluations",
                "answer keys",
                "answer key",
                "private feedback",
            ]
        ),
        "mentions_cloud_course_drive": (
            google_native_placeholder_count > 0
            or any(
                phrase in semantic_text
                for phrase in [
                    ".gsheet",
                    ".gform",
                    "google drive for desktop",
                    "drive folder id",
                    "gog drive",
                    "google-native",
                    "google native",
                    "shared drive",
                ]
            )
        ),
        "mentions_seminal_domain_workspace": any(
            phrase in semantic_text or phrase in repo_path
            for phrase in [
                "finance",
                "tax",
                "accounting",
                "bookkeeping",
                "compliance",
                "legal",
                "contracts",
                "corporate records",
                "cap table",
                "entity formation",
                "board consent",
                "filing",
            ]
        ),
    }
    return signals


def classify_purpose(signals: dict) -> tuple[str, str | None, list[str]]:
    reasons: list[str] = []
    subtype: str | None = None
    strong_engineering = (
        signals["has_roadmap"]
        or signals["has_runbook"]
        or signals["mentions_architecture_guardrails"]
        or signals["mentions_doc_hygiene"]
    )
    policy_library_semantic_signals = 0
    if signals["mentions_policy_library"]:
        policy_library_semantic_signals += 1
    if signals["mentions_policy_harvest"]:
        policy_library_semantic_signals += 1
    if signals["has_policy_dir"] and signals["has_agents"]:
        policy_library_semantic_signals += 1

    policy_library_supporting_signals = 0
    if signals["has_catalog"]:
        policy_library_supporting_signals += 1
    if signals["has_modules_dir"] and signals["has_profiles_dir"]:
        policy_library_supporting_signals += 1
    if signals["has_selector_bundle"] or signals["has_local_harvester"]:
        policy_library_supporting_signals += 1

    policy_library_repo = (
        policy_library_semantic_signals >= 2
        and policy_library_supporting_signals >= 1
    )
    website_semantic_signals = sum(
        1
        for key in [
            "mentions_website_surface",
            "mentions_db_backed_state",
            "mentions_live_drift",
            "mentions_backup_recovery",
            "mentions_visual_release_qa",
        ]
        if signals[key]
    )
    course_workspace_signals = sum(
        1
        for key in [
            "has_lms_config",
            "mentions_lms_course",
            "mentions_course_workspace",
            "mentions_student_assessment_data",
            "mentions_cloud_course_drive",
        ]
        if signals[key]
    )
    course_operational_signals = sum(
        1
        for key in [
            "has_lms_config",
            "mentions_lms_course",
            "mentions_cloud_course_drive",
        ]
        if signals[key]
    )
    seminal_workspace = (
        signals["top_level_document_count"] >= 2
        and signals["top_level_code_manifest_count"] == 0
        and signals["mentions_seminal_domain_workspace"]
    )
    operations_platform_signals = sum(
        1
        for key in [
            "mentions_tenant_runtime",
            "mentions_runtime_home",
            "mentions_fieldwork_productization",
            "mentions_monolith_debt",
        ]
        if signals[key]
    )
    engineering_support_signals = sum(
        1
        for key in [
            "mentions_validation_handoff",
            "mentions_git_policy",
            "mentions_subagents",
            "mentions_parallel",
        ]
        if signals[key]
    )
    engineering_repo = (
        signals["top_level_code_manifest_count"] >= 1
        and signals["has_agents"]
        and signals["has_policy_dir"]
        and engineering_support_signals >= 2
    )

    if signals["mentions_memory"]:
        purpose = "workspace-agent"
        reasons.append("repo uses memory/heartbeat/group-chat style operating rules")
    elif policy_library_repo:
        purpose = "workspace-agent"
        reasons.append("repo appears to curate a reusable policy library and selector tooling")
    elif course_workspace_signals >= 3 and course_operational_signals >= 1:
        purpose = "course-workspace"
        reasons.append("repo appears to be a live LMS-backed course workspace with course data or student assessment risk")
    elif operations_platform_signals >= 2 and strong_engineering:
        purpose = "operations-platform"
        reasons.append("repo appears to mix reusable product code with tenant-scoped runtime and live operator workflows")
    elif website_semantic_signals >= 3 and signals["mentions_website_surface"]:
        purpose = "website-maintenance"
        reasons.append("repo appears to manage live website surfaces, drift, recovery, or visual release workflows")
    elif seminal_workspace and not strong_engineering:
        purpose = "seminal-workspace"
        reasons.append("repo appears to be a document-heavy formative workspace that does not fit existing policy families cleanly")
    elif (signals["mentions_writing_deliverable"] or signals["mentions_analysis_workspace"]) and not strong_engineering:
        purpose = "writing-project"
        reasons.append("repo appears deliverable-oriented rather than software-architecture-oriented")
    elif (
        strong_engineering
        or engineering_repo
        or (signals["has_runbook"] and signals["mentions_validation_handoff"])
    ):
        purpose = "product-engineering"
        reasons.append("repo shows roadmap/runbook discipline or engineering-oriented validation and coordination signals")
    elif signals["mentions_policy_harvest"]:
        purpose = "workspace-agent"
        reasons.append("repo appears to curate reusable policy or skill behavior")
    else:
        purpose = "library-cli"
        reasons.append("repo lacks heavyweight roadmap/runbook signals")

    return purpose, subtype, reasons


def classify_execution_bias(signals: dict, purpose: str) -> tuple[str | None, list[str]]:
    reasons: list[str] = []
    if signals["mentions_dev_speed_bias"] and not signals["mentions_token_efficiency_bias"]:
        reasons.append("repo language explicitly favors wall-clock speed over coordination cost")
        return "max-dev-speed", reasons
    if signals["mentions_token_efficiency_bias"] and not signals["mentions_dev_speed_bias"]:
        reasons.append("repo language explicitly favors token or coordination efficiency")
        return "max-token-efficiency", reasons
    if signals["mentions_subagents"] or signals["mentions_parallel"]:
        reasons.append("repo language suggests delegated or parallel execution with coordination tradeoffs")
        return "balanced", reasons
    if purpose == "library-cli":
        reasons.append("lighter library repos usually prefer lower coordination and token overhead by default")
        return "max-token-efficiency", reasons
    if purpose in {"product-engineering", "operations-platform", "workspace-agent", "writing-project", "website-maintenance", "course-workspace"}:
        reasons.append("repo shape suggests a balanced tradeoff between wall-clock speed and coordination cost")
        return "balanced", reasons
    return None, reasons


def base_modules_for_profile(profile_id: str, installed_library: dict[str, Any]) -> list[str]:
    profile = installed_library.get("parsed_profiles", {}).get(profile_id, {})
    modules = profile.get("modules", [])
    return list(modules) if isinstance(modules, list) else []


def profile_override_value(profile_id: str, key: str, installed_library: dict[str, Any]) -> str | None:
    profile = installed_library.get("parsed_profiles", {}).get(profile_id, {})
    overrides = profile.get("overrides", {})
    if isinstance(overrides, dict):
        value = overrides.get(key)
        return value if isinstance(value, str) else None
    return None


def profile_expectation_gaps(profile_id: str, signals: dict, installed_library: dict[str, Any]) -> list[str]:
    gaps: list[str] = []
    expects_runbook = profile_override_value(profile_id, "expects_runbook", installed_library)
    expects_roadmap = profile_override_value(profile_id, "expects_roadmap", installed_library)
    if expects_runbook == "true" and not signals["has_runbook"]:
        gaps.append("selected profile expects RUNBOOK.md discipline but the repo does not currently expose a canonical runbook")
    if expects_roadmap == "true" and not signals["has_roadmap"]:
        gaps.append("selected profile expects ROADMAP.md discipline but the repo does not currently expose a canonical roadmap")
    return gaps


def policy_adoption_coverage(
    existing_policy_surfaces: list[dict],
    recommended_modules: list[str],
    installed_library: dict[str, Any],
) -> dict[str, Any]:
    canonical_ids = sorted(
        {
            adopted_policy_id(Path(item["path"]))
            for item in existing_policy_surfaces
            if item["source_type"] == "canonical-policy"
        }
    )
    semantic_matches = semantic_module_matches(existing_policy_surfaces, installed_library)
    semantically_adopted = sorted(
        module_id for module_id in recommended_modules if module_id in semantic_matches
    )
    adopted_set = set(canonical_ids) | set(semantically_adopted)
    already_adopted = [module_id for module_id in recommended_modules if module_id in adopted_set]
    missing = [module_id for module_id in recommended_modules if module_id not in canonical_ids]
    missing = [module_id for module_id in recommended_modules if module_id not in adopted_set]
    if not recommended_modules:
        readiness = "fresh-adoption"
    else:
        ratio = len(already_adopted) / len(recommended_modules)
        if ratio >= 0.75:
            readiness = "mostly-installed"
        elif ratio > 0:
            readiness = "partial-local-policy"
        else:
            readiness = "fresh-adoption"
    return {
        "readiness": readiness,
        "canonical_policy_ids": canonical_ids,
        "semantically_matched_modules": semantically_adopted,
        "semantic_match_paths": {
            module_id: semantic_matches[module_id]
            for module_id in semantically_adopted
        },
        "already_adopted_modules": already_adopted,
        "missing_recommended_modules": missing,
    }


def recommendation_mode(coverage: dict[str, Any]) -> str:
    readiness = coverage["readiness"]
    if readiness in {"partial-local-policy", "mostly-installed"}:
        return "patch-missing"
    return "full-profile"


def module_catalog_entry(module_id: str, installed_library: dict[str, Any]) -> dict[str, Any] | None:
    for item in installed_library.get("modules", []):
        if item.get("id") == module_id:
            return item
    return None


def build_install_plan(
    repo_root: Path,
    next_modules: list[str],
    coverage: dict[str, Any],
    installed_library: dict[str, Any],
) -> list[dict[str, Any]]:
    plan: list[dict[str, Any]] = []
    serial = next_policy_serial(repo_root)
    for module_id in next_modules:
        catalog_entry = module_catalog_entry(module_id, installed_library)
        source_path = None
        if catalog_entry and catalog_entry.get("path"):
            source_path = str(Path(installed_library["policy_root"]) / str(catalog_entry["path"]))
        target_name = f"{serial:04d}-{module_id}.md"
        target_path = repo_root / "docs" / "dev" / "policies" / target_name
        merge_candidates = coverage.get("semantic_match_paths", {}).get(module_id, [])
        plan.append(
            {
                "module_id": module_id,
                "action": "merge-existing" if merge_candidates else "install-new",
                "source_module_path": source_path,
                "target_policy_path": str(target_path),
                "merge_candidates": merge_candidates,
                "draft_content": render_local_policy_draft(module_id, installed_library),
            }
        )
        serial += 1
    return plan


def render_local_policy_draft(module_id: str, installed_library: dict[str, Any]) -> str:
    module = installed_library.get("parsed_modules", {}).get(module_id, {})
    title = module.get("title", module_id.replace("-", " ").title())
    body = module.get("body", "").strip()
    body = re.sub(r"^\s*## Adoption Notes\b", "## Adoption Notes", body, flags=re.MULTILINE)
    lines = [f"# Policy | {title}", ""]
    if body:
        lines.append(body)
    return "\n".join(lines).rstrip() + "\n"


def replace_or_append_section(text: str, heading: str, content: str) -> str:
    pattern = re.compile(rf"(?ims)^## {re.escape(heading)}\n.*?(?=^## |\Z)")
    replacement = content.rstrip() + "\n\n"
    if pattern.search(text):
        updated = pattern.sub(replacement, text, count=1)
    else:
        updated = text.rstrip() + "\n\n" + replacement
    return updated.rstrip() + "\n"


def purpose_scaffold_text(repo_purpose: str | None) -> str:
    if repo_purpose == "operations-platform":
        return (
            "## Repo Context\n\n"
            "- Describe the product boundary, runtime home, and tenant or operator model here.\n"
            "- List the canonical roadmap, runbook, progress, and runtime-state authorities used in this repo.\n\n"
            "## Repo-Specific Guidance\n\n"
            "- Add the exact commands, profiles, runtime paths, and validation surfaces this repo expects.\n"
            "- Document what stays in the product repo versus the user-scoped runtime home.\n"
        )
    if repo_purpose == "website-maintenance":
        return (
            "## Repo Context\n\n"
            "- Describe the live surfaces, environments, and recovery-critical deploy constraints here.\n\n"
            "## Repo-Specific Guidance\n\n"
            "- Add the exact domains, deploy targets, validation surfaces, and backup constraints this repo expects.\n"
        )
    if repo_purpose == "course-workspace":
        return (
            "## Repo Context\n\n"
            "- Describe the course, term, LMS course target, cloud-drive folder, and instructor workflow model here.\n"
            "- Identify which folders are active course material, generated artifacts, submissions, grading work, handouts, and archives.\n\n"
            "## Repo-Specific Guidance\n\n"
            "- Add the exact LMS command path, course config file, secrets boundary, cloud-drive connector, and validation checks this workspace expects.\n"
            "- Document student-data and assessment boundaries that agents must treat as sensitive.\n"
        )
    if repo_purpose == "product-engineering":
        return (
            "## Repo Context\n\n"
            "- Describe the product area, architecture boundaries, and canonical planning surfaces here.\n\n"
            "## Repo-Specific Guidance\n\n"
            "- Add the exact build, test, deploy, and service-boundary rules this repo expects.\n"
        )
    return (
        "## Repo Context\n\n"
        "- Describe this repo's purpose, canonical planning surfaces, and operating model here.\n\n"
        "## Repo-Specific Guidance\n\n"
        "- Add the exact commands, constraints, and local conventions this repo expects.\n"
    )


def policy_loading_contract_section() -> str:
    return "\n".join(
        [
            "## Policy Loading Contract",
            "",
            "- `AGENTS.md` is a routing surface, not a one-time pointer.",
            "- Re-read the relevant policy files under `docs/dev/policies/` at the start of any non-trivial turn.",
            "- Re-read the relevant policy files when task scope changes mid-session.",
            "- When behavior is ambiguous, prefer re-reading policy over improvising from stale assumptions.",
        ]
    )


def policy_reread_triggers_section(repo_purpose: str | None) -> str:
    lines = [
        "## Policy Re-read Triggers",
        "",
        "- re-read planning-related policy before opening, revising, or closing a substantive plan",
        "- re-read documentation-related policy before changing docs, contracts, or canonical authorities",
        "- re-read validation and closeout policy before claiming work complete",
    ]
    if repo_purpose in {"operations-platform", "website-maintenance"}:
        lines.append("- re-read runtime or environment-boundary policy before touching live state, tenant state, deploy state, or off-repo operator data")
    if repo_purpose == "course-workspace":
        lines.append("- re-read course, LMS, cloud-drive, and student-data policy before changing course config, assignments, submissions, grades, announcements, files, pages, modules, quizzes, forms, or course folders")
        lines.append("- re-read validation policy before any live LMS write or course-data handoff")
    if repo_purpose in {"operations-platform", "workspace-agent", "product-engineering"}:
        lines.append("- re-read branch, commit, and integration policy before starting a multi-file or multi-step implementation slice")
    return "\n".join(lines)


def runtime_boundary_reminder_section(repo_purpose: str | None) -> tuple[str, str] | None:
    if repo_purpose != "operations-platform":
        return None
    return (
        "Tenant Boundary Reminder",
        "\n".join(
        [
            "## Tenant Boundary Reminder",
            "",
            "- Keep tenant-scoped or user-scoped runtime state out of the product repo unless the repo's runtime-state policy explicitly says it belongs in a separately governed tracked state surface.",
            "- Re-check boundary policy before copying runtime facts, artifacts, or fieldwork output into tracked repo files.",
        ]),
    )


def render_agents_wirein(
    repo_root: Path,
    install_plan: list[dict[str, Any]],
    existing_policy_surfaces: list[dict],
    repo_purpose: str | None = None,
) -> str:
    existing_targets = sorted(
        {
            Path(item["path"]).relative_to(repo_root).as_posix()
            for item in existing_policy_surfaces
            if item["source_type"] == "canonical-policy"
        }
    )
    planned_targets = [Path(item["target_policy_path"]).relative_to(repo_root).as_posix() for item in install_plan]
    all_targets = existing_targets + [target for target in planned_targets if target not in existing_targets]
    repo_name = repo_root.name.replace("-", " ").title()
    policy_lines = [
        "## Policy Entry",
        "",
        "This repo keeps its durable repo-local policy under `docs/dev/policies/`.",
        "",
        "Read and follow:",
    ]
    policy_lines.extend(f"- `{target}`" for target in all_targets)
    policy_section = "\n".join(policy_lines)
    loading_section = policy_loading_contract_section()
    reread_section = policy_reread_triggers_section(repo_purpose)
    runtime_section = runtime_boundary_reminder_section(repo_purpose)
    scope_section = "\n".join(
        [
            "## Scope",
            "",
            "- `AGENTS.md` includes repo-local guidance plus the policy entry section.",
            "- The durable policy body lives under `docs/dev/policies/`.",
            "- Keep repo-specific commands, environment details, and operational caveats in this file or adjacent local docs.",
        ]
    )

    agents_path = repo_root / "AGENTS.md"
    existing_text = read_text(agents_path)
    if existing_text and not is_thin_agents_wirein(existing_text):
        updated = replace_or_append_section(existing_text, "Policy Loading Contract", loading_section)
        updated = replace_or_append_section(updated, "Policy Re-read Triggers", reread_section)
        if runtime_section:
            runtime_heading, runtime_content = runtime_section
            updated = replace_or_append_section(updated, runtime_heading, runtime_content)
        updated = replace_or_append_section(updated, "Policy Entry", policy_section)
        updated = replace_or_append_section(updated, "Scope", scope_section)
        return updated

    lines = [
        f"# {repo_name}",
        "",
        purpose_scaffold_text(repo_purpose).rstrip(),
        "",
        loading_section,
        "",
        reread_section,
    ]
    if runtime_section:
        _, runtime_content = runtime_section
        lines.extend(["", runtime_content])
    lines.extend(["", policy_section, "", scope_section])
    return "\n".join(lines).rstrip() + "\n"


def choose_profile(signals: dict, installed_library: dict[str, Any]) -> tuple[str, str | None, str | None, str, list[str], list[str]]:
    purpose, subtype, reasons = classify_purpose(signals)
    if purpose == "product-engineering":
        profile = "repo-product-engineering"
    elif purpose == "operations-platform":
        profile = "operations-platform"
    elif purpose == "website-maintenance":
        profile = "website-maintenance"
    elif purpose == "course-workspace":
        profile = "course-workspace"
    elif purpose == "seminal-workspace":
        profile = "seminal-workspace"
    elif purpose == "writing-project":
        profile = "writing-project"
        if signals["mentions_review_workspace"]:
            subtype = "grant-proposal-review"
        elif signals["mentions_analysis_workspace"]:
            subtype = "patent-application-writing"
        elif signals["mentions_writing_deliverable"]:
            subtype = "grant-proposal-writing"
    elif purpose == "workspace-agent":
        profile = "skill-repo-maintainer"
    else:
        profile = "standalone-library"
    modules = base_modules_for_profile(profile, installed_library)
    if not modules:
        modules = ["policy-management"]
    execution_bias, bias_reasons = classify_execution_bias(signals, purpose)
    if execution_bias is None:
        execution_bias = profile_override_value(profile, "execution_bias", installed_library)
    reasons.extend(bias_reasons)

    if signals["mentions_parallel"] and "planning-discipline" not in modules:
        modules.insert(0, "planning-discipline")
        reasons.append("repo policy already expects parallel execution")
    if signals["mentions_goal_execution"] and "planning-discipline" not in modules:
        modules.insert(0, "planning-discipline")
        reasons.append("repo language explicitly references long-running goal execution")
    if signals["mentions_goal_execution"] and "goal-execution-governance" not in modules:
        modules.append("goal-execution-governance")
        reasons.append("repo language explicitly references /goal or long-running goal execution")
    if signals["mentions_goal_execution"] and "subagent-workflow-optimization" not in modules:
        modules.append("subagent-workflow-optimization")
        reasons.append("long-running goal execution benefits from explicit context and delegation governance")
    if signals["mentions_goal_execution"] and "parallel-plan-design" not in modules:
        modules.append("parallel-plan-design")
        reasons.append("long-running goal execution needs explicit dependencies, joins, and bounded feedback edges")
    if signals["mentions_goal_execution"] and "validation-and-handoff" not in modules:
        modules.append("validation-and-handoff")
        reasons.append("long-running goal execution needs independent outcome verification and bounded review")
    if signals["mentions_subagents"] and "subagent-workflow-optimization" not in modules:
        modules.append("subagent-workflow-optimization")
        reasons.append("repo language explicitly references delegated or subagent workflows")
    if signals["mentions_subagents"] and "parallel-plan-design" not in modules and purpose in {"product-engineering", "workspace-agent"}:
        modules.append("parallel-plan-design")
        reasons.append("repo language suggests plan structure should support parallel delegated work")
    if signals["mentions_subagents"] and "multi-agent-reconciliation" not in modules and purpose in {"product-engineering", "workspace-agent"}:
        modules.append("multi-agent-reconciliation")
        reasons.append("repo language suggests explicit multi-agent reconciliation rules")
    if signals["mentions_subagent_runtime"] and "subagent-runtime-governance" not in modules:
        modules.append("subagent-runtime-governance")
        reasons.append("repo language suggests spawned-agent runtime lifecycle, tool, transcript, or concurrency governance")
    if (signals["notes_migration_needed"] or signals["has_notes_dir"] or signals["has_memories_dir"]) and "notes-and-memories" not in modules:
        modules.append("notes-and-memories")
        reasons.append("repo shows notes/memories continuity needs or legacy note clutter")
    if signals["mentions_graph_backed_memory"] and "graph-backed-memory-usage" not in modules:
        modules.append("graph-backed-memory-usage")
        reasons.append("repo language suggests installed graph-backed durable memory usage")
    if signals["mentions_graph_backed_memory"] and "notes-and-memories" not in modules:
        modules.append("notes-and-memories")
        reasons.append("graph-backed memory still needs richer human-readable continuity alongside retrievable facts")
    if signals["mentions_codegraph_usage"] and "codegraph-usage" not in modules:
        modules.append("codegraph-usage")
        reasons.append("repo language suggests codegraph-backed code discovery or impact analysis")
    if signals["mentions_memory_service_runtime"] and "memory-service-runtime-governance" not in modules:
        modules.append("memory-service-runtime-governance")
        reasons.append("repo language suggests installed memory-service runtime operations")
    if signals["mentions_architecture_guardrails"] and "architecture-guardrails" not in modules:
        modules.append("architecture-guardrails")
        reasons.append("repo policy emphasizes architecture or service-boundary discipline")
    if signals["mentions_doc_hygiene"] and "documentation-change-control" not in modules:
        modules.append("documentation-change-control")
        reasons.append("repo policy requires same-slice documentation upkeep")
    if signals["mentions_validation_handoff"] and "validation-and-handoff" not in modules:
        modules.append("validation-and-handoff")
        reasons.append("repo policy emphasizes explicit verification and handoff quality")
    if signals["mentions_preview_artifact_review"] and "preview-artifact-review" not in modules:
        modules.append("preview-artifact-review")
        reasons.append("repo language suggests preview sessions or browser-based human review for generated artifacts")
    if signals["mentions_policy_harvest"] and "policy-harvest-loop" not in modules:
        modules.append("policy-harvest-loop")
        reasons.append("repo language suggests reusable policy harvesting")
    if signals["mentions_git_policy"] and "git-worktree-hygiene" not in modules:
        modules.append("git-worktree-hygiene")
    if signals["mentions_upstream_fork"] and "upstream-fork-maintenance" not in modules:
        modules.append("upstream-fork-maintenance")
        reasons.append("repo signals indicate private or local work layered on a non-owned upstream")
    deduped_modules: list[str] = []
    for module_id in modules:
        if module_id not in deduped_modules:
            deduped_modules.append(module_id)
    return purpose, subtype, execution_bias, profile, deduped_modules, reasons


def choose_adoption_mode(signals: dict, expectation_gaps: list[str], coverage: dict[str, Any]) -> tuple[str, list[str], dict[str, str]]:
    reasons: list[str] = []
    duplicate_authorities = signals.get("duplicate_planning_authorities", {})
    if duplicate_authorities:
        for surface_type, paths in duplicate_authorities.items():
            joined = ", ".join(paths)
            reasons.append(
                f"duplicate canonical {surface_type} authorities must be consolidated before policy adoption completes: {joined}"
            )
    if signals["planning_migration_needed"]:
        reasons.append("legacy, cluttered, or duplicate planning files should be migrated into canonical planning surfaces first")
    if signals["notes_migration_needed"]:
        reasons.append("legacy or cluttered notes/memories should be migrated into canonical docs/dev locations first")
    reasons.extend(expectation_gaps)

    if reasons:
        return (
            "migration-first",
            reasons,
            {
                "canonical_roadmap": "choose one canonical ROADMAP.md authority and retire or merge duplicates",
                "canonical_runbook": "choose one canonical RUNBOOK.md authority and retire or merge duplicates",
                "plans": "docs/dev/plans/",
                "notes": "docs/dev/notes/",
                "memories": "docs/dev/memories/",
                "policies": "docs/dev/policies/",
                "policy_entrypoint": "AGENTS.md",
            },
        )
    return (
        "clean-adoption",
        [],
        {
            "policies": "docs/dev/policies/",
            "policy_entrypoint": "AGENTS.md",
        },
    )


def summarize_policy_surface_actions(items: list[dict]) -> list[dict]:
    return [
        {
            "path": item["path"],
            "action": item["action"],
            "rationale": item["rationale"],
        }
        for item in items
    ]


def summarize_migration_surface_actions(items: list[dict]) -> list[dict]:
    return [
        {
            "path": item["path"],
            "surface_type": item["surface_type"],
            "action": item["action"],
            "rationale": item["rationale"],
        }
        for item in items
    ]


def validate_recommendations(profile: str, modules: list[str], library: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    module_ids = set(library["module_ids"])
    profile_ids = set(library["profile_ids"])
    if not library["catalog_found"]:
        problems.append("installed policy library catalog.yaml not found")
        return problems
    if profile not in profile_ids:
        problems.append(f"recommended profile missing from installed library: {profile}")
    for module_id in modules:
        if module_id not in module_ids:
            problems.append(f"recommended module missing from installed library: {module_id}")
    return problems


def write_drafts(repo_root: Path, install_plan: list[dict[str, Any]], agents_patch: str) -> list[str]:
    written: list[str] = []
    for item in install_plan:
        if item["action"] != "install-new":
            continue
        target_path = Path(item["target_policy_path"])
        if target_path.exists():
            raise FileExistsError(f"refusing to overwrite existing policy file: {target_path}")
        target_path.parent.mkdir(parents=True, exist_ok=True)
        target_path.write_text(item["draft_content"], encoding="utf-8")
        written.append(str(target_path))
    agents_path = repo_root / "AGENTS.md"
    agents_path.write_text(agents_patch, encoding="utf-8")
    written.append(str(agents_path))
    return written


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--policy-root", required=False)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--write-drafts", action="store_true")
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    policy_root = Path(args.policy_root).resolve() if args.policy_root else None
    installed_library = enumerate_policy_library(policy_root)
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
    next_modules = (
        coverage["missing_recommended_modules"]
        if rec_mode == "patch-missing"
        else modules
    )
    install_plan = build_install_plan(repo_root, next_modules, coverage, installed_library)
    agents_patch = render_agents_wirein(repo_root, install_plan, existing_policy_surfaces, purpose)
    written_paths: list[str] = []
    if args.write_drafts:
        written_paths = write_drafts(repo_root, install_plan, agents_patch)
    out = {
        "repo_root": str(repo_root),
        "policy_root": installed_library["policy_root"],
        "installed_policy_library": installed_library,
        "repo_purpose": purpose,
        "workflow_subtype": subtype,
        "execution_bias": execution_bias,
        "adoption_mode": adoption_mode,
        "recommendation_mode": rec_mode,
        "migration_reasons": migration_reasons,
        "migration_targets": migration_targets,
        "profile_expectation_gaps": expectation_gaps,
        "policy_adoption_coverage": coverage,
        "existing_policy_surfaces": existing_policy_surfaces,
        "repo_local_policy_findings": repo_local_policy_findings,
        "policy_surface_actions": summarize_policy_surface_actions(existing_policy_surfaces),
        "existing_migration_surfaces": existing_migration_surfaces,
        "migration_surface_actions": summarize_migration_surface_actions(existing_migration_surfaces),
        "recommended_profile": profile,
        "recommended_modules": modules,
        "next_modules": next_modules,
        "install_plan": install_plan,
        "agents_wirein_patch": agents_patch,
        "written_paths": written_paths,
        "validation_problems": validation_problems,
        "signals": signals,
        "reasons": reasons,
    }
    if args.json:
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        print(f"repo_purpose: {purpose}")
        print(f"workflow_subtype: {subtype or '-'}")
        print(f"execution_bias: {execution_bias or '-'}")
        print(f"profile: {profile}")
        print(f"adoption_readiness: {coverage['readiness']}")
        print(f"recommendation_mode: {rec_mode}")
        print(f"modules: {', '.join(modules)}")
        print(f"next_modules: {', '.join(next_modules) if next_modules else '-'}")
        if install_plan:
            print("install_plan:")
            for item in install_plan:
                print(f"- {item['action']} {item['module_id']} -> {item['target_policy_path']}")
        print("agents_wirein_patch:")
        print(agents_patch.rstrip())
        if written_paths:
            print("written_paths:")
            for path in written_paths:
                print(f"- {path}")
        print("reasons:")
        for reason in reasons:
            print(f"- {reason}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
