#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  assertProductionUnchanged,
  assertDefaultDevelopmentUnchanged,
  developmentCandidateBinary,
  developmentRuntimeDescriptor,
  defaultDevelopmentSnapshot,
  developmentExternalDiscoveryChecks,
  observeDevelopmentExternalDiscovery,
  evaluateProtectedLeaseAuthorityStatus,
  garbageCollectDevelopmentRuntime,
  installDevelopmentRuntime,
  publishDevelopmentRuntimeIngress,
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
  printf '%s|%s|%s|%s|%s|%s|%s|%s\\n' "$HOME" "$AGENT_BROWSER_RUNTIME_ENVIRONMENT" "$AGENT_BROWSER_RUNTIME_HOST" "$AGENT_BROWSER_SOCKET_DIR" "$AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE" "$AGENT_BROWSER_DASHBOARD_AUTH_DIR" "$AGENT_BROWSER_EXECUTABLE_PATH" "$AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY"
fi
`,
  { mode: 0o755 },
);
const env = {
  ...process.env,
  AGENT_BROWSER_DEV_USER_HOME: join(fixture, 'user'),
  AGENT_BROWSER_DEV_RUNTIME_DIR: join(fixture, 'run', 'agent-browser-dev'),
  AGENT_BROWSER_DEV_BROWSER_EXECUTABLE: fakeBrowser,
  AGENT_BROWSER_DEV_OPERATOR_USER: 'fixture-provider-operator',
  AGENT_BROWSER_DEV_SKIP_SYSTEMD: '1',
};

try {
  assert.equal(
    developmentCandidateBinary('/repo'),
    '/repo/cli/target/ci/agent-browser',
    'development publication must default to the optimized CI-profile artifact',
  );
  assert.deepEqual(
    evaluateProtectedLeaseAuthorityStatus({
      unit: {
        loadState: 'loaded',
        activeState: 'active',
        unitFileState: 'enabled',
      },
      socket: {
        exists: true,
        socket: true,
        uid: 0,
        gid: 1005,
        mode: 0o660,
      },
      operatorGroupId: 1005,
    }),
    {
      ready: true,
      reasons: [],
    },
  );
  assert.deepEqual(
    evaluateProtectedLeaseAuthorityStatus({
      unit: {
        loadState: 'not-found',
        activeState: 'inactive',
        unitFileState: 'disabled',
      },
      socket: {
        exists: false,
        socket: false,
        uid: null,
        gid: null,
        mode: null,
      },
      operatorGroupId: 1005,
    }),
    {
      ready: false,
      reasons: [
        'socket_unit_not_loaded',
        'socket_unit_not_active',
        'socket_unit_not_enabled',
        'socket_path_missing',
        'socket_owner_not_root',
        'socket_group_mismatch',
        'socket_mode_mismatch',
      ],
    },
  );
  assert.deepEqual(
    evaluateProtectedLeaseAuthorityStatus({
      unit: {
        loadState: 'loaded',
        activeState: 'active',
        unitFileState: 'enabled',
      },
      socket: {
        exists: true,
        socket: false,
        uid: 0,
        gid: 1005,
        mode: 0o660,
      },
      operatorGroupId: 1005,
    }).reasons,
    ['socket_path_not_unix_socket'],
  );
  const descriptor = developmentRuntimeDescriptor(env);
  const namespacedEnv = {
    ...env,
    AGENT_BROWSER_DEV_NAMESPACE: 'p158',
    AGENT_BROWSER_DEV_RUNTIME_DIR: undefined,
    XDG_RUNTIME_DIR: join(fixture, 'parallel-run'),
    AGENT_BROWSER_DEV_DASHBOARD_PORT: '5948',
    AGENT_BROWSER_DEV_BACKEND_PORT: '5949',
    AGENT_BROWSER_DEV_SHADOW_PORT: '5950',
    AGENT_BROWSER_DEV_LANE_STREAM_PORT: '5951',
    AGENT_BROWSER_DEV_GUACAMOLE_PORT: '9093',
    AGENT_BROWSER_DEV_GUACD_PORT: '5823',
    AGENT_BROWSER_DEV_POSTGRES_PORT: '55434',
  };
  for (const namespace of ['', '../escape', 'with-dash', 'toolongname', '1number', 'UPPER']) {
    assert.throws(() => developmentRuntimeDescriptor({ ...namespacedEnv, AGENT_BROWSER_DEV_NAMESPACE: namespace }),
      /AGENT_BROWSER_DEV_NAMESPACE/);
  }
  for (const changed of [
    { AGENT_BROWSER_DEV_DASHBOARD_PORT: undefined },
    { AGENT_BROWSER_DEV_DASHBOARD_PORT: '4948' },
    { AGENT_BROWSER_DEV_BACKEND_PORT: '5948' },
    { AGENT_BROWSER_DEV_SHADOW_PORT: 'not-a-port' },
  ]) {
    assert.throws(() => developmentRuntimeDescriptor({ ...namespacedEnv, ...changed }), /unique port/);
  }
  const parallelDescriptor = developmentRuntimeDescriptor(namespacedEnv);
  assert.equal(parallelDescriptor.namespace, 'p158');
  assert.equal(parallelDescriptor.laneSession, 'development-default-p158');
  for (const key of ['executable', 'installRoot', 'pseudoHome', 'stateDir', 'authDir', 'socketDir',
    'laneManifest', 'runtimeHostIngressState', 'localHost', 'ingressService']) {
    assert.notEqual(parallelDescriptor[key], descriptor[key], `namespace isolates ${key}`);
  }
  assert(parallelDescriptor.units.every((name) => !descriptor.units.includes(name)));
  const parallelUnits = renderDevelopmentUnits(parallelDescriptor, '/candidate/bin/agent-browser');
  assert.deepEqual(Object.keys(parallelUnits), parallelDescriptor.units);
  for (const source of Object.values(parallelUnits)) {
    assert.match(source, /Environment=AGENT_BROWSER_DEV_NAMESPACE=p158/);
    assert.doesNotMatch(source, /agent-browser-dev-(?:runtime-host|dashboard-backend|dashboard)\.service/);
  }
  for (const changed of [
    { AGENT_BROWSER_DEV_INSTALL_ROOT: descriptor.installRoot },
    { AGENT_BROWSER_DEV_HOME: descriptor.pseudoHome },
    { AGENT_BROWSER_DEV_BIN: descriptor.executable },
    { AGENT_BROWSER_DEV_RUNTIME_DIR: join(namespacedEnv.XDG_RUNTIME_DIR, 'agent-browser-dev') },
  ]) {
    assert.throws(() => developmentRuntimeDescriptor({ ...namespacedEnv, ...changed }), /overlaps/);
  }
  assert.equal(descriptor.dashboardPort, 4948);
  assert.equal(descriptor.backendPort, 4949);
  assert.equal(descriptor.laneStreamPort, 4951);
  assert.equal(descriptor.ingressService, 'agent-browser-dev');
  assert.equal(descriptor.presentationProvider.ports.guacamole, 8093);
  assert.equal(descriptor.presentationProvider.warmSlots, 4);
  assert.equal(descriptor.presentationProvider.hardMaxSlots, 6);
  assert.equal(descriptor.guacamoleHeaderUser, 'fixture-provider-operator');
  assert.equal(descriptor.browserExecutable, fakeBrowser);
  assert.equal(descriptor.externalBrowserDiscovery, 'disabled');
  assert.equal(developmentRuntimeDescriptor({ ...env, AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'enabled' })
    .externalBrowserDiscovery, 'disabled');
  const units = renderDevelopmentUnits(descriptor, '/candidate/bin/agent-browser');
  for (const source of Object.values(units)) {
    assert.match(source, /AGENT_BROWSER_RUNTIME_ENVIRONMENT=development/);
    assert.match(source, /^Environment=AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY=disabled$/m);
    assert.match(source, /AGENT_BROWSER_RUNTIME_HOST=1/);
    assert.match(source, /AGENT_BROWSER_SOCKET_DIR=/);
    assert.match(source, /AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE=/);
    assert.match(source, new RegExp(`AGENT_BROWSER_EXECUTABLE_PATH=${fakeBrowser}`));
    assert.match(source, /AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH=/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_WARM_MINIMUM=4/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_HARD_MAXIMUM=6/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_HUMAN_RESERVE=1/);
    assert.match(source, /AGENT_BROWSER_PRESENTATION_RECOVERY_RESERVE=1/);
    assert.match(source, /AGENT_BROWSER_GUACAMOLE_HEADER_USER=fixture-provider-operator/);
    assert.doesNotMatch(source, /\.local\/bin\/agent-browser\n/);
  }
  assert.match(units['agent-browser-dev-dashboard.service'], /AGENT_BROWSER_DASHBOARD_PORT=4948/);
  assert.doesNotMatch(JSON.stringify(units), /4848|4849|agent-browser-dashboard\.service/);

  const installed = installDevelopmentRuntime({ binary: fakeBinary, env, activate: false });
  assert.equal(installed.success, true);
  assert.equal(installed.production.unchanged, true);
  const defaultBefore = defaultDevelopmentSnapshot(namespacedEnv);
  const parallelInstalled = installDevelopmentRuntime({ binary: fakeBinary, env: namespacedEnv, activate: false });
  assert.equal(parallelInstalled.defaultDevelopment.unchanged, true);
  assert.deepEqual(defaultDevelopmentSnapshot(namespacedEnv), defaultBefore);
  assert.notEqual(parallelInstalled.generation.path, installed.generation.path);
  assert.equal(JSON.parse(readFileSync(join(parallelInstalled.generation.path, 'generation.json'), 'utf8')).namespace, 'p158');
  assert.throws(() => assertDefaultDevelopmentUnchanged(defaultBefore,
    { ...defaultBefore, selectedGeneration: '/different-generation' }), /custody changed/);
  const collisionLink = join(fixture, 'default-home-alias');
  symlinkSync(descriptor.pseudoHome, collisionLink);
  assert.throws(() => developmentRuntimeDescriptor({ ...namespacedEnv, AGENT_BROWSER_DEV_HOME: collisionLink }), /overlaps/);
  const unknownGeneration = join(parallelDescriptor.generations, 'unknown-generation');
  const foreignGeneration = join(parallelDescriptor.generations, 'foreign-generation');
  const ownedGeneration = join(parallelDescriptor.generations, 'old-owned-generation');
  for (const path of [unknownGeneration, foreignGeneration, ownedGeneration]) mkdirSync(path);
  writeFileSync(join(foreignGeneration, 'generation.json'), JSON.stringify({ namespace: 'other' }));
  writeFileSync(join(ownedGeneration, 'generation.json'), JSON.stringify({ namespace: 'p158' }));
  const namespaceGc = garbageCollectDevelopmentRuntime({ env: namespacedEnv, retain: 0 });
  assert.deepEqual(namespaceGc.removed, [ownedGeneration]);
  assert(namespaceGc.retained.includes(unknownGeneration));
  assert(namespaceGc.retained.includes(foreignGeneration));
  assert(namespaceGc.retained.includes(parallelInstalled.generation.path));
  assert.deepEqual(defaultDevelopmentSnapshot(namespacedEnv), defaultBefore);
  assert.equal(installed.generation.version, '0.28.0-fixture');
  assert.equal(readFileSync(installed.generation.binary, 'utf8'), readFileSync(fakeBinary, 'utf8'));
  const generationManifest = JSON.parse(
    readFileSync(join(installed.generation.path, 'generation.json'), 'utf8'),
  );
  assert.equal(generationManifest.externalBrowserDiscovery, 'disabled');
  assert.equal(installed.status.externalBrowserDiscovery, 'disabled');
  assert.equal(installed.status.generationMetadata.externalBrowserDiscovery, 'disabled');
  assert.deepEqual(generationManifest.desktopInputProvider, {
    enabled: true,
    providerId: 'controlled-x11-xtest',
    capability: 'guarded_pointer_keyboard_v1',
    recipeId: 'p131-controlled-x11-v1',
  });
  const launcherEnvironment = execFileSync(descriptor.executable, ['print-env'], {
    encoding: 'utf8',
  }).trim();
  assert.equal(
    launcherEnvironment,
    `${descriptor.pseudoHome}|development|1|${descriptor.socketDir}|${descriptor.runtimeHostIngressState}|${descriptor.authDir}|${fakeBrowser}|disabled`,
  );
  const diagnosticBrowser = join(fixture, 'diagnostic-chrome');
  const overriddenLauncherEnvironment = execFileSync(descriptor.executable, ['print-env'], {
    encoding: 'utf8',
    env: { ...process.env, AGENT_BROWSER_EXECUTABLE_PATH: diagnosticBrowser },
  }).trim();
  assert.equal(
    overriddenLauncherEnvironment,
    `${descriptor.pseudoHome}|development|1|${descriptor.socketDir}|${descriptor.runtimeHostIngressState}|${descriptor.authDir}|${diagnosticBrowser}|disabled`,
  );
  for (const inherited of ['enabled', 'invalid-value']) {
    const actual = execFileSync(descriptor.executable, ['print-env'], {
      encoding: 'utf8', env: { ...env, AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: inherited },
    }).trim();
    assert.equal(actual.split('|').at(-1), 'disabled', 'caller cannot enable development host discovery');
  }
  assert.deepEqual(observeDevelopmentExternalDiscovery(null), { state: 'unavailable', policy: null });
  for (const [value, state, policy, accepted] of [
    ['disabled', 'observed', 'disabled', true],
    ['enabled', 'observed', 'enabled', false],
    [undefined, 'missing', null, false],
    ['invalid-private-value', 'invalid', null, false],
  ]) {
    const childEnv = { ...env };
    if (value === undefined) delete childEnv.AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY;
    else childEnv.AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY = value;
    const source = `import {observeDevelopmentExternalDiscovery} from ${JSON.stringify(
      new URL('./lib/development-runtime.js', import.meta.url).href)};
      console.log(JSON.stringify(observeDevelopmentExternalDiscovery(process.pid)));`;
    const observed = JSON.parse(execFileSync(process.execPath, ['--input-type=module', '-e', source], {
      env: childEnv, encoding: 'utf8',
    }));
    assert.deepEqual(observed, { state, policy });
    assert.equal(developmentExternalDiscoveryChecks({ fixture: { externalBrowserDiscovery: observed } })[0].ok,
      accepted, 'doctor must reject missing, wrong, or invalid live policy');
    assert(!JSON.stringify(observed).includes('invalid-private-value'));
  }
  writeFileSync(join(descriptor.socketDir, 'runtime-host.json'), `${JSON.stringify({
    schemaVersion: 'agent-browser.runtime-host.v1',
    hostId: 'runtime-host:4242',
    pid: 4242,
    executableGeneration: installed.generation.sha256,
    socketIdentity: 'unix:fixture',
  })}\n`);
  writeFileSync(join(descriptor.socketDir, 'runtime-host.identity.json'), `${JSON.stringify({
    pid: 4242,
    startToken: 'linux:fixture-boot:100',
    executablePath: installed.generation.binary,
  })}\n`);
  const runtimeHostIngress = publishDevelopmentRuntimeIngress({
    descriptor,
    generationId: installed.generation.generationId,
    generationBinary: installed.generation.binary,
    sha256: installed.generation.sha256,
  });
  assert.equal(runtimeHostIngress.selectedBackend.pid, 4242);
  assert.equal(runtimeHostIngress.selectedBackend.generationId, installed.generation.generationId);
  assert.equal(
    JSON.parse(readFileSync(descriptor.runtimeHostIngressState, 'utf8')).bootEpoch,
    'linux:fixture-boot',
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

  const additionalLaneManifestPath = join(
    descriptor.pseudoHome,
    '.config',
    'agent-browser',
    'session-supervisors',
    'development-presentation-provider-v5-1.json',
  );
  writeFileSync(additionalLaneManifestPath, `${JSON.stringify({
    schemaVersion: 'agent-browser.session-supervisor.v1',
    session: 'development-presentation-provider-v5-1',
    executablePath: '/stale-development-generation/bin/agent-browser',
    executableSha256: '0'.repeat(64),
    streamPort: 37247,
    runtimeProfile: null,
    fixtureExtension: { preserve: true },
    provenance: { installedBy: 'fixture' },
  }, null, 2)}\n`);
  const rebound = installDevelopmentRuntime({ binary: fakeBinary, env, activate: false });
  const additionalLaneManifest = JSON.parse(readFileSync(additionalLaneManifestPath, 'utf8'));
  assert.equal(additionalLaneManifest.executablePath, rebound.generation.binary);
  assert.equal(additionalLaneManifest.executableSha256, rebound.generation.sha256);
  assert.deepEqual(additionalLaneManifest.fixtureExtension, { preserve: true });

  const laneManifestBeforeRejectedInstall = readFileSync(descriptor.laneManifest, 'utf8');
  const additionalLaneManifestBeforeRejectedInstall = readFileSync(additionalLaneManifestPath, 'utf8');
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
  assert.equal(
    readFileSync(additionalLaneManifestPath, 'utf8'),
    additionalLaneManifestBeforeRejectedInstall,
  );
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
  const packageScripts = JSON.parse(readFileSync('package.json', 'utf8')).scripts;
  assert.match(
    packageScripts['build:development-candidate'],
    /cargo-safe\.sh build --profile ci --manifest-path cli\/Cargo\.toml/,
  );
  assert.match(packageScripts['build:native'], /cargo-safe\.sh build --release/);
  assert.doesNotMatch(packageScripts['build:development-candidate'], /--release/);
  for (const documentationPath of [
    'README.md',
    'AGENTS.md',
    'skills/agent-browser/SKILL.md',
    'docs/src/app/dashboard/page.mdx',
  ]) {
    const documentation = readFileSync(documentationPath, 'utf8');
    assert.match(documentation, /pnpm build:development-candidate/);
    assert.match(documentation, /AGENT_BROWSER_CARGO_CACHE=off/);
    assert.match(documentation, /AGENT_BROWSER_CARGO_FAST_LINKER=off/);
  }
  console.log('Development runtime fixture passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
