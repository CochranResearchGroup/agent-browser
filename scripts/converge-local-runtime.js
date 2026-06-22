#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';

const args = process.argv.slice(2);
const options = {
  apply: false,
  json: false,
  evidencePath: '',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--apply') {
    options.apply = true;
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--evidence-path') {
    options.evidencePath = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

const report = {
  schemaVersion: 'agent-browser.local-runtime-convergence.v1',
  apply: options.apply,
  steps: [],
  safeRemedies: [],
  skippedRemedies: [],
  evidencePath: null,
  initial: null,
  final: null,
};

try {
  report.initial = readDoctors('initial');
  report.safeRemedies = staleDaemonRemedies(report.initial.install);

  if (options.apply) {
    runStep('publish_local_dashboard', 'pnpm', [
      'publish:local-dashboard',
      '--',
      '--skip-browser',
      '--json',
    ]);

    const afterPublish = readDoctors('after_publish');
    for (const remedy of staleDaemonRemedies(afterPublish.install)) {
      if (isSafeStaleDaemonRemedy(remedy.argv)) {
        runStep(`close_stale_daemon_${remedy.session}`, remedy.argv[0], remedy.argv.slice(1));
      } else {
        report.skippedRemedies.push({
          session: remedy.session,
          argv: remedy.argv,
          reason: 'unsupported_remedy_shape',
        });
      }
    }

    runOptionalStep('ensure_rdp_guac_postgres', 'pnpm', [
      'ensure:rdp-guac-postgres',
      '--',
      '--apply',
    ]);
    runOptionalStep('rdp_guac_route_pool_readiness', 'pnpm', [
      'test:rdp-guac-route-pool-readiness',
      '--',
      '--report-only',
    ]);
    const afterRoutePool = readDoctors('after_route_pool');
    if (afterRoutePool.remoteView.nextAction === 'grant_route_display_access') {
      runOptionalStep('grant_route_display_access', 'pnpm', [
        'grant:rdp-route-display-access',
        '--',
        '--apply',
      ]);
    }
  }

  report.final = readDoctors('final');
  const successful = report.final.install.success === true &&
    report.final.remoteView.remoteControlReady === true &&
    report.skippedRemedies.length === 0 &&
    !report.steps.some((step) => step.required !== false && step.success === false);

  const payload = {
    success: successful,
    ...report,
  };
  if (options.apply) {
    payload.evidencePath = writeEvidence(payload);
  }
  output(payload);
  process.exit(successful ? 0 : 1);
} catch (error) {
  const payload = {
    success: false,
    error: error instanceof Error ? error.message : String(error),
    ...report,
  };
  if (options.apply) {
    payload.evidencePath = writeEvidence(payload);
  }
  output(payload);
  process.exit(1);
}

function readDoctors(label) {
  const install = runJsonStep(`${label}_install_doctor`, 'agent-browser', [
    'install',
    'doctor',
    '--json',
  ]);
  const remoteView = runJsonStep(`${label}_remote_view_doctor`, 'agent-browser', [
    'doctor',
    'remote-view',
    '--json',
  ]);
  return {
    install: summarizeInstallDoctor(install),
    remoteView: summarizeRemoteViewDoctor(remoteView),
  };
}

function summarizeInstallDoctor(payload) {
  const data = payload.data ?? {};
  const inventory = data.runtimeInventory ?? {};
  const issues = Array.isArray(data.issues) ? data.issues : [];
  return {
    success: payload.success === true,
    issueCodes: issues.map((issue) => issue.code).filter(Boolean),
    issues,
    runtimeInventory: {
      status: inventory.status ?? null,
      runtimeCount: inventory.runtimeCount ?? 0,
      staleCount: inventory.staleCount ?? 0,
      convergedCount: inventory.convergedCount ?? 0,
    },
  };
}

function summarizeRemoteViewDoctor(payload) {
  const data = payload.data ?? {};
  const inventory = data.runtimeInventory ?? {};
  return {
    success: payload.success === true,
    status: data.status ?? null,
    remoteControlReady: data.remoteControl?.ready === true,
    nextAction: data.nextAction ?? null,
    runtimeInventory: {
      status: inventory.status ?? null,
      runtimeCount: inventory.runtimeCount ?? 0,
      staleCount: inventory.staleCount ?? 0,
      convergedCount: inventory.convergedCount ?? 0,
    },
  };
}

function staleDaemonRemedies(install) {
  return install.issues
    .filter((issue) => issue.code === 'active_runtime_stale_executable')
    .map((issue) => ({
      session: issue.session ?? null,
      argv: Array.isArray(issue.remedy?.argv) ? issue.remedy.argv : [],
    }));
}

function isSafeStaleDaemonRemedy(argv) {
  return Array.isArray(argv) &&
    argv.length === 4 &&
    argv[0] === 'agent-browser' &&
    argv[1] === 'close' &&
    argv[2] === '--session' &&
    typeof argv[3] === 'string' &&
    argv[3].trim().length > 0 &&
    !argv[3].startsWith('-');
}

function runJsonStep(name, command, commandArgs) {
  const result = runStep(name, command, commandArgs, { capture: true });
  try {
    return JSON.parse(result.stdout.trim());
  } catch (error) {
    throw new Error(`Failed to parse ${name} JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function runOptionalStep(name, command, commandArgs) {
  return runStep(name, command, commandArgs, { required: false });
}

function runStep(name, command, commandArgs, { capture = false, required = true } = {}) {
  const result = spawnSync(command, commandArgs, {
    encoding: 'utf8',
    stdio: capture ? ['ignore', 'pipe', 'pipe'] : ['ignore', 'pipe', 'pipe'],
  });
  const step = {
    name,
    command: [command, ...commandArgs],
    success: result.status === 0,
    status: result.status,
    required,
    stdoutBytes: result.stdout?.length ?? 0,
    stderr: (result.stderr ?? '').trim(),
  };
  report.steps.push(step);
  if (required && result.status !== 0) {
    throw new Error(`${name} failed with status ${result.status}: ${(result.stderr || result.stdout || '').trim()}`);
  }
  return result;
}

function output(payload) {
  if (options.json) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }
  if (!payload.success) {
    console.error(payload.error ?? 'Local runtime convergence did not complete.');
  }
  const final = payload.final;
  console.log(`install doctor: ${final?.install?.success ? 'ready' : 'not ready'}`);
  console.log(`remote view: ${final?.remoteView?.remoteControlReady ? 'ready' : 'not ready'}`);
  console.log(`stale runtimes: ${final?.install?.runtimeInventory?.staleCount ?? 'unknown'}`);
}

function writeEvidence(payload) {
  const path = resolve(options.evidencePath || `${homedir()}/.agent-browser/convergence/local-runtime-latest.json`);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(payload, null, 2)}\n`);
  return path;
}

function requiredValue(values, index, flag) {
  const value = values[index];
  if (!value) fail(`Missing value for ${flag}`);
  return value;
}

function fail(message) {
  console.error(message);
  process.exit(2);
}

function printHelp() {
  console.log(`Usage: node scripts/converge-local-runtime.js [--apply] [--json]

Dry-run by default. Reports install doctor, remote-view doctor, runtime
inventory, and safe stale-daemon remedies. With --apply, synchronizes the local
dashboard runtime, closes only agent-browser stale daemon sessions reported by
doctor remedies, ensures Guacamole Postgres schema state, applies display grants
only when doctor asks for them, writes an evidence JSON file, and reruns doctors.

Options:
  --apply                 Apply safe local repairs. Default is dry-run.
  --json                  Emit JSON.
  --evidence-path <path>  Apply-mode evidence path. Default:
                          ~/.agent-browser/convergence/local-runtime-latest.json`);
}
