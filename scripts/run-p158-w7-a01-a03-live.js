#!/usr/bin/env node

import { mkdir, open, readFile } from 'node:fs/promises';
import { isAbsolute, join, relative } from 'node:path';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  createP158W7A01A03LiveBundle,
  createP158W7PinnedDevelopmentTransports,
  P158_W7_A01_A03_CASE_IDS,
} from './lib/p158-w7-a01-a03-live.js';

function option(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1] ?? null;
}

const manifestPath = option('--ownership-manifest');
const runRoot = option('--run-root');
if (!manifestPath || !runRoot || !isAbsolute(manifestPath) || !isAbsolute(runRoot)) {
  throw new Error('usage: run-p158-w7-a01-a03-live --ownership-manifest <absolute-path> --run-root <absolute-out-of-repo-path>');
}
const repositoryRoot = new URL('..', import.meta.url).pathname;
if (!relative(repositoryRoot, runRoot).startsWith('..')) {
  throw new Error('run root must be outside the product repository');
}

const [registryText, manifestText] = await Promise.all([
  readFile(new URL('../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url), 'utf8'),
  readFile(manifestPath, 'utf8'),
]);
const registry = JSON.parse(registryText);
const ownershipManifest = JSON.parse(manifestText);
const schedule = compileP158ExecutionSchedule({ registry, seed: `p158-w7:${ownershipManifest.campaignRunId}` });
await mkdir(runRoot, { recursive: true });
const ledgerPath = join(runRoot, 'p158-w7-a01-a03-action-receipts.jsonl');
const ledger = await open(ledgerPath, 'ax', 0o600).catch((error) => {
  if (error.code === 'EEXIST') {
    throw new Error(`append-only receipt ledger already exists; refusing replay: ${ledgerPath}`);
  }
  throw error;
});
const receiptStore = {
  async append(receipt) {
    await ledger.write(`${JSON.stringify(receipt)}\n`);
    await ledger.sync();
  },
};
const transports = createP158W7PinnedDevelopmentTransports();
try {
  const bundle = createP158W7A01A03LiveBundle({ schedule, ownershipManifest, receiptStore, transportFor: transports });
  if (!bundle.freezeEligible) throw new Error('A01-A03 live bundle is not freeze eligible');
  const results = [];
  for (const attempt of schedule.attempts.filter((entry) =>
    P158_W7_A01_A03_CASE_IDS.includes(entry.caseId))) {
    const adapter = bundle.adapters.find((entry) => entry.caseId === attempt.caseId);
    results.push({ attemptId: attempt.attemptId, ...(await adapter.execute({ attempt })) });
  }
  process.stdout.write(`${JSON.stringify({
    success: results.every((result) => result.resultState === 'passed'),
    campaignRunId: ownershipManifest.campaignRunId,
    ownershipManifestSha256: bundle.ownershipManifestSha256,
    results,
    repairAttempted: false,
    retryAttempted: false,
  })}\n`);
} finally {
  transports.close();
  await ledger.close();
}
