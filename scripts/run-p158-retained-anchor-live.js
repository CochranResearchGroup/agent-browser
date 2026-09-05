#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  handoffDigest,
  runP158RetainedAnchorLiveAdapter,
} from './lib/p158-retained-anchor-live-adapter.js';
import { createP158GitHubLiveProvider } from './lib/p158-retained-anchor-github-provider.js';

const repoRoot = realpathSync(fileURLToPath(new URL('..', import.meta.url)));

function git(...args) {
  return execFileSync('git', args, { encoding: 'utf8', cwd: repoRoot }).trim();
}

function positiveInteger(value, fallback) {
  const parsed = Number.parseInt(value ?? '', 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
}

const controller = new AbortController();
process.once('SIGINT', () => controller.abort());
process.once('SIGTERM', () => controller.abort());

async function main() {
  const branch = git('branch', '--show-current');
  const head = git('rev-parse', 'HEAD');
  if (!branch || head !== process.env.P158_CANDIDATE_COMMIT) {
    throw new Error('Current branch and HEAD do not match the authorized candidate');
  }
  if (git('status', '--porcelain', '--untracked-files=all', '--', 'scripts',
    '.github/workflows/p158-external-vantage.yml', 'package.json', 'pnpm-lock.yaml')) {
    throw new Error('Executed campaign source must match the committed candidate');
  }
  if (!process.env.P158_LIVE_OUTPUT_DIR || !isAbsolute(process.env.P158_LIVE_OUTPUT_DIR)) {
    throw new Error('An explicit absolute external live output directory is required');
  }
  const requestedOutput = resolve(process.env.P158_LIVE_OUTPUT_DIR);
  let existingParent = requestedOutput;
  while (!existsSync(existingParent)) existingParent = dirname(existingParent);
  const outputRoot = resolve(realpathSync(existingParent), relative(existingParent, requestedOutput));
  const withinRepo = relative(repoRoot, outputRoot);
  if (!withinRepo || (!withinRepo.startsWith('../') && !isAbsolute(withinRepo))) {
    throw new Error('Live output must stay outside the product repository');
  }
  mkdirSync(outputRoot, { recursive: true, mode: 0o700 });
  const provider = createP158GitHubLiveProvider({ repoRoot, outputRoot, anchorEnv: process.env });
  const aggregate = await runP158RetainedAnchorLiveAdapter({
    config: {
      runId: process.env.P158_RUN_ID,
      anchorId: process.env.P158_ANCHOR_ID,
      expectedCommit: head,
      branch,
      handoffUrlSha256: handoffDigest(process.env.P158_DEV_HANDOFF_URL),
      calibrationStartAt: process.env.P158_CALIBRATION_START_AT,
      probeMode: process.env.P158_PROBE_MODE || 'readiness',
      artifactRetentionDays: positiveInteger(process.env.P158_ARTIFACT_RETENTION_DAYS, 2),
      anchorTimeoutMs: positiveInteger(process.env.P158_ANCHOR_TIMEOUT_MS, 120_000),
      runIdentityTimeoutMs: positiveInteger(process.env.P158_RUN_IDENTITY_TIMEOUT_MS, 120_000),
      workflowTimeoutMs: positiveInteger(process.env.P158_WORKFLOW_TIMEOUT_MS, 90 * 60_000),
      anchorExitTimeoutMs: positiveInteger(process.env.P158_ANCHOR_EXIT_TIMEOUT_MS, 30_000),
      w8ActionManifestSha256: process.env.P158_W8_ACTION_MANIFEST_SHA256,
      w8ActionManifestArtifactRunId: process.env.P158_W8_ACTION_MANIFEST_ARTIFACT_RUN_ID,
      w8ActionManifestArtifactName: process.env.P158_W8_ACTION_MANIFEST_ARTIFACT_NAME,
      sensitiveValues: [
        process.env.P158_DEV_HANDOFF_URL,
        process.env.P158_DEV_DASHBOARD_USERNAME,
        process.env.P158_DEV_DASHBOARD_PASSWORD,
      ],
    },
    provider,
    signal: controller.signal,
  });
  process.stdout.write(`${JSON.stringify({ success: aggregate.success, aggregateSha256: aggregate.aggregateSha256 })}\n`);
  if (!aggregate.success) process.exitCode = 1;
}

main().catch(() => {
  process.stderr.write(`${JSON.stringify({ success: false, failureCode: 'p158_live_adapter_failed' })}\n`);
  process.exitCode = 1;
});
