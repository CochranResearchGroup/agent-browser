#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'));

const schemas = {
  request: readJson('docs/dev/contracts/captcha-guard-request.v1.schema.json'),
  capability: readJson('docs/dev/contracts/captcha-guard-capability.v1.schema.json'),
  receipt: readJson('docs/dev/contracts/captcha-guard-receipt.v1.schema.json'),
};
const fixtures = readJson(
  'docs/dev/contracts/examples/captcha-guard-contract-fixtures.v1.json',
);
const dependencyPolicy = readJson(
  'docs/dev/contracts/captcha-guard-dependency-policy.v1.json',
);
const contract = readFileSync('docs/dev/contracts/captcha-guard.v1.md', 'utf8');

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);
const validators = Object.fromEntries(
  Object.entries(schemas).map(([name, schema]) => [name, ajv.compile(schema)]),
);

for (const fixture of fixtures.valid) {
  const validate = validators[fixture.schema];
  assert(validate, `unknown fixture schema: ${fixture.schema}`);
  assert(
    validate(fixture.document),
    `${fixture.name}: ${ajv.errorsText(validate.errors)}`,
  );
}

for (const fixture of fixtures.invalid) {
  const validate = validators[fixture.schema];
  assert(validate, `unknown fixture schema: ${fixture.schema}`);
  assert(!validate(fixture.document), `${fixture.name}: invalid fixture passed`);
}

const forbiddenDurableKeys = new Set([
  'coordinates',
  'executablePath',
  'ipAddress',
  'ocrText',
  'pageText',
  'password',
  'pid',
  'pixels',
  'providerStderr',
  'proxy',
  'shellCommand',
  'token',
]);

const assertRedacted = (value, path = '$') => {
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertRedacted(item, `${path}[${index}]`));
    return;
  }
  if (value === null || typeof value !== 'object') return;
  for (const [key, nested] of Object.entries(value)) {
    assert(!forbiddenDurableKeys.has(key), `${path} contains forbidden key ${key}`);
    assertRedacted(nested, `${path}.${key}`);
  }
};

for (const fixture of fixtures.valid) {
  assertRedacted(fixture.document, `fixture:${fixture.name}`);
}

assert.equal(dependencyPolicy.schemaVersion, 'v1');
assert.equal(dependencyPolicy.layers.length, 3);
const layerById = new Map(dependencyPolicy.layers.map((layer) => [layer.id, layer]));
const guard = layerById.get('captcha_guard');
const desktop = layerById.get('desktop_services');
const adapter = layerById.get('agent_browser_adapter');
assert(guard);
assert(desktop);
assert(adapter);
assert.deepEqual(guard.mayDependOnWorkspaceCrates, ['agent-browser-desktop-services']);
assert(guard.mustNotDependOnWorkspaceCrates.includes('agent-browser'));
for (const responsibility of [
  'credential_access',
  'filesystem_io',
  'network_io',
  'ocr_execution',
  'operating_system_input',
  'process_lookup',
  'runtime_supervision',
  'service_state_persistence',
]) {
  assert(
    guard.forbiddenResponsibilities.includes(responsibility),
    `CAPTCHA guard must forbid ${responsibility}`,
  );
}
assert(desktop.mustNotDependOnWorkspaceCrates.includes('agent-browser'));
assert(desktop.mustNotDependOnWorkspaceCrates.includes('agent-browser-captcha-guard'));
assert(adapter.mayDependOnWorkspaceCrates.includes('agent-browser-captcha-guard'));
assert(adapter.mayDependOnWorkspaceCrates.includes('agent-browser-desktop-services'));
assert.equal(
  dependencyPolicy.enforcement.currentDisposition,
  'contract_frozen_enforcement_pending',
);

for (const path of [
  'crates/agent-browser-captcha-guard/Cargo.toml',
  'crates/agent-browser-desktop-services/Cargo.toml',
]) {
  if (!existsSync(path)) continue;
  const manifest = readFileSync(path, 'utf8');
  assert(!/^agent-browser\s*=/m.test(manifest), `${path} depends on the CLI package`);
}

for (const requiredSection of [
  '## Trust Boundaries',
  '## Protected Assets',
  '## Threats And Required Controls',
  '## Request Semantics',
  '## Receipt Semantics',
  '## Dependency Direction',
]) {
  assert(contract.includes(requiredSection), `missing contract section: ${requiredSection}`);
}

console.log(
  `captcha guard contract: ${fixtures.valid.length} valid and ${fixtures.invalid.length} adversarial fixtures passed`,
);
