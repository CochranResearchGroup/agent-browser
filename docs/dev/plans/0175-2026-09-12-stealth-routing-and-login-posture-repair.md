# Plan 0175 | Stealth Routing And Login Posture Repair

Date: 2026-09-12

State: OPEN

Consolidation: required

Lane: P175

Roadmap: P175

Branch: `fix/plan-0175-stealth-routing-auth`

Target: `main`

Integration: short-lived branch through the protected `main` workflow

## Objective and authority

Make headed `stealthcdp_chromium` the built-in Google and Gmail posture, allow
first sign-in through its CDP control plane, and preserve explicit browser-build
requests across CLI, HTTP, MCP, and nested service-request command shapes. Keep
detached CDP-free login as a hard site-policy exception. The operator authorized
implementation and commit. This plan authorizes provider-free source, tests,
documentation, and isolated candidate validation. It does not authorize account
sign-in, challenge attempts, production installation, profile mutation, browser
launch, provider effects, or release publication.

## Current state

The same headed Google Search baseline reached `/sorry/` in stock Chrome while
the installed stealth build returned normal results with `navigator.webdriver`
false. Source inspection found that nested `params.browserBuild` is parsed but
not marked explicit, allowing policy or defaults to replace it. The built-in
Google and Gmail policies still select stock Chrome and derive detached manual
seeding even though stealth CDP is now the preferred build.

## Consolidated batch

One batch repairs command-shape precedence and login posture because both decide
the effective build before a service-owned authentication attempt. The build ID
remains `stealthcdp_chromium`; it is not renamed to `chrome` because that label
already identifies stock Chrome and external browser-family claims remain
observable facts.

## Non-goals and invariants

- Do not weaken a site policy with `requiresCdpFree: true`.
- Do not claim stealth is a captcha bypass or authentication guarantee.
- Do not mark an unauthenticated profile fresh without bounded auth evidence.
- Do not retry a failed sign-in or challenge blindly.
- Do not modify production Service State, profiles, cookies, or credentials.
- Preserve explicit site and profile identity selection and browser capability
  validation gates.

## Delivery sequence and budget

1. Add focused regressions for nested explicit build selection, stealth Google
   readiness, and a hard CDP-free exception.
2. Centralize explicit-build detection, align capability evidence, and derive
   manual-seeding posture from the effective browser build.
3. Update built-in Google and Gmail policies plus every required user-facing
   documentation surface.
4. Run focused tests, format, strict clippy, selected contract checks, and the
   repository validation selector. Use one Cargo job if current host thread
   pressure prevents the normal eight-job loop.
5. Commit one reviewable candidate. Installation and live account acceptance
   require a separate explicit checkpoint.

No subagents are assigned; current orchestration policy prohibits delegation.

## Worker assignments

- Critical-path owner: primary Codex agent.
- Parallel workers: none.
- Write surfaces: service command routing, access-plan evidence, profile
  readiness, built-in site policy, required docs, this plan, catalog, and
  runbook.

## Evidence and exit

| Requirement | Evidence | State |
|---|---|---|
| Nested explicit build preserved | Focused service access-plan test | passed |
| Stealth first login attachable | Focused profile-readiness test | passed |
| Hard CDP-free exception preserved | Focused profile-readiness test | passed |
| Public guidance aligned | Help, README, skill, docs-site diff | passed |
| Candidate qualified | fmt, clippy, focused and selected checks | correction focused; full CI pending |
| Reviewable custody | Clean committed branch and exact head readback | passed |

Terminal success for this slice is a committed, provider-free qualified
candidate. Production installation, real Google sign-in, captcha solving, and
consumer workflow acceptance remain separate authority and evidence gates.

## Checkpoint P0175-C01 | 2026-09-12

State transition: `planned to implementation_active`.

Acceptance state: focused regression compilation in progress.

Progress classification: `implementation`; the nested explicit-build defect and
the overly broad detached-login rule are isolated in one bounded batch.

Evidence: source baseline `976e2031`; stock versus stealth Google baseline from
the current operator-directed comparison.

Material blockers: host-wide thread pressure caused two compiler/cache startup
failures before application tests ran. The supported one-job cache-off lane is
continuing without weakening test assertions.

Next action: complete the focused tests, then run batch-matched qualification.

## Checkpoint P0175-C02 | 2026-09-12

State transition: `implementation_active to source_qualified`.

Acceptance state: provider-free source qualification passed; commit and review
custody remain.

Progress classification: `qualification`; nested explicit build precedence,
stealth-CDP first-login posture, and the explicit CDP-free exception now have
focused regression coverage.

Evidence: the complete `service_access_plan` focused group passed 45 tests and
the complete `service_model` focused group passed 39 tests. Formatting and
strict workspace Clippy passed. The selected service API/MCP parity, generated
client contracts and types, capability-registry draft, remote-view docs,
workstation/Guacamole fixtures, documentation production build, and the exact
branch-binary workstation install fixture passed. The validation selector
completed successfully.

Qualification notes: two initial compiler starts failed before tests under
host-wide thread pressure; the policy-supported one-job cache-off lane passed.
The first workstation fixture selected the package-downloaded upstream binary,
which lacks this fork's workstation command. Building the branch CLI and
pinning that exact executable produced a passing source-free fixture. Neither
event changed production state.

Next action: commit the qualified candidate, record exact branch custody, and
submit it for review. Production installation and live account acceptance stay
outside this plan's current authority.

## Checkpoint P0175-C03 | 2026-09-12

State transition: `source_qualified to integration_ready`.

Acceptance state: the provider-free candidate is committed at source checkpoint
`27ec5aae0f9100e67306cc1c0e53694326af8cdd`; review and integration remain.

Progress classification: `custody`; the branch is clean, one source commit
ahead of its `main` baseline, and ready for the protected review workflow.

Next action: review and merge the two-commit source plus custody packet.
Installation and live acceptance require separate operator authority.

## Checkpoint P0175-C04 | 2026-09-12

State transition: `integration_ready to post_merge_correction`.

Acceptance state: PR 55 merged as `97aa399a`, but its comprehensive Rust gate
then failed three stale MCP resource assertions. Installation is withheld.

Progress classification: `qualification_repair`; ordinary built-in Google
readiness now correctly returns `unknown` and `attachable_ok`. The MCP readiness
and allocation fixtures still expected detached manual seeding. The retained
handoff fixture now declares `requiresCdpFree: true` so it continues to protect
the explicit detached exception.

Evidence: the four focused `read_profile_` MCP tests pass, formatting passes,
and strict workspace Clippy passes. The already built merged candidate is not
installable evidence while full CI is red.

Authority update: the operator authorized merge, worktree consolidation, and
production installation. Runtime mutation remains gated on a green corrected
source head, preserving installer preflight, and coherent supervisor and
Service State readback.

Next action: commit and publish the correction through protected CI, merge only
when green, then rebuild and run transactional installation.

## Checkpoint P0175-C05 | 2026-09-12

State transition: `post_merge_correction to correction_integration_ready`.

Acceptance state: correction committed at source checkpoint
`0572b065522e190f29a57e8e6d105836c2ef67d3`; protected CI pending.

Progress classification: `custody`; the corrective branch is clean and one
commit ahead of merged main before this documentation receipt.

Next action: open the correction PR and require every fast gate to pass before
merge or installation.
