import importlib.util
import tempfile
import textwrap
import unittest
from pathlib import Path


AUDIT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "audit_planning_contract.py"


def load_audit_module():
    spec = importlib.util.spec_from_file_location("audit_planning_contract", AUDIT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class GoalExecutionContractAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit = load_audit_module()

    def make_repo(self, policy_text: str | None = None) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        repo_root = Path(temp_dir.name)
        if policy_text is not None:
            policy_dir = repo_root / "docs" / "dev" / "policies"
            policy_dir.mkdir(parents=True)
            (policy_dir / "0001-goal-execution-governance.md").write_text(
                textwrap.dedent(policy_text).strip() + "\n",
                encoding="utf-8",
            )
        return repo_root

    def test_goal_contract_is_not_required_when_policy_is_absent(self):
        report = self.audit.audit_goal_execution_contract(self.make_repo())

        self.assertFalse(report["applicable"])
        self.assertTrue(report["ok"])

    def test_goal_contract_requires_concrete_bounds_and_checkpoint_fields(self):
        report = self.audit.audit_goal_execution_contract(
            self.make_repo("""
                # Policy | Goal Execution Governance

                Keep long goals bounded.
            """)
        )

        self.assertTrue(report["applicable"])
        self.assertFalse(report["ok"])
        self.assertTrue(any("max_work_unit_attempts" in problem for problem in report["problems"]))
        self.assertTrue(any("plan_version" in problem for problem in report["problems"]))

    def test_goal_contract_accepts_complete_local_bounds(self):
        report = self.audit.audit_goal_execution_contract(
            self.make_repo("""
                # Policy | Goal Execution Governance

                ## Local Goal Bounds

                max_work_unit_attempts: 2
                max_review_rework_cycles: 1
                max_hardening_checkpoints: 2
                checkpoint_interval: 3 slices or 60 minutes
                checkpoint_record_fields: plan_version, state_transition, progress_classification, evidence, subagent_status, next_action_or_stop_reason
            """)
        )

        self.assertTrue(report["applicable"])
        self.assertTrue(report["ok"], report["problems"])


class PlanningContractAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit = load_audit_module()

    def make_repo(self, policies: tuple[str, ...] = (), *, wired: bool = True) -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        root = Path(temp_dir.name)
        policy_dir = root / "docs/dev/policies"
        policy_dir.mkdir(parents=True)
        for index, policy in enumerate(policies, start=1):
            (policy_dir / f"{index:04d}-{policy}.md").write_text("policy\n", encoding="utf-8")
        if policies and wired:
            (root / "AGENTS.md").write_text(
                "Read and follow the files under docs/dev/policies.\n",
                encoding="utf-8",
            )
        return root

    def test_is_not_applicable_without_adopted_planning_policy(self):
        root = self.make_repo()
        report = self.audit.audit_repo(root)

        self.assertFalse(report["applicable"])
        self.assertTrue(report["ok"])
        self.assertEqual(report["problems"], [])

    def test_normal_audit_still_enforces_adopted_goal_contract(self):
        root = self.make_repo()
        (root / "docs/dev/policies/0001-goal-execution-governance.md").write_text(
            "# Goal policy without local bounds\n",
            encoding="utf-8",
        )

        report = self.audit.audit_repo(root)

        self.assertFalse(report["applicable"])
        self.assertFalse(report["ok"])
        self.assertTrue(any("max_work_unit_attempts" in item for item in report["problems"]))

    def test_unwired_planning_policy_is_available_but_not_adopted(self):
        root = self.make_repo(("planning-discipline",), wired=False)

        report = self.audit.audit_repo(root)

        self.assertTrue(report["available_contracts"]["planning_discipline"])
        self.assertFalse(report["adopted_contracts"]["planning_discipline"])
        self.assertFalse(report["applicable"])
        self.assertTrue(report["ok"])

    def test_planning_only_does_not_require_roadmap_runbook_or_lane(self):
        root = self.make_repo(("planning-discipline",))
        plans = root / "custom/plans"
        plans.mkdir(parents=True)
        (plans / "0001-2026-07-20-work.md").write_text(
            "State: CLOSED\n",
            encoding="utf-8",
        )

        report = self.audit.audit_repo(root, plans_dir_path="custom/plans")

        self.assertTrue(report["applicable"])
        self.assertTrue(report["ok"], report["problems"])

    def test_roadmap_contract_requires_wiring_but_allows_non_turn_sections(self):
        root = self.make_repo(("planning-discipline", "roadmap-runbook-governance"))
        plans = root / "docs/dev/plans"
        plans.mkdir(parents=True)
        plan_name = "0001-2026-07-20-work.md"
        (plans / plan_name).write_text("State: OPEN\nLane: P01\n## Current State\nReady.\n", encoding="utf-8")
        (root / "ROADMAP.md").write_text(
            f"# Roadmap\n\n## Introduction\nText.\n\n## P01 | Work\nState: OPEN\nCurrent State: Ready\n{plan_name}\n",
            encoding="utf-8",
        )
        (root / "RUNBOOK.md").write_text(
            f"# Runbook\n\n## Operating rules\nText.\n\n## Turn 1 | 2026-07-20\n{plan_name}\n",
            encoding="utf-8",
        )

        report = self.audit.audit_repo(root)

        self.assertTrue(report["ok"], report["problems"])

    def test_active_only_excludes_closed_and_unclassified_legacy_plans(self):
        root = self.make_repo(("planning-discipline",))
        plans = root / "docs/dev/plans"
        plans.mkdir(parents=True)
        (plans / "0001-2026-07-20-closed.md").write_text("State: CLOSED\n", encoding="utf-8")
        (plans / "legacy.md").write_text("No state.\n", encoding="utf-8")
        (plans / "0002-2026-07-20-open.md").write_text(
            "State: OPEN\n## Current State\nReady.\n",
            encoding="utf-8",
        )

        report = self.audit.audit_repo(root, active_only=True)

        self.assertTrue(report["ok"], report["problems"])
        self.assertEqual([item["file"] for item in report["plans"]], ["0002-2026-07-20-open.md"])
        self.assertEqual(report["excluded_closed_plans"], ["0001-2026-07-20-closed.md"])
        self.assertEqual(report["excluded_unclassified_plans"], ["legacy.md"])

    def test_force_preserves_strict_pre_adoption_authority_checks(self):
        root = self.make_repo()

        report = self.audit.audit_repo(root, force=True)

        self.assertFalse(report["ok"])
        self.assertIn("missing ROADMAP.md", report["problems"])
        self.assertIn("missing RUNBOOK.md", report["problems"])
        self.assertTrue(any("missing plans directory" in item for item in report["problems"]))

    def test_active_only_still_rejects_malformed_turn_headings(self):
        root = self.make_repo(("planning-discipline", "roadmap-runbook-governance"))
        (root / "docs/dev/plans").mkdir(parents=True)
        (root / "ROADMAP.md").write_text("# Roadmap\n", encoding="utf-8")
        (root / "RUNBOOK.md").write_text("# Runbook\n\n## Turn latest\n", encoding="utf-8")

        report = self.audit.audit_repo(root, active_only=True)

        self.assertFalse(report["ok"])
        self.assertTrue(any("RUNBOOK.md has headings" in item for item in report["problems"]))
