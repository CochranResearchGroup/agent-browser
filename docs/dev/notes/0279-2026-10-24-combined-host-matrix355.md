# Combined actual-host acceptance for355

Product lane: PL-PLATFORM
Disposition: accepted-evidence
Owning plan or work item: P279; CochranResearchGroup/agent-browser#355
Related lanes: P270, P277, P278

The complete synthetic matrix passes on one frozen pair and one newly owned
target/peer Native installation. Matrix6 reports ACCEPT with qualified child and
Native cleanup, no ambiguous effect and no adapter error. The workload completed
in148.30seconds; total time including Native cleanup was151.77seconds, within the
declared1,200-second workload and180-second cleanup budgets. This closes the
remaining combined acceptance gate. [PR388](https://github.com/CochranResearchGroup/agent-browser/pull/388)
entered canonical main as `2ae94c2a3a91cbb7ed4b27181ac44e1078deb968`; #355
and parent #347 are closed under the previously approved relocation deferral.
Preparation and diagnosis span about110minutes from the first P279 custody
commit to accepted final readback, including retained fixture failures; this is
wall time, not a measurement of aggregate parallel-worker effort.

## Frozen identity

Harness source `e1f40c2575d6494d1feb6d8b4b5d6569c864a284`; consumer source
`3118512a78bae1489db2474b621e2dcbb5a74301`. Agent Browser executable source
`704afe4e96b1bcda0ddf4c5661c9ee6859e5821b`, sealed SHA256
`29370244be31332bc497f31351cb19bfc5dc220ad4a27d4417fe0298a3714505`.
This reuses P278's qualified optimized diagnostic candidate; it is not a formal
release or production-shaped adoption.

Native executable SHA256
`0eb8d8f67cd9fb5b510ac49bfd952e902d89a0478ddc546bed00386dc0772bf5`,
whose isolated install reports source `e8cee02b050bcbdf0e53decab9675b3fe9090f32`.
Native fixture source is `0d7c99edcf500aaeee2a52b3c173ec326bd54a00`.
The Native checkout advanced during preparation; the run freezes its actual
source and independently verifies the producer file hashes afterward. The
reviewed Chromium150.0.7835.0 executable SHA256 is
`aebeac48273efa3a2767763cf0694cfa8f1be52c91b7fbafff0d4698a993ffce`.
Private copies pin both runtime executables; the shared installed Native image
changed independently during preparation and was not used as a moving target.

The complete source/transport pair digest is
`2b8e1fceb18a2cb75f7c72e48d7855f8f77334def9149160771f089aef041ab1`.
Four harness file digests and all producer source identities are retained with
the pre-effect workload manifest. Runtime responses are never combined across
pair digests or runs.

## Qualified workflow

The actual Bots service attaches to one app-owned Chrome process, canonical
profile and existing page, then explicitly shares it with a second participant.
Distinct logical tasks preserve process/tab identity, unfinished draft, cookie,
local storage and the pre-existing peer page. The app's durable handoff renders
the real Native canvas over an actual Native WebSocket; app HTML alone is not
viewer proof.

Viewing remains agent-owned. The Native toolbar takes human control with page
reads paused, then actual Native pointer/keyboard input moves focus to the two
synthetic fields and edits the intended field once. Independent app-owned CDP
readback confirms the text and preserved draft. Explicit return reconciles both
Agent Browser and Native ownership. The public control fixture also qualifies
CLI/MCP/HTTP mutation refusal, optional read/capture pause and fresh return.

Disconnect/reconnect, compact metadata snapshot/change events, scoped
restrictions and override, local artifact delivery and scoped task closure pass.
Detaching both borrowed tasks preserves the app browser and existing pages.
A separately created managed task loses its exact owned process, reports loss,
then explicitly reopens the same logical task/profile without website-effect
replay. The original app and Native peer remain unchanged.

Before restarting Native, the fixture stops its viewer and app service, confirms
zero viewer WebSockets, retires exact tracked grants and independently checks
Native's routes, grants and admitted sessions. Native's actual stop/start then
retires old desktop processes and advances viewing generation1 to2 on the same
assignment. Assignment lifecycle generation remains1. Explicit public rebind
records recovery intent; one explicit reopen preserves logical task/browser/tab/
profile identity and opens an empty `about:blank`. Native independently observes
the replacement PID/window on viewing2. Peer process births and protected files
remain unchanged. Existing-browser relocation578 remains deferred.

## Measured boundaries

| Workload | Validated | Failed or unknown | Median | Nearest-rank p95 | Target |
| --- | --- | --- | --- | --- | --- |
| Public control response |20/20|0|6.73ms|32.57ms|strictly below250ms, pass|
| Consumer/Native coordinated response |6/6|0|115.24ms|169.85ms|strictly below250ms, pass|
| Cold Native first pixels |5/5|0|4,444.73ms|11,061.81ms|reported separately; no invented threshold|

Warm clocks end at decoded, semantically validated responses and exclude UI
pointer/focus completion. The separate functional Native edit proves input;
its completion is not mislabeled as a warm acknowledgment sample. Cold uses
five sequential fresh browser contexts, with no overlap or replacement. Each
starts its clock at durable app navigation and ends at independently read
synthetic canvas markers plus received Native WebSocket frames. Cold minimum
is4,312.69ms and maximum11,061.81ms. The tail remains slow; these isolated
synthetic numbers do not establish production dashboard cold-open latency.

## Retained failures and fixture repairs

Private receipts remain under `~/.local/state/agent-browser/acceptance355-20261024/`.
Matrix1 stops at interpreter preflight before installation. Matrix2 correctly
refuses a loopback public origin. Matrix3's first cold attempt fails before any
Native traffic; a small transport probe reproduces Chromium's
`ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`. One origin-scoped permission in
the disposable context changes that probe to a real authenticated canvas pass.
No global security flag or shared DNS/hosts change is used.

Matrix4 supplies its own five-context distribution but fails the human input
test. Input1/2 add readiness/focus evidence; input2 confirms connected control
and unlocked input before a paint-layer click times out. The actual Native
interaction surface fixes that locator. Input3 independently qualifies the text
edit/return but its outer verdict stays incomplete because Native release
revoked a grant before the adapter's local tracking reconciled it.

Matrix5 qualifies both warm workloads, cold pixels, input and managed loss, then
stops before Native restart: the adapter incorrectly applies control-only
inactivity retirement to observation grants. Public HTTP tests reproduce this.
Observation uses normal revocation; control retains inactivity retirement.
Already-revoked grants reconcile only from exact Native owner/target state with
no retained route or admitted session. Matrix6 qualifies the complete corrected
matrix. No earlier attempt is overwritten, spliced into matrix6 or retried as an
uncertain effect. P277's original Native8 unknown launch remains historical,
unexplained evidence; this pass does not erase it.

## Cleanup and validation

Independent readback proves47retained Agent Browser kernel generations absent,
no private agent-home/socket references, one released target assignment and the
unchanged Native peer until its owned teardown. Native cleanup reports zero
owned processes, containers, networks and volumes. Both uniquely tagged images
from every installed run are removed without force or global pruning. The
consumer auxiliary's exact private dependency symlink is removed, followed by
governed clean auxiliary closeout. Foreign dirty checkouts and candidates are
preserved.

Provider-free checks pass:53Node hosted oracle/ownership/cleanup/performance
cases,15Python RPC scope/generation/grant cases and4Native-store retirement
cases. JavaScript syntax, Python compile and changed documentation links pass.
P278's qualified Rust gates cover the exact704afe4e candidate used here; this
packet changes fixtures, clocks, tests and evidence, not Rust. Current-checkout
artifact equivalence is not claimed: its generated CDP build output is absent
and its generated dashboard index differs from the archived candidate. The run
qualifies the pinned executable, not a newly built production candidate.
GitHub CI remains disabled by operator direction. No production install,
private-account login, shared skill publication or relocation effect occurred.
