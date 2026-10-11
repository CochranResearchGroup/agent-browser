#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync,
  readdirSync, rmSync, writeFileSync,
} from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash, randomUUID } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import {
  createBuildSupportManifest, createCandidateManifest,
} from './lib/candidate-executable-input.js';

const repo = resolve(import.meta.dirname, '..');
assert.ok(process.env.AGENT_BROWSER_FIXTURE_BIN,
  'Set AGENT_BROWSER_FIXTURE_BIN to an explicitly qualified source candidate');
const binary = resolve(process.env.AGENT_BROWSER_FIXTURE_BIN);
const digest = sha256(readFileSync(binary));
const mode = process.env.AGENT_BROWSER_PROGRESS_FIXTURE_MODE || 'workstation';
assert.ok(['workstation', 'candidate'].includes(mode), 'Unknown progress fixture mode');
const artifact = mkdtempSync(join(tmpdir(), 'agent-browser-progress-evidence-'));
const root = mkdtempSync(join(tmpdir(), 'agent-browser-progress-'));
const home = join(root, 'home');
const xdg = join(root, 'xdg');
const bin = join(root, 'bin');
const runId = randomUUID();
for (const path of [home, xdg, bin]) mkdirSync(path, { recursive: true });
// Isolated staging skips unit effects. Reject any unexpected escape.
for (const command of ['systemctl', 'sudo']) {
  const path = join(bin, command);
  writeFileSync(path, '#!/bin/sh\nexit 97\n');
  chmodSync(path, 0o755);
}
const env = Object.fromEntries(
  Object.entries(process.env).filter(([key]) => !key.startsWith('AGENT_BROWSER_')),
);
Object.assign(env, {
  HOME: home, XDG_RUNTIME_DIR: join(xdg, 'runtime'),
  XDG_CONFIG_HOME: join(xdg, 'config'), XDG_DATA_HOME: join(xdg, 'data'),
  XDG_STATE_HOME: join(xdg, 'state'), XDG_CACHE_HOME: join(xdg, 'cache'),
  PATH: `${bin}:/usr/bin:/bin`, AGENT_BROWSER_WORKSTATION_ROOT: join(root, 'workstation'),
  AGENT_BROWSER_FIXTURE_RUN_ID: runId, AGENT_BROWSER_FIXTURE_BIN: binary,
});
let command = ['install', 'workstation', '--apply', '--json', '--progress-json'];
writeEvidence('location.json', { root, home, runId, binary, digest, mode,
  scope: 'synthetic isolated source workflow; not production-shaped build qualification' });

try {
  if (mode === 'candidate') {
    command = candidateCommand();
    const withoutJson = spawnSync(binary, command.filter(arg => arg !== '--json'), {
      cwd: root, env, encoding: 'utf8', timeout: 10000, killSignal: 'SIGKILL',
    });
    assert.equal(withoutJson.status, 1);
    assert.match(withoutJson.stderr, /requires --apply and --json/);
    assert.ok(!existsSync(env.AGENT_BROWSER_WORKSTATION_ROOT),
      'Invalid progress mode must fail before install effects');
  }
  const success = await runApply('success', env);
  assert.equal(success.code, 0, success.stderr || success.stdout);
  const payload = JSON.parse(success.stdout);
  assert.equal(payload.success, true);
  const observations = parseObservations(success.stderr);
  assert.ok(observations.length >= 2, 'Initial and final observations required');
  assert.equal(observations[0].event, 'started');
  assert.equal(observations.at(-1).event, 'observer_finished');
  assert.equal(payload.progressEmission.state, 'finished');
  assert.equal(payload.progressEmission.observationCount, observations.length);
  assert.equal(payload.progressEmission.buildIncluded, false);
  assert.ok(observations.every((event, index) => event.sequence === index));
  if (success.elapsedMs > 2000) {
    assert.ok(success.chunks.some(chunk =>
      chunk.stream === 'stderr' && chunk.elapsedMs < success.elapsedMs - 1000),
    'Observations must precede the final second of a long apply');
    assert.ok(observations.some(event => event.event === 'heartbeat'));
  }
  const failure = await runApply('failure', {
    ...env, AGENT_BROWSER_WORKSTATION_ROOT: join(root, 'failed-workstation'),
    AGENT_BROWSER_WORKSTATION_FAIL_AFTER: 'units-staged',
  });
  assert.equal(failure.signal, null, 'Injected failure must terminate within its bound');
  assert.equal(failure.code, 1, 'Injected apply failure must fail');
  assert.equal(JSON.parse(failure.stdout).success, false);
  assert.ok(parseObservations(failure.stderr).length > 0);
  assert.equal(sha256(readFileSync(binary)), digest, 'Pinned candidate must remain unchanged');
  console.log(JSON.stringify({ mode, observations: observations.length,
    firstObservationMs: success.chunks.find(chunk => chunk.stream === 'stderr')?.elapsedMs,
    emission: payload.progressEmission, injectedFailureExit: failure.code,
    evidenceDirectory: artifact }));
} finally {
  const cleanup = spawnSync('/usr/bin/python3', [
    join(repo, 'scripts/lib/fixture-runtime-cleanup.py'), home, runId, digest,
  ], { encoding: 'utf8', timeout: 15000, killSignal: 'SIGKILL' });
  writeFileSync(join(artifact, 'cleanup.json'), cleanup.stdout ||
    JSON.stringify({ status: cleanup.status, error: cleanup.stderr }));
  assert.equal(cleanup.status, 0, `Exact cleanup failed; retain ${root}: ${cleanup.stderr}`);
  assert.deepEqual(JSON.parse(cleanup.stdout).residue, []);
  makeWritable(root);
  rmSync(root, { recursive: true });
}

async function runApply(label, applyEnv) {
  const started = performance.now();
  const chunks = [];
  let stdout = '';
  let stderr = '';
  const child = spawn(binary, command, { cwd: root, env: applyEnv });
  const timer = setTimeout(() => child.kill('SIGKILL'), 180000);
  child.stdout.on('data', bytes => {
    stdout += bytes;
    chunks.push({ stream: 'stdout', elapsedMs: performance.now() - started, bytes: bytes.length });
  });
  child.stderr.on('data', bytes => {
    stderr += bytes;
    chunks.push({ stream: 'stderr', elapsedMs: performance.now() - started, bytes: bytes.length });
  });
  try {
    const terminal = await new Promise((resolve, reject) => {
      child.on('error', reject);
      child.on('close', (code, signal) => resolve({ code, signal }));
    });
    const observation = { ...terminal, elapsedMs: performance.now() - started, chunks };
    writeFileSync(join(artifact, `${label}-stdout.json`), stdout);
    writeFileSync(join(artifact, `${label}-stderr.jsonl`), stderr);
    writeEvidence(`${label}-observation.json`, observation);
    assert.equal(terminal.signal, null, `${label} exceeded its bound`);
    return { ...observation, stdout, stderr };
  } finally {
    clearTimeout(timer);
  }
}

function parseObservations(stderr) {
  assert.ok(!stderr.includes(root), 'Observations must omit private paths');
  const events = stderr.trim().split('\n').filter(Boolean).map(line => JSON.parse(line));
  assert.ok(events.every(event =>
    event.schemaVersion === 'agent-browser.workstation-install-observation.v1'));
  return events;
}

function writeEvidence(name, value) {
  writeFileSync(join(artifact, name), `${JSON.stringify(value, null, 2)}\n`);
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function makeWritable(path) {
  const stat = lstatSync(path);
  if (stat.isSymbolicLink()) return;
  if (stat.isDirectory()) {
    chmodSync(path, 0o700);
    for (const name of readdirSync(path)) makeWritable(join(path, name));
  } else chmodSync(path, stat.mode | 0o200);
}

function candidateCommand() {
  // These fixture claims exercise the public seal/custody path. They do not
  // describe the supplied binary's build profile or source qualification.
  const closure = {
    schemaVersion: 'agent-browser.executable-input-closure.v1',
    context: {
      target: 'x86_64-unknown-linux-gnu', toolchain: 'rustc 1.90.0', cargoProfile: 'release',
      resolvedBuildProfile: { optLevel: '3', lto: 'fat', codegenUnits: 1, strip: true },
      features: ['service'], reviewedEnvironmentInputs: {},
    },
    inputs: [
      { path: 'cli/src/main.rs', sha256: 'a'.repeat(64), category: 'rust_source' },
      { path: 'packages/dashboard/out/index.html', sha256: 'b'.repeat(64), category: 'embedded_dashboard' },
      { path: 'scripts/install-agent-browser-privileges.sh', sha256: 'c'.repeat(64), category: 'embedded_asset' },
    ],
  };
  const support = createBuildSupportManifest(closure);
  const supportDigest = 'd'.repeat(64);
  const manifest = createCandidateManifest({ closure,
    source: { commit: 'e'.repeat(40), tree: 'f'.repeat(64), state: 'clean' },
    artifactClass: 'production_shaped', binarySha256: digest,
    supportManifestSha256: supportDigest, createdAt: new Date().toISOString(),
    validationReceipts: ['receipt://synthetic/progress-fixture-not-production-qualification'],
  });
  const manifestBytes = `${JSON.stringify(manifest, null, 2)}\n`;
  const identity = {
    schemaVersion: 'agent-browser.candidate-build-identity.v1',
    executableInputSha256: support.executableInputSha256, artifactClass: 'production_shaped',
    target: closure.context.target, toolchain: closure.context.toolchain,
    cargoProfile: closure.context.cargoProfile, features: closure.context.features,
    reviewedEnvironmentInputSha256: support.reviewedEnvironmentInputSha256,
    resolvedBuildProfile: closure.context.resolvedBuildProfile,
    resolvedBuildProfileSha256: support.resolvedBuildProfileSha256,
  };
  const schemaVersion = 'agent-browser.sealed-artifact.v1';
  const operationId = `synthetic-progress-${runId}`;
  const manifestDigest = sha256(manifestBytes);
  const sealed = { schemaVersion, operationId, identity, binarySha256: digest,
    supportManifestSha256: supportDigest, candidateManifestSha256: manifestDigest,
    sealSha256: sha256(JSON.stringify([
      schemaVersion, operationId, identity, digest, supportDigest, manifestDigest,
    ])),
  };
  const manifestPath = join(root, 'manifest.json');
  const closurePath = join(root, 'closure.json');
  const sealPath = join(root, 'seal.json');
  writeFileSync(manifestPath, manifestBytes);
  writeFileSync(closurePath, JSON.stringify(closure));
  writeFileSync(sealPath, JSON.stringify(sealed));
  return ['--json', 'candidate', 'install', '--binary', binary,
    '--manifest', manifestPath, '--input-closure', closurePath,
    '--sealed-artifact', sealPath, '--apply', '--progress-json'];
}
