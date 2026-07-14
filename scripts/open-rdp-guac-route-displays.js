#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const reportOnly = process.argv.includes('--report-only');
const dryRun = process.argv.includes('--dry-run');
const waitMs = numberArg('--wait-ms') ?? 8000;
const agentBrowserTimeoutMs = numberArg('--agent-browser-timeout-ms') ?? 15000;

loadAgentBrowserEnv();

function numberArg(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return null;
  const value = Number.parseInt(process.argv[index + 1] || '', 10);
  return Number.isFinite(value) && value >= 0 ? value : null;
}

function commandResult(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
    ...options,
  });
}

function loadEnvFile(path) {
  if (!existsSync(path)) return;
  const text = readFileSync(path, 'utf8');
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separatorIndex = trimmed.indexOf('=');
    if (separatorIndex <= 0) continue;
    const key = trimmed.slice(0, separatorIndex).trim();
    let value = trimmed.slice(separatorIndex + 1).trim();
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    if (!process.env[key]) process.env[key] = value.replace(/\\"/g, '"');
  }
}

function loadAgentBrowserEnv() {
  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  loadEnvFile(join(agentHome, '.env'));
  loadEnvFile(process.env.AGENT_BROWSER_GUACAMOLE_SECRET_FILE || join(agentHome, 'secrets', 'guacamole.env'));
}

function commandExists(command) {
  const result = commandResult('sh', ['-lc', `command -v ${shellQuote(command)}`]);
  return result.status === 0 ? result.stdout.trim() : null;
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function agentBrowserCommand() {
  return process.env.AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD ||
    process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD ||
    commandExists('agent-browser') ||
    null;
}

function runAgentBrowser(args, label) {
  const command = agentBrowserCommand();
  if (!command) {
    throw new Error('agent_browser_command_missing: install agent-browser or set AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD');
  }
  const result = commandResult(command, args, { timeout: agentBrowserTimeoutMs });
  if (result.error) {
    throw new Error(`${label} failed: ${command} ${args.join(' ')}\n${result.error.message}`.trim());
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed: ${command} ${args.join(' ')}\n${result.stdout}${result.stderr}`.trim());
  }
  return parseJson(result.stdout, label);
}

function parseJson(text, label) {
  try {
    return JSON.parse(text.trim() || '{}');
  } catch (error) {
    throw new Error(`${label} JSON parse failed: ${error.message}\n${text}`);
  }
}

function routePoolFromEnv() {
  const raw = process.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON;
  if (!raw) return null;
  const parsed = JSON.parse(raw);
  if (!Array.isArray(parsed)) throw new Error('AGENT_BROWSER_RDP_ROUTE_POOL_JSON must be an array');
  return parsed;
}

function routePoolFromDoctor() {
  const doctor = runAgentBrowser(['doctor', 'remote-view', '--json'], 'remote-view doctor');
  return doctor?.data?.guacamole?.routePool?.data?.routePoolJson || [];
}

function routeUrl(route) {
  return route?.routeDescriptor?.localEmbedUrl ||
    route?.frameUrl ||
    route?.routeDescriptor?.dashboardEmbedUrl ||
    route?.routeDescriptor?.publicOperatorUrl ||
    route?.externalUrl ||
    route?.routeDescriptor?.externalUrl ||
    null;
}

function routeLabel(index) {
  return index === 0 ? 'A' : 'B';
}

function loadGuacamoleCredentials() {
  const username = process.env.GUACAMOLE_ADMIN_USERNAME;
  const password = process.env.GUACAMOLE_ADMIN_PASSWORD;
  if (!username || !password) {
    throw new Error('guacamole_credentials_missing: GUACAMOLE_ADMIN_USERNAME and GUACAMOLE_ADMIN_PASSWORD are required');
  }
  return { username, password };
}

function guacamoleHeaderUser() {
  return process.env.AGENT_BROWSER_GUACAMOLE_HEADER_USER ||
    process.env.GUACAMOLE_HEADER_USER ||
    process.env.REMOTE_USER ||
    process.env.USER ||
    null;
}

function acquireGuacamoleToken(baseUrl) {
  const headerUser = guacamoleHeaderUser();
  if (headerUser) {
    const headerToken = requestGuacamoleToken(baseUrl, {
      authMode: 'header',
      headerUser,
    });
    if (headerToken.ok) return headerToken;
  }
  const credentials = loadGuacamoleCredentials();
  return requestGuacamoleToken(baseUrl, {
    authMode: 'password',
    username: credentials.username,
    password: credentials.password,
  });
}

function requestGuacamoleToken(baseUrl, auth) {
  const tokenUrl = new URL('api/tokens', baseUrl).toString();
  const args = [
    '--insecure',
    '--silent',
    '--show-error',
    '--max-time',
    '8',
    '--request',
    'POST',
    '--header',
    'Content-Type: application/x-www-form-urlencoded',
  ];
  if (auth.authMode === 'header') {
    args.push('--header', `Remote-User: ${auth.headerUser}`, '--data', '');
  } else {
    args.push(
      '--data-urlencode',
      `username=${auth.username}`,
      '--data-urlencode',
      `password=${auth.password}`,
    );
  }
  args.push('--write-out', '\n%{http_code}', tokenUrl);
  const result = commandResult('curl', args);
  if (result.status !== 0) {
    return {
      ok: false,
      authMode: auth.authMode,
      statusCode: null,
      payload: null,
      error: (result.stderr || result.stdout || 'curl failed').trim(),
    };
  }
  const lines = result.stdout.split(/\r?\n/);
  const statusCode = Number.parseInt(lines.pop()?.trim() || '', 10);
  const body = lines.join('\n');
  let payload = null;
  try {
    payload = JSON.parse(body || '{}');
  } catch (error) {
    return {
      ok: false,
      authMode: auth.authMode,
      statusCode: Number.isInteger(statusCode) ? statusCode : null,
      payload: null,
      error: `failed to parse Guacamole token response: ${error.message}`,
    };
  }
  const ok = Number.isInteger(statusCode) &&
    statusCode >= 200 &&
    statusCode < 300 &&
    typeof payload.authToken === 'string' &&
    payload.authToken.length > 0;
  return {
    ok,
    authMode: auth.authMode,
    statusCode: Number.isInteger(statusCode) ? statusCode : null,
    payload,
    error: ok ? null : `Guacamole ${auth.authMode} token endpoint returned HTTP ${Number.isInteger(statusCode) ? statusCode : 'unknown'} without a usable auth token`,
  };
}

function base64(value) {
  return Buffer.from(value, 'utf8').toString('base64');
}

function openRoute(route, index) {
  const label = routeLabel(index);
  const url = routeUrl(route);
  if (!url) throw new Error(`route_${label.toLowerCase()}_url_missing: ${JSON.stringify(route)}`);
  const token = acquireGuacamoleToken(url);
  if (!token.ok) {
    throw new Error(`guacamole_route_${label.toLowerCase()}_login_failed: ${JSON.stringify({
      authMode: token.authMode,
      statusCode: token.statusCode,
      error: token.error,
    })}`);
  }

  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  const profileRoot = process.env.AGENT_BROWSER_RDP_ROUTE_VIEWER_PROFILE_ROOT ||
    join(agentHome, 'guacamole-route-viewers');
  mkdirSync(profileRoot, { recursive: true });
  const session = process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_VIEWER_SESSION`] ||
    `rdp-guac-route-${label.toLowerCase()}-viewer`;
  const profile = process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_VIEWER_PROFILE`] ||
    join(profileRoot, label.toLowerCase());
  const executable = process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_VIEWER_EXECUTABLE`] ||
    process.env.AGENT_BROWSER_RDP_ROUTE_VIEWER_EXECUTABLE ||
    null;

  const openArgs = [
    '--json',
    '--session',
    session,
    '--profile',
    profile,
    ...(executable ? ['--executable-path', executable] : []),
    '--args',
    '--no-sandbox',
    'open',
    url,
  ];
  const opened = runAgentBrowser(openArgs, `open Guacamole route ${label}`);
  const script = `
(async () => {
  const payload = ${JSON.stringify(token.payload)};
  localStorage.setItem("GUAC_AUTH", JSON.stringify(payload));
  window.location.reload();
  return { ok: true, authMode: ${JSON.stringify(token.authMode)}, username: payload.username, dataSource: payload.dataSource };
})()
`.trim();
  const login = runAgentBrowser([
    '--json',
    '--session',
    session,
    'eval',
    '--base64',
    base64(script),
  ], `login Guacamole route ${label}`);
  if (login?.data?.result?.ok !== true) {
    throw new Error(`guacamole_route_${label.toLowerCase()}_login_failed: ${JSON.stringify(login?.data?.result || login)}`);
  }
  return {
    label,
    session,
    profile,
    url,
    authMode: token.authMode,
    openedSuccess: opened.success === true,
    login: {
      success: login.success === true,
      data: {
        result: {
          ok: login.data?.result?.ok === true,
          authMode: login.data?.result?.authMode || token.authMode,
          username: login.data?.result?.username || null,
          dataSource: login.data?.result?.dataSource || null,
        },
      },
    },
  };
}

function inspectRouteDisplays() {
  const result = commandResult(process.execPath, ['scripts/inspect-rdp-route-displays.js', '--display-content']);
  const parsed = parseJson(result.stdout, 'route display inspector');
  return {
    exitCode: result.status,
    success: parsed.success === true,
    data: parsed,
    stderr: result.stderr.trim(),
  };
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

let output;
try {
  const routes = (routePoolFromEnv() || routePoolFromDoctor()).slice(0, 2);
  if (routes.length < 2) {
    throw new Error(`route_pool_missing: expected at least two route-pool entries, got ${routes.length}`);
  }
  const selectedRoutes = routes.map((route, index) => ({
    label: routeLabel(index),
    id: route.id || null,
    routeId: route.routeId || null,
    connectionId: route.connectionId || null,
    connectionName: route.connectionName || null,
    url: routeUrl(route),
    target: route.target || null,
  }));
  if (dryRun) {
    output = {
      success: true,
      status: 'dry_run',
      selectedRoutes,
      nextStep: 'Run pnpm open:rdp-route-displays to open both Guacamole route clients and inspect XRDP display allocation.',
    };
  } else {
    const openedRoutes = routes.map((route, index) => openRoute(route, index));
    if (waitMs > 0) sleep(waitMs);
    const routeDisplays = inspectRouteDisplays();
    output = {
      success: routeDisplays.success,
      status: routeDisplays.success ? 'ready' : 'blocked',
      selectedRoutes,
      openedRoutes,
      routeDisplays,
      nextStep: routeDisplays.success
        ? 'Route displays are distinct. Run pnpm test:rdp-guac-many-to-many-live.'
        : routeDisplays.data?.nextStep || 'Repair route display allocation, then rerun pnpm open:rdp-route-displays.',
    };
  }
} catch (error) {
  output = {
    success: false,
    status: 'failed',
    error: error.message,
    nextStep: 'Run agent-browser doctor remote-view --json and repair the first reported remote-view issue.',
  };
}

console.log(JSON.stringify(output, null, 2));

if (!output.success && !reportOnly) {
  process.exitCode = 1;
}
