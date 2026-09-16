#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const failures = [];

function read(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

function requireCondition(condition, message) {
  if (!condition) failures.push(message);
}

function rustFilesUnder(path) {
  const root = join(repoRoot, path);
  const files = [];
  const visit = (current) => {
    for (const entry of readdirSync(current)) {
      const candidate = join(current, entry);
      if (statSync(candidate).isDirectory()) visit(candidate);
      else if (candidate.endsWith('.rs')) files.push(candidate);
    }
  };
  visit(root);
  return files;
}

const crateRoot = 'crates/agent-browser-candidate';
const manifestPath = `${crateRoot}/Cargo.toml`;
const manifest = existsSync(join(repoRoot, manifestPath)) ? read(manifestPath) : '';

requireCondition(
  read('Cargo.toml').includes('"crates/agent-browser-candidate"'),
  'root Cargo workspace must include agent-browser-candidate',
);
requireCondition(
  manifest.includes('name = "agent-browser-candidate"'),
  'agent-browser-candidate package manifest must exist',
);
requireCondition(
  existsSync(join(repoRoot, crateRoot, 'src/lib.rs')),
  'candidate crate must own src/lib.rs',
);
requireCondition(
  read('scripts/ci/rust-tests.sh').includes('test -p agent-browser-candidate'),
  'normal Rust test entrypoint must run candidate crate tests',
);
requireCondition(
  read('cli/Cargo.toml').includes('agent-browser-candidate = { path = "../crates/agent-browser-candidate" }'),
  'CLI adapter must depend directly on the candidate kernel',
);
requireCondition(
  read('cli/src/main.rs').includes('candidate::run_candidate_command(&clean, flags.json)'),
  'CLI must dispatch candidate inspection before daemon-backed commands',
);

const candidateAdapter = read('cli/src/candidate.rs');
for (const forbiddenEffect of [
  'ensure_daemon',
  'send_command',
  'std::process::Command',
  'fs::write',
  'fs::create_dir',
  'run_workstation_upgrade_recover',
]) {
  requireCondition(
    !candidateAdapter.includes(forbiddenEffect),
    `candidate CLI adapter must remain read-only: ${forbiddenEffect}`,
  );
}
for (const documentationPath of [
  'cli/src/output.rs',
  'README.md',
  'skills/agent-browser/SKILL.md',
  'docs/src/app/commands/page.mdx',
]) {
  const documentation = read(documentationPath);
  requireCondition(
    documentation.includes('candidate status')
      && documentation.includes('candidate inspect')
      && documentation.includes('--input-closure'),
    `candidate command contract must be documented in ${documentationPath}`,
  );
}

const forbiddenDependencies = [
  'agent-browser',
  'agent-browser-cdp',
  'agent-browser-desktop-services',
  'agent-browser-lease-authority',
  'tokio',
  'reqwest',
  'image',
];
requireCondition(
  !forbiddenDependencies.some((dependency) => (
    new RegExp(`^\\s*${dependency}\\s*=`, 'm').test(manifest)
  )),
  'candidate kernel must not depend on CLI, browser, runtime, network, image, or effect crates',
);

for (const file of rustFilesUnder(`${crateRoot}/src`)) {
  const source = readFileSync(file, 'utf8');
  requireCondition(
    !/(?:crate::native::|agent_browser::|agent_browser_cdp::|std::process::Command|std::fs::)/.test(source),
    `candidate kernel must remain pure: ${relative(repoRoot, file)}`,
  );
}

if (failures.length > 0) {
  console.error('Candidate crate architecture contract failed:');
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}

console.log('Candidate crate architecture contract passed');
