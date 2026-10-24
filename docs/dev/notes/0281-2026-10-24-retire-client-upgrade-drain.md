# P281 upgrade client lockout retirement

Product lane: PL-BUGFIX
Disposition: accepted-evidence
Owning plan or work item: P281 / CochranResearchGroup/agent-browser#394
Related lanes: none
Work item: CochranResearchGroup/agent-browser#394
Pull request: https://github.com/CochranResearchGroup/agent-browser/pull/397
Source checkpoint: 3d94bf152e27814101859e0425482b895e7a4c45
Reconciled executable checkpoint: d58c08184567bbe3086ce97b241c09b2ee526c7d

## Reproduction and behavior

A persistent admission marker rejected ordinary native and managed browser
effects without checking whether an installer was still running. An interrupted
post-commit reconciliation therefore became a workstation-wide client outage.
The operator requested retirement of this deprecated client block.

Three new managed workflow regressions first failed with
`runtime_admission_draining` before dispatch. The repair deletes the global
rejection function, claim exceptions and all ten native/managed gate sites.
Valid and malformed legacy markers no longer block explicit opening,
implicit opening, lazy task-tab acquisition, navigation, physical recovery or
native closure. Exact owner-transfer, hosted-control, profile and publication
fences remain in their existing owners.

Legacy marker producers and transaction metadata remain compatible with guarded
installer recovery. That metadata does not grant a client-wide lockout. The
installer fixture now expects ordinary profiles to reach the deliberately
invalid executable instead of being rejected by global metadata. The real
browser fixture exercises new task creation and original-task continuation
with both valid and malformed metadata, including across owner restarts.

## Validation boundaries

The four focused repaired cases pass. The surrounding consumer run passes
107 cases with one explicit real Native viewer fixture ignored (648.09 seconds).
That run began before the concurrent merge; its consumer module and fixture
inputs are unchanged by that merge. Final merged-source checks pass 17 dispatch,
24 owner-transfer, 51 adoption, one preserving-installer and three physical
current-selection/recovery tests. These are focused source gates, not
comprehensive or cross-platform CI evidence. GitHub CI remains disabled.

Workspace strict Clippy, formatting, policy wiring, handoff documentation,
changed local links and the merged docs build pass. Cargo runs use the safe
admission wrapper with the compiler cache disabled after its identity probe
stalled. The final source candidate uses the ordinary debug profile and is
not production-promotable.

The d58c08184 tested debug binary SHA-256 is
`9a1951f9cd4654649d802b8ef442c7b154203ff100b99e184e2365179f57ae5f`.
Its isolated real Chrome fixture passes both host-exit cycles (55 ms each),
new disposable task opening/closure, and same-task continuation while valid
then malformed legacy metadata remains present. The original browser PID,
CDP websocket and occupied profile lock survive. Fixture cleanup and a fresh
independent process scan both report zero owned residue. This is one binary
with two sequential host generations, not workstation-wide or multi-binary
failed-upgrade shutdown acceptance. The full workstation installer fixture passes, including ordinary profile
admission with legacy metadata, failure injection and recovery. Its cleanup
retires two owned hosts with zero residue; an independent OS scan also finds
no process belonging to that exact fixture root.

PR398 entered main after this candidate freeze. Its exact current-profile
endpoint fallback and four qualified tests are retained unchanged: that change
only discovers a nonzero port from the already proved process when its port
file is absent. It neither reads installer metadata nor changes this repair's
native/managed gate deletion or consumer fixtures. No shared source hunk needs
resolution. Retain each lane's evidence within that scope; this fixture binary
is not represented as a build of the later final canonical merge.

## Operational mitigation

The exact production marker was archived outside Git while holding the
coordination and workstation locks. Readback established no active install
custody, selected executable checksum equality and an active supervisor on
that selected executable. The original failed upgrade transaction and browser
processes were preserved. Production MCP disposable open and task-scoped close
then passed. The marker remains absent. Private operation receipts stay in the
user-scoped runtime tree.

The original installation still reports operator recovery required. Neither
this mitigation nor source qualification attests upgrade acceptance or deploys
this source binary. Future publication must use the canonical installed
candidate workflow. No formal release, provider replacement, private-account
operation or historical shutdown program is included.

## Installed checkpoint

The combined generation bf263f69342b is installed and accepted. See [P282 installed acceptance](0282-2026-10-24-recovery-dashboard-manifest.md#combined-installed-acceptance-2026-10-24) for exact source, public workflow, transfer, warnings and preservation limits. #393 and #399 remain separate.
