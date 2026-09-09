#!/usr/bin/env node

import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { sha256 } from './lib/p158-campaign-controller.js';
import { compileP158ExecutionSchedule } from './lib/p158-execution-schedule.js';
import {
  P158W7A08Error,
  createP158W7A08LiveBundle,
  enumerateP158W7A08LoggingOperations,
  p158W7A08SourceBinding,
  prepareP158W7A08ReplayManifest,
} from './lib/p158-w7-a08-live.js';

const registry = JSON.parse(fs.readFileSync(
  'docs/dev/contracts/p158-historical-failure-registry.v1.json', 'utf8'));
const schedule = compileP158ExecutionSchedule({ registry, seed: 'p158-w7-a08-live' });
const temporary = fs.mkdtempSync(path.join(os.tmpdir(), 'p158-a08-'));
const candidatePath = path.join(temporary, 'agent-browser-dev');
fs.writeFileSync(candidatePath, 'synthetic frozen candidate\n', { mode: 0o700 });
const candidate = { binaryPath: candidatePath,
  binarySha256: sha256(fs.readFileSync(candidatePath)) };
const environment = {
  environmentId: 'E1', runtimeLane: 'development', production: false,
  tenantDataPresent: false, root: path.join(temporary, 'campaign'),
  home: path.join(temporary, 'campaign', 'control-home'),
  agentHome: path.join(temporary, 'campaign', 'control-agent-home'),
  xdgRuntimeDir: path.join(temporary, 'campaign', 'control-xdg'),
  socketDir: path.join(temporary, 'campaign', 'control-socket'),
};
const validatorHelp = 'agent-browser service state validate --path <absolute-path> --json\n';

function expectCode(code, action) {
  assert.throws(action, (error) => error instanceof P158W7A08Error && error.code === code);
}

function store() {
  const claims = new Map();
  const terminals = new Map();
  return {
    claims, terminals,
    async readClaim(id) { return structuredClone(claims.get(id) ?? null); },
    async readTerminal(id) { return structuredClone(terminals.get(id) ?? null); },
    async appendClaim(receipt) {
      assert(!claims.has(receipt.cellId));
      claims.set(receipt.cellId, structuredClone(receipt));
    },
    async appendTerminal(receipt) {
      assert(!terminals.has(receipt.cellId));
      terminals.set(receipt.cellId, structuredClone(receipt));
    },
  };
}

const manifest = await prepareP158W7A08ReplayManifest({
  campaignRunId: 'p158-a08-provider-free-run', candidate, environment,
  scheduleSha256: schedule.scheduleSha256, liveHookManifestSha256: 'a'.repeat(64),
  environmentSealSha256s: { E1: 'b'.repeat(64) },
  run: async (_binary, args) => {
    if (args[0] === 'service' && args[1] === '--help') {
      return { stdout: validatorHelp, stderr: '' };
    }
    const statePath = args.at(-1);
    const stateBytes = fs.readFileSync(statePath);
    return { stdout: JSON.stringify({ success: true, data: {
      accepted: true, classification: 'accepted', stateSha256: sha256(stateBytes),
      parserIdentitySha256: candidate.binarySha256,
    } }), stderr: '' };
  },
});

assert.equal(manifest.cells.length, 8);
assert.equal(new Set(manifest.cells.map((cell) => cell.rootSha256)).size, 8);
const loggingOperations = enumerateP158W7A08LoggingOperations({
  campaignRunId: manifest.campaignRunId });
assert.equal(loggingOperations.length, 8);
assert(loggingOperations.every((entry) => entry.productRequestId === null &&
  entry.correlationState === 'product_request_id_unavailable' &&
  entry.loggingGap.code === 'product_request_id_not_preserved'));
assert.equal(p158W7A08SourceBinding().sourceSha256.length, 64);
assert.equal(p158W7A08SourceBinding().fixtureSha256.length, 64);

const successfulStore = store();
const successfulCalls = [];
const successfulBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: successfulStore,
  driver: { async execute(cell, operationCorrelationId) {
    successfulCalls.push({ cellId: cell.cellId, operationCorrelationId });
    return { success: false, id: `private-product-id-${cell.cellId}`,
      error: `${cell.expectedFailure}: synthetic failure`,
      data: { url: 'https://private.invalid/path', profilePath: '/private/profile' },
      provenance: { requestedAction: cell.action },
      effectEvidence: { beforeStateSha256: cell.stateSha256,
        afterStateSha256: 'c'.repeat(64), browserEffectObserved: false } };
  } }, clock: () => '2026-09-03T12:00:00Z' });
assert.equal(successfulBundle.freezeEligible, false);
const builtinBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: store(), clock: () => '2026-09-03T12:00:00Z' });
assert.equal(builtinBundle.freezeEligible, true,
  'only the source-owned frozen-candidate driver may be freeze eligible');
const attempt = schedule.attempts.find((entry) => entry.attemptId === 'A08-E1-r001');
const result = await successfulBundle.adapters[0].execute({ attempt });
assert.equal(result.resultState, 'reproduced_historical_failure');
assert.equal(result.actionCount, 8);
assert.equal(successfulCalls.length, 8);
assert(result.receipts.every((receipt) =>
  receipt.resultState === 'reproduced_historical_failure' &&
  receipt.effectState === 'verified_no_browser_effect' &&
  receipt.productRequestIdSha256?.length === 64 &&
  receipt.productRequestId === `private-product-id-${receipt.cellId}`));
assert(!JSON.stringify(result).includes('private.invalid'));
assert(!JSON.stringify(result).includes('/private/profile'));
assert(!JSON.stringify(result.receipts.map((receipt) => receipt.responseEvidence))
  .includes('private-product-id'));

const replayed = await successfulBundle.adapters[0].execute({ attempt });
assert.equal(replayed.resultState, 'reproduced_historical_failure');
assert.equal(successfulCalls.length, 8, 'terminal receipts must prevent replay');

const uncertainStore = store();
const claim = structuredClone(successfulStore.claims.values().next().value);
uncertainStore.claims.set(claim.cellId, claim);
let uncertainInvocations = 0;
const uncertainBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: uncertainStore, driver: { async execute(cell) {
    uncertainInvocations += 1;
    return { error: cell.expectedFailure };
  } }, clock: () => '2026-09-03T12:00:01Z' });
const uncertain = await uncertainBundle.adapters[0].execute({ attempt });
assert.equal(uncertain.resultState, 'safety_stopped');
assert.equal(uncertainStore.terminals.get(claim.cellId).failure.code,
  'a08_claimed_without_terminal');
assert.equal(uncertainInvocations, 7, 'claimed cell must not be replayed');

const mismatchStore = store();
const mismatchBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: mismatchStore, driver: { async execute(cell) {
    return { success: false, error: 'different_product_failure',
      provenance: { requestedAction: cell.action },
      effectEvidence: { beforeStateSha256: cell.stateSha256,
        afterStateSha256: 'd'.repeat(64), browserEffectObserved: false,
        actionOracleSatisfied: false } };
  } }, clock: () => '2026-09-03T12:00:02Z' });
const mismatch = await mismatchBundle.adapters[0].execute({ attempt });
assert.equal(mismatch.resultState, 'new_product_failure');

const fixedStore = store();
const fixedBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: fixedStore, driver: { async execute(cell) {
    const data = {
      launch: { url: 'data:text/html,p158-fixed' },
      remote_view_open: { browserId: 'session:p158-a08-retained-session' },
      tab_switch: { index: 0 },
      view_focus: { broughtToFront: true },
    }[cell.action];
    return { success: true, data, provenance: { requestedAction: cell.action },
      effectEvidence: { beforeStateSha256: cell.stateSha256,
        afterStateSha256: 'e'.repeat(64), browserEffectObserved: true,
        actionOracleSatisfied: true } };
  } }, clock: () => '2026-09-03T12:00:02Z' });
const fixed = await fixedBundle.adapters[0].execute({ attempt });
assert.equal(fixed.resultState, 'passed');
assert.equal(fixed.effectState, 'verified_action_effect');
assert(fixed.receipts.every((receipt) => receipt.resultState === 'passed' &&
  receipt.effectState === 'verified_action_effect'));

const falseSuccessStore = store();
const falseSuccessBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: falseSuccessStore, driver: { async execute(cell) {
    return { success: true, data: {}, provenance: { requestedAction: cell.action },
      effectEvidence: { beforeStateSha256: cell.stateSha256,
        afterStateSha256: 'e'.repeat(64), browserEffectObserved: true,
        actionOracleSatisfied: false } };
  } }, clock: () => '2026-09-03T12:00:02Z' });
const falseSuccess = await falseSuccessBundle.adapters[0].execute({ attempt });
assert.equal(falseSuccess.resultState, 'harness_failure',
  'successful response without an action-specific oracle is not a pass');

const missingEffectStore = store();
const missingEffectBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: missingEffectStore, driver: { async execute(cell) {
    return { error: cell.expectedFailure, provenance: { requestedAction: cell.action } };
  } }, clock: () => '2026-09-03T12:00:02Z' });
const missingEffect = await missingEffectBundle.adapters[0].execute({ attempt });
assert.equal(missingEffect.resultState, 'harness_failure',
  'a matching error string without independent no-effect evidence is not a reproduction');

const harnessStore = store();
const harnessBundle = createP158W7A08LiveBundle({ schedule, replayManifest: manifest,
  receiptStore: harnessStore, driver: { async execute() {
    throw Object.assign(new Error('transport broke at a private path'), { code: 'EPIPE' });
  } }, clock: () => '2026-09-03T12:00:03Z' });
const harness = await harnessBundle.adapters[0].execute({ attempt });
assert.equal(harness.resultState, 'harness_failure');
assert(harness.receipts.every((receipt) => receipt.failure.messageSha256.length === 64));
assert(!JSON.stringify(harness).includes('private path'));

const e0Attempt = schedule.attempts.find((entry) => entry.attemptId === 'A08-E0-r001');
const e0 = await successfulBundle.adapters[0].execute({ attempt: e0Attempt });
assert.equal(e0.resultState, 'skipped_blocked');
assert.equal(e0.actionCount, 0);

const tampered = structuredClone(manifest);
tampered.cells[0].stateSha256 = 'f'.repeat(64);
expectCode('a08_frozen_replay_manifest_invalid', () => createP158W7A08LiveBundle({
  schedule, replayManifest: tampered, receiptStore: store(), driver: { execute() {} },
}));

const productionEnvironment = { ...environment, production: true };
await assert.rejects(prepareP158W7A08ReplayManifest({
  campaignRunId: 'p158-production-refusal', candidate, environment: productionEnvironment,
  scheduleSha256: schedule.scheduleSha256, liveHookManifestSha256: 'a'.repeat(64),
  environmentSealSha256s: { E1: 'b'.repeat(64) }, run: async () => ({ stdout: '{}' }),
}), (error) => error instanceof P158W7A08Error &&
  error.code === 'a08_development_isolation_unproven');

const rejectedRoot = path.join(temporary, 'campaign-rejected');
await assert.rejects(prepareP158W7A08ReplayManifest({
  campaignRunId: 'p158-parser-refusal', candidate,
  environment: { ...environment, root: rejectedRoot,
    home: path.join(rejectedRoot, 'control-home'),
    agentHome: path.join(rejectedRoot, 'control-agent-home'),
    xdgRuntimeDir: path.join(rejectedRoot, 'control-xdg'),
    socketDir: path.join(rejectedRoot, 'control-socket') },
  scheduleSha256: schedule.scheduleSha256, liveHookManifestSha256: 'a'.repeat(64),
  environmentSealSha256s: { E1: 'b'.repeat(64) },
  run: async (_binary, args) => args[0] === 'service' && args[1] === '--help'
    ? { stdout: validatorHelp, stderr: '' }
    : { stdout: JSON.stringify({ data: { accepted: false,
      classification: 'error', stateSha256: sha256(fs.readFileSync(args.at(-1))),
      parserIdentitySha256: candidate.binarySha256 } }) },
}), (error) => error instanceof P158W7A08Error && error.code === 'a08_fixture_parser_rejected');

const defaultEnvironment = { ...environment, root: process.env.HOME,
  home: path.join(process.env.HOME, 'p158-home'), agentHome: path.join(process.env.HOME, '.agent-browser'),
  xdgRuntimeDir: path.join(process.env.HOME, 'p158-xdg'),
  socketDir: path.join(process.env.HOME, 'p158-socket') };
await assert.rejects(prepareP158W7A08ReplayManifest({
  campaignRunId: 'p158-default-refusal', candidate, environment: defaultEnvironment,
  scheduleSha256: schedule.scheduleSha256, liveHookManifestSha256: 'a'.repeat(64),
  environmentSealSha256s: { E1: 'b'.repeat(64) }, run: async () => ({ stdout: '{}' }),
}), (error) => error instanceof P158W7A08Error &&
  error.code === 'a08_development_isolation_unproven');

fs.rmSync(temporary, { recursive: true, force: true });
console.log('P158 W7 A08 live replay tests passed');
