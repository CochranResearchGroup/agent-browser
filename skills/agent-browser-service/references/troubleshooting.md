# Troubleshoot service-owned browser work

Classify the failure before taking action. Do not turn diagnostics into effect
authority.

## Browser acquisition failures

Use the access plan, profile allocation, job, and request result.

- Profile freshness warning: run the bounded auth probe or seeding workflow.
- Active exclusive lease: wait when allowed or report the holder conflict.
- Compatible retained browser: use the returned browser and session hints.
- Browser capability failure: repair the selected executable or host evidence.
- Failed or faulted browser: use the service remedy only after the required
  operator review.

Do not create a duplicate profile lane unless reviewed throwaway isolation is
the explicit request.

## Presentation failures

Use scoped remote-view doctor or route preflight for the requested profile,
browser, session, or route.

- `route_pool_exhausted`: wait or request an explicit route switch.
- `reattachable_route_occupied`: keep the browser alive and wait or switch its
  presentation.
- `guacamole_route_unavailable`: repair or reacquire presentation before
  sharing a handoff.
- `wrong_tab`: inspect the selected target. Do not hand off a different tab.
- `stale_route_record`: reconcile retained route state without relaunching the
  browser.

Global advisories remain visible, but they do not override a ready
`requestedScope`.

## Runtime and cleanup findings

Use install doctor for installation and convergence, and use service resources
for process pressure. Read these fields before deciding severity:

- `runtimeMultiplicity.steadyState`
- `runtimeMultiplicity.issues`
- `serviceResources.candidateCount`
- `serviceResources.readinessImpactingCandidates`
- `serviceResources.duplicateProfilePressureWarnings`
- `runtimeLifecycle.blockingIncident`, when present

A warning such as `service_duplicate_profile_pressure` can coexist with zero
readiness-impacting candidates. Report it as maintenance evidence unless the
requested profile, active supervisor, convergence transaction, or request
result makes it blocking.

Do not run process cleanup from a consumer task. Agent Browser owns lifecycle
reconciliation and GC. If reviewed cleanup is required, run a dry-run, review
the exact candidates, and apply only with the matching review token or explicit
operator override.

## Report the result

Report each axis separately:

1. browser acquisition state and selected profile;
2. operator presentation state and route blocker, if any;
3. runtime maintenance state and whether it affects readiness; and
4. the next safe action owned by the caller, service, or operator.
