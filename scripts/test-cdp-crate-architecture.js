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

const workspaceManifest = existsSync(join(repoRoot, 'Cargo.toml'))
  ? read('Cargo.toml')
  : '';
const cliManifest = read('cli/Cargo.toml');
const cdpManifestPath = 'crates/agent-browser-cdp/Cargo.toml';
const cdpManifest = existsSync(join(repoRoot, cdpManifestPath))
  ? read(cdpManifestPath)
  : '';

requireCondition(
  workspaceManifest.includes('"cli"') && workspaceManifest.includes('"crates/agent-browser-cdp"'),
  'root Cargo workspace must include the CLI and CDP crates',
);
requireCondition(
  cdpManifest.includes('name = "agent-browser-cdp"'),
  'agent-browser-cdp package manifest must exist',
);
requireCondition(
  cliManifest.includes('agent-browser-cdp = { path = "../crates/agent-browser-cdp" }'),
  'CLI must depend directly on the local CDP crate',
);
requireCondition(existsSync(join(repoRoot, 'Cargo.lock')), 'workspace lockfile must live at the repository root');
requireCondition(
  read('.cargo/config.toml').includes('target-dir = "cli/target"'),
  'workspace must preserve the established cli/target artifact location',
);

for (const path of [
  'crates/agent-browser-cdp/src/client.rs',
  'crates/agent-browser-cdp/src/types.rs',
  'crates/agent-browser-cdp/build.rs',
  'crates/agent-browser-cdp/cdp-protocol/browser_protocol.json',
  'crates/agent-browser-cdp/cdp-protocol/js_protocol.json',
]) {
  requireCondition(existsSync(join(repoRoot, path)), `CDP crate must own ${path}`);
}

for (const path of [
  'cli/src/native/cdp/client.rs',
  'cli/src/native/cdp/types.rs',
  'cli/cdp-protocol/browser_protocol.json',
  'cli/cdp-protocol/js_protocol.json',
]) {
  requireCondition(!existsSync(join(repoRoot, path)), `legacy CDP owner must be absent: ${path}`);
}

const cliCdpModule = read('cli/src/native/cdp/mod.rs');
requireCondition(!/pub mod (client|types);/.test(cliCdpModule), 'native CDP module must not retain a transport facade');
requireCondition(cliCdpModule.includes('pub mod chrome;'), 'Chrome launch must remain in the CLI crate');
requireCondition(cliCdpModule.includes('pub mod lightpanda;'), 'Lightpanda launch must remain in the CLI crate');
requireCondition(
  !read('cli/build.rs').includes('cdp_generated.rs') && read('crates/agent-browser-cdp/build.rs').includes('cdp_generated.rs'),
  'generated protocol build ownership must move to the CDP crate',
);

const obsoleteImport = /(?:crate::native::|native::|super(?:::\s*super)*::)cdp::(?:client|types)/;
for (const file of rustFilesUnder('cli/src')) {
  const source = readFileSync(file, 'utf8');
  requireCondition(
    !obsoleteImport.test(source),
    `CLI source must import agent_browser_cdp directly: ${relative(repoRoot, file)}`,
  );
}

const rustTests = read('scripts/ci/rust-tests.sh');
requireCondition(
  rustTests.includes('test -p agent-browser-cdp'),
  'normal Rust test entrypoint must run CDP crate tests',
);

const workflow = read('.github/workflows/ci.yml');
const workspaceClippyCommand = ['cargo', 'clippy --workspace --manifest-path Cargo.toml -- -D warnings'].join(' ');
requireCondition(
  workflow.includes(workspaceClippyCommand),
  'Rust Quality CI must run strict Clippy across the workspace',
);

const dockerCompose = read('docker/docker-compose.yml');
requireCondition(
  dockerCompose.includes('- ..:/build') && dockerCompose.includes('working_dir: /build/cli'),
  'cross-compilation containers must mount the workspace while building the CLI member',
);

if (failures.length > 0) {
  console.error('CDP crate architecture contract failed:');
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}

console.log('CDP crate architecture contract passed');
