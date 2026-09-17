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
  executeWorktreeCloseout,
  WorktreeCloseoutConflictError,
} from './lib/worktree-closeout.js';
import {
  createWorktreeCloseoutFilesystemAdapter,
  inspectWorktreeCloseout,
  removeInspectedWorktree,
} from './lib/worktree-closeout-filesystem-adapter.js';
import { createCandidateBuildFilesystemAdapter } from './lib/candidate-build-filesystem-adapter.js';

const scriptPath = fileURLToPath(import.meta.url);
const closeoutCliPath = fileURLToPath(new URL('./dev/worktree-closeout.js', import.meta.url));

function installSyntheticCompletedCandidate(worktreePath, candidateId) {
  const sealedRoot = join(worktreePath, 'cli', 'target', candidateId, 'sealed');
  const completionRoot = join(
    worktreePath,
    'cli',
    'target',
    'candidate-build-state',
    'completed',
  );
  mkdirSync(sealedRoot, { recursive: true });
  mkdirSync(completionRoot, { recursive: true });
  const paths = {
    binaryPath: join(sealedRoot, 'agent-browser'),
    candidateManifestPath: join(sealedRoot, 'candidate-manifest.json'),
    inputClosurePath: join(sealedRoot, 'executable-input-closure.json'),
    supportManifestPath: join(sealedRoot, 'build-support-manifest.json'),
    sealedArtifactPath: join(sealedRoot, 'sealed-artifact.json'),
  };
  for (const [field, path] of Object.entries(paths)) writeFileSync(path, `${field}:${candidateId}\n`);
  writeFileSync(join(completionRoot, `${candidateId}.json`), `${JSON.stringify({
    schemaVersion: 'agent-browser.candidate-build-completion.v1',
    candidateId,
    ...paths,
  })}\n`);
}

if (process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER === 'begin') {
  const result = beginOrJoinWorktreeCloseout({
    stateRoot: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_STATE_ROOT,
    request: JSON.parse(process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_REQUEST),
  });
  process.send?.(result);
} else if (process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER === 'apply') {
  const adapter = createWorktreeCloseoutFilesystemAdapter({
    repositoryRoot: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_REPOSITORY_ROOT,
    stateRoot: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_STATE_ROOT,
    archiveRoot: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_ARCHIVE_ROOT,
  });
  const pause = process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_PAUSE === '1';
  const result = executeWorktreeCloseout({
    operationPath: process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_OPERATION_PATH,
    adapter,
    faultInjector(point) {
      if (pause && point === 'after_dispositions_before_removal') {
        process.send?.({ kind: 'barrier' });
        const waitArray = new Int32Array(new SharedArrayBuffer(4));
        while (!existsSync(process.env.AGENT_BROWSER_WORKTREE_CLOSEOUT_RELEASE_PATH)) {
          Atomics.wait(waitArray, 0, 0, 10);
        }
      }
    },
  });
  process.send?.({ kind: 'result', result });
} else {
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-worktree-closeout-'));
  try {
    const request = {
      schemaVersion: 'agent-browser.worktree-closeout-request.v1',
      repositoryId: 'fixture-repository',
      coordinationStateRoot: join(fixtureRoot, 'state'),
      archiveRoot: join(fixtureRoot, 'archives'),
      worktreeIncarnation: 'fixture-worktree-incarnation',
      worktreePath: join(fixtureRoot, 'worktree'),
      expectedHead: 'a'.repeat(40),
      expectedRef: 'refs/heads/fixture',
      activeCandidateBuilds: [],
      candidateDispositions: [],
    };
    const candidateContract = {
      candidateId: 'candidate-a',
      sourceRoot: join(fixtureRoot, 'worktree'),
      artifacts: [
        'cli/target/sealed/agent-browser',
        'cli/target/sealed/candidate-manifest.json',
        'cli/target/sealed/executable-input-closure.json',
        'cli/target/sealed/build-support-manifest.json',
        'cli/target/sealed/sealed-artifact.json',
        'cli/target/candidate-build-state/completed/request.json',
      ].map((relativePath) => ({ relativePath, sha256: '0'.repeat(64) })),
    };
    const runWorker = () => new Promise((resolve, reject) => {
      const child = fork(scriptPath, [], {
        env: {
          ...process.env,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER: 'begin',
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
          pinnedCandidates: [candidateContract],
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
          pinnedCandidates: [{ ...candidateContract, candidateId: 'candidate-pinned' }],
        },
      }),
      (error) => error instanceof CandidateDispositionRequiredError
        && error.code === 'worktree_closeout_candidate_disposition_required'
        && error.details.candidateIds[0] === 'candidate-pinned'
        && error.details.supportedDispositions.join(',') === 'retain,archive,discard',
      'a pinned candidate without a disposition must return supported choices without selecting an operation',
    );

    let malformedEffectCount = 0;
    const malformedOperationState = join(fixtureRoot, 'malformed-operation-state');
    const malformedOperation = beginOrJoinWorktreeCloseout({
      stateRoot: malformedOperationState,
      request: {
        ...request,
        coordinationStateRoot: malformedOperationState,
        archiveRoot: join(malformedOperationState, 'candidate-archives'),
      },
    });
    const tamperedOperation = JSON.parse(readFileSync(malformedOperation.operationPath, 'utf8'));
    tamperedOperation.request.expectedRef = 'refs/heads/tampered';
    writeFileSync(malformedOperation.operationPath, `${JSON.stringify(tamperedOperation)}\n`);
    assert.throws(
      () => executeWorktreeCloseout({
        operationPath: malformedOperation.operationPath,
        adapter: { removeWorktree() { malformedEffectCount += 1; } },
      }),
      (error) => error.code === 'worktree_closeout_operation_digest_mismatch',
    );
    assert.equal(malformedEffectCount, 0);

    const malformedEffectState = join(fixtureRoot, 'malformed-effect-state');
    const malformedEffectOperation = beginOrJoinWorktreeCloseout({
      stateRoot: malformedEffectState,
      request: {
        ...request,
        coordinationStateRoot: malformedEffectState,
        archiveRoot: join(malformedEffectState, 'candidate-archives'),
      },
    });
    writeFileSync(
      `${malformedEffectOperation.operationPath}.${malformedEffectOperation.operationId}.effect.1.json`,
      `${JSON.stringify({
        schemaVersion: 'agent-browser.worktree-closeout-effect.v1',
        operationId: malformedEffectOperation.operationId,
        generation: 1,
        ownerPid: 'not-an-integer',
      })}\n`,
    );
    assert.throws(
      () => executeWorktreeCloseout({
        operationPath: malformedEffectOperation.operationPath,
        adapter: { removeWorktree() { malformedEffectCount += 1; } },
        recover: true,
      }),
      (error) => error instanceof WorktreeCloseoutConflictError,
    );
    assert.equal(malformedEffectCount, 0);

    const candidateRoot = join(fixtureRoot, 'worktree', 'cli', 'target', 'sealed');
    mkdirSync(candidateRoot, { recursive: true });
    const artifacts = [
      ['agent-browser', Buffer.from('synthetic candidate binary\n')],
      ['candidate-manifest.json', Buffer.from('{"candidateId":"candidate-pinned"}\n')],
      ['executable-input-closure.json', Buffer.from('{"files":[]}\n')],
      ['build-support-manifest.json', Buffer.from('{"inputs":[]}\n')],
      ['sealed-artifact.json', Buffer.from('{"state":"sealed"}\n')],
      ['candidate-build-state/completed/request.json', Buffer.from('{"state":"qualified"}\n')],
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
    const laterReceiptBytes = Buffer.from('{"state":"later"}\n');
    writeFileSync(join(candidateRoot, 'candidate-build-state/completed/later.json'), laterReceiptBytes);
    assert.throws(
      () => freshAdapter.archiveCandidate({
        operationId: first.operationId,
        generation: 1,
        candidate: {
          ...candidate,
          artifacts: [...candidate.artifacts, {
            relativePath: 'candidate-build-state/completed/later.json',
            sha256: createHash('sha256').update(laterReceiptBytes).digest('hex'),
          }],
        },
      }),
      (error) => error.code === 'worktree_closeout_archive_request_mismatch',
      'an existing archive must match the complete requested artifact set',
    );
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
    writeFileSync(join(gitRepository, '.gitignore'), 'cli/target/\n');
    execFileSync('git', ['add', 'tracked.txt', '.gitignore'], { cwd: gitRepository });
    execFileSync('git', ['commit', '-m', 'fixture baseline'], { cwd: gitRepository });
    execFileSync('git', ['branch', 'closeout-fixture'], { cwd: gitRepository });
    execFileSync('git', ['worktree', 'add', gitWorktree, 'closeout-fixture'], { cwd: gitRepository });
    const inspection = inspectWorktreeCloseout({
      repositoryRoot: gitRepository,
      worktreePath: gitWorktree,
    });
    assert.equal(inspection.dirty, false);
    const cliInspection = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'inspect',
      '--repository-root',
      gitRepository,
      '--worktree',
      gitWorktree,
    ], { encoding: 'utf8' }));
    assert.equal(cliInspection.effect, 'none');
    assert.equal(cliInspection.request.worktreeIncarnation, inspection.worktreeIncarnation);
    assert.equal(cliInspection.request.pinnedCandidates.length, 0);

    const discoveredSealedRoot = join(gitWorktree, 'cli', 'target', 'candidate', 'sealed');
    const discoveredCompletionRoot = join(
      gitWorktree,
      'cli',
      'target',
      'candidate-build-state',
      'completed',
    );
    mkdirSync(discoveredSealedRoot, { recursive: true });
    mkdirSync(discoveredCompletionRoot, { recursive: true });
    const discoveredPaths = {
      binaryPath: join(discoveredSealedRoot, 'agent-browser'),
      candidateManifestPath: join(discoveredSealedRoot, 'candidate-manifest.json'),
      inputClosurePath: join(discoveredSealedRoot, 'executable-input-closure.json'),
      supportManifestPath: join(discoveredSealedRoot, 'build-support-manifest.json'),
      sealedArtifactPath: join(discoveredSealedRoot, 'sealed-artifact.json'),
    };
    for (const [field, path] of Object.entries(discoveredPaths)) {
      writeFileSync(path, `${field}\n`);
    }
    writeFileSync(join(discoveredCompletionRoot, 'request.json'), `${JSON.stringify({
      schemaVersion: 'agent-browser.candidate-build-completion.v1',
      candidateId: 'candidate-discovered',
      ...discoveredPaths,
    })}\n`);
    const discoveredInspection = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'inspect',
      '--repository-root',
      gitRepository,
      '--worktree',
      gitWorktree,
    ], { encoding: 'utf8' }));
    assert.equal(discoveredInspection.request.pinnedCandidates.length, 1);
    assert.equal(
      discoveredInspection.request.pinnedCandidates[0].candidateId,
      'candidate-discovered',
    );
    assert.equal(discoveredInspection.request.pinnedCandidates[0].artifacts.length, 6);
    const activeClaimsRoot = join(
      gitWorktree,
      'cli',
      'target',
      'candidate-build-state',
      'claims',
    );
    mkdirSync(activeClaimsRoot, { recursive: true });
    writeFileSync(join(activeClaimsRoot, 'active.json'), `${JSON.stringify({
      schemaVersion: 'agent-browser.candidate-build-claim.v1',
      operationId: 'candidate-build-active-fixture',
      requestDigest: 'f'.repeat(64),
      state: 'building',
      ownerPid: process.pid,
    })}\n`);
    const activeBuildInspection = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'inspect',
      '--repository-root',
      gitRepository,
      '--worktree',
      gitWorktree,
    ], { encoding: 'utf8' }));
    assert.equal(activeBuildInspection.request.activeCandidateBuilds.length, 1);
    assert.equal(
      activeBuildInspection.request.activeCandidateBuilds[0].operationId,
      'candidate-build-active-fixture',
    );
    rmSync(join(gitWorktree, 'cli'), { recursive: true, force: true });

    writeFileSync(join(gitWorktree, 'tracked.txt'), 'drifted\n');
    assert.throws(
      () => removeInspectedWorktree({ repositoryRoot: gitRepository, inspection }),
      (error) => error.code === 'worktree_closeout_revalidation_conflict'
        && error.message.endsWith(':dirtyStateDigest'),
      'worktree drift must stop removal before any Git effect',
    );
    assert.equal(existsSync(gitWorktree), true);
    writeFileSync(join(gitWorktree, 'tracked.txt'), 'baseline\n');
    const removalRequest = {
      schemaVersion: 'agent-browser.worktree-closeout-request.v1',
      ...inspection,
      activeCandidateBuilds: [],
      pinnedCandidates: [],
      candidateDispositions: [],
    };
    assert.throws(
      () => beginOrJoinWorktreeCloseout({
        stateRoot: join(fixtureRoot, 'split-brain-state'),
        request: removalRequest,
      }),
      /stateRoot must match request.coordinationStateRoot/,
      'a caller-selected state root must not split repository coordination',
    );
    const removalOperation = beginOrJoinWorktreeCloseout({
      stateRoot: removalRequest.coordinationStateRoot,
      request: removalRequest,
    });
    const removalAdapter = createWorktreeCloseoutFilesystemAdapter({
      stateRoot: removalRequest.coordinationStateRoot,
      archiveRoot: removalRequest.archiveRoot,
      repositoryRoot: gitRepository,
    });
    const preview = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'apply',
      '--operation',
      removalOperation.operationPath,
    ], { encoding: 'utf8' }));
    assert.equal(preview.effect, 'none');
    assert.equal(preview.outcome, 'apply_required');
    assert.equal(existsSync(gitWorktree), true);
    assert.throws(
      () => executeWorktreeCloseout({
        operationPath: removalOperation.operationPath,
        adapter: removalAdapter,
        faultInjector(point) {
          if (point === 'after_removal_before_receipt') throw new Error('fixture_post_removal_crash');
        },
      }),
      /fixture_post_removal_crash/,
    );
    assert.equal(existsSync(gitWorktree), false);
    const recoveredRemoval = executeWorktreeCloseout({
      operationPath: removalOperation.operationPath,
      adapter: removalAdapter,
      recover: true,
    });
    assert.equal(recoveredRemoval.outcome, 'committed');
    assert.equal(recoveredRemoval.receipt.generation, 2);
    assert.equal(recoveredRemoval.receipt.removal.outcome, 'already_removed');
    const replayedRemoval = executeWorktreeCloseout({
      operationPath: removalOperation.operationPath,
      adapter: removalAdapter,
    });
    assert.equal(replayedRemoval.outcome, 'replayed_terminal');
    assert.deepEqual(replayedRemoval.receipt, recoveredRemoval.receipt);
    const cliReplayAfterRemoval = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'apply',
      '--operation',
      removalOperation.operationPath,
      '--repository-root',
      gitRepository,
      '--apply',
      '--recover',
    ], { encoding: 'utf8' }));
    assert.equal(cliReplayAfterRemoval.outcome, 'replayed_terminal');

    const concurrentWorktree = join(fixtureRoot, 'concurrent-worktree');
    execFileSync('git', ['branch', 'concurrent-fixture'], { cwd: gitRepository });
    execFileSync('git', ['worktree', 'add', concurrentWorktree, 'concurrent-fixture'], {
      cwd: gitRepository,
    });
    const concurrentInspection = inspectWorktreeCloseout({
      repositoryRoot: gitRepository,
      worktreePath: concurrentWorktree,
    });
    const concurrentRequest = {
      schemaVersion: 'agent-browser.worktree-closeout-request.v1',
      ...concurrentInspection,
      activeCandidateBuilds: [],
      pinnedCandidates: [],
      candidateDispositions: [],
    };
    const concurrentOperation = beginOrJoinWorktreeCloseout({
      stateRoot: concurrentRequest.coordinationStateRoot,
      request: concurrentRequest,
    });
    const candidateBuildAdapter = createCandidateBuildFilesystemAdapter({
      repoRoot: concurrentWorktree,
      stateRoot: join(concurrentWorktree, 'cli', 'target', 'candidate-build-state'),
    });
    await assert.rejects(
      () => candidateBuildAdapter.acquireClaim({
        requestDigest: 'e'.repeat(64),
        outputDirectory: join(concurrentWorktree, 'cli', 'target', 'candidate'),
      }),
      (error) => error.code === 'candidate_build_worktree_closeout_active',
      'a candidate build must not publish a claim after closeout selection',
    );
    const releasePath = join(fixtureRoot, 'release-concurrent-removal');
    const spawnApplyWorker = (pause) => {
      const child = fork(scriptPath, [], {
        env: {
          ...process.env,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_FIXTURE_WORKER: 'apply',
          AGENT_BROWSER_WORKTREE_CLOSEOUT_OPERATION_PATH: concurrentOperation.operationPath,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_REPOSITORY_ROOT: gitRepository,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_STATE_ROOT: concurrentRequest.coordinationStateRoot,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_ARCHIVE_ROOT: concurrentRequest.archiveRoot,
          AGENT_BROWSER_WORKTREE_CLOSEOUT_PAUSE: pause ? '1' : '0',
          AGENT_BROWSER_WORKTREE_CLOSEOUT_RELEASE_PATH: releasePath,
        },
        stdio: ['ignore', 'ignore', 'inherit', 'ipc'],
      });
      let signalBarrier;
      const barrier = new Promise((resolveBarrier) => { signalBarrier = resolveBarrier; });
      const result = new Promise((resolveResult, rejectResult) => {
        child.on('message', (message) => {
          if (message.kind === 'barrier') signalBarrier();
          if (message.kind === 'result') resolveResult(message.result);
        });
        child.once('error', rejectResult);
        child.once('exit', (code) => {
          if (code !== 0) rejectResult(new Error(`apply worker exited ${code}`));
        });
      });
      return { barrier, result };
    };
    const firstApply = spawnApplyWorker(true);
    await firstApply.barrier;
    const joiningApply = spawnApplyWorker(false);
    const joined = await joiningApply.result;
    assert.equal(joined.outcome, 'joined_in_progress');
    writeFileSync(releasePath, 'release\n');
    const committed = await firstApply.result;
    assert.equal(committed.outcome, 'committed');
    assert.equal(committed.receipt.removal.outcome, 'removed');
    const replayWorker = spawnApplyWorker(false);
    const replayed = await replayWorker.result;
    assert.equal(replayed.outcome, 'replayed_terminal');
    assert.equal(replayed.receipt.operationId, committed.receipt.operationId);
    assert.equal(existsSync(concurrentWorktree), false);

    const retainedWorktree = join(fixtureRoot, 'retained-worktree');
    execFileSync('git', ['branch', 'retained-fixture'], { cwd: gitRepository });
    execFileSync('git', ['worktree', 'add', retainedWorktree, 'retained-fixture'], {
      cwd: gitRepository,
    });
    installSyntheticCompletedCandidate(retainedWorktree, 'candidate-retained');
    const retainedInspection = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'inspect',
      '--repository-root',
      gitRepository,
      '--worktree',
      retainedWorktree,
    ], { encoding: 'utf8' })).request;
    retainedInspection.candidateDispositions = [{
      candidateId: 'candidate-retained',
      disposition: 'retain',
    }];
    const retainOperation = beginOrJoinWorktreeCloseout({
      stateRoot: retainedInspection.coordinationStateRoot,
      request: retainedInspection,
    });
    const retainedAdapter = createWorktreeCloseoutFilesystemAdapter({
      repositoryRoot: gitRepository,
      stateRoot: retainedInspection.coordinationStateRoot,
      archiveRoot: retainedInspection.archiveRoot,
    });
    const retained = executeWorktreeCloseout({
      operationPath: retainOperation.operationPath,
      adapter: retainedAdapter,
    });
    assert.equal(retained.receipt.removal.outcome, 'retained');
    assert.equal(existsSync(retainedWorktree), true);

    const archiveAfterRetainRequest = {
      ...retainedInspection,
      supersedesRetainedOperationId: retainOperation.operationId,
      candidateDispositions: [{
        candidateId: 'candidate-retained',
        disposition: 'archive',
      }],
    };
    const archiveAfterRetainOperation = beginOrJoinWorktreeCloseout({
      stateRoot: archiveAfterRetainRequest.coordinationStateRoot,
      request: archiveAfterRetainRequest,
    });
    assert.equal(
      archiveAfterRetainOperation.supersededRetainedOperationId,
      retainOperation.operationId,
    );
    assert.throws(
      () => beginOrJoinWorktreeCloseout({
        stateRoot: archiveAfterRetainRequest.coordinationStateRoot,
        request: {
          ...archiveAfterRetainRequest,
          candidateDispositions: [{
            candidateId: 'candidate-retained',
            disposition: 'discard',
          }],
        },
      }),
      (error) => error instanceof WorktreeCloseoutConflictError,
      'a conflicting retained-operation replacement must not acquire a second operation',
    );
    const archivedAfterRetain = executeWorktreeCloseout({
      operationPath: archiveAfterRetainOperation.operationPath,
      adapter: retainedAdapter,
    });
    assert.equal(archivedAfterRetain.receipt.removal.outcome, 'removed');
    assert.equal(existsSync(retainedWorktree), false);
    const retainedLocator = retainedAdapter.resolveArchivedCandidate('candidate-retained');
    assert.equal(retainedLocator.candidateId, 'candidate-retained');
    const freshLookup = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'lookup-archive',
      '--repository-root',
      gitRepository,
      '--candidate-id',
      'candidate-retained',
    ], { encoding: 'utf8' }));
    assert.equal(freshLookup.outcome, 'found');
    assert.deepEqual(freshLookup.locator.artifacts, retainedLocator.artifacts);
    writeFileSync(
      join(retainedLocator.archiveRoot, retainedLocator.artifacts[0].relativePath),
      'corrupted after terminal receipt\n',
    );
    assert.throws(
      () => executeWorktreeCloseout({
        operationPath: archiveAfterRetainOperation.operationPath,
        adapter: retainedAdapter,
      }),
      (error) => error.code === 'worktree_closeout_archive_digest_mismatch',
      'terminal replay must freshly verify archived candidate custody',
    );

    const discardedWorktree = join(fixtureRoot, 'discarded-worktree');
    execFileSync('git', ['branch', 'discarded-fixture'], { cwd: gitRepository });
    execFileSync('git', ['worktree', 'add', discardedWorktree, 'discarded-fixture'], {
      cwd: gitRepository,
    });
    installSyntheticCompletedCandidate(discardedWorktree, 'candidate-e2e-discard');
    const discardRequest = JSON.parse(execFileSync(process.execPath, [
      closeoutCliPath,
      'inspect',
      '--repository-root',
      gitRepository,
      '--worktree',
      discardedWorktree,
    ], { encoding: 'utf8' })).request;
    discardRequest.candidateDispositions = [{
      candidateId: 'candidate-e2e-discard',
      disposition: 'discard',
    }];
    const discardOperation = beginOrJoinWorktreeCloseout({
      stateRoot: discardRequest.coordinationStateRoot,
      request: discardRequest,
    });
    const discardAdapter = createWorktreeCloseoutFilesystemAdapter({
      repositoryRoot: gitRepository,
      stateRoot: discardRequest.coordinationStateRoot,
      archiveRoot: discardRequest.archiveRoot,
    });
    const discardCloseout = executeWorktreeCloseout({
      operationPath: discardOperation.operationPath,
      adapter: discardAdapter,
    });
    assert.equal(discardCloseout.receipt.removal.outcome, 'removed');
    assert.equal(existsSync(discardedWorktree), false);
    assert.equal(discardAdapter.resolveArchivedCandidate('candidate-e2e-discard'), null);
    execFileSync('git', ['show-ref', '--verify', 'refs/heads/discarded-fixture'], {
      cwd: gitRepository,
    });
  } finally {
    rmSync(fixtureRoot, { recursive: true, force: true });
  }
  console.log('worktree closeout tests passed');
}
