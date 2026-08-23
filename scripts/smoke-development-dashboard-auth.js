#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const args = process.argv.slice(2).filter((arg) => arg !== '--');
const dashboardUrl = new URL(takeOption(args, '--dashboard-url') || 'http://127.0.0.1:4948/');
const authEnv = takeOption(args, '--auth-env') ||
  join(homedir(), '.local', 'share', 'agent-browser-dev', 'home', '.agent-browser', 'dashboard-auth', 'dashboard-auth.env');
const json = removeFlag(args, '--json');
if (args.length) fail(`Unknown arguments: ${args.join(' ')}`);

try {
  if (!existsSync(authEnv)) throw new Error(`Development dashboard auth env is missing: ${authEnv}`);
  const credentials = dashboardCredentials(readFileSync(authEnv, 'utf8'));
  const login = await fetch(new URL('/api/dashboard-auth/login', dashboardUrl), {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(credentials),
    redirect: 'manual',
  });
  const loginPayload = await login.json().catch(() => ({}));
  if (!login.ok || loginPayload.authenticated !== true) {
    throw new Error(`Development dashboard login failed with HTTP ${login.status}`);
  }
  const cookie = login.headers.getSetCookie()
    .map((value) => value.split(';', 1)[0])
    .join('; ');
  if (!cookie) throw new Error('Development dashboard login did not issue a session cookie');

  const statusResponse = await fetch(
    new URL('/api/dashboard-auth/status', dashboardUrl),
    { headers: { cookie } },
  );
  const manifestResponse = await fetch(new URL('/api/runtime/manifest', dashboardUrl));
  const status = await statusResponse.json().catch(() => ({}));
  const manifest = await manifestResponse.json().catch(() => ({}));
  if (!statusResponse.ok || status.authenticated !== true) {
    throw new Error(`Development dashboard session verification failed with HTTP ${statusResponse.status}`);
  }
  if (!manifestResponse.ok || manifest.runtimeEnvironment !== 'development') {
    throw new Error(`Development runtime manifest identity failed with HTTP ${manifestResponse.status}`);
  }
  const serviceResponse = await fetchServiceStatus(dashboardUrl, cookie);

  const report = {
    success: true,
    dashboardOrigin: dashboardUrl.origin,
    loginStatus: login.status,
    authenticated: true,
    authenticatedServiceStatus: serviceResponse.status,
    runtimeEnvironment: manifest.runtimeEnvironment,
    executable: manifest.executable || null,
    dashboard: manifest.dashboard || null,
  };
  if (json) console.log(JSON.stringify(report, null, 2));
  else console.log(`Development dashboard authenticated smoke passed: ${dashboardUrl.origin}`);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}

function dashboardCredentials(text) {
  const values = parseEnv(text);
  const username = values.AGENT_BROWSER_DASHBOARD_ADMIN_USERNAME ||
    values.AGENT_BROWSER_DASHBOARD_CODEX_USERNAME ||
    'admin';
  const password = values.AGENT_BROWSER_DASHBOARD_ADMIN_PASSWORD ||
    values.AGENT_BROWSER_DASHBOARD_CODEX_PASSWORD;
  if (!password) throw new Error('Development dashboard auth env has no usable password');
  return { username, password };
}

async function fetchServiceStatus(dashboardUrl, cookie) {
  let response = null;
  for (let attempt = 0; attempt < 4; attempt += 1) {
    response = await fetch(new URL('/api/service/status', dashboardUrl), {
      headers: { cookie },
    });
    if (response.ok) return response;
    await new Promise((resolve) => setTimeout(resolve, 250 * (attempt + 1)));
  }
  throw new Error(`Authenticated development service API failed with HTTP ${response?.status || 0}`);
}

function parseEnv(text) {
  const values = {};
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const index = trimmed.indexOf('=');
    if (index <= 0) continue;
    const key = trimmed.slice(0, index).trim();
    let value = trimmed.slice(index + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    values[key] = value.replaceAll('\\"', '"');
  }
  return values;
}

function takeOption(values, option) {
  const index = values.indexOf(option);
  if (index < 0) return null;
  if (!values[index + 1]) fail(`${option} requires a value`);
  return values.splice(index, 2)[1];
}

function removeFlag(values, flag) {
  const index = values.indexOf(flag);
  if (index < 0) return false;
  values.splice(index, 1);
  return true;
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
