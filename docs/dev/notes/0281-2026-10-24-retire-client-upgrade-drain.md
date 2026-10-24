# P281 upgrade client lockout retirement

Product lane: PL-BUGFIX
Disposition: active-input
Owning plan or work item: P281 / CochranResearchGroup/agent-browser#394
Related lanes: none
Work item: CochranResearchGroup/agent-browser#394
Pull request: https://github.com/CochranResearchGroup/agent-browser/pull/397
Source checkpoint: 3d94bf152e27814101859e0425482b895e7a4c45

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

The four focused repaired cases pass. Broader consumer/dispatch and installer
checks and isolated real-browser continuity are pending final readback.
Workspace strict Clippy, formatting, policy wiring, handoff documentation and
the docs build pass. These are focused source gates, not comprehensive or
cross-platform CI evidence. GitHub CI remains disabled.

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
