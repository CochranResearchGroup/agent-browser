# Plan 0107 | Actions Work Cycle 2 Residual Repair

Date: 2026-08-10

State: ACCEPTED TERMINAL REPAIR

Authority:

- `docs/dev/notes/0101-2026-08-10-route-bound-open-actions-work-audit.md`
- terminal Cycle 2 findings `P0101-W1-01`, `P0101-W1-03` through
  `P0101-W1-07`

## Bound

The two work-audit cycles are exhausted. This packet fixes concrete terminal
risks without opening a third audit. Candidate 4 advances directly to its
distinct final tester afterward.

## Adjudication

- Accept the two raw WSL Cargo launchers as blocking and close them.
- Accept that terminal quarantine must be safe before an external cleanup can
  exhaust the total deadline. Persist `rollback_incomplete` quarantine before
  cleanup, then promote to `rolled_back` only after confirmed compensation.
- Accept opaque JSON wrappers crossing the route coordinator boundary as
  blocking. Replace them with concrete domain records and exercise real ingress
  authorization construction paths.
- Accept relocation of tests that call owner-private helpers from dispatcher
  buckets. Retain genuine `execute_command` integration tests under
  `native::actions`; the dispatcher is their public interface and their size
  alone is not a facade violation.
- Treat the regex architecture checker as a structural regression guard, not a
  proof of transaction or runtime semantics. Behavioral Rust tests own those
  guarantees. Update its output and receipts so they do not overclaim.
- Preserve Plan 0106's truthful cohesive-checkpoint history adjudication.

An operating system may place a filesystem syscall into an uninterruptible
kernel wait. No in-process Rust future can cancel that syscall. The repaired
contract therefore requires bounded lock acquisition, pre-cleanup quarantine,
no detached async task, and explicit reporting of the residual kernel-I/O
limit. It does not claim physically impossible preemption of a syscall already
inside the kernel.

## Terminal Verification

The executor adds behavioral tests for pre-cleanup quarantine, cleanup success
promotion, timeout preservation, concrete route types, all fallback predicates,
and real ingress rejection before effects. It closes both WSL bypasses and the
static detector fixtures, relocates only misowned private-helper tests, corrects
the receipt and roadmap, and runs the guarded canonical Rust, architecture,
client, dashboard, formatting, strict Clippy, selector, and diff gates.

No third work audit, browser, ignored end-to-end, installation, doctor, live,
push, release, or external effect is authorized.
