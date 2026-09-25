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
  write(root, 'cli/src/native/browser_session_host.rs', 'std::env::var("AGENT_BROWSER_SESSION_DISPLAY"); default_service_state_path(); load_or_import_profile_catalog();\n');
  write(root, 'cli/src/native/remote_view_handoff.rs', 'LockedServiceStateRepository<JsonServiceStateStore> service_profile_lease runtime_owner_binding_for_session cleanup_obligation\n');
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
    ['legacy_json_import_in_default_host', 'ordinary_handoff_json_repository', 'ordinary_open_json_repository', 'manual_seeding_json_repository'],
  );
  assert.deepEqual(
    report.rows.find((row) => row.id === 'P05').findings.map((item) => item.id),
    ['competing_handoff_authority', 'ordinary_open_parallel_acquisition', 'manual_seeding_parallel_handoff'],
  );

  write(root, 'cli/src/native/remote_view_handoff.rs', 'pub struct RemoteViewHandoff;\n');
  const finalizerOnlyCut = evaluate(root);
  assert.equal(finalizerOnlyCut.rows.find((row) => row.id === 'P03').status, 'violated');
  assert.equal(finalizerOnlyCut.rows.find((row) => row.id === 'P05').status, 'violated');
  write(root, 'cli/src/native/remote_view_handoff.rs', 'LockedServiceStateRepository<JsonServiceStateStore> service_profile_lease runtime_owner_binding_for_session cleanup_obligation\n');

  assert.equal(report.cuts.serviceModelLeaseAuthority.status, 'fail');
  assert.ok(report.cuts.serviceModelLeaseAuthority.findings.length >= 2);

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
