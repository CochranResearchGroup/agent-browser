#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (relative) => readFileSync(path.join(repoRoot, relative), 'utf8');
const failures = [];

function rejectExisting(relative, finding, reason) {
  if (existsSync(path.join(repoRoot, relative))) {
    failures.push(`${finding}:${relative}:${reason}`);
  }
}

function rejectPattern(relative, pattern, finding, reason) {
  const source = read(relative);
  if (pattern.test(source)) {
    failures.push(`${finding}:${relative}:${reason}`);
  }
}

function requirePattern(relative, pattern, finding, reason) {
  const source = read(relative);
  if (!pattern.test(source)) {
    failures.push(`${finding}:${relative}:${reason}`);
  }
}

function countPattern(relative, pattern) {
  return [...read(relative).matchAll(pattern)].length;
}

function listFiles(relative, suffix) {
  const absolute = path.join(repoRoot, relative);
  return readdirSync(absolute, { withFileTypes: true })
    .flatMap((entry) => {
      const child = path.join(relative, entry.name);
      return entry.isDirectory() ? listFiles(child, suffix) : [child];
    })
    .filter((entry) => entry.endsWith(suffix))
    .sort();
}

function requireExactInventory(actual, declared, finding, label) {
  for (const item of actual) {
    if (!declared.has(item)) {
      failures.push(`${finding}:${item}:unclassified_${label}`);
    }
  }
  for (const item of declared.keys()) {
    if (!actual.includes(item)) {
      failures.push(`${finding}:${item}:stale_${label}_declaration`);
    }
  }
}

function requireExactTestLayout(entries, expectedTotal) {
  let total = 0;
  for (const [relative, expected] of entries) {
    const actual = countPattern(relative, /^#\[(?:tokio::)?test\]/gm);
    if (actual !== expected) {
      failures.push(
        `P0101-W1-04:${relative}:interface_test_count:${actual}:expected:${expected}`,
      );
    }
    total += actual;
  }
  if (total !== expectedTotal) {
    failures.push(
      `P0101-W1-04:interface_test_total:${total}:expected:${expectedTotal}`,
    );
  }
}

const runtimePath = 'cli/src/native/remote_view/open/runtime.rs';
const coordinatorPath = 'cli/src/native/remote_view/open/coordinator.rs';
const compensationPath = 'cli/src/native/remote_view/open/compensation.rs';
const handoffPath = 'cli/src/native/remote_view_handoff.rs';
const serviceStorePath = 'cli/src/native/service_store.rs';
const routeTestsPath = 'cli/src/native/remote_view/open/tests.rs';
const processIdentityPath = 'cli/src/process_identity.rs';

rejectExisting(
  'cli/src/native/action_runtime/common.rs',
  'P0101-W1-04',
  'broad_common_prelude_present',
);
rejectExisting(
  'cli/src/native/action_runtime/tests.rs',
  'P0101-W1-04',
  'central_private_state_test_facade_present',
);
for (const relative of [
  'cli/src/native/page_pdf.rs',
  'cli/src/native/browser_evaluation.rs',
  'cli/src/native/stream_screencast.rs',
  'cli/src/native/service_configuration_inventory.rs',
]) {
  rejectExisting(relative, 'P0101-W1-04', 'shallow_action_owner_present');
}
rejectPattern(
  'cli/src/native/action_runtime.rs',
  /#\[cfg\(test\)\][\s\S]*pub\(crate\) use super::/,
  'P0101-W1-04',
  'test_only_reexport_facade_present',
);

requirePattern(
  processIdentityPath,
  /\bPROCESS_SYNCHRONIZE\b/,
  'P0108-W1-05',
  'windows_process_handle_sync_right_missing',
);

const rustSources = listFiles('cli/src', '.rs');
const declaredProcessIdentityConsumers = new Map([
  ['cli/src/native/action_runtime/runtime/daemon.rs', 'handoff_schema'],
  ['cli/src/native/action_runtime/runtime/launch.rs', 'runtime_termination_and_service_capture'],
  ['cli/src/native/action_runtime/runtime/navigation.rs', 'handoff_assessment'],
  ['cli/src/native/action_runtime/runtime/recovery.rs', 'runtime_recovery'],
  ['cli/src/native/action_runtime/runtime/remote_headed.rs', 'service_capture'],
  ['cli/src/native/action_runtime/runtime/route_host_tests.rs', 'handoff_fixtures'],
  ['cli/src/native/cdp/chrome.rs', 'runtime_state_and_profile_lock'],
  ['cli/src/native/remote_view.rs', 'runtime_remote_view'],
  ['cli/src/native/service_config.rs', 'runtime_service_config'],
  ['cli/src/native/service_health.rs', 'service_browser_assessment'],
  ['cli/src/native/service_health/action_helper_tests.rs', 'service_persistence_fixture'],
  ['cli/src/native/service_model.rs', 'service_identity_schema'],
  ['cli/src/runtime_profile.rs', 'runtime_owner'],
]);
const processIdentityConsumers = rustSources.filter(
  (relative) =>
    relative !== processIdentityPath &&
    /(?:crate::)?process_identity::|runtime_process_assessment\s*\(/.test(read(relative)),
);
requireExactInventory(
  processIdentityConsumers,
  declaredProcessIdentityConsumers,
  'P0108-W1-05',
  'process_identity_consumer',
);

const declaredDirectProcessPaths = new Map([
  ['cli/src/native/e2e_tests.rs', 'windows_e2e_cleanup'],
  ['cli/src/install.rs', 'installer_liveness'],
  ['cli/src/main.rs', 'daemon_process_liveness'],
  ['cli/src/native/cdp/chrome.rs', 'owned_child_and_process_group'],
  ['cli/src/native/service_resources.rs', 'review_token_bound_resource_gc'],
  ['cli/src/native/stream/dashboard.rs', 'dashboard_process_liveness'],
  ['cli/src/native/stream/discovery.rs', 'session_discovery_liveness'],
  ['cli/src/process_identity.rs', 'verified_kernel_handle_termination'],
  ['cli/src/workstation_install.rs', 'workstation_installer_liveness'],
]);
const directProcessPaths = rustSources.filter((relative) =>
  /libc::kill\s*\(|TerminateProcess\s*\(|Command::new\(["']taskkill["']\)/.test(
    read(relative),
  ),
);
requireExactInventory(
  directProcessPaths,
  declaredDirectProcessPaths,
  'P0108-W1-05',
  'direct_process_path',
);
rejectPattern(
  processIdentityPath,
  /\bSYNCHRONIZE\s*[,|]/,
  'P0108-W1-05',
  'windows_wrong_sync_right_constant',
);
rejectPattern(
  processIdentityPath,
  /(?:Command::new\(|command\()["']taskkill["']/i,
  'P0108-W1-05',
  'windows_pid_only_taskkill_present',
);
rejectPattern(
  processIdentityPath,
  /libc::kill\s*\(/,
  'P0108-W1-05',
  'macos_naked_pid_signal_present',
);
requirePattern(
  processIdentityPath,
  /NtQueryInformationProcess[\s\S]*ProcessCommandLineInformation/,
  'P0108-W1-05',
  'windows_metadata_command_line_adapter_missing',
);
requirePattern(
  processIdentityPath,
  /KERN_PROCARGS2/,
  'P0108-W1-05',
  'macos_metadata_command_line_adapter_missing',
);
rejectPattern(
  'cli/src/native/service_health.rs',
  /runtime_process_assessment\s*\(\s*browser\.profile_id/,
  'P0108-W1-05',
  'service_profile_id_reinterpreted_as_runtime_profile',
);
requirePattern(
  'cli/src/native/service_model.rs',
  /browser_process_identities:\s*BTreeMap<String,\s*ServiceBrowserProcessIdentity>/,
  'P0108-W1-05',
  'service_browser_process_identity_persistence_missing',
);
requirePattern(
  'cli/src/native/action_runtime/runtime/daemon.rs',
  /struct RuntimeHandoffDescriptor[\s\S]*process_identity:\s*Option<crate::process_identity::RecordedProcessIdentity>/,
  'P0108-W1-05',
  'runtime_handoff_process_identity_missing',
);

requireExactTestLayout(
  [
    ['cli/src/native/action_runtime/cancellation/tests.rs', 1],
    ['cli/src/native/action_runtime/runtime/close_launch_tests.rs', 6],
    ['cli/src/native/action_runtime/runtime/dispatch_runtime_tests.rs', 19],
    ['cli/src/native/action_runtime/runtime/route_host_tests.rs', 60],
    ['cli/src/native/actions/confirmation_tests.rs', 1],
    ['cli/src/native/actions/dependent_batch_tests.rs', 2],
    ['cli/src/native/actions/dispatch_tests.rs', 9],
    ['cli/src/native/actions/remote_view_route_tests_one.rs', 9],
    ['cli/src/native/actions/remote_view_route_tests_two.rs', 8],
    ['cli/src/native/actions/runtime_route_host_tests.rs', 1],
    ['cli/src/native/actions/service_activity_tests.rs', 4],
    ['cli/src/native/actions/service_config_tests.rs', 3],
    ['cli/src/native/actions/service_health_tests.rs', 7],
    ['cli/src/native/actions/service_incident_mutation_tests.rs', 2],
    ['cli/src/native/actions/service_incidents_tests.rs', 8],
    ['cli/src/native/actions/service_inventory_tests.rs', 17],
    ['cli/src/native/actions/service_jobs_tests.rs', 3],
    ['cli/src/native/actions/service_reconcile_tests.rs', 4],
    ['cli/src/native/actions/service_trace_tests.rs', 3],
    ['cli/src/native/actions/state_tests.rs', 1],
    ['cli/src/native/auth/action_tests.rs', 1],
    ['cli/src/native/browser_input/action_tests.rs', 9],
    ['cli/src/native/browser_inspection/evaluation_action_tests.rs', 1],
    ['cli/src/native/browser_lifecycle/action_tests.rs', 17],
    ['cli/src/native/interaction/action_tests.rs', 6],
    ['cli/src/native/network/action_tests.rs', 5],
    ['cli/src/native/remote_view/helper_action_tests.rs', 3],
    ['cli/src/native/remote_view/open/route_action_helper_tests.rs', 6],
    ['cli/src/native/remote_view/open/visibility_action_tests.rs', 15],
    ['cli/src/native/service_health/action_helper_tests.rs', 14],
    ['cli/src/native/service_health/reconcile_action_helper_tests.rs', 3],
    ['cli/src/native/service_incidents/action_mutation_helper_tests.rs', 1],
    ['cli/src/native/service_retained_state/inventory_action_helper_tests.rs', 10],
    ['cli/src/native/stream_runtime/action_tests.rs', 4],
  ],
  263,
);

rejectPattern(
  runtimePath,
  /ActionsRouteBoundOpenRuntime/,
  'P0101-W1-03',
  'transitional_runtime_adapter_present',
);

for (const outcome of [
  'Planned',
  'NotFound',
  'ExplicitlyClosed',
  'Reopened',
  'Opened',
  'RolledBack',
  'ProviderFallback',
]) {
  requirePattern(
    coordinatorPath,
    new RegExp(`RouteBoundOpenOutcome::${outcome}\\b`),
    'P0101-W1-03',
    `typed_outcome_missing:${outcome}`,
  );
  requirePattern(
    routeTestsPath,
    new RegExp(`RouteBoundOpenOutcome::${outcome}\\b`),
    'P0101-W1-03',
    `coordinator_outcome_test_missing:${outcome}`,
  );
}
for (const predicate of [
  'immutable_snapshot_exists',
  'explicit_close_allows_resolution',
  'exact_opaque_rdp_identity',
  'typed_retained_owner_conflict',
  'current_bounded_route',
  'operator_access_succeeded',
  'best_effort_result',
  'no_new_ownership',
  'retained_browser_and_unrelated_tabs_unchanged',
]) {
  requirePattern(
    coordinatorPath,
    new RegExp(`pub\\(crate\\) ${predicate}: bool`),
    'P0101-W1-03',
    `fallback_predicate_missing:${predicate}`,
  );
}
requirePattern(
  runtimePath,
  /DaemonRouteBoundOpenRuntime/,
  'P0101-W1-03',
  'permanent_runtime_adapter_missing',
);
rejectPattern(
  runtimePath,
  /command:\s*Value|RouteBoundOpenFuture<'_,\s*(?:Option<)?Value/,
  'P0101-W1-03',
  'raw_value_in_route_runtime_seam',
);
rejectPattern(
  'cli/src/native/remote_view/open/runtime_model.rs',
  /struct\s+\w+\s*\(\s*Value\s*\)|impl\s+From<Value>/,
  'P0101-W1-03',
  'opaque_value_wrapper_in_route_model',
);
requirePattern(
  routeTestsPath,
  /concrete_route_records_reject_untyped_payloads/,
  'P0101-W1-03',
  'concrete_route_record_fixture_missing',
);
requirePattern(
  routeTestsPath,
  /provider_fallback_derives_snapshot_ownership_and_operator_predicates/,
  'P0101-W1-03',
  'semantic_fallback_fixture_missing',
);
requirePattern(
  serviceStorePath,
  /enum ServiceStateSaveBoundary\s*\{\s*HandoffWrite,\s*StateWrite,\s*HandoffRename,\s*StateRename,/,
  'P0101-W1-02',
  'four_atomic_store_fault_boundaries_missing',
);
requirePattern(
  serviceStorePath,
  /two_file_service_state_commit_is_atomic_at_every_write_and_rename_boundary/,
  'P0101-W1-02',
  'atomic_store_fault_matrix_missing',
);
rejectPattern(
  coordinatorPath,
  /request:\s*Value|RouteBoundOpenOutcome::\w+\s*\{[^}]*:\s*Value/,
  'P0101-W1-03',
  'raw_value_in_route_invocation_or_outcome',
);
requirePattern(
  routeTestsPath,
  /repository_snapshot_is_dropped_at_the_forward_deadline/,
  'P0101-W1-01',
  'repository_deadline_drop_fixture_missing',
);
requirePattern(
  'cli/src/native/control_plane.rs',
  /coordinated_timeout_drops_unfinished_execution_at_total_deadline/,
  'P0101-W1-01',
  'terminalization_drop_fixture_missing',
);
rejectPattern(
  coordinatorPath,
  /split_once\([^\n]*cleanup|contains\([^\n]*rollback_incomplete|typed_remote_view_handoff_provider_fallback\s*\(/,
  'P0101-W1-03',
  'string_derived_outcome_or_legacy_fallback_present',
);

requirePattern(
  handoffPath,
  /finalize_route_bound_handoff_atomic/,
  'P0101-W1-02',
  'atomic_finalize_and_handoff_operation_missing',
);
rejectPattern(
  handoffPath,
  /complete_route_bound_handoff_plan_acquisition\([\s\S]{0,1800}persist_remote_view_handoff\(/,
  'P0101-W1-02',
  'split_finalize_and_handoff_mutations_present',
);

for (const relative of [coordinatorPath, compensationPath]) {
  rejectPattern(
    relative,
    /repository\.(?:load_snapshot|mutate)\s*\(/,
    'P0101-W1-01',
    'repository_phase_outside_deadline_supervisor',
  );
}
requirePattern(
  runtimePath,
  /trait RouteBoundOpenRepository/,
  'P0101-W1-01',
  'deadline_supervised_repository_seam_missing',
);
rejectPattern(
  'cli/src/native/control_plane.rs',
  /cancellation\.cancel\(\);\s*\n\s*let response = execution\.await/,
  'P0101-W1-01',
  'unbounded_post_deadline_terminalization_present',
);

if (failures.length > 0) {
  console.error('actions remediation architecture gate failed');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log('actions remediation structural regression gate passed');
