#!/usr/bin/env node

import assert from 'node:assert/strict';
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readlinkSync,
  readdirSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const repoRoot = resolve(import.meta.dirname, '..');
const fixtureRoot = mkdtempSync(join(tmpdir(), 'agent-browser-workstation-install-'));
const fakeBin = join(fixtureRoot, 'bin');
const commandLog = join(fixtureRoot, 'commands.jsonl');
const installRoot = join(fixtureRoot, 'workstation');
const failedInstallRoot = join(fixtureRoot, 'failed-workstation');
const lockedInstallRoot = join(fixtureRoot, 'locked-workstation');
const drainBlockedRoot = join(fixtureRoot, 'drain-blocked-workstation');
const home = join(fixtureRoot, 'home');
const xdgRoot = join(fixtureRoot, 'xdg');
const agentBrowser = resolveAgentBrowser();

const expectedUnits = new Set([
  'agent-browser-dashboard-backend.service',
  'agent-browser-dashboard.service',
  'agent-browser-runtime-interlock.service',
  'agent-browser-runtime-interlock.timer',
  'agent-browser-guacamole-postgres-backup.service',
  'agent-browser-guacamole-postgres-backup.timer',
]);

mkdirSync(fakeBin, { recursive: true });
mkdirSync(home, { recursive: true });
writeFileSync(commandLog, '');
installCommandShim('systemctl');
installCommandShim('sudo');

try {
  assertWorkstationInterface();
  const beforeDryRun = treeManifest(fixtureRoot, ignoredFixturePaths());
  const dryRun = runInstaller(installRoot, ['--dry-run', '--json']);
  assert.equal(
    dryRun.status,
    0,
    `workstation install dry-run must succeed:\n${dryRun.stdout}${dryRun.stderr}`,
  );
  assertJsonSuccess(dryRun.stdout, 'dry-run');
  assert.deepEqual(
    treeManifest(fixtureRoot, ignoredFixturePaths()),
    beforeDryRun,
    'dry-run must not mutate HOME, XDG paths, or the workstation install root',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'dry-run must not invoke systemctl or sudo',
  );

  const firstApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.equal(
    firstApply.status,
    0,
    `workstation install apply must succeed:\n${firstApply.stdout}${firstApply.stderr}`,
  );
  assertJsonSuccess(firstApply.stdout, 'first apply');
  assert.ok(existsSync(installRoot), 'apply must create the isolated workstation root');

  const generationStore = join(
    installRoot,
    '.local',
    'lib',
    'agent-browser',
    'generations',
  );
  const currentSelector = join(
    installRoot,
    '.local',
    'lib',
    'agent-browser',
    'current',
  );
  const stableBinary = join(installRoot, '.local', 'bin', 'agent-browser');
  const transactionStore = join(
    installRoot,
    '.agent-browser',
    'runtime-adoption',
    'transactions',
  );
  assert.equal(
    lstatSync(currentSelector).isSymbolicLink(),
    true,
    'apply must atomically select one immutable runtime generation',
  );
  assert.equal(
    lstatSync(stableBinary).isSymbolicLink(),
    true,
    'the stable command must resolve through the current-generation selector',
  );
  const generationIds = readdirSync(generationStore);
  assert.equal(generationIds.length, 1, 'first apply must create exactly one generation');
  const selectedGenerationRoot = join(generationStore, generationIds[0]);
  for (const relative of [
    join('bin', 'agent-browser'),
    join('support', 'manifest.json'),
    join('units', 'agent-browser-dashboard-backend.service'),
    join('units', 'agent-browser-dashboard.service'),
    'generation.json',
  ]) {
    assert.ok(
      existsSync(join(selectedGenerationRoot, relative)),
      `selected generation must contain ${relative}`,
    );
  }
  for (const entry of treeManifest(selectedGenerationRoot)) {
    assert.equal(
      entry.mode & 0o222,
      0,
      `immutable generation entry must not be writable: ${entry.path}`,
    );
  }

  assert.equal(readdirSync(transactionStore).length, 1, 'first apply must write one transaction');
  assertTransactionTerminal(transactionStore, 'old_generation_retirable');
  const transactionStatus = runInstaller(installRoot, ['status', '--json']);
  assert.equal(transactionStatus.status, 0, transactionStatus.stderr);
  const transactionStatusPayload = JSON.parse(transactionStatus.stdout);
  assert.equal(
    transactionStatusPayload.latestTransaction.state,
    'old_generation_retirable',
  );
  assert.equal(transactionStatusPayload.admissionDraining, false);
  assert.equal(transactionStatusPayload.selectedGenerationId, generationIds[0]);
  assert.doesNotMatch(
    transactionStatus.stdout,
    /profileIdentityDigest|cdpEndpoint|immutableInstallationPath|transactionPath/,
    'public transaction status must omit private runtime and filesystem evidence',
  );
  const firstManifest = treeManifest(installRoot, new Set([transactionStore]));
  const installedLinks = firstManifest.filter((entry) => entry.type === 'symlink');
  assert.ok(installedLinks.length > 0, 'immutable generation selection must use stable links');
  for (const entry of installedLinks) {
    const linkPath = join(installRoot, entry.path);
    const target = readlinkSync(linkPath);
    assert.equal(
      target.startsWith('/'),
      false,
      `${entry.path} must not use an absolute link target`,
    );
    assert.ok(
      resolve(dirname(linkPath), target).startsWith(`${installRoot}/`),
      `${entry.path} must resolve only inside the installed workstation root`,
    );
  }
  const installedFiles = regularFiles(installRoot);
  const installedBinary = stableBinary;
  assert.ok(existsSync(installedBinary), 'apply must install the agent-browser executable');
  assert.notEqual(
    statSync(installedBinary).mode & 0o111,
    0,
    'the installed agent-browser payload must be executable',
  );
  for (const unit of expectedUnits) {
    const unitPath = join(installRoot, '.config', 'systemd', 'user', unit);
    assert.ok(existsSync(unitPath), `apply must install ${unit}`);
    assert.equal(
      lstatSync(unitPath).isSymbolicLink(),
      true,
      `${unit} must resolve through the current-generation selector`,
    );
  }
  const dashboardUnit = join(
    installRoot,
    '.config',
    'systemd',
    'user',
    'agent-browser-dashboard.service',
  );
  const dashboardSource = readFileSync(dashboardUnit, 'utf8');
  assert.match(
    dashboardSource,
    /EnvironmentFile=-%h\/.agent-browser\/\.env/,
    'dashboard service must load the managed agent-browser environment',
  );
  assert.match(
    dashboardSource,
    /AGENT_BROWSER_DASHBOARD_INGRESS=1/,
    'dashboard service must remain the stable ingress process',
  );
  const dashboardBackendSource = readFileSync(
    join(installRoot, '.config', 'systemd', 'user', 'agent-browser-dashboard-backend.service'),
    'utf8',
  );
  assert.match(
    dashboardBackendSource,
    /AGENT_BROWSER_DASHBOARD=1/,
    'dashboard backend service must run the generation-specific application server',
  );
  assert.match(
    dashboardBackendSource,
    /AGENT_BROWSER_DASHBOARD_BACKEND_ONLY=1/,
    'dashboard backend must not race stable ingress for the shared service relay',
  );
  const payloadManifestPath = installedFiles.find(
    (path) => basename(path) === 'manifest.json' && !path.includes(`${join('guacamole')}/`),
  );
  const payloadRoot = dirname(payloadManifestPath);
  const payloadManifest = JSON.parse(readFileSync(payloadManifestPath, 'utf8'));
  assert.equal(
    sha256(installedBinary),
    payloadManifest.binary.sha256,
    'payload manifest must bind the installed binary hash',
  );
  for (const file of payloadManifest.controllerAssets.files) {
    const path = join(payloadRoot, file.path);
    assert.equal(sha256(path), file.sha256, `controller asset hash mismatch: ${file.path}`);
  }
  const guacamoleRoot = join(payloadRoot, 'guacamole');
  assert.equal(
    sha256(join(guacamoleRoot, 'manifest.json')),
    payloadManifest.guacamoleBundleManifestSha256,
    'payload manifest must bind the Guacamole manifest hash',
  );
  for (const file of payloadManifest.guacamoleBundle.files) {
    assert.equal(
      sha256(join(guacamoleRoot, file.path)),
      file.sha256,
      `Guacamole asset hash mismatch: ${file.path}`,
    );
  }

  const payloadFiles = installedFiles.filter((path) => (
    path.includes(`${join('lib', 'agent-browser')}/`)
    && !expectedUnits.has(basename(path))
  ));
  assert.ok(
    payloadFiles.length > 0,
    'apply must install at least one immutable runtime payload under lib/agent-browser',
  );

  const textArtifacts = installedFiles.filter((path) => (
    expectedUnits.has(basename(path))
    || /\.(?:json|service|sh|timer|txt|yaml|yml)$/.test(path)
  ));
  for (const path of textArtifacts) {
    const source = readFileSync(path, 'utf8');
    assert.equal(
      source.includes(repoRoot),
      false,
      `${path} must not reference the source checkout`,
    );
    assert.equal(
      /\bpnpm\b/i.test(source),
      false,
      `${path} must not require pnpm`,
    );
  }

  const interlockUnit = join(
    installRoot,
    '.config',
    'systemd',
    'user',
    'agent-browser-runtime-interlock.service',
  );
  const interlockSource = readFileSync(interlockUnit, 'utf8');
  assert.match(
    interlockSource,
    /ExecStart=.+/,
    'the installed interlock unit must declare an executable payload',
  );
  assert.equal(
    /WorkingDirectory=/.test(interlockSource),
    false,
    'the installed interlock unit must not depend on a mutable working directory',
  );
  for (const path of installedFiles.filter((candidate) => candidate.endsWith('.service'))) {
    const unitSource = readFileSync(path, 'utf8');
    const execStart = unitSource.match(/^ExecStart=(.+)$/m)?.[1];
    assert.ok(execStart, `${path} must declare ExecStart`);
    assert.ok(
      execStart.startsWith(installRoot),
      `${path} must execute only the isolated installed payload`,
    );
  }

  const routeOpener = installedFiles.find(
    (path) => basename(path) === 'open-rdp-guac-route-displays.js',
  );
  const routeOpenerSource = readFileSync(routeOpener, 'utf8');
  assert.equal(
    routeOpenerSource.includes('${args.join'),
    false,
    'route-opener failures must not echo reversible authentication arguments',
  );
  assert.match(
    routeOpenerSource,
    /'set',\s*'headers'/,
    'Guacamole route sessions must apply header authentication before navigation',
  );
  assert.equal(
    /'eval'|--base64|--stdin/.test(routeOpenerSource),
    false,
    'Guacamole authentication must not inject tokens through eval arguments or input',
  );
  assert.match(
    routeOpenerSource,
    /waitForRouteDisplay/,
    'long-lived route navigation must require positive Xorg display proof',
  );
  assert.equal(
    /if \(inspection\.success && displayName\)/.test(routeOpenerSource),
    false,
    'route A readiness must not deadlock on two-route global readiness',
  );
  assert.match(
    routeOpenerSource,
    /AGENT_BROWSER_REMOTE_VIEW_SCRIPT_ROOT/,
    'the installed route opener must resolve its inspector from the installed script root',
  );
  assert.match(
    routeOpenerSource,
    /AGENT_BROWSER_GUACAMOLE_BASE_URL/,
    'database-derived route URLs must use an explicit Guacamole base URL',
  );
  assert.equal(
    /const baseUrl = process\.env\.AGENT_BROWSER_REMOTE_VIEW_URL/.test(routeOpenerSource),
    false,
    'a selected route viewer URL must never become the base for another route',
  );
  assert.match(
    routeOpenerSource,
    /Object\.hasOwn\(process\.env, key\)/,
    'an explicitly cleared route-pool env value must not reload stale persisted state',
  );
  assert.match(
    routeOpenerSource,
    /'--data-binary', '@-'/,
    'Guacamole password fallback must pass form data through stdin',
  );
  assert.equal(
    routeOpenerSource.includes('`password=${auth.password}`'),
    false,
    'Guacamole passwords must not appear in curl argv',
  );
  const readinessScript = installedFiles.find(
    (path) => basename(path) === 'smoke-rdp-guac-route-pool-readiness.js',
  );
  const readinessSource = readFileSync(readinessScript, 'utf8');
  assert.match(readinessSource, /'--data-binary',\s*'@-'/);
  assert.equal(readinessSource.includes('`password=${password}`'), false);
  const routeSync = installedFiles.find(
    (path) => basename(path) === 'sync-rdp-guac-route-specific-user-pool.sh',
  );
  const routeSyncSource = readFileSync(routeSync, 'utf8');
  assert.match(routeSyncSource, /python3 \/dev\/fd\/3 3<<'PY'/);
  assert.equal(
    /python3 - \\\n[^]*?\$PASS_A/.test(routeSyncSource),
    false,
    'route passwords must not be passed to Python through argv',
  );
  assert.match(
    routeOpenerSource,
    /agentBrowserTimeoutMs[^]*?\?\? 600000/,
    'slow route-browser commands must retain the bounded ten-minute budget',
  );
  const agentHome = join(home, '.agent-browser');
  mkdirSync(agentHome, { recursive: true });
  writeFileSync(
    join(agentHome, '.env'),
    `AGENT_BROWSER_RDP_ROUTE_POOL_JSON=${JSON.stringify([
      { id: 'legacy-a', routeId: 'guacamole:99', frameUrl: 'http://legacy/a' },
      { id: 'legacy-b', routeId: 'guacamole:100', frameUrl: 'http://legacy/b' },
    ])}\n`,
  );
  writeFileSync(
    join(fakeBin, 'docker'),
    '#!/usr/bin/env bash\nprintf "1\\tAgent Browser RDP Route A\\n2\\tAgent Browser RDP Route B\\n"\n',
  );
  chmodSync(join(fakeBin, 'docker'), 0o755);
  const persistedRouteDriftProbe = spawnSync(
    process.execPath,
    [routeOpener, '--dry-run'],
    {
      encoding: 'utf8',
      env: {
        ...process.env,
        HOME: home,
        PATH: `${fakeBin}:/usr/bin:/bin`,
        AGENT_BROWSER_RDP_ROUTE_POOL_JSON: '',
        AGENT_BROWSER_GUACAMOLE_BASE_URL: 'http://127.0.0.1:8092/guacamole/',
      },
    },
  );
  assert.equal(
    persistedRouteDriftProbe.status,
    0,
    `${persistedRouteDriftProbe.stdout}${persistedRouteDriftProbe.stderr}`,
  );
  assert.deepEqual(
    JSON.parse(persistedRouteDriftProbe.stdout).selectedRoutes.map(
      (route) => route.routeId,
    ),
    ['guacamole:1', 'guacamole:2'],
    'an explicit empty route-pool env value must suppress stale persisted route JSON',
  );
  const installedBinaryInode = statSync(installedBinary).ino;
  const routeOpenerInode = statSync(routeOpener).ino;

  const secondApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.equal(
    secondApply.status,
    0,
    `second workstation install apply must succeed:\n${secondApply.stdout}${secondApply.stderr}`,
  );
  assertJsonSuccess(secondApply.stdout, 'second apply');
  assert.deepEqual(
    treeManifest(installRoot, new Set([transactionStore])),
    firstManifest,
    'a second apply must leave payload byte content and file modes unchanged',
  );
  assert.equal(readdirSync(transactionStore).length, 2, 'second apply must write one new transaction');
  assertTransactionTerminal(transactionStore, 'old_generation_retirable');
  assert.equal(
    statSync(installedBinary).ino,
    installedBinaryInode,
    'a no-op second apply must preserve the installed binary inode',
  );
  assert.equal(
    statSync(routeOpener).ino,
    routeOpenerInode,
    'a no-op second apply must preserve versioned support-file inodes',
  );

  const baselineSelectorTarget = readlinkSync(currentSelector);
  const baselineGenerationRoot = resolve(dirname(currentSelector), baselineSelectorTarget);
  const baselineGenerationManifest = treeManifest(baselineGenerationRoot);
  const baselineStableBinaryTarget = readlinkSync(stableBinary);
  const failurePhases = [
    'binary-staged',
    'support-staged',
    'units-staged',
    'generation-preflight-ready',
    'generation-staged',
    'selector-staged',
    'selector-committed',
  ];
  for (const [index, phase] of failurePhases.entries()) {
    const failedUpgrade = runInstaller(
      installRoot,
      ['--apply', '--dashboard-port', String(4800 + index), '--json'],
      { AGENT_BROWSER_WORKSTATION_FAIL_AFTER: phase },
    );
    assert.notEqual(
      failedUpgrade.status,
      0,
      `failure injection after ${phase} must return a nonzero status`,
    );
    assert.equal(
      readlinkSync(currentSelector),
      baselineSelectorTarget,
      `${phase} failure must preserve the selected generation`,
    );
    assert.deepEqual(
      treeManifest(baselineGenerationRoot),
      baselineGenerationManifest,
      `${phase} failure must leave the selected generation byte-identical`,
    );
    assert.equal(
      readlinkSync(stableBinary),
      baselineStableBinaryTarget,
      `${phase} failure must preserve the stable command link`,
    );
    assertGenerationStoreHealthy(generationStore);
    assert.equal(
      readdirSync(transactionStore).length,
      3 + index,
      `${phase} failure must write exactly one durable transaction`,
    );
    assertTransactionTerminal(transactionStore, 'failed_preserved_old_generation');
  }

  const upgradedApply = runInstaller(
    installRoot,
    ['--apply', '--dashboard-port', '4999', '--json'],
  );
  assert.equal(
    upgradedApply.status,
    0,
    `changed workstation payload must select successfully:\n${upgradedApply.stdout}${upgradedApply.stderr}`,
  );
  assertJsonSuccess(upgradedApply.stdout, 'changed payload apply');
  const upgradedSelectorTarget = readlinkSync(currentSelector);
  assert.notEqual(
    upgradedSelectorTarget,
    baselineSelectorTarget,
    'a changed unit payload must select a distinct runtime generation',
  );
  assert.ok(
    existsSync(baselineGenerationRoot),
    'the prior runtime generation must remain available for rollback acceptance',
  );
  assert.match(
    readFileSync(
      join(resolve(dirname(currentSelector), upgradedSelectorTarget), 'units', 'agent-browser-dashboard.service'),
      'utf8',
    ),
    /AGENT_BROWSER_DASHBOARD_PORT=4999/,
    'the selected generation must contain the changed unit payload',
  );
  assertGenerationStoreHealthy(generationStore);

  const gcPreview = runInstaller(installRoot, ['gc', '--dry-run', '--json']);
  assert.equal(gcPreview.status, 0, gcPreview.stderr);
  const gcPreviewPayload = JSON.parse(gcPreview.stdout);
  assert.ok(
    gcPreviewPayload.retained.some(
      (entry) =>
        entry.generationId === baselineSelectorTarget.split('/').at(-1) &&
        entry.reasonCodes.includes('transaction_old_generation'),
    ),
    'generation gc must retain an old generation referenced by rollback evidence',
  );
  assert.ok(
    gcPreviewPayload.retained.some(
      (entry) =>
        entry.generationId === upgradedSelectorTarget.split('/').at(-1) &&
        entry.reasonCodes.includes('selected_generation'),
    ),
    'generation gc must retain the selected generation',
  );
  assert.ok(existsSync(baselineGenerationRoot), 'gc dry-run must not delete candidates');

  const generationStoreBeforeReconcile = treeManifest(generationStore);
  const selectorBeforeReconcile = readlinkSync(currentSelector);
  const reconcile = runInstaller(installRoot, ['reconcile', '--json']);
  assert.notEqual(
    reconcile.status,
    null,
    'workstation reconcile must finish through the public command interface',
  );
  assert.doesNotMatch(
    `${reconcile.stdout}${reconcile.stderr}`,
    /Usage:/,
    'workstation reconcile must be accepted as an operational command',
  );
  JSON.parse(reconcile.stdout);
  assert.equal(
    readlinkSync(currentSelector),
    selectorBeforeReconcile,
    'workstation reconcile must not select or replace a payload generation',
  );
  assert.deepEqual(
    treeManifest(generationStore),
    generationStoreBeforeReconcile,
    'workstation reconcile must not mutate immutable payload generations',
  );

  const selectedAfterUpgrade = resolve(dirname(currentSelector), readlinkSync(currentSelector));
  const generationManifestPath = join(selectedAfterUpgrade, 'generation.json');
  const generationManifestMode = lstatSync(generationManifestPath).mode & 0o777;
  chmodSync(generationManifestPath, generationManifestMode | 0o200);
  const writableGenerationApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.notEqual(
    writableGenerationApply.status,
    0,
    'apply must reject a writable selected generation before staging',
  );
  assert.match(
    `${writableGenerationApply.stdout}${writableGenerationApply.stderr}`,
    /generation entry is writable/,
  );
  chmodSync(generationManifestPath, generationManifestMode);

  const missingUnitPath = join(
    selectedAfterUpgrade,
    'units',
    'agent-browser-guacamole-postgres-backup.service',
  );
  const generationUnitDir = dirname(missingUnitPath);
  const generationUnitDirMode = lstatSync(generationUnitDir).mode & 0o777;
  chmodSync(generationUnitDir, generationUnitDirMode | 0o200);
  unlinkSync(missingUnitPath);
  chmodSync(generationUnitDir, generationUnitDirMode);
  const incompleteGenerationApply = runInstaller(installRoot, ['--apply', '--json']);
  assert.notEqual(
    incompleteGenerationApply.status,
    0,
    'apply must reject an incomplete selected generation before staging',
  );
  assert.match(
    `${incompleteGenerationApply.stdout}${incompleteGenerationApply.stderr}`,
    /current runtime generation is incomplete/,
  );

  writeFileSync(commandLog, '');
  const failedApply = runInstaller(
    failedInstallRoot,
    ['--apply', '--json'],
    { AGENT_BROWSER_WORKSTATION_FAIL_AFTER: 'units-staged' },
  );
  assert.notEqual(
    failedApply.status,
    0,
    'failure injection after units-staged must return a nonzero status',
  );
  const failedRegularFiles = regularFiles(failedInstallRoot).filter(
    (path) => basename(path) !== 'workstation.lock',
  );
  assert.equal(
    failedRegularFiles.length,
    1,
    'a failed first apply must retain only its durable transaction',
  );
  assert.match(failedRegularFiles[0], /runtime-adoption\/transactions\/upgrade-.*\.json$/);
  assertTransactionTerminal(
    join(failedInstallRoot, '.agent-browser', 'runtime-adoption', 'transactions'),
    'failed_preserved_old_generation',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8')
      .split('\n')
      .filter((line) => /\b(enable|start|restart)\b/.test(line))
      .length,
    0,
    'a failed apply must not activate systemd units',
  );

  const staleDrainPath = join(
    drainBlockedRoot,
    '.agent-browser',
    'runtime-adoption',
    'admission-drain.json',
  );
  mkdirSync(dirname(staleDrainPath), { recursive: true });
  writeFileSync(staleDrainPath, '{}');
  const drainBlockedApply = runInstaller(drainBlockedRoot, ['--apply', '--json']);
  assert.notEqual(drainBlockedApply.status, 0);
  assert.match(
    `${drainBlockedApply.stdout}${drainBlockedApply.stderr}`,
    /earlier workstation transaction still owns runtime admission/,
  );
  assert.equal(
    existsSync(join(drainBlockedRoot, '.local', 'lib', 'agent-browser', 'generations')),
    false,
    'an existing admission drain must block before candidate staging',
  );
  assertTransactionTerminal(
    join(drainBlockedRoot, '.agent-browser', 'runtime-adoption', 'transactions'),
    'blocked_inflight_effect',
  );

  const lockDir = join(lockedInstallRoot, '.agent-browser', 'convergence');
  mkdirSync(lockDir, { recursive: true });
  writeFileSync(join(lockDir, 'workstation.lock'), `${process.pid}\n`);
  writeFileSync(commandLog, '');
  const lockedApply = runInstaller(lockedInstallRoot, ['--apply', '--json']);
  assert.notEqual(lockedApply.status, 0, 'a concurrent apply must fail before staging');
  assert.match(`${lockedApply.stdout}${lockedApply.stderr}`, /already active/);
  assert.equal(
    regularFiles(lockedInstallRoot).filter(
      (path) => basename(path) !== 'workstation.lock',
    ).length,
    1,
    'a lock-rejected apply must retain only its blocked transaction',
  );
  assertTransactionTerminal(
    join(lockedInstallRoot, '.agent-browser', 'runtime-adoption', 'transactions'),
    'blocked_inflight_effect',
  );
  assert.equal(
    readFileSync(commandLog, 'utf8'),
    '',
    'a lock-rejected apply must not quiesce or activate user units',
  );

  console.log('Workstation install source-free fixture passed');
} finally {
  makeTreeWritable(fixtureRoot);
  rmSync(fixtureRoot, { recursive: true, force: true });
}

function makeTreeWritable(path) {
  if (!existsSync(path)) return;
  const metadata = lstatSync(path);
  if (metadata.isSymbolicLink()) return;
  if (metadata.isDirectory()) {
    chmodSync(path, 0o700);
    for (const entry of readdirSync(path)) {
      makeTreeWritable(join(path, entry));
    }
    return;
  }
  chmodSync(path, metadata.mode | 0o200);
}

function resolveAgentBrowser() {
  const configured = process.env.AGENT_BROWSER_FIXTURE_BIN;
  const candidates = [
    configured,
    join(repoRoot, 'cli', 'target', 'debug', 'agent-browser'),
    join(repoRoot, 'cli', 'target', 'release', 'agent-browser'),
    join(repoRoot, 'bin', `agent-browser-${process.platform}-${process.arch}`),
  ].filter(Boolean);
  const candidate = candidates.find((path) => existsSync(path));
  assert.ok(
    candidate,
    'Set AGENT_BROWSER_FIXTURE_BIN to the agent-browser binary under test',
  );
  return resolve(candidate);
}

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function runInstaller(root, flags, extraEnv = {}) {
  return spawnSync(agentBrowser, ['install', 'workstation', ...flags], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      XDG_CONFIG_HOME: join(xdgRoot, 'config'),
      XDG_DATA_HOME: join(xdgRoot, 'data'),
      XDG_STATE_HOME: join(xdgRoot, 'state'),
      XDG_CACHE_HOME: join(xdgRoot, 'cache'),
      XDG_RUNTIME_DIR: join(xdgRoot, 'runtime'),
      PATH: `${fakeBin}:/usr/bin:/bin`,
      AGENT_BROWSER_WORKSTATION_ROOT: root,
      AGENT_BROWSER_WORKSTATION_COMMAND_LOG: commandLog,
      ...extraEnv,
    },
  });
}

function assertWorkstationInterface() {
  const help = spawnSync(agentBrowser, ['install', '--help'], {
    cwd: fixtureRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      HOME: home,
      PATH: `${fakeBin}:/usr/bin:/bin`,
    },
  });
  assert.equal(
    help.status,
    0,
    `install help must succeed before running the fixture:\n${help.stdout}${help.stderr}`,
  );
  assert.match(
    `${help.stdout}${help.stderr}`,
    /install workstation/,
    'binary does not expose the required `agent-browser install workstation` interface',
  );
}

function assertJsonSuccess(stdout, label) {
  assert.ok(stdout.trim(), `${label} must emit JSON`);
  let payload;
  try {
    payload = JSON.parse(stdout);
  } catch (error) {
    assert.fail(`${label} emitted invalid JSON: ${error.message}\n${stdout}`);
  }
  assert.equal(payload.success, true, `${label} JSON must report success`);
}

function installCommandShim(command) {
  const path = join(fakeBin, command);
  writeFileSync(path, `#!/usr/bin/env bash
set -euo pipefail
printf '%s' '${command}' >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
printf ' %q' "$@" >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
printf '\\n' >>"$AGENT_BROWSER_WORKSTATION_COMMAND_LOG"
`);
  chmodSync(path, 0o755);
}

function regularFiles(root) {
  if (!existsSync(root)) return [];
  const paths = [];
  visit(root, (path) => {
    if (lstatSync(path).isFile()) paths.push(path);
  });
  return paths.sort();
}

function treeManifest(root, ignored = new Set()) {
  if (!existsSync(root)) return [];
  const manifest = [];
  visit(root, (path) => {
    if (ignored.has(path)) return 'skip';
    const stat = lstatSync(path);
    const relative = path.slice(root.length + 1);
    const entry = {
      path: relative,
      mode: stat.mode & 0o777,
      type: stat.isDirectory() ? 'directory' : stat.isSymbolicLink() ? 'symlink' : 'file',
    };
    if (stat.isFile()) {
      entry.sha256 = createHash('sha256').update(readFileSync(path)).digest('hex');
    } else if (stat.isSymbolicLink()) {
      entry.target = readlinkSync(path);
    }
    manifest.push(entry);
  });
  return manifest.sort((left, right) => left.path.localeCompare(right.path));
}

function visit(root, callback) {
  if (!existsSync(root)) return;
  for (const entry of readdirSync(root).sort()) {
    const path = join(root, entry);
    const decision = callback(path);
    if (decision !== 'skip' && lstatSync(path).isDirectory()) {
      visit(path, callback);
    }
  }
}

function assertGenerationStoreHealthy(generationStore) {
  for (const generationId of readdirSync(generationStore)) {
    const generationRoot = join(generationStore, generationId);
    for (const relative of [
      join('bin', 'agent-browser'),
      join('support', 'manifest.json'),
      join('units', 'agent-browser-dashboard-backend.service'),
      join('units', 'agent-browser-dashboard.service'),
      'generation.json',
    ]) {
      assert.ok(
        existsSync(join(generationRoot, relative)),
        `runtime generation ${generationId} must contain ${relative}`,
      );
    }
    for (const entry of treeManifest(generationRoot)) {
      assert.equal(
        entry.mode & 0o222,
        0,
        `runtime generation ${generationId} must be sealed: ${entry.path}`,
      );
    }
  }
}

function assertTransactionTerminal(transactionStore, expectedState) {
  const transactions = readdirSync(transactionStore)
    .map((name) => {
      const path = join(transactionStore, name);
      return {
        path,
        modifiedAt: statSync(path).mtimeMs,
        payload: JSON.parse(readFileSync(path, 'utf8')),
      };
    })
    .sort((left, right) => right.modifiedAt - left.modifiedAt);
  assert.ok(transactions.length > 0, 'installer must retain at least one transaction');
  assert.equal(
    transactions[0].payload.state,
    expectedState,
    `latest transaction must end in ${expectedState}`,
  );
  assert.ok(
    Array.isArray(transactions[0].payload.checkpoints) &&
      transactions[0].payload.checkpoints.length > 0,
    'latest transaction must retain checkpoint history',
  );
}

function ignoredFixturePaths() {
  return new Set([
    commandLog,
    agentBrowser.startsWith(fixtureRoot) ? agentBrowser : '',
  ]);
}
