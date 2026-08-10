#!/usr/bin/env node

import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const failures = [];
const compilingCargo = /\bcargo\s+(?:build|check|clippy|fmt|run|test)\b/;

const packageJson = JSON.parse(readFileSync(path.join(repoRoot, 'package.json'), 'utf8'));
for (const [name, command] of Object.entries(packageJson.scripts ?? {})) {
  if (compilingCargo.test(command) && !command.includes('scripts/ci/cargo-safe.sh')) {
    failures.push(`package.json:${name}:raw_compiling_cargo`);
  }
}

function visit(directory) {
  for (const entry of readdirSync(directory)) {
    const absolute = path.join(directory, entry);
    const relative = path.relative(repoRoot, absolute);
    if (
      relative === 'scripts/ci/cargo-safe.sh' ||
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
      const executableCargo =
        /command:\s*['"]cargo['"]/.test(line) ||
        /^\s*cargo\s+(?:build|check|clippy|fmt|run|test)\b/.test(line) ||
        /['"`]cargo\s+(?:build|check|clippy|fmt|run|test)\b/.test(line);
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
