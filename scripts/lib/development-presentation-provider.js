import { createHash } from 'node:crypto';
import {
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';

export const DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA =
  'agent-browser.development-presentation-provider.v1';

const PRODUCTION_PORTS = new Set([3389, 3390, 4822, 5432, 4848, 4849, 8092]);

/**
 * Returns the complete development-owned presentation-provider identity.
 * Callers must consume this descriptor as a unit instead of composing ambient
 * production paths, ports, users, or route names into a development runtime.
 */
export function developmentPresentationProviderDescriptor(env = process.env) {
  const userHome = resolve(env.AGENT_BROWSER_DEV_USER_HOME || homedir());
  const pseudoHome = resolve(
    env.AGENT_BROWSER_DEV_HOME || join(userHome, '.local', 'share', 'agent-browser-dev', 'home'),
  );
  const root = resolve(
    env.AGENT_BROWSER_DEV_PRESENTATION_ROOT ||
      join(userHome, '.local', 'share', 'agent-browser-dev', 'presentation-provider'),
  );
  const warmSlots = positiveInteger(env.AGENT_BROWSER_DEV_PRESENTATION_WARM_SLOTS, 4);
  const hardMaxSlots = positiveInteger(env.AGENT_BROWSER_DEV_PRESENTATION_MAX_SLOTS, 6);
  if (hardMaxSlots < warmSlots) {
    throw new Error('Development presentation hard maximum must be at least the warm slot count');
  }
  const displayBase = positiveInteger(env.AGENT_BROWSER_DEV_DISPLAY_BASE, 120);
  const rdpPortBase = positiveInteger(env.AGENT_BROWSER_DEV_RDP_PORT_BASE, 3490);
  const routes = Array.from({ length: hardMaxSlots }, (_, index) => {
    const ordinal = index + 1;
    return {
      ordinal,
      routeId: `development-route-${ordinal}`,
      slotId: `development-slot-${ordinal}`,
      user: `agent-browser-dev-route-${ordinal}`,
      connectionId: `agent-browser-dev-connection-${ordinal}`,
      display: `:${displayBase + index}`,
      rdpPort: rdpPortBase + index,
      lifecycle: ordinal <= warmSlots ? 'warm' : 'elastic',
    };
  });
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
    environment: 'development',
    userHome,
    pseudoHome,
    root,
    manifest: join(root, 'provider.json'),
    secretsDir: join(root, 'secrets'),
    stateDir: join(root, 'state'),
    receiptsDir: join(root, 'receipts'),
    inventoryPath: join(root, 'state', 'route-inventory.json'),
    composeProject: 'agent-browser-dev-presentation',
    services: {
      guacamole: 'agent-browser-dev-guacamole',
      guacd: 'agent-browser-dev-guacd',
      postgres: 'agent-browser-dev-guacamole-postgres',
    },
    database: {
      name: 'agent_browser_dev_guacamole',
      user: 'agent_browser_dev_guacamole',
    },
    ports: {
      guacamole: port(env.AGENT_BROWSER_DEV_GUACAMOLE_PORT, 8093),
      guacd: port(env.AGENT_BROWSER_DEV_GUACD_PORT, 4823),
      postgres: port(env.AGENT_BROWSER_DEV_POSTGRES_PORT, 55433),
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
  const providerPorts = Object.entries(descriptor.ports);
  const allDevelopmentPorts = [
    ...providerPorts.map(([, value]) => value),
    ...descriptor.routes.map((route) => route.rdpPort),
  ];
  if (new Set(allDevelopmentPorts).size !== allDevelopmentPorts.length) {
    throw new Error('Development presentation provider contains duplicate ports');
  }
  for (const [name, value] of [
    ...providerPorts,
    ...descriptor.routes.map((route) => [`route:${route.routeId}`, route.rdpPort]),
  ]) {
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
    ...descriptor.routes.flatMap((route) => [route.routeId, route.slotId, route.user, route.connectionId]),
  ];
  for (const identity of identities) {
    if (production.identities.includes(identity)) {
      throw new Error(`Development identity ${identity} collides with production`);
    }
  }
  assertUnique(descriptor.routes, 'routeId');
  assertUnique(descriptor.routes, 'slotId');
  assertUnique(descriptor.routes, 'user');
  assertUnique(descriptor.routes, 'connectionId');
  assertUnique(descriptor.routes, 'display');
  assertUnique(descriptor.routes, 'rdpPort');
  if (descriptor.routes.length !== descriptor.hardMaxSlots) {
    throw new Error('Development route inventory must describe every admitted slot');
  }
  return descriptor;
}

export function developmentPresentationProviderManifest(descriptor) {
  return {
    schemaVersion: DEVELOPMENT_PRESENTATION_PROVIDER_SCHEMA,
    environment: descriptor.environment,
    root: descriptor.root,
    secretsDir: descriptor.secretsDir,
    stateDir: descriptor.stateDir,
    receiptsDir: descriptor.receiptsDir,
    inventoryPath: descriptor.inventoryPath,
    composeProject: descriptor.composeProject,
    services: descriptor.services,
    database: descriptor.database,
    ports: descriptor.ports,
    warmSlots: descriptor.warmSlots,
    hardMaxSlots: descriptor.hardMaxSlots,
    routes: descriptor.routes,
    skill: { target: descriptor.skill.target },
  };
}

export function developmentPresentationProviderStatus({ env = process.env } = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  const required = env.AGENT_BROWSER_DEV_PRESENTATION_PROVIDER_REQUIRED === '1';
  let isolationError = null;
  try {
    validateDevelopmentPresentationProviderIsolation(descriptor, productionPresentationProjection(env));
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
      isolationError,
    };
  }
  const manifest = readJson(descriptor.manifest);
  const expected = developmentPresentationProviderManifest(descriptor);
  const matches = manifest !== null && JSON.stringify(manifest) === JSON.stringify(expected);
  return {
    descriptor,
    manifest,
    state: !isolationError && matches ? 'configured' : 'drifted',
    ready: !isolationError && matches,
    blocking: true,
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

export function doctorDevelopmentPresentationProvider({ env = process.env } = {}) {
  const status = developmentPresentationProviderStatus({ env });
  const checks = [
    check('presentation-provider:isolation', !status.isolationError, status.isolationError || 'isolated'),
    check(
      'presentation-provider:configuration',
      status.ready || !status.blocking,
      status.state,
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
  }
  return { success: checks.every((item) => item.ok), checks, status };
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

function pathsOverlap(left, right) {
  const leftToRight = relative(left, right);
  const rightToLeft = relative(right, left);
  return leftToRight === '' || !leftToRight.startsWith('..') || !rightToLeft.startsWith('..');
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
