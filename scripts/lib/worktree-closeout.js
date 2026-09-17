import { createHash, randomUUID } from 'node:crypto';
import {
  closeSync,
  constants,
  fsyncSync,
  existsSync,
  linkSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';

const REQUEST_SCHEMA = 'agent-browser.worktree-closeout-request.v1';
const OPERATION_SCHEMA = 'agent-browser.worktree-closeout-operation.v1';

export class WorktreeCloseoutConflictError extends Error {
  constructor(message, details) {
    super(message);
    this.name = 'WorktreeCloseoutConflictError';
    this.code = 'worktree_closeout_request_conflict';
    this.details = details;
  }
}

export class CandidateDispositionRequiredError extends Error {
  constructor(candidateIds) {
    super('every pinned candidate requires an explicit closeout disposition');
    this.name = 'CandidateDispositionRequiredError';
    this.code = 'worktree_closeout_candidate_disposition_required';
    this.details = {
      candidateIds,
      supportedDispositions: ['retain', 'archive', 'discard'],
    };
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(',')}]`;
  }
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(',')}}`;
  }
  return JSON.stringify(value);
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function requireNonemptyString(value, field) {
  if (typeof value !== 'string' || value.length === 0) {
    throw new TypeError(`${field} must be a nonempty string`);
  }
}

function validateRequest(request) {
  if (request === null || typeof request !== 'object' || Array.isArray(request)) {
    throw new TypeError('request must be an object');
  }
  if (request.schemaVersion !== REQUEST_SCHEMA) {
    throw new TypeError(`request.schemaVersion must be ${REQUEST_SCHEMA}`);
  }
  for (const field of [
    'repositoryId',
    'coordinationStateRoot',
    'archiveRoot',
    'worktreeIncarnation',
    'worktreePath',
    'expectedHead',
    'expectedRef',
  ]) {
    requireNonemptyString(request[field], `request.${field}`);
  }
  if (!Array.isArray(request.candidateDispositions)) {
    throw new TypeError('request.candidateDispositions must be an array');
  }
  if (
    request.supersedesRetainedOperationId !== undefined
    && (typeof request.supersedesRetainedOperationId !== 'string'
      || request.supersedesRetainedOperationId.length === 0)
  ) {
    throw new TypeError('request.supersedesRetainedOperationId must be a nonempty string');
  }
  const pinnedCandidates = request.pinnedCandidates ?? [];
  if (!Array.isArray(pinnedCandidates)) {
    throw new TypeError('request.pinnedCandidates must be an array');
  }
  const pinnedIds = new Set();
  for (const candidate of pinnedCandidates) {
    requireNonemptyString(candidate?.candidateId, 'request.pinnedCandidates[].candidateId');
    requireNonemptyString(candidate?.sourceRoot, 'request.pinnedCandidates[].sourceRoot');
    const sourceFromWorktree = relative(resolve(request.worktreePath), resolve(candidate.sourceRoot));
    if (sourceFromWorktree.startsWith('..') || isAbsolute(sourceFromWorktree)) {
      throw new TypeError('pinned candidate sourceRoot must be inside the worktree');
    }
    if (!Array.isArray(candidate.artifacts)) {
      throw new TypeError('request.pinnedCandidates[].artifacts must be an array');
    }
    const artifactNames = new Set(candidate.artifacts.map((artifact) => {
      requireNonemptyString(artifact?.relativePath, 'candidate.artifacts[].relativePath');
      if (!/^[a-f0-9]{64}$/.test(artifact?.sha256 ?? '')) {
        throw new TypeError('candidate.artifacts[].sha256 must be a lowercase SHA-256 digest');
      }
      return basename(artifact.relativePath);
    }));
    for (const required of [
      'candidate-manifest.json',
      'executable-input-closure.json',
      'build-support-manifest.json',
      'sealed-artifact.json',
    ]) {
      if (!artifactNames.has(required)) {
        throw new TypeError(`pinned candidate is missing ${required}`);
      }
    }
    if (!artifactNames.has('agent-browser') && !artifactNames.has('agent-browser.exe')) {
      throw new TypeError('pinned candidate is missing its binary');
    }
    if (![...candidate.artifacts].some(({ relativePath }) => (
      relativePath.includes('candidate-build-state/completed/')
      && relativePath.endsWith('.json')
    ))) {
      throw new TypeError('pinned candidate is missing its completion receipt');
    }
    if (pinnedIds.has(candidate.candidateId)) {
      throw new TypeError(`duplicate pinned candidate ${candidate.candidateId}`);
    }
    pinnedIds.add(candidate.candidateId);
  }
  const dispositions = new Map();
  for (const selection of request.candidateDispositions) {
    requireNonemptyString(selection?.candidateId, 'request.candidateDispositions[].candidateId');
    if (!['retain', 'archive', 'discard'].includes(selection.disposition)) {
      throw new TypeError(`unsupported candidate disposition ${selection.disposition}`);
    }
    if (dispositions.has(selection.candidateId)) {
      throw new TypeError(`duplicate candidate disposition ${selection.candidateId}`);
    }
    if (!pinnedIds.has(selection.candidateId)) {
      throw new TypeError(`candidate disposition is not pinned: ${selection.candidateId}`);
    }
    dispositions.set(selection.candidateId, selection.disposition);
  }
  const unselected = [...pinnedIds].filter((candidateId) => !dispositions.has(candidateId));
  if (unselected.length > 0) {
    throw new CandidateDispositionRequiredError(unselected);
  }
}

function readOperation(operationPath) {
  const operation = JSON.parse(readFileSync(operationPath, 'utf8'));
  if (operation.schemaVersion !== OPERATION_SCHEMA) {
    throw new Error(`unsupported worktree closeout operation at ${operationPath}`);
  }
  validateRequest(operation.request);
  const requestDigest = sha256(canonicalJson(operation.request));
  if (operation.requestDigest !== requestDigest) {
    const error = new Error(`worktree_closeout_operation_digest_mismatch:${operationPath}`);
    error.code = 'worktree_closeout_operation_digest_mismatch';
    throw error;
  }
  return operation;
}

function syncDirectory(path) {
  const descriptor = openSync(path, constants.O_RDONLY);
  try {
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

function atomicWriteJson(path, document) {
  const temporary = `${path}.tmp-${process.pid}-${randomUUID()}`;
  writeFileSync(temporary, `${canonicalJson(document)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
    flush: true,
  });
  renameSync(temporary, path);
  syncDirectory(dirname(path));
}

export function beginOrJoinWorktreeCloseout({ stateRoot, request }) {
  requireNonemptyString(stateRoot, 'stateRoot');
  validateRequest(request);
  if (resolve(stateRoot) !== resolve(request.coordinationStateRoot)) {
    throw new TypeError('stateRoot must match request.coordinationStateRoot');
  }

  const requestJson = canonicalJson(request);
  const requestDigest = sha256(requestJson);
  const identityDigest = sha256(canonicalJson({
    repositoryId: request.repositoryId,
    worktreeIncarnation: request.worktreeIncarnation,
  }));
  const operationsRoot = join(stateRoot, 'worktree-closeout', 'operations');
  const operationPath = join(operationsRoot, `${identityDigest}.json`);
  mkdirSync(operationsRoot, { recursive: true, mode: 0o700 });

  const operation = {
    schemaVersion: OPERATION_SCHEMA,
    operationId: `worktree-closeout-${randomUUID()}`,
    generation: 1,
    phase: 'selected',
    requestDigest,
    request,
  };
  const temporaryPath = join(
    operationsRoot,
    `.${identityDigest}.${process.pid}.${randomUUID()}.tmp`,
  );
  writeFileSync(temporaryPath, `${canonicalJson(operation)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
    flush: true,
  });

  try {
    try {
      // Publishing a hard link is atomic and cannot expose a partial record.
      linkSync(temporaryPath, operationPath);
    } catch (error) {
      if (error?.code !== 'EEXIST') throw error;
      const existing = readOperation(operationPath);
      if (existing.requestDigest !== requestDigest) {
        const retainedReceiptPath = `${operationPath}.${existing.operationId}.receipt.json`;
        let retainedReceipt = null;
        try {
          retainedReceipt = JSON.parse(readFileSync(retainedReceiptPath, 'utf8'));
        } catch (receiptError) {
          if (receiptError?.code !== 'ENOENT') throw receiptError;
        }
        if (
          request.supersedesRetainedOperationId === existing.operationId
          && retainedReceipt?.schemaVersion === 'agent-browser.worktree-closeout-receipt.v1'
          && retainedReceipt.operationId === existing.operationId
          && retainedReceipt.requestDigest === existing.requestDigest
          && retainedReceipt.removal?.outcome === 'retained'
        ) {
          const selectionPath = `${operationPath}.selection`;
          const selectionTemporary = `${selectionPath}.${process.pid}.${randomUUID()}.tmp`;
          writeFileSync(selectionTemporary, `${canonicalJson({
            schemaVersion: 'agent-browser.worktree-closeout-selection.v1',
            priorOperationId: existing.operationId,
            ownerPid: process.pid,
          })}\n`, { encoding: 'utf8', flag: 'wx', mode: 0o600, flush: true });
          try {
            linkSync(selectionTemporary, selectionPath);
          } catch (selectionError) {
            if (selectionError?.code === 'EEXIST') {
              return { outcome: 'joined_selection', operationId: existing.operationId };
            }
            throw selectionError;
          } finally {
            unlinkSync(selectionTemporary);
          }
          try {
            const confirmed = readOperation(operationPath);
            if (confirmed.operationId !== existing.operationId) {
              return {
                outcome: 'joined_existing',
                operationId: confirmed.operationId,
                generation: confirmed.generation,
                requestDigest: confirmed.requestDigest,
                operationPath,
              };
            }
            operation.generation = existing.generation + 1;
            atomicWriteJson(operationPath, operation);
            return {
              outcome: 'started',
              operationId: operation.operationId,
              generation: operation.generation,
              requestDigest,
              operationPath,
              supersededRetainedOperationId: existing.operationId,
            };
          } finally {
            unlinkSync(selectionPath);
          }
        }
        throw new WorktreeCloseoutConflictError(
          'a different closeout request already owns this worktree incarnation',
          {
            operationId: existing.operationId,
            existingRequestDigest: existing.requestDigest,
            requestedDigest: requestDigest,
          },
        );
      }
      return {
        outcome: 'joined_existing',
        operationId: existing.operationId,
        generation: existing.generation,
        requestDigest,
        operationPath,
      };
    }
    syncDirectory(operationsRoot);
    return {
      outcome: 'started',
      operationId: operation.operationId,
      generation: operation.generation,
      requestDigest,
      operationPath,
    };
  } finally {
    try {
      unlinkSync(temporaryPath);
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error;
    }
  }
}

export function executeWorktreeCloseout({
  operationPath,
  adapter,
  recover = false,
  faultInjector = () => {},
  processAlive = (pid) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch (error) {
      return error?.code === 'EPERM';
    }
  },
}) {
  requireNonemptyString(operationPath, 'operationPath');
  if (!adapter || typeof adapter.removeWorktree !== 'function') {
    throw new TypeError('adapter.removeWorktree is required');
  }
  const operation = readOperation(operationPath);
  const expectedOperationsRoot = resolve(
    operation.request.coordinationStateRoot,
    'worktree-closeout',
    'operations',
  );
  const operationFromRoot = relative(expectedOperationsRoot, resolve(operationPath));
  if (operationFromRoot.startsWith('..') || isAbsolute(operationFromRoot)) {
    const locationError = new Error(`worktree_closeout_operation_location_invalid:${operationPath}`);
    locationError.code = 'worktree_closeout_operation_location_invalid';
    throw locationError;
  }
  const receiptPath = `${operationPath}.${operation.operationId}.receipt.json`;
  const effectPath = `${operationPath}.${operation.operationId}.effect.json`;
  const progressPath = `${operationPath}.${operation.operationId}.progress.json`;
  try {
    const receipt = JSON.parse(readFileSync(receiptPath, 'utf8'));
    if (
      receipt.schemaVersion !== 'agent-browser.worktree-closeout-receipt.v1'
      || receipt.operationId !== operation.operationId
      || receipt.requestDigest !== operation.requestDigest
      || !Number.isInteger(receipt.generation)
      || JSON.stringify(receipt.candidateDispositions)
        !== JSON.stringify(operation.request.candidateDispositions)
      || !receipt.removal?.outcome
    ) {
      const receiptError = new Error(`worktree_closeout_receipt_invalid:${receiptPath}`);
      receiptError.code = 'worktree_closeout_receipt_invalid';
      throw receiptError;
    }
    if (typeof adapter.verifyTerminalReceipt === 'function') {
      adapter.verifyTerminalReceipt(operation.request, receipt);
    }
    return { outcome: 'replayed_terminal', receipt };
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  const effectAlreadyExists = existsSync(effectPath);
  if (typeof adapter.assertCandidatePins === 'function') {
    adapter.assertCandidatePins(operation.request, { allowDisposed: effectAlreadyExists });
  }

  let generation = operation.generation;
  const effect = {
    schemaVersion: 'agent-browser.worktree-closeout-effect.v1',
    operationId: operation.operationId,
    generation,
    ownerPid: process.pid,
  };
  const effectTemporary = `${effectPath}.${process.pid}.${randomUUID()}.tmp`;
  writeFileSync(effectTemporary, `${canonicalJson(effect)}\n`, {
    encoding: 'utf8',
    flag: 'wx',
    mode: 0o600,
    flush: true,
  });
  try {
    linkSync(effectTemporary, effectPath);
    syncDirectory(dirname(effectPath));
  } catch (error) {
    if (error?.code !== 'EEXIST') throw error;
    const existing = JSON.parse(readFileSync(effectPath, 'utf8'));
    if (existing.operationId !== operation.operationId) {
      throw new WorktreeCloseoutConflictError('effect claim belongs to another operation', {
        operationId: existing.operationId,
      });
    }
    if (!recover) {
      return { outcome: 'joined_in_progress', operationId: operation.operationId };
    }
    if (existing.ownerPid !== process.pid && processAlive(existing.ownerPid)) {
      const ownerError = new Error(`worktree_closeout_effect_owner_active:${existing.ownerPid}`);
      ownerError.code = 'worktree_closeout_effect_owner_active';
      throw ownerError;
    }
    const recoveryPath = `${effectPath}.recovery`;
    const recoveryTemporary = `${recoveryPath}.${process.pid}.${randomUUID()}.tmp`;
    try {
      writeFileSync(recoveryTemporary, `${canonicalJson({
        schemaVersion: 'agent-browser.worktree-closeout-recovery-guard.v1',
        operationId: operation.operationId,
        ownerPid: process.pid,
      })}\n`, { encoding: 'utf8', flag: 'wx', mode: 0o600, flush: true });
      linkSync(recoveryTemporary, recoveryPath);
    } catch (recoveryError) {
      if (recoveryError?.code === 'EEXIST') {
        const guard = JSON.parse(readFileSync(recoveryPath, 'utf8'));
        if (
          guard.schemaVersion === 'agent-browser.worktree-closeout-recovery-guard.v1'
          && guard.operationId === operation.operationId
          && Number.isInteger(guard.ownerPid)
          && processAlive(guard.ownerPid)
        ) {
          return { outcome: 'joined_recovery', operationId: operation.operationId };
        }
        try {
          unlinkSync(recoveryPath);
        } catch (unlinkError) {
          if (unlinkError?.code !== 'ENOENT') throw unlinkError;
        }
        return executeWorktreeCloseout({
          operationPath,
          adapter,
          recover,
          faultInjector,
          processAlive,
        });
      }
      throw recoveryError;
    } finally {
      try {
        unlinkSync(recoveryTemporary);
      } catch (unlinkError) {
        if (unlinkError?.code !== 'ENOENT') throw unlinkError;
      }
    }
    try {
      const confirmed = JSON.parse(readFileSync(effectPath, 'utf8'));
      if (
        confirmed.generation !== existing.generation
        || confirmed.ownerPid !== existing.ownerPid
      ) {
        return { outcome: 'joined_recovery', operationId: operation.operationId };
      }
      if (confirmed.ownerPid !== process.pid && processAlive(confirmed.ownerPid)) {
        const ownerError = new Error(`worktree_closeout_effect_owner_active:${confirmed.ownerPid}`);
        ownerError.code = 'worktree_closeout_effect_owner_active';
        throw ownerError;
      }
      generation = confirmed.generation + 1;
      atomicWriteJson(effectPath, {
        ...effect,
        generation,
        recoveredFromGeneration: confirmed.generation,
      });
    } finally {
      unlinkSync(recoveryPath);
    }
  } finally {
    try {
      unlinkSync(effectTemporary);
    } catch (unlinkError) {
      if (unlinkError?.code !== 'ENOENT') throw unlinkError;
    }
  }

  const assertFence = () => {
    const current = JSON.parse(readFileSync(effectPath, 'utf8'));
    if (
      current.operationId !== operation.operationId
      || current.generation !== generation
      || current.ownerPid !== process.pid
    ) {
      const fenceError = new Error(`worktree_closeout_fence_superseded:${operation.operationId}`);
      fenceError.code = 'worktree_closeout_fence_superseded';
      throw fenceError;
    }
  };

  if (typeof adapter.assertDurableStorage === 'function') {
    adapter.assertDurableStorage(operation.request.worktreePath);
  }
  if (typeof adapter.revalidateWorktree === 'function') {
    adapter.revalidateWorktree(operation.request);
  }
  assertFence();

  let completedCandidateIds = [];
  try {
    const progress = JSON.parse(readFileSync(progressPath, 'utf8'));
    if (
      progress.schemaVersion !== 'agent-browser.worktree-closeout-progress.v1'
      || progress.operationId !== operation.operationId
      || !Number.isInteger(progress.generation)
      || !Array.isArray(progress.completedCandidateIds)
      || progress.completedCandidateIds.some((candidateId) => typeof candidateId !== 'string')
    ) {
      throw new WorktreeCloseoutConflictError('progress belongs to another operation', {
        operationId: progress.operationId,
      });
    }
    completedCandidateIds = progress.completedCandidateIds;
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  const completed = new Set(completedCandidateIds);
  const candidates = new Map((operation.request.pinnedCandidates ?? [])
    .map((candidate) => [candidate.candidateId, candidate]));
  let retained = false;
  for (const selection of operation.request.candidateDispositions) {
    if (selection.disposition === 'retain') retained = true;
    const candidate = candidates.get(selection.candidateId);
    if (selection.disposition === 'archive') {
      assertFence();
      adapter.archiveCandidate({ operationId: operation.operationId, generation, candidate });
      assertFence();
    } else if (selection.disposition === 'discard') {
      if (typeof adapter.revalidateWorktree === 'function') {
        adapter.revalidateWorktree(operation.request);
      }
      assertFence();
      adapter.discardCandidate(candidate);
      assertFence();
    }
    completed.add(selection.candidateId);
    atomicWriteJson(progressPath, {
      schemaVersion: 'agent-browser.worktree-closeout-progress.v1',
      operationId: operation.operationId,
      generation,
      completedCandidateIds: [...completed].sort(),
    });
  }
  faultInjector('after_dispositions_before_removal', { operationId: operation.operationId });

  if (typeof adapter.assertCandidatePins === 'function') {
    adapter.assertCandidatePins(operation.request, { allowDisposed: true });
  }

  const removal = retained
    ? { outcome: 'retained', worktreePath: operation.request.worktreePath }
    : (assertFence(), adapter.removeWorktree(operation.request));
  faultInjector('after_removal_before_receipt', { operationId: operation.operationId });
  assertFence();
  const receipt = {
    schemaVersion: 'agent-browser.worktree-closeout-receipt.v1',
    operationId: operation.operationId,
    generation,
    requestDigest: operation.requestDigest,
    candidateDispositions: operation.request.candidateDispositions,
    removal,
  };
  atomicWriteJson(receiptPath, receipt);
  return { outcome: 'committed', receipt };
}
