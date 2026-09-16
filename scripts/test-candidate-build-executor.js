#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  createCandidateBuildPlan,
  executeCandidateBuildPlan,
} from './lib/candidate-build-executor.js';

function input(overrides = {}) {
  return {
    repoRoot: '/workspace/agent-browser',
    artifactClass: 'production_shaped',
    apply: false,
    source: {
      commit: 'a'.repeat(40),
      tree: 'b'.repeat(64),
      state: 'clean',
    },
    target: 'x86_64-unknown-linux-gnu',
    toolchain: 'rustc 1.90.0',
    features: ['service'],
    reviewedEnvironmentInputs: {},
    ...overrides,
  };
}

const production = createCandidateBuildPlan(input());
assert.equal(production.artifactClass, 'production_shaped');
assert.equal(production.cargoProfile, 'release');
assert.deepEqual(production.resolvedBuildProfile, {
  optLevel: '3',
  lto: 'fat',
  codegenUnits: 1,
  strip: true,
});
assert.match(production.requestDigest, /^[a-f0-9]{64}$/u);
assert.match(production.planDigest, /^[a-f0-9]{64}$/u);
assert.match(production.outputDirectory, /candidate-builds\/[a-f0-9]{32}$/u);
assert.deepEqual(production.commands.map((command) => command.program), [
  'pnpm',
  'pnpm',
  'scripts/ci/cargo-safe.sh',
]);
assert.ok(production.commands.at(-1).args.includes('--release'));

const reordered = createCandidateBuildPlan(input({
  apply: true,
  features: ['trace', 'service', 'trace'],
  reviewedEnvironmentInputs: {
    Z_INPUT: 'd'.repeat(64),
    A_INPUT: 'e'.repeat(64),
  },
}));
const canonical = createCandidateBuildPlan(input({
  features: ['service', 'trace'],
  reviewedEnvironmentInputs: {
    A_INPUT: 'e'.repeat(64),
    Z_INPUT: 'd'.repeat(64),
  },
}));
assert.equal(reordered.requestDigest, canonical.requestDigest);
assert.deepEqual(reordered.features, ['service', 'trace']);
assert.notEqual(reordered.planDigest, canonical.planDigest);

assert.throws(
  () => createCandidateBuildPlan(input({ source: { ...input().source, state: 'dirty' } })),
  /candidate_production_source_dirty/u,
);

const fast = createCandidateBuildPlan(input({
  artifactClass: 'fast_iteration',
  apply: true,
  source: { ...input().source, state: 'dirty' },
}));
assert.equal(fast.cargoProfile, 'ci');
assert.equal(fast.resolvedBuildProfile.lto, 'thin');
assert.ok(fast.commands.at(-1).args.includes('ci'));
assert.equal(fast.commands.some((command) => command.args.includes('build')), true);
assert.equal(
  fast.commands.some((command) => command.args.includes('packages/dashboard')),
  false,
);

const dryRunCalls = [];
const dryRun = await executeCandidateBuildPlan(production, {
  lookupSealed: async () => { dryRunCalls.push('lookup'); },
});
assert.equal(dryRun.outcome, 'build_recommended');
assert.deepEqual(dryRunCalls, []);
assert.equal(dryRun.consequences.includes('no_effect_performed'), true);

const tampered = structuredClone(fast);
tampered.commands.at(-1).program = 'cargo';
await assert.rejects(
  executeCandidateBuildPlan(tampered, {
    lookupSealed: async () => { dryRunCalls.push('tampered-lookup'); },
  }),
  /candidate_build_plan_tampered/u,
);
assert.deepEqual(dryRunCalls, []);

let incompleteAdapterClaimed = false;
let incompleteAdapterRan = false;
await assert.rejects(
  executeCandidateBuildPlan(fast, {
    lookupSealed: async () => null,
    acquireClaim: async () => {
      incompleteAdapterClaimed = true;
      return { acquired: true, operationId: 'operation-incomplete' };
    },
    runCommand: async () => { incompleteAdapterRan = true; },
    failClaim: async () => {},
  }),
  /candidate_build_adapter_missing:sealCandidate/u,
);
assert.equal(incompleteAdapterClaimed, false);
assert.equal(incompleteAdapterRan, false);

const calls = [];
const applied = await executeCandidateBuildPlan(fast, {
  lookupSealed: async () => null,
  acquireClaim: async () => ({ acquired: true, operationId: 'operation-1' }),
  runCommand: async (command) => { calls.push(['run', command.program]); },
  sealCandidate: async () => ({
    candidateId: 'candidate-1',
    candidateManifestPath: '/tmp/candidate-manifest.json',
    binarySha256: 'c'.repeat(64),
  }),
  completeClaim: async (_plan, sealed) => { calls.push(['complete', sealed.candidateId]); },
  failClaim: async () => { calls.push(['fail']); },
});
assert.equal(applied.outcome, 'artifact_sealed');
assert.equal(applied.operationId, 'operation-1');
assert.deepEqual(calls, [
  ['run', 'pnpm'],
  ['run', 'scripts/ci/cargo-safe.sh'],
  ['complete', 'candidate-1'],
]);

const reused = await executeCandidateBuildPlan(fast, {
  lookupSealed: async () => ({ candidateId: 'candidate-existing' }),
});
assert.equal(reused.outcome, 'reused_sealed');
assert.equal(reused.candidate.candidateId, 'candidate-existing');

const joined = await executeCandidateBuildPlan(fast, {
  lookupSealed: async () => null,
  acquireClaim: async () => ({
    acquired: false,
    operationId: 'operation-active',
    claimPath: '/tmp/claim.json',
  }),
  runCommand: async () => {},
  sealCandidate: async () => {},
  completeClaim: async () => {},
  failClaim: async () => {},
});
assert.equal(joined.outcome, 'joined_existing');
assert.equal(joined.operationId, 'operation-active');

let failedReceipt = null;
await assert.rejects(
  executeCandidateBuildPlan(fast, {
    lookupSealed: async () => null,
    acquireClaim: async () => ({ acquired: true, operationId: 'operation-failed' }),
    runCommand: async () => { throw new Error('compile failed'); },
    sealCandidate: async () => {},
    completeClaim: async () => {},
    failClaim: async (_plan, error) => { failedReceipt = error.message; },
  }),
  /compile failed/u,
);
assert.equal(failedReceipt, 'compile failed');

console.log('Candidate build executor tests passed');
