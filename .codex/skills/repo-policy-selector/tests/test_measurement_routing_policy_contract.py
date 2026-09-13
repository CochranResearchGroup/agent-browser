from pathlib import Path
import unittest


class MeasurementRoutingPolicyContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repo_root = Path(__file__).resolve().parents[2]
        cls.bundle_root = cls.repo_root / "repo-policy-selector" / "policy-library"

    def module_text(self, module_id: str) -> str:
        return (self.repo_root / "modules" / f"{module_id}.md").read_text(encoding="utf-8")

    def normalized_module_text(self, module_id: str) -> str:
        return " ".join(self.module_text(module_id).split())

    def test_measurement_first_and_causal_invalidation_contract(self) -> None:
        planning = self.normalized_module_text("planning-discipline")
        goal = self.normalized_module_text("goal-execution-governance")
        validation = self.normalized_module_text("validation-and-handoff")

        self.assertIn("smallest usable baseline", planning)
        self.assertIn("independent acceptance axes", planning)
        self.assertIn("every packet required for acceptance", goal)
        self.assertIn("causal path", goal)
        self.assertIn("evidence deadline", goal)
        self.assertIn("Preserve every completed sample", validation)
        self.assertIn("invalidation map", validation)

    def test_economical_worker_contract(self) -> None:
        model = self.normalized_module_text("model-selection-and-calibration")
        subagent = self.normalized_module_text("subagent-workflow-optimization")

        self.assertIn("to tools before any model", model)
        self.assertIn("calibrated economical tier", model)
        self.assertIn("final acceptance claim", model)
        self.assertIn("Prefer economical workers", subagent)
        self.assertIn("Repeating its full investigation", subagent)

    def test_changed_source_modules_match_selector_bundle(self) -> None:
        for module_id in (
            "planning-discipline",
            "goal-execution-governance",
            "validation-and-handoff",
            "model-selection-and-calibration",
            "subagent-workflow-optimization",
        ):
            source = self.repo_root / "modules" / f"{module_id}.md"
            bundled = self.bundle_root / "modules" / f"{module_id}.md"
            self.assertEqual(source.read_bytes(), bundled.read_bytes(), module_id)


if __name__ == "__main__":
    unittest.main()
