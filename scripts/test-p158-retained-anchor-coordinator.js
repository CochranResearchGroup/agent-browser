#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { sha256 as campaignSha256 } from './lib/p158-campaign-controller.js';
import {
  P158_RETAINED_ANCHOR_RECEIPT_SCHEMA,
  retainedAnchorReceiptSha256,
} from './lib/p158-retained-authenticated-anchor.js';
import {
  P158_RETAINED_ANCHOR_CAMPAIGN_AGGREGATE_SCHEMA,
  coordinateRetainedAnchorExternalCampaign,
} from './lib/p158-retained-anchor-coordinator.js';

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]));
}

function digest(value) {
  return campaignSha256(JSON.stringify(canonicalize(value)));
}

const runId = 'p158-anchor-coordinator-test';
const anchorId = 'local-anchor-1';
const handoffUrlSha256 = 'a'.repeat(64);
const privateValues = [
  'https://public.example.test/remote-view/private-token',
  'private-operator',
  'private-password',
];

function anchorReceipt(phase) {
  const body = {
    schemaVersion: P158_RETAINED_ANCHOR_RECEIPT_SCHEMA,
    planId: 'P158',
    runId,
    anchorId,
    phase,
    sequence: phase === 'ready' ? 1 : 2,
    result: 'passed',
    observedAt: phase === 'ready' ? '2026-09-05T12:00:00.000Z' : '2026-09-05T12:20:00.000Z',
    handoffUrlSha256,
    expectedMarkerSha256: 'b'.repeat(64),
    evidence: {
      authenticatedSession: true,
      markerMatched: true,
      iframeReady: true,
      oraclePassed: true,
      oracleFindingCodes: [],
    },
    stopReason: phase === 'final' ? 'sigterm' : null,
    failureCode: null,
    maximumNavigationAttempts: 1,
    retryAttempted: false,
    repairAttempted: false,
    reconnectAttempted: false,
    productActionAttempted: false,
    privatePixelsRetained: false,
    rawUrlRetained: false,
    secretInputRetained: false,
  };
  return { ...body, receiptSha256: retainedAnchorReceiptSha256(body) };
}

function clientReceipt(clientId, paceProfile) {
  return {
    schemaVersion: 'agent-browser.p158-external-vantage-receipt.v1',
    planId: 'P158',
    runId,
    clientId,
    paceProfile,
    success: true,
    handoff: { urlSha256: handoffUrlSha256 },
    repairAttempted: false,
    retryCount: 0,
    runnerRetryCount: 0,
    oracle: { passed: true },
    serverPhysicalBrowserLaunchDelta: 0,
    internalUrlLeakCount: 0,
  };
}

function externalResult() {
  const humanReceipt = clientReceipt('external-runner-human', 'human_controller');
  const slowReceipt = clientReceipt('external-runner-slow', 'slow_concurrency');
  const body = {
    schemaVersion: 'agent-browser.p158-external-vantage-aggregate.v1',
    planId: 'P158',
    runId,
    success: true,
    repairAttempted: false,
    retryCount: 0,
    runnerRetryCount: 0,
    handoffUrlSha256,
    clientIds: [humanReceipt.clientId, slowReceipt.clientId],
    receiptSha256s: [digest(humanReceipt), digest(slowReceipt)],
    checks: {
      distinctOffHostClients: true,
      sameDurableHandoff: true,
      exactRetainedIdentity: true,
      noDuplicateServerBrowserLaunch: true,
      noInternalUrlLeak: true,
      allIngressChecks: true,
    },
  };
  return {
    humanReceipt,
    slowReceipt,
    externalAggregate: { ...body, aggregateSha256: digest(body) },
    workflowConclusion: 'success',
    workflowRunId: '33945994974',
  };
}

const events = [];
const emitted = [];
const child = {
  async terminate(signal) {
    events.push(`terminate:${signal}`);
  },
};
const aggregate = await coordinateRetainedAnchorExternalCampaign({
  runId,
  anchorId,
  handoffUrlSha256,
  startAnchor: async () => {
    events.push('start');
    return child;
  },
  waitForAnchorReceipts: async ({ phase }) => {
    events.push(`receipt:${phase}`);
    return phase === 'ready' ? [anchorReceipt('ready')] : [anchorReceipt('ready'), anchorReceipt('final')];
  },
  dispatchExternalCampaign: async (binding) => {
    events.push('dispatch');
    assert.equal(binding.anchorReadyReceiptSha256, anchorReceipt('ready').receiptSha256);
    return externalResult();
  },
  waitForAnchorExit: async () => events.push('exit'),
  emitAggregate: async (receipt) => {
    events.push('aggregate');
    emitted.push(receipt);
  },
  sensitiveValues: privateValues,
  now: () => '2026-09-05T12:20:01.000Z',
});
assert.deepEqual(events, [
  'start',
  'receipt:ready',
  'dispatch',
  'terminate:SIGTERM',
  'receipt:final',
  'exit',
  'aggregate',
]);
assert.equal(aggregate.schemaVersion, P158_RETAINED_ANCHOR_CAMPAIGN_AGGREGATE_SCHEMA);
assert.equal(aggregate.success, true);
assert.equal(aggregate.result, 'passed');
assert.deepEqual(aggregate.failureCodes, []);
assert.equal(emitted.length, 1);
assert.deepEqual(emitted[0], aggregate);
for (const value of privateValues) assert.doesNotMatch(JSON.stringify(aggregate), new RegExp(value));

const acceptedEvents = [];
const failedAggregates = [];
const acceptedChild = { kill: (signal) => acceptedEvents.push(`kill:${signal}`) };
const duplicateReadyAggregate = await coordinateRetainedAnchorExternalCampaign({
  runId,
  anchorId,
  handoffUrlSha256,
  anchorChild: acceptedChild,
  waitForAnchorReceipts: async ({ phase }) => {
    acceptedEvents.push(`receipt:${phase}`);
    return phase === 'ready'
      ? [anchorReceipt('ready'), anchorReceipt('ready')]
      : [anchorReceipt('final')];
  },
  dispatchExternalCampaign: async () => {
    acceptedEvents.push('dispatch');
    return externalResult();
  },
  emitAggregate: async (receipt) => failedAggregates.push(receipt),
});
assert.deepEqual(acceptedEvents, ['receipt:ready', 'kill:SIGTERM', 'receipt:final']);
assert.equal(duplicateReadyAggregate.success, false);
assert.equal(duplicateReadyAggregate.dispatchAttempted, false);
assert.deepEqual(duplicateReadyAggregate.failureCodes, ['anchor_ready_receipt_invalid']);

const terminalEvents = [];
const dispatchFailureAggregate = await coordinateRetainedAnchorExternalCampaign({
  runId,
  anchorId,
  handoffUrlSha256,
  anchorChild: {
    terminate: async (signal) => terminalEvents.push(`terminate:${signal}`),
  },
  waitForAnchorReceipts: async ({ phase }) => {
    terminalEvents.push(`receipt:${phase}`);
    return [anchorReceipt(phase)];
  },
  dispatchExternalCampaign: async () => {
    terminalEvents.push('dispatch');
    const error = new Error(`provider included ${privateValues.join(' ')}`);
    error.code = 'private_password';
    throw error;
  },
  emitAggregate: async () => terminalEvents.push('aggregate'),
  sensitiveValues: privateValues,
});
assert.deepEqual(terminalEvents, [
  'receipt:ready',
  'dispatch',
  'terminate:SIGTERM',
  'receipt:final',
  'aggregate',
]);
assert.equal(dispatchFailureAggregate.success, false);
assert.equal(dispatchFailureAggregate.checks.anchorFinalPassed, true);
assert.deepEqual(dispatchFailureAggregate.failureCodes.sort(), [
  'external_aggregate_invalid',
  'external_dispatch_failed',
  'human_receipt_invalid',
  'slow_receipt_invalid',
]);
for (const value of privateValues) {
  assert.doesNotMatch(JSON.stringify(dispatchFailureAggregate), new RegExp(value));
}

const retryResult = externalResult();
retryResult.slowReceipt.runnerRetryCount = 1;
const retryEvents = [];
const retryAggregate = await coordinateRetainedAnchorExternalCampaign({
  runId,
  anchorId,
  handoffUrlSha256,
  anchorChild: { terminate: async () => retryEvents.push('terminate') },
  waitForAnchorReceipts: async ({ phase }) => [anchorReceipt(phase)],
  dispatchExternalCampaign: async () => retryResult,
  emitAggregate: async () => {},
});
assert.equal(retryAggregate.success, false);
assert.equal(retryAggregate.checks.zeroRunnerRetry, false);
assert.equal(retryAggregate.checks.slowPassed, false);
assert.equal(retryEvents.length, 1);

const source = readFileSync('scripts/lib/p158-retained-anchor-coordinator.js', 'utf8');
assert.doesNotMatch(source, /github|gh\s|execFile|spawn\(/i);
assert.doesNotMatch(source, /SIGKILL/);

process.stdout.write('P158 retained anchor coordinator tests passed.\n');
