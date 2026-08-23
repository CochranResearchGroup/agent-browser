#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  assertProductionUnchanged,
  developmentRuntimeDescriptor,
  garbageCollectDevelopmentRuntime,
  installDevelopmentRuntime,
  renderDevelopmentUnits,
} from './lib/development-runtime.js';

const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-development-runtime-'));
const fakeBinary = join(fixture, 'agent-browser');
writeFileSync(
  fakeBinary,
  `#!/bin/sh
if [ "\${1:-}" = "--version" ]; then
  echo "agent-browser 0.28.0-fixture"
else
  printf '%s|%s|%s|%s|%s\\n' "$HOME" "$AGENT_BROWSER_RUNTIME_ENVIRONMENT" "$AGENT_BROWSER_RUNTIME_HOST" "$AGENT_BROWSER_SOCKET_DIR" "$AGENT_BROWSER_DASHBOARD_AUTH_DIR"
fi
`,
  { mode: 0o755 },
);
const env = {
  ...process.env,
  AGENT_BROWSER_DEV_USER_HOME: join(fixture, 'user'),
  AGENT_BROWSER_DEV_RUNTIME_DIR: join(fixture, 'run', 'agent-browser-dev'),
  AGENT_BROWSER_DEV_SKIP_SYSTEMD: '1',
};

try {
  const descriptor = developmentRuntimeDescriptor(env);
  assert.equal(descriptor.dashboardPort, 4948);
  assert.equal(descriptor.backendPort, 4949);
  assert.equal(descriptor.laneStreamPort, 4951);
  assert.equal(descriptor.ingressService, 'agent-browser-dev');
  const units = renderDevelopmentUnits(descriptor, '/candidate/bin/agent-browser');
  for (const source of Object.values(units)) {
    assert.match(source, /AGENT_BROWSER_RUNTIME_ENVIRONMENT=development/);
    assert.match(source, /AGENT_BROWSER_RUNTIME_HOST=1/);
    assert.match(source, /AGENT_BROWSER_SOCKET_DIR=/);
    assert.doesNotMatch(source, /\.local\/bin\/agent-browser\n/);
  }
  assert.match(units['agent-browser-dev-dashboard.service'], /AGENT_BROWSER_DASHBOARD_PORT=4948/);
  assert.doesNotMatch(JSON.stringify(units), /4848|4849|agent-browser-dashboard\.service/);

  const installed = installDevelopmentRuntime({ binary: fakeBinary, env, activate: false });
  assert.equal(installed.success, true);
  assert.equal(installed.production.unchanged, true);
  assert.equal(installed.generation.version, '0.28.0-fixture');
  assert.equal(readFileSync(installed.generation.binary, 'utf8'), readFileSync(fakeBinary, 'utf8'));
  const launcherEnvironment = execFileSync(descriptor.executable, ['print-env'], {
    encoding: 'utf8',
  }).trim();
  assert.equal(
    launcherEnvironment,
    `${descriptor.pseudoHome}|development|1|${descriptor.socketDir}|${descriptor.authDir}`,
  );
  const laneManifest = JSON.parse(readFileSync(descriptor.laneManifest, 'utf8'));
  assert.equal(laneManifest.session, 'development-default');
  assert.equal(laneManifest.streamPort, 4951);
  assert.equal(laneManifest.executablePath, installed.generation.binary);
  assert.match(readFileSync(join(descriptor.systemdDir, descriptor.units[0]), 'utf8'), new RegExp(installed.generation.binary));

  const obsoleteGeneration = join(descriptor.generations, '0.27.0-obsolete');
  mkdirSync(obsoleteGeneration, { recursive: true });
  utimesSync(obsoleteGeneration, new Date(0), new Date(0));
  const gc = garbageCollectDevelopmentRuntime({ env, retain: 1 });
  assert.deepEqual(gc.removed, [obsoleteGeneration]);

  const manifestSource = readFileSync('cli/src/native/stream/http.rs', 'utf8');
  const pageSource = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');
  const shellSource = readFileSync('packages/dashboard/src/components/app-shell.tsx', 'utf8');
  assert.match(manifestSource, /"runtimeEnvironment": runtime_environment_label/);
  assert.match(pageSource, /runtimeEnvironment: runtimeManifest\.manifest\?\.runtimeEnvironment/);
  assert.match(shellSource, /data-runtime-environment=\{runtimeEnvironment\}/);
  assert.match(shellSource, /"Development" : "Service Lab"/);
  const productionBefore = {
    selectedGeneration: '/production/generation-a',
    processes: [{ pid: 10, startToken: '100', executable: '/production/generation-a/bin/agent-browser' }],
    dashboardManifest: { executable: { sha256: 'a' } },
    stateFiles: {
      serviceState: { sha256: 'mutable-a' },
      remoteViewHandoffs: { sha256: 'handoff-a' },
    },
    serviceIdentities: {
      browsers: [{ key: 'browser-a', id: 'browser-a', pid: 20 }],
      sessions: [{ key: 'session-a', id: 'session-a' }],
    },
    units: { dashboard: { mainPid: 10 } },
  };
  const productionAfter = structuredClone(productionBefore);
  productionAfter.stateFiles.serviceState.sha256 = 'mutable-b';
  productionAfter.serviceIdentities.sessions.push({ key: 'session-b', id: 'session-b' });
  assert.doesNotThrow(() => assertProductionUnchanged(productionBefore, productionAfter));
  productionAfter.serviceIdentities.browsers[0].pid = 21;
  assert.throws(() => assertProductionUnchanged(productionBefore, productionAfter), /browsers identity changed/);
  execFileSync('node', ['scripts/development-runtime.js', 'help'], { cwd: process.cwd() });
  console.log('Development runtime fixture passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
