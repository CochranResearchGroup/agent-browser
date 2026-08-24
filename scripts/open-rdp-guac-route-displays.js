#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  canonicalRouteInventory,
  selectManagedRouteCandidates,
} from './lib/rdp-route-inventory.js';

const reportOnly = process.argv.includes('--report-only');
const dryRun = process.argv.includes('--dry-run');
const waitMs = numberArg('--wait-ms') ?? 8000;
const agentBrowserTimeoutMs = numberArg('--agent-browser-timeout-ms') ?? 600000;
const routeNavigationTimeoutMs = numberArg('--route-navigation-timeout-ms') ?? 600000;
const routeDisplayTimeoutMs = numberArg('--route-display-timeout-ms') ?? 600000;

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
    if (!Object.hasOwn(process.env, key)) {
      process.env[key] = value.replace(/\\"/g, '"');
    }
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

function runAgentBrowser(args, label, options = {}) {
  const command = agentBrowserCommand();
  if (!command) {
    throw new Error('agent_browser_command_missing: install agent-browser or set AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD');
  }
  const result = commandResult(command, args, {
    timeout: agentBrowserTimeoutMs,
    ...options,
  });
  if (result.error) {
    throw new Error(`${label} failed using ${command}\n${result.error.message}`.trim());
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed using ${command}\n${result.stdout}${result.stderr}`.trim());
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
  return canonicalRouteInventory(parsed);
}

function routePoolFromDoctor() {
  const doctor = runAgentBrowser(['doctor', 'remote-view', '--json'], 'remote-view doctor');
  return doctor?.data?.guacamole?.routePool?.data?.routePoolJson || [];
}

function routePoolFromDatabase() {
  const query = `
SELECT connection_id, connection_name
FROM guacamole_connection
WHERE parent_id IS NULL
  AND connection_name ~ '^Agent Browser RDP (Existing User Route|Route) ([AB]|[0-9]+)$'
ORDER BY connection_id;
`.trim();
  const result = commandResult('docker', [
    'exec',
    'agent-browser-guacamole-postgres',
    'psql',
    '-U',
    'guacamole_user',
    '-d',
    'guacamole_db',
    '-At',
    '-F',
    '\t',
    '-c',
    query,
  ]);
  if (result.status !== 0) return null;
  const rows = result.stdout.trim().split(/\r?\n/).filter(Boolean).map((line) => line.split('\t'));
  if (rows.length < 2 || rows.some(([id, name]) => !id || !name)) return null;
  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  const guacamolePort = envFileValue(
    join(agentHome, 'guacamole', '.env'),
    'AGENT_BROWSER_GUACAMOLE_HTTP_PORT',
  ) || '8092';
  const baseUrl = process.env.AGENT_BROWSER_GUACAMOLE_BASE_URL ||
    `http://127.0.0.1:${guacamolePort}/guacamole/`;
  const dataSource = process.env.AGENT_BROWSER_GUACAMOLE_DATA_SOURCE || 'postgresql';
  const connections = selectManagedRouteCandidates(rows.map(([connectionId, connectionName]) => ({
    connectionId,
    connectionName,
  })));
  return canonicalRouteInventory(connections.map(({ connectionId, connectionName }) => {
    const clientId = Buffer.from(`${connectionId}\0c\0${dataSource}`, 'utf8').toString('base64');
    const frameUrl = `${baseUrl.replace(/\/?$/, '/')}#/client/${clientId}`;
    return {
      id: `guacamole-rdp-${connectionId}`,
      routeId: `guacamole:${connectionId}`,
      connectionId,
      connectionName,
      frameUrl,
      routeDescriptor: { localEmbedUrl: frameUrl },
      target: {},
    };
  }));
}

function envFileValue(path, key) {
  if (!existsSync(path)) return null;
  for (const line of readFileSync(path, 'utf8').split(/\r?\n/)) {
    const [candidate, ...rest] = line.split('=');
    if (candidate?.trim() === key) return rest.join('=').trim().replace(/^['"]|['"]$/g, '');
  }
  return null;
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

function routeLabel(route, index) {
  return route.id || route.routeId || `route-${index + 1}`;
}

function routeSlug(route, index) {
  return routeLabel(route, index).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || `route-${index + 1}`;
}

function inspectedRoute(inspection, route, index) {
  const inventory = inspection.data?.routeInventory || [];
  return inventory.find((candidate) =>
    candidate.id === route.id ||
    candidate.routeId === route.routeId ||
    (candidate.connectionId && candidate.connectionId === route.connectionId),
  ) || inventory[index] || null;
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
    args.push('--data-binary', '@-');
  }
  args.push('--write-out', '\n%{http_code}', tokenUrl);
  const input = auth.authMode === 'password'
    ? new URLSearchParams({
      username: auth.username,
      password: auth.password,
    }).toString()
    : undefined;
  const result = commandResult('curl', args, { input });
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

function openRoute(route, index) {
  const label = routeLabel(route, index);
  const slug = routeSlug(route, index);
  const url = routeUrl(route);
  if (!url) throw new Error(`route_${slug}_url_missing: ${JSON.stringify(route)}`);
  const existingDisplay = inspectedRoute(inspectRouteDisplays(), route, index)?.displayName;
  const forceViewer = process.env.AGENT_BROWSER_ROUTE_DISPLAY_FORCE_VIEWER === '1';
  if (existingDisplay && !forceViewer) {
    return {
      label,
      session: null,
      profile: null,
      url,
      authMode: 'existing_session',
      displayName: existingDisplay,
      openedSuccess: true,
      reused: true,
      login: {
        success: true,
        data: {
          result: {
            ok: true,
            authMode: 'existing_session',
            username: null,
            dataSource: null,
          },
        },
      },
    };
  }
  const token = acquireGuacamoleToken(url);
  if (!token.ok) {
    throw new Error(`guacamole_route_${slug}_login_failed: ${JSON.stringify({
      authMode: token.authMode,
      statusCode: token.statusCode,
      error: token.error,
    })}`);
  }

  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  const profileRoot = process.env.AGENT_BROWSER_RDP_ROUTE_VIEWER_PROFILE_ROOT ||
    join(agentHome, 'guacamole-route-viewers');
  mkdirSync(profileRoot, { recursive: true });
  const session = route.viewerSession || `rdp-guac-${slug}-viewer`;
  const profile = route.viewerProfile || join(profileRoot, slug);
  if (profile.includes('/') || profile.includes('\\')) {
    mkdirSync(profile, { recursive: true });
  }
  const executable = route.viewerExecutable ||
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
    'about:blank',
  ];
  const opened = runAgentBrowser(openArgs, `open Guacamole route ${label}`);
  const headerUser = guacamoleHeaderUser();
  let displayName;
  try {
    if (token.authMode !== 'header' || !headerUser) {
      throw new Error(`guacamole_route_${slug}_header_auth_required`);
    }
    runAgentBrowser([
      '--json',
      '--session',
      session,
      '--profile',
      profile,
      'set',
      'headers',
      JSON.stringify({ 'Remote-User': headerUser }),
    ], `configure Guacamole header authentication for route ${label}`);
    navigateRoute([
      '--json',
      '--session',
      session,
      '--profile',
      profile,
      'open',
      url,
    ], `reload authenticated Guacamole route ${label}`);
    displayName = waitForRouteDisplay(route, index);
  } catch (error) {
    commandResult(agentBrowserCommand(), [
      '--json',
      '--session',
      session,
      '--profile',
      profile,
      'close',
    ], { timeout: 30000 });
    throw error;
  }
  return {
    label,
    session,
    profile,
    url,
    authMode: token.authMode,
    displayName,
    openedSuccess: opened.success === true,
    reused: false,
    login: {
      success: true,
      data: {
        result: {
          ok: true,
          authMode: token.authMode,
          username: token.payload?.username || headerUser,
          dataSource: token.payload?.dataSource || null,
        },
      },
    },
  };
}

function navigateRoute(args, label) {
  const command = agentBrowserCommand();
  if (!command) {
    throw new Error('agent_browser_command_missing: install agent-browser or set AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD');
  }
  const result = commandResult(command, args, { timeout: routeNavigationTimeoutMs });
  if (result.error?.code === 'ETIMEDOUT') return;
  if (result.error) {
    throw new Error(`${label} failed using ${command}\n${result.error.message}`.trim());
  }
  if (result.status !== 0) {
    throw new Error(`${label} failed using ${command}\n${result.stdout}${result.stderr}`.trim());
  }
}

function waitForRouteDisplay(route, index) {
  const label = routeLabel(route, index);
  const slug = routeSlug(route, index);
  const deadline = Date.now() + routeDisplayTimeoutMs;
  do {
    const inspection = inspectRouteDisplays();
    const displayName = inspectedRoute(inspection, route, index)?.displayName;
    if (displayName) return displayName;
    sleep(5000);
  } while (Date.now() < deadline);
  throw new Error(`route_${slug}_display_timeout: no route-specific Xorg display appeared`);
}

function inspectRouteDisplays() {
  const scriptRoot = process.env.AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT || 'scripts';
  const result = commandResult(process.execPath, [join(scriptRoot, 'inspect-rdp-route-displays.js'), '--display-content']);
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
  const routes = canonicalRouteInventory(routePoolFromEnv() || routePoolFromDatabase() || routePoolFromDoctor());
  if (routes.length < 2) {
    throw new Error(`route_pool_missing: expected at least two route-pool entries, got ${routes.length}`);
  }
  const selectedRoutes = routes.map((route, index) => ({
    label: routeLabel(route, index),
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
      nextStep: 'Run node scripts/open-rdp-guac-route-displays.js to open every configured Guacamole route client and inspect XRDP display allocation.',
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
        ? 'Route displays are distinct. Run the reviewed many-to-many live gate.'
        : routeDisplays.data?.nextStep || 'Repair route display allocation, then rerun node scripts/open-rdp-guac-route-displays.js.',
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
