#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  createCandidateBuildPlan,
  executeCandidateBuildPlan,
} from './lib/candidate-build-executor.js';
import { createCandidateBuildFilesystemAdapter } from './lib/candidate-build-filesystem-adapter.js';

const SHA256 = /^[a-f0-9]{64}$/u;

function fail(code, detail) {
  const error = new Error(`${code}:${detail}`);
  error.code = code;
  throw error;
}

function command(program, args, cwd, encoding = 'utf8') {
  return execFileSync(program, args, { cwd, encoding });
}

export function readCandidateBuildSource(repoRoot) {
  const commit = command('git', ['rev-parse', 'HEAD'], repoRoot).trim();
  const treeBytes = command('git', ['cat-file', 'tree', 'HEAD^{tree}'], repoRoot, null);
  const tree = createHash('sha256').update(treeBytes).digest('hex');
  const status = command('git', ['status', '--porcelain'], repoRoot).trim();
  return {
    commit,
    tree,
    state: status ? 'dirty' : 'clean',
  };
}

function toolchainAndHost(repoRoot) {
  const toolchain = command('rustc', ['--version', '--verbose'], repoRoot).trim();
  const host = toolchain.split(/\r?\n/u)
    .find((line) => line.startsWith('host: '))
    ?.slice('host: '.length);
  if (!host) fail('candidate_build_host_target_missing', 'rustc --version --verbose');
  return { toolchain, host };
}

function sourceControlRoots(repoRoot) {
  const resolveGitPath = (argument) => {
    const value = command('git', ['rev-parse', argument], repoRoot).trim();
    return realpathSync(resolve(repoRoot, value));
  };
  return {
    common: resolveGitPath('--git-common-dir'),
    worktree: resolveGitPath('--git-dir'),
  };
}

export function parseCandidateBuildArguments(argv) {
  let repoRoot = null;
  let artifactClass = null;
  let target = null;
  let mode = null;
  let json = false;
  let retryFailedOperationId = null;
  let recoverActiveOperationId = null;
  const features = [];
  const reviewedEnvironmentInputs = {};
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const take = (name) => {
      const value = argv[index + 1];
      if (!value?.trim()) fail('candidate_build_argument_missing', name);
      index += 1;
      return value;
    };
    switch (argument) {
      case '--repo-root': repoRoot = take(argument); break;
      case '--artifact-class': artifactClass = take(argument); break;
      case '--target': target = take(argument); break;
      case '--feature': features.push(take(argument)); break;
      case '--reviewed-environment-input': {
        const value = take(argument);
        const separator = value.indexOf('=');
        const name = value.slice(0, separator);
        const digest = value.slice(separator + 1);
        if (separator < 1 || !SHA256.test(digest)) {
          fail('candidate_build_environment_digest_invalid', value);
        }
        if (Object.hasOwn(reviewedEnvironmentInputs, name)) {
          fail('candidate_build_environment_duplicate', name);
        }
        reviewedEnvironmentInputs[name] = digest;
        break;
      }
      case '--retry-failed-operation':
        if (retryFailedOperationId) fail('candidate_build_recovery_duplicate', argument);
        retryFailedOperationId = take(argument);
        break;
      case '--recover-active-operation':
        if (recoverActiveOperationId) fail('candidate_build_recovery_duplicate', argument);
        recoverActiveOperationId = take(argument);
        break;
      case '--dry-run':
      case '--apply':
        if (mode) fail('candidate_build_mode_duplicate', `${mode},${argument}`);
        mode = argument;
        break;
      case '--json': json = true; break;
      default: fail('candidate_build_argument_unknown', argument);
    }
  }
  if (!repoRoot) fail('candidate_build_argument_missing', '--repo-root');
  if (!['fast_iteration', 'production_shaped'].includes(artifactClass)) {
    fail('candidate_build_artifact_class_invalid', artifactClass ?? 'missing');
  }
  if (!mode) fail('candidate_build_argument_missing', '--dry-run or --apply');
  if (retryFailedOperationId && mode !== '--apply') {
    fail('candidate_build_recovery_requires_apply', retryFailedOperationId);
  }
  if (recoverActiveOperationId && mode !== '--apply') {
    fail('candidate_build_recovery_requires_apply', recoverActiveOperationId);
  }
  if (retryFailedOperationId && recoverActiveOperationId) {
    fail('candidate_build_recovery_conflict', `${retryFailedOperationId},${recoverActiveOperationId}`);
  }
  return {
    repoRoot: realpathSync(resolve(repoRoot)),
    artifactClass,
    target,
    features,
    reviewedEnvironmentInputs,
    apply: mode === '--apply',
    json,
    retryFailedOperationId,
    recoverActiveOperationId,
  };
}

export async function runCandidateBuild(argv, dependencies = {}) {
  const parsed = parseCandidateBuildArguments(argv);
  const readSource = dependencies.readSource ?? readCandidateBuildSource;
  const source = await readSource(parsed.repoRoot);
  const detected = dependencies.toolchainAndHost
    ? await dependencies.toolchainAndHost(parsed.repoRoot)
    : toolchainAndHost(parsed.repoRoot);
  const environment = dependencies.environment ?? process.env;
  for (const [name, expectedDigest] of Object.entries(parsed.reviewedEnvironmentInputs)) {
    const value = environment[name];
    if (typeof value !== 'string') {
      fail('candidate_build_environment_missing', name);
    }
    const observedDigest = createHash('sha256').update(value).digest('hex');
    if (observedDigest !== expectedDigest) {
      fail('candidate_build_environment_changed', name);
    }
  }
  const plan = createCandidateBuildPlan({
    repoRoot: parsed.repoRoot,
    artifactClass: parsed.artifactClass,
    apply: parsed.apply,
    source,
    target: parsed.target ?? detected.host,
    toolchain: detected.toolchain,
    features: parsed.features,
    reviewedEnvironmentInputs: parsed.reviewedEnvironmentInputs,
  });
  const adapter = dependencies.adapter ?? createCandidateBuildFilesystemAdapter({
    repoRoot: parsed.repoRoot,
    sourceReader: readSource,
    sourceControlRoots: dependencies.sourceControlRoots
      ?? sourceControlRoots(parsed.repoRoot),
    retryFailedOperationId: parsed.retryFailedOperationId,
    recoverActiveOperationId: parsed.recoverActiveOperationId,
  });
  return {
    schemaVersion: 'agent-browser.candidate-build-result.v1',
    success: true,
    plan,
    result: await executeCandidateBuildPlan(plan, adapter),
  };
}

async function main() {
  const args = process.argv.slice(2).filter((argument) => argument !== '--');
  const json = args.includes('--json');
  try {
    const report = await runCandidateBuild(args);
    if (json) console.log(JSON.stringify(report, null, 2));
    else {
      console.log(`Candidate build outcome: ${report.result.outcome}`);
      console.log(`Request: ${report.result.requestDigest}`);
      console.log(`Output: ${report.result.outputDirectory ?? report.plan.outputDirectory}`);
      if (report.result.candidate?.candidateId) {
        console.log(`Candidate: ${report.result.candidate.candidateId}`);
      }
    }
  } catch (error) {
    if (json) console.log(JSON.stringify({ success: false, error: error.message }));
    else console.error(`Candidate build failed: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  await main();
}
