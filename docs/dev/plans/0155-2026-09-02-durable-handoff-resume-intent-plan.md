# Plan 0155: Durable Handoff Resume Intent

Date: 2026-09-02

State: CLOSED

Lane: P155

Branch: `fieldwork/research-gov-deterministic-automation`

Target: `main`

Source baseline: `294b86850f56426c01a10df1158affc7b9917e6d`

Authority: SOURCE-ONLY

Dependencies: [P150]

Overlaps: [P144]

## Objective

Turn a resolved durable remote-view handoff into one deterministic client-side
resume context and classify a diagnostics response as unavailable,
observation-only, or effect-capable. Callers must not reconstruct the retained
browser route, canonical attribution tuple, profile identity, or effect
authority from URLs and ad hoc state inspection.

## Current Evidence

- Research.gov handoff `r580584` resolves to the original browser, session,
  target, and valid service tab handle without launching or navigating.
- The profile is allocated per service. Access planning selects the retained
  profile only when the original service, agent, and task tuple is preserved.
- Handle-bound diagnostics and probes work while
  `controlPlaneAttestation.complete=false` correctly withholds effect
  authority because `profile_lease` proof is missing.
- `requestServiceRemoteViewHandoff()` returns only the operator link, while
  lower-level handoff resolution responses already contain the exact tab
  handle and trace filter needed for deterministic resume.
- P144 owns canonical profile-lease authority. This plan consumes its public
  attestation shape and does not change lease state, admission, or recovery.

## Frozen Decisions

1. Keep the new behavior in the existing `@agent-browser/client`
   service-request module. Do not add a server endpoint or service action.
2. Derive resume context only from the resolved handoff response and its valid
   service tab handle. Do not consult process lists, raw provider URLs, profile
   directories, or global service state.
3. Preserve the canonical service, agent, and task tuple from the handle trace
   filter. Missing attribution fails closed.
4. Return managed profile identity, browser ID, session name, target ID, URL,
   and the original handle without exposing a user-data directory.
5. Classify effect authority only from the diagnostics
   `controlPlaneAttestation`. `complete=true` is the sole effect-capable result.
6. A valid handle with incomplete attestation is observation-only. A missing
   or invalid handle is unavailable.
7. Do not mutate the retained Research.gov browser, profile, lease, route, or
   freshness state in this source slice.

## Work Units

| Unit | Scope | Depends on | Exit condition |
|---|---|---|---|
| W1 | Register the plan and active lane | none | Plan and lane metadata preserve P144 overlap |
| W2 | Add resume-context helper test and implementation | W1 | Valid handoff resolution yields the canonical route and attribution tuple; incomplete identity fails closed |
| W3 | Add authority-classifier test and implementation | W2 | Effect capability depends only on complete attestation; incomplete proof is observation-only |
| W4 | Update public guidance and generated types | W3 | CLI help, README, skill, docs site, inline comments, and client declarations agree |
| W5 | Validate and close the source slice | W4 | Focused client, generated contract, type, docs, and patch gates pass |

## Acceptance Criteria

1. One public helper accepts a handoff resolution response and returns the
   canonical service, agent, task, browser, session, profile, target, URL, and
   valid service tab handle.
2. The resume helper rejects missing or invalid handles, missing browser or
   session identity, and incomplete caller attribution.
3. The resume helper does not return provider URLs, profile paths, cookies,
   credentials, or other private browser state.
4. One public helper classifies diagnostics as `effect_capable` only when
   `controlPlaneAttestation.complete=true`.
5. A valid handle plus incomplete attestation classifies as
   `observation_only` and preserves normalized missing-proof codes.
6. Missing or invalid handle evidence classifies as `unavailable` and never
   implies effect authority.
7. Client types and exports cover both helpers without a new server contract.
8. All required user-facing guidance describes the resume and authority gates.
9. Focused client tests, generated-client checks, JavaScript type checking,
   documentation build, validation selection, and patch hygiene pass.

## Validation

```bash
pnpm test:service-request-client
pnpm test:service-client-contract
pnpm test:service-client-types
pnpm test:service-client-exports
pnpm --dir docs build
pnpm validation:select -- --base 294b86850f56426c01a10df1158affc7b9917e6d
git diff --check
```

## Bounds

- Maximum implementation attempts: 2
- Maximum review and remediation cycles: 1
- Maximum no-progress checkpoints: 2
- Checkpoint interval: each completed work unit or 90 minutes

## Non-Goals

- Profile-lease registration, rotation, reconciliation, or admission changes
- A new remote-view action, endpoint, schema, or provider
- Automatic freshness writes or authentication attempts
- Live navigation, input, profile mutation, route switching, or cleanup
- Changing P144 authority or resolving the current legacy Research.gov lease

## Completion Evidence

Implementation checkpoint: `804519f0`

State transition: `source_qualified -> source_complete`.

Acceptance state: W1 through W5 are complete. The bounded source slice is
qualified at a recoverable implementation checkpoint and ready for ordinary
integration review.

Progress classification: `outcome_progress`.

Evidence: the client now derives exact retained route and caller intent from a
valid handoff handle and classifies diagnostics into unavailable,
observation-only, or effect-capable authority. Focused behavior, generated
contract, JavaScript type, package export, service client, documentation,
route-confusion, API/MCP parity, Rust formatting, strict Clippy, and patch
hygiene gates pass. P144 overlap remained read-only; no lease or live browser
state changed.

Material blocker: none for this source slice. Live Research.gov effects remain
withheld by the separately owned incomplete profile-lease attestation.

Next action: integrate the source checkpoint through the normal branch flow,
then resume read-only Research.gov fieldwork unless canonical lease authority
becomes effect-capable.
