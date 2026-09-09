# Plan 0158 W1 Historical Failure Registry

Date: 2026-09-02

Plan: `docs/dev/plans/0158-2026-09-02-frozen-candidate-historical-failure-stress-campaign.md`

State: COMPLETE

## Outcome

W1 freezes a closed-world registry for the diagnostic campaign before any load
generator, installed candidate, provider route, or browser is touched. The
machine-readable authority is
`docs/dev/contracts/p158-historical-failure-registry.v1.json`. It contains 11
historical failure families, all 49 named scenarios, all five combined phases,
the environment and result vocabularies, case dependencies, evidence profiles,
candidate identity fields, resource ceilings, performance ceilings, forbidden
capture fields, and the no-repair execution rules.

The relationship-preserving synthetic seeds are in
`docs/dev/fixtures/p158/historical-failure-seeds.v1.json`. They retain only the
causal shape needed to reproduce seven production signatures and the legacy
null-terminal-envelope defect. Tenant names, account labels, source URLs,
profile paths, provider connection labels, process identifiers, and all
credential-adjacent values are substituted.

No source build, install, runtime restart, runtime repair, browser action,
route action, Profile mutation, cleanup, or provider effect occurred in W1.

## Production Read-Only Reconciliation

A read-only query of the retained 200-job production window beginning
`2026-09-02T18:00:00Z` recomputed the preliminary Plan 0158 sample. Only
allowlisted aggregate counts and error-prefix classifications were retained:

| Redacted signature | Count |
| --- | ---: |
| `existing_session_profile_identity_unproven` | 14 |
| `existing_session_profile_identity_inconsistent` | 3 |
| Xvfb display `:90` automatic-launch failure | 8 |
| Service-resource timeout | 5 |
| Route-pool failure | 4 |
| Presentation proof or authority failure | 2 |
| Other launch or reattach failure | 3 |

The window contains 34 failed and five timed-out jobs. All 39 lack both a
top-level structured `failure` and top-level immutable `provenance`. The
fixture registry treats those missing fields as defects to detect, not as a
legacy format exemption. Raw jobs and raw diagnostic payloads were not copied
into the repository.

## Closed-World Evidence Map

The principal source clusters are:

- P157, P134, and the retained-browser handoffs for self-declared clients,
  shared and same-label occupancy, policy revision, identity ambiguity, crash
  epochs, and lifecycle continuity;
- P142 and the production aggregate for lock contention, timeout effect
  uncertainty, scheduler terminalization, and missing causal envelopes;
- P46, P62, P63, P67, P96, P112, P133, and the route-staleness incident for
  target selection, foreign CDP, route checkout, presentation proof, durable
  handoffs, operator-visible focus, and dashboard state;
- P147, P148, and P156 for ingress generation, stale owner takeover,
  cooperative-preserve failure, and exact full-shutdown boundaries; and
- the P46 and P67 harness implementations plus the current dashboard stream
  implementation for known test-harness blind spots.

Every family maps bidirectionally to one or more case IDs and one or more
sources. Every case has a fixed environment set, deterministic execution
bound, evidence profile, and prerequisite list. New signatures found after
freeze are recorded as findings but cannot add cases or repetitions to the
execution schedule.

## Important Harness Findings

Existing regression assets are useful inputs, but none can serve unchanged as
the P158 controller:

- the P46 runner resets, repairs, retries, cleans, and overwrites mutable
  summaries around scenarios;
- the P67 runner reconciles and cleans state and accepts a loopback URL when a
  public URL is absent;
- P67 dashboard rail persistence checks Service source rows but never
  authenticates and renders the dashboard rail;
- existing external Chromium adapters execute beside the service host and use
  loopback DevTools, so they do not establish an off-host external vantage;
- the current dashboard stream constructs `ws://localhost:<port>`, which must
  be caught by the E2 URL-leak oracle; and
- warning-axis, durable-handoff, and much dashboard-selection coverage is
  source-level or pure projection coverage rather than correlated rendered UI
  evidence.

These behaviors are frozen as negative requirements. W2 through W5 may reuse
pure fixtures and evidence vocabulary, but must place all execution behind the
append-only controller and must reject loopback fallback.

## Frozen Safety And Identity Inputs

W1 fixes the candidate manifest contract at 12 fields spanning source, binary,
dashboard, installed generation, browser executable, runtime manifest,
provider configuration, external ingress, fixture digest, and prepare/freeze
timestamps. W6 must populate and seal those fields before the first case.

The resource guard freezes a 100 GiB artifact quota, 90 percent filesystem
ceiling, 4 GiB available-memory floor, 8 GiB combined available-memory and
free-swap floor, and hard ceilings for campaign processes, Chrome, Xvfb,
displays, routes, external connections, and unresolved jobs. Two consecutive
resource violations trigger the declared safety stop. These are campaign
ceilings, not claims that the target host or provider has equivalent usable
capacity; W6 calibration may choose stricter values but cannot raise them.

## Parallel Inventory Reconciliation

Three read-only inventory owners were reconciled by the primary agent:

- `/root/w1_identity_logging` covered identity, ACL, lifecycle, supervisor,
  history parity, and causal logging;
- `/root/w1_remote_display` covered external ingress, durable handoffs,
  Guacamole and RDP, X11 and Xvfb, window proof, supervisor, and install
  transitions; and
- `/root/w1_dashboard_harness` covered rail bijection, selection, stale links,
  event loss, external URL hygiene, dense performance, resource growth, and
  reusable or forbidden harness behavior.

All three reported no file or runtime mutations. The primary agent verified
the resulting paths against the current repository and encoded the union into
one registry rather than preserving three competing inventories.

## Validation

`pnpm test:p158-historical-failure-registry` passes with:

- 11 unique, source-backed failure families;
- 54 unique cases comprising 49 scenarios and five combined phases;
- seven aggregate production signature fixtures plus one null-envelope seed;
- bidirectional family and case mappings;
- resolvable repository sources;
- complete dependency, environment, evidence-profile, fixture-redaction, and
  terminal-state vocabularies; and
- W10 constrained to depend only on sealed W9 execution evidence.

## Remaining Gate

W1 does not make the campaign executable. W2 is next: build the monotonic,
append-only controller, deterministic scheduler, atomic artifact manifest,
fault-injector interface, safety monitor, and terminal result schema. Its
provider-free self-tests must prove that failures are never overwritten,
opportunistically retried, repaired, or cleaned and that dependency loss
propagates exact `skipped_blocked` results.
