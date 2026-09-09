#!/usr/bin/env node

import { mkdir, open, readFile } from 'node:fs/promises';
import { isAbsolute, join, relative } from 'node:path';

import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  createP158W7A04A06LiveBundle,
  createP158W7A05DevelopmentService,
} from './lib/p158-w7-a04-a06-live.js';

function option(name) {
  const index = process.argv.indexOf(name);
  return index < 0 ? null : process.argv[index + 1] ?? null;
}

const manifestPath = option('--ownership-manifest');
const runRoot = option('--run-root');
if (!manifestPath || !runRoot || !isAbsolute(manifestPath) || !isAbsolute(runRoot)) {
  throw new Error('usage: run-p158-w7-a04-a06-live --ownership-manifest <absolute-path> --run-root <absolute-out-of-repo-path>');
}
const repositoryRoot = new URL('..', import.meta.url).pathname;
if (!relative(repositoryRoot, runRoot).startsWith('..')) {
  throw new Error('run root must be outside the product repository');
}

const [registryText, manifestText] = await Promise.all([
  readFile(new URL('../docs/dev/contracts/p158-historical-failure-registry.v1.json', import.meta.url), 'utf8'),
  readFile(manifestPath, 'utf8'),
]);
const ownershipManifest = JSON.parse(manifestText);
const schedule = compileP158ExecutionSchedule({
  registry: JSON.parse(registryText), seed: `p158-w7:${ownershipManifest.campaignRunId}`,
});
await mkdir(runRoot, { recursive: true });
const ledgerPath = join(runRoot, 'p158-w7-a05-action-receipts.jsonl');
const ledger = await open(ledgerPath, 'ax', 0o600).catch((error) => {
  if (error.code === 'EEXIST') throw new Error(`append-only receipt ledger already exists; refusing replay: ${ledgerPath}`);
  throw error;
});
const receiptStore = {
  async append(receipt) {
    await ledger.write(`${JSON.stringify(receipt)}\n`);
    await ledger.sync();
  },
};
const service = createP158W7A05DevelopmentService();
try {
  const bundle = createP158W7A04A06LiveBundle({ schedule, ownershipManifest, receiptStore, service });
  if (!bundle.freezeEligible) throw new Error('A05 live bundle is not freeze eligible');
  const adapter = bundle.adapters[0];
  const results = [];
  for (const attempt of schedule.attempts.filter((entry) => entry.caseId === 'A05')) {
    results.push({ attemptId: attempt.attemptId, ...(await adapter.execute({ attempt })) });
  }
  process.stdout.write(`${JSON.stringify({
    success: results.every((result) => result.resultState === 'passed'),
    campaignRunId: ownershipManifest.campaignRunId,
    ownershipManifestSha256: bundle.ownershipManifestSha256,
    concreteCaseIds: bundle.concreteCaseIds,
    blockedCaseCounts: { A04: bundle.readiness.counts.A04.blocked, A06: bundle.readiness.counts.A06.blocked },
    results, repairAttempted: false, retryAttempted: false, garbageCollectionAttempted: false,
  })}\n`);
} finally {
  service.close?.();
  await ledger.close();
}
