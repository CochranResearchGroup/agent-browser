#!/usr/bin/env node

import { spawn } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { createServer } from 'node:net';
import { appendFile, lstat, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { homedir, tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

import { developmentRuntimeDescriptor } from './lib/development-runtime.js';
import { sha256 } from './lib/p158-campaign-controller.js';
import { createP158W6MalformedLineSeamReceipt } from './lib/p158-w6-five-surface-journal-calibration.js';

const REPO_ROOT = resolve(new URL('..', import.meta.url).pathname);

function fail(code, message) {
  throw Object.assign(new Error(message), { code });
}

function inside(parent, child) {
  const value = relative(resolve(parent), resolve(child));
  return value === '' || (!value.startsWith(`..${sep}`) && value !== '..' && !isAbsolute(value));
}

async function digestIfPresent(path) {
  try { return sha256(await readFile(path)); } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function reservePort() {
  const server = createServer();
  await new Promise((accept, reject) => server.once('error', reject).listen(0, '127.0.0.1', accept));
  const port = server.address().port;
  await new Promise((accept, reject) => server.close((error) => error ? reject(error) : accept()));
  return port;
}

function parseEnv(text) {
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
  return values;
}

async function fetchJson(fetchImpl, url, init = {}) {
  const response = await fetchImpl(url, { redirect: 'error', ...init });
  const body = await response.json().catch(() => null);
  if (!response.ok || body === null) fail('malformed_line_http_failed', `${url} returned HTTP ${response.status}`);
  return { response, body };
}

async function defaultLaunchDashboard(executable, environment) {
  const child = spawn(executable, [], {
    env: { ...environment, AGENT_BROWSER_DASHBOARD: '1', AGENT_BROWSER_DASHBOARD_BACKEND_ONLY: '1' },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let stderr = '';
  child.stderr.on('data', (chunk) => { stderr = `${stderr}${chunk}`.slice(-8192); });
  return {
    child,
    stderr: () => stderr,
    async stop() {
      if (child.exitCode !== null || child.signalCode !== null) return;
      child.kill('SIGTERM');
      const exited = new Promise((accept) => child.once('exit', accept));
      const forced = new Promise((accept) => setTimeout(() => { child.kill('SIGKILL'); accept(); }, 2_000));
      await Promise.race([exited, forced]);
    },
  };
}

async function waitForManifest(fetchImpl, origin, dashboard) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (dashboard.child.exitCode !== null) {
      fail('isolated_dashboard_exited', `Isolated candidate dashboard exited: ${dashboard.stderr()}`);
    }
    try {
      const response = await fetchImpl(new URL('/api/runtime/manifest', origin), { redirect: 'error' });
      if (response.ok) return response.json();
    } catch {}
    await new Promise((accept) => setTimeout(accept, 50));
  }
  fail('isolated_dashboard_timeout', 'Isolated candidate dashboard did not become ready');
}

export function p158W6ProtectedJournalPaths(descriptor) {
  return [...new Set([
    join(homedir(), '.agent-browser/service/failure-journal.jsonl'),
    join(descriptor.pseudoHome, '.agent-browser/service/failure-journal.jsonl'),
  ].map((path) => resolve(path)))];
}

/** Run the installed candidate writer and parser against one disposable journal. */
export async function executeP158W6MalformedLineLive({
  candidate, outputPath, isolationParent = tmpdir(), env = process.env,
  descriptor = developmentRuntimeDescriptor(env), fetch: fetchImpl = globalThis.fetch,
  launchDashboard = defaultLaunchDashboard, clock = () => new Date().toISOString(), protectedJournals,
} = {}) {
  if (!isAbsolute(outputPath ?? '') || !isAbsolute(isolationParent) || inside(REPO_ROOT, outputPath) ||
      !/^[a-f0-9]{64}$/u.test(candidate?.candidateSha256 ?? '') ||
      !/^[a-f0-9]{64}$/u.test(candidate?.executableSha256 ?? '') || !candidate?.installedGenerationId) {
    fail('malformed_line_live_input_invalid', 'Exact candidate identity and off-repository absolute paths are required');
  }
  const generation = await realpath(descriptor.current);
  const executable = join(generation, 'bin', 'agent-browser');
  const executableSha256 = createHash('sha256').update(await readFile(executable)).digest('hex');
  if (basename(generation) !== candidate.installedGenerationId || executableSha256 !== candidate.executableSha256) {
    fail('malformed_line_candidate_mismatch', 'Installed development candidate differs from the sealed identity');
  }
  const protectedPaths = protectedJournals ?? p158W6ProtectedJournalPaths(descriptor);
  const protectedBefore = await Promise.all(protectedPaths.map(digestIfPresent));
  const root = await mkdtemp(join(resolve(isolationParent), 'agent-browser-p158-malformed-'));
  if (inside(REPO_ROOT, root) || inside(descriptor.pseudoHome, root) || inside(root, descriptor.pseudoHome)) {
    fail('malformed_line_isolation_invalid', 'Disposable runtime overlaps repository or live development state');
  }
  const home = join(root, 'home');
  const socketDir = join(root, 'sockets');
  const runtimeDir = join(root, 'runtime');
  const authDir = join(root, 'auth');
  await Promise.all([home, socketDir, runtimeDir, authDir, join(root, 'config'), join(root, 'state')]
    .map((path) => mkdir(path, { recursive: true, mode: 0o700 })));
  const port = await reservePort();
  const origin = `http://127.0.0.1:${port}`;
  const isolatedEnv = {
    PATH: env.PATH, LANG: env.LANG ?? 'C.UTF-8', HOME: home,
    XDG_RUNTIME_DIR: runtimeDir, XDG_CONFIG_HOME: join(root, 'config'), XDG_STATE_HOME: join(root, 'state'),
    AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development', AGENT_BROWSER_SOCKET_DIR: socketDir,
    AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE: join(root, 'runtime-host-ingress.json'),
    AGENT_BROWSER_DASHBOARD_AUTH_DIR: authDir, AGENT_BROWSER_DASHBOARD_PORT: String(port),
  };
  const journalPath = join(home, '.agent-browser/service/failure-journal.jsonl');
  let dashboard;
  try {
    dashboard = await launchDashboard(executable, isolatedEnv);
    const manifest = await waitForManifest(fetchImpl, origin, dashboard);
    if (manifest?.runtimeEnvironment !== 'development' || manifest?.executable?.sha256 !== executableSha256) {
      fail('malformed_line_manifest_mismatch', 'Isolated dashboard did not report the exact development executable');
    }
    const credentials = parseEnv(await readFile(join(authDir, 'dashboard-auth.env'), 'utf8'));
    const username = credentials.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME ?? 'admin';
    const password = credentials.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD;
    if (!password) fail('malformed_line_auth_missing', 'Candidate did not create isolated dashboard credentials');
    const login = await fetchJson(fetchImpl, new URL('/api/dashboard-auth/login', origin), {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ username, password }),
    });
    const cookie = login.response.headers.get('set-cookie')?.split(';', 1)[0];
    if (login.body.authenticated !== true || !cookie) fail('malformed_line_auth_failed', 'Isolated dashboard login failed');
    const marker = randomBytes(8).toString('hex');
    const beforeCode = `p158_malformed_before_${marker}`;
    const afterCode = `p158_malformed_after_${marker}`;
    const submit = async (code) => fetchJson(fetchImpl, new URL('/api/service/failure-observation', origin), {
      method: 'POST', headers: { 'content-type': 'application/json', cookie },
      body: JSON.stringify({ category: 'dashboard_action', stage: 'p158_w6_malformed_line', code,
        summary: 'Synthetic isolated parser calibration.', action: 'journal_calibration',
        observationId: `p158-${code}` }),
    });
    const before = await submit(beforeCode);
    if (before.response.status !== 202 || before.body.success !== true) fail('malformed_line_before_failed', 'First record was not accepted');
    const journalRealParent = await realpath(dirname(journalPath));
    if (!inside(root, journalRealParent) || protectedPaths.includes(resolve(journalPath))) {
      fail('malformed_line_path_escape', 'Malformed line target escaped disposable runtime');
    }
    await appendFile(journalPath, '{"malformed":\n', { encoding: 'utf8', mode: 0o600 });
    const after = await submit(afterCode);
    if (after.response.status !== 202 || after.body.success !== true) fail('malformed_line_after_failed', 'Second record was not accepted');
    const readbackResult = await fetchJson(fetchImpl, new URL('/api/service/failures?limit=1000', origin), {
      headers: { cookie, accept: 'application/json' },
    });
    const readback = readbackResult.body?.data;
    const receipt = createP158W6MalformedLineSeamReceipt({
      candidateSha256: candidate.candidateSha256, executableSha256,
      installedGenerationId: candidate.installedGenerationId, isolationId: root,
      readback, beforeCode, afterCode, clock,
    });
    const protectedAfter = await Promise.all(protectedPaths.map(digestIfPresent));
    if (sha256(protectedBefore) !== sha256(protectedAfter)) {
      fail('protected_journal_mutated', 'Production or live development journal changed during isolated calibration');
    }
    const body = {
      schemaVersion: 'agent-browser.p158-w6-malformed-line-live-artifact.v1', planId: 'P158',
      candidateSha256: candidate.candidateSha256, executableSha256,
      installedGenerationIdSha256: sha256(candidate.installedGenerationId),
      isolatedRuntimeState: true, liveJournalMutated: false, productionJournalMutated: false,
      candidateWriterUsed: true, candidateReadbackUsed: true, dashboardBackendOnly: true,
      receipt, protectedJournalSnapshotSha256: sha256(protectedAfter), observedAt: clock(),
    };
    const artifact = { ...body, artifactSha256: sha256(body) };
    await writeFile(outputPath, `${JSON.stringify(artifact, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
    return artifact;
  } finally {
    await dashboard?.stop?.();
    const stat = await lstat(root).catch(() => null);
    if (stat?.isDirectory() && inside(resolve(isolationParent), root) && !inside(REPO_ROOT, root)) {
      await rm(root, { recursive: true, force: true });
    }
  }
}

function take(args, name) {
  const index = args.indexOf(name);
  if (index < 0 || !args[index + 1]) fail('missing_option', `${name} requires a value`);
  return args.splice(index, 2)[1];
}

export async function runCli(argv, dependencies = {}) {
  const args = argv.filter((arg) => arg !== '--');
  const configPath = take(args, '--config');
  const outputPath = take(args, '--output');
  if (args.length) fail('unknown_option', `Unknown arguments: ${args.join(' ')}`);
  const config = JSON.parse(await readFile(resolve(configPath), 'utf8'));
  const artifact = await executeP158W6MalformedLineLive({ ...config, outputPath, ...dependencies });
  (dependencies.stdout ?? process.stdout).write(`${JSON.stringify({ state: 'passed', artifactSha256: artifact.artifactSha256 })}\n`);
  return artifact;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  runCli(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${JSON.stringify({ error: error.code ?? 'p158_w6_malformed_line_failed', message: error.message })}\n`);
    process.exitCode = 1;
  });
}
