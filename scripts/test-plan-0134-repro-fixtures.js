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
assert.deepEqual(lifecycle.publicLeaseContract.currentFirstClassOperations, []);
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
for (const absentSurface of [
  'service leases list',
  'service leases inspect',
  'service leases rejoin',
  'service leases renew',
  'service leases reconcile',
]) {
  assert.equal(
    output.includes(absentSurface),
    false,
    `current CLI unexpectedly exposes ${absentSurface}; advance the frozen contract fixture with the implementation`,
  );
}
for (const absentHelper of [
  'listServiceProfileLeases',
  'inspectServiceProfileLease',
  'rejoinServiceProfileLease',
  'renewServiceProfileLease',
  'reconcileServiceProfileLease',
]) {
  assert.equal(
    client.includes(absentHelper),
    false,
    `current generated client unexpectedly exposes ${absentHelper}; advance the frozen contract fixture with the implementation`,
  );
}
assert.equal(
  contracts.includes('agent-browser://profile-leases'),
  false,
  'current MCP contracts unexpectedly expose the first-class profile lease collection',
);

process.stdout.write(
  `${JSON.stringify({
    success: true,
    accessPlanFixtureCount: lifecycle.accessPlanCases.length,
    currentFirstClassLeaseOperationCount:
      lifecycle.publicLeaseContract.currentFirstClassOperations.length,
    installContractFrozen: true,
  })}\n`,
);

function readJson(relativePath) {
  return JSON.parse(readText(relativePath));
}

function readText(relativePath) {
  return readFileSync(resolve(repoRoot, relativePath), 'utf8');
}
