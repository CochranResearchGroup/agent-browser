#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import Ajv2020 from 'ajv/dist/2020.js';

const recordSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-record.v1.schema.json', 'utf8'));
const observationSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-observation.v1.schema.json', 'utf8'));
const readbackSchema = JSON.parse(readFileSync('docs/dev/contracts/service-failure-journal-readback.v1.schema.json', 'utf8'));
const rust = readFileSync('cli/src/native/service_failure_journal.rs', 'utf8');
const controlPlane = readFileSync('cli/src/native/control_plane.rs', 'utf8');
const browser = readFileSync('cli/src/native/browser.rs', 'utf8');
const cdpLoop = readFileSync('cli/src/native/stream/cdp_loop.rs', 'utf8');
const dashboardServer = readFileSync('cli/src/native/stream/dashboard.rs', 'utf8');
const dashboard = readFileSync('packages/dashboard/src/lib/failure-observation.ts', 'utf8');
const viewport = readFileSync('packages/dashboard/src/components/workspace-remote-viewport.tsx', 'utf8');
const handoff = readFileSync('packages/dashboard/src/app/page.tsx', 'utf8');

const ajv = new Ajv2020({ allErrors: true, strict: true, validateFormats: false });
const validateRecord = ajv.compile(recordSchema);
const validateObservation = ajv.compile(observationSchema);
const validateReadback = ajv.compile(readbackSchema);

const record = {
  schemaVersion: 'agent-browser.service-failure-record.v1',
  occurrenceId: 'occurrence-1',
  occurredAt: '2026-09-03T12:00:00Z',
  bootEpoch: 'boot:test',
  runtimeEnvironment: 'development',
  category: 'cdp_stream',
  source: 'authenticated_dashboard_client',
  stage: 'frame_watchdog',
  code: 'cdp_frame_never_received',
  summary: 'Connected without a usable frame.',
  action: 'stream_frame',
  references: { browserId: 'browser-1', handoffIdHash: `sha256:${'a'.repeat(64)}` },
  details: { elapsedMs: 15000 },
};
assert(validateRecord(record), JSON.stringify(validateRecord.errors));
const boundRecord = {...record, buildIdentity: {packageVersion: '0.28.0', sourceRevision: 'a'.repeat(40), sourceTreeState: 'clean', binarySha256: 'b'.repeat(64), supportGenerationId: null, supportManifestSha256: null, unavailableReasons: ['matching_support_generation_unavailable']}};
assert(validateRecord(boundRecord), JSON.stringify(validateRecord.errors));
assert(!validateObservation({category:'dashboard_action', stage:'client', code:'test', summary:'test', observationId:'test', buildIdentity: boundRecord.buildIdentity}), 'client must not forge producer identity');
assert(validateReadback({
  schemaVersion: 'agent-browser.service-failure-journal-readback.v1',
  records: [record],
  malformedLineCount: 1,
  writeFailureCount: 0,
  deliveryOverloadCount: 0,
  deliveryFailureCount: 0,
  pendingRecordCount: 2,
}), JSON.stringify(validateReadback.errors));

for (const category of ['guacamole_load', 'handoff_link', 'cdp_stream', 'dashboard_action']) {
  const observation = {
    category,
    stage: 'external_client',
    code: 'observed_failure',
    summary: 'The client observed a typed failure.',
    observationId: `observation-${category}`,
  };
  assert(validateObservation(observation), JSON.stringify(validateObservation.errors));
}

assert(!validateObservation({
  category: 'browser_launch', stage: 'client', code: 'bad', summary: 'bad', observationId: 'bad',
}), 'client observation schema accepted a server-only category');
assert(!validateObservation({
  category: 'handoff_link', stage: 'client', code: 'bad', summary: 'bad', observationId: 'bad',
  handoffUrl: 'https://private.example/remote-view/secret',
}), 'client observation schema accepted a raw URL field');

assert(rust.includes('failure-journal.jsonl'));
assert(rust.includes('file.sync_data()'));
assert(rust.includes('file.lock()'));
// Lock contention and pending replay are exercised by the Rust filesystem tests.
assert(rust.includes('malformed_line_count'));
assert(rust.includes('Permissions::from_mode(0o600)'));
assert(browser.includes('ServiceFailureCategory::BrowserLaunch'));
assert(cdpLoop.includes('cdp_screencast_start_failed'));
assert(cdpLoop.includes('cdp_frame_never_received'));
assert(cdpLoop.includes('cdp_frame_stream_stalled'));
assert(cdpLoop.includes('append_service_failure_best_effort'));
assert(controlPlane.includes('append_service_failure_best_effort'));
assert(controlPlane.includes('ServiceFailureCategory::DashboardAction'));
assert(dashboardServer.includes('"/api/service/failures"'));
assert(dashboardServer.includes('"/api/service/failure-observation"'));
assert(dashboardServer.includes('dashboard_auth::authenticate_headers'));
assert(dashboard.includes('installDashboardFetchFailureInstrumentation'));
assert(dashboard.includes('FAILURE_OBSERVATION_ROUTE'));
assert(viewport.includes('cdp_frame_never_received'));
assert(viewport.includes('cdp_frame_stream_stalled'));
assert(viewport.includes('guacamole_load'));
// The dashboard now delegates durable handoffs to the canonical presentation
// endpoint; it no longer owns a separate handoff_unusable client flow.
assert(handoff.includes('window.location.replace(`/api/remote-view/'));
assert(handoff.includes('/presentation`);'));

process.stdout.write('Service failure journal contract checks passed\n');

// This registry is deliberately exact-code. A future named producer error in a
// reviewed family cannot inherit a generic plan without review.
const recoveryRegistry = JSON.parse(readFileSync('docs/dev/contracts/public-error-recovery-registry.v1.json', 'utf8'));
for (const path of recoveryRegistry.fragments ?? []) {
  const fragment = JSON.parse(readFileSync(path, 'utf8'));
  assert.equal(fragment.schemaVersion, recoveryRegistry.schemaVersion);
  for (const key of ['recipes', 'codes', 'interfaces']) {
    recoveryRegistry[key] ??= {};
    for (const [name, entry] of Object.entries(fragment[key] ?? {})) {
      assert(!Object.hasOwn(recoveryRegistry[key], name), `Duplicate recovery ${key} ${name}`);
      recoveryRegistry[key][name] = entry;
    }
  }
}
function validateRecoveryInterface(name) {
  const entry = recoveryRegistry.interfaces?.[name];
  assert(entry, `Unavailable recovery interface ${name}`);
  const source = readFileSync(entry.source, 'utf8');
  assert(source.includes(`function ${entry.export}(`) || source.includes(`const ${entry.export} `), `Missing recovery interface symbol ${name}`);
  if (entry.kind === 'client_export') {
    assert(source.includes(`export function ${entry.export}(`) || source.includes(`export async function ${entry.export}(`) || source.includes(`export const ${entry.export} `), `Unexported recovery interface ${name}`);
    const publicModule = name.split('#')[0].replace('@agent-browser/client/', './');
    const exports = JSON.parse(readFileSync('packages/client/package.json', 'utf8')).exports;
    assert.equal(exports[publicModule]?.import, `./${entry.source.replace('packages/client/', '')}`, `Unavailable public module ${publicModule}`);
  } else assert.equal(entry.kind, 'caller_correction', 'Unknown noncallable interface kind');
}

const mcpSource = readFileSync('cli/src/mcp.rs', 'utf8');
const allowedActionsBlock = mcpSource.match(/const BROWSER_COMMAND_ALLOWED_ACTIONS:[\s\S]*?= &\[([\s\S]*?)\];/)[1];
const allowedActions = new Set([...allowedActionsBlock.matchAll(/"([a-z_]+)"/g)].map(match => match[1]));
const toolSchemas = {
  browser_control: readFileSync('cli/src/mcp/host_control.rs', 'utf8'),
  browser_policy: readFileSync('cli/src/mcp/host_policy.rs', 'utf8'),
  browser_continue: readFileSync('cli/src/mcp/workflow.rs', 'utf8'),
  browser_handoff: readFileSync('cli/src/mcp/workflow.rs', 'utf8'),
  browser_discover: readFileSync('cli/src/mcp/catalog.rs', 'utf8'),
};
function validateRecoveryCall(call) {
  if (call.interface === 'caller_error') {
    assert.equal(call.kind, 'source_reference');
    assert.deepEqual(call.fields, ['code', 'source', 'owningFunction', 'message']);
    assert(call.precondition.includes('not a file-read command'));
    const normalization = JSON.parse(readFileSync(call.contract, 'utf8'));
    assert.equal(normalization.schemaVersion, 'agent-browser.client-argument-normalization.v1');
    for (const path of new Set(normalization.reviewedChecks.map(check => check.path))) {
      const reviewed = normalization.reviewedChecks.filter(check => check.path === path).length;
      const emitted = [...readFileSync(path, 'utf8').matchAll(/\bclientArgumentError\(/g)].length;
      assert.equal(emitted, reviewed, `Unreviewed caller normalization added or removed in ${path}`);
    }
    for (const check of normalization.reviewedChecks) {
      const source = readFileSync(check.path, 'utf8');
      assert(source.includes(check.expression) && source.includes(`"${check.owningFunction}"`), `Missing normalized caller check ${check.path}#${check.owningFunction}`);
    }
    return;
  }
  if (call.interface && call.interface !== 'cli') {
    validateRecoveryInterface(call.interface);
    assert(call.precondition?.length > 10, 'Client read interface needs explicit binding preconditions');
    return;
  }
  if (call.interface === 'cli') {
    assert.equal(call.command, 'agent-browser', 'Recovery command must use the current Agent Browser executable');
    assert(['["--help"]', '["--json","install","doctor"]'].includes(JSON.stringify(call.arguments)), `Unavailable recovery CLI arguments ${JSON.stringify(call.arguments)}`);
    assert(readFileSync('cli/src/output.rs', 'utf8').includes('agent-browser install doctor'), 'Independent no-launch doctor must remain advertised');
    return;
  }
  assert(call.tool === 'browser_command' || Object.hasOwn(toolSchemas, call.tool), `Unavailable recovery tool ${call.tool}`);
  if (call.tool === 'browser_discover') {
    assert(['browser', 'diagnostics'].includes(call.arguments.category));
    if (call.arguments.category === 'browser' && call.arguments.operation !== undefined) assert(allowedActions.has(call.arguments.operation), `Unavailable discovery operation ${call.arguments.operation}`);
    else if (call.arguments.category === 'browser') return;
    else {
      const operations = toolSchemas.browser_discover.match(/const DIAGNOSTIC_OPERATIONS:[\s\S]*?= &\[([\s\S]*?)\];/)[1];
      assert(operations.includes(`"${call.arguments.operation}"`), `Unavailable diagnostic operation ${call.arguments.operation}`);
    }
    return;
  }
  if (call.tool === 'browser_command' && call.arguments.action === 'profile_duplicate') {
    assert.equal(call.arguments.params.sourceProfileId, '$sourceProfileId');
    assert.equal(call.arguments.params.operationId, '$operationId');
    const duplicateSchema = readFileSync('cli/src/mcp/catalog.rs', 'utf8').match(/"profile_duplicate" => Some\(json!\(([\s\S]*?)"native_capture" =>/)[1];
    assert(duplicateSchema.includes(`"${call.arguments.params.choice}"`), `Unavailable profile duplicate choice ${call.arguments.params.choice}`);
    return;
  }
  assert.equal(call.arguments.taskId, '$taskId', 'Recovery must retain exact task scope');
  if (call.tool === 'browser_command') {
    assert(allowedActions.has(call.arguments.action), `Unavailable recovery action ${call.arguments.action}`);
    if (call.arguments.action === 'task_operation') {
      assert(call.arguments.params?.operation === 'capacity' || call.arguments.params?.requestId === '$requestId', 'Original/capacity inspection must use supported arguments');
    }
  } else {
    const schema = toolSchemas[call.tool];
    assert(schema.includes(`"name":"${call.tool}"`), `Unrouted recovery tool ${call.tool}`);
    if (call.arguments.operation) {
      const operations = schema.match(/"operation":\{[^}]*"enum":\[([^\]]+)\]/)?.[1];
      assert(operations?.includes(`"${call.arguments.operation}"`), `Unavailable ${call.tool} operation ${call.arguments.operation}`);
    }
  }
}
function validateRecoveryInventory(registry, discovered) {
  const excluded = new Map((registry.exclusions ?? []).map(entry => [entry.code, entry]));
  for (const exclusion of excluded.values()) {
    const source = readFileSync(exclusion.source, 'utf8');
    assert(source.includes(exclusion.code) && source.includes(exclusion.sourceEvidence) && exclusion.reason.length > 20, `Unproven exclusion ${exclusion.code}`);
  }
  for (const code of discovered) assert(registry.codes[code] || excluded.has(code), `Uncovered public code ${code}`);
  for (const [code, entry] of Object.entries(registry.codes)) {
    assert(/^[a-z][a-z0-9_]+$/.test(code));
    const recipe = registry.recipes[entry.recipe];
    assert(recipe, `Missing recipe for ${code}`);
    for (const [action, recipeName] of Object.entries(entry.recipesByAction ?? {})) {
      assert(['browser_session_mode_convert','mode_convert','browser_session_profile_duplicate','profile_duplicate'].includes(action), `Unreviewed owner action ${action}`);
      const variant = registry.recipes[recipeName];
      assert(variant, `Missing action recipe ${recipeName}`);
      validateRecoveryCall(variant.inspect); validateRecoveryCall(variant.verification);
    }

    for (const field of ['rationale', 'requiredActor', 'evidenceRequirement', 'resumeCondition']) assert(recipe[field]?.length > (field === 'requiredActor' ? 0 : 10), `${code} lacks ${field}`);
    assert(['unknown','stale','denied','unavailable','invalid','uncertain','configured_budget','coordination_paused'].includes(entry.evidenceClass), `Missing evidence classification for ${code}`);
    assert.equal(entry.evidenceFreshness, 'not_observed_by_projection', 'Registry must not invent runtime freshness');
    assert(entry.owningChecks.some(check => {
      if (!check.revision) return readFileSync(check.path, 'utf8').includes(code);
      assert.equal(entry.legacy, true, 'Historical anchors are only for retained legacy failures');
      assert(/^[a-f0-9]{40}$/.test(check.revision) && /^(cli\/src|crates|packages)\//.test(check.path) && !check.path.includes('..'), 'Historical source anchor must be immutable and repository scoped');
      return execFileSync('git', ['show', `${check.revision}:${check.path}`], {encoding:'utf8'}).includes(code);
    }), `Missing owning check for ${code}`);
    assert(readFileSync(entry.policySource, 'utf8').trim().length > 0, `Unavailable policy source for ${code}`);
    validateRecoveryCall(recipe.inspect);
    validateRecoveryCall(recipe.verification);
    for (const step of recipe.remediation) {
      assert(step.instruction?.length > 20 && step.precondition?.length > 10, `${code} lacks an actionable remedy precondition`);
      if (step.call) validateRecoveryCall(step.call);
      else {
        assert(step.interface, `${code} has no supported owner interface`);
        if (recoveryRegistry.interfaces?.[step.interface]) validateRecoveryInterface(step.interface);
        else assert(['installation_owner_storage_repair','original_public_request','supported_stable_time_service','retained_native_remote_view_view_only','existing_app_host_binding','existing_app_artifact_reconciliation','existing_profile_copy_owner_reconciliation'].includes(step.interface), `Unavailable recovery owner interface ${step.interface}`);
      }
    }
    assert(recipe.maximumAutomaticAttempts <= 1, 'Recovery cannot create an unbounded retry loop');
  }
}
function rustFiles(root) {
  return readdirSync(root, {withFileTypes: true}).flatMap(entry => entry.isDirectory() ? rustFiles(join(root, entry.name)) : entry.name.endsWith('.rs') ? [join(root, entry.name)] : []);
}
const discoveredReviewedCodes = new Set();
const producerPattern = new RegExp(`["']((?:${recoveryRegistry.enforcedProducerCodePrefixes.join('|')})[a-z0-9_]+)(?=[:"'])`, 'g');
for (const path of [...rustFiles('cli/src'), ...rustFiles('crates/agent-browser-service-model/src')]) {
  const source = readFileSync(path, 'utf8');
  for (const match of source.matchAll(producerPattern)) discoveredReviewedCodes.add(match[1]);
}
validateRecoveryInventory(recoveryRegistry, discoveredReviewedCodes);
assert.throws(() => validateRecoveryInventory(recoveryRegistry, new Set(['unregistered_public_error'])), /Uncovered public code/);
assert.throws(() => validateRecoveryCall({tool:'browser_command',arguments:{taskId:'$taskId',action:'nonexistent_recovery_command'}}), /Unavailable recovery action/);
assert.throws(() => validateRecoveryCall({interface:'cli',command:'agent-browser',arguments:['nonexistent-repair']}), /Unavailable recovery CLI arguments/);
assert.throws(() => validateRecoveryCall({interface:'@agent-browser/client/nonexistent#repair',arguments:{}}), /Unavailable recovery interface/);
assert.throws(() => validateRecoveryCall({tool:'nonexistent_recovery_tool',arguments:{taskId:'$taskId'}}), /Unavailable recovery tool/);
for (const boundary of recoveryRegistry.boundaries) assert(readFileSync(boundary.source, 'utf8').includes(boundary.publication), `Missing ${boundary.surface} publication seam`);
if (process.argv.includes('--require-complete-recovery')) {
  assert.equal(recoveryRegistry.inventoryStatus, 'complete', `Public recovery inventory incomplete: ${recoveryRegistry.knownCoverageGaps.join('; ')}`);
  assert.deepEqual(recoveryRegistry.knownCoverageGaps, []);
}
process.stdout.write(`Exact recovery contracts checked: ${Object.keys(recoveryRegistry.codes).length}; reviewed namespace code candidates checked: ${discoveredReviewedCodes.size}; full inventory: ${recoveryRegistry.inventoryStatus}\n`);
