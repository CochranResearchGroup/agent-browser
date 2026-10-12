import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync, unlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { verifyPolicyWiring } from './test-policy-wiring.js';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function fixture(run) {
  const target = mkdtempSync(join(tmpdir(), 'agent-browser-policy-wiring-'));
  try {
    for (const path of ['AGENTS.md', 'docs/dev/policies', '.governance/policy-context.json', '.codex/skills/repo-policy-selector/release-manifest.json']) {
      cpSync(join(root, path), join(target, path), { recursive: true });
    }
    run(target);
  } finally {
    rmSync(target, { recursive: true, force: true });
  }
}
function changeManifest(target, mutate) {
  const path = join(target, '.governance/policy-context.json');
  const manifest = JSON.parse(readFileSync(path, 'utf8'));
  mutate(manifest);
  writeFileSync(path, JSON.stringify(manifest));
}

test('current bounded pilot routes all canonical policies without legacy AGENTS prose', () => {
  assert.equal(verifyPolicyWiring(root).policyCount, 48);
});
for (const [name, mutate, expected] of [
  ['missing canonical policy', (target) => unlinkSync(join(target, 'docs/dev/policies/0047-multi-session-development-operating-model.md')), /ENOENT/],
  ['canonical policy hash drift', (target) => writeFileSync(join(target, 'docs/dev/policies/0047-multi-session-development-operating-model.md'), '# Changed\n'), /policy source hash drift/],
  ['unregistered added policy', (target) => writeFileSync(join(target, 'docs/dev/policies/9999-added.md'), '# Added\n'), /policy inventory differs/],
  ['missing canonical fallback', (target) => changeManifest(target, (manifest) => { manifest.fallback_paths = manifest.fallback_paths.filter((path) => !path.includes('0047-')); }), /canonical fallback does not route/],
  ['missing task profile', (target) => changeManifest(target, (manifest) => { manifest.profiles.pop(); }), /profile inventory differs/],
  ['duplicate task profile', (target) => changeManifest(target, (manifest) => { manifest.profiles[1] = manifest.profiles[0]; }), /duplicate manifest profile/],
  ['unrouted task kind', (target) => changeManifest(target, (manifest) => { manifest.profiles[0].task_kinds.push('unsupported'); }), /does not route profile\/task kinds/],
  ['profile hash drift', (target) => changeManifest(target, (manifest) => { manifest.profiles[0].sources[0].sha256 = '0'.repeat(64); }), /profile source hash drift/],
  ['missing profile fallback', (target) => changeManifest(target, (manifest) => { manifest.profiles[0].fallback_paths = []; }), /profile fallback does not route/],
  ['missing MCP trigger', (target) => { const path = join(target, 'AGENTS.md'); writeFileSync(path, readFileSync(path, 'utf8').replace('For a supported task, call governance MCP `gov_policy`', 'Removed trigger')); }, /gov_policy/],
]) {
  test(`fails closed for ${name}`, () => fixture((target) => {
    mutate(target);
    assert.throws(() => verifyPolicyWiring(target), expected);
  }));
}
