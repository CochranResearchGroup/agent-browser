#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { resolve } from 'node:path';

const rootDir = new URL('..', import.meta.url).pathname;
const args = process.argv.slice(2);
const options = {
  dashboardUrl: process.env.AGENT_BROWSER_DASHBOARD_URL || 'http://127.0.0.1:4848/',
  expectMarkers: [],
  installBin: process.env.AGENT_BROWSER_INSTALL_BIN || '',
  json: false,
  browserProfile: '',
  release: false,
  skipSmoke: false,
  smokeBrowser: true,
  workspaceSession: '',
};

for (let index = 0; index < args.length; index += 1) {
  const arg = args[index];
  if (arg === '--') continue;
  if (arg === '--dashboard-url') {
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
  } else if (arg === '--workspace-session') {
    options.workspaceSession = requiredValue(args, ++index, arg);
  } else if (['--allow-outside-home', '--skip-reference-sync', '--start-if-missing'].includes(arg)) {
    fail(`${arg} is no longer supported because publication uses the canonical workstation transaction`);
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
  transaction: null,
  service: { before: null, after: null },
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
  const canonicalInstallBin = resolve(homedir(), '.local', 'bin', 'agent-browser');
  if (resolve(installBin) !== canonicalInstallBin) {
    throw new Error(
      `The canonical workstation transaction owns ${canonicalInstallBin}; ` +
      `refusing private binary replacement at ${installBin}`,
    );
  }
  report.installBin = installBin;

  runBuildCommand('pnpm', ['build:dashboard']);
  const cargoArgs = ['build', '--manifest-path', 'cli/Cargo.toml'];
  if (options.release) cargoArgs.push('--release');
  runBuildCommand(resolve(rootDir, 'scripts', 'ci', 'cargo-safe.sh'), cargoArgs);

  const builtBin = resolve(
    rootDir,
    'cli',
    'target',
    options.release ? 'release' : 'debug',
    'agent-browser',
  );
  if (!existsSync(builtBin)) throw new Error(`Built binary was not found: ${builtBin}`);
  report.builtBin = builtBin;
  report.service.before = serviceStatus();

  const install = spawnSync(
    builtBin,
    ['install', 'workstation', '--apply', '--json'],
    {
      cwd: rootDir,
      env: process.env,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      maxBuffer: 16 * 1024 * 1024,
    },
  );
  const installReport = parseJson(install.stdout, 'canonical workstation transaction');
  report.transaction = {
    path: installReport.runtimeCensusTransaction ?? null,
    state: installReport.state ?? null,
    ready: installReport.ready === true,
    phases: installReport.phases ?? [],
  };
  if (install.status !== 0 || installReport.success !== true) {
    throw new Error(
      `Canonical workstation transaction failed: ` +
      `${installReport.error || install.stderr || install.stdout}`,
    );
  }

  report.service.after = serviceStatus();
  if (!options.skipSmoke) {
    report.smoke = runSmoke(installBin);
    report.runtimeManifest = verifyRuntimeManifestReadback(
      installBin,
      report.smoke.runtimeManifest,
    );
  }
}

function resolveInstallBin() {
  return options.installBin
    ? resolve(options.installBin)
    : resolve(homedir(), '.local', 'bin', 'agent-browser');
}

function serviceStatus() {
  const result = spawnSync(
    'systemctl',
    [
      '--user',
      'show',
      'agent-browser-dashboard.service',
      '--property=LoadState',
      '--property=ActiveState',
      '--property=MainPID',
      '--property=ActiveEnterTimestamp',
    ],
    { cwd: rootDir, encoding: 'utf8' },
  );
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
    if (index > 0) values[line.slice(0, index)] = line.slice(index + 1);
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
  for (const marker of options.expectMarkers) smokeArgs.push('--expect-marker', marker);
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
    throw new Error(
      `Local dashboard runtime smoke failed: ${parsed.error || result.stderr || result.stdout}`,
    );
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
    throw new Error(
      `Live runtime manifest executable sha mismatch: ` +
      `manifest=${manifestSha || 'missing'} installed=${installedSha}`,
    );
  }
  if (typeof manifest.dashboard?.sha256 !== 'string' || manifest.dashboard.sha256.length !== 64) {
    throw new Error('Live runtime manifest dashboard sha is missing');
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
    supportedUiFeatures: [...(manifest.supportedUiFeatures || [])].sort(),
  };
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function runBuildCommand(command, commandArgs) {
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

function parseJson(text, label) {
  try {
    return JSON.parse(String(text).trim());
  } catch (error) {
    throw new Error(
      `Failed to parse ${label} JSON: ` +
      `${error instanceof Error ? error.message : String(error)}\n${text}`,
    );
  }
}

function log(message) {
  if (options.json) process.stderr.write(`${message}\n`);
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
  console.log(`Published local dashboard runtime through ${payload.transaction.path}`);
  console.log(`Dashboard: ${payload.dashboardUrl}`);
  console.log(`Service PID: ${payload.service?.after?.mainPid ?? 'none'}`);
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

Build the local dashboard and candidate binary, then submit that candidate to
the canonical workstation transaction. The publisher never replaces a stable
binary or runs a separate browser handoff lifecycle.

Options:
  --dashboard-url <url>       Dashboard URL to smoke. Default: http://127.0.0.1:4848/
  --expect-marker <text>      Require served HTML or JS bundle text. Repeatable.
  --browser-profile <path>    Use an isolated runtime profile for browser smoke.
  --install-bin <path>        Must name the canonical ~/.local/bin/agent-browser link.
  --release                   Build the release candidate instead of debug.
  --skip-browser              Skip browser smoke.
  --skip-smoke                Build and transact without the final smoke.
  --workspace-session <name>  Smoke a workspace viewport route for a daemon session.
  --json                      Print structured JSON.
`);
}
