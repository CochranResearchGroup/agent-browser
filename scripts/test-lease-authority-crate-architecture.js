#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';

const repoRoot = resolve(import.meta.dirname, '..');

function rustFilesUnder(root) {
  const files = [];
  const visit = (directory) => {
    if (!existsSync(directory)) return;
    for (const entry of readdirSync(directory)) {
      const candidate = join(directory, entry);
      if (statSync(candidate).isDirectory()) visit(candidate);
      else if (candidate.endsWith('.rs')) files.push(candidate);
    }
  };
  visit(root);
  return files;
}

function read(path, root = repoRoot) {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function check(root) {
  const failures = [];
  const requireCondition = (condition, message) => { if (!condition) failures.push(message); };
  const workspace = read('Cargo.toml', root);
  const cliManifest = read('cli/Cargo.toml', root);
  const crateManifest = read('crates/agent-browser-lease-authority/Cargo.toml', root);
  const cliRust = rustFilesUnder(join(root, 'cli', 'src'));
  const cliSources = cliRust.map((file) => readFileSync(file, 'utf8'));
  requireCondition(workspace.includes('crates/agent-browser-lease-authority'), 'root Cargo workspace must include agent-browser-lease-authority');
  requireCondition(crateManifest.includes('name = "agent-browser-lease-authority"'), 'agent-browser-lease-authority package manifest must exist');
  requireCondition(cliManifest.includes('agent-browser-lease-authority = { path = "../crates/agent-browser-lease-authority" }'), 'CLI must depend directly on the local Lease-authority crate');
  requireCondition(existsSync(join(root, 'crates/agent-browser-lease-authority/src/lib.rs')), 'Lease-authority crate must own src/lib.rs');
  requireCondition(!existsSync(join(root, 'cli/src/native/service_lease_authority.rs')), 'legacy native::service_lease_authority implementation must be absent');
  requireCondition(!existsSync(join(root, 'cli/src/native/service_lease_authority')), 'legacy native::service_lease_authority directory must be absent');
  requireCondition(!read('cli/src/native/mod.rs', root).match(/(?:pub\s+)?(?:crate\s+)?mod\s+service_lease_authority\s*;/), 'native module must not retain a service_lease_authority facade');
  requireCondition(!cliSources.some((source) => /(?:crate::native::|super(?:::\s*super)*::)service_lease_authority\b/.test(source)), 'CLI source must not import the removed native service_lease_authority owner');
  requireCondition(cliSources.some((source) => /\b(?:use|pub\s+use)\s+agent_browser_lease_authority(?:::|\s*;)/.test(source)), 'CLI consumers must import agent_browser_lease_authority directly');
  requireCondition(!/\bagent-browser\s*=/.test(crateManifest), 'Lease-authority crate must not depend back on the agent-browser package');
  const crateSources = rustFilesUnder(join(root, 'crates/agent-browser-lease-authority/src')).map((file) => readFileSync(file, 'utf8'));
  requireCondition(!crateSources.some((source) => /(?:crate::native::|\bagent_browser::native::|\bagent_browser_cli::native::)/.test(source)), 'Lease-authority crate must not import CLI/native paths upward');
  return failures;
}

function currentTreeCheck() {
  const failures = check(repoRoot);
  if (failures.length) {
    console.error('Lease-authority crate architecture contract failed:');
    for (const failure of failures) console.error(`  - ${failure}`);
    process.exit(1);
  }
  console.log('Lease-authority crate architecture contract passed');
}

function selfTest() {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-lease-authority-contract-'));
  try {
    mkdirSync(join(root, 'cli/src/native'), { recursive: true });
    mkdirSync(join(root, 'crates/agent-browser-lease-authority/src'), { recursive: true });
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["cli", "crates/agent-browser-lease-authority"]\n');
    writeFileSync(join(root, 'cli/Cargo.toml'), '[dependencies]\nagent-browser-lease-authority = { path = "../crates/agent-browser-lease-authority" }\n');
    writeFileSync(join(root, 'crates/agent-browser-lease-authority/Cargo.toml'), '[package]\nname = "agent-browser-lease-authority"\n');
    writeFileSync(join(root, 'crates/agent-browser-lease-authority/src/lib.rs'), 'pub struct Authority;\n');
    writeFileSync(join(root, 'cli/src/native/mod.rs'), 'pub(crate) mod adapter;\n');
    writeFileSync(join(root, 'cli/src/native/adapter.rs'), 'use agent_browser_lease_authority::Authority;\n');
    if (check(root).length) throw new Error('valid extracted fixture was rejected');
    const cases = [
      ['missing crate', 'rm-crate'], ['upward import', 'upward-import'],
      ['retained implementation', 'retained-implementation'], ['retained directory', 'retained-directory'],
      ['retained facade', 'retained-facade'], ['missing direct dependency', 'missing-dependency'],
      ['reverse dependency', 'reverse-dependency'], ['missing direct import', 'missing-import'],
      ['upward crate import', 'upward-crate-import'],
    ];
    for (const [label, mutation] of cases) {
      const mutated = mkdtempSync(join(tmpdir(), `agent-browser-lease-authority-${mutation}-`));
      try {
        mkdirSync(join(mutated, 'cli/src/native'), { recursive: true });
        mkdirSync(join(mutated, 'crates/agent-browser-lease-authority/src'), { recursive: true });
        for (const path of ['Cargo.toml', 'cli/Cargo.toml', 'crates/agent-browser-lease-authority/Cargo.toml', 'crates/agent-browser-lease-authority/src/lib.rs', 'cli/src/native/mod.rs', 'cli/src/native/adapter.rs']) {
          mkdirSync(join(mutated, path, '..'), { recursive: true });
          writeFileSync(join(mutated, path), read(path, root));
        }
        if (mutation === 'rm-crate') rmSync(join(mutated, 'crates/agent-browser-lease-authority'), { recursive: true, force: true });
        if (mutation === 'upward-import') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use crate::native::service_lease_authority::Authority;\n');
        if (mutation === 'retained-implementation') writeFileSync(join(mutated, 'cli/src/native/service_lease_authority.rs'), 'pub struct Authority;\n');
        if (mutation === 'retained-directory') mkdirSync(join(mutated, 'cli/src/native/service_lease_authority'), { recursive: true });
        if (mutation === 'retained-facade') writeFileSync(join(mutated, 'cli/src/native/mod.rs'), 'mod service_lease_authority;\n');
        if (mutation === 'missing-dependency') writeFileSync(join(mutated, 'cli/Cargo.toml'), '[dependencies]\n');
        if (mutation === 'reverse-dependency') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/Cargo.toml'), '[package]\nname = "agent-browser-lease-authority"\n[dependencies]\nagent-browser = { path = "../../cli" }\n');
        if (mutation === 'missing-import') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'pub fn adapter() {}\n');
        if (mutation === 'upward-crate-import') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/lib.rs'), 'use crate::native::service_lease_authority::Authority;\n');
        if (!check(mutated).length) throw new Error(`${label} mutation was not rejected`);
      } finally { rmSync(mutated, { recursive: true, force: true }); }
    }
    console.log('Lease-authority architecture contract self-test passed');
  } finally { rmSync(root, { recursive: true, force: true }); }
}

if (process.argv.includes('--self-test')) selfTest();
else currentTreeCheck();
