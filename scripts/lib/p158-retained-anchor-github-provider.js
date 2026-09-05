import { execFile as execFileCallback, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { promisify } from 'node:util';

import { selectExactDispatchedWorkflowRun } from './p158-retained-anchor-live-adapter.js';

const execFile = promisify(execFileCallback);

function abortError() {
  const error = new Error('Observation aborted');
  error.code = 'observation_aborted';
  return error;
}

async function poll({ observe, accept, timeoutMs, intervalMs = 1000, signal }) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() <= deadline) {
    if (signal?.aborted) throw abortError();
    const value = await observe();
    if (accept(value)) return value;
    await new Promise((resolveWait) => setTimeout(resolveWait, intervalMs));
  }
  const error = new Error('Observation timed out');
  error.code = 'observation_timeout';
  throw error;
}

async function gh(args, { signal, cwd } = {}) {
  const result = await execFile('gh', args, {
    encoding: 'utf8', ...(signal == null ? {} : { signal }), cwd,
    timeout: 60_000, maxBuffer: 10 * 1024 * 1024,
  });
  return result.stdout;
}

function readAnchorReceipts(root) {
  if (!existsSync(root)) return [];
  return readdirSync(root)
    .filter((name) => /^\d+-(ready|final)-receipt\.json$/.test(name))
    .map((name) => JSON.parse(readFileSync(join(root, name), 'utf8')));
}

export function createP158GitHubLiveProvider({ repoRoot, outputRoot, anchorEnv }) {
  const anchorRoot = resolve(outputRoot, 'anchor');
  const downloadsRoot = resolve(outputRoot, 'downloads');
  mkdirSync(outputRoot, { recursive: true, mode: 0o700 });
  if (readdirSync(outputRoot).length !== 0) throw new Error('Live adapter output directory must be empty');
  mkdirSync(anchorRoot, { mode: 0o700 });
  mkdirSync(downloadsRoot, { recursive: true, mode: 0o700 });
  const runGh = (args, options = {}) => gh(args, { ...options, cwd: repoRoot });
  const childStates = new WeakMap();
  return {
    async startAnchor({ runId, anchorId }) {
      const child = spawn(process.execPath, [join(repoRoot, 'scripts/run-p158-retained-authenticated-anchor.js')], {
        cwd: repoRoot,
        env: { ...anchorEnv, P158_RUN_ID: runId, P158_ANCHOR_ID: anchorId, P158_ANCHOR_OUTPUT_DIR: anchorRoot },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      const state = { exited: false, failed: false };
      childStates.set(child, state);
      child.on('error', () => { state.failed = true; });
      child.on('exit', () => { state.exited = true; });
      // Drain private child output without retaining it or allowing pipe backpressure.
      child.stdout.resume();
      child.stderr.resume();
      await new Promise((resolveSpawn, rejectSpawn) => {
        child.once('spawn', resolveSpawn);
        child.once('error', () => rejectSpawn(new Error('Anchor child startup failed')));
      });
      return child;
    },
    async waitForAnchorReceipts({ phase, child, timeoutMs, signal }) {
      return poll({
        observe: async () => {
          const receipts = readAnchorReceipts(anchorRoot);
          const stopped = childStates.get(child)?.exited || childStates.get(child)?.failed ||
            child?.exitCode != null || child?.signalCode != null;
          if (stopped && (phase === 'ready' || !receipts.some((receipt) => receipt.phase === phase))) {
            throw new Error('Anchor child stopped before receipt observation');
          }
          return receipts;
        },
        accept: (receipts) => receipts.filter((receipt) => receipt.phase === phase).length !== 0,
        timeoutMs,
        signal,
      });
    },
    async dispatchWorkflow({ workflowFile, branch, inputs, signal }) {
      const args = ['workflow', 'run', workflowFile, '--ref', branch];
      for (const [name, value] of Object.entries(inputs)) args.push('-f', `${name}=${value}`);
      await runGh(args, { signal });
    },
    async waitForDispatchedWorkflowRun({
      workflowFile, branch, runName, expectedCommit, dispatchedAfter, timeoutMs, signal,
    }) {
      return poll({
        observe: async () => JSON.parse(await runGh([
          'run', 'list', '--workflow', workflowFile, '--branch', branch, '--event',
          'workflow_dispatch', '--limit', '30', '--json',
          'databaseId,displayTitle,headSha,headBranch,event,status,conclusion,createdAt',
        ], { signal })),
        accept: (runs) => {
          try {
            return selectExactDispatchedWorkflowRun(runs, {
              runName, expectedCommit, branch, dispatchedAfter,
            });
          } catch (error) {
            if (error.code === 'workflow_run_not_found') return false;
            throw error;
          }
        },
        timeoutMs,
        signal,
      }).then((runs) => selectExactDispatchedWorkflowRun(runs, {
        runName, expectedCommit, branch, dispatchedAfter,
      }));
    },
    async waitForWorkflowTerminal({ workflowRunId, timeoutMs, signal }) {
      return poll({
        observe: async () => JSON.parse(await runGh([
          'run', 'view', workflowRunId, '--json', 'status,conclusion',
        ], { signal })),
        accept: (run) => run.status === 'completed',
        timeoutMs,
        intervalMs: 5000,
        signal,
      });
    },
    async downloadArtifact({ workflowRunId, artifactName, role, signal }) {
      const destination = resolve(downloadsRoot, role);
      mkdirSync(destination, { recursive: true, mode: 0o700 });
      if (readdirSync(destination).length !== 0) throw new Error('Artifact destination is not empty');
      await runGh(['run', 'download', workflowRunId, '-n', artifactName, '-D', destination], { signal });
      return destination;
    },
    async waitForAnchorExit({ child, timeoutMs }) {
      if (childStates.get(child)?.failed) throw new Error('Anchor child failed');
      if (childStates.get(child)?.exited || child.exitCode != null || child.signalCode != null) {
        if (child.exitCode !== 0 || child.signalCode != null) throw new Error('Anchor child exited abnormally');
        return;
      }
      await new Promise((resolveExit, rejectExit) => {
        let settled = false;
        const cleanup = () => {
          clearTimeout(timer);
          child.off('exit', onExit);
          child.off('error', onError);
        };
        const onExit = (code, signal) => {
          if (!settled) {
            settled = true;
            cleanup();
            if (code !== 0 || signal != null) rejectExit(new Error('Anchor child exited abnormally'));
            else resolveExit();
          }
        };
        const onError = () => { if (!settled) { settled = true; cleanup(); rejectExit(new Error('Anchor child failed')); } };
        const timer = setTimeout(() => {
          if (settled) return;
          settled = true;
          cleanup();
          child.kill('SIGKILL');
          const error = new Error('Anchor exit timed out and was forcibly closed');
          error.code = 'anchor_exit_timeout';
          rejectExit(error);
        }, timeoutMs);
        child.once('exit', onExit);
        child.once('error', onError);
      });
    },
    async emitAggregate(aggregate) {
      writeFileSync(resolve(outputRoot, 'p158-retained-anchor-external-aggregate.json'),
        `${JSON.stringify(aggregate, null, 2)}\n`, { encoding: 'utf8', mode: 0o600, flag: 'wx' });
    },
  };
}
