#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const reportOnly = process.argv.includes('--report-only');
const allowSharedTarget = process.argv.includes('--allow-shared-target');
const shellOutput = process.argv.includes('--shell');

loadAgentBrowserEnv();

function loadEnvFile(envPath) {
  if (!existsSync(envPath)) return;
  const text = readFileSync(envPath, 'utf8');
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

function commandResult(command, args) {
  return spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
  });
}

function routeDisplayProbe(displayName) {
  if (!displayName) {
    return {
      ok: false,
      evidence: 'route target has no displayName',
    };
  }
  const match = /^:(\d+)$/.exec(displayName);
  if (!match) {
    return {
      ok: false,
      evidence: `route target displayName '${displayName}' is not an X11 display name`,
    };
  }
  const socketPath = `/tmp/.X11-unix/X${match[1]}`;
  if (existsSync(socketPath)) {
    return {
      ok: true,
      evidence: `${displayName} has X11 socket ${socketPath}`,
    };
  }
  const abstractSocketName = `@/tmp/.X11-unix/X${match[1]}`;
  try {
    const unixSockets = readFileSync('/proc/net/unix', 'utf8');
    if (unixSockets.split(/\r?\n/).some((line) => line.includes(abstractSocketName))) {
      return {
        ok: true,
        evidence: `${displayName} has abstract X11 socket ${abstractSocketName}`,
      };
    }
  } catch {
    // Fall through to the filesystem-socket failure below.
  }
  return {
    ok: false,
    evidence: `${displayName} has no X11 socket at ${socketPath} or ${abstractSocketName}`,
  };
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, "'\\''")}'`;
}

function curlReachable(url) {
  const result = commandResult('curl', [
    '--insecure',
    '--location',
    '--silent',
    '--show-error',
    '--output',
    '/dev/null',
    '--write-out',
    '%{http_code}',
    '--max-time',
    '8',
    url,
  ]);
  const statusCode = Number.parseInt(result.stdout.trim(), 10);
  return {
    ok: result.status === 0 && Number.isInteger(statusCode) && statusCode >= 200 && statusCode < 500,
    statusCode: Number.isInteger(statusCode) ? statusCode : null,
    error: result.status === 0 ? null : (result.stderr || result.stdout || 'curl failed').trim(),
  };
}

function guacamoleHeaderUser() {
  return process.env.AGENT_BROWSER_GUACAMOLE_HEADER_USER ||
    process.env.GUACAMOLE_HEADER_USER ||
    process.env.REMOTE_USER ||
    process.env.USER ||
    null;
}

function guacamoleLoginReachable(baseUrl) {
  const headerUser = guacamoleHeaderUser();
  if (headerUser) {
    const headerProbe = guacamoleHeaderLoginReachable(baseUrl, headerUser);
    if (headerProbe.ok) return headerProbe;
  }

  const username = process.env.GUACAMOLE_ADMIN_USERNAME;
  const password = process.env.GUACAMOLE_ADMIN_PASSWORD;
  if (!username || !password) {
    return {
      ok: false,
      statusCode: null,
      authTokenPresent: false,
      error: headerUser
        ? `Guacamole header auth failed for configured header user and GUACAMOLE_ADMIN_USERNAME/GUACAMOLE_ADMIN_PASSWORD are unavailable`
        : 'GUACAMOLE_ADMIN_USERNAME and GUACAMOLE_ADMIN_PASSWORD are required for route-client acquisition',
    };
  }
  if (!baseUrl) {
    return {
      ok: false,
      statusCode: null,
      authTokenPresent: false,
      error: 'Guacamole base URL is not configured',
    };
  }
  const tokenUrl = new URL('api/tokens', baseUrl).toString();
  const result = commandResult('curl', [
    '--insecure',
    '--silent',
    '--show-error',
    '--max-time',
    '8',
    '--request',
    'POST',
    '--header',
    'Content-Type: application/x-www-form-urlencoded',
    '--data-urlencode',
    `username=${username}`,
    '--data-urlencode',
    `password=${password}`,
    '--write-out',
    '\n%{http_code}',
    tokenUrl,
  ]);
  if (result.status !== 0) {
    return {
      ok: false,
      statusCode: null,
      authTokenPresent: false,
      error: (result.stderr || result.stdout || 'curl failed').trim(),
    };
  }
  const lines = result.stdout.split(/\r?\n/);
  const statusCode = Number.parseInt(lines.pop()?.trim() || '', 10);
  const body = lines.join('\n');
  let authTokenPresent = false;
  try {
    const payload = JSON.parse(body || '{}');
    authTokenPresent = typeof payload.authToken === 'string' && payload.authToken.length > 0;
  } catch {
    authTokenPresent = false;
  }
  return {
    ok: Number.isInteger(statusCode) && statusCode >= 200 && statusCode < 300 && authTokenPresent,
    authMode: 'password',
    statusCode: Number.isInteger(statusCode) ? statusCode : null,
    authTokenPresent,
    error: authTokenPresent ? null : `Guacamole token endpoint returned HTTP ${Number.isInteger(statusCode) ? statusCode : 'unknown'} without a usable auth token`,
  };
}

function guacamoleHeaderLoginReachable(baseUrl, headerUser) {
  if (!baseUrl) {
    return {
      ok: false,
      authMode: 'header',
      statusCode: null,
      authTokenPresent: false,
      error: 'Guacamole base URL is not configured',
    };
  }
  const tokenUrl = new URL('api/tokens', baseUrl).toString();
  const result = commandResult('curl', [
    '--insecure',
    '--silent',
    '--show-error',
    '--max-time',
    '8',
    '--request',
    'POST',
    '--header',
    `Remote-User: ${headerUser}`,
    '--header',
    'Content-Type: application/x-www-form-urlencoded',
    '--data',
    '',
    '--write-out',
    '\n%{http_code}',
    tokenUrl,
  ]);
  if (result.status !== 0) {
    return {
      ok: false,
      authMode: 'header',
      statusCode: null,
      authTokenPresent: false,
      error: (result.stderr || result.stdout || 'curl failed').trim(),
    };
  }
  const lines = result.stdout.split(/\r?\n/);
  const statusCode = Number.parseInt(lines.pop()?.trim() || '', 10);
  const body = lines.join('\n');
  let authTokenPresent = false;
  try {
    const payload = JSON.parse(body || '{}');
    authTokenPresent = typeof payload.authToken === 'string' && payload.authToken.length > 0;
  } catch {
    authTokenPresent = false;
  }
  return {
    ok: Number.isInteger(statusCode) && statusCode >= 200 && statusCode < 300 && authTokenPresent,
    authMode: 'header',
    statusCode: Number.isInteger(statusCode) ? statusCode : null,
    authTokenPresent,
    error: authTokenPresent ? null : `Guacamole header token endpoint returned HTTP ${Number.isInteger(statusCode) ? statusCode : 'unknown'} without a usable auth token`,
  };
}

function dockerContainerRunning(name) {
  const result = commandResult('docker', ['inspect', '-f', '{{.State.Running}}', name]);
  return result.status === 0 && result.stdout.trim() === 'true';
}

function rdpTcpReachable(connection) {
  const hostname = connection.hostname || '';
  const port = String(connection.port || '3389');
  if (!hostname) {
    return {
      ok: false,
      evidence: 'connection is missing hostname',
    };
  }
  const result = commandResult('docker', [
    'exec',
    'agent-browser-guacd',
    'sh',
    '-lc',
    'nc -z -w 3 "$1" "$2"',
    'sh',
    hostname,
    port,
  ]);
  return {
    ok: result.status === 0,
    evidence: result.status === 0
      ? `${hostname}:${port} accepted a TCP connection from agent-browser-guacd`
      : `${hostname}:${port} did not accept a TCP connection from agent-browser-guacd: ${(result.stderr || result.stdout || 'nc failed').trim()}`,
  };
}

function queryGuacamoleConnections() {
  const sql = `
select coalesce(json_agg(row_to_json(t)), '[]'::json)
from (
  select
    c.connection_id::text as "connectionId",
    c.connection_name as "connectionName",
    c.protocol,
    max(case when p.parameter_name = 'hostname' then p.parameter_value end) as hostname,
    max(case when p.parameter_name = 'port' then p.parameter_value end) as port,
    max(case when p.parameter_name = 'username' then p.parameter_value end) as username,
    max(case when p.parameter_name = 'color-depth' then p.parameter_value end) as "colorDepth",
    c.max_connections as "maxConnections",
    c.max_connections_per_user as "maxConnectionsPerUser"
  from guacamole_connection c
  left join guacamole_connection_parameter p on p.connection_id = c.connection_id
  where c.protocol = 'rdp'
  group by
    c.connection_id,
    c.connection_name,
    c.protocol,
    c.max_connections,
    c.max_connections_per_user
  order by c.connection_id
) t;
`.trim();
  const result = commandResult('docker', [
    'exec',
    'agent-browser-guacamole-postgres',
    'psql',
    '-U',
    'guacamole_user',
    '-d',
    'guacamole_db',
    '-t',
    '-A',
    '-c',
    sql,
  ]);
  if (result.status !== 0) {
    return {
      ok: false,
      error: (result.stderr || result.stdout || 'failed to query Guacamole database').trim(),
      connections: [],
    };
  }
  try {
    return {
      ok: true,
      error: null,
      connections: JSON.parse(result.stdout.trim() || '[]'),
    };
  } catch (error) {
    return {
      ok: false,
      error: `failed to parse Guacamole connection query output: ${error.message}`,
      connections: [],
    };
  }
}

function queryGuacamoleSchema() {
  const requiredTables = [
    'guacamole_user',
    'guacamole_entity',
    'guacamole_connection',
    'guacamole_connection_parameter',
    'guacamole_connection_permission',
  ];
  const sql = `
select coalesce(json_agg(table_name order by table_name), '[]'::json)
from information_schema.tables
where table_schema = 'public'
  and table_name = any(array[${requiredTables.map((table) => `'${table}'`).join(', ')}]);
`.trim();
  const result = commandResult('docker', [
    'exec',
    'agent-browser-guacamole-postgres',
    'psql',
    '-U',
    'guacamole_user',
    '-d',
    'guacamole_db',
    '-t',
    '-A',
    '-c',
    sql,
  ]);
  if (result.status !== 0) {
    return {
      ok: false,
      error: (result.stderr || result.stdout || 'failed to query Guacamole schema').trim(),
      requiredTables,
      presentTables: [],
      missingTables: requiredTables,
    };
  }
  try {
    const presentTables = JSON.parse(result.stdout.trim() || '[]');
    const present = new Set(presentTables);
    const missingTables = requiredTables.filter((table) => !present.has(table));
    return {
      ok: missingTables.length === 0,
      error: missingTables.length === 0 ? null : `missing Guacamole table(s): ${missingTables.join(', ')}`,
      requiredTables,
      presentTables,
      missingTables,
    };
  } catch (error) {
    return {
      ok: false,
      error: `failed to parse Guacamole schema query output: ${error.message}`,
      requiredTables,
      presentTables: [],
      missingTables: requiredTables,
    };
  }
}

function queryGuacamoleConnectionPermissions(connectionIds) {
  const ids = connectionIds
    .map((id) => Number.parseInt(String(id), 10))
    .filter((id) => Number.isInteger(id) && id > 0);
  if (ids.length === 0) {
    return {
      ok: false,
      error: 'no selected Guacamole connection ids to check',
      connectionPermissions: [],
      missingReadConnectionIds: connectionIds,
    };
  }
  const sql = `
with selected(connection_id) as (
  values ${ids.map((id) => `(${id})`).join(', ')}
),
permission_counts as (
  select
    selected.connection_id::text as "connectionId",
    count(permission.*) filter (where permission.permission = 'READ')::int as "readGrantCount",
    count(distinct entity.entity_id) filter (where permission.permission = 'READ')::int as "readUserCount",
    count(distinct required_user.entity_id)::int as "requiredUserCount",
    coalesce(
      json_agg(required_user.name order by required_user.name)
        filter (where permission.entity_id is null),
      '[]'::json
    ) as "missingReadUsers"
  from selected
  cross join guacamole_entity required_user
  left join guacamole_connection_permission permission
    on permission.connection_id = selected.connection_id
   and permission.entity_id = required_user.entity_id
   and permission.permission = 'READ'
  left join guacamole_entity entity
    on entity.entity_id = permission.entity_id
   and entity.type = 'USER'
  where required_user.type = 'USER'
  group by selected.connection_id
)
select coalesce(json_agg(row_to_json(permission_counts) order by "connectionId"), '[]'::json)
from permission_counts;
`.trim();
  const result = commandResult('docker', [
    'exec',
    'agent-browser-guacamole-postgres',
    'psql',
    '-U',
    'guacamole_user',
    '-d',
    'guacamole_db',
    '-t',
    '-A',
    '-c',
    sql,
  ]);
  if (result.status !== 0) {
    return {
      ok: false,
      error: (result.stderr || result.stdout || 'failed to query Guacamole connection permissions').trim(),
      connectionPermissions: [],
      missingReadConnectionIds: connectionIds,
    };
  }
  try {
    const connectionPermissions = JSON.parse(result.stdout.trim() || '[]');
    const missingReadConnectionIds = connectionPermissions
      .filter((entry) => Number(entry.readGrantCount || 0) < Number(entry.requiredUserCount || 0))
      .map((entry) => entry.connectionId);
    const missingReadUsers = connectionPermissions.flatMap((entry) =>
      (entry.missingReadUsers || []).map((user) => `${entry.connectionId}:${user}`),
    );
    return {
      ok: missingReadConnectionIds.length === 0,
      error: missingReadConnectionIds.length === 0
        ? null
        : `missing READ permission for Guacamole connection/user(s): ${missingReadUsers.join(', ')}`,
      connectionPermissions,
      missingReadConnectionIds,
    };
  } catch (error) {
    return {
      ok: false,
      error: `failed to parse Guacamole permission query output: ${error.message}`,
      connectionPermissions: [],
      missingReadConnectionIds: connectionIds,
    };
  }
}

function guacamoleBaseUrl() {
  const configured = process.env.AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL ||
    process.env.AGENT_BROWSER_REMOTE_VIEW_FRAME_URL ||
    process.env.AGENT_BROWSER_REMOTE_VIEW_URL ||
    'http://127.0.0.1:8092/guacamole/';
  const hashIndex = configured.indexOf('#');
  const withoutHash = hashIndex >= 0 ? configured.slice(0, hashIndex) : configured;
  return withoutHash.endsWith('/') ? withoutHash : `${withoutHash}/`;
}

function isLocalUrl(value) {
  try {
    const url = new URL(value);
    const host = url.hostname.toLowerCase();
    return host === 'localhost' || host === '127.0.0.1' || host === '::1';
  } catch {
    return false;
  }
}

function normalizeGuacamoleBaseUrl(value) {
  if (!value) return null;
  const hashIndex = value.indexOf('#');
  const withoutHash = hashIndex >= 0 ? value.slice(0, hashIndex) : value;
  return withoutHash.endsWith('/') ? withoutHash : `${withoutHash}/`;
}

function guacamoleRouteBases() {
  const configuredBase = guacamoleBaseUrl();
  const localBase = normalizeGuacamoleBaseUrl(process.env.AGENT_BROWSER_REMOTE_VIEW_LOCAL_URL) ||
    (isLocalUrl(configuredBase) ? configuredBase : 'http://127.0.0.1:8092/guacamole/');
  const publicBase = normalizeGuacamoleBaseUrl(process.env.AGENT_BROWSER_REMOTE_VIEW_PUBLIC_URL) ||
    normalizeGuacamoleBaseUrl(process.env.AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL) ||
    (!isLocalUrl(configuredBase) ? configuredBase : null);
  const healthBase = normalizeGuacamoleBaseUrl(process.env.AGENT_BROWSER_REMOTE_VIEW_HEALTH_URL) ||
    localBase ||
    publicBase ||
    configuredBase;
  return {
    configuredBase,
    localBase,
    publicBase,
    healthBase,
  };
}

function guacamoleClientId(connectionId) {
  return Buffer.from(`${connectionId}\0c\0postgresql`, 'utf8').toString('base64');
}

function targetIdentity(connection) {
  return [
    connection.hostname || 'unknown-host',
    connection.port || '3389',
    connection.username || 'unknown-user',
    `bpp:${connection.colorDepth || 'default'}`,
  ].join(':');
}

function targetIdentityKey(connection) {
  const usernameKey = connection.username
    ? createHash('sha256').update(connection.username).digest('hex').slice(0, 10)
    : 'none';
  return [
    connection.hostname || 'unknown-host',
    connection.port || '3389',
    `user:${usernameKey}`,
    `bpp:${connection.colorDepth || 'default'}`,
  ].join(':');
}

function inspectRouteDisplays() {
  const result = commandResult(process.execPath, ['scripts/inspect-rdp-route-displays.js']);
  if (result.status !== 0) return {};
  try {
    const parsed = JSON.parse(result.stdout.trim());
    return parsed.env || {};
  } catch {
    return {};
  }
}

function redactConnection(connection) {
  return {
    connectionId: connection.connectionId,
    connectionName: connection.connectionName,
    protocol: connection.protocol,
    hostname: connection.hostname || null,
    port: connection.port || null,
    colorDepth: connection.colorDepth || null,
    usernamePresent: Boolean(connection.username),
    maxConnections: connection.maxConnections ?? null,
    maxConnectionsPerUser: connection.maxConnectionsPerUser ?? null,
    targetIdentityKey: targetIdentityKey(connection),
  };
}

const inferredRouteDisplays = inspectRouteDisplays();

function routeTargetDisplayName(index) {
  const label = index === 0 ? 'A' : 'B';
  return process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_DISPLAY_NAME`] ||
    inferredRouteDisplays[`AGENT_BROWSER_RDP_ROUTE_${label}_DISPLAY_NAME`] ||
    null;
}

function routeTargetUser(connection, index) {
  const label = index === 0 ? 'A' : 'B';
  const configured = process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_USERNAME`] ||
    inferredRouteDisplays[`AGENT_BROWSER_RDP_ROUTE_${label}_USERNAME`];
  if (configured) return configured;
  if (/^Agent Browser RDP Existing User Route [AB]$/.test(connection.connectionName || '')) {
    return process.env.AGENT_BROWSER_RDP_USERNAME || 'agent-browser-rdp';
  }
  if (/^Agent Browser RDP Route [AB]$/.test(connection.connectionName || '')) {
    return label === 'A'
      ? (process.env.AGENT_BROWSER_RDP_ROUTE_A_USERNAME || 'agent-browser-rdp-a')
      : (process.env.AGENT_BROWSER_RDP_ROUTE_B_USERNAME || 'agent-browser-rdp-b');
  }
  return null;
}

function routePoolCandidates(connections) {
  const routeSpecific = connections.filter((connection) =>
    /^Agent Browser RDP Route [AB]$/.test(connection.connectionName || ''),
  );
  if (routeSpecific.length >= 2) return routeSpecific.slice(0, 2);

  const existingUser = connections.filter((connection) =>
    /^Agent Browser RDP Existing User Route [AB]$/.test(connection.connectionName || ''),
  );
  if (existingUser.length >= 2) return existingUser.slice(0, 2);

  const managed = connections.filter((connection) =>
    /^Agent Browser RDP (Existing User Route|Route) [AB]$/.test(connection.connectionName || ''),
  );
  return (managed.length >= 2 ? managed : connections).slice(0, 2);
}

function routePoolEntry(connection, index, routeBases, routeReadiness) {
  const label = index === 0 ? 'a' : 'b';
  const displayName = routeTargetDisplayName(index);
  const routeUser = routeTargetUser(connection, index);
  const clientId = guacamoleClientId(connection.connectionId);
  const localEmbedUrl = routeBases.localBase ? `${routeBases.localBase}#/client/${clientId}` : null;
  const publicOperatorUrl = routeBases.publicBase ? `${routeBases.publicBase}#/client/${clientId}` : null;
  const healthUrl = routeBases.healthBase ? `${routeBases.healthBase}#/client/${clientId}` : null;
  const frameUrl = localEmbedUrl || publicOperatorUrl || healthUrl;
  const externalUrl = publicOperatorUrl || frameUrl;
  return {
    id: `guacamole-rdp-${label}`,
    routeId: `guacamole:${connection.connectionId}`,
    connectionId: connection.connectionId,
    connectionName: connection.connectionName,
    frameUrl,
    externalUrl,
    routeDescriptor: {
      localEmbedUrl,
      publicOperatorUrl,
      dashboardEmbedUrl: localEmbedUrl || publicOperatorUrl,
      healthUrl,
      externalUrl,
      embeddingPolicy: localEmbedUrl ? 'local_embed_preferred' : 'public_diagnostic_only',
      providerMode: 'simultaneous_view',
    },
    providerMode: 'simultaneous_view',
    target: {
      hostname: connection.hostname || null,
      port: connection.port || '3389',
      colorDepth: connection.colorDepth || null,
      targetIdentityKey: targetIdentityKey(connection),
      ...(displayName ? { displayName } : {}),
      ...(routeUser ? { routeUser } : {}),
    },
    readiness: routeReadiness,
  };
}

function readiness(status, evidence, nextAction, recovery) {
  return { status, evidence, nextAction, recovery };
}

const dockerReady = dockerContainerRunning('agent-browser-guacamole-postgres');
const guacamoleReady = dockerContainerRunning('agent-browser-guacamole');
const guacdReady = dockerContainerRunning('agent-browser-guacd');
const routeBases = guacamoleRouteBases();
const localGuacamoleWebProbe = guacamoleReady && routeBases.localBase ? curlReachable(routeBases.localBase) : {
  ok: false,
  statusCode: null,
  error: routeBases.localBase ? 'agent-browser-guacamole is not running' : 'local Guacamole base URL is not configured',
};
const publicGuacamoleWebProbe = guacamoleReady && routeBases.publicBase ? curlReachable(routeBases.publicBase) : {
  ok: false,
  statusCode: null,
  error: routeBases.publicBase ? 'agent-browser-guacamole is not running' : 'public Guacamole base URL is not configured',
};
const guacamoleWebProbe = guacamoleReady ? curlReachable(routeBases.healthBase) : {
  ok: false,
  statusCode: null,
  error: 'agent-browser-guacamole is not running',
};
const guacamoleLoginProbe = guacamoleReady ? guacamoleLoginReachable(routeBases.healthBase) : {
  ok: false,
  authMode: null,
  statusCode: null,
  authTokenPresent: false,
  error: 'agent-browser-guacamole is not running',
};
const query = dockerReady ? queryGuacamoleConnections() : {
  ok: false,
  error: 'agent-browser-guacamole-postgres is not running',
  connections: [],
};
const connections = query.connections || [];
const selectedConnections = routePoolCandidates(connections);
const schema = dockerReady ? queryGuacamoleSchema() : {
  ok: false,
  error: 'agent-browser-guacamole-postgres is not running',
  requiredTables: [
    'guacamole_user',
    'guacamole_entity',
    'guacamole_connection',
    'guacamole_connection_parameter',
    'guacamole_connection_permission',
  ],
  presentTables: [],
  missingTables: [],
};
const permissions = dockerReady && schema.ok
  ? queryGuacamoleConnectionPermissions(selectedConnections.map((connection) => connection.connectionId))
  : {
      ok: false,
      error: schema.ok ? 'no selected Guacamole connections' : 'Guacamole schema is not ready',
      connectionPermissions: [],
      missingReadConnectionIds: selectedConnections.map((connection) => connection.connectionId),
    };
const rdpTcpProbes = guacdReady
  ? new Map(selectedConnections.map((connection) => [connection.connectionId, rdpTcpReachable(connection)]))
  : new Map(selectedConnections.map((connection) => [
      connection.connectionId,
      {
        ok: false,
        evidence: 'agent-browser-guacd is not running',
      },
    ]));
const routeDisplayProbes = new Map(selectedConnections.map((connection, index) => [
  connection.connectionId,
  routeDisplayProbe(routeTargetDisplayName(index)),
]));
const redactedConnections = connections.map(redactConnection);
const targetIdentities = connections.map(targetIdentity);
const selectedTargetIdentities = selectedConnections.map(targetIdentity);
const distinctTargetIdentities = new Set(targetIdentities);
const distinctSelectedTargetIdentities = new Set(selectedTargetIdentities);
const hasTwoConnections = connections.length >= 2;
const hasTwoSelectedConnections = selectedConnections.length >= 2;
const hasTwoDistinctTargets = distinctSelectedTargetIdentities.size >= 2;
const selectedRdpTargetsReady = selectedConnections.length > 0 &&
  selectedConnections.every((connection) => rdpTcpProbes.get(connection.connectionId)?.ok === true);
const selectedRouteDisplaysReady = selectedConnections.length > 0 &&
  selectedConnections.every((connection) => routeDisplayProbes.get(connection.connectionId)?.ok === true);
const ready = Boolean(
  dockerReady &&
    guacamoleReady &&
    guacdReady &&
    schema.ok &&
    guacamoleWebProbe.ok &&
    guacamoleLoginProbe.ok &&
    query.ok &&
    permissions.ok &&
    hasTwoSelectedConnections &&
    selectedRdpTargetsReady &&
    selectedRouteDisplaysReady &&
    (hasTwoDistinctTargets || allowSharedTarget),
);
const routePoolJson = ready || hasTwoSelectedConnections
  ? selectedConnections.map((connection, index) => {
      const rdpProbe = rdpTcpProbes.get(connection.connectionId);
      const displayProbe = routeDisplayProbes.get(connection.connectionId);
      const routeReady = guacamoleWebProbe.ok &&
        guacamoleLoginProbe.ok &&
        rdpProbe?.ok === true &&
        displayProbe?.ok === true;
      return routePoolEntry(connection, index, routeBases, {
        state: routeReady ? 'ready' : 'failed',
        components: [
          {
            component: 'guacamole_web_route',
            status: guacamoleWebProbe.ok ? 'ready' : 'failed',
            evidence: guacamoleWebProbe.ok
              ? `${routeBases.healthBase} returned HTTP ${guacamoleWebProbe.statusCode}`
              : guacamoleWebProbe.error,
          },
          {
            component: 'guacamole_local_embed_route',
            status: localGuacamoleWebProbe.ok ? 'ready' : 'failed',
            evidence: localGuacamoleWebProbe.ok
              ? `${routeBases.localBase} returned HTTP ${localGuacamoleWebProbe.statusCode}`
              : localGuacamoleWebProbe.error,
          },
          {
            component: 'guacamole_public_operator_route',
            status: publicGuacamoleWebProbe.ok ? 'ready' : 'missing',
            evidence: publicGuacamoleWebProbe.ok
              ? `${routeBases.publicBase} returned HTTP ${publicGuacamoleWebProbe.statusCode}`
              : publicGuacamoleWebProbe.error,
          },
          {
            component: 'guacamole_login',
            status: guacamoleLoginProbe.ok ? 'ready' : 'failed',
            evidence: guacamoleLoginProbe.ok
              ? `Guacamole ${guacamoleLoginProbe.authMode || 'configured'} token endpoint returned HTTP ${guacamoleLoginProbe.statusCode} with an auth token`
              : guacamoleLoginProbe.error,
          },
          {
            component: 'guacamole_connection_permissions',
            status: permissions.ok ? 'ready' : 'failed',
            evidence: permissions.ok
              ? `selected Guacamole connection(s) have READ grants: ${permissions.connectionPermissions.map((entry) => `${entry.connectionId}:${entry.readGrantCount}`).join(', ')}`
              : permissions.error,
          },
          {
            component: 'rdp_backend_tcp',
            status: rdpProbe?.ok ? 'ready' : 'failed',
            evidence: rdpProbe?.evidence || 'RDP TCP probe did not run',
          },
          {
            component: 'route_display_socket',
            status: displayProbe?.ok ? 'ready' : 'failed',
            evidence: displayProbe?.evidence || 'route display probe did not run',
          },
        ],
      });
    })
  : [];

const components = [
  {
    component: 'guacamole_postgres',
    ...readiness(
      dockerReady ? 'ready' : 'failed',
      dockerReady ? 'agent-browser-guacamole-postgres is running' : 'agent-browser-guacamole-postgres is not running',
      dockerReady ? 'none' : 'start_guacamole_compose_stack',
      dockerReady ? 'Guacamole database container is available.' : 'Run docker compose up -d from ~/.agent-browser/guacamole, then rerun this route-pool readiness smoke.',
    ),
  },
  {
    component: 'guacamole_schema',
    ...readiness(
      schema.ok ? 'ready' : 'failed',
      schema.ok
        ? `required Guacamole table(s) present: ${schema.requiredTables.join(', ')}`
        : schema.error || 'required Guacamole schema tables are missing',
      schema.ok ? 'none' : 'initialize_guacamole_schema',
      schema.ok
        ? 'Guacamole database schema is present.'
        : 'Run pnpm ensure:rdp-guac-postgres -- --apply to repair an empty initialized Guacamole PostgreSQL database before validating route-pool entries.',
    ),
  },
  {
    component: 'guacamole_web',
    ...readiness(
      guacamoleReady && guacamoleWebProbe.ok ? 'ready' : 'failed',
      guacamoleReady && guacamoleWebProbe.ok
        ? `agent-browser-guacamole is running and ${routeBases.healthBase} returned HTTP ${guacamoleWebProbe.statusCode}`
        : guacamoleWebProbe.error || 'agent-browser-guacamole is not reachable',
      guacamoleReady && guacamoleWebProbe.ok ? 'none' : 'repair_guacamole_web_route',
      guacamoleReady && guacamoleWebProbe.ok
        ? 'Guacamole web container and ingress route are available.'
        : 'Start or repair the Guacamole web container and ingress route before validating route-pool entries.',
    ),
  },
  {
    component: 'guacamole_login',
    ...readiness(
      guacamoleLoginProbe.ok ? 'ready' : 'failed',
      guacamoleLoginProbe.ok
        ? `Guacamole ${guacamoleLoginProbe.authMode || 'configured'} token endpoint returned HTTP ${guacamoleLoginProbe.statusCode} with an auth token`
        : guacamoleLoginProbe.error,
      guacamoleLoginProbe.ok ? 'none' : 'repair_guacamole_admin_credentials',
      guacamoleLoginProbe.ok
        ? 'Guacamole credentials can acquire an authenticated route-client token.'
        : 'Repair the Guacamole admin credentials in the agent-browser secret file before opening route clients.',
    ),
  },
  {
    component: 'guacd',
    ...readiness(
      guacdReady ? 'ready' : 'failed',
      guacdReady ? 'agent-browser-guacd is running' : 'agent-browser-guacd is not running',
      guacdReady ? 'none' : 'start_guacamole_compose_stack',
      guacdReady ? 'guacd container is available.' : 'Start the guacd container before validating route-pool entries.',
    ),
  },
  {
    component: 'guacamole_rdp_connections',
    ...readiness(
      query.ok && hasTwoConnections ? 'ready' : 'blocked',
      query.ok
        ? `found ${connections.length} RDP Guacamole connection(s)`
        : query.error,
      query.ok && hasTwoConnections ? 'none' : 'provision_second_guacamole_rdp_connection',
      query.ok && hasTwoConnections
        ? 'At least two Guacamole RDP connection records are present.'
        : 'Provision at least two Guacamole RDP connections before running the many-to-many live gate.',
    ),
  },
  {
    component: 'guacamole_connection_permissions',
    ...readiness(
      permissions.ok ? 'ready' : 'blocked',
      permissions.ok
        ? `selected Guacamole connection(s) have READ grants: ${permissions.connectionPermissions.map((entry) => `${entry.connectionId}:${entry.readGrantCount}`).join(', ')}`
        : permissions.error,
      permissions.ok ? 'none' : 'repair_guacamole_connection_permissions',
      permissions.ok
        ? 'Selected Guacamole route connections are visible to at least one user entity.'
        : 'Grant READ permission on every selected Guacamole route connection before treating the route pool as ready.',
    ),
  },
  {
    component: 'distinct_rdp_targets',
    ...readiness(
      hasTwoSelectedConnections && (hasTwoDistinctTargets || allowSharedTarget) ? 'ready' : 'blocked',
      `found ${distinctSelectedTargetIdentities.size} distinct selected target identity key(s) across ${selectedConnections.length} selected route candidate(s); ${distinctTargetIdentities.size} distinct target identity key(s) across ${connections.length} RDP connection(s)`,
      hasTwoSelectedConnections && (hasTwoDistinctTargets || allowSharedTarget) ? 'none' : 'provision_distinct_rdp_targets',
      hasTwoSelectedConnections && hasTwoDistinctTargets
        ? 'The selected route candidates point at distinct target identities.'
        : 'P03 requires routes that do not collapse onto the same shared desktop. Use distinct RDP targets or pass --allow-shared-target only for diagnostics.',
    ),
  },
  ...selectedConnections.map((connection) => {
    const probe = rdpTcpProbes.get(connection.connectionId);
    return {
      component: `rdp_backend_tcp:${connection.connectionId}`,
      ...readiness(
        probe?.ok ? 'ready' : 'failed',
        probe?.evidence || 'RDP TCP probe did not run',
        probe?.ok ? 'none' : 'repair_rdp_backend_reachability',
        probe?.ok
          ? 'guacd can reach this RDP backend over TCP.'
          : 'Repair the RDP backend host, port, firewall, or Docker host routing before using this route.',
      ),
    };
  }),
  ...selectedConnections.map((connection) => {
    const probe = routeDisplayProbes.get(connection.connectionId);
    return {
      component: `route_display_socket:${connection.connectionId}`,
      ...readiness(
        probe?.ok ? 'ready' : 'failed',
        probe?.evidence || 'route display probe did not run',
        probe?.ok ? 'none' : 'repair_rdp_route_display_session',
        probe?.ok
          ? 'The selected route display has a local X11 socket.'
          : 'Start or repair the RDP route desktop session before exporting this route-pool entry.',
      ),
    };
  }),
];

const result = {
  success: ready,
  status: ready ? 'ready' : 'blocked',
  readiness: {
    status: ready ? 'ready' : 'blocked',
    components,
    nextAction: ready
      ? 'run_many_to_many_live_gate'
      : components.find((component) => component.status !== 'ready')?.nextAction || 'inspect_route_pool',
  },
  guacamole: {
    baseUrl: routeBases.configuredBase,
    localBaseUrl: routeBases.localBase,
    publicBaseUrl: routeBases.publicBase,
    healthBaseUrl: routeBases.healthBase,
    localEmbedReady: localGuacamoleWebProbe.ok,
    publicOperatorReady: publicGuacamoleWebProbe.ok,
    loginReady: guacamoleLoginProbe.ok,
    login: {
      ok: guacamoleLoginProbe.ok,
      authMode: guacamoleLoginProbe.authMode || null,
      statusCode: guacamoleLoginProbe.statusCode,
      authTokenPresent: guacamoleLoginProbe.authTokenPresent,
    },
    connections: redactedConnections,
    schema: {
      ok: schema.ok,
      requiredTables: schema.requiredTables,
      presentTables: schema.presentTables,
      missingTables: schema.missingTables,
    },
    permissions: {
      ok: permissions.ok,
      connectionPermissions: permissions.connectionPermissions,
      missingReadConnectionIds: permissions.missingReadConnectionIds,
    },
    selectedConnectionIds: selectedConnections.map((connection) => connection.connectionId),
    connectionCount: connections.length,
    distinctTargetIdentityCount: distinctTargetIdentities.size,
    distinctSelectedTargetIdentityCount: distinctSelectedTargetIdentities.size,
  },
  routePoolJson,
  env: ready && routePoolJson.length >= 2
    ? {
        AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(routePoolJson),
      }
    : {},
  nextStep: ready
    ? 'Export AGENT_BROWSER_RDP_ROUTE_POOL_JSON and run pnpm test:rdp-guac-many-to-many-live.'
    : components.find((component) => component.status !== 'ready')?.recovery ||
      'Provision two distinct Guacamole RDP route candidates, then rerun pnpm test:rdp-guac-route-pool-readiness.',
};

if (shellOutput && ready && result.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON) {
  console.log(`export AGENT_BROWSER_RDP_ROUTE_POOL_JSON=${shellQuote(result.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON)}`);
} else {
  console.log(JSON.stringify(result, null, 2));
}

if (!ready && !reportOnly) {
  process.exitCode = 1;
}
