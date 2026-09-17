#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');

assert.match(
  workflow,
  /concurrency:\s+group: ci-\$\{\{ github\.workflow \}\}-\$\{\{ github\.event\.pull_request\.number \|\| github\.ref \}\}\s+cancel-in-progress: \$\{\{ github\.event_name == 'pull_request' \}\}/,
  'CI must cancel superseded heads only within the same pull request',
);
assert.match(workflow, /classify:\s+name: Validation Selection[\s\S]*scripts\/dev\/select-validation\.js[\s\S]*--github-output/);
assert.match(workflow, /classify:[\s\S]*name: Test validation control plane[\s\S]*pnpm run test:validation-selection/);
assert.match(workflow, /classify:[\s\S]*service_smokes: \$\{\{ steps\.selection\.outputs\.service_smokes \}\}/);

for (const [job, output] of [
  ['docs', 'docs'],
  ['version-sync', 'version_sync'],
  ['rust-quality', 'rust_quality'],
  ['rust-tests', 'rust'],
  ['dashboard', 'dashboard'],
  ['service-client', 'service_client'],
  ['workstation-fixtures', 'workstation'],
  ['comprehensive', 'comprehensive'],
]) {
  const escapedJob = job.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  assert.match(
    workflow,
    new RegExp(`\\n  ${escapedJob}:[\\s\\S]*?if: needs\\.classify\\.outputs\\.${output} == 'true'`),
    `${job} must be selected by its fixed classifier output`,
  );
}

assert.match(workflow, /presubmit:\s+name: Presubmit\s+if: always\(\)/);
for (const dependency of ['classify', 'docs', 'version-sync', 'rust-quality', 'rust-tests', 'dashboard', 'service-client', 'workstation-fixtures', 'comprehensive']) {
  assert.match(workflow, new RegExp(`\\n      - ${dependency.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(?:\\n|$)`));
}
assert.match(workflow, /name: Verify selected jobs[\s\S]*?shell: bash[\s\S]*?set -euo pipefail[\s\S]*?node scripts\/ci\/verify-presubmit\.js \| tee presubmit-receipt\.json/);
assert.match(workflow, /name: Record validation economics[\s\S]*?if: always\(\)[\s\S]*?shell: bash[\s\S]*?set -euo pipefail[\s\S]*?node scripts\/ci\/report-validation-economics\.js \| tee validation-economics\.json/);
assert.match(workflow, /rust-tests:[\s\S]*?name: Run no-launch service smokes\s+if: needs\.classify\.outputs\.service_smokes == 'true'/);
assert.match(workflow, /name: Run affected Rust compartments[\s\S]*?if \[\[ "\$compartment" == cli-\* \]\][\s\S]*?run_lane "\$\{cli_compartments\[@\]\}"[\s\S]*?run_lane "\$\{support_compartments\[@\]\}"/);
assert.doesNotMatch(workflow, /xargs -r -P2[\s\S]*?rust-tests\.sh --compartment/);
assert.match(workflow, /name: Upload presubmit receipts[\s\S]*?if: always\(\)/);
assert.match(workflow, /qualification_args=\(--qualification comprehensive\)/);
assert.match(workflow, /schedule:\s+- cron: '17 5 1 \* \*'/);

console.log('CI workflow selection and aggregate contract checks passed');
