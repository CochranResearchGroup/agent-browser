#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const container = `agent-browser-route-pool-fixture-${process.pid}`;
const image = process.env.AGENT_BROWSER_POSTGRES_FIXTURE_IMAGE || 'postgres:16-alpine';
const schema = readFileSync('cli/assets/workstation/guacamole/init/001-initdb.sql', 'utf8');
const routes = [
  {
    id: 'route-user-a',
    connectionName: 'Agent Browser RDP Route A',
    legacyConnectionName: 'Agent Browser RDP Existing User Route A',
    routeUser: 'agent-browser-rdp-a',
    password: 'route-a-secret',
  },
  {
    id: 'route-user-b',
    connectionName: 'Agent Browser RDP Route B',
    legacyConnectionName: 'Agent Browser RDP Existing User Route B',
    routeUser: 'agent-browser-rdp-b',
    password: 'route-b-secret',
  },
];

try {
  const started = docker([
    'run', '--detach', '--rm', '--pull', 'never', '--name', container,
    '--env', 'POSTGRES_PASSWORD=fixture',
    '--env', 'POSTGRES_DB=guacamole_db',
    '--env', 'POSTGRES_USER=guacamole_user',
    image,
  ]);
  assert.equal(started.status, 0, `start disposable PostgreSQL fixture:\n${started.stderr}`);
  waitForPostgres();
  psql(schema);
  psql(`
INSERT INTO guacamole_entity (name, type) VALUES
  ('operator-fixture', 'USER'),
  ('unrelated-user', 'USER');
INSERT INTO guacamole_user (entity_id, password_hash, password_date)
SELECT entity_id, decode('00', 'hex'), now()
FROM guacamole_entity
WHERE type = 'USER'
  AND name IN ('operator-fixture', 'unrelated-user');
INSERT INTO guacamole_connection (connection_name, protocol) VALUES
  ('Agent Browser RDP Existing User Route A', 'rdp'),
  ('Agent Browser RDP Route B', 'rdp'),
  ('Unrelated SSH Provider Route', 'ssh');
INSERT INTO guacamole_connection_parameter (connection_id, parameter_name, parameter_value)
SELECT connection_id, 'fixture-marker', 'unrelated-parameter'
FROM guacamole_connection WHERE connection_name = 'Unrelated SSH Provider Route';
INSERT INTO guacamole_sharing_profile (sharing_profile_name, primary_connection_id)
SELECT 'Unrelated Shared Session', connection_id
FROM guacamole_connection WHERE connection_name = 'Unrelated SSH Provider Route';
INSERT INTO guacamole_connection_permission (entity_id, connection_id, permission)
SELECT entity.entity_id, connection.connection_id, 'READ'
FROM guacamole_entity entity
CROSS JOIN guacamole_connection connection
WHERE entity.name = 'unrelated-user'
  AND connection.connection_name = 'Unrelated SSH Provider Route';
`);

  const unrelatedBefore = snapshotUnrelated();
  const rendered = spawnSync(
    'python3',
    [
      'scripts/lib/rdp-route-user-pool.py', 'sql',
      '--hostname', 'host.docker.internal',
      '--port', '3389',
      '--rebuild-owned',
      '--header-user', 'operator-fixture',
    ],
    { encoding: 'utf8', input: JSON.stringify(routes) },
  );
  assert.equal(rendered.status, 0, rendered.stderr);
  psql(rendered.stdout);

  assert.equal(snapshotUnrelated(), unrelatedBefore, 'unrelated provider rows must survive exactly');
  assert.deepEqual(
    queryRows(`
SELECT connection_name || '|' || protocol
FROM guacamole_connection
WHERE connection_name LIKE 'Agent Browser RDP Route %'
ORDER BY connection_name;
`),
    ['Agent Browser RDP Route A|rdp', 'Agent Browser RDP Route B|rdp'],
    'the owned canonical route namespace must be reconstructed',
  );
  assert.deepEqual(
    queryRows(`SELECT name FROM guacamole_entity WHERE name = 'operator-fixture';`),
    [],
    'the exact header user must be removed for ordinary recreation',
  );
  console.log('RDP route-user PostgreSQL preservation fixture passed.');
} finally {
  docker(['rm', '--force', container]);
}

function snapshotUnrelated() {
  return queryRows(`
SELECT 'connection|' || connection_id || '|' || connection_name || '|' || protocol
FROM guacamole_connection WHERE connection_name = 'Unrelated SSH Provider Route'
UNION ALL
SELECT 'parameter|' || parameter.connection_id || '|' || parameter_name || '|' || parameter_value
FROM guacamole_connection_parameter parameter
JOIN guacamole_connection connection USING (connection_id)
WHERE connection.connection_name = 'Unrelated SSH Provider Route'
UNION ALL
SELECT 'sharing|' || profile.sharing_profile_id || '|' || profile.sharing_profile_name || '|' || profile.primary_connection_id
FROM guacamole_sharing_profile profile
JOIN guacamole_connection connection ON connection.connection_id = profile.primary_connection_id
WHERE connection.connection_name = 'Unrelated SSH Provider Route'
UNION ALL
SELECT 'permission|' || permission.entity_id || '|' || permission.connection_id || '|' || permission.permission
FROM guacamole_connection_permission permission
JOIN guacamole_connection connection USING (connection_id)
WHERE connection.connection_name = 'Unrelated SSH Provider Route'
UNION ALL
SELECT 'entity|' || entity_id || '|' || name || '|' || type
FROM guacamole_entity WHERE name = 'unrelated-user'
ORDER BY 1;
`).join('\n');
}

function waitForPostgres() {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    const logs = docker(['logs', container]);
    const ready = docker(['exec', container, 'pg_isready', '-U', 'guacamole_user', '-d', 'guacamole_db']);
    const readyEvents = `${logs.stdout}${logs.stderr}`.match(/database system is ready to accept connections/g) || [];
    if (ready.status === 0 && readyEvents.length >= 2) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 100);
  }
  assert.fail('disposable PostgreSQL fixture did not become ready');
}

function psql(input) {
  const result = docker(
    ['exec', '--interactive', container, 'psql', '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'guacamole_user', '-d', 'guacamole_db'],
    input,
  );
  assert.equal(result.status, 0, `execute PostgreSQL fixture SQL:\n${result.stdout}${result.stderr}`);
  return result.stdout;
}

function queryRows(sql) {
  const result = docker(
    ['exec', '--interactive', container, 'psql', '-X', '-A', '-t', '-v', 'ON_ERROR_STOP=1', '-U', 'guacamole_user', '-d', 'guacamole_db'],
    sql,
  );
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim() ? result.stdout.trim().split('\n') : [];
}

function docker(args, input = undefined) {
  return spawnSync('docker', args, { encoding: 'utf8', input });
}
