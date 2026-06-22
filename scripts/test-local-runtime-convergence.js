#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const script = readFileSync('scripts/converge-local-runtime.js', 'utf8');
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'));
const plan = readFileSync('docs/dev/plans/0042-2026-06-22-runtime-convergence-plan.md', 'utf8');

assert.equal(
  packageJson.scripts['converge:local-runtime'],
  'node scripts/converge-local-runtime.js',
  'package script must expose the local convergence command',
);

assert.match(
  script,
  /const options = \{[\s\S]*apply: false,[\s\S]*json: false,[\s\S]*evidencePath: '',[\s\S]*\};/,
  'local convergence command must dry-run by default',
);

assert.match(
  script,
  /if \(arg === '--'\)[\s\S]*continue;/,
  'local convergence command must tolerate pnpm argument separator forwarding',
);

assert.match(
  script,
  /function isSafeStaleDaemonRemedy\(argv\)[\s\S]*argv\[0\] === 'agent-browser'[\s\S]*argv\[1\] === 'close'[\s\S]*argv\[2\] === '--session'/,
  'apply mode must only run agent-browser close --session stale-daemon remedies',
);

assert.match(
  script,
  /publish:local-dashboard[\s\S]*--skip-browser[\s\S]*staleDaemonRemedies\(afterPublish\.install\)[\s\S]*ensure:rdp-guac-postgres[\s\S]*test:rdp-guac-route-pool-readiness/,
  'apply mode must sequence local publish, stale daemon remedies, Guacamole schema ensure, and route-pool readiness',
);

assert.match(
  script,
  /afterRoutePool\.remoteView\.nextAction === 'grant_route_display_access'[\s\S]*grant:rdp-route-display-access/,
  'apply mode must run display-access grant only when remote-view doctor requests it',
);

assert.match(
  script,
  /function writeEvidence\(payload\)[\s\S]*\.agent-browser\/convergence\/local-runtime-latest\.json[\s\S]*writeFileSync/,
  'apply mode must retain convergence evidence in a runtime-local JSON file',
);

assert.doesNotMatch(
  script,
  /killall|pkill|rm -rf|docker compose down|TerminateProcess\(/,
  'local convergence command must not contain broad destructive process or filesystem operations',
);

assert.match(
  plan,
  /### Slice F: One-Command Local Convergence[\s\S]*`pnpm --silent converge:local-runtime -- --json`[\s\S]*`pnpm --silent converge:local-runtime -- --apply --json`[\s\S]*`~\/\.agent-browser\/convergence\/local-runtime-latest\.json`/,
  'P42 must document the local runtime convergence command, apply mode, and retained evidence path',
);

console.log('Local runtime convergence command contract smoke passed');
