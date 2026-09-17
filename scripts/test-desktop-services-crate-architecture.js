#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');

function read(path) {
  const absolute = join(repoRoot, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function rustSources(root) {
  const sources = [];
  if (!existsSync(root)) return sources;
  for (const entry of readdirSync(root)) {
    const path = join(root, entry);
    if (statSync(path).isDirectory()) sources.push(...rustSources(path));
    else if (path.endsWith('.rs')) sources.push(readFileSync(path, 'utf8'));
  }
  return sources;
}

const failures = [];
const requireCondition = (condition, message) => {
  if (!condition) failures.push(message);
};
const workspace = read('Cargo.toml');
const cliManifest = read('cli/Cargo.toml');
const crateManifest = read('crates/agent-browser-desktop-services/Cargo.toml');
const crateSources = rustSources(join(repoRoot, 'crates/agent-browser-desktop-services', 'src'));
const joinedCrateSources = crateSources.join('\n');
const cliDesktopInteraction = read('cli/src/native/desktop_interaction.rs');

requireCondition(
  workspace.includes('crates/agent-browser-desktop-services'),
  'root workspace must include agent-browser-desktop-services',
);
requireCondition(
  crateManifest.includes('name = "agent-browser-desktop-services"'),
  'desktop-services package manifest must exist',
);
requireCondition(
  cliManifest.includes('agent-browser-desktop-services = { path = "../crates/agent-browser-desktop-services" }'),
  'CLI must depend directly on the local desktop-services crate',
);
requireCondition(
  /\bpub\s+fn\s+run_desktop_interaction\b/.test(joinedCrateSources),
  'desktop-services crate must own the transaction entrypoint',
);
requireCondition(
  /\bpub\s+trait\s+DesktopInteractionProvider\b/.test(joinedCrateSources),
  'desktop-services crate must own the provider-neutral adapter contract',
);
requireCondition(
  /\bpub\s+fn\s+admit_desktop_candidate_intent\b/.test(joinedCrateSources),
  'desktop-services crate must own the effect-free candidate-intent admission contract',
);
requireCondition(
  !/\bfn\s+run_claimed_interaction\b/.test(cliDesktopInteraction),
  'CLI adapter must not retain the transaction kernel implementation',
);
for (const forbidden of [
  'crate::native::',
  'agent_browser::native::',
  'ServiceState',
  'service_store',
  'desktop_locator',
  'controlled_x11',
  'agent_browser_challenge_control',
  'agent_browser_challenge_visual_adapter',
  'std::fs',
  'std::net',
  'tokio::',
  'reqwest::',
]) {
  requireCondition(
    !joinedCrateSources.includes(forbidden),
    `desktop-services crate must not import platform or Service State surface: ${forbidden}`,
  );
}
for (const dependency of [
  'agent-browser',
  'agent-browser-cdp',
  'agent-browser-challenge-control',
  'agent-browser-challenge-visual-adapter',
  'tokio',
  'reqwest',
  'image',
]) {
  requireCondition(
    !new RegExp(`^\\s*${dependency}\\s*=`, 'm').test(crateManifest),
    `desktop-services crate must not depend on ${dependency}`,
  );
}

if (failures.length) {
  console.error('Desktop-services crate architecture contract failed:');
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}
console.log('Desktop-services crate architecture contract passed');
