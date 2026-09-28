#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { evaluate } from './check-p218-architecture.mjs';

function write(root, path, contents) {
  const absolute = join(root, path);
  mkdirSync(join(absolute, '..'), { recursive: true });
  writeFileSync(absolute, contents);
}

const root = mkdtempSync(join(tmpdir(), 'p218-architecture-'));
try {
  write(root, 'cli/Cargo.toml', '[dependencies]\nagent-browser-lease-authority = { path = "../crates/agent-browser-lease-authority" }\n');
  write(root, 'crates/agent-browser-service-model/Cargo.toml', '[dependencies]\nagent-browser-lease-authority = { path = "../agent-browser-lease-authority" }\n');
  write(root, 'cli/src/native/browser_session_host.rs', 'std::env::var("AGENT_BROWSER_SESSION_DISPLAY"); default_service_state_path(); load_or_import_profile_catalog(); enum ManagerHandoffAuthority { Legacy, Keeper }\n');
  write(root, 'cli/src/native/remote_view_handoff.rs', 'LockedServiceStateRepository<JsonServiceStateStore> service_profile_lease runtime_owner_binding_for_session cleanup_obligation\n');
  write(root, 'cli/src/native/browser_session_handoff.rs', 'fn attach_manager_handoff() { LockedServiceStateRepository::default_json(); }\n');

  write(root, 'cli/src/native/actions.rs', '"service_remote_view_handoff_resolve" => { handle_service_remote_view_handoff_resolve(cmd, state, attribution).await }\n');
  const fallback = evaluate(root);
  assert.ok(fallback.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'generic_handoff_json_fallback'));
  assert.ok(fallback.rows.find((row) => row.id === 'P05').findings.some((item) => item.id === 'generic_handoff_parallel_authority'));
  write(root, 'cli/src/native/actions.rs', 'service_viewer_lease_request service_viewer_lease_heartbeat service_viewer_lease_release\n');
  write(root, 'cli/src/native/remote_view/open/runtime.rs', 'LockedServiceStateRepository::default_json()\n');
  write(root, 'cli/src/native/remote_view/open/coordinator.rs', 'begin_route_bound_handoff_plan_acquisition(); complete_route_bound_handoff_open();\nfn handle_service_profile_manual_seeding_acquire() { LockedServiceStateRepository::default_json(); }\n');
  write(root, 'crates/agent-browser-service-model/src/service_state.rs', 'agent_browser_lease_authority viewer_leases runtime_owner_registry\n');
  write(root, 'cli/src/native/actions.rs', 'service_viewer_lease_request service_viewer_lease_heartbeat service_viewer_lease_release\n');
  write(root, 'packages/dashboard/src/components/workspace-remote-viewport.tsx', 'service_viewer_lease_heartbeat\n');
  write(root, 'packages/client/src/service-request.js', 'export const ok = true;\n');
  write(root, 'cli/src/workstation_install.rs', 'XRDP_AGENT_BROWSER_ROUTE_A_USERNAME XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD\n');

  const report = evaluate(root);
  assert.equal(report.rows.length, 19);
  assert.deepEqual(
    report.rows.filter((row) => row.status === 'violated').map((row) => row.id),
    ['P02', 'P03', 'P05', 'P09', 'P12', 'P15', 'P16', 'P19'],
  );
  assert.ok(report.rows.find((row) => row.id === 'P15').findings.length >= 3);
  assert.deepEqual(
    report.rows.find((row) => row.id === 'P03').findings.map((item) => item.id),
    ['legacy_json_import_in_default_host', 'legacy_manager_handoff_authority', 'ordinary_handoff_json_repository', 'manager_handoff_json_fallback', 'ordinary_open_json_repository', 'manual_seeding_json_repository'],
  );
  assert.deepEqual(
    report.rows.find((row) => row.id === 'P05').findings.map((item) => item.id),
    ['parallel_manager_handoff_authority', 'competing_handoff_authority', 'manager_handoff_competing_authority', 'ordinary_open_parallel_acquisition', 'manual_seeding_parallel_handoff'],
  );

  write(root, 'cli/src/native/desktop_interaction.rs', `
fn run_configured_interaction() {
  LockedServiceStateRepository::default_json();
  let path = "desktop-input/operations.json";
}

/// Dispatch the controlled provider.
`);
  write(root, 'cli/src/native/desktop_capture.rs', `
impl StateSource for ManagedDesktopStateSource {
  fn snapshot() { load_configured_service_state(); }
}
`);
  const desktopJsonCut = evaluate(root);
  assert.ok(desktopJsonCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'desktop_interaction_json_authority'));
  assert.ok(desktopJsonCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'managed_desktop_json_snapshot'));
  assert.ok(desktopJsonCut.rows.find((row) => row.id === 'P05').findings.some((item) => item.id === 'desktop_interaction_parallel_operation_authority'));
  write(root, 'cli/src/native/desktop_interaction.rs', `
fn run_configured_interaction() { BrowserRuntimeSqliteStore::default_sqlite(); }

/// Dispatch the controlled provider.
`);
  write(root, 'cli/src/native/desktop_capture.rs', `
impl StateSource for ManagedDesktopStateSource {
  fn snapshot() { load_static_configured_service_state(); }
}
`);
  const desktopSqliteCut = evaluate(root);
  assert.ok(!desktopSqliteCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'desktop_interaction_json_authority'));
  assert.ok(!desktopSqliteCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'managed_desktop_json_snapshot'));
  assert.ok(!desktopSqliteCut.rows.find((row) => row.id === 'P05').findings.some((item) => item.id === 'desktop_interaction_parallel_operation_authority'));

  write(root, 'cli/src/native/action_runtime/runtime/launch.rs', 'fn browser_recovery_policy_config_from_env() { std::env::var("AGENT_BROWSER_SERVICE_RECOVERY_RETRY_BUDGET"); }\n');
  const recoveryEnvironmentCut = evaluate(root);
  assert.ok(recoveryEnvironmentCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'ordinary_recovery_environment_authority'));
  assert.ok(recoveryEnvironmentCut.rows.find((row) => row.id === 'P05').findings.some((item) => item.id === 'competing_recovery_policy_authority'));
  write(root, 'cli/src/native/action_runtime/runtime/launch.rs', 'fn recovery_policy() { BrowserRuntimeSqliteStore::default_sqlite(); }\n');
  const recoverySqliteCut = evaluate(root);
  assert.ok(!recoverySqliteCut.rows.find((row) => row.id === 'P03').findings.some((item) => item.id === 'ordinary_recovery_environment_authority'));
  assert.ok(!recoverySqliteCut.rows.find((row) => row.id === 'P05').findings.some((item) => item.id === 'competing_recovery_policy_authority'));

  write(root, 'cli/src/native/remote_view_handoff.rs', 'pub struct RemoteViewHandoff;\n');
  const finalizerOnlyCut = evaluate(root);
  assert.equal(finalizerOnlyCut.rows.find((row) => row.id === 'P03').status, 'violated');
  assert.equal(finalizerOnlyCut.rows.find((row) => row.id === 'P05').status, 'violated');
  write(root, 'cli/src/native/remote_view_handoff.rs', 'LockedServiceStateRepository<JsonServiceStateStore> service_profile_lease runtime_owner_binding_for_session cleanup_obligation\n');

  assert.equal(report.cuts.serviceModelLeaseAuthority.status, 'fail');
  assert.ok(report.cuts.serviceModelLeaseAuthority.findings.length >= 2);

  write(root, 'cli/src/native/daemon.rs', 'async fn attach_browser_session_state() { host.reconcile_liveness_current(); }\nasync fn reap_browser_sessions_if_loaded() {}\n');
  const mutatingStatus = evaluate(root);
  assert.equal(mutatingStatus.cuts.statusReadOnly.status, 'fail');
  assert.equal(mutatingStatus.cuts.statusReadOnly.findings[0].id, 'status_projection_mutates_browser_session_state');
  write(root, 'cli/src/native/daemon.rs', 'async fn attach_browser_session_state() { serde_json::to_value(host.state()); }\nasync fn reap_browser_sessions_if_loaded() {}\n');
  assert.equal(evaluate(root).cuts.statusReadOnly.status, 'pass');

  write(root, 'cli/src/install.rs', 'pub fn run_install_doctor() { persist_privileged_effect_receipt(); }\nfn print_doctor_field() {}\nfn install_doctor_report() {}\nfn workstation_payload_status() {}\n');
  const mutatingInstallDoctor = evaluate(root);
  assert.equal(mutatingInstallDoctor.cuts.doctorReadOnly.status, 'fail');
  assert.equal(
    mutatingInstallDoctor.cuts.doctorReadOnly.findings[0].id,
    'doctor_privileged_receipt_mutation',
  );
  write(root, 'cli/src/install.rs', 'pub fn run_install_doctor() { install_doctor_report(); }\nfn print_doctor_field() {}\nfn install_doctor_report() { default_operational_status_read_only(); }\nfn workstation_payload_status() {}\n');
  write(root, 'cli/src/remote_view_doctor.rs', 'fn remote_view_doctor_report() { run_json_command("doctor", &["--json"]); }\nstruct RequestedRouteSubject;\n');
  assert.equal(evaluate(root).cuts.doctorReadOnly.status, 'pass');

  write(root, 'crates/agent-browser-service-model/Cargo.toml', '[dependencies]\n');
  write(root, 'crates/agent-browser-service-model/src/service_state.rs', 'pub struct ServiceState;\n');
  const modelCut = evaluate(root);
  assert.equal(modelCut.cuts.serviceModelLeaseAuthority.status, 'pass');
  assert.equal(modelCut.rows.find((row) => row.id === 'P15').status, 'violated');
  assert.deepEqual(
    modelCut.rows.filter((row) => row.status === 'violated').map((row) => row.id),
    ['P02', 'P03', 'P05', 'P09', 'P12', 'P15', 'P16', 'P19'],
  );

  write(root, 'cli/Cargo.toml', '[dependencies]\n');
  const changed = evaluate(root);
  assert.equal(changed.rows.find((row) => row.id === 'P15').status, 'detector_gap');
  assert.equal(changed.rows.find((row) => row.id === 'P15').findings.length, 0);

  write(root, 'Cargo.toml', '[workspace]\nmembers = ["crates/agent-browser-lease-authority"]\n');
  write(root, 'crates/agent-browser-lease-authority/Cargo.toml', '[package]\nname = "agent-browser-lease-authority"\n');
  write(root, 'cli/src/native/remote_view_handoff.rs', 'pub struct RemoteViewHandoff;\n');
  write(root, 'cli/src/workstation_install.rs', 'pub fn install() {}\n');
  write(root, 'cli/src/native/browser_session_store.rs', 'CREATE TABLE IF NOT EXISTS provider_credentials\n');
  write(root, 'scripts/lib/rdp-route-user-pool.py', 'SQLITE_KEY = "rdp_route_user_inventory.v1"\nremove_inventory_secrets(secret_file)\n');
  write(root, 'scripts/setup-rdp-guac-route-pool.sh', '--database\n');
  write(root, 'scripts/sync-rdp-guac-route-specific-user-pool.sh', '--database\n');
  for (const path of [
    'cli/src/native/control_plane.rs',
    'cli/src/native/service_store.rs',
    'cli/src/native/service_health.rs',
    'cli/src/native/service_inventory.rs',
    'cli/src/native/service_lifecycle.rs',
    'cli/src/native/service_failure.rs',
    'cli/src/native/stream/guacamole_primary_binding.rs',
    'crates/agent-browser-service-model/src/browser_process.rs',
    'packages/dashboard/src/components/service-panel.tsx',
    'packages/client/src/service-observability.generated.d.ts',
  ]) write(root, path, 'pub struct CurrentAuthority;\n');
  write(
    root,
    'cli/src/native/service_store.rs',
    'pub struct CurrentAuthority;\n#[cfg(any())]\nmod tests { use crate::runtime_owner_transfer; }\n',
  );
  const quarantineCut = evaluate(root);
  assert.equal(quarantineCut.rows.find((row) => row.id === 'P15').status, 'pass');
  assert.equal(quarantineCut.rows.find((row) => row.id === 'P16').status, 'pass');
  assert.equal(quarantineCut.cuts.legacyAuthorityQuarantine.status, 'pass');
  assert.equal(quarantineCut.rows.find((row) => row.id === 'P09').status, 'pass');
  assert.equal(quarantineCut.cuts.providerCredentialCustody.status, 'pass');

  write(root, 'cli/src/native/actions.rs', 'pub fn current_actions() {}\n');
  write(root, 'packages/dashboard/src/components/workspace-remote-viewport.tsx', '/api/live-viewer-authority operation: "heartbeat" operation: "disconnect"\n');
  write(root, 'cli/src/native/stream/guacamole_live_viewer.rs', 'observe_active_connection "connect" "heartbeat" "disconnect" BrowserRuntimeSqliteStore::default_sqlite\n');
  write(root, 'cli/src/native/browser_session_store/desktop_control.rs', 'LIVE_VIEWER_TTL_MS record.expires_at_ms <= now_ms record.boot_epoch != current_boot with_current_desktop_control validate_live_viewer_current\n');
  write(root, 'crates/agent-browser-service-model/src/browser_session_manager.rs', 'pub struct BrowserSessionManager;\n');
  write(root, 'cli/src/native/presentation_request_admission.rs', 'pub struct PresentationAdmissionRequest;\n');
  const liveViewerCut = evaluate(root);
  assert.equal(liveViewerCut.rows.find((row) => row.id === 'P19').status, 'pass');
  write(root, 'cli/src/native/browser_session_store/desktop_control.rs', 'LIVE_VIEWER_TTL_MS record.expires_at_ms < now_ms validate_live_viewer_current with_current_desktop_control\n');
  assert.equal(evaluate(root).rows.find((row) => row.id === 'P19').status, 'detector_gap');

  const closure = JSON.parse(readFileSync(new URL(
    '../../docs/dev/architecture/p218-ordinary-open-handoff-closure.v1.json',
    import.meta.url,
  )));
  assert.equal(closure.schemaVersion, 'p218-ordinary-open-handoff-closure.v1');
  assert.equal(closure.milestone, 'M0');
  assert.deepEqual(
    closure.requiredM1Cuts.map((cut) => cut.edge),
    [
      'session.host.load->legacy.state',
      'handoff.finalize->legacy.state',
      'legacy.state->lease.authority',
    ],
  );
  const edgeIds = new Set(closure.edges.map((edge) => `${edge.from}->${edge.to}`));
  for (const cut of closure.requiredM1Cuts) assert.ok(edgeIds.has(cut.edge), `missing frozen edge ${cut.edge}`);

  const replacement = JSON.parse(readFileSync(new URL(
    '../../docs/dev/contracts/p218-m1-replacement-interfaces.v1.json',
    import.meta.url,
  )));
  assert.equal(replacement.schemaVersion, 'p218-m1-replacement-interfaces.v1');
  assert.equal(replacement.milestone, 'M1');
  assert.equal(replacement.cut, 'agent-browser-service-model->agent-browser-lease-authority');
  assert.deepEqual(
    replacement.allowedNeutralValueTypes.map((entry) => entry.symbol),
    ['agent_browser_service_model::ServicePrincipalProvenance'],
  );
  assert.equal(replacement.migrationDiagnosticBoundary.mayLinkLeaseAuthority, true);
  assert.equal(replacement.migrationDiagnosticBoundary.mayBeLinkedByDefaultProductPackages, false);
  assert.ok(replacement.firstCutExit.length >= 4);
  console.log('P218 architecture detector self-test passed');
} finally {
  rmSync(root, { recursive: true, force: true });
}
