# Candidate 1 Service Request Normalization Test Receipt

Date: 2026-08-10

Role: distinct independent tester

Verdict: **PASS**

Candidate 1 has no scoped test failure. The repository Rust harness still has
one independently reproduced display-environment baseline failure in unchanged
`cli/src/native/cdp/chrome.rs`. That failure is not attributed to Candidate 1
and does not block this Candidate 1 verdict.

## Scope And Target Identity

The test scope is the 16-path Candidate 1 source, contract, documentation, and
generated-client packet, including the bounded Plan 0103 correction. Concurrent
dashboard work and other architecture packets are excluded from failure
attribution.

Base revision:

```text
ae36b272327982e3227f4dc7c5d6dc5b4b16350c
```

The path-sorted binary diff stream was built by applying
`git diff --binary --no-ext-diff <base> -- <path>` to tracked paths and
`git diff --no-index --binary /dev/null <path>` to new paths, in this order:

```text
README.md
cli/src/mcp.rs
cli/src/native/mod.rs
cli/src/native/service_access.rs
cli/src/native/service_request.rs
cli/src/native/stream/http.rs
cli/src/native/stream/mod.rs
cli/src/output.rs
docs/dev/contracts/service-request-field-roles.v1.json
docs/dev/contracts/service-request.v1.schema.json
docs/src/app/service-mode/page.mdx
packages/client/src/service-request.generated.d.ts
packages/client/src/service-request.generated.js
scripts/check-service-api-mcp-parity.js
scripts/test-service-request-client.js
skills/agent-browser/SKILL.md
```

Recomputed aggregate SHA-256:

```text
0e1827046633ce8f375597ecf5e78d0b5bc1ddbfbf98c9f5379386d3d8cad4bc
```

Per-path SHA-256 identity:

```text
55789153c162ffec0fddd52fa4e715db7dc12b06a8261e630b57324d7bd89768  README.md
5f8840b3188dc3826599c0e0a1c874e518b6a31ae1c99614a00c0b0ca1c05f05  cli/src/mcp.rs
4a3bbcc420b07d4041b6e381d6a671048406ebd49a7ff81a0e409eb1ca40a343  cli/src/native/mod.rs
99cbf7d4e2e4b16a328bf506be0066b57ac98be777bf6cf709a3cda503be221d  cli/src/native/service_access.rs
72470f46d78a6837a3041dd068737d6b1b312681484585c392494fa29f1d8f42  cli/src/native/service_request.rs
d1e1ae5840f6c0656e6d056e4651c8ec78a6ffb681d88ac38d16f001b27169e6  cli/src/native/stream/http.rs
4ffa83afd5cbb2ad6406d05750d19d48ad154b633923ab6fd02a9d51d7192078  cli/src/native/stream/mod.rs
ae730ef5e2158684222bd8495742528d06c623f6a07e608fbf2d34cca5ca5558  cli/src/output.rs
0e252f2856b7315525d1da40938c9840e76647ef43056e68596f348b8cc5cf35  docs/dev/contracts/service-request-field-roles.v1.json
bc6031676b781458c2864206c3f0c0a2698f69367c4af60700d339f278e3f01f  docs/dev/contracts/service-request.v1.schema.json
7408155957f707837881d5b9a264f9ef5b1d93e78eec3fb146ce0d57dc627359  docs/src/app/service-mode/page.mdx
09022afeefe5bdcf0f6e778e9c3ef1a1f78bb2f48f66ae4ad744656954295546  packages/client/src/service-request.generated.d.ts
5b1fde4109210f9c8b5b0222b988c4c967fb15630ff752c402e16b705d0d03d4  packages/client/src/service-request.generated.js
c090b475637f4ec20f5bd849ce9be2a972aa379bb6f5e4720cafe275c3e1db60  scripts/check-service-api-mcp-parity.js
38097fbc2f09a651b52117a6d4069c20f1ff7e01272a3abf34043d67e87ee299  scripts/test-service-request-client.js
05d45f8b5189f99195a0b4b4e865e27022320095b3846a9e052f92c4515b856c  skills/agent-browser/SKILL.md
```

## Focused Rust Results

### Service request

```bash
cargo test --manifest-path cli/Cargo.toml service_request -- --test-threads=1
```

Result: 35 passed, 0 failed, 0 ignored, 1,784 filtered out.

### Service access plan

```bash
cargo test --manifest-path cli/Cargo.toml service_access_plan -- --test-threads=1
```

Result: 40 passed, 0 failed, 0 ignored, 1,779 filtered out.

### Exact canonical field ledger

```bash
cargo test --manifest-path cli/Cargo.toml native::service_request::tests::canonical_field_ledger_matches_schema_constraints_and_every_role_set -- --exact --test-threads=1
```

Result: 1 passed, 0 failed, 0 ignored, 1,818 filtered out.

## Contract, Client, Documentation, And Quality Results

- `pnpm test:service-api-mcp-parity`: passed. It checked 66 browser controls,
  26 service tools, 19 service resources, 0 HTTP-only service routes, 62 native
  service actions, and 96 service-request actions.
- `pnpm test:service-request-client`: passed.
- `pnpm test:service-client`: passed every chained contract-generation, type,
  export, request-helper, observability-helper, managed-profile, broker, and
  five dry-run example check.
- `node scripts/generate-service-request-client.js --check`: passed with no
  generated drift.
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`: passed.
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`: passed with
  zero warnings promoted to errors.
- `pnpm --dir docs build`: passed. Next.js compiled, type-checked, and generated
  all 34 routes. The existing multiple-lockfile workspace-root warning was
  informational and did not affect the result.
- `pnpm validation:select -- --base ae36b272`: passed and reported 47 changed
  paths across all concurrent packets. Every Candidate 1 check named by the
  selector was run directly or as part of `pnpm test:service-client`. Dashboard,
  live-browser, installer, release, and other concurrent-packet recommendations
  were excluded from this tester's authority.
- `git diff --check`: passed.

## Canonical Rust Harness

The repository-defined harness was used instead of repeating the known
environment-invalid raw parallel `cargo test` invocation:

```bash
bash scripts/ci/rust-tests.sh
```

The harness produced these results before its first failure:

- parallel-safe partition: 1,175 passed, 0 failed, 57 ignored;
- `agent_env::tests`: 2 passed, 0 failed;
- `connection::tests`: 35 passed, 0 failed;
- `flags::tests`: 97 passed, 0 failed;
- `native::actions::tests`: 260 passed, 0 failed;
- `native::auth::tests`: 8 passed, 0 failed;
- `native::cdp::chrome::tests`: 73 passed, 1 failed.

The one failure was independently reproduced once with the exact test filter:

```text
native::cdp::chrome::tests::test_headed_display_fallback_not_used_when_display_set
left: Some(":9")
right: None
```

`git diff --quiet ae36b272 -- cli/src/native/cdp/chrome.rs` confirmed the file
is unchanged. The test itself sets `DISPLAY=:9`, so this is the already-known
display baseline, not a Candidate 1 regression.

Because that baseline stops the canonical script, the six unrun serial
partitions were completed manually with the harness's exact filter and
single-thread convention:

- `native::control_plane::tests`: 27 passed, 0 failed;
- `native::parity_tests`: 18 passed, 0 failed;
- `native::policy::tests`: 11 passed, 0 failed;
- `native::providers::tests`: 4 passed, 0 failed;
- `native::service_health::tests`: 42 passed, 0 failed;
- `runtime_profile::tests`: 9 passed, 0 failed.

Across the complete partition set, 1,819 tests were discovered: 1,761 passed,
1 unrelated baseline test failed, and 57 were ignored. Candidate 1 had zero
focused or broad failures.

## Independent Semantic Verification

- The canonical schema has exactly 62 properties. It includes `browserHost`,
  `viewStreamProvider`, and `controlInputProvider`, and excludes top-level
  `args`.
- The machine-readable ledger has a 62-field role union with no duplicates
  inside a role: 2 structural, 56 command, 52 trace, 29 routing, and 5
  validation-only fields. Its exact transport-legacy set is `{args}`.
- `serviceTabHandle` now has the routing role. The exact ledger test proves that
  every field in both `SERVICE_REQUEST_ACCESS_PLAN_ROUTING_FIELDS` and
  `SERVICE_REQUEST_HTTP_RELAY_CANONICAL_POINTERS` is present in the routing
  role. The HTTP pointer ledger covers `sessionName`, `browserId`, and both
  nested lane selectors under `serviceTabHandle`.
- The normalizer has 12 semantic interface tests. HTTP retains 9
  transport/relay tests and MCP retains 5 schema, request-identity, queue-lane,
  and transport-envelope tests. The HTTP and MCP service-request test-name sets
  have no intersection, and no duplicated action-semantics family remains in
  both adapters.
- HTTP top-level `args` precedence is retained only in the HTTP adapter. MCP
  rejects top-level `args` with exact JSON-RPC invalid-params code `-32602` and
  preserves `params.args`.
- The cross-adapter invalid-request matrix asserts the exact shared issue
  message inside the HTTP `400 Bad Request` body and the MCP JSON-RPC
  `-32602` envelope. The dedicated HTTP and MCP adapter tests independently
  bind their respective envelope behavior.

## Failures And Attribution

There is one non-Candidate 1 failure: the unchanged display fallback unit test
described above. No Candidate 1 source, contract, client, documentation, or
quality check failed. Concurrent Candidate 2 dashboard changes were not used
to excuse or attribute any Candidate 1 result.

## Effects

This role edited only this test receipt. It did not edit Candidate 1 source,
contracts, generated files, plans, or prior audit notes. It did not start a
browser, contact or mutate an installed service, change runtime or tenant
state, install software, commit, push, release, or perform any live-system
effect. Cargo and Next.js created only ordinary ignored local build artifacts.
