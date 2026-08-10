# Plan 0100 Service Status Projection Final Test Receipt

Date: 2026-08-10

Role: distinct final tester

Verdict: **FAIL**

Commit disposition: **Candidate 3 cannot commit.**

Fixed integration base:
`0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`

## Scope and authority

This receipt independently tested the current Candidate 3 implementation
against Plan 0100, the completed two-cycle work audit, the Plan 0105 bounded
residual repair, and the execution note. It did not reopen work audit or
architecture discovery.

The tester edited only this receipt. No implementation, plan, audit, installed
binary, user service, browser, display, route, tenant state, commit, remote, or
live runtime was changed.

## Final finding

### `P0100-T1-01` — blocking — the typed launch contract breaks three canonical action status calls

Criterion: Plan 0105 R01 requires the nine-field launch configuration to reject
invalid input before observation, while the production action and control-plane
adapters construct the same typed v1 record from their existing launch
configuration. Canonical Rust partitions must remain green apart from the
documented unchanged DISPLAY-sensitive baseline.

Evidence:

- `StatusLaunchConfiguration` is a typed nine-field record and its focused
  omission, type, and pre-observation tests pass.
- `handle_service_status_with_dependencies` obtains `launchConfig` from the
  command and substitutes `{}` when it is absent.
- The canonical `native::actions::tests` partition has three existing status
  calls without `launchConfig`. Each now receives `success: false` where its
  contract expects `success: true`:
  - `test_service_status_via_actions_does_not_launch_browser`
  - `test_service_status_repairs_stale_guacamole_view_url`
  - `test_service_status_leaves_guacamole_root_without_route`
- Exact partition result: 257 passed, 3 failed, 0 ignored, 1,584 filtered.

Consequence: the focused fixed-input harness is green because it supplies a
complete launch record, but the production action ingress is not compatible
with all canonical status calls. This is Candidate 3 scope and is distinct
from the unchanged DISPLAY-sensitive baseline.

Reproducer:

```bash
bash scripts/ci/rust-tests.sh
```

The script passes its parallel-safe partition and reaches the three failures in
the serial `native::actions::tests` partition.

Disposition: `blocking`. Candidate 3 cannot commit until the action ingress
and the frozen required-input behavior are reconciled and the canonical action
partition passes.

## Finding ledger

| Finding | Final result | Independent evidence |
| --- | --- | --- |
| `P0100-W1-01` | **FAIL at production integration** | The typed nine-field record, required-field loop, representative wrong-type cases, and pre-observation rejection pass, but `P0100-T1-01` proves the real action adapter does not construct a valid record for three canonical calls |
| `P0100-W1-02` | PASS | Paired action and control-plane repository failure test passed, 1 of 1 |
| `P0100-W1-03` | PASS | Browser Session Authority tests passed, 4 of 4; partial collection leaves the uncorrelated browser unknown |
| `P0100-W1-04` | PASS | Cached display timestamp and typed freshness tests passed; cache hits preserve their original completion-derived timestamps |
| `P0100-W1-05` | PASS | Timed out, unsupported, unavailable, and failed terminal states remain distinct with null non-observation values |
| `P0100-W1-06` | PASS | Public status handler forwards successful non-JSON bytes without interpretation, 1 of 1 |
| `P0100-W1-07` | PASS | Nine status-cache tests plus late-completion and independent three-phase tests passed; cancellation cleanup, owner cancellation, 32-key overflow, completion TTL, and request-id protection are covered |
| `P0100-W1-08` | PASS | Old-v1 and current-v1 assignments compile against the exported current type; runtime old-v1 fixture preserves omitted additive fields |
| `P0100-W1-09` | PASS | Real fixed-input Rust and generated-client harnesses passed; exact MCP tool, resource, and template allowlists, narrower classifications, payload scans, and both generic full-status rejections passed |

## Focused validation

### Rust

The focused Rust packet completed 66 passing tests with no focused failures:

- Service Status Projection: 15 passed.
- Browser Session Authority: 4 passed.
- combined worker and Service State response: 1 passed.
- real fixed-input producer and transport harness: 1 passed.
- action and control-plane repository failure parity: 1 passed.
- typed display cache: 1 passed.
- dashboard status single-flight selection: 9 passed.
- handler byte identity: 1 passed.
- late completion request-id protection: 1 passed.
- independent connect, write, and read phase bounds: 1 passed.
- service model contract partition, serial: 31 passed.

The dashboard status tests cover complete cancellation cleanup, one-waiter
cancellation, owned-flight cancellation, the 32 in-flight key ceiling, the
uncached 33rd key, late completion, completion-based TTL, and the three
independent I/O timeout phases.

### Client, schema, transport, and MCP

Passed:

- `pnpm test:service-client-types`
- `pnpm test:service-observability-client`
- `pnpm test:service-status-fixed-input-harness`
- `pnpm test:service-client`
- `pnpm test:service-status-no-launch` on the immediate compiled rerun
- `pnpm test:service-contracts-no-launch`
- `pnpm test:mcp-read-no-launch`
- `pnpm test:service-api-mcp-parity`
- `pnpm test:cross-seam-interlocks`
- `pnpm test:route-confusion-gates`

The API and MCP parity gate reported exactly 66 browser controls, 26 service
tools, 19 service resources, 0 HTTP-only service routes, 62 native service
actions, and 96 service-request actions.

The MCP no-launch gate deep-compared the complete ordered inventories: 96
tools, 19 static resources, and 6 resource templates. Every tool has a frozen
narrower-result classification except the two generic routes, which retain
explicit full-status rejection. Every resource and template read or rejection
payload was scanned for both a full status envelope and `statusProjection`.

### Dashboard and P99 boundary

Nine affected dashboard gates passed:

- Workspace View Projection
- view streams
- workspace nodes
- selected workspace context
- workspace navigator
- workspace inspector tab
- inspector actions
- browser table
- workspace viewport controller

The P99 adapter remains the owner of workspace selection and actionability.
Current typed observations contribute presentation only, while Browser Session
Authority remains a separate preprojection authority input. No deleted shallow
selector or dashboard status-repair symbol was found.

### Builds and quality gates

Passed:

- dashboard production build
- documentation production build
- `cargo fmt --manifest-path cli/Cargo.toml -- --check`
- `cargo clippy --manifest-path cli/Cargo.toml -- -D warnings`
- `pnpm validation:select -- --base 0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`
- `git diff --check`

The selector reported 43 changed paths because its inventory also includes the
untracked plan and audit artifacts outside the 36-path implementation manifest.
Live CDP, installed dashboard publication, installed-skill synchronization,
workstation provisioning, and release checks were intentionally withheld by
the no-browser, no-install, and no-live boundary.

## Canonical Rust result

The proportional canonical run used `bash scripts/ci/rust-tests.sh`.

- Parallel-safe partition: 1,198 passed, 0 failed, 57 ignored, 589 filtered.
- `agent_env::tests`: 2 passed.
- `connection::tests`: 35 passed.
- `flags::tests`: 97 passed.
- `native::actions::tests`: 257 passed, 3 failed, 0 ignored, 1,584 filtered.

The canonical script stopped at the Candidate 3 action failures. A manual
continuation reproduced the separately documented unchanged baseline in
`native::cdp::chrome::tests`: 73 passed and 1 failed. The exact assertion was
`Some(":9")` versus `None` in
`test_headed_display_fallback_not_used_when_display_set`. The control-plane
partition then passed 29 of 29. A later parity-partition continuation was
stopped after prolonged silence because the scoped Candidate 3 blocker was
already proven; it is not counted as passed.

## Warnings

- The first `service-status-no-launch` attempt reached the existing 60-second
  wrapper timeout while the Rust binary was being rebuilt. The immediate
  compiled rerun passed in 1.1 seconds. This is recorded as a transient build
  warning, not the blocking finding.
- The dashboard build emitted its established exported-rewrite warnings and
  completed successfully.
- The docs build emitted its established multiple-lockfile workspace-root
  warning and completed successfully.
- Two CDP profile-copy tests emitted their established missing `Local State`
  warnings and passed.

## Deletion and compatibility guards

Passed:

- no `repair_dashboard_service_status_response` or
  `repair_dashboard_service_status_value` remains;
- no deleted P99 shallow stream selector was reintroduced;
- successful dashboard status bytes bypass JSON interpretation;
- a Guacamole root is not fabricated into a client route by the focused
  compatibility projector tests;
- old-v1 clients may omit Browser Session Authority `availability`, summary
  `unknownBrowserCount`, and top-level `statusProjection`;
- current-v1 clients compile and preserve the complete projection contract;
- the generated fields remain additive and optional for old-server payloads;
- raw observations do not become P99 ownership, proof, or actionability.

The three failing canonical action tests prevent a positive legacy-v1 action
boundary conclusion even though the focused projector compatibility cases
pass.

## Exact identities

The 36-path Candidate 3 manifest contains 35 implementation paths plus the
execution receipt. The non-self-referential identity over the sorted
newline-delimited `sha256sum` stream for the 35 implementation paths is:

`8e71ba7abad40156f7b74385635f86df7105103e1beec2970024e60fbad2c0ad`

It exactly matches the execution note's claimed implementation identity.

Separately hashed evidence inputs:

- execution note:
  `79aba0f715605d40bb3dee9eb54d0655b9b7e39acbb29bb9240edf41799aa28e`
- Plan 0100:
  `711896f5a3641629e6b15c09a635fa09e0723f468c19de256013e2a84a60b65a`
- Plan 0105:
  `6ef950da4303cb1ea812fec5b9c930dc4b285cffdc82a9fc9abba6e63c2e3101`
- completed work audit:
  `8595647a23f4166278d42d9697d24c89c1f065fd193df2d02eaf1f8f1d83b478`

This receipt is intentionally not included in the implementation identity. Its
separate SHA-256 is reported by the tester at handoff rather than embedded
self-referentially.

## Residual boundary

Candidate 3 needs one bounded implementation repair for `P0100-T1-01`, followed
by rerunning the three failing action tests and the canonical Rust partitions.
No third work audit is authorized or required by this receipt. Candidate 4
must remain stopped because Plan 0101 requires Candidate 3 to land and validate
first.

## Final bounded retest disposition

Retest date: 2026-08-10

Superseding verdict: **PASS**

Commit disposition: **Candidate 3 is commit-ready.**

This bounded retest supersedes the earlier FAIL disposition for
`P0100-T1-01`. It tested the corrected action and daemon/control ingress seam
without reopening work audit and without editing implementation, plans, or
audits.

### `P0100-T1-01` closure

The correction gives only an absent legacy `launchConfig` a complete typed
nine-field default. A present value is forwarded unchanged into strict typed
validation.

Independent verification:

- absent `launchConfig` produces a complete typed record with
  `defaultBrowserBuild: null`, `stealthCdpChromiumRequired: false`, and
  `stealthCdpChromiumReady: true`;
- explicit `{}` remains `{}` and is rejected;
- explicit JSON `null` remains `null` and is rejected by the object type gate;
- representative malformed field types remain rejected by the nine-field
  launch-configuration test;
- invalid launch authority still fails before observation invocation;
- lowercase legacy `ready` worker and browser-health values are accepted and
  normalized to their typed variants;
- unknown worker and browser-health values are rejected as invalid
  control-plane authority.

The action and daemon ingress adapters use the same absent-only default helper.
No caller may turn an explicit invalid value into the legacy default.

### Retest evidence

The three originally failing tests passed individually, 1 of 1 each:

- `test_service_status_via_actions_does_not_launch_browser`
- `test_service_status_repairs_stale_guacamole_view_url`
- `test_service_status_leaves_guacamole_root_without_route`

Additional results:

- absent legacy default and present malformed action test: 1 passed;
- complete `native::actions::tests` partition: 261 passed, 0 failed, 0
  ignored, 1,586 filtered;
- Service Status Projection partition: 17 passed, 0 failed, 0 ignored, 1,830
  filtered;
- control-plane Service Status response: 1 passed;
- real fixed-input status entries and transports: 1 passed;
- daemon partition: 5 passed;
- `pnpm test:service-status-no-launch`: passed on the first retest attempt;
- Rust formatting: passed;
- strict Clippy with warnings denied: passed;
- `git diff --check`: passed.

No browser, display, installed runtime, user service, route, tenant, commit,
remote, or live-system effect occurred.

### Superseding identity

The corrected Candidate 3 manifest contains 37 paths: 36 implementation paths
plus the execution receipt. `cli/src/native/daemon.rs` is the one added
implementation path relative to the prior 36-path manifest.

The independently recomputed non-self-referential SHA-256 over the sorted
newline-delimited `sha256sum` stream for the 36 implementation paths is:

`81d603d6a1b40588db21c69dd59891e462562de60f492f6294155f46c4fe1ab2`

It exactly matches the execution note's superseding claimed identity.

The current execution-note SHA-256 is:

`52f78399812be905e18154e01396b0884b0cf9c40bfb7b77bcd8556ae1cb0125`

The pre-append test-receipt SHA-256 was:

`21f4079a02b6a9ade395224d99ec7733f6449ec22c37ac23a8c303f228bd782b`

The final appended receipt remains outside the implementation identity. Its
new separate SHA-256 is reported at handoff rather than embedded
self-referentially.

### Residual boundary

`P0100-T1-01` is closed. The earlier documented unchanged DISPLAY-sensitive
CDP test remains a baseline warning and is not a Candidate 3 failure. No
Candidate 3 implementation blocker remains. Candidate 4 may proceed after the
Candidate 3 commit checkpoint required by Plan 0101.
