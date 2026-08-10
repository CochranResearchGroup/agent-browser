# Plan 0105 | Service Status Work Cycle 2 Residual Repair

Date: 2026-08-10

State: ACCEPTED BOUNDED REPAIR

Authority:

- `docs/dev/plans/0100-2026-08-09-service-status-projection-authority-deepening-plan.md`
- `docs/dev/notes/0100-2026-08-10-service-status-projection-work-audit.md`
- terminal findings `P0100-W1-01`, `P0100-W1-08`, and `P0100-W1-09`

## Bound

Candidate 3 exhausted its two implementation-audit cycles after closing six
of nine Cycle 1 findings. This packet splits the three remaining failures into
one exact implementation unit. It is not a third audit cycle and does not
reopen the projector, observation, dashboard-cache, or P99 architecture.

No further work audit is authorized. A distinct tester will verify this packet
and the complete Candidate 3 acceptance surface.

## Repair R01 | Required Launch Configuration

Replace the transparent launch-configuration map with a typed record for the
nine fields required by the v1 schema. Conversion from JSON must reject a
missing field, null where a concrete value is required, a wrong type, and an
unknown field when the schema forbids it. Validation occurs before observation
or serialization.

The public projector tests must include:

- one complete valid launch record;
- every required field omitted individually;
- a representative wrong type for each field type;
- proof that observation is not invoked after invalid authority input.

Production action and control-plane adapters must construct the same typed
record from their existing launch configuration without changing the v1 wire
names or values.

## Repair R02 | Compiled Old-v1 Client Compatibility

Add an old-v1 fixture to the generated-client TypeScript compilation gate. The
fixture must assign a response without Browser Session Authority
`availability` and `unknownBrowserCount` to the exported current
`ServiceStatusResponse` type. The existing current-server fixture remains.

The compiled fixture must run from `pnpm test:service-client-types` and
`pnpm test:service-client`, so regenerating those two additions as required
causes a fast gate failure.

## Repair R03 | Executable Producer and MCP Ledger

Bind the real status entry adapters to one injected fixed-input harness rather
than test-only formatting helpers. The harness must cross:

- direct action status;
- control-plane status;
- direct HTTP status;
- dashboard backend handler;
- dashboard CLI fallback;
- generated-client decoding.

Each path must deep-compare canonical `data`, including the complete
`statusProjection`, for the same typed authority input, fixed clock, and fixed
observations. The harness may use in-memory repositories and transports; it
must not start a daemon, browser, display probe, installed service, or live
network listener.

Freeze the complete MCP tool-name inventory as an executable allowlist. For
every advertised tool, prove either its narrower typed result classification
or its explicit rejection of full Service Status. Freeze the complete MCP
resource and template inventories and prove their payloads contain neither a
full status envelope nor `statusProjection`. Keep generic
`browser_command(service_status)` rejection explicit.

## Verification and Handoff

The executor runs only the targeted Candidate 3 packet, Rust formatting,
strict Clippy, dashboard and docs builds, generated-client checks, fixed-base
validation selection, and `git diff --check`. It updates the execution note
without placing a self-referential aggregate hash inside the hashed path set.

The distinct tester must independently verify the three repairs, all nine
Cycle 1 dispositions, deletion guards, exact content identity, and the known
unchanged DISPLAY-sensitive broad-suite baseline. A tester failure is reported
as a bounded implementation blocker, not a reason to restart architecture
review.

Effects: scoped source, generated client, fixtures, execution evidence, and
this planning receipt only. No installation, runtime, browser, display,
tenant, service, commit, push, release, or live-system effect is authorized.
