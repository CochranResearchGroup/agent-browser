# Runbook

Sole current execution status. Plan0160 owns scope, strategy, frozen acceptance, and current requirement status; receipts and archives preserve detail.
Keep at or below 200 lines under policy0043.

## Turn 287 | 2026-09-09

Live QBO and BILL recovery found both Guacamole pool entries `available` and
capacity slots `warm_idle`, while orphaned route rows retained absent browser
IDs. The installed validator rejected this as
`browser_missing_outside_pending_acquisition`. QBO and BILL were placed on
`:10` and `:11` for immediate manual authentication with profiles preserved.
Plan0161 adds the missing restart fixture; policy0032 treats prolonged denial
of proven-unoccupied owned resources as an availability defect. The last
production installation required a hard stop because the old runtime did not
surrender. Current census has one live production host, but the supervisor is
inactive with `port_conflict` and seven runtime-inventory rows remain. P116 is
reopened for old-runtime surrender, automatic supervisor replacement, and
single production-runtime convergence. Installed A1, A3, A4, remediation, and
graceful upgrade remain OPEN.

## Turn 286 | 2026-09-09

Authority: [Plan0160](docs/dev/plans/0160-2026-09-06-production-profile-identity-and-operational-readiness.md).
Operator authorized candidate installation and deferred standalone CLI compatibility
to the next round. Installation began19:51:02 UTC with a local15-minute bound.
Source052848aa is now installed; no new build or broad diagnosis was needed.
Full A1–A4/AX remains OPEN. Next round retains ownership/identity as highest
priority, with standalone CLI compatibility explicitly deferred to that round.
Do not restart A4 observation before its prerequisites and complete sequence
budget are ready. No consumer payment/CSV action or unrelated cleanup.

Operator direction now makes profile remediation a first-class product surface.
Plan0160 defines the objective and
[Plan0161](docs/dev/plans/0161-2026-09-09-first-class-profile-repair-and-reset-plan.md)
owns the bounded implementation of read-only profile diagnosis, preserving plan/apply repair, and
explicitly scoped reset across CLI, HTTP, MCP, generated client and dashboard.
The next implementation milestone is diagnose plus preserving repair for the
current owner/lease blocker. Runtime and authentication reset follow. Destructive
profile-data reset remains separately gated and is not required for that first
milestone. No profile or runtime was changed by this planning amendment.

Private receipt prefixes:
- R = ~/.local/state/agent-browser/campaigns/p160/consolidated-20260908/
- P = ~/.local/state/agent-browser/campaigns/p160/publication-028597ea/
- N = ~/.local/state/agent-browser/campaigns/p160/ownership-round-20260908T175133Z/

### Installed outcome

Source052848aa2855be5bae7bedfc6b77448ec4fe9c53 is committed and pushed on
plan/profile-permissions-and-request-provenance. Installed generation:
0.28.0-71834ecdb3d3-5a938052448d; host PID21220.
Binary SHA256: 71834ecdb3d3fe527a968303e31fb4c6c7f8cbf4ac4387d348b801581fd8bd93.
Support SHA256: 5a938052448d039141e190660250589f1c78e70c55cf6f297832ed01155b0bfd.

Native download now preserves peer browser download policy and correlates the
owned frame/GUID with process-bound artifact delivery. waitfordownload is unchanged.
The previous pending-confirm repair and process-only supervisor default remain
included. This replacement interrupted only the exact runtime-host process;
it does not establish graceful shutdown preservation.

N/activation-receipt.json verifies five browser roots, host-unit descendants,
39 tab-custody records and recovered temporary storage preserved. Pre-install
synthetic diagnostics returned missing profile_lease; after installation the
same retained handle returned complete attestation with no missing proofs.
This does not establish the cause or permanent resolution of the earlier gap.
The same durable-link local journey passed with two authenticated viewers,
anonymous denial, reconnect and trusted mouse/keyboard input:
local-viewers-RAELII under a2-operator-journey-r2/access-grant-attempt.
Controller refresh used the driver's existing same-link recovery; no browser or
provider restart occurred. N/installation-postjourney-doctor.json exited0; eight advisory warnings remain.
The interlock timer is restored active. Shared skill sync added only the two
download comments, with backup and delta hashes under N.
Immediate rollback is generation0.28.0-ca2134dd8592-7de46dd5e014. Preserve
0.28.0-0df77d7f2693-f452c93718a3 and0.28.0-b69c4e5a4a60-31faac73c7ec and all holds.

### Requirement-to-evidence table

I = implemented, Q = qualified, D = installed/integrated, U = user-workflow proof.
These stages are independent; partial or unknown is not completion.
R receipts below qualify the prior b2207d5c binary; they are retained evidence,
not fresh executions on052848aa. N receipts identify the current installation.

| Requirement | I / Q / D / U | Decisive evidence | Missing proof and next action |
| --- | --- | --- | --- |
| A1 original identity recovery | yes / pass on f816 / retained source / r4 pass | P/consumer-r4-acceptance.json; incident0156 r4 | Preserve original self-declared recovery; no invented consumer credentialed-rejoin gate. Fresh consumer proof after this replacement remains distinct. |
| A1 named/custom lifecycle | yes / prior binary pass / yes / synthetic | R/post-r4-matrix-results.json | Four headless/headed close-reopen rows include two clients, original handles through isolated host interruption, foreign denial and exact cleanup. Other operation/negative-case joins remain due. |
| A1 session-only lifecycle | yes / prior headless and headed pass / yes / synthetic | R/final-session-matrix-results.json; R/final-session-headed-results.json | Five production identities preserved and no fixture residue. Session-only, named and custom lifecycle cases all have prior-binary proof; broader cross-operation/negative joins remain separate. |
| A1 pending confirm | yes / prior binary pass / yes / original synthetic tab pass | R/installed-modal-acceptance.json; R/final-dialog-bmPf4N | Installed original handle passed labeled confirm status/dismiss and preserved actual page title/marker. Isolated snapshot and wrong-target denial pass. Original consumer retry and cross-target pending-modal routing remain unproven. |
| A1 lease findings | dispositions / scoped / yes / partial | P/post-r4-ownership-dispositions.json; R/primary-evidence-disposition.json | Keep seven advisory axes separate from ordinary control failures. Complete UI/network/file-transfer after peer-target selection and remaining negative joins with isolated fixtures. |
| A1 first-class profile remediation | Plan0161 open / incomplete / no / no | Plan0161; Plan0160 first-class profile remediation amendment | Implement profile diagnose and preserving repair first. The BILL fixture joins stale-owner routing and proven-stale Chrome locks through one recovery launch; also prove legacy-principal repair, foreign-owner rejection, idempotency and peer survival. Scoped reset follows. |
| A2 local remote-view journey | yes / installed052848aa pass with same-link recovery / yes / synthetic | local-viewers-RAELII under a2-operator-journey-r2/access-grant-attempt | Two authenticated viewers, anonymous denial, same-link reconnect and trusted input pass. Preserve initial local-viewers-2wL0BG failure; external vantage remains separate. |
| A3 current readiness | yes / installed doctor pass / yes / n/a | N/installation-postjourney-doctor.json; N/configuration-freeze.json | Preserve warning dispositions and current evidence; doctor alone does not prove complete operation. |
| A3 preserving shutdown delivery | scoped / forced recovery only / installed but unaccepted / no | R/activation-receipt.json; operator correction in Turn287 | Last production install required a full hard stop. Prove one ordinary upgrade that preserves or transfers every owned browser and resumes profiles, tabs, leases, routes, handoffs, and scheduled clients without manual termination or state repair. |
| A4 scheduled/restart continuity | partial / three current steady cycles pass / timer active / incomplete | R/steady-cadence/scheduled-control.json | Normal gaps300.875s,300.398s,300.526s; current configuration and five roots preserved. Startup gap298.8s excluded. Full A1 prerequisites and ordered controlled-restart/next-cycle sequence remain incomplete; no second production restart performed. |
| AX selected seven cases | surfaces implemented / partial / yes / partial | R/primary-evidence-disposition.json; P/post-r4-ax-acceptance.json | Seven exact returned-ID failure reconstructions remain unjoined. Profile selection, missing binding, changed generation, stale target/transport, Xvfb, journal interruption and failed transition are distinct predicates. |
| Historical wrong-tab attribution | current repair / current close pass / yes / historical unknown | incident0156; P/exact-target-close and raw-target-close | Original serialized selector/build missing; existing-evidence lookup only, no historical target replay. |
| Consumer CSV and transfer scope | native repair052848aa / final binary shared-case pass / installed052848aa / synthetic only | N/red-policy-reproduction.json; N/qualification-status.json; retained-unit-sim-4lmrgW | Peer policy/bytes and original handle preserved; owned cancel joins response, job and trace. Standalone compatibility deferred by operator; misattribution and consumer cancellation cause unproven. waitfordownload unchanged. |
| Final integration and completion | partial / incomplete / plan branch / incomplete | Plan0160 W5 | Full A1–A4/AX and required final integration remain open. |

### Installation disposition and next action

N/staged-generation.json identifies the exact qualified artifact used by
N/activation-receipt.json. The missing staging directory was reconstructed from
the retained release binary. A second disappearance occurred before activation
mutation; cause remains unproven. Pausing the interlock timer and performing
restage/activation sequentially succeeded. The timer was restored afterward.
N/installation-closeout.json records the final installed state and check results.
No additional reproduction, build, standalone CLI experiment or consumer operation
was performed for installation. Prior45/75-minute allowances remain consumed.

Sol implemented the bounded source repair; primary reviewed owner proof/deadline
and ran candidate/final qualification. A Luna docs spawn hit the thread limit,
so primary handled the small docs edits. Effective model cost remains unknown.
CI build2m16s; release build9m45s; format, strict clippy and docs checks passed.
Rework: original fixture transport identity mismatch, then a newline assertion
and unsupported trace filter. These exceeded the planned single fixture
correction; all failures are retained and no allowance was renewed. Extra
standalone data-URL checks failed on both baseline and candidate; a candidate
about:blank check also failed. Compatibility remains incomplete, not repaired.
Final shared-case proof includes Service/native peer preservation, canceled own
GUID with successful peer completion, returned-ID job/trace join and zero fixture
process residue. The integration fixture is private; a maintained regression
for native policy preservation remains a gap. Do not claim whole-plan completion.

### Validation, rework and bounded delegation

Format, strict workspace clippy, remote-view docs and Plan0160 consolidation
checks passed. The planning audit retains38 legacy findings. The final release
build took10m13s; two useful worker candidate builds took about2m33s each, and a
superseded shallow compile was stopped. Preserved real red/green evidence remains
the dialog regression protection. Production guidance received only relevant text.

Workers reproduced the confirm timeout, prepared bounded fixture drivers and
consolidated evidence. Primary retained the minimal repair and pinned proof while
rejecting an inactive-target refusal, a shallow helper test and incorrect predicate
mappings. Effective worker model cost was unavailable, so no savings are claimed.

The first viewer stalled at Checking stream; a bounded second attempt passed on
the same durable link with new clients, reconnect and controller refresh. No
browser/provider restart or source change established a single cause. Removing an
unneeded headless Xvfb dependency produced a residue-free cold-session pass. The
headed fixture also passed after restoring its proven PrivateTmp condition; only
applicable browser/profile/interaction facts are accepted. Timer evidence excludes
an overdue startup and one corrected shell sequencing mistake; steady observation
still requires the next completed invocation without settings changes.

### Presentation capacity and supervisor completion | 2026-09-09

Available Guacamole pool entries were blocked by historical browser IDs retained
on reconciled orphan route/display rows. Qualification now treats that exact
orphaned plus available plus unallocated combination as warm idle; checked-out
missing-browser ownership still blocks. Seven inventory tests and reconciliation
coverage pass.

Accepted workstation upgrades previously stopped, rewrote and enabled the runtime
host unit but returned unitStarted=false, leaving the candidate unsupervised and
forcing operator recovery. Apply and guarded resume now complete the existing
identity-bound takeover and require fresh proof of the supervisor PID, selected
ingress, executable, ports and conflict-free runtime census. Same-process installer
lock ownership is admitted for this internal step; foreign or unreadable locks
remain blocking. Acceptance now occurs only after takeover while the parent drain
remains active, and final census requires exactly one selected runtime-host
listener. Stale durable aliases may bootstrap staging from one exact reattachable
RDP browser, but the candidate must issue a fresh durable handoff before commit. Old-runtime
BILL reattach job bd44aa98-cf7e-4e57-9937-89aed1822873 failed at
InventoryAdmission without a route change. Transactions b1624cb5, 5c7bc5c0 and b5eb7ee8 exposed drain, cache and dotenv defects. Attempt 82fd55c4 proved BILL on
private :92 cannot appear on route A :10. All rolled back exactly. Bootstrap now requires browser, allocation, owner and route-display agreement. A disposable route
handoff, rebuilt install, supervisor and single-listener proof remain open.

### Git consolidation | 2026-09-08

PR13 merged the three historical Reddit/X incident reports into main at b21c4459.
Mergefffdbfc8 brought main into this active branch; the X report already matched,
and the workflow conflict retained the existing run-name without executable changes.
The clean P0137 and Reddit worktrees were retired after ancestry/content checks;
original branch refs remain recoverable on origin or through the active branch.
Ignored build outputs were archived and compared before removal. One worktree remains.
The full runtime backlog is not merged to main:181 files changed since green
CI49034e95. Reconcile existing validation by changed surface, then run missing
integration gates once. This is a Git readiness gap, separate from full A1–A4/AX
acceptance; installing052848aa does not prove the entire backlog merge-ready.
The pre-existing incident0156 edits and untracked notes0159/0160 remain untouched.

### Policy adoption and preservation

Policy commit e2b506b2 adds consolidation, separate evidence stages, candidate
freeze, semantic compaction, full delivery budgets and economical task routing.
Policies0010/0021/0028/0042/0043/0044/0045 and AGENTS.md carry the rules. The audit
helper checks declared consolidation structure, not evidence truth, model savings
or automatic runtime stopping. Selector baseline remains v0.1.24 with scoped
local overrides and adoption feedback in Plan0160; no shared-library replacement.
Focused auditor tests passed22; legacy migration is not part of this slice.

Preserve unrelated consumer edits to incident0156 and untracked notes0159/0160.
Earlier exact cleanup of six finished P159 fixtures remains accepted; do not
repeat it. No production browser/profile/credential/payment cleanup was performed.

- [Previous runbook through Turn280](RUNBOOK-history-through-2026-09-08-turn280.md)
- [Earlier history through Turn213](RUNBOOK-history-through-2026-09-02.md)
