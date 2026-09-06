import { createHash } from 'node:crypto';
import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { isIP } from 'node:net';
import { dirname, join, relative, resolve } from 'node:path';
import { developmentRuntimeNamespace, requireNamespacedDevelopmentPorts } from './development-runtime-namespace.js';

export const DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA =
  'agent-browser.development-presentation-provider.v2';

const PRODUCTION_PORTS = new Set([3389, 3390, 4822, 5432, 4848, 4849, 8092]);

/**
 * Returns the complete development-owned presentation-provider identity.
 * Callers must consume this descriptor as a unit instead of composing ambient
 * production paths, ports, users, or route names into a development runtime.
 * AGENT_BROWSER_DEV_NAMESPACE gives parallel providers disjoint identities;
 * namespaced callers must explicitly reserve all seven development ports.
 */
export function developmentPresentationProviderDescriptor(env = process.env) {
  const { namespace, suffix, name } = developmentRuntimeNamespace(env);
  requireNamespacedDevelopmentPorts(env);
  const userHome = resolve(env.AGENT_BROWSER_DEV_USER_HOME || homedir());
  const pseudoHome = resolve(
    env.AGENT_BROWSER_DEV_HOME || join(userHome, '.local', 'share', name, 'home'),
  );
  const root = resolve(
    env.AGENT_BROWSER_DEV_PRESENTATION_ROOT ||
      join(userHome, '.local', 'share', name, 'presentation-provider'),
  );
  const warmSlots = positiveInteger(env.AGENT_BROWSER_DEV_PRESENTATION_WARM_SLOTS, 4);
  const hardMaxSlots = positiveInteger(env.AGENT_BROWSER_DEV_PRESENTATION_MAX_SLOTS, 6);
  const dashboardPort = port(env.AGENT_BROWSER_DEV_DASHBOARD_PORT, 4948);
  const localDiagnosticUrl = `http://127.0.0.1:${dashboardPort}`;
  const externalIngress = developmentExternalIngressBinding(env);
  if (hardMaxSlots < warmSlots) {
    throw new Error('Development presentation hard maximum must be at least the warm slot count');
  }
  const routes = Array.from({ length: hardMaxSlots }, (_, index) => {
    const ordinal = index + 1;
    const viewerProfile = `development${suffix}-presentation-provider-v5-${ordinal}`;
    return {
      ordinal,
      routeId: `development${suffix}-route-${ordinal}`,
      slotId: `development${suffix}-slot-${ordinal}`,
      user: `agent-browser-rdp-dev${suffix}-${ordinal}`,
      connectionKey: `${name}-connection-${ordinal}`,
      connectionId: null,
      connectionName: `Agent Browser Dev${namespace ? ` ${namespace}` : ''} RDP Route ${ordinal}`,
      displayReservationId: `development${suffix}-display-${ordinal}`,
      displayName: null,
      viewerSession: viewerProfile,
      viewerProfile,
      viewerProfilePath: join(
        pseudoHome,
        '.agent-browser',
        'runtime-profiles',
        viewerProfile,
        'user-data',
      ),
      lifecycle: ordinal <= warmSlots ? 'warm' : 'elastic',
    };
  });
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
    environment: 'development',
    ...(namespace ? { namespace } : {}),
    userHome,
    pseudoHome,
    root,
    manifest: join(root, 'provider.json'),
    secretsDir: join(root, 'secrets'),
    stateDir: join(root, 'state'),
    receiptsDir: join(root, 'receipts'),
    inventoryPath: join(root, 'state', 'route-inventory.json'),
    composeProject: `${name}-presentation`,
    services: {
      guacamole: `${name}-guacamole`,
      guacd: `${name}-guacd`,
      postgres: `${name}-guacamole-postgres`,
    },
    database: {
      name: `agent_browser_dev${namespace ? `_${namespace}` : ''}_guacamole`,
      user: `agent_browser_dev${namespace ? `_${namespace}` : ''}_guacamole`,
    },
    ports: {
      guacamole: port(env.AGENT_BROWSER_DEV_GUACAMOLE_PORT, 8093),
      guacd: port(env.AGENT_BROWSER_DEV_GUACD_PORT, 4823),
      postgres: port(env.AGENT_BROWSER_DEV_POSTGRES_PORT, 55433),
    },
    // Loopback remains useful for local diagnostics, but it is never an
    // operator handoff. A public operator origin exists only when both pieces
    // of reviewed external-ingress identity are explicitly configured.
    localDiagnosticUrl,
    publicOperatorUrl: externalIngress.publicOperatorUrl,
    externalIngress,
    rdpTarget: {
      host: env.AGENT_BROWSER_DEV_RDP_TARGET_HOST || 'host.docker.internal',
      port: port(env.AGENT_BROWSER_DEV_RDP_TARGET_PORT, 3389),
      isolation: 'route_user',
      sharedDaemon: true,
      restartAllowed: false,
    },
    connectionLimits: {
      maxConnections: 8,
      maxConnectionsPerUser: 8,
    },
    warmSlots,
    hardMaxSlots,
    routes,
    skill: {
      source: resolve(env.AGENT_BROWSER_DEV_SKILL_SOURCE || 'skills/agent-browser'),
      root: join(pseudoHome, '.codex', 'skills'),
      target: join(pseudoHome, '.codex', 'skills', 'agent-browser'),
    },
  };
}

/**
 * Bind a reviewed public HTTPS origin to its immutable ingress deployment
 * revision. Partial, local, private, credential-bearing, and path-scoped
 * configurations fail closed instead of silently falling back to loopback.
 */
export function developmentExternalIngressBinding(env = process.env) {
  const configuredUrl = env.AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL?.trim() || null;
  const reviewedRevision = env.AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION?.trim() || null;
  if (!configuredUrl && !reviewedRevision) {
    return {
      configured: false,
      publicOperatorUrl: null,
      reviewedRevision: null,
      bindingSha256: null,
    };
  }
  if (!configuredUrl || !reviewedRevision) {
    throw new Error(
      'Development external ingress requires both AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL and AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION',
    );
  }
  if (!/^[A-Za-z0-9][A-Za-z0-9._:-]{0,159}$/.test(reviewedRevision)) {
    throw new Error('Development external ingress revision is not a valid immutable revision identifier');
  }
  let parsed;
  try {
    parsed = new URL(configuredUrl);
  } catch {
    throw new Error('Development public operator URL is invalid');
  }
  if (
    parsed.protocol !== 'https:' ||
    parsed.username ||
    parsed.password ||
    parsed.search ||
    parsed.hash ||
    (parsed.pathname !== '/' && parsed.pathname !== '') ||
    !publicHostname(parsed.hostname)
  ) {
    throw new Error(
      'Development public operator URL must be a credential-free public HTTPS origin without a path, query, or fragment',
    );
  }
  const publicOperatorUrl = parsed.origin;
  const bindingDocument = {
    schemaVersion: 'agent-browser.development-external-ingress-binding.v1',
    publicOperatorUrl,
    reviewedRevision,
  };
  return {
    configured: true,
    publicOperatorUrl,
    reviewedRevision,
    bindingSha256: createHash('sha256')
      .update(JSON.stringify(bindingDocument))
      .digest('hex'),
  };
}

/** Rejects any descriptor that borrows a known production identity. */
export function validateDevelopmentPresentationProviderIsolation(
  descriptor,
  production = productionPresentationProjection({
    AGENT_BROWSER_DEV_USER_HOME: descriptor.userHome,
  }),
) {
  if (descriptor.environment !== 'development') {
    throw new Error('Development presentation provider must declare the development environment');
  }
  if (descriptor.namespace !== undefined) {
    const { namespace, suffix, name } = developmentRuntimeNamespace({
      AGENT_BROWSER_DEV_NAMESPACE: descriptor.namespace,
    });
    const defaultRoot = join(descriptor.userHome, '.local', 'share', 'agent-browser-dev');
    for (const path of [descriptor.pseudoHome, descriptor.root, descriptor.manifest,
      descriptor.secretsDir, descriptor.stateDir, descriptor.receiptsDir,
      descriptor.inventoryPath, descriptor.skill.root, descriptor.skill.target,
      ...descriptor.routes.map((route) => route.viewerProfilePath)]) {
      if (pathsOverlap(path, defaultRoot)) {
        throw new Error('Namespaced provider path overlaps default development resources');
      }
    }
    const expected = {
      composeProject: `${name}-presentation`,
      services: { guacamole: `${name}-guacamole`, guacd: `${name}-guacd`, postgres: `${name}-guacamole-postgres` },
      database: { name: `agent_browser_dev_${namespace}_guacamole`, user: `agent_browser_dev_${namespace}_guacamole` },
    };
    for (const key of Object.keys(expected)) {
      if (JSON.stringify(descriptor[key]) !== JSON.stringify(expected[key])) {
        throw new Error(`Provider ${key} does not match its namespace`);
      }
    }
    for (const route of descriptor.routes) {
      const ordinal = route.ordinal;
      const viewer = `development${suffix}-presentation-provider-v5-${ordinal}`;
      const identities = {
        routeId: `development${suffix}-route-${ordinal}`,
        slotId: `development${suffix}-slot-${ordinal}`,
        user: `agent-browser-rdp-dev${suffix}-${ordinal}`,
        connectionKey: `${name}-connection-${ordinal}`,
        connectionName: `Agent Browser Dev ${namespace} RDP Route ${ordinal}`,
        displayReservationId: `development${suffix}-display-${ordinal}`,
        viewerSession: viewer, viewerProfile: viewer,
        viewerProfilePath: join(descriptor.pseudoHome, '.agent-browser', 'runtime-profiles', viewer, 'user-data'),
      };
      if (!Number.isSafeInteger(ordinal) || ordinal < 1 || route.user.length > 32 ||
          Object.entries(identities).some(([key, value]) => route[key] !== value)) {
        throw new Error('Provider route identity does not match its namespace');
      }
    }
    if (Object.values(descriptor.ports).some((value) => [8093, 4823, 55433, 4948, 4949, 4950, 4951].includes(value))) {
      throw new Error('Namespaced provider port overlaps default development resources');
    }
  }
  if (!/^http:\/\/127\.0\.0\.1:\d+$/.test(descriptor.localDiagnosticUrl || '')) {
    throw new Error('Development local diagnostic URL must remain loopback-only');
  }
  if (descriptor.publicOperatorUrl !== descriptor.externalIngress?.publicOperatorUrl) {
    throw new Error('Development public operator URL is not bound to external-ingress identity');
  }
  if (descriptor.externalIngress?.configured) {
    const rebound = developmentExternalIngressBinding({
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: descriptor.externalIngress.publicOperatorUrl,
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: descriptor.externalIngress.reviewedRevision,
    });
    if (rebound.bindingSha256 !== descriptor.externalIngress.bindingSha256) {
      throw new Error('Development external-ingress revision binding is inconsistent');
    }
  }
  const providerPorts = Object.entries(descriptor.ports);
  const allDevelopmentPorts = providerPorts.map(([, value]) => value);
  if (new Set(allDevelopmentPorts).size !== allDevelopmentPorts.length) {
    throw new Error('Development presentation provider contains duplicate ports');
  }
  for (const [name, value] of providerPorts) {
    if (production.ports.includes(value)) {
      throw new Error(`Development ${name} port ${value} collides with production`);
    }
  }
  for (const candidate of [
    descriptor.root,
    descriptor.secretsDir,
    descriptor.stateDir,
    descriptor.receiptsDir,
    descriptor.inventoryPath,
    descriptor.skill.root,
    descriptor.skill.target,
    ...descriptor.routes.map((route) => route.viewerProfilePath),
  ]) {
    for (const productionPath of production.paths) {
      if (pathsOverlap(candidate, productionPath)) {
        throw new Error(`Development path ${candidate} overlaps production path ${productionPath}`);
      }
    }
  }
  const identities = [
    descriptor.composeProject,
    ...Object.values(descriptor.services),
    descriptor.database.name,
    descriptor.database.user,
    ...descriptor.routes.flatMap((route) => [
      route.routeId,
      route.slotId,
      route.user,
      route.connectionKey,
      route.connectionName,
      route.viewerSession,
      route.viewerProfile,
    ]),
  ];
  for (const identity of identities) {
    if (production.identities.includes(identity)) {
      throw new Error(`Development identity ${identity} collides with production`);
    }
  }
  assertUnique(descriptor.routes, 'routeId');
  assertUnique(descriptor.routes, 'slotId');
  assertUnique(descriptor.routes, 'user');
  assertUnique(descriptor.routes, 'connectionKey');
  assertUnique(descriptor.routes, 'connectionName');
  assertUnique(descriptor.routes, 'displayReservationId');
  assertUnique(descriptor.routes, 'viewerSession');
  assertUnique(descriptor.routes, 'viewerProfile');
  assertUnique(descriptor.routes, 'viewerProfilePath');
  if (descriptor.routes.length !== descriptor.hardMaxSlots) {
    throw new Error('Development route inventory must describe every admitted slot');
  }
  return descriptor;
}

export function developmentPresentationProviderManifest(descriptor) {
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
    environment: descriptor.environment,
    ...(descriptor.namespace ? { namespace: descriptor.namespace } : {}),
    root: descriptor.root,
    secretsDir: descriptor.secretsDir,
    stateDir: descriptor.stateDir,
    receiptsDir: descriptor.receiptsDir,
    inventoryPath: descriptor.inventoryPath,
    composeProject: descriptor.composeProject,
    services: descriptor.services,
    database: descriptor.database,
    ports: descriptor.ports,
    localDiagnosticUrl: descriptor.localDiagnosticUrl,
    publicOperatorUrl: descriptor.publicOperatorUrl,
    externalIngress: descriptor.externalIngress,
    rdpTarget: descriptor.rdpTarget,
    warmSlots: descriptor.warmSlots,
    hardMaxSlots: descriptor.hardMaxSlots,
    routes: descriptor.routes,
    skill: { target: descriptor.skill.target },
  };
}

export function developmentPresentationProviderManifestCompatible(manifest, expected) {
  return JSON.stringify(manifest) === JSON.stringify(expected);
}

/**
 * Admit only the additive v1 to v2 authority upgrade. The legacy manifest
 * called the loopback dashboard URL public; every other provider identity must
 * still match before an explicit apply may rewrite current v2 authority.
 */
export function developmentPresentationProviderManifestUpgradeCompatible(manifest, expected) {
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest)) return false;
  if (expected?.externalIngress?.configured !== true) return false;
  const legacyExpected = { ...expected };
  legacyExpected.schemaVersion = 'agent-browser.development-presentation-provider.v1';
  delete legacyExpected.localDiagnosticUrl;
  delete legacyExpected.externalIngress;
  legacyExpected.publicOperatorUrl = expected.localDiagnosticUrl;
  return JSON.stringify(manifest) === JSON.stringify(legacyExpected);
}

/**
 * Resolve the descriptor used by read-only status and doctor commands. A
 * configured v2 provider already owns a durable reviewed ingress binding, so
 * those commands may reuse it when the invoking shell supplies neither member
 * of the pair. Explicit, partial, invalid, or changed environment values never
 * fall back to stored authority and remain visible as configuration drift.
 */
function developmentPresentationProviderStatusDescriptor(env) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  if (
    env.AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL !== undefined ||
    env.AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION !== undefined
  ) {
    return descriptor;
  }
  const manifest = readJson(descriptor.manifest);
  const persisted = manifest?.externalIngress;
  if (
    manifest?.schemaVersion !== DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA ||
    manifest.environment !== 'development' || persisted?.configured !== true
  ) {
    return descriptor;
  }
  let rebound;
  try {
    rebound = developmentExternalIngressBinding({
      AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: persisted.publicOperatorUrl,
      AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: persisted.reviewedRevision,
    });
  } catch {
    return descriptor;
  }
  if (
    manifest.publicOperatorUrl !== rebound.publicOperatorUrl ||
    JSON.stringify(persisted) !== JSON.stringify(rebound)
  ) {
    return descriptor;
  }
  return developmentPresentationProviderDescriptor({
    ...env,
    AGENT_BROWSER_DEV_PUBLIC_OPERATOR_URL: rebound.publicOperatorUrl,
    AGENT_BROWSER_DEV_EXTERNAL_INGRESS_REVISION: rebound.reviewedRevision,
  });
}

export function developmentPresentationProviderStatus({ env = process.env, probe = null } = {}) {
  const descriptor = developmentPresentationProviderStatusDescriptor(env);
  const required = env.AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED === '1';
  let isolationError = null;
  try {
    validateDevelopmentPresentationProviderIsolation(descriptor, productionPresentationProjection(env));
    if (required && descriptor.externalIngress.configured !== true) {
      throw new Error('Required development presentation provider has no reviewed public HTTPS ingress binding');
    }
  } catch (error) {
    isolationError = error instanceof Error ? error.message : String(error);
  }
  if (!existsSync(descriptor.manifest)) {
    return {
      descriptor,
      manifest: null,
      state: isolationError ? 'invalid' : 'unconfigured',
      ready: false,
      blocking: required || Boolean(isolationError),
      externalIngressRequired: required,
      isolationError,
    };
  }
  const manifest = readJson(descriptor.manifest);
  const expected = developmentPresentationProviderManifest(descriptor);
  const matches = manifest !== null &&
    developmentPresentationProviderManifestCompatible(manifest, expected);
  if (!matches || isolationError) {
    return {
      descriptor,
      manifest,
      observation: null,
      readinessChecks: [],
      state: 'drifted',
      ready: false,
      blocking: true,
      externalIngressRequired: required,
      isolationError,
    };
  }
  const observation = probe ? probe(descriptor) : {
    environment: 'development',
    unavailable: 'live provider probe is not configured',
  };
  const readinessChecks = evaluateDevelopmentPresentationProviderObservation(
    descriptor,
    observation,
  );
  const ready = readinessChecks.every((item) => item.ok);
  return {
    descriptor,
    manifest,
    observation,
    readinessChecks,
    state: ready ? 'configured' : 'not_ready',
    ready,
    blocking: true,
    externalIngressRequired: required,
    isolationError,
  };
}

/**
 * Publishes the repository skill into the development pseudo-home only. The
 * shared user-scoped production skill is deliberately outside this target.
 */
export function synchronizeDevelopmentAgentSkill({ env = process.env } = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  validateDevelopmentPresentationProviderIsolation(
    descriptor,
    productionPresentationProjection(env),
  );
  if (!existsSync(join(descriptor.skill.source, 'SKILL.md'))) {
    throw new Error(`Development skill source is unavailable: ${descriptor.skill.source}`);
  }
  mkdirSync(descriptor.skill.root, { recursive: true, mode: 0o700 });
  const staging = `${descriptor.skill.target}.next-${process.pid}`;
  const backup = `${descriptor.skill.target}.previous-${process.pid}`;
  rmSync(staging, { recursive: true, force: true });
  rmSync(backup, { recursive: true, force: true });
  cpSync(descriptor.skill.source, staging, { recursive: true });
  try {
    if (existsSync(descriptor.skill.target)) renameSync(descriptor.skill.target, backup);
    renameSync(staging, descriptor.skill.target);
    rmSync(backup, { recursive: true, force: true });
  } catch (error) {
    rmSync(staging, { recursive: true, force: true });
    if (!existsSync(descriptor.skill.target) && existsSync(backup)) {
      renameSync(backup, descriptor.skill.target);
    }
    throw error;
  }
  return {
    success: true,
    environment: 'development',
    source: descriptor.skill.source,
    target: descriptor.skill.target,
    sha256: directoryDigest(descriptor.skill.target),
  };
}

export function developmentAgentSkillStatus({ env = process.env } = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  const sourceSha256 = directoryDigest(descriptor.skill.source);
  const targetSha256 = directoryDigest(descriptor.skill.target);
  return {
    source: descriptor.skill.source,
    target: descriptor.skill.target,
    sourceSha256,
    targetSha256,
    state: targetSha256 === null ? 'unconfigured' : sourceSha256 === targetSha256 ? 'current' : 'drifted',
    ready: sourceSha256 !== null && sourceSha256 === targetSha256,
  };
}

export function doctorDevelopmentPresentationProvider({ env = process.env, probe = null } = {}) {
  const status = developmentPresentationProviderStatus({ env, probe });
  const checks = [
    check('presentation-provider:isolation', !status.isolationError, status.isolationError || 'isolated'),
    check(
      'presentation-provider:configuration',
      status.ready || !status.blocking,
      status.state,
    ),
    check(
      'presentation-provider:external-ingress',
      !status.externalIngressRequired || status.descriptor.externalIngress.configured === true,
      status.descriptor.externalIngress,
    ),
  ];
  if (status.manifest) {
    checks.push(check(
      'presentation-provider:manifest-environment',
      status.manifest.environment === 'development',
      status.manifest.environment,
    ));
    checks.push(check(
      'presentation-provider:route-count',
      status.manifest.routes?.length === status.descriptor.hardMaxSlots,
      status.manifest.routes?.length,
    ));
    checks.push(...status.readinessChecks);
  }
  return { success: checks.every((item) => item.ok), checks, status };
}

export function evaluateDevelopmentPresentationProviderObservation(descriptor, observation) {
  const checks = [
    check('presentation-provider:observed-environment', observation?.environment === 'development', observation?.environment),
    check('presentation-provider:secrets-private', observation?.secrets?.private === true, observation?.secrets?.private),
    check('presentation-provider:database-schema', observation?.database?.schemaReady === true, observation?.database?.schemaReady),
    check('presentation-provider:loaded-extension', observation?.extension?.matches === true, observation?.extension || null),
  ];
  for (const [service, name] of Object.entries(descriptor.services)) {
    const container = observation?.containers?.find((item) => item.name === name);
    checks.push(check(
      `presentation-provider:container:${service}`,
      container?.running === true && container?.composeProject === descriptor.composeProject,
      container || null,
    ));
  }
  for (const [name, portValue] of Object.entries(descriptor.ports)) {
    const observed = observation?.ports?.[name];
    checks.push(check(
      `presentation-provider:port:${name}`,
      observed?.listening === true && observed?.port === portValue,
      observed || null,
    ));
  }
  for (const route of descriptor.routes) {
    const user = observation?.routeUsers?.find((item) => item.user === route.user);
    checks.push(check(
      `presentation-provider:route-user:${route.routeId}`,
      user?.exists === true,
      user || null,
    ));
    const connection = observation?.database?.routes?.find((item) =>
      item.connectionName === route.connectionName && item.user === route.user,
    );
    checks.push(check(
      `presentation-provider:connection:${route.routeId}`,
      Boolean(connection?.connectionId) &&
        Number(connection?.maxConnections) === descriptor.connectionLimits.maxConnections &&
        Number(connection?.maxConnectionsPerUser) === descriptor.connectionLimits.maxConnectionsPerUser &&
        Boolean(connection?.sharingProfileId) &&
        connection?.sharingProfileName === `Agent Browser Shared Session ${route.routeId}` &&
        connection?.sharingProfileReadOnly === 'false' &&
        Number(connection?.sharingProfilePermissionCount) >= 1,
      connection || null,
    ));
  }
  const connectionIds = observation?.database?.routes
    ?.map((route) => route.connectionId)
    .filter(Boolean) || [];
  checks.push(check(
    'presentation-provider:connection-ids-unique',
    connectionIds.length === descriptor.routes.length &&
      new Set(connectionIds).size === descriptor.routes.length,
    connectionIds,
  ));
  const warmDisplays = [];
  for (const route of descriptor.routes.slice(0, descriptor.warmSlots)) {
    const display = observation?.displays?.find((item) =>
      item.displayReservationId === route.displayReservationId && item.user === route.user,
    );
    const displayReady = display?.ready === true && /^:[0-9]+$/.test(display?.displayName || '');
    checks.push(check(
      `presentation-provider:warm-display:${route.routeId}`,
      displayReady,
      display || null,
    ));
    if (displayReady) warmDisplays.push(display.displayName);
  }
  checks.push(check(
    'presentation-provider:warm-displays-unique',
    warmDisplays.length === descriptor.warmSlots &&
      new Set(warmDisplays).size === descriptor.warmSlots,
    warmDisplays,
  ));
  return checks;
}

function productionPresentationProjection(env = process.env) {
  const userHome = resolve(env.AGENT_BROWSER_DEV_USER_HOME || homedir());
  return {
    ports: csvIntegers(env.AGENT_BROWSER_PRODUCTION_PRESENTATION_PORTS, [...PRODUCTION_PORTS]),
    paths: csvStrings(env.AGENT_BROWSER_PRODUCTION_PRESENTATION_PATHS, [
      join(userHome, '.agent-browser'),
      join(userHome, '.local', 'lib', 'agent-browser'),
      join(userHome, '.codex', 'skills', 'agent-browser'),
    ]).map((path) => resolve(path)),
    identities: csvStrings(env.AGENT_BROWSER_PRODUCTION_PRESENTATION_IDENTITIES, [
      'agent-browser-guacamole',
      'agent-browser-guacd',
      'agent-browser-guacamole-postgres',
      'agent_browser_guacamole',
    ]),
  };
}

function publicHostname(hostname) {
  const host = hostname.replace(/^\[|\]$/g, '').replace(/\.$/, '').toLowerCase();
  if (!host || host === 'localhost' || host.endsWith('.localhost') || host.endsWith('.local')) return false;
  const ipVersion = isIP(host);
  if (ipVersion === 4) {
    const octets = host.split('.').map(Number);
    return !(
      octets[0] === 0 ||
      octets[0] === 10 ||
      octets[0] === 127 ||
      (octets[0] === 169 && octets[1] === 254) ||
      (octets[0] === 172 && octets[1] >= 16 && octets[1] <= 31) ||
      (octets[0] === 192 && octets[1] === 168)
    );
  }
  if (ipVersion === 6) {
    return host !== '::' && host !== '::1' && !/^f[cd]/.test(host) && !/^fe[89ab]/.test(host);
  }
  return host.includes('.');
}

function pathsOverlap(left, right) {
  const canonicalLeft = canonicalProspectivePath(left);
  const canonicalRight = canonicalProspectivePath(right);
  const leftToRight = relative(canonicalLeft, canonicalRight);
  const rightToLeft = relative(canonicalRight, canonicalLeft);
  return leftToRight === '' || !leftToRight.startsWith('..') || !rightToLeft.startsWith('..');
}

/** Resolve existing ancestor symlinks even before a provider path is staged. */
function canonicalProspectivePath(path) {
  const target = resolve(path);
  let ancestor = target;
  while (!existsSync(ancestor)) ancestor = dirname(ancestor);
  return resolve(realpathSync(ancestor), relative(ancestor, target));
}

function assertUnique(items, field) {
  const values = items.map((item) => item[field]);
  if (new Set(values).size !== values.length) {
    throw new Error(`Development presentation routes contain duplicate ${field}`);
  }
}

function positiveInteger(value, fallback) {
  const parsed = Number(value || fallback);
  if (!Number.isInteger(parsed) || parsed < 1) throw new Error(`Expected a positive integer, received ${value}`);
  return parsed;
}

function port(value, fallback) {
  const parsed = positiveInteger(value, fallback);
  if (parsed > 65535) throw new Error(`Invalid port: ${parsed}`);
  return parsed;
}

function csvStrings(value, fallback) {
  return value ? value.split(',').map((item) => item.trim()).filter(Boolean) : fallback;
}

function csvIntegers(value, fallback) {
  return value ? csvStrings(value, []).map((item) => Number(item)) : fallback;
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch {
    return null;
  }
}

function directoryDigest(root) {
  if (!existsSync(root) || !statSync(root).isDirectory()) return null;
  const hash = createHash('sha256');
  for (const path of directoryFiles(root)) {
    hash.update(relative(root, path));
    hash.update('\0');
    hash.update(readFileSync(path));
    hash.update('\0');
  }
  return hash.digest('hex');
}

function directoryFiles(root) {
  const files = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) files.push(...directoryFiles(path));
    else if (entry.isFile()) files.push(path);
  }
  return files.sort();
}

function check(name, ok, observed) {
  return { name, ok, observed: observed ?? null };
}
