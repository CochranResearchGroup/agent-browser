#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join, resolve, relative, dirname } from 'node:path';
import { createRequire } from 'node:module';
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
const externalRecoveryTools = {};
for (const path of recoveryRegistry.fragments ?? []) {
  const fragment = JSON.parse(readFileSync(path, 'utf8'));
  assert.equal(fragment.schemaVersion, recoveryRegistry.schemaVersion);
  if (fragment.externalToolContract) {
    const externalContracts = {
      'docs/dev/contracts/client-external-owner-tools.v1.json':['remote_view_recordings','remote_view_recording_pin'],
      'docs/dev/contracts/service-model-external-owner-tools.v1.json':['remote_view_pool_inspect','remote_view_operation_status'],
    };
    assert(Object.hasOwn(externalContracts,fragment.externalToolContract),'Unreviewed external recovery tool contract');
    const contract = JSON.parse(readFileSync(fragment.externalToolContract, 'utf8'));
    assert(['agent-browser.client-external-owner-tools.v1','agent-browser.service-model-external-owner-tools.v1'].includes(contract.schemaVersion),'Unreviewed external declaration schema');
    for (const [name, declaration] of Object.entries(contract.tools)) {
      assert(externalContracts[fragment.externalToolContract].includes(name), 'Unreviewed external recovery tool');
      assert(declaration.provenance.includes(`mcp__remote_view__${name}`), 'Missing external owner declaration provenance');
      if(Object.hasOwn(externalRecoveryTools,name)) assert.deepEqual(externalRecoveryTools[name],declaration,'Conflicting external owner declaration');
      externalRecoveryTools[name] = declaration;
    }
  }
  for (const key of ['exclusions', 'enforcedProducerCodePrefixes', 'enforcedProducerSources', 'enforcedProducerSourceFamilies', 'legacyProducerChecks', 'legacyNegativeChecks', 'legacyRetirements']) recoveryRegistry[key] = [...(recoveryRegistry[key] ?? []), ...(fragment[key] ?? [])];
  for (const key of ['recipes', 'codes', 'interfaces']) {
    recoveryRegistry[key] ??= {};
    for (const [name, entry] of Object.entries(fragment[key] ?? {})) {
      if (key === 'interfaces' && Object.hasOwn(recoveryRegistry[key], name)) {
        assert.deepEqual(recoveryRegistry[key][name], entry, `Conflicting recovery interface ${name}`);
        continue;
      }
      assert(!Object.hasOwn(recoveryRegistry[key], name), `Duplicate recovery ${key} ${name}`);
      recoveryRegistry[key][name] = entry;
    }
  }
}
function hasExportedClientSymbol(path, symbol, visited = new Set()) {
  assert(/^[A-Za-z_$][A-Za-z0-9_$]*$/.test(symbol), 'Invalid recovery export symbol');
  path = resolve(path);
  assert(path.startsWith(resolve('packages/client/src') + '/'), 'Client re-export escaped source scope');
  if (visited.has(path)) return false;
  visited.add(path);
  const source = readFileSync(path,'utf8');
  if (new RegExp(`export\\s+(?:async\\s+)?function\\s+${symbol}\\s*\\(|export\\s+const\\s+${symbol}\\s*=`).test(source)) return true;
  for (const match of source.matchAll(/export\s*\{([^}]+)\}\s*from\s*['"]([^'"]+)['"]/g)) {
    for (const item of match[1].split(',')) {
      const names = item.trim().match(/^([A-Za-z_$][A-Za-z0-9_$]*)(?:\s+as\s+([A-Za-z_$][A-Za-z0-9_$]*))?$/);
      if (names && (names[2] ?? names[1]) === symbol && match[2].startsWith('.') && hasExportedClientSymbol(resolve(dirname(path),match[2]), names[1], visited)) return true;
    }
  }
  return false;
}

function hasDashboardSourceExport(path, symbol) {
  const ts = createRequire(resolve('docs/package.json'))('typescript');
  const parsed = ts.createSourceFile(path,readFileSync(path,'utf8'),ts.ScriptTarget.Latest,true);
  return parsed.statements.some(statement => {
    if (!statement.modifiers?.some(modifier => modifier.kind === ts.SyntaxKind.ExportKeyword)) return false;
    if (ts.isFunctionDeclaration(statement)) return statement.name?.text === symbol;
    return ts.isVariableStatement(statement) && statement.declarationList.declarations.some(declaration => ts.isIdentifier(declaration.name) && declaration.name.text === symbol);
  });
}

function validateRecoveryInterface(name) {
  const entry = recoveryRegistry.interfaces?.[name];
  assert(entry, `Unavailable recovery interface ${name}`);
  const source = readFileSync(entry.source, 'utf8');
  assert((entry.kind === 'dashboard_export' && hasDashboardSourceExport(entry.source,entry.export)) || source.includes(`function ${entry.export}(`) || source.includes(`const ${entry.export} `) || (entry.kind === 'client_export' && hasExportedClientSymbol(entry.source,entry.export)), `Missing recovery interface symbol ${name}`);
  if (entry.kind === 'client_export') {
    assert(hasExportedClientSymbol(entry.source,entry.export), `Unexported recovery interface ${name}`);
    const publicModule = name.split('#')[0].replace('@agent-browser/client/', './');
    const exports = JSON.parse(readFileSync('packages/client/package.json', 'utf8')).exports;
    assert.equal(exports[publicModule]?.import, `./${entry.source.replace('packages/client/', '')}`, `Unavailable public module ${publicModule}`);
  } else if (entry.kind === 'dashboard_export') {
    assert(/^packages\/dashboard\/src\/lib\/[a-z0-9-]+\.ts$/.test(entry.source), 'Unreviewed dashboard recovery source');
    assert(name.startsWith('dashboard:') && hasDashboardSourceExport(entry.source,entry.export), 'Missing dashboard source export');
  } else assert.equal(entry.kind, 'caller_correction', 'Unknown noncallable interface kind');
}

const mcpSource = readFileSync('cli/src/mcp.rs', 'utf8');
const allowedActionsBlock = mcpSource.match(/const BROWSER_COMMAND_ALLOWED_ACTIONS:[\s\S]*?= &\[([\s\S]*?)\];/)[1];
const allowedActions = new Set([...allowedActionsBlock.matchAll(/"([a-z_]+)"/g)].map(match => match[1]));
const toolSchemas = {
  browser_control: readFileSync('cli/src/mcp/host_control.rs', 'utf8'),
  browser_reconnect: readFileSync('cli/src/mcp/host_reconnect.rs', 'utf8'),
  browser_policy: readFileSync('cli/src/mcp/host_policy.rs', 'utf8'),
  browser_continue: readFileSync('cli/src/mcp/workflow.rs', 'utf8'),
  browser_handoff: readFileSync('cli/src/mcp/workflow.rs', 'utf8'),
  browser_discover: readFileSync('cli/src/mcp/catalog.rs', 'utf8'),
};
function validateClientFactoryInventory(contract, registry) {
  const inventory = contract.reviewedFactoryInventory;
  if (!inventory) return;
  const require = createRequire(resolve('docs/package.json'));
  const ts = require('typescript');
  const root = process.cwd(), seen = new Set(), candidates = [];
  function visitModule(path) {
    path = resolve(path);
    if (seen.has(path) || !existsSync(path) || !path.endsWith('.js')) return;
    seen.add(path);
    const source = readFileSync(path, 'utf8');
    const parsed = ts.createSourceFile(path, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    function walk(node) {
      if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier && ts.isStringLiteral(node.moduleSpecifier) && node.moduleSpecifier.text.startsWith('.')) visitModule(resolve(dirname(path), node.moduleSpecifier.text));
      if ((ts.isCallExpression(node) || ts.isNewExpression(node)) && node.arguments?.[0]) {
        const factory = node.expression.getText(parsed);
        if (['Error','TypeError','RangeError','fail','error','invalid'].includes(factory)) {
          let wrapper = node.parent;
          while (wrapper && !((ts.isCallExpression(wrapper) || ts.isNewExpression(wrapper)) && ['withClientErrorRecovery','clientOwnerFailure'].includes(wrapper.expression.getText(parsed)))) wrapper = wrapper.parent;
          candidates.push({path:relative(root,path),factory,expression:node.arguments[0].getText(parsed),wrapper:wrapper?.expression.getText(parsed)});
        }
      }
      ts.forEachChild(node, walk);
    }
    walk(parsed);
  }
  const pkg = JSON.parse(readFileSync('packages/client/package.json', 'utf8'));
  for (const entry of Object.values(pkg.exports)) if (entry.import) visitModule(resolve('packages/client', entry.import));
  assert.deepEqual([...seen].map(path => relative(root,path)).sort(), [...inventory.modulePaths].sort(), 'Unreviewed exported client import closure');
  const key = entry => JSON.stringify([entry.path,entry.factory,entry.expression]);
  assert.equal(candidates.length, inventory.candidateCount, 'Unreviewed client failure factory count');
  assert.deepEqual(candidates.map(key).sort(), inventory.entries.map(key).sort(), 'Unreviewed client failure factory source');
  for (const entry of inventory.entries) {
    assert(entry.qualification?.length > 0, 'Factory qualification missing');
    const actual = candidates.find(candidate => key(candidate) === key(entry));
    switch (entry.classification) {
      case 'exact_registered_local_failure': assert(registry.recipes[entry.qualification], 'Unregistered client factory recipe'); break;
      case 'reviewed_original_error_preserved_by_public_local_or_owner_normalizer': assert.equal(actual.wrapper,entry.qualification,'Missing original-error normalizer wrapper'); break;
      case 'reviewed_shared_failure_factory': assert(/\/(?:client-recovery|response-failure|client-argument-error|client-wait-budget)\.js$/.test(entry.path), 'Unreviewed shared failure factory'); break;
      case 'qualified_native_literal_or_conditional': assert(contract.checks.some(check => check.path === entry.path && (check.expression.includes(entry.expression) || entry.expression.includes(check.expression))), 'Unqualified Native conditional factory'); break;
      case 'reviewed_original_owner_or_object_guard': assert(/\/service-(?:request|observability)\.js$/.test(entry.path), 'Unreviewed Service owner guard'); break;
      case 'reviewed_caught_installation_configuration_sentinel': assert.equal(entry.path,'packages/client/src/hosted-auth-helper.js'); assert.equal(entry.expression,"'Absolute installation required'"); break;
      case 'reviewed_caught_internal_or_nested_diagnostic': assert(contract.remainingLiteralChecks.some(check => check.path === entry.path && check.expression === entry.expression && check.classification.startsWith('reviewed_')), 'Unqualified nested diagnostic'); break;
      default: assert.fail(`Unreviewed client factory classification ${entry.classification}`);
    }
  }
}

function validateClientLiteralInventory(contract) {
  if (!contract.remainingLiteralChecks) return;
  const ts = createRequire(resolve('docs/package.json'))('typescript');
  const excluded = new Set(['client-recovery.js','client-recovery.generated.js','client-argument-error.js','client-owner-failure.js','client-wait-budget.js','response-failure.js','native-saved-login.js','native-image-challenge.js','challenge-assistance.js','service-request.js','service-observability.js']);
  const found = [];
  function strings(node, carrier = false) {
    if (!node) return [];
    if (ts.isStringLiteral(node) || (!carrier && ts.isNoSubstitutionTemplateLiteral(node))) return carrier || /^[a-z][a-z0-9_]{0,127}$/.test(node.text) ? [node.text] : [];
    if (ts.isConditionalExpression(node)) return [...strings(node.whenTrue,carrier),...strings(node.whenFalse,carrier)];
    if (!carrier && ts.isBinaryExpression(node) && ['??','||'].includes(node.operatorToken.getText())) return [...strings(node.left),...strings(node.right)];
    return [];
  }
  for (const file of readdirSync('packages/client/src').filter(file => file.endsWith('.js') && !file.includes('.generated.') && !excluded.has(file))) {
    const path = 'packages/client/src/' + file;
    const parsed = ts.createSourceFile(path,readFileSync(path,'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
    function walk(node) {
      if (ts.isCallExpression(node) || ts.isNewExpression(node)) {
        const factory = node.expression.getText(parsed);
        const arg = ['fail','error','Error','TypeError','clientFailureError'].includes(factory) ? node.arguments?.[0] : ['result','responseFailureError'].includes(factory) ? node.arguments?.[1] : null;
        let codes = strings(arg);
        if (['Error','TypeError'].includes(factory) && node.parent && ts.isCallExpression(node.parent) && node.parent.expression.getText(parsed) === 'Object.assign') {
          const object = node.parent.arguments[1];
          const code = object && ts.isObjectLiteralExpression(object) ? object.properties.find(property => ts.isPropertyAssignment(property) && property.name.getText(parsed) === 'code') : null;
          if (code) codes = strings(code.initializer);
        }
        for (const code of codes) found.push({path,kind:'local_failure_call',factory,expression:arg?.getText(parsed) ?? node.getText(parsed),code});
      }
      if (ts.isPropertyAssignment(node) && ['reason','reasonCode','category'].includes(node.name.getText(parsed))) for (const code of strings(node.initializer)) found.push({path,kind:'explicit_result_reason',expression:node.initializer.getText(parsed),code});
      if (['login-progress.js','login-discovery.js','owner-login-playbook.js'].includes(file)) {
        let carrier;
        if (ts.isVariableDeclaration(node) && ['category','reason'].includes(node.name.getText(parsed))) carrier = node.initializer;
        if (ts.isBinaryExpression(node) && ['category','reason'].includes(node.left.getText(parsed)) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken) carrier = node.right;
        if (carrier) for (const code of strings(carrier,true)) found.push({path,kind:'reviewed_result_carrier_assignment',expression:carrier.getText(parsed),code});
      }
      ts.forEachChild(node,walk);
    }
    walk(parsed);
  }
  const key = entry => JSON.stringify([entry.path,entry.kind,entry.factory ?? null,entry.expression,entry.code]);
  assert.deepEqual(found.map(key).sort(),contract.remainingLiteralChecks.map(key).sort(),'Unreviewed client literal/result-reason producer');
}

function validateDashboardBoundaryInventory(contract, registry, inventory = contract.dashboardBoundaryInventory, sourceRoot = 'packages/dashboard/src/lib') {
  if (!inventory) return;
  const ts = createRequire(resolve('docs/package.json'))('typescript');
  const files = [], found = [];
  function directory(path) {
    for (const entry of readdirSync(path,{withFileTypes:true})) {
      const item = join(path,entry.name);
      if (entry.isDirectory()) directory(item);
      else if (/\.(ts|tsx|js)$/.test(item)) files.push(item);
    }
  }
  directory(sourceRoot);
  for (const path of files) {
    const parsed = ts.createSourceFile(path,readFileSync(path,'utf8'),ts.ScriptTarget.Latest,true);
    function walk(node) {
      const isReject = ts.isCallExpression(node) && node.expression.getText(parsed) === 'reject';
      const constructor = (ts.isCallExpression(node) || ts.isNewExpression(node)) && ['Error','TypeError','RangeError','DOMException','DashboardReadPausedError','ManualBrowserLaunchRefusal'].includes(node.expression.getText(parsed));
      if (ts.isThrowStatement(node) || isReject || constructor) {
        const expression = ts.isThrowStatement(node) ? node.expression.getText(parsed) : node.getText(parsed);
        let wrapper = node;
        while (wrapper && !((ts.isCallExpression(wrapper) || ts.isNewExpression(wrapper)) && ['dashboardLocalFailure','dashboardResponseFailureError','dashboardConfirmationFailure','foreignCdpFailure'].includes(wrapper.expression.getText(parsed)))) wrapper = wrapper.parent;
        found.push({path,kind:ts.isThrowStatement(node)?'throw':isReject?'reject':'error_constructor',expression,wrapper:wrapper?.expression.getText(parsed)});
      }
      ts.forEachChild(node,walk);
    }
    walk(parsed);
  }
  assert.deepEqual(files.sort(),[...inventory.modulePaths].sort(),'Unreviewed dashboard boundary source module');
  const key = entry => JSON.stringify([entry.path,entry.kind,entry.expression]);
  assert.equal(found.length,inventory.candidateCount,'Unreviewed dashboard boundary count');
  assert.deepEqual(found.map(key).sort(),inventory.entries.map(key).sort(),'Unreviewed dashboard throw/reject/error boundary');
  for (const entry of inventory.entries) {
    const actual = found.find(candidate => key(candidate) === key(entry));
    assert(entry.recourse?.length > 5,'Missing dashboard boundary recourse');
    switch (entry.classification) {
      case 'shared_owner_failure_projection_preserves_whole_owner_recourse': assert(entry.path.endsWith('/response-failure.ts')); break;
      case 'explicit_caller_abort_not_owner_failure_or_replay_authority': assert(entry.expression.includes('DOMException') && entry.expression.includes('AbortError'),'Non-abort error cannot inherit caller cancellation exclusion'); break;
      case 'preserved_original_error_propagation': assert.equal(entry.expression,'error'); break;
      case 'typed_owner_proven_no_effect_failure_preserved': assert(entry.expression.includes('ManualBrowserLaunchRefusal')); break;
      case 'registered_intentional_read_suspension': assert(entry.expression.includes('DashboardReadPausedError') && registry.codes.dashboard_read_paused); break;
      case 'registered_local_or_owner_normalization_boundary': assert(actual.wrapper || entry.expression.startsWith('dashboardLocalFailure(') || entry.expression.includes('dashboardResponseFailureError(') || entry.expression.includes('dashboardConfirmationFailure(') || entry.expression.includes('foreignCdpFailure('),'Missing actual dashboard normalization wrapper'); break;
      default: assert.fail(`Unreviewed dashboard boundary classification ${entry.classification}`);
    }
  }
  for (const check of contract.dashboardLocalChecks ?? []) {
    assert(readFileSync(check.path,'utf8').includes(check.expression),`Missing dashboard local error ${check.path}`);
    assert(registry.codes[check.code],`Uncovered dashboard local code ${check.code}`);
  }
}

function validateDashboardPublicChecks(contract, registry) {
  const ts = createRequire(resolve('docs/package.json'))('typescript');
  for (const check of [...(contract.dashboardPublicChecks ?? []),...(contract.dashboardHookChecks ?? [])]) {
    assert(check.path.startsWith('packages/dashboard/src/'),'Public dashboard check outside reviewed source');
    const source = readFileSync(check.path,'utf8');
    const parsed = ts.createSourceFile(check.path,source,ts.ScriptTarget.Latest,true);
    let actual;
    function walk(node) {
      if (ts.isCallExpression(node) && node.getText(parsed) === check.expression) actual = node;
      ts.forEachChild(node,walk);
    }
    walk(parsed);
    assert(actual,`Missing current dashboard publication call ${check.path}#${check.owningFunction}`);
    assert(registry.codes[check.code],`Uncovered public dashboard code ${check.code}`);
    assert.equal(registry.codes[check.code].recipe,check.recipe,'Dashboard publication recipe mismatch');
    const name = actual.expression.getText(parsed);
    assert(['dashboardLocalFailure','dashboardResponseFailureError','dashboardConfirmationFailure'].includes(name),'Unreviewed dashboard publication normalizer');
    assert(actual.arguments.some(argument => ts.isStringLiteral(argument) && argument.text === check.code),'Dashboard call does not emit its qualified code');
    assert(['local_contract_failure_no_owner_transport','owner_publication_receipt_unconfirmed_preserves_original_ids','owner_response_failure_preserved','local_timing_projection_refresh_only_no_input_authority'].includes(check.classification),'Unreviewed dashboard public classification');
    if(check.classification==='owner_publication_receipt_unconfirmed_preserves_original_ids') {
      assert.equal(name,'dashboardConfirmationFailure');
      assert.equal(actual.arguments[0].getText(parsed),check.payloadExpression,'Dashboard original owner payload lost');
    }
  }
}

function validateClientQualification(contract, registry) {
  assert.equal(contract.schemaVersion, 'agent-browser.client-error-qualification.v1');
  for (const check of contract.remainingLiteralChecks ?? []) {
    assert(readFileSync(check.path,'utf8').includes(check.expression), `Missing client literal source ${check.path}#${check.owningFunction}`);
    if (check.classification === 'exact_registered_local_failure') {
      assert(registry.codes[check.code], `Uncovered client literal failure ${check.code}`);
      assert.equal(registry.codes[check.code].recipe, check.recipe, 'Client literal recipe mismatch');
    } else {
      assert(['reviewed_nonfailure_success_or_retained_evidence_diagnostic','reviewed_nested_diagnostic_not_independent_error_code'].includes(check.classification), 'Unreviewed client literal exclusion');
      assert(check.rationale?.length > 10, 'Client literal exclusion needs source-specific rationale');
      if (check.classification === 'reviewed_nested_diagnostic_not_independent_error_code') assert(check.outerWorkflow?.length > 0, 'Nested diagnostic requires public owner workflow');
    }
  }
  const partition = contract.remainingLiteralPartition;
  if (partition) {
    assert.equal(contract.remainingLiteralChecks.length, partition.total);
    assert.equal(contract.remainingLiteralChecks.filter(check => check.classification === 'exact_registered_local_failure').length, partition.registeredFailureOccurrences);
    assert.equal(contract.remainingLiteralChecks.filter(check => check.classification === 'reviewed_nonfailure_success_or_retained_evidence_diagnostic').length, partition.positiveOrHistoricalAnnotations);
    assert.equal(contract.remainingLiteralChecks.filter(check => check.classification === 'reviewed_nested_diagnostic_not_independent_error_code').length, partition.nestedOrCaughtDiagnostics);
  }
  for (const section of ['authOwnerForwarding','authLocalErrorBoundaries','authResultBoundaries','teachingLocalErrorBoundaries','publicResultObjectBoundaries','authProgressBoundaries']) {
    for (const check of contract[section] ?? []) {
      assert(check.classification?.length > 10 && check.owningFunction?.length > 0 && readFileSync(check.path,'utf8').includes(check.expression), `Missing reviewed client boundary ${section}:${check.path}`);
      for (const code of [check.code,check.normalizationCode].filter(Boolean)) {
        if (code === '$localCode') {
          assert.equal(section, 'teachingLocalErrorBoundaries');
          assert(['packages/client/src/teaching-media.js','packages/client/src/teaching-grilling.js','packages/client/src/workspace-siteguides-host.js'].includes(check.path), 'Unreviewed dynamic local factory');
        } else if (!registry.codes[code]) {
          const nested = contract.remainingLiteralChecks?.some(entry => entry.path === check.path && entry.code === code && entry.classification === 'reviewed_nested_diagnostic_not_independent_error_code');
          const wrapped = contract.reviewedFactoryInventory?.entries.some(entry => entry.path === check.path && check.expression.includes(entry.expression) && entry.classification === 'reviewed_original_error_preserved_by_public_local_or_owner_normalizer');
          assert(nested || wrapped, `Uncovered boundary code ${code}`);
        }
      }
    }
  }
  validateClientFactoryInventory(contract, registry);
  validateClientLiteralInventory(contract);
  validateDashboardBoundaryInventory(contract, registry);
  if (contract.dashboardFrontendInventory) {
    validateDashboardBoundaryInventory(contract,registry,contract.dashboardFrontendInventory,'packages/dashboard/src');
    validateDashboardPublicChecks(contract,registry);
  }

  for (const check of contract.checks) {
    assert(['local_failure_call','explicit_result_reason','local_result_reason_fallback'].includes(check.kind), `Unreviewed client check kind ${check.kind}`);
    const source = readFileSync(check.path, 'utf8');
    assert(source.includes(check.expression), `Missing client check anchor ${check.path}#${check.owningFunction}`);
    assert(check.owningFunction?.length > 0);
    if (check.classification === 'exact_registered_local_failure') {
      assert(registry.codes[check.code], `Uncovered client failure ${check.code}`);
      assert.equal(registry.codes[check.code].recipe, check.recipe);
    } else assert.equal(check.classification, 'verified_success_reason_not_failure');
  }
  for (const boundary of contract.dynamicBoundaries) {
    assert.equal(boundary.automaticExecutionAllowed, false, 'Unknown dynamic client failures cannot authorize automatic effects');
    assert(readFileSync(boundary.path, 'utf8').includes(boundary.owningFunction), 'Missing dynamic failure boundary');
  }
}
const validatedSourceReferences = new Set();
function validateRecoveryCall(call) {
  if (externalRecoveryTools[call.tool]) {
    const declaration = externalRecoveryTools[call.tool];
    const schema = declaration.arguments;
    assert(call.arguments && !Array.isArray(call.arguments), 'External owner arguments must be an object');
    for (const required of schema.required ?? []) assert(Object.hasOwn(call.arguments, required), `Missing external owner argument ${required}`);
    for (const [name, value] of Object.entries(call.arguments)) {
      assert(Object.hasOwn(schema.properties, name), `Unavailable external owner argument ${name}`);
      assert.equal(typeof value, schema.properties[name].type, `Invalid external owner argument ${name}`);
      assert(name !== 'accessToken', 'Recovery recipes must not retain private owner tokens');
    }
    const identity = {remote_view_recordings:'recordingId',remote_view_recording_pin:'recordingId',remote_view_pool_inspect:'poolId',remote_view_operation_status:'operationId'}[call.tool];
    assert.equal(call.arguments[identity],`$${identity}`,'External owner recipe must retain its exact original identity');
    if(['remote_view_pool_inspect','remote_view_operation_status'].includes(call.tool)) {
      assert.deepEqual(schema.required,[identity],'External read must require its actual scoped identity');
      assert.deepEqual(Object.keys(schema.properties),[identity],'Unreviewed external read argument');
      assert.equal(declaration.requiredActor,undefined,'Read-only external declaration cannot grant a new mutation role');
      assert(call.precondition?.length>10,'External owner read requires explicit existing scope and authority');
    }
    return;
  }
  if (call.interface === 'caller_error') {
    assert.equal(call.kind, 'source_reference');
    assert.deepEqual(call.fields, ['code', 'source', 'owningFunction', 'message']);
    assert(call.precondition?.length > 20, 'Source references require an explicit already-returned-error inspection precondition');
    if (validatedSourceReferences.has(call.contract)) return;
    const contract = JSON.parse(readFileSync(call.contract, 'utf8'));
    if (contract.schemaVersion === 'agent-browser.client-error-qualification.v1') {
      validateClientQualification(contract, recoveryRegistry);
    } else {
      assert.equal(contract.schemaVersion, 'agent-browser.client-argument-normalization.v1');
      for (const path of new Set(contract.reviewedChecks.map(check => check.path))) {
        const reviewed = contract.reviewedChecks.filter(check => check.path === path).length;
        const emitted = [...readFileSync(path, 'utf8').matchAll(/\bclientArgumentError\(/g)].length;
        assert.equal(emitted, reviewed, `Unreviewed caller normalization added or removed in ${path}`);
      }
      for (const check of contract.reviewedChecks) {
        const source = readFileSync(check.path, 'utf8');
        assert(source.includes(check.expression) && source.includes(`"${check.owningFunction}"`), `Missing normalized caller check ${check.path}#${check.owningFunction}`);
      }
    }
    validatedSourceReferences.add(call.contract);
    return;
  }
  if (call.interface && call.interface !== 'cli') {
    validateRecoveryInterface(call.interface);
    if (call.method !== undefined) {
      const entry = recoveryRegistry.interfaces[call.interface];
      assert(entry.methods?.includes(call.method), `Undeclared recovery instance method ${call.interface}#${call.method}`);
      const source = readFileSync(entry.source, 'utf8');
      assert(new RegExp(`(?:return\\s*\\{[\\s\\S]*?\\b|^\\s*)${call.method}\\s*[,(:]`, 'm').test(source), `Missing returned recovery instance method ${call.interface}#${call.method}`);
    }
    assert(call.precondition?.length > 10, 'Client read interface needs explicit binding preconditions');
    return;
  }
  if (call.interface === 'cli') {
    assert.equal(call.command, 'agent-browser', 'Recovery command must use the current Agent Browser executable');
    const stateValidate = JSON.stringify(call.arguments) === '["--json","service","state","validate","--path","$statePath"]';
    const handoffInspect = JSON.stringify(call.arguments) === '["--json","remote-view","inspect","$handoffId"]';
    if(handoffInspect) {
      assert(call.precondition?.length>20,'Handoff inspection requires its actual retained ID');
      const commands=readFileSync('cli/src/commands.rs','utf8');
      assert(commands.includes('remote-view inspect <handoff-id>') && /"handoffId"\s*:\s*rest\[1\]\s*,\s*"handoffOperation"\s*:\s*operation/.test(commands),'Missing actual same-link read grammar');
    }
    assert(stateValidate || handoffInspect || ['["--help"]', '["--json","install","doctor"]', '["install","workstation","--dry-run","--runtime-replacement-policy","preserve","--json"]'].includes(JSON.stringify(call.arguments)), `Unavailable recovery CLI arguments ${JSON.stringify(call.arguments)}`);
    if(stateValidate) {
      assert(call.precondition?.length > 10,'State validation requires a reviewed original absolute path binding');
      const commands = readFileSync('cli/src/commands.rs','utf8');
      assert(commands.includes('service state validate --path <absolute-path> [--json]') && commands.includes('"action": "service_state_validate"'),'Missing advertised state validation grammar');
      assert(readFileSync('cli/src/main.rs','utf8').includes('native::service_state_validation::dispatch_service_state_validation(&cmd)'),'State validator must remain public local dispatch');
      assert(readFileSync('cli/src/native/service_state_validation.rs','utf8').includes('Some(validate_service_state_path(&PathBuf::from(path)))'),'State validator must inspect only the provided path');
    }
    assert(readFileSync('cli/src/output.rs', 'utf8').includes('agent-browser install doctor'), 'Independent no-launch doctor must remain advertised');
    if (call.arguments[0] === 'install' && call.arguments[1] === 'workstation') assert(readFileSync('cli/src/connection.rs','utf8').includes(`agent-browser ${call.arguments.join(' ')}`), 'Preserve-mode read must match the existing advertised runtime recovery command');
    return;
  }
  if (call.tool === 'service_request') {
    const args = call.arguments;
    const schema = JSON.parse(readFileSync('docs/dev/contracts/service-request.v1.schema.json','utf8'));
    assert(schema.properties.action.enum.includes(args.action),'Unavailable Service recovery action');
    assert(readFileSync('cli/src/native/service_contracts.rs','utf8').includes(`"${args.action}"`) && mcpSource.includes('"service_request"'),'Service recovery action must retain its actual public owner');
    assert(call.precondition?.length>20,'Service recovery requires exact original authenticated scope');
    const identity = {
      service_authentication_run_status:'authenticationRunId',
      service_challenge_task_status:'challengeTaskId',
    }[args.action];
    if(identity) {
      assert.equal(args[identity],`$${identity}`,'Service recovery must retain its original run/challenge identity');
      assert.equal(args.clientSubjectId,'$clientSubjectId','Service recovery must retain its original principal');
      const fields = ['action',identity,'clientSubjectId',...(identity==='authenticationRunId'?['serviceName','agentName','taskName']:[])];
      assert.deepEqual(Object.keys(args).sort(),fields.sort(),'Unreviewed Service status arguments');
      for(const name of fields.filter(name=>!['action',identity,'clientSubjectId'].includes(name))) assert.equal(args[name],`$${name}`);
      const owner = readFileSync(`cli/src/native/${identity==='authenticationRunId'?'service_authentication_run':'service_challenge_task'}.rs`,'utf8');
      assert(owner.includes(`"${args.action}"`) && owner.includes(`"${identity}"`),'Missing actual original-status owner');
    } else {
      assert.equal(args.action,'service_remote_view_handoff_resolve','Unreviewed Service mutation in recovery contract');
      assert.deepEqual(Object.keys(args).sort(),['action','agentName','params','serviceName','taskName'].sort());
      for(const name of ['serviceName','agentName','taskName']) assert.equal(args[name],`$${name}`);
      assert.deepEqual(args.params,{handoffId:'$handoffId',handoffOperation:'set_link_lifetime',ttlMs:'$reviewedLifetimeMs'},'Same-link repair requires an actual handoffId and separately reviewed lifetime');
      assert(readFileSync('cli/src/native/browser_session_host.rs','utf8').includes('command["handoffOperation"] == "set_link_lifetime"'),'Same-link renewal must remain an actual owner action');
    }
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
  if (call.tool === 'browser_command' && call.arguments.action === 'failure_recovery') {
    const operation = call.arguments.params?.operation;
    assert(['inspect', 'packet', 'history', 'configure', 'run', 'reconcile', 'record_resume'].includes(operation), 'Unavailable recovery journal operation');
    if (operation !== 'configure') assert.equal(call.arguments.params.occurrenceId, '$occurrenceId', 'Recovery must retain original occurrence');
    const schema = readFileSync('cli/src/mcp/failure_recovery.rs', 'utf8');
    assert(schema.includes(`"${operation}"`), 'Recovery operation must be advertised by owner schema');
    return;
  }
  if (call.tool === 'browser_command' && call.arguments.action === 'browser_build') {
    assert.equal(call.arguments.params.profileId, '$profileId');
    assert.equal(call.arguments.params.choice, 'inspect');
    assert(readFileSync('cli/src/mcp/catalog.rs', 'utf8').includes('"select_for_next_launch"'), 'Build inspection schema must remain advertised');
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
function hasFiniteClientStateProducer(code, path) {
  const prefixes = {
    'packages/client/src/owner-login-playbook.js':'auth_',
    'packages/client/src/owner-totp-continuation.js':'auth_',
    'packages/client/src/login-playbook.js':'helper_',
  };
  const prefix = prefixes[path];
  if (!prefix || !code.startsWith(prefix)) return false;
  const suffix = code.slice(prefix.length);
  const ts = createRequire(resolve('docs/package.json'))('typescript');
  const parsed = ts.createSourceFile(path,readFileSync(path,'utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.JS);
  let found = false;
  function walk(node) {
    if (ts.isTemplateExpression(node) && node.head.text === prefix && node.templateSpans.length === 1 && node.templateSpans[0].literal.text === '') {
      const state = node.templateSpans[0].expression.getText(parsed);
      for (let parent = node.parent; parent; parent = parent.parent) {
        if (!ts.isIfStatement(parent)) continue;
        const guard = parent.expression;
        if (ts.isCallExpression(guard) && ts.isPropertyAccessExpression(guard.expression) && guard.expression.name.text === 'includes' && ts.isArrayLiteralExpression(guard.expression.expression) && guard.arguments[0]?.getText(parsed) === state) {
          const states = guard.expression.expression.elements;
          if (states.every(value => ts.isStringLiteral(value)) && states.some(value => value.text === suffix)) found = true;
        }
      }
    }
    ts.forEachChild(node,walk);
  }
  walk(parsed);
  return found;
}

function validateComposedOwningCheck(code, check) {
  const composition = check.codeComposition;
  const pairs = {
    lease_authority_: 'crates/agent-browser-lease-authority/src/authority.rs',
    profile_lease_: 'cli/src/native/service_profile_lease.rs',
    service_principal_: 'crates/agent-browser-lease-authority/src/principal.rs',
    profile_acquisition_principal_: 'crates/agent-browser-lease-authority/src/principal.rs',
    profile_recovery_principal_: 'crates/agent-browser-lease-authority/src/principal.rs',
    profile_reset_principal_: 'crates/agent-browser-lease-authority/src/principal.rs',
  };
  assert.equal(pairs[composition.prefix], composition.source, 'Unreviewed composed producer source');
  const leaseWrapper = ['profile_lease_', 'service_principal_'].includes(composition.prefix);
  assert.equal(check.path, leaseWrapper ? 'cli/src/native/service_profile_lease.rs' : 'cli/src/native/service_profile_recovery.rs', 'Unreviewed producer composition wrapper');
  assert.equal(code, composition.prefix + composition.suffix, 'Composed producer suffix does not match public code');
  assert(/^[a-z][a-z0-9_]*$/.test(composition.suffix), 'Invalid finite producer suffix');
  const expected = composition.prefix === 'profile_lease_'
    ? 'format!("profile_lease_{}:{}", error.code.as_str(), error.message)'
    : composition.prefix === 'service_principal_'
      ? 'format!(\n        "service_principal_{}:{}",\n        error.code.as_str(),\n        error.message\n    )'
      : composition.prefix === 'lease_authority_'
        ? `format!("${composition.prefix}{}", error.as_str())`
        : `format!("${composition.prefix}{}", error.code.as_str())`;
  assert.equal(composition.expression, expected, 'Unreviewed producer composition expression');
  assert(readFileSync(check.path, 'utf8').includes(expected), 'Missing producer composition wrapper');
  assert(composition.sourceEvidence.includes(`"${composition.suffix}"`) && /^Self::[A-Za-z0-9_]+\s*=>/.test(composition.sourceEvidence), 'Composition must identify a canonical enum arm');
  assert(readFileSync(composition.source, 'utf8').includes(composition.sourceEvidence), 'Missing canonical finite producer arm');
  return true;
}

function validateRecoveryInventory(registry, discovered) {
  const excluded = new Map((registry.exclusions ?? []).map(entry => [entry.code, entry]));
  for (const exclusion of excluded.values()) {
    let source = readFileSync(exclusion.source, 'utf8');
    if (exclusion.revision) {
      assert(/^[0-9a-f]{40}$/.test(exclusion.revision) && /^[0-9a-f]{40}$/.test(exclusion.retiredByRevision),'Retired exclusion requires immutable source revisions');
      assert(!source.includes(exclusion.code),`Retired producer reintroduced: ${exclusion.code}`);
      source = execFileSync('git',['show',`${exclusion.revision}:${exclusion.source}`],{encoding:'utf8'});
      const retired = execFileSync('git',['show',`${exclusion.retiredByRevision}:${exclusion.source}`],{encoding:'utf8'});
      assert(!retired.includes(exclusion.code),'Pinned removal revision still emits retired code');
    }
    assert(source.includes(exclusion.code) && source.includes(exclusion.sourceEvidence) && exclusion.reason.length > 20, `Unproven exclusion ${exclusion.code}`);
    if(exclusion.normalizesTo) {
      assert.equal(exclusion.normalizesTo,'client_argument_invalid');
      assert(registry.codes[exclusion.normalizesTo] && exclusion.sourceEvidence.startsWith('clientArgumentError('),'Unproven caller-message normalization');
    }
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
      if (check.codeComposition) return validateComposedOwningCheck(code, check);
      if (!check.revision) return readFileSync(check.path, 'utf8').includes(code) || hasFiniteClientStateProducer(code,check.path);
      assert.equal(entry.legacy, true, 'Historical anchors are only for retained legacy failures');
      assert(/^[a-f0-9]{40}$/.test(check.revision) && /^(cli\/src|crates|packages)\//.test(check.path) && !check.path.includes('..'), 'Historical source anchor must be immutable and repository scoped');
      return execFileSync('git', ['show', `${check.revision}:${check.path}`], {encoding:'utf8'}).includes(code);
    }), `Missing owning check for ${code}`);
    assert(readFileSync(entry.policySource, 'utf8').trim().length > 0, `Unavailable policy source for ${code}`);
    for(const anchor of entry.publicBoundaryAnchors ?? []) assert(readFileSync(anchor.path,'utf8').includes(anchor.sourceEvidence) && anchor.meaning.length>20,`Missing actual public propagation boundary ${code}`);
    validateRecoveryCall(recipe.inspect);
    validateRecoveryCall(recipe.verification);
    for (const step of recipe.remediation) {
      assert(step.instruction?.length > 20 && step.precondition?.length > 10, `${code} lacks an actionable remedy precondition`);
      if (step.call) {
        validateRecoveryCall(step.call);
        const owner = externalRecoveryTools[step.call.tool]?.requiredActor;
        if (owner) assert(recipe.requiredActor.includes(owner) && step.precondition?.length > 20, 'External owner mutation requires explicit actual owner authority');
      }
      else {
        assert(step.interface, `${code} has no supported owner interface`);
        if (recoveryRegistry.interfaces?.[step.interface]) validateRecoveryInterface(step.interface);
        else assert(['installation_owner_storage_repair','original_public_request','supported_stable_time_service','retained_native_remote_view_view_only','existing_app_host_binding','existing_app_artifact_reconciliation','existing_profile_copy_owner_reconciliation'].includes(step.interface), `Unavailable recovery owner interface ${step.interface}`);
      }
    }
    assert(recipe.maximumAutomaticAttempts <= 1, 'Recovery cannot create an unbounded retry loop');
  }
}
// Rust files may resume production helpers after an inline test module. Remove
// only the actual module, respecting literals/comments, rather than truncating
// the rest of the file and silently losing later producer checks.
function withoutRustTestModules(source) {
  const marker = /^#\[cfg\(test\)\]\s*(?:#\[[^\n]*\]\s*)*(?:pub(?:\(crate\))?\s+)?mod\s+\w+\s*\{/gm;
  let match;
  while ((match = marker.exec(source))) {
    let index = marker.lastIndex, depth = 1;
    while (index < source.length && depth) {
      if (source.startsWith('//', index)) {
        const end = source.indexOf('\n', index + 2); index = end < 0 ? source.length : end + 1; continue;
      }
      if (source.startsWith('/*', index)) {
        index += 2; let comments = 1;
        while (index < source.length && comments) {
          if (source.startsWith('/*', index)) { comments++; index += 2; }
          else if (source.startsWith('*/', index)) { comments--; index += 2; }
          else index++;
        }
        continue;
      }
      const raw = source.slice(index).match(/^(?:br|r)(#*)"/);
      if (raw) {
        const end = source.indexOf('"' + raw[1], index + raw[0].length);
        assert(end >= 0, 'Unterminated raw literal in Rust test module');
        index = end + 1 + raw[1].length; continue;
      }
      if (source[index] === '"') {
        index++;
        while (index < source.length) {
          if (source.charCodeAt(index) === 92) index += 2;
          else if (source[index++] === '"') break;
        }
        continue;
      }
      const character = source.slice(index).match(/^'(?:\\.|[^'\\])'/u);
      if (character) { index += character[0].length; continue; }
      if (source[index] === '{') depth++;
      else if (source[index] === '}') depth--;
      index++;
    }
    assert.equal(depth, 0, 'Unterminated Rust test module');
    source = source.slice(0, match.index) + source.slice(index);
    marker.lastIndex = match.index;
  }
  return source;
}
assert.equal(withoutRustTestModules('#[cfg(test)]\nmod tests { fn sample() { let x = r###"}"###; /* { /* } */ } */ } }\nfn live() { "later_producer"; }'), '\nfn live() { "later_producer"; }');

function rustFiles(root) {
  return readdirSync(root, {withFileTypes: true}).flatMap(entry => entry.isDirectory() ? rustFiles(join(root, entry.name)) : entry.name.endsWith('.rs') ? [join(root, entry.name)] : []);
}
const discoveredReviewedCodes = new Set();
const producerPattern = new RegExp(`["']((?:${recoveryRegistry.enforcedProducerCodePrefixes.join('|')})[a-z0-9_]+)(?=[:"'])`, 'g');
for (const path of [...rustFiles('cli/src'), ...rustFiles('crates/agent-browser-service-model/src')]) {
  const source = readFileSync(path, 'utf8');
  for (const match of source.matchAll(producerPattern)) discoveredReviewedCodes.add(match[1]);
}
for (const path of recoveryRegistry.enforcedProducerSources ?? []) {
  const source = readFileSync(path, 'utf8');
  for (const match of source.matchAll(/["'](browser_build_[a-z0-9_]+)(?=[:"'])/g)) discoveredReviewedCodes.add(match[1]);
}
for (const entry of recoveryRegistry.enforcedProducerSourceFamilies ?? []) {
  let source = readFileSync(entry.path, 'utf8');
  if (entry.beforeTestModule || entry.beforeNamedTestModule) source = withoutRustTestModules(source);
  const pattern = new RegExp(`["'](${entry.codePattern})(?=[:"'])`, 'g');
  for (const match of source.matchAll(pattern)) discoveredReviewedCodes.add(match[1]);
}
function validateLegacyProducerContracts(registry) {
  const qualified = new Set();
  for(const retired of registry.legacyRetirements ?? []) {
    assert(!registry.codes[retired.code] && registry.codes[retired.replacementCode],'Retired legacy classifier must use its actual typed replacement contract');
    assert(!readFileSync(retired.classifierPath,'utf8').includes(retired.code),'Obsolete legacy classifier remains active');
    for(const revision of [retired.classifierBeforeRevision,retired.producerBeforeRevision,retired.producerRemovalRevision]) assert(/^[0-9a-f]{40}$/.test(revision),'Legacy retirement requires immutable source evidence');
    assert(execFileSync('git',['show',`${retired.classifierBeforeRevision}:${retired.classifierPath}`],{encoding:'utf8'}).includes(retired.code),'Missing original classifier retirement evidence');
    assert(execFileSync('git',['show',`${retired.producerBeforeRevision}:${retired.producerPath}`],{encoding:'utf8'}).includes(retired.producerEvidence),'Missing original producer retirement evidence');
    assert(!execFileSync('git',['show',`${retired.producerRemovalRevision}:${retired.producerPath}`],{encoding:'utf8'}).includes(retired.producerEvidence),'Retired producer still emitted at removal revision');
    assert(!readFileSync(retired.producerPath,'utf8').includes(retired.producerEvidence),'Retired human-readable producer reintroduced');
  }
  for (const entry of registry.legacyProducerChecks ?? []) {
    assert(registry.codes[entry.code], `Uncovered known legacy producer ${entry.code}`);
    assert.equal(entry.positiveExample.expectedCode,entry.code);
    assert(entry.positiveExample.message?.length > 0 && entry.producerChecks.length > 0,'Legacy classification needs an actual producer and positive example');
    for (const anchor of [...entry.producerChecks,entry.classifierCheck,entry.testCheck]) {
      const source = readFileSync(anchor.path,'utf8');
      assert(source.includes(anchor.sourceEvidence),`Missing legacy source anchor ${entry.code}:${anchor.path}`);
      if (anchor.owningFunction) assert(source.includes(`fn ${anchor.owningFunction}(`),`Missing legacy classifier/test ${anchor.owningFunction}`);
    }
    assert(entry.classifierCheck.sourceEvidence.includes(entry.code),'Legacy classifier must actually emit its exact code');
    qualified.add(entry.code);
  }
  for (const code of Object.keys(registry.codes).filter(code => code.startsWith('browser_action_'))) assert(qualified.has(code),`Missing source-specific legacy classification ${code}`);
  for (const check of registry.legacyNegativeChecks ?? []) {
    const source = readFileSync(check.path,'utf8');
    assert(source.includes(check.sourceEvidence) && source.includes(`fn ${check.owningFunction}(`),'Missing legacy website/unknown negative source');
    assert.equal(check.expectedCode,'service_operation_failed','Unqualified input must remain unclassified');
  }
}
validateLegacyProducerContracts(recoveryRegistry);
validateRecoveryInventory(recoveryRegistry, discoveredReviewedCodes);
const clientQualification = JSON.parse(readFileSync('docs/dev/contracts/public-client-error-qualification.v1.json', 'utf8'));
validateClientQualification(clientQualification, recoveryRegistry);
const unknownClientLiteral = structuredClone(clientQualification);
unknownClientLiteral.remainingLiteralChecks.find(check => check.classification === 'exact_registered_local_failure').code = 'unregistered_client_literal_failure';
assert.throws(() => validateClientQualification(unknownClientLiteral,recoveryRegistry), /Uncovered client literal failure/);
const missingClientFactory = structuredClone(clientQualification);
missingClientFactory.reviewedFactoryInventory.candidateCount++;
assert.throws(() => validateClientFactoryInventory(missingClientFactory,recoveryRegistry), /Unreviewed client failure factory count/);
const missingDashboardBoundary = structuredClone(clientQualification);
missingDashboardBoundary.dashboardBoundaryInventory.candidateCount++;
const missingFrontendBoundary = structuredClone(clientQualification);
if (missingFrontendBoundary.dashboardFrontendInventory) {
  missingFrontendBoundary.dashboardFrontendInventory.candidateCount++;
  assert.throws(()=>validateDashboardBoundaryInventory(missingFrontendBoundary,recoveryRegistry,missingFrontendBoundary.dashboardFrontendInventory,'packages/dashboard/src'),/Unreviewed dashboard boundary count/);
  const uncoveredDashboardPublication = structuredClone(clientQualification);
  uncoveredDashboardPublication.dashboardPublicChecks[0].code='unregistered_dashboard_publication';
  assert.throws(()=>validateDashboardPublicChecks(uncoveredDashboardPublication,recoveryRegistry),/Uncovered public dashboard code/);
}

assert.throws(() => validateDashboardBoundaryInventory(missingDashboardBoundary,recoveryRegistry), /Unreviewed dashboard boundary count/);
const uncoveredClientQualification = structuredClone(clientQualification);
uncoveredClientQualification.checks.find(check => check.classification === 'exact_registered_local_failure').code = 'unregistered_client_failure';
assert.throws(() => validateClientQualification(uncoveredClientQualification, recoveryRegistry), /Uncovered client failure/);

assert.throws(() => validateRecoveryInventory(recoveryRegistry, new Set(['unregistered_public_error'])), /Uncovered public code/);
assert.throws(() => validateRecoveryCall({tool:'browser_command',arguments:{taskId:'$taskId',action:'nonexistent_recovery_command'}}), /Unavailable recovery action/);
assert.throws(() => validateRecoveryCall({interface:'cli',command:'agent-browser',arguments:['nonexistent-repair']}), /Unavailable recovery CLI arguments/);
assert.throws(() => validateRecoveryCall({interface:'@agent-browser/client/nonexistent#repair',arguments:{}}), /Unavailable recovery interface/);
assert.throws(() => validateRecoveryCall({tool:'nonexistent_recovery_tool',arguments:{taskId:'$taskId'}}), /Unavailable recovery tool/);
assert.throws(()=>validateRecoveryCall({tool:'service_request',arguments:{action:'service_remote_view_handoff_resolve',serviceName:'$serviceName',agentName:'$agentName',taskName:'$taskName',params:{handoffOperation:'set_link_lifetime',ttlMs:'$reviewedLifetimeMs'}},precondition:'Retain the exact original authenticated handoff owner'}),/actual handoffId/);
assert.throws(()=>validateRecoveryCall({tool:'service_request',arguments:{action:'service_authentication_run_status',authenticationRunId:'$authenticationRunId',serviceName:'$serviceName',agentName:'$agentName',taskName:'$taskName'},precondition:'Retain the exact original authenticated run owner'}),/original principal/);

for (const check of recoveryRegistry.cliPublicationChecks ?? []) assert(readFileSync(check.path,'utf8').includes(check.sourceEvidence) && check.meaning.length > 20, 'Missing public CLI publication boundary');
for (const check of recoveryRegistry.privateCliExclusions ?? []) assert(readFileSync(check.path,'utf8').includes(check.sourceEvidence) && check.reason.length > 20, 'Unproven private CLI protocol exclusion');
for (const boundary of recoveryRegistry.boundaries) assert(readFileSync(boundary.source, 'utf8').includes(boundary.publication), `Missing ${boundary.surface} publication seam`);
function validateOriginalCandidateCensus(census, registry) {
  assert.equal(census.schemaVersion,'agent-browser.original-error-candidate-census.v1');
  const candidates = Object.keys(census.candidates).sort();
  assert.equal(candidates.length,census.candidateCount,'Original candidate census entry removed');
  assert.equal(createHash('sha256').update(candidates.join('\n')+'\n').digest('hex'),census.candidateCodesSha256,'Original candidate census codes changed without source qualification');
  const dispositions = new Set((registry.exclusions ?? []).map(entry=>entry.code));
  for(const code of candidates) {
    assert(census.candidates[code].length>0 && census.candidates[code].every(path=>typeof path==='string'),'Candidate source provenance missing');
    assert(registry.codes[code] || dispositions.has(code),`Unadjudicated original error candidate ${code}`);
  }
  return candidates.length;
}
const originalCandidateCensus = JSON.parse(readFileSync('docs/dev/contracts/public-original-error-candidate-census.v1.json','utf8'));
const qualifiedOriginalCandidates = validateOriginalCandidateCensus(originalCandidateCensus,recoveryRegistry);
const uncoveredOriginalCensus = structuredClone(originalCandidateCensus);
const originalCode = Object.keys(uncoveredOriginalCensus.candidates)[0];
const withoutOriginal = structuredClone(recoveryRegistry);
delete withoutOriginal.codes[originalCode];
withoutOriginal.exclusions = withoutOriginal.exclusions.filter(entry=>entry.code!==originalCode);
assert.throws(()=>validateOriginalCandidateCensus(uncoveredOriginalCensus,withoutOriginal),/Unadjudicated original error candidate/);

if (process.argv.includes('--require-complete-recovery')) {
  assert.equal(recoveryRegistry.inventoryStatus, 'complete', `Public recovery inventory incomplete: ${recoveryRegistry.knownCoverageGaps.join('; ')}`);
  assert.deepEqual(recoveryRegistry.knownCoverageGaps, []);
}
process.stdout.write(`Exact recovery contracts checked: ${Object.keys(recoveryRegistry.codes).length}; reviewed namespace code candidates checked: ${discoveredReviewedCodes.size}; original candidates adjudicated: ${qualifiedOriginalCandidates}; full inventory: ${recoveryRegistry.inventoryStatus}\n`);

assert.throws(() => validateComposedOwningCheck('unreviewed_anything', {path:'cli/src/native/service_profile_recovery.rs',codeComposition:{prefix:'unreviewed_',source:'fake.rs',suffix:'anything'}}), /Unreviewed composed producer source/);

assert.throws(() => validateRecoveryCall({tool:'remote_view_recording_pin',arguments:{recordingId:'$recordingId',pinned:true,override:true}}), /Unavailable external owner argument/);

assert.throws(() => validateLegacyProducerContracts({...recoveryRegistry,legacyProducerChecks:recoveryRegistry.legacyProducerChecks.slice(1)}), /Missing source-specific legacy classification/);
const falseLegacyRemoval=structuredClone(recoveryRegistry);
falseLegacyRemoval.legacyRetirements[0].producerRemovalRevision=falseLegacyRemoval.legacyRetirements[0].producerBeforeRevision;
assert.throws(()=>validateLegacyProducerContracts(falseLegacyRemoval),/Retired producer still emitted at removal revision/);
