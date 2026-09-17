import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import {
  closeSync,
  constants,
  copyFileSync,
  existsSync,
  fsyncSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';

const LOCATOR_SCHEMA = 'agent-browser.worktree-closeout-archive-locator.v1';

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function fail(code, detail) {
  const error = new Error(`${code}:${detail}`);
  error.code = code;
  throw error;
}

function syncDirectory(path) {
  const descriptor = openSync(path, constants.O_RDONLY);
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function atomicWrite(path, bytes) {
  mkdirSync(dirname(path), { recursive: true, mode: 0o700 });
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  const descriptor = openSync(
    temporary,
    constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY,
    0o600,
  );
  try {
    writeFileSync(descriptor, bytes);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
  renameSync(temporary, path);
  syncDirectory(dirname(path));
}

function checkedArtifactPath(root, relativePath) {
  if (
    typeof relativePath !== 'string'
    || relativePath.length === 0
    || isAbsolute(relativePath)
  ) {
    fail('worktree_closeout_artifact_path_invalid', String(relativePath));
  }
  const absoluteRoot = resolve(root);
  const absolutePath = resolve(absoluteRoot, relativePath);
  const fromRoot = relative(absoluteRoot, absolutePath);
  if (fromRoot === '..' || fromRoot.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`)) {
    fail('worktree_closeout_artifact_path_escape', relativePath);
  }
  return absolutePath;
}

function validateCandidate(candidate) {
  if (typeof candidate?.candidateId !== 'string' || candidate.candidateId.length === 0) {
    throw new TypeError('candidate.candidateId must be a nonempty string');
  }
  if (typeof candidate?.sourceRoot !== 'string' || candidate.sourceRoot.length === 0) {
    throw new TypeError('candidate.sourceRoot must be a nonempty string');
  }
  if (!Array.isArray(candidate.artifacts) || candidate.artifacts.length === 0) {
    throw new TypeError('candidate.artifacts must be a nonempty array');
  }
  const paths = new Set();
  for (const artifact of candidate.artifacts) {
    checkedArtifactPath(candidate.sourceRoot, artifact?.relativePath);
    if (!/^[a-f0-9]{64}$/.test(artifact?.sha256 ?? '')) {
      throw new TypeError('candidate.artifacts[].sha256 must be a lowercase SHA-256 digest');
    }
    if (paths.has(artifact.relativePath)) {
      throw new TypeError(`duplicate candidate artifact ${artifact.relativePath}`);
    }
    paths.add(artifact.relativePath);
  }
}

function candidateDigest(candidate) {
  return sha256(JSON.stringify({
    candidateId: candidate.candidateId,
    sourceRoot: resolve(candidate.sourceRoot),
    artifacts: candidate.artifacts,
  }));
}

export function discoverPinnedCandidates(worktreePath) {
  const root = realpathSync(worktreePath);
  const completedRoot = join(root, 'cli', 'target', 'candidate-build-state', 'completed');
  if (!existsSync(completedRoot)) return [];
  const requiredFields = [
    'binaryPath',
    'candidateManifestPath',
    'inputClosurePath',
    'supportManifestPath',
    'sealedArtifactPath',
  ];
  const byCandidate = new Map();
  for (const entry of readdirSync(completedRoot, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.json')) continue;
    const completionPath = join(completedRoot, entry.name);
    const receipt = JSON.parse(readFileSync(completionPath, 'utf8'));
    if (
      receipt.schemaVersion !== 'agent-browser.candidate-build-completion.v1'
      || typeof receipt.candidateId !== 'string'
      || requiredFields.some((field) => typeof receipt[field] !== 'string')
    ) {
      fail('worktree_closeout_candidate_completion_invalid', completionPath);
    }
    const paths = [...requiredFields.map((field) => receipt[field]), completionPath];
    const artifacts = paths.map((path) => {
      const absolute = resolve(path);
      const relativePath = relative(root, absolute);
      if (
        relativePath === ''
        || relativePath.startsWith('..')
        || isAbsolute(relativePath)
        || !existsSync(absolute)
      ) {
        fail('worktree_closeout_candidate_artifact_invalid', absolute);
      }
      return { relativePath, sha256: sha256(readFileSync(absolute)) };
    }).sort((left, right) => left.relativePath.localeCompare(right.relativePath));
    const candidate = { candidateId: receipt.candidateId, sourceRoot: root, artifacts };
    const existing = byCandidate.get(candidate.candidateId);
    if (existing && candidateDigest(existing) !== candidateDigest(candidate)) {
      fail('worktree_closeout_candidate_identity_conflict', candidate.candidateId);
    }
    byCandidate.set(candidate.candidateId, candidate);
  }
  return [...byCandidate.values()].sort((left, right) => left.candidateId.localeCompare(right.candidateId));
}

export function discoverActiveCandidateBuilds(worktreePath) {
  const root = realpathSync(worktreePath);
  const claimsRoot = join(root, 'cli', 'target', 'candidate-build-state', 'claims');
  if (!existsSync(claimsRoot)) return [];
  const active = [];
  for (const entry of readdirSync(claimsRoot, { withFileTypes: true })) {
    if (!entry.isFile() || !entry.name.endsWith('.json')) continue;
    const claimPath = join(claimsRoot, entry.name);
    const claim = JSON.parse(readFileSync(claimPath, 'utf8'));
    if (claim.state !== 'building') continue;
    if (
      claim.schemaVersion !== 'agent-browser.candidate-build-claim.v1'
      || typeof claim.operationId !== 'string'
      || typeof claim.requestDigest !== 'string'
    ) {
      fail('worktree_closeout_candidate_claim_invalid', claimPath);
    }
    active.push({
      operationId: claim.operationId,
      requestDigest: claim.requestDigest,
      ownerPid: claim.ownerPid ?? null,
      claimPath,
    });
  }
  return active.sort((left, right) => left.operationId.localeCompare(right.operationId));
}

function verifyArchive(locator) {
  for (const artifact of locator.artifacts) {
    const path = checkedArtifactPath(locator.archiveRoot, artifact.relativePath);
    if (!existsSync(path) || sha256(readFileSync(path)) !== artifact.sha256) {
      fail('worktree_closeout_archive_digest_mismatch', artifact.relativePath);
    }
  }
}

function git(args, cwd) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
}

function gitOptional(args, cwd) {
  try {
    return git(args, cwd);
  } catch (error) {
    if (error?.status === 1) return null;
    throw error;
  }
}

function filesystemIdentity(path) {
  const resolved = realpathSync(path);
  const stat = statSync(resolved);
  return sha256(JSON.stringify([resolved, String(stat.dev), String(stat.ino)]));
}

function registeredWorktrees(repositoryRoot) {
  const records = [];
  let current = null;
  for (const line of git(['worktree', 'list', '--porcelain'], repositoryRoot).split('\n')) {
    if (line.startsWith('worktree ')) {
      current = { path: resolve(line.slice('worktree '.length)) };
      records.push(current);
    } else if (current && line.startsWith('HEAD ')) {
      current.head = line.slice('HEAD '.length);
    } else if (current && line.startsWith('branch ')) {
      current.ref = line.slice('branch '.length);
    } else if (current && line === 'detached') {
      current.ref = 'HEAD';
    }
  }
  return records;
}

export function inspectWorktreeCloseout({ repositoryRoot, worktreePath }) {
  const root = resolve(repositoryRoot);
  const path = realpathSync(worktreePath);
  const registration = registeredWorktrees(root).find((entry) => entry.path === path);
  if (!registration) fail('worktree_closeout_not_registered', path);
  const commonGitDirectory = realpathSync(resolve(root, git(['rev-parse', '--git-common-dir'], root)));
  const worktreeGitDirectory = realpathSync(git(['rev-parse', '--absolute-git-dir'], path));
  const head = git(['rev-parse', 'HEAD'], path);
  const ref = gitOptional(['symbolic-ref', '-q', 'HEAD'], path) ?? 'HEAD';
  const dirtyState = git(['status', '--porcelain=v1', '--untracked-files=all'], path);
  return {
    repositoryId: filesystemIdentity(commonGitDirectory),
    coordinationStateRoot: join(commonGitDirectory, 'agent-browser-worktree-closeout'),
    archiveRoot: join(commonGitDirectory, 'agent-browser-worktree-closeout', 'candidate-archives'),
    worktreeIncarnation: filesystemIdentity(worktreeGitDirectory),
    worktreePath: path,
    expectedHead: head,
    expectedRef: ref,
    dirtyStateDigest: sha256(dirtyState),
    dirty: dirtyState.length > 0,
  };
}

function assertSameInspection(expected, actual) {
  for (const field of [
    'repositoryId',
    'coordinationStateRoot',
    'archiveRoot',
    'worktreeIncarnation',
    'worktreePath',
    'expectedHead',
    'expectedRef',
    'dirtyStateDigest',
  ]) {
    if (expected[field] !== actual[field]) {
      fail('worktree_closeout_revalidation_conflict', field);
    }
  }
}

function isWithin(parent, child) {
  const fromParent = relative(resolve(parent), resolve(child));
  return fromParent === '' || (!fromParent.startsWith('..') && !isAbsolute(fromParent));
}

export function removeInspectedWorktree({ repositoryRoot, inspection }) {
  const path = resolve(inspection.worktreePath);
  if (!existsSync(path)) {
    const remainsRegistered = registeredWorktrees(resolve(repositoryRoot))
      .some((entry) => entry.path === path);
    if (remainsRegistered) fail('worktree_closeout_missing_registered_path', path);
    return {
      outcome: 'already_removed',
      worktreePath: path,
      worktreeIncarnation: inspection.worktreeIncarnation,
      expectedHead: inspection.expectedHead,
      expectedRef: inspection.expectedRef,
    };
  }
  const current = inspectWorktreeCloseout({
    repositoryRoot,
    worktreePath: inspection.worktreePath,
  });
  assertSameInspection(inspection, current);
  if (current.dirty) fail('worktree_closeout_dirty_worktree', current.worktreePath);
  git(['worktree', 'remove', current.worktreePath], resolve(repositoryRoot));
  const remainsRegistered = registeredWorktrees(resolve(repositoryRoot))
    .some((entry) => entry.path === current.worktreePath);
  if (remainsRegistered || existsSync(current.worktreePath)) {
    fail('worktree_closeout_removal_readback_failed', current.worktreePath);
  }
  return {
    outcome: 'removed',
    worktreePath: current.worktreePath,
    worktreeIncarnation: current.worktreeIncarnation,
    expectedHead: current.expectedHead,
    expectedRef: current.expectedRef,
  };
}

export function createWorktreeCloseoutFilesystemAdapter({
  stateRoot,
  archiveRoot,
  repositoryRoot = null,
  faultInjector = () => {},
} = {}) {
  if (typeof stateRoot !== 'string' || typeof archiveRoot !== 'string') {
    throw new TypeError('stateRoot and archiveRoot are required');
  }
  const locatorRoot = join(resolve(stateRoot), 'worktree-closeout', 'archives');
  const durableStateRoot = resolve(stateRoot);
  const durableArchiveRoot = resolve(archiveRoot);

  function assertDurableStorage(worktreePath) {
    if (
      isWithin(worktreePath, durableStateRoot)
      || isWithin(worktreePath, durableArchiveRoot)
    ) {
      fail('worktree_closeout_storage_inside_worktree', resolve(worktreePath));
    }
  }

  function revalidateWorktree(inspection) {
    if (!repositoryRoot) throw new TypeError('repositoryRoot is required for revalidation');
    if (!existsSync(inspection.worktreePath)) {
      const registered = registeredWorktrees(resolve(repositoryRoot))
        .some((entry) => entry.path === resolve(inspection.worktreePath));
      if (registered) {
        fail('worktree_closeout_missing_registered_path', inspection.worktreePath);
      }
      return { outcome: 'already_removed', worktreePath: resolve(inspection.worktreePath) };
    }
    const current = inspectWorktreeCloseout({
      repositoryRoot,
      worktreePath: inspection.worktreePath,
    });
    assertSameInspection(inspection, current);
    return current;
  }

  function assertCandidatePins(request, { allowDisposed = false } = {}) {
    if (!existsSync(request.worktreePath)) {
      if (allowDisposed) return [];
      fail('worktree_closeout_worktree_missing_before_effect', request.worktreePath);
    }
    const activeBuilds = discoverActiveCandidateBuilds(request.worktreePath);
    if (activeBuilds.length > 0) {
      fail(
        'worktree_closeout_candidate_build_active',
        activeBuilds.map(({ operationId }) => operationId).join(','),
      );
    }
    const discovered = discoverPinnedCandidates(request.worktreePath);
    const expected = new Map((request.pinnedCandidates ?? [])
      .map((candidate) => [candidate.candidateId, candidate]));
    for (const candidate of discovered) {
      const requested = expected.get(candidate.candidateId);
      if (!requested || candidateDigest(requested) !== candidateDigest(candidate)) {
        fail('worktree_closeout_candidate_pins_changed', candidate.candidateId);
      }
      expected.delete(candidate.candidateId);
    }
    if (!allowDisposed && expected.size > 0) {
      fail('worktree_closeout_candidate_pins_changed', [...expected.keys()].join(','));
    }
    return discovered;
  }

  function locatorPath(candidateId) {
    return join(locatorRoot, `${sha256(candidateId)}.json`);
  }

  function resolveArchivedCandidate(candidateId) {
    const path = locatorPath(candidateId);
    if (!existsSync(path)) return null;
    const locator = JSON.parse(readFileSync(path, 'utf8'));
    if (locator.schemaVersion !== LOCATOR_SCHEMA || locator.candidateId !== candidateId) {
      fail('worktree_closeout_archive_locator_invalid', path);
    }
    verifyArchive(locator);
    return locator;
  }

  function archiveCandidate({ operationId, generation, candidate }) {
    validateCandidate(candidate);
    const candidateKey = sha256(candidate.candidateId);
    const destination = join(durableArchiveRoot, candidateKey);
    const staging = join(
      durableArchiveRoot,
      '.staging',
      `${sha256(operationId)}-${candidateKey}`,
    );
    const existing = resolveArchivedCandidate(candidate.candidateId);
    if (existing) {
      const requestedArtifacts = candidate.artifacts.map(({ relativePath, sha256: digest }) => ({
        relativePath,
        sha256: digest,
      }));
      if (JSON.stringify(existing.artifacts) !== JSON.stringify(requestedArtifacts)) {
        fail('worktree_closeout_archive_request_mismatch', candidate.candidateId);
      }
      return { outcome: 'already_archived', locator: existing };
    }

    rmSync(staging, { recursive: true, force: true });
    mkdirSync(staging, { recursive: true, mode: 0o700 });
    for (const artifact of candidate.artifacts) {
      const source = checkedArtifactPath(candidate.sourceRoot, artifact.relativePath);
      const sourceBytes = readFileSync(source);
      if (sha256(sourceBytes) !== artifact.sha256) {
        fail('worktree_closeout_source_digest_mismatch', artifact.relativePath);
      }
      const target = checkedArtifactPath(staging, artifact.relativePath);
      mkdirSync(dirname(target), { recursive: true, mode: 0o700 });
      copyFileSync(source, target, constants.COPYFILE_EXCL);
      const descriptor = openSync(target, constants.O_RDONLY);
      try {
        fsyncSync(descriptor);
      } finally {
        closeSync(descriptor);
      }
      if (sha256(readFileSync(target)) !== artifact.sha256) {
        fail('worktree_closeout_archive_digest_mismatch', artifact.relativePath);
      }
    }
    syncDirectory(staging);
    faultInjector('after_copy_before_publish', { operationId, candidateId: candidate.candidateId });

    mkdirSync(durableArchiveRoot, { recursive: true, mode: 0o700 });
    try {
      renameSync(staging, destination);
      syncDirectory(durableArchiveRoot);
    } catch (error) {
      if (error?.code !== 'EEXIST' && error?.code !== 'ENOTEMPTY') throw error;
      rmSync(staging, { recursive: true, force: true });
    }
    const locator = {
      schemaVersion: LOCATOR_SCHEMA,
      candidateId: candidate.candidateId,
      archiveRoot: destination,
      operationId,
      generation,
      artifacts: candidate.artifacts.map(({ relativePath, sha256: digest }) => ({
        relativePath,
        sha256: digest,
      })),
    };
    verifyArchive(locator);
    atomicWrite(locatorPath(candidate.candidateId), Buffer.from(`${JSON.stringify(locator, null, 2)}\n`));
    return { outcome: 'archived', locator };
  }

  function discardCandidate(candidate) {
    validateCandidate(candidate);
    const paths = candidate.artifacts.map((artifact) => ({
      ...artifact,
      path: checkedArtifactPath(candidate.sourceRoot, artifact.relativePath),
    }));
    for (const artifact of paths) {
      if (existsSync(artifact.path) && sha256(readFileSync(artifact.path)) !== artifact.sha256) {
        fail('worktree_closeout_source_digest_mismatch', artifact.relativePath);
      }
    }
    const existing = paths.filter((artifact) => existsSync(artifact.path));
    for (const artifact of existing) unlinkSync(artifact.path);
    return {
      outcome: existing.length > 0 ? 'discarded' : 'already_discarded',
      candidateId: candidate.candidateId,
      removedArtifacts: paths.map(({ relativePath, sha256: digest }) => ({
        relativePath,
        sha256: digest,
      })),
    };
  }

  function removeWorktree(inspection) {
    if (!repositoryRoot) throw new TypeError('repositoryRoot is required for removal');
    return removeInspectedWorktree({ repositoryRoot, inspection });
  }

  function verifyTerminalReceipt(request, receipt) {
    for (const selection of receipt.candidateDispositions) {
      if (selection.disposition !== 'archive') continue;
      const candidate = request.pinnedCandidates.find(
        ({ candidateId }) => candidateId === selection.candidateId,
      );
      const locator = resolveArchivedCandidate(selection.candidateId);
      if (!locator) fail('worktree_closeout_archive_locator_missing', selection.candidateId);
      const requested = candidate.artifacts.map(({ relativePath, sha256: digest }) => ({
        relativePath,
        sha256: digest,
      }));
      if (JSON.stringify(locator.artifacts) !== JSON.stringify(requested)) {
        fail('worktree_closeout_archive_request_mismatch', selection.candidateId);
      }
    }
    if (receipt.removal.outcome === 'retained') {
      revalidateWorktree(request);
      assertCandidatePins(request);
    } else {
      const path = resolve(request.worktreePath);
      if (
        existsSync(path)
        || registeredWorktrees(resolve(repositoryRoot)).some((entry) => entry.path === path)
      ) {
        fail('worktree_closeout_terminal_removal_invalid', path);
      }
    }
  }

  return {
    archiveCandidate,
    assertDurableStorage,
    assertCandidatePins,
    discardCandidate,
    removeWorktree,
    resolveArchivedCandidate,
    revalidateWorktree,
    verifyTerminalReceipt,
  };
}
