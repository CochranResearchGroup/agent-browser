#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const read = (path) => existsSync(join(root, path)) ? readFileSync(join(root, path), 'utf8') : '';
const rustSources = (directory) => {
  if (!existsSync(directory)) return [];
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry);
    return statSync(path).isDirectory() ? rustSources(path) : path.endsWith('.rs') ? [readFileSync(path, 'utf8')] : [];
  });
};

const failures = [];
const requireCondition = (condition, message) => { if (!condition) failures.push(message); };
const workspace = read('Cargo.toml');
const manifest = read('crates/agent-browser-challenge-control/Cargo.toml');
const sources = rustSources(join(root, 'crates/agent-browser-challenge-control/src')).join('\n');

requireCondition(workspace.includes('crates/agent-browser-challenge-control'), 'workspace must include challenge-control');
requireCondition(manifest.includes('name = "agent-browser-challenge-control"'), 'challenge-control manifest must exist');
requireCondition(manifest.includes('agent-browser-desktop-services = { path = "../agent-browser-desktop-services" }'), 'challenge control may consume only the shared desktop contract for effect execution');
requireCondition(/\bpub\s+fn\s+decide\b/.test(sources), 'challenge-control must own one pure decision entrypoint');
requireCondition(/\bpub\s+enum\s+ChallengeState\b/.test(sources), 'challenge-control must own the lifecycle state vocabulary');
requireCondition(/\bpub\s+fn\s+validate_visual_round_intent\b/.test(sources), 'challenge-control must validate one exact current visual intent');
requireCondition(/\bpub\s+fn\s+admit_visual_desktop_candidate_intent\b/.test(sources), 'challenge-control must own one effect-free visual desktop adapter');
for (const forbidden of [
  'crate::native::', 'agent_browser::', 'ServiceState', 'std::fs', 'std::process',
  'std::net', 'tokio::', 'reqwest::', 'image::', 'desktop_locator', 'controlled_x11',
  'run_desktop_interaction', 'DesktopInteractionProvider', 'DesktopControlCoordinator',
  'InputEvent', 'Command::',
]) {
  requireCondition(!sources.includes(forbidden), `challenge-control source contains forbidden dependency: ${forbidden}`);
}
for (const dependency of ['agent-browser =', 'agent-browser-cdp', 'tokio', 'reqwest', 'image', 'tesseract']) {
  requireCondition(!manifest.includes(dependency), `challenge-control manifest contains forbidden dependency: ${dependency}`);
}

if (failures.length) {
  console.error('Challenge-control crate architecture contract failed:');
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}
console.log('Challenge-control crate architecture contract passed');
