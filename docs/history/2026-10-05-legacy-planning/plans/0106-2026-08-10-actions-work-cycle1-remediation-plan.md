# Plan 0106 | Actions Work Cycle 1 Remediation

Date: 2026-08-10

State: ACCEPTED BOUNDED REMEDIATION

Authority:

- `docs/dev/plans/0101-2026-08-09-route-bound-open-actions-deepening-plan.md`
- `docs/dev/plans/0102-2026-08-09-route-bound-open-cycle2-residual-repair-plan.md`
- `docs/dev/notes/0101-2026-08-10-route-bound-open-actions-work-audit.md`
- findings `P0101-W1-01` through `P0101-W1-07`

## Adjudication

The orchestrator accepts `P0101-W1-01` through `P0101-W1-04`,
`P0101-W1-06`, and `P0101-W1-07` as blocking in full. It accepts the receipt,
roadmap, and rollback-evidence portions of `P0101-W1-05` as blocking.

Rewriting already-green local history into 81 retroactive commits is rejected
as a destructive, evidence-reducing repair. The implementation already has
cohesive, independently green checkpoints. This packet supersedes only the
81-commit granularity requirement with those durable checkpoints. It does not
waive the 81 responsibility packets, inventory coverage, source rollback
clarity, or truthful receipt requirements. The execution receipt must record
the original deviation and this superseding authority explicitly.

## Bounded Remediation

1. Extend the architecture and WSL-safety harness with red fixtures for every
   accepted structural and execution finding before changing production code.
2. Put every route-bound repository phase and the control-plane terminalization
   path inside the existing total deadline. At return, no coordinator,
   repository, compensation, or cleanup task may remain.
3. Commit acquisition finalization and the optional durable handoff through one
   atomic repository operation. Add failure injection for each write and rename
   boundary and prevent either persisted file from getting ahead of the other.
4. Replace raw route JSON and string-derived outcomes with normalized domain
   types. Install the permanent daemon/browser runtime adapter and delete the
   transitional adapter and older fallback helper. Test every outcome, all nine
   fallback predicates, immutable freshness, unchanged retained resources, and
   ingress authorization parity through the coordinator interface.
5. Delete the broad common prelude and test-only facade. Recombine shallow
   action files into cohesive owners and move tests beside the public domain
   interfaces they specify, while preserving all 615 responsibilities and the
   exact six dispatcher definitions.
6. Make the architecture checker inspect the repaired risks rather than
   generated prose. Correct the execution receipt and durable roadmap to the
   actual acceptance state and checkpoint history.
7. Route every WSL-capable aggregate, selector, build, format, Clippy, and test
   entry point through `scripts/ci/cargo-safe.sh`. Add a static fail-closed gate
   against new raw WSL Cargo paths.

## Verification Bound

All compiling Rust work remains serialized under the WSL cgroup guard. The
executor runs no browser, installation, doctor, ignored end-to-end, live, or
external-effect gate. After one remediation pass, Cycle 2 is closed-world over
the seven finding IDs and critical regressions introduced by their fixes. No
third work-audit cycle is authorized.
