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
const manifest = read('crates/agent-browser-challenge-visual-adapter/Cargo.toml');
const sources = rustSources(join(root, 'crates/agent-browser-challenge-visual-adapter/src')).join('\n');

requireCondition(workspace.includes('crates/agent-browser-challenge-visual-adapter'), 'workspace must include challenge-visual-adapter');
requireCondition(manifest.includes('name = "agent-browser-challenge-visual-adapter"'), 'challenge visual adapter manifest must exist');
requireCondition(manifest.includes('agent-browser-challenge-control = { path = "../agent-browser-challenge-control" }'), 'adapter must consume the challenge-control protocol');
requireCondition(/\bpub\s+trait\s+VisualProviderTransport\b/.test(sources), 'adapter must own one injected transport seam');
requireCondition(/\bpub\s+fn\s+invoke_visual_provider\b/.test(sources), 'adapter must own one invocation entrypoint');
requireCondition(sources.includes('EphemeralProcessLocal'), 'adapter must preserve explicit process-local retention');

for (const forbidden of [
  'crate::native::', 'agent_browser::', 'ServiceState', 'std::fs', 'std::process',
  'tokio::', 'reqwest::', 'image::', 'agent_browser_desktop_services',
  'agent_browser_cdp', 'Command::',
]) {
  requireCondition(!sources.includes(forbidden), `adapter source contains forbidden dependency: ${forbidden}`);
}

for (const dependency of [
  'agent-browser =', 'agent-browser-cdp', 'agent-browser-desktop-services',
  'tokio', 'reqwest', 'image', 'tesseract', 'base64',
]) {
  requireCondition(!manifest.includes(dependency), `adapter manifest contains forbidden dependency: ${dependency}`);
}

if (failures.length) {
  console.error('Challenge visual adapter architecture contract failed:');
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}
console.log('Challenge visual adapter architecture contract passed');
