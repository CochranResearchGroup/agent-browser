#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from 'node:fs';
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
const fakeBrowser = join(fixture, 'chrome');
writeFileSync(fakeBrowser, '#!/bin/sh\nexit 0\n', { mode: 0o755 });
writeFileSync(
  fakeBinary,
  `#!/bin/sh
if [ "\${1:-}" = "--version" ]; then
  echo "agent-browser 0.28.0-fixture"
else
  printf '%s|%s|%s|%s|%s|%s\\n' "$HOME" "$AGENT_BROWSER_RUNTIME_ENVIRONMENT" "$AGENT_BROWSER_RUNTIME_HOST" "$AGENT_BROWSER_SOCKET_DIR" "$AGENT_BROWSER_DASHBOARD_AUTH_DIR" "$AGENT_BROWSER_EXECUTABLE_PATH"
fi
`,
  { mode: 0o755 },
);
const env = {
  ...process.env,
  AGENT_BROWSER_DEV_USER_HOME: join(fixture, 'user'),
  AGENT_BROWSER_DEV_RUNTIME_DIR: join(fixture, 'run', 'agent-browser-dev'),
  AGENT_BROWSER_DEV_BROWSER_EXECUTABLE: fakeBrowser,
  AGENT_BROWSER_DEV_SKIP_SYSTEMD: '1',
};

try {
  const descriptor = developmentRuntimeDescriptor(env);
  assert.equal(descriptor.dashboardPort, 4948);
  assert.equal(descriptor.backendPort, 4949);
  assert.equal(descriptor.laneStreamPort, 4951);
  assert.equal(descriptor.ingressService, 'agent-browser-dev');
  assert.equal(descriptor.presentationProvider.ports.guacamole, 8093);
  assert.equal(descriptor.presentationProvider.warmSlots, 4);
  assert.equal(descriptor.presentationProvider.hardMaxSlots, 6);
  assert.equal(descriptor.browserExecutable, fakeBrowser);
  const units = renderDevelopmentUnits(descriptor, '/candidate/bin/agent-browser');
  for (const source of Object.values(units)) {
    assert.match(source, /AGENT_BROWSER_RUNTIME_ENVIRONMENT=development/);
    assert.match(source, /AGENT_BROWSER_RUNTIME_HOST=1/);
    assert.match(source, /AGENT_BROWSER_SOCKET_DIR=/);
    assert.match(source, new RegExp(`AGENT_BROWSER_EXECUTABLE_PATH=${fakeBrowser}`));
    assert.match(source, /AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH=/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_WARM_MINIMUM=4/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_HARD_MAXIMUM=6/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_HUMAN_RESERVE=1/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_RECOVERY_RESERVE=1/);
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
    `${descriptor.pseudoHome}|development|1|${descriptor.socketDir}|${descriptor.authDir}|${fakeBrowser}`,
  );
  const diagnosticBrowser = join(fixture, 'diagnostic-chrome');
  const overriddenLauncherEnvironment = execFileSync(descriptor.executable, ['print-env'], {
    encoding: 'utf8',
    env: { ...process.env, AGENT_BROWSER_EXECUTABLE_PATH: diagnosticBrowser },
  }).trim();
  assert.equal(
    overriddenLauncherEnvironment,
    `${descriptor.pseudoHome}|development|1|${descriptor.socketDir}|${descriptor.authDir}|${diagnosticBrowser}`,
  );
  assert.throws(
    () => installDevelopmentRuntime({
      binary: fakeBinary,
      env: { ...env, AGENT_BROWSER_DEV_BROWSER_EXECUTABLE: join(fixture, 'missing-chrome') },
      activate: false,
    }),
    /Development browser executable/,
  );
  const fakeWindowsBrowser = join(fixture, 'chrome.exe');
  writeFileSync(fakeWindowsBrowser, '#!/bin/sh\nexit 0\n', { mode: 0o755 });
  assert.throws(
    () => developmentRuntimeDescriptor({
      ...env,
      AGENT_BROWSER_DEV_BROWSER_EXECUTABLE: fakeWindowsBrowser,
    }),
    /incompatible with the Linux profile root/,
  );
  const laneManifest = JSON.parse(readFileSync(descriptor.laneManifest, 'utf8'));
  assert.equal(laneManifest.session, 'development-default');
  assert.equal(laneManifest.streamPort, 4951);
  assert.equal(laneManifest.executablePath, installed.generation.binary);
  assert.match(readFileSync(join(descriptor.systemdDir, descriptor.units[0]), 'utf8'), new RegExp(installed.generation.binary));

  const laneManifestBeforeRejectedInstall = readFileSync(descriptor.laneManifest, 'utf8');
  const currentBeforeRejectedInstall = readlinkSync(descriptor.current);
  assert.throws(
    () => installDevelopmentRuntime({
      binary: fakeBinary,
      env,
      activate: false,
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      verifyProduction: () => { throw new Error('fixture production drift'); },
    }),
    /fixture production drift/,
  );
  assert.equal(readFileSync(descriptor.laneManifest, 'utf8'), laneManifestBeforeRejectedInstall);
  assert.equal(readlinkSync(descriptor.current), currentBeforeRejectedInstall);

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
  productionBefore.processes.push({
    pid: 11,
    startToken: '101',
    executable: '/production/generation-a/bin/agent-browser',
    stable: false,
  });
  assert.doesNotThrow(() => assertProductionUnchanged(productionBefore, productionAfter));
  productionBefore.processes.push({
    pid: 12,
    startToken: '102',
    executable: '/production/generation-a/bin/agent-browser',
    stable: true,
  });
  assert.throws(
    () => assertProductionUnchanged(productionBefore, productionAfter),
    /stable process changed.*12/,
  );
  productionBefore.processes.pop();
  productionAfter.serviceIdentities.browsers[0].pid = 21;
  assert.throws(() => assertProductionUnchanged(productionBefore, productionAfter), /browsers identity changed/);
  const developmentHelp = execFileSync('node', ['scripts/development-runtime.js', 'help'], {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
  assert.match(developmentHelp, /provider-scale-out --apply/);
  assert.match(developmentHelp, /provider-scale-in --apply/);
  console.log('Development runtime fixture passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
