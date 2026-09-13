from pathlib import Path
import unittest


class CollaborativeDevelopmentPolicyContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.repo_root = Path(__file__).resolve().parents[2]
        cls.bundle_root = cls.repo_root / "repo-policy-selector" / "policy-library"

    def module_text(self) -> str:
        return (
            self.repo_root / "modules" / "collaborative-development-workflow.md"
        ).read_text(encoding="utf-8")

    def test_collaboration_contract_has_required_boundaries(self) -> None:
        text = " ".join(self.module_text().split())

        for required in (
            "accountable human owner",
            "shared forge is the coordination and source-custody system of record",
            "only a merged pull request may modify it",
            "Do not push directly to the canonical branch",
            "search open work items and pull requests",
            "Claim it with an accountable owner",
            "never let two people or agent sessions edit the same checkout",
            "pull request linked to its work item",
            "may self-check and merge their own pull request",
            "Production deployment is allowed only from an exact commit",
            "verify the commit entered the branch through a merged pull request",
            "Deployment automation must fail closed",
            "urgent fixes through the same accelerated issue",
            "Do not duplicate active state in GitHub Issues and Jira",
        ):
            self.assertIn(required, text)

        for blanket_gate in (
            "one required peer review",
            "someone other than the author",
            "a second human review is required",
            "one-item WIP limit",
        ):
            self.assertNotIn(blanket_gate, text)

    def test_source_module_matches_selector_bundle(self) -> None:
        source = self.repo_root / "modules" / "collaborative-development-workflow.md"
        bundled = (
            self.bundle_root
            / "modules"
            / "collaborative-development-workflow.md"
        )
        self.assertEqual(source.read_bytes(), bundled.read_bytes())


if __name__ == "__main__":
    unittest.main()
