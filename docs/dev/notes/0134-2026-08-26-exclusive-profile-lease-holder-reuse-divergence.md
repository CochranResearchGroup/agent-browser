# Exclusive Profile Lease Holder Reuse Divergence Handoff

Date: 2026-08-26

Status: OPEN FIELD DEFECT

Scope: Agent Browser browser-acquisition lifecycle and profile identity

Authority: DOCUMENTATION AND PROVIDER-FREE DIAGNOSIS ONLY

## Purpose

Record a field failure in which one live Agent Browser session held the
exclusive lease for a managed profile but was not recognized as a compatible
browser for that same profile. The resulting access plan waited for the holder
instead of reusing it or returning an immediate lifecycle inconsistency.

This note is a cross-repository implementation handoff. It does not authorize
closing or replacing the retained browser, releasing another task's lease,
reconciling live service state, deleting a profile, running garbage collection,
installing a new runtime, or consuming another Last30Days provider attempt.

Private page content, credentials, cookies, raw browser artifacts, and
operator-facing handoff URLs are intentionally omitted. The affected session
identifier is abbreviated in this note because a durable handoff identifier is
an operational routing value, not a public fixture.

## Executive Summary

The Agent Browser daemon and command channel were responsive. The failure was
not a dead browser, an MCP standard-input shutdown hang, an infinite-scroll
problem, or a proven X or LinkedIn authentication failure.

The failing Last30Days tick reached Agent Browser browser acquisition for both
sources and then produced this sequence:

1. X waited 30,232 milliseconds for the exclusive
   `last30days-facebook` profile lease.
2. LinkedIn waited 30,280 milliseconds for the same profile lease.
3. Both waits named the same retained handoff session, abbreviated here as
   `handoff-356...`, as the holder.
4. Agent Browser eventually returned typed lease-conflict failures.
5. Last30Days imposed an equal outer 30-second subprocess timeout and surfaced
   `unexpected_timeout_expired` before preserving the useful broker result.

The Agent Browser defect is the self-blocking lifecycle state. A session could
hold the exclusive lease for profile P while its live browser was absent from
the reusable set for P. Current source derives those two judgments from
different records:

- reusable browser eligibility requires `browser.profile_id == P`;
- lease conflict detection requires `session.profile_id == P` and an exclusive
  or human-takeover lease.

When those identities diverge, the session blocks the profile but its browser
cannot satisfy reuse. The planner therefore selects `wait_for_profile_lease`.
Waiting cannot repair the contradiction when the holder is the retained live
lane that should have been reused or explicitly classified inconsistent.

Last30Days had a separate observability defect that concealed this condition.
That consumer defect has a source fix, but it does not repair Agent Browser
lifecycle state.

## Incident Context

The operator reported the following installed Agent Browser acceptance
immediately before the reproduction:

```text
source main: a54b0f976fb20e801d8e09e844708753c80ac79d
installed binary SHA-256: 05d9da26035e0e86b55d6b2beaed25ae6dfe45ee6eeb0aa14362ce4ec08b0d10
installed generation: 0.28.0-05d9da26035e-7fa3fbcb7248
upgrade transaction: upgrade-3a9d3ace-cd02-48aa-851d-f1452c0832f5
doctor: zero issues, converged, one runtime host, one dashboard, zero legacy daemons
```

That receipt established installation and convergence. It did not prove that
every retained profile lease, browser record, session route, and owner
generation remained mutually consistent for a later consumer request.

The current Agent Browser repository checkpoint when this note was created is
`2d0e9106cb1a9b6aa12b6a920d69dcce9f8acb12` on `main`. The worktree also
contains an unrelated untracked BooksReceipts note. That file is existing work
and is not owned by this handoff.

## Evidence Boundaries

### Evidence before the direct diagnostic mistake

The retained Agent Browser trace from the failed tick proves:

- two independent acquisition waits reached approximately the configured
  30-second boundary;
- both waits targeted managed profile `last30days-facebook`;
- both waits named the same retained session as exclusive holder;
- the holder was not selected as a compatible reusable browser;
- both requests terminated as typed lease conflicts rather than reaching page
  observation.

This is enough to prove an acquisition-lifecycle contradiction. It does not by
itself prove which persisted record first drifted, whether the browser profile
was already missing, whether a runtime-owner transfer had become incomplete,
or whether later diagnostic activity changed the current projection.

### Direct browser and broker responsiveness

Bounded direct checks proved:

- retained-session tab switching, URL reads, title reads, and DOM snapshots
  completed in 1 to 51 milliseconds;
- one bounded X navigation completed in 2,186 milliseconds;
- the exact Last30Days Agent Browser MCP wrapper completed a read-only
  `tab_list` request, returned six tabs, and exited normally in 148
  milliseconds.

These results falsify a general Agent Browser commandability failure and an MCP
process-exit hang. They do not qualify the intended profile because of the
diagnostic identity mistake described next.

### Diagnostic correction

A direct command addressed the retained session but omitted the explicit CLI
flag `--runtime-profile last30days-facebook`. Agent Browser profile resolution
defaults an omitted runtime profile to `default`. After that command, current
service inventory recorded the ready `attached_existing` browser under profile
`default`, while the durable `last30days-facebook` profile allocation had no
browser holder.

The X public landing page and LinkedIn login form observed after this command
therefore belong only to the `default` projection. They are not evidence that
the intended `last30days-facebook` profile is logged out.

No browser, profile, session, lease, route, or lifecycle record was closed,
replaced, released, reconciled, or cleaned after this mistake was detected.

The chronology matters. The command explains the current `default`
attribution, but it must not be retroactively asserted as the cause of the
earlier tick unless a retained pre-command snapshot or deterministic fixture
proves that ordering.

## Exact Product Contradiction

Let:

- P be one selected managed profile;
- S be one live session;
- B be one live browser;
- O be the current lifecycle owner binding.

The service must not publish this combination as a normal wait state:

```text
selected profile = P
S.profile_id = P
S.lease = exclusive or human_takeover
S.browser_ids includes B, or O routes P through S and B
B is live
B.profile_id is missing or differs from P
compatibleLiveBrowserCount = 0
activeLeaseSessionIds includes S
recommendedAction = wait_for_profile_lease
```

This is not ordinary contention. It is an identity contradiction between the
holder and the held browser lane. A wait is valid when another coherent task
temporarily owns the profile. A wait is not a repair for a holder whose own
browser has fallen out of the selected profile's reusable set.

The service should instead do one of the following without launching a second
profile process:

1. prove that P, S, B, and O identify one current managed lane and allow exact
   retained-browser reuse; or
2. fail closed with a typed lifecycle/profile identity inconsistency owned by
   Agent Browser reconciliation.

The access-plan read itself should not silently rewrite lifecycle state.

## Current Source Semantics

CodeGraph was healthy at note creation with 597 indexed files, 22,747 symbols,
and 80,337 edges. Current source identifies the following owning seams.

### Access-plan reuse and lease conflict

`cli/src/native/service_access.rs::service_access_plan_for_state` calls
`access_plan_decision`, which calls `profile_reuse_decision`.

Within `profile_reuse_decision`:

- lines 1515 through 1529 collect reusable browser IDs only when the browser
  record's `profile_id` equals the selected profile and the browser satisfies
  live-health and caller-supplied posture constraints;
- lines 1558 through 1568 separately collect same-profile live browsers using
  the same browser-record profile equality;
- lines 1570 through 1583 collect active lease holders from session records and
  exclude a holder only when one of that session's browser IDs is already in
  the reusable browser set;
- lines 1629 through 1639 prefer `reuse_existing_browser`, then
  `wait_for_profile_lease`, then `launch_new_browser`;
- lines 1649 through 1683 publish reusable browser and session hints, compatible
  counts, active lease IDs, and the selected action.

The relevant helper split is explicit:

- `browser_is_reusable_for_posture` evaluates the browser record;
- `session_blocks_profile_reuse` evaluates the session record;
- `reusable_session_name_for_browser` derives a session route only after a
  reusable browser has already been selected.

This logic correctly blocks duplicate profile processes, but it currently has
no first-class branch for a live holder whose browser and session profile
identities disagree.

### Default runtime-profile resolution

`cli/src/runtime_profile.rs::resolve_profile` uses `default` when neither an
explicit profile nor a runtime profile is supplied. That behavior is reasonable
for a new unbound lane. It is unsafe if an existing service-owned session can
be attached, observed, or republished under that fallback while a canonical
runtime-owner binding already proves another managed profile.

The repair must trace the command path that converts an omitted profile flag
into a service `BrowserProcess.profile_id` or session projection. Do not assume
that `resolve_profile` itself is the only defect. The important contract is
that an omitted selector cannot reattribute an already managed retained
browser away from its proven profile.

### Runtime owner and adoption

Relevant identity authority also spans:

- `cli/src/runtime_owner_transfer.rs::RuntimeOwnerRegistry`, whose current
  owner binds profile identity digest, logical browser ID, daemon session
  route, process identity, and owner generation;
- `cli/src/native/runtime_lifecycle.rs`, which authorizes lifecycle transitions
  against that exact owner generation;
- `cli/src/runtime_adoption.rs`, which classifies cooperative live owners,
  preserve-only attached browsers, stale metadata, and inconsistent evidence;
- `cli/src/native/browser_session_authority.rs`, which projects whether a
  modeled browser is viable for service control.

The durable owner binding should be the higher-confidence identity source when
a bare session command omits a profile selector. Conflicting service records
must be reported, not silently normalized from the caller's fallback.

## Relationship to Accepted Plans

### Plan 0130

Plan 0130, `Access-plan owner reuse coherence`, is closed and established these
installed invariants:

- a compatible live owner returns `reuse_existing_browser` with exact browser
  and session hints;
- transferred owners retain the exact browser, tab, exclusive profile lease,
  and cleanup policy;
- compatible live browser count describes operation-compatible browsers;
- retained tab handles preserve the durable browser and retained profile.

The current field failure is either a regression in those invariants or an
uncovered path involving an unscoped direct session attachment. The repair
should extend Plan 0130 coverage rather than define a second reuse model.

### Plan 0132

Plan 0132, `Terminal-owner supersession route coherence`, is closed and proves
that terminal-owner replacement copies the exact current owner route, advances
the owner generation, and returns to retained-browser reuse after release.

The field failure occurred after the Plan 0132 hotfix installation. That does
not prove Plan 0132 caused the defect. It does make owner-generation,
replacement-route, and post-upgrade reconciliation evidence part of the
required reproducer.

## Consumer-Side Masking

Last30Days called the direct broker acquisition helper under an outer 30-second
`subprocess` timeout. The helper path bypassed the timeout translator already
used by its normal invocation wrapper. The outer timer expired before the
broker's useful lease-conflict response reached the consumer, producing
`unexpected_timeout_expired`.

Last30Days source commit
`5b2bfaa2a1319cb3fe56e63620357f6334842fd5` now:

- translates the direct broker timeout to `agent_browser_timeout`;
- preserves reason code `broker_service_request_timeout`;
- records `service_request:tab_new` command duration and timeout state;
- projects the reason through X diagnostics.

The evidence branch is pushed at
`df7babdd304915f6a05ecfe93c72b9175a99dbc9`. Focused affected suites passed
147 tests with two skips. The source was not built or installed, and no new
20-item X or LinkedIn tick ran.

This consumer repair improves observability only. Agent Browser still owns the
profile lease, retained-browser reuse, owner generation, session routing, and
live reconciliation defect.

## Required Provider-Free Reproducer

Build one deterministic isolated Service State fixture. Do not read or mutate
the real `last30days-facebook` profile.

The fixture should contain:

1. managed profile P with exclusive-process and shared-tab semantics;
2. ready browser B with host `attached_existing` and a stable process identity;
3. session S with `profile_id = P`, exclusive lease, and `browser_ids = [B]`;
4. current ready owner O binding P's identity digest to B, S, one process
   instance, and one owner generation;
5. a browser record for B whose `profile_id` is `default`, null, or otherwise
   inconsistent with P;
6. an access-plan request for P with no additional posture constraint.

Observe the current result through the public CLI, HTTP, and MCP access-plan
surfaces. The expected red behavior is:

```text
compatibleLiveBrowserCount = 0
activeLeaseSessionIds = [S]
recommendedAction = wait_for_profile_lease
```

Also add a command-path fixture that addresses S without an explicit runtime
profile. It must determine whether the current implementation overwrites or
republishes B under `default`, or whether the divergence originates earlier.
Record the exact first write or projection that creates the inconsistent pair.

## Required Repair Contract

The smallest coherent repair must satisfy all of these conditions:

1. A session holding an exclusive profile lease cannot be published as normal
   contention when its own live browser is excluded solely by contradictory
   profile identity.
2. Access planning returns exact reuse hints only when profile, browser,
   session, process, route, and owner-generation evidence agree.
3. When that proof is incomplete or contradictory, access planning returns a
   typed blocker such as `lifecycle_profile_identity_inconsistent` and assigns
   the next action to Agent Browser reconciliation.
4. The read-only access plan does not mutate browser, session, lease, profile,
   route, or owner state.
5. An omitted runtime-profile selector cannot reattribute an existing
   service-owned browser away from its canonical owner profile. The command
   must preserve the proven identity or reject the mismatch before writing.
6. The `default` fallback remains available for genuinely new, unbound browser
   work without weakening managed-profile isolation.
7. `attached_existing` adoption requires sufficient profile and owner evidence
   before becoming effect-capable. Ambiguous attachment remains preserve-only.
8. Reconciliation is compare-and-swap bound to the exact owner generation and
   process identity. It must not adopt stale metadata or overwrite newer human
   control.
9. The repair preserves the one-process-per-profile invariant and never
   launches a duplicate process to escape an inconsistent holder.
10. The profile allocation, service status, access plan, request admission,
    dashboard, HTTP, MCP, and generated client surfaces expose one consistent
    result.
11. Release of a consumer tab preserves the browser process, session route,
    profile lease, and durable owner unless the exact cleanup policy says
    otherwise.
12. Authentication and provider page content remain outside provider-free
    acceptance. A logged-in page is not required to prove identity coherence.

## Required Tests

At minimum, observe these tests red before implementation and green after the
repair:

- exclusive holder with exact browser profile returns retained-browser reuse;
- exclusive holder with mismatched browser profile returns a typed lifecycle
  inconsistency, not `wait_for_profile_lease`;
- omitted runtime-profile selector cannot reattribute an existing managed
  session to `default`;
- a genuinely new unbound session still resolves to `default`;
- an ambiguous `attached_existing` browser remains preserve-only;
- a current owner binding can recover route hints without rewriting state;
- a stale owner generation cannot authorize repair or input;
- an unrelated coherent exclusive holder still produces a normal bounded wait;
- no repair path launches a second browser or releases another session's
  lease;
- CLI, HTTP, MCP, schema, and generated client results agree on blocker and
  next-action fields.

Extend the public access-plan fixtures established by Plan 0130 and the
terminal-owner fixtures established by Plan 0132. Avoid a new isolated helper
with semantics that the public surfaces do not exercise.

## Validation Expectations

Use provider-free validation first:

1. the focused access-plan and profile-lease tests;
2. the direct session and runtime-profile resolution tests;
3. owner-transfer, runtime-lifecycle, reconciliation, and adoption tests;
4. service contract parity across CLI, HTTP, MCP, schema, and generated
   clients;
5. `scripts/ci/cargo-safe.sh fmt --manifest-path cli/Cargo.toml -- --check`;
6. `scripts/ci/cargo-safe.sh clippy --manifest-path cli/Cargo.toml -- -D warnings`;
7. `pnpm validation:select -- --base <known-green-ref>` and every selected
   check.

If source acceptance passes, prepare a source-free candidate and transactional
dry run under the normal Agent Browser release discipline. Installation and
live reconciliation require separate operator authorization.

Installed acceptance should use a harmless `about:blank` target and prove:

- the intended managed profile is selected;
- the current retained browser and exact session route are reused;
- no profile-lease wait occurs against that same session;
- acquisition and release preserve browser PID, process identity, profile,
  session route, route binding, and owner generation as required;
- final access plan still returns coherent reuse hints;
- install doctor and runtime convergence remain green.

Only after Agent Browser installed acceptance should Last30Days consume a new
bounded X and LinkedIn tick. Provider retrieval is downstream acceptance, not
the first proof of this lifecycle repair.

## Hard Stops

- Do not delete, reset, copy, or replace the real authenticated profile.
- Do not close or kill the retained browser to make the contradiction
  disappear.
- Do not release another task's session, tab, viewer, route, or lease.
- Do not run broad garbage collection or workstation reconciliation from a
  consumer task.
- Do not launch a duplicate browser on the selected profile.
- Do not treat the `default` browser's X or LinkedIn pages as intended-profile
  authentication evidence.
- Do not treat a longer consumer timeout as a lifecycle repair.
- Do not consume another provider tick before provider-free and installed
  Agent Browser acceptance.
- Do not publish raw handoff URLs, credentials, cookies, private page content,
  or browser artifacts in fixtures or notes.

## Source Pointers

- `docs/dev/plans/0130-2026-08-24-access-plan-owner-reuse-coherence.md`
- `docs/dev/plans/0132-2026-08-25-terminal-owner-supersession-route-coherence.md`
- `docs/dev/notes/2026-08-10-last30days-stale-runtime-pid-lock-handoff.md`
- `docs/dev/notes/2026-08-21-service-control-plane-attestation-source-acceptance.md`
- Last30Days Plan 0056 version 12, checkpoint C12
- Last30Days RUNBOOK Turn 347
- Last30Days source commit
  `5b2bfaa2a1319cb3fe56e63620357f6334842fd5`

## Best Next Action

Open one bounded Agent Browser defect plan or attach this note to an explicitly
compatible active lane. Start with the provider-free inconsistent-holder
fixture through the public access-plan surface. Do not touch the live browser
until the source path that first diverges `session.profile_id` from
`browser.profile_id` is proven and the repaired fail-closed contract passes.
