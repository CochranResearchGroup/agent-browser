# P207: preserve caller custody during tab refresh

Product lane: PL-BUGFIX
Work item: [issue 175](https://github.com/CochranResearchGroup/agent-browser/issues/175)
Baseline: `4cbea18ecf0879b71a578e508cc3770ab1e719b2`
Branch: `fix/p207-current-custody`
State: COMPLETED within the source qualification below
Integration: [PR 239](https://github.com/CochranResearchGroup/agent-browser/pull/239); verify merged status for canonical custody

When two tasks intentionally share a profile, refreshing one task's closed tab
must preserve the other task's tab. Blank pages and matching origins are discovery
hints, not ownership evidence. Issue 175 also requires explicit cleanup-attempt
fields so consumers can accept a repair without guessing.

This repair adapts the retained PR 184 implementation to current main. It owns
only refresh reuse, canonical replacement persistence, duplicate selection and
response evidence. It does not revive the archived platform requirements or
change Remote View desktop allocation. PR 191 remains separate.

Acceptance:

1. A closed caller handle cannot adopt an unrelated blank or same-origin target.
2. Reuse returns the compatible target's canonical caller-bound handle.
3. A new replacement persists caller identity and is usable by subsequent actions.
4. Duplicate cleanup closes only compatible targets with matching canonical
   session, principal, work lease and profile-access custody.
5. Every structured refresh response reports duplicate and peer cleanup attempts.
6. A disposable real-browser workflow preserves the peer target while opening a
   replacement and closing a caller-owned duplicate.

Validation starts with a failing fixture exercising current main's actual
selection function, followed by focused Rust tests, client contracts, formatting,
strict workspace Clippy and the isolated browser workflow. Review Standards and
Spec separately against the pinned baseline and issue 175.

The clean checkout is the primary writer for these surfaces. The dirty primary
checkout and retained PR 184 branch are preserved. The open operator Facebook
profile, production services and other agents' desktops remain outside the test.
No reboot or memory write is authorized by this packet.

## Validation and limits

Current main's actual selection function failed on a peer blank target. The
isolated installed baseline also reproduced the reported peer adoption once the
fixture supplied distinct client subjects. The first live fixture used labels
without establishing independent custody; that attempt is inconclusive.

The source candidate passed a disposable two-subject, one-browser workflow:
replacement creation, subsequent canonical-handle close, compatible reuse with
the canonical tab ID and caller attribution, caller-owned duplicate cleanup,
and peer target/URL preservation. Fresh process readback found no test residue.
Seven existing Rust test bodies passed against unchanged production helper
bodies and the current Service Model types in an isolated fixture executable.
The replacement-persistence invariant was also exercised through the public
live workflow. These are selected checks, not the comprehensive CLI suite.

Workspace format, strict workspace Clippy, service-client contracts, API/MCP
parity, docs build, documentation links, handoff guidance and the active planning audit passed. Resource failures remain separate: the optimized
candidate compile reached its 4 GiB cap; initial Clippy reached 2 GiB before a
4 GiB run passed; the full CLI focused unit binary reached 4 GiB before its tests
ran. That Cargo test lane is not passed. The bounded source decision uses the
actual-function fixtures plus reproduced public workflow and required source
checks; broader unit regression qualification and production promotion are not
claimed. No host-reserve reduction, reboot or production publication occurred.

The policy-wiring check fails an unchanged baseline assertion requiring a direct AGENTS.md link to policy 0047. AGENTS.md and that checker match the pinned baseline; this repair does not change policy loading. The failure is retained separately.

## Standards

Reviewed against AGENTS.md, policies 0031/0042/0050, and the baseline diff.
The repair uses existing Service State custody and persistence adapters. All
user-facing guidance and generated response contracts are updated. Duplicate
classification stays in one helper before caller filtering. No accepted
standards findings remain.

## Spec

Reviewed separately against issue 175 and the six acceptance statements above.
The live workflow proves the reported unsafe adoption is removed, canonical
replacement/reuse works, explicit no-cleanup evidence is present, and cleanup
preserves the peer. Same-origin, unattributed, missing-owner and shared-physical-
session child-subject cases are covered by the selected Rust fixtures. No
accepted spec findings remain; the full CLI unit-build limit is retained as a
qualification limit rather than relabeled as a passed test.

Review summary: Standards 0 accepted findings; Spec 0 accepted findings.
Memory disposition: forbidden; no user authorization for persistent memory writes.
