#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, relative } from 'node:path';

function argument(name) {
  const index = process.argv.indexOf(name);
  if (index < 0 || !process.argv[index + 1]) throw new Error(`missing ${name}`);
  return process.argv[index + 1];
}

const root = resolve(argument('--root'));
const profilePath = resolve(root, argument('--profile'));
const outputPath = resolve(root, argument('--output'));
const profile = JSON.parse(readFileSync(profilePath, 'utf8'));

if (profile.schemaVersion !== 'agent-browser.policy-capsule-profile.v1') {
  throw new Error('unsupported policy capsule profile');
}
for (const field of ['scope', 'authority', 'requiredChecks', 'hardStops', 'rereadWhen']) {
  if (!profile[field] || (Array.isArray(profile[field]) && profile[field].length === 0)) {
    throw new Error(`empty policy capsule field: ${field}`);
  }
}

const policies = profile.policies.map((path) => {
  const bytes = readFileSync(resolve(root, path));
  return { path, sha256: createHash('sha256').update(bytes).digest('hex') };
});
const lines = [
  '# Policy execution capsule',
  '',
  `Profile: \`${relative(root, profilePath)}\``,
  `Scope: ${profile.scope}`,
  `Authority: ${profile.authority}`,
  '',
  '## Operative rules',
  '',
  ...profile.operativeRules.map((item) => `- ${item}`),
  '',
  '## Required checks',
  '',
  ...profile.requiredChecks.map((item) => `- \`${item}\``),
  '',
  '## Hard stops',
  '',
  ...profile.hardStops.map((item) => `- ${item}`),
  '',
  '## Re-read triggers',
  '',
  ...profile.rereadWhen.map((item) => `- ${item}`),
  '',
  '## Canonical policy hashes',
  '',
  ...policies.map(({ path, sha256 }) => `- \`${path}\` — \`${sha256}\``),
  '',
];
writeFileSync(outputPath, `${lines.join('\n')}\n`);
console.log(`policy capsule written: ${relative(root, outputPath)} policies=${policies.length}`);
