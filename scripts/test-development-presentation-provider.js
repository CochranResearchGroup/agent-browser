#!/usr/bin/env node

import assert from 'node:assert/strict';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
  developmentPresentationProviderDescriptor,
  developmentPresentationProviderManifest,
  developmentAgentSkillStatus,
  doctorDevelopmentPresentationProvider,
  synchronizeDevelopmentAgentSkill,
  validateDevelopmentPresentationProviderIsolation,
} from './lib/development-presentation-provider.js';
import {
  DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA,
  applyDevelopmentPresentationProvider,
  developmentPresentationProviderDeploymentPlan,
  probeDevelopmentPresentationProvider,
  prepareDevelopmentPresentationProviderSecrets,
  renderDevelopmentPresentationProviderBundle,
  stageDevelopmentPresentationProviderBundle,
} from './lib/development-presentation-provider-deployment.js';
import { developmentPresentationProviderSystemPreflight } from './lib/development-presentation-provider-system-effects.js';

const fixture = mkdtempSync(join(tmpdir(), 'agent-browser-dev-provider-'));
const userHome = join(fixture, 'user');
const env = { ...process.env, AGENT_BROWSER_DEV_USER_HOME: userHome };

try {
  const descriptor = developmentPresentationProviderDescriptor(env);
  const routeOpenerSource = readFileSync('scripts/open-rdp-guac-route-displays.js', 'utf8');
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'set'/);
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'open'/);
  assert.match(routeOpenerSource, /'--profile',\s*profile,\s*'close'/);
  assert.match(routeOpenerSource, /AGENT_BROWSER_ROUTE_DISPLAY_FORCE_VIEWER/);
  assert.match(routeOpenerSource, /profile\.includes\('\/'\)/);
  assert.equal(descriptor.environment, 'development');
  assert.equal(descriptor.warmSlots, 4);
  assert.equal(descriptor.hardMaxSlots, 6);
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
          return { status: 0, stdout: '5\n', stderr: '' };
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
          stdout: descriptor.routes.slice(0, descriptor.warmSlots)
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
  assert.equal(probed.displays.length, 4);
  assert.equal(probed.secrets.private, true);
  const configured = doctorDevelopmentPresentationProvider({ env, probe: () => readyObservation });
  assert.equal(configured.success, true);
  assert.equal(configured.status.state, 'configured');
  assert.equal(configured.status.ready, true);
  assert.equal(configured.status.manifest.schemaVersion, DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA);

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
      openWarmRoutes: () => effectCalls.push('open-warm-routes'),
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
