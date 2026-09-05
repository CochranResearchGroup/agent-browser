import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  accessSync,
  chmodSync,
  copyFileSync,
  closeSync,
  existsSync,
  lstatSync,
  mkdirSync,
  openSync,
  readdirSync,
  readFileSync,
  readSync,
  readlinkSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
  constants as fsConstants,
} from 'node:fs';
import { homedir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { developmentRuntimeNamespace, requireNamespacedDevelopmentPorts } from './development-runtime-namespace.js';
import {
  developmentAgentSkillStatus,
  developmentPresentationProviderDescriptor,
  doctorDevelopmentPresentationProvider,
  synchronizeDevelopmentAgentSkill,
} from './development-presentation-provider.js';
import { probeDevelopmentPresentationProvider } from './development-presentation-provider-deployment.js';

export const DEVELOPMENT_RUNTIME_SCHEMA = 'agent-browser.development-runtime.v1';
const PROTECTED_LEASE_AUTHORITY_SOCKET_UNIT = 'agent-browser-lease-authority.socket';
const PROTECTED_LEASE_AUTHORITY_SOCKET_PATH = '/run/agent-browser/lease-authority.sock';
const PROTECTED_LEASE_AUTHORITY_OPERATOR_GROUP = 'agent-browser';

/**
 * Returns the optimized, release-compatible artifact used for ordinary
 * development publication. Full-LTO release artifacts are reserved for the
 * final production installation gate.
 */
export function developmentCandidateBinary(repoRoot = process.cwd()) {
  return resolve(repoRoot, 'cli', 'target', 'ci', 'agent-browser');
}

/**
 * Evaluates the shared root-owned authority endpoint without treating retained
 * lease records, browser history, or a per-lane process as live authority.
 */
export function evaluateProtectedLeaseAuthorityStatus({ unit, socket, operatorGroupId }) {
  const reasons = [];
  if (unit.loadState !== 'loaded') reasons.push('socket_unit_not_loaded');
  if (unit.activeState !== 'active') reasons.push('socket_unit_not_active');
  if (unit.unitFileState !== 'enabled') reasons.push('socket_unit_not_enabled');
  if (!socket.exists) reasons.push('socket_path_missing');
  else if (!socket.socket) reasons.push('socket_path_not_unix_socket');
  if (socket.uid !== 0) reasons.push('socket_owner_not_root');
  if (operatorGroupId === null || socket.gid !== operatorGroupId) {
    reasons.push('socket_group_mismatch');
  }
  if (socket.mode !== 0o660) reasons.push('socket_mode_mismatch');
  return { ready: reasons.length === 0, reasons };
}

export function developmentRuntimeDescriptor(env = process.env) {
  const namespace = developmentRuntimeNamespace(env);
  requireNamespacedDevelopmentPorts(env);
  const userHome = resolve(env.AGENT_BROWSER_DEV_USER_HOME || homedir());
  const installRoot = resolve(
    env.AGENT_BROWSER_DEV_INSTALL_ROOT || join(userHome, '.local', 'lib', namespace.name),
  );
  const pseudoHome = resolve(
    env.AGENT_BROWSER_DEV_HOME || join(userHome, '.local', 'share', namespace.name, 'home'),
  );
  const runtimeBase = resolve(
    env.AGENT_BROWSER_DEV_RUNTIME_DIR ||
      join(env.XDG_RUNTIME_DIR || `/run/user/${process.getuid?.() ?? 1000}`, namespace.name),
  );
  const browserExecutable = resolveDevelopmentBrowserExecutable(env, pseudoHome);
  const presentationProvider = developmentPresentationProviderDescriptor(env);
  const guacamoleHeaderUser = env.AGENT_BROWSER_DEV_OPERATOR_USER || env.USER;
  if (!guacamoleHeaderUser || !/^[A-Za-z0-9._@-]+$/.test(guacamoleHeaderUser)) {
    throw new Error('Development Guacamole header user contains unsupported characters');
  }
  const unitNames = {
    runtimeHost: `${namespace.name}-runtime-host.service`,
    backend: `${namespace.name}-dashboard-backend.service`,
    dashboard: `${namespace.name}-dashboard.service`,
  };
  const descriptor = {
    schemaVersion: DEVELOPMENT_RUNTIME_SCHEMA,
    environment: 'development',
    namespace: namespace.namespace,
    externalBrowserDiscovery: 'disabled',
    executable: resolve(env.AGENT_BROWSER_DEV_BIN || join(userHome, '.local', 'bin', namespace.name)),
    installRoot,
    generations: join(installRoot, 'generations'),
    current: join(installRoot, 'current'),
    pseudoHome,
    stateDir: join(pseudoHome, '.agent-browser'),
    runtimeHostIngressState: join(pseudoHome, '.agent-browser', 'runtime-host-ingress.json'),
    authDir: join(pseudoHome, '.agent-browser', 'dashboard-auth'),
    browserExecutable,
    guacamoleHeaderUser,
    laneManifest: join(
      pseudoHome,
      '.config',
      'agent-browser',
      'session-supervisors',
      `development-default${namespace.suffix}.json`,
    ),
    laneSession: `development-default${namespace.suffix}`,
    laneStreamPort: Number(env.AGENT_BROWSER_DEV_LANE_STREAM_PORT || 4951),
    socketDir: runtimeBase,
    systemdDir: resolve(
      env.AGENT_BROWSER_DEV_SYSTEMD_DIR || join(userHome, '.config', 'systemd', 'user'),
    ),
    dashboardPort: Number(env.AGENT_BROWSER_DEV_DASHBOARD_PORT || 4948),
    backendPort: Number(env.AGENT_BROWSER_DEV_BACKEND_PORT || 4949),
    shadowPort: Number(env.AGENT_BROWSER_DEV_SHADOW_PORT || 4950),
    localHost: `${namespace.name}.localhost`,
    ingressService: namespace.name,
    presentationProvider,
    unitNames,
    units: Object.values(unitNames),
  };
  if (namespace.namespace) validateNamespacedRuntimeIsolation(descriptor, env, userHome);
  return descriptor;
}

function canonicalProspectivePath(path) {
  let ancestor = resolve(path);
  while (!existsSync(ancestor)) ancestor = dirname(ancestor);
  return resolve(realpathSync(ancestor), relative(ancestor, resolve(path)));
}

function pathsOverlap(left, right) {
  const within = (root, path) => {
    const relation = relative(root, path);
    return relation === '' || (!relation.startsWith('../') && !isAbsolute(relation));
  };
  return within(left, right) || within(right, left);
}

/** A namespace cannot redirect its mutable runtime paths into either existing lane. */
function validateNamespacedRuntimeIsolation(descriptor, env, userHome) {
  const protectedPaths = [...new Set([userHome, homedir()])].flatMap((home) => [
    join(home, '.local/lib/agent-browser'), join(home, '.local/lib/agent-browser-dev'),
    join(home, '.local/share/agent-browser-dev'), join(home, '.agent-browser'),
    join(home, '.local/bin/agent-browser'), join(home, '.local/bin/agent-browser-dev'),
    join(home, '.config/agent-browser'),
  ]);
  const runtimeRoot = env.XDG_RUNTIME_DIR || `/run/user/${process.getuid?.() ?? 1000}`;
  protectedPaths.push(join(runtimeRoot, 'agent-browser'), join(runtimeRoot, 'agent-browser-dev'));
  for (const path of [descriptor.executable, descriptor.installRoot, descriptor.pseudoHome,
    descriptor.socketDir, descriptor.systemdDir]) {
    if (protectedPaths.some((protectedPath) => pathsOverlap(
      canonicalProspectivePath(path), canonicalProspectivePath(protectedPath),
    ))) throw new Error('Namespaced development path overlaps production or default development');
  }
}

export function renderDevelopmentUnits(descriptor, generationBinary) {
  const common = [
    `Environment=HOME=${descriptor.pseudoHome}`,
    `Environment=AGENT_BROWSER_RUNTIME_ENVIRONMENT=development`,
    `Environment=AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY=disabled`,
    `Environment=AGENT_BROWSER_RUNTIME_HOST=1`,
    `Environment=AGENT_BROWSER_SOCKET_DIR=${descriptor.socketDir}`,
    `Environment=AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE=${descriptor.runtimeHostIngressState}`,
    `Environment=AGENT_BROWSER_DASHBOARD_AUTH_DIR=${descriptor.authDir}`,
    `Environment=AGENT_BROWSER_EXECUTABLE_PATH=${descriptor.browserExecutable}`,
    `Environment=AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH=${descriptor.presentationProvider.inventoryPath}`,
    `Environment=AGENT_BROWSER_PRESENTATION_WARM_MINIMUM=${descriptor.presentationProvider.warmSlots}`,
    `Environment=AGENT_BROWSER_PRESENTATION_HARD_MAXIMUM=${descriptor.presentationProvider.hardMaxSlots}`,
    `Environment=AGENT_BROWSER_PRESENTATION_HUMAN_RESERVE=1`,
    `Environment=AGENT_BROWSER_PRESENTATION_RECOVERY_RESERVE=1`,
    `Environment=AGENT_BROWSER_GUACAMOLE_HEADER_USER=${descriptor.guacamoleHeaderUser}`,
    ...(descriptor.namespace ? [`Environment=AGENT_BROWSER_DEV_NAMESPACE=${descriptor.namespace}`] : []),
  ].join('\n');
  return {
    [descriptor.unitNames.runtimeHost]: `[Unit]
Description=Agent Browser development runtime host
After=default.target
StartLimitIntervalSec=60
StartLimitBurst=5

[Service]
Type=simple
${common}
ExecStart=${generationBinary} session supervisor run-host
Restart=on-failure
RestartSec=2
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
`,
    [descriptor.unitNames.backend]: `[Unit]
Description=Agent Browser development dashboard backend
After=network-online.target ${descriptor.unitNames.runtimeHost}
Wants=network-online.target ${descriptor.unitNames.runtimeHost}

[Service]
Type=simple
${common}
Environment=AGENT_BROWSER_DASHBOARD=1
Environment=AGENT_BROWSER_DASHBOARD_BACKEND_ONLY=1
Environment=AGENT_BROWSER_DASHBOARD_PORT=${descriptor.backendPort}
ExecStart=${generationBinary}
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
`,
    [descriptor.unitNames.dashboard]: `[Unit]
Description=Agent Browser development stable dashboard ingress
After=${descriptor.unitNames.backend} network-online.target
Wants=${descriptor.unitNames.backend} network-online.target

[Service]
Type=simple
${common}
Environment=AGENT_BROWSER_DASHBOARD_INGRESS=1
Environment=AGENT_BROWSER_DASHBOARD_PORT=${descriptor.dashboardPort}
Environment=AGENT_BROWSER_DASHBOARD_BACKEND_PORT=${descriptor.backendPort}
ExecStart=${generationBinary}
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
`,
  };
}

export function installDevelopmentRuntime({
  binary,
  env = process.env,
  activate = true,
  snapshotProduction = productionSnapshot,
  verifyProduction = assertProductionUnchanged,
}) {
  const descriptor = developmentRuntimeDescriptor(env);
  const sourceBinary = resolve(binary);
  if (!existsSync(sourceBinary)) throw new Error(`Development candidate does not exist: ${sourceBinary}`);
  const bytes = readFileSync(sourceBinary);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  const version = binaryVersion(sourceBinary, env);
  const generationId = `${version}-${sha256.slice(0, 12)}`;
  const generationDir = join(descriptor.generations, generationId);
  const generationBinary = join(generationDir, 'bin', 'agent-browser');
  const before = snapshotProduction(env);
  const defaultDevelopmentBefore = descriptor.namespace ? defaultDevelopmentSnapshot(env) : null;
  const previousCurrent = resolvedLink(descriptor.current);
  const previousExecutable = captureStableExecutable(descriptor.executable);
  const previousLaneManifests = captureDevelopmentLaneManifests(dirname(descriptor.laneManifest));
  const previousRuntimeHostIngress = existsSync(descriptor.runtimeHostIngressState)
    ? readFileSync(descriptor.runtimeHostIngressState, 'utf8')
    : null;
  const previousUnits = new Map();

  mkdirSync(join(generationDir, 'bin'), { recursive: true, mode: 0o700 });
  mkdirSync(descriptor.stateDir, { recursive: true, mode: 0o700 });
  mkdirSync(descriptor.authDir, { recursive: true, mode: 0o700 });
  mkdirSync(descriptor.socketDir, { recursive: true, mode: 0o700 });
  mkdirSync(descriptor.systemdDir, { recursive: true, mode: 0o700 });
  mkdirSync(dirname(descriptor.executable), { recursive: true, mode: 0o755 });
  if (existsSync(generationBinary)) {
    const installedSha256 = createHash('sha256').update(readFileSync(generationBinary)).digest('hex');
    if (installedSha256 !== sha256) {
      throw new Error(`Immutable development generation checksum mismatch: ${generationBinary}`);
    }
  } else {
    copyFileSync(sourceBinary, generationBinary);
    chmodSync(generationBinary, 0o755);
  }
  writeJsonAtomic(join(generationDir, 'generation.json'), {
    schemaVersion: DEVELOPMENT_RUNTIME_SCHEMA,
    environment: descriptor.environment,
    ...(descriptor.namespace ? { namespace: descriptor.namespace } : {}),
    generationId,
    version,
    sha256,
    sourceBinary,
    browserExecutable: descriptor.browserExecutable,
    externalBrowserDiscovery: descriptor.externalBrowserDiscovery,
    desktopInputProvider: {
      enabled: true,
      providerId: 'controlled-x11-xtest',
      capability: 'guarded_pointer_keyboard_v1',
      recipeId: 'p131-controlled-x11-v1',
    },
    installedAt: new Date().toISOString(),
  });
  mkdirSync(dirname(descriptor.laneManifest), { recursive: true, mode: 0o700 });
  writeJsonAtomic(descriptor.laneManifest, {
    schemaVersion: 'agent-browser.session-supervisor.v1',
    session: descriptor.laneSession,
    executablePath: generationBinary,
    executableSha256: sha256,
    streamPort: descriptor.laneStreamPort,
    runtimeProfile: descriptor.laneSession,
    ...(descriptor.namespace ? { namespace: descriptor.namespace } : {}),
    provenance: {
      packageVersion: version,
      installedAt: new Date().toISOString(),
      installedBy: 'agent-browser development-runtime installer',
    },
  });
  for (const [path, content] of previousLaneManifests) {
    if (path === descriptor.laneManifest) continue;
    const manifest = JSON.parse(content);
    if (manifest.schemaVersion !== 'agent-browser.session-supervisor.v1') {
      throw new Error(`Unsupported development lane manifest schema: ${path}`);
    }
    writeJsonAtomic(path, {
      ...manifest,
      executablePath: generationBinary,
      executableSha256: sha256,
      provenance: {
        packageVersion: version,
        installedAt: new Date().toISOString(),
        installedBy: 'agent-browser development-runtime installer',
      },
    });
  }

  const units = renderDevelopmentUnits(descriptor, generationBinary);
  for (const [name, content] of Object.entries(units)) {
    const path = join(descriptor.systemdDir, name);
    previousUnits.set(path, existsSync(path) ? readFileSync(path, 'utf8') : null);
    writeFileAtomic(path, content, 0o644);
  }
  replaceSymlink(descriptor.current, generationDir);
  writeFileAtomic(
    descriptor.executable,
    renderDevelopmentLauncher(descriptor, generationBinary),
    0o755,
  );

  try {
    if (activate && env.AGENT_BROWSER_DEV_SKIP_SYSTEMD !== '1') {
      activateDevelopmentRuntime({ descriptor, generationId, generationBinary, sha256, env });
    }
    synchronizeDevelopmentAgentSkill({ env });
    const after = snapshotProduction(env);
    verifyProduction(before, after);
    const defaultDevelopmentAfter = descriptor.namespace ? defaultDevelopmentSnapshot(env) : null;
    if (descriptor.namespace) assertDefaultDevelopmentUnchanged(defaultDevelopmentBefore, defaultDevelopmentAfter);
    return {
      success: true,
      descriptor,
      generation: { generationId, path: generationDir, binary: generationBinary, version, sha256 },
      production: { before, after, unchanged: true },
      ...(descriptor.namespace ? { defaultDevelopment: {
        before: defaultDevelopmentBefore, after: defaultDevelopmentAfter, unchanged: true,
      } } : {}),
      status: developmentRuntimeStatus({ env }),
    };
  } catch (error) {
    if (activate && env.AGENT_BROWSER_DEV_SKIP_SYSTEMD !== '1' && !previousCurrent) {
      try {
        systemctl(['disable', '--now', ...descriptor.units], env);
      } catch {
        // Preserve the original activation error; doctor will expose rollback issues.
      }
    }
    if (previousCurrent) {
      replaceSymlink(descriptor.current, previousCurrent);
    } else {
      removeLink(descriptor.current);
    }
    restoreStableExecutable(descriptor.executable, previousExecutable);
    restoreDevelopmentLaneManifests(
      new Set([descriptor.laneManifest, ...previousLaneManifests.keys()]),
      previousLaneManifests,
    );
    if (previousRuntimeHostIngress === null) {
      rmSync(descriptor.runtimeHostIngressState, { force: true });
    } else {
      writeFileAtomic(descriptor.runtimeHostIngressState, previousRuntimeHostIngress, 0o600);
    }
    for (const [path, content] of previousUnits) {
      if (content === null) rmSync(path, { force: true });
      else writeFileAtomic(path, content, 0o644);
    }
    if (activate && env.AGENT_BROWSER_DEV_SKIP_SYSTEMD !== '1') {
      try {
        if (previousCurrent) {
          const previousBinary = join(previousCurrent, 'bin', 'agent-browser');
          activateDevelopmentRuntime({ descriptor, generationId: basename(previousCurrent),
            generationBinary: previousBinary,
            sha256: createHash('sha256').update(readFileSync(previousBinary)).digest('hex'), env });
        } else systemctl(['daemon-reload'], env);
      } catch {
        // Preserve the original activation error; doctor will expose rollback issues.
      }
    }
    throw error;
  }
}

/** Start only the runtime host until exact host ownership is ready, then admit dashboard clients. */
export function activateDevelopmentRuntime({
  descriptor, generationId, generationBinary, sha256, env = process.env,
  runSystemctl = systemctl, observeHost = observeDevelopmentRuntimeHostReadiness,
  publishIngress = publishDevelopmentRuntimeIngress, waitForManifest = waitForDevelopmentManifest,
  now = Date.now, wait = () => execFileSync('sleep', ['0.25']),
}) {
  const timeout = Number(env.AGENT_BROWSER_DEV_START_TIMEOUT_MS || 20_000);
  if (!Number.isFinite(timeout) || timeout < 1) throw new Error('Invalid development startup timeout');
  runSystemctl(['daemon-reload'], env);
  runSystemctl(['enable', ...descriptor.units], env);
  runSystemctl(['stop', descriptor.unitNames.dashboard, descriptor.unitNames.backend], env);
  runSystemctl(['reset-failed', descriptor.unitNames.runtimeHost], env);
  runSystemctl(['restart', descriptor.unitNames.runtimeHost], env);
  const deadline = now() + timeout;
  let ready = false;
  do {
    const observation = observeHost({ descriptor, generationBinary, sha256, env });
    if (observation.state === 'wrong_owner') {
      throw new Error('Development runtime host listener belongs to the wrong systemd unit');
    }
    if (observation.state === 'ready') { ready = true; break; }
    wait();
  } while (now() < deadline);
  if (!ready) throw new Error('Development runtime host did not establish exact owned readiness');
  publishIngress({ descriptor, generationId, generationBinary, sha256 });
  runSystemctl(['start', descriptor.unitNames.backend, descriptor.unitNames.dashboard], env);
  waitForManifest(descriptor, generationBinary, env);
}

/** Evaluate the listener, systemd owner and current runtime identity together before client startup. */
export function evaluateDevelopmentRuntimeHostReadiness({
  unit, host, identity, listenerPid, listenerCgroup, startToken, expectedUnit, generationBinary, sha256,
}) {
  if (listenerPid && (listenerPid !== unit.mainPid ||
      !listenerCgroup?.split('\n').some((line) => line.split(':').slice(2).join(':').endsWith(`/${expectedUnit}`)))) {
    return { state: 'wrong_owner' };
  }
  const ready = unit.activeState === 'active' && unit.mainPid > 0 &&
    unit.executable === generationBinary && listenerPid === unit.mainPid &&
    host?.pid === unit.mainPid && identity?.pid === unit.mainPid &&
    identity.executablePath === generationBinary && identity.startToken === startToken &&
    typeof startToken === 'string' && host.executableGeneration === sha256 &&
    typeof host.socketIdentity === 'string' && host.socketIdentity.length > 0;
  return { state: ready ? 'ready' : 'pending' };
}

function observeDevelopmentRuntimeHostReadiness({ descriptor, generationBinary, sha256, env }) {
  const unit = unitStatus(descriptor.unitNames.runtimeHost, env);
  const listenerPid = listeningProcessId(descriptor.laneStreamPort);
  let listenerCgroup = null;
  let startToken = null;
  if (listenerPid) {
    try {
      listenerCgroup = readFileSync(`/proc/${listenerPid}/cgroup`, 'utf8');
      const stat = readFileSync(`/proc/${listenerPid}/stat`, 'utf8');
      const ticks = stat.slice(stat.lastIndexOf(')') + 2).split(/\s+/)[19];
      const boot = readFileSync('/proc/sys/kernel/random/boot_id', 'utf8').trim();
      startToken = `linux:${boot}:${ticks}`;
    } catch { /* An exiting process has not proved readiness. */ }
  }
  return evaluateDevelopmentRuntimeHostReadiness({ unit,
    host: readJson(join(descriptor.socketDir, 'runtime-host.json')),
    identity: readJson(join(descriptor.socketDir, 'runtime-host.identity.json')),
    listenerPid, listenerCgroup, startToken, expectedUnit: descriptor.unitNames.runtimeHost,
    generationBinary, sha256 });
}

export function developmentRuntimeStatus({ env = process.env } = {}) {
  const descriptor = developmentRuntimeDescriptor(env);
  const selectedGeneration = resolvedLink(descriptor.current);
  const executable = selectedGeneration
    ? join(selectedGeneration, 'bin', 'agent-browser')
    : null;
  const stableExecutable = existsSync(descriptor.executable) ? descriptor.executable : null;
  const launcher = stableExecutable ? readFileSync(stableExecutable, 'utf8') : null;
  const units = Object.fromEntries(
    descriptor.units.map((unit) => [unit, unitStatus(unit, env)]),
  );
  const laneManifest = readJson(descriptor.laneManifest);
  const runtimeHostIngress = readJson(descriptor.runtimeHostIngressState);
  const manifest = fetchJson(`http://127.0.0.1:${descriptor.dashboardPort}/api/runtime/manifest`);
  const backendManifest = fetchJson(`http://127.0.0.1:${descriptor.backendPort}/api/runtime/manifest`);
  const localIngressManifest = fetchJson('http://127.0.0.1/api/runtime/manifest', [
    `Host: ${descriptor.localHost}`,
  ]);
  const ports = {
    dashboard: listeningProcessId(descriptor.dashboardPort),
    backend: listeningProcessId(descriptor.backendPort),
    lane: listeningProcessId(descriptor.laneStreamPort),
  };
  const auth = {
    store: privateFileStatus(join(descriptor.authDir, 'dashboard-auth.json')),
    bootstrap: privateFileStatus(join(descriptor.authDir, 'dashboard-auth.env')),
  };
  const presentationProvider = doctorDevelopmentPresentationProvider({
    env,
    probe: probeDevelopmentPresentationProvider,
  }).status;
  const developmentSkill = developmentAgentSkillStatus({ env });
  const protectedLeaseAuthority = protectedLeaseAuthorityStatus(env);
  return {
    schemaVersion: DEVELOPMENT_RUNTIME_SCHEMA,
    descriptor,
    selectedGeneration,
    executable,
    stableExecutable,
    laneManifest,
    runtimeHostIngress,
    units,
    manifest,
    backendManifest,
    localIngressManifest,
    ports,
    auth,
    presentationProvider,
    developmentSkill,
    protectedLeaseAuthority,
    externalBrowserDiscovery: descriptor.externalBrowserDiscovery,
    generationMetadata: selectedGeneration ? readJson(join(selectedGeneration, 'generation.json')) : null,
    ready:
      Boolean(selectedGeneration) &&
      Boolean(executable) &&
      Boolean(stableExecutable) &&
      launcher?.includes(`exec ${shellQuote(executable)} "$@"`) === true &&
      launcher?.includes(`export AGENT_BROWSER_EXECUTABLE_PATH=${shellQuote(descriptor.browserExecutable)}`) === true &&
      laneManifest?.schemaVersion === 'agent-browser.session-supervisor.v1' &&
      laneManifest?.executablePath === executable &&
      runtimeHostIngress?.selectedBackend?.pid === units[descriptor.unitNames.runtimeHost]?.mainPid &&
      runtimeHostIngress?.selectedBackend?.binarySha256 === manifest?.executable?.sha256 &&
      Object.values(units).every((unit) => unit.activeState === 'active') &&
      developmentExternalDiscoveryChecks(units).every((item) => item.ok) &&
      protectedLeaseAuthority.ready &&
      manifest?.runtimeEnvironment === 'development' &&
      manifest?.executable?.path === executable,
  };
}

export function doctorDevelopmentRuntime({ env = process.env } = {}) {
  const status = developmentRuntimeStatus({ env });
  const presentationProviderDoctor = doctorDevelopmentPresentationProvider({
    env,
    probe: probeDevelopmentPresentationProvider,
  });
  const checks = [
    check('selected-generation', Boolean(status.selectedGeneration), status.selectedGeneration),
    check('stable-executable', Boolean(status.stableExecutable), status.stableExecutable),
    check('selected-executable', Boolean(status.executable), status.executable),
    check('lane-manifest', status.laneManifest?.executablePath === status.executable, status.laneManifest?.executablePath),
    check(
      'runtime-host-ingress',
      status.runtimeHostIngress?.selectedBackend?.pid === status.units[status.descriptor.unitNames.runtimeHost].mainPid &&
        status.runtimeHostIngress?.selectedBackend?.binarySha256 === status.manifest?.executable?.sha256,
      status.runtimeHostIngress?.selectedBackend || null,
    ),
    ...Object.entries(status.units).map(([name, unit]) =>
      check(`unit:${name}`, unit.activeState === 'active', unit.activeState),
    ),
    ...Object.entries(status.units).map(([name, unit]) =>
      check(`unit-executable:${name}`, unit.executable === status.executable, unit.executable),
    ),
    ...developmentExternalDiscoveryChecks(status.units),
    check('generation-external-browser-discovery',
      status.generationMetadata?.externalBrowserDiscovery === 'disabled',
      status.generationMetadata?.externalBrowserDiscovery),
    check('port:dashboard', status.ports.dashboard === status.units[status.descriptor.unitNames.dashboard].mainPid, status.ports.dashboard),
    check('port:backend', status.ports.backend === status.units[status.descriptor.unitNames.backend].mainPid, status.ports.backend),
    check('port:lane', status.ports.lane === status.units[status.descriptor.unitNames.runtimeHost].mainPid, status.ports.lane),
    check('auth:store', status.auth.store.private, status.auth.store),
    check('auth:bootstrap', status.auth.bootstrap.private, status.auth.bootstrap),
    check(
      'protected-lease-authority',
      status.protectedLeaseAuthority.ready,
      status.protectedLeaseAuthority.ready
        ? 'ready'
        : status.protectedLeaseAuthority.reasons.join(','),
    ),
    check('manifest-environment', status.manifest?.runtimeEnvironment === 'development', status.manifest?.runtimeEnvironment),
    check('manifest-executable', status.manifest?.executable?.path === status.executable, status.manifest?.executable?.path),
    check('browser-executable', executableFile(status.descriptor.browserExecutable), status.descriptor.browserExecutable),
    check('launcher-browser-executable',
      readFileSync(status.stableExecutable, 'utf8').includes(`export AGENT_BROWSER_EXECUTABLE_PATH=${shellQuote(status.descriptor.browserExecutable)}`),
      status.descriptor.browserExecutable,
    ),
    ...status.descriptor.units.map((name) => {
      const source = readFileSync(join(status.descriptor.systemdDir, name), 'utf8');
      return check(`unit-browser-executable:${name}`,
        source.includes(`Environment=AGENT_BROWSER_EXECUTABLE_PATH=${status.descriptor.browserExecutable}`),
        status.descriptor.browserExecutable,
      );
    }),
    check('backend-manifest', status.backendManifest?.runtimeEnvironment === 'development', status.backendManifest?.runtimeEnvironment),
    check('local-ingress', status.localIngressManifest?.runtimeEnvironment === 'development', status.localIngressManifest?.runtimeEnvironment),
    check('development-skill', status.developmentSkill.ready, status.developmentSkill.state),
    ...presentationProviderDoctor.checks,
  ];
  return { success: checks.every((item) => item.ok), checks, status };
}

export function renderDevelopmentLauncher(descriptor, generationBinary) {
  return `#!/usr/bin/env sh
set -eu
export HOME=${shellQuote(descriptor.pseudoHome)}
${descriptor.namespace ? `export AGENT_BROWSER_DEV_NAMESPACE=${shellQuote(descriptor.namespace)}\n` : ''}export AGENT_BROWSER_RUNTIME_ENVIRONMENT=development
export AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY=disabled
export AGENT_BROWSER_RUNTIME_HOST=1
export AGENT_BROWSER_SOCKET_DIR=${shellQuote(descriptor.socketDir)}
export AGENT_BROWSER_RUNTIME_HOST_INGRESS_STATE=${shellQuote(descriptor.runtimeHostIngressState)}
export AGENT_BROWSER_DASHBOARD_AUTH_DIR=${shellQuote(descriptor.authDir)}
export AGENT_BROWSER_PRESENTATION_PROVIDER_INVENTORY_PATH=${shellQuote(descriptor.presentationProvider.inventoryPath)}
export AGENT_BROWSER_PRESENTATION_WARM_MINIMUM=${descriptor.presentationProvider.warmSlots}
export AGENT_BROWSER_PRESENTATION_HARD_MAXIMUM=${descriptor.presentationProvider.hardMaxSlots}
export AGENT_BROWSER_PRESENTATION_HUMAN_RESERVE=1
export AGENT_BROWSER_PRESENTATION_RECOVERY_RESERVE=1
if [ -z "\${AGENT_BROWSER_EXECUTABLE_PATH:-}" ]; then
  export AGENT_BROWSER_EXECUTABLE_PATH=${shellQuote(descriptor.browserExecutable)}
fi
exec ${shellQuote(generationBinary)} "$@"
`;
}

export function publishDevelopmentRuntimeIngress({
  descriptor,
  generationId,
  generationBinary,
  sha256,
}) {
  const host = readJson(join(descriptor.socketDir, 'runtime-host.json'));
  const identity = readJson(join(descriptor.socketDir, 'runtime-host.identity.json'));
  if (
    !host ||
    !identity ||
    host.pid !== identity.pid ||
    identity.executablePath !== generationBinary ||
    host.executableGeneration !== sha256 ||
    typeof host.socketIdentity !== 'string' ||
    !host.socketIdentity
  ) {
    throw new Error('Development runtime host identity is incomplete or does not match the selected generation');
  }
  const bootId = /^linux:([^:]+):/.exec(identity.startToken || '')?.[1] || null;
  const bootEpoch = bootId ? `linux:${bootId}` : null;
  if (!bootEpoch) throw new Error('Development runtime host start token does not identify the current boot');
  const previous = readJson(descriptor.runtimeHostIngressState);
  if (previous?.activeTransactionId) {
    throw new Error(`Development runtime ingress has an active transaction: ${previous.activeTransactionId}`);
  }
  const registry = {
    schemaVersion: 'agent-browser.runtime-host-ingress.v1',
    revision: Number.isSafeInteger(previous?.revision) ? previous.revision + 1 : 1,
    bootEpoch,
    activeTransactionId: null,
    selectedBackend: {
      topology: 'single_host',
      generationId,
      socketDir: descriptor.socketDir,
      binarySha256: sha256,
      hostId: host.hostId,
      pid: host.pid,
      socketIdentity: host.socketIdentity,
    },
    candidateBackend: null,
    fallbackBackend: null,
  };
  writeJsonAtomic(descriptor.runtimeHostIngressState, registry);
  return registry;
}

function resolveDevelopmentBrowserExecutable(env, pseudoHome) {
  // Development profiles live in the Linux pseudo-home. Pin a compatible host
  // executable so ambient production or Windows manifests cannot select one.
  const explicit = env.AGENT_BROWSER_DEV_BROWSER_EXECUTABLE;
  if (explicit && !explicit.startsWith('/')) {
    throw new Error(`Development browser executable must be an absolute path: ${explicit}`);
  }
  const candidates = explicit
    ? [explicit]
    : [
        '/opt/google/chrome/chrome',
        '/usr/bin/google-chrome',
        '/usr/bin/google-chrome-stable',
        '/usr/bin/chromium',
        '/usr/bin/chromium-browser',
      ];
  const selected = candidates.find(executableFile);
  if (!selected) {
    throw new Error(`Development browser executable is unavailable: ${candidates.join(', ')}`);
  }
  if (selected.toLowerCase().endsWith('.exe') && !pseudoHome.startsWith('/mnt/')) {
    throw new Error(`Development browser executable is incompatible with the Linux profile root: ${selected}`);
  }
  return resolve(selected);
}

function executableFile(path) {
  try {
    accessSync(path, fsConstants.X_OK);
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

export function garbageCollectDevelopmentRuntime({ env = process.env, retain = 2 } = {}) {
  const descriptor = developmentRuntimeDescriptor(env);
  const selected = resolvedLink(descriptor.current);
  const liveExecutables = liveDevelopmentExecutables(descriptor.generations);
  if (!existsSync(descriptor.generations)) return { success: true, removed: [], retained: [] };
  const generations = readdirSync(descriptor.generations, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => join(descriptor.generations, entry.name))
    .sort((left, right) => lstatSync(right).mtimeMs - lstatSync(left).mtimeMs);
  const protectedPaths = new Set([selected, ...liveExecutables].filter(Boolean));
  for (const path of generations.slice(0, Math.max(0, retain))) protectedPaths.add(path);
  const removed = [];
  for (const path of generations) {
    if (protectedPaths.has(path)) continue;
    if (descriptor.namespace && readJson(join(path, 'generation.json'))?.namespace !== descriptor.namespace) continue;
    rmSync(path, { recursive: true, force: true });
    removed.push(path);
  }
  return { success: true, removed, retained: generations.filter((path) => !removed.includes(path)), liveExecutables };
}

/** Snapshot default-development custody and attached identities during an isolated namespace install. */
export function defaultDevelopmentSnapshot(env = process.env) {
  const baselineEnv = Object.fromEntries(Object.entries(env).filter(([key]) =>
    !key.startsWith('AGENT_BROWSER_DEV_') ||
    ['AGENT_BROWSER_DEV_USER_HOME', 'AGENT_BROWSER_DEV_SKIP_SYSTEMD', 'AGENT_BROWSER_DEV_BROWSER_EXECUTABLE',
      'AGENT_BROWSER_DEV_OPERATOR_USER'].includes(key)));
  const descriptor = developmentRuntimeDescriptor(baselineEnv);
  return {
    selectedGeneration: resolvedLink(descriptor.current),
    processes: env.AGENT_BROWSER_DEV_SKIP_SYSTEMD === '1' ? [] : processCensusUnder(descriptor.generations),
    custody: {
      executable: fileIdentity(descriptor.executable),
      laneManifest: fileIdentity(descriptor.laneManifest),
      ingress: fileIdentity(descriptor.runtimeHostIngressState),
      remoteViewHandoffs: fileIdentity(join(descriptor.stateDir, 'service/remote-view-handoffs.json')),
      ...Object.fromEntries(descriptor.units.map((name) => [name, fileIdentity(join(descriptor.systemdDir, name))])),
    },
    units: Object.fromEntries(descriptor.units.map((name) => [name, unitStatus(name, baselineEnv)])),
    serviceIdentities: serviceIdentityProjection(join(descriptor.stateDir, 'service/state.json')),
  };
}

export function assertDefaultDevelopmentUnchanged(before, after) {
  if (JSON.stringify({ ...before, processes: undefined, serviceIdentities: undefined }) !==
      JSON.stringify({ ...after, processes: undefined, serviceIdentities: undefined })) {
    throw new Error('Default development runtime custody changed during namespaced activation');
  }
  assertStableProcessCensusPreserved(before.processes, after.processes);
  assertIdentityProjectionPreserved(before.serviceIdentities, after.serviceIdentities);
}

export function productionSnapshot(env = process.env) {
  if (env.AGENT_BROWSER_DEV_SKIP_SYSTEMD === '1') return { fixture: true };
  const productionCurrent = resolve(
    env.AGENT_BROWSER_PRODUCTION_CURRENT || join(homedir(), '.local', 'lib', 'agent-browser', 'current'),
  );
  const productionState = resolve(
    env.AGENT_BROWSER_PRODUCTION_STATE || join(homedir(), '.agent-browser'),
  );
  return {
    selectedGeneration: resolvedLink(productionCurrent),
    processes: processCensusUnder(join(dirname(productionCurrent), 'generations')),
    dashboardManifest: fetchJson('http://127.0.0.1:4848/api/runtime/manifest'),
    stateFiles: {
      serviceState: fileIdentity(join(productionState, 'service', 'state.json')),
      remoteViewHandoffs: fileIdentity(
        join(productionState, 'service', 'remote-view-handoffs.json'),
      ),
    },
    serviceIdentities: serviceIdentityProjection(
      join(productionState, 'service', 'state.json'),
    ),
    units: Object.fromEntries(
      [
        'agent-browser-runtime-host.service',
        'agent-browser-dashboard-backend.service',
        'agent-browser-dashboard.service',
      ].map((unit) => [unit, unitStatus(unit, env)]),
    ),
  };
}

export function assertProductionUnchanged(before, after) {
  if (before.fixture && after.fixture) return;
  const invariantBefore = {
    ...before,
    processes: undefined,
    stateFiles: { remoteViewHandoffs: before.stateFiles?.remoteViewHandoffs || null },
    serviceIdentities: undefined,
  };
  const invariantAfter = {
    ...after,
    processes: undefined,
    stateFiles: { remoteViewHandoffs: after.stateFiles?.remoteViewHandoffs || null },
    serviceIdentities: undefined,
  };
  if (JSON.stringify(invariantBefore) !== JSON.stringify(invariantAfter)) {
    throw new Error(`Production runtime changed during development activation: ${JSON.stringify({ before, after })}`);
  }
  assertStableProcessCensusPreserved(before.processes || [], after.processes || []);
  assertIdentityProjectionPreserved(before.serviceIdentities, after.serviceIdentities);
}

function binaryVersion(binary, env) {
  const output = execFileSync(binary, ['--version'], { env, encoding: 'utf8' }).trim();
  return output.replace(/^agent-browser\s+/i, '').replace(/[^0-9A-Za-z.+-]/g, '_') || 'unknown';
}

function systemctl(args, env) {
  return execFileSync('systemctl', ['--user', ...args], { env, encoding: 'utf8' });
}

function unitStatus(unit, env) {
  if (env.AGENT_BROWSER_DEV_SKIP_SYSTEMD === '1') {
    return { loadState: 'fixture', activeState: 'fixture', mainPid: null, activeEnterTimestamp: null };
  }
  try {
    const output = systemctl(
      ['show', unit, '--property=LoadState', '--property=ActiveState', '--property=MainPID', '--property=ActiveEnterTimestamp'],
      env,
    );
    const values = Object.fromEntries(output.trim().split(/\r?\n/).map((line) => {
      const index = line.indexOf('=');
      return [line.slice(0, index), line.slice(index + 1)];
    }));
    const mainPid = Number(values.MainPID || 0) || null;
    return {
      loadState: values.LoadState || 'unknown',
      activeState: values.ActiveState || 'unknown',
      mainPid,
      executable: mainPid ? processExecutable(mainPid) : null,
      externalBrowserDiscovery: observeDevelopmentExternalDiscovery(mainPid),
      activeEnterTimestamp: values.ActiveEnterTimestamp || null,
    };
  } catch (error) {
    return { loadState: 'unknown', activeState: 'unknown', mainPid: null, error: String(error.message || error) };
  }
}

/** Read only the exact attached process's discovery policy; never return its other environment values. */
export function observeDevelopmentExternalDiscovery(pid) {
  if (!Number.isSafeInteger(pid) || pid < 1) return { state: 'unavailable', policy: null };
  let fd;
  try {
    fd = openSync(`/proc/${pid}/environ`, 'r');
    const buffer = Buffer.alloc(65_537);
    let length = 0;
    while (length < buffer.length) {
      const count = readSync(fd, buffer, length, buffer.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length === buffer.length) return { state: 'unavailable', policy: null };
    const prefix = 'AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY=';
    const matches = buffer.subarray(0, length).toString('utf8').split('\0')
      .filter((entry) => entry.startsWith(prefix));
    if (matches.length === 0) return { state: 'missing', policy: null };
    const value = matches[0].slice(prefix.length);
    if (matches.length !== 1 || !['enabled', 'disabled'].includes(value)) {
      return { state: 'invalid', policy: null };
    }
    return { state: 'observed', policy: value };
  } catch {
    return { state: 'unavailable', policy: null };
  } finally {
    if (fd !== undefined) closeSync(fd);
  }
}

/** Doctor admission uses live process readback, including missing/invalid-policy failures. */
export function developmentExternalDiscoveryChecks(units) {
  return Object.entries(units).map(([name, unit]) => check(
    `unit-external-browser-discovery:${name}`,
    unit.externalBrowserDiscovery?.state === 'observed' &&
      unit.externalBrowserDiscovery.policy === 'disabled',
    unit.externalBrowserDiscovery ?? { state: 'unavailable', policy: null },
  ));
}

function protectedLeaseAuthorityStatus(env) {
  const unit = systemUnitStatus(PROTECTED_LEASE_AUTHORITY_SOCKET_UNIT, env);
  const socket = unixSocketStatus(PROTECTED_LEASE_AUTHORITY_SOCKET_PATH);
  const operatorGroupId = groupId(PROTECTED_LEASE_AUTHORITY_OPERATOR_GROUP, env);
  return {
    unitName: PROTECTED_LEASE_AUTHORITY_SOCKET_UNIT,
    socketPath: PROTECTED_LEASE_AUTHORITY_SOCKET_PATH,
    operatorGroup: PROTECTED_LEASE_AUTHORITY_OPERATOR_GROUP,
    operatorGroupId,
    unit,
    socket,
    ...evaluateProtectedLeaseAuthorityStatus({ unit, socket, operatorGroupId }),
  };
}

function systemUnitStatus(unit, env) {
  try {
    const output = execFileSync(
      'systemctl',
      [
        'show',
        unit,
        '--property=LoadState',
        '--property=ActiveState',
        '--property=UnitFileState',
        '--property=FragmentPath',
      ],
      { env, encoding: 'utf8' },
    );
    const values = Object.fromEntries(output.trim().split(/\r?\n/).map((line) => {
      const index = line.indexOf('=');
      return [line.slice(0, index), line.slice(index + 1)];
    }));
    return {
      loadState: values.LoadState || 'unknown',
      activeState: values.ActiveState || 'unknown',
      unitFileState: values.UnitFileState || 'unknown',
      fragmentPath: values.FragmentPath || null,
    };
  } catch (error) {
    return {
      loadState: 'unknown',
      activeState: 'unknown',
      unitFileState: 'unknown',
      fragmentPath: null,
      error: String(error.message || error),
    };
  }
}

function unixSocketStatus(path) {
  try {
    const stats = statSync(path);
    return {
      exists: true,
      socket: stats.isSocket(),
      uid: stats.uid,
      gid: stats.gid,
      mode: stats.mode & 0o777,
    };
  } catch {
    return { exists: false, socket: false, uid: null, gid: null, mode: null };
  }
}

function groupId(group, env) {
  try {
    const record = execFileSync('getent', ['group', group], { env, encoding: 'utf8' }).trim();
    const value = Number(record.split(':')[2]);
    return Number.isInteger(value) && value > 0 ? value : null;
  } catch {
    return null;
  }
}

function waitForDevelopmentManifest(descriptor, generationBinary, env) {
  const deadline = Date.now() + Number(env.AGENT_BROWSER_DEV_START_TIMEOUT_MS || 20_000);
  let last = null;
  while (Date.now() < deadline) {
    last = fetchJson(`http://127.0.0.1:${descriptor.dashboardPort}/api/runtime/manifest`);
    const unitsReady = descriptor.units.every(
      (unit) => unitStatus(unit, env).activeState === 'active',
    );
    if (
      unitsReady &&
      last?.runtimeEnvironment === 'development' &&
      last?.executable?.path === generationBinary
    ) return;
    execFileSync('sleep', ['0.25']);
  }
  throw new Error(`Development dashboard did not publish the selected environment and executable: ${JSON.stringify(last)}`);
}

function fetchJson(url, headers = []) {
  try {
    const args = ['--fail', '--silent', '--show-error'];
    for (const header of headers) args.push('--header', header);
    args.push(url);
    return JSON.parse(execFileSync('curl', args, {
      encoding: 'utf8',
      timeout: 2500,
      stdio: ['ignore', 'pipe', 'ignore'],
    }));
  } catch {
    return null;
  }
}

function listeningProcessId(port) {
  try {
    const output = execFileSync('ss', ['-ltnp'], { encoding: 'utf8' });
    const line = output.split(/\r?\n/).find((candidate) =>
      new RegExp(`:${port}\\s`).test(candidate),
    );
    return Number(line?.match(/pid=(\d+)/)?.[1] || 0) || null;
  } catch {
    return null;
  }
}

function processExecutable(pid) {
  try {
    return readlinkSync(`/proc/${pid}/exe`);
  } catch {
    return null;
  }
}

function privateFileStatus(path) {
  try {
    const stats = statSync(path);
    return { path, exists: stats.isFile(), mode: stats.mode & 0o777, private: stats.isFile() && (stats.mode & 0o077) === 0 };
  } catch {
    return { path, exists: false, mode: null, private: false };
  }
}

function fileIdentity(path) {
  try {
    const bytes = readFileSync(path);
    return {
      path,
      bytes: bytes.length,
      sha256: createHash('sha256').update(bytes).digest('hex'),
    };
  } catch {
    return null;
  }
}

function serviceIdentityProjection(path) {
  try {
    const state = JSON.parse(readFileSync(path, 'utf8'));
    return {
      browsers: Object.entries(state.browsers || {}).map(([key, browser]) => ({
        key,
        id: browser.id || null,
        pid: browser.pid || null,
        profileId: browser.profileId || null,
        host: browser.host || null,
        activeSessionIds: [...(browser.activeSessionIds || [])].sort(),
      })).sort((left, right) => left.key.localeCompare(right.key)),
      sessions: Object.entries(state.sessions || {}).map(([key, session]) => ({
        key,
        id: session.id || null,
        profileId: session.profileId || null,
        browserIds: [...(session.browserIds || [])].sort(),
      })).sort((left, right) => left.key.localeCompare(right.key)),
    };
  } catch {
    return null;
  }
}

function assertIdentityProjectionPreserved(before, after) {
  if (!before || !after) {
    if (before !== after) throw new Error('Production service identity projection became unavailable');
    return;
  }
  for (const collection of ['browsers', 'sessions']) {
    const current = new Map(after[collection].map((item) => [item.key, item]));
    for (const item of before[collection]) {
      if (JSON.stringify(current.get(item.key)) !== JSON.stringify(item)) {
        throw new Error(`Production ${collection} identity changed during development activation: ${item.key}`);
      }
    }
  }
}

function assertStableProcessCensusPreserved(before, after) {
  const current = new Map(after.map((item) => [item.pid, item]));
  for (const item of before) {
    if (item.stable === false) continue;
    const observed = current.get(item.pid);
    if (!observed || observed.startToken !== item.startToken || observed.executable !== item.executable) {
      throw new Error(`Production stable process changed during development activation: ${item.pid}`);
    }
  }
}

function readJson(path) {
  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch {
    return null;
  }
}

function liveDevelopmentExecutables(generationsRoot) {
  const live = new Set();
  if (!existsSync('/proc')) return [];
  for (const entry of readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const target = readlinkSync(join('/proc', entry, 'exe'));
      if (target.startsWith(`${generationsRoot}/`)) live.add(dirname(dirname(target)));
    } catch {
      // Processes can exit while the census is being read.
    }
  }
  return [...live];
}

function processCensusUnder(generationsRoot) {
  const processes = [];
  if (!existsSync('/proc')) return processes;
  const uptimeSeconds = Number(readFileSync('/proc/uptime', 'utf8').split(/\s+/)[0] || 0);
  let clockTicks = 100;
  try {
    clockTicks = Number(execFileSync('getconf', ['CLK_TCK'], { encoding: 'utf8' }).trim()) || 100;
  } catch {
    // Linux defaults to 100 ticks on the supported workstation when getconf is unavailable.
  }
  for (const entry of readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const executable = readlinkSync(join('/proc', entry, 'exe'));
      if (!executable.startsWith(`${generationsRoot}/`)) continue;
      const stat = readFileSync(join('/proc', entry, 'stat'), 'utf8').trim().split(/\s+/);
      const startToken = stat[21] || null;
      const ageSeconds = startToken === null
        ? null
        : Math.max(0, uptimeSeconds - Number(startToken) / clockTicks);
      processes.push({
        pid: Number(entry),
        startToken,
        executable,
        stable: ageSeconds !== null && ageSeconds >= 30,
      });
    } catch {
      // Processes can exit while the census is being read.
    }
  }
  return processes.sort((left, right) => left.pid - right.pid);
}

function replaceSymlink(path, target) {
  const temporary = `${path}.next-${process.pid}`;
  rmSync(temporary, { force: true, recursive: true });
  symlinkSync(target, temporary);
  renameSync(temporary, path);
}

function captureStableExecutable(path) {
  try {
    if (lstatSync(path).isSymbolicLink()) return { kind: 'symlink', target: readlinkSync(path) };
    return { kind: 'file', content: readFileSync(path, 'utf8') };
  } catch {
    return null;
  }
}

function restoreStableExecutable(path, captured) {
  if (captured === null) {
    rmSync(path, { force: true });
  } else if (captured.kind === 'symlink') {
    replaceSymlink(path, captured.target);
  } else {
    writeFileAtomic(path, captured.content, 0o755);
  }
}

function removeLink(path) {
  try {
    if (lstatSync(path).isSymbolicLink()) rmSync(path, { force: true });
  } catch {
    // Missing links are already removed.
  }
}

function resolvedLink(path) {
  try {
    return resolve(dirname(path), readlinkSync(path));
  } catch {
    return null;
  }
}

function writeFileAtomic(path, content, mode) {
  const temporary = `${path}.next-${process.pid}`;
  writeFileSync(temporary, content, { mode });
  renameSync(temporary, path);
}

function writeJsonAtomic(path, value) {
  writeFileAtomic(path, `${JSON.stringify(value, null, 2)}\n`, 0o600);
}

function captureDevelopmentLaneManifests(directory) {
  const manifests = new Map();
  if (!existsSync(directory)) return manifests;
  for (const name of readdirSync(directory)) {
    if (!name.endsWith('.json')) continue;
    const path = join(directory, name);
    if (lstatSync(path).isFile()) manifests.set(path, readFileSync(path, 'utf8'));
  }
  return manifests;
}

function restoreDevelopmentLaneManifests(paths, previous) {
  for (const path of paths) {
    const content = previous.get(path);
    if (content === undefined) rmSync(path, { force: true });
    else writeFileAtomic(path, content, 0o600);
  }
}

function check(name, ok, observed) {
  return { name, ok, observed: observed ?? null };
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", `'"'"'`)}'`;
}
