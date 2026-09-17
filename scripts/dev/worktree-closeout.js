#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { isAbsolute, join, relative, resolve } from 'node:path';
import {
  beginOrJoinWorktreeCloseout,
  executeWorktreeCloseout,
} from '../lib/worktree-closeout.js';
import {
  createWorktreeCloseoutFilesystemAdapter,
  discoverActiveCandidateBuilds,
  discoverPinnedCandidates,
  inspectWorktreeCloseout,
} from '../lib/worktree-closeout-filesystem-adapter.js';

function fail(message) {
  const error = new Error(message);
  error.code = 'worktree_closeout_usage';
  throw error;
}

function parseArguments(argv) {
  const [command, ...tokens] = argv;
  const options = {};
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (!token.startsWith('--')) fail(`unexpected argument ${token}`);
    const name = token.slice(2);
    if (name === 'apply' || name === 'recover') {
      options[name] = true;
      continue;
    }
    const value = tokens[index + 1];
    if (!value || value.startsWith('--')) fail(`missing value for ${token}`);
    options[name] = value;
    index += 1;
  }
  return { command, options };
}

function required(options, name) {
  if (!options[name]) fail(`--${name} is required`);
  return resolve(options[name]);
}

function print(value) {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

function assertCurrentInspection(repositoryRoot, request) {
  const current = inspectWorktreeCloseout({
    repositoryRoot,
    worktreePath: request.worktreePath,
  });
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
    if (current[field] !== request[field]) {
      const error = new Error(`worktree_closeout_revalidation_conflict:${field}`);
      error.code = 'worktree_closeout_revalidation_conflict';
      throw error;
    }
  }
  return current;
}

function assertOperationLocation(operationPath, stateRoot) {
  const fromState = relative(resolve(stateRoot), resolve(operationPath));
  if (fromState.startsWith('..') || isAbsolute(fromState)) {
    fail('operation must be inside the repository coordination state root');
  }
}

function main() {
  const { command, options } = parseArguments(process.argv.slice(2));
  if (command === 'inspect') {
    const repositoryRoot = resolve(options['repository-root'] ?? process.cwd());
    const inspection = inspectWorktreeCloseout({
      repositoryRoot,
      worktreePath: required(options, 'worktree'),
    });
    const pinnedCandidates = discoverPinnedCandidates(inspection.worktreePath);
    const activeCandidateBuilds = discoverActiveCandidateBuilds(inspection.worktreePath);
    print({
      effect: 'none',
      request: {
        schemaVersion: 'agent-browser.worktree-closeout-request.v1',
        ...inspection,
        activeCandidateBuilds,
        pinnedCandidates,
        candidateDispositions: [],
      },
      next: 'Add every pinned candidate and an explicit retain, archive, or discard disposition before begin.',
    });
    return;
  }

  if (command === 'begin') {
    const request = JSON.parse(readFileSync(required(options, 'request'), 'utf8'));
    const repositoryRoot = required(options, 'repository-root');
    assertCurrentInspection(repositoryRoot, request);
    const discovered = discoverPinnedCandidates(request.worktreePath);
    if (JSON.stringify(discovered) !== JSON.stringify(request.pinnedCandidates ?? [])) {
      const error = new Error('worktree_closeout_candidate_pins_changed');
      error.code = 'worktree_closeout_candidate_pins_changed';
      throw error;
    }
    print(beginOrJoinWorktreeCloseout({
      stateRoot: request.coordinationStateRoot,
      request,
    }));
    return;
  }

  if (command === 'lookup-archive') {
    const repositoryRoot = resolve(options['repository-root'] ?? process.cwd());
    if (!options['candidate-id']) fail('--candidate-id is required');
    const commonGitDirectory = resolve(
      repositoryRoot,
      execFileSync('git', ['rev-parse', '--git-common-dir'], {
        cwd: repositoryRoot,
        encoding: 'utf8',
      }).trim(),
    );
    const stateRoot = join(commonGitDirectory, 'agent-browser-worktree-closeout');
    const adapter = createWorktreeCloseoutFilesystemAdapter({
      repositoryRoot,
      stateRoot,
      archiveRoot: join(stateRoot, 'candidate-archives'),
    });
    const locator = adapter.resolveArchivedCandidate(options['candidate-id']);
    print(locator
      ? { outcome: 'found', effect: 'none', locator }
      : { outcome: 'not_found', effect: 'none', candidateId: options['candidate-id'] });
    return;
  }

  if (command === 'apply') {
    const operationPath = required(options, 'operation');
    if (!options.apply) {
      const operation = JSON.parse(readFileSync(operationPath, 'utf8'));
      print({
        effect: 'none',
        outcome: 'apply_required',
        operationId: operation.operationId,
        candidateDispositions: operation.request.candidateDispositions,
        worktreePath: operation.request.worktreePath,
        consequence: operation.request.candidateDispositions.some(
          ({ disposition }) => disposition === 'retain',
        )
          ? 'The worktree and every retained candidate remain.'
          : 'Selected archive or discard effects run before the exact worktree is revalidated and removed.',
      });
      return;
    }
    const repositoryRoot = required(options, 'repository-root');
    const operation = JSON.parse(readFileSync(operationPath, 'utf8'));
    assertOperationLocation(operationPath, operation.request.coordinationStateRoot);
    const adapter = createWorktreeCloseoutFilesystemAdapter({
      repositoryRoot,
      stateRoot: operation.request.coordinationStateRoot,
      archiveRoot: operation.request.archiveRoot,
    });
    print(executeWorktreeCloseout({
      operationPath,
      adapter,
      recover: options.recover === true,
    }));
    return;
  }

  fail('expected inspect, begin, lookup-archive, or apply');
}

try {
  main();
} catch (error) {
  print({
    outcome: 'error',
    code: error?.code ?? 'worktree_closeout_failed',
    message: error?.message ?? String(error),
    details: error?.details ?? null,
  });
  process.exitCode = 1;
}
