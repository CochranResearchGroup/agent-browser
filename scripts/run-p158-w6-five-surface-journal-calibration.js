#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { basename, isAbsolute, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';

import { developmentRuntimeDescriptor } from './lib/development-runtime.js';
import {
  executeP158W6FiveSurfaceJournalCalibration,
} from './lib/p158-w6-five-surface-journal-calibration.js';
import { sha256 } from './lib/p158-campaign-controller.js';

function fail(message) {
  throw new Error(message);
}

function takeOption(args, name) {
  const index = args.indexOf(name);
  if (index < 0 || !args[index + 1]) fail(`${name} requires a value`);
  return args.splice(index, 2)[1];
}

function dashboardCredentials(text) {
  const values = {};
  for (const line of text.split(/\r?\n/u)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index <= 0) continue;
    let value = trimmed.slice(index + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) value = value.slice(1, -1);
    values[trimmed.slice(0, index).trim()] = value.replaceAll('\\"', '"');
  }
  const username = values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME ||
    values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME || 'admin';
  const password = values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD ||
    values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD;
  if (!password) fail('Development dashboard auth env has no usable password');
  return { username, password };
}

async function readJson(path, label) {
  if (!isAbsolute(path)) fail(`${label} path must be absolute`);
  try { return JSON.parse(await readFile(path, 'utf8')); } catch {
    fail(`${label} is not readable JSON`);
  }
}

function malformedLineReceiptFromArtifact(value) {
  if (value?.schemaVersion !== 'agent-browser.p158-w6-malformed-line-live-artifact.v1') return value;
  const { artifactSha256, ...body } = value;
  if (artifactSha256 !== sha256(body) || value.receipt?.candidateSha256 !== value.candidateSha256 ||
      value.receipt?.executableSha256 !== value.executableSha256 ||
      value.receipt?.installedGenerationIdSha256 !== value.installedGenerationIdSha256 ||
      value.candidateWriterUsed !== true || value.candidateReadbackUsed !== true ||
      value.liveJournalMutated !== false || value.productionJournalMutated !== false) {
    fail('Malformed-line live artifact is missing or changed');
  }
  return value.receipt;
}

async function authenticatedFetch(origin, authEnv, fetchImpl) {
  if (!isAbsolute(authEnv) || !existsSync(authEnv)) {
    fail('Development dashboard auth env must be an existing absolute path');
  }
  const credentials = dashboardCredentials(await readFile(authEnv, 'utf8'));
  const loginUrl = new URL('/api/dashboard-auth/login', origin);
  const login = await fetchImpl(loginUrl, {
    method: 'POST', redirect: 'error', headers: { 'content-type': 'application/json' },
    body: JSON.stringify(credentials),
  });
  const payload = await login.json().catch(() => ({}));
  if (!login.ok || payload.authenticated !== true) {
    fail(`Development dashboard login failed with HTTP ${login.status}`);
  }
  const cookie = login.headers.getSetCookie().map((value) => value.split(';', 1)[0]).join('; ');
  if (!cookie) fail('Development dashboard login did not issue a session cookie');
  return (url, options = {}) => fetchImpl(url, {
    ...options,
    headers: { ...(options.headers ?? {}), cookie },
  });
}

function runBounded(command, args, { env, timeoutMs = 15_000 } = {}) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(command, args, { env, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    let settled = false;
    const append = (current, chunk) => `${current}${chunk}`.slice(-16_384);
    child.stdout.on('data', (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = append(stderr, chunk); });
    const timer = setTimeout(() => {
      if (!settled) child.kill('SIGKILL');
    }, timeoutMs);
    child.once('error', (error) => {
      settled = true;
      clearTimeout(timer);
      rejectPromise(error);
    });
    child.once('close', (code, signal) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolvePromise({ code, signal, stdout, stderr });
    });
  });
}

export async function createDevelopmentBrowserManagerInducer({
  env = process.env,
  clock = () => new Date().toISOString(),
  descriptor = developmentRuntimeDescriptor(env),
  realpathImpl = realpath,
  readFileImpl = readFile,
  runProcess = runBounded,
  makeTemporaryHome = (prefix) => mkdtemp(prefix),
  removeTemporaryHome = (path) => rm(path, { recursive: true, force: true }),
} = {}) {
  const generationPath = await realpathImpl(descriptor.current);
  const executable = join(generationPath, 'bin', 'agent-browser');
  const executableSha256 = createHash('sha256').update(await readFileImpl(executable)).digest('hex');
  return async ({ calibrationKey, engine, candidate, environment }) => {
    if (environment.environmentId !== 'E2' || environment.runtimeLane !== 'development' ||
        environment.production !== false || basename(generationPath) !== candidate.installedGenerationId ||
        executableSha256 !== candidate.executableSha256) {
      fail('Installed development generation does not match the sealed W6 candidate');
    }
    const startedAt = clock();
    const clientHome = await makeTemporaryHome(join(tmpdir(), 'p158-w6-journal-client-'));
    let result;
    try {
      const childEnv = {
        ...env,
        HOME: clientHome,
        AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
        AGENT_BROWSER_RUNTIME_HOST: '1',
        AGENT_BROWSER_SOCKET_DIR: descriptor.socketDir,
        AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE: descriptor.runtimeHostIngressState,
      };
      delete childEnv.AGENT_BROWSER_PROFILE;
      delete childEnv.AGENT_BROWSER_RUNTIME_PROFILE;
      delete childEnv.AGENT_BROWSER_HEADED;
      result = await runProcess(executable, [
        '--json', '--session', `p158-w6-journal-${calibrationKey}`,
        '--engine', engine, 'open', 'about:blank',
      ], { env: childEnv });
    } finally {
      await removeTemporaryHome(clientHome);
    }
    const lines = result.stdout.trim().split(/\r?\n/u).filter(Boolean);
    let payload;
    try { payload = JSON.parse(lines.at(-1) ?? ''); } catch { payload = null; }
    if (result.signal || payload?.success !== false ||
        typeof payload?.error !== 'string' || !payload.error.includes(`Unknown engine '${engine}'`)) {
      fail('Development BrowserManager induction did not produce the expected invalid-engine failure');
    }
    return {
      schemaVersion: 'agent-browser.p158-w6-browser-manager-induction.v1',
      runtimeEnvironment: 'development', candidateSha256: candidate.candidateSha256,
      executableSha256, installedGenerationIdSha256: sha256(candidate.installedGenerationId),
      engine, browserManagerLaunchInvoked: true, browserProcessSpawnAttempted: false,
      clientHomeIsolated: true, clientHomeRemoved: true,
      resultState: 'failed_as_expected', startedAt, completedAt: clock(),
      retryAttempted: false, repairAttempted: false,
    };
  };
}

export async function runP158W6JournalCalibrationCli(argv, dependencies = {}) {
  const args = argv.filter((arg) => arg !== '--');
  const configPath = takeOption(args, '--config');
  const malformedReceiptPath = takeOption(args, '--malformed-line-receipt');
  const authEnv = takeOption(args, '--auth-env');
  const outputPath = takeOption(args, '--output');
  if (args.length) fail(`Unknown arguments: ${args.join(' ')}`);
  if (![configPath, malformedReceiptPath, authEnv, outputPath].every(isAbsolute)) {
    fail('All W6 journal calibration paths must be absolute');
  }
  const config = await readJson(configPath, 'Calibration config');
  const malformedLineReceipt = malformedLineReceiptFromArtifact(
    await readJson(malformedReceiptPath, 'Malformed-line receipt'),
  );
  if (config.environment?.environmentId !== 'E2' ||
      config.environment?.runtimeLane !== 'development' || config.environment?.production !== false ||
      config.candidate?.candidateSha256 !== malformedLineReceipt.candidateSha256) {
    fail('Calibration config is not bound to the exact E2 development candidate');
  }
  const origin = new URL(config.environment?.dashboardOrigin);
  const fetchImpl = dependencies.fetch ?? globalThis.fetch;
  const sessionFetch = dependencies.authenticatedFetch ??
    await authenticatedFetch(origin, authEnv, fetchImpl);
  const clock = dependencies.clock ?? (() => new Date().toISOString());
  const induceBrowserLaunchFailure = dependencies.induceBrowserLaunchFailure ??
    await createDevelopmentBrowserManagerInducer({ env: dependencies.env ?? process.env, clock });
  const artifact = await executeP158W6FiveSurfaceJournalCalibration({
    ...config, malformedLineReceipt, fetch: sessionFetch,
    clock, induceBrowserLaunchFailure,
    sleep: dependencies.sleep ?? ((milliseconds) =>
      new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds))),
  });
  await writeFile(outputPath, `${JSON.stringify(artifact, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  const summary = {
    state: artifact.resultState,
    artifact: basename(outputPath),
    artifactSha256: createHash('sha256').update(await readFile(outputPath)).digest('hex'),
  };
  (dependencies.stdout ?? process.stdout).write(`${JSON.stringify(summary)}\n`);
  return artifact;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  runP158W6JournalCalibrationCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${JSON.stringify({
      error: error?.code ?? 'p158_w6_journal_calibration_failed',
      message: error?.message ?? String(error),
    })}\n`);
    process.exitCode = 1;
  });
}
