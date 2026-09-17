#!/usr/bin/env node

import assert from 'node:assert/strict';
import { fork } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  beginOrJoinWorktreeCloseout,
  WorktreeCloseoutConflictError,
} from './lib/worktree-closeout.js';

const scriptPath = fileURLToPath(import.meta.url);

if (process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER === '1') {
  const result = beginOrJoinWorktreeCloseout({
    stateRoot: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_STATE_ROOT,
    request: JSON.parse(process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_REQUEST),
  });
  process.send?.(result);
} else {
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-worktree-closeout-'));
  try {
    const request = {
      schemaVersion: 'agent-browser.worktree-closeout-request.v1',
      repositoryId: 'fixture-repository',
      worktreeIncarnation: 'fixture-worktree-incarnation',
      worktreePath: join(fixtureRoot, 'worktree'),
      expectedHead: 'a'.repeat(40),
      expectedRef: 'refs/heads/fixture',
      candidateDispositions: [],
    };
    const runWorker = () => new Promise((resolve, reject) => {
      const child = fork(scriptPath, [], {
        env: {
          ...process.env,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER: '1',
          AGENT_BROWSER_WORKTREE_CLOSEOUT_STATE_ROOT: join(fixtureRoot, 'state'),
          AGENT_BROWSER_WORKTREE_CLOSEOUT_REQUEST: JSON.stringify(request),
        },
        stdio: ['ignore', 'ignore', 'inherit', 'ipc'],
      });
      child.once('message', resolve);
      child.once('error', reject);
      child.once('exit', (code) => {
        if (code !== 0) reject(new Error(`fixture worker exited ${code}`));
      });
    });

    const [first, second] = await Promise.all([runWorker(), runWorker()]);
    assert.equal(
      second.operationId,
      first.operationId,
      'two processes must join one durable closeout operation',
    );
    assert.deepEqual(
      new Set([first.outcome, second.outcome]),
      new Set(['started', 'joined_existing']),
      'one process starts and one process joins the closeout',
    );
    const replay = beginOrJoinWorktreeCloseout({
      stateRoot: join(fixtureRoot, 'state'),
      request,
    });
    assert.equal(replay.outcome, 'joined_existing');
    assert.equal(replay.operationId, first.operationId);

    const durableOperation = JSON.parse(readFileSync(first.operationPath, 'utf8'));
    assert.equal(durableOperation.operationId, first.operationId);
    assert.equal(durableOperation.requestDigest, first.requestDigest);
    assert.equal(durableOperation.generation, 1);
    assert.equal(durableOperation.phase, 'selected');

    assert.throws(
      () => beginOrJoinWorktreeCloseout({
        stateRoot: join(fixtureRoot, 'state'),
        request: {
          ...request,
          candidateDispositions: [{ candidateId: 'candidate-a', disposition: 'archive' }],
        },
      }),
      (error) => error instanceof WorktreeCloseoutConflictError
        && error.code === 'worktree_closeout_request_conflict'
        && error.details.operationId === first.operationId,
      'a changed disposition must not join an already selected operation',
    );
  } finally {
    rmSync(fixtureRoot, { recursive: true, force: true });
  }
  console.log('worktree closeout tests passed');
}
