#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function verifyPolicyWiring(root) {
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

  const agents = readFileSync(join(root, 'AGENTS.md'), 'utf8');
  assert(agents.includes('docs/dev/policies/0053-policy-context-pilot.md'), 'AGENTS.md does not route the policy-context pilot');
  assert(agents.includes('.governance/policy-context.json'), 'AGENTS.md does not route the policy manifest');
  assert.match(agents, /For a supported task, call governance MCP `gov_policy`/);
  assert.match(agents, /If MCP is unavailable[\s\S]*read the relevant canonical files directly/);
  assert.match(agents, /Follow returned `fallback_paths`/);
  assert.match(agents, /use `gov_list` or the real files under `docs\/dev\/policies\/`/);

  const manifest = JSON.parse(readFileSync(join(root, '.governance/policy-context.json'), 'utf8'));
  assert.equal(manifest.schema_version, 1);
  const inventory = new Map();
  for (const source of manifest.governance_inventory) {
    assert(!inventory.has(source.path), `duplicate inventory source ${source.path}`);
    const digest = createHash('sha256').update(readFileSync(join(root, source.path))).digest('hex');
    assert.equal(source.sha256, digest, `policy source hash drift: ${source.path}`);
    inventory.set(source.path, source.sha256);
  }
  const expectedInventory = ['AGENTS.md', ...policyFiles.map((name) => `docs/dev/policies/${name}`)];
  assert.deepEqual([...inventory.keys()].sort(), expectedInventory.sort(), 'policy inventory differs from canonical files');
  for (const path of inventory.keys()) {
    assert(manifest.fallback_paths.includes(path), `canonical fallback does not route ${path}`);
  }
  for (const name of [
    '0047-multi-session-development-operating-model.md',
    '0048-forge-issue-reporting.md',
    '0049-github-issue-operations.md',
    '0050-collaborative-development-workflow.md',
  ]) {
    assert(policyFiles.includes(name), `missing policy file ${name}`);
  }
  const tableRows = agents.split('\n').filter((line) => line.startsWith('| agent-browser-'));
  const declaredProfiles = new Map(tableRows.map((line) => {
    const cells = line.split('|').map((cell) => cell.trim());
    return [cells[1], cells[2].split(',').map((kind) => kind.trim()).sort()];
  }));
  assert.equal(declaredProfiles.size, tableRows.length, 'duplicate AGENTS.md profile route');
  assert.equal(declaredProfiles.size, manifest.profiles.length, 'AGENTS.md profile inventory differs');
  const profileIds = new Set();
  for (const profile of manifest.profiles) {
    assert(!profileIds.has(profile.id), `duplicate manifest profile ${profile.id}`);
    profileIds.add(profile.id);
    assert.deepEqual(declaredProfiles.get(profile.id), [...profile.task_kinds].sort(), `AGENTS.md does not route profile/task kinds ${profile.id}`);
    for (const source of profile.sources) {
      assert(inventory.has(source.path), `profile source is not inventoried: ${source.path}`);
      assert.equal(source.sha256, inventory.get(source.path), `profile source hash drift: ${source.path}`);
      assert(profile.fallback_paths.includes(source.path), `profile fallback does not route ${source.path}`);
    }
  }

  const release = JSON.parse(readFileSync(join(root, '.codex/skills/repo-policy-selector/release-manifest.json'), 'utf8'));
  assert.equal(release.bundle_version, '0.1.26');
  return { policyCount: policyFiles.length, selectorVersion: release.bundle_version };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
  const result = verifyPolicyWiring(root);
  console.log(`Policy wiring verified: ${result.policyCount} unique identities, current pilot routes and hashes, selector v${result.selectorVersion}`);
}
