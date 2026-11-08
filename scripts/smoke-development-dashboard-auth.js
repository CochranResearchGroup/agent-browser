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
  // Native credentials are deliberately unusable, including valid retained
  // bootstrap credentials. This probe never signs in to the shared owner.
  const credentials = existsSync(authEnv) ? dashboardCredentials(readFileSync(authEnv, 'utf8'))
    : { username: 'admin', password: 'disabled-native-auth-probe' };
  const login = await fetch(new URL('/api/dashboard-auth/login', dashboardUrl), {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify(credentials), redirect: 'manual',
  });
  const payload = await login.json().catch(() => ({}));
  if (login.status !== 403 || payload.errorCode !== 'native_auth_disabled'
    || login.headers.getSetCookie().length) {
    throw new Error(`Native dashboard authentication was not disabled: HTTP ${login.status}`);
  }
  const response = await fetch(new URL('/api/dashboard-auth/status', dashboardUrl), { redirect: 'manual' });
  const status = await response.json().catch(() => ({}));
  if (status.authenticated === true || response.headers.getSetCookie().length) {
    throw new Error('Anonymous status unexpectedly established a dashboard session');
  }
  const manifestResponse = await fetch(new URL('/api/runtime/manifest', dashboardUrl));
  const manifest = await manifestResponse.json().catch(() => ({}));
  if (!manifestResponse.ok || manifest.runtimeEnvironment !== 'development') {
    throw new Error('Development runtime manifest identity failed');
  }
  const report = { success: true, dashboardOrigin: dashboardUrl.origin,
    nativeAuthenticationEnabled: false, authenticated: false,
    ownerAuthenticationAcceptance: 'not_run', runtimeEnvironment: manifest.runtimeEnvironment };
  if (json) console.log(JSON.stringify(report, null, 2));
  else console.log(`Development dashboard requires Authelia: ${dashboardUrl.origin}`);
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
