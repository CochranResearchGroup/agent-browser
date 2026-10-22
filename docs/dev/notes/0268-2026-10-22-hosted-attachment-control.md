# Hosted attachment and cooperative control source evidence

Product lane: PL-PLATFORM
Disposition: accepted-source
Owning plan or work item: P268 / #349 and P267 / #350
Related lanes: #347, Bots #162 and consumer plan 0050

## Workflow

An app can attach public Agent Browser workflow operations to its exact observed
browser process, canonical profile, local CDP endpoint and selected existing page.
Attachment omits initial navigation and cannot acquire, replace, relocate or close
the app browser. The owner explicitly names sharing recipients; independent app
sessions receive independent logical task/tab identities over the same physical
browser. Preexisting pages and popup pages remain borrowed. Later task-created
pages retain ordinary scoped task custody. Profile conflicts explain alternatives
without starting a duplicate process. Borrowed profile and task state publish
atomically, including restart and failed-publication fences.

The public browser_control tool joins a durable browser-scoped journal to existing
MCP, CLI and service dispatch. Viewing does not pause automation. Explicit takeover
blocks new mutations, optionally pauses observations/captures, and retains in-flight
outcomes. Explicit return requires fresh same-browser observation without recovery
or relaunch; no timer resumes automation and no uncertain website effect is replayed.
The cooperative-controller contract excludes interception of unmanaged OS/CDP input.
An attributable operator override uses existing trusted admission.

The optional Bots consumer takes attachment descriptors only from its trusted app
observer. HTTP callers cannot supply process, profile-directory or CDP identity.
The existing Native toolbar pauses Agent Browser before native takeover and releases
its own exact Native grant before returning automation. Pending intents remain held
with an explicit readback path. A different handoff or ordinary Native grant cannot
be released by hosted return. Read-only status does not renew expired Native grants;
reopen after expiry remains a separate reconnect/loss outcome.

## Candidate and source custody

Provider PR [361](https://github.com/CochranResearchGroup/agent-browser/pull/361)
precedes consumer PR [175](https://github.com/ecochran76/grok-bots/pull/175).
The optimized ci candidate from clean source
`ef89514cb28a5da1f8821f623a0225487ced5946` has SHA256
`4dbbd5dffec747141f2eb1fcbfa8ee4c17c04ca284aaff84f7fd6d2d3d08d679`.
Later source 9fafa94d6 changes only the wrapper test's in-memory fixture setup.
Bots source 4377ac814d505c4da1375ff43ef42377b4222829 supplies the actual app shell.
These are isolated source fixtures, not installed public app adoption or deployment.

## Validation

- Service model: 375 tests passed across 22 binaries, including compile-fail docs.
- Session regressions: 169 passed; nine real-browser cases intentionally ignored.
- Control: 11 passed, including concurrent dispatch/takeover, restart, pending
  outcome reconciliation and no-recovery fresh return.
- MCP: 138 passed. Native actions: 98 passed.
- Final configured runtime wrapper: eight passed after the forwarding repair.
- Strict workspace Clippy and formatting passed after that repair.
- Service client, feature parity, handoff guidance, policy wiring, planning,
  changed links and documentation build passed for their changed surfaces.
- Bots: 75 focused contracts and eight source-shell/SDK UI checks passed.
- The exact repaired candidate passed standalone original-file retrieval with
  task/profile/process/executable continuity. Fresh OS command-line readback
  found no process referencing its private profile after task closure.

## Joined Native evidence and corrected acceptance

Native run 03 preserved the app's exact process, both original pages, unfinished
input, local storage and the peer page across attachment, public workflow actions,
explicit sharing, takeover/return and both task detachments. Twelve public control
checks exercised actual MCP, CLI and isolated HTTP dispatch. The actual authenticated
Native canvas showed 63 synthetic marker samples and 14 WebSocket frames. Real
Native toolbar takeover/return agreed with both owners and retained task identity.
Metadata closure emitted sequence 2 and matched a fresh ended-task read.

This run is feature evidence, not final cleanup or whole-app visual acceptance.
Visual review exposed an incomplete fixture seed missing submissions. Fresh OS
inspection also disproved its environment-only zero-process census: directly
spawned app browsers did not retain that marker and survived. The exact recorded
browsers and their harnesses were subsequently closed using process/start,
executable, private profile and parent evidence. Fresh readback confirmed their
absence, no private-profile process references and both exact assignments inactive.
Original receipts remain intact with separate correction artifacts outside Git.
Historical descendant coverage is not reconstructed from the faulty census.

Native run 04 qualifies the corrected fixture against the same candidate. The
actual source chat pane reports healthy synthetic chat and the Native SDK viewer
renders 63 marker samples with 26 WebSocket frames. Attachment, scoped sharing,
public MCP/CLI/service control, real toolbar takeover/return, fresh observation and
ended-task event/readback pass without replacing the app browser or its pages.

Cleanup now forces the fixture marker after Native environment merging and records
the directly spawned browser's PID, start generation, UID, executable and private
profile before attachment. Its independent kernel session and freshly confirmed
ancestry identify descendants even when the marker is absent. Fourteen observed
app process generations are kernel-absent before the exact fresh assignment is
released; the private-profile-reference fence and ordinary owned census also pass.
Independent readback confirms the harness absent and the assignment inactive.
This repairs the acceptance harness; it does not retrospectively validate run 03's
cleanup proof or establish why its browser environment marker was missing.

Native run 05 additionally qualifies synthetic cookie continuity through every
stage, including both task detachments, against the same candidate. The exact app
process, both pages, unfinished input, local storage and peer page remain intact.
Twelve public control checks and real Native SDK takeover/return agree. Visual
inspection confirms healthy chat and the actual 1472 by 960 Native canvas, with
63 marker samples and 47 WebSocket frames. Cleanup reports no errors or ambiguity,
zero private-profile references and all 14 observed process generations absent
before the exact assignment release. Independent OS readback confirms all 14 and
the harness absent; the harness also exits naturally with status zero. This is the
final accepted synthetic source workflow boundary.

P268/#349 and P267/#350 close as integrated source outcomes through provider PR361
and dependent consumer PR175 in provider-first order. Merge and publication are
root-owned actions; this note does not assert a production deployment. No private
account workflow or baseline resource was used. Spec #347 and its wider dependent
outcomes remain open.
