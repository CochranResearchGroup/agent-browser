# Plan 0122: Exact Profile Capability Compatibility

Date: 2026-08-23

State: COMPLETE

Lane: P122

Source baseline: `8c81de89e8103f9d990af7fbb7bb752d6473d1e9`

Implementation commit: `990e6b31`

## Goal

Make the read-only Service access plan and the executable no-launch browser
capability preflight use one exact profile, host, and executable compatibility
identity. A compatibility declaration for another profile must never qualify
the selected profile merely because both profiles use the same host or browser
executable.

## Scope

1. Add one provider-neutral exact compatibility predicate to the Service model.
2. Use it in both access-plan evidence selection and executable preflight.
3. Add a deterministic regression fixture for an account-selected
   `bill-soylei` profile when the only compatible row names another profile.
4. Validate the affected Rust model, access-plan, and preflight surfaces plus
   generated client and API/MCP contract parity.

## Acceptance

- The access plan includes compatibility evidence only when `profileId`,
  `hostId`, and `executableId` all match the selected route.
- A same-host, same-executable row for another profile produces zero matching
  rows and `profileCompatibility.status=not_declared`.
- Executable preflight continues to fail closed when the exact triple is absent
  or blocked.
- No browser is launched, no profile is changed, and no installed runtime or
  tenant state is mutated.

## Completion Receipt

Commit `990e6b31` implements the shared predicate and the red-to-green fixture.
The new fixture first failed because one other-profile row was borrowed, then
passed with zero rows after the repair. Formatting, strict Clippy, 34
service-model tests, 41 access-plan tests, 4 capability-preflight tests,
API/MCP parity, generated-client checks, and client type checks pass.

This plan is complete at the source and provider-free validation boundary.
Installing or qualifying this source against a tenant-owned runtime remains a
separate, explicitly governed step for the consuming workflow.

## Hard Stops

- Do not launch or attach a browser while validating this decision contract.
- Do not synthesize compatibility for a profile from host-only or
  executable-only registry evidence.
- Do not edit tenant profiles, capability registries, leases, or authentication
  state as part of this source repair.
- Do not claim installed-runtime acceptance from source tests.
