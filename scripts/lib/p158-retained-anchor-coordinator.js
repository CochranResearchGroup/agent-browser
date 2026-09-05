import { createHash } from 'node:crypto';

import {
  P158_RETAINED_ANCHOR_RECEIPT_SCHEMA,
  retainedAnchorReceiptSha256,
} from './p158-retained-authenticated-anchor.js';

export const P158_RETAINED_ANCHOR_CAMPAIGN_AGGREGATE_SCHEMA =
  'agent-browser.p158-retained-anchor-external-campaign-aggregate.v1';

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(
    Object.keys(value).sort()
      .filter((key) => value[key] !== undefined)
      .map((key) => [key, canonicalize(value[key])]),
  );
}

function digest(value) {
  return createHash('sha256').update(JSON.stringify(canonicalize(value))).digest('hex');
}

function withoutKey(value, key) {
  return Object.fromEntries(Object.entries(value ?? {}).filter(([candidate]) => candidate !== key));
}

function safeId(value, label) {
  if (typeof value !== 'string' || !/^[a-zA-Z0-9._:-]+$/.test(value)) {
    throw new Error(`Invalid ${label}`);
  }
  return value;
}

function safeDigest(value, label) {
  if (typeof value !== 'string' || !/^[a-f0-9]{64}$/.test(value)) {
    throw new Error(`Invalid ${label}`);
  }
  return value;
}

function anchorReceiptValid(receipt, { runId, anchorId, handoffUrlSha256, phase }) {
  if (!receipt || receipt.schemaVersion !== P158_RETAINED_ANCHOR_RECEIPT_SCHEMA ||
      receipt.planId !== 'P158' || receipt.runId !== runId || receipt.anchorId !== anchorId ||
      receipt.handoffUrlSha256 !== handoffUrlSha256 || receipt.phase !== phase ||
      receipt.sequence !== (phase === 'ready' ? 1 : 2) || receipt.result !== 'passed' ||
      receipt.receiptSha256 !== retainedAnchorReceiptSha256(withoutKey(receipt, 'receiptSha256')) ||
      receipt.maximumNavigationAttempts !== 1 || receipt.retryAttempted !== false ||
      receipt.repairAttempted !== false || receipt.reconnectAttempted !== false ||
      receipt.productActionAttempted !== false || receipt.privatePixelsRetained !== false ||
      receipt.rawUrlRetained !== false || receipt.secretInputRetained !== false) {
    return false;
  }
  const evidence = receipt.evidence;
  if (evidence?.authenticatedSession !== true || evidence.markerMatched !== true ||
      evidence.iframeReady !== true || evidence.oraclePassed !== true ||
      !Array.isArray(evidence.oracleFindingCodes) || evidence.oracleFindingCodes.length !== 0) {
    return false;
  }
  return phase !== 'final' || receipt.stopReason === 'sigterm';
}

function selectExactAnchorReceipt(receipts, contract) {
  if (!Array.isArray(receipts)) return null;
  const candidates = receipts.filter((receipt) => receipt?.phase === contract.phase);
  if (candidates.length !== 1) return null;
  return anchorReceiptValid(candidates[0], contract) ? candidates[0] : null;
}

function externalClientValid(receipt, { runId, handoffUrlSha256, paceProfile, clientId }) {
  return Boolean(receipt &&
    ['agent-browser.p158-external-vantage-receipt.v1',
      'agent-browser.p158-external-calibration-receipt.v1'].includes(receipt.schemaVersion) &&
    receipt.planId === 'P158' && receipt.runId === runId && receipt.success === true &&
    receipt.clientId === clientId && receipt.paceProfile === paceProfile &&
    receipt.handoff?.urlSha256 === handoffUrlSha256 &&
    receipt.repairAttempted === false && receipt.retryCount === 0 &&
    receipt.runnerRetryCount === 0 && receipt.oracle?.passed === true &&
    receipt.serverPhysicalBrowserLaunchDelta === 0 && receipt.internalUrlLeakCount === 0);
}

function externalEvidenceChecks(dispatchResult, contract) {
  const human = dispatchResult?.humanReceipt;
  const slow = dispatchResult?.slowReceipt;
  const aggregate = dispatchResult?.externalAggregate;
  const workflowPassed = dispatchResult?.workflowConclusion === 'success' &&
    typeof dispatchResult?.workflowRunId === 'string' && /^\d+$/.test(dispatchResult.workflowRunId);
  const humanPassed = externalClientValid(human, {
    ...contract, paceProfile: 'human_controller', clientId: 'external-runner-human',
  });
  const slowPassed = externalClientValid(slow, {
    ...contract, paceProfile: 'slow_concurrency', clientId: 'external-runner-slow',
  });
  const distinctClients = human?.clientId && slow?.clientId && human.clientId !== slow.clientId;
  const aggregateBody = withoutKey(aggregate, 'aggregateSha256');
  const aggregatePassed = Boolean(aggregate &&
    aggregate.schemaVersion === 'agent-browser.p158-external-vantage-aggregate.v1' &&
    aggregate.planId === 'P158' && aggregate.runId === contract.runId &&
    aggregate.success === true && aggregate.handoffUrlSha256 === contract.handoffUrlSha256 &&
    aggregate.repairAttempted === false && aggregate.retryCount === 0 &&
    aggregate.runnerRetryCount === 0 &&
    aggregate.aggregateSha256 === digest(aggregateBody) &&
    Array.isArray(aggregate.clientIds) && aggregate.clientIds.length === 2 &&
    new Set(aggregate.clientIds).size === 2 &&
    aggregate.clientIds.includes(human?.clientId) && aggregate.clientIds.includes(slow?.clientId) &&
    Array.isArray(aggregate.receiptSha256s) && aggregate.receiptSha256s.length === 2 &&
    aggregate.receiptSha256s.includes(digest(human)) &&
    aggregate.receiptSha256s.includes(digest(slow)) &&
    aggregate.checks?.distinctOffHostClients === true &&
    aggregate.checks?.sameDurableHandoff === true &&
    aggregate.checks?.exactRetainedIdentity === true &&
    aggregate.checks?.noDuplicateServerBrowserLaunch === true &&
    aggregate.checks?.noInternalUrlLeak === true &&
    aggregate.checks?.allIngressChecks === true);
  return {
    workflowPassed,
    humanPassed,
    slowPassed,
    distinctClients: Boolean(distinctClients),
    externalAggregatePassed: aggregatePassed,
    humanReceiptSha256: humanPassed ? digest(human) : null,
    slowReceiptSha256: slowPassed ? digest(slow) : null,
    externalAggregateSha256: aggregatePassed ? aggregate.aggregateSha256 : null,
    humanClientId: humanPassed ? human.clientId : null,
    slowClientId: slowPassed ? slow.clientId : null,
    workflowRunId: workflowPassed ? dispatchResult.workflowRunId : null,
  };
}

async function terminate(child) {
  if (typeof child?.terminate === 'function') return child.terminate('SIGTERM');
  if (typeof child?.kill === 'function') return child.kill('SIGTERM');
  throw new Error('Retained anchor child cannot receive SIGTERM');
}

function safeFailureCode(_value, fallback) {
  return fallback;
}

function assertAggregatePrivacy(aggregate, sensitiveValues) {
  const serialized = JSON.stringify(aggregate);
  for (const value of sensitiveValues ?? []) {
    if (typeof value === 'string' && value && serialized.includes(value)) {
      throw new Error('Coordinator aggregate privacy boundary failed');
    }
  }
}

export async function coordinateRetainedAnchorExternalCampaign({
  runId: rawRunId,
  anchorId: rawAnchorId,
  handoffUrlSha256: rawHandoffUrlSha256,
  anchorChild = null,
  startAnchor = null,
  waitForAnchorReceipts,
  dispatchExternalCampaign,
  emitAggregate,
  waitForAnchorExit = null,
  sensitiveValues = [],
  now = () => new Date().toISOString(),
}) {
  const contract = {
    runId: safeId(rawRunId, 'run ID'),
    anchorId: safeId(rawAnchorId, 'anchor ID'),
    handoffUrlSha256: safeDigest(rawHandoffUrlSha256, 'handoff URL digest'),
  };
  if ((!anchorChild && typeof startAnchor !== 'function') ||
      typeof waitForAnchorReceipts !== 'function' ||
      typeof dispatchExternalCampaign !== 'function' || typeof emitAggregate !== 'function') {
    throw new Error('Invalid retained anchor coordinator callbacks');
  }

  const failureCodes = [];
  let child = anchorChild;
  let readyReceipt = null;
  let finalReceipt = null;
  let dispatchResult = null;
  let dispatchAttempted = false;
  let sigtermSent = false;

  try {
    if (!child) child = await startAnchor();
  } catch (error) {
    failureCodes.push(safeFailureCode(error, 'anchor_start_failed'));
  }

  if (child) {
    try {
      const receipts = await waitForAnchorReceipts({ phase: 'ready', child });
      readyReceipt = selectExactAnchorReceipt(receipts, { ...contract, phase: 'ready' });
      if (!readyReceipt) failureCodes.push('anchor_ready_receipt_invalid');
    } catch (error) {
      failureCodes.push(safeFailureCode(error, 'anchor_ready_receipt_unavailable'));
    }
  }

  if (readyReceipt) {
    dispatchAttempted = true;
    try {
      dispatchResult = await dispatchExternalCampaign({
        runId: contract.runId,
        anchorId: contract.anchorId,
        handoffUrlSha256: contract.handoffUrlSha256,
        anchorReadyReceiptSha256: readyReceipt.receiptSha256,
      });
    } catch (error) {
      failureCodes.push(safeFailureCode(error, 'external_dispatch_failed'));
    }
  }

  if (child) {
    try {
      await terminate(child);
      sigtermSent = true;
    } catch (error) {
      failureCodes.push(safeFailureCode(error, 'anchor_sigterm_failed'));
    }
    if (sigtermSent) {
      try {
        const receipts = await waitForAnchorReceipts({ phase: 'final', child });
        finalReceipt = selectExactAnchorReceipt(receipts, { ...contract, phase: 'final' });
        if (!finalReceipt) failureCodes.push('anchor_final_receipt_invalid');
      } catch (error) {
        failureCodes.push(safeFailureCode(error, 'anchor_final_receipt_unavailable'));
      }
    }
    if (typeof waitForAnchorExit === 'function') {
      try {
        await waitForAnchorExit(child);
      } catch (error) {
        failureCodes.push(safeFailureCode(error, 'anchor_exit_unproven'));
      }
    }
  }

  const external = externalEvidenceChecks(dispatchResult, contract);
  if (dispatchAttempted && !external.humanPassed) failureCodes.push('human_receipt_invalid');
  if (dispatchAttempted && !external.slowPassed) failureCodes.push('slow_receipt_invalid');
  if (dispatchAttempted && !external.externalAggregatePassed) {
    failureCodes.push('external_aggregate_invalid');
  }
  if (dispatchResult && !external.workflowPassed) {
    failureCodes.push('external_workflow_not_successful');
  }
  const checks = {
    anchorReadyPassed: Boolean(readyReceipt),
    humanPassed: external.humanPassed,
    slowPassed: external.slowPassed,
    externalAggregatePassed: external.externalAggregatePassed,
    externalWorkflowPassed: external.workflowPassed,
    anchorFinalPassed: Boolean(finalReceipt),
    distinctExternalClients: external.distinctClients,
    sigtermSent,
    zeroProductRetry: Boolean(readyReceipt && finalReceipt &&
      readyReceipt.retryAttempted === false && finalReceipt.retryAttempted === false &&
      dispatchResult?.humanReceipt?.retryCount === 0 &&
      dispatchResult?.slowReceipt?.retryCount === 0 &&
      dispatchResult?.externalAggregate?.retryCount === 0),
    zeroRunnerRetry: Boolean(dispatchResult?.humanReceipt?.runnerRetryCount === 0 &&
      dispatchResult?.slowReceipt?.runnerRetryCount === 0 &&
      dispatchResult?.externalAggregate?.runnerRetryCount === 0),
    noRepair: Boolean(readyReceipt && finalReceipt &&
      readyReceipt.repairAttempted === false && finalReceipt.repairAttempted === false &&
      dispatchResult?.humanReceipt?.repairAttempted === false &&
      dispatchResult?.slowReceipt?.repairAttempted === false &&
      dispatchResult?.externalAggregate?.repairAttempted === false),
  };
  const success = Object.values(checks).every((value) => value === true) && failureCodes.length === 0;
  const body = {
    schemaVersion: P158_RETAINED_ANCHOR_CAMPAIGN_AGGREGATE_SCHEMA,
    planId: 'P158',
    runId: contract.runId,
    anchorId: contract.anchorId,
    handoffUrlSha256: contract.handoffUrlSha256,
    success,
    result: success ? 'passed' : 'failed',
    observedAt: now(),
    dispatchAttempted,
    failureCodes: [...new Set(failureCodes)].sort(),
    checks,
    bindings: {
      anchorReadyReceiptSha256: readyReceipt?.receiptSha256 ?? null,
      humanReceiptSha256: external.humanReceiptSha256,
      slowReceiptSha256: external.slowReceiptSha256,
      externalAggregateSha256: external.externalAggregateSha256,
      anchorFinalReceiptSha256: finalReceipt?.receiptSha256 ?? null,
      humanClientId: external.humanClientId,
      slowClientId: external.slowClientId,
      workflowRunId: external.workflowRunId,
    },
    retryAttempted: false,
    repairAttempted: false,
  };
  const aggregate = { ...body, aggregateSha256: digest(body) };
  assertAggregatePrivacy(aggregate, sensitiveValues);
  await emitAggregate(structuredClone(aggregate));
  return aggregate;
}
