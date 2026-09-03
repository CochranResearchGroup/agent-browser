# Last30days X shared-profile identity rejection handoff

Date: 2026-09-03

State: unresolved intake note

Consumer: Agent Browser maintainers, Plan 0158 or its successor

Producer: Last30days recurring feed scraper

## Summary

A full Last30days manual tick failed to acquire an X tab three times with
`existing_session_profile_identity_unproven`. Each request became terminal
before execution began: the retained jobs have `startedAt: null` and report the
failure at workspace acquisition.

This does not look like global browser unavailability. Less than one second
after the third X request failed, a LinkedIn `tab_new` request succeeded against
the same retained shared profile. Reddit subsequently succeeded through that
profile as well. LinkedIn and Reddit each returned 80 unique posts. X observed
zero posts because its scraper never received a tab.

The immediate need is to explain and repair the X-specific admission or
identity-validation path. This note does not diagnose an X authentication
failure, a profile lock, a CDP failure, a presentation failure, or a scraper
content failure.

## Reproduction evidence

The source run was Last30days tick
`tick-98e14987fc5e9a7b7b63f8b8ea1abb95`, covering
`2026-08-04T17:24:33Z` through `2026-09-03T17:24:33Z`. It completed degraded at
`2026-09-03T17:30:30.227271Z`.

All three X attempts used:

- service: `last30days`
- agent: `x-scraper`
- task: `x-feed`
- target service: `x`
- requested action: `tab_new`
- runtime profile: `last30days-facebook`

The retained request jobs were:

- `mcp-service-request-tab_new-27259eb3-f8fb-460b-87a4-4a4272c748bc`
- `mcp-service-request-tab_new-cb122ef9-4010-4ef5-ac86-10fe6fceace0`
- `mcp-service-request-tab_new-3b259d71-3bbd-457d-a83c-ef6359c28a70`

Each job reports:

- `success: false`
- `startedAt: null`
- error: `existing_session_profile_identity_unproven`

The Last30days provider receipt classified every attempt as:

- failure class: `transient`
- failure stage: `workspace_acquisition`
- failure reason code: `existing_session_profile_identity_unproven`
- safe error code: `agent_browser_error`
- failure signature:
  `sha256:a489884adfd2a0f6f6d1247c8a3d924910ca0fcb65fe8c9d8f68d11ce58563ef`

The manual invocation inherited the managed capability and fresh-lane
environment used by the recurring timer. Sensitive capability values and paths
are intentionally omitted.

## Same-profile controls

The third X job completed at `2026-09-03T17:24:53.304068357Z`. The following
LinkedIn request was submitted at `2026-09-03T17:24:53.962Z`, started, and
succeeded:

- acquire job:
  `mcp-service-request-tab_new-cfc6da54-8491-40cd-b584-658ccfca58ea`
- release job:
  `mcp-service-request-tab_handle_release-f1060a0e-5898-410d-8d6e-0e26002053f3`
- scrape result: 80 accepted from 1,639 observed, with 80 distinct native IDs
  and 80 distinct non-empty URLs

Reddit later acquired and released a tab successfully through the same profile:

- acquire job:
  `mcp-service-request-tab_new-28b270b6-f2fb-4838-b877-bd9273231ae7`
- release job:
  `mcp-service-request-tab_handle_release-2e277ea9-6b11-43ba-9c25-62d15a2468c9`
- scrape result: 80 accepted from 482 observed, with 80 distinct native IDs
  and URLs

These controls show that the retained browser and profile were usable during
the tick. They do not prove which X-specific identity assertion failed.

## Post-failure access-plan contradiction

A current access-plan readback after the tick selects
`last30days-facebook` as a persistent `shared_service` profile authenticated for
Facebook, LinkedIn, and X. It reports:

- one compatible live browser
- zero active leases
- no blocking identity axes
- no duplicate pressure
- recommended reuse of the existing browser
- acquisition mode `tab_new`
- a service request carrying exact `browserId`, `sessionName`, runtime-profile,
  profile-path, and X target-service hints
- no acquisition, CDP, lifecycle, manual, or policy blocker

The reusable browser is
`session:terminal-profile-519cefb206f3e65f70c67902`, and the reusable session
is `terminal-profile-519cefb206f3e65f70c67902`.

This readback is current state, not proof that every normalized field was
identical at the time of the failed tick. It nevertheless exposes the key
contract question: why did admission reject the X request as unproven when the
same profile admitted adjacent requests and now presents as reusable without an
identity blocker?

## Working inference

The strongest current inference is an X-specific request normalization,
principal binding, retained-route identity, or admission-validation defect.
The evidence localizes the failure before browser effects and before scraper
execution. It does not establish the root cause.

In particular, do not infer any of the following from this failure:

- that the X account is logged out
- that Chrome or the retained profile was unavailable
- that Guacamole or another presentation route was required
- that X returned an empty feed
- that the X scraper rejected valid posts

## Requested Agent Browser investigation

1. Compare the fully normalized third X `tab_new` request with the immediately
   succeeding LinkedIn request. Include runtime profile, resolved profile path,
   browser ID, session name, target-service IDs, principal and capability
   binding, owner generation, and any legacy or duplicate-lane override.
2. Identify the exact identity assertion that was unproven and the source of
   each value used in that comparison. The public error code alone is not
   actionable enough to distinguish caller repair from a service defect.
3. Explain why the failed jobs became terminal with `startedAt: null` while the
   access-plan surface offered no specific transition or recovery action.
4. Reconcile the failure with the post-failure access plan that reports the
   same retained browser as reusable, with exact route hints and no identity
   blocker.
5. Return typed, structured recourse for this rejection. The caller needs to
   know whether it should refresh the access plan once, repair a named field,
   request an explicit lifecycle transition, or stop without retrying.
6. Add a provider-free regression that models serial requests against one
   retained shared profile. When exact identity and route hints are consistent,
   an X-targeted `tab_new` request should either be admitted or return a precise,
   actionable inconsistency rather than a generic unproven-identity rejection.
7. Preserve the pre-effect guarantee: an admission failure must not create or
   navigate a tab. The repair should improve identity coherence and recourse,
   not bypass authorization checks.

The three identical failures already consumed the caller's retry budget. A
fourth blind retry would not add diagnostic value.

## Scope and privacy

No Agent Browser repair, browser mutation, profile replacement, session
replacement, route recovery, lease force, or authentication action was
performed while preparing this note. No cookies, tokens, page bodies, private
capability paths, or raw profile data are included.

## Source pointers

The Last30days source evidence is recorded in:

- repository: `CochranResearchGroup/last30days-skill`
- branch: `feat/recurring-reddit-home-feed`
- commit: `ecba6fc324690ab5acf9209d5e4b4485b3118d4a`
- plan: `docs/dev/plans/0064-2026-09-03-recurring-reddit-home-feed.md`,
  checkpoint C03
- runbook: `RUNBOOK.md`, Turn 411

The Agent Browser retained job records and current access-plan readback are the
authoritative runtime evidence for the acquisition failure described here.
