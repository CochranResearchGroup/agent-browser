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

function fixture({ manifest = '', source = '', workspace = true, retirement = validRetirement, cli = '' } = {}) {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-service-model-architecture-'));
  mkdirSync(join(root, 'crates/agent-browser-service-model/src'), { recursive: true });
  writeFileSync(join(root, 'Cargo.toml'), workspace ? '[workspace]\nmembers = ["crates/agent-browser-service-model"]\n' : 'workspace = false\n');
  writeFileSync(join(root, 'crates/agent-browser-service-model/Cargo.toml'), manifest);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/lib.rs'), source);
  writeFileSync(join(root, 'crates/agent-browser-service-model/src/abandoned_browser_retirement.rs'), retirement);
  mkdirSync(join(root, 'cli/src/native'), { recursive: true });
  writeFileSync(join(root, 'cli/src/native/retirement.rs'), cli);
  return root;
}

const validManifest = '[package]\nname = "agent-browser-service-model"\nversion = "0.1.0"\n[dependencies]\nserde = "1"\n';
const validSource = `
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
