#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import {
  beginOrJoinWorktreeCloseout,
  executeWorktreeCloseout,
} from '../lib/worktree-closeout.js';
import {
  createWorktreeCloseoutFilesystemAdapter,
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

function main() {
  const { command, options } = parseArguments(process.argv.slice(2));
  if (command === 'inspect') {
    const repositoryRoot = resolve(options['repository-root'] ?? process.cwd());
    const inspection = inspectWorktreeCloseout({
      repositoryRoot,
      worktreePath: required(options, 'worktree'),
    });
    print({
      effect: 'none',
      request: {
        schemaVersion: 'agent-browser.worktree-closeout-request.v1',
        ...inspection,
        pinnedCandidates: [],
        candidateDispositions: [],
      },
      next: 'Add every pinned candidate and an explicit retain, archive, or discard disposition before begin.',
    });
    return;
  }

  if (command === 'begin') {
    const request = JSON.parse(readFileSync(required(options, 'request'), 'utf8'));
    print(beginOrJoinWorktreeCloseout({
      stateRoot: required(options, 'state-root'),
      request,
    }));
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
    const adapter = createWorktreeCloseoutFilesystemAdapter({
      repositoryRoot,
      stateRoot: required(options, 'state-root'),
      archiveRoot: required(options, 'archive-root'),
    });
    print(executeWorktreeCloseout({
      operationPath,
      adapter,
      recover: options.recover === true,
    }));
    return;
  }

  fail('expected inspect, begin, or apply');
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
