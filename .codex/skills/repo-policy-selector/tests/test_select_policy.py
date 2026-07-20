import importlib.util
import tempfile
import textwrap
import unittest
from pathlib import Path


SELECT_POLICY_PATH = Path(__file__).resolve().parents[1] / "scripts" / "select_policy.py"


def load_select_policy_module():
    spec = importlib.util.spec_from_file_location("select_policy", SELECT_POLICY_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class SelectPolicyRegressionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.select_policy = load_select_policy_module()
        cls.policy_root = Path(__file__).resolve().parents[2] / "repo-policy-selector" / "policy-library"

    def make_repo(self, agents_text: str = "", readme_text: str = "") -> Path:
        temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(temp_dir.cleanup)
        repo_root = Path(temp_dir.name)
        if agents_text:
            (repo_root / "AGENTS.md").write_text(textwrap.dedent(agents_text).strip() + "\n", encoding="utf-8")
        if readme_text:
            (repo_root / "README.md").write_text(textwrap.dedent(readme_text).strip() + "\n", encoding="utf-8")
        (repo_root / "pyproject.toml").write_text("[project]\nname = 'fixture'\nversion = '0.0.0'\n", encoding="utf-8")
        return repo_root

    def test_agent_skill_repo_is_not_misclassified_as_course_workspace(self):
        repo_root = self.make_repo(
            agents_text="""
            # Agent Browser

            This repo is a browser automation tool for agents.

            ## Project Structure
            - scripts/
            - tests/
            - docs/dev/
            """,
            readme_text="""
            Browser automation CLI for agent workflows.
            Use it to inspect websites, forms, and rendered pages.
            """,
        )

        signals = self.select_policy.detect_signals(repo_root)

        self.assertFalse(signals["has_lms_config"])
        self.assertFalse(signals["mentions_lms_course"])
        self.assertFalse(signals["mentions_course_workspace"])
        self.assertFalse(signals["mentions_student_assessment_data"])

        purpose, _subtype, reasons = self.select_policy.classify_purpose(signals)
        self.assertNotEqual(purpose, "course-workspace", reasons)

        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        purpose, _subtype, _execution_bias, profile, _modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )
        self.assertNotEqual(purpose, "course-workspace", reasons)
        self.assertNotEqual(profile, "course-workspace", reasons)

    def test_graphiti_memory_policy_maps_to_shared_graph_memory_module(self):
        repo_root = self.make_repo()
        policy_dir = repo_root / "docs" / "dev" / "policies"
        policy_dir.mkdir(parents=True, exist_ok=True)
        (policy_dir / "0014-graphiti-memory-usage.md").write_text(
            textwrap.dedent(
                """
                # Policy | Graphiti Memory Usage

                ## Policy

                - Treat Graphiti as durable retrievable context, not as a scratchpad for every turn.
                - Before re-asking the user for likely durable context, prefer a bounded memory read with `search_memory_facts` or `search_nodes`.
                - Avoid memory spam and near-duplicate writes through `add_memory`.
                - Treat destructive maintenance tools and `group_id` partitioning as explicit operations.
                """
            ).strip()
            + "\n",
            encoding="utf-8",
        )

        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        surfaces = self.select_policy.extract_existing_policy_surfaces(repo_root)
        semantic_matches = self.select_policy.semantic_module_matches(surfaces, installed_library)

        self.assertIn("graph-backed-memory-usage", semantic_matches)
        self.assertEqual(
            semantic_matches["graph-backed-memory-usage"],
            [str(policy_dir / "0014-graphiti-memory-usage.md")],
        )
        self.assertNotIn("notes-and-memories", semantic_matches)

    def test_graph_backed_memory_is_in_every_starter_profile(self):
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        for profile_id in installed_library["profile_ids"]:
            modules = self.select_policy.base_modules_for_profile(profile_id, installed_library)
            self.assertIn("graph-backed-memory-usage", modules, profile_id)

    def test_goal_language_recommends_goal_and_subagent_governance(self):
        repo_root = self.make_repo(
            agents_text="""
            # Long-Running Agent Repo

            Use /goal for long-running goal execution across several bounded
            checkpoints. Stop on repeated hardening without outcome progress.
            """,
        )

        signals = self.select_policy.detect_signals(repo_root)
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        _purpose, _subtype, _execution_bias, _profile, modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )

        self.assertTrue(signals["mentions_goal_execution"])
        self.assertIn("goal-execution-governance", modules, reasons)
        self.assertIn("subagent-workflow-optimization", modules, reasons)
        self.assertIn("parallel-plan-design", modules, reasons)
        self.assertIn("validation-and-handoff", modules, reasons)

    def test_ordinary_product_goal_language_does_not_signal_long_goal_execution(self):
        repo_root = self.make_repo(
            readme_text="""
            Our product goal is a fast and reliable command-line interface.
            The long-running goal execution strategy belongs to the business
            roadmap and should stay goal-compatible with customer priorities.
            Browser user agent strings are preserved for interoperability.
            """,
        )

        signals = self.select_policy.detect_signals(repo_root)

        self.assertFalse(signals["mentions_goal_execution"])

    def test_long_horizon_profiles_include_goal_execution_governance(self):
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        for profile_id in ["repo-product-engineering", "operations-platform", "skill-repo-maintainer"]:
            modules = self.select_policy.base_modules_for_profile(profile_id, installed_library)
            self.assertIn("goal-execution-governance", modules, profile_id)

    def test_codegraph_policy_maps_to_shared_codegraph_module(self):
        repo_root = self.make_repo(
            agents_text="""
            # Codegraph-Aware Repo

            Use ../codegraph before non-trivial code edits, architecture tracing,
            callers/callees inspection, refactor planning, or impact analysis.
            Treat codegraph output as discovery evidence and still verify with source reads
            and targeted tests.
            """,
        )
        policy_dir = repo_root / "docs" / "dev" / "policies"
        policy_dir.mkdir(parents=True, exist_ok=True)
        (policy_dir / "0017-codegraph-usage.md").write_text(
            textwrap.dedent(
                """
                # Policy | Codegraph Usage

                ## Policy

                - Consult codegraph before code edits, architecture trace work, callers/callees inspection, refactor planning, or impact analysis.
                - Treat codegraph as discovery evidence; verify with source reads and targeted tests.
                """
            ).strip()
            + "\n",
            encoding="utf-8",
        )

        signals = self.select_policy.detect_signals(repo_root)
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        surfaces = self.select_policy.extract_existing_policy_surfaces(repo_root)
        semantic_matches = self.select_policy.semantic_module_matches(surfaces, installed_library)
        _purpose, _subtype, _execution_bias, _profile, modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )

        self.assertTrue(signals["mentions_codegraph_usage"])
        self.assertIn("codegraph-usage", semantic_matches)
        self.assertEqual(
            semantic_matches["codegraph-usage"],
            [str(policy_dir / "0017-codegraph-usage.md")],
        )
        self.assertIn("codegraph-usage", modules, reasons)

    def test_graphiti_runtime_policy_maps_to_memory_service_runtime_module(self):
        repo_root = self.make_repo(
            agents_text="""
            # Graphiti Runtime

            This repo operates a Graphiti MCP server as an installed memory service runtime.
            Agents must verify the installed release manifest, health endpoint, memory queue,
            dead-letter state, provider boundary, and read-after-write smoke before claiming
            install or restart work is complete.
            """,
        )
        policy_dir = repo_root / "docs" / "dev" / "policies"
        policy_dir.mkdir(parents=True, exist_ok=True)
        (policy_dir / "0015-graphiti-runtime.md").write_text(
            textwrap.dedent(
                """
                # Policy | Graphiti Runtime Operations

                ## Policy

                - Treat Graphiti as an installed memory-service runtime, not just a client tool.
                - Verify the installed release manifest, service manager state, runtime home, health endpoint, and bound listener before diagnosis.
                - Keep memory queue status durable and expose dead-letter list, requeue, and drop operations.
                - Run a read-after-write smoke after install, restart, provider, or backend changes.
                """
            ).strip()
            + "\n",
            encoding="utf-8",
        )

        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        surfaces = self.select_policy.extract_existing_policy_surfaces(repo_root)
        semantic_matches = self.select_policy.semantic_module_matches(surfaces, installed_library)
        signals = self.select_policy.detect_signals(repo_root)
        _purpose, _subtype, _execution_bias, _profile, modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )

        self.assertIn("memory-service-runtime-governance", semantic_matches)
        self.assertEqual(
            semantic_matches["memory-service-runtime-governance"],
            [str(policy_dir / "0015-graphiti-runtime.md")],
        )
        self.assertIn("memory-service-runtime-governance", modules, reasons)

    def test_memory_discovery_policy_maps_to_graph_memory_module(self):
        repo_root = self.make_repo(
            agents_text="""
            # Policy Repo

            Use the graphiti-discovery skill before non-trivial harvest work.
            Query the repo group agent_policies_main first. If the right memory group
            is unclear, use the memory atlas and verify any memory-derived claim against
            repo files before changing policy.
            """,
        )

        signals = self.select_policy.detect_signals(repo_root)
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        _purpose, _subtype, _execution_bias, _profile, modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )

        self.assertTrue(signals["mentions_graph_backed_memory"])
        self.assertIn("graph-backed-memory-usage", modules, reasons)

    def test_preview_artifact_policy_maps_to_shared_preview_module(self):
        repo_root = self.make_repo(
            agents_text="""
            # Artifact Review Repo

            Use the $previews skill when generated artifacts require human review.
            Publish report packets, rendered documents, local HTML builds, PDFs,
            Office documents, screenshots, and galleries as one preview session URL.
            Stop for approval feedback before release, upload, or mutation.
            """,
        )
        policy_dir = repo_root / "docs" / "dev" / "policies"
        policy_dir.mkdir(parents=True, exist_ok=True)
        (policy_dir / "0016-preview-artifact-review.md").write_text(
            textwrap.dedent(
                """
                # Policy | Preview Artifact Review

                ## Policy

                - Use the Previews service for browser review when generated artifacts require human approval.
                - Group review packets, PDFs, Office documents, screenshots, galleries, and local HTML builds into one preview session URL.
                - Stop and read approval feedback before performing the gated mutation.
                """
            ).strip()
            + "\n",
            encoding="utf-8",
        )

        signals = self.select_policy.detect_signals(repo_root)
        installed_library = self.select_policy.enumerate_policy_library(self.policy_root)
        surfaces = self.select_policy.extract_existing_policy_surfaces(repo_root)
        semantic_matches = self.select_policy.semantic_module_matches(surfaces, installed_library)
        _purpose, _subtype, _execution_bias, _profile, modules, reasons = self.select_policy.choose_profile(
            signals, installed_library
        )

        self.assertTrue(signals["mentions_preview_artifact_review"])
        self.assertIn("preview-artifact-review", semantic_matches)
        self.assertEqual(
            semantic_matches["preview-artifact-review"],
            [str(policy_dir / "0016-preview-artifact-review.md")],
        )
        self.assertIn("preview-artifact-review", modules, reasons)

    def test_subagent_runtime_signal_is_specific_to_runtime_lifecycle(self):
        workflow_repo = self.make_repo(
            agents_text="""
            # Workflow Repo

            This repo may use subagents for bounded delegated verification work.
            Keep write scopes disjoint and reconcile results in the main agent.
            """,
        )
        runtime_repo = self.make_repo(
            agents_text="""
            # Runtime Repo

            This repo operates a subagent runtime.
            Track subagent run id, subagent session id, transcript path, announce payload,
            max spawn depth, max children per agent, concurrency cap, cascade stop, and subagent cleanup.
            """,
        )

        workflow_signals = self.select_policy.detect_signals(workflow_repo)
        runtime_signals = self.select_policy.detect_signals(runtime_repo)

        self.assertTrue(workflow_signals["mentions_subagents"])
        self.assertFalse(workflow_signals["mentions_subagent_runtime"])
        self.assertTrue(runtime_signals["mentions_subagents"])
        self.assertTrue(runtime_signals["mentions_subagent_runtime"])


if __name__ == "__main__":
    unittest.main()
