#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  canonicalRouteInventory,
  legacyTwoRouteDisplaySubjects,
  legacyTwoRouteInventory,
  selectManagedRouteCandidates,
} from './lib/rdp-route-inventory.js';

const six = canonicalRouteInventory(
  Array.from({ length: 6 }, (_, index) => ({
    id: `slot-route-${index}`,
    routeId: `guacamole:${100 + index}`,
    connectionId: String(100 + index),
    connectionName: `Agent Browser RDP Route ${index + 1}`,
    target: {
      displayName: `:${20 + index}`,
      routeUser: `agent-browser-rdp-${index + 1}`,
    },
  })),
);
assert.equal(six.length, 6);
assert.deepEqual(six.map((route) => route.id), [
  'slot-route-0',
  'slot-route-1',
  'slot-route-2',
  'slot-route-3',
  'slot-route-4',
  'slot-route-5',
]);
assert.equal(six[5].target.displayName, ':25');
assert.equal(six[5].target.routeUser, 'agent-browser-rdp-6');

const legacy = legacyTwoRouteInventory({
  AGENT_BROWSER_RDP_ROUTE_A_ID: 'guacamole:1',
  AGENT_BROWSER_RDP_ROUTE_A_FRAME_URL: 'http://localhost/a',
  AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME: ':10',
  AGENT_BROWSER_RDP_ROUTE_B_ID: 'guacamole:2',
  AGENT_BROWSER_RDP_ROUTE_B_FRAME_URL: 'http://localhost/b',
  AGENT_BROWSER_RDP_ROUTE_B_DISPLAY_NAME: ':11',
});
assert.deepEqual(legacy.map((route) => route.id), ['pool-a', 'pool-b']);
assert.deepEqual(legacy.map((route) => route.target.displayName), [':10', ':11']);

assert.deepEqual(
  legacyTwoRouteDisplaySubjects({}).map((route) => route.target.routeUser),
  ['agent-browser-rdp-a', 'agent-browser-rdp-b'],
);

const candidates = selectManagedRouteCandidates([
  { connectionId: '8', connectionName: 'Unmanaged Desktop' },
  { connectionId: '3', connectionName: 'Agent Browser RDP Route 3' },
  { connectionId: '1', connectionName: 'Agent Browser RDP Route A' },
  { connectionId: '2', connectionName: 'Agent Browser RDP Route B' },
  { connectionId: '4', connectionName: 'Agent Browser RDP Route 4' },
]);
assert.deepEqual(candidates.map((route) => route.connectionId), ['1', '2', '3', '4']);

assert.throws(
  () => canonicalRouteInventory([
    { id: 'duplicate', routeId: 'route-1' },
    { id: 'duplicate', routeId: 'route-2' },
  ]),
  /route_inventory_duplicate_id/,
);

const fixtureBin = mkdtempSync(join(tmpdir(), 'agent-browser-route-inventory-'));
const psFixture = join(fixtureBin, 'ps');
writeFileSync(
  psFixture,
  `#!/usr/bin/env bash
printf '%s\n' \
  'agent-browser-rdp-1 101 Xorg Xorg :21' \
  'agent-browser-rdp-2 102 Xorg Xorg :22' \
  'agent-browser-rdp-3 103 Xorg Xorg :23' \
  'agent-browser-rdp-4 104 Xorg Xorg :24' \
  'agent-browser-rdp-5 105 Xorg Xorg :25' \
  'agent-browser-rdp-6 106 Xorg Xorg :26'
`,
);
chmodSync(psFixture, 0o755);
const inspection = spawnSync(process.execPath, ['scripts/inspect-rdp-route-displays.js'], {
  cwd: new URL('..', import.meta.url),
  encoding: 'utf8',
  env: {
    ...process.env,
    PATH: `${fixtureBin}:${process.env.PATH || ''}`,
    AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(six),
  },
});
assert.equal(inspection.status, 0, inspection.stderr || inspection.stdout);
const inspectionPayload = JSON.parse(inspection.stdout);
assert.equal(inspectionPayload.routeInventory.length, 6);
assert.equal(
  JSON.parse(inspectionPayload.env.AGENT_BROWSER_RDP_ROUTE_POOL_JSON).length,
  6,
);

console.log('RDP route inventory fixtures passed.');
