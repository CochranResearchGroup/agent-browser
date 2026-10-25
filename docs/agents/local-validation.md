# Local validation batches

Use `pnpm validation:select -- --base <ref>` to identify relevant checks, then review their scope and effects.

For workspace polling, shared-cache/API, selected-context hook/model and their focused tests or startup fixture, the selector recommends the existing coordinator, mounted workspace-startup and selected-context checks. This includes test-only and fixture-only diffs. Run these focused checks before candidate builds or publication; the mounted suite exercises the real hook and cache together. Set `AGENT_BROWSER_TEST_CHROME` to the reviewed Chrome executable when the fixture default is unavailable. Recommendations do not execute checks or authorize runtime effects; CI remains subject to its existing operator-disabled policy.

To run independent checks without losing an earlier failure behind a later success, put each command and its arguments in a JSON plan:

```json
{
  "checks": [
    { "id": "patch", "command": "git", "args": ["diff", "--check"] },
    { "id": "policy", "command": "pnpm", "args": ["test:policy-wiring"], "timeoutMs": 120000 }
  ]
}
```

Save the plan outside the repository when it contains local paths. Run from the intended Git checkout:

```bash
pnpm validation:run --plan /tmp/checks.json --output-dir /tmp/check-results-new
```

The output directory must not exist and its parent must exist. Prior evidence is never overwritten. The runner executes checks serially without a shell and continues independent checks after failure. It prints one bounded status per check and the receipt path. Each private log contains that child's stdout and stderr; large output goes directly to disk. The receipt records starting Git HEAD/dirty status, working directory, argv, timestamps, duration, exit code, signal and launch/timeout error code. Dirty status binds only to the observed starting checkout; it is not a content hash or immutable candidate proof.

Only a terminal receipt with `state: "success"`, every check successful, and a successful runner exit is a passing batch. Nonzero exit, launch failure, signal or timeout fails the batch even when the last check passes. An interrupted runner may leave a running/pending receipt; that is incomplete evidence. Start a new output directory when rerunning. Optional `timeoutMs` is per check; omission imposes no runner timeout. Child stdin is closed, so checks must be noninteractive. Timeouts signal the direct child using Node's synchronous child-process behavior, not a whole process tree; use existing lifecycle-aware runners for browser/provider effects or jobs requiring descendant cleanup. A child that ignores termination can delay return.

Use direct executables and argument arrays, not a shell string with multiple checks. The runner cannot detect a failure hidden inside a caller-supplied shell or tool. Put dependent steps in their existing fail-fast runner rather than treating them as independent checks here. This command executes trusted caller-selected checks; it does not select sufficient validation or authorize their effects. Keep credentials out of plans/arguments and inspect logs before sharing. Nothing here activates CI or installs a runtime.

The existing Rust compartment runner and presubmit verifier retain their own failure semantics. `pnpm test:repository-tooling` includes the local runner's actual CLI regressions. Validate this tooling with `pnpm test:validation-runner` and, after selector/wiring changes, `pnpm test:validation-selection`.

## Selected planning completeness

For a current packet or retained completed outcome, check exact catalog lanes:

```bash
python3 .codex/skills/repo-policy-selector/scripts/audit_planning_contract.py \
  --repo-root . --lane P245 --lane P247 --json
```

This opt-in mode joins each selected catalog record to its on-disk plan header,
accountable owner and product lane. PLANNED, OPEN, BLOCKED and COMPLETED plans
must have actual Markdown links from ROADMAP, RUNBOOK and the plan index.
Corresponding notes use the plan's serial filename prefix; each must be linked
from the notes index and declare product lane, disposition and an owning plan
link or exact work-item locator. COMPLETED outcomes need at least one evidence
note. CLOSED and CANCELLED records retain identity checks without current
execution-link requirements. Completion never grants execution authority.

Unknown or duplicate selections in the catalog, missing artifacts and mismatched
metadata fail with exit code 1. Repeated lane arguments are deduplicated. Scope
and path overrides for legacy audit modes cannot be combined with this mode.
The existing active-only audit and its baseline remain separate, as does the
Git custody auditor. This check supports inline local Markdown links, not
reference-style links or semantic equivalence of narrative evidence. It does
not verify forge state or runtime health and is not automatically invoked by CI.

Run its disposable omission and CLI fixtures with:

```bash
python3 -m unittest discover \
  -s .codex/skills/repo-policy-selector/tests -p test_audit_planning_contract.py
```
