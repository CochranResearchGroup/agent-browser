#!/usr/bin/env python3
from __future__ import annotations
import sys

sys.dont_write_bytecode = True

import argparse
import json
import re
from pathlib import Path


ROADMAP_HEADING_RE = re.compile(r"^##\s+P\d{2}\s+\|\s+.+$")
ROADMAP_LANE_HEADING_PREFIX_RE = re.compile(r"^##\s+P\d+")
RUNBOOK_TURN_RE = re.compile(r"^##\s+Turn\s+\d+\s+\|\s+\d{4}-\d{2}-\d{2}$")
RUNBOOK_TURN_HEADING_PREFIX_RE = re.compile(r"^##\s+Turn\b", re.IGNORECASE)
PLAN_FILE_RE = re.compile(r"^\d{4}-\d{4}-\d{2}-\d{2}-[a-z0-9-]+\.md$")
PLAN_STATE_RE = re.compile(r"(?im)^(?:state|status)\s*:\s*(PLANNED|OPEN|CLOSED|CANCELLED)\s*$")
ROADMAP_LANE_RE = re.compile(r"(?im)^(?:roadmap|lane|phase)\s*:\s*(P\d{2})\b")
CURRENT_STATE_RE = re.compile(r"(?im)^##\s+Current State\s*$|^(?:current state)\s*:", re.MULTILINE)
GOAL_BOUND_PATTERNS = {
    "max_work_unit_attempts": re.compile(r"(?im)^max_work_unit_attempts\s*:\s*[1-9]\d*\s*$"),
    "max_review_rework_cycles": re.compile(r"(?im)^max_review_rework_cycles\s*:\s*[1-9]\d*\s*$"),
    "max_hardening_checkpoints": re.compile(r"(?im)^max_hardening_checkpoints\s*:\s*[1-9]\d*\s*$"),
    "checkpoint_interval": re.compile(
        r"(?im)^checkpoint_interval\s*:\s*[1-9]\d*\s+.*(?:minute|hour|slice|token|turn|context)"
    ),
}
GOAL_CHECKPOINT_FIELD_TOKENS = {
    "plan_version",
    "state_transition",
    "progress_classification",
    "evidence",
    "subagent_status",
    "next_action_or_stop_reason",
}


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return ""


def split_roadmap_sections(roadmap_text: str) -> dict[str, str]:
    sections: dict[str, str] = {}
    current_lane: str | None = None
    current_lines: list[str] = []
    for line in roadmap_text.splitlines():
        if line.startswith("## "):
            if current_lane is not None:
                sections[current_lane] = "\n".join(current_lines).strip()
            current_lines = [line]
            match = re.match(r"^##\s+(P\d{2})\s+\|", line)
            current_lane = match.group(1) if match else None
        elif current_lane is not None:
            current_lines.append(line)
    if current_lane is not None:
        sections[current_lane] = "\n".join(current_lines).strip()
    return sections


def audit_goal_execution_contract(root: Path) -> dict:
    policy_dir = root / "docs" / "dev" / "policies"
    policy_paths = sorted(policy_dir.glob("*goal-execution-governance.md")) if policy_dir.exists() else []
    problems: list[str] = []
    policies: list[dict[str, object]] = []

    for policy_path in policy_paths:
        text = read_text(policy_path)
        missing_bounds = [name for name, pattern in GOAL_BOUND_PATTERNS.items() if not pattern.search(text)]
        fields_match = re.search(r"(?im)^checkpoint_record_fields\s*:\s*(.+)$", text)
        fields = {
            field.strip()
            for field in fields_match.group(1).split(",")
            if field.strip()
        } if fields_match else set()
        missing_fields = sorted(GOAL_CHECKPOINT_FIELD_TOKENS - fields)
        if "## Local Goal Bounds" not in text:
            problems.append(f"goal policy missing Local Goal Bounds section: {policy_path.name}")
        for name in missing_bounds:
            problems.append(f"goal policy missing concrete bound {name}: {policy_path.name}")
        for name in missing_fields:
            problems.append(f"goal policy missing checkpoint field {name}: {policy_path.name}")
        policies.append(
            {
                "path": str(policy_path),
                "missing_bounds": missing_bounds,
                "missing_checkpoint_fields": missing_fields,
            }
        )

    return {
        "repo_root": str(root),
        "applicable": bool(policy_paths),
        "policies": policies,
        "ok": not problems,
        "problems": problems,
    }


def planning_contracts(root: Path) -> tuple[dict[str, bool], dict[str, bool]]:
    policy_dir = root / "docs" / "dev" / "policies"
    available = {
        "planning_discipline": bool(list(policy_dir.glob("*planning-discipline.md"))) if policy_dir.exists() else False,
        "roadmap_runbook_governance": bool(list(policy_dir.glob("*roadmap-runbook-governance.md"))) if policy_dir.exists() else False,
    }
    agents_text = read_text(root / "AGENTS.md") or read_text(root / "AGENT.MD")
    policy_wired = bool(
        re.search(r"docs/dev/policies|docs/dev/agent-policies", agents_text, re.IGNORECASE)
        and re.search(r"\b(?:read|follow|policy entry|policy loading)\b", agents_text, re.IGNORECASE)
    )
    adopted = {name: bool(present and policy_wired) for name, present in available.items()}
    return available, adopted


def resolve_repo_path(root: Path, value: str | Path | None, default: str) -> Path:
    path = Path(value) if value is not None else Path(default)
    return path if path.is_absolute() else root / path


def audit_repo(
    root: Path,
    *,
    roadmap_path: str | Path | None = None,
    runbook_path: str | Path | None = None,
    plans_dir_path: str | Path | None = None,
    active_only: bool = False,
    force: bool = False,
) -> dict:
    roadmap = resolve_repo_path(root, roadmap_path, "ROADMAP.md")
    runbook = resolve_repo_path(root, runbook_path, "RUNBOOK.md")
    plans_dir = resolve_repo_path(root, plans_dir_path, "docs/dev/plans")
    available_contracts, contracts = planning_contracts(root)
    planning_applicable = contracts["planning_discipline"] or contracts["roadmap_runbook_governance"]
    roadmap_applicable = contracts["roadmap_runbook_governance"] or force

    problems: list[str] = []
    report: dict[str, object] = {
        "repo_root": str(root),
        "applicable": planning_applicable or force,
        "available_contracts": available_contracts,
        "adopted_contracts": contracts,
        "audit_scope": "active" if active_only else "all",
        "roadmap_path": str(roadmap),
        "runbook_path": str(runbook),
        "plans_dir": str(plans_dir),
        "plans": [],
        "excluded_closed_plans": [],
        "excluded_unclassified_plans": [],
    }

    if not planning_applicable and not force:
        goal_contract = audit_goal_execution_contract(root)
        goal_problems = goal_contract["problems"]
        assert isinstance(goal_problems, list)
        report["ok"] = bool(goal_contract["ok"])
        report["problems"] = list(goal_problems)
        report["goal_execution_contract"] = goal_contract
        return report

    roadmap_text = read_text(roadmap)
    runbook_text = read_text(runbook)

    if roadmap_applicable and not roadmap_text:
        problems.append("missing ROADMAP.md")
    if roadmap_applicable and not runbook_text:
        problems.append("missing RUNBOOK.md")
    if not plans_dir.exists():
        problems.append(f"missing plans directory: {plans_dir}")

    roadmap_headings = [
        line for line in roadmap_text.splitlines() if ROADMAP_LANE_HEADING_PREFIX_RE.match(line)
    ]
    bad_headings = [line for line in roadmap_headings if not ROADMAP_HEADING_RE.match(line)]
    if roadmap_applicable and roadmap_text and bad_headings:
        problems.append("ROADMAP.md has top-level headings that do not match '## P## | Title'")
    report["roadmap_headings"] = roadmap_headings
    roadmap_sections = split_roadmap_sections(roadmap_text)
    open_roadmap_lanes = [
        lane_id
        for lane_id, section in roadmap_sections.items()
        if re.search(r"(?im)^(?:state|status)\s*:\s*OPEN\s*$", section)
    ]
    report["open_roadmap_lanes"] = open_roadmap_lanes
    for lane_id in open_roadmap_lanes if roadmap_applicable else []:
        section = roadmap_sections[lane_id]
        if not CURRENT_STATE_RE.search(section):
            problems.append(f"OPEN roadmap lane missing Current State note: {lane_id}")

    runbook_turns = [
        line for line in runbook_text.splitlines() if RUNBOOK_TURN_HEADING_PREFIX_RE.match(line)
    ]
    bad_turns = [line for line in runbook_turns if not RUNBOOK_TURN_RE.match(line)]
    if roadmap_applicable and runbook_text and bad_turns:
        problems.append("RUNBOOK.md has headings that do not match '## Turn N | YYYY-MM-DD'")
    report["runbook_turns"] = runbook_turns

    if plans_dir.exists():
        for plan_path in sorted(plans_dir.glob("*.md")):
            entry = {
                "file": plan_path.name,
                "path": str(plan_path),
                "filename_ok": bool(PLAN_FILE_RE.match(plan_path.name)),
                "state": None,
                "state_ok": False,
                "lane_id": None,
                "lane_ok": False,
                "current_state_ok": False,
                "wired_in_roadmap": False,
                "wired_in_runbook": False,
            }
            text = read_text(plan_path)
            state_match = PLAN_STATE_RE.search(text)
            lane_match = ROADMAP_LANE_RE.search(text)
            if active_only and not state_match:
                excluded = report["excluded_unclassified_plans"]
                assert isinstance(excluded, list)
                excluded.append(plan_path.name)
                continue
            if active_only and state_match and state_match.group(1) not in {"PLANNED", "OPEN"}:
                excluded = report["excluded_closed_plans"]
                assert isinstance(excluded, list)
                excluded.append(plan_path.name)
                continue
            if state_match:
                entry["state"] = state_match.group(1)
                entry["state_ok"] = True
            if lane_match:
                entry["lane_id"] = lane_match.group(1)
                entry["lane_ok"] = True
            entry["current_state_ok"] = bool(CURRENT_STATE_RE.search(text))
            entry["wired_in_roadmap"] = plan_path.name in roadmap_text
            entry["wired_in_runbook"] = plan_path.name in runbook_text
            if not entry["filename_ok"]:
                problems.append(f"plan filename does not match deterministic pattern: {plan_path.name}")
            if not entry["state_ok"]:
                problems.append(f"plan missing deterministic state: {plan_path.name}")
            if roadmap_applicable and not entry["lane_ok"]:
                problems.append(f"plan missing roadmap lane id: {plan_path.name}")
            if entry["state"] == "OPEN" and not entry["current_state_ok"]:
                problems.append(f"OPEN plan missing Current State section: {plan_path.name}")
            if roadmap_applicable and not entry["wired_in_roadmap"]:
                problems.append(f"plan not wired in ROADMAP.md: {plan_path.name}")
            if roadmap_applicable and not entry["wired_in_runbook"]:
                problems.append(f"plan not wired in RUNBOOK.md: {plan_path.name}")
            cast_list = report["plans"]
            assert isinstance(cast_list, list)
            cast_list.append(entry)
        plans = report["plans"]
        assert isinstance(plans, list)
        actionable_states = {"PLANNED", "OPEN"}
        for lane_id in open_roadmap_lanes if roadmap_applicable else []:
            if not any(
                plan.get("lane_id") == lane_id and plan.get("state") in actionable_states
                for plan in plans
                if isinstance(plan, dict)
            ):
                problems.append(f"OPEN roadmap lane missing actionable plan coverage: {lane_id}")

    report["ok"] = not problems
    report["problems"] = problems
    goal_contract = audit_goal_execution_contract(root)
    report["goal_execution_contract"] = goal_contract
    if not goal_contract["ok"]:
        goal_problems = goal_contract["problems"]
        assert isinstance(goal_problems, list)
        problems.extend(goal_problems)
        report["ok"] = False
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--goal-only", action="store_true")
    parser.add_argument("--active-only", action="store_true")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--roadmap-path")
    parser.add_argument("--runbook-path")
    parser.add_argument("--plans-dir")
    args = parser.parse_args()

    root = Path(args.repo_root).resolve()
    report = audit_goal_execution_contract(root) if args.goal_only else audit_repo(
        root,
        roadmap_path=args.roadmap_path,
        runbook_path=args.runbook_path,
        plans_dir_path=args.plans_dir,
        active_only=args.active_only,
        force=args.force,
    )
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print(f"ok: {report['ok']}")
        if report["problems"]:
            print("problems:")
            for problem in report["problems"]:
                print(f"- {problem}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
