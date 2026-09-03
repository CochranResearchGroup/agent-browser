#!/usr/bin/env node

import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { basename, join } from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { executeP158W7A11PredispatchProbe } from './lib/p158-w7-a11-predispatch-live.js';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const dashboardUrl = new URL(takeOption(args, '--dashboard-url') || 'http://127.0.0.1:4948/');
const authEnv = takeOption(args, '--auth-env') || join(homedir(), '.local', 'share',
  'agent-browser-dev', 'home', '.agent-browser', 'dashboard-auth', 'dashboard-auth.env');
const outputPath = takeOption(args, '--output') || join(tmpdir(),
  `agent-browser-p158-a11-${new Date().toISOString().replaceAll(/[:.]/gu, '-')}.json`);
if (args.length) fail(`Unknown arguments: ${args.join(' ')}`);

try {
  if (!existsSync(authEnv)) fail(`Development dashboard auth env is missing: ${authEnv}`);
  const credentials = dashboardCredentials(readFileSync(authEnv, 'utf8'));
  const login = await fetch(new URL('/api/dashboard-auth/login', dashboardUrl), {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify(credentials), redirect: 'manual',
  });
  const loginPayload = await login.json().catch(() => ({}));
  if (!login.ok || loginPayload.authenticated !== true) {
    fail(`Development dashboard login failed with HTTP ${login.status}`);
  }
  const cookie = login.headers.getSetCookie().map((value) => value.split(';', 1)[0]).join('; ');
  if (!cookie) fail('Development dashboard login did not issue a session cookie');
  const authenticatedFetch = (url, options = {}) => fetch(url, {
    ...options, headers: { ...(options.headers ?? {}), cookie },
  });
  const manifestResponse = await authenticatedFetch(new URL('/api/runtime/manifest', dashboardUrl));
  const manifest = await manifestResponse.json().catch(() => ({}));
  const executablePath = typeof manifest.executable === 'string'
    ? manifest.executable : manifest.executable?.path;
  const reportedExecutableSha256 = typeof manifest.executable === 'object'
    ? manifest.executable?.sha256 : null;
  if (!manifestResponse.ok || manifest.runtimeEnvironment !== 'development' ||
      typeof executablePath !== 'string' || !existsSync(executablePath)) {
    fail('Development runtime manifest does not identify an installed candidate executable');
  }
  const candidateSha256 = createHash('sha256').update(readFileSync(executablePath)).digest('hex');
  if (reportedExecutableSha256 && reportedExecutableSha256 !== candidateSha256) {
    fail('Development runtime manifest executable digest does not match the installed bytes');
  }
  const environmentSealSha256 = sha256({
    dashboardOrigin: dashboardUrl.origin,
    runtimeEnvironment: manifest.runtimeEnvironment,
    executable: executablePath,
    candidateSha256,
  });
  const receipt = await executeP158W7A11PredispatchProbe({
    runId: `p158-a11-prefreeze-${candidateSha256.slice(0, 12)}`,
    candidateSha256,
    environment: { environmentId: 'E1', runtimeLane: 'development', production: false,
      serviceOrigin: dashboardUrl.origin },
    environmentSealSha256, fetch: authenticatedFetch,
  });
  writeFileSync(outputPath, `${JSON.stringify(receipt, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  process.stdout.write(`P158 A11 live pre-dispatch probe passed: ${basename(outputPath)}\n`);
  process.stdout.write(`Receipt: ${outputPath}\n`);
} catch (error) {
  const details = error?.details ? ` ${JSON.stringify(error.details)}` : '';
  fail(`${error?.code ?? 'p158_a11_live_failed'}: ${error?.message ?? String(error)}${details}`);
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

function takeOption(values, option) {
  const index = values.indexOf(option);
  if (index < 0) return null;
  if (!values[index + 1]) fail(`${option} requires a value`);
  return values.splice(index, 2)[1];
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
