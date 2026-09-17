# Plan 0211 | Simple Cold Upgrade

Date: 2026-09-17

Plan version: 1

State: OPEN

Consolidation: required

Product lane: PL-PLATFORM

Lane: P211

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`

Branch: `platform/p211-simple-cold-upgrade`

Target: `main`

Integration: merge through the protected pull-request workflow after provider-free shutdown, cold-install, restart, and documentation validation

Source baseline: `fb616aeed2233aca06b4379d61af5d34609a804d`

## Objective

Replace the default production hot-upgrade path with one bounded cold workflow:
stop all Agent Browser-owned machinery, install the new payload, and start a
clean runtime. The ordinary operator interface must not require transaction
revisions, census digests, replacement-plan hashes, rollback decisions,
draining, runtime adoption, or semantic recovery orchestration.

Provide one idempotent `agent-browser shutdown` command that promptly closes
Agent Browser-owned browsers, stops Agent Browser units and timers, stops owned
presentation containers, releases Agent Browser leases and runtime ownership,
and leaves profile data present but unowned. Coordination metadata may be
reported as residue; it cannot veto an operator-requested shutdown.

## Current State

The installed recovery on 2026-09-17 required assisted transaction resume,
manual quarantine of a token-only socket, manual repair of ingress fallback
metadata, direct viewer authentication repair, finalize, and generation GC.
The accepted runtime eventually converged, but the process reproduced issues
#181 and #183 and did not meet unattended-install expectations.

Current `origin/main` defaults workstation installation to the preserve-mode
hot transaction. Its full-shutdown alternative still requires a dry-run plan,
a caller-supplied SHA-256 digest, supervisor-takeover census, exact selected
process identity, and the absence of active drains and transactions. Those
preconditions make the terminal operator action subordinate to stale
coordination state.

P211 is admitted from `origin/main@fb616aee`. Issues #181 and #183 are claimed
as in progress. P207 remains the primary writer for overlapping CLI help,
README, agent skill, service docs, and generated service contracts until it
integrates; P211 will rebase before editing those shared documentation
surfaces. P205 owns its service-model extraction and associated Service State
files. P211 will initially keep its state rewrite behind a narrow adapter and
reconcile the final model location after P205 publishes a checkpoint.

Graphiti discovery was healthy but returned no source-backed prior decision
for a simple cold upgrade. Plan 0116 and the current hot-upgrade implementation
are advisory history, not constraints on this replacement contract.

## Consolidated Batch

1. Add a deep cold-shutdown module with one external operation and injected
   process, unit, container, and state adapters for provider-free tests.
2. Route `agent-browser shutdown` through that module. Use fixed short phase
   deadlines, polite termination followed by exact owned-process escalation,
   and idempotent postcondition checks.
3. On shutdown, close every Agent Browser-owned browser, release every runtime
   owner and lease, mark retained profile definitions unowned, preserve profile
   directories, and remove only Agent Browser-owned transient coordination
   artifacts.
4. Make ordinary workstation apply and reviewed-candidate install use cold
   shutdown, payload replacement, unit activation, and a bounded readiness
   probe. Do not create a new hot-upgrade transaction.
5. Park hot-upgrade mutation commands as legacy recovery/readback surfaces.
   They may inspect or close old transactions but are not selected by the
   default install or upgrade command.
6. Update CLI help, README, agent skill, documentation site, and inline docs to
   present the one-command workflow and remove hot-upgrade ceremony from the
   ordinary path.

## Scope And Effect Boundary

Expected implementation writes are limited to the CLI command router and help,
a focused cold-shutdown/cold-install module, the smallest required adapters in
the workstation installer and native runtime, provider-free fixtures, README,
the Agent Browser skill, installation documentation, this plan, and P211's
active-lane projection.

The command may affect only Agent Browser-owned user units, timers, browsers,
runtime hosts, dashboard processes, MCP processes, presentation processes and
containers, leases, ownership records, and transient runtime metadata. It must
not kill unrelated browsers or containers. Unknown foreign processes are
reported and left alone; stale Agent Browser bookkeeping does not block the
owned shutdown set.

This plan does not authorize a production install, production shutdown,
credential or provider use, release, broad process cleanup, deletion of profile
data, or mutation of unrelated worktrees. Installed validation remains a
separate user-directed effect after source qualification.

## Delivery Sequence And Budget

- Critical path: red shutdown fixture, cold-shutdown module, CLI command,
  profile release, cold installer routing, restart fixture, documentation.
- Slice 1: freeze the shutdown and cold-upgrade result contracts and add red
  provider-free fixtures for broken transaction state, active drain, stale
  metadata, partial retry, and a clean machine.
- Slice 2: implement `agent-browser shutdown` and the bounded owned-target
  adapters.
- Slice 3: route workstation and reviewed-candidate apply through
  stop-replace-start while leaving legacy hot transaction inspection intact.
- Slice 4: synchronize all required documentation and run changed-surface
  validation once against the consolidated candidate.
- Maximum work-unit attempts: 3 per slice.
- Maximum review and rework cycles: 1.
- Maximum consecutive hardening checkpoints: 2.
- Reassess after two checkpoints or 30 active minutes without outcome progress.
- Overall effort ceiling: 240 active minutes through a provider-free qualified
  candidate. Installed-runtime validation is excluded until separately
  directed.
- First outcome artifact: a provider-free fixture proving stale drain and
  failed transaction metadata cannot block shutdown.

## Worker Assignments

The P211 lane owner retains architecture, implementation, source custody,
validation, and acceptance. No subagents are assigned. Deterministic repository
and test tools perform discovery and validation.

P205 remains primary writer for the service-model extraction and its migrated
Service State types. P207 remains primary writer for its current CLI help,
README, skill, service documentation, and generated-client changes. P211 owns
the new cold-shutdown module, workstation-install routing, and its tests; it
will rebase after those checkpoints before touching overlapping surfaces.

## Evidence And Exit

| Requirement | Acceptance evidence | Current state |
| --- | --- | --- |
| One-command shutdown | `agent-browser shutdown` fixture returns success from healthy, drained, failed-upgrade, and partial-prior-run inputs | not implemented |
| Bounded completion | injected-clock tests prove fixed phase deadlines and exact escalation without production-scale sleeps | not implemented |
| Complete owned shutdown | receipt proves owned units, timers, browsers, runtime hosts, dashboard, MCP, and owned containers are stopped | not implemented |
| Profiles become unowned | fixture proves profile data remains while runtime owners and leases are released | not implemented |
| Metadata cannot veto | stale hashes, tokens, transaction revisions, drains, and missing prior processes become warnings rather than blockers | not implemented |
| Cold replacement | workstation and reviewed-candidate apply execute stop, replace, start, and readiness in that order | not implemented |
| Clean restart | post-start fixture proves one selected generation, one runtime host, one dashboard, and clients can make a fresh service request | not implemented |
| Simple interface | default operator path requires no preflight digest, transaction ID, revision, census code, rollback choice, or manual recovery command | not implemented |
| Legacy containment | hot transaction mutation is not reachable from the default install or upgrade path | not implemented |
| Documentation parity | CLI help, README, Agent Browser skill, docs site, and inline comments describe the same workflow | not implemented |

Exit requires all rows green against one frozen source candidate. Provider-free
tests must include idempotent replay, a shutdown interrupted after each phase,
stale PID metadata, exact foreign-process preservation, owned container
cleanup, browser close escalation, ownership release, and restart readiness.

## Stop Condition

Stop before production installation or shutdown, profile-data deletion,
unscoped process or container termination, live provider or credential use,
release, or mutation of another lane's checkout. Stop and reconcile if P205 or
P207 publishes an overlapping interface change before P211's corresponding
adapter or documentation work begins.
