#!/usr/bin/env node

import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';

const repoRoot = resolve(import.meta.dirname, '..');

function rustFilesUnder(root) {
  const files = [];
  const visit = (directory) => {
    if (!existsSync(directory)) return;
    for (const entry of readdirSync(directory)) {
      const candidate = join(directory, entry);
      if (statSync(candidate).isDirectory()) visit(candidate);
      else if (candidate.endsWith('.rs')) files.push(candidate);
    }
  };
  visit(root);
  return files;
}

function read(path, root = repoRoot) {
  const absolute = join(root, path);
  return existsSync(absolute) ? readFileSync(absolute, 'utf8') : '';
}

function check(root) {
  const failures = [];
  const requireCondition = (condition, message) => { if (!condition) failures.push(message); };
  const workspace = read('Cargo.toml', root);
  const cliManifest = read('cli/Cargo.toml', root);
  const crateManifest = read('crates/agent-browser-lease-authority/Cargo.toml', root);
  const runtimeOwnerSource = read('crates/agent-browser-lease-authority/src/runtime_owner.rs', root);
  const cliRuntimeOwnerSource = read('cli/src/runtime_owner_transfer.rs', root);
  const cliRuntimeAdoptionSource = read('cli/src/runtime_adoption.rs', root);
  const focusedWorkflow = read('.github/workflows/lease-authority.yml', root) ||
    read('.github/workflows/lease-authority.yml.disabled', root);
  const cliRust = rustFilesUnder(join(root, 'cli', 'src'));
  const cliSources = cliRust.map((file) => readFileSync(file, 'utf8'));
  const registryFields = runtimeOwnerSource.match(/pub\s+struct\s+RuntimeOwnerRegistry\s*\{([\s\S]*?)\n\}/)?.[1] || '';
  requireCondition(
    registryFields.length > 0 &&
      !/\bpub(?:\([^)]*\))?\s+(?:revision|owners|principal_bindings|lifecycle_records)\s*:/.test(registryFields),
    'runtime-owner registry fields must remain private',
  );
  requireCondition(
    !/\bpub(?:\([^)]*\))?\s+fn\s+(?:revision|owners|principal_bindings|lifecycle_records)_mut\b/.test(runtimeOwnerSource) &&
      !/\bpub(?:\([^)]*\))?\s+fn\s+\w+[^{};]*->[^{};]*&(?:'\w+\s+)?mut\b/.test(runtimeOwnerSource),
    'runtime-owner registry must not expose mutable getters',
  );
  for (let index = 0; index < cliRust.length; index += 1) {
    const source = cliSources[index];
    const relative = cliRust[index].slice(root.length + 1);
    // This transport-local Registry owns unrelated primary connections.
    const unrelatedPrimaryRegistry = relative === 'cli/src/native/stream/guacamole_primary_registry.rs';
    // Legacy wire-shape fixtures intentionally inspect their own local type.
    let fieldSource = source.replace(/\blegacy_(?:owner|state)_registry\s*\.\s*(?:revision|owners)\b/g, 'legacy_wire_field');
    // Service Model owns this typed persistence projection. Its public field is
    // not a direct view into RuntimeOwnerRegistry and remains outside this
    // Lease Authority field-access contract.
    if (relative === 'cli/src/native/service_store.rs') {
      fieldSource = fieldSource.replace(
        /\bpersistence\s*\.\s*lifecycle_records\b/g,
        'service_model_persistence_lifecycle_rows',
      );
    }
    if (unrelatedPrimaryRegistry) fieldSource = fieldSource.replace(/\bself\s*\.\s*owners\b/g, 'primary_connection_rows');
    requireCondition(
      !/\.\s*(?:owners|principal_bindings|lifecycle_records)\b(?!\s*\()/.test(fieldSource),
      `CLI must use immutable runtime-owner map accessors: ${relative}`,
    );
    requireCondition(
      !/\bruntime_owner_registry\s*\.\s*revision\b(?!\s*\()/.test(source) &&
        (!['cli/src/runtime_owner_transfer.rs', 'cli/src/native/runtime_lifecycle.rs', 'cli/src/native/service_store.rs', 'cli/src/install.rs'].includes(relative) ||
          !/\b(?:registry|owner_sidecar)\s*\.\s*revision\b(?!\s*\()/.test(source)),
      `CLI must use the runtime-owner revision accessor: ${relative}`,
    );
    requireCondition(
      !/\bRuntimeOwnerRegistry\s*\{/.test(source.replace(/->\s*(?:[\w]+::)*RuntimeOwnerRegistry\s*\{/g, '-> RegistryReturn {')),
      `CLI must not construct runtime-owner registry literals: ${relative}`,
    );
  }
  requireCondition(workspace.includes('crates/agent-browser-lease-authority'), 'root Cargo workspace must include agent-browser-lease-authority');
  requireCondition(crateManifest.includes('name = "agent-browser-lease-authority"'), 'agent-browser-lease-authority package manifest must exist');
  requireCondition(!cliManifest.includes('agent-browser-lease-authority'), 'default CLI must not depend on the quarantined Lease-authority crate');
  requireCondition(existsSync(join(root, 'crates/agent-browser-lease-authority/src/lib.rs')), 'Lease-authority crate must own src/lib.rs');
  requireCondition(runtimeOwnerSource.length > 0, 'Lease-authority crate must own the runtime-owner kernel');
  requireCondition(!existsSync(join(root, 'cli/src/native/service_lease_authority.rs')), 'legacy native::service_lease_authority implementation must be absent');
  requireCondition(!existsSync(join(root, 'cli/src/native/service_lease_authority')), 'legacy native::service_lease_authority directory must be absent');
  requireCondition(!read('cli/src/native/mod.rs', root).match(/(?:pub\s+)?(?:crate\s+)?mod\s+service_lease_authority\s*;/), 'native module must not retain a service_lease_authority facade');
  const focusedCargoCommand = ['cargo', 'test --profile ci -p agent-browser-lease-authority'].join(' ');
  requireCondition(
    focusedWorkflow.includes(focusedCargoCommand) &&
      focusedWorkflow.includes('fail-fast: false') &&
      focusedWorkflow.includes('x86_64-unknown-linux-gnu') &&
      focusedWorkflow.includes('aarch64-apple-darwin') &&
      focusedWorkflow.includes('x86_64-apple-darwin') &&
      focusedWorkflow.includes('x86_64-pc-windows-msvc'),
    'Lease-authority crate must retain an independent non-fail-fast Linux, macOS, and Windows CI gate',
  );
  requireCondition(!cliSources.some((source) => /(?:crate::native::|super(?:::\s*super)*::)service_lease_authority\b/.test(source)), 'CLI source must not import the removed native service_lease_authority owner');
  requireCondition(!cliSources.some((source) => /\bagent_browser_lease_authority\b/.test(source)), 'default CLI source must not import the quarantined Lease-authority crate');
  requireCondition(!/\bagent-browser\s*=/.test(crateManifest), 'Lease-authority crate must not depend back on the agent-browser package');
  const forbiddenDependencies = ['agent-browser-cdp', 'tokio', 'reqwest', 'image'];
  requireCondition(
    !forbiddenDependencies.some((dependency) => new RegExp(`^\\s*${dependency}\\s*=`, 'm').test(crateManifest)),
    'Lease-authority crate must not depend on CDP, browser, async-runtime, HTTP, image, dashboard, or generated-client packages',
  );
  const crateSources = rustFilesUnder(join(root, 'crates/agent-browser-lease-authority/src')).map((file) => readFileSync(file, 'utf8'));
  requireCondition(!crateSources.some((source) => /(?:crate::native::|\bagent_browser::native::|\bagent_browser_cli::native::)/.test(source)), 'Lease-authority crate must not import CLI/native paths upward');
  requireCondition(
    !/(?:ServiceStateRepository|runtime_adoption|service_model|std::process|std::fs)/.test(runtimeOwnerSource),
    'runtime-owner kernel must not import repositories, CLI adoption, Service State, process, or filesystem adapters',
  );
  requireCondition(
    !/\b(?:struct|enum)\s+(?:ProfileOwner|RuntimeOwnerRegistry|OwnerTransferRequest|RuntimeLifecycleRecord)\b/.test(cliRuntimeOwnerSource),
    'CLI runtime-owner module must not retain canonical runtime-owner record definitions',
  );
  requireCondition(
    !/\benum\s+BrowserAdoptionMode\b/.test(cliRuntimeAdoptionSource),
    'CLI runtime-adoption module must not retain the canonical BrowserAdoptionMode definition',
  );
  const joinedCrateSources = crateSources.join('\n');
  requireCondition(
    !/\bpub(?:\([^)]*\))?\s+(?:struct|enum)\s+LeaseAuthority(?:SigningKey|VerificationKeyring|SigningKeyFile)\b/.test(joinedCrateSources),
    'Lease-authority signing keys and verification keyrings must remain private',
  );
  requireCondition(
    !/\bpub(?:\([^)]*\))?\s+fn\s+(?:load_or_create_lease_authority_signing_key|load_selected_lease_authority_signing_key|from_private_bytes(?:_at_epoch)?)\b/.test(joinedCrateSources),
    'Lease-authority secret loaders and private-key constructors must remain private',
  );
  requireCondition(
    !/\bpub(?:\([^)]*\))?\s+(?:active_claims|terminal_records|signed_proof|private_key)\s*:/.test(joinedCrateSources),
    'Lease-authority mutable maps, signed proofs, and private key fields must remain private',
  );
  return failures;
}

function currentTreeCheck() {
  const failures = check(repoRoot);
  if (failures.length) {
    console.error('Lease-authority crate architecture contract failed:');
    for (const failure of failures) console.error(`  - ${failure}`);
    process.exit(1);
  }
  console.log('Lease-authority crate architecture contract passed');
}

function selfTest() {
  const root = mkdtempSync(join(tmpdir(), 'agent-browser-lease-authority-contract-'));
  try {
    mkdirSync(join(root, 'cli/src/native'), { recursive: true });
    mkdirSync(join(root, 'crates/agent-browser-lease-authority/src'), { recursive: true });
    mkdirSync(join(root, '.github/workflows'), { recursive: true });
    writeFileSync(join(root, 'Cargo.toml'), '[workspace]\nmembers = ["cli", "crates/agent-browser-lease-authority"]\n');
    writeFileSync(join(root, 'cli/Cargo.toml'), '[dependencies]\n');
    writeFileSync(join(root, 'crates/agent-browser-lease-authority/Cargo.toml'), '[package]\nname = "agent-browser-lease-authority"\n');
    writeFileSync(join(root, 'crates/agent-browser-lease-authority/src/lib.rs'), 'pub struct Authority;\n');
    writeFileSync(join(root, 'crates/agent-browser-lease-authority/src/runtime_owner.rs'), 'pub struct RuntimeOwnerRegistry {\n  revision: u64,\n  owners: BTreeMap<String, ProfileOwner>,\n  principal_bindings: BTreeMap<String, RuntimeOwnerPrincipalBinding>,\n  lifecycle_records: BTreeMap<String, RuntimeLifecycleRecord>,\n}\n');
    writeFileSync(join(root, '.github/workflows/lease-authority.yml'), 'strategy:\n  fail-fast: false\nrun: cargo test --profile ci -p agent-browser-lease-authority\ntargets: x86_64-unknown-linux-gnu aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc\n');
    writeFileSync(join(root, 'cli/src/native/mod.rs'), 'pub(crate) mod adapter;\n');
    writeFileSync(join(root, 'cli/src/native/adapter.rs'), 'pub fn adapter() {}\n');
    writeFileSync(join(root, 'cli/src/native/service_store.rs'), 'fn project(persistence: &RuntimeOwnerPersistenceParts) { let _ = &persistence.lifecycle_records; }\n');
    writeFileSync(join(root, 'cli/src/runtime_owner_transfer.rs'), 'pub fn removed_runtime_owner_surface() {}\n');
    writeFileSync(join(root, 'cli/src/runtime_adoption.rs'), 'pub fn removed_runtime_adoption_surface() {}\n');
    if (check(root).length) throw new Error('valid extracted fixture was rejected');
    const cases = [
      ['missing crate', 'rm-crate'], ['upward import', 'upward-import'],
      ['retained implementation', 'retained-implementation'], ['retained directory', 'retained-directory'],
      ['retained facade', 'retained-facade'], ['forbidden CLI dependency', 'forbidden-cli-dependency'],
      ['reverse dependency', 'reverse-dependency'], ['forbidden CLI import', 'forbidden-cli-import'],
      ['upward crate import', 'upward-crate-import'],
      ['forbidden dependency', 'forbidden-dependency'], ['public signing type', 'public-signing-type'],
      ['public secret loader', 'public-secret-loader'], ['public claim map', 'public-claim-map'],
      ['runtime owner upward import', 'runtime-owner-upward-import'],
      ['duplicate runtime owner', 'duplicate-runtime-owner'],
      ['duplicate adoption mode', 'duplicate-adoption-mode'],
      ['missing focused workflow', 'missing-focused-workflow'],
      ['public registry revision', 'public-registry-revision'],
      ['public registry owners', 'public-registry-owners'],
      ['public registry principal bindings', 'public-registry-principal-bindings'],
      ['public registry lifecycle records', 'public-registry-lifecycle-records'],
      ['mutable registry getter', 'mutable-registry-getter'],
      ['renamed mutable registry getter', 'renamed-mutable-registry-getter'],
      ['direct CLI registry map access', 'direct-registry-map'],
      ['direct service-store registry map access', 'direct-service-store-registry-map'],
      ['direct CLI registry revision access', 'direct-registry-revision'],
      ['aliased CLI registry revision access', 'aliased-registry-revision'],
      ['CLI registry literal', 'cli-registry-literal'],
    ];
    for (const [label, mutation] of cases) {
      const mutated = mkdtempSync(join(tmpdir(), `agent-browser-lease-authority-${mutation}-`));
      try {
        mkdirSync(join(mutated, 'cli/src/native'), { recursive: true });
        mkdirSync(join(mutated, 'crates/agent-browser-lease-authority/src'), { recursive: true });
        for (const path of ['Cargo.toml', 'cli/Cargo.toml', 'crates/agent-browser-lease-authority/Cargo.toml', 'crates/agent-browser-lease-authority/src/lib.rs', 'crates/agent-browser-lease-authority/src/runtime_owner.rs', '.github/workflows/lease-authority.yml', 'cli/src/native/mod.rs', 'cli/src/native/adapter.rs', 'cli/src/native/service_store.rs', 'cli/src/runtime_owner_transfer.rs', 'cli/src/runtime_adoption.rs']) {
          mkdirSync(join(mutated, path, '..'), { recursive: true });
          writeFileSync(join(mutated, path), read(path, root));
        }
        if (mutation === 'rm-crate') rmSync(join(mutated, 'crates/agent-browser-lease-authority'), { recursive: true, force: true });
        if (mutation === 'upward-import') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use crate::native::service_lease_authority::Authority;\n');
        if (mutation === 'retained-implementation') writeFileSync(join(mutated, 'cli/src/native/service_lease_authority.rs'), 'pub struct Authority;\n');
        if (mutation === 'retained-directory') mkdirSync(join(mutated, 'cli/src/native/service_lease_authority'), { recursive: true });
        if (mutation === 'retained-facade') writeFileSync(join(mutated, 'cli/src/native/mod.rs'), 'mod service_lease_authority;\n');
        if (mutation === 'forbidden-cli-dependency') writeFileSync(join(mutated, 'cli/Cargo.toml'), '[dependencies]\nagent-browser-lease-authority = { path = "../crates/agent-browser-lease-authority" }\n');
        if (mutation === 'reverse-dependency') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/Cargo.toml'), '[package]\nname = "agent-browser-lease-authority"\n[dependencies]\nagent-browser = { path = "../../cli" }\n');
        if (mutation === 'forbidden-cli-import') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use agent_browser_lease_authority::Authority;\n');
        if (mutation === 'upward-crate-import') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/lib.rs'), 'use crate::native::service_lease_authority::Authority;\n');
        if (mutation === 'forbidden-dependency') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/Cargo.toml'), '[package]\nname = "agent-browser-lease-authority"\n[dependencies]\ntokio = "1"\n');
        if (mutation === 'public-signing-type') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/lib.rs'), 'pub struct LeaseAuthoritySigningKey;\n');
        if (mutation === 'public-secret-loader') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/lib.rs'), 'pub fn load_or_create_lease_authority_signing_key() {}\n');
        if (mutation === 'public-claim-map') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/lib.rs'), 'pub struct Authority { pub active_claims: usize }\n');
        if (mutation === 'runtime-owner-upward-import') writeFileSync(join(mutated, 'crates/agent-browser-lease-authority/src/runtime_owner.rs'), 'use crate::native::service_store::ServiceStateRepository;\n');
        if (mutation === 'duplicate-runtime-owner') writeFileSync(join(mutated, 'cli/src/runtime_owner_transfer.rs'), 'pub(crate) struct RuntimeOwnerRegistry;\n');
        if (mutation === 'duplicate-adoption-mode') writeFileSync(join(mutated, 'cli/src/runtime_adoption.rs'), 'pub(crate) enum BrowserAdoptionMode { CooperativeTransfer }\n');
        if (mutation === 'missing-focused-workflow') rmSync(join(mutated, '.github/workflows/lease-authority.yml'));
        const publicRegistryFields = {
          'public-registry-revision': 'revision',
          'public-registry-owners': 'owners',
          'public-registry-principal-bindings': 'principal_bindings',
          'public-registry-lifecycle-records': 'lifecycle_records',
        };
        const publicRegistryField = publicRegistryFields[mutation];
        if (publicRegistryField) {
          const path = 'crates/agent-browser-lease-authority/src/runtime_owner.rs';
          writeFileSync(join(mutated, path), read(path, root).replace(`${publicRegistryField}:`, `pub ${publicRegistryField}:`));
        }
        if (mutation === 'mutable-registry-getter' || mutation === 'renamed-mutable-registry-getter') {
          const path = 'crates/agent-browser-lease-authority/src/runtime_owner.rs';
          const name = mutation === 'mutable-registry-getter' ? 'owners_mut' : 'edit_owner_rows';
          writeFileSync(join(mutated, path), `${read(path, root)}\nimpl RuntimeOwnerRegistry { pub fn ${name}(&mut self) -> &mut BTreeMap<String, ProfileOwner> { &mut self.owners } }\n`);
        }
        if (mutation === 'direct-registry-map') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use agent_browser_lease_authority::Authority;\nfn inspect() { state.runtime_owner_registry\n .owners.values(); }\n');
        if (mutation === 'direct-service-store-registry-map') writeFileSync(join(mutated, 'cli/src/native/service_store.rs'), 'fn inspect() { state.runtime_owner_registry.lifecycle_records.values(); }\n');
        if (mutation === 'direct-registry-revision') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use agent_browser_lease_authority::Authority;\nfn inspect() { state.runtime_owner_registry.revision; }\n');
        if (mutation === 'aliased-registry-revision') writeFileSync(join(mutated, 'cli/src/runtime_owner_transfer.rs'), 'pub(crate) use agent_browser_lease_authority::RuntimeOwnerRegistry;\nfn inspect(registry: &RuntimeOwnerRegistry) { registry.revision; }\n');
        if (mutation === 'cli-registry-literal') writeFileSync(join(mutated, 'cli/src/native/adapter.rs'), 'use agent_browser_lease_authority::Authority;\nfn inspect() { let registry = RuntimeOwnerRegistry { revision: 3 }; }\n');
        if (!check(mutated).length) throw new Error(`${label} mutation was not rejected`);
      } finally { rmSync(mutated, { recursive: true, force: true }); }
    }
    console.log('Lease-authority architecture contract self-test passed');
  } finally { rmSync(root, { recursive: true, force: true }); }
}

if (process.argv.includes('--self-test')) selfTest();
else currentTreeCheck();
