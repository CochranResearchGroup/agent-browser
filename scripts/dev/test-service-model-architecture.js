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

function fixture({ manifest = '', source = '', workspace = true, retirement = validRetirement,
  crash = validCrashRegeneration, capability = validCapabilityRegistry, cli = '',
  serviceModel = validCrashStateUse } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-architecture-'));
  mkdirSync(join(root, 'crates/agent-browser-service-model/src'), { recursive: true });
  writeFileSync(join(root, 'Cargo.toml'), workspace ? '[workspace]\nmembers = ["crates/agent-browser-service-model"]\n' : 'workspace = false\n');
  writeFileSync(join(root, 'crates/agent-browser-service-model/Cargo.toml'), manifest);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/lib.rs'), source);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'), retirement);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/crash_regeneration.rs'), crash);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/browser_capability_registry.rs'), capability);
  mkdirSync(join(root, 'cli/src/native'), { recursive: true });
  writeFileSync(join(root, 'cli/src/native/retirement.rs'), cli);
  writeFileSync(join(root, 'cli/src/native/service_model.rs'), serviceModel);
  return root;
}

const validManifest = '[package]\nname = "agent-browser-service-model"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validSource = `${validCrashExport}
${validCapabilityExport}
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

const missingCrashModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingCrashModule, 'crates/agent-browser-service-model/src/crash_regeneration.rs'));
  ok(check(missingCrashModule).some((failure) => failure.includes('crash_regeneration.rs')),
    'missing crash-regeneration module was accepted');
} finally {
  rmSync(missingCrashModule, { recursive: true, force: true });
}

const missingCapabilityModule = fixture({ manifest: validManifest, source: validSource });
try {
  rmSync(join(missingCapabilityModule,
    'crates/agent-browser-service-model/src/browser_capability_registry.rs'));
  ok(check(missingCapabilityModule).some((failure) =>
    failure.includes('browser_capability_registry.rs')),
  'missing capability-registry module was accepted');
} finally {
  rmSync(missingCapabilityModule, { recursive: true, force: true });
}

const clean = fixture({ manifest: validManifest, source: validSource });
try {
  doesNotThrow(() => ok(check(clean).length === 0, check(clean).join('\n')));
} finally {
  rmSync(clean, { recursive: true, force: true });
}

for (const manifest of [
  `${validManifest}agent-browser = { path = "../../cli" }\n`,
  `${validManifest}tokio = "1"\n`,
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
