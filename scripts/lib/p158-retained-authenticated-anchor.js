import { createHash } from 'node:crypto';

import { classifyOperatorUrl } from './p158-external-handoff-oracle.js';

export const P158_RETAINED_ANCHOR_RECEIPT_SCHEMA =
  'agent-browser.p158-retained-authenticated-anchor-receipt.v1';

function sha256(value) {
  return createHash('sha256').update(String(value)).digest('hex');
}

function canonicalize(value) {
  if (value === null || typeof value !== 'object') return value;
  if (Array.isArray(value)) return value.map(canonicalize);
  return Object.fromEntries(
    Object.keys(value).sort().map((key) => [key, canonicalize(value[key])]),
  );
}

export function retainedAnchorReceiptSha256(receipt) {
  return sha256(JSON.stringify(canonicalize(receipt)));
}

function requiredString(value, label, pattern = null) {
  if (typeof value !== 'string' || !value.trim() || (pattern && !pattern.test(value))) {
    throw new Error(`Invalid ${label}`);
  }
  return value;
}

export function validateRetainedAnchorConfig(config) {
  const handoffUrl = requiredString(config?.handoffUrl, 'durable handoff URL');
  const handoff = classifyOperatorUrl(handoffUrl, { role: 'starting_handoff' });
  if (!handoff.valid || handoff.findingCodes.length > 0) {
    throw new Error('Invalid durable handoff URL');
  }
  const markerRegion = structuredClone(config.markerRegion);
  for (const field of ['x', 'y', 'width', 'height']) {
    if (!Number.isInteger(markerRegion?.[field]) || markerRegion[field] < 0) {
      throw new Error(`Invalid marker region ${field}`);
    }
  }
  if (markerRegion.width < 1 || markerRegion.height < 1) {
    throw new Error('Invalid marker region dimensions');
  }
  if (!['viewport', 'remote-view-iframe'].includes(markerRegion.coordinateSpace)) {
    throw new Error('Invalid marker coordinate space');
  }
  return {
    runId: requiredString(config.runId, 'run ID', /^[a-zA-Z0-9._:-]+$/),
    anchorId: requiredString(config.anchorId, 'anchor ID', /^[a-zA-Z0-9._:-]+$/),
    handoffUrl,
    username: requiredString(config.username, 'dashboard username'),
    password: requiredString(config.password, 'dashboard password'),
    expectedMarkerSha256: requiredString(
      config.expectedMarkerSha256,
      'expected marker digest',
      /^[a-f0-9]{64}$/,
    ),
    markerRegion,
  };
}

function classifyObservation(observation, expectedMarkerSha256) {
  const suppliedFindingCodes = observation?.oracleFindingCodes;
  const findingCodes = new Set(Array.isArray(suppliedFindingCodes) ? suppliedFindingCodes : []);
  if (!Array.isArray(suppliedFindingCodes)) findingCodes.add('oracle_evidence_invalid');
  if (observation?.authenticatedSession !== true) findingCodes.add('authenticated_session_unproven');
  if (observation?.markerSha256 !== expectedMarkerSha256) findingCodes.add('synthetic_marker_mismatch');
  if (observation?.iframeCount !== 1 || observation?.guacamoleIframe !== true) {
    findingCodes.add('guacamole_iframe_invalid');
  }
  if (observation?.streamFailure === true) findingCodes.add('stream_failure_rendered');
  if ([...findingCodes].some((code) => typeof code !== 'string' || !/^[a-z0-9_]+$/.test(code))) {
    findingCodes.add('oracle_evidence_invalid');
  }
  const normalized = [...findingCodes]
    .filter((code) => typeof code === 'string' && /^[a-z0-9_]+$/.test(code))
    .sort();
  return {
    authenticatedSession: observation?.authenticatedSession === true,
    markerMatched: observation?.markerSha256 === expectedMarkerSha256,
    iframeReady: observation?.iframeCount === 1 && observation?.guacamoleIframe === true,
    oraclePassed: normalized.length === 0,
    oracleFindingCodes: normalized,
  };
}

function buildReceipt({ config, phase, result, observedAt, observation, stopReason, failureCode }) {
  const evidence = observation
    ? classifyObservation(observation, config.expectedMarkerSha256)
    : {
        authenticatedSession: false,
        markerMatched: false,
        iframeReady: false,
        oraclePassed: false,
        oracleFindingCodes: [],
      };
  const body = {
    schemaVersion: P158_RETAINED_ANCHOR_RECEIPT_SCHEMA,
    planId: 'P158',
    runId: config.runId,
    anchorId: config.anchorId,
    phase,
    sequence: phase === 'ready' ? 1 : 2,
    result,
    observedAt,
    handoffUrlSha256: sha256(config.handoffUrl),
    expectedMarkerSha256: config.expectedMarkerSha256,
    evidence,
    stopReason: phase === 'final' ? stopReason ?? null : null,
    failureCode: result === 'failed' ? failureCode : null,
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

function safeFailureCode(error, fallback) {
  const code = error?.code;
  return typeof code === 'string' && /^[a-z0-9_]+$/.test(code) ? code : fallback;
}

function assertReceiptPrivacy(receipt, config) {
  const serialized = JSON.stringify(receipt);
  for (const secret of [config.handoffUrl, config.username, config.password]) {
    if (serialized.includes(secret)) throw new Error('Anchor receipt privacy boundary failed');
  }
}

async function emitSafeReceipt(emitReceipt, receipt, config) {
  assertReceiptPrivacy(receipt, config);
  await emitReceipt(structuredClone(receipt));
}

export async function runRetainedAuthenticatedAnchor({
  config: rawConfig,
  adapter,
  waitForStop,
  emitReceipt,
  now = () => new Date().toISOString(),
}) {
  const config = validateRetainedAnchorConfig(rawConfig);
  if (!adapter || typeof adapter.open !== 'function' || typeof adapter.observe !== 'function' ||
      typeof adapter.close !== 'function') {
    throw new Error('Invalid retained anchor adapter');
  }
  if (typeof waitForStop !== 'function' || typeof emitReceipt !== 'function') {
    throw new Error('Invalid retained anchor lifecycle callbacks');
  }

  let openAttempted = false;
  let readyObservation = null;
  try {
    try {
      openAttempted = true;
      await adapter.open({
        handoffUrl: config.handoffUrl,
        username: config.username,
        password: config.password,
        markerRegion: structuredClone(config.markerRegion),
        expectedMarkerSha256: config.expectedMarkerSha256,
      });
      readyObservation = await adapter.observe({ phase: 'ready', waitForConvergence: true });
      const readyEvidence = classifyObservation(readyObservation, config.expectedMarkerSha256);
      if (!readyEvidence.oraclePassed) {
        const error = new Error('Retained anchor readiness observation failed');
        error.code = 'anchor_readiness_rejected';
        throw error;
      }
    } catch (error) {
      const receipt = buildReceipt({
        config,
        phase: 'ready',
        result: 'failed',
        observedAt: now(),
        observation: readyObservation,
        failureCode: safeFailureCode(error, 'anchor_open_or_readiness_failed'),
      });
      await emitSafeReceipt(emitReceipt, receipt, config);
      throw error;
    }

    const readyReceipt = buildReceipt({
      config,
      phase: 'ready',
      result: 'passed',
      observedAt: now(),
      observation: readyObservation,
    });
    await emitSafeReceipt(emitReceipt, readyReceipt, config);

    let stopReason;
    try {
      const suppliedStopReason = await waitForStop();
      stopReason = typeof suppliedStopReason === 'string' && /^[a-z0-9_]+$/.test(suppliedStopReason)
        ? suppliedStopReason
        : 'explicit_stop';
    } catch (error) {
      const receipt = buildReceipt({
        config,
        phase: 'final',
        result: 'failed',
        observedAt: now(),
        observation: null,
        stopReason: null,
        failureCode: safeFailureCode(error, 'anchor_stop_wait_failed'),
      });
      await emitSafeReceipt(emitReceipt, receipt, config);
      throw error;
    }
    let finalObservation = null;
    try {
      finalObservation = await adapter.observe({ phase: 'final', waitForConvergence: false });
      const finalEvidence = classifyObservation(finalObservation, config.expectedMarkerSha256);
      if (!finalEvidence.oraclePassed) {
        const error = new Error('Retained anchor final observation failed');
        error.code = 'anchor_final_observation_rejected';
        throw error;
      }
    } catch (error) {
      const receipt = buildReceipt({
        config,
        phase: 'final',
        result: 'failed',
        observedAt: now(),
        observation: finalObservation,
        stopReason,
        failureCode: safeFailureCode(error, 'anchor_final_observation_failed'),
      });
      await emitSafeReceipt(emitReceipt, receipt, config);
      throw error;
    }

    const finalReceipt = buildReceipt({
      config,
      phase: 'final',
      result: 'passed',
      observedAt: now(),
      observation: finalObservation,
      stopReason,
    });
    await emitSafeReceipt(emitReceipt, finalReceipt, config);
    return { readyReceipt, finalReceipt };
  } finally {
    if (openAttempted) await adapter.close().catch(() => {});
  }
}
