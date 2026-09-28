# P219 Remote View Supersession and Custody Audit

Date: 2026-09-28

Product lane: PL-PLATFORM

Disposition: active-input

Owning plan or work item: Plan 0220 and CochranResearchGroup/agent-browser#202

Related lanes: P219

## Decision

Plan 0219 is cancelled as superseded while incomplete. Its authorized final
fresh-overlay attempt failed before installed acceptance, and its stop rule
prohibits replay. Agent Browser will not repair the Agent Browser-owned
XRDP/Guacamole presentation architecture. Plan 0220 replaces that presentation
path with Remote View public contracts while retaining Agent Browser browser,
profile, CDP, session, affinity, and recovery authority.

The failed `overlay.failed-7c22bdfa505e.qcow2` and
`serial.failed-7c22bdfa505e.log` remain preserved alongside the two earlier
failed overlays. This note does not authorize deletion or another VM attempt.

## Forge receipts

- Agent Browser successor: [Issue #202](https://github.com/CochranResearchGroup/agent-browser/issues/202), created by `ecochran76` at `2026-09-28T22:09:42Z` with `enhancement`, `lane/platform`, `state/triage`, and `effect/live-gated` labels. Idempotency marker: `agent-browser-remote-view-consumer-successor-20260928`.
- P219 disposition: [Issue #195 comment](https://github.com/CochranResearchGroup/agent-browser/issues/195#issuecomment-5879654016), published by `ecochran76` at `2026-09-28T22:09:55Z`. Idempotency marker: `p219-remote-view-supersession-disposition-20260928`.
- Remote View consumer coordination: [Remote View Issue #70](https://github.com/CochranResearchGroup/remote-view/issues/70).

## Unpublished custody

At audit start, `platform/p211-simple-cold-upgrade` is clean, 36 commits ahead
and zero behind `origin/platform/p211-simple-cold-upgrade`, at `47150749`. The
slice changes 63 files with 4,523 insertions and 1,461 deletions. The branch is
preserved in its existing worktree and must not be merged wholesale.

The supersession and transition-custody commits `7fd03d0d` and `6af49f67`
were then added on that preserved branch, bringing it to 38 commits ahead of
its published topic ref without changing the audited implementation slice.

## Initial commit disposition

### Retain

These presentation-neutral groups should be extracted or integrated after
focused impact review:

- `54fed6c8`, `4ed5820d`, and `64dc04a6`: Browser Runtime SQLite storage,
  verified backup, integrity, and custody.
- `fe46f53c` and `e41265ce`: exact navigation-history compaction.
- `91cb79c8`, `9416d265`, `9aad8500`, and `734a4cea`: runtime health,
  observability, and browser-launch resource admission.
- `d32e1ea8`: strict presentation-request queue priority, after renaming or
  adapting presentation-specific vocabulary.
- `9339b54f`: runtime configuration convergence history.
- `c1881d39`, `7d043e45`, `b5d72e04`, `c5f4db2a`, `8acfc247`, `92b2155f`,
  `50e0353c`, `d5e5bab7`, `98c9e5df`, `26898035`, and `6f91709e`: truthful
  status, typed repair recommendations, read-only doctor boundaries, and
  disposable probe isolation, subject to removal of retired provider fields.

### Adapt

These groups contain useful invariants but currently depend on the old
presentation model:

- `9f6505b7` and `b47c672a`: preserve the zero-privilege healthy-rerun and
  narrow-receipt principles, but remove XRDP/Guacamole helper actions.
- `7512b41a`: preserve exact-reference scale-in safety as consumer-side release
  admission, while Remote View owns desktop capacity and deletion.
- `4af5155f` and `7560162a`: migrate only ledger evidence that remains valid
  after the ownership boundary changes.
- `2aef4619` and `47150749`: retain causal-error preservation and the terminal
  failure record, but do not carry the cold-install controller forward as the
  Remote View integration mechanism.

### Retire from the supported path

- `8dcbfd89`, `d8b257e3`, `7e34acce`, and `7d840169`: Agent Browser-owned
  Guacamole namespace deletion and reconstruction.
- `f782a9be`: candidate publication as acceptance input for that retired
  provider-rebuild path.
- `b806f214` and `1656b993`: legacy-authority retirement remains historical
  installer evidence, but it is not a prerequisite for Remote View consumer
  operation.
- XRDP route users, route-specific Guacamole pools, Agent Browser-owned display
  provisioning, and presentation-specific privileged-helper capabilities in
  the mixed installer and fixture files.

### Evidence-only

- All P219 checkpoint commits remain custody and validation history even when
  their source payload is retained, adapted, or retired.
- The three failed VM overlays, serial logs, installed-candidate identities,
  provider fixtures, and PostgreSQL unrelated-row preservation results remain
  historical evidence and are not successor acceptance.

## File-level extraction rule

The initial classification is by coherent commit group, not permission to
cherry-pick blindly. Mixed files such as `cli/src/install.rs`,
`cli/src/workstation_convergence.rs`, `cli/src/workstation_install.rs`,
`scripts/install-agent-browser-privileges.sh`, CLI help, README, the shared
skill, docs, schemas, generated clients, the coverage ledger, roadmap, runbook,
and lane catalog require hunk-level reconciliation against the Plan 0220
ownership boundary.

Provider-neutral files added for runtime storage, launch admission, session
state, status projection, and pure queue behavior should be evaluated first.
Provider scripts and workstation fixtures are presumed retired unless a
specific Remote View consumer contract demonstrates reusable behavior.

## Next gate

The P220 branch is admitted from `origin/main@a3848e16` with one primary owner.
Its first provider-free packet freezes the Remote View F0 consumer boundary and
creates the Agent Browser fixture. J1 through J3 contract replay and selective
retained-domain extraction remain next. Live Remote View, installed runtime,
privilege, public ingress, merge, release, and destructive cleanup remain
separately gated.
