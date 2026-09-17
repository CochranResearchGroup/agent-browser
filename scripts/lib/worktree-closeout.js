import { createHash, randomUUID } from 'node:crypto';
import {
  closeSync,
  constants,
  fsyncSync,
  linkSync,
  mkdirSync,
  openSync,
  readFileSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join } from 'node:path';

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
