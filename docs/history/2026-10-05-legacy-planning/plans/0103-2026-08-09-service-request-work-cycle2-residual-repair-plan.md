# Plan 0103 | Service Request Work Cycle 2 Residual Repair

Date: 2026-08-09

State: APPLIED BOUNDED REPAIR

Authority:

- `docs/dev/plans/0098-2026-08-09-service-request-normalization-deepening-plan.md`
- `docs/dev/notes/0098-2026-08-09-service-request-normalization-work-audit.md`
- terminal finding `P0098-W1-02`

## Bound

Candidate 1 exhausted its two implementation-audit cycles with one narrow
blocking residual. The canonical `serviceTabHandle` field participates in HTTP
relay routing through its nested `sessionName` and `browserId`, but the Rust
and JSON ledgers did not tag the top-level field as routing and the mechanical
consumer gate covered only access-plan routing.

This packet resolves only that closed finding. It is not a third audit cycle,
does not alter the public request schema or behavior, and does not reopen the
normalizer architecture.

## Applied Correction

1. Mark `serviceTabHandle` as routing in the Rust field specification.
2. Add `serviceTabHandle` to the machine-readable JSON routing role.
3. Replace the HTTP relay's duplicated canonical pointer list with one named
   production-used pointer ledger:
   `SERVICE_REQUEST_HTTP_RELAY_CANONICAL_POINTERS`.
4. Make the canonical field-role test require every unique top-level field in
   that HTTP ledger and every access-plan routing consumer to carry the routing
   role.
5. Preserve the existing open `params` relay aliases as adapter compatibility;
   they remain outside canonical top-level equality.

The named HTTP ledger maps:

- `sessionName` to `/sessionName`;
- `browserId` to `/browserId`;
- `serviceTabHandle` to `/serviceTabHandle/sessionName` and
  `/serviceTabHandle/browserId`.

## Verification And Handoff

The bounded correction passed:

- the exact canonical field-ledger Rust test, one passed and zero failed;
- `pnpm test:service-api-mcp-parity` for all 62 canonical properties and 96
  service-request actions;
- Rust formatting;
- `git diff --check`.

The distinct Candidate 1 tester must re-run the complete focused and selected
packet, including strict Clippy and the generated-client checks. No further
work audit is authorized. A test failure is reported as a test receipt or a
bounded implementation blocker, not as a reason to restart architecture
review.

Effects: the four scoped source and contract files named above plus this
planning receipt. No runtime, browser, installation, tenant, commit, push,
release, or live-system effect occurred.
