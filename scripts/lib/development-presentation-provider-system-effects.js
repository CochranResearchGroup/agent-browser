import { spawnSync } from 'node:child_process';
import {
  accessSync,
  constants as fsConstants,
  existsSync,
  readFileSync,
  statSync,
  statfsSync,
} from 'node:fs';
import { join } from 'node:path';
import { developmentRuntimeNamespace } from './development-runtime-namespace.js';
import {
  developmentPresentationProviderDescriptor,
  developmentPresentationProviderManifest,
  developmentPresentationProviderManifestCompatible,
  developmentPresentationProviderManifestUpgradeCompatible,
  validateDevelopmentPresentationProviderIsolation,
} from './development-presentation-provider.js';
import {
  probeDevelopmentPresentationProvider,
  renderDevelopmentPresentationProviderBundle,
} from './development-presentation-provider-deployment.js';
import {
  evaluateDevelopmentPresentationPressure,
  sampleDevelopmentPresentationPressure,
} from './development-presentation-pressure.js';

const MIN_AVAILABLE_MEMORY_BYTES = 8 * 1024 ** 3;
const MIN_FREE_DISK_BYTES = 10 * 1024 ** 3;

/** Read-only admission for a fresh or configured development provider transaction. */
export function developmentPresentationProviderSystemPreflight({
  env = process.env,
  run = commandResult,
} = {}) {
  const descriptor = developmentPresentationProviderDescriptor(env);
  const configured = existsSync(descriptor.manifest);
  const mode = configured ? 'reconcile' : 'fresh';
  const checks = [];
  try {
    validateDevelopmentPresentationProviderIsolation(descriptor);
    checks.push(check('isolation', true, 'development identities isolated'));
  } catch (error) {
    checks.push(check('isolation', false, error instanceof Error ? error.message : String(error)));
  }
  checks.push(check(
    'external-ingress',
    descriptor.externalIngress.configured === true,
    descriptor.externalIngress,
  ));
  const docker = run('docker', ['info', '--format', '{{.ServerVersion}}']);
  checks.push(check('docker', docker.status === 0, docker.status === 0 ? docker.stdout.trim() : commandError(docker)));
  const helper = env.AGENT_BROWSER_PRIVILEGED_HELPER ||
    '/usr/local/libexec/agent-browser/agent-browser-privileged-helper';
  checks.push(check('privileged-helper-file', executableFile(helper), helper));
  const helperCheck = run('sudo', ['-n', helper, 'check']);
  checks.push(check('privileged-helper-noninteractive', helperCheck.status === 0,
    helperCheck.status === 0 ? 'ready' : commandError(helperCheck)));
  const xrdp = run('systemctl', ['is-active', 'xrdp', 'xrdp-sesman']);
  checks.push(check('shared-xrdp-substrate', xrdp.status === 0, xrdp.status === 0 ? 'active' : commandError(xrdp)));
  checks.push(check('shared-xrdp-restart-prohibited', descriptor.rdpTarget.restartAllowed === false,
    descriptor.rdpTarget.restartAllowed));

  const bundle = renderDevelopmentPresentationProviderBundle(descriptor);
  for (const [relativePath, expected] of Object.entries(bundle.files)) {
    const path = join(descriptor.root, relativePath);
    let current = null;
    try {
      current = readFileSync(path, 'utf8');
    } catch {
      current = null;
    }
    checks.push(check(`bundle:${relativePath}`, current === expected, current === null ? 'missing' : 'current'));
  }
  if (configured) {
    let manifest = null;
    try {
      manifest = JSON.parse(readFileSync(descriptor.manifest, 'utf8'));
    } catch {
      manifest = null;
    }
    checks.push(check(
      'configured-manifest',
      manifest !== null && (
        developmentPresentationProviderManifestCompatible(
          manifest,
          developmentPresentationProviderManifest(descriptor),
        ) || developmentPresentationProviderManifestUpgradeCompatible(
          manifest,
          developmentPresentationProviderManifest(descriptor),
        )
      ),
      manifest === null ? 'unreadable' : 'configured',
    ));
  } else {
    checks.push(check('fresh-manifest', true, 'absent'));
  }

  const containerObservations = new Map();
  for (const [service, name] of Object.entries(descriptor.services)) {
    const result = run('docker', [
      'inspect',
      '-f',
      '{{index .Config.Labels "com.docker.compose.project"}}\t{{.State.Running}}',
      name,
    ]);
    const [project, running] = result.status === 0 ? result.stdout.trim().split('\t') : [];
    containerObservations.set(service, { result, project, running });
  }
  for (const [name, port] of Object.entries(descriptor.ports)) {
    const result = run('ss', ['-H', '-ltn', `sport = :${port}`]);
    const listening = result.status === 0 && Boolean(result.stdout.trim());
    const container = containerObservations.get(name);
    const ownedListener = configured && container?.project === descriptor.composeProject &&
      container?.running === 'true';
    checks.push(check(
      configured ? `port-retry-safe:${name}` : `port-free:${name}`,
      result.status === 0 && (!listening || ownedListener),
      listening ? ownedListener ? 'exact managed listener' : result.stdout.trim() : 'free',
    ));
  }
  for (const [service, name] of Object.entries(descriptor.services)) {
    const { result, project, running } = containerObservations.get(service);
    const retrySafe = result.status !== 0 || (project === descriptor.composeProject &&
      (configured || running === 'false'));
    checks.push(check(`container-retry-safe:${service}`, retrySafe,
      result.status !== 0 ? 'absent' : `${project || 'unowned'}:${running || 'unknown'}`));
  }
  for (const route of descriptor.routes) {
    const result = run('getent', ['passwd', route.user]);
    const fields = result.status === 0 ? result.stdout.trim().split(':') : [];
    const retrySafe = result.status !== 0 || (
      fields[0] === route.user &&
      fields[4] === 'agent-browser route-pool RDP session' &&
      fields[5] === `/home/${route.user}` &&
      fields[6] === '/bin/bash'
    );
    checks.push(check(`route-user-retry-safe:${route.routeId}`, retrySafe,
      result.status !== 0 ? 'absent' : retrySafe ? 'exact managed identity' : 'identity drift'));
  }
  const memoryAvailable = meminfoBytes('MemAvailable');
  checks.push(check('memory-available', memoryAvailable >= MIN_AVAILABLE_MEMORY_BYTES, memoryAvailable));
  const disk = statfsSync(descriptor.root);
  const diskAvailable = disk.bavail * disk.bsize;
  checks.push(check('disk-available', diskAvailable >= MIN_FREE_DISK_BYTES, diskAvailable));
  return {
    schemaVersion: 'agent-browser.development-presentation-provider-preflight.v1',
    environment: 'development',
    mode,
    authorizesEffects: false,
    success: checks.every((item) => item.ok),
    descriptor,
    checks,
  };
}

/** Bind the transaction owner to the current workstation effect seams. */
export function createDevelopmentPresentationProviderSystemEffects({
  env = process.env,
  productionSnapshot,
  assertProductionUnchanged,
  defaultDevelopmentSnapshot,
  assertDefaultDevelopmentUnchanged,
  publishIngress,
  run = commandResult,
} = {}) {
  if (typeof productionSnapshot !== 'function' || typeof assertProductionUnchanged !== 'function') {
    throw new Error('Development provider effects require production identity guards');
  }
  const namespaced = developmentRuntimeNamespace(env).namespace !== null;
  if (namespaced && (typeof defaultDevelopmentSnapshot !== 'function' ||
      typeof assertDefaultDevelopmentUnchanged !== 'function')) {
    throw new Error('Namespaced provider effects require default development identity guards');
  }
  const helper = env.AGENT_BROWSER_PRIVILEGED_HELPER ||
    '/usr/local/libexec/agent-browser/agent-browser-privileged-helper';
  const operatorUser = env.AGENT_BROWSER_DEV_OPERATOR_USER || env.USER;
  return {
    snapshotProduction: () => namespaced ? {
      production: productionSnapshot(env),
      defaultDevelopment: defaultDevelopmentSnapshot(env),
    } : productionSnapshot(env),
    assertProductionUnchanged: (before, after) => {
      if (!namespaced) return assertProductionUnchanged(before, after);
      assertProductionUnchanged(before.production, after.production);
      assertDefaultDevelopmentUnchanged(before.defaultDevelopment, after.defaultDevelopment);
    },
    createVolume(descriptor) {
      const volume = `${descriptor.services.postgres}-data`;
      const inspected = run('docker', ['volume', 'inspect', volume]);
      if (inspected.status !== 0) {
        runRequired(run, 'docker', [
          'volume',
          'create',
          '--label',
          'agent-browser.environment=development',
          '--label',
          `com.docker.compose.project=${descriptor.composeProject}`,
          volume,
        ], {}, 'create development Guacamole volume');
      }
    },
    startDatabase(descriptor) {
      composeRequired(run, descriptor, ['up', '-d', 'postgres'], 'start development PostgreSQL');
      waitFor(run, () => containerHealth(run, descriptor.services.postgres) === 'healthy',
        60000, 'development PostgreSQL health');
    },
    ensureRouteUser(route, password) {
      runRequired(
        run,
        'sudo',
        ['-n', helper, 'ensure-rdp-route-user', '--user', route.user],
        { input: `${password}\n` },
        `ensure ${route.routeId}`,
      );
    },
    syncConnections(descriptor, routeSecrets) {
      const helperPath = join(process.cwd(), 'scripts', 'lib', 'rdp-route-user-pool.py');
      const sql = runRequired(
        run,
        'python3',
        [
          helperPath,
          'sql',
          '--hostname',
          descriptor.rdpTarget.host,
          '--port',
          String(descriptor.rdpTarget.port),
          '--max-connections',
          String(descriptor.connectionLimits.maxConnections),
          '--max-connections-per-user',
          String(descriptor.connectionLimits.maxConnectionsPerUser),
        ],
        { input: JSON.stringify(routeSecrets) },
        'render development Guacamole routes',
      ).stdout;
      runRequired(
        run,
        'docker',
        [
          'exec',
          '-i',
          descriptor.services.postgres,
          'psql',
          '-U',
          descriptor.database.user,
          '-d',
          descriptor.database.name,
          '-v',
          'ON_ERROR_STOP=1',
        ],
        { input: sql },
        'sync development Guacamole routes',
      );
      runRequired(run, 'docker', [
        'exec',
        descriptor.services.postgres,
        'psql',
        '-U',
        descriptor.database.user,
        '-d',
        descriptor.database.name,
        '-v',
        'ON_ERROR_STOP=1',
        '-c',
        'CHECKPOINT;',
      ], {}, 'checkpoint development Guacamole database');
    },
    startProvider(descriptor) {
      composeRequired(run, descriptor, [
        'up',
        '-d',
        '--force-recreate',
        'guacd',
        'guacamole',
      ], 'start development provider');
      waitFor(run, () => containerHealth(run, descriptor.services.guacd) === 'healthy',
        60000, 'development guacd health');
      waitFor(run, () => {
        const result = run('curl', [
          '--silent',
          '--show-error',
          '--output',
          '/dev/null',
          '--write-out',
          '%{http_code}',
          '--max-time',
          '5',
          `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole/`,
        ]);
        const status = Number(result.stdout.trim());
        return result.status === 0 && status >= 200 && status < 500;
      }, 60000, 'development Guacamole web readiness');
    },
    grantOperatorRouteAccess(descriptor) {
      const token = runRequired(run, 'curl', [
        '--silent',
        '--show-error',
        '--fail-with-body',
        '--max-time',
        '10',
        '--request',
        'POST',
        '--header',
        `Remote-User: ${operatorUser}`,
        '--header',
        'Content-Type: application/x-www-form-urlencoded',
        '--data',
        '',
        `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole/api/tokens`,
      ], {}, 'create development Guacamole operator').stdout;
      let tokenPayload;
      try {
        tokenPayload = JSON.parse(token);
      } catch {
        throw new Error('create development Guacamole operator returned invalid JSON');
      }
      if (!tokenPayload.authToken) {
        throw new Error('create development Guacamole operator returned no auth token');
      }
      const connectionNames = descriptor.routes.map((route) => postgresLiteral(route.connectionName)).join(', ');
      const operator = postgresLiteral(operatorUser);
      const sql = `insert into guacamole_connection_permission (entity_id, connection_id, permission)
select e.entity_id, c.connection_id, 'READ'::guacamole_object_permission_type
from guacamole_entity e
cross join guacamole_connection c
where e.name = ${operator} and e.type = 'USER'
  and c.connection_name in (${connectionNames})
on conflict do nothing;
select count(*)
from guacamole_connection_permission p
join guacamole_entity e on e.entity_id = p.entity_id
join guacamole_connection c on c.connection_id = p.connection_id
where e.name = ${operator} and e.type = 'USER' and p.permission = 'READ'
  and c.connection_name in (${connectionNames});`;
      const grant = runRequired(run, 'docker', [
        'exec',
        descriptor.services.postgres,
        'psql',
        '-U',
        descriptor.database.user,
        '-d',
        descriptor.database.name,
        '-t',
        '-A',
        '-v',
        'ON_ERROR_STOP=1',
        '-c',
        sql,
      ], {}, 'grant development Guacamole route access');
      if (Number(grant.stdout.trim().split(/\r?\n/).at(-1)) !== descriptor.routes.length) {
        throw new Error('development Guacamole operator route grant count drifted');
      }
    },
    openWarmRoutes(descriptor) {
      const observation = probeDevelopmentPresentationProvider(descriptor, { run });
      const databaseRoutes = new Map(
        observation.database.routes.map((route) => [route.connectionName, route]),
      );
      const baseUrl = `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole/`;
      const routes = descriptor.routes.slice(0, descriptor.warmSlots).map((route) => {
        const connectionId = databaseRoutes.get(route.connectionName)?.connectionId;
        if (!connectionId) throw new Error(`Development Guacamole connection is missing: ${route.routeId}`);
        const clientId = Buffer.from(`${connectionId}\0c\0postgresql`, 'utf8').toString('base64');
        const frameUrl = `${baseUrl}#/client/${clientId}`;
        return {
          id: route.routeId,
          routeId: `guacamole:${connectionId}`,
          connectionId,
          connectionName: route.connectionName,
          frameUrl,
          externalUrl: frameUrl,
          viewerSession: route.viewerSession,
          viewerProfile: route.viewerProfile,
          target: {
            routeUser: route.user,
            displayReservationId: route.displayReservationId,
          },
        };
      });
      runRequired(run, process.execPath, [
        join(process.cwd(), 'scripts', 'open-rdp-guac-route-displays.js'),
        '--wait-ms',
        '1000',
      ], {
        env: {
          ...env,
          HOME: descriptor.pseudoHome,
          AGENT_BROWSER_HOME: join(descriptor.pseudoHome, '.agent-browser'),
          AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD: join(descriptor.userHome, '.local', 'bin', `agent-browser-dev${descriptor.namespace ? `-${descriptor.namespace}` : ''}`),
          AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(routes),
          AGENT_BROWSER_GUACAMOLE_BASE_URL: baseUrl,
          AGENT_BROWSER_GUACAMOLE_HEADER_USER: operatorUser,
          AGENT_BROWSER_ROUTE_DISPLAY_FORCE_VIEWER: '1',
          AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT: join(process.cwd(), 'scripts'),
        },
        timeout: 600000,
      }, 'open development warm route sessions');
    },
    observe: (descriptor) => probeDevelopmentPresentationProvider(descriptor, { run }),
    grantDisplayAccess(display) {
      runRequired(run, 'sudo', [
        '-n',
        helper,
        'grant-display-access',
        '--operator-user',
        operatorUser,
        '--route-user',
        display.user,
        '--display',
        display.displayName,
      ], {}, `grant display access for ${display.displayReservationId}`);
    },
    publishIngress: publishIngress || (() => {
      throw new Error('Development provider ingress publisher is not configured');
    }),
    quarantine({ descriptor }) {
      const processes = run('ps', ['-eo', 'args=']);
      const processText = processes.status === 0 ? processes.stdout : '';
      for (const route of descriptor.routes) {
        if (!processText.includes(route.viewerProfilePath)) continue;
        run(join(descriptor.userHome, '.local', 'bin', `agent-browser-dev${descriptor.namespace ? `-${descriptor.namespace}` : ''}`), [
          '--json',
          '--session',
          route.viewerSession,
          '--profile',
          route.viewerProfile,
          'close',
        ], {
          env: {
            ...env,
            HOME: descriptor.pseudoHome,
            AGENT_BROWSER_HOME: join(descriptor.pseudoHome, '.agent-browser'),
          },
          timeout: 30000,
        });
      }
      composeRequired(run, descriptor, ['stop', 'guacamole', 'guacd', 'postgres'],
        'stop quarantined development provider');
    },
  };
}

/** Live elastic lifecycle effects, bounded to one descriptor-owned route. */
export function createDevelopmentPresentationLifecycleSystemEffects(options = {}) {
  const env = options.env || process.env;
  const run = options.run || commandResult;
  const base = createDevelopmentPresentationProviderSystemEffects({ ...options, env, run });
  const helper = env.AGENT_BROWSER_PRIVILEGED_HELPER ||
    '/usr/local/libexec/agent-browser/agent-browser-privileged-helper';
  const operatorUser = env.AGENT_BROWSER_DEV_OPERATOR_USER || env.USER;
  const reclaimTimeoutMs = Number(options.reclaimTimeoutMs ??
    env.AGENT_BROWSER_DEV_PRESENTATION_RECLAIM_TIMEOUT_MS ?? 5000);
  const reclaimPollMs = Number(options.reclaimPollMs ?? 100);
  const pressureSnapshot = options.pressureSnapshot || sampleDevelopmentPresentationPressure;
  const reclaimCapability = () => {
    const result = run(helper, ['status-json']);
    if (result.error || result.status !== 0) {
      return {
        ready: false,
        reason: `helper_status_unavailable:${commandError(result) || result.status}`,
        helper,
      };
    }
    let status;
    try {
      status = JSON.parse(result.stdout);
    } catch {
      return { ready: false, reason: 'helper_status_json_invalid', helper };
    }
    const termination = status.routeSessionTermination;
    const ready = termination?.supported === true &&
      termination?.exactRouteUser === true &&
      termination?.idempotentWhenAbsent === true;
    return {
      ready,
      reason: ready ? null : 'helper_contract_missing',
      helper,
      helperVersion: status.helperVersion || null,
    };
  };
  return {
    ...base,
    reclaimCapability,
    pressureAdmission(descriptor) {
      return evaluateDevelopmentPresentationPressure(descriptor, pressureSnapshot());
    },
    provisionRoute(route, descriptor) {
      const observation = base.observe(descriptor);
      const connectionId = observation.database.routes
        .find((candidate) => candidate.connectionName === route.connectionName)?.connectionId;
      if (!connectionId) throw new Error(`Development route connection is missing: ${route.routeId}`);
      const baseUrl = `http://127.0.0.1:${descriptor.ports.guacamole}/guacamole/`;
      const clientId = Buffer.from(`${connectionId}\0c\0postgresql`, 'utf8').toString('base64');
      const inventory = [{
        id: route.routeId,
        routeId: `guacamole:${connectionId}`,
        connectionId,
        connectionName: route.connectionName,
        frameUrl: `${baseUrl}#/client/${clientId}`,
        externalUrl: `${baseUrl}#/client/${clientId}`,
        viewerSession: route.viewerSession,
        viewerProfile: route.viewerProfile,
        target: {
          routeUser: route.user,
          displayReservationId: route.displayReservationId,
        },
      }];
      runRequired(run, process.execPath, [
        join(process.cwd(), 'scripts', 'open-rdp-guac-route-displays.js'),
        '--wait-ms',
        '1000',
        '--allow-single-route',
      ], {
        env: {
          ...env,
          HOME: descriptor.pseudoHome,
          AGENT_BROWSER_HOME: join(descriptor.pseudoHome, '.agent-browser'),
          AGENT_BROWSER_ROUTE_DISPLAY_AGENT_BROWSER_CMD: join(descriptor.userHome, '.local', 'bin', `agent-browser-dev${descriptor.namespace ? `-${descriptor.namespace}` : ''}`),
          AGENT_BROWSER_RDP_ROUTE_POOL_JSON: JSON.stringify(inventory),
          AGENT_BROWSER_GUACAMOLE_BASE_URL: baseUrl,
          AGENT_BROWSER_GUACAMOLE_HEADER_USER: operatorUser,
          AGENT_BROWSER_ROUTE_DISPLAY_FORCE_VIEWER: '1',
          AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT: join(process.cwd(), 'scripts'),
        },
        timeout: 600000,
      }, `provision ${route.routeId}`);
    },
    cooldownStatus(_route, descriptor) {
      const requiredMs = Number(env.AGENT_BROWSER_DEV_PRESENTATION_COOLDOWN_MS || 5000);
      const modifiedAt = statSync(descriptor.inventoryPath).mtimeMs;
      const elapsedMs = Math.max(0, Date.now() - modifiedAt);
      return { ready: elapsedMs >= requiredMs, elapsedMs, requiredMs };
    },
    referenceCheck(route, descriptor) {
      const command = join(descriptor.userHome, '.local', 'bin', `agent-browser-dev${descriptor.namespace ? `-${descriptor.namespace}` : ''}`);
      const result = run(command, ['--json', 'service', 'status'], {
        env: {
          ...env,
          HOME: descriptor.pseudoHome,
          AGENT_BROWSER_HOME: join(descriptor.pseudoHome, '.agent-browser'),
        },
        timeout: 30000,
      });
      if (result.error || result.status !== 0) {
        return {
          routeId: route.routeId,
          blockers: [],
          ambiguities: [`service_status_unavailable:${commandError(result) || result.status}`],
        };
      }
      let status;
      try {
        status = JSON.parse(result.stdout);
      } catch {
        return { routeId: route.routeId, blockers: [], ambiguities: ['service_status_json_invalid'] };
      }
      return exactRouteReferences(status?.data?.service_state || status?.data?.serviceState || {}, route);
    },
    reclaimRoute(route, descriptor) {
      const capability = reclaimCapability();
      if (capability.ready !== true) {
        throw new Error(`Development reclaim capability is unavailable: ${capability.reason}`);
      }
      const command = join(descriptor.userHome, '.local', 'bin', `agent-browser-dev${descriptor.namespace ? `-${descriptor.namespace}` : ''}`);
      const close = run(command, [
        '--json',
        '--session', route.viewerSession,
        '--profile', route.viewerProfile,
        'close',
      ], {
        env: {
          ...env,
          HOME: descriptor.pseudoHome,
          AGENT_BROWSER_HOME: join(descriptor.pseudoHome, '.agent-browser'),
        },
        timeout: 30000,
      });
      if (close.error || (close.status !== 0 && !/not found|no browser|not running/i.test(`${close.stdout}${close.stderr}`))) {
        throw new Error(`close ${route.viewerSession} failed: ${commandError(close) || close.status}`);
      }
      const termination = run('sudo', [
        '-n', helper, 'terminate-rdp-route-session', '--user', route.user,
      ]);
      try {
        waitFor(run, () => {
          const processes = run('ps', ['-u', route.user, '-o', 'pid=,args=']);
          return processes.status !== 0 || !processes.stdout.trim();
        }, reclaimTimeoutMs, `reclaim ${route.routeId}`, reclaimPollMs);
      } catch {
        if (termination.error || termination.status !== 0) {
          throw new Error(`terminate ${route.routeId} failed: ${commandError(termination) || termination.status}`);
        }
        throw new Error(`Development route processes remain after reclaim: ${route.routeId}`);
      }
    },
  };
}

function exactRouteReferences(serviceState, route) {
  const blockers = [];
  const ambiguities = [];
  const remoteRoute = serviceState.remoteViewRoutes?.[route.routeId];
  const display = serviceState.displayAllocations?.[route.displayReservationId];
  const pool = serviceState.routePool?.[route.slotId];
  if (!remoteRoute || !display || !pool) ambiguities.push('service_binding_missing');
  if (remoteRoute?.browserId) blockers.push(`browser:${remoteRoute.browserId}`);
  if (remoteRoute?.sessionId) blockers.push(`session:${remoteRoute.sessionId}`);
  for (const leaseId of remoteRoute?.viewerLeaseIds || []) blockers.push(`viewer_lease:${leaseId}`);
  if (remoteRoute?.controllerLeaseId) blockers.push(`controller_lease:${remoteRoute.controllerLeaseId}`);
  if (display?.ownerBrowserId) blockers.push(`display_browser:${display.ownerBrowserId}`);
  if (display?.ownerSessionId) blockers.push(`display_session:${display.ownerSessionId}`);
  for (const [id, lease] of Object.entries(serviceState.remoteViewAcquisitionLeases || {})) {
    if (lease?.routeId === route.routeId || lease?.routePoolEntryId === route.slotId) {
      blockers.push(`acquisition_lease:${id}`);
    }
  }
  for (const [id, lease] of Object.entries(serviceState.viewerLeases || {})) {
    if (lease?.routeId === route.routeId || lease?.routePoolEntryId === route.slotId) {
      blockers.push(`viewer_lease:${id}`);
    }
  }
  for (const [id, handoff] of Object.entries(serviceState.remoteViewHandoffs || {})) {
    if (containsExactValue(handoff, route.routeId) || containsExactValue(handoff, route.slotId)) {
      blockers.push(`handoff:${id}`);
    }
  }
  const slot = serviceState.presentationCapacity?.slots?.find((candidate) => candidate.id === `slot:${route.slotId}`);
  if (slot?.browserId) blockers.push(`slot_browser:${slot.browserId}`);
  if (slot?.leaseRequestId) blockers.push(`slot_lease:${slot.leaseRequestId}`);
  if (slot?.restorationPending === true) blockers.push('restoration_pending');
  for (const id of slot?.cleanupObligationIds || []) blockers.push(`cleanup_obligation:${id}`);
  return {
    routeId: route.routeId,
    blockers: [...new Set(blockers)].sort(),
    ambiguities: [...new Set(ambiguities)].sort(),
  };
}

function containsExactValue(value, expected) {
  if (value === expected) return true;
  if (Array.isArray(value)) return value.some((candidate) => containsExactValue(candidate, expected));
  if (value && typeof value === 'object') {
    return Object.values(value).some((candidate) => containsExactValue(candidate, expected));
  }
  return false;
}

function commandResult(command, args, options = {}) {
  return spawnSync(command, args, { encoding: 'utf8', stdio: 'pipe', ...options });
}

function executableFile(path) {
  try {
    accessSync(path, fsConstants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function meminfoBytes(field) {
  const line = readFileSync('/proc/meminfo', 'utf8')
    .split(/\r?\n/)
    .find((candidate) => candidate.startsWith(`${field}:`));
  const kibibytes = Number(line?.match(/:\s+([0-9]+)/)?.[1] || 0);
  return kibibytes * 1024;
}

function commandError(result) {
  return (result.stderr || result.stdout || result.error?.message || '').trim();
}

function check(name, ok, observed) {
  return { name, ok, observed: observed ?? null };
}

function composeRequired(run, descriptor, args, label) {
  return runRequired(run, 'docker', [
    'compose',
    '--project-name',
    descriptor.composeProject,
    '--env-file',
    join(descriptor.root, '.env'),
    '--env-file',
    join(descriptor.secretsDir, 'provider.env'),
    '-f',
    join(descriptor.root, 'compose.yml'),
    ...args,
  ], {}, label);
}

function runRequired(run, command, args, options, label) {
  const result = run(command, args, options);
  if (result.error || result.status !== 0) {
    throw new Error(`${label} failed: ${commandError(result) || `exit ${result.status}`}`);
  }
  return result;
}

function containerHealth(run, name) {
  const result = run('docker', ['inspect', '-f', '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}', name]);
  return result.status === 0 ? result.stdout.trim() : null;
}

function waitFor(run, predicate, timeoutMs, label, pollMs = 1000) {
  const deadline = Date.now() + timeoutMs;
  do {
    if (predicate()) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, pollMs);
  } while (Date.now() < deadline);
  throw new Error(`${label} timed out`);
}

function postgresLiteral(value) {
  return `'${String(value).replaceAll("'", "''")}'`;
}
