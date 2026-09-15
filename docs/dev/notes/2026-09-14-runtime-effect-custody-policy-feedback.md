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
contract. It makes the runtime control plane authoritative, treats file locks as
serialization only, binds every covered effect to an exact lease revision, and
defines transfer and stale-owner recovery. Until implementation is installed,
one coordinator session serializes production and staging effects with fresh
readback before every covered command.

## Upstream Candidate

The shared policy library should consider a reusable `shared-runtime-effect-custody`
module for operations platforms with concurrent agent sessions. The module
should remain separate from generic active-lane and collaborative-development
policy because source coordination and runtime effect authority have different
lifecycle and enforcement requirements.
