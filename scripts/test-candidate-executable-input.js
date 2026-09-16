#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  collectExecutableInputClosure,
  parseCargoDepInfo,
} from './lib/candidate-executable-input.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-candidate-input-'));

function write(path, body) {
  const target = join(root, path);
  mkdirSync(join(target, '..'), { recursive: true });
  writeFileSync(target, body);
  return target;
}

function context() {
  return {
    target: 'x86_64-unknown-linux-gnu',
    toolchain: 'rustc 1.90.0',
    cargoProfile: 'release',
    resolvedBuildProfile: {
      optLevel: '3',
      lto: 'fat',
      codegenUnits: 1,
      strip: true,
    },
    features: ['service', 'service'],
    reviewedEnvironmentInputs: {
      AGENT_BROWSER_BUILD_MODE: 'a'.repeat(64),
    },
  };
}

try {
  assert.deepEqual(
    parseCargoDepInfo([
      'target: cli/src/main.rs scripts/file\\ with\\ spaces.sh \\',
      ' cli/src/native/mod.rs',
      'other: cli/src/main.rs',
    ].join('\n')),
    ['cli/src/main.rs', 'scripts/file with spaces.sh', 'cli/src/native/mod.rs'],
  );

  write('cli/src/main.rs', 'fn main() {}\n');
  write('crates/example/src/lib.rs', 'pub fn example() {}\n');
  write('Cargo.toml', '[workspace]\n');
  write('Cargo.lock', '# lock\n');
  write('cli/build.rs', 'fn main() {}\n');
  write('package.json', '{"version":"0.28.0"}\n');
  write('scripts/embedded.sh', '#!/bin/sh\n');
  write('packages/dashboard/out/index.html', '<html>ready</html>\n');
  write('packages/dashboard/out/_next/app.js', 'ready();\n');
  const cliDep = write(
    'target/release/deps/agent_browser.d',
    'target/release/agent-browser: cli/src/main.rs cli/src/../src/main.rs scripts/embedded.sh\n',
  );
  const crateDep = write(
    'target/release/deps/agent_browser_example.d',
    'target/release/libexample.rlib: crates/example/src/lib.rs\n',
  );

  const closure = collectExecutableInputClosure({
    repoRoot: root,
    depInfoPaths: [cliDep, crateDep],
    requiredPaths: ['Cargo.toml', 'Cargo.lock', 'cli/build.rs', 'package.json'],
    recursiveRoots: ['packages/dashboard/out'],
    context: context(),
    productionShaped: true,
  });

  assert.equal(closure.schemaVersion, 'agent-browser.executable-input-closure.v1');
  assert.deepEqual(closure.context.features, ['service']);
  assert.deepEqual(
    closure.inputs.map((input) => input.path),
    [
      'Cargo.lock',
      'Cargo.toml',
      'cli/build.rs',
      'cli/src/main.rs',
      'crates/example/src/lib.rs',
      'package.json',
      'packages/dashboard/out/_next/app.js',
      'packages/dashboard/out/index.html',
      'scripts/embedded.sh',
    ],
  );
  assert.equal(
    closure.inputs.find((input) => input.path === 'cli/build.rs').category,
    'build_script',
  );
  assert.equal(
    closure.inputs.find((input) => input.path === 'packages/dashboard/out/index.html').category,
    'embedded_dashboard',
  );
  assert.ok(closure.inputs.every((input) => /^[a-f0-9]{64}$/u.test(input.sha256)));

  write('packages/dashboard/out/index.html', 'Dashboard not built. Run: pnpm build\n');
  assert.throws(
    () => collectExecutableInputClosure({
      repoRoot: root,
      depInfoPaths: [cliDep, crateDep],
      requiredPaths: ['Cargo.toml', 'Cargo.lock', 'cli/build.rs'],
      recursiveRoots: ['packages/dashboard/out'],
      context: context(),
      productionShaped: true,
    }),
    /candidate_dashboard_placeholder/u,
  );

  assert.throws(
    () => collectExecutableInputClosure({
      repoRoot: root,
      depInfoPaths: [cliDep],
      requiredPaths: ['/etc/hosts'],
      context: context(),
    }),
    /candidate_input_outside_repository/u,
  );

  const rawEnvironment = context();
  rawEnvironment.reviewedEnvironmentInputs.AGENT_BROWSER_BUILD_MODE = 'release';
  assert.throws(
    () => collectExecutableInputClosure({
      repoRoot: root,
      depInfoPaths: [cliDep],
      context: rawEnvironment,
    }),
    /candidate_input_environment_digest_invalid/u,
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Candidate executable-input collector tests passed');
