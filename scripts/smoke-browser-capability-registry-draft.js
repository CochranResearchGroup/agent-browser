#!/usr/bin/env node

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const schemaPath = path.join(
  repoRoot,
  'docs/dev/contracts/service-browser-capability-registry.v1.schema.json',
);
const samplePath = path.join(
  repoRoot,
  'docs/dev/contracts/examples/browser-capability-registry.sample.json',
);

function readJson(filePath) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch (error) {
    throw new Error(`Failed to read JSON ${path.relative(repoRoot, filePath)}: ${error.message}`);
  }
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function idsBy(collection, name) {
  assert(Array.isArray(collection), `${name} must be an array`);
  const ids = new Set();
  for (const record of collection) {
    assert(record && typeof record === 'object', `${name} entries must be objects`);
    assert(typeof record.id === 'string' && record.id.length > 0, `${name} entries need non-empty id`);
    assert(!ids.has(record.id), `${name} contains duplicate id ${record.id}`);
    ids.add(record.id);
  }
  return ids;
}

function assertHas(ids, id, collectionName, fieldName, ownerId) {
  if (id === null) {
    return;
  }
  assert(
    typeof id === 'string' && ids.has(id),
    `${collectionName}.${fieldName} on ${ownerId} references missing id ${id}`,
  );
}

function assertArrayItems(collection, fieldName, ownerName) {
  for (const record of collection) {
    assert(Array.isArray(record[fieldName]), `${ownerName}.${fieldName} on ${record.id} must be an array`);
    for (const value of record[fieldName]) {
      assert(
        typeof value === 'string' && value.length > 0,
        `${ownerName}.${fieldName} on ${record.id} contains a non-string or empty value`,
      );
    }
  }
}

function main() {
  const schema = readJson(schemaPath);
  const sample = readJson(samplePath);
  const requiredCollections = [
    'browserHosts',
    'browserExecutables',
    'browserCapabilities',
    'profileCompatibility',
    'browserPreferenceBindings',
    'validationEvidence',
  ];

  assert(schema.$id && schema.$id.includes('service-browser-capability-registry'), 'schema $id is missing');
  for (const collection of requiredCollections) {
    assert(schema.required.includes(collection), `schema does not require ${collection}`);
    assert(Array.isArray(sample[collection]), `sample is missing ${collection}`);
  }

  const hostIds = idsBy(sample.browserHosts, 'browserHosts');
  const executableIds = idsBy(sample.browserExecutables, 'browserExecutables');
  const capabilityIds = idsBy(sample.browserCapabilities, 'browserCapabilities');
  idsBy(sample.profileCompatibility, 'profileCompatibility');
  idsBy(sample.browserPreferenceBindings, 'browserPreferenceBindings');
  idsBy(sample.validationEvidence, 'validationEvidence');

  for (const executable of sample.browserExecutables) {
    assertHas(hostIds, executable.hostId, 'browserExecutables', 'hostId', executable.id);
  }

  for (const capability of sample.browserCapabilities) {
    assertHas(hostIds, capability.hostId, 'browserCapabilities', 'hostId', capability.id);
    assertHas(executableIds, capability.executableId, 'browserCapabilities', 'executableId', capability.id);
  }

  for (const compatibility of sample.profileCompatibility) {
    assertHas(hostIds, compatibility.hostId, 'profileCompatibility', 'hostId', compatibility.id);
    assertHas(executableIds, compatibility.executableId, 'profileCompatibility', 'executableId', compatibility.id);
    assert(
      typeof compatibility.profileId === 'string' && compatibility.profileId.length > 0,
      `profileCompatibility.profileId on ${compatibility.id} must be non-empty`,
    );
  }

  for (const binding of sample.browserPreferenceBindings) {
    assertArrayItems([binding], 'targetServiceIds', 'browserPreferenceBindings');
    assertArrayItems([binding], 'accountIds', 'browserPreferenceBindings');
    assertArrayItems([binding], 'serviceNames', 'browserPreferenceBindings');
    assertArrayItems([binding], 'taskNames', 'browserPreferenceBindings');
    assertHas(hostIds, binding.preferredHostId, 'browserPreferenceBindings', 'preferredHostId', binding.id);
    assertHas(
      executableIds,
      binding.preferredExecutableId,
      'browserPreferenceBindings',
      'preferredExecutableId',
      binding.id,
    );
    assertHas(
      capabilityIds,
      binding.preferredCapabilityId,
      'browserPreferenceBindings',
      'preferredCapabilityId',
      binding.id,
    );
  }

  for (const evidence of sample.validationEvidence) {
    assertHas(hostIds, evidence.hostId, 'validationEvidence', 'hostId', evidence.id);
    assertHas(executableIds, evidence.executableId, 'validationEvidence', 'executableId', evidence.id);
    assertHas(capabilityIds, evidence.capabilityId, 'validationEvidence', 'capabilityId', evidence.id);
  }

  console.log(
    `Browser capability registry draft sample passed: ${sample.browserHosts.length} hosts, ` +
      `${sample.browserExecutables.length} executables, ${sample.browserCapabilities.length} capabilities, ` +
      `${sample.browserPreferenceBindings.length} bindings`,
  );
}

try {
  main();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
