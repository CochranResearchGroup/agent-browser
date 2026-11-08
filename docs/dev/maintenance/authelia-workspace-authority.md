# Authelia workspace authority repair

Product lane: PL-BUGFIX
Work item: #597
Status: ACTIVE
Baseline: 812e436c9a808922d8cb610885536d9771d86bd0

The operator clarified that Authelia admission governs workspace access.
This supersedes591's retained local-role requirement. A verified ingress
principal needs neither local dashboard enrollment nor a local superuser role.
Native passwords and session cookies remain disabled. Preserve exact browser,
target, profile-policy and Native generation checks and Remote View ownership.

Stable seams: dashboard HTTP auth responses prove owner admission without local
roles; request-bound focus proof issuance and verification prove a selected
exact desktop can proceed without local enrollment. They do not prove producer
pixels, latency, footer behavior or installed acceptance. Use the existing serial
isolated Rust runner and synthetic owner; no shared browser effects.

Reuse the retained591 checkout and preserve candidate artifacts. Own auth code
and auth documentation paragraphs;508 retains access diagnostics. Validate
RED/GREEN, stream compartment, Rustfmt, strict workspace Clippy, selected docs
and dashboard checks. Review Standards and Spec separately against baseline.
Merge through a linked PR; production installation is a separate boundary.

## Authentication regression

`AGENT_BROWSER_CARGO_CACHE=off CARGO_TARGET_DIR=cli/target/validation587 scripts/ci/rust-tests.sh --focused authelia_admission_needs_no_local_account_or_role_mapping`
failed before repair: expected200, received503 workspace_auth_unavailable when
the legacy local-user map was absent. The test uses the real owner-verification
HTTP transport, dashboard response and mutation-entry authentication gate.
The first attempt stalled in sccache compiler discovery before compilation;
its exact owned cgroup was stopped and the existing validation target reused
with cache disabled. This is a tooling retry, not a second product RED.

Ranked predictions: local mapping blocks ingress admission; proof issuance
independently reloads local superuser roles; stale target/controller bindings
may independently refuse and must retain their counterexample coverage.

## Desktop selection regression

The authentication regression passes after removing local mapping admission.
`AGENT_BROWSER_CARGO_CACHE=off CARGO_TARGET_DIR=cli/target/validation587 scripts/ci/rust-tests.sh --focused authenticated_operator_focus_survives_dashboard_proxy_and_daemon_verification`
then failed in the actual dashboard normalization/proof issuance path with
operator_focus_authority_required: current superuser and exact target required.
The verified owner principal is absent from the local store. The existing
regression continues through daemon binding and retains all forged-proof,
expiry, target, controller-epoch and reassigned-desktop counterexamples.
The fixture setup now prepopulates only unrelated legacy accounts with inert
password hashes, avoiding unnecessary password generation on each replay.

A selected installer test first chose the unrelated bundled repository binary that lacks
the workstation interface. This is a harness-selection failure; rerun with
an explicitly pinned qualifying binary before claiming that gate.

## Source checkpoint

The desktop proof regression passes through owner verification, normalization
and daemon binding in0.12seconds after fixture setup was minimized. The
authentication regression also passes. Strict workspace Clippy and Rustfmt,
mounted ingress gate/entry, auth transport and operator UI contracts pass.
Policy wiring, links, handoff guidance and docs build pass. The complete stream
compartment and exact-binary installer qualification are pending at this
intermediate source checkpoint. No merge-readiness or installed claim yet.

The candidate build uses the maintained isolated production-shaped builder
through Cargo Signal CLI fallback. Existing dashboard assets are already built;
no dashboard source changed. Preserve the original failed cache attempt and
installer binary-selection failure. Runtime publication remains separate.
