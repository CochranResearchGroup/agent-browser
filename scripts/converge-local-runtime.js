#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import {
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const args = process.argv.slice(2);
const options = {
  apply: false,
  json: false,
  evidencePath: '',
  skipPublish: false,
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
  } else if (arg === '--skip-publish') {
    options.skipPublish = true;
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
  handoffRemedies: [],
  staleMetadataRemedies: [],
  skippedRemedies: [],
  evidencePath: null,
  initial: null,
  final: null,
};
const agentBrowserCommand = process.env.AGENT_BROWSER_BIN || 'agent-browser';
const pnpmCommand = process.env.PNPM_BIN || 'pnpm';

const lockPath = resolve(`${homedir()}/.agent-browser/convergence/local-runtime.lock`);
acquireLock(lockPath);
process.on('exit', () => releaseLock(lockPath));
process.on('SIGINT', () => process.exit(130));
process.on('SIGTERM', () => process.exit(143));

try {
  report.initial = readDoctors('initial', { required: !options.apply });
  report.safeRemedies = staleDaemonRemedies(report.initial.install);

  if (options.apply) {
    if (!options.skipPublish) {
      runStep('publish_local_dashboard', pnpmCommand, [
        'publish:local-dashboard',
        '--',
        '--skip-browser',
        '--json',
      ]);
    }

    const afterPublish = options.skipPublish
      ? report.initial
      : readDoctors('after_publish', { required: false });
    repairConfirmedStaleDaemons(afterPublish.install, 'after_publish');
    const staleMetadataCandidates = staleSessionMetadataNames();
    if (staleMetadataCandidates.length > 0) sleep(2000);
    const confirmedStaleMetadata = new Set(staleSessionMetadataNames());
    for (const session of staleMetadataCandidates) {
      if (!confirmedStaleMetadata.has(session)) continue;
      runStep(
        `close_stale_session_metadata_${session}`,
        agentBrowserCommand,
        ['close', '--session', session],
      );
      report.staleMetadataRemedies.push({
        session,
        action: 'close_stale_session_metadata',
      });
    }

    runOptionalStep('ensure_rdp_guac_postgres', pnpmCommand, [
      'ensure:rdp-guac-postgres',
      '--',
      '--apply',
    ]);
    runOptionalStep('rdp_guac_route_pool_readiness', pnpmCommand, [
      'test:rdp-guac-route-pool-readiness',
      '--',
      '--report-only',
    ]);
    let afterRoutePool = readDoctors('after_route_pool', { required: false });
    if (repairConfirmedStaleDaemons(afterRoutePool.install, 'after_route_pool')) {
      afterRoutePool = readDoctors('after_route_pool_stale_repair', { required: false });
    }
    if (routeDisplayRecoveryRequired(afterRoutePool.remoteView.nextAction)) {
      runOptionalStep('restore_rdp_route_displays', pnpmCommand, [
        'open:rdp-route-displays',
      ]);
      afterRoutePool = readDoctors('after_route_display_restore', { required: false });
      if (repairConfirmedStaleDaemons(afterRoutePool.install, 'after_route_display_restore')) {
        afterRoutePool = readDoctors('after_route_display_restore_stale_repair', {
          required: false,
        });
      }
    }
    if (afterRoutePool.remoteView.nextAction === 'grant_route_display_access') {
      runOptionalStep('grant_route_display_access', pnpmCommand, [
        'grant:rdp-route-display-access',
        '--',
        '--apply',
      ]);
    }
  }

  report.final = readDoctors('final', { required: false });
  if (options.apply && repairConfirmedStaleDaemons(report.final.install, 'final')) {
    report.final = readDoctors('final_after_stale_repair', { required: false });
  }
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

function readDoctors(label, { required = true } = {}) {
  const install = runJsonStep(`${label}_install_doctor`, agentBrowserCommand, [
    'install',
    'doctor',
    '--json',
  ], { required });
  const remoteView = runJsonStep(`${label}_remote_view_doctor`, agentBrowserCommand, [
    'doctor',
    'remote-view',
    '--json',
  ], { required });
  return {
    install: summarizeInstallDoctor(install),
    remoteView: summarizeRemoteViewDoctor(remoteView),
  };
}

function summarizeInstallDoctor(payload) {
  const data = payload.data ?? {};
  const inventory = data.runtimeInventory ?? {};
  const listenerInventory = data.daemonListenerInventory ?? {};
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
    daemonListenerInventory: {
      state: listenerInventory.state ?? null,
      listeners: Array.isArray(listenerInventory.listeners)
        ? listenerInventory.listeners
        : [],
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

function staleDaemonListenerSessions(install) {
  return install.daemonListenerInventory.listeners
    .filter((listener) => (
      listener?.deletedExecutable === true
      || listener?.matchesCurrentExecutable === false
    ))
    .map((listener) => {
      const match = String(listener?.socketPath ?? '').match(/([^/\\]+)\.(?:sock|port)$/);
      return {
        session: match?.[1] ?? null,
        pid: Number.isInteger(listener?.pid) && listener.pid > 0 ? listener.pid : null,
      };
    })
    .filter((listener) => (
      typeof listener.session === 'string'
      && /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(listener.session)
    ));
}

function staleDaemonCandidates(install) {
  const candidates = new Map();
  for (const remedy of staleDaemonRemedies(install)) {
    if (!remedy.session) continue;
    candidates.set(remedy.session, { ...remedy, listenerAuthority: false });
  }
  for (const listener of staleDaemonListenerSessions(install)) {
    const prior = candidates.get(listener.session);
    candidates.set(listener.session, {
      session: listener.session,
      argv: prior?.argv ?? [],
      listenerAuthority: true,
      listenerPid: listener.pid,
    });
  }
  return [...candidates.values()];
}

function repairConfirmedStaleDaemons(install, label) {
  const candidates = staleDaemonCandidates(install);
  if (candidates.length === 0) return false;

  sleep(2000);
  const confirmationPayload = runJsonStep(
    `${label}_confirm_stale_daemons_install_doctor`,
    agentBrowserCommand,
    ['install', 'doctor', '--json'],
    { required: false },
  );
  const confirmedInstall = summarizeInstallDoctor(confirmationPayload);
  const confirmedSessions = new Set(
    staleDaemonCandidates(confirmedInstall).map((remedy) => remedy.session),
  );
  for (const remedy of candidates) {
    if (!confirmedSessions.has(remedy.session)) continue;
    if (remedy.listenerAuthority || isSafeStaleDaemonRemedy(remedy.argv)) {
      const prepared = runJsonStep(
        `${label}_prepare_stale_daemon_handoff_${remedy.session}`,
        agentBrowserCommand,
        ['--json', '--session', remedy.session, 'handoff', 'prepare'],
        { required: false },
      );
      if (prepared.success !== true) {
        report.skippedRemedies.push({
          session: remedy.session,
          argv: remedy.argv,
          reason: 'runtime_handoff_prepare_failed',
          error: prepared.error ?? null,
        });
        continue;
      }
      const handoff = {
        session: remedy.session,
        prepared: prepared.data?.prepared === true,
        browserPid: prepared.data?.browserPid ?? null,
        cdpUrl: prepared.data?.cdpUrl ?? null,
        resumed: false,
      };
      if (handoff.prepared) {
        const resumed = runJsonStep(
          `${label}_resume_stale_daemon_handoff_${remedy.session}`,
          agentBrowserCommand,
          ['--json', '--session', remedy.session, 'handoff', 'resume'],
          { required: false },
        );
        handoff.resumed = resumed.success === true;
        handoff.resumedBrowserPid = resumed.data?.browserPid ?? null;
        handoff.resumedCdpUrl = resumed.data?.cdpUrl ?? null;
        if (
          !handoff.resumed ||
          handoff.resumedBrowserPid !== handoff.browserPid ||
          handoff.resumedCdpUrl !== handoff.cdpUrl
        ) {
          report.skippedRemedies.push({
            session: remedy.session,
            argv: remedy.argv,
            reason: 'runtime_handoff_resume_failed',
            error: resumed.error ?? null,
          });
        }
      } else {
        handoff.idleDaemonRetired = retireConfirmedIdleDaemon(remedy.listenerPid);
        if (!handoff.idleDaemonRetired) {
          report.skippedRemedies.push({
            session: remedy.session,
            argv: remedy.argv,
            reason: 'idle_stale_daemon_retirement_failed',
            error: remedy.listenerPid
              ? `daemon process ${remedy.listenerPid} remained live`
              : 'confirmed listener PID unavailable',
          });
        }
      }
      report.handoffRemedies.push(handoff);
    } else {
      report.skippedRemedies.push({
        session: remedy.session,
        argv: remedy.argv,
        reason: 'unsupported_remedy_shape',
      });
    }
  }
  return true;
}

function retireConfirmedIdleDaemon(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 'SIGTERM');
  } catch (error) {
    return error?.code === 'ESRCH';
  }
  for (let attempt = 0; attempt < 50; attempt += 1) {
    sleep(100);
    try {
      process.kill(pid, 0);
    } catch (error) {
      return error?.code === 'ESRCH';
    }
  }
  return false;
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

function routeDisplayRecoveryRequired(nextAction) {
  return new Set([
    'open_route_specific_rdp_sessions_then_rerun_doctor',
    'open_two_rdp_route_sessions_for_existing_agent_browser_rdp_user_then_rerun_doctor',
    'repair_rdp_route_display_session',
  ]).has(nextAction);
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function staleSessionMetadataNames({ minimumAgeMs = 60_000 } = {}) {
  const socketDir = process.env.AGENT_BROWSER_SOCKET_DIR ||
    (process.env.XDG_RUNTIME_DIR
      ? join(process.env.XDG_RUNTIME_DIR, 'agent-browser')
      : join(homedir(), '.agent-browser'));
  if (!existsSync(socketDir)) return [];
  const observedAt = Date.now();

  return readdirSync(socketDir)
    .filter((name) => name.endsWith('.token'))
    .filter((name) => observedAt - statSync(join(socketDir, name)).mtimeMs >= minimumAgeMs)
    .map((name) => name.slice(0, -'.token'.length))
    .filter((session) => /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(session))
    .filter((session) => ![
      '.pid',
      '.port',
      '.sha256',
      '.sock',
      '.stream',
      '.version',
    ].some((suffix) => existsSync(join(socketDir, `${session}${suffix}`))))
    .sort();
}

function runJsonStep(name, command, commandArgs, { required = true } = {}) {
  const result = runStep(name, command, commandArgs, { capture: true, required });
  try {
    return JSON.parse((result.stdout ?? '').trim());
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
    stderr: (result.stderr ?? result.error?.message ?? '').trim(),
  };
  report.steps.push(step);
  if (result.error) {
    throw new Error(`${name} could not start ${command}: ${result.error.message}`);
  }
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

function acquireLock(path) {
  mkdirSync(dirname(path), { recursive: true });
  try {
    const descriptor = openSync(path, 'wx', 0o600);
    writeFileSync(descriptor, `${process.pid}\n`);
    closeSync(descriptor);
    return;
  } catch (error) {
    if (error?.code !== 'EEXIST') throw error;
  }

  const ownerPid = Number.parseInt(readFileSync(path, 'utf8').trim(), 10);
  if (Number.isInteger(ownerPid) && ownerPid > 0) {
    try {
      process.kill(ownerPid, 0);
      throw new Error(`Local runtime convergence is already active in process ${ownerPid}`);
    } catch (error) {
      if (error?.code !== 'ESRCH') throw error;
    }
  }
  rmSync(path, { force: true });
  const descriptor = openSync(path, 'wx', 0o600);
  writeFileSync(descriptor, `${process.pid}\n`);
  closeSync(descriptor);
}

function releaseLock(path) {
  try {
    const ownerPid = Number.parseInt(readFileSync(path, 'utf8').trim(), 10);
    if (ownerPid === process.pid) rmSync(path, { force: true });
  } catch {
    // Best-effort cleanup. A later run can recover a stale lock.
  }
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
  console.log(`Usage: node scripts/converge-local-runtime.js [--apply] [--json] [--skip-publish]

Dry-run by default. Reports install doctor, remote-view doctor, runtime
inventory, and safe stale-daemon remedies. With --apply, synchronizes the local
dashboard runtime, closes only agent-browser stale daemon sessions reported by
doctor remedies through browser-preserving daemon handoff, ensures Guacamole
Postgres schema state, restores missing RDP route displays, applies display
grants only when doctor asks for them, writes an evidence JSON file, and reruns
doctors.

Options:
  --apply                 Apply safe local repairs. Default is dry-run.
  --json                  Emit JSON.
  --skip-publish          Keep the installed binary/dashboard unchanged. Used by
                          the recurring runtime-health interlock.
  --evidence-path <path>  Apply-mode evidence path. Default:
                          ~/.agent-browser/convergence/local-runtime-latest.json`);
}
