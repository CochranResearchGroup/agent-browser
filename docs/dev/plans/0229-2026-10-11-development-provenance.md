# Repair development runtime provenance

Product lane: PL-BUGFIX
Work item: #79
Branch: fix/development-provenance-79
State: COMPLETED

Restore the default isolated development runtime, skill and source bindings without production effects. Retain verified sealed source identity independently of the original build path; reject mismatched candidate digests before publication. Publication already corrects the old executable/dashboard mismatch. A real disposable launch exposed legacy SQLite receipt incompatibility; headless doctor also inherited unused legacy helper checks.

Acceptance: preserve authoritative legacy database state while reading its original migration evidence; headless development doctor does not demand a legacy helper, while explicit native or legacy provider failures still block. Run failing regression tests before fixes, focused store and doctor tests, format and strict Clippy, current installed doctor and three disposable launch/read/close cycles. Preserve production identity and exact process cleanup. No new viewer, profile reset, authentication or reboot.

Accepted evidence: [repair receipt](../notes/0229-2026-10-11-development-runtime-repair.md). Both installed doctors and all three disposable browser cycles passed; production identity remained unchanged. Integration is through the issue-linked repair PR.
