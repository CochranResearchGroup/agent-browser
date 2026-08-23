#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

function canonicalTwoSlotFindings(relative, source) {
  const findings = [];
  for (const match of source.matchAll(/\.slice\(0,\s*2\)/g)) {
    findings.push(`${relative}:fixed_two_entry_truncation:${match.index}`);
  }
  for (const match of source.matchAll(/AGENT_BROWSER_RDP_ROUTE_(?:A|B)_[A-Z0-9_]*/g)) {
    findings.push(`${relative}:alphabetic_route_configuration:${match[0]}`);
  }
  return findings;
}

assert.deepEqual(
  canonicalTwoSlotFindings('fixture.js', 'routes.slice(0, 2)'),
  ['fixture.js:fixed_two_entry_truncation:6'],
  'the guard must detect fixed two-entry inventory truncation',
);
assert.equal(
  canonicalTwoSlotFindings(
    'fixture.rs',
    'AGENT_BROWSER_RDP_ROUTE_A_DISPLAY_NAME',
  ).length,
  1,
  'the guard must detect canonical alphabetic route configuration',
);

const guardedFiles = [
  'cli/src/native/remote_view.rs',
  'cli/src/remote_view_doctor.rs',
  'cli/src/workstation_install.rs',
  'scripts/grant-rdp-route-display-access.sh',
  'scripts/inspect-rdp-route-displays.js',
  'scripts/open-rdp-guac-route-displays.js',
  'scripts/setup-rdp-guac-route-pool.sh',
  'scripts/smoke-rdp-guac-route-pool-readiness.js',
  'scripts/sync-rdp-guac-route-specific-user-pool.sh',
  'scripts/test-rdp-guac-many-to-many-live.js',
];

const failures = [];
for (const relative of guardedFiles) {
  const source = readFileSync(path.join(repoRoot, relative), 'utf8');
  const findings = canonicalTwoSlotFindings(relative, source);
  if (findings.length !== 0) {
    failures.push(`${relative}:canonical_two_slot_findings=${findings.length}`);
  }
}

if (failures.length > 0) {
  console.error('Presentation capacity architecture guard failed:');
  failures.forEach((failure) => console.error(`- ${failure}`));
  process.exit(1);
}

console.log('Presentation capacity architecture guard passed.');
