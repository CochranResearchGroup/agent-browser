import { createHash, randomUUID } from 'node:crypto';
import { spawn } from 'node:child_process';
import {
  closeSync,
  constants,
  copyFileSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import {
  collectExecutableInputClosure,
  createBuildSupportManifest,
  createCandidateManifest,
  digestExecutableInputClosure,
  encodeBuildSupportManifest,
} from './candidate-executable-input.js';
import { inspectWorktreeCloseout } from './worktree-closeout-filesystem-adapter.js';
import { worktreeCloseoutOperationPath } from './worktree-closeout.js';

const SEALED_ARTIFACT_SCHEMA_VERSION = 'agent-browser.sealed-artifact.v1';
const BUILD_IDENTITY_SCHEMA_VERSION = 'agent-browser.candidate-build-identity.v1';

function fail(code, detail) {
  const error = new Error(`${code}:${detail}`);
  error.code = code;
  throw error;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function encodeJson(value) {
  return Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function atomicWrite(path, bytes) {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  const descriptor = openSync(temporary, constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY, 0o600);
  try {
    writeFileSync(descriptor, bytes);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
  renameSync(temporary, path);
  syncDirectory(dirname(path));
}

function syncDirectory(path) {
  const descriptor = openSync(path, constants.O_RDONLY);
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function atomicCopy(source, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  const temporary = `${destination}.tmp-${process.pid}-${randomUUID()}`;
  copyFileSync(source, temporary, constants.COPYFILE_EXCL);
  const descriptor = openSync(temporary, constants.O_RDONLY);
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
  renameSync(temporary, destination);
  syncDirectory(dirname(destination));
}

function buildIdentity(plan, closure) {
  const support = createBuildSupportManifest(closure);
  return {
    schemaVersion: BUILD_IDENTITY_SCHEMA_VERSION,
    executableInputSha256: digestExecutableInputClosure(closure),
    artifactClass: plan.artifactClass,
    target: plan.target,
    toolchain: plan.toolchain,
    cargoProfile: plan.cargoProfile,
    features: plan.features,
    reviewedEnvironmentInputSha256: support.reviewedEnvironmentInputSha256,
    resolvedBuildProfile: plan.resolvedBuildProfile,
    resolvedBuildProfileSha256: support.resolvedBuildProfileSha256,
  };
}

function sealDigest(operationId, identity, binarySha256, supportSha256, manifestSha256) {
  return sha256(JSON.stringify([
    SEALED_ARTIFACT_SCHEMA_VERSION,
    operationId,
    identity,
    binarySha256,
    supportSha256,
    manifestSha256,
  ]));
}

function executableName(target) {
  return target.includes('windows') ? 'agent-browser.exe' : 'agent-browser';
}

function profileDirectory(plan) {
  return plan.cargoProfile === 'release' ? 'release' : plan.cargoProfile;
}

function artifactPaths(plan) {
  const profile = profileDirectory(plan);
  const buildDirectory = join(plan.outputDirectory, plan.target, profile);
  const artifactDirectory = join(plan.outputDirectory, 'sealed');
  const binary = join(buildDirectory, executableName(plan.target));
  return {
    binary,
    depInfo: `${binary}.d`,
    artifactDirectory,
    copiedBinary: join(artifactDirectory, executableName(plan.target)),
    closure: join(artifactDirectory, 'executable-input-closure.json'),
    support: join(artifactDirectory, 'build-support-manifest.json'),
    manifest: join(artifactDirectory, 'candidate-manifest.json'),
    sealed: join(artifactDirectory, 'sealed-artifact.json'),
  };
}

function defaultCommandRunner(command) {
  return new Promise((accept, reject) => {
    const child = spawn(command.program, command.args, {
      cwd: command.cwd,
      env: { ...process.env, ...command.env },
      stdio: 'inherit',
    });
    child.once('error', reject);
    child.once('exit', (code, signal) => {
      if (code === 0) accept();
      else reject(new Error(
        `candidate_build_command_failed:${command.program}:${code ?? `signal-${signal}`}`,
      ));
    });
  });
}

/**
 * Filesystem and process adapter for the provider-free candidate build planner.
 * Claims and receipts live below cli/target, never in either installed runtime.
 */
export function createCandidateBuildFilesystemAdapter({
  repoRoot,
  stateRoot = resolve(repoRoot, 'cli/target/candidate-build-state'),
  clock = () => new Date().toISOString(),
  commandRunner = defaultCommandRunner,
  operationIdFactory = () => `candidate-build-${randomUUID()}`,
  sourceReader = null,
  sourceControlRoots = {},
  retryFailedOperationId = null,
  recoverActiveOperationId = null,
  ownerPid = process.pid,
  processAlive = (pid) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch (error) {
      return error?.code === 'EPERM';
    }
  },
} = {}) {
  const root = resolve(repoRoot);
  const state = resolve(stateRoot);

  function claimPath(plan) {
    return join(state, 'claims', `${plan.requestDigest}.json`);
  }

  function completionPath(plan) {
    return join(state, 'completed', `${plan.requestDigest}.json`);
  }

  function claimDocument(plan, operationId) {
    return {
      schemaVersion: 'agent-browser.candidate-build-claim.v1',
      requestDigest: plan.requestDigest,
      operationId,
      state: 'building',
      createdAt: clock(),
      ownerPid,
      outputDirectory: plan.outputDirectory,
    };
  }

  function createClaim(path, plan, operationId) {
    const descriptor = openSync(
      path,
      constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY,
      0o600,
    );
    try {
      writeFileSync(descriptor, encodeJson(claimDocument(plan, operationId)));
      fsyncSync(descriptor);
    } finally {
      closeSync(descriptor);
    }
    syncDirectory(dirname(path));
  }

  function selectedCloseoutPath() {
    if (!existsSync(join(root, '.git'))) return null;
    const inspection = inspectWorktreeCloseout({ repositoryRoot: root, worktreePath: root });
    const path = worktreeCloseoutOperationPath({
      stateRoot: inspection.coordinationStateRoot,
      repositoryId: inspection.repositoryId,
      worktreeIncarnation: inspection.worktreeIncarnation,
    });
    if (!existsSync(path)) return null;
    const operation = readJson(path);
    const receipt = `${path}.${operation.operationId}.receipt.json`;
    return existsSync(receipt) ? null : path;
  }

  function assertNoSelectedCloseout() {
    const path = selectedCloseoutPath();
    if (path) fail('candidate_build_worktree_closeout_active', path);
  }

  return {
    async lookupSealed(plan) {
      const completed = completionPath(plan);
      if (!existsSync(completed)) return null;
      const receipt = readJson(completed);
      if (receipt.requestDigest !== plan.requestDigest || !receipt.sealedArtifactPath) {
        fail('candidate_build_completion_invalid', completed);
      }
      const paths = artifactPaths(plan);
      const expectedPaths = {
        binaryPath: paths.copiedBinary,
        candidateManifestPath: paths.manifest,
        inputClosurePath: paths.closure,
        supportManifestPath: paths.support,
        sealedArtifactPath: paths.sealed,
      };
      if (Object.entries(expectedPaths).some(([field, path]) => receipt[field] !== path)) {
        fail('candidate_build_completion_path_mismatch', completed);
      }
      const sealed = readJson(paths.sealed);
      if (sealed.operationId !== receipt.operationId) {
        fail('candidate_build_completion_operation_mismatch', completed);
      }
      if (sealed.sealSha256 !== sealDigest(
        sealed.operationId,
        sealed.identity,
        sealed.binarySha256,
        sealed.supportManifestSha256,
        sealed.candidateManifestSha256,
      )) {
        fail('candidate_build_sealed_artifact_tampered', paths.sealed);
      }
      const binaryBytes = readFileSync(paths.copiedBinary);
      const manifestBytes = readFileSync(paths.manifest);
      const supportBytes = readFileSync(paths.support);
      const closure = readJson(paths.closure);
      const manifest = JSON.parse(manifestBytes.toString('utf8'));
      const support = JSON.parse(supportBytes.toString('utf8'));
      if (
        sha256(binaryBytes) !== sealed.binarySha256
        || sha256(manifestBytes) !== sealed.candidateManifestSha256
        || sha256(supportBytes) !== sealed.supportManifestSha256
        || digestExecutableInputClosure(closure) !== sealed.identity.executableInputSha256
        || manifest.candidateId !== receipt.candidateId
        || manifest.binarySha256 !== sealed.binarySha256
        || manifest.supportManifestSha256 !== sealed.supportManifestSha256
        || manifest.executableInputSha256 !== sealed.identity.executableInputSha256
        || support.executableInputSha256 !== sealed.identity.executableInputSha256
      ) {
        fail('candidate_build_reused_artifact_tampered', receipt.candidateId ?? completed);
      }
      return {
        candidateId: receipt.candidateId,
        ...expectedPaths,
        binarySha256: sealed.binarySha256,
      };
    },

    async acquireClaim(plan) {
      const path = claimPath(plan);
      mkdirSync(dirname(path), { recursive: true });
      const operationId = operationIdFactory();
      const recoveryGuard = `${path}.recovery`;
      assertNoSelectedCloseout();
      if (existsSync(recoveryGuard)) {
        fail('candidate_build_recovery_in_progress', recoveryGuard);
      }
      try {
        createClaim(path, plan, operationId);
        try {
          assertNoSelectedCloseout();
        } catch (error) {
          const created = readJson(path);
          if (created.operationId === operationId) {
            unlinkSync(path);
            syncDirectory(dirname(path));
          }
          throw error;
        }
        return { acquired: true, operationId, claimPath: path };
      } catch (error) {
        if (error?.code !== 'EEXIST') throw error;
        const existing = readJson(path);
        if (existing.requestDigest !== plan.requestDigest || !existing.operationId) {
          fail('candidate_build_claim_invalid', path);
        }
        if (existing.state === 'failed' && retryFailedOperationId === existing.operationId) {
          let guardDescriptor;
          try {
            guardDescriptor = openSync(
              recoveryGuard,
              constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY,
              0o600,
            );
          } catch (guardError) {
            if (guardError?.code === 'EEXIST') {
              fail('candidate_build_recovery_in_progress', recoveryGuard);
            }
            throw guardError;
          }
          try {
            writeFileSync(guardDescriptor, encodeJson({
              schemaVersion: 'agent-browser.candidate-build-recovery.v1',
              requestDigest: plan.requestDigest,
              failedOperationId: existing.operationId,
              replacementOperationId: operationId,
              startedAt: clock(),
            }));
            fsyncSync(guardDescriptor);
          } finally {
            closeSync(guardDescriptor);
          }
          syncDirectory(dirname(recoveryGuard));
          const confirmed = readJson(path);
          if (
            confirmed.state !== 'failed'
            || confirmed.operationId !== retryFailedOperationId
            || confirmed.requestDigest !== plan.requestDigest
          ) {
            fail('candidate_build_failed_claim_changed', path);
          }
          const archivedClaim = join(
            state,
            'failed',
            'claims',
            `${confirmed.operationId}.json`,
          );
          mkdirSync(dirname(archivedClaim), { recursive: true });
          atomicCopy(path, archivedClaim);
          const partialArtifacts = artifactPaths(plan).artifactDirectory;
          if (existsSync(partialArtifacts)) {
            const archivedArtifacts = join(
              state,
              'failed',
              'artifacts',
              confirmed.operationId,
            );
            mkdirSync(dirname(archivedArtifacts), { recursive: true });
            renameSync(partialArtifacts, archivedArtifacts);
            syncDirectory(dirname(partialArtifacts));
            syncDirectory(dirname(archivedArtifacts));
          }
          atomicWrite(path, encodeJson(claimDocument(plan, operationId)));
          unlinkSync(recoveryGuard);
          syncDirectory(dirname(recoveryGuard));
          return {
            acquired: true,
            operationId,
            claimPath: path,
            recoveredOperationId: confirmed.operationId,
            archivedClaim,
          };
        }
        if (existing.state === 'building' && recoverActiveOperationId === existing.operationId) {
          if (!Number.isInteger(existing.ownerPid) || existing.ownerPid < 1) {
            fail('candidate_build_active_owner_invalid', existing.operationId);
          }
          if (processAlive(existing.ownerPid)) {
            fail('candidate_build_active_owner_alive', `${existing.operationId}:${existing.ownerPid}`);
          }
          const guardDescriptor = openSync(
            recoveryGuard,
            constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY,
            0o600,
          );
          try {
            writeFileSync(guardDescriptor, encodeJson({
              schemaVersion: 'agent-browser.candidate-build-recovery.v1',
              requestDigest: plan.requestDigest,
              abandonedOperationId: existing.operationId,
              abandonedOwnerPid: existing.ownerPid,
              replacementOperationId: operationId,
              startedAt: clock(),
            }));
            fsyncSync(guardDescriptor);
          } finally {
            closeSync(guardDescriptor);
          }
          syncDirectory(dirname(recoveryGuard));
          const confirmed = readJson(path);
          if (
            confirmed.state !== 'building'
            || confirmed.operationId !== recoverActiveOperationId
            || confirmed.ownerPid !== existing.ownerPid
            || processAlive(confirmed.ownerPid)
          ) {
            fail('candidate_build_active_claim_changed', path);
          }
          const archivedClaim = join(
            state,
            'abandoned',
            'claims',
            `${confirmed.operationId}.json`,
          );
          atomicCopy(path, archivedClaim);
          const partialArtifacts = artifactPaths(plan).artifactDirectory;
          if (existsSync(partialArtifacts)) {
            const archivedArtifacts = join(
              state,
              'abandoned',
              'artifacts',
              confirmed.operationId,
            );
            mkdirSync(dirname(archivedArtifacts), { recursive: true });
            renameSync(partialArtifacts, archivedArtifacts);
            syncDirectory(dirname(partialArtifacts));
            syncDirectory(dirname(archivedArtifacts));
          }
          atomicWrite(path, encodeJson(claimDocument(plan, operationId)));
          unlinkSync(recoveryGuard);
          syncDirectory(dirname(recoveryGuard));
          return {
            acquired: true,
            operationId,
            claimPath: path,
            recoveredOperationId: confirmed.operationId,
            archivedClaim,
          };
        }
        if (existing.state !== 'building') {
          fail(
            existing.state === 'failed'
              ? 'candidate_build_failed_recovery_required'
              : 'candidate_build_completion_missing',
            `${existing.operationId}:${existing.failureReceipt ?? path}`,
          );
        }
        return { acquired: false, operationId: existing.operationId, claimPath: path };
      }
    },

    async runCommand(command) {
      await commandRunner(command);
    },

    async sealCandidate(plan, claim) {
      const paths = artifactPaths(plan);
      if (sourceReader) {
        const observedSource = await sourceReader(root);
        if (JSON.stringify(observedSource) !== JSON.stringify(plan.source)) {
          fail('candidate_build_source_changed', plan.source.commit);
        }
      }
      if (!existsSync(paths.binary)) fail('candidate_build_binary_missing', paths.binary);
      if (!existsSync(paths.depInfo)) fail('candidate_build_dep_info_missing', paths.depInfo);

      const closure = collectExecutableInputClosure({
        repoRoot: root,
        depInfoPaths: [paths.depInfo],
        requiredPaths: [
          'Cargo.toml',
          'Cargo.lock',
          'cli/Cargo.toml',
          'package.json',
          'packages/dashboard/package.json',
        ],
        recursiveRoots: plan.artifactClass === 'production_shaped'
          ? ['packages/dashboard/out']
          : [],
        context: {
          target: plan.target,
          toolchain: plan.toolchain,
          cargoProfile: plan.cargoProfile,
          resolvedBuildProfile: plan.resolvedBuildProfile,
          features: plan.features,
          reviewedEnvironmentInputs: plan.reviewedEnvironmentInputs,
        },
        productionShaped: plan.artifactClass === 'production_shaped',
        sourceControlRoots,
      });
      const support = createBuildSupportManifest(closure);
      const supportBytes = encodeBuildSupportManifest(support);
      const binarySha256 = sha256(readFileSync(paths.binary));
      const supportManifestSha256 = sha256(supportBytes);
      const manifest = createCandidateManifest({
        closure,
        source: plan.source,
        artifactClass: plan.artifactClass,
        binarySha256,
        supportManifestSha256,
        createdAt: clock(),
      });
      const manifestBytes = encodeJson(manifest);
      const candidateManifestSha256 = sha256(manifestBytes);
      const identity = buildIdentity(plan, closure);
      const sealed = {
        schemaVersion: SEALED_ARTIFACT_SCHEMA_VERSION,
        operationId: claim.operationId,
        identity,
        binarySha256,
        supportManifestSha256,
        candidateManifestSha256,
        sealSha256: sealDigest(
          claim.operationId,
          identity,
          binarySha256,
          supportManifestSha256,
          candidateManifestSha256,
        ),
      };

      mkdirSync(paths.artifactDirectory, { recursive: true });
      atomicCopy(paths.binary, paths.copiedBinary);
      atomicWrite(paths.closure, encodeJson(closure));
      atomicWrite(paths.support, supportBytes);
      atomicWrite(paths.manifest, manifestBytes);
      atomicWrite(paths.sealed, encodeJson(sealed));
      return {
        candidateId: manifest.candidateId,
        binaryPath: paths.copiedBinary,
        candidateManifestPath: paths.manifest,
        inputClosurePath: paths.closure,
        supportManifestPath: paths.support,
        sealedArtifactPath: paths.sealed,
        binarySha256,
      };
    },

    async completeClaim(plan, candidate, claim) {
      atomicWrite(completionPath(plan), encodeJson({
        schemaVersion: 'agent-browser.candidate-build-completion.v1',
        requestDigest: plan.requestDigest,
        operationId: claim.operationId,
        completedAt: clock(),
        ...candidate,
      }));
      atomicWrite(claim.claimPath, encodeJson({
        ...readJson(claim.claimPath),
        state: 'sealed',
        completedAt: clock(),
        candidateId: candidate.candidateId,
      }));
    },

    async failClaim(plan, error, claim) {
      const failure = {
        schemaVersion: 'agent-browser.candidate-build-failure.v1',
        requestDigest: plan.requestDigest,
        operationId: claim.operationId,
        failedAt: clock(),
        error: error instanceof Error ? error.message : String(error),
      };
      atomicWrite(join(state, 'failed', `${claim.operationId}.json`), encodeJson(failure));
      atomicWrite(claim.claimPath, encodeJson({
        ...readJson(claim.claimPath),
        state: 'failed',
        failedAt: failure.failedAt,
        failureReceipt: join(state, 'failed', `${claim.operationId}.json`),
      }));
    },
  };
}

export const candidateBuildArtifactPaths = artifactPaths;
