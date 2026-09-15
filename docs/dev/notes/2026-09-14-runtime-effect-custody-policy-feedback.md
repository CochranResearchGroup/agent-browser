# Runtime Effect Custody Policy Feedback

Date: 2026-09-14

Product lane: PL-PLATFORM

Disposition: accepted-evidence

Owning plan or work item: CochranResearchGroup/agent-browser#132

Related lanes: P186

## Policy Source

- Installed selector bundle: `repo-policy-selector` v0.1.26
- Source commit: `ead42f7d6932fedc9dea83111b234f954a6afa4d`
- Selected profile: `operations-platform`
- Selection mode: `already-aligned`
- Graph memory assessment: `use`; read-only discovery in
  `agent_browser_main` found no exact prior installer-custody contract.

## Observed Friction

Plan 0186 used an issue comment to name one execution owner, but another session
could still start a production full-shutdown installer. The file lock serialized
the overlapping commands, yet it did not express which session, candidate, or
operation was authorized. After the competing command exited, transaction and
runtime state required a separate reconciliation before the intended preserving
install could continue.

Existing modules say to serialize shared runtimes, coordinate active lanes, and
reconcile agent collisions. They do not define an atomic live-effect lease,
renewal, transfer, release, stale-owner recovery, or command-bound validation.
That omission lets advisory coordination be mistaken for enforceable custody.

## Local Override

Policy `0051-shared-runtime-effect-custody.md` adds the missing Agent Browser
contract. A subsequent operator review corrected its initial authority model:
the runtime control plane protects transaction integrity but does not decide
whether the user may build, install, supersede, recover, or roll back. The
revised policy treats its lease as a compare-and-swap and fencing mechanism,
supports explicit operator-selected transitions, and rejects a permanent
coordinator role. Until implementation is installed, sessions serialize each
production or staging mutation from fresh transaction and runtime evidence.

Plan 0190 owns the deterministic advisory orchestrator. It must report state,
recommendations, alternatives, and consequences without turning ordinary
contention into `permission_denied`. It must reuse sealed build artifacts when
their executable-input closure remains equivalent after merge.

Plan 0190 version 2 adds development-build and test coordination. The existing
development publisher can preserve an explicitly selected binary as an
immutable development generation, but the ordinary `ci` build differs from the
production Cargo profile and is not promotable. The planned path builds one
production-shaped release artifact, tests those exact bytes in an isolated
development namespace, and later reclassifies the same artifact for production
only after merged-source ancestry, executable-input equivalence, complete
embedded assets, bound test receipts, and production preflight all pass.

## Upstream Candidate

The shared policy library should consider a reusable runtime-effect
coordination module for operations platforms with concurrent agent sessions.
It should distinguish user authority, advisory workflow, logical transactions,
and short physical locks. Runtime fencing may reject stale writers to preserve
integrity, but it must not become an agent-role permission service.
