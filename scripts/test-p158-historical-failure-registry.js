#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const registryPath = join(root, 'docs/dev/contracts/p158-historical-failure-registry.v1.json');

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const registry = JSON.parse(readFileSync(registryPath, 'utf8'));
assert(registry.schemaVersion === 'agent-browser.p158-historical-failure-registry.v1', 'P158 registry version drifted');
assert(registry.registryState === 'frozen', 'P158 registry is not frozen');
assert(Object.keys(registry.environments).join(',') === 'E0,E1,E2,E3', 'P158 environment set drifted');
assert(registry.resultStates.length === 7, 'P158 terminal result set drifted');
assert(registry.controllerStates.join('>') === 'prepared>frozen>executing>execution_terminal>evidence_sealed>analyzed', 'P158 monotonic controller states drifted');
assert(Object.values(registry.freezeRules).every((value) => typeof value === 'boolean'), 'P158 freeze rules must be boolean');
assert(!registry.freezeRules.opportunisticRetryAllowed, 'P158 cannot permit opportunistic retry');
assert(!registry.freezeRules.reactionaryRepairAllowed, 'P158 cannot permit reactionary repair');
assert(!registry.freezeRules.reactionaryCleanupAllowed, 'P158 cannot permit reactionary cleanup');
assert(registry.candidateManifestRequiredFields.length >= 12, 'P158 candidate identity is incomplete');
assert(registry.resourceCeilings.artifactQuotaBytes > 0, 'P158 artifact quota is not numeric');
assert(registry.resourceCeilings.filesystemMaximumUsedPercent === 85, 'P158 filesystem safety ceiling drifted');
assert(registry.performanceCeilings.internalHandoffUrlLeaks === 0, 'P158 must tolerate zero internal URL leaks');
assert(registry.performanceCeilings.missingTerminalOutcomes === 0, 'P158 must tolerate zero missing terminal outcomes');

const sourceIds = new Set();
for (const source of registry.sources) {
  assert(!sourceIds.has(source.id), `Duplicate P158 source ${source.id}`);
  sourceIds.add(source.id);
  if (source.committable !== false) assert(existsSync(join(root, source.path)), `Missing P158 source ${source.path}`);
}

assert(registry.families.length === 11, 'P158 historical family count drifted');
const familyIds = new Set(registry.families.map((family) => family.id));
assert(familyIds.size === registry.families.length, 'P158 historical family IDs are not unique');

const expectedCases = [
  ...Array.from({ length: 15 }, (_, index) => `A${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 12 }, (_, index) => `H${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 10 }, (_, index) => `X${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 12 }, (_, index) => `D${String(index + 1).padStart(2, '0')}`),
  ...Array.from({ length: 5 }, (_, index) => `C${String(index + 1).padStart(2, '0')}`),
];
const caseIds = new Set(registry.cases.map((testCase) => testCase.id));
assert(registry.cases.length === 54, 'P158 must freeze 49 scenarios and 5 combined phases');
assert(expectedCases.every((id) => caseIds.has(id)), 'P158 scenario arsenal is incomplete');

for (const testCase of registry.cases) {
  assert(testCase.environmentIds.length > 0, `${testCase.id} has no environment`);
  assert(testCase.familyIds.length > 0, `${testCase.id} has no historical family`);
  assert(testCase.sourceIds.length > 0, `${testCase.id} has no evidence source`);
  assert(testCase.executionBound?.length > 20, `${testCase.id} has no deterministic execution bound`);
  assert(Object.hasOwn(registry.evidenceProfiles, testCase.evidenceProfile), `${testCase.id} has no evidence profile`);
  assert(testCase.environmentIds.every((id) => Object.hasOwn(registry.environments, id)), `${testCase.id} cites an unknown environment`);
  assert(testCase.familyIds.every((id) => familyIds.has(id)), `${testCase.id} cites an unknown family`);
  assert(testCase.sourceIds.every((id) => sourceIds.has(id)), `${testCase.id} cites an unknown source`);
  assert(testCase.dependsOn.every((id) => caseIds.has(id) && id !== testCase.id), `${testCase.id} has an invalid dependency`);
}

for (const family of registry.families) {
  assert(family.caseIds.length > 0, `${family.id} has no mapped cases`);
  assert(family.sourceIds.length > 0, `${family.id} has no mapped sources`);
  assert(family.caseIds.every((id) => caseIds.has(id)), `${family.id} cites an unknown case`);
  assert(family.sourceIds.every((id) => sourceIds.has(id)), `${family.id} cites an unknown source`);
  for (const caseId of family.caseIds) {
    const testCase = registry.cases.find((candidate) => candidate.id === caseId);
    assert(testCase.familyIds.includes(family.id), `${family.id} and ${caseId} mapping is not bidirectional`);
  }
}

const fixtureIds = new Set();
for (const fixture of registry.productionSample.signatures) {
  assert(!fixtureIds.has(fixture.id), `Duplicate production fixture ${fixture.id}`);
  fixtureIds.add(fixture.id);
  assert(fixture.count > 0, `${fixture.id} has no observed count`);
  assert(fixture.caseIds.every((id) => caseIds.has(id)), `${fixture.id} cites an unknown case`);
}
assert(fixtureIds.size === 7, 'P158 production fixture count drifted');
assert(registry.productionSample.failedJobs + registry.productionSample.timedOutJobs === 39, 'P158 terminal production count drifted');
assert(registry.productionSample.failedOrTimedOutWithNullTopLevelFailure === 39, 'P158 null failure count drifted');
assert(registry.productionSample.failedOrTimedOutWithNullTopLevelProvenance === 39, 'P158 null provenance count drifted');

assert(registry.fixtureContract.preserve.length >= 15, 'P158 fixture relationships are incomplete');
assert(registry.fixtureContract.substitute.length >= 7, 'P158 fixture redaction substitutions are incomplete');
assert(registry.forbiddenCapturedFields.length >= 9, 'P158 forbidden capture set is incomplete');
assert(registry.fixtureArtifacts.length === 1, 'P158 fixture artifact set drifted');
const fixtureArtifact = JSON.parse(readFileSync(join(root, registry.fixtureArtifacts[0]), 'utf8'));
assert(fixtureArtifact.schemaVersion === 'agent-browser.p158-historical-failure-seeds.v1', 'P158 fixture schema drifted');
assert(fixtureArtifact.redactionState === 'synthetic_relationship_preserving', 'P158 fixtures are not redacted');
assert(fixtureArtifact.fixtures.length === 8, 'P158 must retain seven signatures plus the null terminal envelope');
for (const fixture of fixtureArtifact.fixtures) {
  assert(fixture.fixtureId && fixture.historicalSignature, 'P158 fixture identity is incomplete');
  assert(fixture.observedCount > 0, `${fixture.fixtureId} has no historical count`);
  assert(fixture.caseIds.every((id) => caseIds.has(id)), `${fixture.fixtureId} cites an unknown case`);
  assert(fixture.syntheticIdentity && fixture.syntheticOutcome, `${fixture.fixtureId} lost relationship shape`);
  const serialized = JSON.stringify(fixture);
  for (const forbidden of ['http://', 'https://', '/home/', 'cookie', 'token', 'credential']) {
    assert(!serialized.toLowerCase().includes(forbidden), `${fixture.fixtureId} contains forbidden source-like material: ${forbidden}`);
  }
}
assert(registry.knownHarnessConstraints.length === 3, 'P158 historical harness constraint set drifted');
assert(JSON.stringify(registry.workUnitGraph.W10) === JSON.stringify(['W9']), 'P158 analysis must remain the final work unit');

console.log(`P158 historical failure registry: ${registry.families.length} families, ${registry.cases.length} cases, ${fixtureIds.size} production-shaped fixture signatures`);
