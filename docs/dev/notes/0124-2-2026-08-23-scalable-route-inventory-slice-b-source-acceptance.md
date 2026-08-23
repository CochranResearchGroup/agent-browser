# Plan 0124 Slice B Source Acceptance

Date: 2026-08-23

Status: SOURCE ACCEPTED

Plan: `docs/dev/plans/0124-2026-08-23-scalable-desktop-evidence-and-presentation-capacity-plan.md`

Source baseline: `aeadcc7d1a70f632547fa2c957fa76ee50d8d02c`

## Outcome

Slice B replaces canonical two-route assumptions with list-shaped static route
inventory across route discovery, readiness, display inspection, access grants,
doctor output, workstation reconciliation, installed support assets, and the
many-to-many live harness. The inventory preserves arbitrary route identities,
users, displays, viewer profiles, and executables in deterministic order.

Legacy A and B environment and secret inputs remain supported only through the
explicit JavaScript, Python, and Rust compatibility adapters. The architecture
guard rejects fixed two-entry truncation and alphabetic route configuration in
canonical owners.

The route-user helper resolves an arbitrary bounded inventory, generates only
missing passwords, writes the canonical secret atomically with private mode,
and renders one transactional Guacamole SQL program with exact route counts,
distinct usernames, legacy-row migration, permissions, and postconditions.
The workstation generation packages both shared inventory libraries alongside
the scripts that import them.

## Validation

- Six-route provider-free route inventory integration: passed.
- Four-route route-user resolution and SQL fixture: passed.
- Route-specific user sync and PostgreSQL durability fixtures: passed.
- Route-confusion no-launch gates: passed.
- Source-free workstation installer, host provision, fresh VM harness, and
  Guacamole asset fixtures: passed.
- Presentation capacity architecture guard: passed with zero canonical
  two-slot findings.
- Rust presentation inventory tests: 4 passed.
- Rust canonical route-pool tests: 3 passed.
- Rust workstation installer tests: 87 passed serially.
- Rust output tests: 36 passed.
- Rust formatting and strict Clippy: passed.
- Docs production build, handoff docs, release asset verifier, and diff hygiene:
  passed.

The workstation module initially exposed one parallel-sensitive existing
process-exit timing failure. Its exact serial rerun passed, and the complete
87-test workstation module passed with `--test-threads=1`, matching repository
policy for environment-mutating tests.

## Scope Boundary

No browser, profile, display, Guacamole connection, RDP target, process,
dashboard, installed generation, ingress, or production state was changed.
Slice B does not dynamically provision or remove provider resources. Slice C
is the next authorized source slice and introduces durable presentation
capacity authority without performing installed-runtime acceptance.
