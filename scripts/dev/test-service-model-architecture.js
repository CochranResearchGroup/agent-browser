#!/usr/bin/env node

import { deepEqual, doesNotThrow, ok, throws } from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { check } from './check-service-model-architecture.js';

const retirementRecords = [
  'AbandonedBrowserRetirementPlan', 'AbandonedBrowserRetirementTransaction',
  'AbandonedBrowserRetirementReceipt', 'RetirementTerminalProjection',
  'RetirementExitEvidence', 'RetirementExitFailure', 'RetirementRecourse',
  'ResourceRetirementPolicy',
];
const validRetirement = retirementRecords.map((name) => `pub struct ${name};`).join('\n')
  + '\npub const ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1: &str = "schema";\n';
const crashRegenerationRecords = [
  'CrashRegenerationPhase', 'CrashRegenerationState',
  'CrashRegenerationStableIdentities', 'CrashRegenerationEvidence',
  'CrashRegenerationTransaction', 'CrashRegenerationStatus',
  'CrashRegenerationRequest', 'CrashRegenerationOperation',
  'CrashRegenerationPhaseReceipt',
];
const validCrashRegeneration = crashRegenerationRecords
  .map((name) => `pub struct ${name};`).join('\n')
  + '\npub const CRASH_REGENERATION_STATUS_SCHEMA_VERSION: &str = "schema";\n';
const validCrashExport = `mod crash_regeneration;
pub use crash_regeneration::{
  ${crashRegenerationRecords.join(',\n  ')},
  CRASH_REGENERATION_STATUS_SCHEMA_VERSION,
};
`;
const validCrashStateUse = `
pub struct ServiceState {
  crash_regeneration_transactions:
    BTreeMap<String, agent_browser_service_model::CrashRegenerationTransaction>,
  browser_capability_registry:
    agent_browser_service_model::BrowserCapabilityRegistry,
  #[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]
  authentication_runs:
    BTreeMap<String, agent_browser_service_model::ServiceAuthenticationRunRecord>,
}
`;
const validCapabilityRegistry = `
pub struct BrowserCapabilityRegistry;
pub fn browser_profile_compatibility_matches() {}
`;
const validCapabilityExport = `mod browser_capability_registry;
pub use browser_capability_registry::{
  browser_profile_compatibility_matches,
  BrowserCapabilityRegistry,
};
`;
const serviceAuthenticationDefinitions = [
  'ServiceAuthenticationRunRecord', 'PendingAuthenticationEffect',
  'ServiceAuthenticationRunStartInput', 'PreparedServiceAuthenticationRunStart',
  'ServiceAuthenticationRunStartDecision', 'ServiceAuthenticationRunCompletion',
  'ServiceAuthenticationRunError', 'ServiceAuthenticationRunProjection',
];
const serviceAuthenticationDecisions = [
  'prepare_service_authentication_run_start',
  'complete_service_authentication_run_start', 'project_service_authentication_run',
  'require_live_authentication_run', 'reserve_authentication_effect',
  'complete_site_authentication_action', 'complete_credential_delivery_action',
  'complete_challenge_authentication_action', 'cancel_authentication_run',
  'authentication_run_map_is_empty',
];
const validServiceAuthentication = `${serviceAuthenticationDefinitions
  .map((name) => `pub struct ${name};`).join('\n')}
${serviceAuthenticationDecisions.map((name) => `pub fn ${name}() {}`).join('\n')}
pub const SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION: &str = "schema";
`;
const validServiceAuthenticationExport = `mod service_authentication_run;
pub use service_authentication_run::{
  ${serviceAuthenticationDefinitions.join(',\n  ')},
  ${serviceAuthenticationDecisions.join(',\n  ')},
  SERVICE_AUTHENTICATION_RUN_SCHEMA_VERSION,
};
`;

const authenticationControlRecords = [
  'AuthenticationRunBinding', 'AuthenticationRunState',
  'AuthenticationChallengeChannel', 'AuthenticationActionKind', 'SiteLoginState',
  'PasswordPersistencePolicy', 'SiteLoginObservationReceipt',
  'SiteLoginActionReceipt', 'SiteLoginActionContext', 'ProviderWatchReceipt',
  'SameProfileNewTabProof', 'AuthenticationActionReceipt',
  'AuthenticationActionContext', 'AuthenticationActionFailure',
  'AuthenticationVerifierReceipt', 'AuthenticationVerificationContext',
  'AuthenticationVerifierFailure', 'AuthenticationTransitionReceipt',
  'ActiveAuthenticationChallenge', 'AuthenticationRun', 'AuthenticationRunError',
];
const authenticationControlTraits = [
  'ResponseOnlySiteLoginAction', 'ResponseOnlyAuthenticationAction',
  'AuthenticationVerifier',
];
const validAuthenticationControl = `${authenticationControlRecords
  .map((name) => `pub struct ${name};`).join('\n')}
${authenticationControlTraits.map((name) => `pub trait ${name} {}`).join('\n')}
pub const AUTHENTICATION_RUN_SCHEMA_VERSION: &str = "schema";
`;
const validAuthenticationFacade = 'pub(crate) use agent_browser_authentication_control::*;\n';

function fixture({ manifest = '', source = '', workspace = true, retirement = validRetirement,
  crash = validCrashRegeneration, capability = validCapabilityRegistry, cli = '',
  serviceModel = validCrashStateUse, authenticationManifest = validAuthenticationManifest,
  authentication = validAuthenticationControl,
  authenticationFacade = validAuthenticationFacade,
  serviceAuthentication = validServiceAuthentication } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-architecture-'));
  mkdirSync(join(root, 'crates/agent-browser-service-model/src'), { recursive: true });
  mkdirSync(join(root, 'crates/agent-browser-authentication-control/src'), { recursive: true });
  writeFileSync(join(root, 'Cargo.toml'), workspace
    ? '[workspace]\nmembers = ["crates/agent-browser-service-model", "crates/agent-browser-authentication-control"]\n'
    : 'workspace = false\n');
  writeFileSync(join(root, 'crates/agent-browser-service-model/Cargo.toml'), manifest);
  writeFileSync(join(root, 'crates/agent-browser-authentication-control/Cargo.toml'),
    authenticationManifest);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/lib.rs'), source);
  writeFileSync(join(root, 'crates/agent-browser-authentication-control/src/lib.rs'),
    authentication);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'), retirement);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/crash_regeneration.rs'), crash);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/browser_capability_registry.rs'), capability);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/service_authentication_run.rs'),
    serviceAuthentication);
  mkdirSync(join(root, 'cli/src/native'), { recursive: true });
  writeFileSync(join(root, 'cli/src/native/retirement.rs'), cli);
  writeFileSync(join(root, 'cli/src/native/authentication_run.rs'), authenticationFacade);
  writeFileSync(join(root, 'cli/src/native/service_model.rs'), serviceModel);
  return root;
}

const validManifest = '[package]\nname = "agent-browser-service-model"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validAuthenticationManifest = '[package]\nname = "agent-browser-authentication-control"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validSource = `${validCrashExport}
${validCapabilityExport}
${validServiceAuthenticationExport}
// std::fs and provider are allowed in prose.
pub struct BrowserProfile { pub id: String }
pub fn profile(id: String) -> BrowserProfile { BrowserProfile { id } }
`;

const missing = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-missing-'));
try {
  deepEqual(check(missing).some((failure) => failure.includes('Cargo manifest must exist')), true);
} finally {
  rmSync(missing, { recursive: true, force: true });
}

const missingCrashModule = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  rmSync(join(missingCrashModule, 'crates/agent-browser-service-model/src/crash_regeneration.rs'));
  ok(check(missingCrashModule).some((failure) => failure.includes('crash_regeneration.rs')),
    'missing crash-regeneration module was accepted');
} finally {
  rmSync(missingCrashModule, { recursive: true, force: true });
}

const missingCapabilityModule = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  rmSync(join(missingCapabilityModule,
    'crates/agent-browser-service-model/src/browser_capability_registry.rs'));
  ok(check(missingCapabilityModule).some((failure) =>
    failure.includes('browser_capability_registry.rs')),
  'missing capability-registry module was accepted');
} finally {
  rmSync(missingCapabilityModule, { recursive: true, force: true });
}

const clean = fixture({ manifest: validManifest, source: validSource,
  authenticationManifest: validAuthenticationManifest });
try {
  doesNotThrow(() => ok(check(clean).length === 0, check(clean).join('\n')));
} finally {
  rmSync(clean, { recursive: true, force: true });
}

const missingAuthenticationModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingAuthenticationModule,
    'crates/agent-browser-authentication-control/src/lib.rs'));
  ok(check(missingAuthenticationModule).some((failure) =>
    failure.includes('authentication-control must own src/lib.rs')),
  'missing authentication-control module was accepted');
} finally {
  rmSync(missingAuthenticationModule, { recursive: true, force: true });
}

const missingServiceAuthenticationModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingServiceAuthenticationModule,
    'crates/agent-browser-service-model/src/service_authentication_run.rs'));
  ok(check(missingServiceAuthenticationModule).some((failure) =>
    failure.includes('service_authentication_run.rs')),
  'missing Service authentication module was accepted');
} finally {
  rmSync(missingServiceAuthenticationModule, { recursive: true, force: true });
}

const duplicateServiceAuthenticationDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct ServiceAuthenticationRunRecord;\n',
});
try {
  ok(check(duplicateServiceAuthenticationDefinition).some((failure) =>
    failure.includes('ServiceAuthenticationRunRecord')),
  'duplicate CLI Service authentication definition was accepted');
} finally {
  rmSync(duplicateServiceAuthenticationDefinition, { recursive: true, force: true });
}

const missingServiceAuthenticationExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validServiceAuthenticationExport, ''),
});
try {
  ok(check(missingServiceAuthenticationExport).some((failure) =>
    failure.includes('export the Service authentication')),
  'missing Service authentication export was accepted');
} finally {
  rmSync(missingServiceAuthenticationExport, { recursive: true, force: true });
}

const indirectServiceAuthenticationStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceModel: validCrashStateUse.replace(
    'agent_browser_service_model::ServiceAuthenticationRunRecord',
    'super::service_authentication_run::ServiceAuthenticationRunRecord',
  ),
});
try {
  ok(check(indirectServiceAuthenticationStateType).some((failure) =>
    failure.includes('canonical service-model Service authentication record')),
  'indirect Service authentication aggregate type was accepted');
} finally {
  rmSync(indirectServiceAuthenticationStateType, { recursive: true, force: true });
}

const duplicateServiceAuthenticationDecision = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) fn reserve_authentication_effect() {}\n',
});
try {
  ok(check(duplicateServiceAuthenticationDecision).some((failure) =>
    failure.includes('reserve_authentication_effect')),
  'duplicate CLI Service authentication decision was accepted');
} finally {
  rmSync(duplicateServiceAuthenticationDecision, { recursive: true, force: true });
}

const commentedServiceAuthenticationSkipPredicate = fixture({
  manifest: validManifest,
  source: validSource,
  serviceModel: validCrashStateUse.replace(
    '#[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]',
    '// #[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]',
  ),
});
try {
  ok(check(commentedServiceAuthenticationSkipPredicate).some((failure) =>
    failure.includes('empty-map decision')),
  'commented Service authentication empty-map predicate was accepted');
} finally {
  rmSync(commentedServiceAuthenticationSkipPredicate, { recursive: true, force: true });
}

const predicateOnEarlierFieldOnly = fixture({
  manifest: validManifest,
  source: validSource,
  serviceModel: validCrashStateUse.replace(
    '#[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]\n  authentication_runs:',
    `#[serde(skip_serializing_if = "agent_browser_service_model::authentication_run_map_is_empty")]
  unrelated_runs: BTreeMap<String, String>,
  #[serde(default)]
  authentication_runs:`,
  ),
});
try {
  ok(check(predicateOnEarlierFieldOnly).some((failure) =>
    failure.includes('empty-map decision')),
  'Service authentication guard borrowed a skip predicate from an earlier field');
} finally {
  rmSync(predicateOnEarlierFieldOnly, { recursive: true, force: true });
}

const duplicateAuthenticationDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: `${validAuthenticationFacade}\npub(crate) struct AuthenticationRun;\n`,
});
try {
  ok(check(duplicateAuthenticationDefinition).some((failure) =>
    failure.includes('AuthenticationRun')),
  'duplicate CLI authentication-control definition was accepted');
} finally {
  rmSync(duplicateAuthenticationDefinition, { recursive: true, force: true });
}

const missingAuthenticationFacade = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: '',
});
try {
  ok(check(missingAuthenticationFacade).some((failure) =>
    failure.includes('only the authentication-control compatibility re-export')),
  'missing CLI authentication-control compatibility facade was accepted');
} finally {
  rmSync(missingAuthenticationFacade, { recursive: true, force: true });
}

const forbiddenAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: `${validAuthenticationManifest}agent-browser-service-model = { path = "../agent-browser-service-model" }\n`,
});
try {
  ok(check(forbiddenAuthenticationDependency).some((failure) =>
    failure.includes('agent-browser-service-model')),
  'authentication-control upward Cargo dependency was accepted');
} finally {
  rmSync(forbiddenAuthenticationDependency, { recursive: true, force: true });
}

const unknownAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: `${validAuthenticationManifest}keyring = "3"\n`,
});
try {
  ok(check(unknownAuthenticationDependency).some((failure) =>
    failure.includes('provider-free allowlist')),
  'unknown authentication-control dependency was accepted');
} finally {
  rmSync(unknownAuthenticationDependency, { recursive: true, force: true });
}

const aliasedAuthenticationDependency = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationManifest: validAuthenticationManifest.replace(
    'serde = "1"',
    'serde = { package = "keyring", version = "3" }',
  ),
});
try {
  ok(check(aliasedAuthenticationDependency).some((failure) =>
    failure.includes('must not alias')),
  'aliased authentication-control dependency was accepted');
} finally {
  rmSync(aliasedAuthenticationDependency, { recursive: true, force: true });
}

for (const authenticationManifest of [
  `${validAuthenticationManifest}\n[dependencies.keyring]\nversion = "3"\n`,
  `${validAuthenticationManifest}\n[build-dependencies.codegen]\nversion = "1"\n`,
  `${validAuthenticationManifest}\n[target.'cfg(unix)'.dependencies.keyring]\nversion = "3"\n`,
  `${validAuthenticationManifest}\n[dependencies.serde]\npackage = "keyring"\nversion = "3"\n`,
]) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authenticationManifest,
  });
  try {
    ok(check(root).some((failure) =>
      failure.includes('provider-free allowlist') || failure.includes('must not alias')),
    `authentication-control dependency table bypass was accepted: ${authenticationManifest}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const authenticationFacadeWithLogic = fixture({
  manifest: validManifest,
  source: validSource,
  authenticationFacade: `${validAuthenticationFacade}\npub(crate) fn adapter_logic() {}\n`,
});
try {
  ok(check(authenticationFacadeWithLogic).some((failure) =>
    failure.includes('only the authentication-control compatibility re-export')),
  'CLI authentication-control facade accepted extra logic');
} finally {
  rmSync(authenticationFacadeWithLogic, { recursive: true, force: true });
}

const forbiddenAuthenticationImport = fixture({
  manifest: validManifest,
  source: validSource,
  authentication: `${validAuthenticationControl}\nuse std::process::Command;\n`,
});
try {
  ok(check(forbiddenAuthenticationImport).some((failure) =>
    failure.includes('std::process')),
  'authentication-control process import was accepted');
} finally {
  rmSync(forbiddenAuthenticationImport, { recursive: true, force: true });
}

for (const name of authenticationControlRecords) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authentication: validAuthenticationControl.replace(`pub struct ${name};`, ''),
  });
  try {
    ok(check(root).some((failure) => failure.includes(name)),
      `missing authentication-control definition was accepted: ${name}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

for (const name of authenticationControlTraits) {
  const root = fixture({
    manifest: validManifest,
    source: validSource,
    authentication: validAuthenticationControl.replace(`pub trait ${name} {}`, ''),
  });
  try {
    ok(check(root).some((failure) => failure.includes(name)),
      `missing authentication-control trait was accepted: ${name}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const missingAuthenticationSchema = fixture({
  manifest: validManifest,
  source: validSource,
  authentication: validAuthenticationControl.replace(
    'pub const AUTHENTICATION_RUN_SCHEMA_VERSION',
    'const OTHER_SCHEMA_VERSION',
  ),
});
try {
  ok(check(missingAuthenticationSchema).some((failure) =>
    failure.includes('authentication run schema constant')),
  'missing authentication-control schema constant was accepted');
} finally {
  rmSync(missingAuthenticationSchema, { recursive: true, force: true });
}

for (const manifest of [
  `${validManifest}agent-browser = { path = "../../cli" }\n`,
  `${validManifest}tokio = "1"\n`,
  `${validManifest}"tokio" = "1"\n`,
  `${validManifest}\n[dependencies."tokio"]\nversion = "1"\n`,
  validManifest.replace(
    'serde = "1"',
    '"serde" = { "package" = "tokio", version = "1" }',
  ),
  validManifest.replace('serde = "1"', 'serde = { package = "tokio", version = "1" }'),
]) {
  const root = fixture({ manifest, source: validSource });
  try {
    ok(check(root).length > 0, 'forbidden Cargo dependency was accepted');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

const forbiddenImport = fixture({
  manifest: validManifest,
  source: 'use std::fs;\npub struct Model;\n',
});
try {
  throws(() => ok(check(forbiddenImport).length === 0, 'forbidden Rust import was accepted'));
} finally {
  rmSync(forbiddenImport, { recursive: true, force: true });
}

const forbiddenCrashImport = fixture({
  manifest: validManifest,
  source: validSource,
  crash: `${validCrashRegeneration}\nuse crate::native::service_store::ServiceStateRepository;\n`,
});
try {
  ok(check(forbiddenCrashImport).some((failure) => failure.includes('crate::native')),
    'forbidden crash-regeneration upward import was accepted');
} finally {
  rmSync(forbiddenCrashImport, { recursive: true, force: true });
}

const duplicateCrashDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct CrashRegenerationTransaction;\n',
});
try {
  ok(check(duplicateCrashDefinition).some((failure) => failure.includes('CrashRegenerationTransaction')),
    'duplicate CLI crash-regeneration definition was accepted');
} finally {
  rmSync(duplicateCrashDefinition, { recursive: true, force: true });
}

const missingCrashExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validCrashExport, ''),
});
try {
  ok(check(missingCrashExport).some((failure) => failure.includes('export the crash regeneration')),
    'missing crash-regeneration public export was accepted');
} finally {
  rmSync(missingCrashExport, { recursive: true, force: true });
}

const indirectCrashStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceModel: validCrashStateUse.replace(
    'agent_browser_service_model::CrashRegenerationTransaction',
    'super::service_crash_regeneration::CrashRegenerationTransaction',
  ),
});
try {
  ok(check(indirectCrashStateType).some((failure) => failure.includes('canonical service-model')),
    'indirect CLI crash-regeneration transaction type was accepted');
} finally {
  rmSync(indirectCrashStateType, { recursive: true, force: true });
}

const duplicateCapabilityDefinition = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) struct BrowserCapabilityRegistry;\n',
});
try {
  ok(check(duplicateCapabilityDefinition).some((failure) =>
    failure.includes('BrowserCapabilityRegistry')),
  'duplicate CLI capability-registry definition was accepted');
} finally {
  rmSync(duplicateCapabilityDefinition, { recursive: true, force: true });
}

const missingCapabilityExport = fixture({
  manifest: validManifest,
  source: validSource.replace(validCapabilityExport, ''),
});
try {
  ok(check(missingCapabilityExport).some((failure) =>
    failure.includes('export the browser capability registry')),
  'missing capability-registry public export was accepted');
} finally {
  rmSync(missingCapabilityExport, { recursive: true, force: true });
}

const indirectCapabilityStateType = fixture({
  manifest: validManifest,
  source: validSource,
  serviceModel: validCrashStateUse.replace(
    'agent_browser_service_model::BrowserCapabilityRegistry',
    'BrowserCapabilityRegistry',
  ),
});
try {
  ok(check(indirectCapabilityStateType).some((failure) =>
    failure.includes('canonical service-model browser capability registry')),
  'indirect CLI capability-registry type was accepted');
} finally {
  rmSync(indirectCapabilityStateType, { recursive: true, force: true });
}

const duplicateCapabilityMatcher = fixture({
  manifest: validManifest,
  source: validSource,
  cli: 'pub(crate) fn browser_profile_compatibility_matches() {}\n',
});
try {
  ok(check(duplicateCapabilityMatcher).some((failure) =>
    failure.includes('browser_profile_compatibility_matches')),
  'duplicate CLI capability-registry matcher was accepted');
} finally {
  rmSync(duplicateCapabilityMatcher, { recursive: true, force: true });
}

for (const name of retirementRecords) {
  for (const mutation of [
    { retirement: validRetirement.replace(`pub struct ${name};`, '') },
    { cli: `pub(crate) struct ${name};` },
    { cli: `type ${name} = Other;` },
    { source: validSource + `\npub struct ${name};` },
  ]) {
    const root = fixture({ manifest: validManifest, source: validSource, ...mutation });
    try {
      ok(check(root).some((failure) => failure.includes(name)), `record ownership drift accepted: ${name}`);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }
}

const missingSchema = fixture({
  manifest: validManifest,
  source: validSource,
  retirement: validRetirement.replace('pub const ABANDONED_BROWSER_RETIREMENT_PLAN_SCHEMA_V1', 'const OTHER_SCHEMA'),
});
try {
  ok(check(missingSchema).some((failure) => failure.includes('schema constant')));
} finally {
  rmSync(missingSchema, { recursive: true, force: true });
}

for (const addition of [
  'use crate::native::service_model::ServiceState;',
  'pub fn from_environment() {}',
  'pub fn read_env() { std::env::var("CONFIG"); }',
  'pub struct RetirementObservation;',
  'pub enum RetirementReservation {}',
  'pub trait AbandonedBrowserRetirementRuntime {}',
]) {
  const root = fixture({ manifest: validManifest, source: validSource, retirement: validRetirement + addition });
  try {
    ok(check(root).length > 0, `adapter import accepted: ${addition}`);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

console.log('Service-model architecture guard fixture tests passed');
