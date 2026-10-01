import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '../..');
const paths = {
  cliManifest: 'cli/Cargo.toml',
  modelManifest: 'crates/agent-browser-service-model/Cargo.toml',
  consumer: 'crates/agent-browser-service-model/src/remote_view_consumer.rs',
  application: 'crates/agent-browser-service-model/src/remote_view_application.rs',
  applicationResponse: 'crates/agent-browser-service-model/src/remote_view_application_response.rs',
  applicationAdapter: 'crates/agent-browser-service-model/src/remote_view_application_adapter.rs',
  applicationRecords: 'crates/agent-browser-service-model/src/remote_view_application_records.rs',
  applicationMutation: 'crates/agent-browser-service-model/src/remote_view_application_mutation.rs',
  browserLaunch: 'crates/agent-browser-service-model/src/remote_view_browser_launch.rs',
  sessionEffects: 'crates/agent-browser-service-model/src/remote_view_session_effects.rs',
  applicationCleanup: 'crates/agent-browser-service-model/src/remote_view_application_cleanup.rs',
  applicationStore: 'cli/src/native/remote_view_application_store.rs',
  launchCustodyStore: 'cli/src/native/browser_launch_custody_store.rs',
  launchCustody: 'crates/agent-browser-service-model/src/browser_launch_custody.rs',
  manager: 'crates/agent-browser-service-model/src/browser_session_manager.rs',
  recovery: 'crates/agent-browser-service-model/src/browser_recovery.rs',
  retention: 'crates/agent-browser-service-model/src/remote_view_retention.rs',
  launchAdmission: 'cli/src/native/browser_launch_admission.rs',
  host: 'cli/src/native/browser_session_host.rs',
  runtime: 'cli/src/native/browser_session_runtime.rs',
  chrome: 'cli/src/native/cdp/chrome.rs',
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
  const modelSources = [source.consumer, source.application, source.applicationResponse, source.applicationAdapter, source.applicationRecords, source.applicationMutation, source.applicationCleanup, source.browserLaunch, source.sessionEffects, source.launchCustody, source.manager, source.recovery, source.retention].join('\n');
  const supportedCliSources = [
    source.host,
    source.runtime,
    source.store,
    source.backup,
    source.launchAdmission,
    source.applicationStore,
    source.launchCustodyStore,
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
    source.application.includes('da540f22a6ff851272c5e9b91d7e3117a28bf6cd') &&
      !/^\s*(?:Place|Stop)\s*\{/m.test(source.application),
    'revision 3 external application requests must stay source-bound without managed lifecycle verbs',
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
      /detach_remote_view_presentation/.test(source.retention) &&
      /RemoteViewPresentationRetentionState::Detached/.test(source.retention) &&
      /remote_view_retention_release_still_referenced/.test(source.retention) &&
      /RemoteViewPresentationRetentionState::Released/.test(source.retention) &&
      /pub\(crate\) fn retain_remote_view_presentation/.test(source.store) &&
      /fn mutate_session_state/.test(source.store) &&
      !/(?:provider_url|display_number|credential)/i.test(source.retention),
    'Remote View retention must stay exact, shareable, provider-neutral, and reference-fenced',
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
  requireCondition(
    source.runtime.includes('impl RemoteViewBrowserProcessEffects for BrowserManagerRuntime') &&
      source.runtime.includes('.into_environment(observation, &intent.assignment)') &&
      source.runtime.includes('private_launch_environment: environment'),
    'external process ingress must consume fresh complete private launch inputs',
  );
  requireCondition(
    source.chrome.includes('command.envs(environment)') &&
      source.chrome.includes('let max_attempts = if options.private_launch_environment.is_some()') &&
      source.chrome.includes('let requested_stderr_log_path = if options.private_launch_environment.is_some()'),
    'private process launches must preserve complete inputs without retries or persisted stderr',
  );
  requireCondition(
    source.host.includes('self.effects.begin_operation(&self.persisted_state)?') &&
      source.host.includes('self.effects.pending_launch_intent()') &&
      source.host.includes('self.publish_launch_intent(intent, expected, state)') &&
      source.host.includes('self.effects.acknowledge_launch_publication()') &&
      source.launchCustodyStore.includes('return Err(LaunchCustodyStoreError::LaunchPending)'),
    'ordinary host must publish observed launch custody atomically before acknowledgement',
  );
  requireCondition(
    source.sessionEffects.includes('launch_remote_view_browser(') &&
      source.sessionEffects.includes('unpublished_launch_records()') &&
      source.sessionEffects.includes('fn pending_launch_intent('),
    'ordinary consumer effects must join durable launch admission and retained publication intent',
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
      [paths.application, '\npub struct ProviderFreeHost;\n', 'provider-private application'],
      [paths.applicationResponse, '\npub struct ProviderFreeHost;\n', 'provider-private application response'],
      [paths.applicationAdapter, '\npub struct ProviderFreeHost;\n', 'provider-private application adapter'],
      [paths.applicationRecords, '\npub struct ProviderFreeHost;\n', 'provider-private application records'],
      [paths.applicationMutation, '\npub struct ProviderFreeHost;\n', 'provider-private application mutation'],
      [paths.applicationCleanup, '\npub struct ProviderFreeHost;\n', 'provider-private cleanup'],
      [paths.launchCustody, '\npub struct ProviderFreeHost;\n', 'provider-private launch custody'],
      [paths.launchCustodyStore, '\nstruct RouteKeeperAuthority;\n', 'retired launch custody authority'],
      [paths.browserLaunch, '\npub struct ProviderFreeHost;\n', 'provider-private launch coordinator'],
      [paths.sessionEffects, '\npub struct ProviderFreeHost;\n', 'provider-private session effects'],
      [paths.applicationStore, '\nstruct RouteKeeperAuthority;\n', 'retired application store authority'],
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
    for (const [path, needle] of [
      [paths.runtime, '.into_environment(observation, &intent.assignment)'],
      [paths.chrome, 'command.envs(environment)'],
      [paths.host, 'self.publish_launch_intent(intent, expected, state)'],
      [paths.sessionEffects, 'launch_remote_view_browser('],
    ]) {
      copyFixture(root);
      writeFileSync(join(root, path), read(root, path).replaceAll(needle, 'removed_boundary'));
      if (check(root).length === 0) throw new Error(`self-test missed private launch boundary removal: ${path}`);
    }
    copyFixture(root);
    const runtimeWithoutContextFence = read(root, paths.runtime).replaceAll(
      'remote_view_desktop_runtime_context_stale',
      'remote_view_desktop_context_mismatch',
    );
    writeFileSync(join(root, paths.runtime), runtimeWithoutContextFence);
    if (check(root).length === 0) throw new Error('self-test missed generation context fence removal');
    copyFixture(root);
    const retentionWithoutReferenceFence = read(root, paths.retention).replaceAll(
      'remote_view_retention_release_still_referenced',
      'remote_view_retention_release_unchecked',
    );
    writeFileSync(join(root, paths.retention), retentionWithoutReferenceFence);
    if (check(root).length === 0) throw new Error('self-test missed shared-assignment reference fence removal');
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
