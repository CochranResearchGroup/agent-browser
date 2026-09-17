import { createHash, randomUUID } from 'node:crypto';
import {
  closeSync,
  constants,
  fsyncSync,
  linkSync,
  mkdirSync,
  openSync,
  readFileSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';

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
  const pinnedCandidates = request.pinnedCandidates ?? [];
  if (!Array.isArray(pinnedCandidates)) {
    throw new TypeError('request.pinnedCandidates must be an array');
  }
  const pinnedIds = new Set();
  for (const candidate of pinnedCandidates) {
    requireNonemptyString(candidate?.candidateId, 'request.pinnedCandidates[].candidateId');
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
  const receiptPath = `${operationPath}.receipt.json`;
  const effectPath = `${operationPath}.effect.json`;
  const progressPath = `${operationPath}.progress.json`;
  try {
    const receipt = JSON.parse(readFileSync(receiptPath, 'utf8'));
    return { outcome: 'replayed_terminal', receipt };
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }

  let generation = operation.generation;
  const effect = {
    schemaVersion: 'agent-browser.worktree-closeout-effect.v1',
    operationId: operation.operationId,
    generation,
    ownerPid: process.pid,
  };
  try {
    writeFileSync(effectPath, `${canonicalJson(effect)}\n`, {
      encoding: 'utf8',
      flag: 'wx',
      mode: 0o600,
      flush: true,
    });
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
    let recoveryDescriptor;
    try {
      recoveryDescriptor = openSync(
        recoveryPath,
        constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY,
        0o600,
      );
      writeFileSync(recoveryDescriptor, `${operation.operationId}\n`);
      fsyncSync(recoveryDescriptor);
    } catch (recoveryError) {
      if (recoveryError?.code === 'EEXIST') {
        return { outcome: 'joined_recovery', operationId: operation.operationId };
      }
      throw recoveryError;
    } finally {
      if (recoveryDescriptor !== undefined) closeSync(recoveryDescriptor);
    }
    try {
      const confirmed = JSON.parse(readFileSync(effectPath, 'utf8'));
      generation = confirmed.generation + 1;
      atomicWriteJson(effectPath, {
        ...effect,
        generation,
        recoveredFromGeneration: confirmed.generation,
      });
    } finally {
      unlinkSync(recoveryPath);
    }
  }

  let completedCandidateIds = [];
  try {
    const progress = JSON.parse(readFileSync(progressPath, 'utf8'));
    if (progress.operationId !== operation.operationId) {
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
    if (!completed.has(selection.candidateId)) {
      const candidate = candidates.get(selection.candidateId);
      if (selection.disposition === 'archive') {
        adapter.archiveCandidate({ operationId: operation.operationId, generation, candidate });
      } else if (selection.disposition === 'discard') {
        adapter.discardCandidate(candidate);
      }
      completed.add(selection.candidateId);
      atomicWriteJson(progressPath, {
        schemaVersion: 'agent-browser.worktree-closeout-progress.v1',
        operationId: operation.operationId,
        generation,
        completedCandidateIds: [...completed].sort(),
      });
    }
  }
  faultInjector('after_dispositions_before_removal', { operationId: operation.operationId });

  const removal = retained
    ? { outcome: 'retained', worktreePath: operation.request.worktreePath }
    : adapter.removeWorktree(operation.request);
  faultInjector('after_removal_before_receipt', { operationId: operation.operationId });
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
