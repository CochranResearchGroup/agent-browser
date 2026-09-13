from pathlib import Path
import unittest


class MultiSessionPolicyHarvestContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repo_root = Path(__file__).resolve().parents[2]
        cls.bundle_root = cls.repo_root / "repo-policy-selector" / "policy-library"

    def module_text(self, module_id: str) -> str:
        return (self.repo_root / "modules" / f"{module_id}.md").read_text(
            encoding="utf-8"
        )

    def test_worktree_lifecycle_is_explicit(self) -> None:
        text = " ".join(self.module_text("git-worktree-hygiene").split())
        for required in (
            "whether an existing clean checkout already owns the intended branch and lane",
            "Create a new worktree only when",
            "do not repurpose another active lane's checkout",
            "Close a worktree promptly",
            "Do not accumulate idle worktrees",
            "verify the exact path is absent",
        ):
            self.assertIn(required, text)

    def test_coordination_contracts_are_split_across_existing_modules(self) -> None:
        active_lane = " ".join(self.module_text("active-lane-coordination").split())
        reconciliation = " ".join(
            self.module_text("multi-agent-reconciliation").split()
        )
        parallel = " ".join(self.module_text("parallel-plan-design").split())

        self.assertIn("one accountable execution owner", active_lane)
        self.assertIn("one coordination owner", active_lane)
        self.assertIn("single coordination owner", reconciliation)
        self.assertIn("Session topology is an implementation choice", reconciliation)
        self.assertIn("before dependent implementations fan out", parallel)

    def test_development_runtime_isolation_has_required_boundaries(self) -> None:
        text = " ".join(self.module_text("development-runtime-isolation").split())
        for required in (
            "production, staging, and development runtimes",
            "each concurrently executing development lane",
            "Bind runtime identity to the lane and exact source checkpoint",
            "Do not inherit production credentials",
            "singleton resources as serialized",
            "Teardown must target only the named lane runtime",
        ):
            self.assertIn(required, text)

    def test_source_modules_match_selector_bundle(self) -> None:
        for module_id in (
            "active-lane-coordination",
            "development-runtime-isolation",
            "git-worktree-hygiene",
            "multi-agent-reconciliation",
            "parallel-plan-design",
        ):
            source = self.repo_root / "modules" / f"{module_id}.md"
            bundled = self.bundle_root / "modules" / f"{module_id}.md"
            self.assertEqual(source.read_bytes(), bundled.read_bytes(), module_id)

    def test_new_module_is_cataloged_and_profiled(self) -> None:
        source_catalog = (self.repo_root / "catalog.yaml").read_text(encoding="utf-8")
        bundled_catalog = (self.bundle_root / "catalog.yaml").read_text(
            encoding="utf-8"
        )
        source_profile = (
            self.repo_root / "profiles" / "operations-platform.yaml"
        ).read_text(encoding="utf-8")
        bundled_profile = (
            self.bundle_root / "profiles" / "operations-platform.yaml"
        ).read_text(encoding="utf-8")

        self.assertIn("id: development-runtime-isolation", source_catalog)
        self.assertEqual(source_catalog, bundled_catalog)
        self.assertIn("- development-runtime-isolation", source_profile)
        self.assertEqual(source_profile, bundled_profile)


if __name__ == "__main__":
    unittest.main()
