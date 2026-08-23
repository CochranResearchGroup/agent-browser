# P125 Development Runtime Isolation Acceptance

Date: 2026-08-23

Result: ACCEPTED

Authority: DEVELOPMENT RUNTIME AND INGRESS EFFECTS | PRODUCTION READ-ONLY

## Accepted Runtime

- stable development command: `~/.local/bin/agent-browser-dev`;
- selected generation: `0.28.0-43d481bfc913`;
- selected binary SHA-256:
  `43d481bfc9135f9c13edfecdf5c0fb066715e87841967cb510a08b18ab4e58db`;
- development runtime host, dashboard backend, and dashboard units are active;
- the exact unit processes own ports 4951, 4949, and 4948 respectively;
- the runtime manifest, backend manifest, and local ingress all report
  `runtimeEnvironment=development`;
- the dashboard displays a visible Development identity;
- `agent-browser-dev.localhost` and the protected external
  `agent-browser-dev.ecochran.dyndns.org` route reach the development dashboard;
- the development ingress has no `/guacamole` route to the production provider.

An authenticated external no-browser smoke completed login, cookie reuse,
authenticated status, service API, and runtime-manifest checks. It returned
the exact selected development generation and the development environment.

## Production Non-Interference

The final development publication receipt compared exact production evidence
before and after activation:

- selected production generation remained
  `0.28.0-4b975a51aa89-d0782705d5ff`;
- production process identities remained PID 75310 with start token 28115109,
  PID 87827 with start token 28121093, and PID 87877 with start token 28121095;
- each process retained the same production generation executable;
- production binary SHA-256 remained
  `4b975a51aa892241ea73cc6e8acef42bb67d781c8b9be43edbc1086f4d7956f8`;
- production dashboard SHA-256 remained
  `99a72ebd185da7a519804c6f44f0e28933278a2b5618247e637f9d73c0635a8c`;
- durable handoff digest remained
  `5b656b4600a1a134a67a0b316760d749e0b8b0a184bd480478779dc9a97a44de`;
- all three retained browser identities and all 63 prior session identities
  remained present and unchanged.

The production service ledger is actively written, so its whole-file digest
is observational rather than an equality gate. The receipt instead compares
the existing browser and session identity projections and permits additive
session evidence. A deterministic fixture proves that mutation of an existing
browser identity fails the non-interference assertion.

Stopping and restarting all three development units produced an empty diff
between fresh production identity snapshots. No production Agent Browser unit
was restarted or reconfigured during P125.

## Publication And Cleanup

Development publication installed a second immutable debug generation, then
reselected the release development generation. Generation cleanup retained
exactly the selected generation and one rollback generation. It cannot select
or remove a production generation.

The disposable development profile and browser-smoke directories were moved
to the desktop trash after process ownership was checked. No corresponding
production profile existed. A final process census found only the three exact
development service processes and no development Chrome process.

The development service GC dry-run found three orphaned remote-display
processes with projected RSS of 83.5 MiB. It made no change because apply
requires a reviewed short-lived token. This preserves the service GC authority
boundary while exposing current cleanup pressure.

## Build Admission

The WSL Cargo wrapper now holds an exclusive lock only while reconciling and
changing claims. Its initial policy admits at most two four-job builds while
preserving 16 GiB of available host memory and enforcing aggregate and
per-build user-systemd memory bounds.

Deterministic fixtures cover one-build and two-build admission, pressure,
stale claims, release, and unavailable user-systemd behavior. During live
validation, observed unrelated pressure held one invocation in a typed wait;
after available memory recovered to approximately 53 GiB, formatting and
strict Clippy were both admitted concurrently.

## Validation

The accepted gates include:

- WSL Cargo safety and Build Admission fixtures;
- development publisher, non-interference, and generation cleanup fixtures;
- Rust formatting and strict Clippy;
- focused Rust runtime-manifest tests;
- dashboard production build and dashboard action-surface fixtures;
- documentation production build;
- service API, MCP parity, generated client, and client type checks;
- local and external authenticated development-dashboard smokes;
- installed development doctor with all checks passing;
- source diff hygiene.

## Retained Observation

An initial disposable managed-profile launch timed out at the invoking client,
then answered a follow-up URL read. Its Chrome process was a child of the
development runtime host and its profile existed only under the development
pseudo-home. Later fresh Chrome probes ended before CDP startup with Chrome
exit code zero and empty stderr, including a no-sandbox diagnostic attempt.

Because dashboard, authentication, service API, manifest, ingress, executable,
unit, port, profile-root, replacement, stop/start, and production
non-interference gates passed, this later host-level Chrome behavior does not
invalidate P125. It remains explicit follow-up evidence for browser launch
diagnostics before P124 relies on a fresh development browser.
