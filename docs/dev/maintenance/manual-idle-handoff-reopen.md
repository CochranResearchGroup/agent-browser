# Automatically closed manual handoff recovery

Product lane: PL-BUGFIX
Work item: 599
Owner: ecochran76
Branch: fix/manual-idle-handoff-reopen
Base: 812e436c9a808922d8cb610885536d9771d86bd0
State: INSTALLED_OWNER_RECOVERY_ACCEPTED_AUTOMATIC_OPEN_UNACCEPTED

The exact authenticated browser loop renders the reported desktop-unavailable
page. Direct Native navigation preserves RED in under one second; an absent
handoff produces a different link-not-found page. Owner resolution returns
manual_login_ended before replacement launch. Retained logical joins are valid
and positive automatic closure is recorded. Raw HTTP403 is excluded: it was not
the equivalent browser flow.

Ranked predictions: legacy manual recovery refusal first; unavailable Native
assignment second; stale logical joins third; auth/redirect mismatch fourth.
Current owner readback and exact browser controls support the first. Regression
must exercise real manual RuntimeDriver dispatch joined to the public host and
SQLite, not only the existing fake Process reconciliation implementation.

Reuse the canonical checkout and existing ownership boundaries. This lane is
primary writer for manual runtime recovery classification and its tests.
Dependencies: merged585,579,583. No new worktree; preserve all existing source,
ops and installed-candidate lanes. Active598 owns separate workspace auth files;
shared glossary/catalog integration must preserve both intents.

Qualify a finite retained automatic-close recovery with existing immutable
original identity, completed close and launch custody. Preserve logical browser,
profile, task, tab and links. Unknown, live, changed, unqualified or explicitly
closed targets remain refused. Preserve original attempts and private history;
no public recovery bypass flag or generic manual reopen exemption.

No production database edit, reset, auth-rule change or automatic installation.
Source regression and local checks precede any candidate. Original browser-loop
GREEN and authenticated viewer acceptance remain unproven until a matching
installed repair is exercised. Private runtime evidence stays outside Git.

## Checkpoint

Source13d2efd50 contains the regression and documentation only, not a product
fix. Draft PR600 retains the exact scope and issue599. Policy wiring, link and
patch-hygiene checks pass. The new Rust test never reached compilation: normal
Cargo admission stayed at memory_pressure with zero admitted repository builds,
while available host memory remained26–28GiB against the default30GiB minimum.
Its unadmitted waiter was stopped; this is NOT_RUN, not product RED or test failure.

The actual browser loop is RED twice through the original URL and once through
direct Native navigation. The nonexistent-link control returns distinct text.
Owner probing returns manual_login_ended; saved task/tab/profile/browser joins
all match and automatic close plus original process absence are recorded.
These observations support the manual runtime refusal as the first failing
boundary; source-level regression and downstream launch proof remain unexecuted.

Resume by running the focused manual_idle_handoff_reopens_through_real_runtime
regression through scripts/ci/rust-tests.sh once normal admission can preserve
the host reserve. Correct fixture setup failures before classifying source RED.
No product edit precedes that valid verdict. Then qualify both reconciliation
and launch against completed original retirement without widening mode-conversion
or unknown-effect authority. Do not assume a downstream Native relaunch succeeds.
The public browser loop and actual viewer readiness must be rerun after an
appropriately authorized installed repair before claiming this link fixed.

The isolated named probe browser was closed with verified own profile/process
identity and fresh OS residue checks. The shared dev-browser daemon and its
pre-existing attached session were preserved. No production browser, profile,
auth rule, installed binary or custody record was reset or manually edited.

## Resumed red-green cycle

The real-runtime regression reached a valid RED at sourcef8fc1af39 with exactly
manual_login_ended after all closed-scope fixture preconditions passed. The first
runtime repair advances to replacement launch and then the model's existing
CDP-free publication guard rejects browser_session_recovery_identity_conflict.
This is a second boundary in the same retained automatic-close path. The lane
therefore also owns the narrow model publication qualification: an idle-closed
owned original plus exact pending Native launch, complete replacement process
identity and unchanged logical scope. The general empty-CDP refusal remains.
Do not reuse the boolean reserved for explicit owner replacement as a bypass.

The fuller human-invitation fixture reaches a further valid RED:
browser_handoff_recovery_mapping_unrecorded. The launch custody proof counts
the earlier completed live-retention decision as a second physical recovery.
This lane also owns the narrow selection correction in
browser_launch_custody_store.rs: exclude only completed, owner-resolved live
retention from physical recovery selection. Preserve its uncertain close
attempt and retain all physical-close, publication and process identity checks.

Both positive regressions pass at62cd5453c: the minimal automatic-close path
and the fuller history with original human control, epoch and unsettled earlier
close preserved. Current main through e7131fd05 is integrated at ac1f15257;
the catalog conflict preserves both independent lane entries. Product changes
remain limited to the three observed recovery boundaries. The fixture uses
real RuntimeDriver reconciliation, SQLite and kernel process identities, with
the process/provider launch boundary simulated. It does not yet prove real
Chrome launch or the installed public viewer.

Strict workspace Clippy and the complete service-model package suite pass on
the integrated product source. A changed-target refusal assertion is being
qualified before candidate freeze. All compiler/setup failures are preserved
separately from the three valid product REDs. No production effect has occurred.

## Source integration and sealed candidate

PR600 merged as dd8251b3d9e1c15d3ed6e0776f49c45be5bd7165.
Source793811e2e passes both real-runtime regressions, including changed-target
refusal without launch, plus35 related CLI cases and418 service-model tests.
Workspace strict Clippy, final Rustfmt, policy wiring, changed links, handoff
documentation and the docs build pass locally. No hosted CI was started.

The clean-source production-shaped candidate is
candidate-dfe2e2ecbacc3295-dca8b78d58435157, binary SHA-256
dca8b78d58435157f4f0b51b2933499b16ab514cadc64372ad67cb24aeabb28c.
Its exact source is in main ancestry. The first build's own sccache version
probe stalled; its exact process was stopped and the failed claim archived by
the supported retry. Cache-disabled full-release compilation and sealing passed.
No source or resource ceilings were changed.

The exact sealed binary passes workstation install qualification with2 owned
fixture hosts retired and zero residue. Service contract metadata, MCP reads,
profile sources, site-policy sources and HTTP/MCP incident summaries pass
without browser launch. The MCP read smoke first exposed a pre-existing frozen
allowlist omission: browser_retention has been implemented since4a627690f but
was absent from that assertion. The one-entry correction preserves exact ordered
inventory comparison. The initial failure remains preserved. This fixture-only
correction and this receipt do not change application or embedded support inputs.
All637 recorded input files match except the checkout's advancing Git branch
reference. The sealed binary retains its original source793811e2e provenance,
already verified in main ancestry; it is reused rather than rebuilt for merge
metadata or qualification-only changes under policy0051.

An earlier unpinned older-debug install fixture was cancelled and excluded from
acceptance; its normal cleanup also verified2 owned hosts and zero residue.
Sealed install dry-run reports artifact_sealed, eligible production-shaped reuse,
no integrity preconditions, recommendation install and no_effect_performed.

Production installation remains excluded from this source repair. Installed
workflow guidance is therefore not yet synchronized. The original public browser
loop, real Chrome recovery and native viewer readiness remain unproven until an
authorized preserving installation is followed by that same-link acceptance.
Retain the canonical checkout and sealed candidate for that next operation.
Memory disposition: queued for the source repair, with installed acceptance
explicitly unproven; queue acceptance is not retrieval or persistence proof.

## Installed consumer correction,2026-11-09

The later explicit deployment request installed source7ac7f95ca, including this
repair, with accepted preserving adoption and passing doctor. See the
[modal deployment receipt](handoff-modal-search.md#production-deployment2026-11-09).
The original authenticated public link first reproduced the exact unavailable
page on the installed candidate. An explicit authenticated owner-side resolve
then reported opened and operatorVisible.state=ready. A subsequent same-link
browser check rendered a Remote View canvas. This proves current owner-assisted
recovery and does not erase the earlier automatic-open RED.599 is reopened for
that remaining entrypoint/continuation gap. No new handoff ID, profile reset or
website-effect replay was used. Private target and authentication evidence stays
outside Git.
