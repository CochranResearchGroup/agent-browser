import assert from 'node:assert/strict';

import {
  VALIDATION_SELECTION_SCHEMA_VERSION,
  classifyValidationSelection,
} from './lib/validation-selection.js';

function selection(files, options) {
  return classifyValidationSelection(files, options);
}

function expectSelection(name, files, expected) {
  const actual = selection(files, expected.options);
  assert.equal(actual.schemaVersion, VALIDATION_SELECTION_SCHEMA_VERSION, `${name}: schema version`);
  assert.deepEqual(actual.changedFiles, [...new Set(files)].sort(), `${name}: changed files`);
  assert.equal(actual.tier, expected.tier, `${name}: tier`);
  for (const [job, value] of Object.entries(expected.jobs)) {
    assert.equal(actual.jobs[job], value, `${name}: jobs.${job}`);
  }
  if (expected.compartments) {
    assert.deepEqual(actual.rustCompartments, expected.compartments, `${name}: rust compartments`);
  }
  assert.ok(Array.isArray(actual.matchedSurfaces), `${name}: matched surfaces`);
  assert.ok(Array.isArray(actual.unknownFiles), `${name}: unknown files`);
  assert.ok(Array.isArray(actual.exclusions), `${name}: exclusions`);
  assert.ok(Array.isArray(actual.reasons), `${name}: reasons`);
}

expectSelection('docs and governance only', [
  'docs/dev/policies/0042-code-testing-discipline.md',
  'docs/src/app/guides/validation/page.mdx',
  'ROADMAP.md',
  'RUNBOOK.md',
], {
  tier: 'docs',
  jobs: { docs: true, dashboard: false, serviceClient: false, workstation: false, rustQuality: false, rust: false, comprehensive: false },
});

expectSelection('dashboard only', ['packages/dashboard/src/components/service-panel.tsx'], {
  tier: 'dashboard',
  jobs: { docs: false, dashboard: true, serviceClient: false, workstation: false, rustQuality: false, rust: false, comprehensive: false },
});

expectSelection('service client only', ['packages/client/src/service-request.js'], {
  tier: 'service-client',
  jobs: { docs: false, dashboard: false, serviceClient: true, workstation: false, rustQuality: false, rust: false, comprehensive: false },
});

expectSelection('workstation and release only', ['scripts/release/test-verify-release-assets.sh'], {
  tier: 'workstation',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: true, rustQuality: false, rust: false, comprehensive: false },
});

expectSelection('focused Rust compartment', ['crates/agent-browser-cdp/src/lib.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['transport'],
});

expectSelection('multiple known surfaces remain focused', [
  'docs/src/app/guides/validation/page.mdx',
  'packages/dashboard/src/components/service-panel.tsx',
], {
  tier: 'focused',
  jobs: { docs: true, dashboard: true, serviceClient: false, workstation: false, rustQuality: false, rust: false, comprehensive: false },
});

expectSelection('installer Rust selects workstation and focused Rust', ['cli/src/workstation_install.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: true, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['cli-workstation'],
});

expectSelection('general installer Rust also selects workstation fixtures', ['cli/src/install.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: true, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['cli-core'],
});

expectSelection('CLI candidate adapter selects kernel and CLI tests', ['cli/src/candidate_coordination.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['candidate', 'cli-core'],
});

expectSelection('native challenge adapter selects kernel and native tests', ['cli/src/native/challenge_control_action.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['challenge-control', 'cli-native'],
});

expectSelection('service contract Rust selects service client and focused Rust', ['cli/src/native/service_contracts.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: true, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['cli-native'],
});

expectSelection('all named Rust compartments are explicit', [
  'crates/agent-browser-candidate/src/lib.rs',
  'crates/agent-browser-desktop-services/src/lib.rs',
  'crates/agent-browser-lease-authority/src/lib.rs',
  'crates/agent-browser-cdp/src/lib.rs',
  'crates/agent-browser-challenge-control/src/lib.rs',
  'cli/src/commands.rs',
], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['candidate', 'challenge-control', 'cli-core', 'desktop-services', 'lease-authority', 'transport'],
});

expectSelection('unmapped Rust crate fails safe to every Rust compartment', ['crates/new-surface/src/lib.rs'], {
  tier: 'focused',
  jobs: { docs: false, dashboard: false, serviceClient: false, workstation: false, rustQuality: true, rust: true, comprehensive: false },
  compartments: ['candidate', 'challenge-control', 'cli-core', 'cli-integration', 'cli-native', 'cli-workstation', 'desktop-services', 'lease-authority', 'transport'],
});

expectSelection('unknown path fails safe to broad presubmit', ['third_party/new-unknown-surface.txt'], {
  tier: 'broad',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: true, comprehensive: false },
});

expectSelection('classifier change fails safe to broad presubmit', ['scripts/lib/validation-selection.js'], {
  tier: 'broad',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: true, comprehensive: false },
});

expectSelection('workflow change fails safe to broad presubmit', ['.github/workflows/ci.yml'], {
  tier: 'broad',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: true, comprehensive: false },
});

expectSelection('package script change fails safe to broad presubmit', ['package.json'], {
  tier: 'broad',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: true, comprehensive: false },
});

expectSelection('dependency metadata selects comprehensive qualification', ['pnpm-lock.yaml'], {
  tier: 'comprehensive',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: false, comprehensive: true },
});

expectSelection('crate manifest selects comprehensive qualification', ['crates/agent-browser-cdp/Cargo.toml'], {
  tier: 'comprehensive',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: false, comprehensive: true },
});

expectSelection('toolchain change selects comprehensive qualification', ['rust-toolchain.toml'], {
  tier: 'comprehensive',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: false, comprehensive: true },
});

expectSelection('explicit qualification mode selects comprehensive without duplicate focused Rust', [
  'docs/src/app/guides/validation/page.mdx',
], {
  options: { qualificationMode: 'comprehensive' },
  tier: 'comprehensive',
  jobs: { docs: true, dashboard: true, serviceClient: true, workstation: true, rustQuality: true, rust: false, comprehensive: true },
});

console.log('validation selection behavioral fixtures passed');
