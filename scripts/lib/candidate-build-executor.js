import { createHash } from 'node:crypto';
import { resolve } from 'node:path';

const SHA256 = /^[a-f0-9]{64}$/u;
const GIT_COMMIT = /^[a-f0-9]{40}$/u;
const ARTIFACT_CLASSES = new Set(['fast_iteration', 'production_shaped']);
const SOURCE_STATES = new Set(['clean', 'dirty', 'unknown']);

function fail(code, detail) {
  const error = new Error(`${code}:${detail}`);
  error.code = code;
  throw error;
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, child]) => [key, stableValue(child)]),
    );
  }
  return value;
}

function digestJson(value) {
  return createHash('sha256')
    .update(JSON.stringify(stableValue(value)))
    .digest('hex');
}

function nonEmptyString(value, code, field) {
  if (typeof value !== 'string' || !value.trim()) fail(code, field);
  return value;
}

function normalizedFeatures(features) {
  if (!Array.isArray(features) || features.some((feature) => (
    typeof feature !== 'string' || !feature.trim()
  ))) {
    fail('candidate_build_features_invalid', 'features');
  }
  return [...new Set(features)].sort();
}

function normalizedEnvironmentInputs(inputs) {
  if (!inputs || typeof inputs !== 'object' || Array.isArray(inputs)) {
    fail('candidate_build_environment_invalid', 'reviewedEnvironmentInputs');
  }
  const entries = Object.entries(inputs).sort(([left], [right]) => left.localeCompare(right));
  for (const [name, digest] of entries) {
    if (!name.trim() || !SHA256.test(digest)) {
      fail('candidate_build_environment_digest_invalid', name || 'empty');
    }
  }
  return Object.fromEntries(entries);
}

function sourceIdentity(source) {
  if (!source || typeof source !== 'object') {
    fail('candidate_build_source_invalid', 'source');
  }
  if (!GIT_COMMIT.test(source.commit ?? '')) {
    fail('candidate_build_source_invalid', 'commit');
  }
  if (!SHA256.test(source.tree ?? '')) {
    fail('candidate_build_source_invalid', 'tree');
  }
  if (!SOURCE_STATES.has(source.state)) {
    fail('candidate_build_source_invalid', 'state');
  }
  return {
    commit: source.commit,
    tree: source.tree,
    state: source.state,
  };
}

function buildProfile(artifactClass) {
  if (artifactClass === 'production_shaped') {
    return {
      cargoProfile: 'release',
      resolvedBuildProfile: {
        optLevel: '3',
        lto: 'fat',
        codegenUnits: 1,
        strip: true,
      },
    };
  }
  return {
    cargoProfile: 'ci',
    resolvedBuildProfile: {
      optLevel: '3',
      lto: 'thin',
      codegenUnits: 16,
      strip: true,
    },
  };
}

function command(program, args, cwd, outputDirectory) {
  return {
    program,
    args,
    cwd,
    env: {
      CARGO_TARGET_DIR: outputDirectory,
    },
  };
}

/**
 * Produce a deterministic, zero-effect build recommendation. The request
 * digest identifies a build claim; the post-build executable-input collector
 * seals the exact candidate identity from compiler dep-info and artifact bytes.
 */
export function createCandidateBuildPlan(input) {
  if (!input || typeof input !== 'object') {
    fail('candidate_build_request_invalid', 'input');
  }
  const repoRoot = resolve(nonEmptyString(
    input.repoRoot,
    'candidate_build_request_invalid',
    'repoRoot',
  ));
  if (!ARTIFACT_CLASSES.has(input.artifactClass)) {
    fail('candidate_build_artifact_class_invalid', input.artifactClass ?? 'missing');
  }
  if (typeof input.apply !== 'boolean') {
    fail('candidate_build_apply_invalid', 'apply');
  }

  const source = sourceIdentity(input.source);
  if (input.artifactClass === 'production_shaped' && source.state !== 'clean') {
    fail('candidate_production_source_dirty', source.state);
  }
  const target = nonEmptyString(input.target, 'candidate_build_request_invalid', 'target');
  const toolchain = nonEmptyString(
    input.toolchain,
    'candidate_build_request_invalid',
    'toolchain',
  );
  const features = normalizedFeatures(input.features ?? []);
  const reviewedEnvironmentInputs = normalizedEnvironmentInputs(
    input.reviewedEnvironmentInputs ?? {},
  );
  const profile = buildProfile(input.artifactClass);
  const requestIdentity = {
    schemaVersion: 'agent-browser.candidate-build-request.v1',
    artifactClass: input.artifactClass,
    source,
    target,
    toolchain,
    cargoProfile: profile.cargoProfile,
    resolvedBuildProfile: profile.resolvedBuildProfile,
    features,
    reviewedEnvironmentInputs,
  };
  const requestDigest = digestJson(requestIdentity);
  const outputDirectory = resolve(
    repoRoot,
    'cli/target/candidate-builds',
    requestDigest.slice(0, 32),
  );
  const cargoArgs = [
    'build',
    ...(profile.cargoProfile === 'release' ? ['--release'] : ['--profile', 'ci']),
    '--manifest-path',
    'cli/Cargo.toml',
    '--target',
    target,
    ...(features.length > 0 ? ['--features', features.join(',')] : []),
  ];
  const commands = [command('pnpm', ['version:sync'], repoRoot, outputDirectory)];
  if (input.artifactClass === 'production_shaped') {
    commands.push(command('pnpm', ['build:dashboard'], repoRoot, outputDirectory));
  }
  commands.push(command(
    'scripts/ci/cargo-safe.sh',
    cargoArgs,
    repoRoot,
    outputDirectory,
  ));

  const plan = {
    ...requestIdentity,
    schemaVersion: 'agent-browser.candidate-build-plan.v1',
    apply: input.apply,
    requestDigest,
    outputDirectory,
    commands,
  };
  return {
    ...plan,
    planDigest: digestJson(plan),
  };
}

function requireAdapter(adapter, method) {
  if (typeof adapter?.[method] !== 'function') {
    fail('candidate_build_adapter_missing', method);
  }
  return adapter[method].bind(adapter);
}

/**
 * Execute an already-derived plan through an injected effect adapter. A dry
 * run returns before observing or mutating any external state.
 */
export async function executeCandidateBuildPlan(plan, adapter = {}) {
  if (plan?.schemaVersion !== 'agent-browser.candidate-build-plan.v1') {
    fail('candidate_build_plan_invalid', plan?.schemaVersion ?? 'missing');
  }
  const { planDigest, ...planBody } = plan;
  if (!SHA256.test(planDigest ?? '') || digestJson(planBody) !== planDigest) {
    fail('candidate_build_plan_tampered', plan?.requestDigest ?? 'missing');
  }
  if (!plan.apply) {
    return {
      outcome: 'build_recommended',
      requestDigest: plan.requestDigest,
      outputDirectory: plan.outputDirectory,
      commands: plan.commands,
      consequences: ['no_effect_performed'],
    };
  }

  const lookupSealed = requireAdapter(adapter, 'lookupSealed');
  const sealed = await lookupSealed(plan);
  if (sealed) {
    return {
      outcome: 'reused_sealed',
      requestDigest: plan.requestDigest,
      candidate: sealed,
      consequences: ['no_build_performed', 'sealed_candidate_reused'],
    };
  }

  const acquireClaim = requireAdapter(adapter, 'acquireClaim');
  const runCommand = requireAdapter(adapter, 'runCommand');
  const sealCandidate = requireAdapter(adapter, 'sealCandidate');
  const completeClaim = requireAdapter(adapter, 'completeClaim');
  const failClaim = requireAdapter(adapter, 'failClaim');
  const claim = await acquireClaim(plan);
  if (!claim || typeof claim.acquired !== 'boolean' || !claim.operationId) {
    fail('candidate_build_claim_invalid', plan.requestDigest);
  }
  if (!claim.acquired) {
    return {
      outcome: 'joined_existing',
      requestDigest: plan.requestDigest,
      operationId: claim.operationId,
      claimPath: claim.claimPath ?? null,
      consequences: ['no_duplicate_build_performed'],
    };
  }

  try {
    for (const commandSpec of plan.commands) await runCommand(commandSpec, plan, claim);
    const candidate = await sealCandidate(plan, claim);
    if (!candidate?.candidateId) {
      fail('candidate_build_seal_invalid', plan.requestDigest);
    }
    await completeClaim(plan, candidate, claim);
    return {
      outcome: 'artifact_sealed',
      requestDigest: plan.requestDigest,
      operationId: claim.operationId,
      candidate,
      consequences: ['build_performed', 'candidate_sealed'],
    };
  } catch (error) {
    await failClaim(plan, error, claim);
    throw error;
  }
}
