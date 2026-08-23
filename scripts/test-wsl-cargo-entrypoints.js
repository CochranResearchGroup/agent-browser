#!/usr/bin/env node

import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const failures = [];
const compilingCargo = /\bcargo\s+(?:build|check|clippy|fmt|run|test)\b/;

function launchesRawCargo(line) {
  return (
    /command:\s*['"]cargo['"]/.test(line) ||
    /^\s*cargo\s+(?:build|check|clippy|fmt|run|test)\b/.test(line) ||
    /['"`]cargo\s+(?:build|check|clippy|fmt|run|test)\b/.test(line) ||
    /\b(?:runCommand|spawn|spawnSync|execFileSync)\s*\(\s*['"]cargo['"]/.test(line)
  );
}

for (const [label, source, expectedRaw] of [
  ['shell', 'cargo test --manifest-path cli/Cargo.toml', true],
  ['command-field', "command: 'cargo'", true],
  ['run-command', "runCommand('cargo', cargoArgs)", true],
  ['spawn', "spawn('cargo', cargoArgs(args), options)", true],
  ['spawn-sync', "spawnSync('cargo', ['check'])", true],
  ['exec-file', "execFileSync('cargo', ['build'])", true],
  ['guarded', "spawn('scripts/ci/cargo-safe.sh', ['test'])", false],
]) {
  if (launchesRawCargo(source) !== expectedRaw) {
    failures.push(`detector_fixture:${label}:expected_raw=${expectedRaw}`);
  }
}

const packageJson = JSON.parse(readFileSync(path.join(repoRoot, 'package.json'), 'utf8'));
for (const [name, command] of Object.entries(packageJson.scripts ?? {})) {
  if (compilingCargo.test(command) && !command.includes('scripts/ci/cargo-safe.sh')) {
    failures.push(`package.json:${name}:raw_compiling_cargo`);
  }
}

const cargoSafeSource = readFileSync(
  path.join(repoRoot, 'scripts/ci/cargo-safe.sh'),
  'utf8',
);
for (const required of [
  'Agent Browser Cargo build capacity',
  'MemoryHigh=$aggregate_memory_high',
  '--slice="$cargo_slice"',
  'exec {admission_fd}>&-',
]) {
  if (!cargoSafeSource.includes(required)) {
    failures.push(`scripts/ci/cargo-safe.sh:missing_capacity_contract:${required}`);
  }
}
if (cargoSafeSource.includes('flock --close "$lock_file"')) {
  failures.push('scripts/ci/cargo-safe.sh:full_lifetime_serialization_present');
}

function visit(directory) {
  for (const entry of readdirSync(directory)) {
    const absolute = path.join(directory, entry);
    const relative = path.relative(repoRoot, absolute);
    if (
      relative === 'scripts/ci/cargo-safe.sh' ||
      relative === 'scripts/test-wsl-cargo-entrypoints.js' ||
      relative.startsWith('scripts/windows-debug/') ||
      relative.includes('/target/') ||
      relative.includes('/node_modules/')
    ) {
      continue;
    }
    if (statSync(absolute).isDirectory()) {
      visit(absolute);
      continue;
    }
    if (!/\.(?:js|mjs|sh)$/.test(entry)) continue;
    const lines = readFileSync(absolute, 'utf8').split('\n');
    for (const [index, line] of lines.entries()) {
      const executableCargo = launchesRawCargo(line);
      if (executableCargo && !line.includes('cargo-safe.sh')) {
        failures.push(`${relative}:${index + 1}:raw_compiling_cargo`);
      }
    }
  }
}

visit(path.join(repoRoot, 'scripts'));

if (failures.length > 0) {
  console.error('WSL Cargo entrypoint safety gate failed');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log('WSL Cargo entrypoint safety gate passed');
