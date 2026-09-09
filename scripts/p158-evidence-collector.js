#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

import {
  buildP158AggregateFixtureManifest,
  P158EvidenceCollectorError,
  runP158EvidenceCollector,
} from './lib/p158-evidence-collector.js';
import { canonicalCandidateDigest } from './lib/p158-campaign-preparation.js';
import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import { assembleP158W6LiveBindings } from './lib/p158-w6-evidence-assembler.js';

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
  node scripts/p158-evidence-collector.js --config <path> --assemble-live-bindings
  node scripts/p158-evidence-collector.js --config <path> --freeze --run-root <absolute-path> \\
    --live-hook-manifest <path>

The default config mode is a provider-free, filesystem-read-only dry run. The
--freeze flag is required before campaign evidence is persisted. This command
never starts campaign execution. --assemble-live-bindings constructs the exact
54 zero-effect case adapters and 24 source-sealed hook bindings.
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
  const liveHookManifestPath = takeOption('--live-hook-manifest');
  const assembleLiveBindings = takeFlag('--assemble-live-bindings');
  if (args.length > 0) throw new Error(`Unexpected arguments: ${args.join(' ')}`);
  if (aggregateOnly) {
    if (freeze || configPath || runRoot || liveHookManifestPath || assembleLiveBindings) {
      throw new Error('--aggregate-only cannot be combined with config, assembly, or freeze options');
    }
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
  if (freeze && !liveHookManifestPath && !assembleLiveBindings) {
    throw new Error('--freeze requires --live-hook-manifest or --assemble-live-bindings');
  }
  if (!freeze && runRoot) throw new Error('--run-root is accepted only with --freeze');
  if (!freeze && liveHookManifestPath) throw new Error('--live-hook-manifest is accepted only with --freeze');
  if (liveHookManifestPath && assembleLiveBindings) {
    throw new Error('--live-hook-manifest and --assemble-live-bindings are mutually exclusive');
  }
  const absoluteConfigPath = resolve(configPath);
  const config = JSON.parse(readFileSync(absoluteConfigPath, 'utf8'));
  let liveHookManifest = liveHookManifestPath
    ? JSON.parse(readFileSync(resolve(liveHookManifestPath), 'utf8'))
    : undefined;
  let adapters;
  let liveBindingSummary = null;
  let liveAssembly = null;
  if (assembleLiveBindings) {
    const aggregate = buildP158AggregateFixtureManifest({ repoRoot });
    const registry = JSON.parse(readFileSync(resolve(
      repoRoot, 'docs/dev/contracts/p158-historical-failure-registry.v1.json',
    ), 'utf8'));
    const schedule = compileP158ExecutionSchedule({ registry, seed: config.seed });
    const candidate = {
      ...structuredClone(config.candidate), runId: config.runId,
      aggregateFixtureManifestSha256: aggregate.sha256,
    };
    candidate.candidateSha256 = canonicalCandidateDigest(candidate);
    const assembled = assembleP158W6LiveBindings({
      schedule, candidate, aggregate, runId: config.runId,
      capturedAt: config.aggregateCapturedAt,
    });
    liveHookManifest = assembled.liveHookManifest;
    adapters = assembled.adapters;
    liveBindingSummary = {
      adapterCount: adapters.length,
      hookCount: liveHookManifest.hookBindings.length,
      concreteAdapterCount: liveHookManifest.adapterBindings.filter((entry) =>
        entry.mode === 'concrete_live').length,
      blockedAdapterCount: liveHookManifest.adapterBindings.filter((entry) =>
        entry.mode === 'explicit_blocked').length,
    };
    liveAssembly = {
      schemaVersion: 'agent-browser.p158-w6-live-assembly.v1',
      collectorConfig: structuredClone(config),
      collectorConfigSha256: sha256(config),
      liveHookManifest: structuredClone(liveHookManifest),
      aggregateManifest: structuredClone(aggregate.manifest),
      aggregateSha256: aggregate.sha256,
    };
  }
  const report = await runP158EvidenceCollector({
    config,
    repoRoot,
    baseDir: dirname(absoluteConfigPath),
    freeze,
    runRoot,
    liveHookManifest,
    adapters,
    clock: config.dryRunFrozenAt && !freeze
      ? {
          wallNow: () => config.dryRunFrozenAt,
          monotonicNow: () => 1,
        }
      : undefined,
  });
  process.stdout.write(`${JSON.stringify({ ...report, liveBindingSummary, liveAssembly }, null, 2)}\n`);
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
