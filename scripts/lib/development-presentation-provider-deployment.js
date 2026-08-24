import { spawnSync } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import {
  developmentPresentationProviderManifest,
  developmentPresentationProviderDescriptor,
  evaluateDevelopmentPresentationProviderObservation,
  validateDevelopmentPresentationProviderIsolation,
} from './development-presentation-provider.js';

export const DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA =
  'agent-browser.development-presentation-provider-deployment.v1';

/**
 * Describes the ordered effect boundary without executing it. Every effect is
 * development-owned and the final ingress step remains gated on provider
 * readiness from current resources rather than bundle presence.
 */
export function developmentPresentationProviderDeploymentPlan(descriptor) {
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA,
    environment: 'development',
    authorizesEffects: false,
    requiresExplicitApply: true,
    productionPosture: 'read_only',
    providerRoot: descriptor.root,
    steps: [
      step('stage-bundle', false),
      step('create-volume', false),
      step('start-database', false),
      step('verify-schema', false),
      step('ensure-route-users', true),
      step('sync-connections', false),
      step('start-provider', false),
      step('grant-operator-route-access', false),
      step('open-warm-route-sessions', false),
      step('bind-live-displays', false),
      step('grant-display-access', true),
      step('publish-provider-manifest', false),
      step('publish-ingress', false),
    ],
    rollback: {
      exactCreatedResourcesOnly: true,
      ambiguousResources: 'quarantine',
      productionFallback: false,
    },
  };
}

/** Render a secret-free, reviewable provider bundle from one descriptor. */
export function renderDevelopmentPresentationProviderBundle(descriptor) {
  const routeUsers = descriptor.routes.map((route) => ({
    id: route.routeId,
    connectionName: route.connectionName,
    legacyConnectionName: '',
    routeUser: route.user,
  }));
  const ingress = {
    service: 'agent-browser-dev',
    pathPrefix: '/guacamole',
    upstream: `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole`,
    publishAfter: 'provider-ready',
  };
  const desired = {
    schemaVersion: DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA,
    environment: 'development',
    descriptor: {
      root: descriptor.root,
      composeProject: descriptor.composeProject,
      services: descriptor.services,
      database: descriptor.database,
      ports: descriptor.ports,
      rdpTarget: descriptor.rdpTarget,
      warmSlots: descriptor.warmSlots,
      hardMaxSlots: descriptor.hardMaxSlots,
      routes: descriptor.routes,
    },
    ingress,
  };
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_DEPLOYMENT_SCHEMA,
    environment: 'development',
    routeUsers,
    ingress,
    files: {
      'compose.yml': renderCompose(descriptor),
      '.env': renderEnvironment(descriptor),
      'route-users.json': `${JSON.stringify(routeUsers, null, 2)}\n`,
      'desired-provider.json': `${JSON.stringify(desired, null, 2)}\n`,
      'ingress.json': `${JSON.stringify(ingress, null, 2)}\n`,
    },
  };
}

/**
 * Stages only reviewable, secret-free development assets. It does not create
 * users, containers, database records, displays, manifests, or ingress.
 */
export function stageDevelopmentPresentationProviderBundle({ env = process.env } = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  validateDevelopmentPresentationProviderIsolation(descriptor);
  if (existsSync(descriptor.manifest)) {
    throw new Error('Configured development provider requires reconciled update, not bundle staging');
  }
  const sourceRoot = resolve(
    env.AGENT_BROWSER_DEV_GUACAMOLE_ASSET_SOURCE || 'cli/assets/workstation/guacamole',
  );
  for (const required of [
    'init/001-initdb.sql',
    'extensions/guac-manifest.json',
    'extensions/agent-browser-defaults.js',
    'start-guacamole.sh',
  ]) {
    if (!existsSync(join(sourceRoot, required))) {
      throw new Error(`Development Guacamole asset is unavailable: ${join(sourceRoot, required)}`);
    }
  }
  const bundle = renderDevelopmentPresentationProviderBundle(descriptor);
  mkdirSync(descriptor.root, { recursive: true, mode: 0o755 });
  chmodSync(descriptor.root, 0o755);
  for (const [relativePath, content] of Object.entries(bundle.files)) {
    const destination = join(descriptor.root, relativePath);
    mkdirSync(dirname(destination), { recursive: true, mode: 0o755 });
    chmodSync(dirname(destination), 0o755);
    writeFileAtomic(destination, content, relativePath === '.env' ? 0o600 : 0o644);
  }
  for (const relativePath of [
    'init/001-initdb.sql',
    'extensions/guac-manifest.json',
    'extensions/agent-browser-defaults.js',
    'start-guacamole.sh',
  ]) {
    const destination = join(descriptor.root, relativePath);
    mkdirSync(dirname(destination), { recursive: true, mode: 0o755 });
    chmodSync(dirname(destination), 0o755);
    copyFileAtomic(join(sourceRoot, relativePath), destination, relativePath.endsWith('.sh') ? 0o755 : 0o644);
  }
  const files = [
    ...Object.keys(bundle.files),
    'init/001-initdb.sql',
    'extensions/guac-manifest.json',
    'extensions/agent-browser-defaults.js',
    'start-guacamole.sh',
  ];
  return {
    success: true,
    environment: 'development',
    authorizesProviderEffects: false,
    root: descriptor.root,
    files: Object.fromEntries(files.map((relativePath) => [
      relativePath,
      sha256(join(descriptor.root, relativePath)),
    ])),
  };
}

/** Create development-only provider credentials once without returning them. */
export function prepareDevelopmentPresentationProviderSecrets({ env = process.env } = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  validateDevelopmentPresentationProviderIsolation(descriptor);
  const path = join(descriptor.secretsDir, 'provider.env');
  if (existsSync(path)) {
    assertDevelopmentSecrets(path, descriptor);
    chmodSync(path, 0o600);
    return { success: true, environment: 'development', created: false, path, sha256: sha256(path) };
  }
  const routes = descriptor.routes.map((route) => ({
    id: route.routeId,
    connectionName: route.connectionName,
    legacyConnectionName: '',
    routeUser: route.user,
    password: randomSecret(),
  }));
  const content = [
    `POSTGRES_PASSWORD=${randomSecret()}`,
    `XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON=${JSON.stringify(routes)}`,
    '',
  ].join('\n');
  mkdirSync(descriptor.secretsDir, { recursive: true, mode: 0o700 });
  writeFileAtomic(path, content, 0o600);
  return { success: true, environment: 'development', created: true, path, sha256: sha256(path) };
}

/**
 * Executes the reviewed development provider transaction through injected
 * effect adapters. No production fallback exists, and failures are handed to
 * exact-resource quarantine instead of broad cleanup.
 */
export function applyDevelopmentPresentationProvider({
  env = process.env,
  authorizeEffects = false,
  deferIngress = false,
  effects,
} = {}) {
  if (!authorizeEffects) {
    throw new Error('Development presentation provider apply requires explicit effect authority');
  }
  if (!effects) throw new Error('Development presentation provider effect adapter is required');
  const descriptor = developmentPresentationProviderDescriptor(env);
  validateDevelopmentPresentationProviderIsolation(descriptor);
  if (existsSync(descriptor.manifest)) {
    const manifest = JSON.parse(readFileSync(descriptor.manifest, 'utf8'));
    const expected = developmentPresentationProviderManifest(descriptor);
    if (JSON.stringify(manifest) !== JSON.stringify(expected)) {
      throw new Error('Configured development provider manifest drifted');
    }
    const productionBefore = effects.snapshotProduction();
    const observation = effects.observe(descriptor);
    const readinessChecks = evaluateDevelopmentPresentationProviderObservation(
      descriptor,
      observation,
    );
    assertChecks(readinessChecks, 'Configured development provider reconciliation failed');
    writeProviderAuthority(descriptor, observation);
    const completedSteps = ['reconcile-provider-authority'];
    if (!deferIngress) {
      effects.publishIngress(descriptor, observation);
      completedSteps.push('publish-ingress');
    }
    const productionAfter = effects.snapshotProduction();
    effects.assertProductionUnchanged(productionBefore, productionAfter);
    const state = deferIngress ? 'provider_ready_ingress_pending' : 'applied';
    const receipt = writeDeploymentReceipt(descriptor, {
      state,
      startedAt: new Date().toISOString(),
      completedAt: new Date().toISOString(),
      completedSteps,
      productionUnchanged: true,
      readinessChecks,
    });
    return {
      success: true,
      environment: 'development',
      state,
      providerReady: true,
      ingressPublished: !deferIngress,
      productionUnchanged: true,
      completedSteps,
      receipt,
    };
  }
  stageDevelopmentPresentationProviderBundle({ env });
  prepareDevelopmentPresentationProviderSecrets({ env });
  const routeSecrets = loadDevelopmentRouteSecrets(descriptor);
  const productionBefore = effects.snapshotProduction();
  const startedAt = new Date().toISOString();
  const completedSteps = [];
  try {
    effects.createVolume(descriptor);
    completedSteps.push('create-volume');
    effects.startDatabase(descriptor);
    completedSteps.push('start-database');
    for (const route of descriptor.routes) {
      const secret = routeSecrets.find((candidate) => candidate.id === route.routeId);
      effects.ensureRouteUser(route, secret.password, descriptor);
      completedSteps.push(`ensure-user:${route.routeId}`);
    }
    effects.syncConnections(descriptor, routeSecrets);
    completedSteps.push('sync-connections');
    effects.startProvider(descriptor);
    completedSteps.push('start-provider');
    effects.grantOperatorRouteAccess(descriptor);
    completedSteps.push('grant-operator-route-access');
    effects.openWarmRoutes(descriptor);
    completedSteps.push('open-warm-routes');
    const stagedObservation = effects.observe(descriptor);
    const stagedChecks = evaluateDevelopmentPresentationProviderObservation(
      descriptor,
      stagedObservation,
    );
    assertChecks(stagedChecks, 'Development provider did not become capture-ready');
    for (const display of stagedObservation.displays) {
      effects.grantDisplayAccess(display, descriptor);
      completedSteps.push(`grant:${display.displayReservationId}`);
    }
    const observation = effects.observe(descriptor);
    const readinessChecks = evaluateDevelopmentPresentationProviderObservation(
      descriptor,
      observation,
    );
    assertChecks(readinessChecks, 'Development provider readiness failed after display access');
    writeProviderAuthority(descriptor, observation);
    completedSteps.push('publish-provider-manifest');
    if (deferIngress) {
      const productionAfter = effects.snapshotProduction();
      effects.assertProductionUnchanged(productionBefore, productionAfter);
      const receipt = writeDeploymentReceipt(descriptor, {
        state: 'provider_ready_ingress_pending',
        startedAt,
        completedAt: new Date().toISOString(),
        completedSteps,
        productionUnchanged: true,
        readinessChecks,
      });
      return {
        success: true,
        environment: 'development',
        state: 'provider_ready_ingress_pending',
        providerReady: true,
        ingressPublished: false,
        productionUnchanged: true,
        completedSteps,
        receipt,
      };
    }
    effects.publishIngress(descriptor, observation);
    completedSteps.push('publish-ingress');
    const productionAfter = effects.snapshotProduction();
    effects.assertProductionUnchanged(productionBefore, productionAfter);
    const receipt = writeDeploymentReceipt(descriptor, {
      state: 'applied',
      startedAt,
      completedAt: new Date().toISOString(),
      completedSteps,
      productionUnchanged: true,
      readinessChecks,
    });
    return {
      success: true,
      environment: 'development',
      state: 'applied',
      providerReady: true,
      ingressPublished: true,
      productionUnchanged: true,
      completedSteps,
      receipt,
    };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    let productionUnchanged = false;
    try {
      const productionAfter = effects.snapshotProduction();
      effects.assertProductionUnchanged(productionBefore, productionAfter);
      productionUnchanged = true;
    } catch {
      productionUnchanged = false;
    }
    effects.quarantine({ descriptor, completedSteps, reason: message });
    const receipt = writeDeploymentReceipt(descriptor, {
      state: 'quarantined',
      startedAt,
      completedAt: new Date().toISOString(),
      completedSteps,
      productionUnchanged,
      error: message,
    });
    const failure = new Error(`Development provider apply quarantined: ${message}`);
    failure.receipt = receipt;
    throw failure;
  }
}

/** Read current provider resources without changing them or exposing secrets. */
export function probeDevelopmentPresentationProvider(
  descriptor,
  { run = commandResult, displaySocketExists = displaySocketReady } = {},
) {
  const containers = Object.values(descriptor.services).map((name) => {
    const result = run('docker', [
      'inspect',
      '-f',
      '{{.State.Running}}\t{{index .Config.Labels "com.docker.compose.project"}}',
      name,
    ]);
    const [running, composeProject] = result.status === 0
      ? result.stdout.trim().split('\t')
      : [null, null];
    return {
      name,
      running: running === 'true',
      composeProject: composeProject || null,
      error: result.status === 0 ? null : commandError(result),
    };
  });
  const ports = Object.fromEntries(Object.entries(descriptor.ports).map(([name, value]) => {
    const result = run('ss', ['-H', '-ltn', `sport = :${value}`]);
    return [name, {
      port: value,
      listening: result.status === 0 && Boolean(result.stdout.trim()),
      error: result.status === 0 ? null : commandError(result),
    }];
  }));
  const routeUsers = descriptor.routes.map((route) => {
    const result = run('getent', ['passwd', route.user]);
    return { user: route.user, exists: result.status === 0 && Boolean(result.stdout.trim()) };
  });
  const schemaSql = `select count(*) from information_schema.tables
where table_schema = 'public' and table_name in
('guacamole_user','guacamole_entity','guacamole_connection',
 'guacamole_connection_parameter','guacamole_connection_permission');`;
  const schemaResult = postgresQuery(descriptor, schemaSql, run);
  const routeSql = `select coalesce(json_agg(row_to_json(t)), '[]'::json)
from (
  select c.connection_id::text as "connectionId",
         c.connection_name as "connectionName",
         max(case when p.parameter_name = 'username' then p.parameter_value end) as "user"
  from guacamole_connection c
  left join guacamole_connection_parameter p on p.connection_id = c.connection_id
  where c.connection_name like 'Agent Browser Dev RDP Route %'
  group by c.connection_id, c.connection_name
  order by c.connection_id
) t;`;
  const routesResult = postgresQuery(descriptor, routeSql, run);
  let databaseRoutes = [];
  try {
    databaseRoutes = routesResult.status === 0
      ? JSON.parse(routesResult.stdout.trim() || '[]')
      : [];
  } catch {
    databaseRoutes = [];
  }
  const processResult = run('ps', ['-eo', 'user:64=,args=']);
  const processLines = processResult.status === 0 ? processResult.stdout.split(/\r?\n/) : [];
  const displays = [];
  for (const route of descriptor.routes.slice(0, descriptor.warmSlots)) {
    const line = processLines.find((candidate) =>
      candidate.trimStart().startsWith(`${route.user} `) &&
      /(?:^|[\/\s])Xorg\s+:[0-9]+(?:\s|$)/.test(candidate),
    );
    const match = line?.match(/(?:^|[\/\s])Xorg\s+:([0-9]+)(?:\s|$)/);
    const displayName = match ? `:${match[1]}` : null;
    if (displayName) {
      displays.push({
        displayReservationId: route.displayReservationId,
        displayName,
        user: route.user,
        ready: displaySocketExists(displayName),
      });
    }
  }
  const secretPath = join(descriptor.secretsDir, 'provider.env');
  let privateSecret = false;
  try {
    const status = statSync(secretPath);
    privateSecret = status.isFile() && (status.mode & 0o077) === 0;
  } catch {
    privateSecret = false;
  }
  return {
    environment: 'development',
    containers,
    ports,
    routeUsers,
    database: {
      schemaReady: schemaResult.status === 0 && Number(schemaResult.stdout.trim()) === 5,
      routes: databaseRoutes,
      error: schemaResult.status === 0 && routesResult.status === 0
        ? null
        : commandError(schemaResult.status === 0 ? routesResult : schemaResult),
    },
    displays,
    secrets: { path: secretPath, private: privateSecret },
  };
}

function step(id, privileged) {
  return { id, privileged, state: 'planned' };
}

function renderEnvironment(descriptor) {
  return [
    `COMPOSE_PROJECT_NAME=${descriptor.composeProject}`,
    `AGENT_BROWSER_DEV_GUACAMOLE_PORT=${descriptor.ports.guacamole}`,
    `AGENT_BROWSER_DEV_GUACD_PORT=${descriptor.ports.guacd}`,
    `AGENT_BROWSER_DEV_POSTGRES_PORT=${descriptor.ports.postgres}`,
    `POSTGRES_DB=${descriptor.database.name}`,
    `POSTGRES_USER=${descriptor.database.user}`,
    '',
  ].join('\n');
}

function renderCompose(descriptor) {
  const volume = 'agent-browser-dev-guacamole-postgres-data';
  const postgresDb = '${POSTGRES_DB:?set POSTGRES_DB in .env}';
  const postgresUser = '${POSTGRES_USER:?set POSTGRES_USER in .env}';
  const postgresPassword =
    '${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD in the protected secrets file}';
  return `name: ${descriptor.composeProject}

services:
  postgres:
    image: postgres:16-alpine@sha256:20edbde7749f822887a1a022ad526fde0a47d6b2be9a8364433605cf65099416
    platform: linux/amd64
    container_name: ${descriptor.services.postgres}
    restart: unless-stopped
    stop_grace_period: 30s
    command: [postgres, -c, fsync=on, -c, synchronous_commit=on, -c, full_page_writes=on]
    ports:
      - 127.0.0.1:${descriptor.ports.postgres}:5432
    environment:
      POSTGRES_DB: "${postgresDb}"
      POSTGRES_USER: "${postgresUser}"
      POSTGRES_PASSWORD: "${postgresPassword}"
    volumes:
      - ${volume}:/var/lib/postgresql/data
      - ./init:/docker-entrypoint-initdb.d:ro
    healthcheck:
      test:
        - CMD-SHELL
        - "pg_isready -U ${postgresUser} -d ${postgresDb}"
      interval: 5s
      timeout: 5s
      retries: 20
      start_period: 10s

  guacd:
    image: guacamole/guacd:1.5.5@sha256:38232cae271361ef53db46faf5c49fe64049a1320a05b82c597425b69d6ce77e
    platform: linux/amd64
    container_name: ${descriptor.services.guacd}
    restart: unless-stopped
    ports:
      - 127.0.0.1:${descriptor.ports.guacd}:4822
    extra_hosts:
      - host.docker.internal:host-gateway
    healthcheck:
      test: [CMD-SHELL, nc -z 127.0.0.1 4822 || exit 1]
      interval: 5s
      timeout: 5s
      retries: 20
      start_period: 5s

  guacamole:
    image: guacamole/guacamole:1.5.5@sha256:0f62f6d17ab379e46aa66874b2ff564dab856a6ef5e754a69cbb34c32d3e588a
    platform: linux/amd64
    container_name: ${descriptor.services.guacamole}
    restart: unless-stopped
    entrypoint: [/bin/bash, /opt/agent-browser/start-guacamole.sh]
    depends_on:
      postgres:
        condition: service_healthy
      guacd:
        condition: service_healthy
    ports:
      - 127.0.0.1:${descriptor.ports.guacamole}:8080
    environment:
      GUACAMOLE_HOME: /etc/guacamole
      GUACD_HOSTNAME: guacd
      POSTGRESQL_HOSTNAME: postgres
      POSTGRESQL_DATABASE: "${postgresDb}"
      POSTGRESQL_USER: "${postgresUser}"
      POSTGRESQL_PASSWORD: "${postgresPassword}"
      POSTGRESQL_AUTO_CREATE_ACCOUNTS: "true"
      HEADER_ENABLED: "true"
      HTTP_AUTH_HEADER: Remote-User
    volumes:
      - ./extensions:/etc/guacamole/extensions:ro
      - ./start-guacamole.sh:/opt/agent-browser/start-guacamole.sh:ro

volumes:
  ${volume}:
    name: ${volume}
`;
}

function writeFileAtomic(path, content, mode) {
  const temporary = `${path}.next-${process.pid}`;
  writeFileSync(temporary, content, { mode });
  renameSync(temporary, path);
}

function copyFileAtomic(source, destination, mode) {
  const temporary = `${destination}.next-${process.pid}`;
  copyFileSync(source, temporary);
  chmodSync(temporary, mode);
  renameSync(temporary, destination);
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function commandResult(command, args) {
  return spawnSync(command, args, { encoding: 'utf8', stdio: 'pipe' });
}

function postgresQuery(descriptor, sql, run) {
  return run('docker', [
    'exec',
    descriptor.services.postgres,
    'psql',
    '-U',
    descriptor.database.user,
    '-d',
    descriptor.database.name,
    '-t',
    '-A',
    '-c',
    sql,
  ]);
}

function commandError(result) {
  return (result.stderr || result.stdout || result.error?.message || 'command failed').trim();
}

function displaySocketReady(displayName) {
  const number = displayName.replace(/^:/, '');
  if (existsSync(`/tmp/.X11-unix/X${number}`)) return true;
  try {
    return readFileSync('/proc/net/unix', 'utf8').includes(`@/tmp/.X11-unix/X${number}`);
  } catch {
    return false;
  }
}

function assertDevelopmentSecrets(path, descriptor) {
  const values = parseEnvironment(readFileSync(path, 'utf8'));
  if (!values.POSTGRES_PASSWORD) throw new Error('Development provider PostgreSQL secret is missing');
  let routes;
  try {
    routes = JSON.parse(values.XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON || 'null');
  } catch {
    throw new Error('Development provider route-user secret is invalid');
  }
  if (!Array.isArray(routes) || routes.length !== descriptor.routes.length) {
    throw new Error('Development provider route-user secret count drifted');
  }
  for (const route of descriptor.routes) {
    const secret = routes.find((candidate) => candidate.id === route.routeId);
    if (secret?.routeUser !== route.user || secret?.connectionName !== route.connectionName || !secret?.password) {
      throw new Error(`Development provider route-user secret drifted: ${route.routeId}`);
    }
  }
}

function parseEnvironment(content) {
  return Object.fromEntries(content.split(/\r?\n/).flatMap((line) => {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#') || !trimmed.includes('=')) return [];
    const index = trimmed.indexOf('=');
    return [[trimmed.slice(0, index), trimmed.slice(index + 1)]];
  }));
}

function randomSecret() {
  return randomBytes(32).toString('base64url');
}

function loadDevelopmentRouteSecrets(descriptor) {
  const path = join(descriptor.secretsDir, 'provider.env');
  assertDevelopmentSecrets(path, descriptor);
  return JSON.parse(
    parseEnvironment(readFileSync(path, 'utf8')).XRDP_AGENT_BROWSER_ROUTE_USER_POOL_JSON,
  );
}

function assertChecks(checks, message) {
  const failed = checks.filter((item) => !item.ok).map((item) => item.name);
  if (failed.length) throw new Error(`${message}: ${failed.join(', ')}`);
}

function writeProviderAuthority(descriptor, observation) {
  const databaseRoutes = new Map(
    observation.database.routes.map((route) => [route.connectionName, route]),
  );
  const displays = new Map(
    observation.displays.map((display) => [display.displayReservationId, display]),
  );
  const inventory = descriptor.routes.map((route) => {
    const connectionId = databaseRoutes.get(route.connectionName)?.connectionId || null;
    const clientId = connectionId
      ? Buffer.from(`${connectionId}\0c\0postgresql`, 'utf8').toString('base64')
      : null;
    return {
      ...route,
      connectionId,
      displayName: displays.get(route.displayReservationId)?.displayName || null,
      frameUrl: clientId
        ? `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole/#/client/${clientId}`
        : null,
      state: route.ordinal <= descriptor.warmSlots ? 'ready' : 'absent',
    };
  });
  mkdirSync(descriptor.stateDir, { recursive: true, mode: 0o700 });
  writeFileAtomic(descriptor.inventoryPath, `${JSON.stringify({
    schemaVersion: 'agent-browser.development-presentation-inventory.v1',
    environment: 'development',
    routes: inventory,
  }, null, 2)}\n`, 0o600);
  writeFileAtomic(
    descriptor.manifest,
    `${JSON.stringify(developmentPresentationProviderManifest(descriptor), null, 2)}\n`,
    0o600,
  );
}

function writeDeploymentReceipt(descriptor, receipt) {
  mkdirSync(descriptor.receiptsDir, { recursive: true, mode: 0o700 });
  const id = `${Date.now()}-${process.pid}`;
  const path = join(descriptor.receiptsDir, `apply-${id}.json`);
  writeFileAtomic(path, `${JSON.stringify({
    schemaVersion: 'agent-browser.development-presentation-provider-receipt.v1',
    environment: 'development',
    ...receipt,
  }, null, 2)}\n`, 0o600);
  return path;
}
