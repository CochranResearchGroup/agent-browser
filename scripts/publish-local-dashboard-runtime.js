#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, statSync, chmodSync, renameSync, rmSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const args = process.argv.slice(2);
const options = {
  allowOutsideHome: false,
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'http://127.0.0.1:4848/',
  expectMarkers: [],
  installBin: process.env.AGENT_BROWSER_INSTALL_BIN || '',
  json: false,
  browserProfile: '',
  release: false,
  skipSmoke: false,
  smokeBrowser: true,
  startIfMissing: false,
  workspaceSession: '',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') {
    continue;
  } else if (arg === '--allow-outside-home') {
    options.allowOutsideHome = true;
  } else if (arg === '--dashboard-url') {
    options.dashboardUrl = requiredValue(args, ++index, arg);
  } else if (arg === '--expect-marker') {
    options.expectMarkers.push(requiredValue(args, ++index, arg));
  } else if (arg === '--browser-profile') {
    options.browserProfile = requiredValue(args, ++index, arg);
  } else if (arg === '--install-bin') {
    options.installBin = requiredValue(args, ++index, arg);
  } else if (arg === '--json') {
    options.json = true;
  } else if (arg === '--release') {
    options.release = true;
  } else if (arg === '--skip-browser') {
    options.smokeBrowser = false;
  } else if (arg === '--skip-smoke') {
    options.skipSmoke = true;
  } else if (arg === '--start-if-missing') {
    options.startIfMissing = true;
  } else if (arg === '--workspace-session') {
    options.workspaceSession = requiredValue(args, ++index, arg);
  } else if (arg === '--help' || arg === '-h') {
    printHelp();
    process.exit(0);
  } else {
    fail(`Unknown argument: ${arg}`);
  }
}

const report = {
  dashboardUrl: options.dashboardUrl,
  mode: options.release ? 'release' : 'debug',
  installBin: null,
  builtBin: null,
  backupPath: null,
  service: {
    before: null,
    after: null,
    action: 'none',
  },
  smoke: null,
  runtimeManifest: null,
};

try {
  await run();
  output({ success: true, ...report });
} catch (error) {
  output({
    success: false,
    error: error instanceof Error ? error.message : String(error),
    ...report,
  });
  process.exit(1);
}

async function run() {
  const installBin = resolveInstallBin();
  report.installBin = installBin;
  guardInstallPath(installBin);

  runCommand('pnpm', ['build:dashboard']);

  const cargoArgs = ['build', '--manifest-path', 'cli/Cargo.toml'];
  if (options.release) cargoArgs.push('--release');
  runCommand('cargo', cargoArgs);

  const builtBin = resolve(rootDir, 'cli', 'target', options.release ? 'release' : 'debug', 'agent-browser');
  if (!existsSync(builtBin)) {
    throw new Error(`Built binary was not found: ${builtBin}`);
  }
  report.builtBin = builtBin;

  report.service.before = serviceStatus();
  const beforeStat = existsSync(installBin) ? statSync(installBin) : null;
  const backupPath = `${installBin}.pre-local-dashboard-${timestamp()}`;
  if (beforeStat) {
    copyFileSync(installBin, backupPath);
    chmodSync(backupPath, beforeStat.mode & 0o777);
    report.backupPath = backupPath;
  }

  try {
    installBinaryAtomically(builtBin, installBin, beforeStat ? beforeStat.mode & 0o777 : 0o755);

    await restartOrStartDashboard(installBin);

    if (!options.skipSmoke) {
      report.smoke = runSmoke(installBin);
      report.runtimeManifest = verifyRuntimeManifestReadback(installBin, report.smoke.runtimeManifest);
    }
  } catch (error) {
    if (backupPath && existsSync(backupPath)) {
      installBinaryAtomically(backupPath, installBin, beforeStat ? beforeStat.mode & 0o777 : 0o755);
      report.restoredBackup = true;
      try {
        await restartOrStartDashboard(installBin, { restoring: true });
      } catch (restoreError) {
        report.restoreRestartError = restoreError instanceof Error ? restoreError.message : String(restoreError);
      }
    }
    throw error;
  } finally {
    report.service.after = serviceStatus();
  }
}

function installBinaryAtomically(source, target, mode) {
  mkdirSync(dirname(target), { recursive: true });
  const staged = `${target}.next-${timestamp()}-${process.pid}`;
  try {
    copyFileSync(source, staged);
    chmodSync(staged, mode);
    renameSync(staged, target);
  } catch (error) {
    rmSync(staged, { force: true });
    throw error;
  }
}

function resolveInstallBin() {
  if (options.installBin) return resolve(options.installBin);
  const defaultPath = resolve(homedir(), '.local/bin/agent-browser');
  if (existsSync(defaultPath)) return defaultPath;
  const pathValue = commandOutput('sh', ['-lc', 'command -v agent-browser']).trim();
  if (pathValue) return resolve(pathValue);
  return defaultPath;
}

function guardInstallPath(path) {
  if (options.allowOutsideHome) return;
  const home = resolve(homedir());
  const resolved = resolve(path);
  if (resolved !== home && !resolved.startsWith(`${home}/`)) {
    throw new Error(`Refusing to replace a binary outside the current user's home without --allow-outside-home: ${resolved}`);
  }
}

async function restartOrStartDashboard(installBin, { restoring = false } = {}) {
  const status = serviceStatus();
  if (status.loadState === 'loaded') {
    report.service.action = restoring ? 'restart-after-restore' : 'restart';
    runCommand('systemctl', ['--user', 'restart', 'agent-browser-dashboard.service']);
    return;
  }
  if (!options.startIfMissing) {
    report.service.action = 'not-installed';
    return;
  }
  report.service.action = restoring ? 'start-after-restore' : 'start';
  runCommand(installBin, ['dashboard', 'start']);
}

function serviceStatus() {
  const result = spawnSync('systemctl', [
    '--user',
    'show',
    'agent-browser-dashboard.service',
    '--property=LoadState',
    '--property=ActiveState',
    '--property=MainPID',
    '--property=ActiveEnterTimestamp',
  ], {
    cwd: rootDir,
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    return {
      loadState: 'unknown',
      activeState: 'unknown',
      mainPid: null,
      activeEnterTimestamp: null,
      error: (result.stderr || result.stdout || '').trim(),
    };
  }
  const values = {};
  for (const line of result.stdout.split(/\r?\n/)) {
    const index = line.indexOf('=');
    if (index <= 0) continue;
    values[line.slice(0, index)] = line.slice(index + 1);
  }
  return {
    loadState: values.LoadState || 'unknown',
    activeState: values.ActiveState || 'unknown',
    mainPid: Number(values.MainPID || 0) || null,
    activeEnterTimestamp: values.ActiveEnterTimestamp || null,
  };
}

function runSmoke(installBin) {
  const smokeArgs = [
    'scripts/smoke-local-dashboard-runtime.js',
    '--dashboard-url',
    options.dashboardUrl,
    '--agent-browser-bin',
    installBin,
    '--json',
  ];
  for (const marker of options.expectMarkers) {
    smokeArgs.push('--expect-marker', marker);
  }
  if (!options.smokeBrowser) smokeArgs.push('--skip-browser');
  if (options.browserProfile) smokeArgs.push('--browser-profile', options.browserProfile);
  if (options.workspaceSession) smokeArgs.push('--workspace-session', options.workspaceSession);

  const result = spawnSync('node', smokeArgs, {
    cwd: rootDir,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const parsed = parseJson(result.stdout, 'local dashboard runtime smoke');
  if (result.status !== 0 || !parsed.success) {
    throw new Error(`Local dashboard runtime smoke failed: ${parsed.error || result.stderr || result.stdout}`);
  }
  return parsed;
}

function verifyRuntimeManifestReadback(installBin, manifest) {
  if (!manifest || manifest.schemaVersion !== 'agent-browser.runtime-manifest.v1') {
    throw new Error(`Live runtime manifest is missing or invalid: ${JSON.stringify(manifest)}`);
  }
  if (manifest.serviceContractVersion !== 'service-ui-runtime.v1') {
    throw new Error(`Live runtime manifest contract mismatch: ${manifest.serviceContractVersion}`);
  }
  const installedSha = sha256File(installBin);
  const manifestSha = manifest.executable?.sha256;
  if (manifestSha !== installedSha) {
    throw new Error(`Live runtime manifest executable sha mismatch: manifest=${manifestSha || 'missing'} installed=${installedSha}`);
  }
  if (typeof manifest.dashboard?.sha256 !== 'string' || manifest.dashboard.sha256.length !== 64) {
    throw new Error(`Live runtime manifest dashboard sha is missing: ${JSON.stringify(manifest.dashboard)}`);
  }
  const features = new Set(Array.isArray(manifest.supportedUiFeatures) ? manifest.supportedUiFeatures : []);
  for (const feature of ['workspace.detectedBrowsers', 'workspace.noRetainedLiveRail']) {
    if (!features.has(feature)) {
      throw new Error(`Live runtime manifest missing feature ${feature}`);
    }
  }
  return {
    schemaVersion: manifest.schemaVersion,
    packageVersion: manifest.packageVersion,
    serviceContractVersion: manifest.serviceContractVersion,
    dashboardSha256: manifest.dashboard.sha256,
    dashboardAssetCount: manifest.dashboard.assetCount,
    executablePath: manifest.executable?.path ?? null,
    executableSha256: manifestSha,
    installedSha256: installedSha,
    supportedUiFeatures: [...features].sort(),
  };
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function runCommand(command, commandArgs) {
  log(`$ ${command} ${commandArgs.join(' ')}`);
  const result = spawnSync(command, commandArgs, {
    cwd: rootDir,
    env: process.env,
    encoding: 'utf8',
    stdio: options.json ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  });
  if (options.json && result.stdout) process.stderr.write(result.stdout);
  if (options.json && result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`${command} ${commandArgs.join(' ')} failed with status ${result.status}`);
  }
}

function commandOutput(command, commandArgs, extra = {}) {
  try {
    return execFileSync(command, commandArgs, {
      cwd: rootDir,
      encoding: 'utf8',
      ...extra,
    });
  } catch {
    return '';
  }
}

function parseJson(text, label) {
  try {
    return JSON.parse(String(text).trim());
  } catch (error) {
    throw new Error(`Failed to parse ${label} JSON: ${error instanceof Error ? error.message : String(error)}\n${text}`);
  }
}

function timestamp() {
  const now = new Date();
  return now.toISOString().replace(/[-:]/g, '').replace(/\..+/, '').replace('T', '');
}

function log(message) {
  if (options.json) {
    process.stderr.write(`${message}\n`);
  }
}

function output(payload) {
  if (options.json) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }
  if (!payload.success) {
    console.error(payload.error);
    return;
  }
  console.log(`Published local dashboard runtime to ${payload.installBin}`);
  console.log(`Backup: ${payload.backupPath ?? 'none'}`);
  console.log(`Dashboard: ${payload.dashboardUrl}`);
  console.log(`Service PID: ${payload.service?.after?.mainPid ?? 'none'}`);
  if (payload.smoke?.browser) {
    console.log(`Browser smoke: ${payload.smoke.browser.smokeUrl}`);
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
  console.log(`Usage: node scripts/publish-local-dashboard-runtime.js [options]

Build and install the dashboard-embedded local agent-browser binary, restart the
user dashboard service, and verify the externally visible dashboard runtime.

Options:
  --dashboard-url <url>       Dashboard URL to smoke. Default: http://127.0.0.1:4848/
  --expect-marker <text>      Require served HTML or JS bundle to contain text. Repeatable.
  --browser-profile <path>    Use an isolated runtime profile for browser smoke.
  --install-bin <path>        Installed binary path. Default: ~/.local/bin/agent-browser.
  --release                   Build cli/target/release/agent-browser instead of debug.
  --skip-browser              Skip browser smoke, keep HTTP and bundle marker smoke.
  --skip-smoke                Build, install, and restart without smoke.
  --start-if-missing          Start dashboard if the user service is not installed.
  --workspace-session <name>  Smoke a workspace viewport route for a daemon session.
  --json                      Print structured JSON.
`);
}
