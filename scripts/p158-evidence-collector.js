#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import {
  buildP158AggregateFixtureManifest,
  P158EvidenceCollectorError,
  runP158EvidenceCollector,
} from './lib/p158-evidence-collector.js';

const args = process.argv.slice(2);
const repoRoot = new URL('..', import.meta.url).pathname;

function takeOption(name) {
  const index = args.indexOf(name);
  if (index < 0) return null;
  if (index === args.length - 1) throw new Error(`${name} requires a value`);
  const value = args[index + 1];
  args.splice(index, 2);
  return value;
}

function takeFlag(name) {
  const index = args.indexOf(name);
  if (index < 0) return false;
  args.splice(index, 1);
  return true;
}

function printHelp() {
  process.stdout.write(`Usage:
  node scripts/p158-evidence-collector.js --aggregate-only
  node scripts/p158-evidence-collector.js --config <path>
  node scripts/p158-evidence-collector.js --config <path> --freeze --run-root <absolute-path>

The default config mode is a provider-free, filesystem-read-only dry run. The
--freeze flag is required before campaign evidence is persisted. This command
never starts campaign execution.
`);
}

try {
  if (takeFlag('--help') || takeFlag('-h')) {
    printHelp();
    process.exit(0);
  }
  const aggregateOnly = takeFlag('--aggregate-only');
  const freeze = takeFlag('--freeze');
  const configPath = takeOption('--config');
  const runRoot = takeOption('--run-root');
  if (args.length > 0) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  if (aggregateOnly) {
    if (freeze || configPath || runRoot) throw new Error('--aggregate-only cannot be combined with config or freeze options');
    const aggregate = buildP158AggregateFixtureManifest({ repoRoot });
    process.stdout.write(`${JSON.stringify({
      schemaVersion: 'agent-browser.p158-fixture-aggregate-report.v1',
      planId: 'P158',
      sha256: aggregate.sha256,
      byteCount: aggregate.byteCount,
      manifest: aggregate.manifest,
    }, null, 2)}\n`);
    process.exit(0);
  }
  if (!configPath) throw new Error('--config is required unless --aggregate-only is used');
  if (freeze && !runRoot) throw new Error('--freeze requires --run-root');
  if (!freeze && runRoot) throw new Error('--run-root is accepted only with --freeze');
  const absoluteConfigPath = resolve(configPath);
  const config = JSON.parse(readFileSync(absoluteConfigPath, 'utf8'));
  const report = await runP158EvidenceCollector({
    config,
    repoRoot,
    baseDir: dirname(absoluteConfigPath),
    freeze,
    runRoot,
    clock: config.dryRunFrozenAt && !freeze
      ? {
          wallNow: () => config.dryRunFrozenAt,
          monotonicNow: () => 1,
        }
      : undefined,
  });
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
} catch (error) {
  const payload = {
    success: false,
    code: error instanceof P158EvidenceCollectorError ? error.code : 'invalid_invocation',
    message: error.message,
    details: error instanceof P158EvidenceCollectorError ? error.details ?? null : null,
  };
  process.stderr.write(`${JSON.stringify(payload, null, 2)}\n`);
  process.exitCode = 1;
}
