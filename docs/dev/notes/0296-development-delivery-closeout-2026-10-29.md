# P296 merged source and isolated development delivery

Product lane: PL-RECIPES
Owner: [P296](../plans/0296-2026-10-26-browser-journeys-teaching.md), issue466

## Qualified result

[PR480](https://github.com/CochranResearchGroup/agent-browser/pull/480) merged at
`6c596cc648c6ee85adc95c665e19e5943c5b4514`. A clean build of that exact merged
source produced optimized development candidate
`cc5c41faa991033509aeb2ebf86f46c8b28a88e5d57be0fd5dbf1cdb81b408bc`.
The candidate was published into the isolated p296 development namespace.
This is a development checkpoint, not a production release candidate seal.

On 2026-10-28 the namespaced development doctor passed all checks and
`scripts/smoke-development-browser-launch.js` passed three disposable open,
URL-read, close and process-residue iterations. On 2026-10-29 doctor was
rechecked successfully against the same executable digest. A fresh OS readback
retained the three expected development service processes after smoke cleanup.
The production and default-development selected generations were unchanged;
no production publication occurred. Existing production state evolved during
observation, so full historical byte/process equality is not claimed.

## Failures and repair

The first development start failed to bind its dashboard port and rolled back.
After selecting different pinned ports, the second start still failed because
retained dashboard ingress selected the old backend port. Final publication
succeeded after registry reinitialization. Preserve these failures as diagnostic
history; successful publication alone did not qualify the browser workflow.

Doctor then isolated one remaining failure: local ingress. The raw dashboard
manifest was healthy, the local hostname returned404, and Traefik could reach
the upstream. Adding the missing local-only Cooper inventory repaired the route.
External and Tailscale publication remain disabled. No application code change
was required. [Cooper PR6](https://github.com/ecochran76/cooper-webservices/pull/6) retains the inventory and generated
local router/upstream, excluding unrelated checkout edits.

During earlier diagnosis, invoking the Cooper renderer with a help argument
unexpectedly rendered existing dirty inputs. Exact prior bytes were not captured,
so historical byte equality cannot be reconstructed. The repair captured a fresh
baseline, verified every pre-existing router/service/middleware definition and all
bastion output bytes were preserved, and proved a second render byte-identical.
No shared container restart or authentication change occurred.

## Remaining boundaries

Original children467 through477 are closed. Actual narrated walkthrough478 stays
explicitly deferred and parent466 stays open. Generic development launch acceptance
does not qualify a teaching video, a real account, provider-specific Auth acceptance,
or a new Native image implementation. The final broad review and its bounded
remediation already completed once; this delivery closeout adds neither.

Detailed runtime receipts, failed publication output, process inventories and
Cooper configuration backups remain in user-scoped operational state outside Git.
The earlier final review record remains authoritative for source adjudication:
[final review](0296-final-review-2026-10-28.md).
