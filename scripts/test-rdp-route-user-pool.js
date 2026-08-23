#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-route-users-'));
const secretFile = join(fixtureRoot, 'guacamole.env');
writeFileSync(secretFile, 'GUACAMOLE_ADMIN_PASSWORD=fixture\n');
const requested = Array.from({ length: 4 }, (_, index) => ({
  id: `route-user-${index + 1}`,
  connectionName: `Agent Browser RDP Route ${index + 1}`,
  routeUser: `agent-browser-rdp-${index + 1}`,
}));
const resolve = spawnSync(
  'python3',
  ['scripts/lib/rdp-route-user-pool.py', 'resolve', '--secret-file', secretFile, '--generate-passwords'],
  {
    encoding: 'utf8',
    env: {
      ...process.env,
      AGENT_BROWSER_RDP_ROUTE_USER_POOL_JSON: JSON.stringify(requested),
    },
  },
);
assert.equal(resolve.status, 0, resolve.stderr);
const resolved = JSON.parse(resolve.stdout);
assert.equal(resolved.length, 4);
assert.equal(new Set(resolved.map((route) => route.routeUser)).size, 4);
assert(resolved.every((route) => typeof route.password === 'string' && route.password.length === 32));
assert.match(readFileSync(secretFile, 'utf8'), /XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON=/);

const sql = spawnSync(
  'python3',
  ['scripts/lib/rdp-route-user-pool.py', 'sql', '--hostname', 'host.docker.internal', '--port', '3389'],
  { encoding: 'utf8', input: JSON.stringify(resolved) },
);
assert.equal(sql.status, 0, sql.stderr);
for (const route of requested) assert(sql.stdout.includes(route.connectionName));
assert.match(sql.stdout, /distinct_username_count <> 4/);

const legacySecret = join(fixtureRoot, 'legacy.env');
writeFileSync(legacySecret, [
  'XRDP_AGENT_BROWSER_ROUTE_A_USERNAME=legacy-a',
  'XRDP_AGENT_BROWSER_ROUTE_A_PASSWORD=password-a',
  'XRDP_AGENT_BROWSER_ROUTE_B_USERNAME=legacy-b',
  'XRDP_AGENT_BROWSER_ROUTE_B_PASSWORD=password-b',
].join('\n'));
const legacy = spawnSync(
  'python3',
  ['scripts/lib/rdp-route-user-pool.py', 'resolve', '--secret-file', legacySecret],
  { encoding: 'utf8', env: { ...process.env, AGENT_BROWSER_RDP_ROUTE_USER_POOL_JSON: '' } },
);
assert.equal(legacy.status, 0, legacy.stderr);
assert.deepEqual(JSON.parse(legacy.stdout).map((route) => route.routeUser), ['legacy-a', 'legacy-b']);

console.log('RDP route-user pool fixtures passed.');
