#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultRoot = resolve(scriptDirectory, '../..');

function read(root, path) {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function filesUnder(root, path, suffixes) {
  const base = join(root, path);
  const files = [];
  const visit = (directory) => {
    if (!existsSync(directory)) return;
    for (const entry of readdirSync(directory)) {
      const candidate = join(directory, entry);
      const info = statSync(candidate);
      if (info.isDirectory()) visit(candidate);
      else if (suffixes.some((suffix) => candidate.endsWith(suffix))) files.push(candidate);
    }
  };
  visit(base);
  return files.sort();
}

function finding(id, path, detail) {
  return { id, path, detail };
}

function manifestDependsOnLeaseAuthority(source) {
  return /^\s*agent-browser-lease-authority\s*=|^\s*[A-Za-z0-9_-]+\s*=\s*\{[^}\n]*\bpackage\s*=\s*["']agent-browser-lease-authority["']/m.test(source);
}

export function evaluate(root = defaultRoot) {
  const cliManifest = read(root, 'cli/Cargo.toml');
  const serviceModelManifest = read(root, 'crates/agent-browser-service-model/Cargo.toml');
  const host = read(root, 'cli/src/native/browser_session_host.rs');
  const handoff = read(root, 'cli/src/native/remote_view_handoff.rs');
  const openRuntime = read(root, 'cli/src/native/remote_view/open/runtime.rs');
  const openCoordinator = read(root, 'cli/src/native/remote_view/open/coordinator.rs');
  const serviceState = read(root, 'crates/agent-browser-service-model/src/service_state.rs');
  const actions = read(root, 'cli/src/native/actions.rs');
  const dashboardViewport = read(root, 'packages/dashboard/src/components/workspace-remote-viewport.tsx');
  const workstationInstall = read(root, 'cli/src/workstation_install.rs');
  const productFiles = [
    ...filesUnder(root, 'cli/src', ['.rs']),
    ...filesUnder(root, 'crates/agent-browser-service-model/src', ['.rs']),
    ...filesUnder(root, 'packages/dashboard/src', ['.js', '.jsx', '.ts', '.tsx']),
    ...filesUnder(root, 'packages/client/src', ['.js', '.ts']),
  ];
  const serviceModelFiles = [
    ...filesUnder(root, 'crates/agent-browser-service-model/src', ['.rs']),
    ...filesUnder(root, 'crates/agent-browser-service-model/tests', ['.rs']),
  ];
  const productSources = productFiles.map((path) => ({
    path: relative(root, path),
    source: readFileSync(path, 'utf8'),
  }));

  const violations = new Map();
  const add = (prohibitionId, evidence) => {
    if (!violations.has(prohibitionId)) violations.set(prohibitionId, []);
    violations.get(prohibitionId).push(evidence);
  };

  if (/AGENT_BROWSER_SESSION_DISPLAY/.test(host)) {
    add('P02', finding('browser_session_display_environment', 'cli/src/native/browser_session_host.rs', 'default Browser Session Manager launch reads a display selection from the environment'));
  }
  if (/default_service_state_path\(\)/.test(host) && /load_or_import_profile_catalog/.test(host)) {
    add('P03', finding('legacy_json_import_in_default_host', 'cli/src/native/browser_session_host.rs', 'the default host supplies legacy Service State to ordinary host loading'));
  }
  if (/LockedServiceStateRepository<JsonServiceStateStore>/.test(handoff)) {
    add('P03', finding('ordinary_handoff_json_repository', 'cli/src/native/remote_view_handoff.rs', 'ordinary handoff finalization still requires the JSON Service State repository'));
    add('P05', finding('competing_handoff_authority', 'cli/src/native/remote_view_handoff.rs', 'ordinary handoff identity is committed outside the Browser Runtime SQLite authority'));
  }
  if (/LockedServiceStateRepository<JsonServiceStateStore>/.test(openRuntime)
    || /LockedServiceStateRepository::default_json\(\)/.test(openRuntime)) {
    add('P03', finding('ordinary_open_json_repository', 'cli/src/native/remote_view/open/runtime.rs', 'ordinary remote-view open still loads and mutates JSON Service State'));
  }
  if (/begin_route_bound_handoff_plan_acquisition/.test(openCoordinator)
    || /complete_route_bound_handoff_open/.test(openCoordinator)) {
    add('P05', finding('ordinary_open_parallel_acquisition', 'cli/src/native/remote_view/open/coordinator.rs', 'ordinary remote-view open still reserves or finalizes a handoff outside the SQLite session operation'));
  }
  const credentialVariables = workstationInstall.match(/XRDP_AGENT_BROWSER_ROUTE_[A-Z]_(?:USERNAME|PASSWORD)/g) || [];
  if (credentialVariables.length > 0) {
    add('P09', finding('persistent_route_credential_environment', 'cli/src/workstation_install.rs', `${new Set(credentialVariables).size} route credential environment keys remain in installation projection`));
  }
  if (/viewer_leases/.test(serviceState) || /viewer_leases/.test(handoff)) {
    add('P12', finding('stored_viewer_authority', 'crates/agent-browser-service-model/src/service_state.rs', 'stored viewer lease records remain part of Service State authority'));
  }
  if (/service_viewer_lease_(?:request|heartbeat|release)/.test(actions)) {
    add('P12', finding('viewer_lease_dispatch', 'cli/src/native/actions.rs', 'the default runtime dispatches stored viewer lease lifecycle actions'));
  }

  if (manifestDependsOnLeaseAuthority(cliManifest)) {
    add('P15', finding('cli_cargo_dependency', 'cli/Cargo.toml', 'the default CLI directly depends on agent-browser-lease-authority'));
  }
  if (manifestDependsOnLeaseAuthority(serviceModelManifest)) {
    add('P15', finding('service_model_cargo_dependency', 'crates/agent-browser-service-model/Cargo.toml', 'the Service model directly depends on agent-browser-lease-authority'));
  }
  for (const { path, source } of productSources) {
    if (/agent_browser_lease_authority/.test(source)) {
      add('P15', finding('compiled_product_reference', path, 'default product source references the Lease Authority crate'));
    }
  }
  if (/service_profile_lease|runtime_owner_binding_for_session|cleanup_obligation/.test(handoff)) {
    add('P16', finding('ordinary_handoff_legacy_admission_record', 'cli/src/native/remote_view_handoff.rs', 'ordinary handoff code consults lease, runtime-owner, or cleanup-obligation state'));
  }
  if (/runtime_owner_registry/.test(serviceState)) {
    add('P16', finding('runtime_owner_in_ordinary_state', 'crates/agent-browser-service-model/src/service_state.rs', 'runtime-owner state remains embedded in the ordinary Service State model'));
  }
  if (/service_viewer_lease_heartbeat/.test(actions)) {
    add('P19', finding('persisted_viewer_heartbeat_authority', 'cli/src/native/actions.rs', 'viewer heartbeat is represented by the persisted viewer-lease action surface'));
    if (/service_viewer_lease_(?:request|release)/.test(dashboardViewport)) {
      add('P19', finding('dashboard_persisted_viewer_lifecycle', 'packages/dashboard/src/components/workspace-remote-viewport.tsx', 'dashboard viewer presence is projected through the persisted viewer-lease lifecycle'));
    }
  }

  const definitelyViolated = new Set(['P02', 'P03', 'P05', 'P09', 'P12', 'P15', 'P16', 'P19']);
  const rows = Array.from({ length: 19 }, (_, index) => {
    const id = `P${String(index + 1).padStart(2, '0')}`;
    const evidence = violations.get(id) || [];
    let status = 'unverified';
    if (definitelyViolated.has(id)) status = evidence.length > 0 ? 'violated' : 'detector_gap';
    return { id, status, findings: evidence };
  });
  const serviceModelLeaseAuthorityFindings = [];
  if (manifestDependsOnLeaseAuthority(serviceModelManifest)) {
    serviceModelLeaseAuthorityFindings.push(finding(
      'service_model_cargo_dependency',
      'crates/agent-browser-service-model/Cargo.toml',
      'the Service model manifest depends on agent-browser-lease-authority',
    ));
  }
  for (const path of serviceModelFiles) {
    const source = readFileSync(path, 'utf8');
    if (/agent_browser_lease_authority/.test(source)) {
      serviceModelLeaseAuthorityFindings.push(finding(
        'service_model_compiled_product_reference',
        relative(root, path),
        'Service model Rust source references the Lease Authority crate',
      ));
    }
  }
  return {
    schemaVersion: 'p218-architecture-conformance.v1',
    plan: 'docs/dev/plans/0218-2026-09-23-grilling-contract-remote-view-conformance.md',
    milestone: 'M0',
    rows,
    cuts: {
      serviceModelLeaseAuthority: {
        status: serviceModelLeaseAuthorityFindings.length === 0 ? 'pass' : 'fail',
        findings: serviceModelLeaseAuthorityFindings,
      },
    },
    summary: rows.reduce((counts, row) => {
      counts[row.status] = (counts[row.status] || 0) + 1;
      return counts;
    }, {}),
  };
}

function printHuman(report) {
  for (const row of report.rows) {
    console.log(`${row.id} ${row.status} findings=${row.findings.length}`);
    for (const item of row.findings.slice(0, 5)) console.log(`  ${item.id} ${item.path}: ${item.detail}`);
    if (row.findings.length > 5) console.log(`  ... ${row.findings.length - 5} more`);
  }
  console.log(`summary ${JSON.stringify(report.summary)}`);
}

function parseArguments(argv) {
  const options = { root: defaultRoot, format: 'human', enforce: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--root') {
      const value = argv[index + 1];
      if (!value) throw new Error('missing value for --root');
      options.root = resolve(value);
      index += 1;
    } else if (argument === '--json') options.format = 'json';
    else if (argument === '--enforce') options.enforce = true;
    else if (argument === '--help') options.help = true;
    else throw new Error(`unknown argument: ${argument}`);
  }
  return options;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArguments(process.argv.slice(2));
    if (options.help) {
      console.log('Usage: node scripts/dev/check-p218-architecture.mjs [--root PATH] [--json] [--enforce]');
      console.log('Without --enforce the command reports the intentionally red M0 baseline and exits zero.');
      process.exit(0);
    }
    const report = evaluate(options.root);
    if (options.format === 'json') console.log(JSON.stringify(report, null, 2));
    else printHuman(report);
    const red = report.rows.filter((row) => row.status === 'violated' || row.status === 'detector_gap');
    if (options.enforce && red.length > 0) process.exit(1);
  } catch (error) {
    console.error(`p218_architecture_check_failed:${error.message}`);
    process.exit(2);
  }
}
