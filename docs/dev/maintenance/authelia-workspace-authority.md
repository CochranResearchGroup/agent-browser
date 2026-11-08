# Authelia workspace authority repair

Product lane: PL-BUGFIX
Work item: #597
Status: INSTALLED_ACCEPTED
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

Pre-build review caught two remaining local-role references in the workspace
auth guide and denied-access UI. Stop the exact owned first candidate Cargo
scope before qualification, retain its superseded failure receipt, correct both
references and build a clean successor. No failed candidate is installable.

## Final source review

The complete stream compartment passes268 tests and five child fixtures; the
same two producer drivers remain ignored. Strict workspace Clippy, Rustfmt,
patch hygiene, owner-auth UI/transport/operator contracts, dashboard TypeScript
and production build, policy wiring, changed links, handoff docs and docs site
build pass. Five selected workstation/provider tooling checks pass. Installer
qualification remains pending on the explicitly pinned successor binary.

The first denied-access copy change removed the established Access denied.
prefix and the mounted assertion timed out. Restoring that prefix while removing
local-role guidance passes the unchanged UI test; retain the failed run.

### Standards

Review baseline812e436c9 through the complete working diff against AGENTS.md,
code discovery/runtime proof, architecture and testing policies. No accepted
blocking finding remains. The final follow-up changes only denied-access UI
copy and guidance; all Rust source bytes are unchanged from the successful
stream/Clippy checkpoint. Reuse those Rust behavior gates; freshly qualify
embedded assets and the exact binary after the dashboard rebuild.

### Spec

Issue597 and the operator clarification supersede591's local-role requirement.
Owner verification alone grants workspace operator access; old local mappings
and roles cannot add a gate. Exact owner principal attribution, disabled native
password/cookie authentication, private signing material, target/controller/
generation checks and profile policies remain. No accepted blocking finding
remains. Producer pixels, rail latency, footer behavior and live publication
are separate from this source qualification.

Review summary: Standards0 accepted blockers; Spec0 accepted blockers.

## Exact artifact qualification

The production-shaped release candidate seals successfully from clean source
cdf4b2b52. All634 executable inputs and the binary digest verify unchanged.
Its public isolated HTTP checks admit an owner-verified principal without any
local account or mapping and reach operator request validation. Anonymous,
spoofed, expired, denied and unavailable-owner cases remain refused. Valid old
native credentials and a correctly signed legacy cookie cannot authenticate.
The same public admission probe fails on the installed pre-repair artifact.
Refreshed embedded assets pass13 tests after the final denied-access copy change.

The API/MCP parity check exposed a pre-existing skill-size violation: baseline
14995bytes exceeds the unchanged10000byte limit. Move detailed profile-sharing
and operation guidance into the existing linked CLI reference, preserving those
blocks and correcting only relative links/headings. The main skill is9585bytes;
the real parity, handoff and changed-link checks now pass without weakening the
checker. This docs-only repair changes none of the634 sealed executable inputs.

The exact installer fixture and no-launch collection smoke remain pending at
this custody checkpoint. Runtime publication and live desktop acceptance remain
separate gates. No comprehensive-project or cross-platform claim is made.

## Merge qualification

Exact-candidate installer qualification passes in33.159seconds and retires its
two owned hosts with zero residue. The exact-candidate collection smoke passes
without browser launch. All source and artifact gates above are complete; the
previous skill-size, UI-prefix and binary-selection failures are resolved.
The maintained builder's release profile and local Linux/WSL target differ from
unexecuted hosted/platform environments; no hosted CI was dispatched. The source
candidate remains the clean cdf4b2b52 seal, with634 unchanged executable inputs
after docs-only follow-ups. Standards and Spec reviews have no accepted blocker.
Preserving runtime adoption and actual desktop viewing remain the next gate.

## Installed acceptance and closeout

PR598 is merged. Supported preserving adoption of the exact sealed release
candidate is accepted. The live executable digest matches the seal and its
dashboard manifest is built with82 assets. Doctor through the installed
entrypoint passes; advisory workspace-build and operator-journey observations
remain. An initial doctor invoked through the build-artifact path hit its
expected executable-path comparison; live supervisor path/hash match the
installed generation. No supervisor repair or broad restart was needed.

Independent anonymous browser access redirects to the expected Cooper realm.
The existing independent agent sign-in reaches the workspace with ingress
authentication and native authentication disabled. A current managed-tab
selection returns200 and operatorVisible ready, visibly renders the actual
native browser desktop in about7.3seconds, and removes the empty-selection hint.
Native login still returns403 native_auth_disabled inside that session. The
original Manual login row is absent from current inventory, so this is a current
managed-task acceptance, not a claim that the original target was replayed.

All immediately preceding browser-root identities survive the upgrade. One
child process is absent in the later comparison; no cause or universal process
preservation is claimed. Both task-owned browser contexts are closed with fresh
OS proof of zero owned residue. Shared daemon and unrelated browsers remain.
Private instance, transaction, process, screenshot and authenticated acceptance
evidence stays outside Git. No website login, desktop input, all-tile, broad
rail-performance or cross-platform qualification is claimed.

Installed guidance receives the reviewed auth paragraphs and sign-in reference
in both skill roots. Preserve its other local customizations rather than
claiming full installed/repository byte parity. Source and runtime acceptance
are complete for597. Retain the existing checkout for sealed-candidate custody
with next review2026-11-09; do not remove it outside governed closeout.
