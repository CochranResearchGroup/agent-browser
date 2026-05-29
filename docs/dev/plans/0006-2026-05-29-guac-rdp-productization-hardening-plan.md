# Guacamole RDP Productization Hardening Plan

Date: 2026-05-29
State: OPEN
Lane: P06
Outcome: PARTIAL

Current state: P05 proved that the installed `0.27.0` checkpoint runtime can
operate two simultaneous Guacamole/RDP browser routes when local route-pool,
display, access, viewer, and Guacamole URL preconditions are already satisfied.
P06 turns that checkpoint into productization work. It does not prepare or
publish a formal release.

2026-05-29 update: the first P06 implementation slice addressed the known
P05 operator issues around undocumented live-gate inputs and incomplete doctor
diagnostics. The rebuilt installed checkpoint now reports remote-view privilege
readiness from install doctor, reports top-level remote-view issue codes plus
viewer prerequisites, hydrates the many-to-many live harness from doctor output,
auto-discovers common viewer browsers, and fails non-local Guacamole route URLs
with the explicit `non_embeddable_guacamole_url` diagnostic. The installed
local many-to-many gate passed from the installed command.

P06 remains open because this turn did not freshly prove the clean-machine
first-install path asks for sudo exactly once, and because the product
invariant that names service readiness inside install doctor still needs an
explicit ownership decision.

## Purpose

Harden the Guacamole/RDP remote browser operation milestone until it is
supportable as a release target. The release bar is operational, not merely
versioned:

- many-to-many Guacamole/RDP browser operation works from the installed command
- first install asks for sudo exactly once
- recurring route-pool, display-access, and helper maintenance does not ask for
  interactive sudo
- `agent-browser install doctor` and `agent-browser doctor remote-view` explain
  every supported blocker with concrete remediation
- live gates can distinguish missing preconditions from product regressions

## Non-Goals

- Do not publish a GitHub release or move release markers.
- Do not add a docs changelog entry for `0.27.0`.
- Do not broaden the milestone to CDP streaming or VNC/noVNC.
- Do not store workstation secrets, browser auth state, or raw live artifacts
  in repo docs.
- Do not replace the Guacamole/RDP path with a new backend while this lane is
  open.

## Product Invariants

P06 is not complete until these invariants are true:

- `agent-browser install --with-deps --with-remote-view-privileges` has a
  clear one-time-sudo contract and records whether the required privileged
  setup is already complete.
- Re-running install, route-pool setup, route-display access grants, and doctor
  commands after first install does not require interactive sudo.
- `agent-browser install doctor --json` reports installed binary, workspace
  binary, pnpm package binary, helper, sudoers, group, browser-build, service,
  and version drift with stable machine-readable fields.
- `agent-browser doctor remote-view --json` reports Guacamole web reachability,
  guacd reachability, RDP backend reachability, route-pool inventory, route
  display discovery, display access, viewer prerequisites, and simultaneous
  viewing readiness with actionable issue codes.
- Many-to-many live validation can run from the installed command without
  undocumented environment guessing.
- Public or authenticated Guacamole URLs fail with an explicit diagnostic when
  the harness needs a local embeddable URL.

## Slices

### Slice A | Installer And Doctor Contract Audit

Status: COMPLETE for the already-provisioned checkpoint.

Goal: compare the current installed checkpoint against the one-time-sudo and
fully diagnostic release bar.

Tasks:

- Run or inspect `agent-browser install doctor --json` and
  `agent-browser doctor remote-view --json` from the installed checkpoint.
- Identify which P05 manual preconditions are installer-owned setup,
  doctor-owned diagnostics, or live-test-only inputs.
- Audit the privilege helper, sudoers rule, group membership, route-display
  access grants, and route-pool setup scripts for idempotence.
- Record gaps in a dated P06 note before editing source.

Exit criteria:

- Every manual precondition from P05 has an owner.
- Any required first-install sudo action is named and scoped.
- Re-run behavior is documented for already-installed machines.

Result:

- Manual P05 preconditions are now assigned to installer, doctor, or live-test
  ownership in
  `docs/dev/notes/2026-05-29-p06-installer-doctor-productization.md`.
- Current installed doctors prove `requiresInteractiveSudo=false` on the
  already-provisioned machine.
- The clean-machine first-install proof is still deferred to Slice B.

### Slice B | One-Time Sudo Install Path

Status: OPEN.

Goal: make the first install perform all required privileged setup behind one
interactive sudo boundary.

Tasks:

- Ensure `agent-browser install --with-deps --with-remote-view-privileges`
  groups privileged setup into one explicit authorization sequence.
- Keep the privileged helper root-owned and narrowly scoped.
- Make repeated setup detect completed helper, group, sudoers, and display
  access state without re-prompting.
- Add focused tests or script checks for idempotent helper installation.

Exit criteria:

- First install has one clear sudo prompt boundary.
- Subsequent setup and doctor runs report `requiresInteractiveSudo=false` when
  the machine is already provisioned.

Remaining issue:

- Re-run readiness is proven on the current machine, but the first-install
  "exactly once" behavior has not been validated on a clean host or equivalent
  reset fixture.

### Slice C | Fully Diagnostic Doctor Surface

Status: MOSTLY COMPLETE for blockers observed in P03 through P05.

Goal: make doctors good enough to replace ad hoc operator spelunking.

Tasks:

- Add stable issue codes and remediation text for missing helper, missing group
  membership, stale binary, missing Chrome build, Guacamole ingress failure,
  guacd failure, RDP TCP failure, empty route pool, duplicate route display,
  missing display access, missing viewer executable, and non-embeddable
  Guacamole URL.
- Keep JSON output structured for automation and text output useful for humans.
- Align CLI help, README, skill guidance, docs, and contracts when fields or
  commands change.

Exit criteria:

- Doctors identify every blocker that appeared during P03 through P05.
- A failed many-to-many gate points back to a doctor issue instead of requiring
  manual log archaeology.

Result:

- `agent-browser install doctor --json` now reports remote-view privilege
  readiness, helper, sudoers, group, membership, helper check, nested issues,
  installed binary, workspace binary, pnpm package binary, browser-build state,
  and version state.
- `agent-browser doctor remote-view --json` now reports stable top-level
  issues, remediations, viewer prerequisites, privilege readiness, route-pool
  readiness, route-display readiness, display access, and many-to-many
  readiness.
- The remaining diagnostic decision is whether install doctor should directly
  report service readiness, or whether service readiness remains composed in
  remote-view doctor.

### Slice D | Route-Pool And Display Maintenance

Goal: make route-pool state durable and repairable.

Tasks:

- Make route-pool setup and sync idempotent for existing route users and
  Guacamole records.
- Preserve clear boundaries between managed route records and legacy shared
  route records.
- Verify route-display inspection reports display names, route identity, and
  missing access grants without leaking secrets.
- Add repair or apply boundaries where a doctor can explain a fix without
  silently mutating state.

Exit criteria:

- Route-pool readiness is reproducible after service restarts.
- Duplicate or missing display assignments produce deterministic diagnostics.

### Slice E | Live Gate Productization

Status: COMPLETE for the current Guacamole/RDP gate.

Goal: turn the many-to-many live harness into a reliable release gate.

Tasks:

- Prefer installed `agent-browser` when testing runtime behavior.
- Validate viewer executable discovery or report explicit missing-input issue
  codes.
- Validate local embeddable Guacamole URL requirements before opening browsers.
- Keep artifacts under `/tmp` and record only redacted summaries in repo notes.

Exit criteria:

- A clean provisioned machine can run the gate from documented commands.
- Harness failures are classified as precondition failure, product regression,
  or external service failure.

Result:

- The harness now prefers installed `agent-browser`, hydrates route-pool and
  route-display environment from `agent-browser doctor remote-view --json`,
  auto-discovers common viewer browsers, checks OCR tooling, and fails public
  Guacamole route URLs with `non_embeddable_guacamole_url`.
- The local embeddable Guacamole gate passed from the installed command with
  artifacts at
  `/tmp/agent-browser-rdp-guac-many-to-many-2026-05-29T14-06-07-291Z`.

### Slice F | Release-Readiness Handoff

Status: OPEN.

Goal: decide whether the hardened operational milestone is ready for a future
formal release lane.

Tasks:

- Run selected validation from the changed surfaces.
- Run installed install doctor, remote-view doctor, default-profile attach, and
  many-to-many live gate.
- Write a P06 validation note with pass/fail evidence and residual risk.
- Update `ROADMAP.md` and `RUNBOOK.md`.

Exit criteria:

- Either P06 closes with release-readiness evidence, or it leaves a bounded
  next plan for remaining operational blockers.
- Formal release work remains a separate explicit lane.

Current handoff:

- P06 does not close in this slice.
- The next bounded work is Slice B clean-install sudo proof plus the Slice C
  install-doctor service-readiness ownership decision.
