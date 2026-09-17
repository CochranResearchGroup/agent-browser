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

const FORBIDDEN_DEPENDENCIES = [
  'agent-browser',
  'agent-browser-cdp',
  'agent-browser-challenge-control',
  'agent-browser-desktop-services',
  'tokio',
  'reqwest',
  'image',
  'rust-embed',
  'axum',
  'hyper',
];

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

function importedPaths(source) {
  const clean = withoutCommentsAndStrings(source);
  const paths = [];
  for (const match of clean.matchAll(/\b(?:pub\s+)?use\s+([^;]+);/g)) paths.push(match[1]);
  for (const match of clean.matchAll(/\b(?:pub\s+)?(?:mod|extern\s+crate)\s+([A-Za-z0-9_]+)/g)) {
    paths.push(match[1]);
  }
  return paths;
}

function cargoDependencyDeclarations(manifest) {
  let section = '';
  let tableDeclaration = null;
  const declarations = [];
  for (const rawLine of manifest.split('\n')) {
    const line = rawLine.replace(/#.*/, '').trim();
    const header = line.match(/^\[([^\]]+)\]$/);
    if (header) {
      section = header[1];
      tableDeclaration = null;
      const table = section.match(
        /^(.*(?:^|\.)(?:dev-|build-)?dependencies)\.([A-Za-z0-9_-]+)$/,
      );
      if (table) {
        tableDeclaration = {
          section: table[1], name: table[2], value: '',
        };
        declarations.push(tableDeclaration);
      }
      continue;
    }
    if (tableDeclaration) {
      if (/^package\s*=/.test(line)) tableDeclaration.value += ` ${line}`;
      continue;
    }
    if (!/(?:^|\.)(?:dev-|build-)?dependencies$/.test(section)) continue;
    const declaration = line.match(/^([A-Za-z0-9_-]+)\s*=\s*(.+)$/);
    if (declaration) {
      declarations.push({ section, name: declaration[1], value: declaration[2] });
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

  const dependencyLines = manifest
    .split('\n')
    .map((line) => line.replace(/#.*/, ''))
    .filter((line) => /^\s*[A-Za-z0-9_-]+\s*=/.test(line));
  for (const dependency of FORBIDDEN_DEPENDENCIES) {
    requireCondition(
      !dependencyLines.some((line) => new RegExp(`^\\s*${dependency.replaceAll('-', '[\\-]')}\\s*=`).test(line)),
      `service-model crate must not depend on effect/runtime/provider crate: ${dependency}`,
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
      !/\bpackage\s*=/.test(dependency.value),
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
  const cliServiceModel = withoutCommentsAndStrings(read(root,
    'cli/src/native/service_model.rs'));
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
