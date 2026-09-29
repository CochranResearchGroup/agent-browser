import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '../..');
const paths = {
  cliManifest: 'cli/Cargo.toml',
  modelManifest: 'crates/agent-browser-service-model/Cargo.toml',
  consumer: 'crates/agent-browser-service-model/src/remote_view_consumer.rs',
  manager: 'crates/agent-browser-service-model/src/browser_session_manager.rs',
  recovery: 'crates/agent-browser-service-model/src/browser_recovery.rs',
  retention: 'crates/agent-browser-service-model/src/remote_view_retention.rs',
  launchAdmission: 'cli/src/native/browser_launch_admission.rs',
  host: 'cli/src/native/browser_session_host.rs',
  runtime: 'cli/src/native/browser_session_runtime.rs',
  store: 'cli/src/native/browser_session_store.rs',
  backup: 'cli/src/native/service_runtime_backup.rs',
};

function read(root, path) {
  return readFileSync(join(root, path), 'utf8');
}

function check(root) {
  const source = Object.fromEntries(
    Object.entries(paths).map(([name, path]) => [name, read(root, path)]),
  );
  const failures = [];
  const requireCondition = (condition, message) => {
    if (!condition) failures.push(message);
  };
  const modelSources = [source.consumer, source.manager, source.recovery, source.retention].join('\n');
  const supportedCliSources = [
    source.host,
    source.runtime,
    source.store,
    source.backup,
    source.launchAdmission,
  ].join('\n');
  const retiredProviderTerms = /\b(?:RouteKeeperAuthority|PresentationRequestQueue|ProviderFreeHost|ProviderFreeApplicationEffect|ProviderFreeControl)\b|\bguacamole\b|\bxrdp\b/i;
  const handoffProjection = source.retention.slice(
    source.retention.indexOf('pub fn project_remote_view_operator_handoff'),
    source.retention.indexOf('pub fn retain_remote_view_presentation'),
  );
  const operationalStatusProjection = source.store.slice(
    source.store.indexOf('pub(crate) fn default_operational_status_read_only'),
    source.store.indexOf('pub(crate) fn open_or_migrate'),
  );

  requireCondition(
    !retiredProviderTerms.test(modelSources),
    'service-model consumer, session, and recovery modules must remain provider-neutral',
  );
  requireCondition(
    !retiredProviderTerms.test(supportedCliSources),
    'supported P220 CLI modules must not restore retired presentation authority',
  );
  requireCondition(
    !/^\s*rusqlite\s*=/m.test(source.modelManifest),
    'service-model crate must not own SQLite persistence',
  );
  requireCondition(
    /^\s*rusqlite\s*=.*features\s*=\s*\[[^\]]*"backup"[^\]]*"bundled"/m.test(
      source.cliManifest,
    ),
    'CLI adapter must own the reviewed bundled SQLite backup dependency',
  );
  requireCondition(
    !source.runtime.includes('remote_view_application_placement_contract_unavailable') &&
      source.runtime.includes('resolve_remote_view_display') &&
      source.runtime.includes('remote_view_desktop_runtime_context_unavailable') &&
      source.runtime.includes('remote_view_desktop_runtime_context_stale'),
    'runtime must require exact Remote View desktop identity and generation context without application placement',
  );
  requireCondition(
    source.runtime.indexOf('observe_browser_launch_admission') <
      source.runtime.indexOf('BrowserManager::launch'),
    'host resource admission must run before the local browser launch effect',
  );
  requireCondition(
    /browser_launch_resource_pressure/.test(source.launchAdmission) &&
      !/(?:remote.?view|guacamole|xrdp|presentation|route.?keeper)/i.test(
        source.launchAdmission.replace(/Provider-neutral/g, ''),
      ),
    'browser launch resource admission must remain provider-neutral and fail closed',
  );
  requireCondition(
    !/desktop\s*\.\s*(?:route_label|friendly_route_label)[\s\S]{0,160}(?:display|DISPLAY)/i.test(
      source.runtime,
    ),
    'runtime must not infer a local display from a Remote View route label',
  );
  requireCondition(
    /REMOTE_VIEW_J3_SOURCE_CHECKPOINT\s*:\s*&str\s*=\s*"018d3d752f9d99805242f99c00034f0caabc53d1"/.test(
      source.consumer,
    ),
    'Remote View J3 consumer must stay bound to the canonical source checkpoint',
  );
  requireCondition(
    /deny_unknown_fields/.test(source.consumer) &&
      /rejects_j3_private_browser_state_and_inexact_joined_cleanup/.test(source.consumer),
    'Remote View consumer must reject private fields and inexact cleanup',
  );
  requireCondition(
    /Self::Dormant/.test(source.recovery) &&
      /OldBrowserUsability::Unknown/.test(source.recovery) &&
      /browser_recovery_observed_live_fence_mismatch/.test(source.recovery),
    'browser recovery must retain demand, proof, and generation fences',
  );
  requireCondition(
    /BROWSER_RECOVERY_REGISTRY_DOCUMENT/.test(source.store) &&
      /transaction_with_behavior\(TransactionBehavior::Immediate\)/.test(source.store),
    'recovery state must remain under transactional CLI persistence authority',
  );
  requireCondition(
    /remote_view_presentations/.test(source.manager) &&
      /release_remote_view_presentation/.test(source.retention) &&
      /RemoteViewPresentationRetentionState::Released/.test(source.retention) &&
      /pub\(crate\) fn retain_remote_view_presentation/.test(source.store) &&
      /fn mutate_session_state/.test(source.store) &&
      !/(?:provider_url|display_number|credential)/i.test(source.retention),
    'Remote View retention must stay exact, provider-neutral, and release-fenced',
  );
  requireCondition(
    /\/remote-view\/\{handoff_id\}/.test(handoffProjection) &&
      !/(?:route_id|desktop_id|provider|display|credential)/i.test(handoffProjection),
    'operator handoff projection must return only Agent Browser durable identity',
  );
  requireCondition(
    /handle_service_runtime_backup_status/.test(source.backup) &&
      /handle_service_runtime_backup_create/.test(source.backup) &&
      !/restore/i.test(source.backup.replace(/do not restore/gi, '')),
    'backup surface must expose status and creation without a restore effect',
  );
  requireCondition(
    /SQLITE_OPEN_READ_ONLY/.test(source.store) &&
      /unavailable_operational_status/.test(operationalStatusProjection) &&
      !/(?:databasePath|archive_directory\s*:|to_string_lossy)/.test(
        operationalStatusProjection,
      ),
    'Browser Runtime operational status must stay read-only and path-redacted',
  );
  return failures;
}

function requireClean(root, label) {
  const failures = check(root);
  if (failures.length) {
    throw new Error(`${label}:\n${failures.map((failure) => `  - ${failure}`).join('\n')}`);
  }
}

function copyFixture(root) {
  for (const path of Object.values(paths)) {
    const destination = join(root, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, read(repoRoot, path));
  }
}

function selfTest() {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-p220-architecture-'));
  try {
    copyFixture(root);
    requireClean(root, 'valid fixture rejected');
    const cases = [
      [paths.consumer, '\npub struct ProviderFreeHost;\n', 'provider-private model'],
      [paths.retention, '\npub provider_url: String;\n', 'provider-private retention'],
      [paths.runtime, '\nfn infer() { let _ = desktop.route_label; let _ = DISPLAY; }\n', 'display inference'],
      [paths.store, '\nstruct RouteKeeperAuthority;\n', 'retired store authority'],
      [paths.backup, '\nstruct ProviderFreeControl;\n', 'provider-private backup action'],
      [paths.launchAdmission, '\nstruct GuacamoleRoute;\n', 'provider-private launch admission'],
      [paths.modelManifest, '\nrusqlite = "0.40"\n', 'model persistence'],
    ];
    for (const [path, mutation, label] of cases) {
      copyFixture(root);
      writeFileSync(join(root, path), `${read(root, path)}${mutation}`);
      if (check(root).length === 0) throw new Error(`self-test missed ${label}`);
    }
    copyFixture(root);
    const runtimeWithoutContextFence = read(root, paths.runtime).replaceAll(
      'remote_view_desktop_runtime_context_stale',
      'remote_view_desktop_context_mismatch',
    );
    writeFileSync(join(root, paths.runtime), runtimeWithoutContextFence);
    if (check(root).length === 0) throw new Error('self-test missed generation context fence removal');
    copyFixture(root);
    const retention = read(root, paths.retention).replace(
      'handoff_url: format!("/remote-view/{handoff_id}"),',
      'handoff_url: retained.route_id.clone(),',
    );
    writeFileSync(join(root, paths.retention), retention);
    if (check(root).length === 0) throw new Error('self-test missed route identity handoff leak');
    copyFixture(root);
    const store = read(root, paths.store).replace(
      '"state": "available",',
      '"state": "available", "databasePath": path.to_string_lossy(),',
    );
    writeFileSync(join(root, paths.store), store);
    if (check(root).length === 0) throw new Error('self-test missed runtime status path leak');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

selfTest();
requireClean(repoRoot, 'P220 architecture contract failed');
console.log('P220 architecture contract passed');
