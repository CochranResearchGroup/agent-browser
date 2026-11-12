# Exact lost-task retirement source qualification

Issues: agent-browser650 and auth-receipts339.

A lost browser is not proof of page disposal. The owning service freezes exact
published task/browser/profile/selection/process/control custody, qualifies
cross-boot absence and current profile ownership, and commits local retirement
plus a durable operation receipt atomically. No OS signal, provider cleanup or
credential authority follows. Unknown replies permit original receipt lookup
only; a missing or ambiguous witness remains held.

The source integrates mode/handoff main276d58c and its documentation-only
successor250c36dc. Combined source gates pass: formatting; workspace Clippy with
warnings denied; all423 service-model tests; ten owner/MCP tests. Compiler and
test errors, warnings and failed tests are zero. The later current-main delta
changes only RUNBOOK and the interrupted-close restoration plan; executable
source and Cargo.lock remain byte-identical to the tested combined source.

Commands use the repository's cargo-safe admission wrapper:

```sh
scripts/ci/cargo-safe.sh fmt --all --manifest-path Cargo.toml -- --check
scripts/ci/cargo-safe.sh clippy --workspace --manifest-path Cargo.toml -- -D warnings
scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser-service-model -- --test-threads=1
scripts/ci/cargo-safe.sh test --manifest-path Cargo.toml -p agent-browser --bin agent-browser lost_task_retirement -- --test-threads=1
```

Earlier exact-code synthetic owner/SQLite/kernel/MCP and actual Auth transport
composition qualified durable restart/readback and one send after lost reply.
Independent CI-profile packaging qualified real dashboard assets and workstation,
Guacamole, durability, route, docs and no-launch fixtures. The installer failure
fixture now checks the exact transaction introduced by each injected failure,
retaining its rollback and terminal-state assertions. Raw failed evidence stays
retained outside Git.

These results qualify source review, not installed or provider acceptance. A
later actual CI-candidate retirement was held by kernel evidence; its unknown
intent remains preserved. A redacted host census observed an unrelated live
browser without a canonical profile path, a sufficient reason to hold. Do not
weaken that interlock, kill unrelated owners, replay an uncertain operation, or
claim an exact failure-line cause from that census alone. Production-profile
build, exact installed identity and real owner acceptance remain separate gates.
Local validation replaces hosted CI under the operator's no-paid-CI policy.
