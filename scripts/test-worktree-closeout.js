#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, fork } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  beginOrJoinWorktreeCloseout,
  CandidateDispositionRequiredError,
  WorktreeCloseoutConflictError,
} from './lib/worktree-closeout.js';
import {
  createWorktreeCloseoutFilesystemAdapter,
  inspectWorktreeCloseout,
  removeInspectedWorktree,
} from './lib/worktree-closeout-filesystem-adapter.js';

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
          pinnedCandidates: [{ candidateId: 'candidate-a' }],
          candidateDispositions: [{ candidateId: 'candidate-a', disposition: 'archive' }],
        },
      }),
      (error) => error instanceof WorktreeCloseoutConflictError
        && error.code === 'worktree_closeout_request_conflict'
        && error.details.operationId === first.operationId,
      'a changed disposition must not join an already selected operation',
    );

    assert.throws(
      () => beginOrJoinWorktreeCloseout({
        stateRoot: join(fixtureRoot, 'unselected-state'),
        request: {
          ...request,
          pinnedCandidates: [{ candidateId: 'candidate-pinned' }],
        },
      }),
      (error) => error instanceof CandidateDispositionRequiredError
        && error.code === 'worktree_closeout_candidate_disposition_required'
        && error.details.candidateIds[0] === 'candidate-pinned'
        && error.details.supportedDispositions.join(',') === 'retain,archive,discard',
      'a pinned candidate without a disposition must return supported choices without selecting an operation',
    );

    const candidateRoot = join(fixtureRoot, 'worktree', 'cli', 'target', 'sealed');
    mkdirSync(candidateRoot, { recursive: true });
    const artifacts = [
      ['agent-browser', Buffer.from('synthetic candidate binary\n')],
      ['candidate-manifest.json', Buffer.from('{"candidateId":"candidate-pinned"}\n')],
      ['nested/receipt.json', Buffer.from('{"state":"qualified"}\n')],
    ].map(([relativePath, bytes]) => {
      const path = join(candidateRoot, relativePath);
      mkdirSync(join(path, '..'), { recursive: true });
      writeFileSync(path, bytes);
      return {
        relativePath,
        sha256: createHash('sha256').update(bytes).digest('hex'),
      };
    });
    const candidate = { candidateId: 'candidate-pinned', sourceRoot: candidateRoot, artifacts };
    let faultOnce = true;
    const archiveOptions = {
      stateRoot: join(fixtureRoot, 'archive-state'),
      archiveRoot: join(fixtureRoot, 'archives'),
    };
    const interruptedAdapter = createWorktreeCloseoutFilesystemAdapter({
      ...archiveOptions,
      faultInjector(point) {
        if (point === 'after_copy_before_publish' && faultOnce) {
          faultOnce = false;
          throw new Error('fixture_interruption');
        }
      },
    });
    assert.throws(
      () => interruptedAdapter.archiveCandidate({
        operationId: first.operationId,
        generation: 1,
        candidate,
      }),
      /fixture_interruption/,
    );
    assert.equal(
      interruptedAdapter.resolveArchivedCandidate(candidate.candidateId),
      null,
      'a partial archive must not be discoverable',
    );
    const recovered = interruptedAdapter.archiveCandidate({
      operationId: first.operationId,
      generation: 1,
      candidate,
    });
    assert.equal(recovered.outcome, 'archived');
    assert.equal(existsSync(candidateRoot), true, 'archive publication must not remove the source');

    const freshAdapter = createWorktreeCloseoutFilesystemAdapter(archiveOptions);
    const resolved = freshAdapter.resolveArchivedCandidate(candidate.candidateId);
    assert.equal(resolved.operationId, first.operationId);
    for (const artifact of artifacts) {
      assert.deepEqual(
        readFileSync(join(resolved.archiveRoot, artifact.relativePath)),
        readFileSync(join(candidateRoot, artifact.relativePath)),
        `archived bytes must match for ${artifact.relativePath}`,
      );
    }
    const discardRoot = join(fixtureRoot, 'discard-candidate');
    mkdirSync(discardRoot, { recursive: true });
    const discardBytes = Buffer.from('discard only this artifact\n');
    writeFileSync(join(discardRoot, 'selected.bin'), discardBytes);
    writeFileSync(join(discardRoot, 'unrelated.bin'), 'must remain\n');
    const discarded = freshAdapter.discardCandidate({
      candidateId: 'candidate-discard',
      sourceRoot: discardRoot,
      artifacts: [{
        relativePath: 'selected.bin',
        sha256: createHash('sha256').update(discardBytes).digest('hex'),
      }],
    });
    assert.equal(discarded.outcome, 'discarded');
    assert.equal(existsSync(join(discardRoot, 'selected.bin')), false);
    assert.equal(existsSync(join(discardRoot, 'unrelated.bin')), true);

    const gitRepository = join(fixtureRoot, 'git-repository');
    const gitWorktree = join(fixtureRoot, 'git-worktree');
    execFileSync('git', ['init', '--initial-branch=main', gitRepository]);
    execFileSync('git', ['config', 'user.email', 'fixture@example.invalid'], { cwd: gitRepository });
    execFileSync('git', ['config', 'user.name', 'Fixture'], { cwd: gitRepository });
    writeFileSync(join(gitRepository, 'tracked.txt'), 'baseline\n');
    execFileSync('git', ['add', 'tracked.txt'], { cwd: gitRepository });
    execFileSync('git', ['commit', '-m', 'fixture baseline'], { cwd: gitRepository });
    execFileSync('git', ['branch', 'closeout-fixture'], { cwd: gitRepository });
    execFileSync('git', ['worktree', 'add', gitWorktree, 'closeout-fixture'], { cwd: gitRepository });
    const inspection = inspectWorktreeCloseout({
      repositoryRoot: gitRepository,
      worktreePath: gitWorktree,
    });
    assert.equal(inspection.dirty, false);
    writeFileSync(join(gitWorktree, 'tracked.txt'), 'drifted\n');
    assert.throws(
      () => removeInspectedWorktree({ repositoryRoot: gitRepository, inspection }),
      (error) => error.code === 'worktree_closeout_revalidation_conflict'
        && error.message.endsWith(':dirtyStateDigest'),
      'worktree drift must stop removal before any Git effect',
    );
    assert.equal(existsSync(gitWorktree), true);
    writeFileSync(join(gitWorktree, 'tracked.txt'), 'baseline\n');
    const removal = removeInspectedWorktree({ repositoryRoot: gitRepository, inspection });
    assert.equal(removal.outcome, 'removed');
    assert.equal(existsSync(gitWorktree), false);
  } finally {
    rmSync(fixtureRoot, { recursive: true, force: true });
  }
  console.log('worktree closeout tests passed');
}
