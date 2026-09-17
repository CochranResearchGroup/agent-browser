#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '../..');

const ABANDONED_RETIREMENT_RECORDS = [
  'AbandonedBrowserRetirementPlan',
  'AbandonedBrowserRetirementTransaction',
  'AbandonedBrowserRetirementReceipt',
  'RetirementTerminalProjection',
  'RetirementExitEvidence',
  'RetirementExitFailure',
  'RetirementRecourse',
  'ResourceRetirementPolicy',
];

const FORBIDDEN_DEPENDENCIES = [
  'agent-browser',
  'agent-browser-cdp',
  'agent-browser-challenge-control',
  'agent-browser-desktop-services',
  'tokio',
  'reqwest',
  'image',
  'rust-embed',
  'axum',
  'hyper',
];

const FORBIDDEN_IMPORT_PATHS = [
  'crate::native',
  'agent_browser::native',
  'cli::src::native',
  'std::process',
  'std::fs',
  'std::net',
  'tokio::',
  'reqwest::',
  'image::',
  'hyper::',
  'axum::',
];

const FORBIDDEN_ADAPTER_MODULES = new Set([
  'browser',
  'cdp',
  'capture',
  'controlled_x11',
  'dashboard',
  'desktop',
  'filesystem',
  'http',
  'input',
  'install',
  'mcp',
  'platform',
  'process',
  'provider',
  'remote_view',
  'runtime',
]);

function read(root, path) {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function rustFilesUnder(root) {
  if (!existsSync(root)) return [];
  return readdirSync(root).flatMap((entry) => {
    const absolute = join(root, entry);
    if (statSync(absolute).isDirectory()) return rustFilesUnder(absolute);
    return absolute.endsWith('.rs') ? [absolute] : [];
  });
}

// Boundary checks should inspect syntax-bearing text, not prose or string fixtures.
function withoutCommentsAndStrings(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/.*$/gm, ' ')
    .replace(/"(?:\\.|[^"\\])*"/g, '""')
    .replace(/'(?:\\.|[^'\\])*'/g, "''");
}

function importedPaths(source) {
  const clean = withoutCommentsAndStrings(source);
  const paths = [];
  for (const match of clean.matchAll(/\b(?:pub\s+)?use\s+([^;]+);/g)) paths.push(match[1]);
  for (const match of clean.matchAll(/\b(?:pub\s+)?(?:mod|extern\s+crate)\s+([A-Za-z0-9_]+)/g)) {
    paths.push(match[1]);
  }
  return paths;
}

function check(root = repoRoot) {
  const failures = [];
  const requireCondition = (condition, message) => {
    if (!condition) failures.push(message);
  };

  const workspace = read(root, 'Cargo.toml');
  const manifestPath = join(root, 'crates/agent-browser-service-model/Cargo.toml');
  const sourceRoot = join(root, 'crates/agent-browser-service-model/src');
  const manifest = read(root, 'crates/agent-browser-service-model/Cargo.toml');
  const sourceFiles = rustFilesUnder(sourceRoot);
  const sources = sourceFiles.map((path) => readFileSync(path, 'utf8'));
  requireCondition(existsSync(manifestPath), 'agent-browser-service-model Cargo manifest must exist');
  requireCondition(existsSync(join(sourceRoot, 'lib.rs')), 'agent-browser-service-model must own src/lib.rs');
  requireCondition(
    workspace.includes('crates/agent-browser-service-model'),
    'root Cargo workspace must include agent-browser-service-model',
  );
  requireCondition(
    /\bname\s*=\s*"agent-browser-service-model"/.test(manifest),
    'agent-browser-service-model manifest must declare the expected package name',
  );

  const dependencyLines = manifest
    .split('\n')
    .map((line) => line.replace(/#.*/, ''))
    .filter((line) => /^\s*[A-Za-z0-9_-]+\s*=/.test(line));
  for (const dependency of FORBIDDEN_DEPENDENCIES) {
    requireCondition(
      !dependencyLines.some((line) => new RegExp(`^\\s*${dependency.replaceAll('-', '[\\-]')}\\s*=`).test(line)),
      `service-model crate must not depend on effect/runtime/provider crate: ${dependency}`,
    );
  }
  requireCondition(
    !/\bpath\s*=\s*["'][^"']*(?:^|[/\\])cli(?:[/\\]|["'])/m.test(manifest),
    'service-model crate must not path-depend on the CLI crate',
  );

  for (const source of sources) {
    const clean = withoutCommentsAndStrings(source);
    for (const forbidden of FORBIDDEN_IMPORT_PATHS) {
      requireCondition(
        !new RegExp(`\\b${forbidden.replaceAll(':', '\\s*:\\s*')}`).test(clean),
        `service-model Rust source must not reference forbidden boundary: ${forbidden}`,
      );
    }
    for (const path of importedPaths(source)) {
      const segments = path.trim().split(/\s*::\s*/);
      requireCondition(
        !segments.some((segment) => FORBIDDEN_ADAPTER_MODULES.has(segment)),
        `service-model Rust source must not import runtime/provider/platform adapter module: ${path.trim()}`,
      );
    }
  }

  const retirement = withoutCommentsAndStrings(read(root,
    'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'));
  const cliSources = rustFilesUnder(join(root, 'cli/src'))
    .map((path) => withoutCommentsAndStrings(readFileSync(path, 'utf8')));
  for (const name of ABANDONED_RETIREMENT_RECORDS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...retirement.matchAll(definition)].length === 1,
      `service-model abandoned retirement module must own exactly one definition: ${name}`,
    );
    requireCondition(
      sources.reduce((count, source) => count + [...withoutCommentsAndStrings(source).matchAll(definition)].length, 0) === 1,
      `service-model must not duplicate abandoned retirement record: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate abandoned retirement record: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+const\s+ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1\b/.test(retirement),
    'service-model must own the abandoned retirement plan schema constant',
  );
  for (const adapter of [
    'RetirementObservation', 'RetirementReservation', 'ProcessSample',
    'ServiceState', 'ServiceStateRepository', 'AbandonedBrowserRetirementRuntime',
    'from_environment', 'resource_retirement_policy_from_environment',
    'plan_abandoned_browser_retirement', 'reserve_abandoned_browser_retirement',
    'finalize_abandoned_browser_retirement',
  ]) {
    requireCondition(!new RegExp(`\\b${adapter}\\b`).test(retirement),
      `abandoned retirement model must not absorb CLI adapter: ${adapter}`);
  }
  requireCondition(!/\bstd\s*::\s*env\b/.test(retirement),
    'abandoned retirement model must not read the environment');

  return failures;
}

function main() {
  const failures = check();
  if (failures.length) {
    console.error('Service-model crate architecture contract failed:');
    for (const failure of failures) console.error(`  - ${failure}`);
    process.exitCode = 1;
    return;
  }
  console.log('Service-model crate architecture contract passed');
}

if (import.meta.url === `file://${process.argv[1]}`) main();

export { check };
