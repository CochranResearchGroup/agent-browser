#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '../..');

const ABANDONED_RETIREMENT_RECORDS = [
  'AbandonedBrowserRetirementPlan',
  'AbandonedBrowserRetirementTransaction',
  'AbandonedBrowserRetirementReceipt',
  'RetirementTerminalProjection',
  'RetirementExitEvidence',
  'RetirementExitFailure',
  'RetirementRecourse',
  'ResourceRetirementPolicy',
];

const CRASH_REGENERATION_DEFINITIONS = [
  'CrashRegenerationPhase',
  'CrashRegenerationState',
  'CrashRegenerationStableIdentities',
  'CrashRegenerationEvidence',
  'CrashRegenerationTransaction',
  'CrashRegenerationStatus',
  'CrashRegenerationRequest',
  'CrashRegenerationOperation',
  'CrashRegenerationPhaseReceipt',
];

const SERVICE_STATE_CRASH_REGENERATION_METHODS = [
  'crash_regeneration_transaction',
  'crash_regeneration_statuses',
  'begin_or_resume_crash_regeneration',
  'apply_crash_regeneration_phase',
  'interrupt_crash_regeneration',
  'finish_crash_regeneration',
  'without_crash_regeneration_transactions',
];

const CAPABILITY_REGISTRY_DEFINITIONS = [
  'BrowserCapabilityRegistry',
];

const AUTHENTICATION_CONTROL_DEFINITIONS = [
  'AuthenticationRunBinding',
  'AuthenticationRunState',
  'AuthenticationChallengeChannel',
  'AuthenticationActionKind',
  'SiteLoginState',
  'PasswordPersistencePolicy',
  'SiteLoginObservationReceipt',
  'SiteLoginActionReceipt',
  'SiteLoginActionContext',
  'ProviderWatchReceipt',
  'SameProfileNewTabProof',
  'AuthenticationActionReceipt',
  'AuthenticationActionContext',
  'AuthenticationActionFailure',
  'AuthenticationVerifierReceipt',
  'AuthenticationVerificationContext',
  'AuthenticationVerifierFailure',
  'AuthenticationTransitionReceipt',
  'ActiveAuthenticationChallenge',
  'AuthenticationRun',
  'AuthenticationRunError',
];

const AUTHENTICATION_CONTROL_TRAITS = [
  'ResponseOnlySiteLoginAction',
  'ResponseOnlyAuthenticationAction',
  'AuthenticationVerifier',
];

const SERVICE_AUTHENTICATION_DEFINITIONS = [
  'ServiceAuthenticationRunRecord',
  'PendingAuthenticationEffect',
  'ServiceAuthenticationRunStartInput',
  'PreparedServiceAuthenticationRunStart',
  'ServiceAuthenticationRunStartDecision',
  'ServiceAuthenticationRunCompletion',
  'ServiceAuthenticationRunError',
  'ServiceAuthenticationRunProjection',
];

const SERVICE_AUTHENTICATION_DECISIONS = [
  'prepare_service_authentication_run_start',
  'complete_service_authentication_run_start',
  'project_service_authentication_run',
  'require_live_authentication_run',
  'reserve_authentication_effect',
  'complete_site_authentication_action',
  'complete_credential_delivery_action',
  'complete_challenge_authentication_action',
  'cancel_authentication_run',
  'authentication_run_map_is_empty',
];

const SERVICE_STATE_AUTHENTICATION_METHODS = [
  'service_authentication_run',
  'prepare_service_authentication_run_start',
  'complete_service_authentication_run_start',
  'reserve_service_authentication_effect',
  'observe_service_authentication_run',
  'prepare_service_authentication_watch',
  'verify_service_authentication_run',
  'complete_service_authentication_site_action',
  'complete_service_authentication_delivery_action',
  'complete_service_authentication_challenge_action',
  'cancel_service_authentication_run',
];

const SERVICE_STATE_LEASE_AUTHORITY_METHODS = [
  'current_lease_claim',
  'replay_lease_claim_release',
  'replay_lease_claim_recovery',
  'replay_lease_claim_revocation',
  'release_lease_claim',
  'recover_lease_claim',
  'revoke_lease_claim',
];

const SERVICE_STATE_RECEIPT_TYPES = [
  'ProfileRecoveryReceiptIdentity',
  'ProfileResetReceiptIdentity',
  'ProfileReceiptReplayError',
];

const SERVICE_STATE_RECEIPT_METHODS = [
  'replay_profile_lease_reconciliation',
  'record_profile_lease_reconciliation',
  'profile_recovery_receipt',
  'replay_profile_recovery',
  'record_profile_recovery',
  'replay_profile_reset',
  'record_profile_reset',
];

const SERVICE_STATE_RECEIPT_FIELDS = [
  'profile_lease_reconcile_receipts',
  'profile_recovery_receipts',
  'profile_reset_receipts',
];

const SERVICE_STATE_PRINCIPAL_METHODS = [
  'service_principal_registry_revision',
  'service_principal',
  'profile_capability',
  'profile_capabilities',
  'authenticate_profile_capability',
  'authenticated_authority_is_current',
  'register_profile_capability',
  'rotate_profile_capability',
  'lease_authority_view',
];

const PRINCIPAL_CONTINUITY_DEFINITIONS = [
  'PrincipalContinuityDecision',
  'LegacyPrincipalMigrationDisposition',
  'LegacySessionPrincipalMigrationPlan',
];

const SERVICE_STATE_CONTINUITY_METHODS = [
  'authenticated_session_work_authority',
  'principal_continuity_decision',
  'plan_legacy_session_principal_migration',
  'bind_session_work_lease',
  'bind_tab_work_lease',
];

const RUNTIME_OWNER_PERSISTENCE_DEFINITIONS = [
  'RuntimeOwnerPersistenceSnapshot',
  'RuntimeOwnerPersistenceRestore',
  'RuntimeOwnerPersistenceParts',
];

const RUNTIME_OWNER_PERSISTENCE_METHODS = [
  'restore_runtime_owner_persistence',
  'runtime_owner_persistence_parts',
  'strip_runtime_lifecycle_for_persistence',
];

const RUNTIME_OWNER_PROJECTION_DEFINITIONS = [
  'RuntimeLifecycleAuthoritySummary',
  'RuntimeLifecycleBootEpochObservation',
  'ProfileRuntimeAuthority',
  'RuntimeControlPlaneAuthority',
  'RuntimeLaneAuthority',
  'RuntimeResourceLane',
];

const RUNTIME_OWNER_PROJECTION_METHODS = [
  'runtime_lifecycle_authority_summary',
  'runtime_lifecycle_boot_epoch_observations',
  'profile_runtime_authority',
  'runtime_control_plane_authority',
  'runtime_lane_authority',
  'runtime_resource_lanes',
];

const ATOMIC_RUNTIME_LIFECYCLE_METHOD = 'apply_runtime_lifecycle_transition_atomically';
const ATOMIC_PROFILE_SYNC_LIFECYCLE_METHOD =
  'apply_runtime_lifecycle_transition_with_profile_sync_atomically';

const RUNTIME_OWNER_PROJECTION_CLI_FILES = [
  'cli/src/install.rs',
  'cli/src/native/service_boot_epoch.rs',
  'cli/src/native/service_diagnostics.rs',
  'cli/src/native/service_profile_diagnosis.rs',
  'cli/src/native/service_status_projection.rs',
  'cli/src/native/service_resources.rs',
  'cli/src/native/service_resources/retained_tree.rs',
];

const SERVICE_CHALLENGE_DEFINITIONS = [
  'ServiceChallengeTaskState',
  'ServiceChallengeTaskSummary',
  'PendingChallengeTaskEffect',
  'ChallengeTaskEffectKind',
  'ServiceChallengeTaskRecord',
  'ServiceChallengeTaskStartInput',
  'PreparedServiceChallengeTaskStart',
  'ServiceChallengeTaskStartDecision',
  'ServiceChallengeTaskResumeInput',
  'PreparedServiceChallengeTaskResume',
  'ServiceChallengeTaskResumeDecision',
  'ServiceChallengeTaskCancelInput',
  'ServiceChallengeTaskCancelDecision',
  'ServiceChallengeTaskError',
  'ServiceChallengeTaskProjection',
];

const SERVICE_CHALLENGE_EXPORTED_DEFINITIONS = SERVICE_CHALLENGE_DEFINITIONS.filter(
  (name) => !['PendingChallengeTaskEffect', 'ChallengeTaskEffectKind'].includes(name),
);

const SERVICE_CHALLENGE_DECISIONS = [
  'prepare_service_challenge_task_start',
  'complete_service_challenge_task_start',
  'service_challenge_task_status',
  'prepare_service_challenge_task_resume',
  'complete_service_challenge_task_resume',
  'cancel_service_challenge_task',
  'project_service_challenge_task',
  'challenge_task_map_is_empty',
  'challenge_task_summary',
  'admit_challenge_consumer_from_receipt',
];

const SERVICE_CHALLENGE_CONSTANTS = [
  'SERVICE_CHALLENGE_TASK_SCHEMA_VERSION',
  'AUTHENTICATION_CHALLENGE_INTENT_ID',
  'NAVIGATION_CHALLENGE_INTENT_ID',
];

const SERVICE_STATE_CHALLENGE_METHODS = [
  'service_challenge_task',
  'service_challenge_task_summary',
  'start_service_challenge_task',
  'status_service_challenge_task',
  'resume_service_challenge_task',
  'cancel_service_challenge_task',
];

const SERVICE_STATE_DIRECT_OWNER_FIELDS = [
  [
    'presentation_capacity',
    /presentation_capacity\s*:\s*Option\s*<\s*agent_browser_service_model\s*::\s*PresentationCapacityAuthority\s*>/,
  ],
  [
    'profile_policy_migration',
    /profile_policy_migration\s*:\s*Option\s*<\s*agent_browser_service_model\s*::\s*ProfilePolicyMigrationReport\s*>/,
  ],
  [
    'service_principals',
    /service_principals\s*:\s*agent_browser_lease_authority\s*::\s*ServicePrincipalRegistry/,
  ],
  [
    'profile_lease_reconcile_receipts',
    /profile_lease_reconcile_receipts\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ProfileLeaseReconcileReceipt\s*>/,
  ],
  [
    'profile_recovery_receipts',
    /profile_recovery_receipts\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*RecoveryReceipt\s*>/,
  ],
  [
    'profile_reset_receipts',
    /profile_reset_receipts\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ProfileResetReceipt\s*>/,
  ],
  [
    'profile_lifecycle_authorizations',
    /profile_lifecycle_authorizations\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ProfileLifecycleAuthorization\s*>/,
  ],
  [
    'profile_lifecycle_effect_receipts',
    /profile_lifecycle_effect_receipts\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ProfileLifecycleEffectReceipt\s*>/,
  ],
  [
    'browser_retirement_receipts',
    /browser_retirement_receipts\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*BrowserRetirementReceipt\s*>/,
  ],
];

const SERVICE_STATE_MIGRATION_FIELDS = [
  'schema_version', 'state_revision', 'profile_lease_schema_version',
  'presentation_capacity', 'profile_policy_migration', 'service_principals',
  'lease_authority', 'profile_lease_reconcile_receipts', 'profile_recovery_receipts',
  'profile_reset_receipts', 'profile_lifecycle_authorizations',
  'profile_lifecycle_effect_receipts', 'browser_retirement_receipts',
  'abandoned_browser_retirements', 'crash_regeneration_transactions',
  'protected_browser_owner_observations', 'runtime_owner_registry',
  'authentication_runs', 'challenge_tasks', 'unknown_fields',
];

const SERVICE_STATE_CODEC_EXPORTS = [
  'builtin_site_policies', 'builtin_site_policy', 'decode_persisted_service_state_json',
  'default_profile_seeding_url',
  'encode_prepared_service_state_pretty', 'prepare_service_state_for_persistence',
  'service_profile_sources', 'service_site_policy_sources',
  'validate_service_state_invariants', 'ConfiguredServiceStateInput', 'ServiceState',
  'ServiceStateCodecError',
  'LEGACY_SERVICE_STATE_SCHEMA_VERSION', 'SERVICE_STATE_SCHEMA_VERSION',
];

const SERVICE_MODEL_ALLOWED_DEPENDENCIES = new Set([
  'agent-browser-authentication-control',
  'agent-browser-challenge-control',
  'agent-browser-lease-authority',
  'chrono',
  'serde',
  'serde_json',
  'sha2',
]);

const FORBIDDEN_IMPORT_PATHS = [
  'crate::native',
  'agent_browser::native',
  'cli::src::native',
  'std::process',
  'std::fs',
  'std::net',
  'tokio::',
  'reqwest::',
  'image::',
  'hyper::',
  'axum::',
];

const FORBIDDEN_ADAPTER_MODULES = new Set([
  'browser',
  'cdp',
  'capture',
  'controlled_x11',
  'dashboard',
  'desktop',
  'filesystem',
  'http',
  'input',
  'install',
  'mcp',
  'platform',
  'process',
  'provider',
  'remote_view',
  'runtime',
]);

function read(root, path) {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function rustFilesUnder(root) {
  if (!existsSync(root)) return [];
  return readdirSync(root).flatMap((entry) => {
    const absolute = join(root, entry);
    if (statSync(absolute).isDirectory()) return rustFilesUnder(absolute);
    return absolute.endsWith('.rs') ? [absolute] : [];
  });
}

// Boundary checks should inspect syntax-bearing text, not prose or string fixtures.
function withoutCommentsAndStrings(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/.*$/gm, ' ')
    .replace(/"(?:\\.|[^"\\])*"/g, '""')
    .replace(/'(?:\\.|[^'\\])*'/g, "''");
}

function withoutComments(source) {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/\/\/.*$/gm, ' ');
}

// Remove items compiled only for tests while retaining production items that
// follow an early cfg(test) import or helper in the same Rust source file.
function withoutCfgTestItems(source) {
  const attribute = /^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/gm;
  const ranges = [];
  let match;
  while ((match = attribute.exec(source)) !== null) {
    let cursor = match.index + match[0].length;
    while (cursor < source.length) {
      while (/\s/.test(source[cursor])) cursor += 1;
      if (source[cursor] !== '#' || source[cursor + 1] !== '[') break;
      let bracketDepth = 1;
      cursor += 2;
      while (cursor < source.length && bracketDepth > 0) {
        if (source[cursor] === '[') bracketDepth += 1;
        else if (source[cursor] === ']') bracketDepth -= 1;
        cursor += 1;
      }
    }

    let state = 'code';
    let depth = 0;
    let itemEnd = cursor;
    let itemState = state;
    while (itemEnd < source.length) {
      const character = source[itemEnd];
      const next = source[itemEnd + 1];
      if (itemState === 'line-comment') {
        if (character === '\n') itemState = 'code';
      } else if (itemState === 'block-comment') {
        if (character === '*' && next === '/') {
          itemState = 'code';
          itemEnd += 1;
        }
      } else if (itemState === 'string') {
        if (character === '\\') itemEnd += 1;
        else if (character === '"') itemState = 'code';
      } else if (itemState === 'character') {
        if (character === '\\') itemEnd += 1;
        else if (character === "'") itemState = 'code';
      } else if (character === '/' && next === '/') {
        itemState = 'line-comment';
        itemEnd += 1;
      } else if (character === '/' && next === '*') {
        itemState = 'block-comment';
        itemEnd += 1;
      } else if (character === '"') {
        itemState = 'string';
      } else if (character === "'") {
        itemState = 'character';
      } else if (character === '{') {
        depth += 1;
      } else if (character === '}') {
        depth -= 1;
        if (depth === 0) {
          itemEnd += 1;
          break;
        }
      } else if (character === ';' && depth === 0) {
        itemEnd += 1;
        break;
      }
      itemEnd += 1;
    }
    ranges.push([match.index, itemEnd]);
    attribute.lastIndex = itemEnd;
  }

  let result = source;
  for (const [start, end] of ranges.reverse()) {
    result = `${result.slice(0, start)}${result.slice(start, end).replace(/[^\n]/g, ' ')}${result.slice(end)}`;
  }
  return result;
}

function rustStructDefinition(source, name) {
  const header = new RegExp(`\\bpub(?:\\([^)]*\\))?\\s+struct\\s+${name}\\b[^\\{]*\\{`);
  const match = header.exec(source);
  if (!match) return '';

  let depth = 1;
  let cursor = match.index + match[0].length;
  let state = 'code';
  while (cursor < source.length && depth > 0) {
    const character = source[cursor];
    const next = source[cursor + 1];
    if (state === 'line-comment') {
      if (character === '\n') state = 'code';
    } else if (state === 'block-comment') {
      if (character === '*' && next === '/') {
        state = 'code';
        cursor += 1;
      }
    } else if (state === 'string') {
      if (character === '\\') cursor += 1;
      else if (character === '"') state = 'code';
    } else if (state === 'character') {
      if (character === '\\') cursor += 1;
      else if (character === "'") state = 'code';
    } else if (character === '/' && next === '/') {
      state = 'line-comment';
      cursor += 1;
    } else if (character === '/' && next === '*') {
      state = 'block-comment';
      cursor += 1;
    } else if (character === '"') {
      state = 'string';
    } else if (character === "'") {
      state = 'character';
    } else if (character === '{') {
      depth += 1;
    } else if (character === '}') {
      depth -= 1;
    }
    cursor += 1;
  }
  return depth === 0 ? source.slice(match.index, cursor) : '';
}

function rustPublicFunctionDefinition(source, name) {
  const header = new RegExp(`\\bpub\\s+fn\\s+${name}\\b[^\\{]*\\{`);
  const match = header.exec(source);
  if (!match) return '';

  let depth = 1;
  let cursor = match.index + match[0].length;
  while (cursor < source.length && depth > 0) {
    if (source[cursor] === '{') depth += 1;
    else if (source[cursor] === '}') depth -= 1;
    cursor += 1;
  }
  return depth === 0 ? source.slice(match.index, cursor) : '';
}

function rustNamedFunctionDefinition(source, name) {
  const header = new RegExp(`\\bfn\\s+${name}\\b[^\\{]*\\{`);
  const match = header.exec(source);
  if (!match) return '';

  let depth = 1;
  let cursor = match.index + match[0].length;
  while (cursor < source.length && depth > 0) {
    if (source[cursor] === '{') depth += 1;
    else if (source[cursor] === '}') depth -= 1;
    cursor += 1;
  }
  return depth === 0 ? source.slice(match.index, cursor) : '';
}

function normalizeRust(source) {
  return source.replace(/\s+/g, ' ').trim();
}

function compactRust(source) {
  return source.replace(/\s+/g, '');
}

function importedPaths(source) {
  const clean = withoutCommentsAndStrings(source);
  const paths = [];
  for (const match of clean.matchAll(/\b(?:pub\s+)?use\s+([^;]+);/g)) paths.push(match[1]);
  for (const match of clean.matchAll(/\b(?:pub\s+)?(?:mod|extern\s+crate)\s+([A-Za-z0-9_]+)/g)) {
    paths.push(match[1]);
  }
  return paths;
}

function tomlDottedKey(key) {
  const segments = [];
  let cursor = 0;
  while (cursor < key.length) {
    while (/\s/.test(key[cursor])) cursor += 1;
    if (cursor >= key.length) return null;
    const quote = key[cursor] === '"' || key[cursor] === "'" ? key[cursor] : null;
    let segment = '';
    if (quote) {
      cursor += 1;
      let closed = false;
      while (cursor < key.length) {
        const character = key[cursor];
        if (character === quote) {
          cursor += 1;
          closed = true;
          break;
        }
        if (quote === '"' && character === '\\') {
          // Dependency names never require escapes. Retain them as an invalid
          // name so the positive allowlist fails closed rather than aliasing
          // an escaped spelling to an allowed crate.
          segment += character;
          cursor += 1;
          if (cursor < key.length) segment += key[cursor];
          cursor += 1;
          continue;
        }
        segment += character;
        cursor += 1;
      }
      if (!closed) return null;
    } else {
      const match = key.slice(cursor).match(/^[A-Za-z0-9_-]+/);
      if (!match) return null;
      segment = match[0];
      cursor += segment.length;
    }
    segments.push(segment);
    while (/\s/.test(key[cursor])) cursor += 1;
    if (cursor >= key.length) break;
    if (key[cursor] !== '.') return null;
    cursor += 1;
  }
  return segments;
}

function cargoDependencyDeclarations(manifest) {
  let section = '';
  let sectionSegments = [];
  let tableDeclaration = null;
  const declarations = [];
  for (const rawLine of manifest.split('\n')) {
    const line = rawLine.replace(/#.*/, '').trim();
    const header = line.match(/^\[([^\]]+)\]$/);
    if (header) {
      sectionSegments = tomlDottedKey(header[1]) ?? [];
      section = sectionSegments.join('.');
      tableDeclaration = null;
      const dependencyIndex = sectionSegments.length - 2;
      if (dependencyIndex >= 0
          && /^(?:dev-|build-)?dependencies$/.test(sectionSegments[dependencyIndex])) {
        tableDeclaration = {
          section: sectionSegments.slice(0, -1).join('.'),
          name: sectionSegments.at(-1),
          value: '',
        };
        declarations.push(tableDeclaration);
      }
      continue;
    }
    if (tableDeclaration) {
      const assignment = line.match(/^(.+?)\s*=\s*(.+)$/);
      const key = assignment ? tomlDottedKey(assignment[1]) : null;
      if (key?.length === 1 && key[0] === 'package') {
        tableDeclaration.value += ` package = ${assignment[2]}`;
      }
      continue;
    }
    if (!/^(?:dev-|build-)?dependencies$/.test(sectionSegments.at(-1) ?? '')) continue;
    const declaration = line.match(/^(.+?)\s*=\s*(.+)$/);
    const key = declaration ? tomlDottedKey(declaration[1]) : null;
    if (key?.length === 1) {
      declarations.push({ section, name: key[0], value: declaration[2] });
    } else if (line) {
      declarations.push({ section, name: '<unparsed>', value: line });
    }
  }
  return declarations;
}

function check(root = repoRoot) {
  const failures = [];
  const requireCondition = (condition, message) => {
    if (!condition) failures.push(message);
  };

  const workspace = read(root, 'Cargo.toml');
  const manifestPath = join(root, 'crates/agent-browser-service-model/Cargo.toml');
  const sourceRoot = join(root, 'crates/agent-browser-service-model/src');
  const manifest = read(root, 'crates/agent-browser-service-model/Cargo.toml');
  const sourceFiles = rustFilesUnder(sourceRoot);
  const sources = sourceFiles.map((path) => readFileSync(path, 'utf8'));
  const modelSource = sources.join('\n');
  requireCondition(existsSync(manifestPath), 'agent-browser-service-model Cargo manifest must exist');
  requireCondition(existsSync(join(sourceRoot, 'lib.rs')), 'agent-browser-service-model must own src/lib.rs');
  requireCondition(
    workspace.includes('crates/agent-browser-service-model'),
    'root Cargo workspace must include agent-browser-service-model',
  );
  requireCondition(
    /\bname\s*=\s*"agent-browser-service-model"/.test(manifest),
    'agent-browser-service-model manifest must declare the expected package name',
  );

  for (const dependency of cargoDependencyDeclarations(manifest)) {
    requireCondition(
      dependency.section === 'dependencies'
        && SERVICE_MODEL_ALLOWED_DEPENDENCIES.has(dependency.name),
      `service-model crate dependency is outside the provider-free allowlist: ${dependency.section}.${dependency.name}`,
    );
    requireCondition(
      !/(?:\bpackage\b|["']package["'])\s*=/.test(dependency.value),
      `service-model crate must not alias an allowed dependency: ${dependency.name}`,
    );
  }
  requireCondition(
    !/\bpath\s*=\s*["'][^"']*(?:^|[/\\])cli(?:[/\\]|["'])/m.test(manifest),
    'service-model crate must not path-depend on the CLI crate',
  );

  for (const source of sources) {
    const clean = withoutCommentsAndStrings(source);
    for (const forbidden of FORBIDDEN_IMPORT_PATHS) {
      requireCondition(
        !new RegExp(`\\b${forbidden.replaceAll(':', '\\s*:\\s*')}`).test(clean),
        `service-model Rust source must not reference forbidden boundary: ${forbidden}`,
      );
    }
    for (const path of importedPaths(source)) {
      const segments = path.trim().split(/\s*::\s*/);
      requireCondition(
        !segments.some((segment) => FORBIDDEN_ADAPTER_MODULES.has(segment)),
        `service-model Rust source must not import runtime/provider/platform adapter module: ${path.trim()}`,
      );
    }
  }

  const retirement = withoutCommentsAndStrings(read(root,
    'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'));
  const crashRegenerationPath = join(sourceRoot, 'crash_regeneration.rs');
  const serviceModelLib = withoutCommentsAndStrings(read(root,
    'crates/agent-browser-service-model/src/lib.rs'));
  const runtimeOwnerProjectionPath = join(sourceRoot, 'runtime_owner_projection.rs');
  const runtimeOwnerProjectionSource = read(root,
    'crates/agent-browser-service-model/src/runtime_owner_projection.rs');
  const runtimeOwnerProjectionProduction = withoutCommentsAndStrings(
    withoutCfgTestItems(runtimeOwnerProjectionSource),
  );
  const crashRegenerationSource = read(root,
    'crates/agent-browser-service-model/src/crash_regeneration.rs');
  const crashRegeneration = withoutCommentsAndStrings(read(root,
    'crates/agent-browser-service-model/src/crash_regeneration.rs'));
  const capabilityRegistryPath = join(sourceRoot, 'browser_capability_registry.rs');
  const capabilityRegistrySource = read(root,
    'crates/agent-browser-service-model/src/browser_capability_registry.rs');
  const capabilityRegistry = withoutCommentsAndStrings(capabilityRegistrySource);
  const serviceAuthenticationPath = join(sourceRoot, 'service_authentication_run.rs');
  const serviceAuthenticationSource = read(root,
    'crates/agent-browser-service-model/src/service_authentication_run.rs');
  const serviceAuthentication = withoutCommentsAndStrings(serviceAuthenticationSource);
  const serviceChallengePath = join(sourceRoot, 'service_challenge_task.rs');
  const serviceChallengeSource = read(root,
    'crates/agent-browser-service-model/src/service_challenge_task.rs');
  const serviceChallenge = withoutCommentsAndStrings(serviceChallengeSource);
  const serviceStatePath = join(sourceRoot, 'service_state.rs');
  const serviceStateSource = read(root, 'crates/agent-browser-service-model/src/service_state.rs');
  const serviceState = withoutCommentsAndStrings(serviceStateSource);
  const serviceStateCode = serviceStateSource.split('#[cfg(test)]', 1)[0];
  const principalContinuityPath = join(sourceRoot, 'principal_continuity.rs');
  const principalContinuitySource = read(root,
    'crates/agent-browser-service-model/src/principal_continuity.rs');
  const canonicalServiceState = withoutComments(serviceStateCode)
    .replace(/\bcrate\s*::/g, 'agent_browser_service_model::');
  const cliSources = rustFilesUnder(join(root, 'cli/src'))
    .map((path) => withoutCommentsAndStrings(readFileSync(path, 'utf8')));
  const cliProductionEntries = rustFilesUnder(join(root, 'cli/src'))
    .filter((path) => !/(?:^|[/\\])(?:tests?[/\\]|(?:[^/\\]*_)?tests?\.rs$)/.test(path))
    .map((path) => ({
      path,
      source: withoutCommentsAndStrings(withoutCfgTestItems(readFileSync(path, 'utf8'))),
    }));
  const cliProductionSources = cliProductionEntries.map(({ source }) => source);

  const authenticationManifestPath = join(root,
    'crates/agent-browser-authentication-control/Cargo.toml');
  const authenticationSourceRoot = join(root,
    'crates/agent-browser-authentication-control/src');
  const authenticationManifest = read(root,
    'crates/agent-browser-authentication-control/Cargo.toml');
  const authenticationSourceFiles = rustFilesUnder(authenticationSourceRoot);
  const authenticationSources = authenticationSourceFiles
    .map((path) => readFileSync(path, 'utf8'));
  const authenticationSource = authenticationSources.join('\n');
  const authenticationClean = withoutCommentsAndStrings(authenticationSource);
  const cliAuthentication = withoutCommentsAndStrings(read(root,
    'cli/src/native/authentication_run.rs'));
  const cliPrincipal = withoutCommentsAndStrings(withoutCfgTestItems(read(root,
    'cli/src/native/service_principal.rs')));
  const cliServiceStore = withoutCommentsAndStrings(withoutCfgTestItems(read(root,
    'cli/src/native/service_store.rs')));
  const cliRuntimeLifecycle = withoutCommentsAndStrings(withoutCfgTestItems(read(root,
    'cli/src/native/runtime_lifecycle.rs')));
  const cliRuntimeOwnerProjectionSources = RUNTIME_OWNER_PROJECTION_CLI_FILES.map((path) => ({
    path,
    source: withoutCommentsAndStrings(withoutCfgTestItems(read(root, path))),
  }));

  requireCondition(existsSync(authenticationManifestPath),
    'agent-browser-authentication-control Cargo manifest must exist');
  requireCondition(existsSync(join(authenticationSourceRoot, 'lib.rs')),
    'agent-browser-authentication-control must own src/lib.rs');
  requireCondition(
    workspace.includes('crates/agent-browser-authentication-control'),
    'root Cargo workspace must include agent-browser-authentication-control',
  );
  requireCondition(
    /\bname\s*=\s*"agent-browser-authentication-control"/.test(authenticationManifest),
    'agent-browser-authentication-control manifest must declare the expected package name',
  );
  const authenticationDependencies = cargoDependencyDeclarations(authenticationManifest);
  for (const dependency of authenticationDependencies) {
    const allowed = (dependency.section === 'dependencies' && dependency.name === 'serde')
      || (dependency.section === 'dev-dependencies' && dependency.name === 'serde_json');
    requireCondition(
      allowed,
      `authentication-control crate dependency is outside the provider-free allowlist: ${dependency.section}.${dependency.name}`,
    );
    requireCondition(
      !/(?:\bpackage\b|["']package["'])\s*=/.test(dependency.value),
      `authentication-control crate must not alias an allowed dependency: ${dependency.name}`,
    );
  }
  requireCondition(
    !/\bpath\s*=\s*["'][^"']*(?:^|[/\\])cli(?:[/\\]|["'])/m.test(authenticationManifest),
    'authentication-control crate must not path-depend on the CLI crate',
  );
  for (const source of authenticationSources) {
    const clean = withoutCommentsAndStrings(source);
    for (const forbidden of FORBIDDEN_IMPORT_PATHS) {
      requireCondition(
        !new RegExp(`\\b${forbidden.replaceAll(':', '\\s*:\\s*')}`).test(clean),
        `authentication-control Rust source must not reference forbidden boundary: ${forbidden}`,
      );
    }
    for (const path of importedPaths(source)) {
      const segments = path.trim().split(/\s*::\s*/);
      requireCondition(
        !segments.some((segment) => FORBIDDEN_ADAPTER_MODULES.has(segment)),
        `authentication-control Rust source must not import runtime/provider/platform adapter module: ${path.trim()}`,
      );
    }
  }
  for (const name of AUTHENTICATION_CONTROL_DEFINITIONS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...authenticationSource.matchAll(definition)].length === 1,
      `authentication-control crate must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate authentication-control definition: ${name}`,
    );
  }
  for (const name of AUTHENTICATION_CONTROL_TRAITS) {
    const definition = new RegExp(`\\btrait\\s+${name}\\b`, 'g');
    requireCondition(
      [...authenticationSource.matchAll(definition)].length === 1,
      `authentication-control crate must own exactly one trait: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate authentication-control trait: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+const\s+AUTHENTICATION_RUN_SCHEMA_VERSION\b/.test(authenticationClean),
    'authentication-control crate must own the authentication run schema constant',
  );
  requireCondition(
    !cliSources.some((source) =>
      /\b(?:pub\s*)?(?:\(?crate\)?\s*)?const\s+AUTHENTICATION_RUN_SCHEMA_VERSION\b/.test(source)),
    'CLI must not duplicate the authentication run schema constant',
  );
  requireCondition(
    /^\s*pub\s*\(\s*crate\s*\)\s+use\s+agent_browser_authentication_control\s*::\s*\*\s*;\s*$/.test(
      cliAuthentication,
    ),
    'CLI authentication_run module must contain only the authentication-control compatibility re-export',
  );
  for (const adapter of [
    'ServiceState', 'ServiceStateRepository', 'ServiceAuthenticationRunRecord',
    'BrowserManager', 'CredentialResolver', 'ProviderClient', 'from_environment',
  ]) {
    requireCondition(!new RegExp(`\\b${adapter}\\b`).test(authenticationClean),
      `authentication-control model must not absorb CLI adapter: ${adapter}`);
  }
  requireCondition(!/\bstd\s*::\s*(?:env|fs|net|process)\b/.test(authenticationClean),
    'authentication-control model must not access environment, filesystem, network, or processes');
  requireCondition(existsSync(crashRegenerationPath),
    'service-model must own src/crash_regeneration.rs');
  const crashRegenerationExport = serviceModelLib.match(
    /\bpub\s+use\s+crash_regeneration\s*::\s*\{([\s\S]*?)\}\s*;/,
  );
  requireCondition(/\bmod\s+crash_regeneration\s*;/.test(serviceModelLib),
    'service-model lib must declare the crash regeneration module');
  requireCondition(Boolean(crashRegenerationExport),
    'service-model lib must export the crash regeneration interface');
  for (const name of CRASH_REGENERATION_DEFINITIONS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...crashRegenerationSource.matchAll(definition)].length === 1,
      `service-model crash regeneration module must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate crash regeneration definition: ${name}`,
    );
    requireCondition(
      Boolean(crashRegenerationExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export crash regeneration definition: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+const\s+CRASH_REGENERATION_STATUS_SCHEMA_VERSION\b/.test(crashRegeneration),
    'service-model must own the crash regeneration status schema constant',
  );
  requireCondition(
    !cliSources.some((source) => /\b(?:pub\s*)?(?:\(?crate\)?\s*)?const\s+CRASH_REGENERATION_STATUS_SCHEMA_VERSION\b/.test(source)),
    'CLI must not duplicate crash regeneration status schema constant',
  );
  requireCondition(
    Boolean(crashRegenerationExport?.[1].match(/\bCRASH_REGENERATION_STATUS_SCHEMA_VERSION\b/)),
    'service-model lib must export the crash regeneration status schema constant',
  );
  const cliServiceModelSource = read(root, 'cli/src/native/service_model.rs');
  const cliServiceModel = withoutCommentsAndStrings(cliServiceModelSource);
  requireCondition(existsSync(serviceStatePath), 'service-model must own src/service_state.rs');
  const serviceStateDefinitions = [...serviceStateCode.matchAll(/\b(?:pub\s+)?struct\s+ServiceState\b/g)];
  requireCondition(serviceStateDefinitions.length === 1,
    'service-model aggregate module must own exactly one ServiceState struct');
  requireCondition([...serviceStateCode.matchAll(/\bimpl\s+ServiceState\b/g)].length === 1,
    'service-model aggregate module must own exactly one inherent ServiceState impl');
  requireCondition(
    sources.reduce((count, source) => count
      + [...withoutComments(source).matchAll(/^\s*impl\s+ServiceState\b/gm)].length, 0) === 1,
    'service-model crate must own exactly one inherent ServiceState impl globally',
  );
  requireCondition(!/\bsuper\s*::/.test(serviceStateCode),
    'service-model aggregate must not use super:: imports');
  requireCondition(!/\b(?:crate|agent_browser_service_model)\s*::\s*native\b/.test(serviceState),
    'service-model aggregate must not import upward CLI adapters');
  const serviceStateStruct = rustStructDefinition(serviceStateCode, 'ServiceState');
  const hiddenFields = [...serviceStateStruct.matchAll(
    /#\s*\[\s*doc\s*\(\s*hidden\s*\)\s*\]\s*(?:pub(?:\([^)]*\))?\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/g,
  )].map((match) => match[1]);
  requireCondition(hiddenFields.length === 20,
    `service-model aggregate must mark exactly 20 migration fields #[doc(hidden)] (found ${hiddenFields.length})`);
  for (const field of SERVICE_STATE_MIGRATION_FIELDS) {
    requireCondition(hiddenFields.includes(field),
      `service-model aggregate migration field must be #[doc(hidden)]: ${field}`);
  }
  for (const [field, pattern] of SERVICE_STATE_DIRECT_OWNER_FIELDS) {
    requireCondition(
      pattern.test(canonicalServiceState),
      `service-model ServiceState field must use its direct canonical owner: ${field}`,
    );
  }
  const servicePrincipalField = canonicalServiceState.match(
    /((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?service_principals\s*:/,
  );
  requireCondition(
    Boolean(servicePrincipalField?.[1].match(
      /#\s*\[\s*serde\s*\([^\]]*skip_serializing_if\s*=\s*["']agent_browser_lease_authority\s*::\s*ServicePrincipalRegistry\s*::\s*is_empty["'][^\]]*\)\s*\]/,
    )),
    'service-model ServiceState must use the direct Lease Authority principal omission predicate',
  );
  requireCondition(
    /BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*CrashRegenerationTransaction\s*>/
      .test(canonicalServiceState),
    'service-model ServiceState must use the canonical crash regeneration transaction',
  );
  for (const name of SERVICE_STATE_CRASH_REGENERATION_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own crash-regeneration aggregate method: ${name}`,
    );
  }
  requireCondition(
    !cliProductionSources.some((source) => /\.\s*crash_regeneration_transactions\b/.test(source)),
    'CLI production code must not access ServiceState.crash_regeneration_transactions directly',
  );
  for (const name of SERVICE_STATE_CODEC_EXPORTS) {
    requireCondition(new RegExp(`\\b(?:pub\\s+)?(?:fn|struct|enum|type|const)\\s+${name}\\b`).test(serviceStateCode),
      `service-model aggregate must own codec/revision interface: ${name}`);
    requireCondition(name === 'validate_service_state_invariants' || !cliSources.some((source) =>
      new RegExp(`\\b(?:fn|struct|enum|type|const)\\s+${name}\\b`).test(source)),
    `CLI must not duplicate service-model codec/revision interface: ${name}`);
  }
  requireCondition(/\bpub\s+fn\s+state_revision\s*\(/.test(serviceStateCode),
    'service-model aggregate must own the state_revision accessor');
  requireCondition(/\bpub\s+fn\s+from_configured_entities\s*\(/.test(serviceStateCode),
    'service-model aggregate must own the configured-entities constructor');
  for (const name of SERVICE_STATE_LEASE_AUTHORITY_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own Lease Authority method: ${name}`,
    );
  }
  requireCondition(
    !cliProductionSources.some((source) => /\.\s*lease_authority\b(?!\s*\()/.test(source)),
    'CLI production code must not access ServiceState.lease_authority directly',
  );
  for (const name of SERVICE_STATE_RECEIPT_TYPES) {
    requireCondition(
      new RegExp(`\\bpub\\s+(?:struct|enum)\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own receipt interface type: ${name}`,
    );
  }
  for (const name of SERVICE_STATE_RECEIPT_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own receipt method: ${name}`,
    );
  }
  for (const field of SERVICE_STATE_RECEIPT_FIELDS) {
    requireCondition(
      !cliProductionSources.some((source) => new RegExp(`\\.\\s*${field}\\b`).test(source)),
      `CLI production code must not access ServiceState.${field} directly`,
    );
  }
  for (const name of SERVICE_STATE_PRINCIPAL_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own principal-authority method: ${name}`,
    );
  }
  requireCondition(
    !cliProductionSources.some((source) => /\.\s*service_principals\b/.test(source)),
    'CLI production code must not access ServiceState.service_principals directly',
  );
  requireCondition(existsSync(principalContinuityPath),
    'service-model must own src/principal_continuity.rs');
  for (const name of PRINCIPAL_CONTINUITY_DEFINITIONS) {
    const definition = new RegExp(`\\bpub\\s+(?:struct|enum)\\s+${name}\\b`, 'g');
    requireCondition(
      [...principalContinuitySource.matchAll(definition)].length === 1,
      `service-model principal-continuity module must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliProductionSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate principal-continuity definition: ${name}`,
    );
  }
  for (const name of SERVICE_STATE_CONTINUITY_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own principal-continuity method: ${name}`,
    );
  }
  requireCondition(
    !/\.\s*(?:runtime_owner_registry|sessions|tabs)\b/.test(cliPrincipal),
    'CLI service_principal production facade must not retain principal-continuity state decisions',
  );
  for (const name of RUNTIME_OWNER_PERSISTENCE_DEFINITIONS) {
    const definition = new RegExp(`\\bpub\\s+struct\\s+${name}\\b`, 'g');
    requireCondition(
      [...modelSource.matchAll(definition)].length === 1,
      `service-model must own exactly one runtime-owner persistence type: ${name}`,
    );
  }
  requireCondition(existsSync(runtimeOwnerProjectionPath),
    'service-model must own src/runtime_owner_projection.rs');
  for (const name of RUNTIME_OWNER_PROJECTION_DEFINITIONS) {
    const definition = new RegExp(`\\bpub\\s+struct\\s+${name}\\b`, 'g');
    requireCondition(
      [...modelSource.matchAll(definition)].length === 1,
      `service-model must own exactly one runtime-owner projection type: ${name}`,
    );
    const projection = rustStructDefinition(modelSource, name);
    requireCondition(
      !/\bRuntimeOwnerRegistry\b|&\s*mut\b|\b(?:Fn|FnMut|FnOnce)\s*[<(]/.test(projection),
      `runtime-owner projection type must not expose registry, mutable, or callback access: ${name}`,
    );
  }
  for (const name of RUNTIME_OWNER_PROJECTION_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own runtime-owner projection method: ${name}`,
    );
    const signature = serviceStateCode.match(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b[^\\{]*\\{`),
    )?.[0] ?? '';
    requireCondition(
      !/\bRuntimeOwnerRegistry\b|&\s*mut\b|->\s*impl\s+Iterator\b|\b(?:Fn|FnMut|FnOnce)\s*[<(]/.test(signature),
      `runtime-owner projection method must not expose registry, mutable, iterator, or callback access: ${name}`,
    );
  }
  const atomicLifecycleMethod = rustPublicFunctionDefinition(
    serviceStateCode,
    ATOMIC_RUNTIME_LIFECYCLE_METHOD,
  );
  requireCondition(
    normalizeRust(atomicLifecycleMethod) === normalizeRust(`
      pub fn apply_runtime_lifecycle_transition_atomically(
        &mut self,
        intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
      ) -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {
        let mut staged = self.runtime_owner_registry.clone();
        let transition = staged.apply_lifecycle_transition(intent)?;
        self.runtime_owner_registry = staged;
        Ok(transition)
      }
    `),
    'atomic runtime lifecycle method must preserve its exact signature and clone, apply, commit, return body',
  );
  const atomicProfileSyncLifecycleMethod = rustPublicFunctionDefinition(
    serviceStateCode,
    ATOMIC_PROFILE_SYNC_LIFECYCLE_METHOD,
  );
  requireCondition(
    compactRust(atomicProfileSyncLifecycleMethod) === compactRust(`
      pub fn apply_runtime_lifecycle_transition_with_profile_sync_atomically(
        &mut self,
        profile_id: &str,
        user_data_dir: String,
        intent: agent_browser_lease_authority::RuntimeLifecycleIntent,
      ) -> Result<agent_browser_lease_authority::RuntimeLifecycleTransition, String> {
        let profile = self
          .profiles
          .get(profile_id)
          .ok_or_else(|| "runtime_lifecycle_profile_record_missing".to_string())?;
        if profile.id != profile_id || profile_id.trim().is_empty() {
          return Err("runtime_lifecycle_profile_record_sync_rejected".to_string());
        }
        let mut staged = self.runtime_owner_registry.clone();
        let transition = staged.apply_lifecycle_transition(intent)?;
        self.profiles
          .get_mut(profile_id)
          .expect("validated profile remains present")
          .user_data_dir = Some(user_data_dir);
        self.runtime_owner_registry = staged;
        Ok(transition)
      }
    `),
    'profile-sync runtime lifecycle method must preserve its exact validation, stage, apply, cross-field commit, return body',
  );
  const ordinaryTransition = cliRuntimeLifecycle.match(
    /pub\s*\(\s*crate\s*\)\s+fn\s+transition\b[\s\S]*?(?=\n\s*fn\s+transition_terminal_replacement_with_profile_sync\b)/,
  )?.[0] ?? '';
  requireCondition(
    ordinaryTransition.includes(ATOMIC_RUNTIME_LIFECYCLE_METHOD),
    'CLI ordinary runtime lifecycle transition must delegate to the aggregate atomic method',
  );
  requireCondition(
    !/\.\s*runtime_owner_registry\b/.test(ordinaryTransition),
    'CLI ordinary runtime lifecycle transition must not clone or assign the aggregate registry field',
  );
  const terminalProfileSyncTransition = rustNamedFunctionDefinition(
    cliRuntimeLifecycle,
    'transition_terminal_replacement_with_profile_sync',
  );
  const compactTerminalProfileSyncTransition = compactRust(terminalProfileSyncTransition);
  const pathConversionIndex = compactTerminalProfileSyncTransition.indexOf(
    'profile_root.to_str()',
  );
  const repositoryMutationIndex = compactTerminalProfileSyncTransition.indexOf(
    'self.repository.mutate',
  );
  const profileLookupIndex = compactTerminalProfileSyncTransition.indexOf(
    '.profiles.get(profile_id)',
  );
  const routeValidationIndex = compactTerminalProfileSyncTransition.indexOf(
    'canonical_route_viewer_runtime_profile(profile_id)',
  );
  const nonblankValidationIndex = compactTerminalProfileSyncTransition.indexOf(
    'profile_id.trim().is_empty()',
  );
  const intentPreparationIndex = compactTerminalProfileSyncTransition.indexOf(
    'prepare_lifecycle_intent',
  );
  const aggregateProfileSyncIndex = compactTerminalProfileSyncTransition.indexOf(
    ATOMIC_PROFILE_SYNC_LIFECYCLE_METHOD,
  );
  requireCondition(
    pathConversionIndex >= 0
      && repositoryMutationIndex > pathConversionIndex
      && profileLookupIndex > repositoryMutationIndex
      && routeValidationIndex > profileLookupIndex
      && nonblankValidationIndex > routeValidationIndex
      && intentPreparationIndex > nonblankValidationIndex
      && aggregateProfileSyncIndex > intentPreparationIndex,
    'CLI terminal profile-sync transition must preserve path, profile, route, and nonblank preflight before intent preparation and aggregate mutation',
  );
  requireCondition(
    !/\.\s*runtime_owner_registry\b/.test(terminalProfileSyncTransition)
      && !/\.\s*profiles\s*\.\s*get_mut\b/.test(terminalProfileSyncTransition)
      && !/\.\s*user_data_dir\s*=/.test(terminalProfileSyncTransition)
      && !/\bapply_transition\s*\(/.test(terminalProfileSyncTransition),
    'CLI terminal profile-sync transition must not retain direct registry or profile-path mutation',
  );
  requireCondition(
    !/\bRuntimeOwnerPersistenceSnapshot\b/.test(runtimeOwnerProjectionSource),
    'runtime-owner projections must not reuse the persistence snapshot as an access path',
  );
  requireCondition(
    !/\bRuntimeOwnerRegistry\b/.test(runtimeOwnerProjectionProduction)
      && !/\bpub\s+fn\b/.test(runtimeOwnerProjectionProduction)
      && !/\bimpl\b/.test(runtimeOwnerProjectionProduction),
    'runtime-owner projection module must remain data-only without registry, function, or impl escape hatches',
  );
  for (const { path, source } of cliRuntimeOwnerProjectionSources) {
    requireCondition(
      !/\.\s*runtime_owner_registry\b/.test(source),
      `CLI runtime-owner projection caller must not access ServiceState.runtime_owner_registry directly: ${path}`,
    );
    requireCondition(
      !/\bfn\s+\w+\s*\([^)]*\bRuntimeOwnerRegistry\b/s.test(source),
      `CLI runtime-owner projection caller must not accept a raw RuntimeOwnerRegistry: ${path}`,
    );
  }
  for (const name of RUNTIME_OWNER_PERSISTENCE_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own runtime-owner persistence method: ${name}`,
    );
  }
  requireCondition(
    !/\.\s*runtime_owner_registry\b/.test(cliServiceStore),
    'CLI service_store production code must not access ServiceState.runtime_owner_registry directly',
  );
  requireCondition(
    !/\.\s*(?:restore_lifecycle_records|persistence_projection_without_lifecycle_records)\s*\(/.test(cliServiceStore),
    'CLI service_store must delegate runtime-owner lifecycle persistence decisions to ServiceState',
  );
  requireCondition(
    !/\bimpl\s+RuntimeOwnerPersistenceSnapshot\b/.test(modelSource)
      && !/impl\s+(?:(?:std\s*::\s*(?:ops|convert|borrow)\s*::\s*)?)(?:DerefMut|Deref|AsRef|AsMut|Borrow|BorrowMut|From|Into)\b[^\{]*RuntimeOwnerPersistenceSnapshot\b/.test(modelSource),
    'runtime-owner persistence snapshot must not expose inherent methods or conversion escape hatches',
  );
  const persistenceSnapshotDefinition = rustStructDefinition(
    modelSource,
    'RuntimeOwnerPersistenceSnapshot',
  );
  requireCondition(
    !/\bpub(?:\([^)]*\))?\s+registry\s*:/.test(persistenceSnapshotDefinition),
    'runtime-owner persistence snapshot registry field must remain private',
  );
  const persistenceCalls = new RegExp(
    `\\.\\s*(?:${RUNTIME_OWNER_PERSISTENCE_METHODS.join('|')})\\s*\\(`,
  );
  requireCondition(
    !cliProductionEntries.some(({ path, source }) =>
      !/(?:^|[/\\])service_store\.rs$/.test(path) && persistenceCalls.test(source)),
    'runtime-owner persistence methods may be called only by the CLI service_store adapter',
  );
  const serviceStateExport = serviceModelLib.match(/\bpub\s+use\s+service_state\s*::\s*\{([\s\S]*?)\}\s*;/);
  requireCondition(/\bmod\s+service_state\s*;/.test(serviceModelLib),
    'service-model lib must declare the ServiceState aggregate module');
  requireCondition(Boolean(serviceStateExport),
    'service-model lib must export the ServiceState aggregate interface');
  for (const name of SERVICE_STATE_CODEC_EXPORTS) {
    requireCondition(Boolean(serviceStateExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export ServiceState aggregate interface: ${name}`);
  }
  for (const name of SERVICE_STATE_RECEIPT_TYPES) {
    requireCondition(Boolean(serviceStateExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export ServiceState receipt interface: ${name}`);
  }
  for (const name of RUNTIME_OWNER_PERSISTENCE_DEFINITIONS) {
    requireCondition(Boolean(serviceStateExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export runtime-owner persistence type: ${name}`);
  }
  for (const name of RUNTIME_OWNER_PROJECTION_DEFINITIONS) {
    requireCondition(
      new RegExp(`\\bpub\\s+use\\b[^;]*\\b${name}\\b[^;]*;`, 's').test(serviceModelLib),
      `service-model lib must export runtime-owner projection type: ${name}`,
    );
  }
  const principalContinuityExport = serviceModelLib.match(
    /\bpub\s+use\s+principal_continuity\s*::\s*\{([\s\S]*?)\}\s*;/,
  );
  requireCondition(Boolean(principalContinuityExport),
    'service-model lib must export the principal-continuity interface');
  for (const name of PRINCIPAL_CONTINUITY_DEFINITIONS) {
    requireCondition(Boolean(principalContinuityExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export principal-continuity definition: ${name}`);
  }
  requireCondition(/\bpub\s+use\s+agent_browser_service_model\s*::\s*(?:ServiceState\s*;|\{[^}]*\bServiceState\b[^}]*\}\s*;)/s.test(cliServiceModelSource)
    && !/\b(?:struct|enum|type)\s+ServiceState\b/.test(cliServiceModel)
    && !/\bimpl\s+ServiceState\b/.test(cliServiceModel),
  'CLI service_model module must be a reexport-only ServiceState facade');
  requireCondition(existsSync(capabilityRegistryPath),
    'service-model must own src/browser_capability_registry.rs');
  const capabilityRegistryExport = serviceModelLib.match(
    /\bpub\s+use\s+browser_capability_registry\s*::\s*\{([\s\S]*?)\}\s*;/,
  );
  requireCondition(/\bmod\s+browser_capability_registry\s*;/.test(serviceModelLib),
    'service-model lib must declare the browser capability registry module');
  requireCondition(Boolean(capabilityRegistryExport),
    'service-model lib must export the browser capability registry interface');
  for (const name of CAPABILITY_REGISTRY_DEFINITIONS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...capabilityRegistrySource.matchAll(definition)].length === 1,
      `service-model capability registry module must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate capability registry definition: ${name}`,
    );
    requireCondition(
      Boolean(capabilityRegistryExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export capability registry definition: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+fn\s+browser_profile_compatibility_matches\b/.test(capabilityRegistry),
    'service-model must own browser_profile_compatibility_matches',
  );
  requireCondition(
    !cliSources.some((source) => /\bfn\s+browser_profile_compatibility_matches\b/.test(source)),
    'CLI must not duplicate browser_profile_compatibility_matches',
  );
  requireCondition(
    Boolean(capabilityRegistryExport?.[1]
      .match(/\bbrowser_profile_compatibility_matches\b/)),
    'service-model lib must export browser_profile_compatibility_matches',
  );
  requireCondition(
    /browser_capability_registry\s*:\s*agent_browser_service_model\s*::\s*BrowserCapabilityRegistry/
      .test(canonicalServiceState),
    'service-model ServiceState must use the canonical browser capability registry',
  );
  requireCondition(existsSync(serviceAuthenticationPath),
    'service-model must own src/service_authentication_run.rs');
  const serviceAuthenticationExport = serviceModelLib.match(
    /\bpub\s+use\s+service_authentication_run\s*::\s*\{([\s\S]*?)\}\s*;/,
  );
  requireCondition(/\bmod\s+service_authentication_run\s*;/.test(serviceModelLib),
    'service-model lib must declare the Service authentication module');
  requireCondition(Boolean(serviceAuthenticationExport),
    'service-model lib must export the Service authentication interface');
  for (const name of SERVICE_AUTHENTICATION_DEFINITIONS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...serviceAuthenticationSource.matchAll(definition)].length === 1,
      `service-model Service authentication module must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate Service authentication definition: ${name}`,
    );
    requireCondition(
      Boolean(serviceAuthenticationExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export Service authentication definition: ${name}`,
    );
  }
  for (const name of SERVICE_AUTHENTICATION_DECISIONS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceAuthentication),
      `service-model must own Service authentication decision: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(`\\bfn\\s+${name}\\b`).test(source)),
      `CLI must not duplicate Service authentication decision: ${name}`,
    );
    requireCondition(
      Boolean(serviceAuthenticationExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export Service authentication decision: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+const\s+SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION\b/.test(serviceAuthentication),
    'service-model must own the Service authentication schema constant',
  );
  requireCondition(
    !cliSources.some((source) =>
      /\b(?:pub\s*)?(?:\(?crate\)?\s*)?const\s+SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION\b/
        .test(source)),
    'CLI must not duplicate the Service authentication schema constant',
  );
  requireCondition(
    Boolean(serviceAuthenticationExport?.[1]
      .match(/\bSERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION\b/)),
    'service-model lib must export the Service authentication schema constant',
  );
  requireCondition(
    /authentication_runs\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ServiceAuthenticationRunRecord\s*>/
      .test(canonicalServiceState),
    'service-model ServiceState must use the canonical Service authentication record',
  );
  const serviceAuthenticationField = canonicalServiceState.match(
    /((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?authentication_runs\s*:/,
  );
  requireCondition(
    Boolean(serviceAuthenticationField?.[1].match(
      /#\s*\[\s*serde\s*\([^\]]*skip_serializing_if\s*=\s*["']agent_browser_service_model\s*::\s*authentication_run_map_is_empty["'][^\]]*\)\s*\]/,
    )),
    'service-model ServiceState must use the canonical Service authentication empty-map decision',
  );
  for (const name of SERVICE_STATE_AUTHENTICATION_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own Service authentication aggregate method: ${name}`,
    );
  }
  requireCondition(
    !cliProductionSources.some((source) => /\.\s*authentication_runs\b/.test(source)),
    'CLI production code must not access ServiceState.authentication_runs directly',
  );
  requireCondition(existsSync(serviceChallengePath),
    'service-model must own src/service_challenge_task.rs');
  const serviceChallengeExport = serviceModelLib.match(
    /\bpub\s+use\s+service_challenge_task\s*::\s*\{([\s\S]*?)\}\s*;/,
  );
  requireCondition(/\bmod\s+service_challenge_task\s*;/.test(serviceModelLib),
    'service-model lib must declare the Service challenge module');
  requireCondition(Boolean(serviceChallengeExport),
    'service-model lib must export the Service challenge interface');
  for (const name of SERVICE_CHALLENGE_DEFINITIONS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...serviceChallengeSource.matchAll(definition)].length === 1,
      `service-model Service challenge module must own exactly one definition: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate Service challenge definition: ${name}`,
    );
    if (SERVICE_CHALLENGE_EXPORTED_DEFINITIONS.includes(name)) {
      requireCondition(
        Boolean(serviceChallengeExport?.[1].match(new RegExp(`\\b${name}\\b`))),
        `service-model lib must export Service challenge definition: ${name}`,
      );
    }
  }
  for (const name of SERVICE_CHALLENGE_DECISIONS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceChallenge),
      `service-model must own Service challenge decision: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(`\\bfn\\s+${name}\\b`).test(source)),
      `CLI must not duplicate Service challenge decision: ${name}`,
    );
    requireCondition(
      Boolean(serviceChallengeExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export Service challenge decision: ${name}`,
    );
  }
  for (const name of SERVICE_CHALLENGE_CONSTANTS) {
    requireCondition(
      new RegExp(`\\bpub\\s+const\\s+${name}\\b`).test(serviceChallenge),
      `service-model must own Service challenge constant: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) =>
        new RegExp(`\\b(?:pub\\s*)?(?:\\(?crate\\)?\\s*)?const\\s+${name}\\b`).test(source)),
      `CLI must not duplicate Service challenge constant: ${name}`,
    );
    requireCondition(
      Boolean(serviceChallengeExport?.[1].match(new RegExp(`\\b${name}\\b`))),
      `service-model lib must export Service challenge constant: ${name}`,
    );
  }
  requireCondition(
    /challenge_tasks\s*:\s*BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*ServiceChallengeTaskRecord\s*>/
      .test(canonicalServiceState),
    'service-model ServiceState must use the canonical Service challenge record',
  );
  const serviceChallengeField = canonicalServiceState.match(
    /((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?challenge_tasks\s*:/,
  );
  requireCondition(
    Boolean(serviceChallengeField?.[1].match(
      /#\s*\[\s*serde\s*\([^\]]*skip_serializing_if\s*=\s*["']agent_browser_service_model\s*::\s*challenge_task_map_is_empty["'][^\]]*\)\s*\]/,
    )),
    'service-model ServiceState must use the canonical Service challenge empty-map decision',
  );
  for (const name of SERVICE_STATE_CHALLENGE_METHODS) {
    requireCondition(
      new RegExp(`\\bpub\\s+fn\\s+${name}\\b`).test(serviceStateCode),
      `service-model ServiceState must own Service challenge aggregate method: ${name}`,
    );
  }
  requireCondition(
    !cliProductionSources.some((source) => /\.\s*challenge_tasks\b/.test(source)),
    'CLI production code must not access ServiceState.challenge_tasks directly',
  );
  for (const name of ABANDONED_RETIREMENT_RECORDS) {
    const definition = new RegExp(`\\b(?:struct|enum|type)\\s+${name}\\b`, 'g');
    requireCondition(
      [...retirement.matchAll(definition)].length === 1,
      `service-model abandoned retirement module must own exactly one definition: ${name}`,
    );
    requireCondition(
      sources.reduce((count, source) => count + [...withoutCommentsAndStrings(source).matchAll(definition)].length, 0) === 1,
      `service-model must not duplicate abandoned retirement record: ${name}`,
    );
    requireCondition(
      !cliSources.some((source) => new RegExp(definition.source).test(source)),
      `CLI must not duplicate abandoned retirement record: ${name}`,
    );
  }
  requireCondition(
    /\bpub\s+const\s+ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1\b/.test(retirement),
    'service-model must own the abandoned retirement plan schema constant',
  );
  for (const adapter of [
    'RetirementObservation', 'RetirementReservation', 'ProcessSample',
    'ServiceState', 'ServiceStateRepository', 'AbandonedBrowserRetirementRuntime',
    'from_environment', 'resource_retirement_policy_from_environment',
    'plan_abandoned_browser_retirement', 'reserve_abandoned_browser_retirement',
    'finalize_abandoned_browser_retirement',
  ]) {
    requireCondition(!new RegExp(`\\b${adapter}\\b`).test(retirement),
      `abandoned retirement model must not absorb CLI adapter: ${adapter}`);
  }
  requireCondition(!/\bstd\s*::\s*env\b/.test(retirement),
    'abandoned retirement model must not read the environment');

  return failures;
}

function main() {
  const failures = check();
  if (failures.length) {
    console.error('Service-model crate architecture contract failed:');
    for (const failure of failures) console.error(`  - ${failure}`);
    process.exitCode = 1;
    return;
  }
  console.log('Service-model crate architecture contract passed');
}

if (import.meta.url === `file://${process.argv[1]}`) main();

export { check };
