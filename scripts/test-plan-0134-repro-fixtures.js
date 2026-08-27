#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const lifecycle = readJson(
  'docs/dev/fixtures/profile-lifecycle/plan-0134-red-fixtures.v1.json',
);
const install = readJson(
  'docs/dev/fixtures/profile-lifecycle/plan-0134-install-migration-red-fixtures.v1.json',
);

assert.equal(lifecycle.schemaVersion, 'agent-browser.plan-0134-red-fixtures.v1');
assert.equal(lifecycle.accessPlanCases.length, 5);
assert.deepEqual(
  lifecycle.accessPlanCases.filter((fixture) => fixture.samePrincipal).map((fixture) => fixture.consumerShape),
  [
    'profile-attribution-contradiction',
    'last30days-repeated-task',
    'books-receipts-post-crash-reconnect',
    'odollo-fulfillment-fedex-tracking-lookup',
  ],
);
assert.equal(
  lifecycle.accessPlanCases.find((fixture) => !fixture.samePrincipal)?.requiredFutureAction,
  'wait_for_foreign_principal',
);
assert.deepEqual(lifecycle.publicLeaseContract.baselineFirstClassOperations, []);
assert.deepEqual(lifecycle.publicLeaseContract.requiredReadOperations, [
  'list',
  'inspect',
  'explain',
  'doctor',
  'watch',
]);
assert.deepEqual(lifecycle.publicLeaseContract.requiredOwnerOperations, [
  'rejoin',
  'renew',
  'release',
]);
assert.deepEqual(lifecycle.publicLeaseContract.requiredReconcileOperations, [
  'reconcile_plan',
  'reconcile_apply',
]);

assert.equal(
  install.schemaVersion,
  'agent-browser.plan-0134-install-migration-red-fixtures.v1',
);
assert.equal(install.legacyState.authoritativeInputMutationAllowed, false);
assert.equal(install.dryRun.createsActiveTransaction, false);
assert.equal(install.blockedPreflight.terminalState, 'blocked_preflight');
assert.equal(install.blockedPreflight.effectOccurred, false);
assert.deepEqual(install.exactMutationKeys, [
  'transactionId',
  'expectedTransactionRevision',
  'candidateGenerationId',
  'currentCensusDigest',
]);
assert.equal(install.resume.requiresExactMutationKeys, true);
assert.equal(install.rollback.requiresExactMutationKeys, true);
assert.equal(install.skillStaging.failedCandidatePreservesAcceptedSkill, true);
assert.equal(install.skillStaging.rollbackRestoresAcceptedSkill, true);

const output = readText('cli/src/output.rs');
const client = readText('packages/client/src/service-observability.js');
const contracts = readText('cli/src/native/service_contracts.rs');
const dashboard = readText('packages/dashboard/src/components/service-panel.tsx');
for (const acceptedInstallSurface of [
  'agent-browser install doctor --json',
  'agent-browser install workstation --dry-run --json',
  'agent-browser install transactions list [--json]',
  'agent-browser install transactions inspect --transaction-id <id> [--json]',
  'agent-browser install transactions <resume|rollback|close> --transaction-id <id> --expected-revision <revision> --candidate-generation <generation> --census-digest <sha256|none> [--json]',
]) {
  assert.equal(
    output.includes(acceptedInstallSurface),
    true,
    `accepted install control plane is missing ${acceptedInstallSurface}`,
  );
}
for (const acceptedSurface of [
  'agent-browser service leases',
  'agent-browser service leases doctor',
  'agent-browser service leases watch',
  'agent-browser service leases register',
  'service leases <lease-id> explain',
  'service leases <lease-id> rejoin',
  'service leases <lease-id> renew',
  'service leases <lease-id> release',
  'service leases <lease-id> reconcile plan',
  'service leases <lease-id> reconcile apply',
]) {
  assert.equal(
    output.includes(acceptedSurface),
    true,
    `accepted CLI is missing ${acceptedSurface}`,
  );
}
for (const acceptedHelper of [
  'getServiceProfileLeases',
  'watchServiceProfileLeases',
  'getServiceProfileLease',
  'explainServiceProfileLease',
  'doctorServiceProfileLeases',
  'rejoinServiceProfileLease',
  'renewServiceProfileLease',
  'releaseServiceProfileLease',
  'planServiceProfileLeaseReconciliation',
  'applyServiceProfileLeaseReconciliation',
]) {
  assert.equal(
    client.includes(acceptedHelper),
    true,
    `accepted generated client is missing ${acceptedHelper}`,
  );
}
for (const acceptedContract of [
  'agent-browser://profile-leases',
  'agent-browser://profile-leases/doctor',
  'agent-browser://profile-leases/{lease_id}',
  'agent-browser://profile-leases/{lease_id}/explain',
  '/api/service/profile-leases/<id>/<rejoin|renew|release>',
  '/api/service/profile-leases/<id>/reconcile/plan',
  '/api/service/profile-leases/<id>/reconcile/apply',
]) {
  assert.equal(
    contracts.includes(acceptedContract),
    true,
    `accepted service contracts are missing ${acceptedContract}`,
  );
}
for (const dashboardContract of [
  'onManageProfileLease',
  'profileLeaseActionAllowed',
  'rejoinServiceProfileLease(common)',
  'renewServiceProfileLease({',
  'releaseServiceProfileLease(common)',
  'planServiceProfileLeaseReconciliation({',
  'applyServiceProfileLeaseReconciliation({',
]) {
  assert.equal(
    dashboard.includes(dashboardContract),
    true,
    `accepted dashboard is missing ${dashboardContract}`,
  );
}

process.stdout.write(
  `${JSON.stringify({
    success: true,
    accessPlanFixtureCount: lifecycle.accessPlanCases.length,
    baselineFirstClassLeaseOperationCount:
      lifecycle.publicLeaseContract.baselineFirstClassOperations.length,
    acceptedFirstClassLeaseOperationCount: 10,
    publicLeaseContractAccepted: true,
    installContractBaselineFrozen: true,
    installPublicTransactionSurfaceAccepted: true,
  })}\n`,
);

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function readText(relativePath) {
  return readFileSync(resolve(repoRoot, relativePath), 'utf8');
}
