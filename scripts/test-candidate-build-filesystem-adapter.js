#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import {
  createCandidateBuildPlan,
  executeCandidateBuildPlan,
} from './lib/candidate-build-executor.js';
import {
  candidateBuildArtifactPaths,
  createCandidateBuildFilesystemAdapter,
} from './lib/candidate-build-filesystem-adapter.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-candidate-build-adapter-'));

function write(path, body) {
  const target = join(root, path);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, body);
  return target;
}

function request(apply = true) {
  return createCandidateBuildPlan({
    repoRoot: root,
    artifactClass: 'fast_iteration',
    apply,
    source: {
      commit: 'a'.repeat(40),
      tree: 'b'.repeat(64),
      state: 'dirty',
    },
    target: 'x86_64-unknown-linux-gnu',
    toolchain: 'rustc 1.90.0',
    features: [],
    reviewedEnvironmentInputs: {},
  });
}

try {
  write('Cargo.toml', '[workspace]\n');
  write('Cargo.lock', '# lock\n');
  write('cli/Cargo.toml', '[package]\nname = "agent-browser"\n');
  write('cli/src/main.rs', 'fn main() {}\n');
  write('package.json', '{"version":"0.28.0"}\n');
  write('packages/dashboard/package.json', '{"version":"0.28.0"}\n');

  const plan = request();
  let commandCount = 0;
  const runner = async (command) => {
    commandCount += 1;
    if (command.program !== 'scripts/ci/cargo-safe.sh') return;
    const paths = candidateBuildArtifactPaths(plan);
    mkdirSync(dirname(paths.binary), { recursive: true });
    writeFileSync(paths.binary, 'candidate-binary');
    writeFileSync(
      paths.depInfo,
      `${paths.binary}: cli/src/main.rs Cargo.toml Cargo.lock cli/Cargo.toml package.json packages/dashboard/package.json\n`,
    );
  };
  const adapter = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    commandRunner: runner,
    clock: () => '2026-09-16T12:00:00Z',
    operationIdFactory: () => 'build-operation-1',
  });

  const [first, joined] = await Promise.all([
    executeCandidateBuildPlan(plan, adapter),
    executeCandidateBuildPlan(plan, adapter),
  ]);
  assert.equal(first.outcome, 'artifact_sealed');
  assert.equal(joined.outcome, 'joined_existing');
  assert.equal(commandCount, 2);
  assert.equal(first.candidate.binarySha256.length, 64);
  assert.equal(readFileSync(first.candidate.binaryPath, 'utf8'), 'candidate-binary');

  const sealed = JSON.parse(readFileSync(first.candidate.sealedArtifactPath, 'utf8'));
  assert.equal(sealed.schemaVersion, 'agent-browser.sealed-artifact.v1');
  assert.equal(sealed.operationId, 'build-operation-1');
  assert.equal(sealed.sealSha256.length, 64);
  const manifest = JSON.parse(readFileSync(first.candidate.candidateManifestPath, 'utf8'));
  assert.equal(manifest.candidateId, first.candidate.candidateId);
  assert.equal(manifest.binarySha256, first.candidate.binarySha256);

  const reused = await executeCandidateBuildPlan(plan, adapter);
  assert.equal(reused.outcome, 'reused_sealed');
  assert.equal(reused.candidate.candidateId, first.candidate.candidateId);
  assert.equal(commandCount, 2);

  writeFileSync(first.candidate.binaryPath, 'tampered-binary');
  await assert.rejects(
    executeCandidateBuildPlan(plan, adapter),
    /candidate_build_reused_artifact_tampered/u,
  );
  writeFileSync(first.candidate.binaryPath, 'candidate-binary');

  const dryRun = await executeCandidateBuildPlan(request(false), adapter);
  assert.equal(dryRun.outcome, 'build_recommended');
  assert.equal(commandCount, 2);

  const failedPlan = createCandidateBuildPlan({
    repoRoot: root,
    artifactClass: 'fast_iteration',
    source: { commit: 'c'.repeat(40), tree: 'd'.repeat(64), state: 'dirty' },
    target: 'x86_64-unknown-linux-gnu',
    toolchain: 'rustc 1.90.0',
    features: [],
    reviewedEnvironmentInputs: {},
    apply: true,
  });
  const failedAdapter = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    commandRunner: async () => { throw new Error('fixture compile failed'); },
    operationIdFactory: () => 'build-operation-failed',
  });
  await assert.rejects(
    executeCandidateBuildPlan(failedPlan, failedAdapter),
    /fixture compile failed/u,
  );
  const failure = JSON.parse(readFileSync(
    join(root, 'cli/target/candidate-build-state/failed/build-operation-failed.json'),
    'utf8',
  ));
  assert.equal(failure.error, 'fixture compile failed');
  await assert.rejects(
    executeCandidateBuildPlan(failedPlan, failedAdapter),
    /candidate_build_failed_recovery_required/u,
  );
  const recoveredAdapter = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    retryFailedOperationId: 'build-operation-failed',
    operationIdFactory: () => 'build-operation-retry',
    commandRunner: async (command) => {
      if (command.program !== 'scripts/ci/cargo-safe.sh') return;
      const paths = candidateBuildArtifactPaths(failedPlan);
      mkdirSync(dirname(paths.binary), { recursive: true });
      writeFileSync(paths.binary, 'recovered-candidate-binary');
      writeFileSync(
        paths.depInfo,
        `${paths.binary}: cli/src/main.rs Cargo.toml Cargo.lock cli/Cargo.toml package.json packages/dashboard/package.json\n`,
      );
    },
  });
  const recovered = await executeCandidateBuildPlan(failedPlan, recoveredAdapter);
  assert.equal(recovered.outcome, 'artifact_sealed');
  assert.equal(recovered.operationId, 'build-operation-retry');
  const archivedClaim = JSON.parse(readFileSync(
    join(root, 'cli/target/candidate-build-state/failed/claims/build-operation-failed.json'),
    'utf8',
  ));
  assert.equal(archivedClaim.state, 'failed');

  const abandonedPlan = createCandidateBuildPlan({
    repoRoot: root,
    artifactClass: 'fast_iteration',
    source: { commit: 'e'.repeat(40), tree: 'f'.repeat(64), state: 'dirty' },
    target: 'x86_64-unknown-linux-gnu',
    toolchain: 'rustc 1.90.0',
    features: [],
    reviewedEnvironmentInputs: {},
    apply: true,
  });
  const abandonedOwner = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    ownerPid: 424242,
    operationIdFactory: () => 'build-operation-abandoned',
  });
  const claim = await abandonedOwner.acquireClaim(abandonedPlan);
  assert.equal(claim.acquired, true);
  const refusedActive = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    recoverActiveOperationId: 'build-operation-abandoned',
    processAlive: () => true,
  });
  await assert.rejects(
    executeCandidateBuildPlan(abandonedPlan, refusedActive),
    /candidate_build_active_owner_alive/u,
  );
  const recoveredActive = createCandidateBuildFilesystemAdapter({
    repoRoot: root,
    ownerPid: 434343,
    recoverActiveOperationId: 'build-operation-abandoned',
    processAlive: () => false,
    operationIdFactory: () => 'build-operation-after-abandonment',
    commandRunner: async (command) => {
      if (command.program !== 'scripts/ci/cargo-safe.sh') return;
      const paths = candidateBuildArtifactPaths(abandonedPlan);
      mkdirSync(dirname(paths.binary), { recursive: true });
      writeFileSync(paths.binary, 'abandoned-recovery-binary');
      writeFileSync(
        paths.depInfo,
        `${paths.binary}: cli/src/main.rs Cargo.toml Cargo.lock cli/Cargo.toml package.json packages/dashboard/package.json\n`,
      );
    },
  });
  const activeRecovery = await executeCandidateBuildPlan(abandonedPlan, recoveredActive);
  assert.equal(activeRecovery.outcome, 'artifact_sealed');
  const archivedActive = JSON.parse(readFileSync(
    join(root, 'cli/target/candidate-build-state/abandoned/claims/build-operation-abandoned.json'),
    'utf8',
  ));
  assert.equal(archivedActive.ownerPid, 424242);
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Candidate build filesystem adapter tests passed');
