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
  const cliSources = rustFilesUnder(join(root, 'cli/src'))
    .map((path) => withoutCommentsAndStrings(readFileSync(path, 'utf8')));

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
  requireCondition(
    /BTreeMap\s*<\s*String\s*,\s*agent_browser_service_model\s*::\s*CrashRegenerationTransaction\s*>/
      .test(cliServiceModel),
    'CLI ServiceState must use the canonical service-model crash regeneration transaction',
  );
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
      .test(cliServiceModel),
    'CLI ServiceState must use the canonical service-model browser capability registry',
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
      .test(cliServiceModel),
    'CLI ServiceState must use the canonical service-model Service authentication record',
  );
  const serviceAuthenticationField = withoutComments(cliServiceModelSource).match(
    /((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?authentication_runs\s*:/,
  );
  requireCondition(
    Boolean(serviceAuthenticationField?.[1].match(
      /#\s*\[\s*serde\s*\([^\]]*skip_serializing_if\s*=\s*["']agent_browser_service_model\s*::\s*authentication_run_map_is_empty["'][^\]]*\)\s*\]/,
    )),
    'CLI ServiceState must use the canonical Service authentication empty-map decision',
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
