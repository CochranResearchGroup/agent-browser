from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "scripts" / "audit_active_lanes.py"


class ActiveLaneAuditTests(unittest.TestCase):
    def git(self, repo: Path, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(repo), *args],
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout.strip()

    def write(self, path: Path, body: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8")

    def make_registered_active_repo(self) -> tuple[Path, tempfile.TemporaryDirectory[str]]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        remote = root / "remote.git"
        repo = root / "repo"
        worktree = root / "lane-p42"

        subprocess.run(["git", "init", "--bare", "-q", str(remote)], check=True)
        subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
        self.git(repo, "config", "user.email", "test@example.com")
        self.git(repo, "config", "user.name", "Test")
        self.git(repo, "remote", "add", "origin", str(remote))

        self.write(repo / "README.md", "fixture\n")
        self.git(repo, "add", "README.md")
        self.git(repo, "commit", "-q", "-m", "Initialize fixture")
        self.git(repo, "branch", "feature/p42-carrier-reconciliation")
        self.git(repo, "worktree", "add", "-q", str(worktree), "feature/p42-carrier-reconciliation")

        plan_path = "docs/dev/plans/0042-2026-08-20-carrier-reconciliation.md"
        self.write(
            worktree / plan_path,
            """# Plan 0042 | Carrier Reconciliation

State: OPEN
Lane: P42
Branch: feature/p42-carrier-reconciliation
Target: main
Integration: merge

## Current State

Implementation is active.
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Open carrier lane")
        checkpoint = self.git(worktree, "rev-parse", "HEAD")

        self.write(
            repo / "docs/dev/active-lanes.yaml",
            f"""schema_version: 1
lanes:
  - id: P42
    objective: Carrier reconciliation
    plan: {plan_path}
    plan_ref: refs/heads/feature/p42-carrier-reconciliation
    branch: feature/p42-carrier-reconciliation
    target: main
    plan_state: OPEN
    custody_state: ACTIVE_WORKTREE
    checkpoint: {checkpoint}
    remote_ref: refs/remotes/origin/feature/p42-carrier-reconciliation
    integration: merge
    dependencies: []
    overlaps: []
    updated_at: 2026-08-20
""",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Register carrier lane")
        self.git(repo, "push", "-q", "-u", "origin", "main")
        self.git(worktree, "push", "-q", "-u", "origin", "feature/p42-carrier-reconciliation")
        return repo, temporary

    def run_audit(self, repo: Path, *extra_args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--repo-root",
                str(repo),
                "--default-ref",
                "refs/heads/main",
                "--json",
                *extra_args,
            ],
            capture_output=True,
            text=True,
        )

    def test_registered_pushed_lane_with_worktree_is_active(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["ok"], report["problems"])
        self.assertEqual(report["default_ref"], "refs/heads/main")
        self.assertEqual(len(report["lanes"]), 1)
        lane = report["lanes"][0]
        self.assertEqual(lane["id"], "P42")
        self.assertIn("registered_active", lane["findings"])
        self.assertEqual(lane["local_tip"], lane["remote_tip"])
        self.assertEqual(lane["local_remote_relation"], "equal")
        self.assertEqual(lane["checkpoint"], lane["local_tip"])
        self.assertEqual(len(lane["worktrees"]), 1)

    def test_duplicate_lane_ids_fail_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            + """  - id: P42
    objective: Duplicate carrier lane
    plan: docs/dev/plans/duplicate.md
    plan_ref: refs/heads/feature/duplicate
    branch: feature/duplicate
    target: main
    plan_state: OPEN
    custody_state: ACTIVE_WORKTREE
    checkpoint: deadbeef
    remote_ref: refs/remotes/origin/feature/duplicate
    integration: merge
    dependencies: []
    overlaps: []
    updated_at: 2026-08-20
""",
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Duplicate lane fixture")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertFalse(report["ok"])
        self.assertIn("duplicate lane id: P42", report["problems"])

    def test_local_only_lane_is_not_remotely_custodied(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        self.git(repo, "push", "-q", "origin", "--delete", "feature/p42-carrier-reconciliation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("registered_active", lane["findings"])
        self.assertIn("local_only", lane["findings"])
        self.assertIsNone(lane["remote_tip"])
        self.assertIn("P42: local branch has no configured remote custody", report["problems"])

    def test_dirty_worktree_is_uncheckpointed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.write(worktree / "untracked.txt", "not committed\n")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("dirty_uncheckpointed", lane["findings"])
        self.assertEqual(lane["worktrees"][0]["status"], ["?? untracked.txt"])
        self.assertIn("P42: assigned worktree has uncommitted state", report["problems"])

    def test_newer_pushed_tip_makes_catalog_checkpoint_stale(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.write(worktree / "next.txt", "new checkpoint\n")
        self.git(worktree, "add", "next.txt")
        self.git(worktree, "commit", "-q", "-m", "Advance carrier lane")
        self.git(worktree, "push", "-q", "origin", "feature/p42-carrier-reconciliation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("stale_checkpoint", lane["findings"])
        self.assertEqual(lane["local_tip"], lane["remote_tip"])
        self.assertNotEqual(lane["checkpoint"], lane["local_tip"])
        self.assertIn("P42: catalog checkpoint does not match the local branch tip", report["problems"])

    def test_active_local_checkpoint_ahead_of_remote_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.write(worktree / "local-only.txt", "unpublished checkpoint\n")
        self.git(worktree, "add", "local-only.txt")
        self.git(worktree, "commit", "-q", "-m", "Advance local checkpoint")
        checkpoint = self.git(worktree, "rev-parse", "HEAD")
        catalog = repo / "docs/dev/active-lanes.yaml"
        body = catalog.read_text(encoding="utf-8")
        body = re.sub(
            r"(?m)^    checkpoint: .+$",
            f"    checkpoint: {checkpoint}",
            body,
        )
        catalog.write_text(body, encoding="utf-8")
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Record local checkpoint")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("local_ahead_of_remote", lane["findings"])
        self.assertEqual(lane["local_remote_relation"], "local_ahead")
        self.assertEqual(lane["checkpoint"], lane["local_tip"])
        self.assertNotEqual(lane["local_tip"], lane["remote_tip"])
        self.assertIn("P42: active local checkpoint is ahead of remote custody", report["problems"])

    def test_active_remote_tip_ahead_of_local_checkpoint_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        writer = repo.parent / "remote-writer"
        remote = self.git(repo, "remote", "get-url", "origin")
        subprocess.run(["git", "clone", "-q", remote, str(writer)], check=True)
        self.git(writer, "config", "user.email", "test@example.com")
        self.git(writer, "config", "user.name", "Test")
        self.git(writer, "checkout", "-q", "feature/p42-carrier-reconciliation")
        self.write(writer / "remote-only.txt", "remote checkpoint\n")
        self.git(writer, "add", "remote-only.txt")
        self.git(writer, "commit", "-q", "-m", "Advance remote checkpoint")
        self.git(writer, "push", "-q", "origin", "feature/p42-carrier-reconciliation")
        self.git(repo, "fetch", "-q", "origin", "feature/p42-carrier-reconciliation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("remote_ahead_of_local", lane["findings"])
        self.assertEqual(lane["local_remote_relation"], "remote_ahead")
        self.assertEqual(lane["checkpoint"], lane["local_tip"])
        self.assertNotEqual(lane["local_tip"], lane["remote_tip"])
        self.assertIn("P42: active remote custody is ahead of the local checkpoint", report["problems"])

    def test_active_local_and_remote_divergence_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.write(worktree / "local-only.txt", "local checkpoint\n")
        self.git(worktree, "add", "local-only.txt")
        self.git(worktree, "commit", "-q", "-m", "Advance local checkpoint")
        checkpoint = self.git(worktree, "rev-parse", "HEAD")
        catalog = repo / "docs/dev/active-lanes.yaml"
        body = re.sub(
            r"(?m)^    checkpoint: .+$",
            f"    checkpoint: {checkpoint}",
            catalog.read_text(encoding="utf-8"),
        )
        catalog.write_text(body, encoding="utf-8")
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Record local checkpoint")

        writer = repo.parent / "remote-writer"
        remote = self.git(repo, "remote", "get-url", "origin")
        subprocess.run(["git", "clone", "-q", remote, str(writer)], check=True)
        self.git(writer, "config", "user.email", "test@example.com")
        self.git(writer, "config", "user.name", "Test")
        self.git(writer, "checkout", "-q", "feature/p42-carrier-reconciliation")
        self.write(writer / "remote-only.txt", "remote checkpoint\n")
        self.git(writer, "add", "remote-only.txt")
        self.git(writer, "commit", "-q", "-m", "Advance remote checkpoint")
        self.git(writer, "push", "-q", "origin", "feature/p42-carrier-reconciliation")
        self.git(repo, "fetch", "-q", "origin", "feature/p42-carrier-reconciliation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("local_remote_diverged", lane["findings"])
        self.assertEqual(lane["local_remote_relation"], "diverged")
        self.assertEqual(lane["checkpoint"], lane["local_tip"])
        self.assertNotEqual(lane["local_tip"], lane["remote_tip"])
        self.assertIn("P42: active local and remote custody have diverged", report["problems"])

    def test_active_worktree_registration_without_worktree_is_missing(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.git(repo, "worktree", "remove", str(worktree))

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("registered_but_missing", lane["findings"])
        self.assertEqual(lane["worktrees"], [])
        self.assertIn("P42: ACTIVE_WORKTREE lane has no assigned worktree", report["problems"])

    def test_pushed_branch_without_worktree_can_be_paused(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.git(repo, "worktree", "remove", str(worktree))
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "custody_state: ACTIVE_WORKTREE", "custody_state: PAUSED_REF"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Pause carrier lane")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertEqual(lane["findings"], ["paused_ref"])
        self.assertEqual(lane["worktrees"], [])

    def test_open_plan_on_unregistered_topic_branch_is_discovered(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        root = repo.parent
        worktree = root / "lane-p43"
        self.git(repo, "worktree", "add", "-q", "-b", "feature/p43-unregistered", str(worktree), "main")
        plan_path = "docs/dev/plans/0043-2026-08-20-unregistered.md"
        self.write(
            worktree / plan_path,
            """# Plan 0043 | Unregistered

State: OPEN
Lane: P43
Branch: feature/p43-unregistered
Target: main
Integration: merge

## Current State

Implementation is active.
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Open unregistered lane")
        self.git(worktree, "push", "-q", "-u", "origin", "feature/p43-unregistered")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = next(item for item in report["lanes"] if item["id"] == "P43")
        self.assertEqual(lane["branch"], "feature/p43-unregistered")
        self.assertIn("unregistered_active", lane["findings"])
        self.assertIn("P43: active branch is absent from the default-ref catalog", report["problems"])

    def test_catalog_only_mode_skips_unregistered_branch_discovery(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = repo.parent / "lane-p43"
        self.git(repo, "worktree", "add", "-q", "-b", "feature/p43-unregistered", str(worktree), "main")
        plan_path = "docs/dev/plans/0043-2026-08-21-unregistered.md"
        self.write(
            worktree / plan_path,
            """# Plan 0043 | Unregistered

State: OPEN
Lane: P43
Branch: feature/p43-unregistered
Target: main
Integration: merge

## Current State

Implementation is active.
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Open unregistered lane")
        self.git(worktree, "push", "-q", "-u", "origin", "feature/p43-unregistered")

        result = self.run_audit(repo, "--catalog-only")

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertTrue(report["ok"], report["problems"])
        self.assertEqual(report["discovery_mode"], "catalog-only")
        self.assertEqual(report["branch_prefixes"], [])
        self.assertEqual([lane["id"] for lane in report["lanes"]], ["P42"])

    def test_catalog_only_mode_skips_unregistered_detached_discovery(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = repo.parent / "lane-p44-detached"
        self.git(repo, "worktree", "add", "-q", "--detach", str(worktree), "main")
        plan_path = "docs/dev/plans/0044-2026-08-21-detached.md"
        self.write(
            worktree / plan_path,
            """# Plan 0044 | Detached

State: OPEN
Lane: P44
Branch: feature/p44-detached
Target: main
Integration: merge

## Current State

Implementation exists only at detached HEAD.
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Create detached work")

        results = (
            self.run_audit(repo, "--catalog-only"),
            self.run_audit(repo, "--branch", "feature/p42-carrier-reconciliation"),
        )

        for result in results:
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(result.stdout)
            self.assertTrue(report["ok"], report["problems"])
            self.assertEqual([lane["id"] for lane in report["lanes"]], ["P42"])

    def test_detached_open_plan_without_shared_ref_is_orphaned(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = repo.parent / "lane-p44-detached"
        self.git(repo, "worktree", "add", "-q", "--detach", str(worktree), "main")
        plan_path = "docs/dev/plans/0044-2026-08-20-detached.md"
        self.write(
            worktree / plan_path,
            """# Plan 0044 | Detached

State: OPEN
Lane: P44
Branch: feature/p44-detached
Target: main
Integration: merge

## Current State

Implementation exists only at detached HEAD.
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Create detached work")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = next(item for item in report["lanes"] if item["id"] == "P44")
        self.assertIn("orphaned_detached", lane["findings"])
        self.assertEqual(lane["worktrees"][0]["detached"], True)
        self.assertIn("P44: detached worktree commit is not reachable from a shared ref", report["problems"])

    def test_declared_overlap_without_disposition_is_unreconciled(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "overlaps: []", "overlaps: [P43]\n    reconciled_overlaps: []"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Declare unresolved overlap")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("overlap_unreconciled", lane["findings"])
        self.assertEqual(lane["unreconciled_overlaps"], ["P43"])
        self.assertIn("P42: declared overlap lacks disposition: P43", report["problems"])

    def test_validated_published_lane_can_be_integration_ready(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        body = catalog.read_text(encoding="utf-8")
        checkpoint = next(
            line.split(":", 1)[1].strip()
            for line in body.splitlines()
            if line.strip().startswith("checkpoint:")
        )
        body = body.replace("custody_state: ACTIVE_WORKTREE", "custody_state: INTEGRATION_READY")
        body = body.replace(
            "integration: merge",
            f"integration: merge\n    validation_status: passed\n    validation_ref: {checkpoint}",
        )
        catalog.write_text(body, encoding="utf-8")
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Mark carrier lane ready")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertEqual(lane["findings"], ["integration_ready"])
        self.assertFalse(lane["integrated_into_target"])

    def test_integrated_state_without_ancestry_or_receipt_is_ambiguous(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            .replace("plan_state: OPEN", "plan_state: CLOSED")
            .replace("custody_state: ACTIVE_WORKTREE", "custody_state: INTEGRATED"),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Claim ambiguous integration")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("integration_ambiguous", lane["findings"])
        self.assertFalse(lane["integrated_into_target"])
        self.assertIn("P42: INTEGRATED state lacks ancestry or a verified integration receipt", report["problems"])

    def test_merged_lane_with_live_branch_is_cleanup_pending(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        self.git(
            repo,
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Integrate carrier lane",
            "feature/p42-carrier-reconciliation",
        )
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            .replace("plan_state: OPEN", "plan_state: CLOSED")
            .replace("custody_state: ACTIVE_WORKTREE", "custody_state: INTEGRATED"),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Record carrier integration")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertEqual(lane["findings"], ["integrated_cleanup_pending"])
        self.assertTrue(lane["integrated_into_target"])

    def test_squash_receipt_can_prove_integration_without_branch_ancestry(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        self.write(repo / "carrier-result.txt", "squashed result\n")
        self.git(repo, "add", "carrier-result.txt")
        self.git(repo, "commit", "-q", "-m", "Squash carrier result")
        receipt = self.git(repo, "rev-parse", "HEAD")
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            .replace("plan_state: OPEN", "plan_state: CLOSED")
            .replace("custody_state: ACTIVE_WORKTREE", "custody_state: INTEGRATED")
            .replace("integration: merge", f"integration: squash\n    integration_receipt: {receipt}"),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Record squash integration")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertFalse(lane["integrated_into_target"])
        self.assertTrue(lane["integration_receipt_verified"])
        self.assertIn("integrated_cleanup_pending", lane["findings"])

    def test_archive_refs_preserve_unmerged_lane_without_worktree(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        checkpoint = self.git(worktree, "rev-parse", "HEAD")
        archive_branch = "archive/p42-carrier-20260820"
        self.git(repo, "branch", archive_branch, checkpoint)
        self.git(repo, "push", "-q", "-u", "origin", archive_branch)
        self.git(repo, "worktree", "remove", str(worktree))
        self.git(repo, "branch", "-d", "feature/p42-carrier-reconciliation")
        self.git(repo, "push", "-q", "origin", "--delete", "feature/p42-carrier-reconciliation")
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            .replace("plan_state: OPEN", "plan_state: CANCELLED")
            .replace("custody_state: ACTIVE_WORKTREE", "custody_state: ARCHIVED")
            .replace(
                "integration: merge",
                "integration: merge\n"
                f"    archive_ref: refs/heads/{archive_branch}\n"
                f"    archive_remote_ref: refs/remotes/origin/{archive_branch}",
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Archive carrier lane")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertEqual(lane["findings"], ["archived_ref"])
        self.assertEqual(lane["archive_local_tip"], checkpoint)
        self.assertEqual(lane["archive_remote_tip"], checkpoint)

    def test_closed_plan_with_active_worktree_is_inconsistent(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace("plan_state: OPEN", "plan_state: CLOSED"),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Close plan without Git disposition")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("closed_plan_live_branch", lane["findings"])
        self.assertIn("P42: CLOSED plan still has active worktree custody", report["problems"])

    def test_missing_required_catalog_field_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "    objective: Carrier reconciliation\n", ""
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Remove required lane field")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertIn("P42: missing required catalog field: objective", report["problems"])

    def test_two_lanes_cannot_claim_the_same_branch(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8")
            + """  - id: P43
    objective: Conflicting owner
    plan: docs/dev/plans/0043-conflict.md
    plan_ref: refs/heads/feature/p42-carrier-reconciliation
    branch: feature/p42-carrier-reconciliation
    target: main
    plan_state: OPEN
    custody_state: ACTIVE_WORKTREE
    checkpoint: deadbeef
    remote_ref: refs/remotes/origin/feature/p42-carrier-reconciliation
    integration: merge
    dependencies: []
    overlaps: []
    updated_at: 2026-08-20
""",
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Create conflicting branch ownership")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertIn(
            "branch claimed by multiple lanes: feature/p42-carrier-reconciliation",
            report["problems"],
        )

    def test_registered_plan_metadata_must_match_catalog(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "plan_state: OPEN", "plan_state: BLOCKED"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Drift plan state in catalog")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("plan_catalog_drift", lane["findings"])
        self.assertIn("P42: plan state OPEN does not match catalog state BLOCKED", report["problems"])

    def test_unknown_dependency_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "dependencies: []", "dependencies: [P99]"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Reference unknown dependency")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertIn("P42: unknown dependency lane: P99", report["problems"])

    def test_catalog_branch_that_does_not_exist_is_missing(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = Path(self.git(repo, "worktree", "list", "--porcelain").split("worktree ")[2].splitlines()[0])
        self.git(repo, "worktree", "remove", str(worktree))
        self.git(repo, "branch", "-D", "feature/p42-carrier-reconciliation")
        self.git(repo, "push", "-q", "origin", "--delete", "feature/p42-carrier-reconciliation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("registered_but_missing", lane["findings"])
        self.assertIn("P42: registered branch has no local or remote ref", report["problems"])

    def test_audit_does_not_change_refs_worktrees_status_config_or_index(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)

        def fingerprint() -> tuple[str, str, tuple[tuple[str, str, bytes], ...], bytes]:
            worktree_rows: list[tuple[str, str, bytes]] = []
            worktree_output = self.git(repo, "worktree", "list", "--porcelain")
            for line in worktree_output.splitlines():
                if not line.startswith("worktree "):
                    continue
                worktree = Path(line.removeprefix("worktree "))
                git_dir = Path(self.git(worktree, "rev-parse", "--absolute-git-dir"))
                worktree_rows.append(
                    (
                        str(worktree),
                        self.git(worktree, "status", "--porcelain=v1", "--untracked-files=all"),
                        (git_dir / "index").read_bytes(),
                    )
                )
            return (
                self.git(repo, "show-ref"),
                worktree_output,
                tuple(worktree_rows),
                (repo / ".git/config").read_bytes(),
            )

        before = fingerprint()
        results = (
            self.run_audit(repo),
            self.run_audit(repo, "--catalog-only"),
            self.run_audit(repo, "--branch", "feature/p42-carrier-reconciliation"),
        )
        after = fingerprint()

        for result in results:
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(after, before)

    def test_invalid_catalog_syntax_returns_stable_json_error(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text("schema_version: 1\nlanes:\n  - not-a-mapping\n", encoding="utf-8")
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Break lane catalog syntax")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertEqual(report["problems"], ["invalid catalog syntax at line 3: expected key: value"])

    def test_unknown_state_vocabulary_fails_closed(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "custody_state: ACTIVE_WORKTREE", "custody_state: MAYBE_DONE"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Use unknown custody state")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertIn("P42: invalid custody_state: MAYBE_DONE", report["problems"])

    def test_integration_ready_state_requires_complete_readiness_evidence(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "custody_state: ACTIVE_WORKTREE", "custody_state: INTEGRATION_READY"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Claim readiness without validation")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = report["lanes"][0]
        self.assertIn("integration_ambiguous", lane["findings"])
        self.assertIn("P42: INTEGRATION_READY state lacks complete readiness evidence", report["problems"])

    def test_backup_namespace_is_excluded_from_unregistered_discovery(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = repo.parent / "backup-old-lane"
        self.git(repo, "worktree", "add", "-q", "-b", "backup/old-lane", str(worktree), "main")
        plan_path = "docs/dev/plans/0099-2026-01-01-old-lane.md"
        self.write(
            worktree / plan_path,
            """# Plan 0099 | Old Lane

State: OPEN
Lane: P99
Branch: backup/old-lane
Target: main
Integration: merge
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Preserve historical backup plan")
        self.git(worktree, "push", "-q", "-u", "origin", "backup/old-lane")

        result = self.run_audit(repo)

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual([lane["id"] for lane in report["lanes"]], ["P42"])

    def test_custom_topic_prefix_can_expand_bounded_discovery(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        worktree = repo.parent / "topic-p43"
        self.git(repo, "worktree", "add", "-q", "-b", "topic/p43", str(worktree), "main")
        plan_path = "docs/dev/plans/0043-2026-08-20-custom-topic.md"
        self.write(
            worktree / plan_path,
            """# Plan 0043 | Custom Topic

State: OPEN
Lane: P43
Branch: topic/p43
Target: main
Integration: merge
""",
        )
        self.git(worktree, "add", plan_path)
        self.git(worktree, "commit", "-q", "-m", "Open custom topic lane")
        self.git(worktree, "push", "-q", "-u", "origin", "topic/p43")

        result = self.run_audit(repo, "--branch-prefix", "topic/")

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        lane = next(item for item in report["lanes"] if item["id"] == "P43")
        self.assertIn("unregistered_active", lane["findings"])

    def test_exact_branch_selection_excludes_other_unregistered_branches(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)

        for lane_id in ("P43", "P44"):
            branch = f"feature/{lane_id.lower()}-unregistered"
            worktree = repo.parent / f"lane-{lane_id.lower()}"
            self.git(repo, "worktree", "add", "-q", "-b", branch, str(worktree), "main")
            plan_path = f"docs/dev/plans/00{lane_id[1:]}-2026-08-21-unregistered.md"
            self.write(
                worktree / plan_path,
                f"""# Plan 00{lane_id[1:]} | Unregistered

State: OPEN
Lane: {lane_id}
Branch: {branch}
Target: main
Integration: merge

## Current State

Implementation is active.
""",
            )
            self.git(worktree, "add", plan_path)
            self.git(worktree, "commit", "-q", "-m", f"Open {lane_id} lane")
            self.git(worktree, "push", "-q", "-u", "origin", branch)

        result = self.run_audit(repo, "--branch", "feature/p43-unregistered")

        self.assertEqual(result.returncode, 1)
        report = json.loads(result.stdout)
        self.assertEqual(report["discovery_mode"], "exact-branches")
        self.assertEqual(report["selected_branches"], ["feature/p43-unregistered"])
        self.assertEqual(report["branch_prefixes"], [])
        self.assertEqual([lane["id"] for lane in report["lanes"]], ["P42", "P43"])

    def test_discovery_modes_are_mutually_exclusive(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)

        result = self.run_audit(
            repo,
            "--catalog-only",
            "--branch",
            "feature/p42-carrier-reconciliation",
        )

        self.assertEqual(result.returncode, 2)
        self.assertIn("not allowed with argument --catalog-only", result.stderr)

    def test_configured_remote_bounds_remote_ref_discovery(self) -> None:
        repo, temporary = self.make_registered_active_repo()
        self.addCleanup(temporary.cleanup)
        self.git(repo, "remote", "rename", "origin", "upstream")
        catalog = repo / "docs/dev/active-lanes.yaml"
        catalog.write_text(
            catalog.read_text(encoding="utf-8").replace(
                "refs/remotes/origin/", "refs/remotes/upstream/"
            ),
            encoding="utf-8",
        )
        self.git(repo, "add", "docs/dev/active-lanes.yaml")
        self.git(repo, "commit", "-q", "-m", "Configure alternate custody remote")

        result = self.run_audit(repo, "--remote", "upstream")

        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["remote"], "upstream")
        self.assertEqual(report["lanes"][0]["local_tip"], report["lanes"][0]["remote_tip"])


if __name__ == "__main__":
    unittest.main()
