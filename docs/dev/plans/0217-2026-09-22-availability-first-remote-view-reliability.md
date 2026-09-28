# Plan 0217 | Availability-First Remote View Reliability

Date: 2026-09-22

Plan version: 3

State: CANCELLED

Consolidation: required

Product lane: PL-PLATFORM

Successor: [Plan 0218](0218-2026-09-23-grilling-contract-remote-view-conformance.md)

Lane: P217

Predecessor: [Plan 0211](0211-2026-09-17-simple-cold-upgrade.md), superseded after its integration-ready verdict was withdrawn

Work items: `CochranResearchGroup/agent-browser#181`, `CochranResearchGroup/agent-browser#183`, and `CochranResearchGroup/agent-browser#195`

Branch: `platform/p211-simple-cold-upgrade` with inherited P211 custody

Pull request: draft PR #191

Target: `main`

Overall effort ceiling: 2,000,000 cumulative tokens across implementation, validation, review, documentation, and every successor or retry

## Objective

Make the trusted single-user remote-view path simple and dependable. Ordinary profile open must return a working browser route even when historical ownership or cleanup evidence is stale. Guacamole/XRDP must repeatedly show the current browser correctly and accept desktop and mobile input through the durable authenticated handoff.

Success removes denial-first lease authority from the ordinary path and proves the visible operator experience. A ready protocol, process witness, URL, doctor result, or unit test cannot substitute for rendered pixels and responsive input.

## Current State

P211 source `9e7c5719` and installed development generation `0.28.0-1d808efe0797` passed provider-free qualification, development doctor, three disposable browser cycles, one named-profile ready handoff, exact close, and residue checks. Commit `131fc5a2` withdrew the integration verdict because that evidence did not prove the original product requirements.

The ordinary path still carries route-generation, quarantine, cleanup-obligation, and ownership checks capable of withholding service. Four warm XRDP routes were protocol-ready on displays `:20` through `:23`, but there is no accepted repeated external desktop/mobile proof for correct rendering, z-order, input, resize, or restart recovery. Draft PR #191 is a review vehicle and is not ready for integration.

## Acceptance Contract

1. A named profile opens through the ordinary client path and returns one authenticated opaque `/remote-view/<handoff-id>` with `operatorVisible.state=ready`.
2. Stale or ambiguous historical ownership may prevent destructive cleanup of the exact uncertain resource. It cannot create a phantom capacity reservation or prevent selecting, creating, or restarting a separate working route when real capacity is available. When real capacity is exhausted, the ordinary path returns a concrete capacity failure without destructively reclaiming an uncertain resource.
3. The ordinary path does not consult lease claims, fencing, custody, quarantine, cleanup obligations, or equivalent denial-first authority to decide whether the user may receive a working route.
4. Repeated external desktop and mobile checks cover every route in the M3-frozen candidate and provider inventory and show current browser pixels on a clean desktop with correct visibility and z-order.
5. Focus, pointer, keyboard, scrolling, viewport resize, and reconnect remain responsive through the same durable handoff. Browser, automation-runtime, XRDP-route, Guacamole-connection, and provider restart cases use the recovery identity and timing contract frozen at M0; required continuity always preserves the same authenticated operator URL and handoff ID even when the underlying route changes.
6. Terminal-only desktops, blank or mostly white frames, stale frames, partial rectangle damage, hidden browsers, unresponsive windows, wrong-route tiles, and stale-metadata service denial are acceptance failures.
7. Close leaves no unowned browser or daemon tree. Exact uncertain foreign resources remain untouched and visible as diagnostics without blocking new service.

## Consolidated Batch

- Reduce the ordinary open, route selection, close, and recovery path to the minimum availability-first state needed for one trusted user.
- Delete or bypass P211 lease-era machinery that can deny ordinary service on the trusted single-user service-admission call path. Retain exact non-destructive protection at cleanup boundaries, and do not delete the extracted Lease Authority library or adversarial and multi-tenant contracts merely because the ordinary path no longer consults them.
- Repair the existing Guacamole/XRDP launch and presentation path for clean desktop state, correct browser foregrounding, complete rendering, responsive input, and bounded restart recovery.
- Qualify one frozen candidate provider-free, install it once in the isolated development runtime, and run one visual-operational acceptance campaign.
- Reconcile the draft PR and canonical status only after both product axes pass.

## Scope And Non-Goals

Included:

- ordinary named-profile open, route selection, recovery, close, and durable handoff resolution;
- removal or bypass of denial-first lease and equivalent ownership gates from that path;
- existing Guacamole, XRDP, desktop session, browser launch, focus, and route presentation adapters;
- provider-free regressions and bounded development-runtime visual acceptance;
- required CLI help, README, skill, docs-site, inline documentation, roadmap, runbook, and active-lane parity for changed behavior.

Excluded:

- multi-tenant or adversarial lease authority;
- a new lease, scheduler, allocator, reconciliation, or quarantine framework;
- dynamic fleet scaling, generalized desktop orchestration, or unrelated Service State extraction;
- production installation, formal release, automatic CI restoration, or upstream publication;
- private-site browsing or capture during visual acceptance.

## Delivery Sequence And Budget

### M0 | Executable cut line and baseline | 200,000 tokens

Trace the ordinary named-profile open through route selection, browser launch, handoff publication, close, and recovery. Record the exact denial points to remove and the smallest provider-free reproducer for stale ownership denying a fresh route. Capture one visual baseline that distinguishes protocol readiness from actual rendering. Freeze a recovery table for browser, automation-runtime, XRDP-route, Guacamole-connection, and provider restart with the required URL, handoff, profile and session identity, maximum recovery interval, and allowed interaction interruption for each case. Record the current source head, installed generation, provider manifest digest, and observed route inventory with the baseline. End M0 with a file and symbol cut line plus failing or characterizing evidence in the runbook or a routed evidence note; do not create another plan or architecture document.

### M1 | Availability-first ordinary path | 650,000 tokens

Implement the smallest change that makes working-route acquisition independent of stale lease-era authority. Preserve exact cleanup safety without allowing uncertain history to reserve capacity or deny a new route. Remove dead ordinary-path machinery and add focused tests for stale owner, quarantine, cleanup obligation, restart, and concurrent open cases. Exit only when those cases produce service or a concrete capacity/process failure unrelated to historical authority.

### M2 | Simple reliable Guacamole/XRDP path | 550,000 tokens

Repair only observed presentation defects in the existing path. Ensure a clean desktop, one visible foreground browser, correct route binding, responsive input, complete repaint and bounded recovery after browser, runtime, and route restart. Do not add a second presentation controller or general scheduler. Freeze one candidate after focused and changed-surface validation passes.

### M3 | Visual-operational acceptance | 400,000 tokens

Install the frozen candidate once in the isolated development runtime under fresh shared-runtime effect custody. Before testing, freeze the acceptance denominator: candidate source and binary identity, installed generation, provider manifest digest, exact route IDs, displays, and route count. Test every route in that frozen inventory from authenticated external desktop and mobile viewports. Use the protected Plan 0158 external-vantage workflow when its contract applies; otherwise record the distinct reviewed manual procedure, operator roles, deliberate route-selection mechanism, synthetic marker, timestamps, visual artifacts, and input receipts before the first case. Exercise focus, pointer, keyboard, scrolling, resize, reconnect, and every restart case in the M0 recovery table. Preserve timestamped route and candidate identities plus synthetic, non-private visual evidence. One repair and one repeat of a failed case are allowed; batch related source defects before rebuilding. Routes appearing after the denominator is frozen are diagnostics for a successor run and cannot silently change this campaign's denominator.

### M4 | Qualification and handoff | 200,000 tokens

Run validation selected from the complete successor diff, reconcile the single evidence table, update PR #191, and state the integration verdict. Use one closed-world review limited to the acceptance contract and regressions caused by the repairs. Do not merge, install production, release, delete the branch, or remove the worktree without separate authority.

## Worker Assignments

The fresh primary agent owns the critical path, branch, candidate freeze, runtime effects, evidence adjudication, and final verdict. No worker is required to start. If delegation later provides real parallel value, use at most one bounded provider-free code task and one read-only visual evidence audit with disjoint files and explicit stop conditions. Workers cannot revise scope, mutate shared runtime, or declare acceptance.

## Controls And Stop Rules

- Start the campaign with: `/goal execute Plan 0217 availability-first remote-view reliability with a cumulative 2,000,000-token ceiling`.
- The milestone caps sum to the overall ceiling. Unused allowance may move forward; no milestone or successor can raise the total.
- Checkpoint only at a milestone boundary, material blocker, required runtime custody transition, or 30 minutes without outcome progress.
- Two consecutive checkpoints or 30 active minutes without outcome progress end the current tactic. Change to one evidence-backed tactic or report the exact blocked criterion.
- Allow one implementation attempt and one repair per milestone. M3 permits one repeated failed visual case after repair, not a full open-ended matrix replay.
- Do not revise this plan to explain failed tactics. Put execution checkpoints in `RUNBOOK.md` or a routed evidence note and keep the objective and acceptance contract fixed.
- Do not replace lease terminology while preserving denial-first behavior. Do not add ownership metadata unless a failing acceptance case proves it is the smallest necessary fix.
- Do not call P217 complete from protocol, process, URL, test, or doctor evidence alone. The visual-operational campaign is mandatory.

## Evidence And Exit

| Requirement | Existing evidence | Required successor evidence | State |
| --- | --- | --- | --- |
| Provider-free source baseline | P211 qualification at `9e7c5719` | changed-surface qualification for frozen P217 candidate | incomplete |
| Availability-first open | explicit-close and route residue evidence | stale ownership cannot deny a fresh working route through ordinary open when real capacity is available; real exhaustion is typed without destructive uncertain-resource reclamation | incomplete |
| Opaque durable handoff | one authenticated ready P211 handoff | same handoff contract across route and runtime recovery | incomplete |
| Frozen route coverage | four protocol-ready warm routes were previously observed | exact candidate, provider manifest, route IDs, displays, count, and a result for every route in that frozen denominator | incomplete |
| Clean desktop and z-order | none accepted | one visible foreground browser on a clean desktop for every frozen route | incomplete |
| Complete rendering | protocol-ready warm routes | repeated current and complete pixels with explicit rejection checks for blank, stale, partial, hidden, and wrong-route frames | incomplete |
| Desktop and mobile input | none accepted | focus, pointer, keyboard, scroll, resize, and reconnect receipts | incomplete |
| Recovery identity and timing | component and doctor checks | every M0 recovery case meets its frozen URL, handoff, profile, session, interruption, and recovery-time contract without stale or blank presentation | incomplete |
| Residue and safety | P211 exact close and process residue checks | fresh OS process census proves no unowned browser or daemon tree; exact uncertain resources remain untouched and diagnostic without creating phantom reservations or denying available service | incomplete |
| Integration | draft PR #191 | all rows complete and closed-world review passes | incomplete |

P217 completes only when every row is supported by current evidence from one frozen source candidate and its exact installed development generation. A partial pass remains OPEN with the failed criterion and reproducible evidence.

## Supersession

Plan 0217 is superseded by Plan 0218 because this plan captured only the
availability-first and visible-operation subset of the accepted September 19
grilling contract. Plan 0218 owns the complete authority, persistence,
provider, capacity, recovery, control, retention, migration, and acceptance
contract without resetting P217 evidence, retry history, or cumulative effort.
Do not resume P217 revisions or implementation packets.
