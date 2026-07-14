# One-Time Profile Sprawl Defect

Date: 2026-07-05
Source incident: Texas SOSDirect temporary-login handoff.

## Context

During the Texas SOSDirect temporary-login handoff, I created multiple named
runtime profiles while trying to recover from route visibility problems,
browser-build confusion, and SOSDirect 500 errors.

Profiles and sessions created or used in this workflow included:

- `tx-sos-temp`
- `tx-sos-temp-b`
- `tx-sos-temp-remote-b`
- `tx-sos-temp-stock-b`
- `tx-sos-google-chrome`
- `tx-sos-google-chrome-b`

That was the wrong default behavior for a one-time operator-assisted task.

## What Went Wrong

The task did not require a long-lived named identity. It required a visible
browser for a temporary login/payment flow and then agent-side scraping after
the operator finished the human-only step.

Instead of using the agent-browser recommended one-time/default task profile
path, I started by hand-naming temporary profiles and kept creating new ones as
the route/browser state became confusing.

This made the system more fragile:

- user data could not accumulate in one predictable profile;
- SOSDirect cookies and ASP session state were scattered across profiles;
- retained profile, browser, route, display, and tab records multiplied;
- browser-family proof became harder because profile name and requested
  browser build did not prove the actual executable;
- stale Route A and Route B state became harder to reason about;
- operator handoff debugging required inspecting multiple sessions instead of
  one canonical task lane.

The profile sprawl also undermined a core agent-browser goal: operators should
not need to coordinate browser identity manually for ordinary one-off work.

## Expected Behavior

For ordinary one-time tasks, agent-browser should make the recommended
one-time/default profile path the natural first move.

An agent opening an operator-visible browser for a temporary login should not
need to invent profile names unless one of these conditions is true:

- the operator explicitly asks for a separate browser identity;
- the selected profile is locked and cannot be safely reused;
- the requested browser family is incompatible with the existing profile;
- the task requires durable login state under a known account identity;
- a reviewed isolation policy says a throwaway profile is required.

When a new throwaway profile is required, agent-browser should create and label
it as a managed one-time task profile, not leave the agent to invent arbitrary
runtime-profile names.

## Product Requirements

Agent-browser should provide a first-class one-time task profile contract:

- one canonical generated profile/session identity per one-time task;
- explicit reuse of that profile across retries inside the same task;
- clear retention and cleanup policy for that profile;
- visible service-state evidence showing whether the profile is default,
  one-time, durable, imported, or operator-named;
- browser-family compatibility proof before launch;
- actual executable proof after launch.

The service access-plan and `remote-view open` path should warn when an agent
passes a new arbitrary `--runtime-profile` for a task that looks like a
one-time operator handoff.

## Acceptance Criteria

- A one-time operator handoff can be opened without hand-naming a runtime
  profile.
- Retries for the same one-time task reuse the same managed task profile unless
  there is a concrete incompatibility or lock conflict.
- Service state identifies the profile class, for example `default`,
  `managed_one_time`, `durable_named`, or `operator_supplied`.
- `remote-view open` reports both requested browser build and actual executable
  path in compact proof.
- A requested `stock_chrome` launch cannot silently run the stealth Chromium
  executable without a warning or hard failure.
- Retained profile/session cleanup can remove abandoned one-time profiles
  without touching durable user profiles.

## Follow-Up

This incident should become a regression case: start an SOSDirect-style
temporary login handoff, force one route retry, and verify the system keeps one
managed one-time profile while preserving route, browser, display, and
executable proof.
