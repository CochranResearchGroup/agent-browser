# Automatically closed manual handoff recovery

Product lane: PL-BUGFIX
Work item: 599
Owner: ecochran76
Branch: fix/manual-idle-handoff-reopen
Base: 812e436c9a808922d8cb610885536d9771d86bd0
State: ACTIVE

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
