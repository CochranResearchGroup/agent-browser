#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const contractsDir = join(root, 'docs/dev/contracts');
const oraclePath = join(contractsDir, 'p157-regression-oracle.v1.json');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function valueAtPath(value, path) {
  return path.split('.').reduce(
    (current, segment) => current && Object.hasOwn(current, segment) ? current[segment] : undefined,
    value,
  );
}

const oracle = readJson(oraclePath);
assert(
  oracle.schemaVersion === 'agent-browser.p157-regression-oracle.v1',
  'P157 oracle schema version drifted',
);
assert(oracle.contractSchemas.length === 6, 'P157 must freeze exactly six W2 contract schemas');

for (const filename of oracle.contractSchemas) {
  const path = join(contractsDir, filename);
  assert(existsSync(path), `P157 contract schema is missing: ${filename}`);
  const schema = readJson(path);
  assert(schema.$schema === 'https://json-schema.org/draft/2020-12/schema', `${filename} draft drifted`);
  assert(schema.$id?.endsWith(`/${filename}`), `${filename} id drifted`);
  assert(Array.isArray(schema.required) && schema.required.length > 0, `${filename} required fields missing`);
}

const accessPolicy = readJson(join(contractsDir, 'service-profile-access-policy.v1.schema.json'));
assert(accessPolicy.properties.mode.default === 'shared-local', 'shared-local must remain the default mode');
assert(
  JSON.stringify(accessPolicy.properties.mode.enum) === JSON.stringify(['shared-local', 'restricted', 'exclusive']),
  'profile access modes drifted',
);
const accessDecision = readJson(join(contractsDir, 'service-profile-access-decision.v1.schema.json'));
assert(
  ['subject', 'resource', 'operation', 'policyRevision', 'allowed', 'missingPermission', 'blockingOccupancy', 'nextAction']
    .every((field) => accessDecision.required.includes(field)),
  'profile access denial evidence or executable recourse drifted',
);

const provenance = readJson(join(contractsDir, 'service-request-provenance.v1.schema.json'));
const provenanceFields = [
  'schemaVersion', 'requestId', 'jobId', 'traceId', 'causedByRequestId', 'clientSubjectId',
  'identityAssurance', 'connectionInstanceId', 'runtimeEnvironmentId', 'runtimeLaneId',
  'profileId', 'profileResourceKey', 'browserId', 'sessionId', 'tabId', 'serviceName',
  'agentName', 'taskName', 'action', 'policyRevision', 'accessDecisionId',
];
assert(
  JSON.stringify(provenance.required) === JSON.stringify(provenanceFields),
  'request provenance field order or membership drifted',
);
assert(provenance.additionalProperties === false, 'request provenance must reject private payload expansion');
for (const forbidden of ['url', 'profilePath', 'userDataDir', 'capability', 'credential', 'pageContent']) {
  assert(!Object.hasOwn(provenance.properties, forbidden), `request provenance exposes ${forbidden}`);
}

const terminal = readJson(join(contractsDir, 'service-terminal-outcome.v1.schema.json'));
assert(terminal.required.includes('provenance') && terminal.required.includes('failure'), 'terminal outcome lost causal fields');
const migration = readJson(join(contractsDir, 'service-profile-policy-migration.v1.schema.json'));
assert(JSON.stringify(migration.required).includes('blockingIssueCount'), 'migration blocker count drifted');
const dashboardHealth = readJson(join(contractsDir, 'service-dashboard-health.v1.schema.json'));
assert(
  JSON.stringify(dashboardHealth.required) === JSON.stringify(['schemaVersion', 'runtime', 'convergence', 'access', 'acquisition']),
  'dashboard health axes drifted',
);

const controlPlaneSource = readFileSync(join(root, 'cli/src/native/control_plane.rs'), 'utf8');
const daemonSource = readFileSync(join(root, 'cli/src/native/daemon.rs'), 'utf8');
const controlRequestSource = controlPlaneSource.slice(
  controlPlaneSource.indexOf('pub struct ControlRequest'),
  controlPlaneSource.indexOf('enum WorkerMessage'),
);
assert(controlRequestSource.includes('provenance'), 'ControlRequest lost immutable provenance');
assert(
  controlPlaneSource.includes('runtime_lane_id: String') &&
    controlPlaneSource.includes('ServiceRequestProvenance::capture') &&
    controlPlaneSource.includes('provenance: request.provenance.clone()') &&
    daemonSource.includes('submit_from_connection(cmd, &connection_instance_id)'),
  'runtime-lane provenance capture or job persistence drifted',
);
const schedulerRejectStart = controlPlaneSource.indexOf('SchedulerLeaseDecision::Reject(error) =>');
const schedulerRejectEnd = controlPlaneSource.indexOf('SchedulerLeaseDecision::Wait {', schedulerRejectStart);
const schedulerRejectSource = controlPlaneSource.slice(schedulerRejectStart, schedulerRejectEnd);
assert(
  schedulerRejectSource.includes('finalize_service_request') &&
    schedulerRejectSource.includes('ServiceTerminalState::Rejected') &&
    schedulerRejectSource.includes('ServiceTerminalPhase::SchedulerAdmission') &&
    schedulerRejectSource.includes('response_tx.send') &&
    !schedulerRejectSource.includes('persist_service_job_failed_to_enqueue'),
  'scheduler rejection bypasses the unified terminal outcome path',
);

const failureSource = readFileSync(join(root, 'cli/src/native/service_failure.rs'), 'utf8');
const identityFailureStart = failureSource.indexOf('"existing_session_profile_identity_unproven"');
const identityFailureEnd = failureSource.indexOf('ServiceFailureRecourse::default()', identityFailureStart);
assert(
  failureSource.slice(identityFailureStart, identityFailureEnd).includes(
    'recommended_action: "inspect_profile_recovery_plan"',
  ) &&
    failureSource.slice(identityFailureStart, identityFailureEnd).includes(
      'missing_permission: Some("lifecycle_manage"',
    ) &&
    !failureSource.slice(identityFailureStart, identityFailureEnd).includes(
      'recommended_action: "acquire_profile"',
    ),
  'identity denial recourse is circular or lacks lifecycle-specific authority',
);

const acquisitionSource = readFileSync(join(root, 'cli/src/native/service_profile_acquisition.rs'), 'utf8');
const serviceAccessSource = readFileSync(join(root, 'cli/src/native/service_access.rs'), 'utf8');
const routeHostSource = readFileSync(join(root, 'cli/src/native/action_runtime/runtime/daemon.rs'), 'utf8');
assert(
  acquisitionSource.includes('evaluate_profile_access') &&
    acquisitionSource.includes('strict_identity_required') &&
    acquisitionSource.includes('ProfileAccessMode::SharedLocal'),
  'profile acquisition does not gate strict identity checks on the selected access policy',
);
assert(
  controlPlaneSource.includes('stable_self_declared_subject') &&
    serviceAccessSource.includes('managed_ephemeral_profile_planned') &&
    acquisitionSource.includes('managed_ephemeral_profile_selected') &&
    routeHostSource.includes('apply_shared_local_session_profile_continuity'),
  'self-declared ephemeral profile launch or shared-local continuity drifted',
);

const dashboardSource = readFileSync(join(root, 'packages/dashboard/src/app/page.tsx'), 'utf8');
assert(
  dashboardSource.includes('workstationConvergenceIssue') &&
    dashboardSource.includes('health.dashboardHealth?.acquisition') &&
    dashboardSource.includes('axes.runtime.findings') &&
    dashboardSource.includes('axes.convergence.findings') &&
    !dashboardSource.includes('health.ready === false') &&
    !dashboardSource.includes('health.issues?.[0]?.message'),
  'dashboard warning composition does not consume independent typed health axes',
);

const expectedCases = new Set([
  'scheduler-rejection-has-one-terminal-outcome',
  'runtime-lane-survives-ingress-routing',
  'identity-denial-recourse-is-not-circular',
  'shared-local-does-not-require-cryptographic-enrollment',
  'access-ambiguity-cannot-block-runtime-health',
  'self-declared-ephemeral-client-has-executable-continuity',
]);
assert(oracle.cases.length === expectedCases.size, 'P157 regression case count drifted');

for (const regression of oracle.cases) {
  assert(expectedCases.delete(regression.id), `Unexpected or duplicate P157 case: ${regression.id}`);
  assert(['red', 'green'].includes(regression.currentStatus), `${regression.id} has an invalid status`);
  assert(/^W(?:3|4|5|9|11)$/.test(regression.implementationWorkUnit), `${regression.id} has no bounded owner`);
  assert(regression.risk?.length > 20, `${regression.id} has no named failure risk`);
  assert(regression.requiredPaths?.length > 0, `${regression.id} has no target invariant`);
  for (const requiredPath of regression.requiredPaths) {
    const currentValue = valueAtPath(regression.currentProjection, requiredPath);
    const targetValue = regression.targetValues?.[requiredPath];
    if (regression.currentStatus === 'green') {
      assert(currentValue !== undefined, `${regression.id} is green but lacks ${requiredPath}`);
      if (targetValue !== undefined) {
        assert(currentValue === targetValue, `${regression.id} is green but violates ${requiredPath}`);
      }
    } else {
      assert(
        targetValue === undefined ? currentValue === undefined : currentValue !== targetValue,
        `${regression.id} is marked red but already satisfies ${requiredPath}`,
      );
    }
  }
}
assert(expectedCases.size === 0, `P157 regression cases missing: ${[...expectedCases].join(', ')}`);

const greenCases = oracle.cases.filter((regression) => regression.currentStatus === 'green').length;
console.log(
  `P157 contract oracle passed: 6 frozen schemas, ${greenCases} green cases, and ${oracle.cases.length - greenCases} reproducible red cases`,
);
