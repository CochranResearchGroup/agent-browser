#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const reportOnly = process.argv.includes('--report-only');
const allowSharedTarget = process.argv.includes('--allow-shared-target');
const shellOutput = process.argv.includes('--shell');

loadAgentBrowserEnv();

function loadAgentBrowserEnv() {
  const agentHome = process.env.AGENT_BROWSER_HOME || join(process.env.HOME || '', '.agent-browser');
  const envPath = join(agentHome, '.env');
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

function commandResult(command, args) {
  return spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
  });
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

function guacamoleBaseUrl() {
  const configured = process.env.AGENT_BROWSER_REMOTE_VIEW_EXTERNAL_URL ||
    process.env.AGENT_BROWSER_REMOTE_VIEW_FRAME_URL ||
    process.env.AGENT_BROWSER_REMOTE_VIEW_URL ||
    'http://127.0.0.1:8092/guacamole/';
  const hashIndex = configured.indexOf('#');
  const withoutHash = hashIndex >= 0 ? configured.slice(0, hashIndex) : configured;
  return withoutHash.endsWith('/') ? withoutHash : `${withoutHash}/`;
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

function routeTargetDisplayName(index) {
  const label = index === 0 ? 'A' : 'B';
  return process.env[`AGENT_BROWSER_RDP_ROUTE_${label}_DISPLAY_NAME`] || null;
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

function routePoolEntry(connection, index, baseUrl, routeReadiness) {
  const label = index === 0 ? 'a' : 'b';
  const displayName = routeTargetDisplayName(index);
  const frameUrl = `${baseUrl}#/client/${guacamoleClientId(connection.connectionId)}`;
  return {
    id: `guacamole-rdp-${label}`,
    routeId: `guacamole:${connection.connectionId}`,
    connectionId: connection.connectionId,
    connectionName: connection.connectionName,
    frameUrl,
    externalUrl: frameUrl,
    providerMode: 'simultaneous_view',
    target: {
      hostname: connection.hostname || null,
      port: connection.port || '3389',
      colorDepth: connection.colorDepth || null,
      targetIdentityKey: targetIdentityKey(connection),
      ...(displayName ? { displayName } : {}),
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
const baseUrl = guacamoleBaseUrl();
const guacamoleWebProbe = guacamoleReady ? curlReachable(baseUrl) : {
  ok: false,
  statusCode: null,
  error: 'agent-browser-guacamole is not running',
};
const query = dockerReady ? queryGuacamoleConnections() : {
  ok: false,
  error: 'agent-browser-guacamole-postgres is not running',
  connections: [],
};
const connections = query.connections || [];
const selectedConnections = routePoolCandidates(connections);
const rdpTcpProbes = guacdReady
  ? new Map(selectedConnections.map((connection) => [connection.connectionId, rdpTcpReachable(connection)]))
  : new Map(selectedConnections.map((connection) => [
      connection.connectionId,
      {
        ok: false,
        evidence: 'agent-browser-guacd is not running',
      },
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
const ready = Boolean(
  dockerReady &&
    guacamoleReady &&
    guacdReady &&
    guacamoleWebProbe.ok &&
    query.ok &&
    hasTwoSelectedConnections &&
    selectedRdpTargetsReady &&
    (hasTwoDistinctTargets || allowSharedTarget),
);
const routePoolJson = ready || hasTwoSelectedConnections
  ? selectedConnections.map((connection, index) => {
      const rdpProbe = rdpTcpProbes.get(connection.connectionId);
      const routeReady = guacamoleWebProbe.ok && rdpProbe?.ok === true;
      return routePoolEntry(connection, index, baseUrl, {
        state: routeReady ? 'ready' : 'failed',
        components: [
          {
            component: 'guacamole_web_route',
            status: guacamoleWebProbe.ok ? 'ready' : 'failed',
            evidence: guacamoleWebProbe.ok
              ? `${baseUrl} returned HTTP ${guacamoleWebProbe.statusCode}`
              : guacamoleWebProbe.error,
          },
          {
            component: 'rdp_backend_tcp',
            status: rdpProbe?.ok ? 'ready' : 'failed',
            evidence: rdpProbe?.evidence || 'RDP TCP probe did not run',
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
    component: 'guacamole_web',
    ...readiness(
      guacamoleReady && guacamoleWebProbe.ok ? 'ready' : 'failed',
      guacamoleReady && guacamoleWebProbe.ok
        ? `agent-browser-guacamole is running and ${baseUrl} returned HTTP ${guacamoleWebProbe.statusCode}`
        : guacamoleWebProbe.error || 'agent-browser-guacamole is not reachable',
      guacamoleReady && guacamoleWebProbe.ok ? 'none' : 'repair_guacamole_web_route',
      guacamoleReady && guacamoleWebProbe.ok
        ? 'Guacamole web container and ingress route are available.'
        : 'Start or repair the Guacamole web container and ingress route before validating route-pool entries.',
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
    baseUrl,
    connections: redactedConnections,
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
    : 'Provision two distinct Guacamole RDP route candidates, then rerun pnpm test:rdp-guac-route-pool-readiness.',
};

if (shellOutput && ready && result.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON) {
  console.log(`export AGENT_BROWSER_RDP_ROUTE_POOL_JSON=${shellQuote(result.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON)}`);
} else {
  console.log(JSON.stringify(result, null, 2));
}

if (!ready && !reportOnly) {
  process.exitCode = 1;
}
