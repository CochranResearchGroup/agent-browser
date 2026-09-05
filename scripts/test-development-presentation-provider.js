#!/usr/bin/env node

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
  developmentExternalIngressBinding,
  developmentPresentationProviderDescriptor,
  developmentPresentationProviderManifest,
  developmentPresentationProviderManifestCompatible,
  developmentPresentationProviderManifestUpgradeCompatible,
  developmentAgentSkillStatus,
  doctorDevelopmentPresentationProvider,
  synchronizeDevelopmentAgentSkill,
  validateDevelopmentPresentationProviderIsolation,
} from './lib/development-presentation-provider.js';
import {
  DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA,
  applyDevelopmentPresentationProvider,
  developmentPresentationProviderDeploymentPlan,
  ensureDevelopmentPresentationBootstrapInventory,
  probeDevelopmentPresentationProvider,
  prepareDevelopmentPresentationProviderSecrets,
  renderDevelopmentPresentationProviderBundle,
  stageDevelopmentPresentationProviderBundle,
} from './lib/development-presentation-provider-deployment.js';
import {
  createDevelopmentPresentationProviderSystemEffects,
  createDevelopmentPresentationLifecycleSystemEffects,
  developmentPresentationProviderSystemPreflight,
} from './lib/development-presentation-provider-system-effects.js';
import {
  evaluateDevelopmentPresentationPressure,
  sampleDevelopmentPresentationPressure,
} from './lib/development-presentation-pressure.js';
import {
  scaleInDevelopmentPresentation,
  scaleOutDevelopmentPresentation,
} from './lib/development-presentation-lifecycle.js';

const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-dev-provider-'));
const userHome = join(fixture, 'user');
const env = {
  ...process.env,
  AGENT_BROWSER_DEV_USER_HOME: userHome,
  AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: 'https://agent-browser-dev.example.test/',
  AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: 'cooper-test-revision-001',
};

function readExtensionArchive(path) {
  const result = spawnSync('python3', ['-c', `
import base64, io, json, sys, zipfile
with zipfile.ZipFile(io.BytesIO(sys.stdin.buffer.read())) as archive:
    assert archive.testzip() is None
    assert len(archive.namelist()) == len(set(archive.namelist()))
    print(json.dumps({name: base64.b64encode(archive.read(name)).decode() for name in archive.namelist()}))
`], { input: readFileSync(path), encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  return Object.fromEntries(Object.entries(JSON.parse(result.stdout))
    .map(([name, content]) => [name, Buffer.from(content, 'base64')]));
}

try {
  const descriptor = developmentPresentationProviderDescriptor(env);
  const namespaceEnv = {
    ...env,
    AGENT_BROWSER_DEV_NAMESPACE: 'p158',
    AGENT_BROWSER_DEV_DASHBOARD_PORT: '5148',
    AGENT_BROWSER_DEV_BACKEND_PORT: '5149',
    AGENT_BROWSER_DEV_SHADOW_PORT: '5150',
    AGENT_BROWSER_DEV_LANE_STREAM_PORT: '5151',
    AGENT_BROWSER_DEV_GUACAMOLE_PORT: '8193',
    AGENT_BROWSER_DEV_GUACD_PORT: '4923',
    AGENT_BROWSER_DEV_POSTGRES_PORT: '56433',
  };
  const namespaced = developmentPresentationProviderDescriptor(namespaceEnv);
  assert.notEqual(namespaced.root, descriptor.root);
  assert.notEqual(namespaced.pseudoHome, descriptor.pseudoHome);
  assert.equal(namespaced.routes[0].routeId, 'development-p158-route-1');
  assert.equal(namespaced.routes[0].user, 'agent-browser-rdp-dev-p158-1');
  assert.equal(namespaced.routes[0].viewerSession, 'development-p158-presentation-provider-v5-1');
  assert.equal(namespaced.composeProject, 'agent-browser-dev-p158-presentation');
  validateDevelopmentPresentationProviderIsolation(namespaced);
  for (const field of ['root', 'pseudoHome', 'composeProject', 'services', 'database', 'routes']) {
    assert.throws(() => validateDevelopmentPresentationProviderIsolation({
      ...namespaced, [field]: descriptor[field],
    }), /default development|namespace/);
  }
  assert.throws(() => developmentPresentationProviderDescriptor({
    ...namespaceEnv, AGENT_BROWSER_DEV_POSTGRES_PORT: '55433',
  }), /outside production/);
  const namespacedBundle = renderDevelopmentPresentationProviderBundle(namespaced);
  assert.ok(JSON.stringify(namespacedBundle).includes('agent-browser-dev-p158-guacamole-postgres-data'));
  stageDevelopmentPresentationProviderBundle({ env: namespaceEnv });
  assert.equal(existsSync(namespaced.root), true);
  assert.equal(existsSync(descriptor.root), false);
  const defaultDevelopmentRoot = join(userHome, '.local', 'share', 'agent-browser-dev');
  mkdirSync(defaultDevelopmentRoot, { recursive: true });
  const defaultAlias = join(fixture, 'default-development-alias');
  symlinkSync(defaultDevelopmentRoot, defaultAlias, 'dir');
  assert.throws(() => stageDevelopmentPresentationProviderBundle({ env: {
    ...namespaceEnv, AGENT_BROWSER_DEV_PRESENTATION_ROOT: join(defaultAlias, 'not-created', 'provider'),
  } }), /default development/);
  assert.equal(existsSync(join(defaultDevelopmentRoot, 'not-created')), false);
  const productionRoot = join(userHome, '.agent-browser');
  mkdirSync(productionRoot, { recursive: true });
  const productionAlias = join(fixture, 'production-alias');
  symlinkSync(productionRoot, productionAlias, 'dir');
  assert.throws(() => stageDevelopmentPresentationProviderBundle({ env: {
    ...namespaceEnv, AGENT_BROWSER_DEV_PRESENTATION_ROOT: join(productionAlias, 'not-created', 'provider'),
  } }), /production path/);
  assert.equal(existsSync(join(productionRoot, 'not-created')), false);
  assert.throws(() => stageDevelopmentPresentationProviderBundle({ env: {
    ...namespaceEnv, AGENT_BROWSER_DEV_PRESENTATION_ROOT: descriptor.root,
  } }), /default development/);
  assert.equal(existsSync(descriptor.root), false);
  const secondProvider = developmentPresentationProviderDescriptor({ ...namespaceEnv,
    AGENT_BROWSER_DEV_NAMESPACE: 'p159',
    AGENT_BROWSER_DEV_DASHBOARD_PORT: '5248',
    AGENT_BROWSER_DEV_BACKEND_PORT: '5249',
    AGENT_BROWSER_DEV_SHADOW_PORT: '5250',
    AGENT_BROWSER_DEV_LANE_STREAM_PORT: '5251',
    AGENT_BROWSER_DEV_GUACAMOLE_PORT: '8293',
    AGENT_BROWSER_DEV_GUACD_PORT: '5023',
    AGENT_BROWSER_DEV_POSTGRES_PORT: '57433',
  });
  assert.notEqual(secondProvider.root, namespaced.root);
  validateDevelopmentPresentationProviderIsolation(secondProvider);
  assert.ok(Object.values(secondProvider.ports).every((port) => !Object.values(namespaced.ports).includes(port)));
  assert.equal(developmentPresentationProviderDeploymentPlan(namespaced).providerRoot, namespaced.root);
  assert.equal(namespaced.warmSlots, 4);
  assert.equal(namespaced.hardMaxSlots, 6);
  const bootstrapOrder = [];
  assert.throws(() => applyDevelopmentPresentationProvider({
    env: namespaceEnv, authorizeEffects: true,
    effects: {
      snapshotProduction: () => ({}), assertProductionUnchanged: assert.deepEqual,
      createVolume: () => {}, startDatabase: () => {}, ensureRouteUser: () => {},
      syncConnections: () => {}, startProvider: () => {},
      grantOperatorRouteAccess: () => bootstrapOrder.push('access'),
      openWarmRoutes: () => {
        bootstrapOrder.push('open');
        assert.equal(existsSync(namespaced.manifest), false);
        assert.deepEqual(JSON.parse(readFileSync(namespaced.inventoryPath, 'utf8')).routes, []);
        throw new Error('bootstrap viewer failure');
      },
      quarantine: () => {},
    },
  }), /apply quarantined: bootstrap viewer failure/);
  assert.deepEqual(bootstrapOrder, ['access', 'open']);
  assert.equal(existsSync(namespaced.manifest), false);
  const retainedBootstrap = readFileSync(namespaced.inventoryPath, 'utf8');
  assert.equal(ensureDevelopmentPresentationBootstrapInventory(namespaced), false);
  assert.equal(readFileSync(namespaced.inventoryPath, 'utf8'), retainedBootstrap);
  mkdirSync(secondProvider.root, { recursive: true });
  writeFileSync(secondProvider.manifest, 'existing configured authority');
  assert.equal(ensureDevelopmentPresentationBootstrapInventory(secondProvider), false);
  assert.equal(readFileSync(secondProvider.manifest, 'utf8'), 'existing configured authority');
  assert.equal(existsSync(secondProvider.inventoryPath), false);
  for (const key of ['routeId', 'slotId', 'user', 'connectionKey', 'displayReservationId', 'viewerProfilePath']) {
    assert.ok(secondProvider.routes.every((route) => !namespaced.routes.some((other) => other[key] === route[key])));
  }
  assert.throws(() => createDevelopmentPresentationProviderSystemEffects({
    env: namespaceEnv, productionSnapshot: () => ({}), assertProductionUnchanged: () => {},
  }), /default development identity guards/);
  const guardedEffects = createDevelopmentPresentationProviderSystemEffects({
    env: namespaceEnv,
    productionSnapshot: () => ({ production: true }),
    assertProductionUnchanged: assert.deepEqual,
    defaultDevelopmentSnapshot: () => ({ defaultDevelopment: true }),
    assertDefaultDevelopmentUnchanged: assert.deepEqual,
  });
  const guardedBefore = guardedEffects.snapshotProduction();
  guardedEffects.assertProductionUnchanged(guardedBefore, guardedEffects.snapshotProduction());
  assert.throws(() => guardedEffects.assertProductionUnchanged(guardedBefore, {
    ...guardedBefore, defaultDevelopment: { changed: true },
  }));
  const routeOpenerSource = readFileSync('scripts/open-rdp-guac-route-displays.js', 'utf8');
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'set'/);
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'open'/);
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'close'/);
  assert.match(routeOpenerSource, /AGENT_BROWSER_ROUTE_DISPLAY_FORCE_VIEWER/);
  assert.match(routeOpenerSource, /profile\.includes\('\/'\)/);
  assert.equal(descriptor.environment, 'development');
  assert.equal(descriptor.warmSlots, 4);
  assert.equal(descriptor.hardMaxSlots, 6);
  assert.deepEqual(descriptor.connectionLimits, {
    maxConnections: 8,
    maxConnectionsPerUser: 8,
  });
  assert.equal(descriptor.routes.length, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.routeId)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.user)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.connectionKey)).size, 6);
  assert.deepEqual(descriptor.routes.map((route) => route.connectionId), Array(6).fill(null));
  assert.equal(new Set(descriptor.routes.map((route) => route.displayReservationId)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.viewerSession)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.viewerProfile)).size, 6);
  assert.equal(new Set(descriptor.routes.map((route) => route.viewerProfilePath)).size, 6);
  assert.ok(descriptor.routes.every((route) => route.viewerSession.includes('provider-v5')));
  assert.deepEqual(descriptor.routes.map((route) => route.displayName), Array(6).fill(null));
  assert.equal(descriptor.ports.guacamole, 8093);
  assert.equal(descriptor.ports.guacd, 4823);
  assert.equal(descriptor.ports.postgres, 55433);
  assert.equal(descriptor.localDiagnosticUrl, 'http://127.0.0.1:4948');
  assert.equal(descriptor.publicOperatorUrl, 'https://agent-browser-dev.example.test');
  assert.deepEqual(descriptor.externalIngress, {
    configured: true,
    publicOperatorUrl: 'https://agent-browser-dev.example.test',
    reviewedRevision: 'cooper-test-revision-001',
    bindingSha256: developmentExternalIngressBinding(env).bindingSha256,
  });
  assert.equal(descriptor.externalIngress.bindingSha256.length, 64);
  const unconfiguredDescriptor = developmentPresentationProviderDescriptor({
    ...env,
    AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: '',
    AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: '',
  });
  assert.equal(unconfiguredDescriptor.publicOperatorUrl, null);
  assert.equal(unconfiguredDescriptor.localDiagnosticUrl, 'http://127.0.0.1:4948');
  assert.equal(unconfiguredDescriptor.externalIngress.configured, false);
  const unconfiguredIngressEnv = {
    ...env,
    AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: '',
    AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: '',
  };
  assert.throws(
    () => stageDevelopmentPresentationProviderBundle({ env: unconfiguredIngressEnv }),
    /staging requires a reviewed public HTTPS external-ingress binding/,
  );
  assert.throws(
    () => applyDevelopmentPresentationProvider({
      env: unconfiguredIngressEnv,
      authorizeEffects: true,
      effects: {},
    }),
    /apply requires a reviewed public HTTPS external-ingress binding/,
  );
  for (const publicOperatorUrl of [
    'http://agent-browser-dev.example.test',
    'https://127.0.0.1',
    'https://10.1.2.3',
    'https://172.20.1.2',
    'https://192.168.1.2',
    'https://169.254.2.3',
    'https://provider.local',
    'https://user:secret@agent-browser-dev.example.test',
    'https://agent-browser-dev.example.test/remote-view',
    'https://agent-browser-dev.example.test?route=1',
    'https://agent-browser-dev.example.test#route',
  ]) {
    assert.throws(() => developmentExternalIngressBinding({
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: publicOperatorUrl,
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: 'cooper-test-revision-001',
    }), /public HTTPS origin/);
  }
  assert.throws(() => developmentExternalIngressBinding({
    AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: 'https://agent-browser-dev.example.test',
  }), /requires both/);
  assert.throws(() => developmentExternalIngressBinding({
    AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: 'cooper-test-revision-001',
  }), /requires both/);
  const currentManifest = developmentPresentationProviderManifest(descriptor);
  const legacyManifest = { ...currentManifest };
  delete legacyManifest.publicOperatorUrl;
  assert.equal(
    developmentPresentationProviderManifestCompatible(legacyManifest, currentManifest),
    false,
  );
  assert.equal(
    developmentPresentationProviderManifestCompatible(
      { ...legacyManifest, composeProject: 'foreign-provider' },
      currentManifest,
    ),
    false,
  );
  const legacyV1Manifest = { ...currentManifest };
  legacyV1Manifest.schemaVersion = 'agent-browser.development-presentation-provider.v1';
  delete legacyV1Manifest.localDiagnosticUrl;
  delete legacyV1Manifest.externalIngress;
  legacyV1Manifest.publicOperatorUrl = descriptor.localDiagnosticUrl;
  assert.equal(
    developmentPresentationProviderManifestUpgradeCompatible(legacyV1Manifest, currentManifest),
    true,
  );
  assert.equal(
    developmentPresentationProviderManifestUpgradeCompatible(
      { ...legacyV1Manifest, composeProject: 'foreign-provider' },
      currentManifest,
    ),
    false,
  );
  assert.equal(
    developmentPresentationProviderManifestUpgradeCompatible(
      { ...legacyV1Manifest, publicOperatorUrl: 'http://127.0.0.1:9999' },
      currentManifest,
    ),
    false,
  );
  assert.deepEqual(descriptor.rdpTarget, {
    host: 'host.docker.internal',
    port: 3389,
    isolation: 'route_user',
    sharedDaemon: true,
    restartAllowed: false,
  });
  assert.ok(descriptor.routes.every((route) => route.rdpPort === undefined));
  assert.match(descriptor.root, /agent-browser-dev\/presentation-provider$/);
  assert.match(descriptor.skill.target, /agent-browser-dev\/home\/\.codex\/skills\/agent-browser$/);
  assert.doesNotMatch(JSON.stringify(descriptor.routes), /route-a|route-b/i);
  assert.doesNotThrow(() => validateDevelopmentPresentationProviderIsolation(descriptor));
  const headroomReadings = {
    memoryAvailableBytes: 32 * 1024 ** 3,
    swapFreeBytes: 4 * 1024 ** 3,
    swapTotalBytes: 8 * 1024 ** 3,
    loadOne: 24,
    cpuCount: 20,
    cpuSampleAvailable: true,
    cpuIdleFraction: 0.25,
    ioWaitFraction: 0.01,
    fileHandlesAllocated: 100,
    fileHandlesMaximum: 1_000,
  };
  const laggingLoadWithCpuHeadroom = evaluateDevelopmentPresentationPressure(
    descriptor,
    headroomReadings,
  );
  assert.equal(laggingLoadWithCpuHeadroom.admittedMaximum, descriptor.hardMaxSlots);
  assert.deepEqual(laggingLoadWithCpuHeadroom.reasons, []);
  assert.equal(laggingLoadWithCpuHeadroom.readings.cpuAdmissionSource, 'sampled_idle_headroom');
  const pressureEffects = createDevelopmentPresentationLifecycleSystemEffects({
    env,
    productionSnapshot: () => ({ identity: 'production-fixture' }),
    assertProductionUnchanged: () => {},
    pressureSnapshot: () => headroomReadings,
  });
  const effectAdmission = pressureEffects.pressureAdmission(descriptor);
  assert.equal(effectAdmission.admittedMaximum, descriptor.hardMaxSlots);
  assert.equal(effectAdmission.readings.cpuAdmissionSource, 'sampled_idle_headroom');
  const saturatedAdmission = evaluateDevelopmentPresentationPressure(descriptor, {
    ...headroomReadings,
    loadOne: 10,
    cpuIdleFraction: 0,
  });
  assert.equal(saturatedAdmission.admittedMaximum, descriptor.warmSlots);
  assert.deepEqual(saturatedAdmission.reasons, ['cpu_capacity']);
  let cpuStatReads = 0;
  const sampledHeadroom = sampleDevelopmentPresentationPressure({
    cpuCount: 20,
    wait: (milliseconds) => assert.equal(milliseconds, 1_000),
    readFile: (path) => {
      if (path === '/proc/stat') {
        cpuStatReads += 1;
        return cpuStatReads === 1
          ? 'cpu 100 0 100 700 0 0 0 0 0 0\n'
          : 'cpu 120 0 120 860 0 0 0 0 0 0\n';
      }
      if (path === '/proc/meminfo') {
        return 'MemAvailable: 33554432 kB\nSwapFree: 4194304 kB\nSwapTotal: 8388608 kB\n';
      }
      if (path === '/proc/loadavg') return '30.00 25.00 20.00 1/100 1\n';
      if (path === '/proc/sys/fs/file-nr') return '100 0 1000\n';
      throw new Error(`unexpected pressure fixture path: ${path}`);
    },
  });
  const sampledAdmission = evaluateDevelopmentPresentationPressure(descriptor, sampledHeadroom);
  assert.equal(sampledHeadroom.cpuIdleFraction, 0.8);
  assert.equal(sampledAdmission.admittedMaximum, descriptor.hardMaxSlots);
  assert.deepEqual(sampledAdmission.reasons, []);
  const fallbackAdmission = evaluateDevelopmentPresentationPressure(descriptor, {
    ...headroomReadings,
    cpuSampleAvailable: false,
    cpuIdleFraction: null,
    ioWaitFraction: null,
  });
  assert.equal(fallbackAdmission.admittedMaximum, descriptor.warmSlots);
  assert.deepEqual(fallbackAdmission.reasons, ['cpu_load']);
  assert.equal(fallbackAdmission.readings.cpuAdmissionSource, 'load_average_fallback');
  const ioPressureAdmission = evaluateDevelopmentPresentationPressure(descriptor, {
    ...headroomReadings,
    ioWaitFraction: 0.2,
  });
  assert.equal(ioPressureAdmission.admittedMaximum, descriptor.warmSlots);
  assert.deepEqual(ioPressureAdmission.reasons, ['io_pressure']);
  const singleRouteInventory = JSON.stringify([{
    id: 'development-route-5',
    routeId: 'guacamole:105',
    connectionId: '105',
    connectionName: 'Agent Browser Dev RDP Route 5',
    frameUrl: 'http://127.0.0.1:8093/guacamole/#/client/fixture',
    target: { routeUser: 'agent-browser-rdp-dev-5' },
  }]);
  const routeOpenerEnv = {
    ...env,
    HOME: userHome,
    AGENT_BROWSER_HOME: join(fixture, 'empty-agent-home'),
    AGENT_BROWSER_RDP_ROUTE_POOL_JSON: singleRouteInventory,
  };
  const rejectedSingleRoute = spawnSync(process.execPath, [
    'scripts/open-rdp-guac-route-displays.js',
    '--dry-run',
  ], { encoding: 'utf8', env: routeOpenerEnv });
  assert.equal(rejectedSingleRoute.status, 1);
  assert.match(JSON.parse(rejectedSingleRoute.stdout).error, /expected at least two route-pool entries/);
  const acceptedLifecycleRoute = spawnSync(process.execPath, [
    'scripts/open-rdp-guac-route-displays.js',
    '--dry-run',
    '--allow-single-route',
  ], { encoding: 'utf8', env: routeOpenerEnv });
  assert.equal(acceptedLifecycleRoute.status, 0);
  assert.equal(JSON.parse(acceptedLifecycleRoute.stdout).selectedRoutes.length, 1);
  const fixtureBin = join(fixture, 'bin');
  mkdirSync(fixtureBin, { recursive: true });
  const fixturePs = join(fixtureBin, 'ps');
  writeFileSync(fixturePs, [
    '#!/bin/sh',
    "printf '%s\\n' 'agent-browser-rdp-dev-5 4242 Xorg /usr/lib/xorg/Xorg :16 -auth .Xauthority'",
    '',
  ].join('\n'));
  chmodSync(fixturePs, 0o755);
  const rejectedSingleDisplay = spawnSync(process.execPath, [
    'scripts/inspect-rdp-route-displays.js',
  ], {
    encoding: 'utf8',
    env: { ...routeOpenerEnv, PATH: `${fixtureBin}:${process.env.PATH}` },
  });
  assert.equal(rejectedSingleDisplay.status, 1);
  const acceptedSingleDisplay = spawnSync(process.execPath, [
    'scripts/inspect-rdp-route-displays.js',
  ], {
    encoding: 'utf8',
    env: {
      ...routeOpenerEnv,
      PATH: `${fixtureBin}:${process.env.PATH}`,
      AGENT_BROWSER_ROUTE_DISPLAY_ALLOW_SINGLE_ROUTE: '1',
    },
  });
  assert.equal(acceptedSingleDisplay.status, 0);
  assert.equal(JSON.parse(acceptedSingleDisplay.stdout).routeInventory[0].displayName, ':16');
  const plan = developmentPresentationProviderDeploymentPlan(descriptor);
  assert.equal(plan.schemaVersion, DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA);
  assert.equal(plan.environment, 'development');
  assert.equal(plan.authorizesEffects, false);
  assert.equal(plan.requiresExplicitApply, true);
  assert.equal(plan.productionPosture, 'read_only');
  assert.equal(plan.steps.at(-1).id, 'publish-ingress');

  const bundle = renderDevelopmentPresentationProviderBundle(descriptor);
  assert.match(bundle.files['compose.yml'], /^name: agent-browser-dev-presentation$/m);
  assert.match(bundle.files['compose.yml'], /127\.0\.0\.1:8093:8080/);
  assert.match(bundle.files['compose.yml'], /127\.0\.0\.1:4823:4822/);
  assert.match(bundle.files['compose.yml'], /127\.0\.0\.1:55433:5432/);
  assert.match(bundle.files['compose.yml'], /POSTGRES_DB: "\$\{POSTGRES_DB:\?set POSTGRES_DB in \.env\}"/);
  assert.match(bundle.files['compose.yml'], /test:\n\s+- CMD-SHELL\n\s+- "pg_is_ready|test:\n\s+- CMD-SHELL\n\s+- "pg_isready/);
  assert.doesNotMatch(bundle.files['compose.yml'], /^\s*container_name: agent-browser-guacamole$/m);
  assert.equal(bundle.routeUsers.length, 6);
  assert.ok(bundle.routeUsers.every((route) => route.password === undefined));
  assert.ok(bundle.routeUsers.every((route) => route.legacyConnectionName === ''));
  assert.equal(bundle.ingress.pathPrefix, '/guacamole');
  assert.equal(bundle.ingress.upstream, 'http://127.0.0.1:8093/guacamole');
  const staged = stageDevelopmentPresentationProviderBundle({ env });
  assert.equal(staged.success, true);
  assert.equal(staged.authorizesProviderEffects, false);
  assert.equal(readFileSync(join(descriptor.root, 'compose.yml'), 'utf8'), bundle.files['compose.yml']);
  assert.equal(statSync(descriptor.root).mode & 0o777, 0o755);
  assert.equal(statSync(join(descriptor.root, 'init')).mode & 0o777, 0o755);
  assert.equal(existsSync(join(descriptor.root, 'init', '001-initdb.sql')), true);
  assert.equal(existsSync(join(descriptor.root, 'extensions', 'guac-manifest.json')), true);
  const extensionPath = join(descriptor.root, 'extensions', 'agent-browser-defaults.jar');
  const extensionEntries = readExtensionArchive(extensionPath);
  assert.deepEqual(Object.keys(extensionEntries), ['guac-manifest.json', 'agent-browser-defaults.js']);
  for (const [name, content] of Object.entries(extensionEntries)) {
    assert.deepEqual(content, readFileSync(join('cli/assets/workstation/guacamole/extensions', name)));
  }
  assert.equal(statSync(extensionPath).mode & 0o777, 0o644);
  assert.match(staged.files['extensions/agent-browser-defaults.jar'], /^[a-f0-9]{64}$/);
  assert.equal(existsSync(join(descriptor.root, 'secrets', 'provider.env')), false);
  assert.equal(existsSync(descriptor.manifest), false);
  const preflight = developmentPresentationProviderSystemPreflight({
    env,
    run(command, args) {
      if (command === 'docker' && args[0] === 'info') {
        return { status: 0, stdout: '29.7.2\n', stderr: '' };
      }
      if (command === 'sudo') return { status: 0, stdout: 'ready\n', stderr: '' };
      if (command === 'systemctl') return { status: 0, stdout: 'active\n', stderr: '' };
      if (command === 'ss') return { status: 0, stdout: '', stderr: '' };
      if (command === 'docker' && args[0] === 'inspect') {
        return { status: 1, stdout: '', stderr: 'not found' };
      }
      if (command === 'getent') return { status: 2, stdout: '', stderr: '' };
      throw new Error(`Unexpected preflight command: ${command} ${args.join(' ')}`);
    },
  });
  assert.equal(preflight.success, true);
  assert.equal(preflight.authorizesEffects, false);
  assert.ok(preflight.checks.every((check) => check.ok));
  const firstSecrets = prepareDevelopmentPresentationProviderSecrets({ env });
  const secondSecrets = prepareDevelopmentPresentationProviderSecrets({ env });
  assert.equal(firstSecrets.created, true);
  assert.equal(secondSecrets.created, false);
  assert.equal(firstSecrets.sha256, secondSecrets.sha256);
  assert.equal(statSync(firstSecrets.path).mode & 0o777, 0o600);
  const secretText = readFileSync(firstSecrets.path, 'utf8');
  assert.match(secretText, /^POSTGRES_PASSWORD=/m);
  assert.match(secretText, /^XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON=/m);
  assert.doesNotMatch(JSON.stringify(firstSecrets), /POSTGRES_PASSWORD|password/);
  assert.equal(developmentAgentSkillStatus({ env }).state, 'unconfigured');
  const skill = synchronizeDevelopmentAgentSkill({ env });
  assert.equal(skill.environment, 'development');
  assert.equal(developmentAgentSkillStatus({ env }).state, 'current');
  assert.notEqual(descriptor.skill.target, join(userHome, '.codex', 'skills', 'agent-browser'));

  for (const [field, value] of [
    ['AGENT_BROWSER_DEV_GUACAMOLE_PORT', '8092'],
    ['AGENT_BROWSER_DEV_GUACD_PORT', '4822'],
    ['AGENT_BROWSER_DEV_POSTGRES_PORT', '5432'],
  ]) {
    assert.throws(
      () => validateDevelopmentPresentationProviderIsolation(
        developmentPresentationProviderDescriptor({ ...env, [field]: value }),
      ),
      /collides with production/i,
    );
  }
  assert.throws(
    () => validateDevelopmentPresentationProviderIsolation(
      developmentPresentationProviderDescriptor({
        ...env,
        AGENT_BROWSER_DEV_PRESENTATION_ROOT: join(userHome, '.agent-browser', 'presentation-provider'),
      }),
    ),
    /overlaps production/i,
  );
  assert.throws(
    () => validateDevelopmentPresentationProviderIsolation(
      developmentPresentationProviderDescriptor({
        ...env,
        AGENT_BROWSER_DEV_GUACD_PORT: '8093',
      }),
    ),
    /duplicate ports/i,
  );

  const optional = doctorDevelopmentPresentationProvider({ env });
  assert.equal(optional.success, true);
  assert.equal(optional.status.state, 'unconfigured');
  assert.equal(optional.status.ready, false);
  assert.equal(optional.status.blocking, false);
  const required = doctorDevelopmentPresentationProvider({
    env: { ...env, AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED: '1' },
  });
  assert.equal(required.success, false);
  assert.equal(required.status.blocking, true);
  const requiredWithoutIngress = doctorDevelopmentPresentationProvider({
    env: {
      ...env,
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: '',
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: '',
      AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED: '1',
    },
  });
  assert.equal(requiredWithoutIngress.success, false);
  assert.match(requiredWithoutIngress.status.isolationError, /no reviewed public HTTPS ingress binding/);

  mkdirSync(descriptor.root, { recursive: true });
  writeFileSync(
    descriptor.manifest,
    `${JSON.stringify(developmentPresentationProviderManifest(descriptor), null, 2)}\n`,
  );
  const configuredWithoutResources = doctorDevelopmentPresentationProvider({ env });
  assert.equal(configuredWithoutResources.success, false);
  assert.equal(configuredWithoutResources.status.state, 'not_ready');
  const readyObservation = {
    environment: 'development',
    containers: Object.values(descriptor.services).map((name) => ({
      name,
      running: true,
      composeProject: descriptor.composeProject,
    })),
    ports: Object.fromEntries(Object.entries(descriptor.ports).map(([name, port]) => [
      name,
      { port, listening: true },
    ])),
    routeUsers: descriptor.routes.map((route) => ({ user: route.user, exists: true })),
    database: {
      schemaReady: true,
      routes: descriptor.routes.map((route, index) => ({
        connectionId: String(index + 100),
        connectionName: route.connectionName,
        user: route.user,
        maxConnections: 8,
        maxConnectionsPerUser: 8,
        sharingProfileId: String(index + 200),
        sharingProfileName: `Agent Browser Shared Session ${route.routeId}`,
        sharingProfileReadOnly: 'false',
        sharingProfilePermissionCount: 1,
      })),
    },
    displays: descriptor.routes.slice(0, descriptor.warmSlots).map((route, index) => ({
      displayReservationId: route.displayReservationId,
      displayName: `:${30 + index}`,
      user: route.user,
      ready: true,
    })),
    secrets: { private: true },
  };
  const probed = probeDevelopmentPresentationProvider(descriptor, {
    run(command, args) {
      if (command === 'docker' && args[0] === 'inspect') {
        return { status: 0, stdout: `true\t${descriptor.composeProject}\n`, stderr: '' };
      }
      if (command === 'ss') return { status: 0, stdout: 'LISTEN\n', stderr: '' };
      if (command === 'getent') return { status: 0, stdout: 'fixture-user\n', stderr: '' };
      if (command === 'docker' && args[0] === 'exec') {
        const sql = args.at(-1);
        if (sql.includes('information_schema.tables')) {
          return { status: 0, stdout: '8\n', stderr: '' };
        }
        return {
          status: 0,
          stdout: `${JSON.stringify(readyObservation.database.routes)}\n`,
          stderr: '',
        };
      }
      if (command === 'ps') {
        assert.deepEqual(args, ['-eo', 'user:64=,args=']);
        return {
          status: 0,
          stdout: descriptor.routes
            .flatMap((route, index) => [
              `${route.user} /usr/lib/systemd/systemd --user`,
              `${route.user} /usr/lib/xorg/Xorg :${30 + index}`,
            ])
            .join('\n'),
          stderr: '',
        };
      }
      throw new Error(`Unexpected probe command: ${command} ${args.join(' ')}`);
    },
    displaySocketExists: () => true,
  });
  assert.equal(probed.database.schemaReady, true);
  assert.equal(probed.database.routes.length, 6);
  assert.equal(probed.displays.length, 6);
  assert.equal(probed.displays.at(-1).displayReservationId, 'development-display-6');
  assert.equal(probed.secrets.private, true);
  for (const probeDescriptor of [descriptor, namespaced, {
    ...namespaced,
    routes: [{ ...namespaced.routes[0], connectionName: "Fixture O'Brien\\route" }],
  }]) {
    let routeQueryObserved = false;
    const exactRows = probeDescriptor.routes.map((route, index) => ({
      connectionId: String(index + 100), connectionName: route.connectionName, user: route.user,
    }));
    const exactProbe = probeDevelopmentPresentationProvider(probeDescriptor, {
      run(command, args) {
        if (command === 'docker' && args[0] === 'exec') {
          assert.equal(args[1], probeDescriptor.services.postgres);
          const sql = args.at(-1);
          if (sql.includes('information_schema.tables')) return { status: 0, stdout: '8\n' };
          routeQueryObserved = true;
          const exactNames = probeDescriptor.routes.map((route) =>
            `E'${route.connectionName.replaceAll('\\', '\\\\').replaceAll("'", "''")}'`).join(', ');
          assert.ok(sql.includes(`where c.connection_name in (${exactNames})`), sql);
          assert.doesNotMatch(sql, /c\.connection_name like/i);
          return { status: 0, stdout: JSON.stringify(exactRows) };
        }
        return { status: 0, stdout: '' };
      },
      displaySocketExists: () => false,
    });
    assert.equal(routeQueryObserved, true);
    assert.deepEqual(exactProbe.database.routes, exactRows);
  }
  const configured = doctorDevelopmentPresentationProvider({ env, probe: () => readyObservation });
  assert.equal(configured.success, true);
  assert.equal(configured.status.state, 'configured');
  assert.equal(configured.status.ready, true);
  assert.equal(configured.status.manifest.schemaVersion, DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA);
  const statusEnvWithoutIngress = { ...env };
  delete statusEnvWithoutIngress.AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL;
  delete statusEnvWithoutIngress.AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION;
  const configuredFromPersistedIngress = doctorDevelopmentPresentationProvider({
    env: {
      ...statusEnvWithoutIngress,
      AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED: '1',
    },
    probe: () => readyObservation,
  });
  assert.equal(configuredFromPersistedIngress.success, true);
  assert.equal(configuredFromPersistedIngress.status.state, 'configured');
  assert.deepEqual(
    configuredFromPersistedIngress.status.descriptor.externalIngress,
    descriptor.externalIngress,
  );
  let mismatchedProbeCalls = 0;
  const explicitlyMismatchedIngress = doctorDevelopmentPresentationProvider({
    env: {
      ...env,
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: 'cooper-different-revision-002',
    },
    probe: () => {
      mismatchedProbeCalls += 1;
      return readyObservation;
    },
  });
  assert.equal(explicitlyMismatchedIngress.status.state, 'drifted');
  assert.equal(mismatchedProbeCalls, 0);
  const explicitlyEmptyIngress = doctorDevelopmentPresentationProvider({
    env: {
      ...statusEnvWithoutIngress,
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: '',
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: '',
    },
    probe: () => readyObservation,
  });
  assert.equal(explicitlyEmptyIngress.status.state, 'drifted');
  assert.throws(() => doctorDevelopmentPresentationProvider({
    env: {
      ...statusEnvWithoutIngress,
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: descriptor.publicOperatorUrl,
    },
  }), /requires both/);
  const canonicalManifest = developmentPresentationProviderManifest(descriptor);
  const tamperedIngressManifests = [
    { ...structuredClone(canonicalManifest), publicOperatorUrl: 'https://other.example.test' },
    {
      ...structuredClone(canonicalManifest),
      publicOperatorUrl: 'https://127.0.0.1',
      externalIngress: {
        ...canonicalManifest.externalIngress,
        publicOperatorUrl: 'https://127.0.0.1',
      },
    },
    {
      ...structuredClone(canonicalManifest),
      externalIngress: {
        ...canonicalManifest.externalIngress,
        reviewedRevision: 'tampered-revision',
      },
    },
    {
      ...structuredClone(canonicalManifest),
      externalIngress: {
        ...canonicalManifest.externalIngress,
        bindingSha256: '00'.repeat(32),
      },
    },
  ];
  for (const tamperedManifest of tamperedIngressManifests) {
    writeFileSync(descriptor.manifest, `${JSON.stringify(tamperedManifest, null, 2)}\n`);
    let tamperedProbeCalls = 0;
    const tampered = doctorDevelopmentPresentationProvider({
      env: statusEnvWithoutIngress,
      probe: () => {
        tamperedProbeCalls += 1;
        return readyObservation;
      },
    });
    assert.equal(tampered.success, false);
    assert.equal(tampered.status.state, 'drifted');
    assert.equal(tamperedProbeCalls, 0);
  }
  writeFileSync(descriptor.manifest, `${JSON.stringify(canonicalManifest, null, 2)}\n`);
  const capacityDrift = structuredClone(readyObservation);
  capacityDrift.database.routes[0].maxConnections = 4;
  capacityDrift.database.routes[0].maxConnectionsPerUser = 2;
  const driftedCapacity = doctorDevelopmentPresentationProvider({ env, probe: () => capacityDrift });
  assert.equal(driftedCapacity.success, false);
  assert.equal(
    driftedCapacity.checks.find((item) => item.name === 'presentation-provider:connection:development-route-1')?.ok,
    false,
  );
  const missingSharingProfile = structuredClone(readyObservation);
  missingSharingProfile.database.routes[0].sharingProfileId = null;
  const driftedSharingProfile = doctorDevelopmentPresentationProvider({ env, probe: () => missingSharingProfile });
  assert.equal(driftedSharingProfile.success, false);
  assert.equal(
    driftedSharingProfile.checks.find((item) => item.name === 'presentation-provider:connection:development-route-1')?.ok,
    false,
  );

  rmSync(descriptor.manifest, { force: true });
  const effectCalls = [];
  const applied = applyDevelopmentPresentationProvider({
    env,
    authorizeEffects: true,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      createVolume: () => effectCalls.push('create-volume'),
      startDatabase: () => effectCalls.push('start-database'),
      ensureRouteUser: (route) => effectCalls.push(`ensure-user:${route.routeId}`),
      syncConnections: () => effectCalls.push('sync-connections'),
      startProvider: () => effectCalls.push('start-provider'),
      grantOperatorRouteAccess: () => effectCalls.push('grant-operator-route-access'),
      openWarmRoutes: () => {
        assert.equal(existsSync(descriptor.manifest), false);
        assert.equal(existsSync(descriptor.inventoryPath), true);
        const bootstrap = JSON.parse(readFileSync(descriptor.inventoryPath, 'utf8'));
        assert.equal(bootstrap.schemaVersion, 'agent-browser.development-presentation-inventory.v1');
        assert.deepEqual(bootstrap.routes, []);
        effectCalls.push('open-warm-routes');
      },
      observe: () => readyObservation,
      grantDisplayAccess: (display) => effectCalls.push(`grant:${display.displayReservationId}`),
      publishIngress: () => effectCalls.push('publish-ingress'),
      quarantine: () => effectCalls.push('quarantine'),
    },
  });
  assert.equal(applied.success, true);
  assert.equal(applied.state, 'applied');
  assert.equal(applied.providerReady, true);
  assert.equal(applied.productionUnchanged, true);
  assert.equal(existsSync(descriptor.manifest), true);
  assert.equal(existsSync(descriptor.inventoryPath), true);
  const authorityInventory = JSON.parse(readFileSync(descriptor.inventoryPath, 'utf8'));
  assert.equal(
    authorityInventory.routes[0].routeDescriptor.publicOperatorUrl,
    'https://agent-browser-dev.example.test',
  );
  assert.equal(authorityInventory.localDiagnosticUrl, 'http://127.0.0.1:4948');
  assert.equal(authorityInventory.externalIngress.bindingSha256, descriptor.externalIngress.bindingSha256);
  assert.match(
    authorityInventory.routes[0].routeDescriptor.localEmbedUrl,
    /^http:\/\/127\.0\.0\.1:8093\/guacamole\/#\/client\//,
  );
  assert.deepEqual(effectCalls, [
    'create-volume',
    'start-database',
    ...descriptor.routes.map((route) => `ensure-user:${route.routeId}`),
    'sync-connections',
    'start-provider',
    'grant-operator-route-access',
    'open-warm-routes',
    ...descriptor.routes.slice(0, descriptor.warmSlots)
      .map((route) => `grant:${route.displayReservationId}`),
    'publish-ingress',
  ]);
  assert.doesNotMatch(JSON.stringify(applied), /POSTGRES_PASSWORD|"password"/);

  const restaged = stageDevelopmentPresentationProviderBundle({ env });
  assert.equal(restaged.success, true);
  assert.equal(restaged.state, 'refreshed_configured');
  assert.equal(restaged.files['extensions/agent-browser-defaults.jar'],
    staged.files['extensions/agent-browser-defaults.jar'], 'identical inputs produce identical JAR bytes');
  const updatedAssetRoot = join(fixture, 'updated-guacamole-assets');
  cpSync('cli/assets/workstation/guacamole', updatedAssetRoot, { recursive: true });
  const updatedScript = Buffer.concat([extensionEntries['agent-browser-defaults.js'],
    Buffer.from('\n// provider staging refresh fixture\n')]);
  writeFileSync(join(updatedAssetRoot, 'extensions', 'agent-browser-defaults.js'), updatedScript);
  const refreshedBundle = stageDevelopmentPresentationProviderBundle({
    env: { ...env, AGENT_BROWSER_DEV_GUACAMOLE_ASSET_SOURCE: updatedAssetRoot },
  });
  assert.equal(refreshedBundle.state, 'refreshed_configured');
  assert.notEqual(refreshedBundle.files['extensions/agent-browser-defaults.jar'],
    staged.files['extensions/agent-browser-defaults.jar']);
  assert.deepEqual(readExtensionArchive(extensionPath), {
    'guac-manifest.json': extensionEntries['guac-manifest.json'],
    'agent-browser-defaults.js': updatedScript,
  });
  assert.equal(existsSync(descriptor.manifest), true);
  const reconcilePreflight = developmentPresentationProviderSystemPreflight({
    env,
    run(command, args) {
      if (command === 'docker' && args[0] === 'info') {
        return { status: 0, stdout: '29.7.2\n', stderr: '' };
      }
      if (command === 'sudo') return { status: 0, stdout: 'ready\n', stderr: '' };
      if (command === 'systemctl') return { status: 0, stdout: 'active\n', stderr: '' };
      if (command === 'ss') return { status: 0, stdout: 'LISTEN owned\n', stderr: '' };
      if (command === 'docker' && args[0] === 'inspect') {
        return {
          status: 0,
          stdout: `${descriptor.composeProject}\ttrue\n`,
          stderr: '',
        };
      }
      if (command === 'getent') {
        const user = args.at(-1);
        return {
          status: 0,
          stdout: `${user}:x:2000:2000:agent-browser route-pool RDP session:/home/${user}:/bin/bash\n`,
          stderr: '',
        };
      }
      throw new Error(`Unexpected configured preflight command: ${command} ${args.join(' ')}`);
    },
  });
  assert.equal(reconcilePreflight.mode, 'reconcile');
  assert.equal(reconcilePreflight.success, true);
  assert.ok(reconcilePreflight.checks.every((check) => check.ok));

  rmSync(descriptor.manifest, { force: true });
  effectCalls.length = 0;
  const deferred = applyDevelopmentPresentationProvider({
    env,
    authorizeEffects: true,
    deferIngress: true,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      createVolume: () => effectCalls.push('create-volume'),
      startDatabase: () => effectCalls.push('start-database'),
      ensureRouteUser: (route) => effectCalls.push(`ensure-user:${route.routeId}`),
      syncConnections: () => effectCalls.push('sync-connections'),
      startProvider: () => effectCalls.push('start-provider'),
      grantOperatorRouteAccess: () => effectCalls.push('grant-operator-route-access'),
      openWarmRoutes: () => effectCalls.push('open-warm-routes'),
      observe: () => readyObservation,
      grantDisplayAccess: (display) => effectCalls.push(`grant:${display.displayReservationId}`),
      quarantine: () => effectCalls.push('quarantine'),
    },
  });
  assert.equal(deferred.state, 'provider_ready_ingress_pending');
  assert.equal(deferred.providerReady, true);
  assert.equal(deferred.ingressPublished, false);
  assert.equal(existsSync(descriptor.manifest), true);
  assert.equal(effectCalls.includes('publish-ingress'), false);
  assert.equal(effectCalls.includes('quarantine'), false);

  effectCalls.length = 0;
  const reconciled = applyDevelopmentPresentationProvider({
    env,
    authorizeEffects: true,
    deferIngress: true,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      observe: () => readyObservation,
      publishIngress: () => effectCalls.push('publish-ingress'),
    },
  });
  assert.equal(reconciled.state, 'provider_ready_ingress_pending');
  assert.deepEqual(reconciled.completedSteps, ['reconcile-provider-authority']);
  assert.deepEqual(effectCalls, []);

  effectCalls.length = 0;
  let capacityObservation = capacityDrift;
  const capacityReconciled = applyDevelopmentPresentationProvider({
    env,
    authorizeEffects: true,
    deferIngress: true,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      createVolume: () => effectCalls.push('create-volume'),
      startDatabase: () => effectCalls.push('start-database'),
      ensureRouteUser: (route) => effectCalls.push(`ensure-user:${route.routeId}`),
      syncConnections: () => {
        effectCalls.push('sync-connections');
        capacityObservation = readyObservation;
      },
      startProvider: () => effectCalls.push('start-provider'),
      grantOperatorRouteAccess: () => effectCalls.push('grant-operator-route-access'),
      openWarmRoutes: () => effectCalls.push('open-warm-routes'),
      observe: () => capacityObservation,
      grantDisplayAccess: (display) => effectCalls.push(`grant:${display.displayReservationId}`),
      quarantine: () => effectCalls.push('quarantine'),
    },
  });
  assert.equal(capacityReconciled.providerReady, true);
  assert.equal(effectCalls.includes('sync-connections'), true);
  assert.equal(effectCalls.includes('quarantine'), false);

  effectCalls.length = 0;
  const stoppedObservation = structuredClone(readyObservation);
  stoppedObservation.containers.find((item) =>
    item.name === descriptor.services.guacamole
  ).running = false;
  stoppedObservation.ports.guacamole.listening = false;
  stoppedObservation.displays = [];
  let reconcileObservation = stoppedObservation;
  const recovered = applyDevelopmentPresentationProvider({
    env,
    authorizeEffects: true,
    deferIngress: true,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      createVolume: () => effectCalls.push('create-volume'),
      startDatabase: () => effectCalls.push('start-database'),
      ensureRouteUser: (route) => effectCalls.push(`ensure-user:${route.routeId}`),
      syncConnections: () => effectCalls.push('sync-connections'),
      startProvider: () => effectCalls.push('start-provider'),
      grantOperatorRouteAccess: () => effectCalls.push('grant-operator-route-access'),
      openWarmRoutes: () => {
        effectCalls.push('open-warm-routes');
        reconcileObservation = readyObservation;
      },
      observe: () => reconcileObservation,
      grantDisplayAccess: (display) => effectCalls.push(`grant:${display.displayReservationId}`),
      quarantine: () => effectCalls.push('quarantine'),
    },
  });
  assert.equal(recovered.state, 'provider_ready_ingress_pending');
  assert.equal(recovered.providerReady, true);
  assert.deepEqual(effectCalls, [
    'create-volume',
    'start-database',
    ...descriptor.routes.map((route) => `ensure-user:${route.routeId}`),
    'sync-connections',
    'start-provider',
    'grant-operator-route-access',
    'open-warm-routes',
    ...descriptor.routes.slice(0, descriptor.warmSlots)
      .map((route) => `grant:${route.displayReservationId}`),
  ]);

  let configuredQuarantine = null;
  effectCalls.length = 0;
  assert.throws(
    () => applyDevelopmentPresentationProvider({
      env,
      authorizeEffects: true,
      deferIngress: true,
      effects: {
        snapshotProduction: () => ({ identity: 'production-fixture' }),
        assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
        createVolume: () => effectCalls.push('create-volume'),
        startDatabase: () => effectCalls.push('start-database'),
        ensureRouteUser: (route) => effectCalls.push(`ensure-user:${route.routeId}`),
        syncConnections: () => effectCalls.push('sync-connections'),
        startProvider: () => effectCalls.push('start-provider'),
        grantOperatorRouteAccess: () => effectCalls.push('grant-operator-route-access'),
        openWarmRoutes: () => { throw new Error('configured warm-route failure'); },
        observe: () => stoppedObservation,
        grantDisplayAccess: () => {},
        quarantine: (receipt) => { configuredQuarantine = receipt; },
      },
    }),
    /apply quarantined: configured warm-route failure/,
  );
  assert.equal(configuredQuarantine.reason, 'configured warm-route failure');
  assert.equal(configuredQuarantine.completedSteps.includes('start-provider'), true);

  const scaleCalls = [];
  const scaledObservation = structuredClone(readyObservation);
  scaledObservation.displays.push({
    displayReservationId: descriptor.routes[4].displayReservationId,
    displayName: ':35',
    user: descriptor.routes[4].user,
    ready: true,
  });
  let scaleObservation = readyObservation;
  const scaled = scaleOutDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      reclaimCapability: () => ({ ready: true }),
      pressureAdmission: () => ({ admittedMaximum: 6, reasons: [] }),
      observe: () => scaleObservation,
      provisionRoute: (route) => {
        scaleCalls.push(`provision:${route.routeId}`);
        scaleObservation = scaledObservation;
      },
      grantDisplayAccess: (display) => scaleCalls.push(`grant:${display.displayReservationId}`),
    },
  });
  assert.equal(scaled.state, 'provisioned');
  assert.equal(scaled.routeId, 'development-route-5');
  assert.equal(scaled.beforeSlots, 4);
  assert.equal(scaled.afterSlots, 5);
  assert.deepEqual(scaleCalls, ['provision:development-route-5', 'grant:development-display-5']);

  let failedScaleObservation = readyObservation;
  const failedScale = scaleOutDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      reclaimCapability: () => ({ ready: true }),
      pressureAdmission: () => ({ admittedMaximum: 6, reasons: [] }),
      observe: () => failedScaleObservation,
      provisionRoute: () => { failedScaleObservation = scaledObservation; },
      grantDisplayAccess: () => { throw new Error('fixture grant failure'); },
    },
  });
  assert.equal(failedScale.state, 'quarantined');
  assert.equal(failedScale.success, false);
  assert.equal(failedScale.reason, 'provision_failed');
  assert.equal(failedScale.routeId, 'development-route-5');
  assert.equal(failedScale.afterSlots, 5);
  assert.match(failedScale.error, /fixture grant failure/);

  let pressureProvisioned = false;
  const pressureDeferred = scaleOutDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      reclaimCapability: () => ({ ready: true }),
      pressureAdmission: () => ({ admittedMaximum: 4, reasons: ['memory_reserve'] }),
      observe: () => readyObservation,
      provisionRoute: () => { pressureProvisioned = true; },
    },
  });
  assert.equal(pressureDeferred.state, 'deferred');
  assert.equal(pressureDeferred.reason, 'pressure_admission');
  assert.equal(pressureProvisioned, false);

  let unsafeProvisioned = false;
  const reclaimCapabilityDeferred = scaleOutDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      reclaimCapability: () => ({ ready: false, reason: 'helper_contract_missing' }),
      pressureAdmission: () => ({ admittedMaximum: 6, reasons: [] }),
      observe: () => readyObservation,
      provisionRoute: () => { unsafeProvisioned = true; },
    },
  });
  assert.equal(reclaimCapabilityDeferred.state, 'deferred');
  assert.equal(reclaimCapabilityDeferred.reason, 'reclaim_capability_unavailable');
  assert.equal(unsafeProvisioned, false);

  const reclaimCalls = [];
  let reclaimObservation = scaledObservation;
  const reclaimed = scaleInDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      reclaimCapability: () => ({ ready: true }),
      observe: () => reclaimObservation,
      referenceCheck: (route) => ({
        routeId: route.routeId,
        blockers: [],
        ambiguities: [],
      }),
      reclaimRoute: (route) => {
        reclaimCalls.push(`reclaim:${route.routeId}`);
        reclaimObservation = readyObservation;
      },
    },
  });
  assert.equal(reclaimed.state, 'reclaimed');
  assert.equal(reclaimed.routeId, 'development-route-5');
  assert.equal(reclaimed.beforeSlots, 5);
  assert.equal(reclaimed.afterSlots, 4);
  assert.deepEqual(reclaimCalls, ['reclaim:development-route-5']);

  let blockedReclaim = false;
  const referenceDeferred = scaleInDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      reclaimCapability: () => ({ ready: true }),
      observe: () => scaledObservation,
      referenceCheck: (route) => ({
        routeId: route.routeId,
        blockers: ['viewer_lease:fixture'],
        ambiguities: [],
      }),
      reclaimRoute: () => { blockedReclaim = true; },
    },
  });
  assert.equal(referenceDeferred.state, 'deferred');
  assert.equal(referenceDeferred.reason, 'referenced');
  assert.equal(blockedReclaim, false);

  const referenceQuarantined = scaleInDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      reclaimCapability: () => ({ ready: true }),
      observe: () => scaledObservation,
      referenceCheck: (route) => ({
        routeId: route.routeId,
        blockers: [],
        ambiguities: ['service_binding_missing'],
      }),
      reclaimRoute: () => assert.fail('ambiguous route must not be reclaimed'),
    },
  });
  assert.equal(referenceQuarantined.state, 'quarantined');
  assert.equal(referenceQuarantined.reason, 'reference_ambiguity');

  const cooldownDeferred = scaleInDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      observe: () => scaledObservation,
      cooldownStatus: () => ({ ready: false, elapsedMs: 100, requiredMs: 5000 }),
      referenceCheck: () => assert.fail('cooldown must defer before reference qualification'),
      reclaimRoute: () => assert.fail('cooldown route must not be reclaimed'),
    },
  });
  assert.equal(cooldownDeferred.state, 'deferred');
  assert.equal(cooldownDeferred.reason, 'cooldown_not_elapsed');

  const reclaimFailed = scaleInDevelopmentPresentation({
    env,
    effects: {
      snapshotProduction: () => ({ identity: 'production-fixture' }),
      assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
      reclaimCapability: () => ({ ready: true }),
      observe: () => scaledObservation,
      referenceCheck: (route) => ({ routeId: route.routeId, blockers: [], ambiguities: [] }),
      reclaimRoute: () => { throw new Error('fixture termination failure'); },
    },
  });
  assert.equal(reclaimFailed.state, 'quarantined');
  assert.equal(reclaimFailed.success, false);
  assert.equal(reclaimFailed.reason, 'reclaim_failed');
  assert.equal(reclaimFailed.routeId, 'development-route-5');
  assert.match(reclaimFailed.error, /fixture termination failure/);

  const referencedRoute = descriptor.routes[4];
  let convergingProcessChecks = 0;
  const convergingReclaimEffects = createDevelopmentPresentationLifecycleSystemEffects({
    env,
    reclaimTimeoutMs: 20,
    reclaimPollMs: 1,
    productionSnapshot: () => ({ identity: 'production-fixture' }),
    assertProductionUnchanged: () => {},
    run(command, args) {
      if (command.endsWith('/agent-browser-privileged-helper') && args[0] === 'status-json') {
        return {
          status: 0,
          stdout: JSON.stringify({
            helperVersion: 'fixture-v5',
            routeSessionTermination: {
              supported: true,
              exactRouteUser: true,
              idempotentWhenAbsent: true,
            },
          }),
          stderr: '',
        };
      }
      if (command.endsWith('/agent-browser-dev') && args.at(-1) === 'close') {
        return { status: 0, stdout: '{"success":true}', stderr: '' };
      }
      if (command === 'sudo') {
        return { status: 1, stdout: '', stderr: 'route user processes remain after termination' };
      }
      if (command === 'ps') {
        convergingProcessChecks += 1;
        return convergingProcessChecks === 1
          ? { status: 0, stdout: '4242 Xorg :16\n', stderr: '' }
          : { status: 1, stdout: '', stderr: '' };
      }
      throw new Error(`Unexpected converging reclaim command: ${command} ${args.join(' ')}`);
    },
  });
  assert.doesNotThrow(() => convergingReclaimEffects.reclaimRoute(referencedRoute, descriptor));
  assert.equal(convergingProcessChecks, 2);
  const serviceState = {
    remoteViewRoutes: {
      [referencedRoute.routeId]: {
        browserId: null,
        sessionId: null,
        viewerLeaseIds: [],
        controllerLeaseId: null,
      },
    },
    displayAllocations: {
      [referencedRoute.displayReservationId]: {
        ownerBrowserId: null,
        ownerSessionId: null,
      },
    },
    routePool: { [referencedRoute.slotId]: { id: referencedRoute.slotId } },
    remoteViewAcquisitionLeases: {},
    viewerLeases: {
      'viewer-exact': { routeId: referencedRoute.routeId },
      'viewer-other': { routeId: `${referencedRoute.routeId}0` },
    },
    remoteViewHandoffs: {
      'handoff-exact': { binding: { routeId: referencedRoute.routeId } },
      'handoff-other': { binding: { routeId: `${referencedRoute.routeId}0` } },
    },
    presentationCapacity: {
      slots: [{ id: `slot:${referencedRoute.slotId}`, restorationPending: false }],
    },
  };
  const referenceEffects = createDevelopmentPresentationLifecycleSystemEffects({
    env,
    productionSnapshot: () => ({ identity: 'production-fixture' }),
    assertProductionUnchanged: () => {},
    run(command, args) {
      if (command.endsWith('/agent-browser-privileged-helper') && args[0] === 'status-json') {
        return {
          status: 0,
          stdout: JSON.stringify({ helperVersion: 'fixture-v4' }),
          stderr: '',
        };
      }
      if (command.endsWith('/agent-browser-dev') && args.at(-2) === 'service') {
        return {
          status: 0,
          stdout: JSON.stringify({ data: { service_state: serviceState } }),
          stderr: '',
        };
      }
      throw new Error(`Unexpected reference command: ${command} ${args.join(' ')}`);
    },
  });
  assert.deepEqual(referenceEffects.reclaimCapability(), {
    ready: false,
    reason: 'helper_contract_missing',
    helper: '/usr/local/libexec/agent-browser/agent-browser-privileged-helper',
    helperVersion: 'fixture-v4',
  });
  assert.deepEqual(referenceEffects.referenceCheck(referencedRoute, descriptor), {
    routeId: referencedRoute.routeId,
    blockers: ['handoff:handoff-exact', 'viewer_lease:viewer-exact'],
    ambiguities: [],
  });

  rmSync(descriptor.manifest, { force: true });
  let quarantined = null;
  assert.throws(
    () => applyDevelopmentPresentationProvider({
      env,
      authorizeEffects: true,
      effects: {
        snapshotProduction: () => ({ identity: 'production-fixture' }),
        assertProductionUnchanged: (before, after) => assert.deepEqual(after, before),
        createVolume: () => {},
        startDatabase: () => {},
        ensureRouteUser: () => {},
        syncConnections: () => { throw new Error('fixture connection failure'); },
        startProvider: () => {},
        grantOperatorRouteAccess: () => {},
        openWarmRoutes: () => {},
        observe: () => readyObservation,
        grantDisplayAccess: () => {},
        publishIngress: () => {},
        quarantine: (receipt) => { quarantined = receipt; },
      },
    }),
    /apply quarantined: fixture connection failure/,
  );
  assert.equal(existsSync(descriptor.manifest), false);
  assert.equal(quarantined.reason, 'fixture connection failure');
  assert.deepEqual(quarantined.completedSteps.slice(0, 2), ['create-volume', 'start-database']);

  const drifted = developmentPresentationProviderManifest(descriptor);
  drifted.ports.guacamole = 8092;
  writeFileSync(descriptor.manifest, `${JSON.stringify(drifted, null, 2)}\n`);
  const drift = doctorDevelopmentPresentationProvider({ env, probe: () => readyObservation });
  assert.equal(drift.success, false);
  assert.equal(drift.status.state, 'drifted');
  assert.equal(drift.status.ready, false);

  console.log('Development presentation provider fixture passed');
} finally {
  rmSync(fixture, { recursive: true, force: true });
}
