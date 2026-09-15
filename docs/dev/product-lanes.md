# Agent Browser Product Lanes

Date: 2026-09-13

This document defines stable product ownership for concurrent Agent Browser
development. Product lanes group outcomes and long-lived source responsibility.
They do not replace numbered plans, active execution lanes, work items, or Git
receipts.

Every new substantive plan and active-lane catalog entry declares one primary
product lane. A plan may name related lanes, but one lane owns integration and
the final acceptance decision.

## Lane Summary

| ID | Product lane | Outcome | Default branch prefix |
| --- | --- | --- | --- |
| `PL-BUGFIX` | Reliability and Bug Fixes | Restore promised behavior with the smallest evidence-backed repair | `fix/` |
| `PL-AUTH` | Credentials and Automated Authentication | Complete governed authentication without exposing secrets | `auth/` |
| `PL-CHALLENGE` | Challenge and Anti-Automation Countermeasures | Detect and solve eligible challenges through one bounded automated attempt | `challenge/` |
| `PL-RECIPES` | Automation Recipes and Helpers | Package deterministic reusable browser workflows and fixtures | `recipes/` |
| `PL-PLATFORM` | Architecture and Infrastructure | Own the service, runtime, state, installation, desktop-services, and development foundations | `platform/` |

## PL-BUGFIX | Reliability and Bug Fixes

This lane owns regressions, incident-driven repairs, compatibility failures,
and incorrect projections in already-promised behavior. It must preserve the
original product contract or explicitly hand a changed contract to another
product lane.

Typical scope includes retained-browser identity conflicts, stale handles,
incorrect effect classification, timeout and locking defects, lifecycle races,
and deterministic installer or supervisor failures.

This is not a permanent ownership sink. A repair that introduces a new
credential workflow, challenge strategy, recipe framework, or platform
abstraction moves to the corresponding product lane. The bugfix owner may
retain the reproducer and regression test while the owning product lane delivers
the larger change.

## PL-AUTH | Credentials and Automated Authentication

This lane owns opaque credential references, credential-provider integration,
site authentication state machines, browser password-manager behavior,
authentication freshness, passkeys, second factors, account verification, and
resume-safe Authentication Runs.

Automated authentication is the default product outcome. Human intervention is
reserved for missing authority, unsupported factors, ambiguous identity,
exhausted bounded attempts, or effects that cannot be verified safely. Public
actions never accept plaintext secrets, and durable evidence never records
credentials, cookies, raw second-factor values, or private page bodies.

This lane consumes profile and lifecycle primitives from `PL-PLATFORM` and
exposes reusable authentication steps to `PL-RECIPES`. It may ask
`PL-CHALLENGE` to resolve an eligible challenge, but it retains the overall
account and authentication outcome.

## PL-CHALLENGE | Challenge and Anti-Automation Countermeasures

This lane owns challenge detection, classification, policy, bounded automated
solving, result verification, cooldown, and typed intervention. It includes
CAPTCHA families, browser-external prompts, access-denial classification, and
site anti-automation observations that affect authorized automation.

Eligible challenges proceed through one policy-authorized automated attempt
before human handoff. Failed, unsupported, ambiguous, externally rejected, or
exhausted attempts become typed intervention states. The lane does not farm
challenges, rotate identities to escape rejection, disguise unsupported
automation, or blindly retry.

The lane builds on shared desktop capture, semantics, coordinates, controller
authority, input, journaling, and verification from `PL-PLATFORM`. CAPTCHA
policy must not create a parallel desktop-services stack. The current
`feature/turnstile-desktop-challenge` worktree remains the active source for
Plan 0169 and its CAPTCHA roadmap until its conflicting P173 roadmap identifier
is corrected.

## PL-RECIPES | Automation Recipes and Helpers

This lane owns versioned, deterministic compositions of existing browser
capabilities: site recipes, access-plan helpers, bounded monitors, form and
navigation sequences, reusable fixtures, examples, and recipe-development
tooling.

A recipe declares inputs, required capabilities, target binding, expected
states, effect budget, idempotency, verification, and typed exits. It composes
platform, authentication, and challenge services rather than duplicating their
authority or embedding credentials. Site-specific knowledge belongs here when
it is a reusable workflow; a one-time observation remains a field note until a
plan adopts it.

## PL-PLATFORM | Architecture and Infrastructure

This lane owns the Agent Browser service authority, Service State persistence,
runtime supervisor, browser lifecycle, leases, transport, installation,
development runtime, build acceleration, remote-view infrastructure, shared
desktop services, capability registry, observability, and architectural
extractions.

[Plan 0161](plans/0161-2026-09-09-first-class-profile-repair-and-reset-plan.md)
belongs primarily here because it establishes the profile aggregate, sealed
repair and reset transactions, lifecycle joins, and shared public adapters.
`PL-AUTH` consumes its authentication-reset and readiness contracts but does
not independently own profile repair, Service State, leases, or runtime reset.

This lane also owns the fast development boundary: ordinary candidates use the
isolated development runtime and optimized CI Cargo profile; full release builds
remain the production or release gate.

## Dependency Direction

```text
PL-RECIPES
  |-- composes --> PL-AUTH
  |-- composes --> PL-CHALLENGE
  `-- composes --> PL-PLATFORM

PL-AUTH -- uses challenge result --> PL-CHALLENGE
PL-AUTH -- uses profile/runtime --> PL-PLATFORM
PL-CHALLENGE -- uses desktop services --> PL-PLATFORM
PL-BUGFIX -- repairs one owning lane without creating a shadow architecture
```

Platform contracts land before dependent authentication, challenge, or recipe
changes. A dependent lane may develop against a published platform checkpoint,
but the integration order must preserve that dependency.

## Multi-Agent Execution Contract

1. Select one primary product lane before creating a plan, branch, or worktree.
2. Record `Product lane: <ID>` in the plan and `product_lane: <ID>` in the
   active-lane catalog.
3. Give each explicitly admitted top-level development session one primary
   implementation worktree. The bugfix lane may hold two sessions only when
   their source and runtime surfaces are demonstrably disjoint.
4. Keep six as the repository-wide hard ceiling, not a standing allocation.
   The current operative cap is the number of admitted top-level development
   sessions when that number is lower. Four admitted sessions therefore permit
   four primary worktrees. Apply policy 0052 before any primary or auxiliary
   checkout is created or assigned.
5. Assign each shared source surface to one active writer. Other lanes depend
   on a published checkpoint or work through an explicitly recorded overlap.
6. Separate shared contracts from adapters. The platform lane owns common
   authority and state; feature lanes own their policy and composition.
7. Merge dependency providers before consumers. Rebase or merge current
   `main`, rerun changed-surface validation, then integrate the dependent lane.
8. A note, agent session, or passing local test does not establish branch
   custody or completion. Use the active-lane catalog, Git refs, CI, installed
   identity, and receipts for those boundaries.

## Shared-Surface Arbitration

The following surfaces require an explicit primary writer whenever two lanes
need them in the same period:

- service schemas, request actions, generated clients, and output formatting;
- profile acquisition, lease authority, browser lifecycle, and Service State;
- shared desktop capture, perception, controller, input, and verification;
- installer, supervisor, runtime host, development runtime, and doctor;
- README, CLI help, agent skill, and documentation-site parity.

The primary writer freezes the shared contract and publishes a checkpoint.
Dependent lanes limit their branch to lane-owned policy, fixtures, and adapters
until that checkpoint is available. Silent parallel edits to a shared surface
are prohibited.

## Notes and Evidence Routing

[The notes index](notes/README.md) maps current reusable evidence into these
product lanes. Historical notes remain immutable at their current paths. New
notes declare a primary product lane, disposition, owning plan or work item,
and any related lanes. Notes do not become a second backlog: an actionable open
finding must be adopted by a plan or recorded as deferred with a named product
lane.
