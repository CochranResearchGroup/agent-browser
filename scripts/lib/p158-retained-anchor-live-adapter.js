import { createHash } from 'node:crypto';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

import { coordinateRetainedAnchorExternalCampaign } from './p158-retained-anchor-coordinator.js';

export const P158_EXTERNAL_WORKFLOW_FILE = 'p158-external-vantage.yml';
export const P158_EXTERNAL_ARTIFACTS = Object.freeze({
  human: 'p158-external-runner-human',
  slow: 'p158-external-runner-slow',
  aggregate: 'p158-external-vantage-receipt',
});
export const P158_WORKFLOW_CREATED_AT_TOLERANCE_MS = 2_000;

function sha256(value) {
  return createHash('sha256').update(String(value)).digest('hex');
}

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
}

function safeId(value, label, pattern) {
  if (typeof value !== 'string' || !pattern.test(value)) throw new Error(`Invalid ${label}`);
  return value;
}

export function p158ExternalRunName(runId, expectedCommit) {
  safeId(runId, 'campaign run ID', /^[a-zA-Z0-9._:-]+$/);
  safeId(expectedCommit, 'expected commit', /^[a-f0-9]{40}$/);
  return `p158-${runId}-${expectedCommit}`;
}

export function selectExactDispatchedWorkflowRun(runs, {
  runName, expectedCommit, branch, dispatchedAfter,
}) {
  const threshold = Date.parse(dispatchedAfter);
  if (!Array.isArray(runs) || !Number.isFinite(threshold)) {
    throw new Error('Invalid workflow run observations');
  }
  const matches = runs.filter((run) =>
    String(run.databaseId ?? '').match(/^\d+$/) &&
    run.displayTitle === runName && run.headSha === expectedCommit &&
    run.headBranch === branch && run.event === 'workflow_dispatch' &&
    Number.isFinite(Date.parse(run.createdAt)) &&
    Date.parse(run.createdAt) >= threshold - P158_WORKFLOW_CREATED_AT_TOLERANCE_MS);
  if (matches.length !== 1) {
    const error = new Error('Dispatched workflow run identity is ambiguous');
    error.code = matches.length === 0 ? 'workflow_run_not_found' : 'workflow_run_ambiguous';
    throw error;
  }
  return { workflowRunId: String(matches[0].databaseId) };
}

function listFiles(root) {
  const files = [];
  for (const name of readdirSync(root)) {
    const path = join(root, name);
    if (statSync(path).isDirectory()) files.push(...listFiles(path));
    else files.push(path);
  }
  return files;
}

export function readExactDownloadedReceipt(root, role) {
  const allowed = role === 'aggregate'
    ? new Set(['p158-external-vantage-receipt.json'])
    : new Set(['receipt.json', 'failure-receipt.json']);
  const candidates = listFiles(root).filter((path) => allowed.has(path.split('/').at(-1)));
  if (candidates.length !== 1) {
    const error = new Error('Downloaded workflow artifact has missing or duplicate receipt evidence');
    error.code = candidates.length === 0 ? 'artifact_receipt_missing' : 'artifact_receipt_duplicate';
    throw error;
  }
  return JSON.parse(readFileSync(candidates[0], 'utf8'));
}

export async function runP158RetainedAnchorLiveAdapter({ config, provider, signal = null }) {
  const runId = safeId(config?.runId, 'campaign run ID', /^[a-zA-Z0-9._:-]+$/);
  const anchorId = safeId(config?.anchorId, 'anchor ID', /^[a-zA-Z0-9._:-]+$/);
  const expectedCommit = safeId(config?.expectedCommit, 'expected commit', /^[a-f0-9]{40}$/);
  const branch = safeId(config?.branch, 'branch', /^[a-zA-Z0-9._\/-]+$/);
  const handoffUrlSha256 = safeId(config?.handoffUrlSha256, 'handoff digest', /^[a-f0-9]{64}$/);
  if (!['readiness', 'calibration'].includes(config.probeMode)) throw new Error('Invalid probe mode');
  if (!Number.isInteger(config.artifactRetentionDays) || config.artifactRetentionDays < 1 ||
      config.artifactRetentionDays > 3) throw new Error('Invalid artifact retention');
  if (typeof config.calibrationStartAt !== 'string' ||
      !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?Z$/.test(config.calibrationStartAt) ||
      !Number.isFinite(Date.parse(config.calibrationStartAt))) {
    throw new Error('Invalid calibration timestamp');
  }
  for (const name of ['anchorTimeoutMs', 'runIdentityTimeoutMs', 'workflowTimeoutMs', 'anchorExitTimeoutMs']) {
    if (!Number.isSafeInteger(config[name]) || config[name] < 1) throw new Error(`Invalid ${name}`);
  }
  const runName = p158ExternalRunName(runId, expectedCommit);
  const required = [
    'startAnchor', 'waitForAnchorReceipts', 'dispatchWorkflow', 'waitForDispatchedWorkflowRun',
    'waitForWorkflowTerminal', 'downloadArtifact', 'waitForAnchorExit', 'emitAggregate',
  ];
  if (required.some((name) => typeof provider?.[name] !== 'function')) {
    throw new Error('Incomplete P158 live adapter provider');
  }

  const workflowObservation = { workflowRunId: null, expectedCommit, conclusion: null,
    failureCode: null, artifacts: {} };
  let emittedAggregate;
  await coordinateRetainedAnchorExternalCampaign({
    runId,
    anchorId,
    handoffUrlSha256,
    startAnchor: () => provider.startAnchor({ runId, anchorId, signal }),
    waitForAnchorReceipts: ({ phase, child }) => provider.waitForAnchorReceipts({
      phase,
      child,
      timeoutMs: config.anchorTimeoutMs,
      signal: phase === 'ready' ? signal : null,
    }),
    dispatchExternalCampaign: async ({ anchorReadyReceiptSha256 }) => {
      const dispatchedAfter = new Date().toISOString();
      await provider.dispatchWorkflow({
        workflowFile: P158_EXTERNAL_WORKFLOW_FILE,
        branch,
        inputs: {
          campaign_run_id: runId,
          expected_commit: expectedCommit,
          calibration_start_at: config.calibrationStartAt,
          artifact_retention_days: String(config.artifactRetentionDays),
          probe_mode: config.probeMode,
          w8_action_manifest_sha256: config.w8ActionManifestSha256 ?? '',
          w8_action_manifest_artifact_run_id: config.w8ActionManifestArtifactRunId ?? '',
          w8_action_manifest_artifact_name: config.w8ActionManifestArtifactName ?? '',
        },
        anchorReadyReceiptSha256,
        signal,
      });
      const { workflowRunId } = await provider.waitForDispatchedWorkflowRun({
        workflowFile: P158_EXTERNAL_WORKFLOW_FILE,
        branch,
        runName,
        expectedCommit,
        dispatchedAfter,
        timeoutMs: config.runIdentityTimeoutMs,
        signal,
      });
      workflowObservation.workflowRunId = workflowRunId;
      let terminal = null;
      try {
        terminal = await provider.waitForWorkflowTerminal({
          workflowRunId, timeoutMs: config.workflowTimeoutMs, signal,
        });
        workflowObservation.conclusion = terminal.conclusion;
      } catch (error) {
        workflowObservation.failureCode = ['observation_timeout', 'observation_aborted'].includes(error.code)
          ? error.code : 'workflow_observation_failed';
      }
      const receipts = {};
      for (const [role, artifactName] of Object.entries(P158_EXTERNAL_ARTIFACTS)) {
        // Retain every available artifact, including after interruption or a sibling download failure.
        let destination;
        try {
          destination = await provider.downloadArtifact({ workflowRunId, artifactName, role, signal: null });
        } catch {
          workflowObservation.artifacts[role] = { downloaded: false, failureCode: 'artifact_download_failed' };
          continue;
        }
        try {
          receipts[role] = readExactDownloadedReceipt(destination, role);
          workflowObservation.artifacts[role] = { downloaded: true, failureCode: null };
        } catch {
          workflowObservation.artifacts[role] = { downloaded: true, failureCode: 'artifact_receipt_invalid' };
        }
      }
      return {
        workflowRunId,
        workflowConclusion: terminal?.conclusion ?? null,
        humanReceipt: receipts.human,
        slowReceipt: receipts.slow,
        externalAggregate: receipts.aggregate,
      };
    },
    waitForAnchorExit: (child) => provider.waitForAnchorExit({
      child, timeoutMs: config.anchorExitTimeoutMs,
    }),
    emitAggregate: async (aggregate) => {
      const { aggregateSha256: _previousHash, ...body } = aggregate;
      body.bindings.workflowRunId = workflowObservation.workflowRunId;
      body.workflowObservation = workflowObservation;
      emittedAggregate = { ...body, aggregateSha256: sha256(JSON.stringify(canonicalize(body))) };
      await provider.emitAggregate(emittedAggregate);
    },
    sensitiveValues: config.sensitiveValues ?? [],
  });
  return emittedAggregate;
}

export function handoffDigest(handoffUrl) {
  return sha256(handoffUrl);
}
