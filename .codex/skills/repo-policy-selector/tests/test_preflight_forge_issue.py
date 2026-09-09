import importlib.util
import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch


SCRIPT_PATH = Path(__file__).resolve().parents[1] / "scripts" / "preflight_forge_issue.py"


def load_module():
    spec = importlib.util.spec_from_file_location("preflight_forge_issue", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ForgeIssuePreflightTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.module = load_module()

    def github_target(self):
        return {
            "id": "odollo",
            "forge": "github",
            "host": "github.com",
            "repository": "example/odollo",
            "relationship": "owned",
            "allowed_actions": ["read", "create", "comment", "apply_labels", "close"],
            "security_route": "private_vulnerability_reporting",
            "label_map": {
                "intent/defect": {"provider_label": "bug"},
                "priority/high": {"provider_label": "priority: high"},
            },
        }

    def github_snapshot(self):
        return {
            "forge": "github",
            "host": "github.com",
            "repository": "Example/Odollo",
            "authenticated_actor": "operator",
            "effective_role": "TRIAGE",
            "issues_enabled": True,
            "archived": False,
            "available_labels": ["bug", "priority: high"],
            "duplicate_candidates": [],
            "source": "fixture",
        }

    def test_owned_github_create_with_governed_labels_passes(self):
        result = self.module.evaluate(
            self.github_target(),
            self.github_snapshot(),
            action="create",
            label_intents=["intent/defect", "priority/high"],
            report_kind="defect",
            idempotency_key="odollo:duplicate-order:v1",
        )

        self.assertTrue(result["ok"], result)
        self.assertEqual(result["resolved_labels"], ["bug", "priority: high"])
        self.assertEqual(
            result["label_resolution"],
            [
                {"intent": "intent/defect", "provider_label": "bug"},
                {"intent": "priority/high", "provider_label": "priority: high"},
            ],
        )
        self.assertEqual(result["required_actions"], ["create", "apply_labels"])
        self.assertFalse(result["operator_authority_verified"])
        self.assertFalse(result["mutation_authorized"])

    def test_unknown_label_intent_fails_closed(self):
        result = self.module.evaluate(
            self.github_target(),
            self.github_snapshot(),
            action="create",
            label_intents=["priority/emergency"],
            report_kind="defect",
            idempotency_key="odollo:duplicate-order:v1",
        )

        self.assertFalse(result["ok"])
        self.assertIn("unknown normalized label intent: priority/emergency", result["problems"])

    def test_missing_provider_label_is_not_created_during_preflight(self):
        snapshot = self.github_snapshot()
        snapshot["available_labels"] = ["bug"]
        result = self.module.evaluate(
            self.github_target(),
            snapshot,
            action="create",
            label_intents=["priority/high"],
            report_kind="defect",
            idempotency_key="odollo:duplicate-order:v1",
        )

        self.assertFalse(result["ok"])
        self.assertIn("mapped provider label does not exist: priority: high", result["problems"])

    def test_label_application_requires_separate_allowlist_and_role(self):
        target = self.github_target()
        target["allowed_actions"] = ["read", "create"]
        snapshot = self.github_snapshot()
        snapshot["effective_role"] = "READ"
        result = self.module.evaluate(
            target,
            snapshot,
            action="create",
            label_intents=["intent/defect"],
            report_kind="defect",
            idempotency_key="odollo:duplicate-order:v1",
        )

        self.assertFalse(result["ok"])
        self.assertIn("action is not allowlisted: apply_labels", result["problems"])
        self.assertIn("insufficient provider role for apply_labels: READ < TRIAGE", result["problems"])

    def test_duplicate_candidate_blocks_create(self):
        snapshot = self.github_snapshot()
        snapshot["duplicate_candidates"] = [{"number": 42, "title": "Existing"}]
        result = self.module.evaluate(
            self.github_target(),
            snapshot,
            action="create",
            label_intents=[],
            report_kind="defect",
            idempotency_key="odollo:duplicate-order:v1",
        )

        self.assertFalse(result["ok"])
        self.assertIn("duplicate candidates found for the idempotency key", result["problems"])

    def test_target_drift_archive_and_disabled_issues_fail_closed(self):
        snapshot = self.github_snapshot()
        snapshot["host"] = "github.example.com"
        snapshot["repository"] = "example/other"
        snapshot["archived"] = True
        snapshot["issues_enabled"] = False
        result = self.module.evaluate(
            self.github_target(),
            snapshot,
            action="read",
            label_intents=[],
            report_kind="governance",
            idempotency_key=None,
        )

        self.assertFalse(result["ok"])
        self.assertIn("target drift: snapshot host does not match registry", result["problems"])
        self.assertIn("target drift: snapshot repository does not match registry", result["problems"])
        self.assertIn("target repository is archived or archive state is unknown", result["problems"])
        self.assertIn("target issue surface is disabled or unknown", result["problems"])

    def test_gitlab_nested_namespace_guest_can_create_with_existing_label(self):
        target = {
            "id": "litscout",
            "forge": "gitlab",
            "host": "gitlab.example.com",
            "repository": "research/tools/litscout",
            "relationship": "permissioned",
            "allowed_actions": ["read", "create", "comment", "apply_labels"],
            "security_route": "confidential_issue",
            "label_map": {"intent/defect": {"provider_label": "type::bug"}},
        }
        snapshot = {
            "forge": "gitlab",
            "host": "gitlab.example.com",
            "repository": "research/tools/litscout",
            "authenticated_actor": "operator",
            "effective_role": "GUEST",
            "issues_enabled": True,
            "archived": False,
            "available_labels": ["type::bug"],
            "duplicate_candidates": [],
            "source": "fixture",
        }
        result = self.module.evaluate(
            target,
            snapshot,
            action="create",
            label_intents=["intent/defect"],
            report_kind="defect",
            idempotency_key="litscout:parser:v1",
        )

        self.assertTrue(result["ok"], result)

        later_mutation = self.module.evaluate(
            target,
            snapshot,
            action="apply_labels",
            label_intents=["intent/defect"],
            report_kind="defect",
            idempotency_key=None,
        )
        self.assertFalse(later_mutation["ok"])
        self.assertIn(
            "insufficient provider role for apply_labels: GUEST < PLANNER",
            later_mutation["problems"],
        )

    def test_security_report_requires_private_route(self):
        target = self.github_target()
        target["security_route"] = "none"
        result = self.module.evaluate(
            target,
            self.github_snapshot(),
            action="create",
            label_intents=[],
            report_kind="security",
            idempotency_key="odollo:security:v1",
        )

        self.assertFalse(result["ok"])
        self.assertIn("security report has no approved private route", result["problems"])

    def test_registry_rejects_duplicate_ids_and_duplicate_provider_labels(self):
        target = self.github_target()
        duplicate = dict(target)
        duplicate["label_map"] = {
            "intent/defect": {"provider_label": "bug"},
            "intent/regression": {"provider_label": "bug"},
        }
        problems = self.module.validate_registry({"schema_version": 1, "targets": [target, duplicate]})

        self.assertIn("duplicate target id: odollo", problems)
        self.assertTrue(any("maps more than one intent to bug" in problem for problem in problems))

    def test_provider_api_timeout_fails_closed(self):
        expired = self.module.subprocess.TimeoutExpired(cmd=["glab"], timeout=20)
        with patch.object(self.module.subprocess, "run", side_effect=expired):
            with self.assertRaisesRegex(self.module.PreflightError, "provider command timed out"):
                self.module.run_json(["glab", "api", "user"])

    def test_cli_snapshot_mode_is_provider_free(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            registry_path = root / "targets.json"
            snapshot_path = root / "snapshot.json"
            registry_path.write_text(
                json.dumps({"schema_version": 1, "targets": [self.github_target()]}),
                encoding="utf-8",
            )
            snapshot_path.write_text(json.dumps(self.github_snapshot()), encoding="utf-8")

            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = self.module.main(
                    [
                        "--registry",
                        str(registry_path),
                        "--target",
                        "odollo",
                        "--action",
                        "create",
                        "--label",
                        "intent/defect",
                        "--idempotency-key",
                        "odollo:duplicate-order:v1",
                        "--snapshot",
                        str(snapshot_path),
                    ]
                )

        self.assertEqual(exit_code, 0)
        self.assertTrue(json.loads(output.getvalue())["ok"])


if __name__ == "__main__":
    unittest.main()
