# P225: Native launch preflight

Product lane: PL-BUGFIX
Status: COMPLETED (source repair)
Work item: https://github.com/CochranResearchGroup/agent-browser/issues/227
Owner: ecochran76, implemented by Codex
Branch: fix/default-launch-preflight
Checkout: agent-browser-p204, clean integrated checkout reused after fresh origin/main and six-checkout inventory. Retain the prior sealed candidate.

## Outcome and seams

Reject an invalid relative profile directory or insufficient local capacity before
recording durable native cold-launch custody. Return the specific safe preflight
error, and allow a corrected request to retry without clearing a claim. Validate
through the public launch coordinator with the real BrowserManagerRuntime adapter,
and through consumer effects for error propagation. Existing process outcomes
remain conservative once custody is admitted.

## Boundaries

No production publication, reboot, profile reset, authentication edits or speculative
clearing of the historical Default claim. That cold claim lacks an exact retained
browser and canonical profile path; preserve it unless supported reconciliation
can prove absence. Existing absolute profile normalization remains at ingress;
never guess a replacement directory for a relative legacy catalog entry.
P225 is primary writer for cold-launch preflight, runtime adapter and regression
fixtures. Completed P224 is the predecessor; draft PRs 191/184 and other checkouts
remain preserved. No new checkout is created.

## Validation

Run red/green native coordinator tests, Service Model coordinator and consumer
contracts, native runtime tests, format and strict workspace Clippy. Review the
published diff and retain exact validation receipts. Source acceptance does not
prove installed recovery of Default.

## Validation and review receipt

Baseline: `7f27ba5954a79506be1eef613a059a40a8f6824a`.
The real-adapter regression failed at durable admission before the fix, then
passed. The same-host retry regression separately failed because the consumer
replaced the capacity reason with `remote_view_session_launch_readback_required`;
the corrected consumer passed without manual state changes.

Commands, using `AGENT_BROWSER_CARGO_CACHE=off` after a stalled compiler-cache
version probe was terminated before tests:

- `scripts/ci/rust-tests.sh --focused native_launch_preflight_rejects_relative_profile_before_custody`: red then green; final coverage includes relative path and zero browser capacity through the real adapter.
- `scripts/ci/rust-tests.sh --focused consumer_preflight_failure_preserves_reason_and_allows_same_host_retry`: red; final green is included in the following two runs.
- `scripts/ci/rust-tests.sh --focused preflight`: 16 passed.
- `scripts/ci/rust-tests.sh --focused browser_session`: 68 passed, one existing real-Chrome test ignored.
- `scripts/ci/cargo-safe.sh test -p agent-browser-service-model --manifest-path Cargo.toml -- --test-threads=1`: full suite passed; existing unused-import warning in an unrelated manager fixture remains.
- `scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check`: passed.
- `scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings`: passed.
- `pnpm test:remote-view-handoff-docs`, `pnpm --dir docs build`, `git diff --check`: passed. Docs build retains the existing multiple-lockfile warning.

Standards review and independent Spec-axis review in the primary context found
no remaining findings. Validation selection also recommended workstation and
legacy-provider fixtures by broad path matching; no installer, workstation or
provider surface changed, so these were excluded. Shared installed guidance is
preserved because this is source-only acceptance, not runtime publication.

The historical capitalized Default claim is unchanged. Supported reconciliation
requires a retained browser and a prior published assignment; this cold claim
has neither. Missing observation alone cannot establish process absence. The
lowercase default entry already has an absolute managed directory. Do not alias
these entries or guess an intended directory. No production mutation, reboot,
real browser launch or authentication operation was performed by this packet.
The prior sealed candidate remains pinned in the reused checkout.
