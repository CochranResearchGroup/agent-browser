# Local validation batches

Use `pnpm validation:select -- --base <ref>` to identify relevant checks, then review their scope and effects. To run independent checks without losing an earlier failure behind a later success, put each command and its arguments in a JSON plan:

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
