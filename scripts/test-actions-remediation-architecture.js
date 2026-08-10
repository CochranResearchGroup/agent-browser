#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
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

const runtimePath = 'cli/src/native/remote_view/open/runtime.rs';
const coordinatorPath = 'cli/src/native/remote_view/open/coordinator.rs';
const compensationPath = 'cli/src/native/remote_view/open/compensation.rs';
const handoffPath = 'cli/src/native/remote_view_handoff.rs';

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

rejectPattern(
  runtimePath,
  /ActionsRouteBoundOpenRuntime/,
  'P0101-W1-03',
  'transitional_runtime_adapter_present',
);
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
  coordinatorPath,
  /request:\s*Value|RouteBoundOpenOutcome::\w+\s*\{[^}]*:\s*Value/,
  'P0101-W1-03',
  'raw_value_in_route_invocation_or_outcome',
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

console.log('actions remediation architecture gate passed');
