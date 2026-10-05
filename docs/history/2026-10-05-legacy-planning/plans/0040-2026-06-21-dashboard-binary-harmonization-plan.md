# Dashboard Binary Harmonization Plan

Date: 2026-06-21
State: DONE
Lane: P40

## Purpose

Keep the operator-visible dashboard UI and the installed `agent-browser`
binary from drifting apart silently.

The current failure mode is that source and dashboard tests can pass while the
running user service still serves an older embedded dashboard bundle or an
older executable contract. That makes the left rail, remote-view routes, and
detected-browser controls appear current when the binary that serves them is
not current.

## Requirements

- The running dashboard must expose the binary and embedded-dashboard contract
  it is serving.
- The local publish command must prove the live dashboard service is serving
  the just-built bundle, not only that source tests passed.
- The UI must have a clear path to detect and display runtime drift.
- The contract must distinguish version, binary identity, dashboard bundle
  identity, service contract version, and feature support.
- The solution must preserve the existing user-scoped install boundary and not
  turn local dashboard publication into a formal release.

## Non-Goals

- Do not publish to GitHub Releases, npm, Homebrew, or upstream channels.
- Do not require live publication for test-only source edits.
- Do not expose secrets, auth state, profile contents, or private browsing
  data in runtime manifest responses.
- Do not treat detected non-owned browsers as agent-browser-owned streams.

## Slice A | Runtime Manifest And Publish Readback

State: DONE

Goal: make the running binary identify the dashboard bundle and feature
contract it is serving, then make `pnpm publish:local-dashboard` verify that
identity after restart.

Deliverables:

- Add a read-only dashboard runtime manifest endpoint.
- Include package version, service contract version, dashboard asset count,
  dashboard asset SHA-256, supported UI feature flags, current executable path,
  and current executable SHA-256 when readable.
- Teach `scripts/smoke-local-dashboard-runtime.js` to read and report the
  manifest.
- Teach `scripts/publish-local-dashboard-runtime.js` to fail if the live
  manifest does not match the built binary and embedded dashboard bundle after
  restart.
- Add focused Rust and Node coverage for the manifest shape and publish smoke
  expectations.

Acceptance:

- `curl http://127.0.0.1:4848/api/runtime/manifest` returns public-safe JSON
  without depending on dashboard login.
- The publish script JSON includes `runtimeManifest`.
- The publish script fails on a manifest mismatch instead of succeeding based
  only on a served marker string.
- The manifest includes `workspace.detectedBrowsers` and
  `workspace.noRetainedLiveRail` feature flags.

Completed on 2026-06-21:

- Added `/api/runtime/manifest` as a public-safe read-only dashboard endpoint
  before dashboard API auth enforcement.
- Added the same manifest response to the lower-level stream HTTP router.
- The manifest reports:
  - `schemaVersion=agent-browser.runtime-manifest.v1`
  - `packageVersion`
  - `serviceContractVersion=service-ui-runtime.v1`
  - embedded dashboard asset count, total bytes, and SHA-256
  - current executable path and SHA-256 when readable
  - supported UI features, including `workspace.detectedBrowsers` and
    `workspace.noRetainedLiveRail`
  - API shape versions for sessions, service status, and view streams
- Updated `scripts/smoke-local-dashboard-runtime.js` to fetch and validate the
  runtime manifest.
- Updated `scripts/publish-local-dashboard-runtime.js` to fail when the live
  runtime manifest executable SHA-256 does not match the installed binary.

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture
node --check scripts/smoke-local-dashboard-runtime.js
node --check scripts/publish-local-dashboard-runtime.js
pnpm publish:local-dashboard -- --skip-browser --expect-marker "Detected non-owned browsers" --json
```

Live publish evidence:

- Dashboard service restarted from PID `1154` to PID `84963`.
- Live manifest executable SHA-256:
  `2f3a5286b32e014fa79a7daa2851ec800ee33e709bd1e703ae6f3c6ef3d4f92b`.
- Installed binary SHA-256:
  `2f3a5286b32e014fa79a7daa2851ec800ee33e709bd1e703ae6f3c6ef3d4f92b`.
- Embedded dashboard asset SHA-256:
  `222719195880512f64ae9be0d633bfd190e258cae6d7fbf6a2b75aa4308f769a`.
- Served bundle marker `Detected non-owned browsers` was found in a live JS
  chunk.

Validation:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture
node --check scripts/smoke-local-dashboard-runtime.js
node --check scripts/publish-local-dashboard-runtime.js
pnpm publish:local-dashboard -- --skip-browser --expect-marker "Detected non-owned browsers" --json
```

## Slice B | UI Drift Surface

State: DONE

Goal: make runtime mismatch visible in the dashboard instead of relying on
operator memory.

Deliverables:

- Fetch `/api/runtime/manifest` from the dashboard shell.
- Compare the served manifest with the UI bundle's expected contract.
- Render a persistent warning when the running binary lacks required UI
  features or when the bundle/manifest contract is inconsistent.
- Gate feature-specific UI controls on manifest feature flags.

Acceptance:

- A simulated stale manifest renders a visible runtime drift warning.
- Unsupported controls degrade with an explanatory disabled state.
- The warning names the publish command or doctor command needed to resolve
  the mismatch.

Completed on 2026-06-21:

- Dashboard shell fetches `/api/runtime/manifest` with `cache: "no-store"`.
- Dashboard shell validates `service-ui-runtime.v1`, dashboard bundle identity,
  and required runtime features.
- A visible `Runtime contract drift` warning renders above the dashboard
  control surface when the manifest is missing, stale, or missing required
  feature support.
- The warning includes the local publish command needed to republish the live
  dashboard runtime.

Validation passed:

```bash
pnpm test:dashboard-workspace-navigator
pnpm build:dashboard
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture
```

## Slice C | Doctor Integration

State: DONE

Goal: make CLI doctor output and dashboard manifest readback describe the same
runtime identity.

Deliverables:

- Add runtime manifest fields to `agent-browser install doctor --json`.
- Add dashboard service manifest readback to remote-view doctor when the
  dashboard service is running.
- Classify manifest mismatch as readiness-impacting when the requested route or
  UI feature requires the missing binary capability.

Acceptance:

- Install doctor reports installed binary checksum and dashboard manifest
  checksum.
- Remote-view doctor reports whether the served dashboard contract supports the
  selected route provider.
- Doctor next action is a runnable command.

Completed on 2026-06-21:

- Re-exported the runtime manifest builder as the single crate-local source for
  dashboard runtime identity.
- Added `dashboardRuntime` to `agent-browser install doctor --json` and printed
  the contract, dashboard SHA-256, and executable SHA-256 in human output.
- Added `dashboardRuntime` to `agent-browser doctor remote-view --json` by
  lifting the install doctor manifest readback, and printed the same fields in
  human output.
- Added focused Rust coverage for the shared runtime manifest shape and
  remote-view doctor manifest lifting.

Validation passed:

```bash
cargo fmt --manifest-path cli/Cargo.toml -- --check
cargo test --manifest-path cli/Cargo.toml install_doctor -- --nocapture
cargo test --manifest-path cli/Cargo.toml remote_view_doctor -- --nocapture
cargo test --manifest-path cli/Cargo.toml runtime_manifest -- --nocapture
pnpm publish:local-dashboard -- --skip-browser --expect-marker "Runtime contract drift" --json
~/.local/bin/agent-browser install doctor --json
~/.local/bin/agent-browser doctor remote-view --json
```

Live doctor evidence:

- Published dashboard service restarted from PID `38524` to PID `77873`.
- Live manifest executable SHA-256:
  `1ceea7e15dc984996fdecaca6143a6f122676861555a8608bcc31719cb13f94d`.
- Installed binary SHA-256:
  `1ceea7e15dc984996fdecaca6143a6f122676861555a8608bcc31719cb13f94d`.
- Embedded dashboard asset SHA-256:
  `602d1b4d7bbb84b7c956065fa4ac3bb6320ce99e2fbea7d56e609db2e63b1341`.
- `agent-browser install doctor --json` reported `dashboardRuntime` with
  `service-ui-runtime.v1`, dashboard SHA-256
  `602d1b4d7bbb84b7c956065fa4ac3bb6320ce99e2fbea7d56e609db2e63b1341`, and
  executable SHA-256
  `1ceea7e15dc984996fdecaca6143a6f122676861555a8608bcc31719cb13f94d`. The
  command exited nonzero because of existing PATH versus pnpm/workspace binary
  drift, not because the runtime manifest was unavailable.
- `agent-browser doctor remote-view --json` reported `status=ready` and the
  same `dashboardRuntime` dashboard and executable SHA-256 values.

## Slice D | Documentation And Closeout Discipline

State: DONE

Goal: make the operator contract durable.

Deliverables:

- Update README dashboard development notes.
- Update `skills/agent-browser/SKILL.md`.
- Update docs site dashboard/service-mode guidance if needed.
- Add closeout wording for dashboard-visible changes:
  - source-only validation
  - live published validation
  - manifest mismatch or skipped publish

Acceptance:

- Future agents can tell whether the live dashboard is current from the
  manifest and publish output.
- Docs explain the source bundle, embedded binary, and running systemd service
  boundary.

Completed on 2026-06-21:

- Updated README dashboard runtime notes to describe `/api/runtime/manifest`,
  live manifest checksum validation, and the `Runtime contract drift` warning.
- Updated `skills/agent-browser/SKILL.md` so future agents distinguish source
  validation, installed-binary publication, manifest readback, and detected
  non-owned browser grouping.
- Updated docs site dashboard guidance with the manifest boundary and current
  left-rail grouping.
