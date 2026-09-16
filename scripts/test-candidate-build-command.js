#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseCandidateBuildArguments, runCandidateBuild } from './candidate-build.js';

const root = mkdtempSync(join(tmpdir(), 'agent-browser-candidate-build-command-'));
try {
  const parsed = parseCandidateBuildArguments([
    '--repo-root', root,
    '--artifact-class', 'production_shaped',
    '--target', 'x86_64-unknown-linux-gnu',
    '--feature', 'service',
    '--feature', 'trace',
    '--reviewed-environment-input', `BUILD_MODE=${'a'.repeat(64)}`,
    '--retry-failed-operation', 'build-failed-1',
    '--apply',
    '--json',
  ]);
  assert.equal(parsed.repoRoot, root);
  assert.equal(parsed.artifactClass, 'production_shaped');
  assert.equal(parsed.apply, true);
  assert.deepEqual(parsed.features, ['service', 'trace']);
  assert.equal(parsed.retryFailedOperationId, 'build-failed-1');

  const report = await runCandidateBuild([
    '--repo-root', root,
    '--artifact-class', 'fast_iteration',
    '--dry-run',
  ], {
    readSource: async () => ({
      commit: 'a'.repeat(40),
      tree: 'b'.repeat(64),
      state: 'dirty',
    }),
    toolchainAndHost: async () => ({
      toolchain: 'rustc 1.90.0',
      host: 'x86_64-unknown-linux-gnu',
    }),
    adapter: {},
  });
  assert.equal(report.success, true);
  assert.equal(report.result.outcome, 'build_recommended');
  assert.equal(report.plan.apply, false);

  assert.throws(
    () => parseCandidateBuildArguments([
      '--repo-root', root,
      '--artifact-class', 'fast_iteration',
      '--dry-run',
      '--apply',
    ]),
    /candidate_build_mode_duplicate/u,
  );
  assert.throws(
    () => parseCandidateBuildArguments([
      '--repo-root', root,
      '--artifact-class', 'production_shaped',
      '--reviewed-environment-input', 'SECRET=raw-value',
      '--dry-run',
    ]),
    /candidate_build_environment_digest_invalid/u,
  );
  assert.throws(
    () => parseCandidateBuildArguments([
      '--repo-root', root,
      '--artifact-class', 'fast_iteration',
      '--retry-failed-operation', 'build-failed-1',
      '--dry-run',
    ]),
    /candidate_build_recovery_requires_apply/u,
  );
  await assert.rejects(
    runCandidateBuild([
      '--repo-root', root,
      '--artifact-class', 'fast_iteration',
      '--reviewed-environment-input', `BUILD_MODE=${'a'.repeat(64)}`,
      '--dry-run',
    ], {
      readSource: async () => ({
        commit: 'a'.repeat(40),
        tree: 'b'.repeat(64),
        state: 'dirty',
      }),
      toolchainAndHost: async () => ({
        toolchain: 'rustc 1.90.0',
        host: 'x86_64-unknown-linux-gnu',
      }),
      environment: { BUILD_MODE: 'reviewed-value' },
    }),
    /candidate_build_environment_changed/u,
  );
} finally {
  rmSync(root, { recursive: true, force: true });
}

console.log('Candidate build command tests passed');
