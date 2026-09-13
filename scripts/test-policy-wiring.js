#!/usr/bin/env node

import assert from 'node:assert/strict';
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

for (const name of expected) {
  assert(policyFiles.includes(name), `missing policy file ${name}`);
  assert(agents.includes(`docs/dev/policies/${name}`), `AGENTS.md does not route ${name}`);
}

assert.match(agents, /before\s+starting or resuming a substantive development lane/i);
assert.match(agents, /before any issue provider mutation/i);

const release = JSON.parse(
  readFileSync(join(root, '.codex/skills/repo-policy-selector/release-manifest.json'), 'utf8'),
);
assert.equal(release.bundle_version, '0.1.26');

console.log(`Policy wiring verified: ${policyFiles.length} unique identities, selector v${release.bundle_version}`);
