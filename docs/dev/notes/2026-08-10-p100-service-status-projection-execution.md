# P100 Service Status Projection Execution

Date: 2026-08-10

Integration base: `0528e5db48dc26f630b1d1994bfc2fbf9f0f76bb`

## Locality verdict

The local observation adapter is valid only in the daemon process selected as
the Service Status backend. The workstation and dashboard user-unit sources
both inherit `%h/.agent-browser/.env`; the dashboard gateway selects the
dedicated `dashboard-service-backend` session; display observations run only
for configured or explicitly bound route displays. Missing process, runtime
profile, X11, route, or source-host capability is represented by typed partial
or unavailable observation state. It is never promoted to Service State,
Browser Session Authority, route proof, inventory, or actionability.

Static source evidence:

- `cli/src/workstation_install.rs` and
  `scripts/install-dashboard-user-service.sh` install dashboard units with the
  shared environment file.
- `cli/src/native/stream/dashboard.rs` binds status cache entries to the
  dedicated backend session, backend port, and complete request path.
- `cli/src/native/remote_view.rs` authorizes display probes against configured
  route displays and bounds each display command to two seconds.
- `cli/src/native/service_status_projection/local_observation.rs` owns runtime,
  process, display, freshness, and typed-unavailable observation mapping.

The frozen bounds remain ten seconds independently for backend connect, write,
and read; five seconds for completed successful status responses; five seconds
for display observation freshness; two seconds for the display command; and 32
status cache keys. Successful gateway responses are forwarded byte for byte.

## Validation

The execution passed the focused projector, control-plane, five single-flight
cache, CDP stream, schema and generated-client, API and MCP parity, dashboard
projection and consumer, no-launch CLI status, no-launch service-contract, MCP
read, route-confusion, dashboard build, documentation build, Rust formatting,
and strict Clippy gates.

The full parallel Rust run completed 1,763 tests successfully with 57 ignored
and one failure in the unchanged
`native::cdp::chrome::tests::test_headed_display_fallback_not_used_when_display_set`.
An exact serial rerun reproduced its pre-existing assertion mismatch: inherited
`DISPLAY=:9` returns `Some(":9")`, while that test expects `None`.

## Runtime boundary

No installed binary, user service, live browser, Guacamole route, display, or
tenant state was changed. Optional installed-runtime readback was withheld
because it would cross the execution packet's no-install boundary. Therefore
there is no claimed installed binary, unit, schema, response, or source-host
hash for this candidate. Source and no-launch proof are complete; installed
runtime convergence remains a separately authorized operation.

## Cycle 1 remediation evidence

The closed-world Cycle 1 remediation addresses `P0100-W1-01` through
`P0100-W1-09` without changing the P99 authority boundary:

- Authority, launch configuration, Browser Session Authority, observation
  state, source, component, stream, route-presentation, and error vocabulary
  are typed. Current-server queue, summary, verdict, resource-count, source
  host, timestamp, freshness, and nullability invariants fail before response
  serialization.
- Action and control-plane status paths share typed conversion and both surface
  repository persistence failures through their existing failure envelopes.
- Partial resource collection leaves unsupported per-browser authority unknown;
  partial local PID evidence remains partial and identifies the missing modeled
  browser.
- Display cache entries retain the original observation completion and expiry.
  Timeout, unsupported, unavailable, and failed probes remain distinct, and
  non-observed stream values remain null.
- Dashboard status success bypasses JSON interpretation. The 32-key cache has
  request-ID-scoped owned-task cleanup, late-result protection, success TTL
  from completion, independently bounded connect, write, and read phases, and
  uncached overflow when every bounded entry is in flight.
- Generated v1 types keep the additive Browser Session Authority fields
  optional and runtime fixtures cover both old and current v1 responses.
- A fixed authority, clock, and observation fixture deep-compares the real
  projector, control envelope, direct HTTP encoding, dashboard handler bytes,
  and dashboard CLI fallback at the data seam. The MCP no-launch smoke scans
  every advertised tool, static resource, resource template payload or
  rejection, and rejects both generic status-action attempts without exposing
  a full status envelope.

Focused remediation validation passed:

- Projector and observation tests: 12 passed.
- Browser Session Authority tests: 4 passed.
- Dashboard status cache tests: 9 passed, including reachable uncached overflow
  for the 33rd all-in-flight key, plus the handler byte-identity,
  late-completion, and three-phase timeout tests.
- Display-cache freshness and paired action/control-plane persistence-failure
  tests passed.
- Service model contract tests: 32 passed.
- MCP exhaustive read no-launch, Service Status no-launch, service API and MCP
  parity, generated service client, cross-seam interlocks, dashboard workspace
  projection, and route-confusion no-launch gates passed.
- Rust format and strict Clippy passed.
- Dashboard and documentation production builds passed. Their emitted Next.js
  workspace-root and exported-rewrite warnings are unchanged build warnings;
  both builds completed successfully.
- `git diff --check` and validation selection against the fixed integration
  base passed. Live CDP, workstation install, embedded-runtime publication, and
  installed-skill synchronization recommendations were withheld by the
  explicit no-live, no-install, and no-browser boundary.

The first expanded MCP smoke attempt reached its existing 60-second wrapper
limit while Cargo rebuilt the binary. The immediate compiled rerun reached the
new assertions, and the final corrected exhaustive run passed. No MCP or
browser runtime was launched outside the isolated no-launch stdio process.

The complete Candidate 3 implementation manifest contains 33 paths:

1. `README.md`
2. `cli/src/native/actions.rs`
3. `cli/src/native/browser_session_authority.rs`
4. `cli/src/native/control_plane.rs`
5. `cli/src/native/remote_view.rs`
6. `cli/src/native/service_model.rs`
7. `cli/src/native/service_status_projection.rs`
8. `cli/src/native/service_status_projection/authority.rs`
9. `cli/src/native/service_status_projection/compatibility.rs`
10. `cli/src/native/service_status_projection/local_observation.rs`
11. `cli/src/native/service_status_projection/observation.rs`
12. `cli/src/native/service_status_projection/tests.rs`
13. `cli/src/native/stream/dashboard.rs`
14. `cli/src/native/stream/http.rs`
15. `cli/src/native/stream/mod.rs`
16. `cli/src/output.rs`
17. `docs/dev/contracts/README.md`
18. `docs/dev/contracts/service-status-response.v1.schema.json`
19. `docs/dev/notes/2026-08-10-p100-service-status-projection-execution.md`
20. `docs/src/app/commands/page.mdx`
21. `packages/client/src/service-observability.generated.d.ts`
22. `packages/dashboard/src/components/service-panel.tsx`
23. `packages/dashboard/src/components/workspace-navigator.tsx`
24. `packages/dashboard/src/hooks/use-selected-workspace-context.ts`
25. `packages/dashboard/src/lib/service-workspaces.ts`
26. `packages/dashboard/src/lib/workspace-view-projection.ts`
27. `scripts/generate-service-observability-client.js`
28. `scripts/smoke-mcp-read-no-launch.js`
29. `scripts/smoke-schema-utils.js`
30. `scripts/test-cross-seam-interlocks.js`
31. `scripts/test-dashboard-workspace-view-projection.js`
32. `scripts/test-service-observability-client.js`
33. `skills/agent-browser/SKILL.md`

Plans 0100 through 0102 and all plan or work audit notes are explicitly outside
this implementation identity and remain unmodified by Candidate 3.

## Plan 0105 terminal residual repair

The terminal bounded repair closes residuals R01 through R03 without reopening
the passed Cycle 1 cache, handler, projector, or observation decisions:

- R01 replaces the transparent launch map with a typed nine-field record. The
  focused projector gate accepts the complete record, rejects every individually
  omitted required field and representative wrong types, and proves invalid
  launch authority cannot invoke observation.
- R02 compiles old-v1 and current-v1 assignments against the exported current
  `ServiceStatusResponse` type from both `test:service-client-types` and the
  aggregate `test:service-client` gate.
- R03 injects one fixed authority, clock, observation, repository, and transport
  harness through the real action, control-plane, direct HTTP, dashboard backend,
  dashboard CLI fallback, and generated-client paths. The MCP no-launch smoke
  freezes exact tool, resource, and resource-template allowlists, classifies
  every tool as narrower output or explicit full-status rejection, scans every
  resource and template payload, and retains explicit generic browser-command
  rejection coverage.

Terminal focused validation passed: 12 projector tests, the real fixed-input
Rust harness, the generated-client fixed-input harness, the paired repository
failure test, direct and aggregate client gates, exhaustive MCP no-launch smoke,
Rust formatting, strict Clippy, dashboard production build, documentation
production build, fixed-base validation selection, and `git diff --check`.
The successful builds retained the existing Next.js exported-rewrite and
workspace-root warnings.

The complete Candidate 3 manifest now contains 36 paths. It is the 33-path
manifest above plus `package.json`,
`scripts/service-status-v1-compatibility-types.ts`, and
`scripts/test-service-status-fixed-input-harness.js`.

The non-self-referential implementation content identity is
`8e71ba7abad40156f7b74385635f86df7105103e1beec2970024e60fbad2c0ad`.
It is SHA-256 over the newline-delimited `sha256sum` output for the 35 sorted
implementation paths, excluding this execution note. The execution note is a
separate evidence receipt whose hash is reported at handoff and is intentionally
not written into itself. Plans and audit notes remain outside both identities.

## Final tester-blocker correction

`P0100-T1-01` is corrected at the action and daemon/control ingress seam. An
absent legacy `launchConfig` now receives one typed-by-construction complete
nine-field default. A present value remains untouched, so an explicit empty,
null, or wrongly typed record still fails the strict projector contract.

The typed control-plane vocabulary accepts the preexisting lowercase wire
aliases, including `ready`, while preserving its canonical serialization and
rejecting unknown worker and browser-health values.

The three named action regressions passed individually. The complete
`native::actions::tests` partition passed 261 of 261, and the projector
partition passed 14 of 14. The Service Status no-launch smoke passed on the
immediate compiled rerun after its established first-build wrapper timeout.
Rust formatting, strict Clippy, and `git diff --check` passed.

The complete Candidate 3 manifest now contains 37 paths: the preceding 36-path
manifest plus `cli/src/native/daemon.rs`. The superseding non-self-referential
implementation content identity is
`81d603d6a1b40588db21c69dd59891e462562de60f492f6294155f46c4fe1ab2`.
It is SHA-256 over the newline-delimited `sha256sum` output for the 36 sorted
implementation paths excluding this execution note.
