#!/usr/bin/env node

import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const wrapper = join(repoRoot, 'scripts', 'ci', 'cargo-safe.sh');
const defaultJobs = [4, 6, 8];
const defaultMemoryLimitKib = 24 * 1024 * 1024;

function valueAfter(args, flag) {
  const index = args.indexOf(flag);
  if (index === -1) return undefined;
  if (index === args.length - 1) throw new Error(`${flag} requires a value`);
  return args[index + 1];
}

function parseJobs(raw) {
  const jobs = (raw ?? defaultJobs.join(','))
    .split(',')
    .map((value) => Number.parseInt(value, 10));
  if (jobs.length === 0 || jobs.some((value) => !Number.isSafeInteger(value) || value < 1)) {
    throw new Error('job counts must be comma-separated positive integers');
  }
  return [...new Set(jobs)];
}

function parseMaximumRssKib(timeOutput) {
  const match = timeOutput.match(/Maximum resident set size \(kbytes\):\s*(\d+)/);
  return match ? Number.parseInt(match[1], 10) : null;
}

function timestamp() {
  return new Date().toISOString().replaceAll(':', '').replaceAll('.', '-');
}

function benchmarkPlan(args) {
  const jobs = parseJobs(valueAfter(args, '--jobs'));
  const memoryLimitKib = Number.parseInt(
    process.env.AGENT_BROWSER_CARGO_BENCHMARK_MEMORY_LIMIT_KIB ?? String(defaultMemoryLimitKib),
    10,
  );
  if (!Number.isSafeInteger(memoryLimitKib) || memoryLimitKib < 1) {
    throw new Error('AGENT_BROWSER_CARGO_BENCHMARK_MEMORY_LIMIT_KIB must be a positive integer');
  }
  return {
    jobs,
    isolatedTargetDirectories: true,
    sharedTargetPreserved: true,
    cargoCache: 'off',
    fastLinker: 'off',
    memoryLimitKib,
  };
}

function main() {
  const args = process.argv.slice(2);
  const plan = benchmarkPlan(args);
  if (args.includes('--plan')) {
    console.log(JSON.stringify(plan, null, 2));
    return;
  }

  const outputRoot = resolve(
    valueAfter(args, '--output') ?? join(repoRoot, 'cli', 'target', 'build-benchmarks', timestamp()),
  );
  mkdirSync(outputRoot, { recursive: true });

  const report = {
    schemaVersion: 1,
    startedAt: new Date().toISOString(),
    host: {
      cpuCount: Number.parseInt(spawnSync('nproc', { encoding: 'utf8' }).stdout.trim(), 10),
      rustc: spawnSync('rustc', ['--version'], { encoding: 'utf8' }).stdout.trim(),
      cargo: spawnSync(wrapper, ['--version'], { cwd: repoRoot, encoding: 'utf8' }).stdout.trim(),
      sccache: spawnSync('sccache', ['--version'], { encoding: 'utf8' }).stdout.trim() || null,
      mold: spawnSync('mold', ['--version'], { encoding: 'utf8' }).stdout.split('\n')[0].trim() || null,
    },
    methodology: plan,
    runs: [],
    selectedJobs: null,
  };

  for (const jobs of plan.jobs) {
    const runRoot = mkdtempSync(join(tmpdir(), `agent-browser-cargo-${jobs}-jobs-`));
    const targetDir = join(runRoot, 'target');
    const timePath = join(runRoot, 'time.txt');
    const started = Date.now();
    console.error(`Benchmarking cargo check with ${jobs} jobs in ${runRoot}`);
    try {
      const result = spawnSync(
        '/usr/bin/time',
        [
          '-v', '-o', timePath,
          wrapper,
          'check',
          '--manifest-path', join(repoRoot, 'cli', 'Cargo.toml'),
          '--target-dir', targetDir,
          '--timings',
        ],
        {
          cwd: repoRoot,
          env: {
            ...process.env,
            AGENT_BROWSER_CARGO_BUILD_JOBS: String(jobs),
            AGENT_BROWSER_CARGO_CACHE: 'off',
            AGENT_BROWSER_CARGO_FAST_LINKER: 'off',
            CARGO_INCREMENTAL: '0',
          },
          stdio: ['ignore', 'inherit', 'inherit'],
        },
      );
      const timeOutput = existsSync(timePath) ? readFileSync(timePath, 'utf8') : '';
      const timingSource = join(targetDir, 'cargo-timings', 'cargo-timing.html');
      const timingArtifact = `${jobs}-jobs-cargo-timing.html`;
      if (existsSync(timingSource)) copyFileSync(timingSource, join(outputRoot, timingArtifact));
      report.runs.push({
        jobs,
        status: result.status,
        wallTimeSeconds: Number(((Date.now() - started) / 1000).toFixed(3)),
        maximumRssKib: parseMaximumRssKib(timeOutput),
        cargoTimingArtifact: existsSync(join(outputRoot, timingArtifact)) ? timingArtifact : null,
      });
    } finally {
      rmSync(runRoot, { recursive: true, force: true });
    }
  }

  const eligible = report.runs.filter((run) =>
    run.status === 0
    && run.maximumRssKib !== null
    && run.maximumRssKib <= plan.memoryLimitKib);
  if (eligible.length === plan.jobs.length) {
    report.selectedJobs = eligible.reduce((best, run) =>
      run.wallTimeSeconds < best.wallTimeSeconds ? run : best).jobs;
  }
  report.completedAt = new Date().toISOString();
  const reportPath = join(outputRoot, 'report.json');
  writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(JSON.stringify({ report: reportPath, selectedJobs: report.selectedJobs }));
  if (report.runs.some((run) => run.status !== 0)) process.exitCode = 1;
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 2;
}
