#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const policiesDir = join(root, 'docs/dev/policies');
const policyFiles = readdirSync(policiesDir).filter((name) => /^\d{4}-.+\.md$/.test(name));
const identities = new Map();

for (const name of policyFiles) {
  const identity = name.replace(/^\d{4}-/, '').replace(/\.md$/, '');
  const owners = identities.get(identity) ?? [];
  owners.push(name);
  identities.set(identity, owners);
}

for (const [identity, owners] of identities) {
  assert.equal(owners.length, 1, `policy identity ${identity} has multiple owners: ${owners.join(', ')}`);
}

const expected = [
  '0047-multi-session-development-operating-model.md',
  '0048-forge-issue-reporting.md',
  '0049-github-issue-operations.md',
  '0050-collaborative-development-workflow.md',
];
const agents = readFileSync(join(root, 'AGENTS.md'), 'utf8');

const manifest = JSON.parse(readFileSync(join(root, '.governance/policy-context.json'), 'utf8'));
assert.equal(manifest.schema_version, 1);
assert.match(agents, /gov_policy/);
assert.match(agents, /fallback_paths/);
assert(agents.includes('.governance/policy-context.json'), 'AGENTS.md must identify the checked manifest');
assert(agents.includes('docs/dev/policies/0053-policy-context-pilot.md'), 'AGENTS.md must route the canonical pilot');

const inventory = new Map();
for (const entry of manifest.governance_inventory) {
  assert(!inventory.has(entry.path), `duplicate inventory path ${entry.path}`);
  inventory.set(entry.path, entry.sha256);
  const actual = createHash('sha256').update(readFileSync(join(root, entry.path))).digest('hex');
  assert.equal(actual, entry.sha256, `unreviewed source drift: ${entry.path}`);
}
assert(inventory.has('AGENTS.md'), 'manifest must bind the repository entrypoint');
for (const name of policyFiles) {
  assert(inventory.has(`docs/dev/policies/${name}`), `policy missing from inventory: ${name}`);
}
for (const name of expected) {
  assert(policyFiles.includes(name), `missing policy file ${name}`);
  assert(manifest.fallback_paths.includes(`docs/dev/policies/${name}`), `missing canonical fallback: ${name}`);
}
for (const [profileId, taskKinds] of [
  ['agent-browser-documentation', ['documentation']],
  ['agent-browser-implementation', ['implementation', 'review']],
  ['agent-browser-planning', ['planning']],
  ['agent-browser-runtime', ['runtime']],
]) {
  assert(agents.includes(profileId), `AGENTS.md does not route ${profileId}`);
  const profile = manifest.profiles.find(value => value.id === profileId);
  assert(profile, `missing policy profile ${profileId}`);
  for (const kind of taskKinds) assert(profile.task_kinds.includes(kind), `${profileId} missing ${kind}`);
  for (const source of profile.sources) {
    assert.equal(source.sha256, inventory.get(source.path), `${profileId} has inconsistent source binding`);
  }
  for (const path of profile.fallback_paths) assert(inventory.has(path), `${profileId} fallback is not inventoried: ${path}`);
}

const release = JSON.parse(
  readFileSync(join(root, '.codex/skills/repo-policy-selector/release-manifest.json'), 'utf8'),
);
assert.equal(release.bundle_version, '0.1.26');

console.log(`Policy wiring verified: ${policyFiles.length} unique identities, selector v${release.bundle_version}`);
