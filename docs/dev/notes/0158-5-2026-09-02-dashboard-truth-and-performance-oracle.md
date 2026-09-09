# Plan 0158 W5 Dashboard Truth And Performance Oracle

Date: 2026-09-02

Status: complete

## Outcome

W5 adds a provider-free, no-repair oracle for rendered dashboard truth and
performance evidence. It does not infer correctness from DOM text or source
shape. Materialized fixtures carry independently captured authoritative
resources, rendered rows, selection and action readback, per-client state,
health axes, handoff URLs, stream barriers, console and network observations,
accessibility checks, timing samples, and resource samples.

The closed corpus contains 51 cases and 46 defect codes. Four inventory
controls cover empty, sparse, normal, and dense states. A separate clean
convergence case proves that only typed convergence failure produces the
convergence warning and exactly one executable action. Every remaining case
isolates one seeded defect.

## Dense Control

The deterministic dense generator creates the full requested inventory:

- 100 Profiles;
- 500 browsers;
- 2,000 tabs;
- 10,000 jobs; and
- 10,000 events.

That is 22,600 material resources and 600 current actionable rail rows. IDs are
unique and deterministic while labels deliberately repeat. The clean dense
fixture passes the bijection and stable-identity oracle.

## Integration Findings

Primary-agent review found and closed six problems before acceptance:

- resource budgets were initially indexed by sample-field names instead of
  their per-minute output names;
- dense ordering reset inside each resource type;
- duplicate rows cascaded into an unrelated stable-identity finding;
- the same-label seed did not initially contain two distinct same-label
  resources;
- the viewport controls omitted the typical-width band; and
- multi-client selection leakage lacked an explicit typed fixture and finding.

The final oracle keeps these checks independent and uses immutable resource and
client IDs rather than display labels.

## Validation

The primary agent ran:

```text
pnpm test:p158-campaign
pnpm test:p158-logging-auditor
pnpm test:p158-external-handoff-oracle
pnpm test:p158-dashboard-oracle
```

All passed. The dashboard battery validates strict input and report schemas,
all exact classifications, deterministic output, input immutability,
`repairAttempted: false`, timing percentiles, all 12 resource slopes, external
handoff hygiene, stale-readiness rejection, responsive and accessibility
evidence, and the full dense control.

No candidate, installed runtime, browser, provider, route, Profile, or tenant
state was touched in W5.

## Next Gate

W6 must publish one isolated development candidate, prepare separate E1 and E2
environments, prove E2 is a genuinely external network vantage, calibrate
without starting a test case, and freeze every candidate and environment
digest. No W7 or W8 work may begin before that seal exists.
